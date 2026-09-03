use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::murder4;
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn co_28(db: &'static Job) -> impl Query<R = Id<Company>, D = Id<Movie>> + Drive + Probe {
    let Movie { company, .. } = &db.movie;
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    company.with(
        country
            .ne("[us]")
            .and(company_note.nrx(r"\(USA\)"))
            .and(company_note.rx(r"\(200.*\)")),
    )
}

fn dt_28b(db: &'static Job) -> impl Query<R = Id<Data>, D = Id<Movie>> + Drive + Probe {
    let Movie { data, .. } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    data.with(
        data_ty
            .select(infotype_text)
            .eq("rating")
            .and(data_text.gt("6.5")),
    )
}

fn q28b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        info,
        complete_cast,
        ..
    } = &db.movie;
    let CompCastType {
        text: compcasttype_text,
        ..
    } = &db.comp_cast_type;
    let Company {
        name: company_name, ..
    } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Data {
        text: data_text, ..
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
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .eq("crew")
                        .and(status.select(compcasttype_text).ne("complete+verified")),
                )
                .and(co_28(db))
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("countries")
                            .and(info_info.is_in(["Sweden", "Germany", "Swedish", "German"])),
                    ),
                )
                .and(dt_28b(db))
                .and(keyword.select(keyword_text).is_in(murder4()))
                .and(kind.select(kind_text).is_in(["movie", "episode"]))
                .and(production_year.gt(2005)),
        )
        .select(
            co_28(db)
                .select(company_name)
                .and(dt_28b(db).select(data_text))
                .and(title),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q28b(db), 3))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q28b(db))
}
