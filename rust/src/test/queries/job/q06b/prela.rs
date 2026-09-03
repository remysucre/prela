use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::kw8;
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q6_comic(db: &'static Job, year: i64) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person {
        name: person_name, ..
    } = &db.person;
    let kw = || keyword.select(keyword_text).is_in(kw8());
    let downey = cast
        .select(person)
        .select(person_name)
        .rx(r"Downey.*Robert");
    db.movie
        .with(production_year.gt(year).and(kw()))
        .select(kw().and(downey).and(title))
}

fn q6b(db: &'static Job) -> impl Drive<R: Row> {
    q6_comic(db, 2014)
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q6b(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q6b(db))
}
