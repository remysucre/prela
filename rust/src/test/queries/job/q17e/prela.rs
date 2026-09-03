use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q17e(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person {
        name: person_name, ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword.select(keyword_text).eq("character-name-in-title")),
        )
        .select(cast.select(person).select(person_name))
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q17e(db), 1))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q17e(db))
}
