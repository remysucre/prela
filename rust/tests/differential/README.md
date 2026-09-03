# PBT for Prela

This is the differential testing harness for Prela. It's probably buggy, for which I apologize in advance.
It generates small databases using [Hegel](https://hegel.dev), a new PBT tool
written in Rust. These databases are *schema-directed*, meaning that their structure is dictated by the shape of an
existing schema, in this case the JOB schema. It's also possible to add some additional refinement (e.g., specify 
that a `String` should be between 1 and 7 characters).

Provided this information, Hegel generates a database in a generic representation. This representation is then
converted to both SQL and Prela-style databases. Each of the 113 job queries run against each generated database
in both SQL and Prela; any mismatch is logged to the relevant query subdirectory of the form `tests/differential/queries/job/q*/mismatch.sql`, with Hegel automatically shrinking the error-producing example.

You can run it like this:

```sh
cargo test --manifest-path rust/Cargo.toml --features test --test differential \
  prela_matches_duckdb_on_generated_job_databases -- --nocapture
```

It should be run from the repository root. One case runs all 113 query pairs; the default
is 100 cases. You can change this using `HEGEL_TEST_CASES=<n>`, and anything past a few
hundred warrants `--release`, as the process can become time-consuming:

```sh
HEGEL_TEST_CASES=10 cargo test ...                       
HEGEL_TEST_CASES=10000 cargo test --release ...          
```

Cargo's test harness imposes no wall-clock timeout, so to bound the
time the run takes you can do this:

```sh
timeout 10m env HEGEL_TEST_CASES=10000 cargo test --release ...
```

### What now?

The JOB database does not reliably produce errors, so either Prela is perfect or it's not exhaustive enough. We should look at other (nullable) benchmarks, implement their schemas, and do the same thing we do here. We should also make the generation "smarter," although there's really no upper limit on how far we can go with this, and at some point it kind of turns into a paper in and of itself. The obvious thing to do is to make the databases both schema *and* query-directed---injecting `NULLs` specifically in columns that a given query touches, generating data that satisfies a particular predicate, etc.
