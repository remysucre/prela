use crate::db::{Db, Line};
use crate::render::point_side;
use crate::wad::{Node, Sector, NONE};
use prela::engine::*;
use std::hash::Hash;

// Linedef flag bits and the max step-up height.
pub const ML_BLOCKING: i64 = 1;
pub const ML_BLOCKMONSTERS: i64 = 2;
pub const STEP: f64 = 24.0;

// A linedef joined with its front sector and optional back sector.
pub type LineRow = ((Line, Sector), Option<Sector>);

// Deterministic pseudo-random byte (0..255) from tic, object id and a salt.
pub fn rnd(tic: u64, id: usize, salt: u64) -> i32 {
    let mut z = tic.wrapping_mul(0x9E3779B97F4A7C15) ^ (id as u64).wrapping_mul(0xBF58476D1CE4E5B9) ^ salt.wrapping_mul(0x94D049BB133111EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    ((z ^ (z >> 31)) & 0xFF) as i32
}

// Which sector each point is in: walk the BSP tree from the root with `reach`.
pub fn locate<'a, K, Q>(db: &'a Db, pts: &'a Q) -> impl Drive<D = K, R = usize> + 'a
where
    K: Copy + Eq + Hash + 'a,
    Q: Drive<D = K, R = (f64, f64)> + Probe,
{
    let root = db.bsp_root;
    let keyed = || pts.and(Same::<K>::new());
    // One BSP step: from a node, go to the child on the point's side.
    let step = keyed()
        .cross((&db.node_children).and(&db.nodes))
        .filt(|((p, _), ((side, _), n)): (((f64, f64), K), ((usize, usize), Node))| side == point_side(n, p.0, p.1))
        .map(|((_, k), ((_, child), _)): (((f64, f64), K), ((usize, usize), Node))| (k, child));
    // Start every point at the root, walk down, keep the leaf, map leaf to sector.
    keyed()
        .map(move |(_, k)| (k, root))
        .reach(step, 64)
        .and(Same::<(K, usize)>::new())
        .key_by(|(_, (_, b))| b)
        .and(&db.leaf_sector)
        .key_by(|((_, (k, _)), _)| k)
        .map(|(_, sec)| sec)
}

// Join each line with its current front/back sectors (heights change with doors).
pub fn line_rows(db: &Db, sectors: &VecRel<usize, Sector>) -> VecRel<usize, LineRow> {
    (&db.lines)
        .and((&db.lines).map(|l: Line| l.fsec).select(sectors))
        .and((&db.lines).map(|l: Line| l.bsec).select(sectors).opt())
        .collect()
}

// Signed side of a point relative to a line (sign tells which side).
pub fn side_of(l: Line, x: f64, y: f64) -> f64 {
    (l.x2 - l.x1) * (y - l.y1) - (l.y2 - l.y1) * (x - l.x1)
}

// Does a box of half-size r at (x, y) straddle the line?
pub fn touches(l: Line, x: f64, y: f64, r: f64) -> bool {
    let overlap = l.x1.min(l.x2) < x + r && l.x1.max(l.x2) > x - r && l.y1.min(l.y2) < y + r && l.y1.max(l.y2) > y - r;
    let s = [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)].map(|(dx, dy)| side_of(l, x + dx * r, y + dy * r) > 0.0);
    overlap && s.iter().any(|&b| b != s[0])
}

// Where segment a->b crosses the line, as a fraction t along a->b.
pub fn crossing(l: Line, (ax, ay): (f64, f64), (bx, by): (f64, f64)) -> Option<f64> {
    let (dx, dy) = (bx - ax, by - ay);
    let (ex, ey) = (l.x2 - l.x1, l.y2 - l.y1);
    let den = dx * ey - dy * ex;
    if den.abs() < 1e-9 {
        return None;
    }
    let t = ((l.x1 - ax) * ey - (l.y1 - ay) * ex) / den;
    let u = ((l.x1 - ax) * dy - (l.y1 - ay) * dx) / den;
    (t > 0.0 && t <= 1.0 && (0.0..=1.0).contains(&u)).then_some(t)
}

// Vertical gap (floor, ceiling) through a two-sided line; one-sided lines are closed.
pub fn opening(((_, f), b): LineRow) -> (f64, f64) {
    match b {
        None => (f64::MAX, f64::MIN),
        Some(b) => (f.floor.max(b.floor), f.ceil.min(b.ceil)),
    }
}

#[derive(Clone, Copy, Debug)]
// Something solid or shootable: the player or a mobj.
pub struct Body {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub radius: f64,
    pub height: f64,
    pub solid: bool,
    pub shootable: bool,
}

#[derive(Clone, Copy, Debug)]
// A candidate position to try moving to (several per mover, tried in idx order).
pub struct Cand {
    pub who: usize,
    pub idx: usize,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub radius: f64,
    pub height: f64,
    pub monster: bool,
    pub missile: bool,
    pub stay: bool,
}

#[derive(Clone, Copy, Debug)]
// Result of testing a candidate: tightest floor/ceiling and whether it's blocked.
pub struct Clip {
    pub c: Cand,
    pub floor: f64,
    pub ceil: f64,
    pub blocked: bool,
}

// Combine two clip results: highest floor, lowest ceiling, blocked if either is.
fn merge(a: Clip, b: Clip) -> Clip {
    Clip { c: b.c, floor: a.floor.max(b.floor), ceil: a.ceil.min(b.ceil), blocked: a.blocked || b.blocked }
}

// Can the mover stand at this candidate (height, step-up, drop-off rules)?
fn fits(k: Clip) -> bool {
    let c = k.c;
    c.stay
        || (!k.blocked
            && if c.missile {
                k.floor <= c.z && k.ceil >= c.z + c.height
            } else {
                k.ceil - k.floor >= c.height
                    && k.ceil - c.z >= c.height
                    && k.floor - c.z <= STEP
                    && !(c.monster && c.z - k.floor > STEP)
            })
}

// Clip a candidate against one line it touches.
fn line_clip(c: Cand, ((l, f), b): LineRow) -> Clip {
    match b {
        None => Clip { c, floor: f64::MIN, ceil: f64::MAX, blocked: true },
        Some(b) => Clip {
            c,
            floor: f.floor.max(b.floor),
            ceil: f.ceil.min(b.ceil),
            blocked: !c.missile && (l.flags & ML_BLOCKING != 0 || (c.monster && l.flags & ML_BLOCKMONSTERS != 0)),
        },
    }
}

// Does a candidate overlap another solid body?
fn overlaps(c: Cand, b: Body) -> bool {
    b.solid && b.id != c.who && !c.stay && !c.missile && (c.x - b.x).abs() < c.radius + b.radius && (c.y - b.y).abs() < c.radius + b.radius
}

// For each mover, the first candidate move that fits.
pub fn clip_moves(
    db: &Db,
    sectors: &VecRel<usize, Sector>,
    lines: &VecRel<usize, LineRow>,
    bodies: &HashIdx<usize, Body>,
    cands: &HashIdx<(usize, usize), Cand>,
) -> HashIdx<usize, Clip> {
    let pts: HashIdx<(usize, usize), (f64, f64)> = cands.map(|c: Cand| (c.x, c.y)).collect();
    // Start from the floor/ceiling of the sector the candidate lands in.
    let base = locate(db, &pts)
        .select(sectors)
        .and(cands)
        .map(|(s, c): (Sector, Cand)| Clip { c, floor: s.floor, ceil: s.ceil, blocked: false });
    // Lines the candidate touches.
    let walls = cands
        .cross(lines)
        .filt(|(c, ((l, _), _)): (Cand, LineRow)| touches(l, c.x, c.y, c.radius))
        .map(|(c, row): (Cand, LineRow)| line_clip(c, row))
        .key_by(|k: Clip| (k.c.who, k.c.idx));
    // Bodies the candidate bumps into.
    let things = cands
        .cross(bodies)
        .filt(|(c, b): (Cand, Body)| overlaps(c, b))
        .map(|(c, _): (Cand, Body)| Clip { c, floor: f64::MIN, ceil: f64::MAX, blocked: true })
        .key_by(|k: Clip| (k.c.who, k.c.idx));
    let init = Clip {
        c: Cand { who: NONE, idx: NONE, x: 0.0, y: 0.0, z: 0.0, radius: 0.0, height: 0.0, monster: false, missile: false, stay: false },
        floor: f64::MIN,
        ceil: f64::MAX,
        blocked: false,
    };
    // Merge all constraints per candidate, keep those that fit, pick lowest idx per mover.
    base.union(walls)
        .union(things)
        .fold(init, merge)
        .filt(fits)
        .key_by(|k: Clip| k.c.who)
        .fold(None, |a: Option<Clip>, k: Clip| match a {
            Some(a) if a.c.idx <= k.c.idx => Some(a),
            _ => Some(k),
        })
        .flat_map(|k: Option<Clip>| k)
        .collect()
}

// A sight line: eye position to target position.
pub type Sight = ((f64, f64, f64), (f64, f64, f64));

// Does this line block the sight line (crossing point outside its opening)?
fn blocks_sight(((ax, ay, az), (bx, by, bz)): Sight, row: LineRow) -> bool {
    crossing(row.0.0, (ax, ay), (bx, by)).is_some_and(|t| {
        let z = az + t * (bz - az);
        let (lo, hi) = opening(row);
        z < lo || z > hi
    })
}

// Sight pairs not blocked by any line.
pub fn visible(lines: &VecRel<usize, LineRow>, pairs: &HashIdx<usize, Sight>) -> HashIdx<usize, Sight> {
    let blocked = pairs
        .and(Same::<usize>::new())
        .cross(lines)
        .filt(|((s, _), row): ((Sight, usize), LineRow)| blocks_sight(s, row))
        .key_by(|((_, id), _): ((Sight, usize), LineRow)| id)
        .fold(true, |_, _| true);
    pairs.minus(&blocked).collect()
}

#[derive(Clone, Copy, Debug)]
// An instant-hit bullet ray.
pub struct Shot {
    pub id: usize,
    pub from: usize,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ang: f64,
    pub range: f64,
    pub damage: i32,
}

#[derive(Clone, Copy, Debug)]
// Where a shot hit: a wall (target NONE) or a body.
pub struct Impact {
    pub shot: Shot,
    pub t: f64,
    pub target: usize,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

// Shot hitting a wall: crosses a line outside its opening.
fn wall_impact(s: Shot, row: LineRow) -> Option<Impact> {
    let end = (s.x + s.range * s.ang.cos(), s.y + s.range * s.ang.sin());
    crossing(row.0.0, (s.x, s.y), end).and_then(|t| {
        let (lo, hi) = opening(row);
        let d = (t * s.range - 4.0).max(0.0);
        (s.z < lo || s.z > hi).then(|| Impact { shot: s, t: d, target: NONE, x: s.x + d * s.ang.cos(), y: s.y + d * s.ang.sin(), z: s.z })
    })
}

// Shot hitting a body: ray passes within its radius.
fn body_impact(s: Shot, b: Body) -> Option<Impact> {
    let (dx, dy) = (b.x - s.x, b.y - s.y);
    let (sn, cs) = s.ang.sin_cos();
    let along = dx * cs + dy * sn;
    let perp = (-dx * sn + dy * cs).abs();
    (b.shootable && b.id != s.from && along > 0.0 && along < s.range && perp < b.radius).then(|| {
        let t = along - (b.radius * b.radius - perp * perp).sqrt();
        Impact { shot: s, t, target: b.id, x: s.x + t * cs, y: s.y + t * sn, z: b.z + b.height / 2.0 }
    })
}

// For each shot, the nearest wall or body it hits.
pub fn hitscan(lines: &VecRel<usize, LineRow>, bodies: &HashIdx<usize, Body>, shots: &HashIdx<usize, Shot>) -> HashIdx<usize, Impact> {
    shots
        .cross(lines)
        .flat_map(|(s, row): (Shot, LineRow)| wall_impact(s, row))
        .key_by(|i: Impact| i.shot.id)
        .union(shots.cross(bodies).flat_map(|(s, b): (Shot, Body)| body_impact(s, b)).key_by(|i: Impact| i.shot.id))
        .fold(None, |a: Option<Impact>, i: Impact| match a {
            Some(a) if a.t <= i.t => Some(a),
            _ => Some(i),
        })
        .flat_map(|i: Option<Impact>| i)
        .collect()
}
