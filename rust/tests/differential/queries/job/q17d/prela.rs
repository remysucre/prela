use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q17_any_co(db: &'static Job, re: &str) -> impl Drive<R: Row> {
    let Movie {
        keyword,
        company,
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
    db.movie
        .with(company.and(keyword.select(keyword_text).eq("character-name-in-title")))
        .select(cast.select(person).select(person_name).rx(re))
}

fn q17d(db: &'static Job) -> impl Drive<R: Row> {
    q17_any_co(db, r"Bert")
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q17d(db), 1))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q17d(db))
}
