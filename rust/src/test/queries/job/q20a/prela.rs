use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::kw8;
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q20a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        cast,
        complete_cast,
        ..
    } = &db.movie;
    let Cast { character, .. } = &db.cast;
    let Character {
        text: character_text,
        ..
    } = &db.character;
    let CompCastType {
        text: compcasttype_text,
        ..
    } = &db.comp_cast_type;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Kind {
        text: kind_text, ..
    } = &db.kind;
    db.movie
        .with(
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .eq("cast")
                        .and(status.select(compcasttype_text).rx(r"complete")),
                )
                .and(keyword.select(keyword_text).is_in(kw8()))
                .and(kind.select(kind_text).eq("movie"))
                .and(production_year.gt(1950))
                .and(
                    cast.select(
                        character.select(
                            character_text
                                .nrx(r"Sherlock")
                                .and(character_text.rx(r"Tony.*Stark|Iron.*Man")),
                        ),
                    ),
                ),
        )
        .select(title)
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q20a(db), 1))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q20a(db))
}
