use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q32(db: &'static Job, kw: &'static str) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        link,
        ..
    } = &db.movie;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let LinkType {
        text: linktype_text,
        ..
    } = &db.link_type;
    let MovieLink {
        target,
        ty: movielink_ty,
        ..
    } = &db.movie_link;
    db.movie
        .with(link.and(keyword.select(keyword_text).eq(kw)))
        .select(
            link.select(movielink_ty)
                .select(linktype_text)
                .and(title)
                .and(link.select(target).select(title)),
        )
}

fn q32b(db: &'static Job) -> impl Drive<R: Row> {
    q32(db, "character-name-in-title")
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q32b(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q32b(db))
}
