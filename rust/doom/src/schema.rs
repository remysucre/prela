use crate::info::Action;
pub use prela::engine::Id;
use prela::engine::{Drive, IntoQuery, QueryExt};
use std::rc::Rc;
use prela::loader::{Col, Key, Set};

// Static map entities. wad.rs fills the columns read from the WAD; columns marked
// "derived" start empty and are computed by db.rs with queries over the others.
// Missing references (no back side, no texture) hold Id NONE and probe to nothing.

#[derive(IntoQuery)]
pub struct Vertex {
    #[primary_key]
    pub id: Key<Self>,
    pub x: Col<Self, f64>,
    pub y: Col<Self, f64>,
}

#[derive(IntoQuery)]
pub struct Sidedef {
    #[primary_key]
    pub id: Key<Self>,
    pub xoff: Col<Self, f64>,
    pub yoff: Col<Self, f64>,
    pub upper: Col<Self, Id<Image>>,
    pub lower: Col<Self, Id<Image>>,
    pub middle: Col<Self, Id<Image>>,
    pub sector: Col<Self, Id<Sector>>,
}

#[derive(IntoQuery)]
pub struct Linedef {
    #[primary_key]
    pub id: Key<Self>,
    pub v1: Col<Self, Id<Vertex>>,
    pub v2: Col<Self, Id<Vertex>>,
    pub flags: Col<Self, i64>,
    pub special: Col<Self, i64>,
    pub tag: Col<Self, i64>,
    pub front: Col<Self, Id<Sidedef>>,
    pub back: Col<Self, Id<Sidedef>>,
    // Derived: endpoints, and the sectors on each side.
    pub x1: Col<Self, f64>,
    pub y1: Col<Self, f64>,
    pub x2: Col<Self, f64>,
    pub y2: Col<Self, f64>,
    pub fsec: Col<Self, Id<Sector>>,
    pub bsec: Col<Self, Id<Sector>>,
}

// A piece of a linedef bounding one BSP leaf; the renderer draws segs.
#[derive(IntoQuery)]
pub struct Seg {
    #[primary_key]
    pub id: Key<Self>,
    pub v1: Col<Self, Id<Vertex>>,
    pub v2: Col<Self, Id<Vertex>>,
    pub line: Col<Self, Id<Linedef>>,
    pub side: Col<Self, usize>,
    pub offset: Col<Self, f64>,
    // Derived: endpoints, texture offsets, sectors and textures as seen from this seg's side,
    // line flags, and the fake-contrast light delta.
    pub x1: Col<Self, f64>,
    pub y1: Col<Self, f64>,
    pub x2: Col<Self, f64>,
    pub y2: Col<Self, f64>,
    pub xoff: Col<Self, f64>,
    pub yoff: Col<Self, f64>,
    pub fsec: Col<Self, Id<Sector>>,
    pub bsec: Col<Self, Id<Sector>>,
    pub upper: Col<Self, Id<Image>>,
    pub middle: Col<Self, Id<Image>>,
    pub lower: Col<Self, Id<Image>>,
    pub flags: Col<Self, i64>,
    pub light_delta: Col<Self, i64>,
}

// BSP tree: ids 0..nodes are split nodes, the rest are leaves (subsectors).
// Node-only and leaf-only attributes are partial (Set) columns.
#[derive(IntoQuery)]
pub struct Bsp {
    #[primary_key]
    pub id: Key<Self>,
    pub x: Set<Self, f64>,
    pub y: Set<Self, f64>,
    pub dx: Set<Self, f64>,
    pub dy: Set<Self, f64>,
    pub child0: Set<Self, Id<Bsp>>,
    pub child1: Set<Self, Id<Bsp>>,
    pub segs: Set<Self, Id<Seg>>,
    // Derived: a leaf's sector.
    pub sector: Set<Self, Id<Sector>>,
}

// Starting sector values; live floor/ceiling heights are tic state.
#[derive(IntoQuery)]
pub struct Sector {
    #[primary_key]
    pub id: Key<Self>,
    pub floor: Col<Self, f64>,
    pub ceil: Col<Self, f64>,
    pub floorpic: Col<Self, Id<Image>>,
    pub ceilpic: Col<Self, Id<Image>>,
    pub light: Col<Self, i64>,
    pub special: Col<Self, i64>,
    pub tag: Col<Self, i64>,
    // Derived: sectors sharing a two-sided line.
    pub neighbors: Set<Self, Id<Sector>>,
}

// A map THINGS entry: kind is the editor number.
#[derive(IntoQuery)]
pub struct Thing {
    #[primary_key]
    pub id: Key<Self>,
    pub x: Col<Self, f64>,
    pub y: Col<Self, f64>,
    pub angle: Col<Self, f64>,
    pub kind: Col<Self, i64>,
    pub flags: Col<Self, i64>,
}

// Any picture: wall texture, flat, sprite or HUD glyph. Its texels start at `base` in the
// texel table, column-major; left/top are the drawing offsets.
#[derive(IntoQuery)]
pub struct Image {
    #[primary_key]
    pub id: Key<Self>,
    pub base: Col<Self, usize>,
    pub w: Col<Self, usize>,
    pub h: Col<Self, usize>,
    pub left: Col<Self, i64>,
    pub top: Col<Self, i64>,
}

// Game-definition entities, loaded by db.rs from the tables written in info.rs.

// One animation/AI state: sprite frame to show, how long, what to do on entry, and the next state.
#[derive(IntoQuery)]
pub struct AnimState {
    #[primary_key]
    pub id: Key<Self>,
    pub sprite: Col<Self, usize>,
    pub frame: Col<Self, usize>,
    pub bright: Col<Self, bool>,
    pub tics: Col<Self, i32>,
    pub action: Col<Self, Action>,
    pub next: Col<Self, Id<AnimState>>,
}

// A kind of thing (Doom's mobjinfo): its states, size, health and flags.
#[derive(IntoQuery)]
pub struct ThingType {
    #[primary_key]
    pub id: Key<Self>,
    pub ednum: Col<Self, i64>,
    pub spawn: Col<Self, Id<AnimState>>,
    pub see: Col<Self, Id<AnimState>>,
    pub pain: Col<Self, Id<AnimState>>,
    pub melee: Col<Self, Id<AnimState>>,
    pub missile: Col<Self, Id<AnimState>>,
    pub death: Col<Self, Id<AnimState>>,
    pub speed: Col<Self, f64>,
    pub radius: Col<Self, f64>,
    pub height: Col<Self, f64>,
    pub health: Col<Self, i32>,
    pub painchance: Col<Self, i32>,
    pub damage: Col<Self, i32>,
    pub flags: Col<Self, u32>,
    // The item a monster drops when killed (NONE for most).
    pub drop: Col<Self, Id<ThingType>>,
    // What picking it up gives (items only).
    pub pickup: Set<Self, Id<Pickup>>,
}

// What touching an item gives the player.
#[derive(IntoQuery)]
pub struct Pickup {
    #[primary_key]
    pub id: Key<Self>,
    pub health: Col<Self, i32>,
    pub health_max: Col<Self, i32>,
    pub armor: Col<Self, i32>,
    pub armor_max: Col<Self, i32>,
    pub class_set: Col<Self, i32>,
    pub class_min: Col<Self, i32>,
    pub ammo_kind: Col<Self, Id<AmmoKind>>,
    pub ammo: Col<Self, i32>,
    pub weapon: Col<Self, Id<Weapon>>,
    pub weapon_bit: Col<Self, u8>,
    pub key: Col<Self, u8>,
    // Taken even when it changes nothing.
    pub always: Col<Self, bool>,
}

// A weapon slot: ammo use, its ready/attack/flash states, and bullet damage, range and spread.
#[derive(IntoQuery)]
pub struct Weapon {
    #[primary_key]
    pub id: Key<Self>,
    pub ammo: Col<Self, Id<AmmoKind>>,
    pub per_shot: Col<Self, i32>,
    pub ready: Col<Self, Id<AnimState>>,
    pub attack: Col<Self, Id<AnimState>>,
    pub flash: Col<Self, Id<AnimState>>,
    pub flash_tics: Col<Self, i32>,
    pub damage: Col<Self, i32>,
    pub damage_roll: Col<Self, i32>,
    pub range: Col<Self, f64>,
    pub spread_div: Col<Self, f64>,
}

// An ammo kind: carrying cap and starting amount.
#[derive(IntoQuery)]
pub struct AmmoKind {
    #[primary_key]
    pub id: Key<Self>,
    pub cap: Col<Self, i32>,
    pub start: Col<Self, i32>,
}

// The share of damage an armor class absorbs: num / den.
#[derive(IntoQuery)]
pub struct ArmorClass {
    #[primary_key]
    pub id: Key<Self>,
    pub num: Col<Self, i32>,
    pub den: Col<Self, i32>,
}

// A direction a chasing monster tries, in preference order: turn from its heading, step scale.
#[derive(IntoQuery)]
pub struct ChaseDir {
    #[primary_key]
    pub id: Key<Self>,
    pub turn: Col<Self, f64>,
    pub step: Col<Self, f64>,
    pub stay: Col<Self, bool>,
}

// A player move attempt, in preference order: which momentum axes it keeps.
#[derive(IntoQuery)]
pub struct Slide {
    #[primary_key]
    pub id: Key<Self>,
    pub keep_x: Col<Self, f64>,
    pub keep_y: Col<Self, f64>,
    pub stay: Col<Self, bool>,
}

// What a line special does: mover kind, speed, tagged sectors vs the sector behind, key needed, walk vs use.
#[derive(IntoQuery)]
pub struct LineSpecial {
    #[primary_key]
    pub id: Key<Self>,
    pub num: Col<Self, i64>,
    pub act: Col<Self, u8>,
    pub speed: Col<Self, f64>,
    pub tagged: Col<Self, bool>,
    pub key: Col<Self, u8>,
    pub walk: Col<Self, bool>,
    pub use_: Col<Self, bool>,
}

// What a special does to a sector's mover, keyed by (mover kind, direction, special act).
// Target heights are weighted sums of the lowest neighbouring ceiling, the floor, and the lowest floor.
#[derive(IntoQuery)]
pub struct Activation {
    #[primary_key]
    pub id: Key<Self>,
    pub key: Col<Self, usize>,
    pub kind: Col<Self, u8>,
    pub dir: Col<Self, i8>,
    pub init: Col<Self, bool>,
    pub plane: Col<Self, u8>,
    pub up_lc: Col<Self, f64>,
    pub up_floor: Col<Self, f64>,
    pub down_floor: Col<Self, f64>,
    pub down_lowest: Col<Self, f64>,
}

// Rule tables: a rule applies to rows with its key whose facts satisfy (facts & mask) == want;
// among those the lowest prio wins.

// Monster AI rule, keyed by the action of the state the monster just entered.
#[derive(IntoQuery)]
pub struct AiRule {
    #[primary_key]
    pub id: Key<Self>,
    pub key: Col<Self, usize>,
    pub prio: Col<Self, u8>,
    pub mask: Col<Self, u32>,
    pub want: Col<Self, u32>,
    // Goto label (G_*), wake (+1) / sleep (-1), reaction timer (-1 = keep).
    pub goto: Col<Self, u8>,
    pub awake: Col<Self, i8>,
    pub reaction: Col<Self, i32>,
    // Attack: kind (ATK_*), pellets, damage mult * (1..=modulo).
    pub attack: Col<Self, u8>,
    pub pellets: Col<Self, usize>,
    pub mult: Col<Self, i32>,
    pub modulo: Col<Self, i32>,
    // Other effects, as eff::* bits.
    pub effects: Col<Self, u32>,
}

// Weapon rule, keyed by the action of the weapon state being entered.
#[derive(IntoQuery)]
pub struct WeaponRule {
    #[primary_key]
    pub id: Key<Self>,
    pub key: Col<Self, usize>,
    pub prio: Col<Self, u8>,
    pub mask: Col<Self, u32>,
    pub want: Col<Self, u32>,
    // Jump to the attack state instead, or fire this many pellets.
    pub redirect: Col<Self, bool>,
    pub pellets: Col<Self, usize>,
}

// Door/lift rule, keyed by (mover kind, direction): new kind, direction, and wait timer
// (wait < 0 keeps the timer, minus one if wait_dec).
#[derive(IntoQuery)]
pub struct MoverRule {
    #[primary_key]
    pub id: Key<Self>,
    pub key: Col<Self, usize>,
    pub prio: Col<Self, u8>,
    pub mask: Col<Self, u32>,
    pub want: Col<Self, u32>,
    pub kind: Col<Self, u8>,
    pub dir: Col<Self, i8>,
    pub wait: Col<Self, i32>,
    pub wait_dec: Col<Self, bool>,
}

// Status-bar face rule: which face, and whether it varies with pain and look direction.
#[derive(IntoQuery)]
pub struct FaceRule {
    #[primary_key]
    pub id: Key<Self>,
    pub key: Col<Self, usize>,
    pub prio: Col<Self, u8>,
    pub mask: Col<Self, u32>,
    pub want: Col<Self, u32>,
    pub kind: Col<Self, u8>,
    pub by_pain: Col<Self, bool>,
    pub by_look: Col<Self, bool>,
}

// AI rule effects.
pub mod eff {
    pub const FACE: u32 = 1;
    pub const REACTION_DEC: u32 = 1 << 1;
    pub const GO: u32 = 1 << 2;
    pub const CLEAR_SOLID: u32 = 1 << 3;
    pub const EXPLODE: u32 = 1 << 4;
    pub const CLEAR_SHOOTABLE: u32 = 1 << 5;
    pub const RESET: u32 = 1 << 6;
    pub const TO_CLIP: u32 = 1 << 7;
    pub const FACE_MOVE: u32 = 1 << 8;
    pub const STOP: u32 = 1 << 9;
}

// A fixed status bar image: x, y, depth (lower draws on top).
#[derive(IntoQuery)]
pub struct HudPatch {
    #[primary_key]
    pub id: Key<Self>,
    pub image: Col<Self, Id<Image>>,
    pub x: Col<Self, f64>,
    pub y: Col<Self, f64>,
    pub depth: Col<Self, f32>,
}

// Where one stat is drawn (ids are the STAT_* slots): right edge, top, font, max digits, percent sign.
#[derive(IntoQuery)]
pub struct NumberSlot {
    #[primary_key]
    pub id: Key<Self>,
    pub x: Col<Self, f64>,
    pub y: Col<Self, f64>,
    pub font: Col<Self, usize>,
    pub digits: Col<Self, usize>,
    pub percent: Col<Self, bool>,
}

// Arms panel entry: weapon slot, position, digit shown.
#[derive(IntoQuery)]
pub struct HudArm {
    #[primary_key]
    pub id: Key<Self>,
    pub weapon: Col<Self, usize>,
    pub x: Col<Self, f64>,
    pub y: Col<Self, f64>,
    pub digit: Col<Self, usize>,
}

// Key card slot (ids are key bits): position and image.
#[derive(IntoQuery)]
pub struct HudKey {
    #[primary_key]
    pub id: Key<Self>,
    pub x: Col<Self, f64>,
    pub y: Col<Self, f64>,
    pub image: Col<Self, Id<Image>>,
}

// Doom's tuning numbers, as one row (SQLDoom's doom_constants). Read with `one(&k.field)`.
#[derive(IntoQuery)]
pub struct Konst {
    #[primary_key]
    pub id: Key<Self>,
    pub friction: Col<Self, f64>,
    pub walk_thrust: Col<Self, f64>,
    pub run_thrust: Col<Self, f64>,
    pub strafe_scale: Col<Self, f64>,
    pub turn_walk: Col<Self, f64>,
    pub turn_run: Col<Self, f64>,
    pub gravity: Col<Self, f64>,
    pub player_radius: Col<Self, f64>,
    pub player_height: Col<Self, f64>,
    pub view_height: Col<Self, f64>,
    pub dead_view_height: Col<Self, f64>,
    pub max_step: Col<Self, f64>,
    pub use_range: Col<Self, f64>,
    pub door_gap: Col<Self, f64>,
    pub melee_range: Col<Self, f64>,
    pub close_range: Col<Self, f64>,
    pub missile_near: Col<Self, f64>,
    pub missile_no_melee: Col<Self, f64>,
    pub missile_far: Col<Self, f64>,
    pub sight_z: Col<Self, f64>,
    pub shot_z: Col<Self, f64>,
    pub attack_range: Col<Self, f64>,
    pub hitscan_spread_div: Col<Self, f64>,
    pub puff_back: Col<Self, f64>,
    pub blast_radius: Col<Self, f64>,
    pub missile_reach: Col<Self, f64>,
    pub missile_z_reach: Col<Self, f64>,
    pub pickup_z_reach: Col<Self, f64>,
    pub bonus_add: Col<Self, i32>,
    pub damage_cap: Col<Self, i32>,
    pub floor_damage_period: Col<Self, u64>,
    pub reaction: Col<Self, i32>,
    pub start_health: Col<Self, i32>,
    pub start_owned: Col<Self, u8>,
    pub start_weapon: Col<Self, Id<Weapon>>,
}

// The value of a one-row column.
pub fn one<E: 'static, T: Copy>(c: &Col<E, T>) -> T {
    c.v[0]
}

// Live game state. Columns sit behind Rc so a tic shares every column it doesn't change
// with the tic before.
pub type C<E, T> = Rc<Col<E, T>>;

// Materialize a query as a shared column.
pub fn rc<E: 'static, T: Copy, Q: Drive<D = Id<E>, R = T>>(q: Q) -> C<E, T> {
    Rc::new(q.collect())
}

// The player (one row).
#[derive(IntoQuery, Clone)]
pub struct Player {
    #[primary_key]
    pub id: Key<Self>,
    pub x: C<Self, f64>,
    pub y: C<Self, f64>,
    pub z: C<Self, f64>,
    pub ang: C<Self, f64>,
    pub momx: C<Self, f64>,
    pub momy: C<Self, f64>,
    pub momz: C<Self, f64>,
    pub sector: C<Self, Id<Sector>>,
    // Use key held last tic, level exit reached, dead.
    pub usedown: C<Self, bool>,
    pub exited: C<Self, bool>,
    pub dead: C<Self, bool>,
    pub health: C<Self, i32>,
    pub armor: C<Self, i32>,
    pub armor_class: C<Self, Id<ArmorClass>>,
    // Owned weapons and keys, one bit each.
    pub owned: C<Self, u8>,
    pub keys: C<Self, u8>,
    // Current weapon, and one asked for but not switched to yet (NONE if none).
    pub weapon: C<Self, Id<Weapon>>,
    pub wanted: C<Self, Id<Weapon>>,
    // Weapon sprite state and its timer; muzzle flash state (0 = none) and its timer.
    pub wstate: C<Self, Id<AnimState>>,
    pub wtics: C<Self, i32>,
    pub flash: C<Self, Id<AnimState>>,
    pub flash_tics: C<Self, i32>,
    // Screen tint timers: red after damage, gold after a pickup.
    pub damagecount: C<Self, i32>,
    pub bonuscount: C<Self, i32>,
}

// Map objects: monsters, items, decorations, projectiles, puffs. Ids are renumbered 0..n every
// tic; `uid` is the object's lasting identity (it seeds the random rolls).
#[derive(IntoQuery, Clone)]
pub struct Mobj {
    #[primary_key]
    pub id: Key<Self>,
    pub uid: C<Self, usize>,
    pub kind: C<Self, Id<ThingType>>,
    pub x: C<Self, f64>,
    pub y: C<Self, f64>,
    pub z: C<Self, f64>,
    pub ang: C<Self, f64>,
    pub momx: C<Self, f64>,
    pub momy: C<Self, f64>,
    pub state: C<Self, Id<AnimState>>,
    pub tics: C<Self, i32>,
    pub health: C<Self, i32>,
    pub flags: C<Self, u32>,
    pub awake: C<Self, bool>,
    pub reaction: C<Self, i32>,
    pub sector: C<Self, Id<Sector>>,
    // uid of whoever fired this projectile (NONE otherwise).
    pub source: C<Self, usize>,
}

// Live per-sector state: plane heights, the door/lift moving them, and whether the sector
// has heard the player fire.
#[derive(Clone)]
pub struct SectorState {
    pub floor: C<Sector, f64>,
    pub ceil: C<Sector, f64>,
    // Mover: MV_* kind, plane (0 ceiling, 1 floor), direction (1 up, -1 down, 0 waiting),
    // speed, target heights up/down, tics left to wait.
    pub kind: C<Sector, u8>,
    pub plane: C<Sector, u8>,
    pub dir: C<Sector, i8>,
    pub speed: C<Sector, f64>,
    pub up: C<Sector, f64>,
    pub down: C<Sector, f64>,
    pub wait: C<Sector, i32>,
    pub heard: C<Sector, bool>,
}
