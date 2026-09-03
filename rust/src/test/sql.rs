//! Generic SQL fixture rendering for generated differential-test databases.
//!
//! SQL spelling and scalar types come from [`Schema`] metadata. Benchmark
//! modules therefore do not need their own `CREATE TABLE`/`INSERT` renderer.

use super::generate::{Cell, Database};
use super::schema::{ColumnId, ID_FIELD, ScalarKind, Schema};
use std::fmt::Write;

/// Render a complete generated database as portable DuckDB SQL.
///
/// Types, nullability, primary keys, and available foreign-key targets come
/// from the canonical schema. Partial hand-built fixtures omit references to
/// tables they intentionally did not include.
pub fn to_sql(schema: &Schema, database: &Database) -> String {
    let mut out = String::new();
    for table in &database.tables {
        let table_name = schema
            .sql_table(table.entity)
            .expect("generated entity must have a canonical SQL table mapping");
        let columns = entity_columns(schema, table.entity);
        let mut definitions = Vec::new();
        for &column in &columns {
            let name = schema
                .sql_column(column)
                .expect("generated column must have a canonical SQL name");
            let ty = sql_type(schema, column);
            let required = if schema.is_nullable(column) {
                ""
            } else {
                " NOT NULL"
            };
            definitions.push(format!("{name} {ty}{required}"));
        }
        if let Some(key) = schema.primary_key(table.entity) {
            definitions.push(format!(
                "PRIMARY KEY ({})",
                key.iter()
                    .map(|column| schema.sql_column(*column).expect("key has SQL name"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        for column in schema
            .columns
            .iter()
            .filter(|column| column.entity == table.entity)
        {
            if let ScalarKind::ForeignKey(parent) = column.kind {
                // Legacy schemas encoded typed ids but did not carry physical
                // key metadata. Only emit a SQL FK when the canonical target
                // key is available to back it.
                if schema.primary_key(parent).is_none() {
                    continue;
                }
                if !database.tables.iter().any(|table| table.entity == parent) {
                    continue;
                }
                let parent_table = schema.sql_table(parent).expect("FK target has SQL table");
                let parent_column = schema
                    .sql_column(ColumnId::new(parent, ID_FIELD))
                    .expect("typed FK target has implicit id");
                definitions.push(format!(
                    "FOREIGN KEY ({}) REFERENCES {parent_table}({parent_column})",
                    column.field
                ));
            }
        }
        writeln!(out, "CREATE TABLE {table_name} (").unwrap();
        writeln!(out, "  {}", definitions.join(",\n  ")).unwrap();
        out.push_str(");\n");

        if table.rows.is_empty() {
            continue;
        }
        writeln!(out, "INSERT INTO {table_name} VALUES").unwrap();
        for (row_index, row) in table.rows.iter().enumerate() {
            let values = columns
                .iter()
                .map(|column| sql_cell(row.cells.get(column).expect("generated cell")))
                .collect::<Vec<_>>()
                .join(", ");
            let end = if row_index + 1 == table.rows.len() {
                ";"
            } else {
                ","
            };
            writeln!(out, "({values}){end}").unwrap();
        }
    }
    out
}

/// Return logical columns in their stable SQL tuple order.
fn entity_columns(schema: &Schema, entity: &'static str) -> Vec<ColumnId> {
    let mut columns = Vec::new();
    if !schema.has_key(entity) {
        columns.push(ColumnId::new(entity, ID_FIELD));
    }
    columns.extend(
        schema
            .columns
            .iter()
            .filter(|column| column.entity == entity)
            .map(|column| ColumnId::new(column.entity, column.field)),
    );
    columns
}

fn sql_type(schema: &Schema, column: ColumnId) -> String {
    if column.field == ID_FIELD {
        return "BIGINT".to_owned();
    }
    match schema.column(column).expect("generated schema column").kind {
        ScalarKind::Str => "VARCHAR".to_owned(),
        ScalarKind::I64 | ScalarKind::ForeignKey(_) => "BIGINT".to_owned(),
    }
}

fn sql_cell(cell: &Cell) -> String {
    match cell {
        Cell::Null => "NULL".to_owned(),
        Cell::I64(value) => value.to_string(),
        Cell::Text(value) => format!("'{}'", value.replace('\'', "''")),
    }
}
