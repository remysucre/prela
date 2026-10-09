# Doom in Prela — port plan

Goal: play E1M1 where every frame and every game tic is a Prela query. Rust only
parses the WAD, runs the 35 Hz loop, reads the keyboard, and blits the framebuffer.
Reference: CedarDB's SQLDoom (https://cedardb.com/blog/sqldoom/).

## Ground rules

Keep closures as simple as possible: scalar math on their arguments. Anything beyond
simple math gets raised with Paul before it goes in. Joins, ordering, grouping and
lookups go through Prela operators.

## Layout

- `rust/doom/` — new workspace member crate `prela-doom`, depends on `prela`.
  Window/input via `minifb` (keeps `prela` itself dependency-free).
- `src/wad.rs` — WAD parser → column relations (the ETL step; SQLDoom does the same
  outside the DB).
- `src/schema.rs` — `#[derive(IntoQuery)]` structs: Vertex, Linedef, Sidedef, Seg,
  Subsector, Node, Sector, Thing, Texture/Patch texels, Flat texels, Palette,
  Colormap, Blockmap, Reject.
- `src/render.rs` — the frame query.
- `src/tic.rs` — the game-tic query.
- `src/main.rs` — loop + display.
- WAD: Freedoom Phase 1 (BSD-licensed) by default; shareware `doom1.wad` also works.
  Neither is committed — downloaded by a script into a gitignored `data/`.

## Renderer (one query per frame, 320×200)

Mirrors SQLDoom's pipeline; each stage is a relation feeding the next.

1. **BSP order without recursion at frame time.** Precompute at load:
   `subsector → (ancestor node, depth, child side)` as a `MultiRel`. Per frame:
   `node → near_side` (which side of the partition the player is on) and
   `node → bbox_visible` (frustum test). Then
   `ancestors.select(...)` + `group_by(subsector).fold` builds a u64 order key
   (bit `63-depth` = took far child) and an all-visible flag. Sorting by the key
   gives front-to-back order — no traversal state.
2. **Segs → columns.** Visible subsectors → segs (`select`), project endpoints,
   backface-cull, `flat_map` over `x0..x1` emitting
   `(column, (order_key, depth, top, bottom, seg))` panels (upper/lower/mid).
3. **Occlusion clip.** Vanilla's mutable `ceilingclip/floorclip` arrays become a
   `window` per column ordered by `order_key`: running max of top-clip / min of
   bottom-clip over preceding panels gives each panel's visible span. (SQLDoom uses
   `ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING` for the same thing.)
4. **Wall pixels.** Visible spans `flat_map` to rows; texel lookup is a join
   `(texture, u, v) → palette index`; light via `colormap` join.
5. **Floors/ceilings.** The open band between a panel's clip and the previous one
   belongs to that panel's sector plane; per pixel compute world (x, y) from plane
   height and row distance, join into flat texels.
6. **Sprites.** Things in visible subsectors → projected sprite rectangles →
   texels, transparent texels dropped.
7. **Composite.** `union` of all candidate pixels, each packed as
   `depth << 8 | color`; `dense_fold` over pixel ids `0..64000` with `min` picks the
   visible one (SQLDoom's `MIN(key) GROUP BY pixel`). Palette join → RGB.

## Game tic (one query per tic)

State is a set of relations; each tic computes `state' = tic(state, input)` and
Rust swaps it in. No `UPDATE` needed — every tic is a consistent snapshot.

- Player: pose, momentum, health, armor, ammo, weapon, weapon frame.
- Mobjs: type, pose, state id, tics left, health, target.
- Sectors: floor/ceiling height (doors, lifts), special.
- Static tables: mobj info, state machine (`state → next, duration, action`),
  weapon defs/frames — all rows, like SQLDoom.

Sub-queries in order: input → movement + collision (blockmap cell → linedefs,
segment-intersection filter, `fold` to slide) → use (doors) → pickups → weapon /
hitscan (ray vs. linedefs and mobjs, `fold` min distance) → monster AI (REJECT
table + LOS check, state-machine join) → damage/deaths → sector movers.

## Milestones (check in after each)

- **M0** crate skeleton, WAD → relations, sanity counts vs. known E1M1 numbers.
- **M1** one static frame from the start position, flat-colored walls, written to
  PNG/PPM. Validate visually.
- **M2** textured + lit walls.
- **M3** floors/ceilings.
- **M4** interactive: window, keyboard, movement tic (no collision). First playable
  walk-through.
- **M5** collision, stairs/heights, doors.
- **M6** sprites (things, weapon overlay).
- **M7** combat: hitscan, monsters, damage, pickups.

Stretch: HUD, sound (out of scope for the "in the DB" claim anyway), multiplayer.

## Expected engine gaps

These would land in `prela` itself, each in its own small commit:

- ~~`collect` into dense `VecRel` / `MultiRel`~~ — done. `MultiRel` now holds
  `Cow<'static>` slices so collected relations own (and free) their storage.
- Possibly a `min`/`max` fold helper and a running-aggregate window helper
  (sugar over `fold` / `window`).
- `reach` keeps no visited set — fine for the BSP tree, but LOS / flood fills
  over sector adjacency would need one.

## Risks

- Frame budget: target ≥ 20 fps at 320×200 single-threaded. The per-pixel
  candidate union is the hot spot; stage 3 clipping keeps it bounded.
- Vanilla's renderer uses fixed-point tricks and quirks; we target "looks right",
  not pixel-exact.
- Contract drift: the easy way out is always a fat closure.
