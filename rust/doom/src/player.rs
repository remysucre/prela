use crate::db::Db;
use crate::info::{PICKUP, wfact};
use crate::physics::{Cand, Clip, Shot, rnd, slot_key};
use crate::rules::{bit, choice_key};
use crate::schema::{
    AmmoKind, AnimState, ArmorClass, C, Mobj, Player, SectorState, Slide, Weapon, one, rc,
};
use crate::wad::NONE;
use prela::engine::*;
use std::f64::consts::TAU;

// The player's body id (mobj m is body m + 1) and the uid it shoots and is hurt as.
pub const PLAYER: usize = 0;

#[derive(Clone, Copy, Default)]
// One tic of player controls, from the host.
pub struct Input {
    pub forward: f64,
    pub strafe: f64,
    pub turn: f64,
    pub look: f64,
    pub use_: bool,
    pub fire: bool,
    pub run: bool,
    pub weapon: Option<usize>,
}

// The player's row id.
fn me(p: &Player) -> impl Drive<D = Row<Player>, R = usize> + Probe + '_ {
    (&p.id).map(|_| PLAYER)
}

// Turns and accelerates from input, with friction; dead players just slide to a stop.
pub fn moved(db: &Db, p: &Player, inp: Input) -> Player {
    let k = &db.t.k;
    let (friction, turn_run, turn_walk, run_thrust, walk_thrust, strafe_scale) = (
        one(&k.friction),
        one(&k.turn_run),
        one(&k.turn_walk),
        one(&k.run_thrust),
        one(&k.walk_thrust),
        one(&k.strafe_scale),
    );
    let t: VecRel<Row<Player>, (f64, f64, f64)> = (&*p.ang)
        .and(&*p.momx)
        .and(&*p.momy)
        .and(&*p.dead)
        .map(
            move |(((ang, momx), momy), dead): (((f64, f64), f64), bool)| {
                if dead {
                    return (ang, momx * friction, momy * friction);
                }
                let ang = ang + inp.turn * if inp.run { turn_run } else { turn_walk } + inp.look;
                // Forward thrust along the facing, strafe thrust 90° to the right of it.
                let (s, c) = ang.sin_cos();
                let speed = if inp.run { run_thrust } else { walk_thrust };
                let (fwd, side) = (inp.forward * speed, inp.strafe * speed * strafe_scale);
                (
                    ang,
                    (momx + fwd * c + side * s) * friction,
                    (momy + fwd * s - side * c) * friction,
                )
            },
        )
        .collect();
    Player {
        ang: rc((&t).map(|v: (f64, f64, f64)| v.0)),
        momx: rc((&t).map(|v: (f64, f64, f64)| v.1)),
        momy: rc((&t).map(|v: (f64, f64, f64)| v.2)),
        ..p.clone()
    }
}

// One candidate move per slide row (full move, x only, y only, stay); this gives wall sliding.
pub fn cands(db: &Db, p: &Player) -> MultiRel<usize, Cand> {
    let k = &db.t.k;
    let (radius, height) = (one(&k.player_radius), one(&k.player_height));
    let sl = &db.t.slide;
    (&*p.x)
        .and(&*p.y)
        .and(&*p.z)
        .and(&*p.momx)
        .and(&*p.momy)
        .cross(
            (&sl.keep_x)
                .and(&sl.keep_y)
                .and(&sl.stay)
                .and(Same::<Row<Slide>>::new()),
        )
        // The slide's id is the candidate's preference order.
        .map(
            move |(((((x, y), z), momx), momy), (((kx, ky), stay), idx)): (
                ((((f64, f64), f64), f64), f64),
                (((f64, f64), bool), Row<Slide>),
            )| {
                (
                    PLAYER,
                    idx.0,
                    x + kx * momx,
                    y + ky * momy,
                    z,
                    radius,
                    height,
                    false,
                    false,
                    stay,
                )
            },
        )
        .key_by(|c: Cand| slot_key(c.0, c.1))
        .collect()
}

// Takes the chosen candidate, keeps only the momentum its slide row keeps, applies gravity.
pub fn walk(db: &Db, p: &Player, clips: &MultiRel<usize, Clip>) -> Player {
    let g = one(&db.t.k.gravity);
    let sl = &db.t.slide;
    let clip = || me(p).select(clips);
    let t: VecRel<Row<Player>, (f64, f64, f64, f64, f64, f64)> = (&*p.z)
        .and(&*p.momx)
        .and(&*p.momy)
        .and(&*p.momz)
        .and(clip())
        .and(
            clip()
                .map(|c: Clip| Row::<Slide>::new(c.0.1))
                .select((&sl.keep_x).and(&sl.keep_y)),
        )
        .map(
            move |(((((z, momx), momy), momz), (c, floor, _, _)), (kx, ky)): (
                ((((f64, f64), f64), f64), Clip),
                (f64, f64),
            )| {
                // Fall when above the floor; snap up onto a higher floor (stairs).
                let momz = if z > floor { momz - g } else { 0.0 };
                let z = (z + momz).max(floor);
                (
                    c.2,
                    c.3,
                    z,
                    momx * kx,
                    momy * ky,
                    if z == floor { 0.0 } else { momz },
                )
            },
        )
        .collect();
    type T = (f64, f64, f64, f64, f64, f64);
    Player {
        x: rc((&t).map(|v: T| v.0)),
        y: rc((&t).map(|v: T| v.1)),
        z: rc((&t).map(|v: T| v.2)),
        momx: rc((&t).map(|v: T| v.3)),
        momy: rc((&t).map(|v: T| v.4)),
        momz: rc((&t).map(|v: T| v.5)),
        ..p.clone()
    }
}

// A node of the weapon state machine walk: player, weapon, fire held, alive, weapon state and
// timer, flash state and timer, ammo count, state to enter next (NONE = settled), pellets fired, steps taken.
type Psp = (
    Row<Player>,
    Row<Weapon>,
    bool,
    bool,
    Row<AnimState>,
    i32,
    Row<AnimState>,
    i32,
    i32,
    Row<AnimState>,
    usize,
    u8,
);
// A weapon's ammo kind, ammo per shot, attack state, flash state and flash duration.
type WeaponRow = (Row<AmmoKind>, i32, Row<AnimState>, Row<AnimState>, i32);

// Facts the weapon rules match on.
fn weapon_facts(q: Psp, per_shot: i32) -> usize {
    (bit(q.2, wfact::FIRE) | bit(q.3, wfact::ALIVE) | bit(q.8 >= per_shot, wfact::AMMO)) as usize
}

// Enters the pending weapon state (its tics and next state), or redirects to the attack state;
// firing spends ammo and starts the flash. `o` = the matched rule's (redirect, pellets).
fn enter(
    q: Psp,
    (tics, next): (i32, Row<AnimState>),
    w: WeaponRow,
    (redirect, pellets): (bool, usize),
) -> Psp {
    let (who, weapon, fire, alive, wstate, wtics, flash, flash_tics, ammo, pending, fired, depth) =
        q;
    let (_, per_shot, attack, wflash, wflash_tics) = w;
    if redirect {
        return (
            who,
            weapon,
            fire,
            alive,
            wstate,
            wtics,
            flash,
            flash_tics,
            ammo,
            attack,
            fired,
            depth + 1,
        );
    }
    let fires = pellets > 0;
    (
        who,
        weapon,
        fire,
        alive,
        pending,
        tics,
        if fires { wflash } else { flash },
        if fires { wflash_tics } else { flash_tics },
        ammo - per_shot * fires as i32,
        // A 0-tic state passes straight through to its next state this same tic.
        if tics == 0 { next } else { Row::new(NONE) },
        fired.max(pellets),
        depth + 1,
    )
}

// Weapon logic for this tic (switching, animation, firing); returns the updated player, ammo and bullets.
pub fn weapon(
    db: &Db,
    p: &Player,
    ammo: &C<AmmoKind, i32>,
    inp: Input,
    tic: u64,
) -> (Player, C<AmmoKind, i32>, MultiRel<usize, Shot>) {
    let (k, st, wp, wr) = (&db.t.k, &db.t.state, &db.t.weapon, &db.t.weapon_rule);
    let shot_z = one(&k.shot_z);
    let wrow = || {
        (&wp.ammo)
            .and(&wp.per_shot)
            .and(&wp.attack)
            .and(&wp.flash)
            .and(&wp.flash_tics)
            .map(
                |((((a, s), at), f), ft): (
                    (((Row<AmmoKind>, i32), Row<AnimState>), Row<AnimState>),
                    i32,
                )| (a, s, at, f, ft),
            )
    };
    // Weapon asked for: this tic's key, else a remembered request (or NONE).
    let target = move |wanted: Row<Weapon>| inp.weapon.map_or(wanted, Row::new);
    // A different, owned weapon was asked for; the switch happens only in the ready state.
    let wants = move |cur: Row<Weapon>, wanted: Row<Weapon>, owned: u8, exists: bool| {
        exists && target(wanted) != cur && owned & (1 << target(wanted).0) != 0
    };
    // Switch weapons if asked and ready, else remember the request and tick down the current state's timer.
    // Value: (weapon, wanted, wtics, state to enter now or NONE).
    let switched: VecRel<Row<Player>, (Row<Weapon>, Row<Weapon>, i32, Row<AnimState>)> = (&*p
        .weapon)
        .and(&*p.wanted)
        .and(&*p.owned)
        .and(&*p.wstate)
        .and(&*p.wtics)
        .and((&*p.weapon).select(&wp.ready))
        .and((&*p.wanted).map(target).select(&wp.ready).opt())
        .and((&*p.wstate).select(&st.next))
        .map(
            move |(((((((cur, wanted), owned), ws), wt), cur_ready), new_ready), next): (
                (
                    (
                        ((((Row<Weapon>, Row<Weapon>), u8), Row<AnimState>), i32),
                        Row<AnimState>,
                    ),
                    Option<Row<AnimState>>,
                ),
                Row<AnimState>,
            )| {
                let want = wants(cur, wanted, owned, new_ready.is_some());
                match new_ready {
                    Some(ready) if want && ws == cur_ready => {
                        (target(wanted), Row::new(NONE), wt, ready)
                    }
                    _ => (
                        cur,
                        if want { target(wanted) } else { wanted },
                        wt - 1,
                        if wt - 1 <= 0 { next } else { Row::new(NONE) },
                    ),
                }
            },
        )
        .collect();
    // Start node: weapon fields plus the ammo count for the (possibly new) weapon.
    let start = (&switched)
        .and(&*p.wstate)
        .and(&*p.flash)
        .and(&*p.flash_tics)
        .and(&*p.dead)
        .and(
            (&switched)
                .map(|s: (Row<Weapon>, Row<Weapon>, i32, Row<AnimState>)| s.0)
                .select(&wp.ammo)
                .select(ammo),
        )
        .and(Same::<Row<Player>>::new())
        .map(
            move |((((((s, ws), fl), ft), dead), a), who): (
                (
                    (
                        (
                            (
                                (
                                    (Row<Weapon>, Row<Weapon>, i32, Row<AnimState>),
                                    Row<AnimState>,
                                ),
                                Row<AnimState>,
                            ),
                            i32,
                        ),
                        bool,
                    ),
                    i32,
                ),
                Row<Player>,
            )|
                  -> Psp { (who, s.0, inp.fire, !dead, ws, s.2, fl, ft, a, s.3, 0, 0) },
        );
    // One step: look up the pending state and weapon, pick the weapon rule, enter the state. At most 4 per tic.
    // The step relation is keyed by the Psp node itself: node -> next node.
    let node = || Same::<Psp>::new();
    let ctx = node()
        .filt(|q: Psp| q.9.0 != NONE && q.11 < 4)
        .and(
            node()
                .map(|q: Psp| q.9)
                .select((&st.tics).and(&st.next).and(&st.action)),
        )
        .and(node().map(|q: Psp| q.1).select(wrow()));
    let step = (&ctx)
        .and(
            (&ctx)
                .map(
                    |((q, (_, a)), w): (
                        (Psp, ((i32, Row<AnimState>), crate::info::Action)),
                        WeaponRow,
                    )| choice_key(a as usize, weapon_facts(q, w.1), 3),
                )
                .select(&db.t.weapon_choice)
                .select((&wr.redirect).and(&wr.pellets))
                .opt(),
        )
        .map(
            |(((q, (r, _)), w), o): (
                (
                    (Psp, ((i32, Row<AnimState>), crate::info::Action)),
                    WeaponRow,
                ),
                Option<(bool, usize)>,
            )| enter(q, r, w, o.unwrap_or((false, 0))),
        );
    // The last node of each chain: every node visited (node -> depth), inverted back to node values.
    let done: MultiRel<Row<Player>, Psp> = start
        .reach(&step, 4)
        .inv()
        .filt(|q: Psp| q.9.0 == NONE || q.11 == 4)
        .key_by(|q: Psp| q.0)
        .collect();
    // Weapon fields after the walk, settling any state still pending.
    let fired: VecRel<
        Row<Player>,
        (
            Row<Weapon>,
            Row<Weapon>,
            Row<AnimState>,
            i32,
            Row<AnimState>,
            i32,
        ),
    > = (&switched)
        .and(&done)
        .map(
            |((w, wanted, _, _), q): ((Row<Weapon>, Row<Weapon>, i32, Row<AnimState>), Psp)| {
                let (wstate, wtics) = if q.9.0 != NONE { (q.9, 1) } else { (q.4, q.5) };
                (w, wanted, wstate, wtics, q.6, q.7)
            },
        )
        .collect();
    // Advance the muzzle flash one tic: count down, then move to the next flash state.
    type F = (
        Row<Weapon>,
        Row<Weapon>,
        Row<AnimState>,
        i32,
        Row<AnimState>,
        i32,
    );
    let flash: VecRel<Row<Player>, (Row<AnimState>, i32)> = (&fired)
        .and((&fired).map(|f: F| f.4).select(&st.next))
        .and((&fired).map(|f: F| f.4).select(&st.next).select(&st.tics))
        .map(
            |((f, next), next_tics): ((F, Row<AnimState>), i32)| match (f.4.0, f.5) {
                (0, _) => (f.4, f.5),
                (_, t) if t > 1 => (f.4, t - 1),
                _ => (next, next_tics),
            },
        )
        .collect();
    // One bullet per pellet fired, with random spread and damage.
    let wd = (&wp.damage)
        .and(&wp.damage_roll)
        .and(&wp.range)
        .and(&wp.spread_div);
    let shots = (&*p.x)
        .and(&*p.y)
        .and(&*p.z)
        .and(&*p.ang)
        .and(&done)
        .and((&done).map(|q: Psp| q.1).select(wd))
        .expand(
            |((_, q), _): (((((f64, f64), f64), f64), Psp), (((i32, i32), f64), f64))| (0, q.10),
        )
        .map(
            move |((((((x, y), z), ang), _), (((damage, roll), range), div)), n): (
                (((((f64, f64), f64), f64), Psp), (((i32, i32), f64), f64)),
                usize,
            )| {
                // Difference of two rolls: spread centred on zero.
                let spread = (rnd(tic, n, 1) - rnd(tic, n, 2)) as f64 * TAU / div;
                (
                    slot_key(PLAYER, n),
                    (
                        (1 << 23) | n,
                        PLAYER,
                        x,
                        y,
                        z + shot_z,
                        ang + spread,
                        range,
                        damage * (rnd(tic, n, 3) % roll + 1),
                    ),
                )
            },
        )
        .key_by(|(id, _)| id)
        .map(|(_, s)| s)
        .collect();
    // Write the spent count back to its ammo kind; keep the other kinds.
    let spent: MultiRel<Row<AmmoKind>, i32> = (&done)
        .map(|q: Psp| q.1)
        .select(&wp.ammo)
        .and((&done).map(|q: Psp| q.8))
        .key_by(|(k, _)| k)
        .map(|(_, n)| n)
        .collect();
    let ammo = rc((&**ammo).minus(&spent).union(&spent));
    type W = (
        Row<Weapon>,
        Row<Weapon>,
        Row<AnimState>,
        i32,
        Row<AnimState>,
        i32,
    );
    let player = Player {
        weapon: rc((&fired).map(|f: W| f.0)),
        wanted: rc((&fired).map(|f: W| f.1)),
        wstate: rc((&fired).map(|f: W| f.2)),
        wtics: rc((&fired).map(|f: W| f.3)),
        flash: rc((&flash).map(|f: (Row<AnimState>, i32)| f.0)),
        flash_tics: rc((&flash).map(|f: (Row<AnimState>, i32)| f.1)),
        ..p.clone()
    };
    (player, ammo, shots)
}

// The player fields a pickup changes: health, armor, armor class, owned weapons, wanted weapon, keys.
type Gains = (i32, i32, Row<ArmorClass>, u8, Row<Weapon>, u8);
// A pickup row: health, health max, armor, armor max, class set, class min, ammo kind, ammo,
// weapon, weapon bit, key, always taken.
type PickupRow = (
    i32,
    i32,
    i32,
    i32,
    i32,
    i32,
    Row<AmmoKind>,
    i32,
    Row<Weapon>,
    u8,
    u8,
    bool,
);

// A pickup's effect on the player's own fields; health and armor only ever go up, toward the item's max.
fn gain((health, armor, class, owned, wanted, keys): Gains, d: PickupRow) -> Gains {
    let (dh, dh_max, da, da_max, class_set, class_min, _, _, weapon, weapon_bit, key, _) = d;
    (
        health.max((health + dh).min(dh_max)),
        armor.max((armor + da).min(da_max)),
        Row::new(
            (if class_set > 0 {
                class_set as usize
            } else {
                class.0
            })
            .max(class_min as usize),
        ),
        owned | weapon_bit,
        if weapon_bit & !owned != 0 {
            weapon
        } else {
            wanted
        },
        keys | key,
    )
}

// Picks up touching items that do something; returns the updated player, ammo, and the items taken.
pub fn pickups(
    db: &Db,
    p: &Player,
    ammo: &C<AmmoKind, i32>,
    m: &Mobj,
    live: &C<Mobj, bool>,
) -> (Player, C<AmmoKind, i32>, DenseFold<Row<Mobj>, bool>) {
    let (k, tt, pk, am) = (&db.t.k, &db.t.ttype, &db.t.pickup, &db.t.ammo);
    let (pr, ph, zr, bonus) = (
        one(&k.player_radius),
        one(&k.player_height),
        one(&k.pickup_z_reach),
        one(&k.bonus_add),
    );
    let def = (&pk.health)
        .and(&pk.health_max)
        .and(&pk.armor)
        .and(&pk.armor_max)
        .and(&pk.class_set)
        .and(&pk.class_min)
        .and(&pk.ammo_kind)
        .and(&pk.ammo)
        .and(&pk.weapon)
        .and(&pk.weapon_bit)
        .and(&pk.key)
        .and(&pk.always)
        .map(
            |(((((((((((h, hm), a), am), cs), cm), ak), n), w), wb), key), al): (
                (
                    (
                        (
                            (((((((i32, i32), i32), i32), i32), i32), Row<AmmoKind>), i32),
                            Row<Weapon>,
                        ),
                        u8,
                    ),
                    u8,
                ),
                bool,
            )|
             -> PickupRow { (h, hm, a, am, cs, cm, ak, n, w, wb, key, al) },
        );
    let gains = (&*p.health)
        .and(&*p.armor)
        .and(&*p.armor_class)
        .and(&*p.owned)
        .and(&*p.wanted)
        .and(&*p.keys)
        .map(
            |(((((h, a), c), o), w), k): (
                ((((i32, i32), Row<ArmorClass>), u8), Row<Weapon>),
                u8,
            )|
             -> Gains { (h, a, c, o, w, k) },
        );
    let pos = (&*p.x).and(&*p.y).and(&*p.z).and(&*p.dead).and(gains);
    // Items overlapping the player, with the ammo each would add (capped): (item, pickup, player gains, more ammo).
    let touching: MultiRel<usize, (Row<Mobj>, PickupRow, Gains, i32)> = (&**live)
        .filt(|l: bool| l)
        .and(&*m.flags)
        .filt(|(_, f): (bool, u32)| f & PICKUP != 0)
        .and(Same::<Row<Mobj>>::new())
        .and((&*m.x).and(&*m.y).and(&*m.z))
        .and((&*m.kind).select(&tt.radius))
        .and((&*m.kind).select(&tt.pickup).select(def))
        .key_by(|_| Row::<Player>::new(0))
        .and(pos)
        .filt(
            move |((((((_, _), _), ((mx, my), mz)), r), _), ((((px, py), pz), dead), _)): (
                (
                    ((((bool, u32), Row<Mobj>), ((f64, f64), f64)), f64),
                    PickupRow,
                ),
                ((((f64, f64), f64), bool), Gains),
            )| {
                !dead
                    && (mx - px).abs() < pr + r
                    && (my - py).abs() < pr + r
                    && mz <= pz + ph
                    && mz + zr >= pz
            },
        )
        .map(
            |((((((_, _), uid), _), _), d), (_, g)): (
                (
                    ((((bool, u32), Row<Mobj>), ((f64, f64), f64)), f64),
                    PickupRow,
                ),
                ((((f64, f64), f64), bool), Gains),
            )| (uid, d, g),
        )
        // Join the current count and cap of the ammo kind it gives.
        .key_by(|(_, d, _): (Row<Mobj>, PickupRow, Gains)| d.6)
        .and((&**ammo).and(&am.cap))
        .map(
            |((uid, d, g), (a, cap)): ((Row<Mobj>, PickupRow, Gains), (i32, i32))| {
                (uid, d, g, (a + d.7).min(cap) - a)
            },
        )
        // Skip items that would change nothing (full health, full ammo), unless `always`.
        .filt(|(_, d, g, more): (Row<Mobj>, PickupRow, Gains, i32)| {
            let n = gain(g, d);
            d.11 || more > 0 || n.0 > g.0 || n.1 > g.1
        })
        .key_by(|_| PLAYER)
        .collect();
    // Apply the player effects in turn.
    let gained = (&touching).dense_fold(
        1,
        None,
        |a: Option<Gains>, (_, d, g, _): (Row<Mobj>, PickupRow, Gains, i32)| {
            Some(gain(a.unwrap_or(g), d))
        },
    );
    let applied = me(p).select(&gained).flat_map(|g: Option<Gains>| g);
    let any = me(p).select(&gained).map(|_| bonus);
    type G = Gains;
    let player = Player {
        health: rc((&*p.health)
            .minus(&applied)
            .union((&applied).map(|g: G| g.0))),
        armor: rc((&*p.armor)
            .minus(&applied)
            .union((&applied).map(|g: G| g.1))),
        armor_class: rc((&*p.armor_class)
            .minus(&applied)
            .union((&applied).map(|g: G| g.2))),
        owned: rc((&*p.owned)
            .minus(&applied)
            .union((&applied).map(|g: G| g.3))),
        wanted: rc((&*p.wanted)
            .minus(&applied)
            .union((&applied).map(|g: G| g.4))),
        keys: rc((&*p.keys).minus(&applied).union((&applied).map(|g: G| g.5))),
        bonuscount: rc((&*p.bonuscount).minus(&any).union(&any)),
        ..p.clone()
    };
    // Ammo added per kind, capped.
    let added = (&touching)
        .key_by(|(_, d, _, _): (Row<Mobj>, PickupRow, Gains, i32)| d.6)
        .map(|(_, _, _, more): (Row<Mobj>, PickupRow, Gains, i32)| more)
        .dense_fold(am.id.n, 0, |a: i32, n: i32| a + n);
    let ammo = rc((&**ammo)
        .and(added.opt())
        .and(&am.cap)
        .map(|((a, n), cap): ((i32, Option<i32>), i32)| (a + n.unwrap_or(0)).min(cap)));
    // Items to remove from the map.
    let taken = (&touching)
        .key_by(|(uid, _, _, _): (Row<Mobj>, PickupRow, Gains, i32)| uid)
        .dense_fold(m.id.n, true, |_, _| true);
    (player, ammo, taken)
}

// The player fields damage changes: health, armor, armor class, dead, damagecount.
type Hurt = (i32, i32, Row<ArmorClass>, bool, i32);

// Applies damage after armor absorbs its share (num / den); sets dead at 0 health.
fn hurt(p: Hurt, d: i32, (num, den): (i32, i32), cap: i32) -> Hurt {
    let (health, armor, class, dead, dc) = p;
    if dead || d <= 0 {
        return p;
    }
    let saved = (d * num / den).min(armor);
    let h = health - (d - saved);
    (
        h.max(0),
        armor - saved,
        if armor - saved == 0 {
            Row::new(0)
        } else {
            class
        },
        h <= 0,
        (dc + d).min(cap),
    )
}

// End of tic: damage, damaging floors every 32 tics, use-key latch, exit flag, screen-tint timers.
pub fn finish(
    db: &Db,
    p: &Player,
    damage: &DenseFold<usize, i32>,
    s: &SectorState,
    inp: Input,
    exited: bool,
    tic: u64,
) -> Player {
    let (k, ac) = (&db.t.k, &db.t.armor);
    let (period, cap) = (one(&k.floor_damage_period), one(&k.damage_cap));
    let t: VecRel<Row<Player>, (Hurt, i32)> = (&*p.health)
        .and(&*p.armor)
        .and(&*p.armor_class)
        .and(&*p.dead)
        .and(&*p.damagecount)
        .and(&*p.bonuscount)
        .and(&*p.z)
        .and(me(p).select(damage).opt())
        .and((&*p.sector).select(&*s.floor))
        .and(
            (&*p.sector)
                .select(&db.sector.special)
                .select(&db.t.sector_damage)
                .opt(),
        )
        .and((&*p.armor_class).select((&ac.num).and(&ac.den)))
        .map(
            move |((((((((((h, a), c), dead), dc), bc), z), d), floor), fd), armor_frac): (
                (
                    (
                        (
                            ((((((i32, i32), Row<ArmorClass>), bool), i32), i32), f64),
                            Option<i32>,
                        ),
                        f64,
                    ),
                    Option<i32>,
                ),
                (i32, i32),
            )| {
                // Floor damage only on the period tick and only when standing on the floor.
                let fd = if tic % period == 0 && z <= floor {
                    fd.unwrap_or(0)
                } else {
                    0
                };
                let (h, a, c, dead, dc) =
                    hurt((h, a, c, dead, dc), d.unwrap_or(0) + fd, armor_frac, cap);
                ((h, a, c, dead, (dc - 1).max(0)), (bc - 1).max(0))
            },
        )
        .collect();
    type T = (Hurt, i32);
    Player {
        health: rc((&t).map(|v: T| v.0.0)),
        armor: rc((&t).map(|v: T| v.0.1)),
        armor_class: rc((&t).map(|v: T| v.0.2)),
        dead: rc((&t).map(|v: T| v.0.3)),
        damagecount: rc((&t).map(|v: T| v.0.4)),
        bonuscount: rc((&t).map(|v: T| v.1)),
        usedown: rc((&p.id).map(move |_| inp.use_)),
        exited: rc((&*p.exited).map(move |e: bool| e || exited)),
        ..p.clone()
    }
}
