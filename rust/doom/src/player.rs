use crate::db::{Db, Info, StateRow, Weapon};
use crate::info::{wfact, Pickup, WOut, AM_CLIP, AM_NONE, MAX_AMMO, PICKUP, WP_PISTOL};
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
    pub ammo: [i32; 4],
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
            ammo: [50, 0, 0, 0],
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

pub fn moved(players: &VecRel<usize, Player>, inp: Input) -> VecRel<usize, Player> {
    players.map(move |p| thrust(p, inp)).collect()
}

pub fn cands(players: &VecRel<usize, Player>) -> HashIdx<(usize, usize), Cand> {
    players
        .flat_map(|p: Player| {
            [(p.x + p.momx, p.y + p.momy), (p.x + p.momx, p.y), (p.x, p.y + p.momy), (p.x, p.y)]
                .into_iter()
                .enumerate()
                .map(move |(idx, (x, y))| Cand {
                    who: PLAYER,
                    idx,
                    x,
                    y,
                    z: p.z,
                    radius: RADIUS,
                    height: HEIGHT,
                    monster: false,
                    missile: false,
                    stay: idx == 3,
                })
        })
        .key_by(|c: Cand| (c.who, c.idx))
        .collect()
}

pub fn walk(players: &VecRel<usize, Player>, clips: &HashIdx<usize, Clip>) -> VecRel<usize, Player> {
    players
        .and(clips)
        .map(|(p, k): (Player, Clip)| {
            let (momx, momy) = match k.c.idx {
                0 => (p.momx, p.momy),
                1 => (p.momx, 0.0),
                2 => (0.0, p.momy),
                _ => (0.0, 0.0),
            };
            let momz = if p.z > k.floor { p.momz - 1.0 } else { 0.0 };
            let z = (p.z + momz).max(k.floor);
            Player { x: k.c.x, y: k.c.y, z, momx, momy, momz: if z == k.floor { 0.0 } else { momz }, ..p }
        })
        .collect()
}

#[derive(Clone, Copy)]
struct Psp {
    p: Player,
    pending: usize,
    pellets: usize,
}

fn can_fire(p: Player, w: Weapon) -> bool {
    w.ammo == AM_NONE || p.ammo[w.ammo] >= w.per_shot
}

fn enter(q: Psp, r: StateRow, w: Weapon, o: WOut) -> Psp {
    if o.redirect {
        return Psp { pending: w.attack, ..q };
    }
    let fires = o.pellets > 0;
    let mut p = Player { wstate: q.pending, wtics: r.tics, ..q.p };
    if fires && w.ammo != AM_NONE {
        p.ammo[w.ammo] -= w.per_shot;
    }
    if fires {
        p.flash = w.flash;
        p.flash_tics = -2;
    }
    Psp { p, pending: if r.tics == 0 { r.next } else { NONE }, pellets: q.pellets.max(o.pellets) }
}

fn psp_stage(db: &Db, q: &VecRel<usize, Psp>, fire: bool) -> VecRel<usize, Psp> {
    let rows: HashIdx<usize, ((Psp, StateRow, Weapon), usize, u32)> = q
        .and(q.map(|q: Psp| q.pending).select(&db.t.states))
        .and(q.map(|q: Psp| q.p.weapon).select(&db.t.weapons))
        .map(move |((q, r), w): ((Psp, StateRow), Weapon)| {
            let facts = bit(fire, wfact::FIRE) | bit(!q.p.dead, wfact::ALIVE) | bit(can_fire(q.p, w), wfact::AMMO);
            ((q, r, w), r.action as usize, facts)
        })
        .collect();
    let chosen = matching(&rows, &db.t.weapon_rules, |_| PLAYER);
    let entered = (&rows)
        .and(chosen.opt())
        .map(|(((q, r, w), _, _), o): (((Psp, StateRow, Weapon), usize, u32), Option<((Psp, StateRow, Weapon), WOut)>)| {
            enter(q, r, w, o.map_or(WOut::default(), |(_, o)| o))
        });
    q.minus(&rows).union(entered).collect()
}

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

pub fn weapon(db: &Db, players: &VecRel<usize, Player>, inp: Input, tic: u64) -> (VecRel<usize, Player>, HashIdx<usize, Shot>) {
    let target = move |p: Player| inp.weapon.unwrap_or(p.wanted);
    let rows = players
        .and(players.map(|p: Player| p.weapon).select(&db.t.weapons))
        .and(players.map(target).select(&db.t.weapons).opt())
        .and(players.map(|p: Player| p.wstate).select(&db.t.states));
    let switching = move |(((p, cur), new), _): (((Player, Weapon), Option<Weapon>), StateRow)| {
        new.is_some() && target(p) != p.weapon && p.owned & (1 << target(p)) != 0 && p.wstate == cur.ready
    };
    let switched = (&rows)
        .filt(switching)
        .flat_map(move |(((p, _), new), _): (((Player, Weapon), Option<Weapon>), StateRow)| {
            new.map(|w| Psp { p: Player { weapon: target(p), wanted: NONE, ..p }, pending: w.ready, pellets: 0 })
        })
        .union((&rows).filt(move |r| !switching(r)).map(|(((p, _), _), row): (((Player, Weapon), Option<Weapon>), StateRow)| {
            let wtics = p.wtics - 1;
            Psp { p: Player { wtics, ..p }, pending: if wtics <= 0 { row.next } else { NONE }, pellets: 0 }
        }))
        .collect();
    let mut q = switched;
    for _ in 0..4 {
        q = psp_stage(db, &q, inp.fire);
    }
    let fired: VecRel<usize, Psp> = q.map(|q: Psp| if q.pending != NONE { Psp { p: Player { wstate: q.pending, wtics: 1, ..q.p }, ..q } } else { q }).collect();
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
    let players = (&fired).map(|q: Psp| q.p).collect();
    (flash_stage(db, &flash_stage(db, &players)), shots)
}

fn applicable(p: Player, pk: Pickup) -> bool {
    match pk {
        Pickup::Health { max, .. } => p.health < max,
        Pickup::Armor { bonus: true, .. } => p.armor < 200,
        Pickup::Armor { amount, .. } => p.armor < amount,
        Pickup::Ammo { kind, .. } => p.ammo[kind] < MAX_AMMO[kind],
        _ => true,
    }
}

fn apply(p: Player, pk: Pickup) -> Player {
    let mut p = Player { bonuscount: 6, ..p };
    match pk {
        Pickup::Health { amount, max } => p.health = (p.health + amount).min(max),
        Pickup::Armor { bonus: true, .. } => {
            p.armor = (p.armor + 1).min(200);
            p.armor_class = p.armor_class.max(1);
        }
        Pickup::Armor { amount, class, .. } => {
            p.armor = amount;
            p.armor_class = class;
        }
        Pickup::Ammo { kind, amount } => p.ammo[kind] = (p.ammo[kind] + amount).min(MAX_AMMO[kind]),
        Pickup::Weapon { weapon, kind, amount } => {
            if p.owned & (1 << weapon) == 0 {
                p.wanted = weapon;
            }
            p.owned |= 1 << weapon;
            p.ammo[kind] = (p.ammo[kind] + amount).min(MAX_AMMO[kind]);
        }
        Pickup::Key(k) => p.keys |= k,
        Pickup::Backpack => p.ammo[AM_CLIP] = (p.ammo[AM_CLIP] + 10).min(MAX_AMMO[AM_CLIP]),
        Pickup::None => {}
    }
    p
}

pub fn pickups(db: &Db, players: &VecRel<usize, Player>, mobjs: &HashIdx<usize, Mobj>) -> (VecRel<usize, Player>, Fold<usize, bool>) {
    let touching: HashIdx<usize, (Mobj, Pickup, Player)> = mobjs
        .filt(|m: Mobj| m.flags & PICKUP != 0)
        .key_by(|m: Mobj| m.kind)
        .and(&db.t.infos)
        .key_by(|_| PLAYER)
        .and(players)
        .filt(|((m, i), p): ((Mobj, Info), Player)| {
            !p.dead && (m.x - p.x).abs() < RADIUS + i.radius && (m.y - p.y).abs() < RADIUS + i.radius && m.z <= p.z + HEIGHT && m.z + 16.0 >= p.z
        })
        .filt(|((_, i), p): ((Mobj, Info), Player)| applicable(p, i.pickup))
        .map(|((m, i), p): ((Mobj, Info), Player)| (m, i.pickup, p))
        .collect();
    let gained = (&touching).fold(None, |a: Option<Player>, (_, k, p): (Mobj, Pickup, Player)| Some(apply(a.unwrap_or(p), k)));
    let applied = players.and(gained.opt()).map(|(p, g): (Player, Option<Option<Player>>)| g.flatten().unwrap_or(p)).collect();
    let taken = (&touching).key_by(|(m, _, _): (Mobj, Pickup, Player)| m.id).fold(true, |_, _| true);
    (applied, taken)
}

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

pub fn finish(players: &VecRel<usize, Player>, damage: &Fold<usize, i32>, sectors: &VecRel<usize, Sector>, inp: Input, exited: bool, tic: u64) -> VecRel<usize, Player> {
    players
        .and(damage.opt())
        .and(players.map(|p: Player| p.sector).select(sectors))
        .map(move |((p, d), s): ((Player, Option<i32>), Sector)| {
            let floor = match s.special {
                5 => 10,
                7 => 5,
                4 | 16 => 20,
                _ => 0,
            };
            let floor = if tic % 32 == 0 && p.z <= s.floor { floor } else { 0 };
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
