use crate::info::{self, eff_bits, G_DEATH, G_MELEE, G_MISSILE, G_PAIN, G_SEE, G_SPAWN};
use crate::rules::{by_key, decisions};
use crate::schema::*;
use crate::wad::{Game, NONE};
use prela::engine::*;
use prela::loader::{Col, Key};
use std::collections::HashMap;

// The game-rule entities from info.rs, plus the lookup indexes over them.
pub struct Tables {
    pub state: AnimState,
    pub ttype: ThingType,
    pub pickup: Pickup,
    pub weapon: Weapon,
    pub ammo: AmmoKind,
    pub armor: ArmorClass,
    pub chase: ChaseDir,
    pub slide: Slide,
    pub special: LineSpecial,
    pub activation: Activation,
    pub ai_rule: AiRule,
    pub weapon_rule: WeaponRule,
    pub mover_rule: MoverRule,
    pub face_rule: FaceRule,
    pub hud_patch: HudPatch,
    pub hud_number: NumberSlot,
    pub hud_arm: HudArm,
    pub hud_key: HudKey,
    pub k: Konst,
    // Map editor number -> thing type.
    pub ednum_type: HashIdx<i64, Id<ThingType>>,
    // (thing type, goto label) -> state, so rules can say "go to SEE".
    pub goto: HashIdx<(Id<ThingType>, u8), Id<AnimState>>,
    // frame_key(sprite, frame, rotation) -> image and whether to mirror it.
    pub sprite_frames: HashIdx<u64, (Id<Image>, bool)>,
    // Line special number -> special.
    pub special_num: HashIdx<i64, Id<LineSpecial>>,
    // act_key(kind, dir, act) -> activation.
    pub activation_key: HashIdx<usize, Id<Activation>>,
    // Rule key -> rules, per rule table.
    pub ai_by_key: MultiRel<usize, Id<AiRule>>,
    pub mover_by_key: MultiRel<usize, Id<MoverRule>>,
    pub face_by_key: MultiRel<usize, Id<FaceRule>>,
    // (weapon state action, weapon facts) -> the weapon rule that applies.
    pub weapon_choice: HashIdx<(usize, usize), Id<WeaponRule>>,
    // Sector special -> floor damage.
    pub sector_damage: HashIdx<i64, i32>,
    // (font, glyph) -> image; (face kind, pain, look) -> image.
    pub hud_fonts: HashIdx<(usize, usize), Id<Image>>,
    pub faces: HashIdx<(u8, usize, usize), Id<Image>>,
    pub puff: Id<ThingType>,
    pub blood: Id<ThingType>,
    pub ball: Id<ThingType>,
}

// Packs (sprite, frame, rotation) into one lookup key for sprite images.
pub fn frame_key(sprite: usize, frame: usize, rot: usize) -> u64 {
    (sprite * 1000 + frame * 10 + rot) as u64
}

fn key<E>(n: usize) -> Key<E> {
    Universe::new(n)
}

// One column from a row list.
fn col<E, R, T: Copy>(rows: &[R], f: impl Fn(&R) -> T) -> Col<E, T> {
    VecRel::new(rows.iter().map(f).collect())
}

// A lookup index from (key, value) pairs.
fn index<K: Copy + Eq + std::hash::Hash, V: Copy>(pairs: Vec<(K, V)>) -> HashIdx<K, V> {
    VecRel::<usize, (K, V)>::new(pairs).key_by(|(k, _)| k).map(|(_, v)| v).collect()
}

// Turns the Rust-literal tables in info.rs into entities, resolving names to ids.
fn tables(sprite_names: &HashMap<String, usize>, patch_names: &HashMap<String, usize>) -> Tables {
    // Animation states, with names and sprites replaced by ids.
    let defs = info::states();
    let ids: HashMap<String, usize> = defs.iter().enumerate().map(|(i, d)| (d.name.clone(), i)).collect();
    let sid = |n: &str| Id::<AnimState>::new(*ids.get(n).unwrap_or_else(|| panic!("unknown state {n}")));
    // Sprite id = index in the sorted list of 4-letter sprite names.
    let mut sprites: Vec<&str> = defs.iter().map(|d| d.sprite).collect();
    sprites.sort();
    sprites.dedup();
    let sprite_id = |n: &str| sprites.iter().position(|&s| s == n).unwrap();
    let state = AnimState {
        id: key(defs.len()),
        sprite: col(&defs, |d| sprite_id(d.sprite)),
        frame: col(&defs, |d| d.frame as usize),
        bright: col(&defs, |d| d.bright),
        tics: col(&defs, |d| d.tics),
        action: col(&defs, |d| d.action),
        next: col(&defs, |d| sid(&d.next)),
    };
    // Thing types, with state names, drop items and pickups resolved to ids.
    let mdefs = info::mobjs();
    let by_ednum: HashMap<i64, usize> = mdefs.iter().enumerate().rev().map(|(i, m)| (m.doomednum, i)).collect();
    let pdefs: Vec<(usize, info::PickupDef)> = mdefs.iter().enumerate().filter_map(|(i, m)| m.pickup.map(|p| (i, p))).collect();
    let ttype = ThingType {
        id: key(mdefs.len()),
        ednum: col(&mdefs, |m| m.doomednum),
        spawn: col(&mdefs, |m| sid(&m.spawn)),
        see: col(&mdefs, |m| sid(&m.see)),
        pain: col(&mdefs, |m| sid(&m.pain)),
        melee: col(&mdefs, |m| sid(&m.melee)),
        missile: col(&mdefs, |m| sid(&m.missile)),
        death: col(&mdefs, |m| sid(&m.death)),
        speed: col(&mdefs, |m| m.speed),
        radius: col(&mdefs, |m| m.radius),
        height: col(&mdefs, |m| m.height),
        health: col(&mdefs, |m| m.health),
        painchance: col(&mdefs, |m| m.painchance),
        damage: col(&mdefs, |m| m.damage),
        flags: col(&mdefs, |m| m.flags),
        drop: col(&mdefs, |m| Id::new(by_ednum.get(&m.drop).copied().unwrap_or(NONE))),
        pickup: MultiRel::from_pairs(mdefs.len(), pdefs.iter().enumerate().map(|(p, &(t, _))| (t, Id::new(p)))),
    };
    let pickup = Pickup {
        id: key(pdefs.len()),
        health: col(&pdefs, |p| p.1.health),
        health_max: col(&pdefs, |p| p.1.health_max),
        armor: col(&pdefs, |p| p.1.armor),
        armor_max: col(&pdefs, |p| p.1.armor_max),
        class_set: col(&pdefs, |p| p.1.class_set),
        class_min: col(&pdefs, |p| p.1.class_min),
        ammo_kind: col(&pdefs, |p| Id::new(p.1.ammo_kind)),
        ammo: col(&pdefs, |p| p.1.ammo),
        weapon: col(&pdefs, |p| Id::new(p.1.weapon)),
        weapon_bit: col(&pdefs, |p| p.1.weapon_bit),
        key: col(&pdefs, |p| p.1.key),
        always: col(&pdefs, |p| p.1.always),
    };
    // Map editor number -> thing type, for spawning map things.
    let ednum_type = (&ttype.ednum).filt(|e: i64| e > 0).inv().collect();
    let kind = |n: &str| Id::new(mdefs.iter().position(|m| m.name == n).unwrap());
    // (thing type, goto label) -> state id, for each label the type has.
    let goto = (&ttype.see)
        .map(|s| (G_SEE, s))
        .union((&ttype.spawn).map(|s| (G_SPAWN, s)))
        .union((&ttype.melee).map(|s| (G_MELEE, s)))
        .union((&ttype.missile).map(|s| (G_MISSILE, s)))
        .union((&ttype.death).map(|s| (G_DEATH, s)))
        .union((&ttype.pain).map(|s| (G_PAIN, s)))
        .filt(|(_, s): (u8, Id<AnimState>)| s.0 != 0)
        .and(Same::<Id<ThingType>>::new())
        .key_by(|((g, _), t): ((u8, Id<AnimState>), Id<ThingType>)| (t, g))
        .map(|((_, s), _)| s)
        .collect();
    // Sprite lump names like TROOA2A8 -> (sprite, frame, rotation) -> image id; the second half is the mirrored rotation.
    let mut frames = Vec::new();
    for (name, &gid) in sprite_names {
        let b = name.as_bytes();
        if b.len() < 6 {
            continue;
        }
        let Some(sp) = sprites.iter().position(|&s| s.as_bytes() == &b[..4]) else { continue };
        frames.push((frame_key(sp, (b[4] - b'A') as usize, (b[5] - b'0') as usize), (Id::new(gid), false)));
        if b.len() >= 8 {
            frames.push((frame_key(sp, (b[6] - b'A') as usize, (b[7] - b'0') as usize), (Id::new(gid), true)));
        }
    }
    let wdefs = info::weapons();
    let weapon = Weapon {
        id: key(wdefs.len()),
        ammo: col(&wdefs, |w| Id::new(w.ammo)),
        per_shot: col(&wdefs, |w| w.per_shot),
        ready: col(&wdefs, |w| sid(w.ready)),
        attack: col(&wdefs, |w| sid(w.attack)),
        flash: col(&wdefs, |w| sid(w.flash)),
        flash_tics: VecRel::new(vec![]),
        damage: col(&wdefs, |w| w.damage),
        damage_roll: col(&wdefs, |w| w.damage_roll),
        range: col(&wdefs, |w| w.range),
        spread_div: col(&wdefs, |w| w.spread_div),
    };
    // Each weapon's flash duration, from its flash state.
    let weapon = Weapon { flash_tics: (&weapon.flash).select(&state.tics).collect(), ..weapon };
    let adefs = info::ammo_defs();
    let ammo = AmmoKind { id: key(adefs.len()), cap: col(&adefs, |a| a.cap), start: col(&adefs, |a| a.start) };
    let cdefs = info::armor_classes();
    let armor = ArmorClass { id: key(cdefs.len()), num: col(&cdefs, |c| c.num), den: col(&cdefs, |c| c.den) };
    let ddefs = info::chase_dirs();
    let chase = ChaseDir { id: key(ddefs.len()), turn: col(&ddefs, |d| d.turn), step: col(&ddefs, |d| d.step), stay: col(&ddefs, |d| d.stay) };
    let ldefs = info::slides();
    let slide = Slide { id: key(ldefs.len()), keep_x: col(&ldefs, |s| s.keep_x), keep_y: col(&ldefs, |s| s.keep_y), stay: col(&ldefs, |s| s.stay) };
    let spdefs = info::specials();
    let special = LineSpecial {
        id: key(spdefs.len()),
        num: col(&spdefs, |s| s.0),
        act: col(&spdefs, |s| s.1.act),
        speed: col(&spdefs, |s| s.1.speed),
        tagged: col(&spdefs, |s| s.1.tagged),
        key: col(&spdefs, |s| s.1.key),
        walk: col(&spdefs, |s| s.1.walk),
        use_: col(&spdefs, |s| s.1.use_),
    };
    let special_num = (&special.num).inv().collect();
    let acdefs = info::activations();
    let activation = Activation {
        id: key(acdefs.len()),
        key: col(&acdefs, |a| a.0),
        kind: col(&acdefs, |a| a.1.kind),
        dir: col(&acdefs, |a| a.1.dir),
        init: col(&acdefs, |a| a.1.init),
        plane: col(&acdefs, |a| a.1.plane),
        up_lc: col(&acdefs, |a| a.1.up_lc),
        up_floor: col(&acdefs, |a| a.1.up_floor),
        down_floor: col(&acdefs, |a| a.1.down_floor),
        down_lowest: col(&acdefs, |a| a.1.down_lowest),
    };
    let activation_key = (&activation.key).inv().collect();
    // Rule tables.
    let rdefs = info::ai_rules();
    let ai_rule = AiRule {
        id: key(rdefs.len()),
        key: col(&rdefs, |r| r.0 as usize),
        prio: col(&rdefs, |r| r.1.prio),
        mask: col(&rdefs, |r| r.1.mask),
        want: col(&rdefs, |r| r.1.want),
        goto: col(&rdefs, |r| r.1.out.goto),
        awake: col(&rdefs, |r| r.1.out.awake),
        reaction: col(&rdefs, |r| r.1.out.reaction),
        attack: col(&rdefs, |r| r.1.out.attack.kind),
        pellets: col(&rdefs, |r| r.1.out.attack.pellets),
        mult: col(&rdefs, |r| r.1.out.attack.mult),
        modulo: col(&rdefs, |r| r.1.out.attack.modulo),
        effects: col(&rdefs, |r| eff_bits(r.1.out)),
    };
    let wrdefs = info::weapon_rules();
    let weapon_rule = WeaponRule {
        id: key(wrdefs.len()),
        key: col(&wrdefs, |r| r.0 as usize),
        prio: col(&wrdefs, |r| r.1.prio),
        mask: col(&wrdefs, |r| r.1.mask),
        want: col(&wrdefs, |r| r.1.want),
        redirect: col(&wrdefs, |r| r.1.out.redirect),
        pellets: col(&wrdefs, |r| r.1.out.pellets),
    };
    let mrdefs = info::mover_rules();
    let mover_rule = MoverRule {
        id: key(mrdefs.len()),
        key: col(&mrdefs, |r| r.0),
        prio: col(&mrdefs, |r| r.1.prio),
        mask: col(&mrdefs, |r| r.1.mask),
        want: col(&mrdefs, |r| r.1.want),
        kind: col(&mrdefs, |r| r.1.out.kind),
        dir: col(&mrdefs, |r| r.1.out.dir),
        wait: col(&mrdefs, |r| r.1.out.wait),
        wait_dec: col(&mrdefs, |r| r.1.out.wait_dec),
    };
    let frdefs = info::face_rules();
    let face_rule = FaceRule {
        id: key(frdefs.len()),
        key: col(&frdefs, |_| 0),
        prio: col(&frdefs, |r| r.prio),
        mask: col(&frdefs, |r| r.mask),
        want: col(&frdefs, |r| r.want),
        kind: col(&frdefs, |r| r.out.kind),
        by_pain: col(&frdefs, |r| r.out.by_pain),
        by_look: col(&frdefs, |r| r.out.by_look),
    };
    let ai_by_key = by_key(&ai_rule.key);
    let weapon_choice = decisions(&weapon_rule, &by_key(&weapon_rule.key), 3);
    // Status bar tables, with lump names resolved to image ids.
    let lump = |n: &str| Id::<Image>::new(patch_names.get(n).copied().unwrap_or(NONE));
    let hp = info::hud_patches();
    let hud_patch = HudPatch { id: key(hp.len()), image: col(&hp, |p| lump(p.0)), x: col(&hp, |p| p.1), y: col(&hp, |p| p.2), depth: col(&hp, |p| p.3) };
    let hn = info::hud_numbers();
    let hud_number = NumberSlot {
        id: key(hn.len()),
        x: col(&hn, |n| n.x),
        y: col(&hn, |n| n.y),
        font: col(&hn, |n| n.font),
        digits: col(&hn, |n| n.digits),
        percent: col(&hn, |n| n.percent),
    };
    let ha = info::hud_arms();
    let hud_arm = HudArm { id: key(ha.len()), weapon: col(&ha, |a| a.0), x: col(&ha, |a| a.1), y: col(&ha, |a| a.2), digit: col(&ha, |a| a.3) };
    let hk = info::hud_keys();
    let hud_key = HudKey { id: key(hk.len()), x: col(&hk, |k| k.0), y: col(&hk, |k| k.1), image: col(&hk, |k| lump(k.2)) };
    let c = [info::consts()];
    let k = Konst {
        id: key(1),
        friction: col(&c, |c| c.friction),
        walk_thrust: col(&c, |c| c.walk_thrust),
        run_thrust: col(&c, |c| c.run_thrust),
        strafe_scale: col(&c, |c| c.strafe_scale),
        turn_walk: col(&c, |c| c.turn_walk),
        turn_run: col(&c, |c| c.turn_run),
        gravity: col(&c, |c| c.gravity),
        player_radius: col(&c, |c| c.player_radius),
        player_height: col(&c, |c| c.player_height),
        view_height: col(&c, |c| c.view_height),
        dead_view_height: col(&c, |c| c.dead_view_height),
        max_step: col(&c, |c| c.max_step),
        use_range: col(&c, |c| c.use_range),
        door_gap: col(&c, |c| c.door_gap),
        melee_range: col(&c, |c| c.melee_range),
        close_range: col(&c, |c| c.close_range),
        missile_near: col(&c, |c| c.missile_near),
        missile_no_melee: col(&c, |c| c.missile_no_melee),
        missile_far: col(&c, |c| c.missile_far),
        sight_z: col(&c, |c| c.sight_z),
        shot_z: col(&c, |c| c.shot_z),
        attack_range: col(&c, |c| c.attack_range),
        hitscan_spread_div: col(&c, |c| c.hitscan_spread_div),
        puff_back: col(&c, |c| c.puff_back),
        blast_radius: col(&c, |c| c.blast_radius),
        missile_reach: col(&c, |c| c.missile_reach),
        missile_z_reach: col(&c, |c| c.missile_z_reach),
        pickup_z_reach: col(&c, |c| c.pickup_z_reach),
        bonus_add: col(&c, |c| c.bonus_add),
        damage_cap: col(&c, |c| c.damage_cap),
        floor_damage_period: col(&c, |c| c.floor_damage_period),
        reaction: col(&c, |c| c.reaction),
        start_health: col(&c, |c| c.start_health),
        start_owned: col(&c, |c| c.start_owned),
        start_weapon: col(&c, |c| Id::new(c.start_weapon)),
    };
    Tables {
        mover_by_key: by_key(&mover_rule.key),
        face_by_key: by_key(&face_rule.key),
        sprite_frames: index(frames),
        hud_fonts: index(info::hud_fonts().into_iter().map(|(k, n)| (k, lump(&n))).collect()),
        faces: index(info::faces().into_iter().map(|(k, n)| (k, lump(&n))).collect()),
        sector_damage: index(info::sector_damage()),
        puff: kind("PUFF"),
        blood: kind("BLOOD"),
        ball: kind("TBALL"),
        state,
        ttype,
        pickup,
        weapon,
        ammo,
        armor,
        chase,
        slide,
        special,
        activation,
        ai_rule,
        weapon_rule,
        mover_rule,
        face_rule,
        hud_patch,
        hud_number,
        hud_arm,
        hud_key,
        k,
        ednum_type,
        goto,
        special_num,
        activation_key,
        ai_by_key,
        weapon_choice,
    }
}

// Everything static about the level, its graphics and the game rules, as relations.
pub struct Db {
    pub t: Tables,
    pub linedef: Linedef,
    pub seg: Seg,
    pub bsp: Bsp,
    pub bsp_root: Id<Bsp>,
    pub sector: Sector,
    pub tag_sectors: HashIdx<i64, Id<Sector>>,
    pub thing: Thing,
    pub image: Image,
    pub texels: VecRel<usize, u16>,
    pub colormap: VecRel<usize, u8>,
    pub palette: VecRel<usize, u32>,
    pub sky: Id<Image>,
    pub sky_flat: Id<Image>,
}

impl Db {
    // Builds the database from the parsed WAD, filling in the derived columns.
    pub fn new(g: Game) -> Db {
        let t = tables(&g.sprite_names, &g.patch_names);
        // The root is the last split node.
        let bsp_root = Id::new((&g.bsp.x).fold_flat(0, |n: usize, _| n + 1) - 1);
        let linedef = derive_linedef(g.linedef, &g.vertex, &g.sidedef);
        let seg = derive_seg(g.seg, &g.vertex, &linedef, &g.sidedef);
        // A leaf's sector: the front sector of its first seg.
        let leaf_sector = (&g.bsp.segs).fold(Id::new(NONE), |a: Id<Seg>, s: Id<Seg>| a.min(s)).select(&seg.fsec).collect();
        let bsp = Bsp { sector: leaf_sector, ..g.bsp };
        // Sectors sharing a two-sided line, both ways.
        let neighbors = (&linedef.fsec)
            .and(&linedef.bsec)
            .filt(|(_, b): (Id<Sector>, Id<Sector>)| b.0 != NONE)
            .flat_map(|(f, b): (Id<Sector>, Id<Sector>)| [(f, b), (b, f)])
            .key_by(|(a, _)| a)
            .map(|(_, b)| b)
            .collect();
        let sector = Sector { neighbors, ..g.sector };
        // Tag -> sectors with that tag, for switches that move tagged sectors.
        let tag_sectors = (&sector.tag).inv().collect();
        Db {
            t,
            linedef,
            seg,
            bsp,
            bsp_root,
            sector,
            tag_sectors,
            thing: g.thing,
            image: g.image,
            texels: g.texels,
            colormap: g.colormap,
            palette: g.palette,
            sky: g.sky,
            sky_flat: g.sky_flat,
        }
    }
}

// Missing ids become NONE so the column stays total.
fn or_none<E: 'static>(o: Option<Id<E>>) -> Id<E> {
    o.unwrap_or(Id::new(NONE))
}

// Linedef endpoints and the sector on each side.
fn derive_linedef(l: Linedef, v: &Vertex, sd: &Sidedef) -> Linedef {
    let x1 = (&l.v1).select(&v.x).collect();
    let y1 = (&l.v1).select(&v.y).collect();
    let x2 = (&l.v2).select(&v.x).collect();
    let y2 = (&l.v2).select(&v.y).collect();
    let fsec = (&l.front).select(&sd.sector).collect();
    let bsec = (&l).and((&l.back).select(&sd.sector).opt()).map(|(_, s)| or_none(s)).collect();
    Linedef { x1, y1, x2, y2, fsec, bsec, ..l }
}

// Seg endpoints, plus what its line and sidedefs say as seen from the seg's side.
fn derive_seg(s: Seg, v: &Vertex, l: &Linedef, sd: &Sidedef) -> Seg {
    // The sidedef facing this seg (side 0 = the line's front) and the one behind it.
    let sides = (&s.side).and((&s.line).select(&l.front)).and((&s.line).select(&l.back));
    let front: VecRel<Id<Seg>, Id<Sidedef>> = (&sides).map(|((side, f), b): ((usize, Id<Sidedef>), Id<Sidedef>)| if side == 0 { f } else { b }).collect();
    let back: VecRel<Id<Seg>, Id<Sidedef>> = (&sides).map(|((side, f), b): ((usize, Id<Sidedef>), Id<Sidedef>)| if side == 0 { b } else { f }).collect();
    let x1: VecRel<Id<Seg>, f64> = (&s.v1).select(&v.x).collect();
    let y1: VecRel<Id<Seg>, f64> = (&s.v1).select(&v.y).collect();
    let x2: VecRel<Id<Seg>, f64> = (&s.v2).select(&v.x).collect();
    let y2: VecRel<Id<Seg>, f64> = (&s.v2).select(&v.y).collect();
    // Doom's fake contrast: horizontal walls darker, vertical walls lighter.
    let light_delta = (&x1)
        .and(&y1)
        .and(&x2)
        .and(&y2)
        .map(|(((x1, y1), x2), y2): (((f64, f64), f64), f64)| if y1 == y2 { -1 } else if x1 == x2 { 1 } else { 0 })
        .collect();
    Seg {
        xoff: (&s.offset).and((&front).select(&sd.xoff)).map(|(o, x): (f64, f64)| o + x).collect(),
        yoff: (&front).select(&sd.yoff).collect(),
        fsec: (&front).select(&sd.sector).collect(),
        bsec: (&s).and((&back).select(&sd.sector).opt()).map(|(_, b)| or_none(b)).collect(),
        upper: (&front).select(&sd.upper).collect(),
        middle: (&front).select(&sd.middle).collect(),
        lower: (&front).select(&sd.lower).collect(),
        flags: (&s.line).select(&l.flags).collect(),
        x1,
        y1,
        x2,
        y2,
        light_delta,
        ..s
    }
}
