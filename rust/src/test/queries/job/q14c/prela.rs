use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::{murder4, nordic10};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q14c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        info,
        data,
        ..
    } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
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
    let Kind {
        text: kind_text, ..
    } = &db.kind;
    db.movie
        .with(
            keyword
                .select(keyword_text)
                .is_in(murder4())
                .and(kind.select(kind_text).is_in(["movie", "episode"]))
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("countries")
                            .and(info_info.is_in(nordic10())),
                    ),
                )
                .and(production_year.gt(2005)),
        )
        .select(
            data.with(
                data_ty
                    .select(infotype_text)
                    .eq("rating")
                    .and(data_text.lt("8.5")),
            )
            .select(data_text)
            .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q14c(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q14c(db))
}
