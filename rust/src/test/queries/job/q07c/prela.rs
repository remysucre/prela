use crate::engine::*;
use crate::job_queries::helpers::{Row, min_result, min_row};
use crate::job_schema::*;
use crate::test::result::ResultSet;

fn bio_filter_7c(db: &'static Job) -> impl Query<D = Id<PersonInfo>> + Probe {
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let PersonInfo {
        ty: personinfo_ty,
        note: personinfo_note,
        ..
    } = &db.person_info;
    personinfo_ty
        .select(infotype_text)
        .eq("mini biography")
        .and(personinfo_note)
}

fn q7c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        production_year,
        cast,
        linked_by,
        ..
    } = &db.movie;
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast { person, .. } = &db.cast;
    let LinkType {
        text: linktype_text,
        ..
    } = &db.link_type;
    let MovieLink {
        ty: movielink_ty, ..
    } = &db.movie_link;
    let Person {
        name: person_name,
        gender,
        alias,
        bio,
        name_pcode_cf,
        ..
    } = &db.person;
    let PersonInfo {
        info: personinfo_info,
        ..
    } = &db.person_info;
    db.movie
        .with(production_year.ge(1980).and(production_year.le(2010)).and(
            linked_by.select(movielink_ty).select(linktype_text).is_in([
                "references",
                "referenced in",
                "features",
                "featured in",
            ]),
        ))
        .select(
            cast.select(
                person
                    .with(
                        alias
                            .select(akaname_text)
                            .rx(r"a|^A")
                            .and(name_pcode_cf.ge("A"))
                            .and(name_pcode_cf.le("F"))
                            // m ∨ (f ∧ name~^A), spelled {m,f} ∖ (f ∖ ^A):
                            // ∨ is member-only and can't sit inside a probed ∧-tree.
                            .and(
                                gender
                                    .is_in(["m", "f"])
                                    .minus(gender.eq("f").minus(person_name.rx(r"^A"))),
                            )
                            .and(bio.with(bio_filter_7c(db))),
                    )
                    .select(person_name.and(bio.with(bio_filter_7c(db)).select(personinfo_info))),
            ),
        )
}

pub(super) fn run(db: &'static Job) -> Result<ResultSet, String> {
    Ok(min_result(q7c(db), 2))
}

pub(super) fn run_text(db: &'static Job) -> String {
    min_row(q7c(db))
}
