use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q10c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast {
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Character {
        text: character_text,
        ..
    } = &db.character;
    let Company { country, .. } = &db.company;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(production_year.gt(1990)),
        )
        .select(
            cast.with(cast_note.rx(r"\(producer\)"))
                .select(character)
                .select(character_text)
                .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q10c(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q10c(db))
}
