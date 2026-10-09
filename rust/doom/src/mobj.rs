use crate::db::{Db, Info, StateRow};
use crate::info::{fact, Action, AiOut, Attack, HANGING, KEEP, MISSILE, MONSTER, NOGRAV, SHOOTABLE, SOLID};
use crate::rules::{bit, matching};
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
pub struct Decision {
    pub m: Mobj,
    pub goto: usize,
    pub go: Option<f64>,
    pub attack: Attack,
    pub explode: bool,
    pub hit: usize,
}

fn idle(m: Mobj) -> Decision {
    Decision { m, goto: NONE, go: None, attack: Attack::None, explode: false, hit: NONE }
}

fn face(m: Mobj, p: Player) -> f64 {
    (p.y - m.y).atan2(p.x - m.x)
}

fn ai_facts(m: Mobj, i: Info, p: Player, sees: bool, heard: bool, tic: u64) -> u32 {
    use fact::*;
    let dist = (p.x - m.x).hypot(p.y - m.y);
    let ahead = ((face(m, p) - m.ang + PI).rem_euclid(TAU) - PI).abs() <= FRAC_PI_2;
    let d = (dist - 64.0 - 128.0 * (i.melee == 0) as i32 as f64).clamp(0.0, 200.0);
    bit(p.dead, DEAD)
        | bit(sees, SEES)
        | bit(heard, HEARD)
        | bit(ahead, AHEAD)
        | bit(dist < 64.0, CLOSE)
        | bit(dist < 60.0, MELEE_RANGE)
        | bit(i.melee != 0, HAS_MELEE)
        | bit(i.missile != 0, HAS_MISSILE)
        | bit(m.reaction <= 1, REACTION0)
        | bit(rnd(tic, m.id, 1) as f64 >= d, ROLL)
}

#[derive(Clone, Copy)]
struct Ctx {
    d: Decision,
    face: f64,
    damage: i32,
    clip: Option<Clip>,
    hit: Option<usize>,
}

fn ctx(d: Decision) -> Ctx {
    Ctx { d, face: d.m.ang, damage: 0, clip: None, hit: None }
}

fn apply(c: Ctx, o: AiOut, goto: Option<usize>) -> Decision {
    let (d, m) = (c.d, c.d.m);
    let (x, y) = c.clip.filter(|_| o.to_clip).map_or((m.x, m.y), |k| (k.c.x, k.c.y));
    let ang = if o.face { c.face } else if o.face_move { (y - m.y).atan2(x - m.x) } else { m.ang };
    let reaction = if o.reaction >= 0 { o.reaction } else { (m.reaction - o.reaction_dec as i32).max(0) };
    let cleared = SOLID * o.clear_solid as u32 | SHOOTABLE * o.clear_shootable as u32 | MISSILE * o.stop as u32;
    let flags = m.flags & !cleared | NOGRAV * o.stop as u32;
    let (momx, momy) = if o.stop { (0.0, 0.0) } else { (m.momx, m.momy) };
    let m = Mobj { x, y, ang, reaction, flags, momx, momy, health: m.health - c.damage, awake: if o.awake == 0 { m.awake } else { o.awake > 0 }, ..m };
    Decision {
        m,
        goto: goto.unwrap_or(d.goto),
        go: if o.reset { None } else if o.go { Some(c.face) } else { d.go },
        attack: if o.reset { Attack::None } else if matches!(o.attack, Attack::None) { d.attack } else { o.attack },
        explode: !o.reset && (d.explode || o.explode),
        hit: if o.stop { c.hit.unwrap_or(NONE) } else { d.hit },
    }
}

fn run_rules<Q: Drive<R = (Ctx, usize, u32)>>(db: &Db, rows: Q) -> HashIdx<usize, Decision> {
    let rows: HashIdx<usize, (Ctx, usize, u32)> = rows.key_by(|(c, _, _): (Ctx, usize, u32)| c.d.m.id).collect();
    let chosen = matching(&rows, &db.t.ai_rules, |c: Ctx| c.d.m.id);
    let applied = (&rows)
        .and(chosen.opt())
        .map(|((c, _, _), o): ((Ctx, usize, u32), Option<(Ctx, AiOut)>)| (c, o.map_or(KEEP, |(_, o)| o)))
        .key_by(|(c, o): (Ctx, AiOut)| (c.d.m.kind, o.goto))
        .and((&db.t.info_goto).opt())
        .map(|((c, o), g): ((Ctx, AiOut), Option<usize>)| apply(c, o, g));
    goto(db, applied)
}

pub fn think(
    db: &Db,
    advanced: &HashIdx<usize, (Mobj, Action)>,
    visible: &HashIdx<usize, Sight>,
    heard: &HashIdx<usize, bool>,
    players: &VecRel<usize, Player>,
    tic: u64,
) -> HashIdx<usize, Decision> {
    let rows = advanced
        .and(visible.opt())
        .key_by(|((m, _), _): ((Mobj, Action), Option<Sight>)| m.sector)
        .and(heard.opt())
        .key_by(|(((m, _), _), _): (((Mobj, Action), Option<Sight>), Option<bool>)| m.kind)
        .and(&db.t.infos)
        .key_by(|_| PLAYER)
        .and(players)
        .map(move |(((((m, a), s), h), i), p): (((((Mobj, Action), Option<Sight>), Option<bool>), Info), Player)| {
            (Ctx { face: face(m, p), ..ctx(idle(m)) }, a as usize, ai_facts(m, i, p, s.is_some(), h.is_some(), tic))
        });
    run_rules(db, rows)
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
    use fact::*;
    let hits = decided
        .filt(|d: Decision| d.m.flags & MISSILE != 0 && (d.m.momx != 0.0 || d.m.momy != 0.0))
        .cross(bodies)
        .filt(|(d, b): (Decision, Body)| {
            let m = d.m;
            b.shootable && b.id != m.source && b.id != m.id && (m.x + m.momx - b.x).abs() < b.radius + 6.0 && (m.y + m.momy - b.y).abs() < b.radius + 6.0 && m.z <= b.z + b.height && m.z + 8.0 >= b.z
        })
        .key_by(|(d, _): (Decision, Body)| d.m.id)
        .fold(NONE, |_, (_, b): (Decision, Body)| b.id);
    let rows = decided.and(clips.opt()).and(hits.opt()).map(|((d, k), hit): ((Decision, Option<Clip>), Option<usize>)| {
        let m = d.m;
        let facts = bit(m.flags & MISSILE != 0 && (m.momx != 0.0 || m.momy != 0.0), FLYING)
            | bit(k.is_some(), CLIP_OK)
            | bit(hit.is_some(), HIT)
            | bit(d.go.is_some(), GO)
            | bit(k.is_some_and(|k| k.c.stay), STAY);
        (Ctx { clip: k, hit, ..ctx(d) }, Action::Move as usize, facts)
    });
    run_rules(db, rows)
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
        .flat_map(move |d: Decision| match d.attack {
            Attack::Melee { mult, modulo } => Some((PLAYER, mult * (rnd(tic, d.m.id, 2) % modulo + 1))),
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
    use fact::*;
    let hurt = decided
        .and(dmg)
        .filt(|(d, h): (Decision, i32)| h > 0 && d.m.flags & SHOOTABLE != 0 && d.m.health > 0)
        .key_by(|(d, _): (Decision, i32)| d.m.kind)
        .and(&db.t.infos)
        .map(move |((d, h), i): ((Decision, i32), Info)| {
            let m = d.m;
            let facts = bit(m.health - h <= 0, KILLED) | bit(rnd(tic, m.id, 77) < i.painchance, PAIN_ROLL) | bit(!m.awake, ASLEEP) | bit(i.see != 0, HAS_SEE);
            (Ctx { damage: h, ..ctx(d) }, Action::Hurt as usize, facts)
        });
    let hurt = run_rules(db, hurt);
    decided.minus(&hurt).union(&hurt).collect()
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
