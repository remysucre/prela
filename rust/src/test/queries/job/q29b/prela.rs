use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::voice3;
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q29b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        cast,
        info,
        complete_cast,
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
    let CompCastType {
        text: compcasttype_text,
        ..
    } = &db.comp_cast_type;
    let Company { country, .. } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
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
        bio,
        ..
    } = &db.person;
    let PersonInfo {
        ty: personinfo_ty, ..
    } = &db.person_info;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    db.movie
        .with(
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .eq("cast")
                        .and(status.select(compcasttype_text).eq("complete+verified")),
                )
                .and(company.select(country).eq("[us]"))
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("release dates")
                            .and(info_info.rx(r"^USA:.*200")),
                    ),
                )
                .and(keyword.select(keyword_text).eq("computer-animation"))
                .and(title.eq("Shrek 2"))
                .and(production_year.ge(2000))
                .and(production_year.le(2005)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice3())
                    .and(role.select(roletype_text).eq("actress"))
                    .and(character.select(character_text).eq("Queen"))
                    .and(
                        person.with(
                            gender
                                .eq("f")
                                .and(person_name.rx(r"An"))
                                .and(alias)
                                .and(bio.select(personinfo_ty.select(infotype_text).eq("height"))),
                        ),
                    ),
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
    Ok(min_result(q29b(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q29b(db))
}
