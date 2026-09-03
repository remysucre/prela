use crate::engine::*;
use crate::job_queries::helpers::{Row, film_or_warner_co, follow_link, min_result, min_row};
use crate::job_queries::sets::nordic9;
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q27c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        complete_cast,
        link,
        ..
    } = &db.movie;
    let CompCastType {
        text: compcasttype_text,
        ..
    } = &db.comp_cast_type;
    let Company {
        name: company_name, ..
    } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    db.movie
        .with(
            link.and(keyword.select(keyword_text).eq("sequel"))
                .and(production_year.between(1950, 2010))
                .and(info.select(info_info.is_in(nordic9())))
                .and(
                    complete_cast.select(
                        subject
                            .select(compcasttype_text)
                            .eq("cast")
                            .and(status.select(compcasttype_text).rx(r"^complete")),
                    ),
                )
                .and(follow_link(db)),
        )
        .select(
            film_or_warner_co(db)
                .select(company_name)
                .and(follow_link(db))
                .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q27c(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q27c(db))
}
