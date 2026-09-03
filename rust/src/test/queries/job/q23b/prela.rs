use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn k_23ab(db: &'static Job) -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    let Movie { kind, .. } = &db.movie;
    let Kind {
        text: kind_text, ..
    } = &db.kind;
    kind.select(kind_text).eq("movie")
}

fn q23b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        info,
        complete_cast,
        ..
    } = &db.movie;
    let CompCastType {
        text: compcasttype_text,
        ..
    } = &db.comp_cast_type;
    let Company { country, .. } = &db.company;
    let CompleteCast { status, .. } = &db.complete_cast;
    let Info {
        info: info_info,
        ty: info_ty,
        note: info_note,
        ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    db.movie
        .with(
            complete_cast
                .select(status)
                .select(compcasttype_text)
                .eq("complete+verified")
                .and(company.select(country).eq("[us]"))
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("release dates")
                            .and(info_note.rx(r"internet"))
                            .and(info_info.rx(r"^USA:.* 200")),
                    ),
                )
                .and(k_23ab(db))
                .and(
                    keyword
                        .select(keyword_text)
                        .is_in(["nerd", "loner", "alienation", "dignity"]),
                )
                .and(production_year.gt(2000)),
        )
        .select(k_23ab(db).and(title))
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q23b(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q23b(db))
}
