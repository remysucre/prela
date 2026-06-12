// prela — the typed relational-algebra engine and its benchmark suites.
//
// The lib exists so the two binaries share one source of truth:
//   - `prela` (src/main.rs) runs the JOB / TPC-H suites over the typed
//     `schema!` declarations (src/job_schema.rs, src/tpch_schema.rs);
//   - `regen` (src/bin/regen.rs, feature `regen`) rebuilds the binary
//     cache from parquet and verifies its outputs against the schemas'
//     generated manifests.

pub mod cache;
pub mod engine;
pub mod format;
pub mod job_schema;
pub mod queries;
pub mod schema;
pub mod tpch;
pub mod tpch_schema;

/// A registered query: (name, expected output, runner). Runners take no
/// data argument — they read the schema's global `OnceLock` store.
pub type Entry = (&'static str, &'static str, fn() -> String);
