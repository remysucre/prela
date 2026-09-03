# Differential tests

The JOB differential suite generates small nullable IMDb databases, runs all
113 queries through both DuckDB and Prela, and compares their normalized
results.

```sh
cargo test --manifest-path rust/Cargo.toml --features test --test differential \
  job_queries_match_generated_nullable_fixtures -- --nocapture
```

Run it from the repository root. One case runs all 113 query pairs; the default
is 100 cases. `HEGEL_TEST_CASES=<n>` changes that, and anything past a few
hundred wants `--release` — a debug build takes hours:

```sh
HEGEL_TEST_CASES=10 cargo test ...                        # quick check
HEGEL_TEST_CASES=10000 cargo test --release ...           # deep run
```

Cargo's test harness imposes no wall-clock timeout, so bound a deep run
yourself with `timeout` (GNU coreutils' `gtimeout` on macOS):

```sh
timeout 10m env HEGEL_TEST_CASES=10000 cargo test --release ...
```

Each case leaks the `&'static` Prela database it shreds — the engine requires
that lifetime — so memory grows linearly in the case count. It is a few KB per
case, which is why a 10,000-case run is fine and an unbounded one is not.

When a comparison fails, the test output identifies the query and writes a
reproduction to that query's
`tests/differential/queries/job/q*/mismatch.sql` file.

## What the suite does and does not cover

It covers query semantics and the `job_shred` transformation, which is the same
code `regen job` uses in production.

It does not cover the mapping *into* that shredder. Each caller writes its own:
`runner/job.rs` maps from the generated `Database`, and `bin/regen.rs` maps from
parquet column indices. Only the first is exercised here, so a wrong column
index on the regeneration side is invisible to all 113 queries at any case
count.

## Generated database examples

`generated-databases.sql` contains 100 deterministic examples from the same
generator. Each numbered block holds a full JOB schema and its rows, wrapped in
a transaction that rolls back so the whole file can be executed in DuckDB.
Remove a block's `ROLLBACK` to keep that example loaded for interactive
queries. Regenerate from the repository root with:

```sh
cargo test --manifest-path rust/Cargo.toml --features test --test differential \
  write_generated_job_databases -- --ignored --nocapture
```
