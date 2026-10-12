use crate::db::Db;
use crate::info::{MV_IDLE, SOLID};
use crate::mobj;
use crate::physics::{clip_moves, hitscan, lines, locate, visible, Shot};
use crate::player::{self, Input};
use crate::schema::{one, rc, AmmoKind, Mobj, Player, Sector, SectorState, C};
use crate::wad::NONE;
use crate::world;
use prela::engine::*;
use prela::loader::Key;

// Linedef flag: sound stops here. Thing type of the player 1 start.
const ML_SOUNDBLOCK: i64 = 64;
const PLAYER1_START: usize = 1;

// The whole game state; each tic builds a new one, sharing the columns it doesn't change.
pub struct State {
    pub tic: u64,
    pub player: Player,
    pub ammo: C<AmmoKind, i32>,
    pub sector: SectorState,
    pub mobj: Mobj,
}

// A one-row player column holding v.
fn konst<T: Copy + 'static>(id: &Key<Player>, v: T) -> C<Player, T> {
    rc(id.map(move |_| v))
}

// Fill in the player's current sector.
fn with_sector(db: &Db, p: Player) -> Player {
    let here = (&*p.x).and(&*p.y);
    // player -> sector, via the BSP walk.
    Player { sector: rc(locate(db, &here)), ..p }
}

// Put the player on the floor of its sector.
fn on_floor(s: &SectorState, p: Player) -> Player {
    Player { z: rc((&*p.sector).select(&*s.floor)), ..p }
}

// Starting state: player at the start thing, standing on its sector floor; nothing moving.
pub fn initial(db: &Db) -> State {
    let fill = |v| rc((&db.sector).map(move |_| v));
    let sector = SectorState {
        floor: rc(&db.sector.floor),
        ceil: rc(&db.sector.ceil),
        kind: rc((&db.sector).map(|_| MV_IDLE)),
        plane: rc((&db.sector).map(|_| 0u8)),
        dir: rc((&db.sector).map(|_| 0i8)),
        speed: fill(0.0),
        up: fill(0.0),
        down: fill(0.0),
        wait: rc((&db.sector).map(|_| 0i32)),
        heard: rc((&db.sector).map(|_| false)),
    };
    let (k, th) = (&db.t.k, &db.thing);
    let weapon = one(&k.start_weapon);
    // The player 1 start thing, as the player's row.
    let start: VecRel<Id<Player>, ((f64, f64), f64)> = (&th.kind)
        .filt(|kind: usize| kind == PLAYER1_START)
        .and((&th.x).and(&th.y).and(&th.angle))
        .key_by(|_| Id::<Player>::new(0))
        .map(|(_, p): (usize, ((f64, f64), f64))| p)
        .collect();
    let id = Universe::new(1);
    let player = Player {
        x: rc((&start).map(|((x, _), _): ((f64, f64), f64)| x)),
        y: rc((&start).map(|((_, y), _): ((f64, f64), f64)| y)),
        z: konst(&id, 0.0),
        ang: rc((&start).map(|(_, a): ((f64, f64), f64)| a.to_radians())),
        momx: konst(&id, 0.0),
        momy: konst(&id, 0.0),
        momz: konst(&id, 0.0),
        sector: rc((&id).map(|_| Id::<Sector>::new(0))),
        usedown: konst(&id, false),
        exited: konst(&id, false),
        dead: konst(&id, false),
        health: konst(&id, one(&k.start_health)),
        armor: konst(&id, 0),
        armor_class: rc((&id).map(|_| Id::new(0))),
        owned: konst(&id, one(&k.start_owned)),
        keys: konst(&id, 0),
        weapon: rc((&id).map(move |_| weapon)),
        wanted: rc((&id).map(|_| Id::new(NONE))),
        wstate: rc((&id).map(move |_| weapon).select(&db.t.weapon.ready)),
        wtics: konst(&id, 1),
        flash: rc((&id).map(|_| Id::new(0))),
        flash_tics: konst(&id, 0),
        damagecount: konst(&id, 0),
        bonuscount: konst(&id, 0),
        id,
    };
    let player = on_floor(&sector, with_sector(db, player));
    let mobj = mobj::spawn_things(db, &sector);
    State { tic: 0, player, ammo: rc(&db.t.ammo.start), sector, mobj }
}

// Teleport the player (debug `--at`).
pub fn place(db: &Db, s: State, x: f64, y: f64, ang: f64) -> State {
    let p = &s.player;
    let moved = Player { x: rc((&p.id).map(move |_| x)), y: rc((&p.id).map(move |_| y)), ang: rc((&p.id).map(move |_| ang)), ..p.clone() };
    let player = on_floor(&s.sector, with_sector(db, moved));
    State { player, ..s }
}

// One game tic: old state + input -> new state.
pub fn tic(db: &Db, s: &State, inp: Input) -> State {
    let p = &s.player;
    // Dead or finished, and use freshly pressed: restart the level.
    // `usedown` is last tic's use key, so holding use doesn't restart twice.
    let restart = (&*p.dead).and(&*p.exited).and(&*p.usedown).filt(move |((d, e), held): ((bool, bool), bool)| (d || e) && inp.use_ && !held).fold_flat(false, |_, _| true);
    if restart {
        return initial(db);
    }
    let t = s.tic;
    // Inputs shared by several steps: line openings at the start of the tic, and every solid/shootable box.
    let lines = lines(db, &s.sector);
    let everyone = rc((&s.mobj.id).map(|_| true));
    let bodies = mobj::bodies(db, p, &s.mobj, &everyone);

    // Player movement: apply input, try candidate moves, keep the first that fits.
    let moved = player::moved(db, p, inp);
    let walked = with_sector(db, player::walk(db, &moved, &clip_moves(db, &s.sector, &lines, &bodies, &player::cands(db, &moved))));

    // Lines walked over or used this tic.
    let events = world::line_events(db, &lines, p, &walked, inp.use_);
    // Sectors with something solid in them (doors won't close on them).
    let m = &s.mobj;
    let occupied = (&*m.flags)
        .filt(|f: u32| f & SOLID != 0)
        .and(&*m.sector)
        .map(|(_, sec)| sec)
        .key_by(|sec| sec)
        .union((&*walked.sector).key_by(|sec| sec))
        .dense_fold(db.sector.id.n, true, |_, _| true);
    let sector = world::movers(db, &s.sector, &events, &occupied);

    // Weapon state machine and any bullets fired.
    let (armed, ammo, player_shots) = player::weapon(db, &walked, &s.ammo, inp, t);

    // Sector adjacency that sound can travel through.
    // Sound passes lines with an open gap and no sound-block flag; edges go both ways.
    let l = &db.linedef;
    let sound: MultiRel<Id<Sector>, Id<Sector>> = (&lines.lo)
        .and(&lines.hi)
        .and(&l.flags)
        .filt(|((lo, hi), flags): ((f64, f64), i64)| hi > lo && flags & ML_SOUNDBLOCK == 0)
        .and((&l.fsec).and(&l.bsec))
        .flat_map(|(_, (f, b)): (((f64, f64), i64), (Id<Sector>, Id<Sector>))| [(f, b), (b, f)])
        .key_by(|(a, _)| a)
        .map(|(_, b)| b)
        .collect();
    // Sectors that have heard the player fire (sticky across tics).
    let reached: MultiRel<Id<Sector>, usize> = (&player_shots)
        // Player's sector, once per bullet fired (none if no shot).
        .key_by(|_| Id::<Player>::new(0))
        .and(&*armed.sector)
        .map(|(_, sec): (Shot, Id<Sector>)| sec)
        // Flood out through the sound graph: sector -> depth.
        .reach_set(&sound, usize::MAX)
        .collect();
    let sector = SectorState { heard: rc((&*s.sector.heard).and((&reached).opt()).map(|(h, r): (bool, Option<usize>)| h || r.is_some())), ..sector };
    // Monsters: advance animation, check sight, decide, then move.
    let (advanced, action, live) = mobj::advance(db, m);
    let seen = visible(db, &lines, &mobj::sight_pairs(db, &advanced, &action, &live, &armed));
    let plan = mobj::think(db, &advanced, &action, &live, &seen, &sector, &armed, t);
    // Bodies rebuilt from post-move player and post-AI monsters for monster collision and hits.
    let bodies = mobj::bodies(db, &armed, &plan.m, &plan.live);
    let clips = clip_moves(db, &sector, &lines, &bodies, &mobj::move_cands(db, &plan));
    let moved = mobj::apply_moves(db, &plan, &clips, &bodies);

    // All bullets, their hits, and the resulting damage, deaths, drops and puffs.
    let shots: MultiRel<usize, Shot> = (&player_shots).union(&mobj::monster_shots(db, &moved, t)).collect();
    let impacts = hitscan(db, &lines, &bodies, &shots);
    let dmg = mobj::damage(db, &impacts, &moved, &bodies, t);
    let wounded = mobj::wound(db, &moved, &dmg, t);
    let spawns: MultiRel<usize, mobj::Spawn> = (&mobj::drops(db, &moved, &wounded, t)).union(&mobj::effects(db, &impacts, &wounded, &armed, t)).collect();

    // Pickups, final player update, and the new mobj set.
    let (picked, ammo, taken) = player::pickups(db, &armed, &ammo, &wounded.m, &wounded.live);
    let player = player::finish(db, &picked, &dmg, &sector, inp, world::exited(db, &events), t);
    let mobj = mobj::survivors(db, &sector, &wounded, &taken, &spawns);
    State { tic: t + 1, player, ammo, sector, mobj }
}
