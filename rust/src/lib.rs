// prela — the typed relational-algebra engine and its benchmark suites.
//
// The lib exists so the two binaries share one source of truth:
//   - `prela` (src/main.rs) runs the JOB / TPC-H suites over the typed
//     `schema!` declarations (src/job_schema.rs, src/tpch_schema.rs);
//   - `regen` (src/bin/regen.rs, feature `regen`) rebuilds the binary
//     cache from parquet and verifies its outputs against the schemas'
//     generated manifests.

// `schema_simpl!`'s shared field walker costs ~2 expansion frames per field,
// and the manifest walks a whole schema in one chain — a TPC-H-sized schema
// (~60 fields) would overflow the default limit of 128.
#![recursion_limit = "256"]

pub mod cache;
pub mod engine;
pub mod format;
pub mod job_schema;
pub mod queries;
pub mod schema;
pub mod schema_proc;
pub mod schema_simpl;
pub mod tpch;
pub mod tpch_schema;
pub mod tpch_schema_proc;

/// A registered query: (name, expected output, runner). Runners take no
/// data argument — they read the schema's global `OnceLock` store.
pub type Entry = (&'static str, &'static str, fn() -> String);
