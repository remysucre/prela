use crate::db::{Db, Line};
use crate::physics::{crossing, side_of, LineRow};
use crate::player::Player;
use crate::wad::Sector;
use prela::engine::*;

const USERANGE: f64 = 64.0;

#[derive(Clone, Copy, Debug)]
pub enum Mover {
    Idle,
    Door { dir: i8, speed: f64, top: f64, wait: i32, stay: bool },
    Lift { dir: i8, low: f64, high: f64, wait: i32 },
    Floor { low: f64 },
}

#[derive(Clone, Copy, Debug)]
pub enum Act {
    Door { speed: f64, stay: bool },
    Lift,
    Floor,
    Exit,
}

fn act_for(special: i64) -> Option<Act> {
    match special {
        1 | 26 => Some(Act::Door { speed: 2.0, stay: false }),
        117 => Some(Act::Door { speed: 8.0, stay: false }),
        2 => Some(Act::Door { speed: 2.0, stay: true }),
        62 | 88 => Some(Act::Lift),
        23 => Some(Act::Floor),
        11 => Some(Act::Exit),
        _ => None,
    }
}

fn tagged(special: i64) -> bool {
    matches!(special, 2 | 23 | 62 | 88)
}

fn key_needed(special: i64) -> u8 {
    match special {
        26 | 32 => 1,
        27 | 34 => 2,
        28 | 33 => 4,
        _ => 0,
    }
}

fn step_mover(m: Mover, act: Option<Act>, (lc, lf): (f64, f64), s: Sector, occupied: bool) -> Mover {
    let m = match (m, act) {
        (Mover::Idle, Some(Act::Door { speed, stay })) => Mover::Door { dir: 1, speed, top: lc - 4.0, wait: 0, stay },
        (Mover::Door { dir: -1, speed, top, stay, .. }, Some(Act::Door { .. })) => Mover::Door { dir: 1, speed, top, wait: 0, stay },
        (Mover::Door { speed, top, stay: false, .. }, Some(Act::Door { stay: false, .. })) => {
            Mover::Door { dir: -1, speed, top, wait: 0, stay: false }
        }
        (Mover::Idle, Some(Act::Lift)) => Mover::Lift { dir: -1, low: lf.min(s.floor), high: s.floor, wait: 0 },
        (Mover::Idle, Some(Act::Floor)) => Mover::Floor { low: lf.min(s.floor) },
        (m, _) => m,
    };
    match m {
        Mover::Door { dir: 1, speed, top, stay, .. } if s.ceil + speed >= top => {
            if stay { Mover::Idle } else { Mover::Door { dir: 0, speed, top, wait: 150, stay } }
        }
        Mover::Door { dir: 0, speed, top, wait: 0, stay } => Mover::Door { dir: -1, speed, top, wait: 0, stay },
        Mover::Door { dir: 0, speed, top, wait, stay } => Mover::Door { dir: 0, speed, top, wait: wait - 1, stay },
        Mover::Door { dir: -1, speed, top, stay, .. } if occupied => Mover::Door { dir: 1, speed, top, wait: 0, stay },
        Mover::Door { dir: -1, speed, .. } if s.ceil - speed <= s.floor => Mover::Idle,
        Mover::Lift { dir: -1, low, high, .. } if s.floor - 4.0 <= low => Mover::Lift { dir: 0, low, high, wait: 105 },
        Mover::Lift { dir: 0, low, high, wait: 0 } => Mover::Lift { dir: 1, low, high, wait: 0 },
        Mover::Lift { dir: 0, low, high, wait } => Mover::Lift { dir: 0, low, high, wait: wait - 1 },
        Mover::Lift { dir: 1, high, .. } if s.floor + 4.0 >= high => Mover::Idle,
        Mover::Floor { low } if s.floor - 1.0 <= low => Mover::Idle,
        m => m,
    }
}

fn move_sector(s: Sector, m: Mover) -> Sector {
    match m {
        Mover::Door { dir: 1, speed, top, .. } => Sector { ceil: (s.ceil + speed).min(top), ..s },
        Mover::Door { dir: -1, speed, .. } => Sector { ceil: (s.ceil - speed).max(s.floor), ..s },
        Mover::Lift { dir: -1, low, .. } => Sector { floor: (s.floor - 4.0).max(low), ..s },
        Mover::Lift { dir: 1, high, .. } => Sector { floor: (s.floor + 4.0).min(high), ..s },
        Mover::Floor { low } => Sector { floor: (s.floor - 1.0).max(low), ..s },
        _ => s,
    }
}

#[derive(Clone, Copy)]
struct Hit {
    t: f64,
    line: Line,
    special: bool,
    closed: bool,
}

fn use_hit(p: Player, row: LineRow) -> Option<Hit> {
    let ((l, f), b) = row;
    let reach = (p.x + USERANGE * p.ang.cos(), p.y + USERANGE * p.ang.sin());
    crossing(l, (p.x, p.y), reach).map(|t| Hit {
        t,
        line: l,
        special: l.special != 0 && side_of(l, p.x, p.y) < 0.0 && p.keys & key_needed(l.special) == key_needed(l.special),
        closed: b.is_none_or(|b| f.ceil.min(b.ceil) <= f.floor.max(b.floor)),
    })
}

pub fn line_events(db: &Db, lines: &VecRel<usize, LineRow>, old: &VecRel<usize, Player>, new: &VecRel<usize, Player>, use_: bool) -> MultiRel<usize, (Line, Act)> {
    let crossed = new
        .and(old)
        .filt(|(p, _): (Player, Player)| !p.dead)
        .cross(&db.lines)
        .filt(|(_, l): ((Player, Player), Line)| matches!(l.special, 2 | 88))
        .filt(|((n, o), l): ((Player, Player), Line)| crossing(l, (o.x, o.y), (n.x, n.y)).is_some())
        .map(|(_, l)| l)
        .key_by(|_| 0usize);
    let used = new
        .and(old)
        .filt(move |(p, o): (Player, Player)| use_ && !o.usedown && !p.dead)
        .cross(lines)
        .flat_map(|((p, _), row): ((Player, Player), LineRow)| use_hit(p, row))
        .filt(|h: Hit| h.special || h.closed)
        .key_by(|_| 0usize)
        .fold(None, |a: Option<Hit>, h: Hit| match a {
            Some(a) if a.t <= h.t => Some(a),
            _ => Some(h),
        })
        .flat_map(|h: Option<Hit>| h.filter(|h| h.special).map(|h| h.line));
    crossed.union(used).flat_map(|l: Line| act_for(l.special).map(|a| (l, a))).collect()
}

pub fn exited(events: &MultiRel<usize, (Line, Act)>) -> bool {
    events.fold_flat(false, |e, (_, a): (Line, Act)| e || matches!(a, Act::Exit))
}

pub fn movers(
    db: &Db,
    sectors: &VecRel<usize, Sector>,
    movers: &VecRel<usize, Mover>,
    events: &MultiRel<usize, (Line, Act)>,
    occupied: &Fold<usize, bool>,
) -> (VecRel<usize, Sector>, VecRel<usize, Mover>) {
    let acts = events
        .filt(|(l, _): (Line, Act)| tagged(l.special))
        .key_by(|(l, _)| l.tag as usize)
        .and(&db.tag_sectors)
        .key_by(|(_, sec)| sec)
        .map(|((_, a), _): ((Line, Act), usize)| a)
        .union(events.filt(|(l, _): (Line, Act)| !tagged(l.special)).key_by(|(l, _)| l.bsec).map(|(_, a): (Line, Act)| a))
        .fold(None, |_, a: Act| Some(a));
    let lowest = (&db.neighbors)
        .select(sectors)
        .fold((f64::MAX, f64::MAX), |(c, f): (f64, f64), s: Sector| (c.min(s.ceil), f.min(s.floor)));
    let next: VecRel<usize, Mover> = movers
        .and(acts.opt())
        .and(lowest.opt())
        .and(sectors)
        .and(occupied.opt())
        .map(|((((m, act), low), sec), occ): ((((Mover, Option<Option<Act>>), Option<(f64, f64)>), Sector), Option<bool>)| {
            step_mover(m, act.flatten(), low.unwrap_or((sec.ceil, sec.floor)), sec, occ.is_some())
        })
        .collect();
    let moved = sectors.and(&next).map(|(s, m)| move_sector(s, m)).collect();
    (moved, next)
}
