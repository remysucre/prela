use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q11d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    db.movie
        .with(
            keyword
                .select(keyword_text)
                .is_in(["sequel", "revenge", "based-on-novel"])
                .and(production_year.gt(1950))
                .and(link),
        )
        .select(
            company
                .with(
                    country
                        .ne("[pl]")
                        .and(
                            company_ty
                                .select(companytype_text)
                                .ne("production companies"),
                        )
                        .and(company_note),
                )
                .select(company_name.and(company_note))
                .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q11d(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q11d(db))
}
