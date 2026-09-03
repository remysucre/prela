use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q8a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
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
        ..
    } = &db.cast;
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    let Person {
        name: person_name,
        alias,
        ..
    } = &db.person;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    db.movie
        .with(
            company.select(
                country
                    .eq("[jp]")
                    .and(company_note.rx(r"\(Japan\)"))
                    .and(company_note.nrx(r"\(USA\)")),
            ),
        )
        .select(
            cast.with(
                cast_note
                    .eq("(voice: English version)")
                    .and(role.select(roletype_text).eq("actress"))
                    .and(person.with(person_name.rx(r"Yo").and(person_name.nrx(r"Yu")))),
            )
            .select(person)
            .select(alias)
            .select(akaname_text)
            .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q8a(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q8a(db))
}
