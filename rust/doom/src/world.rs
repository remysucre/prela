use crate::db::Db;
use crate::info::{act_key, mfact, step_key, MV_EXIT};
use crate::physics::{crossing, geom, side_of, view, Gap, Geom, Lines};
use crate::rules::{bit, matching};
use crate::schema::{one, rc, Linedef, LineSpecial, MoverRule, Player, Sector, SectorState};
use prela::engine::*;

// A door/lift as a row: kind, plane (0 ceiling, 1 floor), direction, speed, target up, target down, wait.
type Mover = (u8, u8, i8, f64, f64, f64, i32);
// An activation row: kind, direction, init (else a reversal), plane, and the target-height weights.
type Act = (u8, i8, bool, u8, f64, f64, f64, f64);
// A mover after one tic of travel: sector, mover, new ceiling and floor.
type Moved = (Id<Sector>, Mover, f64, f64);
// Lines triggered this tic, with their special.
pub type Events = MultiRel<usize, (Id<Linedef>, Id<LineSpecial>)>;

// Start (or reverse) a mover from a triggered special of the given speed; sets target heights.
// (lc, lf) = lowest neighbouring ceiling and floor. A reversal (`!init`) keeps speed, plane and targets.
fn activate(m: Mover, speed: f64, (lc, lf): (f64, f64), (_, floor): (f64, f64), o: Act, gap: f64) -> Mover {
    let (kind, dir, init, plane, up_lc, up_floor, down_floor, down_lowest) = o;
    let lowest = lf.min(floor);
    (
        kind,
        if init { plane } else { m.1 },
        dir,
        if init { speed } else { m.3 },
        if init { up_lc * (lc - gap) + up_floor * floor } else { m.4 },
        if init { down_floor * floor + down_lowest * lowest } else { m.5 },
        0,
    )
}

// Move the plane one step toward its target; return facts for the rule lookup.
fn travel(id: Id<Sector>, m: Mover, (ceil, floor): (f64, f64), occupied: bool) -> (Moved, usize, u32) {
    let (kind, plane, dir, speed, up, down, wait) = m;
    // p selects the plane: h is the current height of the moving plane.
    let p = plane as f64;
    let h = ceil * (1.0 - p) + floor * p;
    let target = up * (dir == 1) as i32 as f64 + down * (dir == -1) as i32 as f64;
    // Move by speed, but never past the target.
    let step = dir as f64 * speed.min((target - h).abs());
    let facts = bit(dir != 0 && h + step == target, mfact::REACHED) | bit(wait == 0, mfact::EXPIRED) | bit(occupied, mfact::OCCUPIED);
    ((id, m, ceil + (1.0 - p) * step, floor + p * step), step_key(kind, dir), facts)
}

// Apply the chosen mover rule (kind, dir, wait, wait_dec): a negative wait keeps the timer, minus one if wait_dec.
fn settle((id, m, c, f): Moved, (((kind, dir), wait), dec): (((u8, i8), i32), bool)) -> Moved {
    let wait = if wait >= 0 { wait } else { m.6 - dec as i32 };
    (id, (kind, m.1, dir, m.3, m.4, m.5, wait), c, f)
}

// A line hit by the player's use ray: distance, line, usable special, closed.
type Hit = (f64, Id<Linedef>, bool, bool);

// Test the use ray from (x, y, ang) holding `keys` against one line. `special`: a use special,
// usable from this side with the right key. `closed`: no gap to see through, so the ray stops here.
fn use_hit((x, y, ang, keys): (f64, f64, f64, u8), (((g, (lo, hi)), line), sp): (((Geom, Gap), Id<Linedef>), Option<(bool, u8)>), range: f64) -> Option<Hit> {
    let reach = (x + range * ang.cos(), y + range * ang.sin());
    let usable = sp.is_some_and(|(use_, key)| use_ && keys & key == key);
    crossing(g, (x, y), reach).map(|t| (t, line, usable && side_of(g, x, y) < 0.0, hi <= lo))
}

// Specials triggered this tic: walk-over lines crossed and use lines pressed.
pub fn line_events(db: &Db, lines: &Lines, old: &Player, new: &Player, use_: bool) -> Events {
    let range = one(&db.t.k.use_range);
    let sp = &db.t.special;
    // Each line's special, if it has one.
    let lsp = || (&db.linedef.special).select(&db.t.special_num);
    // Lines with a walk-over special, with their segment.
    let walkable: MultiRel<usize, ((Id<Linedef>, Id<LineSpecial>), Geom)> = lsp()
        .and(lsp().select(&sp.walk))
        .filt(|(_, walk): (Id<LineSpecial>, bool)| walk)
        .and(Same::<Id<Linedef>>::new())
        .map(|((s, _), l)| (l, s))
        .and(geom(db))
        .key_by(|_| 0usize)
        .collect();
    // Walk lines the player crossed between old and new position.
    let crossed = (&*new.x)
        .and(&*new.y)
        .and((&*old.x).and(&*old.y))
        .and(&*new.dead)
        .filt(|(_, dead): (((f64, f64), (f64, f64)) , bool)| !dead)
        .cross(&walkable)
        .filt(|((((nx, ny), (ox, oy)), _), (_, g)): ((((f64, f64), (f64, f64)), bool), ((Id<Linedef>, Id<LineSpecial>), Geom))| crossing(g, (ox, oy), (nx, ny)).is_some())
        .map(|(_, (ls, _))| ls)
        .key_by(|_| 0usize);
    // Nearest line hit by the use ray; fires only if it has a use special.
    let used = (&*new.x)
        .and(&*new.y)
        .and(&*new.ang)
        .and(&*new.keys)
        .and((&*new.dead).and(&*old.usedown))
        .filt(move |(_, (dead, held)): ((((f64, f64), f64), u8), (bool, bool))| use_ && !held && !dead)
        .map(|((((x, y), ang), keys), _)| (x, y, ang, keys))
        .cross(view(db, lines).and(Same::<Id<Linedef>>::new()).and(lsp().select((&sp.use_).and(&sp.key)).opt()))
        .flat_map(move |(p, row): ((f64, f64, f64, u8), (((Geom, Gap), Id<Linedef>), Option<(bool, u8)>))| use_hit(p, row, range))
        // Only lines that could stop the ray matter.
        .filt(|h: Hit| h.2 || h.3)
        // Nearest one.
        .key_by(|_| 0usize)
        .fold(None, |a: Option<Hit>, h: Hit| match a {
            Some(a) if a.0 <= h.0 => Some(a),
            _ => Some(h),
        })
        // Fires only if that nearest line is a usable special (a plain wall in front blocks it).
        .flat_map(|h: Option<Hit>| h.filter(|h| h.2).map(|h| h.1))
        .key_by(|l| l)
        .and(lsp())
        .key_by(|_| 0usize);
    crossed.union(used).collect()
}

// Did any event trigger the level exit?
pub fn exited(db: &Db, events: &Events) -> bool {
    events.map(|(_, s)| s).select(&db.t.special.act).fold_flat(false, |e, act: u8| e || act == MV_EXIT)
}

// Advance doors/lifts one tic: activate from events, move, then apply mover rules.
pub fn movers(db: &Db, s: &SectorState, events: &Events, occupied: &Fold<Id<Sector>, bool>) -> SectorState {
    let gap = one(&db.t.k.door_gap);
    let (sp, ac) = (&db.t.special, &db.t.activation);
    // Which sectors each event activates (by tag, or the line's back sector).
    let tagged = || events.key_by(|(_, s)| s).and(&sp.tagged).key_by(|((l, _), _)| l);
    let acts = tagged()
        .filt(|(_, t): ((Id<Linedef>, Id<LineSpecial>), bool)| t)
        .and(&db.linedef.tag)
        .key_by(|(_, tag)| tag)
        .and(&db.tag_sectors)
        .key_by(|(_, sec)| sec)
        .map(|((((_, s), _), _), _): ((((Id<Linedef>, Id<LineSpecial>), bool), i64), Id<Sector>)| s)
        .union(
            tagged()
                .filt(|(_, t): ((Id<Linedef>, Id<LineSpecial>), bool)| !t)
                .and(&db.linedef.bsec)
                .key_by(|(_, sec)| sec)
                .map(|(((_, s), _), _): (((Id<Linedef>, Id<LineSpecial>), bool), Id<Sector>)| s),
        )
        // One activation per sector (last one wins).
        .fold(None, |_, s: Id<LineSpecial>| Some(s));
    let heights = || (&*s.ceil).and(&*s.floor);
    // Lowest neighboring ceiling/floor per sector (lift and door targets).
    let lowest = (&db.sector.neighbors)
        .select(heights())
        .fold((f64::MAX, f64::MAX), |(c, f): (f64, f64), (sc, sf): (f64, f64)| (c.min(sc), f.min(sf)));
    let mover = (&*s.kind)
        .and(&*s.plane)
        .and(&*s.dir)
        .and(&*s.speed)
        .and(&*s.up)
        .and(&*s.down)
        .and(&*s.wait)
        .map(|((((((k, p), d), sp), u), dn), w): ((((((u8, u8), i8), f64), f64), f64), i32)| (k, p, d, sp, u, dn, w));
    // The activating special's act and speed, per sector.
    let special = (&acts).flat_map(|a: Option<Id<LineSpecial>>| a).select((&sp.act).and(&sp.speed));
    let act_row = (&ac.kind)
        .and(&ac.dir)
        .and(&ac.init)
        .and(&ac.plane)
        .and(&ac.up_lc)
        .and(&ac.up_floor)
        .and(&ac.down_floor)
        .and(&ac.down_lowest)
        .map(|(((((((k, d), i), p), ul), uf), df), dl): (((((((u8, i8), bool), u8), f64), f64), f64), f64)| (k, d, i, p, ul, uf, df, dl));
    // Start or reverse movers that got an activation.
    let activated: VecRel<Id<Sector>, Mover> = mover
        // Per sector: mover, (act, speed)?, lowest neighbours?, heights, sector id.
        .and(special.opt())
        .and(lowest.opt())
        .and(heights())
        .and(Same::<Id<Sector>>::new())
        .map(|((((m, a), low), cf), id): ((((Mover, Option<(u8, f64)>), Option<(f64, f64)>), (f64, f64)), Id<Sector>)| (id, m, a, low.unwrap_or(cf), cf))
        // Look up (current kind, dir, incoming act) in the activation table; no activation = usize::MAX = no match.
        .key_by(|(_, m, a, _, _): (Id<Sector>, Mover, Option<(u8, f64)>, (f64, f64), (f64, f64))| a.map_or(usize::MAX, |(act, _)| act_key(m.0, m.2, act)))
        .and((&db.t.activation_key).select(act_row).opt())
        .map(|((id, m, a, low, cf), o): ((Id<Sector>, Mover, Option<(u8, f64)>, (f64, f64), (f64, f64)), Option<Act>)| {
            (id, a.zip(o).map_or(m, |((_, speed), o)| activate(m, speed, low, cf, o, gap)))
        })
        .key_by(|(id, _)| id)
        .map(|(_, m)| m)
        .collect();
    // Move each plane one step and compute facts.
    let rows: HashIdx<Id<Sector>, (Moved, usize, u32)> = (&activated)
        .and(heights())
        .and(occupied.opt())
        .and(Same::<Id<Sector>>::new())
        .map(|(((m, cf), occ), id): (((Mover, (f64, f64)), Option<bool>), Id<Sector>)| travel(id, m, cf, occ.is_some()))
        .key_by(|(v, _, _): (Moved, usize, u32)| v.0)
        .collect();
    // Pick the mover rule for each sector and apply it.
    let r = &db.t.mover_rule;
    let chosen = matching(&rows, r, &db.t.mover_by_key, |v: Moved| v.0);
    let settled: VecRel<Id<Sector>, Moved> = (&rows)
        .and(chosen.map(|(_, rule): (Moved, Id<MoverRule>)| rule).select((&r.kind).and(&r.dir).and(&r.wait).and(&r.wait_dec)).opt())
        .map(|((v, _, _), o): ((Moved, usize, u32), Option<(((u8, i8), i32), bool)>)| o.map_or(v, |o| settle(v, o)))
        .collect();
    SectorState {
        floor: rc((&settled).map(|v: Moved| v.3)),
        ceil: rc((&settled).map(|v: Moved| v.2)),
        kind: rc((&settled).map(|v: Moved| v.1.0)),
        plane: rc((&settled).map(|v: Moved| v.1.1)),
        dir: rc((&settled).map(|v: Moved| v.1.2)),
        speed: rc((&settled).map(|v: Moved| v.1.3)),
        up: rc((&settled).map(|v: Moved| v.1.4)),
        down: rc((&settled).map(|v: Moved| v.1.5)),
        wait: rc((&settled).map(|v: Moved| v.1.6)),
        heard: s.heard.clone(),
    }
}
