# JOB property tests against the production implementation

Hegel generates a small SQL fixture from the JOB schema in `queries/job/schema.rs`.
The fixture is loaded once into DuckDB. Every test case then follows this path:

1. Export the stored SQL tables to Parquet, using the standard JOB column order.
2. Run the existing `regen job` binary to produce the production binary cache.
3. Load that cache with `job_schema::load` and run the production JOB query plans.
4. Compare typed results with the 113 corresponding SQL queries in DuckDB.

`src/bin/regen.rs`, the production schema structs, cache loader, engine, and
TPC-H queries are unchanged from upstream. The JOB registry has a generic
output consumer: normal benchmarks still use `min_row` and return the same text;
tests retain NULL/integer/text values and adapt SQL projection order. There is
one implementation of each query plan, with no test-specific shredding or
replacement database schema.

The query worker is a subprocess of the test executable. Production intentionally
keeps its mmaps and loaded database for the process lifetime, so exiting the worker
reclaims them after every fixture. Temporary Parquet, cache, and result files are
removed after the worker returns, including on errors. This tests the actual disk
pipeline at the cost of file I/O and process startup on every case.

## Running

From the repository root:

```sh
# Fast checks and one generated case; also builds the regen binary.
HEGEL_TEST_CASES=1 cargo test --manifest-path rust/Cargo.toml --features test --test differential

# Default: 100 generated cases, each comparing all 113 JOB queries.
cargo test --manifest-path rust/Cargo.toml --features test --test differential

# Longer runs should use release mode.
HEGEL_TEST_CASES=1000 cargo test --release --manifest-path rust/Cargo.toml --features test --test differential
```

The optional `test` feature enables Hegel, DuckDB with Parquet support, JSON
transport for typed worker results, and the existing `regen` feature. Plain
`cargo build` retains upstream's dependency set and behavior.

## Failure replay

A query mismatch is saved as `queries/job/q*/mismatch.sql`, including the fixture
and both answers. Hegel shrinks the generated draws while retaining the failure.
To replay the saved SQL, or edit it and try again:

```sh
PRELA_REPLAY=path/to/mismatch.sql PRELA_QUERY=6a \
  cargo test --manifest-path rust/Cargo.toml --features test --test differential \
  replay_saved_job_fixture -- --ignored --nocapture
```

Omit `PRELA_QUERY` to compare every query. The worker entry point and the example
fixture writer are ignored tests used explicitly by the harness and tooling.

## Scope

This suite tests JOB using the production shredder's existing data conventions,
including positive, one-based SQL IDs. Generated fixtures use those conventions.
It does not provide the other branch's generic catalog-driven importer or TPC-H
import bindings. Unusual fixtures that production cannot ingest fail in the
production path rather than being remapped by an alternative shredder.

Generation currently uses one to three rows per table, schema-derived nullability
and references, and optional value rules such as text lengths, ranges, and finite
vocabularies. The generator's intermediate row model is only used to render SQL;
Prela always receives the values actually stored by DuckDB.
