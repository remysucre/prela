use crate::db::{goto_key, Db};
use crate::info::{fact, Action, ATK_BALL, ATK_HITSCAN, ATK_MELEE, ATK_NONE, G_NONE, HANGING, MISSILE, MONSTER, NOGRAV, SHOOTABLE, SOLID};
use crate::physics::{body_of, locate, rnd, slot_key, Body, Cand, Clip, Impact, Shot, Sight};
use crate::player::PLAYER;
use crate::rules::{bit, matching};
use crate::schema::{eff, one, rc, AiRule, AnimState, Mobj, Player, Sector, SectorState, Thing, ThingType, C};
use crate::wad::{MTF_MEDIUM, MTF_MULTI, NONE};
use prela::engine::*;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

// A request to create a mobj: uid, type, x, y, z, angle, speed, source uid.
pub type Spawn = (usize, Row<ThingType>, f64, f64, f64, f64, f64, usize);

// What each mobj decided this tic, beside its updated columns. Columns cover every id;
// `live` marks the mobjs still in play.
#[derive(Clone)]
pub struct Plan {
    pub m: Mobj,
    pub live: C<Mobj, bool>,
    // State asked for this tic (NONE = none), re-applied whenever rules run.
    pub goto: C<Mobj, Row<AnimState>>,
    // Direction to walk this tic.
    pub go: C<Mobj, Option<f64>>,
    // Attack this tic: kind (ATK_*), bullets, damage mult * (1..=modulo).
    pub atk: C<Mobj, u8>,
    pub pellets: C<Mobj, usize>,
    pub mult: C<Mobj, i32>,
    pub modulo: C<Mobj, i32>,
    pub explode: C<Mobj, bool>,
    // Body id a projectile hit (NONE = none).
    pub hit: C<Mobj, usize>,
}

// Ids of live mobjs.
fn alive(live: &C<Mobj, bool>) -> impl Drive<D = Row<Mobj>, R = bool> + Probe + '_ {
    (&**live).filt(|l: bool| l)
}

// A constant column over the mobjs.
fn fill<T: Copy + 'static>(m: &Mobj, v: T) -> C<Mobj, T> {
    rc((&m.id).map(move |_| v))
}

// Replace some rows of a column.
fn upd<T: Copy + 'static, Q: Drive<D = Row<Mobj>, R = T> + Probe>(old: &C<Mobj, T>, new: Q) -> C<Mobj, T> {
    rc((&**old).minus(&new).union(&new))
}

// Each mobj's body id.
fn bid(m: &Mobj) -> impl Drive<D = Row<Mobj>, R = usize> + Probe + '_ {
    (&m.id).map(|i: Row<Mobj>| body_of(i.0))
}

// One column of the next tic's mobjs. Rows are the kept old ids, then spawn key k at row
// `base + k`; `rank` gives each placed row its new id.
fn carry<T: Copy + 'static, Q: Drive<D = usize, R = T>>(keep: &MultiRel<Row<Mobj>, bool>, base: usize, rank: &MultiRel<usize, Row<Mobj>>, old: &C<Mobj, T>, new: Q) -> C<Mobj, T> {
    let kept = keep.and(&**old).and(Same::<Row<Mobj>>::new()).key_by(|(_, i): ((bool, T), Row<Mobj>)| i.0).map(|((_, v), _)| v);
    let spawned = new.and(Same::<usize>::new()).key_by(move |(_, k): (T, usize)| base + k).map(|(v, _)| v);
    rc(kept.union(spawned).and(rank).key_by(|(_, i)| i).map(|(v, _)| v))
}

// The next tic's mobjs: kept rows of `m` plus new spawns (by spawn key), each placed in its
// sector at its floor (or ceiling, or flying height), renumbered 0..n in uid order. Mobjs
// outside the map drop out.
fn assemble(db: &Db, s: &SectorState, m: Option<&Mobj>, keep: &MultiRel<Row<Mobj>, bool>, spawns: &MultiRel<usize, Spawn>) -> Mobj {
    let (tt, st) = (&db.t.ttype, &db.t.state);
    let reaction = one(&db.t.k.reaction);
    let none = Mobj {
        id: Universe::new(0),
        uid: rc(VecRel::<Row<Mobj>, usize>::new(vec![])),
        kind: rc(VecRel::<Row<Mobj>, Row<ThingType>>::new(vec![])),
        x: rc(VecRel::<Row<Mobj>, f64>::new(vec![])),
        y: rc(VecRel::<Row<Mobj>, f64>::new(vec![])),
        z: rc(VecRel::<Row<Mobj>, f64>::new(vec![])),
        ang: rc(VecRel::<Row<Mobj>, f64>::new(vec![])),
        momx: rc(VecRel::<Row<Mobj>, f64>::new(vec![])),
        momy: rc(VecRel::<Row<Mobj>, f64>::new(vec![])),
        state: rc(VecRel::<Row<Mobj>, Row<AnimState>>::new(vec![])),
        tics: rc(VecRel::<Row<Mobj>, i32>::new(vec![])),
        health: rc(VecRel::<Row<Mobj>, i32>::new(vec![])),
        flags: rc(VecRel::<Row<Mobj>, u32>::new(vec![])),
        awake: rc(VecRel::<Row<Mobj>, bool>::new(vec![])),
        reaction: rc(VecRel::<Row<Mobj>, i32>::new(vec![])),
        sector: rc(VecRel::<Row<Mobj>, Row<Sector>>::new(vec![])),
        source: rc(VecRel::<Row<Mobj>, usize>::new(vec![])),
    };
    let m = m.unwrap_or(&none);
    let base = m.id.n;
    // A per-row value from the kept rows' column and the spawns.
    let rows = |old: &C<Mobj, (f64, f64)>, new: fn(Spawn) -> (f64, f64)| -> MultiRel<usize, (f64, f64)> {
        keep.and(&**old)
            .and(Same::<Row<Mobj>>::new())
            .key_by(|(_, i): ((bool, (f64, f64)), Row<Mobj>)| i.0)
            .map(|((_, v), _)| v)
            .union(spawns.map(new).and(Same::<usize>::new()).key_by(move |(_, k): ((f64, f64), usize)| base + k).map(|(v, _)| v))
            .collect()
    };
    // Where each row is, then its sector.
    let pts = rows(&rc((&*m.x).and(&*m.y)), |s| (s.2, s.3));
    let sector: MultiRel<usize, Row<Sector>> = locate(db, &pts).collect();
    // Each row's uid, the order the placed rows are numbered in.
    let uid: MultiRel<usize, usize> = keep
        .and(&*m.uid)
        .and(Same::<Row<Mobj>>::new())
        .key_by(|(_, i): ((bool, usize), Row<Mobj>)| i.0)
        .map(|((_, u), _)| u)
        .union(spawns.map(|s: Spawn| s.0).and(Same::<usize>::new()).key_by(move |(_, k): (usize, usize)| base + k).map(|(u, _)| u))
        .collect();
    let rank: MultiRel<usize, Row<Mobj>> = (&sector)
        .and(Same::<usize>::new())
        .key_by(|_| 0usize)
        .map(|(_, r)| r)
        .window(row_number, &uid, asc)
        .map(|n: i64| Row::<Mobj>::new(n as usize - 1))
        .collect();
    let n = (&rank).fold_flat(0, |n: usize, _| n + 1);
    let sp = |f: fn(Spawn) -> f64| spawns.map(f);
    let kind = || spawns.map(|s: Spawn| s.1);
    let flags = carry(keep, base, &rank, &m.flags, kind().select(&tt.flags));
    let kinds = carry(keep, base, &rank, &m.kind, kind());
    let z = carry(keep, base, &rank, &m.z, sp(|s| s.4));
    let sector = rc((&sector).and(&rank).key_by(|(_, i)| i).map(|(s, _)| s));
    // Floor for walkers, ceiling for hanging things, unchanged for flyers.
    let z = rc((&*z)
        .and(&*flags)
        .and((&*kinds).select(&tt.height))
        .and((&*sector).select((&*s.floor).and(&*s.ceil)))
        .map(|(((z, f), h), (floor, ceil)): (((f64, u32), f64), (f64, f64))| if f & (MISSILE | NOGRAV) != 0 { z } else if f & HANGING != 0 { ceil - h } else { floor }));
    let spawn_state = || kind().select(&tt.spawn);
    Mobj {
        id: Universe::new(n),
        uid: carry(keep, base, &rank, &m.uid, spawns.map(|s: Spawn| s.0)),
        x: carry(keep, base, &rank, &m.x, sp(|s| s.2)),
        y: carry(keep, base, &rank, &m.y, sp(|s| s.3)),
        ang: carry(keep, base, &rank, &m.ang, sp(|s| s.5)),
        momx: carry(keep, base, &rank, &m.momx, sp(|s| s.6 * s.5.cos())),
        momy: carry(keep, base, &rank, &m.momy, sp(|s| s.6 * s.5.sin())),
        state: carry(keep, base, &rank, &m.state, spawn_state()),
        tics: carry(keep, base, &rank, &m.tics, spawn_state().select(&st.tics)),
        health: carry(keep, base, &rank, &m.health, kind().select(&tt.health)),
        awake: carry(keep, base, &rank, &m.awake, spawns.map(|_| false)),
        reaction: carry(keep, base, &rank, &m.reaction, spawns.map(move |_| reaction)),
        source: carry(keep, base, &rank, &m.source, spawns.map(|s: Spawn| s.7)),
        kind: kinds,
        flags,
        z,
        sector,
    }
}

// The map's THINGS that appear on this skill level and aren't multiplayer-only, as mobjs.
pub fn spawn_things(db: &Db, s: &SectorState) -> Mobj {
    let th = &db.thing;
    let reqs: MultiRel<usize, Spawn> = (&th.flags)
        .filt(|f: i64| f & MTF_MEDIUM != 0 && f & MTF_MULTI == 0)
        .and((&th.x).and(&th.y).and(&th.angle))
        // Editor number -> thing type; unknown numbers drop out here.
        .and((&th.kind).select(&db.t.ednum_type))
        .and(Same::<Row<Thing>>::new())
        .map(|(((_, ((x, y), a)), kind), t): (((i64, ((f64, f64), f64)), Row<ThingType>), Row<Thing>)| (t.0, (t.0 + 1, kind, x, y, 0.0, a.to_radians(), 0.0, NONE)))
        .key_by(|(t, _)| t)
        .map(|(_, s)| s)
        .collect();
    assemble(db, s, None, &MultiRel::from_pairs(0, []), &reqs)
}

// Collision/hit boxes for live solid or shootable mobjs, plus the player, by body id.
pub fn bodies(db: &Db, p: &Player, m: &Mobj, live: &C<Mobj, bool>) -> MultiRel<usize, Body> {
    let (k, tt) = (&db.t.k, &db.t.ttype);
    let (pr, ph) = (one(&k.player_radius), one(&k.player_height));
    alive(live)
        .and(&*m.flags)
        .filt(|(_, f): (bool, u32)| f & (SOLID | SHOOTABLE) != 0)
        .and(bid(m).and(&*m.uid).and(&*m.x).and(&*m.y).and(&*m.z))
        .and((&*m.kind).select((&tt.radius).and(&tt.height)))
        .map(|(((_, f), ((((b, u), x), y), z)), (r, h)): (((bool, u32), ((((usize, usize), f64), f64), f64)), (f64, f64))| (b, u, x, y, z, r, h, f & SOLID != 0, f & SHOOTABLE != 0))
        .key_by(|b: Body| b.0)
        .union(
            (&*p.x)
                .and(&*p.y)
                .and(&*p.z)
                .and(&*p.dead)
                .map(move |(((x, y), z), dead): (((f64, f64), f64), bool)| (PLAYER, PLAYER, x, y, z, pr, ph, !dead, !dead))
                .key_by(|b: Body| b.0),
        )
        .collect()
}

// Counts down each mobj's state timer and steps to the next state when it runs out.
// Returns the updated mobjs, the action of any state just entered, and which mobjs are still live
// (state 0 = gone).
pub fn advance(db: &Db, m: &Mobj) -> (Mobj, C<Mobj, Action>, C<Mobj, bool>) {
    let st = &db.t.state;
    // -1 tics = stay forever; otherwise count down, and at 1 take the next state.
    let next = || (&*m.tics).and((&*m.state).select(&st.next)).map(|(t, n): (i32, Row<AnimState>)| if t == -1 || t > 1 { Row::new(NONE) } else { n });
    let t: VecRel<Row<Mobj>, (Row<AnimState>, i32, Action)> = (&*m.state)
        .and(&*m.tics)
        .and(next())
        .and(next().select((&st.tics).and(&st.action)).opt())
        .map(|(((s, t), n), e): (((Row<AnimState>, i32), Row<AnimState>), Option<(i32, Action)>)| match e {
            Some((nt, a)) => (n, nt, a),
            None => (s, if t > 1 { t - 1 } else { t }, Action::None),
        })
        .collect();
    type T = (Row<AnimState>, i32, Action);
    let m = Mobj { state: rc((&t).map(|v: T| v.0)), tics: rc((&t).map(|v: T| v.1)), ..m.clone() };
    (m, rc((&t).map(|v: T| v.2)), rc((&t).map(|v: T| v.0.0 != 0)))
}

// Eye-to-eye segments from each looking/chasing monster to the player, for the line-of-sight check.
pub fn sight_pairs(db: &Db, m: &Mobj, action: &C<Mobj, Action>, live: &C<Mobj, bool>, p: &Player) -> MultiRel<Row<Mobj>, Sight> {
    let z = one(&db.t.k.sight_z);
    let pp = (&*p.x).and(&*p.y).and(&*p.z);
    alive(live)
        .and(&**action)
        .filt(|(_, a): (bool, Action)| matches!(a, Action::Look | Action::Chase))
        .and((&*m.x).and(&*m.y).and(&*m.z))
        .and(Same::<Row<Mobj>>::new())
        .key_by(|_| Row::<Player>::new(0))
        .and(pp)
        .map(move |(((_, ((mx, my), mz)), id), ((px, py), pz)) : ((((bool, Action), ((f64, f64), f64)), Row<Mobj>), ((f64, f64), f64))| (id, ((mx, my, mz + z), (px, py, pz + z))))
        .key_by(|(id, _)| id)
        .map(|(_, s)| s)
        .collect()
}

// Angle from (x, y) to (px, py).
fn face(x: f64, y: f64, px: f64, py: f64) -> f64 {
    (py - y).atan2(px - x)
}

// Packs the conditions the AI rules test (sees player, in range, random roll...) into a bitmask.
// m = (uid, x, y, ang, reaction); i = (has melee, has missile); p = (x, y, dead).
#[allow(clippy::too_many_arguments)]
fn ai_facts((uid, x, y, ang, reaction): (usize, f64, f64, f64, i32), (melee, missile): (bool, bool), (px, py, dead): (f64, f64, bool), sees: bool, heard: bool, tic: u64, (close, mrange, near, no_melee, far): (f64, f64, f64, f64, f64)) -> u32 {
    use fact::*;
    let dist = (px - x).hypot(py - y);
    // Player within 90° of the monster's facing.
    let ahead = ((face(x, y, px, py) - ang + PI).rem_euclid(TAU) - PI).abs() <= FRAC_PI_2;
    // Missile odds: the farther away, the higher the roll must be (vanilla P_CheckMissileRange).
    let d = (dist - near - no_melee * (!melee) as i32 as f64).clamp(0.0, far);
    bit(dead, DEAD)
        | bit(sees, SEES)
        | bit(heard, HEARD)
        | bit(ahead, AHEAD)
        | bit(dist < close, CLOSE)
        | bit(dist < mrange, MELEE_RANGE)
        | bit(melee, HAS_MELEE)
        | bit(missile, HAS_MISSILE)
        | bit(reaction <= 1, REACTION0)
        | bit(rnd(tic, uid, 1) as f64 >= d, ROLL)
}

// The mobj fields rules change: x, y, ang, reaction, flags, momx, momy, health, awake.
type Body9 = (f64, f64, f64, i32, u32, f64, f64, i32, bool);
// The decision fields: goto, go, attack (kind, pellets, mult, modulo), explode, hit.
type Dec = (Row<AnimState>, Option<f64>, (u8, usize, i32, i32), bool, usize);
// Extra inputs a rule may use: facing toward the target, damage taken, the chosen move, the body hit.
type Ctx = (f64, i32, Option<(f64, f64)>, Option<usize>);
// A rule's outputs: wake, reaction, attack, effects (eff::* bits).
type Out = (i8, i32, (u8, usize, i32, i32), u32);

const NO_ATTACK: (u8, usize, i32, i32) = (ATK_NONE, 0, 0, 1);
const KEEP: Out = (0, -1, NO_ATTACK, 0);

// Applies a chosen rule's output to the mobj and its decision; `goto` = the rule's target state, if any.
fn apply(m: Body9, d: Dec, (face, damage, clip, hit): Ctx, (awake_o, reaction_o, attack_o, e): Out, goto: Option<Row<AnimState>>) -> (Body9, Dec) {
    let (x0, y0, ang, reaction, flags, momx, momy, health, awake) = m;
    let on = |b: u32| e & b != 0;
    let (x, y) = clip.filter(|_| on(eff::TO_CLIP)).unwrap_or((x0, y0));
    let ang = if on(eff::FACE) { face } else if on(eff::FACE_MOVE) { (y - y0).atan2(x - x0) } else { ang };
    let reaction = if reaction_o >= 0 { reaction_o } else { (reaction - on(eff::REACTION_DEC) as i32).max(0) };
    // A stopped missile loses MISSILE and gains NOGRAV so it explodes in place.
    let cleared = SOLID * on(eff::CLEAR_SOLID) as u32 | SHOOTABLE * on(eff::CLEAR_SHOOTABLE) as u32 | MISSILE * on(eff::STOP) as u32;
    let flags = flags & !cleared | NOGRAV * on(eff::STOP) as u32;
    let (momx, momy) = if on(eff::STOP) { (0.0, 0.0) } else { (momx, momy) };
    let awake = if awake_o == 0 { awake } else { awake_o > 0 };
    let (goto_d, go, attack, explode, hit_d) = d;
    let reset = on(eff::RESET);
    (
        (x, y, ang, reaction, flags, momx, momy, health - damage, awake),
        (
            goto.unwrap_or(goto_d),
            if reset { None } else if on(eff::GO) { Some(face) } else { go },
            if reset { NO_ATTACK } else if attack_o.0 == ATK_NONE { attack } else { attack_o },
            !reset && (explode || on(eff::EXPLODE)),
            if on(eff::STOP) { hit.unwrap_or(NONE) } else { hit_d },
        ),
    )
}

// Picks the best matching AI rule for each row (key, facts), applies it with the row's context,
// and enters any state it jumps to. Rows not listed keep their values.
fn run_rules(db: &Db, plan: &Plan, rows: &MultiRel<Row<Mobj>, (usize, u32)>, ctx: &MultiRel<Row<Mobj>, Ctx>) -> Plan {
    let (m, r, st) = (&plan.m, &db.t.ai_rule, &db.t.state);
    // Best rule per mobj, looked up by key and fact bits.
    let chosen: MultiRel<Row<Mobj>, Row<AiRule>> =
        matching(rows.and(Same::<Row<Mobj>>::new()).map(|((k, f), id)| (id, k, f)), r, &db.t.ai_by_key, |id: Row<Mobj>| id, m.id.n).map(|(_, rule)| rule).collect();
    let out = (&chosen)
        .select((&r.awake).and(&r.reaction).and((&r.attack).and(&r.pellets).and(&r.mult).and(&r.modulo)).and(&r.effects))
        .map(|(((a, re), (((k, p), mu), mo)), e): (((i8, i32), (((u8, usize), i32), i32)), u32)| -> Out { (a, re, (k, p, mu, mo), e) });
    // The rule's goto label -> this type's state (e.g. SEE -> TROO_RUN).
    let goto = (&*m.kind).and((&chosen).select(&r.goto).opt()).map(|(k, l): (Row<ThingType>, Option<u8>)| goto_key(k, l.unwrap_or(G_NONE))).select(&db.t.goto).opt();
    let body = (&*m.x)
        .and(&*m.y)
        .and(&*m.ang)
        .and(&*m.reaction)
        .and(&*m.flags)
        .and(&*m.momx)
        .and(&*m.momy)
        .and(&*m.health)
        .and(&*m.awake)
        .map(|((((((((x, y), a), r), f), mx), my), h), aw): ((((((((f64, f64), f64), i32), u32), f64), f64), i32), bool)| -> Body9 { (x, y, a, r, f, mx, my, h, aw) });
    let dec = (&*plan.goto)
        .and(&*plan.go)
        .and((&*plan.atk).and(&*plan.pellets).and(&*plan.mult).and(&*plan.modulo))
        .and(&*plan.explode)
        .and(&*plan.hit)
        .map(|((((g, go), (((k, p), mu), mo)), ex), h): ((((Row<AnimState>, Option<f64>), (((u8, usize), i32), i32)), bool), usize)| -> Dec { (g, go, (k, p, mu, mo), ex, h) });
    let applied: MultiRel<Row<Mobj>, (Body9, Dec)> = rows
        .and(body)
        .and(dec)
        .and(ctx)
        .and(out.opt())
        .and(goto)
        .map(|(((((_, b), d), c), o), g): (((((( usize, u32), Body9), Dec), Ctx), Option<Out>), Option<Row<AnimState>>)| apply(b, d, c, o.unwrap_or(KEEP), g))
        .collect();
    // Enter the requested state, restarting its timer.
    let entered = (&applied).map(|(_, d): (Body9, Dec)| d.0).and((&applied).map(|(_, d): (Body9, Dec)| d.0).select(&st.tics));
    let b = |f: fn((Body9, Dec)) -> f64| (&applied).map(f);
    Plan {
        m: Mobj {
            x: upd(&m.x, b(|v| v.0.0)),
            y: upd(&m.y, b(|v| v.0.1)),
            ang: upd(&m.ang, b(|v| v.0.2)),
            reaction: upd(&m.reaction, (&applied).map(|v: (Body9, Dec)| v.0.3)),
            flags: upd(&m.flags, (&applied).map(|v: (Body9, Dec)| v.0.4)),
            momx: upd(&m.momx, b(|v| v.0.5)),
            momy: upd(&m.momy, b(|v| v.0.6)),
            health: upd(&m.health, (&applied).map(|v: (Body9, Dec)| v.0.7)),
            awake: upd(&m.awake, (&applied).map(|v: (Body9, Dec)| v.0.8)),
            state: upd(&m.state, (&entered).map(|(s, _): (Row<AnimState>, i32)| s)),
            tics: upd(&m.tics, (&entered).map(|(_, t): (Row<AnimState>, i32)| t)),
            ..m.clone()
        },
        goto: upd(&plan.goto, (&applied).map(|v: (Body9, Dec)| v.1.0)),
        go: upd(&plan.go, (&applied).map(|v: (Body9, Dec)| v.1.1)),
        atk: upd(&plan.atk, (&applied).map(|v: (Body9, Dec)| v.1.2.0)),
        pellets: upd(&plan.pellets, (&applied).map(|v: (Body9, Dec)| v.1.2.1)),
        mult: upd(&plan.mult, (&applied).map(|v: (Body9, Dec)| v.1.2.2)),
        modulo: upd(&plan.modulo, (&applied).map(|v: (Body9, Dec)| v.1.2.3)),
        explode: upd(&plan.explode, (&applied).map(|v: (Body9, Dec)| v.1.3)),
        hit: upd(&plan.hit, (&applied).map(|v: (Body9, Dec)| v.1.4)),
        live: plan.live.clone(),
    }
}

// Monster AI for this tic: look for the player, chase, choose attacks.
#[allow(clippy::too_many_arguments)]
pub fn think(db: &Db, m: &Mobj, action: &C<Mobj, Action>, live: &C<Mobj, bool>, seen: &MultiRel<Row<Mobj>, Sight>, s: &SectorState, p: &Player, tic: u64) -> Plan {
    let (k, tt) = (&db.t.k, &db.t.ttype);
    let ranges = (one(&k.close_range), one(&k.melee_range), one(&k.missile_near), one(&k.missile_no_melee), one(&k.missile_far));
    let plan = Plan {
        live: live.clone(),
        goto: fill(m, Row::new(NONE)),
        go: fill(m, None),
        atk: fill(m, ATK_NONE),
        pellets: fill(m, 0),
        mult: fill(m, 0),
        modulo: fill(m, 1),
        explode: fill(m, false),
        hit: fill(m, NONE),
        m: m.clone(),
    };
    let pp = (&*p.x).and(&*p.y).and(&*p.dead);
    // Each live mobj with its action, sight and hearing, type, and the player: (facts, facing toward the player).
    let t: MultiRel<Row<Mobj>, ((usize, u32), Ctx)> = alive(live)
        .and(&**action)
        .and((&*m.uid).and(&*m.x).and(&*m.y).and(&*m.ang).and(&*m.reaction))
        .and(seen.opt())
        .and((&*m.sector).select(&*s.heard))
        .and((&*m.kind).select((&tt.melee).and(&tt.missile)))
        .and(Same::<Row<Mobj>>::new())
        .key_by(|_| Row::<Player>::new(0))
        .and(pp)
        .map(
            move |(((((((_, a), ((((uid, x), y), ang), re)), sees), heard), (melee, missile)), id), ((px, py), dead)) : (
                ((((((bool, Action), ((((usize, f64), f64), f64), i32)), Option<Sight>), bool), (Row<AnimState>, Row<AnimState>)), Row<Mobj>),
                ((f64, f64), bool),
            )| {
                let facts = ai_facts((uid, x, y, ang, re), (melee.0 != 0, missile.0 != 0), (px, py, dead), sees.is_some(), heard, tic, ranges);
                (id, ((a as usize, facts), (face(x, y, px, py), 0, None, None)))
            },
        )
        .key_by(|(id, _)| id)
        .map(|(_, v)| v)
        .collect();
    let rows: MultiRel<Row<Mobj>, (usize, u32)> = (&t).map(|(r, _): ((usize, u32), Ctx)| r).collect();
    let ctx: MultiRel<Row<Mobj>, Ctx> = (&t).map(|(_, c): ((usize, u32), Ctx)| c).collect();
    run_rules(db, &plan, &rows, &ctx)
}

// Candidate positions: one per chase direction (the last is stay put), or a projectile's next step.
pub fn move_cands(db: &Db, plan: &Plan) -> MultiRel<usize, Cand> {
    let (m, tt, cd) = (&plan.m, &db.t.ttype, &db.t.chase);
    let base = || alive(&plan.live).and(bid(m).and(&*m.x).and(&*m.y).and(&*m.z).and(&*m.flags)).and((&*m.kind).select((&tt.radius).and(&tt.height).and(&tt.speed)));
    type B = ((bool, ((((usize, f64), f64), f64), u32)), ((f64, f64), f64));
    // Walkers: one candidate per chase direction, idx = preference order.
    let chase = base()
        .and(&*plan.go)
        .flat_map(|(b, go): (B, Option<f64>)| go.map(|a| (b, a)))
        .cross((&cd.turn).and(&cd.step).and(&cd.stay).and(Same::<Row<crate::schema::ChaseDir>>::new()))
        .map(|((((_, ((((u, x), y), z), f)), ((r, h), speed)), a), (((turn, step), stay), idx)): ((B, f64), (((f64, f64), bool), Row<crate::schema::ChaseDir>))| -> Cand {
            (u, idx.0, x + step * speed * (a + turn).cos(), y + step * speed * (a + turn).sin(), z, r, h, f & MONSTER != 0, false, stay)
        });
    // Flying missiles: a single candidate one momentum step ahead.
    let fly = base()
        .and((&*m.momx).and(&*m.momy))
        .filt(|(((_, (_, f)), _), (mx, my)): (B, (f64, f64))| f & MISSILE != 0 && (mx != 0.0 || my != 0.0))
        .map(|(((_, ((((u, x), y), z), f)), ((r, h), _)), (mx, my)): (B, (f64, f64))| -> Cand { (u, 0, x + mx, y + my, z, r, h, f & MONSTER != 0, true, false) });
    chase.key_by(|c: Cand| slot_key(c.0, c.1)).union(fly.key_by(|c: Cand| slot_key(c.0, c.1))).collect()
}

// Applies collision results: moves monsters and projectiles, detonates projectiles that hit something.
pub fn apply_moves(db: &Db, plan: &Plan, clips: &MultiRel<usize, Clip>, bodies: &MultiRel<usize, Body>) -> Plan {
    use fact::*;
    let k = &db.t.k;
    let (reach, zreach) = (one(&k.missile_reach), one(&k.missile_z_reach));
    let m = &plan.m;
    let flying = || alive(&plan.live).and(&*m.flags).and((&*m.momx).and(&*m.momy)).map(|((_, f), (mx, my)): ((bool, u32), (f64, f64))| f & MISSILE != 0 && (mx != 0.0 || my != 0.0));
    // Projectiles whose next step overlaps a shootable body: missile -> body id of one body hit.
    let hits = flying()
        .filt(|f: bool| f)
        .and((&*m.uid).and(&*m.source).and(&*m.x).and(&*m.y).and(&*m.z).and((&*m.momx).and(&*m.momy)))
        .and(Same::<Row<Mobj>>::new())
        .cross(bodies)
        .filt(move |(((_, (((((u, src), x), y), z), (mx, my))), _), (_, buid, bx, by, bz, br, bh, _, shootable)): (((bool, (((((usize, usize), f64), f64), f64), (f64, f64))), Row<Mobj>), Body)| {
            shootable && buid != src && buid != u && (x + mx - bx).abs() < br + reach && (y + my - by).abs() < br + reach && z <= bz + bh && z + zreach >= bz
        })
        .key_by(|((_, me), _): (((bool, (((((usize, usize), f64), f64), f64), (f64, f64))), Row<Mobj>), Body)| me)
        .map(|(_, b): (((bool, (((((usize, usize), f64), f64), f64), (f64, f64))), Row<Mobj>), Body)| b.0)
        .dense_fold(m.id.n, NONE, |_, b: usize| b);
    let clip = || bid(m).select(clips);
    // Fact bits for the Move rules: flying, found a clear spot, hit, wants to walk, stayed put.
    let t: MultiRel<Row<Mobj>, ((usize, u32), Ctx)> = flying()
        .and(clip().opt())
        .and(hits.opt())
        .and(&*plan.go)
        .and(&*m.ang)
        .and(Same::<Row<Mobj>>::new())
        .map(|(((((fly, c), hit), go), ang), id): (((((bool, Option<Clip>), Option<usize>), Option<f64>), f64), Row<Mobj>)| {
            let facts = bit(fly, FLYING) | bit(c.is_some(), CLIP_OK) | bit(hit.is_some(), HIT) | bit(go.is_some(), GO) | bit(c.is_some_and(|c| c.0.9), STAY);
            (id, ((Action::Move as usize, facts), (ang, 0, c.map(|c| (c.0.2, c.0.3)), hit)))
        })
        .key_by(|(id, _)| id)
        .map(|(_, v)| v)
        .collect();
    let rows: MultiRel<Row<Mobj>, (usize, u32)> = (&t).map(|(r, _): ((usize, u32), Ctx)| r).collect();
    let ctx: MultiRel<Row<Mobj>, Ctx> = (&t).map(|(_, c): ((usize, u32), Ctx)| c).collect();
    run_rules(db, plan, &rows, &ctx)
}

// Hitscan bullets fired by monsters this tic, with random spread and damage (keyed by shooter body id * 8 + pellet).
pub fn monster_shots(db: &Db, plan: &Plan, tic: u64) -> MultiRel<usize, Shot> {
    let k = &db.t.k;
    let (div, range, shot_z) = (one(&k.hitscan_spread_div), one(&k.attack_range), one(&k.shot_z));
    let m = &plan.m;
    alive(&plan.live)
        .and(&*plan.atk)
        .filt(|(_, a): (bool, u8)| a == ATK_HITSCAN)
        .and((&*plan.pellets).and(&*plan.mult).and(&*plan.modulo))
        .and((&*m.uid).and(bid(m)).and(&*m.x).and(&*m.y).and(&*m.z).and(&*m.ang))
        .expand(|((_, ((p, _), _)), _): (((bool, u8), ((usize, i32), i32)), (((((usize, usize), f64), f64), f64), f64))| (0, p))
        .map(move |(((_, ((_, mult), modulo)), (((((u, b), x), y), z), ang)), n): ((((bool, u8), ((usize, i32), i32)), (((((usize, usize), f64), f64), f64), f64)), usize)| {
            let spread = (rnd(tic, u, 20 + n as u64) - rnd(tic, u, 40 + n as u64)) as f64 * TAU / div;
            (slot_key(b, n), (u * 8 + n, b, x, y, z + shot_z, ang + spread, range, mult * (rnd(tic, u, 60 + n as u64) % modulo + 1)))
        })
        .key_by(|(k, _)| k)
        .map(|(_, s)| s)
        .collect()
}

// Total damage per target body id this tic from bullets, melee, projectile hits and explosions.
pub fn damage(db: &Db, impacts: &MultiRel<usize, Impact>, plan: &Plan, bodies: &MultiRel<usize, Body>, tic: u64) -> DenseFold<usize, i32> {
    let (m, tt) = (&plan.m, &db.t.ttype);
    let r = one(&db.t.k.blast_radius);
    // Bullets that hit a body.
    let shots = impacts.filt(|i: Impact| i.1 != NONE).map(|i: Impact| (i.1, i.5)).key_by(|(t, _)| t);
    // Monster melee attacks (always on the player).
    let melee = alive(&plan.live)
        .and(&*plan.atk)
        .filt(|(_, a): (bool, u8)| a == ATK_MELEE)
        .and((&*plan.mult).and(&*plan.modulo).and(&*m.uid))
        .map(move |(_, ((mult, modulo), u)): ((bool, u8), ((i32, i32), usize))| (PLAYER, mult * (rnd(tic, u, 2) % modulo + 1)))
        .key_by(|(t, _)| t);
    // Projectile direct hits.
    let missiles = alive(&plan.live)
        .and(&*plan.hit)
        .filt(|(_, h): (bool, usize)| h != NONE)
        .and((&*m.uid).and((&*m.kind).select(&tt.damage)))
        .map(move |((_, h), (u, d)): ((bool, usize), (usize, i32))| (h, d * (rnd(tic, u, 9) % 8 + 1)))
        .key_by(|(t, _)| t);
    // Explosion splash, falling off with distance up to the blast radius.
    let blast = alive(&plan.live)
        .and(&*plan.explode)
        .filt(|(_, e): (bool, bool)| e)
        .and(bid(m).and(&*m.x).and(&*m.y))
        .cross(bodies)
        .flat_map(move |((_, ((u, x), y)), (id, _, bx, by, _, br, _, _, shootable)): (((bool, bool), ((usize, f64), f64)), Body)| {
            // Box distance (max of |dx|, |dy|) to the body's edge, as in vanilla.
            let dist = ((x - bx).abs().max((y - by).abs()) - br).max(0.0);
            (shootable && id != u && dist < r).then_some((id, (r - dist) as i32))
        })
        .key_by(|(t, _)| t);
    // Sum per target.
    shots.union(melee).union(missiles).union(blast).map(|(_, d)| d).dense_fold(body_of(m.id.n), 0, |a: i32, d: i32| a + d)
}

// Applies damage to mobjs and runs the Hurt rules (die, pain, wake up).
pub fn wound(db: &Db, plan: &Plan, dmg: &DenseFold<usize, i32>, tic: u64) -> Plan {
    use fact::*;
    let (m, tt) = (&plan.m, &db.t.ttype);
    let t: MultiRel<Row<Mobj>, ((usize, u32), Ctx)> = alive(&plan.live)
        .and(bid(m).select(dmg))
        .and((&*m.flags).and(&*m.health).and(&*m.uid).and(&*m.awake).and(&*m.ang))
        .filt(|((_, h), ((((f, health), _), _), _)): ((bool, i32), ((((u32, i32), usize), bool), f64))| h > 0 && f & SHOOTABLE != 0 && health > 0)
        .and((&*m.kind).select((&tt.painchance).and(&tt.see)))
        .and(Same::<Row<Mobj>>::new())
        .map(move |((((_, h), ((((_, health), u), awake), ang)), (pain, see)), id): ((((bool, i32), ((((u32, i32), usize), bool), f64)), (i32, Row<AnimState>)), Row<Mobj>)| {
            let facts = bit(health - h <= 0, KILLED) | bit(rnd(tic, u, 77) < pain, PAIN_ROLL) | bit(!awake, ASLEEP) | bit(see.0 != 0, HAS_SEE);
            (id, ((Action::Hurt as usize, facts), (ang, h, None, None)))
        })
        .key_by(|(id, _)| id)
        .map(|(_, v)| v)
        .collect();
    let rows: MultiRel<Row<Mobj>, (usize, u32)> = (&t).map(|(r, _): ((usize, u32), Ctx)| r).collect();
    let ctx: MultiRel<Row<Mobj>, Ctx> = (&t).map(|(_, c): ((usize, u32), Ctx)| c).collect();
    run_rules(db, plan, &rows, &ctx)
}

// Items dropped by monsters that died this tic, keyed by the monster's id.
pub fn drops(db: &Db, before: &Plan, after: &Plan, tic: u64) -> MultiRel<usize, Spawn> {
    let base = (tic as usize + 1) << 24;
    let (b, a) = (&before.m, &after.m);
    alive(&before.live)
        .and((&*b.health).and(&*a.health))
        .filt(|(_, (hb, ha)): (bool, (i32, i32))| hb > 0 && ha <= 0)
        .and((&*b.kind).select(&db.t.ttype.drop))
        .filt(|(_, d): ((bool, (i32, i32)), Row<ThingType>)| d.0 != NONE)
        .and((&*b.uid).and(&*b.x).and(&*b.y).and(&*b.z))
        .and(Same::<Row<Mobj>>::new())
        .map(move |(((_, kind), (((u, x), y), z)), i): ((((bool, (i32, i32)), Row<ThingType>), (((usize, f64), f64), f64)), Row<Mobj>)| (i.0, (base + (1 << 21) + u, kind, x, y, z, 0.0, 0.0, NONE)))
        .key_by(|(k, _)| k)
        .map(|(_, s): (usize, Spawn)| s)
        .collect()
}

// Bullet puffs and blood at impacts, and imp fireballs. Keyed after the drops: fireball of
// mobj i at n + i, puff of shot key k at 2n + k (n = mobj count).
pub fn effects(db: &Db, impacts: &MultiRel<usize, Impact>, plan: &Plan, p: &Player, tic: u64) -> MultiRel<usize, Spawn> {
    let base = (tic as usize + 1) << 24;
    let (puff, blood, ball) = (db.t.puff, db.t.blood, db.t.ball);
    let shot_z = one(&db.t.k.shot_z);
    let speed = (&db.t.ttype.speed).get(ball).unwrap_or(0.0);
    let m = &plan.m;
    let n = m.id.n;
    // Puff on walls, blood on bodies.
    let puffs = impacts
        .and(Same::<usize>::new())
        .map(move |((_, target, x, y, z, _, sid), k): (Impact, usize)| (2 * n + k, (base + sid, if target == NONE { puff } else { blood }, x, y, z, 0.0, 0.0, NONE)));
    // Imp fireballs aimed at the player.
    let balls = alive(&plan.live)
        .and(&*plan.atk)
        .filt(|(_, a): (bool, u8)| a == ATK_BALL)
        .and((&*m.uid).and(&*m.x).and(&*m.y).and(&*m.z))
        .and(Same::<Row<Mobj>>::new())
        .key_by(|_| Row::<Player>::new(0))
        .and((&*p.x).and(&*p.y))
        .map(move |(((_, (((u, x), y), z)), i), (px, py)): ((((bool, u8), (((usize, f64), f64), f64)), Row<Mobj>), (f64, f64))| {
            (n + i.0, (base + (1 << 22) + u, ball, x, y, z + shot_z, (py - y).atan2(px - x), speed, u))
        });
    puffs.key_by(|(k, _)| k).union(balls.key_by(|(k, _)| k)).map(|(_, s): (usize, Spawn)| s).collect()
}

// Next tic's mobjs: removed and picked-up ones dropped, new drops and effects added.
pub fn survivors(db: &Db, s: &SectorState, plan: &Plan, taken: &DenseFold<Row<Mobj>, bool>, spawns: &MultiRel<usize, Spawn>) -> Mobj {
    let m = &plan.m;
    let keep: MultiRel<Row<Mobj>, bool> = alive(&plan.live)
        .and(&*m.state)
        .filt(|(_, st): (bool, Row<AnimState>)| st.0 != 0)
        .and(taken.opt())
        .filt(|(_, t): ((bool, Row<AnimState>), Option<bool>)| t.is_none())
        .map(|_| true)
        .collect();
    assemble(db, s, Some(m), &keep, spawns)
}
