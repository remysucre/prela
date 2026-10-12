use crate::schema::{Bsp, Image, Linedef, Sector, Seg, Sidedef, Thing, Vertex};
use prela::engine::{Id, MultiRel, Universe, VecRel};
use prela::loader::{Col, Key};
use std::collections::HashMap;

// Thing flags in the map: appears on skill 3 (medium), multiplayer only.
pub const MTF_MEDIUM: i64 = 2;
pub const MTF_MULTI: i64 = 16;

// Texel value for "no pixel"; NONE means "no id".
pub const TRANSPARENT: u16 = u16::MAX;
pub const NONE: usize = usize::MAX;

// A WAD file: raw bytes plus the lump directory (name, offset, size).
pub struct Wad {
    data: Vec<u8>,
    lumps: Vec<(String, usize, usize)>,
}

// Little-endian readers for WAD records.
fn i16_at(b: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([b[o], b[o + 1]])
}
fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn i32_at(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn name_at(b: &[u8], o: usize) -> String {
    let raw = &b[o..o + 8];
    let end = raw.iter().position(|&c| c == 0).unwrap_or(8);
    String::from_utf8_lossy(&raw[..end]).to_ascii_uppercase()
}
// 0xFFFF in the WAD means "none".
fn id16(v: u16) -> usize {
    if v == u16::MAX { NONE } else { v as usize }
}

impl Wad {
    // Reads the file and its lump directory.
    pub fn open(path: &str) -> Wad {
        let data = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        // Header: magic, lump count, directory offset. Each directory entry: offset, size, 8-byte name.
        assert!(&data[0..4] == b"IWAD" || &data[0..4] == b"PWAD", "{path}: not a WAD");
        let n = i32_at(&data, 4) as usize;
        let dir = i32_at(&data, 8) as usize;
        let lumps = (0..n)
            .map(|i| {
                let o = dir + 16 * i;
                (name_at(&data, o + 8), i32_at(&data, o) as usize, i32_at(&data, o + 4) as usize)
            })
            .collect();
        Wad { data, lumps }
    }

    fn index(&self, name: &str) -> Option<usize> {
        self.lumps.iter().position(|l| l.0 == name)
    }

    fn lump_at(&self, i: usize) -> &[u8] {
        let (_, pos, size) = &self.lumps[i];
        &self.data[*pos..*pos + *size]
    }

    pub fn lump(&self, name: &str) -> &[u8] {
        self.lump_at(self.index(name).unwrap_or_else(|| panic!("missing lump {name}")))
    }

    // Lumps between two marker lumps, e.g. F_START..F_END.
    fn between(&self, start: &str, end: &str) -> Vec<(String, &[u8])> {
        let (a, b) = (self.index(start).unwrap(), self.index(end).unwrap());
        (a + 1..b)
            .filter(|&i| self.lumps[i].2 > 0)
            .map(|i| (self.lumps[i].0.clone(), self.lump_at(i)))
            .collect()
    }

    // A lump belonging to a map, e.g. E1M1's LINEDEFS (the lumps right after the map marker).
    fn map_lump(&self, map: &str, name: &str) -> &[u8] {
        let m = self.index(map).unwrap_or_else(|| panic!("missing map {map}"));
        let i = (m + 1..m + 12).find(|&i| self.lumps[i].0 == name).unwrap();
        self.lump_at(i)
    }
}

// Images built from the WAD, plus the name maps the level and the game tables resolve
// texture, flat, sprite and HUD lump names through (load time only).
pub struct Graphics {
    pub names: HashMap<String, usize>,
    pub flat_names: HashMap<String, usize>,
    pub sprite_names: HashMap<String, usize>,
    pub patch_names: HashMap<String, usize>,
    pub base: Vec<usize>,
    pub width: Vec<usize>,
    pub height: Vec<usize>,
    pub left: Vec<i64>,
    pub top: Vec<i64>,
    pub texels: Vec<u16>,
}

impl Graphics {
    // Appends one image (column-major) and returns its id.
    fn push(&mut self, w: usize, h: usize, left: i64, top: i64, cols: Vec<u16>) -> usize {
        let id = self.base.len();
        self.base.push(self.texels.len());
        self.width.push(w);
        self.height.push(h);
        self.left.push(left);
        self.top.push(top);
        self.texels.extend(cols);
        id
    }
}

// Everything loaded from the WAD: the map's entities, all images, palettes and colormaps.
pub struct Game {
    pub vertex: Vertex,
    pub linedef: Linedef,
    pub sidedef: Sidedef,
    pub seg: Seg,
    pub bsp: Bsp,
    pub sector: Sector,
    pub thing: Thing,
    pub image: Image,
    pub texels: VecRel<usize, u16>,
    pub palette: VecRel<usize, u32>,
    pub colormap: VecRel<usize, u8>,
    pub sky: Id<Image>,
    pub sky_flat: Id<Image>,
    pub sprite_names: HashMap<String, usize>,
    pub patch_names: HashMap<String, usize>,
}

fn col<E, T: Copy>(v: Vec<T>) -> Col<E, T> {
    VecRel::new(v)
}

fn key<E>(n: usize) -> Key<E> {
    Universe::new(n)
}

fn empty<E, T: Copy>() -> Col<E, T> {
    VecRel::new(vec![])
}

// Decodes a standalone patch (sprite or HUD glyph) into an image.
fn patch_columns(p: &[u8]) -> (usize, usize, i64, i64, Vec<u16>) {
    let w = u16_at(p, 0) as usize;
    let h = u16_at(p, 2) as usize;
    let mut out = vec![TRANSPARENT; w * h];
    draw_patch(p, &mut out, w, h, 0, 0);
    (w, h, i16_at(p, 4) as i64, i16_at(p, 6) as i64, out)
}

// Draws a patch's column posts into an image at (ox, oy).
fn draw_patch(p: &[u8], out: &mut [u16], w: usize, h: usize, ox: i64, oy: i64) {
    let pw = u16_at(p, 0) as i64;
    for c in 0..pw {
        let x = ox + c;
        if x < 0 || x >= w as i64 {
            continue;
        }
        // Each column is a list of posts (top row, length, pixels), ended by 0xFF.
        let mut o = i32_at(p, 8 + 4 * c as usize) as usize;
        while p[o] != 0xFF {
            let top = p[o] as i64;
            let len = p[o + 1] as usize;
            for k in 0..len {
                let y = oy + top + k as i64;
                if y >= 0 && y < h as i64 {
                    out[x as usize * h + y as usize] = p[o + 3 + k] as u16;
                }
            }
            o += len + 4;
        }
    }
}

impl Game {
    // Loads graphics, the map, the 14 palettes, and the 34 light-level colormaps.
    pub fn load(path: &str, map: &str) -> Game {
        let wad = Wad::open(path);
        let gfx = Self::graphics(&wad);
        let pal = wad.lump("PLAYPAL");
        let palette = (0..14 * 256)
            .map(|i| ((pal[3 * i] as u32) << 16) | ((pal[3 * i + 1] as u32) << 8) | pal[3 * i + 2] as u32)
            .collect();
        let colormap = wad.lump("COLORMAP")[..34 * 256].to_vec();
        let n = gfx.base.len();
        let (vertex, linedef, sidedef, seg, bsp, sector, thing) = Self::level(&wad, map, &gfx);
        Game {
            vertex,
            linedef,
            sidedef,
            seg,
            bsp,
            sector,
            thing,
            image: Image {
                id: key(n),
                base: col(gfx.base),
                w: col(gfx.width),
                h: col(gfx.height),
                left: col(gfx.left),
                top: col(gfx.top),
            },
            texels: VecRel::new(gfx.texels),
            palette: VecRel::new(palette),
            colormap: VecRel::new(colormap),
            sky: Id::new(gfx.names["SKY1"]),
            sky_flat: Id::new(gfx.flat_names["F_SKY1"]),
            sprite_names: gfx.sprite_names,
            patch_names: gfx.patch_names,
        }
    }
    // Builds every image: wall textures (composed from patches), flats, sprites, HUD glyphs.
    fn graphics(wad: &Wad) -> Graphics {
        let mut g = Graphics {
            names: HashMap::new(),
            flat_names: HashMap::new(),
            sprite_names: HashMap::new(),
            patch_names: HashMap::new(),
            base: vec![],
            width: vec![],
            height: vec![],
            left: vec![],
            top: vec![],
            texels: vec![],
        };
        // Patch index -> patch lump.
        let pn = wad.lump("PNAMES");
        let patches: Vec<Option<&[u8]>> = (0..i32_at(pn, 0) as usize)
            .map(|i| wad.index(&name_at(pn, 4 + 8 * i)).map(|j| wad.lump_at(j)))
            .collect();
        // Wall textures: draw each texture's patches into one image.
        for tl in ["TEXTURE1", "TEXTURE2"] {
            let Some(ti) = wad.index(tl) else { continue };
            let t = wad.lump_at(ti);
            for k in 0..i32_at(t, 0) as usize {
                let o = i32_at(t, 4 + 4 * k) as usize;
                let name = name_at(t, o);
                let w = i16_at(t, o + 12) as usize;
                let h = i16_at(t, o + 14) as usize;
                let n = i16_at(t, o + 20) as usize;
                let mut cols = vec![TRANSPARENT; w * h];
                for j in 0..n {
                    let po = o + 22 + 10 * j;
                    if let Some(Some(p)) = patches.get(i16_at(t, po + 4) as usize) {
                        draw_patch(p, &mut cols, w, h, i16_at(t, po) as i64, i16_at(t, po + 2) as i64);
                    }
                }
                let id = g.push(w, h, 0, 0, cols);
                g.names.entry(name).or_insert(id);
            }
        }
        // Flats: 64x64, transposed to column-major.
        for (name, f) in wad.between("F_START", "F_END") {
            if f.len() < 4096 {
                continue;
            }
            let cols = (0..4096).map(|i| f[(i % 64) * 64 + i / 64] as u16).collect();
            let id = g.push(64, 64, 0, 0, cols);
            g.flat_names.insert(name, id);
        }
        // Sprites.
        for (name, p) in wad.between("S_START", "S_END") {
            let (w, h, l, t, cols) = patch_columns(p);
            let id = g.push(w, h, l, t, cols);
            g.sprite_names.insert(name, id);
        }
        // Status bar glyphs.
        for name in crate::info::hud_lumps() {
            if let Some(i) = wad.index(&name) {
                let (w, h, l, t, cols) = patch_columns(wad.lump_at(i));
                let id = g.push(w, h, l, t, cols);
                g.patch_names.insert(name, id);
            }
        }
        g
    }

    // Parses one map's lumps into entity columns, resolving texture and flat names to image ids.
    fn level(wad: &Wad, map: &str, gfx: &Graphics) -> (Vertex, Linedef, Sidedef, Seg, Bsp, Sector, Thing) {
        let l = |n| wad.map_lump(map, n);
        let tex = |b: &[u8], o| {
            let n = name_at(b, o);
            Id::new(if n == "-" { NONE } else { gfx.names.get(&n).copied().unwrap_or(NONE) })
        };
        let flat = |b: &[u8], o| Id::new(gfx.flat_names.get(&name_at(b, o)).copied().unwrap_or(NONE));
        // Offsets of each fixed-size record in a lump.
        let recs = |b: &[u8], size: usize| (0..b.len() / size).map(move |i| i * size).collect::<Vec<_>>();
        let v = l("VERTEXES");
        let vo = recs(v, 4);
        let vertex = Vertex {
            id: key(vo.len()),
            x: col(vo.iter().map(|&o| i16_at(v, o) as f64).collect()),
            y: col(vo.iter().map(|&o| i16_at(v, o + 2) as f64).collect()),
        };
        let ld = l("LINEDEFS");
        let lo = recs(ld, 14);
        let lcol = |off: usize| col::<Linedef, i64>(lo.iter().map(|&o| u16_at(ld, o + off) as i64).collect());
        let lid = |off: usize| lo.iter().map(|&o| id16(u16_at(ld, o + off))).collect::<Vec<usize>>();
        let linedef = Linedef {
            id: key(lo.len()),
            v1: col(lid(0).into_iter().map(Id::new).collect()),
            v2: col(lid(2).into_iter().map(Id::new).collect()),
            flags: lcol(4),
            special: col(lo.iter().map(|&o| u16_at(ld, o + 6) as usize).collect()),
            tag: col(lo.iter().map(|&o| u16_at(ld, o + 8) as usize).collect()),
            front: col(lid(10).into_iter().map(Id::new).collect()),
            back: col(lid(12).into_iter().map(Id::new).collect()),
            x1: empty(),
            y1: empty(),
            x2: empty(),
            y2: empty(),
            fsec: empty(),
            bsec: empty(),
        };
        let sd = l("SIDEDEFS");
        let so = recs(sd, 30);
        let sidedef = Sidedef {
            id: key(so.len()),
            xoff: col(so.iter().map(|&o| i16_at(sd, o) as f64).collect()),
            yoff: col(so.iter().map(|&o| i16_at(sd, o + 2) as f64).collect()),
            upper: col(so.iter().map(|&o| tex(sd, o + 4)).collect()),
            lower: col(so.iter().map(|&o| tex(sd, o + 12)).collect()),
            middle: col(so.iter().map(|&o| tex(sd, o + 20)).collect()),
            sector: col(so.iter().map(|&o| Id::new(u16_at(sd, o + 28) as usize)).collect()),
        };
        let sg = l("SEGS");
        let go = recs(sg, 12);
        let seg = Seg {
            id: key(go.len()),
            v1: col(go.iter().map(|&o| Id::new(u16_at(sg, o) as usize)).collect()),
            v2: col(go.iter().map(|&o| Id::new(u16_at(sg, o + 2) as usize)).collect()),
            line: col(go.iter().map(|&o| Id::new(u16_at(sg, o + 6) as usize)).collect()),
            side: col(go.iter().map(|&o| u16_at(sg, o + 8) as usize).collect()),
            offset: col(go.iter().map(|&o| i16_at(sg, o + 10) as f64).collect()),
            x1: empty(),
            y1: empty(),
            x2: empty(),
            y2: empty(),
            xoff: empty(),
            yoff: empty(),
            fsec: empty(),
            bsec: empty(),
            upper: empty(),
            middle: empty(),
            lower: empty(),
            flags: empty(),
            light_delta: empty(),
        };
        // BSP: nodes keep their ids; subsector i becomes leaf nodes + i.
        let nd = l("NODES");
        let no = recs(nd, 28);
        let ss = l("SSECTORS");
        let sso = recs(ss, 4);
        let nn = no.len();
        let n = nn + sso.len();
        let f = |k: usize| MultiRel::from_pairs(n, no.iter().enumerate().map(|(i, &o)| (i, i16_at(nd, o + 2 * k) as f64)));
        // Child ids; the high bit marks a subsector (leaf).
        let child = |k: usize| {
            MultiRel::from_pairs(
                n,
                no.iter().enumerate().map(|(i, &o)| {
                    let c = u16_at(nd, o + 24 + 2 * k);
                    (i, Id::new(if c & 0x8000 != 0 { nn + (c & 0x7FFF) as usize } else { c as usize }))
                }),
            )
        };
        // Each subsector is (seg count, first seg).
        let segs = MultiRel::from_pairs(
            n,
            sso.iter().enumerate().flat_map(|(i, &o)| {
                let (count, first) = (u16_at(ss, o) as usize, u16_at(ss, o + 2) as usize);
                (first..first + count).map(move |s| (nn + i, Id::new(s)))
            }),
        );
        let bsp = Bsp {
            id: key(n),
            x: f(0),
            y: f(1),
            dx: f(2),
            dy: f(3),
            child0: child(0),
            child1: child(1),
            segs,
            sector: MultiRel::from_pairs(0, []),
        };
        let se = l("SECTORS");
        let eo = recs(se, 26);
        let sector = Sector {
            id: key(eo.len()),
            floor: col(eo.iter().map(|&o| i16_at(se, o) as f64).collect()),
            ceil: col(eo.iter().map(|&o| i16_at(se, o + 2) as f64).collect()),
            floorpic: col(eo.iter().map(|&o| flat(se, o + 4)).collect()),
            ceilpic: col(eo.iter().map(|&o| flat(se, o + 12)).collect()),
            light: col(eo.iter().map(|&o| i16_at(se, o + 20) as i64).collect()),
            special: col(eo.iter().map(|&o| u16_at(se, o + 22) as usize).collect()),
            tag: col(eo.iter().map(|&o| u16_at(se, o + 24) as usize).collect()),
            neighbors: MultiRel::from_pairs(0, []),
        };
        let th = l("THINGS");
        let to = recs(th, 10);
        let thing = Thing {
            id: key(to.len()),
            x: col(to.iter().map(|&o| i16_at(th, o) as f64).collect()),
            y: col(to.iter().map(|&o| i16_at(th, o + 2) as f64).collect()),
            angle: col(to.iter().map(|&o| u16_at(th, o + 4) as f64).collect()),
            kind: col(to.iter().map(|&o| u16_at(th, o + 6) as usize).collect()),
            flags: col(to.iter().map(|&o| u16_at(th, o + 8) as i64).collect()),
        };
        (vertex, linedef, sidedef, seg, bsp, sector, thing)
    }
}
