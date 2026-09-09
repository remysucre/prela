//! Prela — the typed relational-algebra engine and its benchmark suites.
//!
//! Prela is an embedded query language focused on compositionality and
//! control. It's implemented as a library of *query combinators* (think
//! [parser combinators]), so queries freely intermix with ordinary Rust
//! code instead of living in a separate string or macro DSL. Unlike
//! almost all SQL databases, Prela ships no query optimizer: a query
//! *is* its own query plan, executed exactly as written, giving the
//! caller complete control over join ordering, operator pushdown,
//! materialization, and the choice of physical data structures.
//!
//! ```rust,ignore
//! // every movie made after 2008, mapped to its title
//! movie.with(production_year.gt(2008)).select(title)
//! ```
//!
//! Every combinator lives on [`engine::QueryExt`] — start there for the
//! full operator list.
//!
//! [parser combinators]: https://en.wikipedia.org/wiki/Parser_combinator
//!
//! The lib exists so the two binaries share one source of truth:
//!   - `prela` (src/main.rs) runs the JOB / TPC-H suites over the struct
//!     schemas (src/job_schema.rs, src/tpch_schema.rs);
//!   - `regen` (src/bin/regen.rs, feature `regen`) rebuilds the binary
//!     cache from parquet and verifies its outputs against those schemas'
//!     `manifest()` lists.
//!
//! ## Crate Features
//!
//! - `regen` — off by default. Only needed to build the `regen` binary,
//!   which pulls in the `parquet`/`arrow` dependencies to rebuild the
//!   binary cache. The library itself and the `prela` binary need
//!   nothing beyond the default dependencies.
//!
//! ## Rust Version
//!
//! Requires at least Rust 1.85, the minimum version that understands
//! this crate's `edition = "2024"` setting in `Cargo.toml`. This is a
//! hard floor the edition itself imposes, not a maintainer-tested MSRV
//! guarantee — a newer minimum may be required if the code ends up
//! depending on something stabilized after 1.85.

// `#[derive(IntoQuery)]` (macros/) expands to `::prela::engine::..` paths so it works
// from any crate; this alias makes them resolve inside prela itself too.
extern crate self as prela;

pub mod cache;
pub mod engine;
pub mod format;
pub mod job_queries;
pub mod job_schema;
pub mod loader;
pub mod tpch_queries;
pub mod tpch_schema;

/// A registered query: (name, expected output, runner).
///
/// Runners take the loaded database, which `main` leaks to `&'static` so
/// that plans built from it carry no lifetime and the TPC-H tables can stay
/// `const` arrays of fn pointers. JOB's entries are the same tuple with a
/// boxed closure over the once-destructured columns (`job_queries::Entry`).
pub type Entry<D> = (&'static str, &'static str, fn(&'static D) -> String);
