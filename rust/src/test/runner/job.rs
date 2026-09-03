//! End-to-end Join Order Benchmark differential execution.

use super::run_sql;
use crate::engine::{Dense, DictMultiRel, DictRel, Drive, Id, MultiRel, QueryExt, Universe, VecRel};
use crate::job_schema::{
    self, Cast, Company, CompleteCast, Data, Info, Job, Movie, MovieLink, Person, PersonInfo,
};
use crate::loader::Loader;
use crate::test::generate::Database;
use crate::test::queries::job::{Query, schema};
use crate::test::relations::Relations;
use crate::test::result::ResultSet;
use crate::test::sql::to_sql;
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
    compare_with_job(database, query, job)
}

fn compare_with_job(
    database: &Database,
    query: Query,
    job: &'static Job,
) -> Result<Comparison, String> {
    let (columns, sql) = run_sql(
        &schema::SCHEMA,
        database,
        query.name,
        query.sql,
        query.order,
    )?;
    let prela = crate::job_queries::differential(query.name, job)?;
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

/// Shred one normalized SQL fixture into the production JOB representation.
/// Query logic stays in `job_queries::queries`; this is only the shared data
/// boundary between the canonical SQL tables and that representation.
fn adapt_job(database: &Database) -> Result<&'static Job, String> {
    let db = Relations::new(database);
    let movie_count = db.table::<Movie>("Title")?.n;
    let person_count = db.table::<Person>("Name")?.n;
    let company_count = db.table::<Company>("MovieCompanies")?.n;
    let cast_count = db.table::<Cast>("CastInfo")?.n;
    let info_count = db.table::<Info>("MovieInfo")?.n;
    let data_count = db.table::<Data>("MovieInfoIdx")?.n;
    let person_info_count = db.table::<PersonInfo>("PersonInfo")?.n;
    let movie_link_count = db.table::<MovieLink>("MovieLink")?.n;
    let complete_cast_count = db.table::<CompleteCast>("CompleteCast")?.n;

    let movie_keyword_movie =
        db.foreign::<schema::MovieKeyword, Movie>("MovieKeyword", "movie_id")?;
    let movie_keyword_keyword =
        db.foreign::<schema::MovieKeyword, schema::Keyword>("MovieKeyword", "keyword_id")?;
    let company_movie = db.foreign::<Company, Movie>("MovieCompanies", "movie_id")?;
    let company_name =
        db.foreign::<Company, schema::CompanyName>("MovieCompanies", "company_id")?;
    let cast_movie = db.foreign::<Cast, Movie>("CastInfo", "movie_id")?;
    let info_movie = db.foreign::<Info, Movie>("MovieInfo", "movie_id")?;
    let data_movie = db.foreign::<Data, Movie>("MovieInfoIdx", "movie_id")?;
    let complete_cast_movie = db.foreign::<CompleteCast, Movie>("CompleteCast", "movie_id")?;
    let movie_link_movie = db.foreign::<MovieLink, Movie>("MovieLink", "movie_id")?;
    let movie_link_target = db.foreign::<MovieLink, Movie>("MovieLink", "linked_movie_id")?;
    let aka_title_movie = db.foreign::<schema::AkaTitle, Movie>("AkaTitle", "movie_id")?;
    let aka_name_person = db.foreign::<schema::AkaName, Person>("AkaName", "person_id")?;
    let person_info_person = db.foreign::<PersonInfo, Person>("PersonInfo", "person_id")?;

    let mut job = job_schema::build(&mut Loader::probing());
    job.movie.id = Universe::new(movie_count);
    job.cast.id = Universe::new(cast_count);
    job.person.id = Universe::new(person_count);
    job.company.id = Universe::new(company_count);
    job.info.id = Universe::new(info_count);
    job.data.id = Universe::new(data_count);
    job.person_info.id = Universe::new(person_info_count);
    job.movie_link.id = Universe::new(movie_link_count);
    job.complete_cast.id = Universe::new(complete_cast_count);
    job.movie.title = db
        .static_text::<Movie>("Title", "title")?
        .into_dense("Title.title")?;
    job.movie.kind = dict_dense(
        db.foreign::<Movie, schema::KindType>("Title", "kind_id")?
            .into_dense("Title.kind_id")?,
        db.static_text::<schema::KindType>("KindType", "kind")?
            .into_dense("KindType.kind")?,
    );
    job.movie.production_year = db
        .integer::<Movie>("Title", "production_year")?
        .into_multi();
    job.movie.episode_nr = db.integer::<Movie>("Title", "episode_nr")?.into_multi();
    job.movie.keyword = dict_multi(
        movie_count,
        (&movie_keyword_movie).inv().select(&movie_keyword_keyword),
        db.static_text::<schema::Keyword>("Keyword", "keyword")?
            .into_dense("Keyword.keyword")?,
    );
    job.movie.company = materialize_multi(movie_count, (&company_movie).inv());
    job.movie.cast = materialize_multi(movie_count, (&cast_movie).inv());
    job.movie.info = materialize_multi(movie_count, (&info_movie).inv());
    job.movie.data = materialize_multi(movie_count, (&data_movie).inv());
    job.movie.complete_cast = materialize_multi(movie_count, (&complete_cast_movie).inv());
    job.movie.link = materialize_multi(movie_count, (&movie_link_movie).inv());
    job.movie.linked_by = materialize_multi(movie_count, (&movie_link_target).inv());
    job.movie.aka = dict_multi(
        movie_count,
        (&aka_title_movie).inv(),
        db.static_text::<schema::AkaTitle>("AkaTitle", "title")?
            .into_dense("AkaTitle.title")?,
    );
    job.company.name = materialize_dense(
        company_count,
        (&company_name).select(&db.static_text::<schema::CompanyName>("CompanyName", "name")?),
        "MovieCompanies.company_id -> CompanyName.name",
    )?;
    job.company.country = materialize_multi(
        company_count,
        (&company_name)
            .select(&db.static_text::<schema::CompanyName>("CompanyName", "country_code")?),
    );
    job.company.note = db
        .static_text::<Company>("MovieCompanies", "note")?
        .into_multi();
    job.company.ty = dict_dense(
        db.foreign::<Company, schema::CompanyType>("MovieCompanies", "company_type_id")?
            .into_dense("MovieCompanies.company_type_id")?,
        db.static_text::<schema::CompanyType>("CompanyType", "kind")?
            .into_dense("CompanyType.kind")?,
    );
    job.cast.person = db
        .foreign::<Cast, Person>("CastInfo", "person_id")?
        .into_dense("CastInfo.person_id")?;
    job.cast.role = dict_dense(
        db.foreign::<Cast, schema::RoleType>("CastInfo", "role_id")?
            .into_dense("CastInfo.role_id")?,
        db.static_text::<schema::RoleType>("RoleType", "role")?
            .into_dense("RoleType.role")?,
    );
    job.cast.note = db.static_text::<Cast>("CastInfo", "note")?.into_multi();
    job.cast.character = dict_multi(
        cast_count,
        db.foreign::<Cast, schema::CharName>("CastInfo", "person_role_id")?,
        db.static_text::<schema::CharName>("CharName", "name")?
            .into_dense("CharName.name")?,
    );
    job.person.name = db
        .static_text::<Person>("Name", "name")?
        .into_dense("Name.name")?;
    job.person.gender = db.static_text::<Person>("Name", "gender")?.into_multi();
    job.person.alias = dict_multi(
        person_count,
        (&aka_name_person).inv(),
        db.static_text::<schema::AkaName>("AkaName", "name")?
            .into_dense("AkaName.name")?,
    );
    job.person.bio = materialize_multi(person_count, (&person_info_person).inv());
    job.person.name_pcode_cf = db
        .static_text::<Person>("Name", "name_pcode_cf")?
        .into_multi();
    job.info.info = db
        .static_text::<Info>("MovieInfo", "info")?
        .into_dense("MovieInfo.info")?;
    job.info.ty = dict_dense(
        db.foreign::<Info, schema::InfoType>("MovieInfo", "info_type_id")?
            .into_dense("MovieInfo.info_type_id")?,
        db.static_text::<schema::InfoType>("InfoType", "info")?
            .into_dense("InfoType.info")?,
    );
    job.info.note = db.static_text::<Info>("MovieInfo", "note")?.into_multi();
    job.data.text = db
        .static_text::<Data>("MovieInfoIdx", "info")?
        .into_dense("MovieInfoIdx.info")?;
    job.data.ty = dict_dense(
        db.foreign::<Data, schema::InfoType>("MovieInfoIdx", "info_type_id")?
            .into_dense("MovieInfoIdx.info_type_id")?,
        db.static_text::<schema::InfoType>("InfoType", "info")?
            .into_dense("InfoType.info")?,
    );
    job.person_info.info = db
        .static_text::<PersonInfo>("PersonInfo", "info")?
        .into_dense("PersonInfo.info")?;
    job.person_info.ty = dict_dense(
        db.foreign::<PersonInfo, schema::InfoType>("PersonInfo", "info_type_id")?
            .into_dense("PersonInfo.info_type_id")?,
        db.static_text::<schema::InfoType>("InfoType", "info")?
            .into_dense("InfoType.info")?,
    );
    job.person_info.note = db
        .static_text::<PersonInfo>("PersonInfo", "note")?
        .into_multi();
    job.movie_link.target = movie_link_target.into_dense("MovieLink.linked_movie_id")?;
    job.movie_link.ty = dict_dense(
        db.foreign::<MovieLink, schema::LinkType>("MovieLink", "link_type_id")?
            .into_dense("MovieLink.link_type_id")?,
        db.static_text::<schema::LinkType>("LinkType", "link")?
            .into_dense("LinkType.link")?,
    );
    job.complete_cast.status = dict_dense(
        db.foreign::<CompleteCast, schema::CompCastType>("CompleteCast", "status_id")?
            .into_dense("CompleteCast.status_id")?,
        db.static_text::<schema::CompCastType>("CompCastType", "kind")?
            .into_dense("CompCastType.kind")?,
    );
    job.complete_cast.subject = dict_dense(
        db.foreign::<CompleteCast, schema::CompCastType>("CompleteCast", "subject_id")?
            .into_dense("CompleteCast.subject_id")?,
        db.static_text::<schema::CompCastType>("CompCastType", "kind")?
            .into_dense("CompCastType.kind")?,
    );

    Ok(Box::leak(Box::new(job)))
}

fn dict_dense<E: 'static, T: 'static>(
    codes: VecRel<Id<T>, Id<E>>,
    table: VecRel<&'static str, Id<T>>,
) -> DictRel<&'static str, Id<E>> {
    DictRel::new(
        VecRel::new(codes.v.into_iter().map(Dense::idx).collect()),
        VecRel::new(table.v),
    )
}

fn dict_multi<E: 'static, T: 'static, Q>(
    n: usize,
    codes: Q,
    table: VecRel<&'static str, Id<T>>,
) -> DictMultiRel<&'static str, Id<E>>
where
    Q: Drive<D = Id<E>, R = Id<T>>,
{
    DictMultiRel::new(
        materialize_multi(n, codes.map(Dense::idx)),
        VecRel::new(table.v),
    )
}

fn materialize_dense<Q>(n: usize, query: Q, name: &str) -> Result<VecRel<Q::R, Q::D>, String>
where
    Q: Drive,
    Q::D: Dense,
    Q::R: Copy + 'static,
{
    let mut values = vec![None; n];
    let mut duplicate = None;
    query.drive(|key, value| {
        let index = key.idx();
        if values[index].replace(value).is_some() {
            duplicate = Some(index);
        }
    });
    if let Some(index) = duplicate {
        return Err(format!("{name} has multiple values for row {index}"));
    }
    let values = values
        .into_iter()
        .enumerate()
        .map(|(index, value)| value.ok_or_else(|| format!("{name} is missing row {index}")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(VecRel::new(values))
}

fn materialize_multi<Q>(n: usize, query: Q) -> MultiRel<Q::R, Q::D>
where
    Q: Drive,
    Q::D: Dense,
    Q::R: Copy + 'static,
{
    let mut pairs = Vec::new();
    query.drive(|key, value| pairs.push((key.idx(), value)));
    MultiRel::from_pairs(n, pairs)
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
    use crate::test::generate::{Cell, Row, Table, generator};
    use crate::test::queries::job as queries;
    use crate::test::result::ResultCell;
    use crate::test::schema::{ColumnId, ID_FIELD};
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
        let mut cells = BTreeMap::from([(ColumnId::new(entity, ID_FIELD), Cell::I64(id))]);
        cells.extend(
            schema::SCHEMA
                .columns
                .iter()
                .filter(|column| column.entity == entity)
                .map(|column| (ColumnId::new(entity, column.field), Cell::Null)),
        );
        cells.extend(
            values
                .into_iter()
                .map(|(field, value)| (ColumnId::new(entity, field), value)),
        );
        Row { cells }
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

    #[hegel::test(
        test_cases = 100,
        suppress_health_check = [HealthCheck::TooSlow]
    )]
    fn job_queries_match_generated_nullable_fixtures(tc: TestCase) {
        let database = tc.draw(generator(&schema::SCHEMA));
        let job = adapt_job(&database).unwrap();
        for query in queries::all() {
            let comparison = compare_with_job(&database, query, job).unwrap();
            assert!(comparison.equivalent(), "{}", comparison.failure(&database));
        }
    }

    #[hegel::test(
        test_cases = 10_000,
        suppress_health_check = [HealthCheck::TooSlow]
    )]
    #[ignore = "run explicitly to search the JOB query suite for counterexamples"]
    fn differential_job_suite(tc: TestCase) {
        let database = tc.draw(generator(&schema::SCHEMA));
        let job = adapt_job(&database).unwrap();
        for query in queries::all() {
            let comparison = compare_with_job(&database, query, job).unwrap();
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
                .join("src/test/generated-databases.sql");
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
