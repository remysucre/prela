use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn ib_18a(db: &'static Job) -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    let Movie { info, .. } = &db.movie;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    info.with(info_ty.select(infotype_text).eq("budget"))
        .select(info_info)
}

fn q18a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title, cast, data, ..
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
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    db.movie
        .with(
            ib_18a(db).and(
                cast.select(
                    cast_note
                        .is_in(["(producer)", "(executive producer)"])
                        .and(person.select(gender.eq("m").and(person_name.rx(r"Tim")))),
                ),
            ),
        )
        .select(
            ib_18a(db)
                .and(
                    data.with(data_ty.select(infotype_text).eq("votes"))
                        .select(data_text),
                )
                .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q18a(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q18a(db))
}
