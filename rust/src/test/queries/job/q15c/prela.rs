use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q15c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        info,
        aka,
        ..
    } = &db.movie;
    let Company { country, .. } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        note: info_note,
        ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword)
                .and(aka)
                .and(production_year.gt(1990)),
        )
        .select(
            info.with(
                info_ty
                    .select(infotype_text)
                    .eq("release dates")
                    .and(info_info.rx(r"^USA:.* 199|^USA:.* 200"))
                    .and(info_note.rx(r"internet")),
            )
            .select(info_info)
            .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q15c(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q15c(db))
}
