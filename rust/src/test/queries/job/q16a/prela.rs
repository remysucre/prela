use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q16ad(db: &'static Job, lo: i64) -> impl Drive<R: Row> {
    let Movie {
        title,
        episode_nr,
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast { person, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person { alias, .. } = &db.person;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword.select(keyword_text).eq("character-name-in-title"))
                .and(episode_nr.ge(lo))
                .and(episode_nr.lt(100)),
        )
        .select(
            cast.select(person)
                .select(alias)
                .select(akaname_text)
                .and(title),
        )
}

fn q16a(db: &'static Job) -> impl Drive<R: Row> {
    q16ad(db, 50)
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q16a(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q16a(db))
}
