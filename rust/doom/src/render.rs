use crate::db::{frame_key, Db, StateRow, Tex, Wall};
use crate::mobj::Mobj;
use crate::player::{Player, VIEW};
use crate::tic::State;
use crate::wad::{Node, Sector, TRANSPARENT};
use std::f64::consts::{PI, TAU};
use prela::engine::*;

pub const W: usize = 320;
pub const H: usize = 200;
const CX: f64 = 160.0;
const CY: f64 = 100.0;
const FOCAL: f64 = 160.0;
const NEAR: f64 = 1.0;
const DONTPEGTOP: i64 = 8;
const DONTPEGBOTTOM: i64 = 16;

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ang: f64,
}

impl Pose {
    fn view(&self, x: f64, y: f64) -> (f64, f64) {
        let (dx, dy) = (x - self.x, y - self.y);
        let (s, c) = self.ang.sin_cos();
        (dx * c + dy * s, -dx * s + dy * c)
    }
}

pub fn point_side(n: Node, x: f64, y: f64) -> usize {
    if n.dx * (y - n.y) - n.dy * (x - n.x) < 0.0 { 0 } else { 1 }
}

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
    fn closes(&self) -> (i32, i32) {
        match self.back {
            None => (H as i32, 0),
            Some(_) => (self.yc.max(self.ybc), self.yf.min(self.ybf)),
        }
    }
}

#[derive(Clone, Copy)]
enum Part {
    Ceiling,
    Upper,
    Middle,
    Lower,
    Floor,
}

#[derive(Clone, Copy)]
struct Span {
    p: Panel,
    part: Part,
    y0: i32,
    y1: i32,
}

#[derive(Clone, Copy)]
struct Frag {
    pix: usize,
    gfx: usize,
    u: i64,
    v: i64,
    cm: i64,
    depth: f32,
}

fn row(y: f64) -> i32 {
    (y - 0.5).ceil().clamp(0.0, H as f64) as i32
}

fn columns(pose: Pose, w: Wall) -> impl Iterator<Item = (usize, f64, f64)> {
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
    (x0..x1.max(x0)).map(move |x| {
        let k = (CX - (x as f64 + 0.5)) / FOCAL;
        let t = ((k * fa - la) / ((lb - la) - k * (fb - fa))).clamp(0.0, 1.0);
        (x, t, (fa + t * (fb - fa)).max(NEAR))
    })
}

fn panel(pose: Pose, x: usize, t: f64, depth: f64, row_: PanelRow) -> Panel {
    let (wall, front, back, upper, middle, lower) = row_;
    let scale = FOCAL / depth;
    let y = |h: f64| row(CY - (h - pose.z) * scale);
    let len = ((wall.x2 - wall.x1).powi(2) + (wall.y2 - wall.y1).powi(2)).sqrt();
    let (yc, yf) = (y(front.ceil), y(front.floor));
    let (ybc, ybf) = back.map_or((yc, yf), |b| (y(b.ceil), y(b.floor)));
    Panel { x, depth, scale, u: wall.offset + t * len, wall, front, back, upper, middle, lower, yc, yf, ybc, ybf }
}

type PanelRow = (Wall, Sector, Option<Sector>, Option<Tex>, Option<Tex>, Option<Tex>);

fn spans(pose: Pose, sky_flat: usize, (p, (top, bot)): (Panel, (i32, i32))) -> impl Iterator<Item = Span> {
    let mut out = [None; 5];
    let sp = |part, y0: i32, y1: i32| (y0 < y1).then_some(Span { p, part, y0, y1 });
    if p.front.ceil > pose.z {
        out[0] = sp(Part::Ceiling, top, bot.min(p.yc));
    }
    if p.front.floor < pose.z {
        out[4] = sp(Part::Floor, top.max(p.yf), bot);
    }
    match p.back {
        None => out[2] = sp(Part::Middle, top.max(p.yc), bot.min(p.yf)),
        Some(b) => {
            let both_sky = p.front.ceilpic == sky_flat && b.ceilpic == sky_flat;
            if b.ceil < p.front.ceil && !both_sky {
                out[1] = sp(Part::Upper, top.max(p.yc), bot.min(p.ybc));
            }
            if b.floor > p.front.floor {
                out[3] = sp(Part::Lower, top.max(p.ybf), bot.min(p.yf));
            }
        }
    }
    out.into_iter().flatten()
}

fn light(level: i64, delta: i64) -> i64 {
    (15 - ((level >> 4) + delta).clamp(0, 15)) * 4
}

fn frags(pose: Pose, sky: usize, sky_flat: usize, s: Span) -> impl Iterator<Item = Frag> {
    let p = s.p;
    (s.y0..s.y1).filter_map(move |y| {
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
    })
}

pub fn pose(p: Player) -> Pose {
    Pose { x: p.x, y: p.y, z: p.z + if p.dead { 8.0 } else { VIEW }, ang: p.ang }
}

fn rotation(pose: Pose, m: Mobj) -> usize {
    let a = (m.y - pose.y).atan2(m.x - pose.x);
    (((a - m.ang + PI * 9.0 / 8.0).rem_euclid(TAU) / (PI / 4.0)) as usize).min(7) + 1
}

fn sprite_frags(pose: Pose, m: Mobj, row: StateRow, (gfx, flip): (usize, bool), t: Tex, sec: Sector) -> impl Iterator<Item = Frag> {
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
    let (ya, yb) = span(y0, t.h, H);
    (xa..xb).flat_map(move |x| {
        (ya..yb).map(move |y| {
            let u = (((x as f64 + 0.5 - x0) / scale) as i64).clamp(0, t.w as i64 - 1);
            let v = (((y as f64 + 0.5 - y0) / scale) as i64).clamp(0, t.h as i64 - 1);
            let u = if flip { t.w as i64 - 1 - u } else { u };
            Frag { pix: y as usize * W + x as usize, gfx, u, v, cm, depth: f as f32 }
        })
    })
}

fn patch_frags(x0: f64, y0: f64, gfx: usize, t: Tex, cm: i64, depth: f32) -> impl Iterator<Item = Frag> {
    let x0 = (x0 - t.left as f64) as i64;
    let y0 = (y0 - t.top as f64) as i64;
    (0..t.w as i64).flat_map(move |u| {
        (0..t.h as i64)
            .filter(move |v| (0..W as i64).contains(&(x0 + u)) && (0..H as i64).contains(&(y0 + v)))
            .map(move |v| Frag { pix: (y0 + v) as usize * W + (x0 + u) as usize, gfx, u, v, cm, depth })
    })
}

fn digits(slot: usize, value: i32, right: f64, y: f64, percent: bool) -> impl Iterator<Item = (usize, usize, f64, f64)> {
    let v = value.max(0);
    let n = if v >= 100 { 3 } else if v >= 10 { 2 } else { 1 };
    let pct = percent.then_some((slot, 10, right, y));
    (0..n)
        .map(move |k| (slot, ((v / 10i32.pow(k as u32)) % 10) as usize, right - 14.0 * (k as f64 + 1.0), y))
        .chain(pct)
}

fn hud(p: Player) -> impl Iterator<Item = (usize, usize, f64, f64)> {
    let ammo = match p.weapon {
        0 => -1,
        2 => p.ammo[1],
        _ => p.ammo[0],
    };
    let keys = (0..3).filter(move |k| p.keys & (1 << k) != 0).map(|k| (9, 11 + k, 239.0, 171.0 + 10.0 * k as f64));
    (ammo >= 0)
        .then(|| digits(0, ammo, 44.0, 171.0, false))
        .into_iter()
        .flatten()
        .chain(digits(1, p.health, 90.0, 171.0, true))
        .chain(digits(2, p.armor, 221.0, 171.0, true))
        .chain(keys)
}

fn tint(p: Player) -> usize {
    if p.damagecount > 0 {
        ((p.damagecount + 7) >> 3).min(8) as usize
    } else if p.bonuscount > 0 {
        8 + ((p.bonuscount + 7) >> 3).min(4) as usize
    } else {
        0
    }
}

pub fn render(db: &Db, s: &State, out: &mut [u32]) {
    let sectors = &s.sectors;
    let pose = (&s.player).fold_flat(Pose { x: 0.0, y: 0.0, z: 0.0, ang: 0.0 }, |_, p: Player| pose(p));
    let (sky, sky_flat) = (db.sky, db.sky_flat);
    let tex_of = |f: fn(Wall) -> usize| (&db.walls).map(f).select(&db.tex).opt();
    let rows = (&db.walls)
        .and((&db.walls).map(|w: Wall| w.fsec).select(sectors))
        .and((&db.walls).map(|w: Wall| w.bsec).select(sectors).opt())
        .and(tex_of(|w| w.upper).and(tex_of(|w| w.middle)).and(tex_of(|w| w.lower)))
        .map(|(((w, f), b), ((u, m), l))| (w, f, b, u, m, l));
    let panels = rows
        .flat_map(move |r: PanelRow| columns(pose, r.0).map(move |(x, t, d)| panel(pose, x, t, d, r)))
        .key_by(|p: Panel| p.x)
        .window(
            preceding((0, H as i32), |(t, b): (i32, i32), p: Panel| {
                let (ct, cb) = p.closes();
                (t.max(ct), b.min(cb))
            }),
            |p: Panel| p.depth,
            |a: &f64, b: &f64| a.total_cmp(b),
        );
    let world = (&panels)
        .flat_map(move |pc| spans(pose, sky_flat, pc))
        .flat_map(move |s| frags(pose, sky, sky_flat, s))
        .key_by(|f: Frag| f.pix);
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
        .flat_map(move |(((m, r, g), t), sec): (((Mobj, StateRow, (usize, bool)), Tex), Sector)| sprite_frags(pose, m, r, g, t, sec))
        .key_by(|f: Frag| f.pix);
    let weapon = (&s.player)
        .flat_map(|p: Player| [(p.wstate, 0.5f32), (p.flash, 0.25f32)].into_iter().filter(|&(st, _)| st != 0).map(move |(st, d)| (p, st, d)))
        .key_by(|(_, st, _)| st)
        .and(&db.t.states)
        .key_by(|((_, _, _), r): ((Player, usize, f32), StateRow)| frame_key(r.sprite, r.frame, 0))
        .and(&db.t.sprite_frames)
        .key_by(|(_, (g, _)): (((Player, usize, f32), StateRow), (usize, bool))| g)
        .and(&db.tex)
        .key_by(|((((p, _, _), _), _), _): ((((Player, usize, f32), StateRow), (usize, bool)), Tex)| p.sector)
        .and(sectors)
        .flat_map(|(((((_, _, d), r), (g, _)), t), sec): (((((Player, usize, f32), StateRow), (usize, bool)), Tex), Sector)| {
            let cm = if r.bright { 0 } else { (light(sec.light, 0) - 23).clamp(0, 31) };
            patch_frags(1.0, 31.5, g, t, cm, d)
        })
        .key_by(|f: Frag| f.pix);
    let status = (&s.player)
        .flat_map(hud)
        .key_by(|(_, d, _, _)| d)
        .and(&db.t.hud)
        .key_by(|(_, g)| g)
        .and(&db.tex)
        .flat_map(|(((_, _, x, y), g), t): (((usize, usize, f64, f64), usize), Tex)| patch_frags(x, y, g, t, 0, 0.0))
        .key_by(|f: Frag| f.pix);
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
