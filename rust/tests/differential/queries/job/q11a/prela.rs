use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q11a(db: &'static Job) -> impl Drive<R: Row> {
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
    let LinkType {
        text: linktype_text,
        ..
    } = &db.link_type;
    let MovieLink {
        ty: movielink_ty, ..
    } = &db.movie_link;
    db.movie
        .with(
            link.and(keyword.select(keyword_text).eq("sequel"))
                .and(production_year.between(1950, 2000)),
        )
        .select(
            company
                .with(
                    country
                        .ne("[pl]")
                        .and(company_name.rx(r"Film|Warner"))
                        .and(
                            company_ty
                                .select(companytype_text)
                                .eq("production companies"),
                        )
                        .minus(company_note),
                )
                .select(company_name)
                .and(
                    link.select(movielink_ty)
                        .select(linktype_text)
                        .rx(r"follow"),
                )
                .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q11a(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q11a(db))
}
