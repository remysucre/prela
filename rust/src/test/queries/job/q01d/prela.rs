use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q1d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Data { ty: data_ty, .. } = &db.data;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    db.movie
        .with(
            data.select(data_ty)
                .select(infotype_text)
                .eq("bottom 10 rank")
                .and(production_year.gt(2000)),
        )
        .select(
            company
                .with(
                    company_ty
                        .select(companytype_text)
                        .eq("production companies")
                        .and(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")),
                )
                .select(company_note)
                .and(title)
                .and(production_year),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q1d(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q1d(db))
}
