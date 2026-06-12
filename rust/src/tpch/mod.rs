// TPC-H queries, in three variants (originally ported from the julia-engine branch):
//
//   idiomatic — direct algebraic ports of the Julia queries
//   optimized — same algebra, hand-encoding the plans a stats-driven
//               optimizer (DuckDB's) would pick
//   ddbcheat  — bounds what hand-rolled SQL-shaped code can do; skips the
//               algebra where a raw loop or dense array wins
//
// common.rs owns the oracles and the baseline (idiomatic) implementations;
// each variant overlays only the queries it rewrites on the base registry.

pub mod common;
pub mod ddbcheat;
pub mod idiomatic;
pub mod optimized;

pub use common::Entry;
