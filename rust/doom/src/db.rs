use crate::info::{self, Action, Pickup};
use crate::wad::{Game, Linedef, Node, Sector, Seg, Sidedef, Thing, NONE};
use prela::engine::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct StateRow {
    pub sprite: usize,
    pub frame: usize,
    pub bright: bool,
    pub tics: i32,
    pub action: Action,
    pub next: usize,
}

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
    pub flags: u32,
    pub pickup: Pickup,
    pub drop: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Weapon {
    pub ammo: usize,
    pub per_shot: i32,
    pub ready: usize,
    pub attack: usize,
    pub flash: usize,
}

pub struct Tables {
    pub states: VecRel<usize, StateRow>,
    pub infos: VecRel<usize, Info>,
    pub ednum_info: HashIdx<i64, usize>,
    pub sprite_frames: HashIdx<u64, (usize, bool)>,
    pub weapons: VecRel<usize, Weapon>,
    pub hud: VecRel<usize, usize>,
    pub puff: usize,
    pub blood: usize,
    pub ball: usize,
}

pub fn frame_key(sprite: usize, frame: usize, rot: usize) -> u64 {
    (sprite * 1000 + frame * 10 + rot) as u64
}

fn tables(gfx: &crate::wad::Graphics) -> Tables {
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
                flags: m.flags,
                pickup: m.pickup,
                drop: by_ednum.get(&m.drop).copied().unwrap_or(NONE),
            })
            .collect(),
    );
    let ednum_info = (&infos)
        .and(Same::<usize>::new())
        .filt(|(i, _): (Info, usize)| i.ednum > 0)
        .key_by(|(i, _): (Info, usize)| i.ednum)
        .map(|(_, k)| k)
        .collect();
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
    let weapons = VecRel::new(
        info::weapons()
            .iter()
            .map(|w| Weapon { ammo: w.ammo, per_shot: w.per_shot, ready: id(w.ready), attack: id(w.attack), flash: id(w.flash) })
            .collect(),
    );
    let hud_names = ["STTNUM0", "STTNUM1", "STTNUM2", "STTNUM3", "STTNUM4", "STTNUM5", "STTNUM6", "STTNUM7", "STTNUM8", "STTNUM9", "STTPRCNT", "STKEYS0", "STKEYS1", "STKEYS2"];
    let hud = VecRel::new(hud_names.iter().map(|n| gfx.patch_names.get(*n).copied().unwrap_or(NONE)).collect());
    let kind = |n: &str| mdefs.iter().position(|m| m.name == n).unwrap();
    Tables { states, infos, ednum_info, sprite_frames, weapons, hud, puff: kind("PUFF"), blood: kind("BLOOD"), ball: kind("TBALL") }
}

#[derive(Clone, Copy)]
pub struct Tex {
    pub base: usize,
    pub w: usize,
    pub h: usize,
    pub left: i64,
    pub top: i64,
}

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
    pub line: usize,
}

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

pub struct Db {
    pub t: Tables,
    pub lines: VecRel<usize, Line>,
    pub neighbors: MultiRel<usize, usize>,
    pub tag_sectors: MultiRel<usize, usize>,
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
    pub fn new(g: Game) -> (Db, VecRel<usize, Sector>) {
        let l = g.level;
        let n_nodes = l.nodes.len();
        let gfx = g.gfx;
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
        let ss = l.subsectors.clone();
        let ss_segs = MultiRel::from_pairs(
            ss.len(),
            ss.iter().enumerate().flat_map(|(i, &(first, n))| (first..first + n).map(move |s| (i, s))),
        );
        let node_children = MultiRel::from_pairs(
            n_nodes,
            l.nodes.iter().enumerate().flat_map(|(i, n)| {
                (0..2).map(move |s| (i, (s, if n.child_is_leaf[s] { n_nodes + n.child[s] } else { n.child[s] })))
            }),
        );
        let t = tables(&gfx);
        let db = Db {
            t,
            lines: VecRel::new(vec![]),
            neighbors: MultiRel::from_pairs(0, []),
            tag_sectors: MultiRel::from_pairs(0, []),
            vertices: VecRel::new(l.vertices),
            linedefs: VecRel::new(l.linedefs),
            sidedefs: VecRel::new(l.sidedefs),
            segs: VecRel::new(l.segs),
            ss_segs,
            nodes: VecRel::new(l.nodes),
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
        let neighbors = (&lines)
            .filt(|l: Line| l.bsec != NONE)
            .flat_map(|l: Line| [(l.fsec, l.bsec), (l.bsec, l.fsec)])
            .key_by(|(a, _)| a)
            .map(|(_, b)| b)
            .collect();
        let sectors = VecRel::new(l.sectors);
        let tag_sectors = (&sectors)
            .and(Same::<usize>::new())
            .key_by(|(s, _): (Sector, usize)| s.tag as usize)
            .map(|(_, id)| id)
            .collect();
        (Db { leaf_sector, lines, neighbors, tag_sectors, ..db }, sectors)
    }

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
                line: s.line,
            })
            .collect()
    }

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
