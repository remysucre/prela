use std::collections::HashMap;

pub const TRANSPARENT: u16 = u16::MAX;
pub const NONE: usize = usize::MAX;

pub struct Wad {
    data: Vec<u8>,
    lumps: Vec<(String, usize, usize)>,
}

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
fn id16(v: u16) -> usize {
    if v == u16::MAX { NONE } else { v as usize }
}

impl Wad {
    pub fn open(path: &str) -> Wad {
        let data = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
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

    fn between(&self, start: &str, end: &str) -> Vec<(String, &[u8])> {
        let (a, b) = (self.index(start).unwrap(), self.index(end).unwrap());
        (a + 1..b)
            .filter(|&i| self.lumps[i].2 > 0)
            .map(|i| (self.lumps[i].0.clone(), self.lump_at(i)))
            .collect()
    }

    fn map_lump(&self, map: &str, name: &str) -> &[u8] {
        let m = self.index(map).unwrap_or_else(|| panic!("missing map {map}"));
        let i = (m + 1..m + 12).find(|&i| self.lumps[i].0 == name).unwrap();
        self.lump_at(i)
    }
}

#[derive(Clone, Copy)]
pub struct Linedef {
    pub v1: usize,
    pub v2: usize,
    pub flags: i64,
    pub special: i64,
    pub tag: i64,
    pub front: usize,
    pub back: usize,
}

#[derive(Clone, Copy)]
pub struct Sidedef {
    pub xoff: f64,
    pub yoff: f64,
    pub upper: usize,
    pub lower: usize,
    pub middle: usize,
    pub sector: usize,
}

#[derive(Clone, Copy)]
pub struct Seg {
    pub v1: usize,
    pub v2: usize,
    pub line: usize,
    pub side: usize,
    pub offset: f64,
}

#[derive(Clone, Copy)]
pub struct Node {
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
    pub bbox: [[f64; 4]; 2],
    pub child: [usize; 2],
    pub child_is_leaf: [bool; 2],
}

#[derive(Clone, Copy)]
pub struct Sector {
    pub floor: f64,
    pub ceil: f64,
    pub floorpic: usize,
    pub ceilpic: usize,
    pub light: i64,
    pub special: i64,
    pub tag: i64,
}

#[derive(Clone, Copy)]
pub struct Thing {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    pub kind: i64,
    pub flags: i64,
}

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

pub struct Level {
    pub vertices: Vec<(f64, f64)>,
    pub linedefs: Vec<Linedef>,
    pub sidedefs: Vec<Sidedef>,
    pub segs: Vec<Seg>,
    pub subsectors: Vec<(usize, usize)>,
    pub nodes: Vec<Node>,
    pub sectors: Vec<Sector>,
    pub things: Vec<Thing>,
    pub reject: Vec<u8>,
}

pub struct Game {
    pub level: Level,
    pub gfx: Graphics,
    pub palette: Vec<u32>,
    pub colormap: Vec<u8>,
    pub sky: usize,
    pub sky_flat: usize,
}

fn patch_columns(p: &[u8]) -> (usize, usize, i64, i64, Vec<u16>) {
    let w = u16_at(p, 0) as usize;
    let h = u16_at(p, 2) as usize;
    let mut out = vec![TRANSPARENT; w * h];
    draw_patch(p, &mut out, w, h, 0, 0);
    (w, h, i16_at(p, 4) as i64, i16_at(p, 6) as i64, out)
}

fn draw_patch(p: &[u8], out: &mut [u16], w: usize, h: usize, ox: i64, oy: i64) {
    let pw = u16_at(p, 0) as i64;
    for c in 0..pw {
        let x = ox + c;
        if x < 0 || x >= w as i64 {
            continue;
        }
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
    pub fn load(path: &str, map: &str) -> Game {
        let wad = Wad::open(path);
        let gfx = Self::graphics(&wad);
        let level = Self::level(&wad, map, &gfx);
        let pal = wad.lump("PLAYPAL");
        let palette = (0..14 * 256)
            .map(|i| ((pal[3 * i] as u32) << 16) | ((pal[3 * i + 1] as u32) << 8) | pal[3 * i + 2] as u32)
            .collect();
        let colormap = wad.lump("COLORMAP")[..34 * 256].to_vec();
        let sky = gfx.names["SKY1"];
        let sky_flat = gfx.flat_names["F_SKY1"];
        Game { level, gfx, palette, colormap, sky, sky_flat }
    }

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
        let pn = wad.lump("PNAMES");
        let patches: Vec<Option<&[u8]>> = (0..i32_at(pn, 0) as usize)
            .map(|i| wad.index(&name_at(pn, 4 + 8 * i)).map(|j| wad.lump_at(j)))
            .collect();
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
        for (name, f) in wad.between("F_START", "F_END") {
            if f.len() < 4096 {
                continue;
            }
            let cols = (0..4096).map(|i| f[(i % 64) * 64 + i / 64] as u16).collect();
            let id = g.push(64, 64, 0, 0, cols);
            g.flat_names.insert(name, id);
        }
        for (name, p) in wad.between("S_START", "S_END") {
            let (w, h, l, t, cols) = patch_columns(p);
            let id = g.push(w, h, l, t, cols);
            g.sprite_names.insert(name, id);
        }
        let hud = (0..10).map(|i| format!("STTNUM{i}")).chain(["STTPRCNT", "STKEYS0", "STKEYS1", "STKEYS2"].map(String::from));
        for name in hud {
            if let Some(i) = wad.index(&name) {
                let (w, h, l, t, cols) = patch_columns(wad.lump_at(i));
                let id = g.push(w, h, l, t, cols);
                g.patch_names.insert(name, id);
            }
        }
        g
    }

    fn level(wad: &Wad, map: &str, gfx: &Graphics) -> Level {
        let l = |n| wad.map_lump(map, n);
        let tex = |b: &[u8], o| {
            let n = name_at(b, o);
            if n == "-" { NONE } else { gfx.names.get(&n).copied().unwrap_or(NONE) }
        };
        let flat = |b: &[u8], o| gfx.flat_names.get(&name_at(b, o)).copied().unwrap_or(NONE);
        let recs = |b: &[u8], size: usize| (0..b.len() / size).map(move |i| i * size);
        let v = l("VERTEXES");
        let vertices = recs(v, 4).map(|o| (i16_at(v, o) as f64, i16_at(v, o + 2) as f64)).collect();
        let ld = l("LINEDEFS");
        let linedefs = recs(ld, 14)
            .map(|o| Linedef {
                v1: u16_at(ld, o) as usize,
                v2: u16_at(ld, o + 2) as usize,
                flags: u16_at(ld, o + 4) as i64,
                special: u16_at(ld, o + 6) as i64,
                tag: u16_at(ld, o + 8) as i64,
                front: id16(u16_at(ld, o + 10)),
                back: id16(u16_at(ld, o + 12)),
            })
            .collect();
        let sd = l("SIDEDEFS");
        let sidedefs = recs(sd, 30)
            .map(|o| Sidedef {
                xoff: i16_at(sd, o) as f64,
                yoff: i16_at(sd, o + 2) as f64,
                upper: tex(sd, o + 4),
                lower: tex(sd, o + 12),
                middle: tex(sd, o + 20),
                sector: u16_at(sd, o + 28) as usize,
            })
            .collect();
        let sg = l("SEGS");
        let segs = recs(sg, 12)
            .map(|o| Seg {
                v1: u16_at(sg, o) as usize,
                v2: u16_at(sg, o + 2) as usize,
                line: u16_at(sg, o + 6) as usize,
                side: u16_at(sg, o + 8) as usize,
                offset: i16_at(sg, o + 10) as f64,
            })
            .collect();
        let ss = l("SSECTORS");
        let subsectors = recs(ss, 4).map(|o| (u16_at(ss, o + 2) as usize, u16_at(ss, o) as usize)).collect();
        let nd = l("NODES");
        let nodes = recs(nd, 28)
            .map(|o| {
                let f = |k: usize| i16_at(nd, o + 2 * k) as f64;
                let c = |k: usize| u16_at(nd, o + 24 + 2 * k);
                Node {
                    x: f(0),
                    y: f(1),
                    dx: f(2),
                    dy: f(3),
                    bbox: [[f(4), f(5), f(6), f(7)], [f(8), f(9), f(10), f(11)]],
                    child: [(c(0) & 0x7FFF) as usize, (c(1) & 0x7FFF) as usize],
                    child_is_leaf: [c(0) & 0x8000 != 0, c(1) & 0x8000 != 0],
                }
            })
            .collect();
        let se = l("SECTORS");
        let sectors = recs(se, 26)
            .map(|o| Sector {
                floor: i16_at(se, o) as f64,
                ceil: i16_at(se, o + 2) as f64,
                floorpic: flat(se, o + 4),
                ceilpic: flat(se, o + 12),
                light: i16_at(se, o + 20) as i64,
                special: u16_at(se, o + 22) as i64,
                tag: u16_at(se, o + 24) as i64,
            })
            .collect();
        let th = l("THINGS");
        let things = recs(th, 10)
            .map(|o| Thing {
                x: i16_at(th, o) as f64,
                y: i16_at(th, o + 2) as f64,
                angle: u16_at(th, o + 4) as f64,
                kind: u16_at(th, o + 6) as i64,
                flags: u16_at(th, o + 8) as i64,
            })
            .collect();
        Level {
            vertices,
            linedefs,
            sidedefs,
            segs,
            subsectors,
            nodes,
            sectors,
            things,
            reject: l("REJECT").to_vec(),
        }
    }
}
