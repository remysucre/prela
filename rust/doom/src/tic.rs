use crate::db::Db;
use crate::info::AmmoDef;
use crate::mobj::{self, Decision, Mobj};
use crate::physics::{clip_moves, hitscan, line_rows, locate, opening, visible, LineRow, Shot};
use crate::player::{self, Input, Player, PLAYER};
use crate::wad::{Sector, Thing};
use crate::world::{self, Mover};
use prela::engine::*;

const ML_SOUNDBLOCK: i64 = 64;

// The whole game state; each tic builds a new one.
pub struct State {
    pub tic: u64,
    pub player: VecRel<usize, Player>,
    pub ammo: VecRel<usize, i32>,
    pub sectors: VecRel<usize, Sector>,
    pub movers: VecRel<usize, Mover>,
    pub mobjs: HashIdx<usize, Mobj>,
    pub heard: HashIdx<usize, bool>,
}

// Fill in each player's current sector.
fn with_sector(db: &Db, players: &VecRel<usize, Player>) -> VecRel<usize, Player> {
    let here: VecRel<usize, (f64, f64)> = players.map(|p: Player| (p.x, p.y)).collect();
    let sec: HashIdx<usize, usize> = locate(db, &here).collect();
    players.and(&sec).map(|(p, s): (Player, usize)| Player { sector: s, ..p }).collect()
}

// Starting state: player at the start thing, standing on its sector floor.
pub fn initial(db: &Db, sectors: VecRel<usize, Sector>) -> State {
    let pistol = db.t.weapons.get(crate::info::WP_PISTOL).unwrap().ready;
    let start = (&db.things)
        .filt(|t: Thing| t.kind == 1)
        .map(move |t: Thing| Player::new(t.x, t.y, 0.0, t.angle.to_radians(), 0, pistol))
        .key_by(|_| 0usize)
        .collect();
    let player = with_sector(db, &start);
    let player = (&player)
        .and((&player).map(|p: Player| p.sector).select(&sectors))
        .map(|(p, s): (Player, Sector)| Player { z: s.floor, ..p })
        .collect();
    // No doors or lifts moving yet.
    let movers = (&sectors).map(|_| world::IDLE).collect();
    let mobjs = mobj::spawn_things(db, &sectors);
    let ammo = (&db.t.ammo_defs).map(|a: AmmoDef| a.start).collect();
    State { tic: 0, player, ammo, sectors, movers, mobjs, heard: HashIdx { idx: Default::default() } }
}

// Teleport the player (debug `--at`).
pub fn place(db: &Db, s: State, x: f64, y: f64, ang: f64) -> State {
    let moved: VecRel<usize, Player> = (&s.player).map(move |p: Player| Player { x, y, ang, ..p }).collect();
    let moved = with_sector(db, &moved);
    let player = (&moved)
        .and((&moved).map(|p: Player| p.sector).select(&s.sectors))
        .map(|(p, sec): (Player, Sector)| Player { z: sec.floor, ..p })
        .collect();
    State { player, ..s }
}

// One game tic: old state + input -> new state.
pub fn tic(db: &Db, s: &State, inp: Input) -> State {
    let t = s.tic;
    let lines = line_rows(db, &s.sectors);
    let bodies = mobj::bodies(db, &s.player, &s.mobjs);

    // Player movement: apply input, try candidate moves, keep the first that fits.
    let moved = player::moved(&s.player, inp);
    let walked = player::walk(db, &moved, &clip_moves(db, &s.sectors, &lines, &bodies, &player::cands(db, &moved)));
    let walked = with_sector(db, &walked);

    // Lines walked over or used this tic.
    let events = world::line_events(db, &lines, &s.player, &walked, inp.use_);
    // Sectors with something solid in them (doors won't close on them).
    let occupied = (&s.mobjs)
        .filt(|m: Mobj| m.flags & crate::info::SOLID != 0)
        .map(|m: Mobj| m.sector)
        .union((&walked).map(|p: Player| p.sector))
        .key_by(|sec| sec)
        .fold(true, |_, _| true);
    let (sectors, movers) = world::movers(db, &s.sectors, &s.movers, &events, &occupied);

    // Weapon state machine and any bullets fired.
    let (armed, ammo, player_shots) = player::weapon(db, &walked, &s.ammo, inp, t);

    // Sector adjacency that sound can travel through.
    let sound: MultiRel<usize, usize> = (&lines)
        .filt(|row: LineRow| {
            let (lo, hi) = opening(row);
            hi > lo && row.0.0.flags & ML_SOUNDBLOCK == 0
        })
        .flat_map(|((l, _), _): LineRow| [(l.fsec, l.bsec), (l.bsec, l.fsec)])
        .key_by(|(a, _)| a)
        .map(|(_, b)| b)
        .collect();
    // Sectors that have heard the player fire (sticky across tics).
    let heard: HashIdx<usize, bool> = (&player_shots)
        .key_by(|_| PLAYER)
        .and(&armed)
        .map(|(_, p): (Shot, Player)| p.sector)
        .reach_set(&sound, usize::MAX)
        .map(|_| true)
        .union(&s.heard)
        .fold(true, |_, _| true)
        .collect();
    // Monsters: advance animation, check sight, decide, then move.
    let advanced = mobj::advance(db, &s.mobjs);
    let seen = visible(&lines, &mobj::sight_pairs(&advanced, &armed));
    let decided = mobj::think(db, &advanced, &seen, &heard, &armed, t);
    let bodies = mobj::bodies(db, &armed, &(&decided).map(|d: Decision| d.m).collect());
    let clips = clip_moves(db, &sectors, &lines, &bodies, &mobj::move_cands(db, &decided));
    let moved = mobj::apply_moves(db, &decided, &clips, &bodies);

    // All bullets, their hits, and the resulting damage, deaths, drops and puffs.
    let shots: HashIdx<usize, Shot> = (&player_shots).union(&mobj::monster_shots(&moved, t)).collect();
    let impacts = hitscan(&lines, &bodies, &shots);
    let dmg = mobj::damage(&impacts, &moved, &bodies, t);
    let wounded = mobj::wound(db, &moved, &dmg, t);
    let dropped = mobj::drops(db, &moved, &wounded, t);
    let fx = mobj::effects(db, &impacts, &wounded, &armed, t);

    // Pickups, final player update, and the new mobj set.
    let (picked, ammo, taken) = player::pickups(db, &armed, &ammo, &(&wounded).map(|d: Decision| d.m).collect());
    let player = player::finish(db, &picked, &dmg, &sectors, inp, world::exited(&events), t);
    let mobjs = mobj::relocate(db, &sectors, &mobj::survivors(&wounded, &taken, &dropped, &fx));
    State { tic: t + 1, player, ammo, sectors, movers, mobjs, heard }
}
