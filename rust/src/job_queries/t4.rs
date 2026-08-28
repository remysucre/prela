// queries: queries.jl lines 757-856 (templates 16-18)

use crate::engine::*;
use crate::job_queries::helpers::{Row, min_row};
use crate::job_queries::sets::{genre6, writer5};
use crate::job_schema::*;

pub const ENTRIES: &[super::Entry] = &[
    (
        "16a",
        "Adams, Stan || Carol Burnett vs. Anthony Perkins",
        |db| min_row(q16a(db)),
    ),
    ("16b", "!!!, Toy || & Teller", |db| min_row(q16b(db))),
    ("16c", "\"Brooklyn\" Tony Danza || (#1.5)", |db| {
        min_row(q16c(db))
    }),
    ("16d", "\"Brooklyn\" Tony Danza || (#1.5)", |db| {
        min_row(q16d(db))
    }),
    ("17a", "B, Khaz", |db| min_row(q17a(db))),
    ("17b", "Z'Dar, Robert", |db| min_row(q17b(db))),
    ("17c", "X'Volaitis, John", |db| min_row(q17c(db))),
    ("17d", "Abrahamsson, Bertil", |db| min_row(q17d(db))),
    ("17e", "$hort, Too", |db| min_row(q17e(db))),
    ("17f", "'El Galgo PornoStar', Blanquito", |db| {
        min_row(q17f(db))
    }),
    ("18a", "$1,000 || 10 || 40 Days and 40 Nights", |db| {
        min_row(q18a(db))
    }),
    ("18b", "Horror || 8.1 || Agorable", |db| min_row(q18b(db))),
    ("18c", "Action || 10 || #PostModem", |db| min_row(q18c(db))),
];

// q16a/q16d differ only in the episode_nr lower bound.
fn q16ad(db: &'static Job, lo: i64) -> impl Drive<R: Row> {
    let Movie {
        title,
        episode_nr,
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast { person, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person { alias, .. } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword.select(keyword_text).eq("character-name-in-title"))
                .and(episode_nr.ge(lo))
                .and(episode_nr.lt(100)),
        )
        .select(
            cast.select(person)
                .select(alias)
                .select(akaname_text)
                .and(title),
        )
}

fn q16a(db: &'static Job) -> impl Drive<R: Row> {
    q16ad(db, 50)
}
fn q16d(db: &'static Job) -> impl Drive<R: Row> {
    q16ad(db, 5)
}

fn q16b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast { person, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person { alias, .. } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword.select(keyword_text).eq("character-name-in-title")),
        )
        .select(
            cast.select(person)
                .select(alias)
                .select(akaname_text)
                .and(title),
        )
}

fn q16c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        episode_nr,
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast { person, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person { alias, .. } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword.select(keyword_text).eq("character-name-in-title"))
                .and(episode_nr.lt(100)),
        )
        .select(
            cast.select(person)
                .select(alias)
                .select(akaname_text)
                .and(title),
        )
}

fn q17a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person {
        name: person_name, ..
    } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword.select(keyword_text).eq("character-name-in-title")),
        )
        .select(cast.select(person).select(person_name).rx(r"^B"))
}

// q17b/c/d/f differ only in the person-name regex.
fn q17_any_co(db: &'static Job, re: &str) -> impl Drive<R: Row> {
    let Movie {
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person {
        name: person_name, ..
    } = &db.person;
    let movie = db.movie.all();
    movie
        .with(company.and(keyword.select(keyword_text).eq("character-name-in-title")))
        .select(cast.select(person).select(person_name).rx(re))
}

fn q17b(db: &'static Job) -> impl Drive<R: Row> {
    q17_any_co(db, r"^Z")
}
fn q17c(db: &'static Job) -> impl Drive<R: Row> {
    q17_any_co(db, r"^X")
}
fn q17d(db: &'static Job) -> impl Drive<R: Row> {
    q17_any_co(db, r"Bert")
}
fn q17f(db: &'static Job) -> impl Drive<R: Row> {
    q17_any_co(db, r"B")
}

fn q17e(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person {
        name: person_name, ..
    } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword.select(keyword_text).eq("character-name-in-title")),
        )
        .select(cast.select(person).select(person_name))
}

fn ib_18a(db: &'static Job) -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    let Movie { info, .. } = &db.movie;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    info.with(info_ty.select(infotype_text).eq("budget"))
        .select(info_info)
}

fn q18a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title, cast, data, ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            ib_18a(db).and(
                cast.select(
                    cast_note
                        .is_in(["(producer)", "(executive producer)"])
                        .and(person.select(gender.eq("m").and(person_name.rx(r"Tim")))),
                ),
            ),
        )
        .select(
            ib_18a(db)
                .and(
                    data.with(data_ty.select(infotype_text).eq("votes"))
                        .select(data_text),
                )
                .and(title),
        )
}

// Conjunct/diff tree (∧ = Prod, - = Diff) — consumed via `member` only, so
// the value type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_18b(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
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
    info_ty
        .select(infotype_text)
        .eq("genres")
        .and(info_info.is_in(["Horror", "Thriller"]))
        .minus(info_note)
}

fn q18b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        cast,
        info,
        data,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Person { gender, .. } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            info.with(gf_18b(db))
                .and(production_year.ge(2008))
                .and(production_year.le(2014))
                .and(
                    cast.select(
                        cast_note
                            .is_in(writer5())
                            .and(person.select(gender.eq("f"))),
                    ),
                ),
        )
        .select(
            info.with(gf_18b(db))
                .select(info_info)
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

fn gf_18c(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    info_ty
        .select(infotype_text)
        .eq("genres")
        .and(info_info.is_in(genre6()))
}

fn q18c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        cast,
        info,
        data,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Person { gender, .. } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            info.with(gf_18c(db)).and(
                cast.select(
                    cast_note
                        .is_in(writer5())
                        .and(person.select(gender.eq("m"))),
                ),
            ),
        )
        .select(
            info.with(gf_18c(db))
                .select(info_info)
                .and(
                    data.with(data_ty.select(infotype_text).eq("votes"))
                        .select(data_text),
                )
                .and(title),
        )
}
