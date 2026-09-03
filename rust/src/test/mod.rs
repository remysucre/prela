//! Infrastructure for comparing a SQL query with the equivalent Prela query.
//!
//! The important architectural decision is that generation produces a neutral
//! [`Database`], not SQL rows or Prela relations directly.  The intended flow
//! is:
//!
//! 1. use a canonical Rust [`Schema`] with its value constraints;
//! 2. generate a tiny, constraint-respecting [`Database`], with NULL exercised
//!    only where the benchmark schema permits it and annotated row
//!    relationships expressed as dependent draws;
//! 3. adapt that one logical database to SQL, where absence is written as
//!    `NULL`, and to Prela, where absence is represented by a missing relation
//!    fact;
//! 4. execute both queries and compare normalized results.
//!
//! Generating once and adapting twice matters: independently generated SQL and
//! Prela fixtures could differ and make a result mismatch meaningless.
//!
//! Module responsibilities:
//!
//! - [`schema`] exposes canonical metadata plus the value-only constraints DSL.
//! - [`rules`] defines extensible constraints and their generation phases.
//! - [`generate`] builds the neutral in-memory databases.
//! - [`queries`] registers benchmark SQL against production Prela queries.
//! - [`result`] defines typed result cells and equality policies.
//! - [`sql`] renders neutral fixtures from canonical schema metadata.

/// Neutral logical values and the schema-directed Hegel generator.
pub mod generate;
/// Benchmark SQL and generated mismatch artifacts.
pub mod queries;
/// Converts generated database rows into Prela's typed relations, treating
/// `NULL` as a missing fact.
pub mod relations;
/// Typed query results and explicit row-order equality policies.
pub mod result;
/// Extensible constraints and the inline rule-syntax translator.
pub mod rules;
/// End-to-end DuckDB-versus-Prela differential runner.
pub mod runner;
/// Canonical schema metadata and the value-constraint overlay DSL.
pub mod schema;
/// Generic SQL fixture rendering from canonical schema metadata.
pub mod sql;
