use crate::db::{consts, frame_key, Db, StateRow, Tex, Wall, Weapon};
use crate::info::{ffact, AmmoDef, Consts, FaceOut, NumberSlot, FONT_GREY, FONT_YELLOW, STAT_AMMO, STAT_AMMO0, STAT_ARMOR, STAT_HEALTH, STAT_MAX0};
use crate::physics::rnd;
use crate::player::PLAYER;
use crate::rules::{bit, matching};
use crate::mobj::Mobj;
use crate::player::Player;
use crate::tic::State;
use crate::wad::{Node, Sector, NONE, TRANSPARENT};
use std::f64::consts::{PI, TAU};
use prela::engine::*;

pub const W: usize = 320;
pub const H: usize = 200;
const VIEW_H: usize = 168;
const CX: f64 = 160.0;
const CY: f64 = 84.0;
const FOCAL: f64 = 160.0;
const NEAR: f64 = 1.0;
const DONTPEGTOP: i64 = 8;
const DONTPEGBOTTOM: i64 = 16;

// Camera position and facing angle.
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ang: f64,
}

impl Pose {
    // Map coords -> camera coords (forward, left).
    fn view(&self, x: f64, y: f64) -> (f64, f64) {
        let (dx, dy) = (x - self.x, y - self.y);
        let (s, c) = self.ang.sin_cos();
        (dx * c + dy * s, -dx * s + dy * c)
    }
}

// Which side (0/1) of a BSP node's split line a point is on.
pub fn point_side(n: Node, x: f64, y: f64) -> usize {
    if n.dx * (y - n.y) - n.dy * (x - n.x) < 0.0 { 0 } else { 1 }
}

// One wall seen in one screen column: depth, texture u, and screen rows of its edges.
#[derive(Clone, Copy)]
struct Panel {
    x: usize,
    depth: f64,
    scale: f64,
    u: f64,
    wall: Wall,
    front: Sector,
    back: Option<Sector>,
    upper: Option<Tex>,
    middle: Option<Tex>,
    lower: Option<Tex>,
    yc: i32,
    yf: i32,
    ybc: i32,
    ybf: i32,
}

impl Panel {
    // Rows this panel hides from walls behind it (solid walls hide the whole column).
    fn closes(&self) -> (i32, i32) {
        match self.back {
            None => (VIEW_H as i32, 0),
            Some(_) => (self.yc.max(self.ybc), self.yf.min(self.ybf)),
        }
    }
}

// Which piece of a column a span is.
#[derive(Clone, Copy)]
enum Part {
    Ceiling,
    Upper,
    Middle,
    Lower,
    Floor,
}

// A vertical run of rows in one column, belonging to one Part.
#[derive(Clone, Copy)]
struct Span {
    p: Panel,
    part: Part,
    y0: i32,
    y1: i32,
}

// One candidate pixel: which texture/texel, light level, and depth.
#[derive(Clone, Copy)]
struct Frag {
    pix: usize,
    gfx: usize,
    u: i64,
    v: i64,
    cm: i64,
    depth: f32,
}

// Screen y -> clamped row index.
fn row(y: f64) -> i32 {
    (y - 0.5).ceil().clamp(0.0, VIEW_H as f64) as i32
}

// Screen columns a wall covers, as a half-open range x0..x1.
fn screen_span(pose: Pose, w: Wall) -> (usize, usize) {
    let (fa, la) = pose.view(w.x1, w.y1);
    let (fb, lb) = pose.view(w.x2, w.y2);
    let clip = |(f0, l0): (f64, f64), (f1, l1): (f64, f64)| {
        if f0 >= NEAR { (f0, l0) } else { (NEAR, l0 + (NEAR - f0) / (f1 - f0) * (l1 - l0)) }
    };
    let (a, b) = (clip((fa, la), (fb, lb)), clip((fb, lb), (fa, la)));
    let sx = |(f, l): (f64, f64)| CX - l / f * FOCAL;
    let visible = fa >= NEAR || fb >= NEAR;
    let x0 = if visible { (sx(a) - 0.5).ceil().clamp(0.0, W as f64) as usize } else { 0 };
    let x1 = if visible { (sx(b) - 0.5).ceil().clamp(0.0, W as f64) as usize } else { 0 };
    (x0, x1.max(x0))
}

// Position along the wall and depth where screen column x hits it.
fn column(pose: Pose, w: Wall, x: usize) -> (f64, f64) {
    let (fa, la) = pose.view(w.x1, w.y1);
    let (fb, lb) = pose.view(w.x2, w.y2);
    let k = (CX - (x as f64 + 0.5)) / FOCAL;
    let t = ((k * fa - la) / ((lb - la) - k * (fb - fa))).clamp(0.0, 1.0);
    (t, (fa + t * (fb - fa)).max(NEAR))
}

// Build the Panel for one wall in one column.
fn panel(pose: Pose, x: usize, t: f64, depth: f64, row_: PanelRow) -> Panel {
    let (wall, front, back, upper, middle, lower) = row_;
    let scale = FOCAL / depth;
    let y = |h: f64| row(CY - (h - pose.z) * scale);
    let len = ((wall.x2 - wall.x1).powi(2) + (wall.y2 - wall.y1).powi(2)).sqrt();
    let (yc, yf) = (y(front.ceil), y(front.floor));
    let (ybc, ybf) = back.map_or((yc, yf), |b| (y(b.ceil), y(b.floor)));
    Panel { x, depth, scale, u: wall.offset + t * len, wall, front, back, upper, middle, lower, yc, yf, ybc, ybf }
}

// A wall with its front/back sectors and upper/middle/lower textures.
type PanelRow = (Wall, Sector, Option<Sector>, Option<Tex>, Option<Tex>, Option<Tex>);

// The rows of a panel's still-visible stretch (top..bot) that belong to one part, if any.
fn span(pose: Pose, sky_flat: usize, (p, (top, bot)): (Panel, (i32, i32)), part: Part) -> Option<Span> {
    let sp = |y0: i32, y1: i32| (y0 < y1).then_some(Span { p, part, y0, y1 });
    match (part, p.back) {
        (Part::Ceiling, _) if p.front.ceil > pose.z => sp(top, bot.min(p.yc)),
        (Part::Floor, _) if p.front.floor < pose.z => sp(top.max(p.yf), bot),
        (Part::Middle, None) => sp(top.max(p.yc), bot.min(p.yf)),
        (Part::Upper, Some(b)) if b.ceil < p.front.ceil && !(p.front.ceilpic == sky_flat && b.ceilpic == sky_flat) => sp(top.max(p.yc), bot.min(p.ybc)),
        (Part::Lower, Some(b)) if b.floor > p.front.floor => sp(top.max(p.ybf), bot.min(p.yf)),
        _ => None,
    }
}

// Sector light level -> colormap index (higher = darker).
fn light(level: i64, delta: i64) -> i64 {
    (15 - ((level >> 4) + delta).clamp(0, 15)) * 4
}

// The fragment for screen row y of a span: wall texel, floor/ceiling texel, or sky (None for an untextured wall).
fn frag(pose: Pose, sky: usize, sky_flat: usize, s: Span, y: i32) -> Option<Frag> {
    let p = s.p;
    let pix = y as usize * W + p.x;
    let ry = y as f64 + 0.5 - CY;
    let plane = |h: f64, pic: usize| {
        if pic == sky_flat {
            let a = pose.ang + ((CX - (p.x as f64 + 0.5)) / FOCAL).atan();
            let u = (a / std::f64::consts::TAU * 1024.0).floor() as i64;
            return Frag { pix, gfx: sky, u, v: y as i64, cm: 0, depth: f32::MAX };
        }
        let dist = (pose.z - h) * FOCAL / ry;
        let k = (CX - (p.x as f64 + 0.5)) / FOCAL;
        let (sn, cs) = pose.ang.sin_cos();
        let wx = pose.x + dist * (cs - k * sn);
        let wy = pose.y + dist * (sn + k * cs);
        let cm = (light(p.front.light, 0) - 80 / ((dist / 16.0) as i64 + 1)).clamp(0, 31);
        Frag { pix, gfx: pic, u: wx.floor() as i64, v: (-wy).floor() as i64, cm, depth: dist as f32 }
    };
    let wall = |t: Option<Tex>, gfx: usize, vref: f64| {
        t.map(|_| {
            let v = vref + p.wall.yoff - pose.z + ry / p.scale;
            let cm = (light(p.front.light, p.wall.light_delta) - (p.scale * 8.0).min(23.0) as i64).clamp(0, 31);
            Frag { pix, gfx, u: p.u.floor() as i64, v: v.floor() as i64, cm, depth: p.depth as f32 }
        })
    };
    let th = |t: Option<Tex>| t.map_or(0.0, |t| t.h as f64);
    let back = p.back.unwrap_or(p.front);
    match s.part {
        Part::Ceiling => Some(plane(p.front.ceil, p.front.ceilpic)),
        Part::Floor => Some(plane(p.front.floor, p.front.floorpic)),
        Part::Middle => {
            let vref = if p.wall.flags & DONTPEGBOTTOM != 0 { p.front.floor + th(p.middle) } else { p.front.ceil };
            wall(p.middle, p.wall.middle, vref)
        }
        Part::Upper => {
            let vref = if p.wall.flags & DONTPEGTOP != 0 { p.front.ceil } else { back.ceil + th(p.upper) };
            wall(p.upper, p.wall.upper, vref)
        }
        Part::Lower => {
            let vref = if p.wall.flags & DONTPEGBOTTOM != 0 { p.front.ceil } else { back.floor };
            wall(p.lower, p.wall.lower, vref)
        }
    }
}

// Camera at the player's eye height.
pub fn pose(p: Player, k: Consts) -> Pose {
    Pose { x: p.x, y: p.y, z: p.z + if p.dead { k.dead_view_height } else { k.view_height }, ang: p.ang }
}

// Which of 8 sprite rotations faces the camera.
fn rotation(pose: Pose, m: Mobj) -> usize {
    let a = (m.y - pose.y).atan2(m.x - pose.x);
    (((a - m.ang + PI * 9.0 / 8.0).rem_euclid(TAU) / (PI / 4.0)) as usize).min(7) + 1
}

// A thing sprite projected to the screen: image placement, scale, light, and the screen rectangle it covers.
#[derive(Clone, Copy)]
struct Billboard {
    gfx: usize,
    flip: bool,
    t: Tex,
    x0: f64,
    y0: f64,
    scale: f64,
    depth: f32,
    cm: i64,
    xa: i32,
    xb: i32,
    ya: i32,
    yb: i32,
}

// Project a thing sprite.
fn billboard(pose: Pose, m: Mobj, row: StateRow, (gfx, flip): (usize, bool), t: Tex, sec: Sector) -> Billboard {
    let (f, l) = pose.view(m.x, m.y);
    let scale = FOCAL / f.max(NEAR);
    let x0 = CX - l * scale - t.left as f64 * scale;
    let y0 = CY - (m.z + t.top as f64 - pose.z) * scale;
    let cm = if row.bright { 0 } else { (light(sec.light, 0) - (scale * 8.0).min(23.0) as i64).clamp(0, 31) };
    let visible = f >= 4.0;
    let span = |a: f64, n: usize, lim: usize| {
        if visible { ((a - 0.5).ceil().clamp(0.0, lim as f64) as i32, (a + n as f64 * scale - 0.5).ceil().clamp(0.0, lim as f64) as i32) } else { (0, 0) }
    };
    let (xa, xb) = span(x0, t.w, W);
    let (ya, yb) = span(y0, t.h, VIEW_H);
    Billboard { gfx, flip, t, x0, y0, scale, depth: f as f32, cm, xa, xb, ya, yb }
}

// The sprite texel at screen pixel (x, y).
fn sprite_frag(b: Billboard, x: i32, y: i32) -> Frag {
    let t = b.t;
    let u = (((x as f64 + 0.5 - b.x0) / b.scale) as i64).clamp(0, t.w as i64 - 1);
    let v = (((y as f64 + 0.5 - b.y0) / b.scale) as i64).clamp(0, t.h as i64 - 1);
    let u = if b.flip { t.w as i64 - 1 - u } else { u };
    Frag { pix: y as usize * W + x as usize, gfx: b.gfx, u, v, cm: b.cm, depth: b.depth }
}

// An unscaled screen-space graphic (weapon, HUD): top-left corner after offsets, light, depth.
#[derive(Clone, Copy)]
struct Stamp {
    gfx: usize,
    t: Tex,
    x0: i64,
    y0: i64,
    cm: i64,
    depth: f32,
}

fn stamp(x0: f64, y0: f64, gfx: usize, t: Tex, cm: i64, depth: f32) -> Stamp {
    Stamp { gfx, t, x0: (x0 - t.left as f64) as i64, y0: (y0 - t.top as f64) as i64, cm, depth }
}

// Image columns / rows of a stamp that land on screen.
fn stamp_cols(s: Stamp) -> (i64, i64) {
    ((-s.x0).max(0), (s.t.w as i64).min(W as i64 - s.x0))
}

fn stamp_rows(s: Stamp) -> (i64, i64) {
    ((-s.y0).max(0), (s.t.h as i64).min(H as i64 - s.y0))
}

fn stamp_frag(s: Stamp, u: i64, v: i64) -> Frag {
    Frag { pix: (s.y0 + v) as usize * W + (s.x0 + u) as usize, gfx: s.gfx, u, v, cm: s.cm, depth: s.depth }
}

// Facts the face rules match on.
fn face_facts(p: Player) -> u32 {
    bit(p.dead, ffact::DEAD) | bit(p.damagecount > 0, ffact::HURT) | bit(p.bonuscount > 0 && p.wanted != NONE, ffact::GRIN)
}

// Face image key: kind, pain level from health, and a look direction that changes every 17 tics.
fn face_key(p: Player, o: FaceOut, tic: u64) -> (u8, usize, usize) {
    let pain = ((100 - p.health.clamp(0, 100)) * 5 / 101) as usize;
    let look = (rnd(tic / 17, PLAYER, 99) % 3) as usize;
    (o.kind, pain * o.by_pain as usize, look * o.by_look as usize)
}

// Palette to use: red when hurt, gold after a pickup.
fn tint(p: Player) -> usize {
    if p.damagecount > 0 {
        ((p.damagecount + 7) >> 3).min(8) as usize
    } else if p.bonuscount > 0 {
        8 + ((p.bonuscount + 7) >> 3).min(4) as usize
    } else {
        0
    }
}

// Draw one frame into `out` as a single query.
pub fn render(db: &Db, s: &State, out: &mut [u32]) {
    let sectors = &s.sectors;
    let k = consts(db);
    let pose = (&s.player).fold_flat(Pose { x: 0.0, y: 0.0, z: 0.0, ang: 0.0 }, |_, p: Player| pose(p, k));
    let (sky, sky_flat) = (db.sky, db.sky_flat);
    let tex_of = |f: fn(Wall) -> usize| (&db.walls).map(f).select(&db.tex).opt();
    // Every wall joined with its sectors and textures.
    let rows = (&db.walls)
        .and((&db.walls).map(|w: Wall| w.fsec).select(sectors))
        .and((&db.walls).map(|w: Wall| w.bsec).select(sectors).opt())
        .and(tex_of(|w| w.upper).and(tex_of(|w| w.middle)).and(tex_of(|w| w.lower)))
        .map(|(((w, f), b), ((u, m), l))| (w, f, b, u, m, l));
    // Project walls to columns; per column, nearest first, track rows already covered by closer walls.
    let panels = rows
        .expand(move |r: PanelRow| screen_span(pose, r.0))
        .map(move |(r, x): (PanelRow, usize)| {
            let (t, d) = column(pose, r.0, x);
            panel(pose, x, t, d, r)
        })
        .key_by(|p: Panel| p.x)
        .window(
            preceding((0, VIEW_H as i32), |(t, b): (i32, i32), p: Panel| {
                let (ct, cb) = p.closes();
                (t.max(ct), b.min(cb))
            }),
            |p: Panel| p.depth,
            |a: &f64, b: &f64| a.total_cmp(b),
        );
    // Wall, floor, ceiling and sky fragments.
    // Every column splits into these pieces, top to bottom.
    let parts: VecRel<usize, Part> = VecRel::new(vec![Part::Ceiling, Part::Upper, Part::Middle, Part::Lower, Part::Floor]);
    let world = (&panels)
        .cross(&parts)
        .flat_map(move |(pc, part): ((Panel, (i32, i32)), Part)| span(pose, sky_flat, pc, part))
        .expand(|s: Span| (s.y0, s.y1))
        .flat_map(move |(s, y): (Span, i32)| frag(pose, sky, sky_flat, s, y))
        .key_by(|f: Frag| f.pix);
    // Thing sprites: state -> sprite frame (rotated, else rotation 0) -> texture, lit by its sector.
    let sprites = (&s.mobjs)
        .key_by(|m: Mobj| m.state)
        .and(&db.t.states)
        .key_by(move |(m, r): (Mobj, StateRow)| frame_key(r.sprite, r.frame, rotation(pose, m)))
        .and((&db.t.sprite_frames).opt())
        .key_by(|((_, r), _): ((Mobj, StateRow), Option<(usize, bool)>)| frame_key(r.sprite, r.frame, 0))
        .and((&db.t.sprite_frames).opt())
        .flat_map(|(((m, r), a), b): (((Mobj, StateRow), Option<(usize, bool)>), Option<(usize, bool)>)| a.or(b).map(|g| (m, r, g)))
        .key_by(|(_, _, (g, _))| g)
        .and(&db.tex)
        .key_by(|((m, _, _), _): ((Mobj, StateRow, (usize, bool)), Tex)| m.sector)
        .and(sectors)
        .map(move |(((m, r, g), t), sec): (((Mobj, StateRow, (usize, bool)), Tex), Sector)| billboard(pose, m, r, g, t, sec))
        .expand(|b: Billboard| (b.xa, b.xb))
        .expand(|(b, _): (Billboard, i32)| (b.ya, b.yb))
        .map(|((b, x), y): ((Billboard, i32), i32)| sprite_frag(b, x, y))
        .key_by(|f: Frag| f.pix);
    // Weapon and muzzle flash overlay.
    let weapon = (&s.player)
        .filt(|p: Player| p.wstate != 0)
        .map(|p: Player| (p, p.wstate, 0.5f32))
        .union((&s.player).filt(|p: Player| p.flash != 0).map(|p: Player| (p, p.flash, 0.25f32)))
        .key_by(|(_, st, _)| st)
        .and(&db.t.states)
        .key_by(|((_, _, _), r): ((Player, usize, f32), StateRow)| frame_key(r.sprite, r.frame, 0))
        .and(&db.t.sprite_frames)
        .key_by(|(_, (g, _)): (((Player, usize, f32), StateRow), (usize, bool))| g)
        .and(&db.tex)
        .key_by(|((((p, _, _), _), _), _): ((((Player, usize, f32), StateRow), (usize, bool)), Tex)| p.sector)
        .and(sectors)
        .map(|(((((_, _, d), r), (g, _)), t), sec): (((((Player, usize, f32), StateRow), (usize, bool)), Tex), Sector)| {
            let cm = if r.bright { 0 } else { (light(sec.light, 0) - 23).clamp(0, 31) };
            stamp(1.0, 31.5 - (100.0 - CY), g, t, cm, d)
        })
        .expand(stamp_cols)
        .expand(|(s, _): (Stamp, i64)| stamp_rows(s))
        .map(|((s, u), v): ((Stamp, i64), i64)| stamp_frag(s, u, v))
        .key_by(|f: Frag| f.pix);
    // Status bar: values for the number widgets.
    let player = &s.player;
    let stats = player
        .map(|p: Player| p.weapon)
        .select(&db.t.weapons)
        .map(|w: Weapon| w.ammo)
        .select((&s.ammo).and(&db.t.ammo_defs))
        .filt(|(_, k): (i32, AmmoDef)| k.cap > 0)
        .key_by(|_| STAT_AMMO)
        .map(|(a, _)| a)
        .union(player.key_by(|_| STAT_HEALTH).map(|p: Player| p.health))
        .union(player.key_by(|_| STAT_ARMOR).map(|p: Player| p.armor))
        .union(
            (&s.ammo)
                .and(&db.t.ammo_defs)
                .and(Same::<usize>::new())
                .filt(|((_, k), _): ((i32, AmmoDef), usize)| k.cap > 0)
                .key_by(|(_, kind)| STAT_AMMO0 + kind)
                .map(|((a, _), _)| a),
        )
        .union(
            (&db.t.ammo_defs)
                .and(Same::<usize>::new())
                .filt(|(k, _): (AmmoDef, usize)| k.cap > 0)
                .key_by(|(_, kind)| STAT_MAX0 + kind)
                .map(|(k, _): (AmmoDef, usize)| k.cap),
        );
    // One glyph per digit place, right-aligned using the width of the font's 0, no leading zeros.
    let digits = (&stats)
        .and(&db.t.hud_numbers)
        .key_by(|(_, n): (i32, NumberSlot)| (n.font, 0))
        .and(&db.t.hud_fonts)
        .key_by(|(_, g)| g)
        .and(&db.tex)
        .map(|(((v, n), _), t): (((i32, NumberSlot), usize), Tex)| (v.max(0), n, t.w as f64))
        .expand(|(_, n, _): (i32, NumberSlot, f64)| (0, n.digits as u32))
        .filt(|((v, _, _), place): ((i32, NumberSlot, f64), u32)| place == 0 || v >= 10i32.pow(place))
        .map(|((v, n, w), place): ((i32, NumberSlot, f64), u32)| ((n.font, (v / 10i32.pow(place) % 10) as usize), n.x - w * (place as f64 + 1.0), n.y));
    let percents = (&stats)
        .and(&db.t.hud_numbers)
        .filt(|(_, n): (i32, NumberSlot)| n.percent)
        .map(|(_, n): (i32, NumberSlot)| ((n.font, 10), n.x, n.y));
    // Arms panel: owned weapons in yellow, the rest in grey.
    let arms = (&db.t.hud_arms).cross(player);
    let arms = (&arms)
        .filt(|((w, _, _, _), p): ((usize, f64, f64, usize), Player)| p.owned & (1 << w) != 0)
        .map(|((_, x, y, d), _): ((usize, f64, f64, usize), Player)| ((FONT_YELLOW, d), x, y))
        .union((&arms).filt(|((w, _, _, _), p): ((usize, f64, f64, usize), Player)| p.owned & (1 << w) == 0).map(|((_, x, y, d), _): ((usize, f64, f64, usize), Player)| ((FONT_GREY, d), x, y)));
    let glyphs = digits
        .key_by(|_| 0usize)
        .union(percents.key_by(|_| 0usize))
        .union(arms.key_by(|_| 0usize))
        .key_by(|(f, _, _)| f)
        .and(&db.t.hud_fonts)
        .map(|((_, x, y), g): (((usize, usize), f64, f64), usize)| (g, x, y, 0.0f32));
    // Keys held, and the face picked by the face rules.
    let keys = (&db.t.hud_keys)
        .and(Same::<usize>::new())
        .cross(player)
        .filt(|(((_, _, _), k), p): (((f64, f64, usize), usize), Player)| p.keys & (1 << k) != 0)
        .map(|(((x, y, g), _), _): (((f64, f64, usize), usize), Player)| (g, x, y, 0.0f32));
    let tic = s.tic;
    let face_rows: HashIdx<usize, (Player, usize, u32)> = player.map(|p: Player| (p, 0usize, face_facts(p))).collect();
    let face = matching(&face_rows, &db.t.face_rules, |_| PLAYER)
        .key_by(move |(p, o): (Player, FaceOut)| face_key(p, o, tic))
        .and(&db.t.faces)
        .map(|(_, g)| (g, 143.0, 168.0, 0.0f32));
    let status = (&db.t.hud_patches)
        .key_by(|_| 0usize)
        .union(glyphs.key_by(|_| 0usize))
        .union(keys.key_by(|_| 0usize))
        .union(face.key_by(|_| 0usize))
        .key_by(|(g, _, _, _)| g)
        .and(&db.tex)
        .map(|((g, x, y, d), t): ((usize, f64, f64, f32), Tex)| stamp(x, y, g, t, 0, d))
        .expand(stamp_cols)
        .expand(|(s, _): (Stamp, i64)| stamp_rows(s))
        .map(|((s, u), v): ((Stamp, i64), i64)| stamp_frag(s, u, v))
        .key_by(|f: Frag| f.pix);
    // Composite: texel -> palette index, drop transparent, apply light, keep nearest per pixel, apply tint, -> RGB.
    let image = world
        .union(sprites)
        .union(weapon)
        .union(status)
        .key_by(|f: Frag| f.gfx)
        .and(&db.tex)
        .map(|(f, t): (Frag, Tex)| (f, t.base + f.u.rem_euclid(t.w as i64) as usize * t.h + f.v.rem_euclid(t.h as i64) as usize))
        .key_by(|(_, texel)| texel)
        .and(&db.texels)
        .filt(|(_, pal)| pal != TRANSPARENT)
        .key_by(|((f, _), pal): ((Frag, usize), u16)| f.cm as usize * 256 + pal as usize)
        .and(&db.colormap)
        .key_by(|(((f, _), _), _)| f.pix)
        .map(|(((f, _), _), c): (((Frag, usize), u16), u8)| ((f.depth.to_bits() as u64) << 8) | c as u64)
        .dense_fold(W * H, u64::MAX, |a: u64, k: u64| a.min(k))
        .cross((&s.player).map(tint))
        .map(|(k, pal): (u64, usize)| pal * 256 + (k & 0xFF) as usize)
        .select(&db.palette);
    out.fill(0);
    image.drive(|(pix, _), rgb| out[pix] = rgb);
}
