use crate::engine::*;
use crate::job_queries::helpers::{Row, film_or_warner_co, follow_link, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q21(db: &'static Job, countries: Vec<&'static str>, ylo: i64, yhi: i64) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name, ..
    } = &db.company;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    db.movie
        .with(
            link.and(keyword.select(keyword_text).eq("sequel"))
                .and(production_year.between(ylo, yhi))
                .and(info.select(info_info).is_in(countries))
                .and(follow_link(db)),
        )
        .select(
            film_or_warner_co(db)
                .select(company_name)
                .and(follow_link(db))
                .and(title),
        )
}

fn q21b(db: &'static Job) -> impl Drive<R: Row> {
    q21(db, vec!["Germany", "German"], 2000, 2010)
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q21b(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q21b(db))
}
