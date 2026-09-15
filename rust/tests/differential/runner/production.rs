//! Exercise the unmodified production Parquet -> regen -> cache -> query path.

use super::timing::Timing;
use duckdb::Connection;
use prela::job_queries::helpers::Result as QueryCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub type Answers = BTreeMap<String, Vec<QueryCell>>;

pub fn sql_database(sql: &str) -> Result<Connection, String> {
    let _time = Timing::new("load SQL fixture");
    let connection = Connection::open_in_memory().map_err(|e| e.to_string())?;
    // On these tiny fixtures, join-order search costs much more than execution.
    // Keep the other SQL optimizations enabled for the differential comparison.
    connection
        .execute_batch("SET disabled_optimizers='join_order';")
        .map_err(|e| format!("configure DuckDB optimizer: {e}"))?;
    connection
        .execute_batch(sql)
        .map_err(|e| format!("load SQL fixture: {e}"))?;
    Ok(connection)
}

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Result<Self, String> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "prela-pbt-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir(&path).map_err(|e| e.to_string())?;
        Ok(Self(path))
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn checked(command: &mut Command, stage: &str) -> Result<Output, String> {
    let _time = Timing::new(stage);
    let output = command.output().map_err(|e| format!("{stage}: {e}"))?;
    if std::env::var_os("PRELA_PROFILE").is_some() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    if !output.status.success() {
        return Err(format!(
            "{stage} failed ({}):\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output)
}

/// Prepare all production answers once per SQL fixture. Subprocesses release
/// production's deliberately leaked mmaps and database before the next case.
pub fn run(connection: &Connection) -> Result<Answers, String> {
    let _time = Timing::new("production pipeline");
    let workspace = Workspace::new()?;
    let parquet = workspace.0.join("parquet");
    let cache = workspace.0.join("cache");
    let answers = workspace.0.join("answers.json");
    std::fs::create_dir(&parquet).map_err(|e| e.to_string())?;
    let schema = &crate::queries::job::schema::SCHEMA;
    let export_time = Timing::new("Parquet export");
    for &entity in schema.tables {
        let table = schema
            .sql_table(entity)
            .ok_or_else(|| format!("missing SQL table for {entity}"))?;
        let path = parquet.join(format!("{table}.parquet"));
        // regen reads the standard JOB columns by position. Explicit projection
        // also makes replay independent of a fixture's physical column order.
        let fields = schema
            .entity_columns(entity)
            .iter()
            .map(|&column| {
                format!(
                    "\"{}\"",
                    schema.sql_column(column).unwrap().replace('"', "\"\"")
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        connection
            .execute_batch(&format!(
                "COPY (SELECT {fields} FROM \"{}\") TO '{}' (FORMAT PARQUET)",
                table.replace('"', "\"\""),
                path.to_str()
                    .ok_or("non-UTF-8 temporary path")?
                    .replace('\'', "''"),
            ))
            .map_err(|e| format!("export {table}: {e}"))?;
    }
    drop(export_time);
    checked(
        Command::new(env!("CARGO_BIN_EXE_regen"))
            .arg("job")
            .arg(&parquet)
            .arg(&cache),
        "regen job",
    )?;
    checked(
        Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
            .args([
                "--exact",
                "runner::production::query_worker",
                "--ignored",
                "--nocapture",
            ])
            .env("PRELA_PBT_WORKER_CACHE", &cache)
            .env("PRELA_PBT_WORKER_OUTPUT", &answers),
        "production query worker",
    )?;
    let bytes = std::fs::read(&answers).map_err(|e| format!("read production answers: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("decode production answers: {e}"))
}

#[test]
#[ignore = "subprocess entry point used by the differential harness"]
fn query_worker() {
    let cache = std::env::var_os("PRELA_PBT_WORKER_CACHE").expect("worker cache path");
    let output = std::env::var_os("PRELA_PBT_WORKER_OUTPUT").expect("worker output path");
    let load_time = Timing::new("Prela cache load");
    let db = Box::leak(Box::new(prela::job_schema::load(Path::new(&cache))));
    drop(load_time);
    let query_time = Timing::new("Prela all queries");
    let answers: Answers = prela::job_queries::typed_queries(db)
        .into_iter()
        .map(|(name, _, run)| {
            let _time = Timing::new(format!("Prela Q{name}"));
            (name.to_owned(), run(db))
        })
        .collect();
    drop(query_time);
    std::fs::write(output, serde_json::to_vec(&answers).unwrap()).unwrap();
}
