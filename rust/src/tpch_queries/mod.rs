// TPC-H queries, in two variants:
//
//   idiomatic — direct algebraic ports of the reference queries
//   optimized — same algebra, hand-encoding the plans a stats-driven
//               optimizer (DuckDB's) would pick
//
// common.rs owns the oracles and the baseline (idiomatic) implementations;
// the optimized variant overlays only the queries it rewrites on the base registry.

pub mod common;
pub mod idiomatic;
pub mod optimized;

pub use common::Entry;
