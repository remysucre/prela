//! Generic SQL fixture rendering for generated differential-test databases.
//!
//! SQL spelling and scalar types come from [`Schema`] metadata. Benchmark
//! modules therefore do not need their own `CREATE TABLE`/`INSERT` renderer.

use super::generate::{Cell, Database, RowId};
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
        let columns = schema.entity_columns(table.entity);
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
                let Some(parent_key) = schema.reference_key(parent) else {
                    continue;
                };
                if !database.tables.iter().any(|table| table.entity == parent) {
                    continue;
                }
                let parent_table = schema.sql_table(parent).expect("FK target has SQL table");
                let parent_column = schema
                    .sql_column(parent_key)
                    .expect("typed FK target has a SQL column");
                let child_column = schema
                    .sql_column(ColumnId::new(column.entity, column.field))
                    .expect("declared column has SQL name");
                definitions.push(format!(
                    "FOREIGN KEY ({child_column}) REFERENCES {parent_table}({parent_column})"
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
                .map(|column| {
                    if column.field == ID_FIELD {
                        return (row.id.index + 1).to_string();
                    }
                    sql_cell(
                        schema,
                        database,
                        row.cells.get(column).expect("generated cell"),
                    )
                })
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

fn sql_type(schema: &Schema, column: ColumnId) -> String {
    if column.field == ID_FIELD {
        return "BIGINT".to_owned();
    }
    match schema.column(column).expect("generated schema column").kind {
        ScalarKind::Str => "VARCHAR".to_owned(),
        ScalarKind::I64 | ScalarKind::ForeignKey(_) => "BIGINT".to_owned(),
    }
}

fn sql_cell(schema: &Schema, database: &Database, cell: &Cell) -> String {
    match cell {
        Cell::Null => "NULL".to_owned(),
        Cell::I64(value) => value.to_string(),
        Cell::Text(value) => format!("'{}'", value.replace('\'', "''")),
        Cell::Reference(target) => sql_reference(schema, database, *target),
    }
}

fn sql_reference(schema: &Schema, database: &Database, target: RowId) -> String {
    let table = database
        .tables
        .iter()
        .find(|table| table.entity == target.entity)
        .expect("generated reference names an existing table");
    let row = table
        .rows
        .iter()
        .find(|row| row.id == target)
        .expect("generated reference names an existing row");
    let key = schema
        .reference_key(target.entity)
        .expect("validated reference target has one key column");
    if key.field == ID_FIELD {
        return (row.id.index + 1).to_string();
    }
    sql_cell(
        schema,
        database,
        row.cells
            .get(&key)
            .expect("generated target row has its key"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{Row, Table};
    use crate::schema::{ColumnMeta, PrimaryKey, TableMeta};
    use std::collections::BTreeMap;

    static COLUMNS: &[ColumnMeta] = &[
        ColumnMeta::new("Parent", "key", ScalarKind::I64),
        ColumnMeta::new("Child", "parent", ScalarKind::ForeignKey("Parent")),
    ];
    static SQL: &[TableMeta] = &[
        TableMeta::new("Parent", "parents", None),
        TableMeta::new("Child", "children", Some("id")),
    ];
    static REQUIRED: &[ColumnId] = &[ColumnId::new("Child", "parent")];
    static KEYS: &[PrimaryKey] = &[
        PrimaryKey::new("Parent", &[ColumnId::new("Parent", "key")]),
        PrimaryKey::new("Child", &[ColumnId::new("Child", ID_FIELD)]),
    ];
    static SCHEMA: Schema = Schema::canonical(&["Parent", "Child"], COLUMNS, SQL, REQUIRED, KEYS);

    #[test]
    fn references_render_the_target_rows_explicit_key() {
        SCHEMA.validate().unwrap();
        let database = Database {
            tables: vec![
                Table {
                    entity: "Parent",
                    rows: vec![Row {
                        id: RowId {
                            entity: "Parent",
                            index: 0,
                        },
                        cells: BTreeMap::from([(ColumnId::new("Parent", "key"), Cell::I64(42))]),
                    }],
                },
                Table {
                    entity: "Child",
                    rows: vec![Row {
                        id: RowId {
                            entity: "Child",
                            index: 0,
                        },
                        cells: BTreeMap::from([(
                            ColumnId::new("Child", "parent"),
                            Cell::Reference(RowId {
                                entity: "Parent",
                                index: 0,
                            }),
                        )]),
                    }],
                },
            ],
        };

        let sql = to_sql(&SCHEMA, &database);
        assert!(sql.contains("FOREIGN KEY (parent) REFERENCES parents(key)"));
        assert!(sql.contains("INSERT INTO children VALUES\n(1, 42);"));
        duckdb::Connection::open_in_memory()
            .unwrap()
            .execute_batch(&sql)
            .unwrap();
    }
}
