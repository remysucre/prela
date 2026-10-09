use crate::db::{Db, Info, StateRow};
use crate::info::{Action, HANGING, MISSILE, MONSTER, NOGRAV, SHOOTABLE, SOLID};
use crate::physics::{locate, rnd, Body, Cand, Clip, Impact, Shot, Sight};
use crate::player::{Player, HEIGHT, PLAYER, RADIUS};
use crate::wad::{Sector, Thing, NONE};
use prela::engine::*;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

#[derive(Clone, Copy, Debug)]
pub struct Mobj {
    pub id: usize,
    pub kind: usize,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ang: f64,
    pub momx: f64,
    pub momy: f64,
    pub state: usize,
    pub tics: i32,
    pub health: i32,
    pub flags: u32,
    pub awake: bool,
    pub reaction: i32,
    pub sector: usize,
    pub source: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Spawn {
    pub id: usize,
    pub kind: usize,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ang: f64,
    pub speed: f64,
    pub source: usize,
}

pub fn spawn<Q: Drive<R = Spawn>>(db: &Db, reqs: Q) -> HashIdx<usize, Mobj> {
    reqs.key_by(|s: Spawn| s.kind)
        .and(&db.t.infos)
        .key_by(|(_, i): (Spawn, Info)| i.spawn)
        .and(&db.t.states)
        .map(|((s, i), r): ((Spawn, Info), StateRow)| Mobj {
            id: s.id,
            kind: s.kind,
            x: s.x,
            y: s.y,
            z: s.z,
            ang: s.ang,
            momx: s.speed * s.ang.cos(),
            momy: s.speed * s.ang.sin(),
            state: i.spawn,
            tics: r.tics,
            health: i.health,
            flags: i.flags,
            awake: false,
            reaction: 8,
            sector: NONE,
            source: s.source,
        })
        .key_by(|m: Mobj| m.id)
        .collect()
}

pub fn spawn_things(db: &Db, sectors: &VecRel<usize, Sector>) -> HashIdx<usize, Mobj> {
    let reqs = (&db.things)
        .and(Same::<usize>::new())
        .filt(|(t, _): (Thing, usize)| t.flags & 2 != 0 && t.flags & 16 == 0)
        .key_by(|(t, _): (Thing, usize)| t.kind)
        .and(&db.t.ednum_info)
        .map(|((t, i), kind): ((Thing, usize), usize)| Spawn {
            id: i + 1,
            kind,
            x: t.x,
            y: t.y,
            z: 0.0,
            ang: t.angle.to_radians(),
            speed: 0.0,
            source: NONE,
        });
    relocate(db, sectors, &spawn(db, reqs))
}

pub fn relocate(db: &Db, sectors: &VecRel<usize, Sector>, mobjs: &HashIdx<usize, Mobj>) -> HashIdx<usize, Mobj> {
    let pts: HashIdx<usize, (f64, f64)> = mobjs.map(|m: Mobj| (m.x, m.y)).collect();
    locate(db, &pts)
        .select(sectors.and(Same::<usize>::new()))
        .and(mobjs)
        .key_by(|(_, m): ((Sector, usize), Mobj)| m.kind)
        .and(&db.t.infos)
        .map(|(((s, sec), m), i): (((Sector, usize), Mobj), Info)| {
            let z = if m.flags & (MISSILE | NOGRAV) != 0 {
                m.z
            } else if m.flags & HANGING != 0 {
                s.ceil - i.height
            } else {
                s.floor
            };
            Mobj { z, sector: sec, ..m }
        })
        .key_by(|m: Mobj| m.id)
        .collect()
}

pub fn bodies(db: &Db, players: &VecRel<usize, Player>, mobjs: &HashIdx<usize, Mobj>) -> HashIdx<usize, Body> {
    mobjs
        .filt(|m: Mobj| m.flags & (SOLID | SHOOTABLE) != 0)
        .key_by(|m: Mobj| m.kind)
        .and(&db.t.infos)
        .map(|(m, i): (Mobj, Info)| Body {
            id: m.id,
            x: m.x,
            y: m.y,
            z: m.z,
            radius: i.radius,
            height: i.height,
            solid: m.flags & SOLID != 0,
            shootable: m.flags & SHOOTABLE != 0,
        })
        .key_by(|b: Body| b.id)
        .union(
            players
                .map(|p: Player| Body { id: PLAYER, x: p.x, y: p.y, z: p.z, radius: RADIUS, height: HEIGHT, solid: !p.dead, shootable: !p.dead })
                .key_by(|b: Body| b.id),
        )
        .collect()
}

pub fn advance(db: &Db, mobjs: &HashIdx<usize, Mobj>) -> HashIdx<usize, (Mobj, Action)> {
    mobjs
        .key_by(|m: Mobj| m.state)
        .and(&db.t.states)
        .map(|(m, r): (Mobj, StateRow)| match m.tics {
            -1 => (m, NONE),
            t if t > 1 => (Mobj { tics: t - 1, ..m }, NONE),
            _ => (m, r.next),
        })
        .key_by(|(_, next)| next)
        .and((&db.t.states).opt())
        .map(|((m, next), r): ((Mobj, usize), Option<StateRow>)| match r {
            Some(r) => (Mobj { state: next, tics: r.tics, ..m }, r.action),
            None => (m, Action::None),
        })
        .filt(|(m, _): (Mobj, Action)| m.state != 0)
        .key_by(|(m, _): (Mobj, Action)| m.id)
        .collect()
}

pub fn sight_pairs(advanced: &HashIdx<usize, (Mobj, Action)>, players: &VecRel<usize, Player>) -> HashIdx<usize, Sight> {
    advanced
        .filt(|(_, a): (Mobj, Action)| matches!(a, Action::Look | Action::Chase))
        .key_by(|_| PLAYER)
        .and(players)
        .map(|((m, _), p): ((Mobj, Action), Player)| (m.id, ((m.x, m.y, m.z + 40.0), (p.x, p.y, p.z + 40.0))))
        .key_by(|(id, _)| id)
        .map(|(_, s)| s)
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub enum Attack {
    None,
    Hitscan { pellets: usize, mult: i32, modulo: i32 },
    Melee { damage: i32 },
    Ball,
}

#[derive(Clone, Copy, Debug)]
pub struct Decision {
    pub m: Mobj,
    pub goto: usize,
    pub go: Option<f64>,
    pub attack: Attack,
    pub explode: bool,
    pub hit: usize,
}

fn decide(m: Mobj, act: Action, i: Info, p: Player, sees: bool, heard: bool, tic: u64) -> Decision {
    let (dx, dy) = (p.x - m.x, p.y - m.y);
    let dist = (dx * dx + dy * dy).sqrt();
    let face = dy.atan2(dx);
    let none = Decision { m, goto: NONE, go: None, attack: Attack::None, explode: false, hit: NONE };
    let r = |salt| rnd(tic, m.id, salt);
    let melee_range = dist < 64.0 - 20.0 + RADIUS;
    match act {
        Action::Look => {
            let ahead = ((face - m.ang + PI).rem_euclid(TAU) - PI).abs() <= FRAC_PI_2;
            if !p.dead && (heard || (sees && (ahead || dist < 64.0))) {
                Decision { m: Mobj { awake: true, reaction: 8, ang: face, ..m }, goto: i.see, ..none }
            } else {
                none
            }
        }
        Action::Chase if p.dead => Decision { goto: i.spawn, m: Mobj { awake: false, ..m }, ..none },
        Action::Chase => {
            let m = Mobj { reaction: (m.reaction - 1).max(0), ..m };
            let mut d = dist - 64.0;
            if i.melee == 0 {
                d -= 128.0;
            }
            let shoot = i.missile != 0 && m.reaction == 0 && sees && r(1) as f64 >= d.clamp(0.0, 200.0);
            if i.melee != 0 && melee_range {
                Decision { m: Mobj { ang: face, ..m }, goto: i.melee, ..none }
            } else if shoot {
                Decision { m: Mobj { ang: face, ..m }, goto: i.missile, ..none }
            } else {
                Decision { m, go: Some(face), ..none }
            }
        }
        Action::FaceTarget => Decision { m: Mobj { ang: face, ..m }, ..none },
        Action::PosAttack => Decision { m: Mobj { ang: face, ..m }, attack: Attack::Hitscan { pellets: 1, mult: 3, modulo: 5 }, ..none },
        Action::SPosAttack => Decision { m: Mobj { ang: face, ..m }, attack: Attack::Hitscan { pellets: 3, mult: 3, modulo: 5 }, ..none },
        Action::TroopAttack if melee_range => Decision { attack: Attack::Melee { damage: 3 * (r(2) % 8 + 1) }, ..none },
        Action::TroopAttack => Decision { m: Mobj { ang: face, ..m }, attack: Attack::Ball, ..none },
        Action::SargAttack if melee_range => Decision { attack: Attack::Melee { damage: 4 * (r(2) % 10 + 1) }, ..none },
        Action::Fall => Decision { m: Mobj { flags: m.flags & !SOLID, ..m }, ..none },
        Action::Explode => Decision { explode: true, ..none },
        _ => none,
    }
}

pub fn think(
    db: &Db,
    advanced: &HashIdx<usize, (Mobj, Action)>,
    visible: &HashIdx<usize, Sight>,
    heard: &HashIdx<usize, bool>,
    players: &VecRel<usize, Player>,
    tic: u64,
) -> HashIdx<usize, Decision> {
    let decided = advanced
        .and(visible.opt())
        .key_by(|((m, _), _): ((Mobj, Action), Option<Sight>)| m.sector)
        .and(heard.opt())
        .key_by(|(((m, _), _), _): (((Mobj, Action), Option<Sight>), Option<bool>)| m.kind)
        .and(&db.t.infos)
        .key_by(|_| PLAYER)
        .and(players)
        .map(move |(((((m, a), s), h), i), p): (((((Mobj, Action), Option<Sight>), Option<bool>), Info), Player)| {
            decide(m, a, i, p, s.is_some(), h.is_some(), tic)
        });
    goto(db, decided)
}

pub fn goto<Q: Drive<R = Decision>>(db: &Db, decided: Q) -> HashIdx<usize, Decision> {
    decided
        .key_by(|d: Decision| d.goto)
        .and((&db.t.states).opt())
        .map(|(d, r): (Decision, Option<StateRow>)| match r {
            Some(r) => Decision { m: Mobj { state: d.goto, tics: r.tics, ..d.m }, ..d },
            None => d,
        })
        .key_by(|d: Decision| d.m.id)
        .collect()
}

pub fn move_cands(db: &Db, decided: &HashIdx<usize, Decision>) -> HashIdx<(usize, usize), Cand> {
    decided
        .key_by(|d: Decision| d.m.kind)
        .and(&db.t.infos)
        .flat_map(|(d, i): (Decision, Info)| {
            let m = d.m;
            let chase = d.go.map(|a| [0.0, FRAC_PI_4, -FRAC_PI_4, FRAC_PI_2, -FRAC_PI_2, 0.0].map(|o| a + o));
            let fly = (m.flags & MISSILE != 0 && (m.momx != 0.0 || m.momy != 0.0)).then_some(m.ang);
            let base = Cand { who: m.id, idx: 0, x: m.x, y: m.y, z: m.z, radius: i.radius, height: i.height, monster: m.flags & MONSTER != 0, missile: false, stay: false };
            let chase = chase.into_iter().flat_map(move |angs| {
                angs.into_iter().enumerate().map(move |(k, a)| Cand {
                    idx: k,
                    x: m.x + if k == 5 { 0.0 } else { i.speed * a.cos() },
                    y: m.y + if k == 5 { 0.0 } else { i.speed * a.sin() },
                    stay: k == 5,
                    ..base
                })
            });
            let fly = fly.into_iter().map(move |_| Cand { x: m.x + m.momx, y: m.y + m.momy, missile: true, ..base });
            chase.chain(fly)
        })
        .key_by(|c: Cand| (c.who, c.idx))
        .collect()
}

pub fn apply_moves(db: &Db, decided: &HashIdx<usize, Decision>, clips: &HashIdx<usize, Clip>, bodies: &HashIdx<usize, Body>) -> HashIdx<usize, Decision> {
    let hits = decided
        .filt(|d: Decision| d.m.flags & MISSILE != 0 && (d.m.momx != 0.0 || d.m.momy != 0.0))
        .cross(bodies)
        .filt(|(d, b): (Decision, Body)| {
            let m = d.m;
            b.shootable && b.id != m.source && b.id != m.id && (m.x + m.momx - b.x).abs() < b.radius + 6.0 && (m.y + m.momy - b.y).abs() < b.radius + 6.0 && m.z <= b.z + b.height && m.z + 8.0 >= b.z
        })
        .key_by(|(d, _): (Decision, Body)| d.m.id)
        .fold(NONE, |_, (_, b): (Decision, Body)| b.id);
    let moved = decided
        .and(clips.opt())
        .and(hits.opt())
        .key_by(|((d, _), _): ((Decision, Option<Clip>), Option<usize>)| d.m.kind)
        .and(&db.t.infos)
        .map(|(((d, k), hit), i): (((Decision, Option<Clip>), Option<usize>), Info)| {
            let m = d.m;
            let flying = m.flags & MISSILE != 0 && (m.momx != 0.0 || m.momy != 0.0);
            match (flying, k, hit) {
                (true, Some(k), None) => Decision { m: Mobj { x: k.c.x, y: k.c.y, ..m }, ..d },
                (true, _, _) => Decision {
                    m: Mobj { momx: 0.0, momy: 0.0, flags: m.flags & !MISSILE | NOGRAV, ..m },
                    goto: i.death,
                    hit: hit.unwrap_or(NONE),
                    ..d
                },
                (false, Some(k), _) if d.go.is_some() && !k.c.stay => {
                    Decision { m: Mobj { x: k.c.x, y: k.c.y, ang: (k.c.y - m.y).atan2(k.c.x - m.x), ..m }, ..d }
                }
                _ => d,
            }
        });
    goto(db, moved)
}

pub fn monster_shots(decided: &HashIdx<usize, Decision>, tic: u64) -> HashIdx<usize, Shot> {
    decided
        .flat_map(move |d: Decision| {
            let n = match d.attack {
                Attack::Hitscan { pellets, .. } => pellets,
                _ => 0,
            };
            let (mult, modulo) = match d.attack {
                Attack::Hitscan { mult, modulo, .. } => (mult, modulo),
                _ => (0, 1),
            };
            let m = d.m;
            (0..n).map(move |k| {
                let spread = (rnd(tic, m.id, 20 + k as u64) - rnd(tic, m.id, 40 + k as u64)) as f64 * TAU / 4096.0;
                Shot { id: m.id * 8 + k, from: m.id, x: m.x, y: m.y, z: m.z + 32.0, ang: m.ang + spread, range: 2048.0, damage: mult * (rnd(tic, m.id, 60 + k as u64) % modulo + 1) }
            })
        })
        .key_by(|s: Shot| s.id)
        .collect()
}

pub fn damage(impacts: &HashIdx<usize, Impact>, decided: &HashIdx<usize, Decision>, bodies: &HashIdx<usize, Body>, tic: u64) -> Fold<usize, i32> {
    let shots = impacts.filt(|i: Impact| i.target != NONE).map(|i: Impact| (i.target, i.shot.damage)).key_by(|(t, _)| t);
    let melee = decided
        .flat_map(|d: Decision| match d.attack {
            Attack::Melee { damage } => Some((PLAYER, damage)),
            _ => None,
        })
        .key_by(|(t, _)| t);
    let missiles = decided
        .filt(|d: Decision| d.hit != NONE)
        .map(move |d: Decision| (d.hit, 3 * (rnd(tic, d.m.id, 9) % 8 + 1)))
        .key_by(|(t, _)| t);
    let blast = decided
        .filt(|d: Decision| d.explode)
        .cross(bodies)
        .flat_map(|(d, b): (Decision, Body)| {
            let dist = ((d.m.x - b.x).abs().max((d.m.y - b.y).abs()) - b.radius).max(0.0);
            (b.shootable && b.id != d.m.id && dist < 128.0).then_some((b.id, (128.0 - dist) as i32))
        })
        .key_by(|(t, _)| t);
    shots.union(melee).union(missiles).union(blast).map(|(_, d)| d).fold(0, |a: i32, d: i32| a + d)
}

pub fn wound(db: &Db, decided: &HashIdx<usize, Decision>, dmg: &Fold<usize, i32>, tic: u64) -> HashIdx<usize, Decision> {
    let hurt = decided
        .and(dmg.opt())
        .key_by(|(d, _): (Decision, Option<i32>)| d.m.kind)
        .and(&db.t.infos)
        .map(move |((d, h), i): ((Decision, Option<i32>), Info)| {
            let m = d.m;
            match h {
                Some(h) if h > 0 && m.flags & SHOOTABLE != 0 && m.health > 0 => {
                    let health = m.health - h;
                    if health <= 0 {
                        Decision { m: Mobj { health, flags: m.flags & !SHOOTABLE, ..m }, goto: i.death, go: None, attack: Attack::None, explode: false, hit: NONE }
                    } else if rnd(tic, m.id, 77) < i.painchance {
                        Decision { m: Mobj { health, awake: true, ..m }, goto: i.pain, ..d }
                    } else if !m.awake && i.see != 0 {
                        Decision { m: Mobj { health, awake: true, reaction: 0, ..m }, goto: i.see, ..d }
                    } else {
                        Decision { m: Mobj { health, awake: true, ..m }, ..d }
                    }
                }
                _ => d,
            }
        });
    goto(db, hurt)
}

pub fn drops(db: &Db, before: &HashIdx<usize, Decision>, after: &HashIdx<usize, Decision>, tic: u64) -> HashIdx<usize, Mobj> {
    let base = (tic as usize + 1) << 24;
    let reqs = before
        .filt(|d: Decision| d.m.health > 0)
        .key_by(|d: Decision| d.m.id)
        .and(after)
        .filt(|(_, a): (Decision, Decision)| a.m.health <= 0)
        .key_by(|(d, _): (Decision, Decision)| d.m.kind)
        .and(&db.t.infos)
        .filt(|(_, i): ((Decision, Decision), Info)| i.drop != NONE)
        .map(move |((d, _), i): ((Decision, Decision), Info)| Spawn {
            id: base + (1 << 21) + d.m.id,
            kind: i.drop,
            x: d.m.x,
            y: d.m.y,
            z: d.m.z,
            ang: 0.0,
            speed: 0.0,
            source: NONE,
        });
    spawn(db, reqs)
}

pub fn effects(db: &Db, impacts: &HashIdx<usize, Impact>, decided: &HashIdx<usize, Decision>, players: &VecRel<usize, Player>, tic: u64) -> HashIdx<usize, Mobj> {
    let base = (tic as usize + 1) << 24;
    let (puff, blood, ball) = (db.t.puff, db.t.blood, db.t.ball);
    let puffs = impacts.map(move |i: Impact| Spawn {
        id: base + i.shot.id,
        kind: if i.target == NONE { puff } else { blood },
        x: i.x,
        y: i.y,
        z: i.z,
        ang: 0.0,
        speed: 0.0,
        source: NONE,
    });
    let balls = decided
        .filt(|d: Decision| matches!(d.attack, Attack::Ball))
        .key_by(|_| PLAYER)
        .and(players)
        .map(move |(d, p): (Decision, Player)| Spawn {
            id: base + (1 << 22) + d.m.id,
            kind: ball,
            x: d.m.x,
            y: d.m.y,
            z: d.m.z + 32.0,
            ang: (p.y - d.m.y).atan2(p.x - d.m.x),
            speed: 10.0,
            source: d.m.id,
        });
    spawn(db, puffs.key_by(|_| 0usize).union(balls.key_by(|_| 0usize)))
}

pub fn survivors(decided: &HashIdx<usize, Decision>, taken: &Fold<usize, bool>, dropped: &HashIdx<usize, Mobj>, effects: &HashIdx<usize, Mobj>) -> HashIdx<usize, Mobj> {
    decided.map(|d: Decision| d.m).filt(|m: Mobj| m.state != 0).minus(taken).union(dropped).union(effects).collect()
}
