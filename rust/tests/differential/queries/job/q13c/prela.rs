use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q13c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
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
    let Info { ty: info_ty, .. } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Kind {
        text: kind_text, ..
    } = &db.kind;
    db.movie
        .with(
            kind.select(kind_text)
                .eq("movie")
                .and(
                    info.select(info_ty)
                        .select(infotype_text)
                        .eq("release dates"),
                )
                .and(title.ne(""))
                .and(title.rx(r"^Champion|^Loser")),
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
                    data.with(data_ty.select(infotype_text).eq("rating"))
                        .select(data_text),
                )
                .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q13c(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q13c(db))
}
