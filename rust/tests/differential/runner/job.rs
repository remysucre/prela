//! End-to-end Join Order Benchmark differential execution.

use super::run_sql;
use crate::generate::{Cell, Database, Row, RowId};
use crate::queries::job::{Query, schema};
use crate::result::{ResultCell, ResultSet};
use crate::sql::to_sql;
use prela::job_shred::{JobShredder, OwnedJob};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Comparison {
    pub query: Query,
    pub columns: Vec<String>,
    pub sql: ResultSet,
    pub prela: ResultSet,
}

impl Comparison {
    pub fn equivalent(&self) -> bool {
        self.query
            .order
            .equivalent(&self.columns, &self.sql, &self.prela)
    }

    pub fn failure(&self, database: &Database) -> String {
        format!(
            "JOB Q{} differs\nMismatch written to {}\n\nColumns: {:?}\n\nSQL result:\n{}\n\nPrela result:\n{}\n\nReproduction SQL:\n{}\n{}",
            self.query.name,
            self.query.mismatch_path().display(),
            self.columns,
            self.sql,
            self.prela,
            to_sql(&schema::SCHEMA, database),
            self.query.sql,
        )
    }

    pub fn write_mismatch(&self, database: &Database) -> Result<PathBuf, String> {
        let path = self.query.mismatch_path();
        std::fs::write(&path, self.mismatch_sql(database))
            .map_err(|error| format!("write {}: {error}", path.display()))?;
        Ok(path)
    }

    fn mismatch_sql(&self, database: &Database) -> String {
        format!(
            "-- Generated JOB Q{} SQL/Prela mismatch.\n-- SQL result:\n{}\n-- Prela result:\n{}\n-- Columns: {:?}\n{}\n{}",
            self.query.name,
            sql_comment(&self.sql),
            sql_comment(&self.prela),
            self.columns,
            to_sql(&schema::SCHEMA, database),
            self.query.sql,
        )
    }
}

pub fn compare(database: &Database, query: Query) -> Result<Comparison, String> {
    let job = adapt_job(database)?;
    compare_with_job(database, query, &job)
}

fn compare_with_job(
    database: &Database,
    query: Query,
    job: &OwnedJob,
) -> Result<Comparison, String> {
    let (columns, sql) = run_sql(
        &schema::SCHEMA,
        database,
        query.name,
        query.sql,
        query.order,
    )?;
    let prela = run_prela(query.name, job)?;
    validate_result_width(query, &columns, &prela)?;
    let comparison = Comparison {
        query,
        columns,
        sql,
        prela,
    };
    if !comparison.equivalent() {
        comparison.write_mismatch(database)?;
    }
    Ok(comparison)
}

fn run_prela(name: &str, job: &OwnedJob) -> Result<ResultSet, String> {
    use prela::job_queries::helpers::Result as QueryResult;

    let row = job
        .differential(name)?
        .into_iter()
        .map(|cell| match cell {
            QueryResult::Null => ResultCell::Null,
            QueryResult::Integer(value) => ResultCell::Integer(i128::from(value)),
            QueryResult::Text(value) => ResultCell::Text(value),
        });
    Ok(ResultSet::from_rows([row.collect()]))
}

/// Shred one normalized SQL fixture into the production JOB representation.
/// Query logic stays in `job_queries::queries`; this is only the shared data
/// boundary between the canonical SQL tables and that representation.
fn adapt_job(database: &Database) -> Result<OwnedJob, String> {
    Ok(shred_job(database)?.into_job())
}

fn shred_job(database: &Database) -> Result<JobShredder, String> {
    let mut shred = JobShredder::default();

    for row in rows(database, "CompCastType")? {
        shred.comp_cast_type(
            integer(row, "CompCastType", "__id")?,
            text(row, "CompCastType", "kind")?,
        );
    }
    for row in rows(database, "CompanyName")? {
        shred.company_name(
            integer(row, "CompanyName", "__id")?,
            text(row, "CompanyName", "name")?,
            text(row, "CompanyName", "country_code")?,
        );
    }
    for row in rows(database, "CompanyType")? {
        shred.company_type(
            integer(row, "CompanyType", "__id")?,
            text(row, "CompanyType", "kind")?,
        );
    }
    for row in rows(database, "InfoType")? {
        shred.info_type(
            integer(row, "InfoType", "__id")?,
            text(row, "InfoType", "info")?,
        );
    }
    for row in rows(database, "Keyword")? {
        shred.keyword(
            integer(row, "Keyword", "__id")?,
            text(row, "Keyword", "keyword")?,
        );
    }
    for row in rows(database, "KindType")? {
        shred.kind_type(
            integer(row, "KindType", "__id")?,
            text(row, "KindType", "kind")?,
        );
    }
    for row in rows(database, "LinkType")? {
        shred.link_type(
            integer(row, "LinkType", "__id")?,
            text(row, "LinkType", "link")?,
        );
    }
    for row in rows(database, "Name")? {
        shred.name(
            integer(row, "Name", "__id")?,
            text(row, "Name", "name")?,
            text(row, "Name", "gender")?,
            text(row, "Name", "name_pcode_cf")?,
        );
    }
    for row in rows(database, "RoleType")? {
        shred.role_type(
            integer(row, "RoleType", "__id")?,
            text(row, "RoleType", "role")?,
        );
    }
    for row in rows(database, "CharName")? {
        shred.char_name(
            integer(row, "CharName", "__id")?,
            text(row, "CharName", "name")?,
        );
    }
    for row in rows(database, "Title")? {
        shred.title(
            integer(row, "Title", "__id")?,
            text(row, "Title", "title")?,
            integer(row, "Title", "kind_id")?,
            integer(row, "Title", "production_year")?,
            integer(row, "Title", "episode_nr")?,
        );
    }
    for row in rows(database, "AkaName")? {
        shred.aka_name(
            integer(row, "AkaName", "__id")?,
            integer(row, "AkaName", "person_id")?,
            text(row, "AkaName", "name")?,
        );
    }
    for row in rows(database, "AkaTitle")? {
        shred.aka_title(
            integer(row, "AkaTitle", "__id")?,
            integer(row, "AkaTitle", "movie_id")?,
            text(row, "AkaTitle", "title")?,
        );
    }
    for row in rows(database, "CastInfo")? {
        shred.cast_info(
            integer(row, "CastInfo", "__id")?,
            integer(row, "CastInfo", "person_id")?,
            integer(row, "CastInfo", "movie_id")?,
            integer(row, "CastInfo", "person_role_id")?,
            text(row, "CastInfo", "note")?,
            integer(row, "CastInfo", "role_id")?,
        );
    }
    for row in rows(database, "CompleteCast")? {
        shred.complete_cast(
            integer(row, "CompleteCast", "__id")?,
            integer(row, "CompleteCast", "movie_id")?,
            integer(row, "CompleteCast", "subject_id")?,
            integer(row, "CompleteCast", "status_id")?,
        );
    }
    for row in rows(database, "MovieCompanies")? {
        shred.movie_company(
            integer(row, "MovieCompanies", "__id")?,
            integer(row, "MovieCompanies", "movie_id")?,
            integer(row, "MovieCompanies", "company_id")?,
            integer(row, "MovieCompanies", "company_type_id")?,
            text(row, "MovieCompanies", "note")?,
        );
    }
    for row in rows(database, "MovieInfo")? {
        shred.movie_info(
            integer(row, "MovieInfo", "__id")?,
            integer(row, "MovieInfo", "movie_id")?,
            integer(row, "MovieInfo", "info_type_id")?,
            text(row, "MovieInfo", "info")?,
            text(row, "MovieInfo", "note")?,
        );
    }
    for row in rows(database, "MovieInfoIdx")? {
        shred.movie_info_idx(
            integer(row, "MovieInfoIdx", "__id")?,
            integer(row, "MovieInfoIdx", "movie_id")?,
            integer(row, "MovieInfoIdx", "info_type_id")?,
            text(row, "MovieInfoIdx", "info")?,
        );
    }
    for row in rows(database, "MovieKeyword")? {
        shred.movie_keyword(
            integer(row, "MovieKeyword", "movie_id")?,
            integer(row, "MovieKeyword", "keyword_id")?,
        );
    }
    for row in rows(database, "MovieLink")? {
        shred.movie_link(
            integer(row, "MovieLink", "__id")?,
            integer(row, "MovieLink", "movie_id")?,
            integer(row, "MovieLink", "linked_movie_id")?,
            integer(row, "MovieLink", "link_type_id")?,
        );
    }
    for row in rows(database, "PersonInfo")? {
        shred.person_info(
            integer(row, "PersonInfo", "__id")?,
            integer(row, "PersonInfo", "person_id")?,
            integer(row, "PersonInfo", "info_type_id")?,
            text(row, "PersonInfo", "info")?,
            text(row, "PersonInfo", "note")?,
        );
    }

    Ok(shred)
}

fn rows<'a>(database: &'a Database, entity: &str) -> Result<&'a [Row], String> {
    database
        .tables
        .iter()
        .find(|table| table.entity == entity)
        .map(|table| table.rows.as_slice())
        .ok_or_else(|| format!("generated database has no {entity} table"))
}

fn integer(row: &Row, entity: &'static str, field: &'static str) -> Result<Option<i64>, String> {
    if field == crate::schema::ID_FIELD {
        return Ok(Some(
            i64::try_from(row.id.index).expect("tiny row identity fits i64") + 1,
        ));
    }
    match row.cells.get(&crate::schema::ColumnId::new(entity, field)) {
        Some(Cell::Null) => Ok(None),
        Some(Cell::I64(value)) => Ok(Some(*value)),
        Some(Cell::Reference(target)) => Ok(Some(
            i64::try_from(target.index).expect("tiny row identity fits i64") + 1,
        )),
        Some(other) => Err(format!("{entity}.{field} expected integer, got {other:?}")),
        None => Err(format!("generated row has no {entity}.{field}")),
    }
}

fn text<'a>(
    row: &'a Row,
    entity: &'static str,
    field: &'static str,
) -> Result<Option<&'a str>, String> {
    match row.cells.get(&crate::schema::ColumnId::new(entity, field)) {
        Some(Cell::Null) => Ok(None),
        Some(Cell::Text(value)) => Ok(Some(value)),
        Some(other) => Err(format!("{entity}.{field} expected text, got {other:?}")),
        None => Err(format!("generated row has no {entity}.{field}")),
    }
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
    use crate::generate::{Cell, Row, Table, generator};
    use crate::queries::job as queries;
    use crate::result::ResultCell;
    use crate::schema::ColumnId;
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
            assert!(comparison.equivalent(), "{}", comparison.failure(&database));
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
            assert!(comparison.equivalent(), "{}", comparison.failure(&database));
            assert_eq!(comparison.sql.rows, vec![vec![ResultCell::Null; width]]);
        }
    }

    #[test]
    fn shared_shredder_cache_matches_in_memory_relations() {
        let database = fixture();
        let in_memory = adapt_job(&database).unwrap();
        let cache_dir = std::env::temp_dir().join(format!(
            "prela_job_shred_roundtrip_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let written = shred_job(&database).unwrap().write_cache(&cache_dir);
        assert_eq!(written.len(), prela::job_schema::manifest().len());
        let from_cache = prela::job_schema::load(&cache_dir);

        for &query in queries::all() {
            assert_eq!(
                in_memory.differential(query.name).unwrap(),
                prela::job_queries::differential(query.name, &from_cache).unwrap(),
                "JOB Q{} differs between in-memory and cache shredding",
                query.name,
            );
        }

        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[hegel::test(
        test_cases = 100,
        suppress_health_check = [HealthCheck::TooSlow]
    )]
    fn prela_matches_duckdb_on_generated_job_databases(tc: TestCase) {
        let database = tc.draw(generator(&schema::SCHEMA));
        let job = adapt_job(&database).unwrap();
        for &query in queries::all() {
            let comparison = compare_with_job(&database, query, &job).unwrap();
            assert!(comparison.equivalent(), "{}", comparison.failure(&database));
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
