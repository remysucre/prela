use crate::db::{Db, Info, StateRow, Weapon};
use crate::info::{wfact, AmmoDef, PickupDef, Slide, WOut, PICKUP, WP_PISTOL};
use crate::rules::{bit, matching};
use crate::mobj::Mobj;
use crate::physics::{rnd, Cand, Clip, Shot};
use crate::wad::{Sector, NONE};
use prela::engine::*;
use std::f64::consts::TAU;

pub const RADIUS: f64 = 16.0;
pub const HEIGHT: f64 = 56.0;
pub const VIEW: f64 = 41.0;
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
    // A fresh player with a pistol (starting ammo comes from the ammo table).
    pub fn new(x: f64, y: f64, z: f64, ang: f64, sector: usize, wstate: usize) -> Player {
        Player {
            x,
            y,
            z,
            ang,
            momx: 0.0,
            momy: 0.0,
            momz: 0.0,
            sector,
            usedown: false,
            exited: false,
            dead: false,
            health: 100,
            armor: 0,
            armor_class: 0,
            owned: 0b11,
            keys: 0,
            weapon: WP_PISTOL,
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
fn thrust(p: Player, inp: Input) -> Player {
    if p.dead {
        return Player { momx: p.momx * 0.90625, momy: p.momy * 0.90625, ..p };
    }
    let ang = p.ang + inp.turn * if inp.run { 0.1227 } else { 0.0614 } + inp.look;
    let (s, c) = ang.sin_cos();
    let k = if inp.run { 1.5625 } else { 0.78125 };
    let fwd = inp.forward * k;
    let side = inp.strafe * k * 0.96;
    Player {
        ang,
        momx: (p.momx + fwd * c + side * s) * 0.90625,
        momy: (p.momy + fwd * s - side * c) * 0.90625,
        ..p
    }
}

// Applies thrust to the player.
pub fn moved(players: &VecRel<usize, Player>, inp: Input) -> VecRel<usize, Player> {
    players.map(move |p| thrust(p, inp)).collect()
}

// One candidate move per slide row (full move, x only, y only, stay); this gives wall sliding.
pub fn cands(db: &Db, players: &VecRel<usize, Player>) -> HashIdx<(usize, usize), Cand> {
    players
        .cross(&db.t.slides)
        .and(Same::<(usize, usize)>::new())
        .map(|((p, s), (who, idx)): ((Player, Slide), (usize, usize))| Cand {
            who,
            idx,
            x: p.x + s.keep_x * p.momx,
            y: p.y + s.keep_y * p.momy,
            z: p.z,
            radius: RADIUS,
            height: HEIGHT,
            monster: false,
            missile: false,
            stay: s.stay,
        })
        .collect()
}

// Takes the chosen candidate, keeps only the momentum its slide row keeps, applies gravity.
pub fn walk(db: &Db, players: &VecRel<usize, Player>, clips: &HashIdx<usize, Clip>) -> VecRel<usize, Player> {
    players
        .and(clips)
        .and(clips.map(|k: Clip| k.c.idx).select(&db.t.slides))
        .map(|((p, k), s): ((Player, Clip), Slide)| {
            let momz = if p.z > k.floor { p.momz - 1.0 } else { 0.0 };
            let z = (p.z + momz).max(k.floor);
            Player { x: k.c.x, y: k.c.y, z, momx: p.momx * s.keep_x, momy: p.momy * s.keep_y, momz: if z == k.floor { 0.0 } else { momz }, ..p }
        })
        .collect()
}

#[derive(Clone, Copy)]
// Weapon state while it's being advanced: the player, its ammo for the current weapon, the state to enter next, pellets fired.
struct Psp {
    p: Player,
    ammo: i32,
    pending: usize,
    pellets: usize,
}

// Enters the pending weapon state, or redirects to the attack state; firing spends ammo and starts the flash.
fn enter(q: Psp, r: StateRow, w: Weapon, o: WOut) -> Psp {
    if o.redirect {
        return Psp { pending: w.attack, ..q };
    }
    let fires = o.pellets > 0;
    let mut p = Player { wstate: q.pending, wtics: r.tics, ..q.p };
    if fires {
        p.flash = w.flash;
        p.flash_tics = -2;
    }
    Psp { p, ammo: q.ammo - w.per_shot * fires as i32, pending: if r.tics == 0 { r.next } else { NONE }, pellets: q.pellets.max(o.pellets) }
}

// One step of the weapon state machine: match weapon rules on the pending state and enter it.
fn psp_stage(db: &Db, q: &VecRel<usize, Psp>, fire: bool) -> VecRel<usize, Psp> {
    // Pending state and current weapon for the player, plus fact bits.
    let rows: HashIdx<usize, ((Psp, StateRow, Weapon), usize, u32)> = q
        .and(q.map(|q: Psp| q.pending).select(&db.t.states))
        .and(q.map(|q: Psp| q.p.weapon).select(&db.t.weapons))
        .map(move |((q, r), w): ((Psp, StateRow), Weapon)| {
            let facts = bit(fire, wfact::FIRE) | bit(!q.p.dead, wfact::ALIVE) | bit(q.ammo >= w.per_shot, wfact::AMMO);
            ((q, r, w), r.action as usize, facts)
        })
        .collect();
    // Best weapon rule for that state's action.
    let chosen = matching(&rows, &db.t.weapon_rules, |_| PLAYER);
    let entered = (&rows)
        .and(chosen.opt())
        .map(|(((q, r, w), _, _), o): (((Psp, StateRow, Weapon), usize, u32), Option<((Psp, StateRow, Weapon), WOut)>)| {
            enter(q, r, w, o.map_or(WOut::default(), |(_, o)| o))
        });
    // Nothing pending: pass through unchanged.
    q.minus(&rows).union(entered).collect()
}

// Advances the muzzle flash animation by one step.
fn flash_stage(db: &Db, players: &VecRel<usize, Player>) -> VecRel<usize, Player> {
    players
        .and(players.map(|p: Player| p.flash).select(&db.t.states))
        .map(|(p, r): (Player, StateRow)| match p.flash_tics {
            -2 => Player { flash_tics: r.tics, ..p },
            _ if p.flash == 0 => p,
            t if t <= 1 => Player { flash: r.next, flash_tics: -2, ..p },
            t => Player { flash_tics: t - 1, ..p },
        })
        .collect()
}

// Weapon logic for this tic (switching, animation, firing); returns the updated player, ammo and bullets.
pub fn weapon(db: &Db, players: &VecRel<usize, Player>, ammo: &VecRel<usize, i32>, inp: Input, tic: u64) -> (VecRel<usize, Player>, VecRel<usize, i32>, HashIdx<usize, Shot>) {
    let target = move |p: Player| inp.weapon.unwrap_or(p.wanted);
    // Player with current weapon, requested weapon and current weapon state.
    let rows = players
        .and(players.map(|p: Player| p.weapon).select(&db.t.weapons))
        .and(players.map(target).select(&db.t.weapons).opt())
        .and(players.map(|p: Player| p.wstate).select(&db.t.states));
    let wants = move |p: Player, new: Option<Weapon>| new.is_some() && target(p) != p.weapon && p.owned & (1 << target(p)) != 0;
    let switching = move |(((p, cur), new), _): (((Player, Weapon), Option<Weapon>), StateRow)| wants(p, new) && p.wstate == cur.ready;
    // Switch weapons if asked and ready, else remember the request and tick down the current state's timer.
    let switched: VecRel<usize, Psp> = (&rows)
        .filt(switching)
        .flat_map(move |(((p, _), new), _): (((Player, Weapon), Option<Weapon>), StateRow)| {
            new.map(|w| Psp { p: Player { weapon: target(p), wanted: NONE, ..p }, ammo: 0, pending: w.ready, pellets: 0 })
        })
        .union((&rows).filt(move |r| !switching(r)).map(move |(((p, _), new), row): (((Player, Weapon), Option<Weapon>), StateRow)| {
            let wtics = p.wtics - 1;
            let wanted = if wants(p, new) { target(p) } else { p.wanted };
            Psp { p: Player { wtics, wanted, ..p }, ammo: 0, pending: if wtics <= 0 { row.next } else { NONE }, pellets: 0 }
        }))
        .collect();
    // Ammo count for the (possibly new) weapon.
    let mut q: VecRel<usize, Psp> = (&switched)
        .and((&switched).map(|q: Psp| q.p.weapon).select(&db.t.weapons).map(|w: Weapon| w.ammo).select(ammo))
        .map(|(q, a): (Psp, i32)| Psp { ammo: a, ..q })
        .collect();
    // Follow up to 4 zero-length state transitions this tic.
    for _ in 0..4 {
        q = psp_stage(db, &q, inp.fire);
    }
    // Settle any state still pending.
    let fired: VecRel<usize, Psp> = q.map(|q: Psp| if q.pending != NONE { Psp { p: Player { wstate: q.pending, wtics: 1, ..q.p }, ..q } } else { q }).collect();
    // One bullet per pellet fired, with random spread and damage.
    let shots = (&fired)
        .flat_map(move |q: Psp| {
            let p = q.p;
            (0..q.pellets).map(move |k| {
                let spread = (rnd(tic, k, 1) - rnd(tic, k, 2)) as f64 * TAU / 16384.0;
                Shot { id: (1 << 23) | k, from: PLAYER, x: p.x, y: p.y, z: p.z + 32.0, ang: p.ang + spread, range: 2048.0, damage: 5 * (rnd(tic, k, 3) % 3 + 1) }
            })
        })
        .key_by(|s: Shot| s.id)
        .collect();
    // Write the spent count back to its ammo kind.
    let spent: HashIdx<usize, i32> = (&fired)
        .and((&fired).map(|q: Psp| q.p.weapon).select(&db.t.weapons))
        .key_by(|(_, w): (Psp, Weapon)| w.ammo)
        .map(|(q, _): (Psp, Weapon)| q.ammo)
        .collect();
    let ammo = ammo.minus(&spent).union(&spent).collect();
    let players = (&fired).map(|q: Psp| q.p).collect();
    (flash_stage(db, &flash_stage(db, &players)), ammo, shots)
}

// A pickup's effect on the player's own fields; health and armor only ever go up, toward the item's max.
fn gain(p: Player, d: PickupDef) -> Player {
    Player {
        health: p.health.max((p.health + d.health).min(d.health_max)),
        armor: p.armor.max((p.armor + d.armor).min(d.armor_max)),
        armor_class: (if d.class_set > 0 { d.class_set } else { p.armor_class }).max(d.class_min),
        owned: p.owned | d.weapon_bit,
        wanted: if d.weapon_bit & !p.owned != 0 { d.weapon } else { p.wanted },
        keys: p.keys | d.key,
        bonuscount: 6,
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
    // Items overlapping the player, with the ammo each would add (capped).
    let touching: HashIdx<usize, (Mobj, PickupDef, Player, i32)> = mobjs
        .filt(|m: Mobj| m.flags & PICKUP != 0)
        .key_by(|m: Mobj| m.kind)
        .and(&db.t.infos)
        .and(&db.t.pickups)
        .key_by(|_| PLAYER)
        .and(players)
        .filt(|(((m, i), _), p): (((Mobj, Info), PickupDef), Player)| {
            !p.dead && (m.x - p.x).abs() < RADIUS + i.radius && (m.y - p.y).abs() < RADIUS + i.radius && m.z <= p.z + HEIGHT && m.z + 16.0 >= p.z
        })
        .key_by(|((_, d), _): (((Mobj, Info), PickupDef), Player)| d.ammo_kind)
        .and(ammo.and(&db.t.ammo_defs))
        .map(|((((m, _), d), p), (a, k)): ((((Mobj, Info), PickupDef), Player), (i32, AmmoDef))| (m, d, p, (a + d.ammo).min(k.cap) - a))
        .filt(|(_, d, p, more): (Mobj, PickupDef, Player, i32)| {
            let g = gain(p, d);
            d.always || more > 0 || g.health > p.health || g.armor > p.armor
        })
        .key_by(|_| PLAYER)
        .collect();
    // Apply the player effects in turn.
    let gained = (&touching).fold(None, |a: Option<Player>, (_, d, p, _): (Mobj, PickupDef, Player, i32)| Some(gain(a.unwrap_or(p), d)));
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
pub fn hurt(p: Player, d: i32) -> Player {
    if p.dead || d <= 0 {
        return p;
    }
    let saved = match p.armor_class {
        1 => d / 3,
        2 => d / 2,
        _ => 0,
    }
    .min(p.armor);
    let health = p.health - (d - saved);
    Player {
        armor: p.armor - saved,
        armor_class: if p.armor - saved == 0 { 0 } else { p.armor_class },
        health: health.max(0),
        dead: health <= 0,
        damagecount: (p.damagecount + d).min(100),
        ..p
    }
}

// End of tic: damage, damaging floors every 32 tics, use-key latch, exit flag, screen-tint timers.
pub fn finish(db: &Db, players: &VecRel<usize, Player>, damage: &Fold<usize, i32>, sectors: &VecRel<usize, Sector>, inp: Input, exited: bool, tic: u64) -> VecRel<usize, Player> {
    let sector = || players.map(|p: Player| p.sector).select(sectors);
    players
        .and(damage.opt())
        .and(sector())
        .and(sector().map(|s: Sector| s.special).select(&db.t.sector_damage).opt())
        .map(move |(((p, d), s), floor): (((Player, Option<i32>), Sector), Option<i32>)| {
            let floor = if tic % 32 == 0 && p.z <= s.floor { floor.unwrap_or(0) } else { 0 };
            let p = hurt(p, d.unwrap_or(0) + floor);
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
