# Differential tests

The JOB differential suite generates small nullable IMDb databases, runs all
113 queries through both DuckDB and Prela, and compares their normalized
results. Run the commands below from the repository root.

The regular smoke run checks 100 generated databases:

```sh
cargo test --manifest-path rust/Cargo.toml --features test job_queries_match_generated_nullable_fixtures -- --nocapture
```

Set `HEGEL_TEST_CASES` to use a different limit. Each generated database is
checked against all 113 queries:

```sh
# Quick check: 10 generated databases
HEGEL_TEST_CASES=10 cargo test --manifest-path rust/Cargo.toml --features test job_queries_match_generated_nullable_fixtures -- --nocapture

# Longer check: 1,000 generated databases (use an optimized build)
HEGEL_TEST_CASES=1000 cargo test --release --manifest-path rust/Cargo.toml --features test job_queries_match_generated_nullable_fixtures -- --nocapture
```

One case runs all 113 query pairs. Large limits can therefore take hours in a
debug build; use `--release` for hundreds or thousands of cases.

The dedicated deep run is ignored by default and uses 10,000 cases unless
`HEGEL_TEST_CASES` overrides it:

```sh
HEGEL_TEST_CASES=10000 cargo test --release --manifest-path rust/Cargo.toml --features test differential_job_suite -- --ignored --nocapture
```

Cargo's test harness does not impose a wall-clock timeout. On Linux, wrap the
command with `timeout`; on macOS, use GNU coreutils' `gtimeout` if installed:

```sh
# Linux: stop after 10 minutes
timeout 10m env HEGEL_TEST_CASES=10000 cargo test --release --manifest-path rust/Cargo.toml --features test differential_job_suite -- --ignored --nocapture

# macOS: stop after 10 minutes
gtimeout 10m env HEGEL_TEST_CASES=10000 cargo test --release --manifest-path rust/Cargo.toml --features test differential_job_suite -- --ignored --nocapture
```

When a comparison fails, the test output identifies the query and writes a
reproduction to that query's `src/test/queries/job/q*/mismatch.sql` file.

## Generated database examples

`generated-databases.sql` contains 100 deterministic examples from the same
generator used by the differential suite. Each numbered block contains a full
JOB schema and its generated rows, wrapped in a transaction that rolls back so
the complete file can be executed in DuckDB. Remove the block's `ROLLBACK` to
keep that example loaded for interactive queries.

Regenerate the file from the repository root with:

```sh
cargo test --manifest-path rust/Cargo.toml --features test write_generated_job_databases -- --ignored --nocapture
```
