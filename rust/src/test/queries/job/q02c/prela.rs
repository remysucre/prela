use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q2(db: &'static Job, cc: &'static str) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        company,
        ..
    } = &db.movie;
    let Company { country, .. } = &db.company;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    db.movie
        .with(
            keyword
                .select(keyword_text)
                .eq("character-name-in-title")
                .and(company.select(country).eq(cc)),
        )
        .select(title)
}

fn q2c(db: &'static Job) -> impl Drive<R: Row> {
    q2(db, "[sm]")
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q2c(db), 1))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q2c(db))
}
