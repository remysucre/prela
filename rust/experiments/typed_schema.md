# Typed schema prototype — `schema!`, `Id<E>` columns, `.p()` elision

Layers 2–3 on top of the `Dense`/`Id<E>` engine generics (layer 1):
a `macro_rules!` schema declaration that generates typed column storage +
accessors, and a `.p()` extension that elides the entity → primary-column
compose. Evaluated by porting all of `src/queries/t1.rs` (29 JOB queries)
and TPC-H Q1/Q21 (idiomatic) to the typed surface.

## What was built (per-layer cost)

| layer | new/changed code | size |
|---|---|---|
| `schema!` macro | `src/schema.rs` (macro + synthetic-cache round-trip test) | ~190 macro lines + 110 test lines |
| typed readers | `src/cache.rs`: `_in` variants (dir-parameterized, `D: Dense`-generic, `Id<T>` bulk reinterpret) | ~70 lines, old entry points kept as wrappers |
| schema decls | `src/job_schema.rs` (20 entities, 46 columns), `src/tpch_schema.rs` (8 entities, 61 columns) | 70 + 75 lines |
| `.p()` elision | `src/engine.rs`: `trait Primary`, `QueryExt::p`, `is_in`, `n_keys`, `Ord for Id` | ~50 lines |
| typed ports | `src/queries_typed/{t1,tpch}.rs` + `main.rs` suites `job-typed`/`tpch-typed` | 430 + 65 lines |

No proc-macro was needed: `macro_rules!` covers the whole feature
(see friction log for the two contortions it forced).

Design notes, as planned:

- Entity tags are unit structs; `Movie(movie)` explicitly names the bare
  universe accessor (`fn movie() -> Universe<Id<Movie>>`, sized from the
  entity's first column). `pub` on a field explicitly grants a bare
  accessor fn; every field gets an entity-qualified one (`Data::type_()`).
- Id-valued columns are **bulk-reinterpreted**: `Id<E>` is
  `repr(transparent)` over `usize`, so `cast_slice::<Id<T>>` reads the v2
  cache words as tagged ids directly (chosen over per-element map — zero
  conversion cost, same one `unsafe` already used for `usize`).
- `type_`-style raw-keyword fields stringify with the trailing underscore;
  the readers trim it at open time (`Data_type_` → `Data_type.bin`).
- `Primary` is generated from the first declared field;
  `q.p()` = `Compose { a: q, b: E::primary() }` for `Q: Query<R = Id<E>>`.
- The macro becoming regen's source of truth (deriving the cache layout
  from the declaration) is the planned follow-up, **not** in scope here.

## Friction log

1. **The single-name `eq` wall** (decided before this build; recording it).
   A Julia-style `eq` that dispatches on scalar vs entity-valued columns
   can't be expressed: overloading by a second trait is coherence-ambiguous
   regardless of argument types, and the dispatch-trait route
   (`Self::R: EqWith<V>` with a GAT `Out<Q>`) cannot name the output type
   now that `Filter` holds bare `Fn` closures — there is no `impl Trait`
   in GAT position, so it would force boxing or named predicate structs.
   `.p()` (compose the primary column, then ordinary `eq`) costs two
   characters and zero coherence fights. No cleaner single-name route
   surfaced during the build; if one ever exists it likely needs inherent
   per-column methods from a proc-macro, not trait dispatch.
2. **`$(pub)?` is ambiguous in `macro_rules!`** — the `ident` fragment
   matches keywords, so `$(pub)? $f:ident` is a "local ambiguity" error.
   Fixed by push-down accumulator munchers for the cols-struct and
   init-literal rules and a `pub`-stripping rule for the first-field rule
   (~40 extra macro lines). Related: captured `:vis` fragments become
   opaque (can't later re-match the literal `pub`), so the macro munches
   raw token trees throughout.
3. **No ident manipulation in `macro_rules!`** — can't lowercase
   (`Movie` → `movie`) or concatenate (`MovieCols`), hence the explicit
   universe ident in the declaration and the cols struct reusing the
   entity's own name inside the generated schema module
   (`JOB::Movie` vs tag `Movie`; storage fields are non-snake-case entity
   names under `#[allow(non_snake_case)]`).
4. **Bare-name collisions are the schema author's job** — macro_rules
   can't see across entities. JOB collides heavily: `title` (Movie,
   AkaTitle), `kind` (×4), `info` (×5), `name` (×4), `note` (×4),
   `type_` (×5), `keyword`, `data`, `link`, `aka`, `role`. So the most-used
   JOB columns stay entity-qualified, and only 14 fields are bare.
   TPC-H is the opposite: almost all of Lineitem/Part/Order is bare, and
   typed Q1 reads nearly like the Julia original.
5. **Dead-code lint vs generated surface** — a binary crate warns on
   unused pub fns, and a schema legitimately declares more than any one
   suite uses. The macro emits `#[allow(dead_code)]` on tags, storage,
   and accessors; one engine method (`MultiRel::n_keys`, only needed when
   an entity's *first* column is Multi) carries a commented allow.
6. **Misc**: `Id` needed `Ord` (`count_distinct` sorts groups — untyped
   `usize` keys had it for free); Rust 2024's `impl Trait` capture rules
   forced one `let` binding in the suite wiring; `oracle()`/`Q1` in
   `tpch::common` had to become `pub` for the typed module to share them.

## Notation comparison

Methodology: whole `fn` text for Rust (signature through closing brace),
`do … end` body for Julia (its harness owns oracle/row formatting, which
slightly favors Julia, especially on TPC-H where the Rust fns inline
sort/format scaffolding). Chars are non-whitespace. Rust totals include
the shared family fns (`q2`, `q4`, `q3ac`); Julia spells every variant out.

**t1 totals (29 queries):**

| | lines | chars |
|---|---|---|
| untyped Rust | 503 | 11,533 |
| typed Rust | 366 | 9,154 (−21%) |
| Julia | 207 | 5,732 |

**Exemplars (lines / chars):**

| query | untyped | typed | Julia |
|---|---|---|---|
| 2a (incl. family fn) | 8 / 257 | 7 / 183 | 4 / 86 |
| 3b | 10 / 221 | 7 / 162 | 5 / 87 |
| 4a (incl. family fn) | 14 / 354 | 13 / 280 | 5 / 113 |
| 1a | 19 / 439 | 14 / 364 | 7 / 244 |
| 5a | 16 / 345 | 11 / 274 | 7 / 175 |
| 11a | 24 / 529 | 17 / 414 | 9 / 265 |
| 12a | 25 / 543 | 20 / 426 | 7 / 266 |
| 13a | 16 / 434 | 13 / 328 | 6 / 187 |
| 14a | 22 / 543 | 17 / 442 | 8 / 191 |
| 22a | 36 / 734 | 23 / 584 | 12 / 365 |
| TPC-H Q1 | 25 / 853 | 20 / 703 | 12 / 296 |
| TPC-H Q21 | 31 / 1213 | 22 / 876 | 10 / 392 |

Where the savings come from: `.p()` collapses every
`(&d.movie_kind).o(&d.kind_kind)` to `Movie::kind().p()`; accessors drop
the `(&d.…)` plumbing and the `d: &Data` parameter; `is_in([..])` drops
`in_v(vec![..])`. The residual gap to Julia is symbol operators
(`: ∧ → ×` vs `.in_s .and .o .x`), Julia's bare unique column names
(its global namespace tolerates what Rust's collision rules made explicit
here), and Rust's `min_row(...)`/`String` scaffolding.

## Compile time

Full crate rebuild (`touch src/**/*.rs; cargo build --release`,
lto=fat, codegen-units=1, M-series):

- before prototype: **27.2s**
- after (both schemas + 31 typed queries + suites): **31.0s** (+3.8s, +14%)

## Verification

- `cargo check --all-targets` and `cargo check --all-targets
  --features regen`: clean, **zero warnings** (the pre-existing `Id::new`
  dead-code warning is consumed by `Dense::from_idx`).
- `cargo test`: **21/21** (18 existing + 3 new: typed fixture composition
  & `.p()` elision; `schema!` round-trip over a synthetic v2 cache dir
  incl. the `type_` filename trim and the `Id` bulk reinterpret;
  full `job_init` parity vs the untyped loaders — universe sizes, column
  lengths, value spot-check, end-to-end count equality).
- `job`: **114/114** both runs (run 2: 5.06s — matches the 5.13s baseline).
- `tpch` idiomatic/optimized/ddbcheat: **22/22 × 3**.
- `job-typed`: **29/29** both runs.
- `tpch-typed`: **2/2**.
- Run-2 timings, typed vs untyped, same binary: t1 total 0.997s vs 1.004s
  (every query within ±10%, i.e. noise); Q1 0.059s vs 0.058s, Q21 0.312s
  vs 0.316s. The phantom types are fully erased, as expected.

## Recommendation: **sweep**

The typed surface is strictly nicer to write (−21% chars on t1, −28% on
Q21), turns cross-entity plumbing mistakes into compile errors, costs
nothing at runtime, and adds 14% to a full release rebuild. Porting was
mechanical (no query needed restructuring). Sweep the remaining suites
(t2–t6, demo_methods, the other 20 TPC-H queries), then retire the
hand-written `Data`/`TpchData` loaders; make `schema!` regen's source of
truth as the follow-up. One adjustment worth folding into the sweep
rather than doing first: queries over collision-heavy JOB names could
read better with per-query `use`-style local aliases, but that's
ergonomics, not a blocker.

## Swept (follow-up, done)

The sweep landed: the typed surface is now THE way and the untyped path is
retired.

- **Ports**: all 114 JOB queries (t1–t6 + demo_methods, incl. the shared
  `film_or_warner_co`/`follow_link` helpers) and all 22 TPC-H queries in
  all three variants (idiomatic in common.rs, 10 optimized overrides,
  12 ddbcheat overrides). ddbcheat's raw loops port faithfully —
  `Id` is `repr(transparent)`, so they index `.values` via `.idx()`.
- **Retired**: `src/data.rs`, `src/tpch_data.rs` (the hand-written
  loaders, 363 lines), the whole untyped query text, the side-by-side
  `src/queries_typed/`, and the `job-typed`/`tpch-typed` suites — suites
  are plain `job`/`tpch` again. Net source delta of the ENTIRE feature vs
  the pre-prototype tree: **+51 lines** (+2988/−2937 across 23 files) —
  the macro + schemas + manifest paid for themselves in deleted loaders
  and `&d.` plumbing.
- **Crate shape**: `src/lib.rs` now exists so `prela` (runner) and `regen`
  share one source of truth; `Entry` is the no-argument
  `fn() -> String` registry type, queries read the schemas' `OnceLock`
  stores.
- **`schema!` as regen's source of truth**: the macro now also emits a
  `MANIFEST: &[(entity, field, KIND_*)]` per schema. regen's
  parquet→cache transformation logic stays hand-written (it is
  parquet-specific), but regen records every file it writes and verifies
  the set against the manifest — name for name (raw-keyword underscore
  trimmed) and header kind for kind, both directions, failing the run on
  drift. Verified live: `regen tpch` (55 columns) + `regen job`
  (48 columns) to a scratch dir, spot-`cmp` byte-identical to the
  existing cache.
- **Verification**: `cargo check --all-targets` ± `--features regen`
  zero warnings; `cargo test` 21/21; JOB **114/114** with result strings
  byte-identical to the pre-sweep baseline (timing-stripped diff); TPC-H
  **22/22 × 3** oracle-exact.
- **Final perf** (run 2, same machine/day): JOB 5.20–5.23s (pre-sweep
  baseline 5.09s, band ≤ ~5.5); idiomatic 1.16s (1.17 before);
  optimized 0.52–0.54s (0.52); ddbcheat 0.49–0.52s (0.47). All within
  the bands; per-query timings move ±10% with code layout across
  rebuilds, as before.
- **Compile time**: full clean release build 32.5s (≈31s expected ±
  macro growth).
- **Docs**: TRANSLATION.md rewritten to the typed spellings (accessors,
  `.p()`, `is_in`, `Id<E>`/`Dense::NONE`); README's one Rust example is
  now `movie().in_s(production_year().gt(2008)).o(title())`
  (`Movie.title` was made `pub` for the bare accessor);
  benchmarking.md needed no change (commands unchanged).
