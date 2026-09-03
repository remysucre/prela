// prela — the typed relational-algebra engine and its benchmark suites.
//
// The lib exists so the two binaries share one source of truth:
//   - `prela` (src/main.rs) runs the JOB / TPC-H suites over the struct
//     schemas (src/job_schema.rs, src/tpch_schema.rs);
//   - `regen` (src/bin/regen.rs, feature `regen`) rebuilds the binary
//     cache from parquet and verifies its outputs against those schemas'
//     `manifest()` lists.

#![recursion_limit = "256"]

// `#[derive(IntoQuery)]` (macros/) expands to `::prela::engine::..` paths so it works
// from any crate; this alias makes them resolve inside prela itself too.
extern crate self as prela;

pub mod cache;
pub mod engine;
pub mod format;
pub mod job_queries;
pub mod job_schema;
pub mod loader;
#[cfg(all(test, feature = "test"))]
pub(crate) mod test;
pub mod tpch_queries;
pub mod tpch_schema;

/// A registered query: (name, expected output, runner).
///
/// Runners take the loaded database, which `main` leaks to `&'static` so
/// that plans built from it carry no lifetime and the `ENTRIES` tables can
/// stay `const` arrays of fn pointers.
pub type Entry<D> = (&'static str, &'static str, fn(&'static D) -> String);
