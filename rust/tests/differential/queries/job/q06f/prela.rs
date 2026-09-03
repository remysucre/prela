use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::kw8;
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q6f(db: &'static Job) -> impl Drive<R: Row> {
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
    let cast_name = cast.select(person).select(person_name);
    db.movie
        .with(production_year.gt(2000).and(kw()))
        .select(kw().and(cast_name).and(title))
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q6f(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q6f(db))
}
