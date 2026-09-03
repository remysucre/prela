use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::nordic10;
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q5c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        info,
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
    let Info {
        info: info_info, ..
    } = &db.info;
    db.movie
        .with(
            company
                .select(
                    company_ty
                        .select(companytype_text)
                        .eq("production companies")
                        .and(company_note.nrx(r"\(TV\)"))
                        .and(company_note.rx(r"\(USA\)")),
                )
                .and(info.select(info_info).is_in(nordic10()))
                .and(production_year.gt(1990)),
        )
        .select(title)
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q5c(db), 1))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q5c(db))
}
