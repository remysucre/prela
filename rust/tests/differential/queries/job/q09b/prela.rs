use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q9b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        ..
    } = &db.movie;
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
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
        country,
        note: company_note,
        ..
    } = &db.company;
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
                .and(production_year.ge(2007))
                .and(production_year.le(2010)),
        )
        .select(
            cast.with(
                cast_note
                    .eq("(voice)")
                    .and(role.select(roletype_text).eq("actress"))
                    .and(person.with(gender.eq("f").and(person_name.rx(r"Angel")))),
            )
            .select(
                person
                    .select(alias)
                    .select(akaname_text)
                    .and(character.select(character_text))
                    .and(person.select(person_name)),
            )
            .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q9b(db), 4))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q9b(db))
}
