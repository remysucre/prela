# Prela: Purely Algebraic Relational Combinators

Prela is an embedded query language
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
Notably, tables are decomposed into [sixth normal form](https://en.wikipedia.org/wiki/Sixth_normal_form),
 so `keyword` is a *relation* mapping each movie ID to a string.
The overhead of "joining back together" the decomposed columns is eliminated by continuation passing style
 which produces code that co-iterates the column tables.

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

**Take performance numbers with a grain of salt, Prela is not (and doesn't
want to be) a database.** Numbers below are SF=1 TPC-H (~6M lineitems)
and JOB on the full IMDB dataset (~36M cast records), warm run, single
thread on an Apple M-series laptop. DuckDB-ST is `SET threads = 1`.

<p>
  <img src="./rust/bench/tpch_scatter.png" width="49%" alt="TPC-H SF=1">
  <img src="./rust/bench/job_scatter.png" width="49%" alt="JOB">
</p>

**TPC-H SF=1** (left): same algebraic queries in Rust and Julia, against
DuckDB-ST. Rust prela (gray) is within striking distance of DuckDB even
at the *idiomatic* level — no per-query algorithmic rewriting. There are
also `optimized` and `ddbcheat` Rust variants (sequence of broader
engine fixes + DuckDB-plan-inspired tricks) that pull below the diagonal;
the `ddbcheat` variant scaled to SF=10 lands ~2× faster than DuckDB-ST.
See `rust/src/tpch_queries_{idiomatic,optimized,ddbcheat}.rs` for the
sources.

**JOB** (right): all 113 queries on the full IMDB dataset. The Rust
algebra is ~9× faster than DuckDB-ST; Julia, which uses the same algebra
in its native form, comes in at ~1.8×.

Reproduce: see `rust/bench/plot_{tpch,job}.py` and `julia/bench.jl`. The
underlying data files live in `rust/bench/data/`.

## Prerequisites

- **JOB dataset cache** in `cache/`. The Rust build *reads* this cache;
  the Julia build *generates* it. So the first-time setup is: run Julia
  once to populate `cache/`, then the AOT builds can use it.
- **TPC-H cache** (if you want to run TPC-H): a binary cache in `cache/`
  (Rust) and parquet files in `cache/tpch/` (Julia). See *TPC-H setup*
  below.
- **Julia 1.11+** — needed to populate the JOB cache and to run the
  Julia benchmark.
- **Rust 1.85+** (edition 2024).
- **DuckDB** (only if you want to run the comparison plots) — used by
  the bench scripts and to seed TPC-H parquet at any scale factor.

## First-time setup: JOB cache

```bash
cd julia
julia --project=. -e 'include("JOB.jl")'
```

Ingests the raw JOB CSVs (~9 GB) and writes a binary relation cache into
`prela/cache/*.bin` — 48 files (~hundreds of MB total). Takes ~30 s the
first time; subsequent runs mmap straight from the cache in ~2 s.

## First-time setup: TPC-H cache

Both the Rust regen tool and Julia load from `cache/tpch/*.parquet`.
Generate them via DuckDB at any scale factor (synthetic IDs go first
because the Rust regen reads parquet via `arrow-rs` projection which
preserves file order):

```bash
cd /path/to/prela
duckdb < /dev/stdin <<'EOF'
INSTALL tpch; LOAD tpch;
CALL dbgen(sf = 1);                    -- or sf = 10 for the big run
COPY (SELECT CAST(row_number() OVER () AS BIGINT) AS ps_id,
             CAST(ps_partkey AS BIGINT) AS ps_partkey,
             CAST(ps_suppkey AS BIGINT) AS ps_suppkey,
             CAST(ps_availqty AS BIGINT) AS ps_availqty,
             CAST(ps_supplycost AS DOUBLE) AS ps_supplycost,
             ps_comment
        FROM partsupp)
  TO 'cache/tpch/partsupp.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(row_number() OVER () AS BIGINT) AS l_id,
             CAST(l_orderkey AS BIGINT) AS l_orderkey,
             CAST(l_partkey AS BIGINT) AS l_partkey,
             CAST(l_suppkey AS BIGINT) AS l_suppkey,
             CAST(l_linenumber AS BIGINT) AS l_linenumber,
             CAST(l_quantity AS DOUBLE) AS l_quantity,
             CAST(l_extendedprice AS DOUBLE) AS l_extendedprice,
             CAST(l_discount AS DOUBLE) AS l_discount,
             CAST(l_tax AS DOUBLE) AS l_tax,
             l_returnflag, l_linestatus,
             strftime(l_shipdate, '%Y-%m-%d') AS l_shipdate,
             strftime(l_commitdate, '%Y-%m-%d') AS l_commitdate,
             strftime(l_receiptdate, '%Y-%m-%d') AS l_receiptdate,
             l_shipinstruct, l_shipmode, l_comment
        FROM lineitem)
  TO 'cache/tpch/lineitem.parquet' (FORMAT PARQUET);
-- The other six tables don't need synthetic IDs — direct CAST works:
COPY (SELECT CAST(r_regionkey AS BIGINT) AS r_regionkey,
             r_name, r_comment FROM region) TO 'cache/tpch/region.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(n_nationkey AS BIGINT) AS n_nationkey, n_name,
             CAST(n_regionkey AS BIGINT) AS n_regionkey, n_comment
        FROM nation) TO 'cache/tpch/nation.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(s_suppkey AS BIGINT) AS s_suppkey, s_name, s_address,
             CAST(s_nationkey AS BIGINT) AS s_nationkey, s_phone,
             CAST(s_acctbal AS DOUBLE) AS s_acctbal, s_comment
        FROM supplier) TO 'cache/tpch/supplier.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(c_custkey AS BIGINT) AS c_custkey, c_name, c_address,
             CAST(c_nationkey AS BIGINT) AS c_nationkey, c_phone,
             CAST(c_acctbal AS DOUBLE) AS c_acctbal, c_mktsegment, c_comment
        FROM customer) TO 'cache/tpch/customer.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(p_partkey AS BIGINT) AS p_partkey, p_name, p_mfgr, p_brand,
             p_type, CAST(p_size AS BIGINT) AS p_size, p_container,
             CAST(p_retailprice AS DOUBLE) AS p_retailprice, p_comment
        FROM part) TO 'cache/tpch/part.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(o_orderkey AS BIGINT) AS o_orderkey,
             CAST(o_custkey AS BIGINT) AS o_custkey, o_orderstatus,
             CAST(o_totalprice AS DOUBLE) AS o_totalprice,
             strftime(o_orderdate, '%Y-%m-%d') AS o_orderdate,
             o_orderpriority, o_clerk,
             CAST(o_shippriority AS BIGINT) AS o_shippriority, o_comment
        FROM orders) TO 'cache/tpch/orders.parquet' (FORMAT PARQUET);
EOF
# Convert parquet → Rust binary cache (one-shot).
cd rust
cargo build --release --features regen --bin regen
./target/release/regen ../cache/tpch ../cache
```

For SF=10, swap `sf = 1` to `sf = 10` above. The Julia TPC-H loader
currently materializes the whole DataFrame in memory, so SF=10 needs
~30 GB — the plots check in are SF=1 for that reason. Rust + DuckDB
both handle SF=10 fine on a 32 GB machine.

The Rust binary cache (`cache/*.bin`) is what `./target/release/prela tpch`
mmaps at startup; the parquet files are what Julia reads directly via
Parquet2.jl.

## Run the suites

### Julia (JOB)

```bash
cd julia
julia --project=. -e 'include("JOB.jl"); include("queries.jl"); runall()'
```

Prints each query's result + match-against-reference timing in parallel
(`@threads`), then a `N/113 queries match reference` summary.

For an interactive REPL workflow with Revise auto-reload on edits:

```bash
julia --project=. -i -e 'include("start.jl")'
```

### Julia (TPC-H)

```bash
cd julia
julia --project=. -e 'include("TPCH.jl"); include("tpch_queries.jl"); runall_tpch()'
```

### Julia (single-thread warm bench, used by the plot scripts)

```bash
cd julia
julia --project=. -t1 bench.jl job  > ../rust/bench/data/julia_job.txt
julia --project=. -t1 bench.jl tpch > ../rust/bench/data/julia_tpch.txt
```

### Rust (JOB)

```bash
cd rust
cargo build --release
./target/release/prela           # default: JOB suite
```

Prints `load: …s`, runs the 113 queries twice (cold + warm), reports
`N/N ok` plus per-query timing. Build takes ~20–30 s clean (LLVM
optimizing 113 generic monomorphizations); warm runs land at ~6 s.

### Rust (TPC-H)

```bash
./target/release/prela tpch                   # default: optimized variant
QS=idiomatic ./target/release/prela tpch      # the algebra-port baseline
QS=optimized ./target/release/prela tpch      # engine-wide Tier-1+2 fixes
QS=ddbcheat  ./target/release/prela tpch      # plus all DuckDB-plan-inspired tricks
```

Same protocol as JOB: cold + warm runs, per-query timing. The `QS` env
var picks which variant (`tpch_queries_idiomatic.rs` /
`tpch_queries_optimized.rs` / `tpch_queries_ddbcheat.rs`) to run.

### Regenerate the comparison plots

```bash
cd rust/bench
python3 plot_tpch.py   # → tpch_scatter.png  (3 panels, one per variant)
python3 plot_job.py    # → job_scatter.png
```

Both scripts read from `data/` and write the PNG next to themselves. The
data files are captured from the bench runs above.
