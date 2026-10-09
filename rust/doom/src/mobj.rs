use crate::db::{consts, Db, Info, StateRow};
use crate::info::{fact, Action, AiOut, Attack, ChaseDir, Consts, ATK_BALL, ATK_HITSCAN, ATK_MELEE, ATK_NONE, NO_ATTACK, HANGING, KEEP, MISSILE, MONSTER, NOGRAV, SHOOTABLE, SOLID};
use crate::rules::{bit, matching};
use crate::physics::{locate, rnd, Body, Cand, Clip, Impact, Shot, Sight};
use crate::player::{Player, PLAYER};
use crate::wad::{Sector, Thing, MTF_MEDIUM, MTF_MULTI, NONE};
use prela::engine::*;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

#[derive(Clone, Copy, Debug)]
// A live map object (monster, item, decoration, projectile, effect) and its per-tic state.
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
// A request to create a new mobj.
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

// Turns spawn requests into mobjs in their type's spawn state.
pub fn spawn<Q: Drive<R = Spawn>>(db: &Db, reqs: Q) -> HashIdx<usize, Mobj> {
    let reaction = consts(db).reaction;
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
            reaction,
            sector: NONE,
            source: s.source,
        })
        .key_by(|m: Mobj| m.id)
        .collect()
}

// Spawns the map's THINGS that appear on this skill level and aren't multiplayer-only.
pub fn spawn_things(db: &Db, sectors: &VecRel<usize, Sector>) -> HashIdx<usize, Mobj> {
    let reqs = (&db.things)
        .and(Same::<usize>::new())
        .filt(|(t, _): (Thing, usize)| t.flags & MTF_MEDIUM != 0 && t.flags & MTF_MULTI == 0)
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

// Finds each mobj's sector and sets its z (floor, ceiling for hanging things, unchanged for flyers).
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

// Collision/hit boxes for solid or shootable mobjs, plus the player.
pub fn bodies(db: &Db, players: &VecRel<usize, Player>, mobjs: &HashIdx<usize, Mobj>) -> HashIdx<usize, Body> {
    let k = consts(db);
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
                .map(move |p: Player| Body { id: PLAYER, x: p.x, y: p.y, z: p.z, radius: k.player_radius, height: k.player_height, solid: !p.dead, shootable: !p.dead })
                .key_by(|b: Body| b.id),
        )
        .collect()
}

// Counts down each mobj's state timer, steps to the next state when it runs out, and returns that state's action.
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

// Eye-to-eye segments from each looking/chasing monster to the player, for the line-of-sight check.
pub fn sight_pairs(db: &Db, advanced: &HashIdx<usize, (Mobj, Action)>, players: &VecRel<usize, Player>) -> HashIdx<usize, Sight> {
    let z = consts(db).sight_z;
    advanced
        .filt(|(_, a): (Mobj, Action)| matches!(a, Action::Look | Action::Chase))
        .key_by(|_| PLAYER)
        .and(players)
        .map(move |((m, _), p): ((Mobj, Action), Player)| (m.id, ((m.x, m.y, m.z + z), (p.x, p.y, p.z + z))))
        .key_by(|(id, _)| id)
        .map(|(_, s)| s)
        .collect()
}

#[derive(Clone, Copy, Debug)]
// A mobj plus what it decided this tic: state to jump to, direction to walk, attack, explosion, projectile hit.
pub struct Decision {
    pub m: Mobj,
    pub goto: usize,
    pub go: Option<f64>,
    pub attack: Attack,
    pub explode: bool,
    pub hit: usize,
}

// A decision that changes nothing.
fn idle(m: Mobj) -> Decision {
    Decision { m, goto: NONE, go: None, attack: NO_ATTACK, explode: false, hit: NONE }
}

// Angle from the mobj to the player.
fn face(m: Mobj, p: Player) -> f64 {
    (p.y - m.y).atan2(p.x - m.x)
}

// Packs the conditions the AI rules test (sees player, in range, random roll...) into a bitmask.
fn ai_facts(m: Mobj, i: Info, p: Player, sees: bool, heard: bool, tic: u64, k: Consts) -> u32 {
    use fact::*;
    let dist = (p.x - m.x).hypot(p.y - m.y);
    let ahead = ((face(m, p) - m.ang + PI).rem_euclid(TAU) - PI).abs() <= FRAC_PI_2;
    let d = (dist - k.missile_near - k.missile_no_melee * (i.melee == 0) as i32 as f64).clamp(0.0, k.missile_far);
    bit(p.dead, DEAD)
        | bit(sees, SEES)
        | bit(heard, HEARD)
        | bit(ahead, AHEAD)
        | bit(dist < k.close_range, CLOSE)
        | bit(dist < k.melee_range, MELEE_RANGE)
        | bit(i.melee != 0, HAS_MELEE)
        | bit(i.missile != 0, HAS_MISSILE)
        | bit(m.reaction <= 1, REACTION0)
        | bit(rnd(tic, m.id, 1) as f64 >= d, ROLL)
}

#[derive(Clone, Copy)]
// Working row for rule matching: the decision so far plus inputs the chosen rule may use.
struct Ctx {
    d: Decision,
    face: f64,
    damage: i32,
    clip: Option<Clip>,
    hit: Option<usize>,
}

// Starts a Ctx from a decision with no extra inputs.
fn ctx(d: Decision) -> Ctx {
    Ctx { d, face: d.m.ang, damage: 0, clip: None, hit: None }
}

// Applies a chosen rule's output to the mobj and its decision.
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
        attack: if o.reset { NO_ATTACK } else if o.attack.kind == ATK_NONE { d.attack } else { o.attack },
        explode: !o.reset && (d.explode || o.explode),
        hit: if o.stop { c.hit.unwrap_or(NONE) } else { d.hit },
    }
}

// Picks the best matching AI rule per mobj, applies it, and resolves its state jump.
fn run_rules<Q: Drive<R = (Ctx, usize, u32)>>(db: &Db, rows: Q) -> HashIdx<usize, Decision> {
    let rows: HashIdx<usize, (Ctx, usize, u32)> = rows.key_by(|(c, _, _): (Ctx, usize, u32)| c.d.m.id).collect();
    // Best rule per mobj, looked up by action and fact bits.
    let chosen = matching(&rows, &db.t.ai_rules, |c: Ctx| c.d.m.id);
    // Apply it (KEEP if none matched) and look up the target state for this mobj type.
    let applied = (&rows)
        .and(chosen.opt())
        .map(|((c, _, _), o): ((Ctx, usize, u32), Option<(Ctx, AiOut)>)| (c, o.map_or(KEEP, |(_, o)| o)))
        .key_by(|(c, o): (Ctx, AiOut)| (c.d.m.kind, o.goto))
        .and((&db.t.info_goto).opt())
        .map(|((c, o), g): ((Ctx, AiOut), Option<usize>)| apply(c, o, g));
    goto(db, applied)
}

// Monster AI for this tic: look for the player, chase, choose attacks.
pub fn think(
    db: &Db,
    advanced: &HashIdx<usize, (Mobj, Action)>,
    visible: &HashIdx<usize, Sight>,
    heard: &HashIdx<usize, bool>,
    players: &VecRel<usize, Player>,
    tic: u64,
) -> HashIdx<usize, Decision> {
    let k = consts(db);
    // Join each mobj with its sight/hearing result, type info and the player, then compute its fact bits.
    let rows = advanced
        .and(visible.opt())
        .key_by(|((m, _), _): ((Mobj, Action), Option<Sight>)| m.sector)
        .and(heard.opt())
        .key_by(|(((m, _), _), _): (((Mobj, Action), Option<Sight>), Option<bool>)| m.kind)
        .and(&db.t.infos)
        .key_by(|_| PLAYER)
        .and(players)
        .map(move |(((((m, a), s), h), i), p): (((((Mobj, Action), Option<Sight>), Option<bool>), Info), Player)| {
            (Ctx { face: face(m, p), ..ctx(idle(m)) }, a as usize, ai_facts(m, i, p, s.is_some(), h.is_some(), tic, k))
        });
    run_rules(db, rows)
}

// Puts decisions that requested a state jump into that state.
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

// Candidate positions: one per chase direction (the last is stay put), or a projectile's next step.
pub fn move_cands(db: &Db, decided: &HashIdx<usize, Decision>) -> HashIdx<(usize, usize), Cand> {
    let base = |m: Mobj, i: Info| Cand { who: m.id, idx: 0, x: m.x, y: m.y, z: m.z, radius: i.radius, height: i.height, monster: m.flags & MONSTER != 0, missile: false, stay: false };
    let infos = || decided.key_by(|d: Decision| d.m.kind).and(&db.t.infos);
    let chase = infos()
        .flat_map(|(d, i): (Decision, Info)| d.go.map(|a| (d.m, i, a)))
        .key_by(|(m, _, _): (Mobj, Info, f64)| m.id)
        .cross(&db.t.chase_dirs)
        .and(Same::<(usize, usize)>::new())
        .map(move |(((m, i, a), c), (_, idx)): (((Mobj, Info, f64), ChaseDir), (usize, usize))| Cand {
            idx,
            x: m.x + c.step * i.speed * (a + c.turn).cos(),
            y: m.y + c.step * i.speed * (a + c.turn).sin(),
            stay: c.stay,
            ..base(m, i)
        });
    let fly = infos()
        .filt(|(d, _): (Decision, Info)| d.m.flags & MISSILE != 0 && (d.m.momx != 0.0 || d.m.momy != 0.0))
        .map(move |(d, i): (Decision, Info)| Cand { x: d.m.x + d.m.momx, y: d.m.y + d.m.momy, missile: true, ..base(d.m, i) });
    chase.key_by(|c: Cand| (c.who, c.idx)).union(fly.key_by(|c: Cand| (c.who, c.idx))).collect()
}

// Applies collision results: moves monsters and projectiles, detonates projectiles that hit something.
pub fn apply_moves(db: &Db, decided: &HashIdx<usize, Decision>, clips: &HashIdx<usize, Clip>, bodies: &HashIdx<usize, Body>) -> HashIdx<usize, Decision> {
    let k = consts(db);
    use fact::*;
    // Projectiles whose next step overlaps a shootable body.
    let hits = decided
        .filt(|d: Decision| d.m.flags & MISSILE != 0 && (d.m.momx != 0.0 || d.m.momy != 0.0))
        .cross(bodies)
        .filt(move |(d, b): (Decision, Body)| {
            let m = d.m;
            b.shootable && b.id != m.source && b.id != m.id && (m.x + m.momx - b.x).abs() < b.radius + k.missile_reach && (m.y + m.momy - b.y).abs() < b.radius + k.missile_reach && m.z <= b.z + b.height && m.z + k.missile_z_reach >= b.z
        })
        .key_by(|(d, _): (Decision, Body)| d.m.id)
        .fold(NONE, |_, (_, b): (Decision, Body)| b.id);
    // Fact bits for the Move rules: flying, found a clear spot, hit, wants to walk, stayed put.
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

// Hitscan bullets fired by monsters this tic, with random spread and damage.
pub fn monster_shots(db: &Db, decided: &HashIdx<usize, Decision>, tic: u64) -> HashIdx<usize, Shot> {
    let k = consts(db);
    decided
        .filt(|d: Decision| d.attack.kind == ATK_HITSCAN)
        .expand(|d: Decision| (0, d.attack.pellets))
        .map(move |(d, n): (Decision, usize)| {
            let (m, a) = (d.m, d.attack);
            let spread = (rnd(tic, m.id, 20 + n as u64) - rnd(tic, m.id, 40 + n as u64)) as f64 * TAU / k.hitscan_spread_div;
            Shot { id: m.id * 8 + n, from: m.id, x: m.x, y: m.y, z: m.z + k.shot_z, ang: m.ang + spread, range: k.attack_range, damage: a.mult * (rnd(tic, m.id, 60 + n as u64) % a.modulo + 1) }
        })
        .key_by(|s: Shot| s.id)
        .collect()
}

// Total damage per target this tic from bullets, melee, projectile hits and explosions.
pub fn damage(db: &Db, impacts: &HashIdx<usize, Impact>, decided: &HashIdx<usize, Decision>, bodies: &HashIdx<usize, Body>, tic: u64) -> Fold<usize, i32> {
    // Bullets that hit a body.
    let shots = impacts.filt(|i: Impact| i.target != NONE).map(|i: Impact| (i.target, i.shot.damage)).key_by(|(t, _)| t);
    // Monster melee attacks (always on the player).
    let melee = decided
        .filt(|d: Decision| d.attack.kind == ATK_MELEE)
        .map(move |d: Decision| (PLAYER, d.attack.mult * (rnd(tic, d.m.id, 2) % d.attack.modulo + 1)))
        .key_by(|(t, _)| t);
    // Projectile direct hits.
    let missiles = decided
        .filt(|d: Decision| d.hit != NONE)
        .key_by(|d: Decision| d.m.kind)
        .and(&db.t.infos)
        .map(move |(d, i): (Decision, Info)| (d.hit, i.damage * (rnd(tic, d.m.id, 9) % 8 + 1)))
        .key_by(|(t, _)| t);
    // Explosion splash, falling off with distance up to the blast radius.
    let r = consts(db).blast_radius;
    let blast = decided
        .filt(|d: Decision| d.explode)
        .cross(bodies)
        .flat_map(move |(d, b): (Decision, Body)| {
            let dist = ((d.m.x - b.x).abs().max((d.m.y - b.y).abs()) - b.radius).max(0.0);
            (b.shootable && b.id != d.m.id && dist < r).then_some((b.id, (r - dist) as i32))
        })
        .key_by(|(t, _)| t);
    // Sum per target.
    shots.union(melee).union(missiles).union(blast).map(|(_, d)| d).fold(0, |a: i32, d: i32| a + d)
}

// Applies damage to mobjs and runs the Hurt rules (die, pain, wake up).
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
    // Replace hurt mobjs, keep the rest.
    decided.minus(&hurt).union(&hurt).collect()
}

// Spawns items dropped by monsters that died this tic.
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

// Spawns bullet puffs/blood at impacts and imp fireballs.
pub fn effects(db: &Db, impacts: &HashIdx<usize, Impact>, decided: &HashIdx<usize, Decision>, players: &VecRel<usize, Player>, tic: u64) -> HashIdx<usize, Mobj> {
    let base = (tic as usize + 1) << 24;
    let (puff, blood, ball) = (db.t.puff, db.t.blood, db.t.ball);
    let shot_z = consts(db).shot_z;
    // Puff on walls, blood on bodies.
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
    // Imp fireballs aimed at the player.
    let balls = decided
        .filt(|d: Decision| d.attack.kind == ATK_BALL)
        .key_by(|_| PLAYER)
        .and(players)
        .key_by(move |_| ball)
        .and(&db.t.infos)
        .map(move |((d, p), i): ((Decision, Player), Info)| Spawn {
            id: base + (1 << 22) + d.m.id,
            kind: ball,
            x: d.m.x,
            y: d.m.y,
            z: d.m.z + shot_z,
            ang: (p.y - d.m.y).atan2(p.x - d.m.x),
            speed: i.speed,
            source: d.m.id,
        });
    spawn(db, puffs.key_by(|_| 0usize).union(balls.key_by(|_| 0usize)))
}

// Next tic's mobjs: removed and picked-up ones dropped, new drops and effects added.
pub fn survivors(decided: &HashIdx<usize, Decision>, taken: &Fold<usize, bool>, dropped: &HashIdx<usize, Mobj>, effects: &HashIdx<usize, Mobj>) -> HashIdx<usize, Mobj> {
    decided.map(|d: Decision| d.m).filt(|m: Mobj| m.state != 0).minus(taken).union(dropped).union(effects).collect()
}
