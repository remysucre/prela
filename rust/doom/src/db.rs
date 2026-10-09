use crate::info::{self, Action, ActOut, AiOut, AmmoDef, ArmorClass, Consts, ChaseDir, FaceOut, NumberSlot, PickupDef, Rule, Slide, Special, StepOut, WOut, G_DEATH, G_MELEE, G_MISSILE, G_PAIN, G_SEE, G_SPAWN};
use crate::rules::decisions;
use crate::wad::{Game, Linedef, Node, Sector, Seg, Sidedef, Thing, NONE};
use prela::engine::*;
use std::collections::HashMap;

// One animation/AI state: sprite frame to show, how long, what action to run, and the next state.
#[derive(Clone, Copy, Debug)]
pub struct StateRow {
    pub sprite: usize,
    pub frame: usize,
    pub bright: bool,
    pub tics: i32,
    pub action: Action,
    pub next: usize,
}

// Static stats for one kind of thing (monster, item, decoration): states to jump to, size, health, flags.
#[derive(Clone, Copy, Debug)]
pub struct Info {
    pub ednum: i64,
    pub spawn: usize,
    pub see: usize,
    pub pain: usize,
    pub melee: usize,
    pub missile: usize,
    pub death: usize,
    pub speed: f64,
    pub radius: f64,
    pub height: f64,
    pub health: i32,
    pub painchance: i32,
    pub damage: i32,
    pub flags: u32,
    pub drop: usize,
}

// One weapon: which ammo it uses and its ready/attack/flash states.
#[derive(Clone, Copy, Debug)]
pub struct Weapon {
    pub ammo: usize,
    pub per_shot: i32,
    pub ready: usize,
    pub attack: usize,
    pub flash: usize,
    pub flash_tics: i32,
    pub damage: i32,
    pub damage_roll: i32,
    pub range: f64,
    pub spread_div: f64,
}

// All the game-rule tables from info.rs, loaded as Prela relations.
pub struct Tables {
    pub states: VecRel<usize, StateRow>,
    pub infos: VecRel<usize, Info>,
    pub ednum_info: HashIdx<i64, usize>,
    pub sprite_frames: HashIdx<u64, (usize, bool)>,
    pub weapons: VecRel<usize, Weapon>,
    pub hud_fonts: HashIdx<(usize, usize), usize>,
    pub hud_patches: VecRel<usize, (usize, f64, f64, f32)>,
    pub hud_numbers: VecRel<usize, NumberSlot>,
    pub hud_arms: VecRel<usize, (usize, f64, f64, usize)>,
    pub hud_keys: VecRel<usize, (f64, f64, usize)>,
    pub face_rules: MultiRel<usize, Rule<FaceOut>>,
    pub faces: HashIdx<(u8, usize, usize), usize>,
    pub info_goto: HashIdx<(usize, u8), usize>,
    pub ai_rules: MultiRel<usize, Rule<AiOut>>,
    pub weapon_choice: HashIdx<(usize, usize), WOut>,
    pub specials: HashIdx<i64, Special>,
    pub activations: HashIdx<usize, ActOut>,
    pub mover_rules: MultiRel<usize, Rule<StepOut>>,
    pub pickups: HashIdx<usize, PickupDef>,
    pub ammo_defs: VecRel<usize, AmmoDef>,
    pub sector_damage: HashIdx<i64, i32>,
    pub armor_classes: VecRel<usize, ArmorClass>,
    pub consts: VecRel<usize, Consts>,
    pub chase_dirs: VecRel<usize, ChaseDir>,
    pub slides: VecRel<usize, Slide>,
    pub puff: usize,
    pub blood: usize,
    pub ball: usize,
}

// The constants row, read once at the top of a function (like SQLDoom's doom_const()).
pub fn consts(db: &Db) -> Consts {
    db.t.consts.get(0).unwrap()
}

// Packs (sprite, frame, rotation) into one lookup key for sprite images.
pub fn frame_key(sprite: usize, frame: usize, rot: usize) -> u64 {
    (sprite * 1000 + frame * 10 + rot) as u64
}

// Turns the Rust-literal tables in info.rs into relations, resolving state names to ids.
fn tables(gfx: &crate::wad::Graphics) -> Tables {
    // Animation states, with names and sprites replaced by ids.
    let defs = info::states();
    let ids: HashMap<String, usize> = defs.iter().enumerate().map(|(i, d)| (d.name.clone(), i)).collect();
    let id = |n: &str| *ids.get(n).unwrap_or_else(|| panic!("unknown state {n}"));
    let mut sprites: Vec<&str> = defs.iter().map(|d| d.sprite).collect();
    sprites.sort();
    sprites.dedup();
    let sprite_id = |n: &str| sprites.iter().position(|&s| s == n).unwrap();
    let states = VecRel::new(
        defs.iter()
            .map(|d| StateRow {
                sprite: sprite_id(d.sprite),
                frame: d.frame as usize,
                bright: d.bright,
                tics: d.tics,
                action: d.action,
                next: id(&d.next),
            })
            .collect(),
    );
    // Thing kinds, with state names and drop items resolved to ids.
    let mdefs = info::mobjs();
    let by_ednum: HashMap<i64, usize> = mdefs.iter().enumerate().rev().map(|(i, m)| (m.doomednum, i)).collect();
    let infos: VecRel<usize, Info> = VecRel::new(
        mdefs
            .iter()
            .map(|m| Info {
                ednum: m.doomednum,
                spawn: id(&m.spawn),
                see: id(&m.see),
                pain: id(&m.pain),
                melee: id(&m.melee),
                missile: id(&m.missile),
                death: id(&m.death),
                speed: m.speed,
                radius: m.radius,
                height: m.height,
                health: m.health,
                painchance: m.painchance,
                damage: m.damage,
                flags: m.flags,
                drop: by_ednum.get(&m.drop).copied().unwrap_or(NONE),
            })
            .collect(),
    );
    // Map editor number -> thing kind, for spawning map things.
    let ednum_info = (&infos)
        .and(Same::<usize>::new())
        .filt(|(i, _): (Info, usize)| i.ednum > 0)
        .key_by(|(i, _): (Info, usize)| i.ednum)
        .map(|(_, k)| k)
        .collect();
    // Sprite lump names like TROOA2A8 -> (sprite, frame, rotation) -> image id; the second half is the mirrored rotation.
    let mut frames = Vec::new();
    for (name, &gid) in &gfx.sprite_names {
        let b = name.as_bytes();
        if b.len() < 6 {
            continue;
        }
        let Some(sp) = sprites.iter().position(|&s| s.as_bytes() == &b[..4]) else { continue };
        frames.push((frame_key(sp, (b[4] - b'A') as usize, (b[5] - b'0') as usize), (gid, false)));
        if b.len() >= 8 {
            frames.push((frame_key(sp, (b[6] - b'A') as usize, (b[7] - b'0') as usize), (gid, true)));
        }
    }
    let sprite_frames = VecRel::<usize, (u64, (usize, bool))>::new(frames).key_by(|(k, _)| k).map(|(_, v)| v).collect();
    // Weapons, with state names resolved.
    let weapons: VecRel<usize, Weapon> = VecRel::new(
        info::weapons()
            .iter()
            .map(|w| Weapon {
                ammo: w.ammo,
                per_shot: w.per_shot,
                ready: id(w.ready),
                attack: id(w.attack),
                flash: id(w.flash),
                flash_tics: 0,
                damage: w.damage,
                damage_roll: w.damage_roll,
                range: w.range,
                spread_div: w.spread_div,
            })
            .collect(),
    );
    // Fill in each weapon's flash duration from its flash state.
    let weapons: VecRel<usize, Weapon> = (&weapons).and((&weapons).map(|w: Weapon| w.flash).select(&states)).map(|(w, r): (Weapon, StateRow)| Weapon { flash_tics: r.tics, ..w }).collect();
    // Status bar glyphs: digits 0-9, percent sign, three keys.
    // Status bar tables, with lump names resolved to image ids.
    let lump = |n: &str| gfx.patch_names.get(n).copied().unwrap_or(NONE);
    let hud_fonts = VecRel::<usize, ((usize, usize), usize)>::new(info::hud_fonts().iter().map(|(k, n)| (*k, lump(n))).collect()).key_by(|(k, _)| k).map(|(_, g)| g).collect();
    let hud_patches = VecRel::new(info::hud_patches().iter().map(|&(n, x, y, d)| (lump(n), x, y, d)).collect());
    let hud_keys = VecRel::new(info::hud_keys().iter().map(|&(x, y, n)| (x, y, lump(n))).collect());
    let faces = VecRel::<usize, ((u8, usize, usize), usize)>::new(info::faces().iter().map(|(k, n)| (*k, lump(n))).collect()).key_by(|(k, _)| k).map(|(_, g)| g).collect();
    let face_rules = VecRel::<usize, Rule<FaceOut>>::new(info::face_rules()).key_by(|_| 0usize).collect();
    let kind = |n: &str| mdefs.iter().position(|m| m.name == n).unwrap();
    // (thing kind, goto label) -> state id, so rules can say "go to SEE" and get the right state per monster.
    let info_goto = (&infos)
        .and(Same::<usize>::new())
        .flat_map(|(i, k): (Info, usize)| {
            [(G_SEE, i.see), (G_SPAWN, i.spawn), (G_MELEE, i.melee), (G_MISSILE, i.missile), (G_DEATH, i.death), (G_PAIN, i.pain)].map(|(g, s)| (k, g, s))
        })
        .filt(|(_, _, s)| s != 0)
        .key_by(|(k, g, _)| (k, g))
        .map(|(_, _, s)| s)
        .collect();
    // Rule tables keyed by the action (or special/mover key) they apply to.
    let by_action = |rules: Vec<(Action, Rule<AiOut>)>| VecRel::<usize, (Action, Rule<AiOut>)>::new(rules).key_by(|(a, _)| a as usize).map(|(_, r)| r).collect();
    let ai_rules = by_action(info::ai_rules());
    let weapon_rules: MultiRel<usize, Rule<WOut>> = VecRel::<usize, (Action, Rule<WOut>)>::new(info::weapon_rules()).key_by(|(a, _)| a as usize).map(|(_, r)| r).collect();
    let weapon_choice = decisions(&weapon_rules, 3);
    let specials = VecRel::<usize, (i64, Special)>::new(info::specials()).key_by(|(k, _)| k).map(|(_, v)| v).collect();
    let activations = VecRel::<usize, (usize, ActOut)>::new(info::activations()).key_by(|(k, _)| k).map(|(_, v)| v).collect();
    let mover_rules = VecRel::<usize, (usize, Rule<StepOut>)>::new(info::mover_rules()).key_by(|(k, _)| k).map(|(_, v)| v).collect();
    // Thing kind -> what picking it up gives.
    let pickups = VecRel::<usize, Option<PickupDef>>::new(mdefs.iter().map(|m| m.pickup).collect()).flat_map(|p| p).collect();
    let sector_damage = VecRel::<usize, (i64, i32)>::new(info::sector_damage()).key_by(|(k, _)| k).map(|(_, v)| v).collect();
    Tables {
        states,
        infos,
        ednum_info,
        sprite_frames,
        weapons,
        hud_fonts,
        hud_patches,
        hud_numbers: VecRel::new(info::hud_numbers()),
        hud_arms: VecRel::new(info::hud_arms()),
        hud_keys,
        face_rules,
        faces,
        info_goto,
        ai_rules,
        weapon_choice,
        specials,
        activations,
        mover_rules,
        pickups,
        ammo_defs: VecRel::new(info::ammo_defs()),
        sector_damage,
        armor_classes: VecRel::new(info::armor_classes()),
        consts: VecRel::new(vec![info::consts()]),
        chase_dirs: VecRel::new(info::chase_dirs()),
        slides: VecRel::new(info::slides()),
        puff: kind("PUFF"),
        blood: kind("BLOOD"),
        ball: kind("TBALL"),
    }
}

// Where an image's texels start in the texel table, plus its size and draw offset.
#[derive(Clone, Copy)]
pub struct Tex {
    pub base: usize,
    pub w: usize,
    pub h: usize,
    pub left: i64,
    pub top: i64,
}

// A seg joined with its line, vertices, and sidedefs: everything the renderer needs per wall piece.
#[derive(Clone, Copy)]
pub struct Wall {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub offset: f64,
    pub yoff: f64,
    pub fsec: usize,
    pub bsec: usize,
    pub upper: usize,
    pub lower: usize,
    pub middle: usize,
    pub flags: i64,
    pub light_delta: i64,
}

// A linedef joined with its vertices and sectors: what collision and triggers need.
#[derive(Clone, Copy)]
pub struct Line {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub flags: i64,
    pub special: i64,
    pub tag: i64,
    pub fsec: usize,
    pub bsec: usize,
}

// Everything static about the level and graphics, as relations.
pub struct Db {
    pub t: Tables,
    pub lines: VecRel<usize, Line>,
    pub neighbors: MultiRel<usize, usize>,
    pub tag_sectors: MultiRel<usize, usize>,
    pub sectors: VecRel<usize, Sector>,
    pub vertices: VecRel<usize, (f64, f64)>,
    pub linedefs: VecRel<usize, Linedef>,
    pub sidedefs: VecRel<usize, Sidedef>,
    pub segs: VecRel<usize, Seg>,
    pub ss_segs: MultiRel<usize, usize>,
    pub nodes: VecRel<usize, Node>,
    pub node_children: MultiRel<usize, (usize, usize)>,
    pub leaf_sector: MultiRel<usize, usize>,
    pub bsp_root: usize,
    pub things: VecRel<usize, Thing>,
    pub tex: VecRel<usize, Tex>,
    pub texels: VecRel<usize, u16>,
    pub colormap: VecRel<usize, u8>,
    pub palette: VecRel<usize, u32>,
    pub walls: VecRel<usize, Wall>,
    pub sky: usize,
    pub sky_flat: usize,
}

impl Db {
    // Builds the database from the parsed WAD. Also returns the starting sector heights (game state, not static).
    pub fn new(g: Game) -> Db {
        let l = g.level;
        let n_nodes = l.nodes.len();
        let gfx = g.gfx;
        // Image metadata, one row per texture/flat/sprite/HUD glyph.
        let tex = VecRel::new(
            (0..gfx.base.len())
                .map(|i| Tex {
                    base: gfx.base[i],
                    w: gfx.width[i],
                    h: gfx.height[i],
                    left: gfx.left[i],
                    top: gfx.top[i],
                })
                .collect(),
        );
        // Subsector -> its run of segs.
        let ss_segs: MultiRel<usize, usize> = VecRel::<usize, (usize, usize)>::new(l.subsectors.clone())
            .expand(|(first, n): (usize, usize)| (first, first + n))
            .map(|(_, s): ((usize, usize), usize)| s)
            .collect();
        // Node -> (side, child); leaf children are numbered after the nodes.
        let nodes: VecRel<usize, Node> = VecRel::new(l.nodes);
        let node_children: MultiRel<usize, (usize, usize)> = (&nodes)
            .and(Same::<usize>::new())
            .cross(Universe::<usize>::new(2))
            .key_by(|((_, i), _): ((Node, usize), usize)| i)
            .map(move |((n, _), s): ((Node, usize), usize)| (s, if n.child_is_leaf[s] { n_nodes + n.child[s] } else { n.child[s] }))
            .collect();
        let t = tables(&gfx);
        // First pass with placeholders; the derived tables below are filled in from it.
        let db = Db {
            t,
            lines: VecRel::new(vec![]),
            neighbors: MultiRel::from_pairs(0, []),
            tag_sectors: MultiRel::from_pairs(0, []),
            sectors: VecRel::new(vec![]),
            vertices: VecRel::new(l.vertices),
            linedefs: VecRel::new(l.linedefs),
            sidedefs: VecRel::new(l.sidedefs),
            segs: VecRel::new(l.segs),
            ss_segs,
            nodes,
            node_children,
            leaf_sector: MultiRel::from_pairs(0, []),
            bsp_root: n_nodes - 1,
            things: VecRel::new(l.things),
            tex,
            texels: VecRel::new(gfx.texels),
            colormap: VecRel::new(g.colormap),
            palette: VecRel::new(g.palette),
            walls: VecRel::new(vec![]),
            sky: g.sky,
            sky_flat: g.sky_flat,
        };
        let db = Db { walls: db.derive_walls(), ..db };
        let leaf_sector = db.derive_leaf_sector(n_nodes);
        let lines = db.derive_lines();
        // Sector -> sectors that share a two-sided line with it.
        let neighbors = (&lines)
            .filt(|l: Line| l.bsec != NONE)
            .flat_map(|l: Line| [(l.fsec, l.bsec), (l.bsec, l.fsec)])
            .key_by(|(a, _)| a)
            .map(|(_, b)| b)
            .collect();
        let sectors = VecRel::new(l.sectors);
        // Tag -> sectors with that tag, for switches that move tagged sectors.
        let tag_sectors = (&sectors)
            .and(Same::<usize>::new())
            .key_by(|(s, _): (Sector, usize)| s.tag as usize)
            .map(|(_, id)| id)
            .collect();
        Db { leaf_sector, lines, neighbors, tag_sectors, sectors, ..db }
    }

    // Joins each seg with its linedef, vertices, and front/back sidedefs.
    fn derive_walls(&self) -> VecRel<usize, Wall> {
        let line = (&self.segs).map(|s: Seg| s.line).select(&self.linedefs);
        let p1 = (&self.segs).map(|s: Seg| s.v1).select(&self.vertices);
        let p2 = (&self.segs).map(|s: Seg| s.v2).select(&self.vertices);
        let front = (&self.segs)
            .and(&line)
            .map(|(s, l)| if s.side == 0 { l.front } else { l.back })
            .select(&self.sidedefs);
        let back = (&self.segs)
            .and(&line)
            .map(|(s, l)| if s.side == 0 { l.back } else { l.front })
            .select(&self.sidedefs)
            .opt();
        (&self.segs)
            .and(&line)
            .and(p1.and(p2))
            .and(front.and(back))
            .map(|(((s, l), (a, b)), (f, bk)): (((Seg, Linedef), ((f64, f64), (f64, f64))), (Sidedef, Option<Sidedef>))| Wall {
                x1: a.0,
                y1: a.1,
                x2: b.0,
                y2: b.1,
                offset: s.offset + f.xoff,
                yoff: f.yoff,
                fsec: f.sector,
                bsec: bk.map_or(NONE, |b| b.sector),
                upper: f.upper,
                lower: f.lower,
                middle: f.middle,
                flags: l.flags,
                light_delta: if a.1 == b.1 { -1 } else if a.0 == b.0 { 1 } else { 0 },
            })
            .collect()
    }

    // Joins each linedef with its vertices and front/back sidedefs.
    fn derive_lines(&self) -> VecRel<usize, Line> {
        let p1 = (&self.linedefs).map(|l: Linedef| l.v1).select(&self.vertices);
        let p2 = (&self.linedefs).map(|l: Linedef| l.v2).select(&self.vertices);
        let front = (&self.linedefs).map(|l: Linedef| l.front).select(&self.sidedefs);
        let back = (&self.linedefs).map(|l: Linedef| l.back).select(&self.sidedefs).opt();
        (&self.linedefs)
            .and(p1.and(p2))
            .and(front.and(back))
            .map(|((l, (a, b)), (f, bk)): ((Linedef, ((f64, f64), (f64, f64))), (Sidedef, Option<Sidedef>))| Line {
                x1: a.0,
                y1: a.1,
                x2: b.0,
                y2: b.1,
                flags: l.flags,
                special: l.special,
                tag: l.tag,
                fsec: f.sector,
                bsec: bk.map_or(NONE, |b| b.sector),
            })
            .collect()
    }

    // BSP leaf -> sector, taken from the leaf's first seg.
    fn derive_leaf_sector(&self, n_nodes: usize) -> MultiRel<usize, usize> {
        let first_seg = (&self.ss_segs).fold(NONE, |a: usize, s: usize| a.min(s));
        let sector = (&first_seg).select(&self.walls).map(|w: Wall| w.fsec);
        Universe::<usize>::new(self.ss_segs.n_dom())
            .and(sector)
            .key_by(move |(ss, _)| n_nodes + ss)
            .map(|(_, sec)| sec)
            .collect()
    }
}
