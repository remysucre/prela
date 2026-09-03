use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q12b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Data { ty: data_ty, .. } = &db.data;
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
            company
                .select(
                    country.eq("[us]").and(
                        company_ty
                            .select(companytype_text)
                            .is_in(["production companies", "distributors"]),
                    ),
                )
                .and(
                    data.select(data_ty)
                        .select(infotype_text)
                        .eq("bottom 10 rank"),
                )
                .and(production_year.gt(2000))
                .and(title.rx(r"^Birdemic|Movie")),
        )
        .select(
            info.with(info_ty.select(infotype_text).eq("budget"))
                .select(info_info)
                .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q12b(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q12b(db))
}
