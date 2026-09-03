use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q10a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast {
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Character {
        text: character_text,
        ..
    } = &db.character;
    let Company { country, .. } = &db.company;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    db.movie
        .with(
            company
                .select(country)
                .eq("[ru]")
                .and(production_year.gt(2005)),
        )
        .select(
            cast.with(
                cast_note
                    .rx(r"\(voice\)")
                    .and(cast_note.rx(r"\(uncredited\)"))
                    .and(role.select(roletype_text).eq("actor")),
            )
            .select(character)
            .select(character_text)
            .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q10a(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q10a(db))
}
