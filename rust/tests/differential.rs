#![recursion_limit = "256"]

//! Infrastructure for comparing a SQL query with the equivalent Prela query.
//!
//! Hegel generates a SQL fixture and loads it once into DuckDB. The fixture is
//! exported to Parquet, processed by the unmodified production `regen` binary,
//! and loaded by the production JOB schema. The same production queries run in
//! a worker process and return typed cells for comparison with SQL. Exiting the
//! worker releases production's lifetime-long cache allocations after each case.

/// Neutral logical values and the schema-directed Hegel generator.
#[path = "differential/generate.rs"]
pub mod generate;
/// Benchmark SQL and generated mismatch artifacts.
#[path = "differential/queries/mod.rs"]
pub mod queries;
/// Typed query results and explicit row-order equality policies.
#[path = "differential/result.rs"]
pub mod result;
#[path = "differential/result_tests.rs"]
mod result_tests;
/// Extensible constraints and the inline rule-syntax translator.
#[path = "differential/rules.rs"]
pub mod rules;
/// End-to-end DuckDB-versus-Prela differential runner.
#[path = "differential/runner.rs"]
pub mod runner;
/// Canonical schema metadata and the value-constraint overlay DSL.
#[path = "differential/schema.rs"]
pub mod schema;
/// Generic SQL fixture rendering from canonical schema metadata.
#[path = "differential/sql.rs"]
pub mod sql;
