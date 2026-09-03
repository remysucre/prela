use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::voice4;
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q19d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    let Info { ty: info_ty, .. } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(
                    info.select(info_ty)
                        .select(infotype_text)
                        .eq("release dates"),
                )
                .and(production_year.gt(2000)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.select(roletype_text).eq("actress"))
                    .and(character)
                    .and(person.with(gender.eq("f").and(alias))),
            )
            .select(person)
            .select(person_name)
            .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q19d(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q19d(db))
}
