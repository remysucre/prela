use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q12a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
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
    db.movie
        .with(
            info.select(
                info_ty
                    .select(infotype_text)
                    .eq("genres")
                    .and(info_info.is_in(["Drama", "Horror"])),
            )
            .and(production_year.ge(2005))
            .and(production_year.le(2008)),
        )
        .select(
            company
                .with(
                    country.eq("[us]").and(
                        company_ty
                            .select(companytype_text)
                            .eq("production companies"),
                    ),
                )
                .select(company_name)
                .and(
                    data.with(
                        data_ty
                            .select(infotype_text)
                            .eq("rating")
                            .and(data_text.gt("8.0")),
                    )
                    .select(data_text),
                )
                .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q12a(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q12a(db))
}
