use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_queries::sets::link3;
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn qlink_33a(db: &'static Job) -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    let Movie {
        kind,
        production_year,
        company,
        data,
        link,
        ..
    } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Kind {
        text: kind_text, ..
    } = &db.kind;
    let LinkType {
        text: linktype_text,
        ..
    } = &db.link_type;
    let MovieLink {
        target,
        ty: movielink_ty,
        ..
    } = &db.movie_link;
    link.with(
        movielink_ty.select(linktype_text).is_in(link3()).and(
            target.with(
                kind.select(kind_text)
                    .eq("tv series")
                    .and(company)
                    .and(
                        data.with(
                            data_ty
                                .select(infotype_text)
                                .eq("rating")
                                .and(data_text.lt("3.0")),
                        ),
                    )
                    .and(production_year.ge(2005))
                    .and(production_year.le(2008)),
            ),
        ),
    )
}

fn q33a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Kind {
        text: kind_text, ..
    } = &db.kind;
    let MovieLink { target, .. } = &db.movie_link;
    db.movie
        .with(
            kind.select(kind_text)
                .eq("tv series")
                .and(company.select(country).eq("[us]"))
                .and(qlink_33a(db)),
        )
        .select(
            company
                .with(country.eq("[us]"))
                .select(company_name)
                .and(
                    qlink_33a(db)
                        .select(target)
                        .select(company)
                        .select(company_name),
                )
                .and(
                    data.with(data_ty.select(infotype_text).eq("rating"))
                        .select(data_text),
                )
                .and(
                    qlink_33a(db).select(target).select(
                        data.with(
                            data_ty
                                .select(infotype_text)
                                .eq("rating")
                                .and(data_text.lt("3.0")),
                        )
                        .select(data_text),
                    ),
                )
                .and(title)
                .and(qlink_33a(db).select(target).select(title)),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q33a(db), 6))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q33a(db))
}
