use crate::db::{self, frame_key, glyph_key, Db};
use crate::info::{ffact, FONT_GREY, FONT_YELLOW, STAT_AMMO, STAT_AMMO0, STAT_ARMOR, STAT_HEALTH, STAT_MAX0};
use crate::physics::{rnd, Geom};
use crate::player::PLAYER;
use crate::rules::{bit, matching};
use crate::schema::{one, AmmoKind, AnimState, FaceRule, HudKey, Image, NumberSlot, Player, Seg, Weapon};
use crate::tic::State;
use crate::wad::{NONE, TRANSPARENT};
use prela::engine::*;
use prela::loader::Col;
use std::f64::consts::{PI, TAU};

// Screen size; the 3D view is the top VIEW_H rows, the status bar below.
pub const W: usize = 320;
pub const H: usize = 200;
const VIEW_H: usize = 168;
// View centre, focal length in pixels (90° field of view), near clip distance.
const CX: f64 = 160.0;
const CY: f64 = 84.0;
const FOCAL: f64 = 160.0;
const NEAR: f64 = 1.0;
// Linedef flags that change how textures are aligned vertically.
const DONTPEGTOP: i64 = 8;
const DONTPEGBOTTOM: i64 = 16;

// The values flowing through the frame query are plain tuples, named here.

// Camera: x, y, eye z, facing angle.
pub type Pose = (f64, f64, f64, f64);
// A wall's front sector: floor, ceiling, floor flat, ceiling flat, light.
type Front = (f64, f64, Id<Image>, Id<Image>, i64);
// A wall's back sector: floor, ceiling, ceiling flat.
type Back = (f64, f64, Id<Image>);
// A wall texture slot: the image and its height, if the slot has one.
type TexRef = Option<(Id<Image>, f64)>;
// Everything drawn for one seg: segment, (u offset, v offset, line flags, light delta),
// front and back sectors, and the upper, middle and lower textures.
type WallRow = (Geom, (f64, f64, i64, i64), Front, Option<Back>, (TexRef, TexRef, TexRef));
// One wall in one screen column: x, depth, scale, texture u, the screen rows of the
// front ceiling/floor and back ceiling/floor edges, and the wall.
type Panel = (usize, f64, f64, f64, (i32, i32, i32, i32), WallRow);
// Rows y0..y1 of one part of a panel's column.
type Span = (Panel, Part, i32, i32);
// One candidate pixel: pixel index, image, texel column and row, colormap level, depth.
type Frag = (usize, Id<Image>, i64, i64, i64, f32);
// An image's width, height and drawing offsets.
type Img = (usize, usize, i64, i64);
// A projected sprite: image, mirrored, size, screen corner, scale, depth, colormap level,
// and the screen rectangle it covers (x range, y range).
type Billboard = (Id<Image>, bool, Img, f64, f64, f64, f32, i64, (i32, i32, i32, i32));
// An unscaled screen graphic (weapon, HUD): image, size, top-left corner, colormap level, depth.
type Stamp = (Id<Image>, Img, i64, i64, i64, f32);

// Which piece of a column a span is.
#[derive(Clone, Copy)]
enum Part {
    Ceiling,
    Upper,
    Middle,
    Lower,
    Floor,
}

// Map coords -> camera coords (forward, left).
fn view((px, py, _, ang): Pose, x: f64, y: f64) -> (f64, f64) {
    let (dx, dy) = (x - px, y - py);
    let (s, c) = ang.sin_cos();
    (dx * c + dy * s, -dx * s + dy * c)
}

// Screen y -> clamped row index.
fn row(y: f64) -> i32 {
    (y - 0.5).ceil().clamp(0.0, VIEW_H as f64) as i32
}

// Screen columns a wall covers, as a half-open range x0..x1.
fn screen_span(pose: Pose, (x1, y1, x2, y2): Geom) -> (usize, usize) {
    let (fa, la) = view(pose, x1, y1);
    let (fb, lb) = view(pose, x2, y2);
    // Pull an endpoint behind the camera forward to the near plane.
    let clip = |(f0, l0): (f64, f64), (f1, l1): (f64, f64)| {
        if f0 >= NEAR { (f0, l0) } else { (NEAR, l0 + (NEAR - f0) / (f1 - f0) * (l1 - l0)) }
    };
    let (a, b) = (clip((fa, la), (fb, lb)), clip((fb, lb), (fa, la)));
    // Perspective: screen x from (forward, left).
    let sx = |(f, l): (f64, f64)| CX - l / f * FOCAL;
    let visible = fa >= NEAR || fb >= NEAR;
    let x0 = if visible { (sx(a) - 0.5).ceil().clamp(0.0, W as f64) as usize } else { 0 };
    let x1 = if visible { (sx(b) - 0.5).ceil().clamp(0.0, W as f64) as usize } else { 0 };
    // x1 < x0 means the wall faces away (back-face cull): empty range.
    (x0, x1.max(x0))
}

// Position along the wall and depth where screen column x hits it.
fn column(pose: Pose, (x1, y1, x2, y2): Geom, x: usize) -> (f64, f64) {
    let (fa, la) = view(pose, x1, y1);
    let (fb, lb) = view(pose, x2, y2);
    // The column's view ray is left = k * forward; t solves for where the wall meets it.
    let k = (CX - (x as f64 + 0.5)) / FOCAL;
    let t = ((k * fa - la) / ((lb - la) - k * (fb - fa))).clamp(0.0, 1.0);
    (t, (fa + t * (fb - fa)).max(NEAR))
}

// Build the panel for one wall in one column.
fn panel(pose: Pose, x: usize, t: f64, depth: f64, w: WallRow) -> Panel {
    let ((x1, y1, x2, y2), (xoff, _, _, _), (floor, ceil, _, _, _), back, _) = w;
    let scale = FOCAL / depth;
    // World height -> screen row at this depth.
    let y = |h: f64| row(CY - (h - pose.2) * scale);
    let len = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
    let (yc, yf) = (y(ceil), y(floor));
    let (ybc, ybf) = back.map_or((yc, yf), |(bf, bc, _)| (y(bc), y(bf)));
    (x, depth, scale, xoff + t * len, (yc, yf, ybc, ybf), w)
}

// Rows this panel hides from walls behind it (solid walls hide the whole column).
fn closes((_, _, _, _, (yc, yf, ybc, ybf), (_, _, _, back, _)): Panel) -> (i32, i32) {
    match back {
        None => (VIEW_H as i32, 0),
        Some(_) => (yc.max(ybc), yf.min(ybf)),
    }
}

// The rows of a panel's still-visible stretch (top..bot) that belong to one part, if any.
fn span(pose: Pose, sky_flat: Id<Image>, (p, (top, bot)): (Panel, (i32, i32)), part: Part) -> Option<Span> {
    let (_, _, _, _, (yc, yf, ybc, ybf), (_, _, (floor, ceil, _, ceilpic, _), back, _)) = p;
    let sp = |y0: i32, y1: i32| (y0 < y1).then_some((p, part, y0, y1));
    // Ceiling above the wall, floor below it; middle only on one-sided walls; upper/lower where the back sector steps.
    match (part, back) {
        (Part::Ceiling, _) if ceil > pose.2 => sp(top, bot.min(yc)),
        (Part::Floor, _) if floor < pose.2 => sp(top.max(yf), bot),
        (Part::Middle, None) => sp(top.max(yc), bot.min(yf)),
        (Part::Upper, Some((_, bc, bpic))) if bc < ceil && !(ceilpic == sky_flat && bpic == sky_flat) => sp(top.max(yc), bot.min(ybc)),
        (Part::Lower, Some((bf, _, _))) if bf > floor => sp(top.max(ybf), bot.min(yf)),
        _ => None,
    }
}

// Sector light level -> colormap index (higher = darker).
fn light(level: i64, delta: i64) -> i64 {
    (15 - ((level >> 4) + delta).clamp(0, 15)) * 4
}

// The fragment for screen row y of a span: wall texel, floor/ceiling texel, or sky (None for an untextured wall).
fn frag(pose: Pose, sky: Id<Image>, sky_flat: Id<Image>, (p, part, _, _): Span, y: i32) -> Option<Frag> {
    let (x, depth, scale, u, _, (_, (_, yoff, flags, delta), (floor, ceil, floorpic, ceilpic, lev), back, (upper, middle, lower))) = p;
    let (px, py, pz, ang) = pose;
    let pix = y as usize * W + x;
    let ry = y as f64 + 0.5 - CY;
    let plane = |h: f64, pic: Id<Image>| {
        // Sky: texture column from the view angle, row = screen row, drawn behind everything.
        if pic == sky_flat {
            let a = ang + ((CX - (x as f64 + 0.5)) / FOCAL).atan();
            let u = (a / TAU * 1024.0).floor() as i64;
            return (pix, sky, u, y as i64, 0, f32::MAX);
        }
        // Distance to the plane along this row, then the world point under the pixel.
        let dist = (pz - h) * FOCAL / ry;
        let k = (CX - (x as f64 + 0.5)) / FOCAL;
        let (sn, cs) = ang.sin_cos();
        let wx = px + dist * (cs - k * sn);
        let wy = py + dist * (sn + k * cs);
        let cm = (light(lev, 0) - 80 / ((dist / 16.0) as i64 + 1)).clamp(0, 31);
        (pix, pic, wx.floor() as i64, (-wy).floor() as i64, cm, dist as f32)
    };
    // vref = world height of texture row 0 (set by the pegging flags).
    let wall = |t: TexRef, vref: f64| {
        t.map(|(gfx, _)| {
            let v = vref + yoff - pz + ry / scale;
            let cm = (light(lev, delta) - (scale * 8.0).min(23.0) as i64).clamp(0, 31);
            (pix, gfx, u.floor() as i64, v.floor() as i64, cm, depth as f32)
        })
    };
    let th = |t: TexRef| t.map_or(0.0, |(_, h)| h);
    let (bfloor, bceil) = back.map_or((floor, ceil), |(bf, bc, _)| (bf, bc));
    match part {
        Part::Ceiling => Some(plane(ceil, ceilpic)),
        Part::Floor => Some(plane(floor, floorpic)),
        Part::Middle => wall(middle, if flags & DONTPEGBOTTOM != 0 { floor + th(middle) } else { ceil }),
        Part::Upper => wall(upper, if flags & DONTPEGTOP != 0 { ceil } else { bceil + th(upper) }),
        Part::Lower => wall(lower, if flags & DONTPEGBOTTOM != 0 { ceil } else { bfloor }),
    }
}

// Camera at the player's eye height.
pub fn pose(db: &Db, p: &Player) -> Pose {
    let (vh, dvh) = (one(&db.t.k.view_height), one(&db.t.k.dead_view_height));
    (&*p.x)
        .and(&*p.y)
        .and(&*p.z)
        .and(&*p.ang)
        .and(&*p.dead)
        .fold_flat((0.0, 0.0, 0.0, 0.0), move |_, ((((x, y), z), ang), dead): ((((f64, f64), f64), f64), bool)| (x, y, z + if dead { dvh } else { vh }, ang))
}

// Which of 8 sprite rotations of a thing at (x, y) facing ang faces the camera.
fn rotation((px, py, _, _): Pose, (x, y, _, ang): (f64, f64, f64, f64)) -> usize {
    let a = (y - py).atan2(x - px);
    (((a - ang + PI * 9.0 / 8.0).rem_euclid(TAU) / (PI / 4.0)) as usize).min(7) + 1
}

// Project a thing sprite.
fn billboard(pose: Pose, (x, y, z, _): (f64, f64, f64, f64), bright: bool, (gfx, flip): (Id<Image>, bool), t: Img, lev: i64) -> Billboard {
    let (w, h, left, top) = t;
    let (f, l) = view(pose, x, y);
    let scale = FOCAL / f.max(NEAR);
    // Top-left of the image on screen; left/top are the sprite's hot-spot offsets.
    let x0 = CX - l * scale - left as f64 * scale;
    let y0 = CY - (z + top as f64 - pose.2) * scale;
    let cm = if bright { 0 } else { (light(lev, 0) - (scale * 8.0).min(23.0) as i64).clamp(0, 31) };
    let visible = f >= 4.0;
    let span = |a: f64, n: usize, lim: usize| {
        if visible { ((a - 0.5).ceil().clamp(0.0, lim as f64) as i32, (a + n as f64 * scale - 0.5).ceil().clamp(0.0, lim as f64) as i32) } else { (0, 0) }
    };
    let (xa, xb) = span(x0, w, W);
    let (ya, yb) = span(y0, h, VIEW_H);
    (gfx, flip, t, x0, y0, scale, f as f32, cm, (xa, xb, ya, yb))
}

// The sprite texel at screen pixel (x, y).
fn sprite_frag((gfx, flip, (w, h, _, _), x0, y0, scale, depth, cm, _): Billboard, x: i32, y: i32) -> Frag {
    let u = (((x as f64 + 0.5 - x0) / scale) as i64).clamp(0, w as i64 - 1);
    let v = (((y as f64 + 0.5 - y0) / scale) as i64).clamp(0, h as i64 - 1);
    let u = if flip { w as i64 - 1 - u } else { u };
    (y as usize * W + x as usize, gfx, u, v, cm, depth)
}

// Place an image at (x0, y0) minus its offsets.
fn stamp(x0: f64, y0: f64, gfx: Id<Image>, t: Img, cm: i64, depth: f32) -> Stamp {
    (gfx, t, (x0 - t.2 as f64) as i64, (y0 - t.3 as f64) as i64, cm, depth)
}

// Image columns / rows of a stamp that land on screen.
fn stamp_cols((_, (w, _, _, _), x0, _, _, _): Stamp) -> (i64, i64) {
    ((-x0).max(0), (w as i64).min(W as i64 - x0))
}

fn stamp_rows((_, (_, h, _, _), _, y0, _, _): Stamp) -> (i64, i64) {
    ((-y0).max(0), (h as i64).min(H as i64 - y0))
}

// The fragment for image texel (u, v) of a stamp.
fn stamp_frag((gfx, _, x0, y0, cm, depth): Stamp, u: i64, v: i64) -> Frag {
    ((y0 + v) as usize * W + (x0 + u) as usize, gfx, u, v, cm, depth)
}

// Facts the face rules match on: dead, hurt, grinning at a new weapon.
fn face_facts(dead: bool, damagecount: i32, bonuscount: i32, wanted: Id<Weapon>) -> u32 {
    bit(dead, ffact::DEAD) | bit(damagecount > 0, ffact::HURT) | bit(bonuscount > 0 && wanted.0 != NONE, ffact::GRIN)
}

// Face image key: kind, pain level from health, and a look direction that changes every 17 tics.
fn face_key(health: i32, ((kind, by_pain), by_look): ((u8, bool), bool), tic: u64) -> usize {
    let pain = ((100 - health.clamp(0, 100)) * 5 / 101) as usize;
    let look = (rnd(tic / 17, PLAYER, 99) % 3) as usize;
    db::face_key(kind, pain * by_pain as usize, look * by_look as usize)
}

// Palette to use: red when hurt, gold after a pickup.
fn tint((damagecount, bonuscount): (i32, i32)) -> usize {
    if damagecount > 0 {
        ((damagecount + 7) >> 3).min(8) as usize
    } else if bonuscount > 0 {
        8 + ((bonuscount + 7) >> 3).min(4) as usize
    } else {
        0
    }
}

// Each image's size and offsets.
fn img(db: &Db) -> impl Probe<D = Id<Image>, R = Img> + '_ {
    let i = &db.image;
    (&i.w).and(&i.h).and(&i.left).and(&i.top).map(|(((w, h), l), t): (((usize, usize), i64), i64)| (w, h, l, t))
}

// seg -> Some((texture, height)) for one of its texture slots, None for "-".
fn tex<'a>(db: &'a Db, c: &'a Col<Seg, Id<Image>>) -> impl Probe<D = Id<Seg>, R = TexRef> + 'a {
    c.select((&db.image.h).and(Same::<Id<Image>>::new()).map(|(h, i): (usize, Id<Image>)| (i, h as f64))).opt()
}

// Draw one frame into `out` as a single query.
pub fn render(db: &Db, s: &State, out: &mut [u32]) {
    let pose = pose(db, &s.player);
    let (sky, sky_flat) = (db.sky, db.sky_flat);
    let (g, sec, hs) = (&db.seg, &db.sector, &s.sector);
    // Every seg with its line data, live sectors and textures.
    let geom = (&g.x1).and(&g.y1).and(&g.x2).and(&g.y2).map(|(((x1, y1), x2), y2): (((f64, f64), f64), f64)| (x1, y1, x2, y2));
    let line = (&g.xoff).and(&g.yoff).and(&g.flags).and(&g.light_delta).map(|(((xo, yo), f), d): (((f64, f64), i64), i64)| (xo, yo, f, d));
    let front = (&g.fsec)
        .select((&*hs.floor).and(&*hs.ceil).and(&sec.floorpic).and(&sec.ceilpic).and(&sec.light))
        .map(|((((f, c), fp), cp), l): ((((f64, f64), Id<Image>), Id<Image>), i64)| (f, c, fp, cp, l));
    let back = (&g.bsec).select((&*hs.floor).and(&*hs.ceil).and(&sec.ceilpic)).map(|((f, c), cp): ((f64, f64), Id<Image>)| (f, c, cp)).opt();
    let tex = |c| tex(db, c);
    let rows = geom
        .and(line)
        .and(front)
        .and(back)
        .and(tex(&g.upper).and(tex(&g.middle)).and(tex(&g.lower)))
        .map(|((((gm, l), f), b), ((u, m), lo)): ((((Geom, (f64, f64, i64, i64)), Front), Option<Back>), ((TexRef, TexRef), TexRef))| (gm, l, f, b, (u, m, lo)));
    // Project walls to columns; per column, nearest first, track rows already covered by closer walls.
    let panels = rows
        // One row per screen column the wall covers.
        .expand(move |w: WallRow| screen_span(pose, w.0))
        .map(move |(w, x): (WallRow, usize)| {
            let (t, d) = column(pose, w.0, x);
            panel(pose, x, t, d, w)
        })
        // Per column, sorted by depth: each panel gets the (top, bottom) rows still open after the panels in front.
        .key_by(|p: Panel| p.0)
        .window_rows(
            preceding((0, VIEW_H as i32), |(t, b): (i32, i32), (_, p): (f64, Panel)| {
                let (ct, cb) = closes(p);
                (t.max(ct), b.min(cb))
            }),
            |p: Panel| p.1,
            |a: &f64, b: &f64| a.total_cmp(b),
        );
    // Wall, floor, ceiling and sky fragments.
    // Every column splits into these pieces, top to bottom.
    let parts: VecRel<usize, Part> = VecRel::new(vec![Part::Ceiling, Part::Upper, Part::Middle, Part::Lower, Part::Floor]);
    let world = (&panels)
        // Panel x part -> the span of rows that part owns, if any.
        .cross(&parts)
        .flat_map(move |(pc, part): ((Panel, (i32, i32)), Part)| span(pose, sky_flat, pc, part))
        // One row per pixel in the span, then its fragment.
        .expand(|s: Span| (s.2, s.3))
        .flat_map(move |(s, y): (Span, i32)| frag(pose, sky, sky_flat, s, y))
        .key_by(|f: Frag| f.0);
    let (m, p, st) = (&s.mobj, &s.player, &db.t.state);
    // A state's sprite and frame, and whether it's drawn full bright.
    let frame = || (&st.sprite).and(&st.frame).and(&st.bright);
    // Thing sprites: state -> sprite frame (rotated, else rotation 0) -> image, lit by its sector.
    let sprites = (&*m.state)
        .select(frame())
        .and((&*m.x).and(&*m.y).and(&*m.z).and(&*m.ang).map(|(((x, y), z), a): (((f64, f64), f64), f64)| (x, y, z, a)))
        .and((&*m.sector).select(&sec.light))
        // Try the rotation facing the camera, and rotation 0 (all-angle sprites); prefer the first.
        .key_by(move |((((sp, fr), _), at), _): ((((usize, usize), bool), (f64, f64, f64, f64)), i64)| frame_key(sp, fr, rotation(pose, at)))
        .and((&db.t.sprite_frames).opt())
        .key_by(|(((((sp, fr), _), _), _), _): (((((usize, usize), bool), (f64, f64, f64, f64)), i64), Option<(Id<Image>, bool)>)| frame_key(sp, fr, 0))
        .and((&db.t.sprite_frames).opt())
        .flat_map(|((r, a), b): ((((((usize, usize), bool), (f64, f64, f64, f64)), i64), Option<(Id<Image>, bool)>), Option<(Id<Image>, bool)>)| a.or(b).map(|g| (r, g)))
        .key_by(|(_, (g, _))| g)
        .and(img(db))
        .map(move |((((((_, _), bright), at), lev), g), t): ((((((usize, usize), bool), (f64, f64, f64, f64)), i64), (Id<Image>, bool)), Img)| billboard(pose, at, bright, g, t, lev))
        // Every screen pixel in the sprite's rectangle.
        .expand(|b: Billboard| (b.8.0, b.8.1))
        .expand(|(b, _): (Billboard, i32)| (b.8.2, b.8.3))
        .map(|((b, x), y): ((Billboard, i32), i32)| sprite_frag(b, x, y))
        .key_by(|f: Frag| f.0);
    // Weapon and muzzle flash overlay; depth 0.5 / 0.25 puts them in front of the world, flash over weapon.
    let weapon = (&*p.wstate)
        .filt(|w: Id<AnimState>| w.0 != 0)
        .map(|w| (w, 0.5f32))
        .union((&*p.flash).filt(|f: Id<AnimState>| f.0 != 0).map(|f| (f, 0.25f32)))
        .and((&*p.sector).select(&sec.light))
        .key_by(|((w, _), _): ((Id<AnimState>, f32), i64)| w)
        .and(frame())
        .key_by(|(_, ((sp, fr), _)): (((Id<AnimState>, f32), i64), ((usize, usize), bool))| frame_key(sp, fr, 0))
        .and(&db.t.sprite_frames)
        .key_by(|(_, (g, _)): ((((Id<AnimState>, f32), i64), ((usize, usize), bool)), (Id<Image>, bool))| g)
        .and(img(db))
        .map(|(((((_, d), lev), (_, bright)), (g, _)), t): (((((Id<AnimState>, f32), i64), ((usize, usize), bool)), (Id<Image>, bool)), Img)| {
            let cm = if bright { 0 } else { (light(lev, 0) - 23).clamp(0, 31) };
            stamp(1.0, 31.5 - (100.0 - CY), g, t, cm, d)
        })
        .expand(stamp_cols)
        .expand(|(s, _): (Stamp, i64)| stamp_rows(s))
        .map(|((s, u), v): ((Stamp, i64), i64)| stamp_frag(s, u, v))
        .key_by(|f: Frag| f.0);
    // Status bar: values for the number widgets.
    let (am, hn) = (&db.t.ammo, &db.t.hud_number);
    let slot = |i: usize| Id::<NumberSlot>::new(i);
    // stat slot -> value: current weapon's ammo, health, armor, each ammo count, each ammo cap.
    let stats = (&*p.weapon)
        .select(&db.t.weapon.ammo)
        .select((&*s.ammo).and(&am.cap))
        .filt(|(_, cap): (i32, i32)| cap > 0)
        .key_by(move |_| slot(STAT_AMMO))
        .map(|(a, _)| a)
        .union((&*p.health).key_by(move |_| slot(STAT_HEALTH)))
        .union((&*p.armor).key_by(move |_| slot(STAT_ARMOR)))
        .union(
            (&*s.ammo)
                .and(&am.cap)
                .and(Same::<Id<AmmoKind>>::new())
                .filt(|((_, cap), _): ((i32, i32), Id<AmmoKind>)| cap > 0)
                .key_by(move |(_, kind)| slot(STAT_AMMO0 + kind.0))
                .map(|((a, _), _)| a),
        )
        .union(
            (&am.cap)
                .and(Same::<Id<AmmoKind>>::new())
                .filt(|(cap, _): (i32, Id<AmmoKind>)| cap > 0)
                .key_by(move |(_, kind)| slot(STAT_MAX0 + kind.0))
                .map(|(cap, _)| cap),
        );
    // A number slot's position, font, digit count and percent sign.
    let slots = || (&hn.x).and(&hn.y).and(&hn.font).and(&hn.digits).and(&hn.percent).map(|((((x, y), f), d), pc): ((((f64, f64), usize), usize), bool)| (x, y, f, d, pc));
    type Slot = (f64, f64, usize, usize, bool);
    // One glyph per digit place, right-aligned using the width of the font's 0, no leading zeros.
    let digits = (&stats)
        .and(slots())
        .key_by(|(_, n): (i32, Slot)| glyph_key(n.2, 0))
        .and(&db.t.hud_fonts)
        .key_by(|(_, g)| g)
        .and(&db.image.w)
        .map(|(((v, n), _), w): (((i32, Slot), Id<Image>), usize)| (v.max(0), n, w as f64))
        // place 0 = ones, 1 = tens, ...; glyph = (font, digit), x steps left one glyph width per place.
        .expand(|(_, n, _): (i32, Slot, f64)| (0, n.3 as u32))
        .filt(|((v, _, _), place): ((i32, Slot, f64), u32)| place == 0 || v >= 10i32.pow(place))
        .map(|((v, n, w), place): ((i32, Slot, f64), u32)| ((n.2, (v / 10i32.pow(place) % 10) as usize), n.0 - w * (place as f64 + 1.0), n.1));
    let percents = (&stats)
        .and(slots())
        .filt(|(_, n): (i32, Slot)| n.4)
        .map(|(_, n): (i32, Slot)| ((n.2, 10), n.0, n.1));
    // Arms panel: owned weapons in yellow, the rest in grey.
    let ha = &db.t.hud_arm;
    let arms = (&ha.weapon)
        .and(&ha.x)
        .and(&ha.y)
        .and(&ha.digit)
        .cross(&*p.owned)
        .map(|((((w, x), y), d), owned): ((((usize, f64), f64), usize), u8)| ((if owned & (1 << w) != 0 { FONT_YELLOW } else { FONT_GREY }, d), x, y));
    // All (font, glyph) requests -> image ids, as (image, x, y, depth).
    let glyphs = digits
        .key_by(|_| 0usize)
        .union(percents.key_by(|_| 0usize))
        .union(arms.key_by(|_| 0usize))
        .key_by(|((font, glyph), _, _): ((usize, usize), f64, f64)| glyph_key(font, glyph))
        .and(&db.t.hud_fonts)
        .map(|((_, x, y), g): (((usize, usize), f64, f64), Id<Image>)| (g, x, y, 0.0f32));
    // Keys held, and the face picked by the face rules.
    let hk = &db.t.hud_key;
    let keys = (&hk.x)
        .and(&hk.y)
        .and(&hk.image)
        .and(Same::<Id<HudKey>>::new())
        .cross(&*p.keys)
        .filt(|((_, k), keys): ((((f64, f64), Id<Image>), Id<HudKey>), u8)| keys & (1 << k.0) != 0)
        .map(|((((x, y), g), _), _): ((((f64, f64), Id<Image>), Id<HudKey>), u8)| (g, x, y, 0.0f32));
    let tic = s.tic;
    let fr = &db.t.face_rule;
    let face_rows: MultiRel<Id<Player>, (Id<Player>, usize, u32)> = (&*p.dead)
        .and(&*p.damagecount)
        .and(&*p.bonuscount)
        .and(&*p.wanted)
        .and(Same::<Id<Player>>::new())
        .map(|((((d, dc), bc), w), id): ((((bool, i32), i32), Id<Weapon>), Id<Player>)| (id, 0usize, face_facts(d, dc, bc, w)))
        .collect();
    let face = matching(&face_rows, fr, &db.t.face_by_key, |id: Id<Player>| id, 1)
        .map(|(_, r): (Id<Player>, Id<FaceRule>)| r)
        .select((&fr.kind).and(&fr.by_pain).and(&fr.by_look))
        .and(&*p.health)
        .map(move |(o, h): (((u8, bool), bool), i32)| face_key(h, o, tic))
        .select(&db.t.faces)
        .map(|g| (g, 143.0, 168.0, 0.0f32));
    // Every status bar image, stamped at depth 0 (always on top).
    let hp = &db.t.hud_patch;
    let status = (&hp.image)
        .and(&hp.x)
        .and(&hp.y)
        .and(&hp.depth)
        .map(|(((g, x), y), d): (((Id<Image>, f64), f64), f32)| (g, x, y, d))
        .key_by(|_| 0usize)
        .union(glyphs.key_by(|_| 0usize))
        .union(keys.key_by(|_| 0usize))
        .union(face.key_by(|_| 0usize))
        .key_by(|(g, _, _, _)| g)
        .and(img(db))
        .map(|((g, x, y, d), t): ((Id<Image>, f64, f64, f32), Img)| stamp(x, y, g, t, 0, d))
        .expand(stamp_cols)
        .expand(|(s, _): (Stamp, i64)| stamp_rows(s))
        .map(|((s, u), v): ((Stamp, i64), i64)| stamp_frag(s, u, v))
        .key_by(|f: Frag| f.0);
    // Composite: texel -> palette index, drop transparent, apply light, keep nearest per pixel, apply tint, -> RGB.
    let image = world
        .union(sprites)
        .union(weapon)
        .union(status)
        // Texel index in the column-major texel array; u and v wrap (tiling).
        .key_by(|f: Frag| f.1)
        .and((&db.image.base).and(&db.image.w).and(&db.image.h))
        .map(|(f, ((base, w), h)): (Frag, ((usize, usize), usize))| (f, base + f.2.rem_euclid(w as i64) as usize * h + f.3.rem_euclid(h as i64) as usize))
        .key_by(|(_, texel)| texel)
        .and(&db.texels)
        .filt(|(_, pal)| pal != TRANSPARENT)
        // Darken through the colormap for this light level.
        .key_by(|((f, _), pal): ((Frag, usize), u16)| f.4 as usize * 256 + pal as usize)
        .and(&db.colormap)
        // Pack depth above colour so `min` per pixel picks the nearest and carries its colour.
        .key_by(|(((f, _), _), _)| f.0)
        .map(|(((f, _), _), c): (((Frag, usize), u16), u8)| ((f.5.to_bits() as u64) << 8) | c as u64)
        .dense_fold(W * H, u64::MAX, |a: u64, k: u64| a.min(k))
        // Pick the palette (normal, pain red, pickup gold) and look up RGB.
        .cross((&*p.damagecount).and(&*p.bonuscount).map(tint))
        .map(|(k, pal): (u64, usize)| pal * 256 + (k & 0xFF) as usize)
        .select(&db.palette);
    out.fill(0);
    // Key is (pixel, player); write each pixel.
    image.drive(|(pix, _), rgb| out[pix] = rgb);
}
