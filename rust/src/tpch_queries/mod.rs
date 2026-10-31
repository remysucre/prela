// TPC-H queries, in two variants:
//
//   idiomatic — direct algebraic renderings of the TPC-H queries
//   optimized — same algebra, hand-encoding the plans a stats-driven
//               optimizer (DuckDB's) would pick
//
// common.rs owns the oracles and the baseline (idiomatic) implementations;
// the optimized variant overlays only the queries it rewrites on the base registry.

pub mod common;
// common.rs's queries over the Parquet files, via the loader (feature `loaded`).
#[cfg(feature = "loaded")]
pub mod common_loaded;
pub mod idiomatic;
pub mod optimized;

pub use common::Entry;
