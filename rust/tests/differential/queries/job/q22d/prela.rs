use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::{murder4, nordic10};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q22d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Kind {
        text: kind_text, ..
    } = &db.kind;
    db.movie
        .with(
            info.select(
                info_ty
                    .select(infotype_text)
                    .eq("countries")
                    .and(info_info.is_in(nordic10())),
            )
            .and(keyword.select(keyword_text).is_in(murder4()))
            .and(production_year.gt(2005))
            .and(kind.select(kind_text).is_in(["movie", "episode"])),
        )
        .select(
            title
                .and(
                    data.with(
                        data_text
                            .lt("8.5")
                            .and(data_ty.select(infotype_text).eq("rating")),
                    )
                    .select(data_text),
                )
                .and(
                    company
                        .with(
                            country.ne("[us]").and(
                                company_ty
                                    .select(companytype_text)
                                    .eq("production companies"),
                            ),
                        )
                        .select(company_name),
                ),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q22d(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q22d(db))
}
