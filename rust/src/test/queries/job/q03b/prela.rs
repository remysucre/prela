use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q3b(db: &'static Job) -> impl Drive<R: Row> {
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
                .and(info.select(info_info).eq("Bulgaria"))
                .and(production_year.gt(2010)),
        )
        .select(title)
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q3b(db), 1))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q3b(db))
}
