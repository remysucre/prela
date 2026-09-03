use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::voice4;
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q24b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
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
    let Character {
        text: character_text,
        ..
    } = &db.character;
    let Company {
        name: company_name,
        country,
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
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
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
                        .and(company_name.eq("DreamWorks Animation")),
                )
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*201|^USA:.*201")),
                    ),
                )
                .and(keyword.select(keyword_text).is_in([
                    "hero",
                    "martial-arts",
                    "hand-to-hand-combat",
                    "computer-animated-movie",
                ]))
                .and(production_year.gt(2010))
                .and(title.rx(r"^Kung Fu Panda")),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.select(roletype_text).eq("actress"))
                    .and(person.with(gender.eq("f").and(person_name.rx(r"An")).and(alias))),
            )
            .select(
                character
                    .select(character_text)
                    .and(person.select(person_name)),
            )
            .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q24b(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q24b(db))
}
