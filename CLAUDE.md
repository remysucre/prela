# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What Prela is

Prela is a research-prototype embedded query language based on Tarski's Algebra of Relations. Queries are built from *relation combinators* — overloaded Julia operators that take and produce binary relations. The implementation is a shallow embedding: a Prela query *is* a Julia expression, and there is no query optimizer — the query as written *is* the plan.

`main` carries only the Julia implementation (`julia/`). Experimental Rust and Zig ports live on separate branches (`git checkout rust`); the `rust/` and `zig/` dirs are gitignored here. README references to `rust/bench/*` scripts mean files on the `rust` branch, not `main`.

## Data model (read before touching queries)

Everything is a relation of arity ≤ 2, thought of as a multi-valued function `D → R`. A k-column SQL table is "shredded" into k binary relations, one per column, each keyed by a phantom-typed entity ID (`ID{E} where E <: Entity`). The PK column itself is the identity relation (the entity *universe*). There are **no NULLs / no 3VL** — a missing value is simply an absent pair, so SQL `IS NULL` becomes set difference (`- r`).

Schemas are declared with macros (see `julia/JOB.jl` for the canonical example):
- `@declare A B C …` forward-declares entity types (needed for cyclic FK refs).
- `@entity E begin field :: T … end` declares E's fields. `f :: T` is a 1:1 function; `f :: Multi{T}` is multi-valued. `f :: ID{Other}` is a foreign key. The first field is the *primary field* unless otherwise designated.
- `@expose E` makes E's fields bare-name accessible (`title`, `keyword`, …). A polymorphic field shared by two entities must stay qualified (`Movie.info` vs `Info.info`).

## The operators

Full semantics live in `LANGUAGE.md`; precedence/paren rules in `julia/STYLE.md` (read it before writing or editing query expressions — Julia's operator precedence makes parens load-bearing). Quick reference:

- `→` composition/join (also field navigation: `movie → Movie.info → Info.note`)
- `∧` intersection / AND (an **alias of `×`**); `∨` membership union / OR (probe-only, never enumerated)
- `×` product — pairs same-domain columns into a tuple value (`x→y` and `x→z` ⇒ `x→(y,z)`)
- `:` restriction/filter (`a : b` keeps rows of `a` whose value is a member of `b`)
- `-` set difference; predicates `== != < > <= >=`, `in`, regex `~` / negated `≁`
- `'` (postfix adjoint) inverse; `←` left-compose (`r ← s` ≡ `r' → s`)
- `▷` group-by-key fold (`q ▷ (op, init)`; 3-tuple `(op, init, n)` opts into dense-array grouping; single fn = buffered reduce); `⊵` no-group fold; `↦` per-row map; `unwrap` escapes a `⊵` result to a scalar
- `materialize(q)` / `!q` forces a leg once for reuse; `bitset(q, n)` precomputes dense membership. **Prela is non-materialized by default** — a subexpression used twice is re-evaluated twice unless wrapped.

Predicate elision: comparing an entity-typed expression to a scalar auto-traverses to the primary field (`keyword == "sequel"` ≡ `keyword → Keyword.keyword == "sequel"`).

## Architecture: how a query executes

The core is `julia/Prela.jl` (~1300 lines). The pipeline is:

1. **Build** — operators construct a typed query tree where `Query{D,R}` nodes carry the *entire plan in their type parameters* (`Compose`, `Filter`, `Prod`, `Restrict`, `Fold`, `Inv`, `LeftCompose`, …). Nothing runs at build time.
2. **`prepare(plan, Driven())`** — treat this as *compilation*. It rewrites the tree top-down so each node knows its access mode (`Driven` vs `Probed`) at the type level, and lowers mode-polymorphic nodes into concrete single-mode forms (e.g. `Inv` → `InvStream` when driven / `InvIndexed` when probed; `Materialized` → `MatStream`/`MatProbed`; `Fold` → an eagerly-built `FoldP` cache). Memo-free and type-stable.
3. **Execute via the CPS protocol** — four functions fuse the whole tree into one loop nest through Julia's monomorphization + inlining:
   - `drive(q, k)` — call `k(x, y)` for every pair
   - `probe(q, x, k)` — call `k(y)` for every `y` related to key `x`
   - `probe_any(q, x, k)` — like probe but short-circuits (Bool threaded through returns, allocation-free); `member(q, x)` = `probe_any(q, x, _->true)`
   - `drivekeys(q, k)` — emit each domain key

This continuation-passing style is the central trick: combinators compose by passing continuations, so a multi-operator query fuses into a single streaming pass with **no materialized intermediates** — recovering columnar execution from algebra-style queries. When editing the executor, the invariant is that drive/probe/probe_any carry *no per-row format branch*: the node type *is* the physical layout.

**Leaf storage** (sealed once after load by `seal_entities!`, dispatched on declared multiplicity + data density): `VecRel` (dense 1:1 column store), `SparseRel` (1:1 with gaps + presence `BitVector`), `MultiRel` (dense forward index for multi-valued). A `Universe{E}` stores just `n` and iterates a range. Leaves start as mutable `Staging` (filled at load) and `seal_entities!` rebinds each to its immutable sealed form — `Staging` is deliberately *not* a `Query`, so an unsealed leaf cannot run.

Performance notes baked into the code (preserve when refactoring): `Prod` drive/probe/member are `@eval`'d per-arity (1–8) to avoid recursive-closure tuple allocations; `@inbounds` on the hot leaf loops; dense `Vector{Vector}` indexes for entity-keyed relations instead of hashing. The whole point is matching DuckDB's single-thread speed without a vectorized/compiled engine, so allocation in hot paths is a regression.

## Common commands

All commands run from `julia/`. Requires Julia 1.11+.

```bash
# First-time JOB setup: ingest raw CSVs/parquet, write binary cache to ../cache/*.bin (~30s once, ~2s mmap after)
julia --project=. -e 'include("JOB.jl")'

# Run all 113 JOB queries (parallel via @threads; use -tN for threads), check each against its recorded reference
julia --project=. -e 'include("JOB.jl"); include("queries.jl"); runall()'

# TPC-H (after generating cache/tpch/*.parquet via DuckDB — see benchmarking.md)
julia --project=. -e 'include("TPCH.jl"); include("tpch_queries_idiomatic.jl")'   # auto-runs runall_tpch()
# (or tpch_queries_optimized.jl for the hand-tuned variants)

# Interactive REPL with Revise auto-reload on edits to Prela.jl — the dev loop
julia --project=. -i -e 'include("start.jl")'
# then in the REPL: include("JOB.jl"); includet("queries.jl")  (includet = re-run on edit)

# Single-thread warm benchmark (what the plot scripts consume)
julia --project=. -t1 bench.jl job                > bench/data/julia_job.txt
QS=idiomatic julia --project=. -t1 bench.jl tpch  > bench/data/julia_tpch_idiomatic.txt

# Regenerate comparison plots (reads bench/data/, writes PNGs alongside)
cd bench && python3 plot_job.py && python3 plot_tpch.py
```

There is no separate lint/test suite: correctness *is* the `runall()` / `runall_tpch()` reference check (`N/113 queries match reference`). To check a single query during development, run it in the REPL workflow above, or temporarily narrow the `_Q` registry. Data lives in `../jobdata/parquet/` (JOB) and `../cache/tpch/*.parquet` (TPC-H); the binary `../cache/` is gitignored and regenerated on a miss. Full dataset setup (including the DuckDB `dbgen` invocation for TPC-H) is in `benchmarking.md`.

## Adding or editing queries

Queries register via `_q(thunk, name, oracle)` into the `_Q` vector and are checked by min-tuple against the `oracle` reference string. Idiomatic style (per `queries.jl` header): root movie-queries at `movie`, cast-queries at `cast`; fuse each filter onto the column it constrains so intermediates stay small. When a query expression doesn't parse the way you expect, the cause is almost always operator precedence — consult `julia/STYLE.md`, which documents exactly where parens are required (around comparison predicates inside `∧` chains, around `→`/`:` conjuncts) and where they are redundant.
