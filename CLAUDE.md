# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What Prela is, and what this branch is

Prela is a research-prototype embedded query language based on Tarski's Algebra of Relations. Queries are built from relation combinators; the implementation is a shallow embedding with no query optimizer — the query as written *is* the plan.

Each implementation lives on its own branch: `main` carries the Julia reference implementation (`julia/`), and the `rust` branch carries the Rust port. **This branch (`haskell`) carries the Haskell port in `haskell/`** — there is no `julia/` directory here, and README references to `julia/*` or `rust/bench/*` mean files on those branches. The concept docs `LANGUAGE.md` (operator semantics) also live on `main`.

## Data model

Everything is a binary relation — a set of (x, y) pairs read as a multi-valued function `d -> r`. A k-column SQL table is "shredded" into k binary relations, one per column, each keyed by a phantom-typed entity id (`Id e` where `e` is an empty tag type like `data Movie`). The PK column is the identity relation (the entity *universe*). There are no NULLs / no 3VL — a missing value is an absent pair, so `IS NULL` becomes set difference (`diff`).

## Architecture: modes and the tagless encoding

A relation is used in exactly two ways, and which one applies is a property of the relation's *position* in the query, not of the relation: **drive** (enumerate every pair — the outer-loop role, the left of a composition) and **probe** (given a key, produce its values — the inner role; `probeAny` is its short-circuiting cousin, membership is `probeAny` with a constant-true test).

The port encodes this tagless-final, not as a GADT/plan tree (MODES.md records why, plus the two rejected designs — read it before proposing an encoding change):

- `Prela.Mode` — the two mode records: `Drv d r` (just `drive`) and `Prb d r` (`probe` + `probeAny`).
- `Prela.Ops` — `class Mode q` with **exactly those two records as its only instances**. Leaves (`universe`, `column`, `fromIndex`, …) are class methods, so a leaf is mode-polymorphic and instantiates at whatever its position demands. Mode-free operators (`compose`, `prod`, `restrict`, `diff`, `filt`, `mapv`) take `q` and return `q`; the right-hand side of every binary operator is concretely `Prb` because that side is probed regardless. A query names no mode anywhere — the signature at the top (`Drv …` vs `Prb …` vs `Mode q => q …`) picks it and it flows down.
- `Prela.Predicate` — comparison sugar over `filt`: `eq ne gt lt ge le isIn between range rx nrx`.
- `Prela.Stream` — mode-*fixed* operators that still fuse: `invStream` (drive-only inverse), `leftCompose`, `groupBy`, `union`, `disj` (OR — membership only, value type `()`), `foldAll`, and the strict state monad `Acc` that folds thread accumulators through.
- `Prela.Materialize` — the deliberate pipeline breakers: `index`, `materialize`, `inv` (indexed inverse), `fold` (grouped, open-addressed table), `bufFold`, `countDistinct`, `foldDense`/`foldDenseOuter` (dense-array grouping by entity id), `bitset`. Each consumes a `Drv`, builds real storage once, and hands back something mode-polymorphic. Prela is non-materialized by default — a subexpression used twice re-evaluates twice unless one of these wraps it.
- `Prela.Storage` — `Id`, the physical column layouts (`Col`, `SparseCol`, `MultiCol`), and the `Elem` class that gives each element type its flat layout. The types *are* the on-disk layouts.
- `Prela.Cache` — mmap reader/writer for cache format v2 (one `<Entity>_<field>.bin` per column), byte-compatible with the Rust port; spec is `rust/src/format.rs` on the `rust` branch. `noId` is `-1` on purpose (matches the on-disk hole word); sparse-entity validity masks are derived at load, not stored. See CACHE.md.
- `Prela.Schema` — Template Haskell `declareSchema`: from one declaration it generates the tag types, a record of loaded columns, a loader, and **top-level accessor functions** taking the record. Accessors must stay top-level functions, not record fields — a relation stored in a record field defeats specialization and costs ~250 B/row (measured in `design/SchemaProbe.hs`).

`import Prela` re-exports everything except Cache and Schema, which only a dataset loader imports.

## Fusion is the whole bet — invariants

The entire point is that an operator chain fuses into one unboxed loop with no per-row allocation, matching columnar-engine speed. This regresses **silently** (no type error, no warning), so read FUSION.md before touching `Prela.Ops` or `Prela.Storage`, and re-verify with `design/CoreProbe.hs` after adding operators or storage kinds (its header has the exact ghc invocation; grep the dump for `Prb`, `$fMonad`, `MutVar#`, `((), I#` — all four must be absent). The rules:

- A leaf constructor must reach WHNF without forcing its storage: match the `Col`/`Store` *inside* the returned lambdas, never on the left of the `=`. Otherwise the record is a thunk GHC won't duplicate into the loop. No pragma recovers this.
- Never carry a per-row accumulator in an `STRef` — thread it through `Acc` so GHC unboxes it as a loop argument. (`ST` is fine for build-once caches like `fold`'s table.)
- Every `Mode` method and streaming operator carries `INLINE`; that is what makes fusion survive module boundaries. Both `Mode` instances must stay in `Prela.Ops` — as orphans their unfoldings would not reliably reach use sites.
- Materialized relations must be bound *monomorphically* (`Drv`/`Prb`, or the record type) — a mode-polymorphic binding is re-elaborated per use and would build its index twice.

## Common commands

All from `haskell/`. Toolchain: GHC 9.10, cabal 3.x.

```bash
cabal build all
cabal test                    # cache round-trips + schema layer (test/Spec.hs, TinySchema)
cabal run prela-demo          # tiny in-memory dataset exercising the core

# TPC-H: all 22 queries checked against recorded DuckDB oracles (../oracles/tpch,
# shared with the Rust port). Needs a binary cache built by the Rust `regen` tool
# (rust branch) — see benchmarking.md.
cabal run prela-tpch                  # cache at ../cache; ROUNDS=n, PRELA_CACHE=dir env overrides
cabal run prela-tpch -- 1 6 14        # just those queries

# Fusion check (see design/CoreProbe.hs header for the full flags):
cabal exec -- ghc -O2 -isrc -fforce-recomp design/CoreProbe.hs \
  -outputdir /tmp/coreprobe -o /tmp/coreprobe/probe -ddump-simpl -dsuppress-all -ddump-to-file
```

There is no separate lint/test suite beyond this: correctness for the query layer *is* the oracle check (`22/22 match reference`). To check one query during development, pass its number to `prela-tpch`. `design/*.hs` are standalone measurement probes and design prototypes, not part of the build; they document their own invocations and results in their headers.

## Writing queries

`tpch/TPCH/Queries.hs` is the canonical style. Conventions:

- A query takes the loaded schema record `s` and reaches columns via generated accessors (`quantity s`, `liOrder s`); a module wanting bare names rebinds them in a `where` with signatures (free at runtime).
- Shared subqueries get `Mode q => q d r` signatures so they work driven or probed; the top-level binding is concretely `Drv`/`Prb`.
- `collect` is the boundary: above it fused algebra, below it a plain list. Sorting and formatting (SQL ORDER BY) happen after `collect`, on the few rows that survive.
- Field names clashing across entities are renamed at declaration with `` `as` `` (entity prefix + field: `supplierName`, `liPart`); unique names keep their bare spelling.
- Oracle quirk: Q9's reference is patched for two one-cent float-summation-order rows (see `tpch/Main.hs`); the Rust port does the same.
