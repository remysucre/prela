use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q26b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        cast,
        data,
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
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
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
                .and(keyword.select(keyword_text).is_in([
                    "superhero",
                    "marvel-comics",
                    "based-on-comic",
                    "fight",
                ]))
                .and(kind.select(kind_text).eq("movie"))
                .and(production_year.gt(2005)),
        )
        .select(
            cast.with(character.select(character_text).rx(r"[Mm]an"))
                .select(character)
                .select(character_text)
                .and(
                    data.with(
                        data_ty
                            .select(infotype_text)
                            .eq("rating")
                            .and(data_text.gt("8.0")),
                    )
                    .select(data_text),
                )
                .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q26b(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q26b(db))
}
