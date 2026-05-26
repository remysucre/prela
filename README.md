# Prela: Purely Algebraic Relational Combinators

Prela is an embedded relational query language
 based on [Tarski's Algebra of Relations](https://en.wikipedia.org/wiki/Relation_algebra).
Prela queries are concise, clear, and fast.
The language is implemented by direct embedding 
 (a.k.a. [shallow embedding](https://decomposition.al/blog/2015/06/02/embedding-deep-and-shallow/))
 in a host programming language:
 Prela operators are implemented as regular functions in the host language.
The implementation follows [continuation passing style](https://en.wikipedia.org/wiki/Continuation-passing_style),
 which produces highly efficient code when combined with monomorphization and inlining.
We provide two implementations:
 the Julia engine enjoys elegant syntax thanks to operator overloading and multiple dispatch,
 while the Rust engine gives you (slightly) ugly but fast code.

## Examples

Prela queries are readable even to the untrained eye.

Join Order Benchmark [22a](https://github.com/gregrahn/join-order-benchmark/blob/master/22a.sql):

```julia
movie
   → (info → (Info.type == "countries")
           ∧ (Info.info in ("Germany", "German", "USA", "American")))
   ∧ (keyword in ("murder", "murder-in-title", "blood", "violence"))
   ∧ (production_year > 2008)
   ∧ (kind in ("movie", "episode"))
   : title
   × ((data → (Data.data < "7.0") ∧ (Data.type == "rating")) → Data.data)
   × ((company → (Company.note ≁ r"\(USA\)")
              ∧ (Company.note ~ r"\(200.*\)")
              ∧ (Company.country != "[us]")
              ∧ (Company.type == "production companies")) → Company.name)
```

TPCH [q21](https://github.com/dragansah/tpch-dbgen/blob/master/tpch-queries/21.sql):

```julia
late = lineitem ∧ (receiptdate > commitdate)
n_distinct = vs -> length(unique(vs))
qualifying = (late
    ∧ (Li.supplier → supplier ∧ (Su.nation → Na.name == "SAUDI ARABIA"))
    ∧ (order → (orders ∧ (Ord.status == "F"))
                # EXISTS another supplier on the order (across all lineitems)
                ∧ ((order ← Li.supplier) ▷ n_distinct > 1)
                # NOT EXISTS another LATE supplier (only L1 is late)
                ∧ ((order ← (late : Li.supplier)) ▷ n_distinct == 1)))
counts = (Li.supplier ← qualifying) ▷ ((a, _) -> a + 1, 0)
counts ⊗ Su.name
```

In the examples, constructs like `movie`, `Info.type` are regular Julia variables of type
 `Relation`, and operators like `→`, `∧`, and `in` are regular Julia functions overloaded
 to operate on relations.
Directly embedding Prela like this allows one to freely intermix queries with
 code of the host language to extend the reach of Prela,
 both in terms of expressiveness and performance.
For example, the Prela version of [TPCH Q13](https://github.com/dragansah/tpch-dbgen/blob/master/tpch-queries/13.sql)
 uses Julia code to implement `LEFT JOIN` semantics (Prela currently [has no `NULL`s](https://arxiv.org/abs/2307.15751)):

```julia
let live_orders = orders ∧ (Ord.comment ≁ r"special.*requests"),
    # Per-customer order count (only for customers with at least one match)
    count_per_cust = (Ord.customer ← (live_orders → date)) ▷ ((a, _) -> a + 1, 0)
    # Build the c_count → custdist distribution. Customers with no matching
    # orders get c_count = 0 (LEFT JOIN semantic).
    dist = Dict{Int, Int}()
    n_with = 0
    Prela.drive(count_per_cust, (_, c) -> begin
        dist[c] = get(dist, c, 0) + 1
        n_with += 1
    end)
    dist[0] = customer.n - n_with
    InlineRel{Int, Int}([k => v for (k, v) in dist])
end
```

Unlike SQL's user-defined functions, Prela "UDF"s are inlined and compiled together with the outer query
 without penalizing performance.
User can also swap out parts of the query with custom kernels to squeeze out extra performance,
 as exercised in the [Rust TPCH queries](./rust/src).

See [julia/queries.jl](./julia/queries.jl) and [julia/tpch_queries.jl](./julia/tpch_queries.jl) for more examples,
 or the corresponding Rust versions under [rust/src](./rust/src).

## Benchmark

**Take performance numbers with a grain of salt, Prela is not (and doesn't want to be) a database**

![TPCH performance](./rust/bench/tpch_scatter.png)

![JOB performance](./rust/bench/job_scatter.png)

## Prerequisites

- **JOB dataset cache** in `cache/`. The Rust and Zig builds *read* this cache;
  the Julia build *generates* it. So the first-time setup is: run Julia once
  to populate `cache/`, then the AOT builds can use it.

- **Julia 1.11+** — only needed to populate the cache, then for the Julia
  benchmark.
- **Rust 1.85+** (edition 2024).

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
