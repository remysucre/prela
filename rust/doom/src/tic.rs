use crate::db::Db;
use crate::mobj::{self, Decision, Mobj};
use crate::physics::{clip_moves, hitscan, line_rows, locate, opening, visible, LineRow, Shot};
use crate::player::{self, Input, Player, PLAYER};
use crate::wad::{Sector, Thing};
use crate::world::{self, Mover};
use prela::engine::*;

const ML_SOUNDBLOCK: i64 = 64;

pub struct State {
    pub tic: u64,
    pub player: VecRel<usize, Player>,
    pub sectors: VecRel<usize, Sector>,
    pub movers: VecRel<usize, Mover>,
    pub mobjs: HashIdx<usize, Mobj>,
    pub heard: HashIdx<usize, bool>,
}

fn with_sector(db: &Db, players: &VecRel<usize, Player>) -> VecRel<usize, Player> {
    let here: VecRel<usize, (f64, f64)> = players.map(|p: Player| (p.x, p.y)).collect();
    let sec: HashIdx<usize, usize> = locate(db, &here).collect();
    players.and(&sec).map(|(p, s): (Player, usize)| Player { sector: s, ..p }).collect()
}

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
    let movers = (&sectors).map(|_| world::IDLE).collect();
    let mobjs = mobj::spawn_things(db, &sectors);
    State { tic: 0, player, sectors, movers, mobjs, heard: HashIdx { idx: Default::default() } }
}

pub fn place(db: &Db, s: State, x: f64, y: f64, ang: f64) -> State {
    let moved: VecRel<usize, Player> = (&s.player).map(move |p: Player| Player { x, y, ang, ..p }).collect();
    let moved = with_sector(db, &moved);
    let player = (&moved)
        .and((&moved).map(|p: Player| p.sector).select(&s.sectors))
        .map(|(p, sec): (Player, Sector)| Player { z: sec.floor, ..p })
        .collect();
    State { player, ..s }
}

pub fn tic(db: &Db, s: &State, inp: Input) -> State {
    let t = s.tic;
    let lines = line_rows(db, &s.sectors);
    let bodies = mobj::bodies(db, &s.player, &s.mobjs);

    let moved = player::moved(&s.player, inp);
    let walked = player::walk(&moved, &clip_moves(db, &s.sectors, &lines, &bodies, &player::cands(&moved)));
    let walked = with_sector(db, &walked);

    let events = world::line_events(db, &lines, &s.player, &walked, inp.use_);
    let occupied = (&s.mobjs)
        .filt(|m: Mobj| m.flags & crate::info::SOLID != 0)
        .map(|m: Mobj| m.sector)
        .union((&walked).map(|p: Player| p.sector))
        .key_by(|sec| sec)
        .fold(true, |_, _| true);
    let (sectors, movers) = world::movers(db, &s.sectors, &s.movers, &events, &occupied);

    let (armed, player_shots) = player::weapon(db, &walked, inp, t);

    let sound: MultiRel<usize, usize> = (&lines)
        .filt(|row: LineRow| {
            let (lo, hi) = opening(row);
            hi > lo && row.0.0.flags & ML_SOUNDBLOCK == 0
        })
        .flat_map(|((l, _), _): LineRow| [(l.fsec, l.bsec), (l.bsec, l.fsec)])
        .key_by(|(a, _)| a)
        .map(|(_, b)| b)
        .collect();
    let heard: HashIdx<usize, bool> = (&player_shots)
        .key_by(|_| PLAYER)
        .and(&armed)
        .map(|(_, p): (Shot, Player)| p.sector)
        .reach_set(&sound, usize::MAX)
        .map(|_| true)
        .union(&s.heard)
        .fold(true, |_, _| true)
        .collect();
    let advanced = mobj::advance(db, &s.mobjs);
    let seen = visible(&lines, &mobj::sight_pairs(&advanced, &armed));
    let decided = mobj::think(db, &advanced, &seen, &heard, &armed, t);
    let bodies = mobj::bodies(db, &armed, &(&decided).map(|d: Decision| d.m).collect());
    let clips = clip_moves(db, &sectors, &lines, &bodies, &mobj::move_cands(db, &decided));
    let moved = mobj::apply_moves(db, &decided, &clips, &bodies);

    let shots: HashIdx<usize, Shot> = (&player_shots).union(&mobj::monster_shots(&moved, t)).collect();
    let impacts = hitscan(&lines, &bodies, &shots);
    let dmg = mobj::damage(&impacts, &moved, &bodies, t);
    let wounded = mobj::wound(db, &moved, &dmg, t);
    let dropped = mobj::drops(db, &moved, &wounded, t);
    let fx = mobj::effects(db, &impacts, &wounded, &armed, t);

    let (picked, taken) = player::pickups(db, &armed, &(&wounded).map(|d: Decision| d.m).collect());
    let player = player::finish(&picked, &dmg, &sectors, inp, world::exited(&events), t);
    let mobjs = mobj::relocate(db, &sectors, &mobj::survivors(&wounded, &taken, &dropped, &fx));
    State { tic: t + 1, player, sectors, movers, mobjs, heard }
}
