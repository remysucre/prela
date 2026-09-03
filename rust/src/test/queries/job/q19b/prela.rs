use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q19b(db: &'static Job) -> impl Drive<R: Row> {
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
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
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
                .select(
                    country
                        .eq("[us]")
                        .and(company_note.rx(r"\(200.*\)"))
                        .and(company_note.rx(r"\(USA\)|\(worldwide\)")),
                )
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*2007|^USA:.*2008")),
                    ),
                )
                .and(production_year.ge(2007))
                .and(production_year.le(2008))
                .and(title.rx(r"Kung.*Fu.*Panda")),
        )
        .select(
            cast.with(
                cast_note
                    .eq("(voice)")
                    .and(role.select(roletype_text).eq("actress"))
                    .and(character)
                    .and(person.with(gender.eq("f").and(person_name.rx(r"Angel")).and(alias))),
            )
            .select(person)
            .select(person_name)
            .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q19b(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q19b(db))
}
