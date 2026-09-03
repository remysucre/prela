use crate::engine::*;
use crate::job_queries::helpers::{Row, kw_rx, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q4(db: &'static Job, year: i64, rating: &'static str) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        data,
        ..
    } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    db.movie
        .with(
            keyword
                .with(kw_rx(db, r"sequel"))
                .and(production_year.gt(year)),
        )
        .select(
            data.with(
                data_ty
                    .select(infotype_text)
                    .eq("rating")
                    .and(data_text.gt(rating)),
            )
            .select(data_text)
            .and(title),
        )
}

fn q4b(db: &'static Job) -> impl Drive<R: Row> {
    q4(db, 2010, "9.0")
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q4b(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q4b(db))
}
