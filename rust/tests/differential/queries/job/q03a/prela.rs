use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::nordic8;
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q3ac(db: &'static Job, countries: Vec<&'static str>, year: i64) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        ..
    } = &db.movie;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    db.movie
        .with(
            keyword
                .select(keyword_text)
                .rx(r"sequel")
                .and(info.select(info_info).is_in(countries))
                .and(production_year.gt(year)),
        )
        .select(title)
}

fn q3a(db: &'static Job) -> impl Drive<R: Row> {
    q3ac(db, nordic8(), 2005)
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q3a(db), 1))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q3a(db))
}
