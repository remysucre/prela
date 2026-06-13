## Prerequisites

- **JOB parquet** at `../jobdata/parquet/` (the IMDB tables) — regen
  converts these into the binary cache in `cache/` that the engine mmaps.
- **TPC-H cache** (if you want to run TPC-H): parquet files in
  `cache/tpch/`, converted to the binary cache by the same regen tool.
  See *TPC-H setup* below.
- **Rust 1.85+** (edition 2024).
- **DuckDB** (only if you want to run the comparison plots) — used by
  the bench scripts and to seed TPC-H parquet at any scale factor.

(The historic Julia engine and its benchmark setup live on the
`julia-engine` branch.)

## First-time setup: JOB cache

```bash
cd rust
cargo run --release --features regen --bin regen -- job   # defaults: ../../jobdata/parquet → ../cache
```

Converts the JOB parquet into a binary relation cache at
`prela/cache/*.bin` — 48 files (~4.2 GB), ~7 s. The cache (format v2 —
see `rust/src/format.rs`) stores the final physical layouts (0-based ids,
holes pre-filled, CSR multi columns), so the engine startup is just
mmap + bulk copy: the JOB tables load in ~0.2 s.

## First-time setup: TPC-H cache

The regen tool loads from `cache/tpch/*.parquet`.
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
./target/release/regen tpch ../cache/tpch ../cache
```

For SF=10, swap `sf = 1` to `sf = 10` above. Rust + DuckDB both handle
SF=10 fine on a 32 GB machine (the checked-in plots are SF=1).

The Rust binary cache (`cache/*.bin`) is what `./target/release/prela tpch`
mmaps at startup.

## Run the suites

### Rust

The Rust implementation lives in-tree under `rust/` (eager physical state,
compile-time access modes — see `rust/src/engine.rs`).

```bash
cd rust
cargo run --release            # JOB suite
cargo run --release -- tpch    # TPC-H (QS=idiomatic|optimized)
```

### Regenerate the comparison plots

```bash
cd rust/bench
python3 plot_tpch.py   # → tpch_scatter.png  (idiomatic + optimized vs DuckDB)
python3 plot_job.py    # → job_scatter.png
```

Both scripts read from `data/` and write the PNG next to themselves. The
Prela timings come from the bench runs above; the DuckDB baselines
(`data/job_duck.txt`, `data/duckdb_st.txt`) are checked in.

### Regenerate the DuckDB baseline + TPCH oracles

The capture scripts live under `rust/bench/`; the baselines they produce
are checked in as `rust/bench/data/{job_duck,duckdb_st}.txt`.

```bash
cd rust/bench
./run_job_duck.sh         # → data/job_duck.txt (canonical JOB, ST DuckDB)
./regen_tpch_oracles.sh   # → ../../oracles/tpch/Q{2,7,8,...}.txt
```

`run_job_duck.sh` runs each canonical JOB query (from `../../join-order-benchmark/`)
against a single-threaded DuckDB instance built from `~/projects/jobdata/parquet/`,
captures cold/warm timings, and writes them in the `Run Time (s): real …`
format that `plot_job.py` expects.

`regen_tpch_oracles.sh` rebuilds the 14 file-loaded TPCH oracles checked in
under `oracles/tpch/`, which the suites read (the ones
not inlined as string constants). It runs each canonical TPCH SQL against
`cache/tpch/*.parquet` with `PRAGMA threads=1` (so Float64 sums are
deterministic and the Q15 self-equality holds) and formats every decimal
field to `%.2f` to match the engine's float formatting.
