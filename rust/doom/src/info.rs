#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    None,
    Look,
    Chase,
    FaceTarget,
    PosAttack,
    SPosAttack,
    TroopAttack,
    SargAttack,
    Pain,
    Scream,
    Fall,
    Explode,
    WeaponReady,
    FirePistol,
    FireShotgun,
    FireCGun,
    ReFire,
}

pub struct StateDef {
    pub name: String,
    pub sprite: &'static str,
    pub frame: u8,
    pub bright: bool,
    pub tics: i32,
    pub action: Action,
    pub next: String,
}

pub const SOLID: u32 = 1;
pub const SHOOTABLE: u32 = 2;
pub const MONSTER: u32 = 4;
pub const PICKUP: u32 = 8;
pub const MISSILE: u32 = 16;
pub const NOBLOOD: u32 = 32;
pub const HANGING: u32 = 64;
pub const NOGRAV: u32 = 128;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pickup {
    None,
    Health { amount: i32, max: i32 },
    Armor { amount: i32, class: i32, bonus: bool },
    Ammo { kind: usize, amount: i32 },
    Weapon { weapon: usize, kind: usize, amount: i32 },
    Key(u8),
    Backpack,
}

pub struct MobjDef {
    pub doomednum: i64,
    pub name: &'static str,
    pub spawn: String,
    pub see: String,
    pub pain: String,
    pub melee: String,
    pub missile: String,
    pub death: String,
    pub speed: f64,
    pub radius: f64,
    pub height: f64,
    pub health: i32,
    pub painchance: i32,
    pub flags: u32,
    pub pickup: Pickup,
    pub drop: i64,
}

pub struct WeaponDef {
    pub ammo: usize,
    pub per_shot: i32,
    pub ready: &'static str,
    pub attack: &'static str,
    pub flash: &'static str,
}

pub const AM_CLIP: usize = 0;
pub const AM_SHELL: usize = 1;
pub const AM_ROCKET: usize = 2;
pub const AM_CELL: usize = 3;
pub const AM_NONE: usize = 4;
pub const MAX_AMMO: [i32; 5] = [200, 50, 50, 300, 0];

pub const WP_FIST: usize = 0;
pub const WP_PISTOL: usize = 1;
pub const WP_SHOTGUN: usize = 2;
pub const WP_CHAINGUN: usize = 3;

pub fn weapons() -> Vec<WeaponDef> {
    vec![
        WeaponDef { ammo: AM_NONE, per_shot: 0, ready: "PUNCH", attack: "PUNCH1", flash: "NULL" },
        WeaponDef { ammo: AM_CLIP, per_shot: 1, ready: "PISTOL", attack: "PISTOL1", flash: "PISTOLFLASH" },
        WeaponDef { ammo: AM_SHELL, per_shot: 1, ready: "SGUN", attack: "SGUN1", flash: "SGUNFLASH1" },
        WeaponDef { ammo: AM_CLIP, per_shot: 1, ready: "CHAIN", attack: "CHAIN1", flash: "CHAINFLASH1" },
    ]
}

struct B(Vec<StateDef>);

impl B {
    fn s(&mut self, name: &str, sprite: &'static str, frame: char, tics: i32, action: Action, next: &str) {
        let (bright, frame) = (frame.is_ascii_lowercase(), frame.to_ascii_uppercase() as u8 - b'A');
        self.0.push(StateDef { name: name.into(), sprite, frame, bright, tics, action, next: next.into() });
    }
    fn anim(&mut self, name: &str, sprite: &'static str, frames: &str, tics: i32) {
        let fs: Vec<char> = frames.chars().collect();
        for (i, &f) in fs.iter().enumerate() {
            let me = if i == 0 { name.to_string() } else { format!("{name}{i}") };
            let next = if fs.len() == 1 { "SELF".to_string() } else if i + 1 == fs.len() { name.to_string() } else { format!("{name}{}", i + 1) };
            let tics = if fs.len() == 1 { -1 } else { tics };
            let next = if next == "SELF" { me.clone() } else { next };
            self.s(&me, sprite, f, tics, Action::None, &next);
        }
    }
    fn seq(&mut self, name: &str, sprite: &'static str, frames: &[(char, i32, Action)], last: &str) {
        for (i, &(f, t, a)) in frames.iter().enumerate() {
            let me = if i == 0 { name.to_string() } else { format!("{name}{}", i + 1) };
            let next = if i + 1 == frames.len() { last.to_string() } else { format!("{name}{}", i + 2) };
            self.s(&me, sprite, f, t, a, &next);
        }
    }
}

pub fn states() -> Vec<StateDef> {
    use Action::*;
    let mut b = B(vec![]);
    b.s("NULL", "TROO", 'A', -1, None, "NULL");
    b.s("PUNCH", "PUNG", 'A', 1, WeaponReady, "PUNCH");
    b.seq("PUNCH1", "PUNG", &[('B', 4, None), ('C', 4, None), ('D', 5, None), ('C', 4, None), ('B', 5, ReFire)], "PUNCH");
    b.s("PISTOL", "PISG", 'A', 1, WeaponReady, "PISTOL");
    b.seq("PISTOL1", "PISG", &[('A', 4, None), ('B', 6, FirePistol), ('C', 4, None), ('B', 5, ReFire)], "PISTOL");
    b.s("PISTOLFLASH", "PISF", 'a', 7, None, "NULL");
    b.s("SGUN", "SHTG", 'A', 1, WeaponReady, "SGUN");
    b.seq(
        "SGUN1",
        "SHTG",
        &[('A', 3, None), ('A', 7, FireShotgun), ('B', 5, None), ('C', 5, None), ('D', 4, None), ('C', 5, None), ('B', 5, None), ('A', 3, None), ('A', 7, ReFire)],
        "SGUN",
    );
    b.seq("SGUNFLASH1", "SHTF", &[('a', 4, None), ('b', 3, None)], "NULL");
    b.s("CHAIN", "CHGG", 'A', 1, WeaponReady, "CHAIN");
    b.seq("CHAIN1", "CHGG", &[('A', 4, FireCGun), ('B', 4, FireCGun), ('B', 0, ReFire)], "CHAIN");
    b.seq("CHAINFLASH1", "CHGF", &[('a', 5, None)], "NULL");

    for (p, atk, atk_tics) in [("POSS", PosAttack, 8), ("SPOS", SPosAttack, 10)] {
        b.seq(&format!("{p}_STND"), p, &[('A', 10, Look), ('B', 10, Look)], &format!("{p}_STND"));
        b.seq(
            &format!("{p}_RUN"),
            p,
            &[('A', 4, Chase), ('A', 4, Chase), ('B', 4, Chase), ('B', 4, Chase), ('C', 4, Chase), ('C', 4, Chase), ('D', 4, Chase), ('D', 4, Chase)],
            &format!("{p}_RUN"),
        );
        b.seq(&format!("{p}_ATK"), p, &[('E', 10, FaceTarget), ('f', atk_tics, atk), ('E', atk_tics, None)], &format!("{p}_RUN"));
        b.seq(&format!("{p}_PAIN"), p, &[('G', 3, None), ('G', 3, Pain)], &format!("{p}_RUN"));
        b.seq(&format!("{p}_DIE"), p, &[('H', 5, None), ('I', 5, Scream), ('J', 5, Fall), ('K', 5, None), ('L', -1, None)], "NULL");
    }
    b.seq("TROO_STND", "TROO", &[('A', 10, Look), ('B', 10, Look)], "TROO_STND");
    b.seq(
        "TROO_RUN",
        "TROO",
        &[('A', 3, Chase), ('A', 3, Chase), ('B', 3, Chase), ('B', 3, Chase), ('C', 3, Chase), ('C', 3, Chase), ('D', 3, Chase), ('D', 3, Chase)],
        "TROO_RUN",
    );
    b.seq("TROO_ATK", "TROO", &[('E', 8, FaceTarget), ('F', 8, FaceTarget), ('G', 6, TroopAttack)], "TROO_RUN");
    b.seq("TROO_PAIN", "TROO", &[('H', 2, None), ('H', 2, Pain)], "TROO_RUN");
    b.seq("TROO_DIE", "TROO", &[('I', 8, None), ('J', 8, Scream), ('K', 6, None), ('L', 6, Fall), ('M', -1, None)], "NULL");
    b.seq("TBALL", "BAL1", &[('a', 4, None), ('b', 4, None)], "TBALL");
    b.seq("TBALLX", "BAL1", &[('c', 6, None), ('d', 6, None), ('e', 6, None)], "NULL");
    b.seq("SARG_STND", "SARG", &[('A', 10, Look), ('B', 10, Look)], "SARG_STND");
    b.seq(
        "SARG_RUN",
        "SARG",
        &[('A', 2, Chase), ('A', 2, Chase), ('B', 2, Chase), ('B', 2, Chase), ('C', 2, Chase), ('C', 2, Chase), ('D', 2, Chase), ('D', 2, Chase)],
        "SARG_RUN",
    );
    b.seq("SARG_ATK", "SARG", &[('E', 8, FaceTarget), ('F', 8, FaceTarget), ('G', 8, SargAttack)], "SARG_RUN");
    b.seq("SARG_PAIN", "SARG", &[('H', 2, None), ('H', 2, Pain)], "SARG_RUN");
    b.seq("SARG_DIE", "SARG", &[('I', 8, None), ('J', 8, Scream), ('K', 4, None), ('L', 4, Fall), ('M', 4, None), ('N', -1, None)], "NULL");
    b.anim("BAR1", "BAR1", "AB", 6);
    b.seq("BEXP", "BEXP", &[('a', 5, None), ('b', 5, Scream), ('c', 5, None), ('d', 10, Explode), ('e', 10, None)], "NULL");
    b.seq("PUFF", "PUFF", &[('a', 4, None), ('B', 4, None), ('C', 4, None), ('D', 4, None)], "NULL");
    b.seq("BLOOD", "BLUD", &[('C', 8, None), ('B', 8, None), ('A', 8, None)], "NULL");
    for (name, sprite, frames, tics) in DECOR_ANIMS {
        b.anim(name, sprite, frames, *tics);
    }
    b.0
}

const DECOR_ANIMS: &[(&str, &str, &str, i32)] = &[
    ("COLU", "COLU", "a", 0),
    ("SMIT", "SMIT", "A", 0),
    ("TRE1", "TRE1", "A", 0),
    ("TRE2", "TRE2", "A", 0),
    ("ELEC", "ELEC", "A", 0),
    ("MESS", "PLAY", "W", 0),
    ("DPLAY", "PLAY", "N", 0),
    ("DPOSS", "POSS", "L", 0),
    ("DSPOS", "SPOS", "L", 0),
    ("DTROO", "TROO", "M", 0),
    ("DSARG", "SARG", "N", 0),
    ("POL5", "POL5", "A", 0),
    ("GOR1", "GOR1", "ABCB", 10),
    ("GOR2", "GOR2", "A", 0),
    ("GOR3", "GOR3", "A", 0),
    ("GOR4", "GOR4", "A", 0),
    ("GOR5", "GOR5", "A", 0),
    ("POL1", "POL1", "A", 0),
    ("POL6", "POL6", "AB", 6),
    ("POL4", "POL4", "A", 0),
    ("POL2", "POL2", "A", 0),
    ("POL3", "POL3", "ab", 6),
    ("COL1", "COL1", "A", 0),
    ("COL2", "COL2", "A", 0),
    ("COL3", "COL3", "A", 0),
    ("COL4", "COL4", "A", 0),
    ("CAND", "CAND", "a", 0),
    ("CBRA", "CBRA", "a", 0),
    ("COL5", "COL5", "AB", 14),
    ("COL6", "COL6", "A", 0),
    ("CEYE", "CEYE", "abcb", 6),
    ("FSKU", "FSKU", "abc", 6),
    ("TBLU", "TBLU", "abcd", 4),
    ("TGRN", "TGRN", "abcd", 4),
    ("TRED", "TRED", "abcd", 4),
    ("SMBT", "SMBT", "abcd", 4),
    ("SMGT", "SMGT", "abcd", 4),
    ("SMRT", "SMRT", "abcd", 4),
    ("BON1", "BON1", "ABCDCB", 6),
    ("BON2", "BON2", "ABCDCB", 6),
    ("STIM", "STIM", "A", 0),
    ("MEDI", "MEDI", "A", 0),
    ("CLIP", "CLIP", "A", 0),
    ("AMMO", "AMMO", "A", 0),
    ("SHEL", "SHEL", "A", 0),
    ("SBOX", "SBOX", "A", 0),
    ("ROCK", "ROCK", "A", 0),
    ("BROK", "BROK", "A", 0),
    ("CELL", "CELL", "A", 0),
    ("CELP", "CELP", "A", 0),
    ("SHOT", "SHOT", "A", 0),
    ("MGUN", "MGUN", "A", 0),
    ("LAUN", "LAUN", "A", 0),
    ("PLAS", "PLAS", "A", 0),
    ("CSAW", "CSAW", "A", 0),
    ("BFUG", "BFUG", "A", 0),
    ("ARM1", "ARM1", "Ab", 6),
    ("ARM2", "ARM2", "Ab", 6),
    ("BKEY", "BKEY", "Ab", 10),
    ("YKEY", "YKEY", "Ab", 10),
    ("RKEY", "RKEY", "Ab", 10),
    ("BSKU", "BSKU", "Ab", 10),
    ("YSKU", "YSKU", "Ab", 10),
    ("RSKU", "RSKU", "Ab", 10),
    ("BPAK", "BPAK", "A", 0),
    ("SOUL", "SOUL", "abcdcb", 6),
    ("PSTR", "PSTR", "a", 0),
    ("PINV", "PINV", "abcd", 6),
    ("PINS", "PINS", "abcd", 6),
    ("SUIT", "SUIT", "a", 0),
    ("PMAP", "PMAP", "abcdcb", 6),
    ("PVIS", "PVIS", "Ab", 6),
];

fn m(doomednum: i64, name: &'static str, spawn: &str, radius: f64, height: f64, flags: u32, pickup: Pickup) -> MobjDef {
    MobjDef {
        doomednum,
        name,
        spawn: spawn.into(),
        see: "NULL".into(),
        pain: "NULL".into(),
        melee: "NULL".into(),
        missile: "NULL".into(),
        death: "NULL".into(),
        speed: 0.0,
        radius,
        height,
        health: 1000,
        painchance: 0,
        flags,
        pickup,
        drop: 0,
    }
}

fn monster(doomednum: i64, name: &'static str, p: &str, speed: f64, radius: f64, health: i32, painchance: i32, melee: bool, missile: bool, drop: i64) -> MobjDef {
    MobjDef {
        see: format!("{p}_RUN"),
        pain: format!("{p}_PAIN"),
        melee: if melee { format!("{p}_ATK") } else { "NULL".into() },
        missile: if missile { format!("{p}_ATK") } else { "NULL".into() },
        death: format!("{p}_DIE"),
        speed,
        health,
        painchance,
        drop,
        ..m(doomednum, name, &format!("{p}_STND"), radius, 56.0, SOLID | SHOOTABLE | MONSTER, Pickup::None)
    }
}

pub fn mobjs() -> Vec<MobjDef> {
    use Pickup::*;
    let deco = |n, s, r, solid: bool| m(n, s, s, r, 16.0, if solid { SOLID } else { 0 }, None);
    let hang = |n, s| m(n, s, s, 16.0, 68.0, SOLID | HANGING, None);
    let item = |n, s, p| m(n, s, s, 20.0, 16.0, PICKUP, p);
    vec![
        monster(3004, "POSS", "POSS", 8.0, 20.0, 20, 200, false, true, 2007),
        monster(9, "SPOS", "SPOS", 8.0, 20.0, 30, 170, false, true, 2001),
        monster(3001, "TROO", "TROO", 8.0, 20.0, 60, 200, true, true, 0),
        monster(3002, "SARG", "SARG", 10.0, 30.0, 150, 180, true, false, 0),
        monster(58, "SARG", "SARG", 10.0, 30.0, 150, 180, true, false, 0),
        MobjDef { death: "BEXP".into(), health: 20, ..m(2035, "BAR1", "BAR1", 10.0, 42.0, SOLID | SHOOTABLE | NOBLOOD, None) },
        MobjDef { death: "TBALLX".into(), speed: 10.0, ..m(-1, "TBALL", "TBALL", 6.0, 8.0, MISSILE, None) },
        m(-2, "PUFF", "PUFF", 1.0, 1.0, NOGRAV, None),
        m(-3, "BLOOD", "BLOOD", 1.0, 1.0, NOGRAV, None),
        deco(2028, "COLU", 16.0, true),
        deco(47, "SMIT", 16.0, true),
        deco(43, "TRE1", 16.0, true),
        deco(54, "TRE2", 32.0, true),
        deco(48, "ELEC", 16.0, true),
        deco(10, "MESS", 16.0, false),
        deco(12, "MESS", 16.0, false),
        deco(15, "DPLAY", 16.0, false),
        deco(18, "DPOSS", 16.0, false),
        deco(19, "DSPOS", 16.0, false),
        deco(20, "DTROO", 16.0, false),
        deco(21, "DSARG", 16.0, false),
        deco(24, "POL5", 16.0, false),
        deco(59, "GOR2", 16.0, false),
        deco(60, "GOR4", 16.0, false),
        deco(61, "GOR3", 16.0, false),
        deco(62, "GOR5", 16.0, false),
        deco(63, "GOR1", 16.0, false),
        hang(49, "GOR1"),
        hang(50, "GOR2"),
        hang(51, "GOR3"),
        hang(52, "GOR4"),
        hang(53, "GOR5"),
        deco(25, "POL1", 16.0, true),
        deco(26, "POL6", 16.0, true),
        deco(27, "POL4", 16.0, true),
        deco(28, "POL2", 16.0, true),
        deco(29, "POL3", 16.0, true),
        deco(30, "COL1", 16.0, true),
        deco(31, "COL2", 16.0, true),
        deco(32, "COL3", 16.0, true),
        deco(33, "COL4", 16.0, true),
        deco(34, "CAND", 20.0, false),
        deco(35, "CBRA", 16.0, true),
        deco(36, "COL5", 16.0, true),
        deco(37, "COL6", 16.0, true),
        deco(41, "CEYE", 16.0, true),
        deco(42, "FSKU", 16.0, true),
        deco(44, "TBLU", 16.0, true),
        deco(45, "TGRN", 16.0, true),
        deco(46, "TRED", 16.0, true),
        deco(55, "SMBT", 16.0, true),
        deco(56, "SMGT", 16.0, true),
        deco(57, "SMRT", 16.0, true),
        item(2014, "BON1", Health { amount: 1, max: 200 }),
        item(2015, "BON2", Armor { amount: 1, class: 1, bonus: true }),
        item(2011, "STIM", Health { amount: 10, max: 100 }),
        item(2012, "MEDI", Health { amount: 25, max: 100 }),
        item(2013, "SOUL", Health { amount: 100, max: 200 }),
        item(2007, "CLIP", Ammo { kind: AM_CLIP, amount: 10 }),
        item(2048, "AMMO", Ammo { kind: AM_CLIP, amount: 50 }),
        item(2008, "SHEL", Ammo { kind: AM_SHELL, amount: 4 }),
        item(2049, "SBOX", Ammo { kind: AM_SHELL, amount: 20 }),
        item(2010, "ROCK", Ammo { kind: AM_ROCKET, amount: 1 }),
        item(2046, "BROK", Ammo { kind: AM_ROCKET, amount: 5 }),
        item(2047, "CELL", Ammo { kind: AM_CELL, amount: 20 }),
        item(17, "CELP", Ammo { kind: AM_CELL, amount: 100 }),
        item(2001, "SHOT", Weapon { weapon: WP_SHOTGUN, kind: AM_SHELL, amount: 8 }),
        item(2002, "MGUN", Weapon { weapon: WP_CHAINGUN, kind: AM_CLIP, amount: 20 }),
        item(2003, "LAUN", Ammo { kind: AM_ROCKET, amount: 2 }),
        item(2004, "PLAS", Ammo { kind: AM_CELL, amount: 40 }),
        item(2005, "CSAW", None),
        item(2006, "BFUG", Ammo { kind: AM_CELL, amount: 40 }),
        item(2018, "ARM1", Armor { amount: 100, class: 1, bonus: false }),
        item(2019, "ARM2", Armor { amount: 200, class: 2, bonus: false }),
        item(5, "BKEY", Key(1)),
        item(6, "YKEY", Key(2)),
        item(13, "RKEY", Key(4)),
        item(40, "BSKU", Key(1)),
        item(39, "YSKU", Key(2)),
        item(38, "RSKU", Key(4)),
        item(8, "BPAK", Backpack),
        item(2023, "PSTR", Health { amount: 100, max: 100 }),
        item(2022, "PINV", None),
        item(2024, "PINS", None),
        item(2025, "SUIT", None),
        item(2026, "PMAP", None),
        item(2045, "PVIS", None),
    ]
}
