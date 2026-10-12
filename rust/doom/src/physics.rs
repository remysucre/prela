use crate::db::Db;
use crate::schema::{one, Bsp, Linedef, Sector, SectorState, C};
use crate::wad::NONE;
use prela::engine::*;
use prela::loader::Col;
use std::hash::Hash;

// Linedef flag bits: blocks everything / blocks monsters.
pub const ML_BLOCKING: i64 = 1;
pub const ML_BLOCKMONSTERS: i64 = 2;

// A line segment (x1, y1, x2, y2).
pub type Geom = (f64, f64, f64, f64);

// A vertical gap (lo, hi) through a line.
pub type Gap = (f64, f64);

// Per-tic opening through each line: highest floor and lowest ceiling of its two sectors.
// One-sided lines get (MAX, MIN), which nothing fits through.
pub struct Lines {
    pub lo: Col<Linedef, f64>,
    pub hi: Col<Linedef, f64>,
}

// Deterministic pseudo-random byte (0..255) from tic, object id and a salt.
pub fn rnd(tic: u64, id: usize, salt: u64) -> i32 {
    let mut z = tic.wrapping_mul(0x9E3779B97F4A7C15) ^ (id as u64).wrapping_mul(0xBF58476D1CE4E5B9) ^ salt.wrapping_mul(0x94D049BB133111EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    ((z ^ (z >> 31)) & 0xFF) as i32
}

// Which side (0/1) of a BSP split line (x, y, dx, dy) the point (px, py) is on.
pub fn point_side((((x, y), dx), dy): (((f64, f64), f64), f64), px: f64, py: f64) -> usize {
    if dx * (py - y) - dy * (px - x) < 0.0 { 0 } else { 1 }
}

// Which sector each point is in: walk the BSP tree from the root with `reach`.
pub fn locate<'a, K, Q>(db: &'a Db, pts: &'a Q) -> impl Drive<D = K, R = Row<Sector>> + 'a
where
    K: Copy + Eq + Hash + 'a,
    Q: Drive<D = K, R = (f64, f64)> + Probe,
{
    let (b, root) = (&db.bsp, db.bsp_root);
    // point key -> (point, key)
    let keyed = || pts.and(Same::<K>::new());
    // Node -> (split line, front child, back child).
    let split = (&b.x).and(&b.y).and(&b.dx).and(&b.dy).and(&b.child0).and(&b.child1);
    // One BSP step (key, node) -> (key, child on the point's side); `cross` makes the key (point key, node).
    let step = keyed().cross(split).map(|((p, k), ((line, c0), c1)): (((f64, f64), K), (((((f64, f64), f64), f64), Row<Bsp>), Row<Bsp>))| {
        (k, if point_side(line, p.0, p.1) == 0 { c0 } else { c1 })
    });
    // Start every point at the root, walk down, keep the leaf, map leaf to sector.
    keyed()
        .map(move |(_, k)| (k, root))
        // Every (key, node) visited on the way down, with its depth.
        .reach(step, 64)
        .and(Same::<(K, Row<Bsp>)>::new())
        // Only leaves have a sector, so this join keeps just the last node.
        .key_by(|(_, (_, n))| n)
        .and(&b.sector)
        .key_by(|((_, (k, _)), _)| k)
        .map(|(_, sec)| sec)
}

// Each line's segment.
pub fn geom(db: &Db) -> impl Drive<D = Row<Linedef>, R = Geom> + Probe + '_ {
    let l = &db.linedef;
    (&l.x1).and(&l.y1).and(&l.x2).and(&l.y2).map(|(((x1, y1), x2), y2): (((f64, f64), f64), f64)| (x1, y1, x2, y2))
}

// Each line's segment and current gap.
pub fn view<'a>(db: &'a Db, lines: &'a Lines) -> impl Drive<D = Row<Linedef>, R = (Geom, Gap)> + Probe + 'a {
    geom(db).and((&lines.lo).and(&lines.hi))
}

// This tic's line openings from the live sector heights.
pub fn lines(db: &Db, h: &SectorState) -> Lines {
    let l = &db.linedef;
    let side = |c: &C<Sector, f64>| (&l.fsec).select(c.clone()).and((&l.bsec).select(c.clone()).opt());
    Lines {
        lo: side(&h.floor).map(|(f, b): (f64, Option<f64>)| b.map_or(f64::MAX, |b| f.max(b))).collect(),
        hi: side(&h.ceil).map(|(c, b): (f64, Option<f64>)| b.map_or(f64::MIN, |b| c.min(b))).collect(),
    }
}

// Signed side of a point relative to a line (sign tells which side).
pub fn side_of((x1, y1, x2, y2): Geom, x: f64, y: f64) -> f64 {
    (x2 - x1) * (y - y1) - (y2 - y1) * (x - x1)
}

// Does a box of half-size r at (x, y) straddle the line?
// True when bounding boxes overlap and the box's corners are not all on one side.
pub fn touches(g: Geom, x: f64, y: f64, r: f64) -> bool {
    let (x1, y1, x2, y2) = g;
    let overlap = x1.min(x2) < x + r && x1.max(x2) > x - r && y1.min(y2) < y + r && y1.max(y2) > y - r;
    let s = [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)].map(|(dx, dy)| side_of(g, x + dx * r, y + dy * r) > 0.0);
    overlap && s.iter().any(|&b| b != s[0])
}

// Where segment a->b crosses the line, as a fraction t along a->b.
pub fn crossing((x1, y1, x2, y2): Geom, (ax, ay): (f64, f64), (bx, by): (f64, f64)) -> Option<f64> {
    let (dx, dy) = (bx - ax, by - ay);
    let (ex, ey) = (x2 - x1, y2 - y1);
    let den = dx * ey - dy * ex;
    if den.abs() < 1e-9 {
        return None;
    }
    // t along a->b, u along the line; a hit needs both inside their segments.
    let t = ((x1 - ax) * ey - (y1 - ay) * ex) / den;
    let u = ((x1 - ax) * dy - (y1 - ay) * dx) / den;
    (t > 0.0 && t <= 1.0 && (0.0..=1.0).contains(&u)).then_some(t)
}

// Rows passed between the steps of a tic, as tuples.

// Bodies (the player and the mobjs) share one dense id space per tic: body id 0 is the player,
// body id m + 1 is mobj m. Move candidates are keyed body id * 8 + idx, shots body id * 8 + pellet.

// The body id of a mobj.
pub fn body_of(m: usize) -> usize {
    m + 1
}

// Something solid or shootable: body id, uid (lasting identity), x, y, z, radius, height, solid, shootable.
pub type Body = (usize, usize, f64, f64, f64, f64, f64, bool, bool);
// A candidate position for a mover, tried in idx order: mover's body id, idx, x, y, z, radius,
// height, is a monster, is a missile, stays put.
pub type Cand = (usize, usize, f64, f64, f64, f64, f64, bool, bool, bool);
// A candidate after collision: tightest floor and ceiling, and whether it's blocked.
pub type Clip = (Cand, f64, f64, bool);
// An instant-hit bullet: shot uid (names its puff), shooter body id, x, y, z, angle, range, damage.
pub type Shot = (usize, usize, f64, f64, f64, f64, f64, i32);
// Where a shot hit: distance, target body id (NONE for a wall), x, y, z, damage, shot uid.
pub type Impact = (f64, usize, f64, f64, f64, i32, usize);

// The key of a mover's candidate, or of a shooter's pellet.
pub fn slot_key(body: usize, i: usize) -> usize {
    body * 8 + i
}

// Combine two clip results: highest floor, lowest ceiling, blocked if either is.
fn merge((_, af, ac, ab): Clip, (c, bf, bc, bb): Clip) -> Clip {
    (c, af.max(bf), ac.min(bc), ab || bb)
}

// Can the mover stand at this candidate (height, step-up, drop-off rules)?
fn fits(((_, _, _, _, z, _, height, monster, missile, stay), floor, ceil, blocked): Clip, step: f64) -> bool {
    stay
        || (!blocked
            && if missile {
                // Missiles fly at fixed z: the gap must contain them.
                floor <= z && ceil >= z + height
            } else {
                // Gap tall enough, head room, step up at most `step`, monsters won't step off ledges.
                ceil - floor >= height && ceil - z >= height && floor - z <= step && !(monster && z - floor > step)
            })
}

// Clip a candidate against one line it touches; `two` = the line has a back side.
fn line_clip(c: Cand, (lo, hi): Gap, flags: i64, two: bool) -> Clip {
    let (monster, missile) = (c.7, c.8);
    if !two {
        return (c, f64::MIN, f64::MAX, true);
    }
    (c, lo, hi, !missile && (flags & ML_BLOCKING != 0 || (monster && flags & ML_BLOCKMONSTERS != 0)))
}

// Does a candidate overlap another solid body?
fn overlaps((who, _, x, y, _, radius, _, _, missile, stay): Cand, (id, _, bx, by, _, br, _, solid, _): Body) -> bool {
    solid && id != who && !stay && !missile && (x - bx).abs() < radius + br && (y - by).abs() < radius + br
}

// For each mover, the first candidate move that fits.
pub fn clip_moves(db: &Db, h: &SectorState, lines: &Lines, bodies: &MultiRel<usize, Body>, cands: &MultiRel<usize, Cand>) -> MultiRel<usize, Clip> {
    let step = one(&db.t.k.max_step);
    let n = cands.n_dom();
    // Every source below is keyed by candidate key.
    let pts: MultiRel<usize, (f64, f64)> = cands.map(|c: Cand| (c.2, c.3)).collect();
    // Start from the floor/ceiling of the sector the candidate lands in.
    let base = locate(db, &pts)
        .select((&*h.floor).and(&*h.ceil))
        .and(cands)
        .map(|((floor, ceil), c): ((f64, f64), Cand)| (c, floor, ceil, false));
    // Lines the candidate touches.
    let two_sided = (&db.linedef.bsec).map(|b: Row<Sector>| b.0 != NONE);
    let walls = cands
        .cross(view(db, lines).and(&db.linedef.flags).and(two_sided))
        .filt(|(c, (((g, _), _), _)): (Cand, (((Geom, Gap), i64), bool))| touches(g, c.2, c.3, c.5))
        .map(|(c, (((_, gap), flags), two)): (Cand, (((Geom, Gap), i64), bool))| line_clip(c, gap, flags, two))
        .key_by(|k: Clip| slot_key(k.0.0, k.0.1));
    // Bodies the candidate bumps into.
    let things = cands
        .cross(bodies)
        .filt(|(c, b): (Cand, Body)| overlaps(c, b))
        .map(|(c, _): (Cand, Body)| (c, f64::MIN, f64::MAX, true))
        .key_by(|k: Clip| slot_key(k.0.0, k.0.1));
    let init: Clip = ((NONE, NONE, 0.0, 0.0, 0.0, 0.0, 0.0, false, false, false), f64::MIN, f64::MAX, false);
    // Merge all constraints per candidate, keep those that fit, pick lowest idx per mover.
    base.union(walls)
        .union(things)
        // One merged Clip per candidate.
        .dense_fold(n, init, merge)
        .filt(move |k: Clip| fits(k, step))
        // Then per mover, the fitting candidate with the lowest idx.
        .key_by(|k: Clip| k.0.0)
        .dense_fold(n / 8 + 1, None, |a: Option<Clip>, k: Clip| match a {
            Some(a) if a.0.1 <= k.0.1 => Some(a),
            _ => Some(k),
        })
        .flat_map(|k: Option<Clip>| k)
        .collect()
}

// A sight line: eye position to target position.
pub type Sight = ((f64, f64, f64), (f64, f64, f64));

// Does this line block the sight line (crossing point outside its gap)?
fn blocks_sight(((ax, ay, az), (bx, by, bz)): Sight, (g, (lo, hi)): (Geom, Gap)) -> bool {
    crossing(g, (ax, ay), (bx, by)).is_some_and(|t| {
        let z = az + t * (bz - az);
        z < lo || z > hi
    })
}

// Sight pairs not blocked by any line.
pub fn visible<K: Dense>(db: &Db, lines: &Lines, pairs: &MultiRel<K, Sight>) -> MultiRel<K, Sight> {
    // Ids of pairs with at least one blocking line.
    let blocked = pairs
        .and(Same::<K>::new())
        .cross(view(db, lines))
        .filt(|((s, _), row): ((Sight, K), (Geom, Gap))| blocks_sight(s, row))
        .key_by(|((_, id), _): ((Sight, K), (Geom, Gap))| id)
        .dense_fold(pairs.n_dom(), true, |_, _| true);
    pairs.minus(&blocked).collect()
}

// Shot hitting a wall: crosses a line outside its gap. The puff is pulled back `back` units toward the shooter.
fn wall_impact((sid, _, x, y, z, ang, range, damage): Shot, (g, (lo, hi)): (Geom, Gap), back: f64) -> Option<Impact> {
    let end = (x + range * ang.cos(), y + range * ang.sin());
    crossing(g, (x, y), end).and_then(|t| {
        let d = (t * range - back).max(0.0);
        (z < lo || z > hi).then(|| (d, NONE, x + d * ang.cos(), y + d * ang.sin(), z, damage, sid))
    })
}

// Shot hitting a body: ray passes within its radius.
fn body_impact((sid, from, x, y, _, ang, range, damage): Shot, (id, _, bx, by, bz, radius, height, _, shootable): Body) -> Option<Impact> {
    let (dx, dy) = (bx - x, by - y);
    let (sn, cs) = ang.sin_cos();
    // Body centre in ray coordinates: distance along the ray and off to the side.
    let along = dx * cs + dy * sn;
    let perp = (-dx * sn + dy * cs).abs();
    (shootable && id != from && along > 0.0 && along < range && perp < radius).then(|| {
        let t = along - (radius * radius - perp * perp).sqrt();
        (t, id, x + t * cs, y + t * sn, bz + height / 2.0, damage, sid)
    })
}

// For each shot, the nearest wall or body it hits.
pub fn hitscan(db: &Db, lines: &Lines, bodies: &MultiRel<usize, Body>, shots: &MultiRel<usize, Shot>) -> MultiRel<usize, Impact> {
    let back = one(&db.t.k.puff_back);
    shots
        .and(Same::<usize>::new())
        .cross(view(db, lines))
        .flat_map(move |((s, id), row): ((Shot, usize), (Geom, Gap))| wall_impact(s, row, back).map(|i| (id, i)))
        .key_by(|(id, _)| id)
        .union(shots.and(Same::<usize>::new()).cross(bodies).flat_map(|((s, id), b): ((Shot, usize), Body)| body_impact(s, b).map(|i| (id, i))).key_by(|(id, _)| id))
        .map(|(_, i)| i)
        .dense_fold(shots.n_dom(), None, |a: Option<Impact>, i: Impact| match a {
            Some(a) if a.0 <= i.0 => Some(a),
            _ => Some(i),
        })
        .flat_map(|i: Option<Impact>| i)
        .collect()
}
