# Prela

A small navigational query language over typed, strictly-binary relations,
with three reference implementations sharing the same algebraic engine:

- **`julia/`** — original prototype with infix surface syntax (`→ ∧ × :`),
  top-down CPS, JIT-fused.
- **`rust/`** — AOT port, generic monomorphization + `#[inline(always)]`.
- **`zig/`** — AOT port, `comptime` monomorphization + sink-struct CPS.

For the language design, see [`LANGUAGE.md`](LANGUAGE.md).

## Benchmark — JOB (Join Order Benchmark), 113 queries, single-threaded

|  | total (warm) | per-query bench |
|---|---|---|
| **prela-zig** | **5.49 s** | full suite under [zig/](zig/) |
| **prela-rs**  | **5.82 s** | full suite under [rust/](rust/) |
| DuckDB        | 19 s       | reference column store |
| **prela-julia** (steady) | 90 s | full suite under [julia/](julia/) |
| Postgres (indexed) | 152 s | reference baseline |

All three impls produce identical results — the same algebra emits the same
fused loop nest in each target. The Rust and Zig builds beat DuckDB by ~3×;
the Julia version with JIT trails but still beats indexed Postgres.

## Repo layout

```
prela/
├── README.md            this file
├── LANGUAGE.md          language design + operator reference
├── cache/               JOB binary cache (gitignored; generated on first Julia run)
├── julia/               original Julia implementation
├── rust/                AOT Rust port
└── zig/                 AOT Zig port
```

## Prerequisites

- **JOB dataset cache** in `cache/`. The Rust and Zig builds *read* this cache;
  the Julia build *generates* it. So the first-time setup is: run Julia once
  to populate `cache/`, then the AOT builds can use it.

- **Julia 1.11+** — only needed to populate the cache, then for the Julia
  benchmark.
- **Rust 1.85+** (edition 2024).
- **Zig 0.16+** — uses the new `std.process.Init` main signature + `Io`
  vtable.

## First-time setup: populate the cache

```bash
cd julia
julia --project=. -e 'include("JOB.jl")'
```

This ingests the raw JOB CSVs (~9 GB) and writes the binary relation cache
into `prela/cache/*.bin` — 48 files, all small (~hundreds of MB total). Takes
roughly 30 s on the first run. Subsequent runs mmap straight from the cache
in ~2 s.

## Run the Julia suite

```bash
cd julia
julia --project=. -e 'include("JOB.jl"); include("queries.jl"); runall()'
```

Prints each query's result + match-against-reference timing, then a
`N/113 queries match reference` summary.

For an interactive REPL workflow (Revise auto-reload on edits):

```bash
cd julia
julia --project=. -i -e 'include("start.jl")'
```

## Run the Rust suite

```bash
cd rust
cargo build --release
./target/release/prela
```

Prints `load: …s`, runs the 113 queries twice (cold + warm), reports
`N/N ok` plus per-query timing for slow queries. Build takes ~20 s clean
(LLVM optimizing 113 generic monomorphizations); steady runs land at ~5.8 s.

## Run the Zig suite

```bash
cd zig
zig build -Doptimize=ReleaseFast
./zig-out/bin/prela-zig
```

Same output shape as the Rust version. Build takes ~15 s clean; steady runs
land at ~5.5 s.

## Notes on the build numbers

The ~15–20 s release-mode build is **almost entirely LLVM optimization** of
the 113 deeply-nested monomorphizations (each query is a unique concrete
instantiation of `Restrict<Universe, Restrict<Conj<…>, Prod<…>>>`). The
engine itself (operator structs + CPS protocol) is generic and produces
almost no code on its own — compile time scales with how many queries you
register and how deeply they nest.

Incremental builds:
- **Zig**: ~12 s with a real edit, **~90 ms with `touch` only** (content-hash).
- **Rust** (LTO=fat, codegen-units=1): ~17 s either way, since cargo rebuilds
  the crate on any mtime change.

## How the three impls share an algebra

Every port has the same set of nodes and the same CPS protocol:

| Node    | Role                                               |
|---------|----------------------------------------------------|
| `Vec1`  | total 1:1 dense leaf relation, `Vec<R>` by id      |
| `Many`  | multi-valued or partial leaf, `Vec<Vec<R>>` by id  |
| `Universe` | a `SetQ` over `[1, n]`                          |
| `Compose<A, B>` | Query ∘ Query — bridge is value           |
| `Restrict<S, Q>` | SetQ ∘ Query — bridge is key            |
| `Filter<A, P>` | value-side predicate                       |
| `Conj` / `Disj` / `SetDiff` | SetQ boolean algebra          |
| `Prod`  | Cartesian product per key                          |
| `Keys`  | Query → SetQ (forget value)                        |

All three ports use a single method `.o` that picks compose vs restrict based
on the receiver's kind, via:
- Julia: multiple dispatch on operator `→`.
- Rust: trait method on `QueryExt` vs `SetQExt`.
- Zig: method lookup on the receiver's struct type (Query types vs SetQ
  types define different `.o` methods).

The result is one unified surface (`.o`, `.k`, `.and`, `.or`, `.minus`, `.x`,
`.eq`, `.ne`, `.gt`, `.lt`, `.ge`, `.le`, `.in_v`, `.in_s`, `.rx`, `.nrx`)
across all three.

## License

See [LICENSE](LICENSE) if present, otherwise treat as research code — no
warranty.
