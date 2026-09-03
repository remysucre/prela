use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn q8cd(db: &'static Job, role_: &'static str) -> impl Drive<R: Row> {
    let Movie {
        title,
        company,
        cast,
        ..
    } = &db.movie;
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast { person, role, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Person { alias, .. } = &db.person;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    db.movie.with(company.select(country).eq("[us]")).select(
        cast.with(role.select(roletype_text).eq(role_))
            .select(person)
            .select(alias)
            .select(akaname_text)
            .and(title),
    )
}

fn q8d(db: &'static Job) -> impl Drive<R: Row> {
    q8cd(db, "costume designer")
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q8d(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q8d(db))
}
