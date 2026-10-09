use crate::db::{consts, Db, Info, StateRow, Weapon};
use crate::info::{wfact, AmmoDef, ArmorClass, Consts, PickupDef, Slide, WOut, PICKUP};
use crate::rules::bit;
use crate::mobj::Mobj;
use crate::physics::{rnd, Cand, Clip, Shot};
use crate::wad::{Sector, NONE};
use prela::engine::*;
use std::f64::consts::TAU;

pub const PLAYER: usize = 0;

#[derive(Clone, Copy, Default)]
// One tic of player controls.
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

#[derive(Clone, Copy, Debug)]
// The player's full state.
pub struct Player {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ang: f64,
    pub momx: f64,
    pub momy: f64,
    pub momz: f64,
    pub sector: usize,
    pub usedown: bool,
    pub exited: bool,
    pub dead: bool,
    pub health: i32,
    pub armor: i32,
    pub armor_class: i32,
    pub owned: u8,
    pub keys: u8,
    pub weapon: usize,
    pub wanted: usize,
    pub wstate: usize,
    pub wtics: i32,
    pub flash: usize,
    pub flash_tics: i32,
    pub damagecount: i32,
    pub bonuscount: i32,
}

impl Player {
    // A fresh player holding the starting weapon (starting ammo comes from the ammo table).
    pub fn new(x: f64, y: f64, ang: f64, wstate: usize, k: Consts) -> Player {
        Player {
            x,
            y,
            z: 0.0,
            ang,
            momx: 0.0,
            momy: 0.0,
            momz: 0.0,
            sector: 0,
            usedown: false,
            exited: false,
            dead: false,
            health: k.start_health,
            armor: 0,
            armor_class: 0,
            owned: k.start_owned,
            keys: 0,
            weapon: k.start_weapon,
            wanted: NONE,
            wstate,
            wtics: 1,
            flash: 0,
            flash_tics: 0,
            damagecount: 0,
            bonuscount: 0,
        }
    }
}

// Turns and accelerates from input, with friction; dead players just slide to a stop.
fn thrust(p: Player, inp: Input, k: Consts) -> Player {
    if p.dead {
        return Player { momx: p.momx * k.friction, momy: p.momy * k.friction, ..p };
    }
    let ang = p.ang + inp.turn * if inp.run { k.turn_run } else { k.turn_walk } + inp.look;
    let (s, c) = ang.sin_cos();
    let speed = if inp.run { k.run_thrust } else { k.walk_thrust };
    let fwd = inp.forward * speed;
    let side = inp.strafe * speed * k.strafe_scale;
    Player {
        ang,
        momx: (p.momx + fwd * c + side * s) * k.friction,
        momy: (p.momy + fwd * s - side * c) * k.friction,
        ..p
    }
}

// Applies thrust to the player.
pub fn moved(db: &Db, players: &VecRel<usize, Player>, inp: Input) -> VecRel<usize, Player> {
    let k = consts(db);
    players.map(move |p| thrust(p, inp, k)).collect()
}

// One candidate move per slide row (full move, x only, y only, stay); this gives wall sliding.
pub fn cands(db: &Db, players: &VecRel<usize, Player>) -> HashIdx<(usize, usize), Cand> {
    let k = consts(db);
    players
        .cross(&db.t.slides)
        .and(Same::<(usize, usize)>::new())
        .map(move |((p, s), (who, idx)): ((Player, Slide), (usize, usize))| Cand {
            who,
            idx,
            x: p.x + s.keep_x * p.momx,
            y: p.y + s.keep_y * p.momy,
            z: p.z,
            radius: k.player_radius,
            height: k.player_height,
            monster: false,
            missile: false,
            stay: s.stay,
        })
        .collect()
}

// Takes the chosen candidate, keeps only the momentum its slide row keeps, applies gravity.
pub fn walk(db: &Db, players: &VecRel<usize, Player>, clips: &HashIdx<usize, Clip>) -> VecRel<usize, Player> {
    let g = consts(db).gravity;
    players
        .and(clips)
        .and(clips.map(|k: Clip| k.c.idx).select(&db.t.slides))
        .map(move |((p, k), s): ((Player, Clip), Slide)| {
            let momz = if p.z > k.floor { p.momz - g } else { 0.0 };
            let z = (p.z + momz).max(k.floor);
            Player { x: k.c.x, y: k.c.y, z, momx: p.momx * s.keep_x, momy: p.momy * s.keep_y, momz: if z == k.floor { 0.0 } else { momz }, ..p }
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
// Weapon state machine node: the player's weapon fields and ammo, the state to enter next, pellets fired, steps taken.
struct Psp {
    who: usize,
    weapon: usize,
    fire: bool,
    alive: bool,
    wstate: usize,
    wtics: i32,
    flash: usize,
    flash_tics: i32,
    ammo: i32,
    pending: usize,
    pellets: usize,
    depth: u8,
}

// Facts the weapon rules match on.
fn weapon_facts(q: Psp, w: Weapon) -> usize {
    (bit(q.fire, wfact::FIRE) | bit(q.alive, wfact::ALIVE) | bit(q.ammo >= w.per_shot, wfact::AMMO)) as usize
}

// Enters the pending weapon state, or redirects to the attack state; firing spends ammo and starts the flash.
fn enter(q: Psp, r: StateRow, w: Weapon, o: WOut) -> Psp {
    let q = Psp { depth: q.depth + 1, ..q };
    if o.redirect {
        return Psp { pending: w.attack, ..q };
    }
    let fires = o.pellets > 0;
    Psp {
        wstate: q.pending,
        wtics: r.tics,
        flash: if fires { w.flash } else { q.flash },
        flash_tics: if fires { w.flash_tics } else { q.flash_tics },
        ammo: q.ammo - w.per_shot * fires as i32,
        pending: if r.tics == 0 { r.next } else { NONE },
        pellets: q.pellets.max(o.pellets),
        ..q
    }
}

// Advances the muzzle flash one tic: count down, then move to the next flash state.
fn flash_tick(db: &Db, players: &VecRel<usize, Player>) -> VecRel<usize, Player> {
    let state = || players.map(|p: Player| p.flash).select(&db.t.states);
    players
        .and(state())
        .and(state().map(|r: StateRow| r.next).select(&db.t.states))
        .map(|((p, r), n): ((Player, StateRow), StateRow)| {
            if p.flash == 0 {
                p
            } else if p.flash_tics > 1 {
                Player { flash_tics: p.flash_tics - 1, ..p }
            } else {
                Player { flash: r.next, flash_tics: n.tics, ..p }
            }
        })
        .collect()
}

// Weapon logic for this tic (switching, animation, firing); returns the updated player, ammo and bullets.
pub fn weapon(db: &Db, players: &VecRel<usize, Player>, ammo: &VecRel<usize, i32>, inp: Input, tic: u64) -> (VecRel<usize, Player>, VecRel<usize, i32>, HashIdx<usize, Shot>) {
    let k = consts(db);
    let target = move |p: Player| inp.weapon.unwrap_or(p.wanted);
    // Player with current weapon, requested weapon and current weapon state.
    let rows = players
        .and(players.map(|p: Player| p.weapon).select(&db.t.weapons))
        .and(players.map(target).select(&db.t.weapons).opt())
        .and(players.map(|p: Player| p.wstate).select(&db.t.states));
    let wants = move |p: Player, new: Option<Weapon>| new.is_some() && target(p) != p.weapon && p.owned & (1 << target(p)) != 0;
    let switching = move |(((p, cur), new), _): (((Player, Weapon), Option<Weapon>), StateRow)| wants(p, new) && p.wstate == cur.ready;
    // Switch weapons if asked and ready, else remember the request and tick down the current state's timer.
    let switched: VecRel<usize, (Player, usize)> = (&rows)
        .filt(switching)
        .flat_map(move |(((p, _), new), _): (((Player, Weapon), Option<Weapon>), StateRow)| new.map(|w| (Player { weapon: target(p), wanted: NONE, ..p }, w.ready)))
        .union((&rows).filt(move |r| !switching(r)).map(move |(((p, _), new), row): (((Player, Weapon), Option<Weapon>), StateRow)| {
            let wtics = p.wtics - 1;
            let wanted = if wants(p, new) { target(p) } else { p.wanted };
            (Player { wtics, wanted, ..p }, if wtics <= 0 { row.next } else { NONE })
        }))
        .collect();
    // Start node: weapon fields plus the ammo count for the (possibly new) weapon.
    let start = (&switched)
        .and((&switched).map(|(p, _): (Player, usize)| p.weapon).select(&db.t.weapons).map(|w: Weapon| w.ammo).select(ammo))
        .and(Same::<usize>::new())
        .map(move |(((p, pending), a), who): (((Player, usize), i32), usize)| Psp {
            who,
            weapon: p.weapon,
            fire: inp.fire,
            alive: !p.dead,
            wstate: p.wstate,
            wtics: p.wtics,
            flash: p.flash,
            flash_tics: p.flash_tics,
            ammo: a,
            pending,
            pellets: 0,
            depth: 0,
        });
    // One step: look up the pending state and weapon, pick the weapon rule, enter the state. At most 4 per tic.
    let node = || Same::<Psp>::new();
    let ctx = node()
        .filt(|q: Psp| q.pending != NONE && q.depth < 4)
        .and(node().map(|q: Psp| q.pending).select(&db.t.states))
        .and(node().map(|q: Psp| q.weapon).select(&db.t.weapons));
    let step = (&ctx)
        .and((&ctx).map(|((q, r), w): ((Psp, StateRow), Weapon)| (r.action as usize, weapon_facts(q, w))).select(&db.t.weapon_choice).opt())
        .map(|(((q, r), w), o): (((Psp, StateRow), Weapon), Option<WOut>)| enter(q, r, w, o.unwrap_or_default()));
    // The last node of each chain.
    let done: HashIdx<usize, Psp> = start
        .reach(&step, 4)
        .inv()
        .filt(|q: Psp| q.pending == NONE || q.depth == 4)
        .key_by(|q: Psp| q.who)
        .collect();
    // Copy the weapon fields back, settling any state still pending.
    let fired: VecRel<usize, Player> = (&switched)
        .and(&done)
        .map(|((p, _), q): ((Player, usize), Psp)| {
            let (wstate, wtics) = if q.pending != NONE { (q.pending, 1) } else { (q.wstate, q.wtics) };
            Player { wstate, wtics, flash: q.flash, flash_tics: q.flash_tics, ..p }
        })
        .collect();
    let weapon_of = || (&done).map(|q: Psp| q.weapon).select(&db.t.weapons);
    // One bullet per pellet fired, with random spread and damage.
    let shots = (&fired)
        .and(&done)
        .and(weapon_of())
        .expand(|((_, q), _): ((Player, Psp), Weapon)| (0, q.pellets))
        .map(move |(((p, _), w), n): (((Player, Psp), Weapon), usize)| {
            let spread = (rnd(tic, n, 1) - rnd(tic, n, 2)) as f64 * TAU / w.spread_div;
            Shot { id: (1 << 23) | n, from: PLAYER, x: p.x, y: p.y, z: p.z + k.shot_z, ang: p.ang + spread, range: w.range, damage: w.damage * (rnd(tic, n, 3) % w.damage_roll + 1) }
        })
        .key_by(|s: Shot| s.id)
        .collect();
    // Write the spent count back to its ammo kind.
    let spent: HashIdx<usize, i32> = (&done)
        .and(weapon_of())
        .key_by(|(_, w): (Psp, Weapon)| w.ammo)
        .map(|(q, _): (Psp, Weapon)| q.ammo)
        .collect();
    let ammo = ammo.minus(&spent).union(&spent).collect();
    (flash_tick(db, &fired), ammo, shots)
}

// A pickup's effect on the player's own fields; health and armor only ever go up, toward the item's max.
fn gain(p: Player, d: PickupDef, bonus: i32) -> Player {
    Player {
        health: p.health.max((p.health + d.health).min(d.health_max)),
        armor: p.armor.max((p.armor + d.armor).min(d.armor_max)),
        armor_class: (if d.class_set > 0 { d.class_set } else { p.armor_class }).max(d.class_min),
        owned: p.owned | d.weapon_bit,
        wanted: if d.weapon_bit & !p.owned != 0 { d.weapon } else { p.wanted },
        keys: p.keys | d.key,
        bonuscount: bonus,
        ..p
    }
}

// Picks up touching items that do something; returns the updated player, ammo, and ids of items taken.
pub fn pickups(
    db: &Db,
    players: &VecRel<usize, Player>,
    ammo: &VecRel<usize, i32>,
    mobjs: &HashIdx<usize, Mobj>,
) -> (VecRel<usize, Player>, VecRel<usize, i32>, Fold<usize, bool>) {
    let k = consts(db);
    // Items overlapping the player, with the ammo each would add (capped).
    let touching: HashIdx<usize, (Mobj, PickupDef, Player, i32)> = mobjs
        .filt(|m: Mobj| m.flags & PICKUP != 0)
        .key_by(|m: Mobj| m.kind)
        .and(&db.t.infos)
        .and(&db.t.pickups)
        .key_by(|_| PLAYER)
        .and(players)
        .filt(move |(((m, i), _), p): (((Mobj, Info), PickupDef), Player)| {
            !p.dead && (m.x - p.x).abs() < k.player_radius + i.radius && (m.y - p.y).abs() < k.player_radius + i.radius && m.z <= p.z + k.player_height && m.z + k.pickup_z_reach >= p.z
        })
        .key_by(|((_, d), _): (((Mobj, Info), PickupDef), Player)| d.ammo_kind)
        .and(ammo.and(&db.t.ammo_defs))
        .map(|((((m, _), d), p), (a, k)): ((((Mobj, Info), PickupDef), Player), (i32, AmmoDef))| (m, d, p, (a + d.ammo).min(k.cap) - a))
        .filt(move |(_, d, p, more): (Mobj, PickupDef, Player, i32)| {
            let g = gain(p, d, k.bonus_add);
            d.always || more > 0 || g.health > p.health || g.armor > p.armor
        })
        .key_by(|_| PLAYER)
        .collect();
    // Apply the player effects in turn.
    let gained = (&touching).fold(None, move |a: Option<Player>, (_, d, p, _): (Mobj, PickupDef, Player, i32)| Some(gain(a.unwrap_or(p), d, k.bonus_add)));
    let applied = players.and(gained.opt()).map(|(p, g): (Player, Option<Option<Player>>)| g.flatten().unwrap_or(p)).collect();
    // Ammo added per kind, capped.
    let added = (&touching)
        .key_by(|(_, d, _, _): (Mobj, PickupDef, Player, i32)| d.ammo_kind)
        .map(|(_, _, _, more): (Mobj, PickupDef, Player, i32)| more)
        .fold(0, |a: i32, n: i32| a + n);
    let ammo = ammo
        .and(added.opt())
        .and(&db.t.ammo_defs)
        .map(|((a, n), k): ((i32, Option<i32>), AmmoDef)| (a + n.unwrap_or(0)).min(k.cap))
        .collect();
    // Items to remove from the map.
    let taken = (&touching).key_by(|(m, _, _, _): (Mobj, PickupDef, Player, i32)| m.id).fold(true, |_, _| true);
    (applied, ammo, taken)
}

// Applies damage after armor absorbs its share; sets dead at 0 health.
pub fn hurt(p: Player, d: i32, ac: ArmorClass, cap: i32) -> Player {
    if p.dead || d <= 0 {
        return p;
    }
    let saved = (d * ac.num / ac.den).min(p.armor);
    let health = p.health - (d - saved);
    Player {
        armor: p.armor - saved,
        armor_class: if p.armor - saved == 0 { 0 } else { p.armor_class },
        health: health.max(0),
        dead: health <= 0,
        damagecount: (p.damagecount + d).min(cap),
        ..p
    }
}

// End of tic: damage, damaging floors every 32 tics, use-key latch, exit flag, screen-tint timers.
pub fn finish(db: &Db, players: &VecRel<usize, Player>, damage: &Fold<usize, i32>, sectors: &VecRel<usize, Sector>, inp: Input, exited: bool, tic: u64) -> VecRel<usize, Player> {
    let k = consts(db);
    let sector = || players.map(|p: Player| p.sector).select(sectors);
    players
        .and(damage.opt())
        .and(sector())
        .and(sector().map(|s: Sector| s.special).select(&db.t.sector_damage).opt())
        .and(players.map(|p: Player| p.armor_class as usize).select(&db.t.armor_classes))
        .map(move |((((p, d), s), floor), ac): ((((Player, Option<i32>), Sector), Option<i32>), ArmorClass)| {
            let floor = if tic % k.floor_damage_period == 0 && p.z <= s.floor { floor.unwrap_or(0) } else { 0 };
            let p = hurt(p, d.unwrap_or(0) + floor, ac, k.damage_cap);
            Player {
                usedown: inp.use_,
                exited: p.exited || exited,
                damagecount: (p.damagecount - 1).max(0),
                bonuscount: (p.bonuscount - 1).max(0),
                ..p
            }
        })
        .collect()
}
