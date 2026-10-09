use crate::db::{Db, Line};
use crate::info::{act_key, mfact, step_key, ActOut, Special, StepOut, MV_EXIT, MV_IDLE};
use crate::physics::{crossing, side_of, LineRow};
use crate::player::Player;
use crate::rules::{bit, matching};
use crate::wad::Sector;
use prela::engine::*;

const USERANGE: f64 = 64.0;

#[derive(Clone, Copy, Debug)]
pub struct Mover {
    pub kind: u8,
    pub plane: u8,
    pub dir: i8,
    pub speed: f64,
    pub up: f64,
    pub down: f64,
    pub wait: i32,
}

pub const IDLE: Mover = Mover { kind: MV_IDLE, plane: 0, dir: 0, speed: 0.0, up: 0.0, down: 0.0, wait: 0 };

fn activate(m: Mover, a: Special, (lc, lf): (f64, f64), s: Sector, o: ActOut) -> Mover {
    let lowest = lf.min(s.floor);
    Mover {
        kind: o.kind,
        dir: o.dir,
        wait: 0,
        speed: if o.init { a.speed } else { m.speed },
        plane: if o.init { o.plane } else { m.plane },
        up: if o.init { o.up_lc * (lc - 4.0) + o.up_floor * s.floor } else { m.up },
        down: if o.init { o.down_floor * s.floor + o.down_lowest * lowest } else { m.down },
    }
}

#[derive(Clone, Copy)]
struct Moved {
    id: usize,
    m: Mover,
    s: Sector,
}

fn travel(id: usize, m: Mover, s: Sector, occupied: bool) -> (Moved, usize, u32) {
    let p = m.plane as f64;
    let h = s.ceil * (1.0 - p) + s.floor * p;
    let target = m.up * (m.dir == 1) as i32 as f64 + m.down * (m.dir == -1) as i32 as f64;
    let step = m.dir as f64 * m.speed.min((target - h).abs());
    let s = Sector { ceil: s.ceil + (1.0 - p) * step, floor: s.floor + p * step, ..s };
    let facts = bit(m.dir != 0 && h + step == target, mfact::REACHED) | bit(m.wait == 0, mfact::EXPIRED) | bit(occupied, mfact::OCCUPIED);
    (Moved { id, m, s }, step_key(m.kind, m.dir), facts)
}

fn settle(v: Moved, o: StepOut) -> Moved {
    let wait = if o.wait >= 0 { o.wait } else { v.m.wait - o.wait_dec as i32 };
    Moved { m: Mover { kind: o.kind, dir: o.dir, wait, ..v.m }, ..v }
}

#[derive(Clone, Copy)]
struct Hit {
    t: f64,
    line: Line,
    special: bool,
    closed: bool,
}

fn use_hit(p: Player, (((l, f), b), sp): (LineRow, Option<Special>)) -> Option<Hit> {
    let reach = (p.x + USERANGE * p.ang.cos(), p.y + USERANGE * p.ang.sin());
    let usable = sp.is_some_and(|sp| sp.use_ && p.keys & sp.key == sp.key);
    crossing(l, (p.x, p.y), reach).map(|t| Hit {
        t,
        line: l,
        special: usable && side_of(l, p.x, p.y) < 0.0,
        closed: b.is_none_or(|b| f.ceil.min(b.ceil) <= f.floor.max(b.floor)),
    })
}

pub fn line_events(db: &Db, lines: &VecRel<usize, LineRow>, old: &VecRel<usize, Player>, new: &VecRel<usize, Player>, use_: bool) -> MultiRel<usize, (Line, Special)> {
    let walkable: MultiRel<usize, (Line, Special)> = (&db.lines)
        .key_by(|l: Line| l.special)
        .and(&db.t.specials)
        .filt(|(_, sp): (Line, Special)| sp.walk)
        .key_by(|_| 0usize)
        .collect();
    let crossed = new
        .and(old)
        .filt(|(p, _): (Player, Player)| !p.dead)
        .cross(&walkable)
        .filt(|((n, o), (l, _)): ((Player, Player), (Line, Special))| crossing(l, (o.x, o.y), (n.x, n.y)).is_some())
        .map(|(_, ls)| ls)
        .key_by(|_| 0usize);
    let with_special = lines.and(lines.map(|((l, _), _): LineRow| l.special).select(&db.t.specials).opt());
    let used = new
        .and(old)
        .filt(move |(p, o): (Player, Player)| use_ && !o.usedown && !p.dead)
        .cross(with_special)
        .flat_map(|((p, _), row): ((Player, Player), (LineRow, Option<Special>))| use_hit(p, row))
        .filt(|h: Hit| h.special || h.closed)
        .key_by(|_| 0usize)
        .fold(None, |a: Option<Hit>, h: Hit| match a {
            Some(a) if a.t <= h.t => Some(a),
            _ => Some(h),
        })
        .flat_map(|h: Option<Hit>| h.filter(|h| h.special).map(|h| h.line))
        .key_by(|l: Line| l.special)
        .and(&db.t.specials)
        .key_by(|_| 0usize);
    crossed.union(used).collect()
}

pub fn exited(events: &MultiRel<usize, (Line, Special)>) -> bool {
    events.fold_flat(false, |e, (_, sp): (Line, Special)| e || sp.act == MV_EXIT)
}

pub fn movers(
    db: &Db,
    sectors: &VecRel<usize, Sector>,
    movers: &VecRel<usize, Mover>,
    events: &MultiRel<usize, (Line, Special)>,
    occupied: &Fold<usize, bool>,
) -> (VecRel<usize, Sector>, VecRel<usize, Mover>) {
    let acts = events
        .filt(|(_, sp): (Line, Special)| sp.tagged)
        .key_by(|(l, _)| l.tag as usize)
        .and(&db.tag_sectors)
        .key_by(|(_, sec)| sec)
        .map(|((_, sp), _): ((Line, Special), usize)| sp)
        .union(events.filt(|(_, sp): (Line, Special)| !sp.tagged).key_by(|(l, _)| l.bsec).map(|(_, sp): (Line, Special)| sp))
        .fold(None, |_, sp: Special| Some(sp));
    let lowest = (&db.neighbors)
        .select(sectors)
        .fold((f64::MAX, f64::MAX), |(c, f): (f64, f64), s: Sector| (c.min(s.ceil), f.min(s.floor)));
    let activated: VecRel<usize, Mover> = movers
        .and(acts.opt())
        .and(lowest.opt())
        .and(sectors)
        .and(Same::<usize>::new())
        .map(|((((m, a), low), s), id): ((((Mover, Option<Option<Special>>), Option<(f64, f64)>), Sector), usize)| {
            (id, m, a.flatten(), low.unwrap_or((s.ceil, s.floor)), s)
        })
        .key_by(|(_, m, a, _, _): (usize, Mover, Option<Special>, (f64, f64), Sector)| a.map_or(usize::MAX, |a| act_key(m.kind, m.dir, a.act)))
        .and((&db.t.activations).opt())
        .map(|((id, m, a, low, s), o): ((usize, Mover, Option<Special>, (f64, f64), Sector), Option<ActOut>)| {
            (id, a.zip(o).map_or(m, |(a, o)| activate(m, a, low, s, o)))
        })
        .key_by(|(id, _)| id)
        .map(|(_, m)| m)
        .collect();
    let rows: HashIdx<usize, (Moved, usize, u32)> = (&activated)
        .and(sectors)
        .and(occupied.opt())
        .and(Same::<usize>::new())
        .map(|(((m, s), occ), id): (((Mover, Sector), Option<bool>), usize)| travel(id, m, s, occ.is_some()))
        .key_by(|(v, _, _): (Moved, usize, u32)| v.id)
        .collect();
    let chosen = matching(&rows, &db.t.mover_rules, |v: Moved| v.id);
    let settled: VecRel<usize, Moved> = (&rows)
        .and(chosen.opt())
        .map(|((v, _, _), o): ((Moved, usize, u32), Option<(Moved, StepOut)>)| o.map_or(v, |(_, o)| settle(v, o)))
        .collect();
    ((&settled).map(|v: Moved| v.s).collect(), (&settled).map(|v: Moved| v.m).collect())
}
