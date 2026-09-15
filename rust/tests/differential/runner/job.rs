//! End-to-end Join Order Benchmark differential execution.

use super::production::{self, Answers};
use super::run_sql;
use crate::queries::job::Query;
use crate::result::{ResultCell, ResultSet};
use duckdb::Connection;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Comparison {
    pub query: Query,
    pub columns: Vec<String>,
    pub sql: ResultSet,
    pub prela: ResultSet,
    fixture: String,
}

impl Comparison {
    pub fn equivalent(&self) -> bool {
        self.query
            .order
            .equivalent(&self.columns, &self.sql, &self.prela)
    }

    pub fn failure(&self) -> String {
        format!(
            "JOB Q{} differs\nMismatch written to {}\n\nColumns: {:?}\n\nSQL result:\n{}\n\nPrela result:\n{}\n\nReproduction SQL:\n{}\n{}",
            self.query.name,
            self.query.mismatch_path().display(),
            self.columns,
            self.sql,
            self.prela,
            self.fixture,
            self.query.sql,
        )
    }

    pub fn write_mismatch(&self) -> Result<PathBuf, String> {
        let path = self.query.mismatch_path();
        std::fs::write(&path, self.mismatch_sql())
            .map_err(|error| format!("write {}: {error}", path.display()))?;
        Ok(path)
    }

    fn mismatch_sql(&self) -> String {
        format!(
            "-- Generated JOB Q{} SQL/Prela mismatch.\n-- SQL result:\n{}\n-- Prela result:\n{}\n-- Columns: {:?}\n{}\n{}",
            self.query.name,
            sql_comment(&self.sql),
            sql_comment(&self.prela),
            self.columns,
            self.fixture,
            self.query.sql,
        )
    }
}

/// One authoritative SQL fixture, exported through production regeneration.
pub struct PreparedJob {
    connection: Connection,
    answers: Answers,
    fixture: String,
}

impl PreparedJob {
    /// Also accepts saved mismatch.sql files for replay.
    pub fn from_sql(fixture: &str) -> Result<Self, String> {
        let connection = production::sql_database(fixture)?;
        let answers = production::run(&connection)?;
        Ok(Self {
            connection,
            answers,
            fixture: fixture.to_owned(),
        })
    }

    pub fn compare(&self, query: Query) -> Result<Comparison, String> {
        let (columns, sql) = run_sql(&self.connection, query.name, query.sql, query.order)?;
        let prela = run_prela(query.name, &self.answers)?;
        validate_result_width(query, &columns, &prela)?;
        let comparison = Comparison {
            query,
            columns,
            sql,
            prela,
            fixture: self.fixture.clone(),
        };
        if !comparison.equivalent() {
            comparison.write_mismatch()?;
        }
        Ok(comparison)
    }
}

fn run_prela(name: &str, answers: &Answers) -> Result<ResultSet, String> {
    use prela::job_queries::helpers::Result as QueryResult;

    let row = answers
        .get(name)
        .ok_or_else(|| format!("unknown JOB query {name}"))?
        .iter()
        .cloned()
        .map(|cell| match cell {
            QueryResult::Null => ResultCell::Null,
            QueryResult::Integer(value) => ResultCell::Integer(i128::from(value)),
            QueryResult::Text(value) => ResultCell::Text(value),
        });
    Ok(ResultSet::from_rows([row.collect()]))
}

fn validate_result_width(
    query: Query,
    columns: &[String],
    result: &ResultSet,
) -> Result<(), String> {
    if let Some((row, actual)) = result
        .rows
        .iter()
        .enumerate()
        .find_map(|(row, cells)| (cells.len() != columns.len()).then_some((row, cells.len())))
    {
        return Err(format!(
            "JOB Q{} Prela row {row} has {actual} columns; expected {} ({columns:?})",
            query.name,
            columns.len(),
        ));
    }
    Ok(())
}

fn sql_comment(output: &ResultSet) -> String {
    output
        .to_string()
        .lines()
        .map(|line| format!("-- {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{Cell, Database, Row, RowId, Table, generator, sql_generator};
    use crate::queries::job as queries;
    use crate::queries::job::schema;
    use crate::result::ResultCell;
    use crate::schema::ColumnId;
    use crate::sql::to_sql;
    use hegel::{HealthCheck, TestCase};
    use std::collections::BTreeMap;
    use std::fs::File;
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};

    struct ExampleFile {
        file: File,
        count: usize,
    }

    static EXAMPLE_FILE: OnceLock<Mutex<ExampleFile>> = OnceLock::new();

    fn compare(database: &Database, query: Query) -> Result<Comparison, String> {
        PreparedJob::from_sql(&to_sql(&schema::SCHEMA, database))?.compare(query)
    }

    fn row(
        entity: &'static str,
        id: i64,
        values: impl IntoIterator<Item = (&'static str, Cell)>,
    ) -> Row {
        let mut cells = BTreeMap::new();
        cells.extend(
            schema::SCHEMA
                .columns
                .iter()
                .filter(|column| column.entity == entity)
                .map(|column| (ColumnId::new(entity, column.field), Cell::Null)),
        );
        cells.extend(values.into_iter().map(|(field, value)| {
            let column = ColumnId::new(entity, field);
            let value = match (
                schema::SCHEMA.column(column).map(|column| column.kind),
                value,
            ) {
                (Some(crate::schema::ScalarKind::ForeignKey(parent)), Cell::I64(id)) => {
                    Cell::Reference(RowId {
                        entity: parent,
                        index: usize::try_from(id - 1).expect("fixture ids are positive"),
                    })
                }
                (_, value) => value,
            };
            (column, value)
        }));
        Row {
            id: RowId {
                entity,
                index: usize::try_from(id - 1).expect("fixture ids are positive"),
            },
            cells,
        }
    }

    fn text(value: &str) -> Cell {
        Cell::Text(value.to_owned())
    }

    fn result_text(value: &str) -> ResultCell {
        ResultCell::Text(value.to_owned())
    }

    fn fixture() -> Database {
        let mut database = Database {
            tables: vec![
                Table {
                    entity: "CompanyName",
                    rows: vec![
                        row(
                            "CompanyName",
                            1,
                            [
                                ("name", text("German Studio")),
                                ("country_code", text("[de]")),
                            ],
                        ),
                        row(
                            "CompanyName",
                            2,
                            [
                                ("name", text("Russian Studio")),
                                ("country_code", text("[ru]")),
                            ],
                        ),
                        row("CompanyName", 3, [("name", text("Unknown Studio"))]),
                    ],
                },
                Table {
                    entity: "CompanyType",
                    rows: vec![row(
                        "CompanyType",
                        1,
                        [("kind", text("production companies"))],
                    )],
                },
                Table {
                    entity: "InfoType",
                    rows: vec![row("InfoType", 1, [("info", text("countries"))])],
                },
                Table {
                    entity: "Keyword",
                    rows: vec![
                        row("Keyword", 1, [("keyword", text("character-name-in-title"))]),
                        row(
                            "Keyword",
                            2,
                            [("keyword", text("marvel-cinematic-universe"))],
                        ),
                        row("Keyword", 3, [("keyword", text("hero-sequel"))]),
                    ],
                },
                Table {
                    entity: "KindType",
                    rows: vec![row("KindType", 1, [("kind", text("movie"))])],
                },
                Table {
                    entity: "Name",
                    rows: vec![
                        row("Name", 1, [("name", text("Downey Jr., Robert"))]),
                        row("Name", 2, [("name", text("Other Person"))]),
                    ],
                },
                Table {
                    entity: "RoleType",
                    rows: vec![row("RoleType", 1, [("role", text("actor"))])],
                },
                Table {
                    entity: "CharName",
                    rows: vec![row("CharName", 1, [("name", text("Voice Character"))])],
                },
                Table {
                    entity: "Title",
                    rows: vec![
                        row(
                            "Title",
                            1,
                            [("title", text("Character Film")), ("kind_id", Cell::I64(1))],
                        ),
                        row(
                            "Title",
                            2,
                            [
                                ("title", text("Bulgaria Sequel")),
                                ("kind_id", Cell::I64(1)),
                                ("production_year", Cell::I64(2011)),
                            ],
                        ),
                        row(
                            "Title",
                            3,
                            [
                                ("title", text("Marvel Film")),
                                ("kind_id", Cell::I64(1)),
                                ("production_year", Cell::I64(2012)),
                            ],
                        ),
                        row(
                            "Title",
                            4,
                            [
                                ("title", text("Russian Voice")),
                                ("kind_id", Cell::I64(1)),
                                ("production_year", Cell::I64(2006)),
                            ],
                        ),
                        row(
                            "Title",
                            5,
                            [("title", text("Null Year")), ("kind_id", Cell::I64(1))],
                        ),
                    ],
                },
                Table {
                    entity: "CastInfo",
                    rows: vec![
                        row(
                            "CastInfo",
                            1,
                            [
                                ("person_id", Cell::I64(1)),
                                ("movie_id", Cell::I64(3)),
                                ("role_id", Cell::I64(1)),
                            ],
                        ),
                        row(
                            "CastInfo",
                            2,
                            [
                                ("person_id", Cell::I64(2)),
                                ("movie_id", Cell::I64(4)),
                                ("person_role_id", Cell::I64(1)),
                                ("note", text("(voice) (uncredited)")),
                                ("role_id", Cell::I64(1)),
                            ],
                        ),
                    ],
                },
                Table {
                    entity: "MovieCompanies",
                    rows: vec![
                        row(
                            "MovieCompanies",
                            1,
                            [
                                ("movie_id", Cell::I64(1)),
                                ("company_id", Cell::I64(1)),
                                ("company_type_id", Cell::I64(1)),
                            ],
                        ),
                        row(
                            "MovieCompanies",
                            2,
                            [
                                ("movie_id", Cell::I64(4)),
                                ("company_id", Cell::I64(2)),
                                ("company_type_id", Cell::I64(1)),
                            ],
                        ),
                        row(
                            "MovieCompanies",
                            3,
                            [
                                ("movie_id", Cell::I64(5)),
                                ("company_id", Cell::I64(3)),
                                ("company_type_id", Cell::I64(1)),
                            ],
                        ),
                    ],
                },
                Table {
                    entity: "MovieInfo",
                    rows: vec![row(
                        "MovieInfo",
                        1,
                        [
                            ("movie_id", Cell::I64(2)),
                            ("info_type_id", Cell::I64(1)),
                            ("info", text("Bulgaria")),
                        ],
                    )],
                },
                Table {
                    entity: "MovieKeyword",
                    rows: vec![
                        row(
                            "MovieKeyword",
                            1,
                            [("movie_id", Cell::I64(1)), ("keyword_id", Cell::I64(1))],
                        ),
                        row(
                            "MovieKeyword",
                            2,
                            [("movie_id", Cell::I64(2)), ("keyword_id", Cell::I64(3))],
                        ),
                        row(
                            "MovieKeyword",
                            3,
                            [("movie_id", Cell::I64(3)), ("keyword_id", Cell::I64(2))],
                        ),
                    ],
                },
            ],
        };
        for &entity in schema::SCHEMA.tables {
            if !database.tables.iter().any(|table| table.entity == entity) {
                database.tables.push(Table {
                    entity,
                    rows: Vec::new(),
                });
            }
        }
        database
    }

    fn set_null(database: &mut Database, entity: &'static str, row: usize, field: &'static str) {
        database
            .tables
            .iter_mut()
            .find(|table| table.entity == entity)
            .unwrap()
            .rows[row]
            .cells
            .insert(ColumnId::new(entity, field), Cell::Null);
    }

    #[test]
    fn all_queries_match_on_nonempty_nullable_fixture() {
        let database = fixture();
        let expected = [
            ("2a", vec![result_text("Character Film")]),
            ("3b", vec![result_text("Bulgaria Sequel")]),
            (
                "6a",
                vec![
                    result_text("marvel-cinematic-universe"),
                    result_text("Downey Jr., Robert"),
                    result_text("Marvel Film"),
                ],
            ),
            (
                "10a",
                vec![result_text("Voice Character"), result_text("Russian Voice")],
            ),
        ];

        for (name, expected_row) in expected {
            let comparison = compare(&database, queries::get(name).unwrap()).unwrap();
            assert!(comparison.equivalent(), "{}", comparison.failure());
            assert_eq!(comparison.sql.rows, vec![expected_row]);
        }
    }

    #[test]
    fn nullable_predicates_and_joins_have_sql_semantics() {
        let cases = [
            ("2a", "CompanyName", 0, "country_code", 1),
            ("3b", "Title", 1, "production_year", 1),
            ("6a", "Title", 2, "production_year", 3),
            ("10a", "CastInfo", 1, "note", 2),
            ("10a", "CastInfo", 1, "person_role_id", 2),
        ];

        for (name, entity, row, field, width) in cases {
            let mut database = fixture();
            set_null(&mut database, entity, row, field);
            let comparison = compare(&database, queries::get(name).unwrap()).unwrap();
            assert!(comparison.equivalent(), "{}", comparison.failure());
            assert_eq!(comparison.sql.rows, vec![vec![ResultCell::Null; width]]);
        }
    }

    #[test]
    fn stored_sql_values_are_authoritative_and_replayable() {
        let mut sql = to_sql(&schema::SCHEMA, &fixture());
        sql.push_str("\nUPDATE title SET title = 'edited in SQL';\n");
        let prepared = PreparedJob::from_sql(&sql).unwrap();
        let query = queries::get("2a").unwrap();
        let comparison = prepared.compare(query).unwrap();
        assert!(comparison.equivalent(), "{}", comparison.failure());
        assert_eq!(
            comparison.prela.rows,
            vec![vec![result_text("edited in SQL")]]
        );
        let replay = PreparedJob::from_sql(&comparison.mismatch_sql())
            .unwrap()
            .compare(query)
            .unwrap();
        assert_eq!(replay.prela, comparison.prela);
        assert_eq!(replay.sql, comparison.sql);
    }

    #[test]
    fn worker_preserves_text_that_looks_like_formatted_output() {
        let mut sql = to_sql(&schema::SCHEMA, &fixture());
        let title = "NULL || \"quoted\"\n café";
        sql.push_str(&format!(
            "\nUPDATE title SET title = '{}';\n",
            title.replace('\'', "''")
        ));
        let prepared = PreparedJob::from_sql(&sql).unwrap();
        let comparison = prepared.compare(queries::get("2a").unwrap()).unwrap();
        assert!(comparison.equivalent(), "{}", comparison.failure());
        assert_eq!(comparison.prela.rows, vec![vec![result_text(title)]]);
    }

    #[test]
    #[ignore = "set PRELA_REPLAY to a saved SQL fixture; PRELA_QUERY optionally selects one JOB query"]
    fn replay_saved_job_fixture() {
        let path =
            std::env::var_os("PRELA_REPLAY").expect("set PRELA_REPLAY to a SQL fixture path");
        let fixture = std::fs::read_to_string(path).expect("read SQL fixture");
        let prepared = PreparedJob::from_sql(&fixture).unwrap();
        let selected = std::env::var("PRELA_QUERY").ok();
        if let Some(name) = &selected {
            assert!(queries::get(name).is_some(), "unknown JOB query {name}");
        }
        for &query in queries::all()
            .iter()
            .filter(|query| selected.as_deref().is_none_or(|name| name == query.name))
        {
            let comparison = prepared.compare(query).unwrap();
            assert!(comparison.equivalent(), "{}", comparison.failure());
        }
    }

    #[test]
    fn production_cache_round_trip_is_repeatable() {
        let sql = to_sql(&schema::SCHEMA, &fixture());
        let a = PreparedJob::from_sql(&sql).unwrap();
        let b = PreparedJob::from_sql(&sql).unwrap();
        assert_eq!(a.answers, b.answers);
        for &query in queries::all() {
            let comparison = a.compare(query).unwrap();
            assert!(comparison.equivalent(), "{}", comparison.failure());
        }
    }

    #[hegel::test(
        test_cases = 100,
        suppress_health_check = [HealthCheck::TooSlow]
    )]
    fn prela_matches_duckdb_on_generated_job_databases(tc: TestCase) {
        let fixture = tc.draw(sql_generator(&schema::SCHEMA));
        let prepared = PreparedJob::from_sql(&fixture).unwrap();
        for &query in queries::all() {
            let comparison = prepared.compare(query).unwrap();
            assert!(comparison.equivalent(), "{}", comparison.failure());
        }
    }

    #[hegel::test(
        test_cases = 100,
        derandomize = true,
        database = None,
        suppress_health_check = [HealthCheck::TooSlow]
    )]
    #[ignore = "run explicitly to regenerate generated-databases.sql"]
    fn write_generated_job_databases(tc: TestCase) {
        let database = tc.draw(generator(&schema::SCHEMA));
        let output = EXAMPLE_FILE.get_or_init(|| {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/differential/generated-databases.sql");
            let mut file = File::create(path).expect("create generated database examples");
            writeln!(
                file,
                "-- 100 deterministic examples produced by the JOB differential-test generator.\n\
                 -- Each block is isolated in a transaction so this entire file can be run at once.\n\
                 -- Remove its ROLLBACK to keep a particular database for interactive queries.\n"
            )
            .unwrap();
            Mutex::new(ExampleFile { file, count: 0 })
        });

        let mut output = output.lock().unwrap();
        output.count += 1;
        let number = output.count;
        writeln!(
            output.file,
            "-- ============================================================================\n\
             -- Generated database {number:03}/100\n\
             -- ============================================================================\n\
             BEGIN TRANSACTION;"
        )
        .unwrap();
        write!(output.file, "{}", to_sql(&schema::SCHEMA, &database)).unwrap();
        writeln!(output.file, "ROLLBACK;\n").unwrap();
    }
}
