use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::{genre6, writer5};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn gf_18c(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    info_ty
        .select(infotype_text)
        .eq("genres")
        .and(info_info.is_in(genre6()))
}

fn q18c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        cast,
        info,
        data,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Person { gender, .. } = &db.person;
    db.movie
        .with(
            info.with(gf_18c(db)).and(
                cast.select(
                    cast_note
                        .is_in(writer5())
                        .and(person.select(gender.eq("m"))),
                ),
            ),
        )
        .select(
            info.with(gf_18c(db))
                .select(info_info)
                .and(
                    data.with(data_ty.select(infotype_text).eq("votes"))
                        .select(data_text),
                )
                .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q18c(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q18c(db))
}
