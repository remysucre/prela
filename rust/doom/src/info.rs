// What a state does when entered; rule tables are keyed by this.
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
    Hurt,
    Move,
}

// A state as written in the tables below, using names; db.rs resolves them to ids.
pub struct StateDef {
    pub name: String,
    pub sprite: &'static str,
    pub frame: u8,
    pub bright: bool,
    pub tics: i32,
    pub action: Action,
    pub next: String,
}

// Thing flags.
pub const SOLID: u32 = 1;
pub const SHOOTABLE: u32 = 2;
pub const MONSTER: u32 = 4;
pub const PICKUP: u32 = 8;
pub const MISSILE: u32 = 16;
pub const NOBLOOD: u32 = 32;
pub const HANGING: u32 = 64;
pub const NOGRAV: u32 = 128;

// What touching an item gives the player; unused columns stay zero.
#[derive(Clone, Copy, Debug)]
pub struct PickupDef {
    pub health: i32,
    pub health_max: i32,
    pub armor: i32,
    pub armor_max: i32,
    pub class_set: i32,
    pub class_min: i32,
    pub ammo_kind: usize,
    pub ammo: i32,
    pub weapon: usize,
    pub weapon_bit: u8,
    pub key: u8,
    pub always: bool,
}

// Constructors for pickup rows; `always` items are taken even when they change nothing.
mod pickup {
    use super::*;

    pub const NOTHING: PickupDef = PickupDef {
        health: 0,
        health_max: 0,
        armor: 0,
        armor_max: 0,
        class_set: 0,
        class_min: 0,
        ammo_kind: AM_NONE,
        ammo: 0,
        weapon: 0,
        weapon_bit: 0,
        key: 0,
        always: false,
    };
    pub const POWERUP: PickupDef = PickupDef { always: true, ..NOTHING };
    pub const BACKPACK: PickupDef = PickupDef { ammo_kind: AM_CLIP, ammo: 10, always: true, ..NOTHING };
    pub const BONUS_ARMOR: PickupDef = PickupDef { armor: 1, armor_max: 200, class_min: 1, ..NOTHING };

    pub fn health(amount: i32, max: i32) -> PickupDef {
        PickupDef { health: amount, health_max: max, ..NOTHING }
    }
    pub fn armor(amount: i32, class: i32) -> PickupDef {
        PickupDef { armor: amount, armor_max: amount, class_set: class, ..NOTHING }
    }
    pub fn ammo(kind: usize, amount: i32) -> PickupDef {
        PickupDef { ammo_kind: kind, ammo: amount, ..NOTHING }
    }
    pub fn weapon(weapon: usize, kind: usize, amount: i32) -> PickupDef {
        PickupDef { ammo_kind: kind, ammo: amount, weapon, weapon_bit: 1 << weapon, always: true, ..NOTHING }
    }
    pub fn key(k: u8) -> PickupDef {
        PickupDef { key: k, always: true, ..NOTHING }
    }
}

// A thing kind as written below, using state names.
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
    pub damage: i32,
    pub flags: u32,
    pub pickup: Option<PickupDef>,
    pub drop: i64,
}

// A weapon as written below, using state names.
pub struct WeaponDef {
    pub ammo: usize,
    pub per_shot: i32,
    pub ready: &'static str,
    pub attack: &'static str,
    pub flash: &'static str,
    pub damage: i32,
    pub damage_roll: i32,
    pub range: f64,
    pub spread_div: f64,
}

// Ammo kinds.
pub const AM_CLIP: usize = 0;
pub const AM_SHELL: usize = 1;
pub const AM_ROCKET: usize = 2;
pub const AM_CELL: usize = 3;
pub const AM_NONE: usize = 4;

// One ammo kind: carrying cap and starting amount.
#[derive(Clone, Copy, Debug)]
pub struct AmmoDef {
    pub cap: i32,
    pub start: i32,
}

// Ammo table, indexed by kind; AM_NONE (fists) has cap 0.
pub fn ammo_defs() -> Vec<AmmoDef> {
    let a = |cap, start| AmmoDef { cap, start };
    vec![a(200, 50), a(50, 0), a(50, 0), a(300, 0), a(0, 0)]
}

// Weapon slots (0 is fists).
pub const WP_PISTOL: usize = 1;
pub const WP_SHOTGUN: usize = 2;
pub const WP_CHAINGUN: usize = 3;

// Weapon table: ammo use, states, and bullet damage (damage * (1..=damage_roll)), range, spread (angle units of TAU / spread_div).
pub fn weapons() -> Vec<WeaponDef> {
    let w = |ammo, per_shot, ready, attack, flash| WeaponDef { ammo, per_shot, ready, attack, flash, damage: 5, damage_roll: 3, range: 2048.0, spread_div: 16384.0 };
    vec![
        w(AM_NONE, 0, "PUNCH", "PUNCH1", "NULL"),
        w(AM_CLIP, 1, "PISTOL", "PISTOL1", "PISTOLFLASH"),
        w(AM_SHELL, 1, "SGUN", "SGUN1", "SGUNFLASH1"),
        w(AM_CLIP, 1, "CHAIN", "CHAIN1", "CHAINFLASH1"),
    ]
}

// Helper for writing the state table: s = one state, anim = looping frames, seq = a chain ending in `last`.
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

// The animation state table (Doom's info.c states), trimmed to what E1M1 uses.
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

// Decorations and items that just loop frames: (name, sprite, frames, tics).
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

// Default thing-kind row; monster() and the closures in mobjs() fill in the rest.
fn m(doomednum: i64, name: &'static str, spawn: &str, radius: f64, height: f64, flags: u32, pickup: Option<PickupDef>) -> MobjDef {
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
        damage: 0,
        flags,
        pickup,
        drop: 0,
    }
}

// Monster row; its states follow the naming pattern <p>_STND/_RUN/_ATK/_PAIN/_DIE.
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
        ..m(doomednum, name, &format!("{p}_STND"), radius, 56.0, SOLID | SHOOTABLE | MONSTER, None)
    }
}

// Thing-kind table (Doom's mobjinfo), trimmed to what E1M1 uses.
pub fn mobjs() -> Vec<MobjDef> {
    use pickup::*;
    let deco = |n, s, r, solid: bool| m(n, s, s, r, 16.0, if solid { SOLID } else { 0 }, None);
    let hang = |n, s| m(n, s, s, 16.0, 68.0, SOLID | HANGING, None);
    let item = |n, s, p| m(n, s, s, 20.0, 16.0, PICKUP, Some(p));
    vec![
        monster(3004, "POSS", "POSS", 8.0, 20.0, 20, 200, false, true, 2007),
        monster(9, "SPOS", "SPOS", 8.0, 20.0, 30, 170, false, true, 2001),
        monster(3001, "TROO", "TROO", 8.0, 20.0, 60, 200, true, true, 0),
        monster(3002, "SARG", "SARG", 10.0, 30.0, 150, 180, true, false, 0),
        monster(58, "SARG", "SARG", 10.0, 30.0, 150, 180, true, false, 0),
        MobjDef { death: "BEXP".into(), health: 20, ..m(2035, "BAR1", "BAR1", 10.0, 42.0, SOLID | SHOOTABLE | NOBLOOD, None) },
        MobjDef { death: "TBALLX".into(), speed: 10.0, damage: 3, ..m(-1, "TBALL", "TBALL", 6.0, 8.0, MISSILE, None) },
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
        item(2014, "BON1", health(1, 200)),
        item(2015, "BON2", BONUS_ARMOR),
        item(2011, "STIM", health(10, 100)),
        item(2012, "MEDI", health(25, 100)),
        item(2013, "SOUL", health(100, 200)),
        item(2007, "CLIP", ammo(AM_CLIP, 10)),
        item(2048, "AMMO", ammo(AM_CLIP, 50)),
        item(2008, "SHEL", ammo(AM_SHELL, 4)),
        item(2049, "SBOX", ammo(AM_SHELL, 20)),
        item(2010, "ROCK", ammo(AM_ROCKET, 1)),
        item(2046, "BROK", ammo(AM_ROCKET, 5)),
        item(2047, "CELL", ammo(AM_CELL, 20)),
        item(17, "CELP", ammo(AM_CELL, 100)),
        item(2001, "SHOT", weapon(WP_SHOTGUN, AM_SHELL, 8)),
        item(2002, "MGUN", weapon(WP_CHAINGUN, AM_CLIP, 20)),
        item(2003, "LAUN", ammo(AM_ROCKET, 2)),
        item(2004, "PLAS", ammo(AM_CELL, 40)),
        item(2005, "CSAW", POWERUP),
        item(2006, "BFUG", ammo(AM_CELL, 40)),
        item(2018, "ARM1", armor(100, 1)),
        item(2019, "ARM2", armor(200, 2)),
        item(5, "BKEY", key(1)),
        item(6, "YKEY", key(2)),
        item(13, "RKEY", key(4)),
        item(40, "BSKU", key(1)),
        item(39, "YSKU", key(2)),
        item(38, "RSKU", key(4)),
        item(8, "BPAK", BACKPACK),
        item(2023, "PSTR", health(100, 100)),
        item(2022, "PINV", POWERUP),
        item(2024, "PINS", POWERUP),
        item(2025, "SUIT", POWERUP),
        item(2026, "PMAP", POWERUP),
        item(2045, "PVIS", POWERUP),
    ]
}

// Monster facts, one bit each; an AI rule matches when (facts & mask) == want.
pub mod fact {
    pub const DEAD: u32 = 1;
    pub const SEES: u32 = 1 << 1;
    pub const HEARD: u32 = 1 << 2;
    pub const AHEAD: u32 = 1 << 3;
    pub const CLOSE: u32 = 1 << 4;
    pub const MELEE_RANGE: u32 = 1 << 5;
    pub const HAS_MELEE: u32 = 1 << 6;
    pub const HAS_MISSILE: u32 = 1 << 7;
    pub const REACTION0: u32 = 1 << 8;
    pub const ROLL: u32 = 1 << 9;
    pub const KILLED: u32 = 1 << 10;
    pub const PAIN_ROLL: u32 = 1 << 11;
    pub const ASLEEP: u32 = 1 << 12;
    pub const HAS_SEE: u32 = 1 << 13;
    pub const FLYING: u32 = 1 << 14;
    pub const CLIP_OK: u32 = 1 << 15;
    pub const HIT: u32 = 1 << 16;
    pub const GO: u32 = 1 << 17;
    pub const STAY: u32 = 1 << 18;
}

// One rule row: if (facts & mask) == want, output `out`; the lowest prio wins.
#[derive(Clone, Copy, Debug)]
pub struct Rule<O> {
    pub prio: u8,
    pub mask: u32,
    pub want: u32,
    pub out: O,
}

// Attack a monster performs this tic: kind, bullets, and damage mult * (1..=modulo).
#[derive(Clone, Copy, Debug)]
pub struct Attack {
    pub kind: u8,
    pub pellets: usize,
    pub mult: i32,
    pub modulo: i32,
}

// Attack kinds.
pub const ATK_NONE: u8 = 0;
pub const ATK_HITSCAN: u8 = 1;
pub const ATK_MELEE: u8 = 2;
pub const ATK_BALL: u8 = 3;

pub const NO_ATTACK: Attack = Attack { kind: ATK_NONE, pellets: 0, mult: 0, modulo: 1 };

fn hitscan(pellets: usize, mult: i32, modulo: i32) -> Attack {
    Attack { kind: ATK_HITSCAN, pellets, mult, modulo }
}

fn melee(mult: i32, modulo: i32) -> Attack {
    Attack { kind: ATK_MELEE, pellets: 0, mult, modulo }
}

const BALL: Attack = Attack { kind: ATK_BALL, ..NO_ATTACK };

// Goto labels: which of the thing's states (Info.see, .pain, ...) to jump to.
pub const G_NONE: u8 = 0;
pub const G_SEE: u8 = 1;
pub const G_SPAWN: u8 = 2;
pub const G_MELEE: u8 = 3;
pub const G_MISSILE: u8 = 4;
pub const G_DEATH: u8 = 5;
pub const G_PAIN: u8 = 6;

// What a matched AI rule does to the monster.
#[derive(Clone, Copy, Debug)]
pub struct AiOut {
    pub goto: u8,
    pub face: bool,
    pub awake: i8,
    pub reaction: i32,
    pub reaction_dec: bool,
    pub attack: Attack,
    pub go: bool,
    pub clear_solid: bool,
    pub explode: bool,
    pub clear_shootable: bool,
    pub reset: bool,
    pub to_clip: bool,
    pub face_move: bool,
    pub stop: bool,
}

// Do nothing; rules override fields of this.
pub const KEEP: AiOut = AiOut {
    goto: G_NONE,
    face: false,
    awake: 0,
    reaction: -1,
    reaction_dec: false,
    attack: NO_ATTACK,
    go: false,
    clear_solid: false,
    explode: false,
    clear_shootable: false,
    reset: false,
    to_clip: false,
    face_move: false,
    stop: false,
};

// Monster AI as rules, keyed by the current state's action (replaces A_Look, A_Chase, etc.).
pub fn ai_rules() -> Vec<(Action, Rule<AiOut>)> {
    use fact::*;
    use Action::*;
    let r = |prio, mask, out| Rule { prio, mask, want: mask, out };
    let wake = AiOut { goto: G_SEE, face: true, awake: 1, reaction: 8, ..KEEP };
    let chase = AiOut { reaction_dec: true, ..KEEP };
    vec![
        (Look, r(0, DEAD, KEEP)),
        (Look, r(1, HEARD, wake)),
        (Look, r(2, SEES | AHEAD, wake)),
        (Look, r(3, SEES | CLOSE, wake)),
        (Chase, r(0, DEAD, AiOut { goto: G_SPAWN, awake: -1, ..KEEP })),
        (Chase, r(1, HAS_MELEE | MELEE_RANGE, AiOut { goto: G_MELEE, face: true, ..chase })),
        (Chase, r(2, HAS_MISSILE | REACTION0 | SEES | ROLL, AiOut { goto: G_MISSILE, face: true, ..chase })),
        (Chase, r(3, 0, AiOut { go: true, ..chase })),
        (FaceTarget, r(0, 0, AiOut { face: true, ..KEEP })),
        (PosAttack, r(0, 0, AiOut { face: true, attack: hitscan(1, 3, 5), ..KEEP })),
        (SPosAttack, r(0, 0, AiOut { face: true, attack: hitscan(3, 3, 5), ..KEEP })),
        (TroopAttack, r(0, MELEE_RANGE, AiOut { attack: melee(3, 8), ..KEEP })),
        (TroopAttack, r(1, 0, AiOut { face: true, attack: BALL, ..KEEP })),
        (SargAttack, r(0, MELEE_RANGE, AiOut { attack: melee(4, 10), ..KEEP })),
        (Fall, r(0, 0, AiOut { clear_solid: true, ..KEEP })),
        (Explode, r(0, 0, AiOut { explode: true, ..KEEP })),
        (Hurt, r(0, KILLED, AiOut { goto: G_DEATH, clear_shootable: true, reset: true, ..KEEP })),
        (Hurt, r(1, PAIN_ROLL, AiOut { goto: G_PAIN, awake: 1, ..KEEP })),
        (Hurt, r(2, ASLEEP | HAS_SEE, AiOut { goto: G_SEE, awake: 1, reaction: 0, ..KEEP })),
        (Hurt, r(3, 0, AiOut { awake: 1, ..KEEP })),
        (Move, Rule { prio: 0, mask: FLYING | CLIP_OK | HIT, want: FLYING | CLIP_OK, out: AiOut { to_clip: true, ..KEEP } }),
        (Move, r(1, FLYING, AiOut { goto: G_DEATH, stop: true, ..KEEP })),
        (Move, Rule { prio: 2, mask: GO | CLIP_OK | STAY, want: GO | CLIP_OK, out: AiOut { to_clip: true, face_move: true, ..KEEP } }),
    ]
}

// Weapon facts for the weapon rules.
pub mod wfact {
    pub const FIRE: u32 = 1;
    pub const ALIVE: u32 = 1 << 1;
    pub const AMMO: u32 = 1 << 2;
}

// What a matched weapon rule does: jump to the attack state, or fire n pellets.
#[derive(Clone, Copy, Debug, Default)]
pub struct WOut {
    pub redirect: bool,
    pub pellets: usize,
}

// Weapon behaviour as rules, keyed by the weapon state's action.
pub fn weapon_rules() -> Vec<(Action, Rule<WOut>)> {
    use wfact::*;
    use Action::*;
    let r = |mask, out| Rule { prio: 0, mask, want: mask, out };
    let redirect = WOut { redirect: true, pellets: 0 };
    vec![
        (WeaponReady, r(FIRE | ALIVE | AMMO, redirect)),
        (ReFire, r(FIRE | ALIVE | AMMO, redirect)),
        (FirePistol, r(AMMO, WOut { redirect: false, pellets: 1 })),
        (FireCGun, r(AMMO, WOut { redirect: false, pellets: 1 })),
        (FireShotgun, r(AMMO, WOut { redirect: false, pellets: 7 })),
    ]
}

// Sector mover kinds (doors, lifts, floors) and the exit special.
pub const MV_IDLE: u8 = 0;
pub const MV_DOOR: u8 = 1;
pub const MV_DOOR_STAY: u8 = 2;
pub const MV_LIFT: u8 = 3;
pub const MV_FLOOR: u8 = 4;
pub const MV_EXIT: u8 = 9;

// What a line special does: which mover, speed, whether it moves tagged sectors or the sector behind it, key needed, walk vs use.
#[derive(Clone, Copy, Debug)]
pub struct Special {
    pub act: u8,
    pub speed: f64,
    pub tagged: bool,
    pub key: u8,
    pub walk: bool,
    pub use_: bool,
}

// Line special number -> what it does.
pub fn specials() -> Vec<(i64, Special)> {
    let s = |act, speed, tagged, key, walk| Special { act, speed, tagged, key, walk, use_: !walk };
    vec![
        (1, s(MV_DOOR, 2.0, false, 0, false)),
        (26, s(MV_DOOR, 2.0, false, 1, false)),
        (27, s(MV_DOOR, 2.0, false, 2, false)),
        (28, s(MV_DOOR, 2.0, false, 4, false)),
        (117, s(MV_DOOR, 8.0, false, 0, false)),
        (2, s(MV_DOOR_STAY, 2.0, true, 0, true)),
        (62, s(MV_LIFT, 4.0, true, 0, false)),
        (88, s(MV_LIFT, 4.0, true, 0, true)),
        (23, s(MV_FLOOR, 1.0, true, 0, false)),
        (11, s(MV_EXIT, 0.0, false, 0, false)),
    ]
}

// What happens to a sector's mover when a special hits it; heights are weighted sums of ceiling/floor values.
#[derive(Clone, Copy, Debug)]
pub struct ActOut {
    pub kind: u8,
    pub dir: i8,
    pub init: bool,
    pub plane: u8,
    pub up_lc: f64,
    pub up_floor: f64,
    pub down_floor: f64,
    pub down_lowest: f64,
}

// Key for (current mover kind, direction, incoming special).
pub fn act_key(kind: u8, dir: i8, act: u8) -> usize {
    kind as usize * 1000 + (dir + 1) as usize * 100 + act as usize
}

// Mover activation table: starting a door/lift, or reversing a moving door.
pub fn activations() -> Vec<(usize, ActOut)> {
    let door = ActOut { kind: MV_DOOR, dir: 1, init: true, plane: 0, up_lc: 1.0, up_floor: 0.0, down_floor: 1.0, down_lowest: 0.0 };
    let lift = ActOut { kind: MV_LIFT, dir: -1, init: true, plane: 1, up_lc: 0.0, up_floor: 1.0, down_floor: 0.0, down_lowest: 1.0 };
    let reverse = |dir| ActOut { kind: MV_DOOR, dir, init: false, ..door };
    vec![
        (act_key(MV_IDLE, 0, MV_DOOR), door),
        (act_key(MV_IDLE, 0, MV_DOOR_STAY), ActOut { kind: MV_DOOR_STAY, ..door }),
        (act_key(MV_IDLE, 0, MV_LIFT), lift),
        (act_key(MV_IDLE, 0, MV_FLOOR), ActOut { kind: MV_FLOOR, ..lift }),
        (act_key(MV_DOOR, -1, MV_DOOR), reverse(1)),
        (act_key(MV_DOOR, 0, MV_DOOR), reverse(-1)),
        (act_key(MV_DOOR, 1, MV_DOOR), reverse(-1)),
    ]
}

// Mover facts for the mover rules.
pub mod mfact {
    pub const REACHED: u32 = 1;
    pub const EXPIRED: u32 = 1 << 1;
    pub const OCCUPIED: u32 = 1 << 2;
}

// What a matched mover rule does: new kind, direction, and wait timer.
#[derive(Clone, Copy, Debug)]
pub struct StepOut {
    pub kind: u8,
    pub dir: i8,
    pub wait: i32,
    pub wait_dec: bool,
}

// Key for (mover kind, direction).
pub fn step_key(kind: u8, dir: i8) -> usize {
    kind as usize * 10 + (dir + 1) as usize
}

// Per-tic mover behaviour: stop at the top, wait, go back down, reopen if blocked.
pub fn mover_rules() -> Vec<(usize, Rule<StepOut>)> {
    use mfact::*;
    let r = |prio, mask, kind, dir, wait, wait_dec| Rule { prio, mask, want: mask, out: StepOut { kind, dir, wait, wait_dec } };
    vec![
        (step_key(MV_DOOR, 1), r(0, REACHED, MV_DOOR, 0, 150, false)),
        (step_key(MV_DOOR_STAY, 1), r(0, REACHED, MV_IDLE, 0, -1, false)),
        (step_key(MV_DOOR, 0), r(0, EXPIRED, MV_DOOR, -1, -1, false)),
        (step_key(MV_DOOR, 0), r(1, 0, MV_DOOR, 0, -1, true)),
        (step_key(MV_DOOR, -1), r(0, OCCUPIED, MV_DOOR, 1, -1, false)),
        (step_key(MV_DOOR, -1), r(1, REACHED, MV_IDLE, 0, -1, false)),
        (step_key(MV_LIFT, -1), r(0, REACHED, MV_LIFT, 0, 105, false)),
        (step_key(MV_LIFT, 0), r(0, EXPIRED, MV_LIFT, 1, -1, false)),
        (step_key(MV_LIFT, 0), r(1, 0, MV_LIFT, 0, -1, true)),
        (step_key(MV_LIFT, 1), r(0, REACHED, MV_IDLE, 0, -1, false)),
        (step_key(MV_FLOOR, -1), r(0, REACHED, MV_IDLE, 0, -1, false)),
    ]
}

// Damaging floors: sector special -> damage every 32 tics.
pub fn sector_damage() -> Vec<(i64, i32)> {
    vec![(5, 10), (7, 5), (4, 20), (16, 20)]
}

// One direction a chasing monster tries: turn from its heading, and 0 step to stand still.
#[derive(Clone, Copy, Debug)]
pub struct ChaseDir {
    pub turn: f64,
    pub step: f64,
    pub stay: bool,
}

// Chase directions in preference order: straight, 45° either way, 90° either way, stay put.
pub fn chase_dirs() -> Vec<ChaseDir> {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};
    let d = |turn, step| ChaseDir { turn, step, stay: step == 0.0 };
    vec![d(0.0, 1.0), d(FRAC_PI_4, 1.0), d(-FRAC_PI_4, 1.0), d(FRAC_PI_2, 1.0), d(-FRAC_PI_2, 1.0), d(0.0, 0.0)]
}

// One player move attempt: which momentum axes it keeps.
#[derive(Clone, Copy, Debug)]
pub struct Slide {
    pub keep_x: f64,
    pub keep_y: f64,
    pub stay: bool,
}

// Player move attempts in preference order: full move, x only, y only, stay (gives wall sliding).
pub fn slides() -> Vec<Slide> {
    let s = |keep_x, keep_y| Slide { keep_x, keep_y, stay: keep_x == 0.0 && keep_y == 0.0 };
    vec![s(1.0, 1.0), s(1.0, 0.0), s(0.0, 1.0), s(0.0, 0.0)]
}

// Status bar fonts; glyph 10 of the big font is the percent sign.
pub const FONT_BIG: usize = 0;
pub const FONT_YELLOW: usize = 1;
pub const FONT_GREY: usize = 2;

// Font table: (font, glyph) -> lump name.
pub fn hud_fonts() -> Vec<((usize, usize), String)> {
    let mut v: Vec<((usize, usize), String)> = vec![((FONT_BIG, 10), "STTPRCNT".into())];
    for d in 0..10 {
        v.push(((FONT_BIG, d), format!("STTNUM{d}")));
        v.push(((FONT_YELLOW, d), format!("STYSNUM{d}")));
        v.push(((FONT_GREY, d), format!("STGNUM{d}")));
    }
    v
}

// Fixed status bar images: lump, x, y, depth (lower draws on top).
pub fn hud_patches() -> Vec<(&'static str, f64, f64, f32)> {
    vec![("STBAR", 0.0, 168.0, 0.2), ("STARMS", 104.0, 168.0, 0.1)]
}

// Stats shown as numbers; STAT_AMMO0 + kind and STAT_MAX0 + kind for the ammo tallies.
pub const STAT_AMMO: usize = 0;
pub const STAT_HEALTH: usize = 1;
pub const STAT_ARMOR: usize = 2;
pub const STAT_AMMO0: usize = 3;
pub const STAT_MAX0: usize = 7;

// Where one stat is drawn: right edge, top, font, max digits, trailing percent sign.
#[derive(Clone, Copy, Debug)]
pub struct NumberSlot {
    pub x: f64,
    pub y: f64,
    pub font: usize,
    pub digits: usize,
    pub percent: bool,
}

// Number widgets, indexed by stat.
pub fn hud_numbers() -> Vec<NumberSlot> {
    let n = |x, y, font, percent| NumberSlot { x, y, font, digits: 3, percent };
    let ys = [173.0, 179.0, 185.0, 191.0];
    let mut v = vec![n(44.0, 171.0, FONT_BIG, false), n(90.0, 171.0, FONT_BIG, true), n(221.0, 171.0, FONT_BIG, true)];
    v.extend(ys.map(|y| n(288.0, y, FONT_YELLOW, false)));
    v.extend(ys.map(|y| n(314.0, y, FONT_YELLOW, false)));
    v
}

// Arms panel: weapon slot -> (x, y, digit shown); yellow when owned, grey otherwise.
pub fn hud_arms() -> Vec<(usize, f64, f64, usize)> {
    (1..7).map(|w| (w, 111.0 + ((w - 1) % 3) as f64 * 12.0, 172.0 + ((w - 1) / 3) as f64 * 10.0, w + 1)).collect()
}

// Key cards, indexed by key bit: x, y, lump.
pub fn hud_keys() -> Vec<(f64, f64, &'static str)> {
    vec![(239.0, 171.0, "STKEYS0"), (239.0, 181.0, "STKEYS1"), (239.0, 191.0, "STKEYS2")]
}

// Face facts, one bit each.
pub mod ffact {
    pub const DEAD: u32 = 1;
    pub const HURT: u32 = 1 << 1;
    pub const GRIN: u32 = 1 << 2;
}

// Face kinds.
pub const FACE_LOOK: u8 = 0;
pub const FACE_KILL: u8 = 1;
pub const FACE_EVIL: u8 = 2;
pub const FACE_DEAD: u8 = 3;

// Which face to show, and whether its image depends on the pain level and the look direction.
#[derive(Clone, Copy, Debug)]
pub struct FaceOut {
    pub kind: u8,
    pub by_pain: bool,
    pub by_look: bool,
}

// Face rules: dead, else evil grin on a new weapon, else grimace when hurt, else look around.
pub fn face_rules() -> Vec<Rule<FaceOut>> {
    use ffact::*;
    let r = |prio, mask, kind, by_pain, by_look| Rule { prio, mask, want: mask, out: FaceOut { kind, by_pain, by_look } };
    vec![r(0, DEAD, FACE_DEAD, false, false), r(1, GRIN, FACE_EVIL, true, false), r(2, HURT, FACE_KILL, true, false), r(3, 0, FACE_LOOK, true, true)]
}

// Face images: (kind, pain level 0-4, look 0-2) -> lump.
pub fn faces() -> Vec<((u8, usize, usize), String)> {
    let mut v = vec![((FACE_DEAD, 0, 0), "STFDEAD0".to_string())];
    for p in 0..5 {
        for l in 0..3 {
            v.push(((FACE_LOOK, p, l), format!("STFST{p}{l}")));
        }
        v.push(((FACE_KILL, p, 0), format!("STFKILL{p}")));
        v.push(((FACE_EVIL, p, 0), format!("STFEVL{p}")));
    }
    v
}

// Every status bar lump the WAD loader should read.
pub fn hud_lumps() -> Vec<String> {
    let mut v: Vec<String> = hud_fonts().into_iter().map(|(_, n)| n).collect();
    v.extend(hud_patches().into_iter().map(|(n, _, _, _)| n.to_string()));
    v.extend(hud_keys().into_iter().map(|(_, _, n)| n.to_string()));
    v.extend(faces().into_iter().map(|(_, n)| n));
    v
}

// Armor classes, indexed by class: the share of damage armor absorbs (num / den).
#[derive(Clone, Copy, Debug)]
pub struct ArmorClass {
    pub num: i32,
    pub den: i32,
}

// No armor, green (1/3), blue (1/2).
pub fn armor_classes() -> Vec<ArmorClass> {
    vec![ArmorClass { num: 0, den: 1 }, ArmorClass { num: 1, den: 3 }, ArmorClass { num: 1, den: 2 }]
}

// Doom's #defines and tuning numbers, as one table row (SQLDoom's doom_constants); code reads it with `consts(db)`.
#[derive(Clone, Copy, Debug)]
pub struct Consts {
    pub friction: f64,
    pub walk_thrust: f64,
    pub run_thrust: f64,
    pub strafe_scale: f64,
    pub turn_walk: f64,
    pub turn_run: f64,
    pub gravity: f64,
    pub player_radius: f64,
    pub player_height: f64,
    pub view_height: f64,
    pub dead_view_height: f64,
    pub max_step: f64,
    pub use_range: f64,
    pub door_gap: f64,
    pub melee_range: f64,
    pub close_range: f64,
    pub missile_near: f64,
    pub missile_no_melee: f64,
    pub missile_far: f64,
    pub sight_z: f64,
    pub shot_z: f64,
    pub attack_range: f64,
    pub hitscan_spread_div: f64,
    pub puff_back: f64,
    pub blast_radius: f64,
    pub missile_reach: f64,
    pub missile_z_reach: f64,
    pub pickup_z_reach: f64,
    pub bonus_add: i32,
    pub damage_cap: i32,
    pub floor_damage_period: u64,
    pub reaction: i32,
    pub start_health: i32,
    pub start_owned: u8,
    pub start_weapon: usize,
}

pub fn consts() -> Consts {
    Consts {
        friction: 0.90625,
        walk_thrust: 0.78125,
        run_thrust: 1.5625,
        strafe_scale: 0.96,
        turn_walk: 0.0614,
        turn_run: 0.1227,
        gravity: 1.0,
        player_radius: 16.0,
        player_height: 56.0,
        view_height: 41.0,
        dead_view_height: 8.0,
        max_step: 24.0,
        use_range: 64.0,
        door_gap: 4.0,
        melee_range: 60.0,
        close_range: 64.0,
        missile_near: 64.0,
        missile_no_melee: 128.0,
        missile_far: 200.0,
        sight_z: 40.0,
        shot_z: 32.0,
        attack_range: 2048.0,
        hitscan_spread_div: 4096.0,
        puff_back: 4.0,
        blast_radius: 128.0,
        missile_reach: 6.0,
        missile_z_reach: 8.0,
        pickup_z_reach: 16.0,
        bonus_add: 6,
        damage_cap: 100,
        floor_damage_period: 32,
        reaction: 8,
        start_health: 100,
        start_owned: 0b11,
        start_weapon: WP_PISTOL,
    }
}
