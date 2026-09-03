use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::{kw7, writer5};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn gf_horror(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
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
        .and(info_info.is_in(["Horror", "Thriller"]))
}

fn q31a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        company,
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
    let Company {
        name: company_name, ..
    } = &db.company;
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
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(company_name)
                .rx(r"^Lionsgate")
                .and(info.with(gf_horror(db)))
                .and(keyword.select(keyword_text).is_in(kw7())),
        )
        .select(
            info.with(gf_horror(db))
                .select(info_info)
                .and(
                    data.with(data_ty.select(infotype_text).eq("votes"))
                        .select(data_text),
                )
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()).and(person.with(gender.eq("m"))))
                        .select(person)
                        .select(person_name),
                ),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q31a(db), 4))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q31a(db))
}
