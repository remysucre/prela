// queries: 19a-26c (queries.jl lines 859-1111)

use super::helpers::{film_or_warner_co, follow_link};
use crate::engine::*;
use crate::job_queries::helpers::{Row, min_row};
use crate::job_queries::sets::{genre6, kw7, kw8, kw10, nordic8, nordic9, voice4, writer5};
use crate::job_schema::*;

fn k_23ab(db: &'static Job) -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    let Movie { kind, .. } = &db.movie;
    let Kind {
        text: kind_text, ..
    } = &db.kind;
    kind.select(kind_text).eq("movie")
}

fn k_23c(db: &'static Job) -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    let Movie { kind, .. } = &db.movie;
    let Kind {
        text: kind_text, ..
    } = &db.kind;
    kind.select(kind_text)
        .is_in(["movie", "tv movie", "video movie", "video game"])
}

// Conjunct trees (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_25ab(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
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
        .and(info_info.eq("Horror"))
}

fn gf_25c(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
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

pub const ENTRIES: &[super::Entry] = &[
    ("19a", "Angeline, Moriah || Blue Harvest", |db| {
        min_row(q19a(db))
    }),
    ("19b", "Jolie, Angelina || Kung Fu Panda", |db| {
        min_row(q19b(db))
    }),
    (
        "19c",
        "Alborg, Ana Esther || .hack//Akusei heni vol. 2",
        |db| min_row(q19c(db)),
    ),
    ("19d", "Aaron, Caroline || $9.99", |db| min_row(q19d(db))),
    ("20a", "Disaster Movie", |db| min_row(q20a(db))),
    ("20b", "Iron Man", |db| min_row(q20b(db))),
    ("20c", "Abell, Alistair || ...And Then I...", |db| {
        min_row(q20c(db))
    }),
    (
        "21a",
        "Det Danske Filminstitut || followed by || Der Serienkiller - Klinge des Todes",
        |db| min_row(q21a(db)),
    ),
    (
        "21b",
        "Filmlance International AB || followed by || Hämndens pris",
        |db| min_row(q21b(db)),
    ),
    (
        "21c",
        "Churchill Films || followed by || Batman Beyond",
        |db| min_row(q21c(db)),
    ),
    ("23a", "movie || The Analysts", |db| min_row(q23a(db))),
    ("23b", "movie || The Big Mope", |db| min_row(q23b(db))),
    ("23c", "movie || Dirt Merchant", |db| min_row(q23c(db))),
    (
        "24a",
        "Additional Voices || Baker, Andrea || Baiohazâdo 6",
        |db| min_row(q24a(db)),
    ),
    (
        "24b",
        "Tigress || Jolie, Angelina || Kung Fu Panda 2",
        |db| min_row(q24b(db)),
    ),
    (
        "25a",
        "Horror || 10 || -- And Now the Screaming Starts! || Abdallah, Damon",
        |db| min_row(q25a(db)),
    ),
    (
        "25b",
        "Horror || 138 || Vampire Boys || Campbell, Jeremiah",
        |db| min_row(q25b(db)),
    ),
    ("25c", "Action || 10 || $ || Aakeson, Kim Fupz", |db| {
        min_row(q25c(db))
    }),
    (
        "26a",
        "'Agua' Man || Acereda, Hermie || 7.1 || 3:10 to Yuma",
        |db| min_row(q26a(db)),
    ),
    ("26b", "Bank Manager || 8.2 || Inception", |db| {
        min_row(q26b(db))
    }),
    ("26c", "'Agua' Man || 1.9 || 12 Rounds", |db| {
        min_row(q26c(db))
    }),
];

fn q19a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(
                    country
                        .eq("[us]")
                        .and(company_note.rx(r"\(USA\)|\(worldwide\)")),
                )
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*200|^USA:.*200")),
                    ),
                )
                .and(production_year.ge(2005))
                .and(production_year.le(2009)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.select(roletype_text).eq("actress"))
                    .and(character)
                    .and(person.with(gender.eq("f").and(person_name.rx(r"Ang")).and(alias))),
            )
            .select(person)
            .select(person_name)
            .and(title),
        )
}

fn q19b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(
                    country
                        .eq("[us]")
                        .and(company_note.rx(r"\(200.*\)"))
                        .and(company_note.rx(r"\(USA\)|\(worldwide\)")),
                )
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*2007|^USA:.*2008")),
                    ),
                )
                .and(production_year.ge(2007))
                .and(production_year.le(2008))
                .and(title.rx(r"Kung.*Fu.*Panda")),
        )
        .select(
            cast.with(
                cast_note
                    .eq("(voice)")
                    .and(role.select(roletype_text).eq("actress"))
                    .and(character)
                    .and(person.with(gender.eq("f").and(person_name.rx(r"Angel")).and(alias))),
            )
            .select(person)
            .select(person_name)
            .and(title),
        )
}

fn q19c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*200|^USA:.*200")),
                    ),
                )
                .and(production_year.gt(2000)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.select(roletype_text).eq("actress"))
                    .and(character)
                    .and(person.with(gender.eq("f").and(person_name.rx(r"An")).and(alias))),
            )
            .select(person)
            .select(person_name)
            .and(title),
        )
}

fn q19d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    let Info { ty: info_ty, .. } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(
                    info.select(info_ty)
                        .select(infotype_text)
                        .eq("release dates"),
                )
                .and(production_year.gt(2000)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.select(roletype_text).eq("actress"))
                    .and(character)
                    .and(person.with(gender.eq("f").and(alias))),
            )
            .select(person)
            .select(person_name)
            .and(title),
        )
}

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
    let movie = db.movie.all();
    movie
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

fn q20b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        cast,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person, character, ..
    } = &db.cast;
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
    let Person {
        name: person_name, ..
    } = &db.person;
    let movie = db.movie.all();
    movie
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
                .and(production_year.gt(2000))
                .and(
                    cast.select(
                        character
                            .select(
                                character_text
                                    .nrx(r"Sherlock")
                                    .and(character_text.rx(r"Tony.*Stark|Iron.*Man")),
                            )
                            .and(person.select(person_name).rx(r"Downey.*Robert")),
                    ),
                ),
        )
        .select(title)
}

fn q20c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        cast,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person, character, ..
    } = &db.cast;
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
    let Person {
        name: person_name, ..
    } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .eq("cast")
                        .and(status.select(compcasttype_text).rx(r"complete")),
                )
                .and(keyword.select(keyword_text).is_in(kw10()))
                .and(kind.select(kind_text).eq("movie"))
                .and(production_year.gt(2000)),
        )
        .select(
            cast.with(character.select(character_text).rx(r"[Mm]an"))
                .select(person)
                .select(person_name)
                .and(title),
        )
}

// q21a/b/c differ only in the country list and year range.
//
// `co` appears ONLY in the select, like q11a's: the select is a join, so a
// movie with no Film/Warner company simply probes to no row — repeating it as
// a `with` conjunct filters nothing extra and costs a company walk plus the
// `Film|Warner` regex on all 2.5M movies. What is left in `with` is ordered
// cheapest-and-most-selective first: the keyword test alone cuts the drive to
// a few thousand movies, so the year, country and link tests run on almost
// nothing.
//
// Bare `link` leads: only 6.4k of the 2.5M movies have one, against ~10k for
// the keyword, and its test is a CSR-emptiness check rather than a walk of
// the movie's keyword ids. `lk` in the select makes it redundant, so it costs
// nothing but the ordering.
fn q21(db: &'static Job, countries: Vec<&'static str>, ylo: i64, yhi: i64) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name, ..
    } = &db.company;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let movie = db.movie.all();
    movie
        .with(
            link.and(keyword.select(keyword_text).eq("sequel"))
                .and(production_year.between(ylo, yhi))
                .and(info.select(info_info).is_in(countries))
                .and(follow_link(db)),
        )
        .select(
            film_or_warner_co(db)
                .select(company_name)
                .and(follow_link(db))
                .and(title),
        )
}

fn q21a(db: &'static Job) -> impl Drive<R: Row> {
    q21(db, nordic8(), 1950, 2000)
}
fn q21b(db: &'static Job) -> impl Drive<R: Row> {
    q21(db, vec!["Germany", "German"], 2000, 2010)
}
fn q21c(db: &'static Job) -> impl Drive<R: Row> {
    q21(db, nordic9(), 1950, 2010)
}

fn q23a(db: &'static Job) -> impl Drive<R: Row> {
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
    let movie = db.movie.all();
    movie
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
                            .and(info_info.rx(r"^USA:.* 199|^USA:.* 200")),
                    ),
                )
                .and(k_23ab(db))
                .and(keyword)
                .and(production_year.gt(2000)),
        )
        .select(k_23ab(db).and(title))
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
    let movie = db.movie.all();
    movie
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

fn q23c(db: &'static Job) -> impl Drive<R: Row> {
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
    let movie = db.movie.all();
    movie
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
                            .and(info_info.rx(r"^USA:.* 199|^USA:.* 200")),
                    ),
                )
                .and(k_23c(db))
                .and(keyword)
                .and(production_year.gt(1990)),
        )
        .select(k_23c(db).and(title))
}

fn q24a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Character {
        text: character_text,
        ..
    } = &db.character;
    let Company { country, .. } = &db.company;
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
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*201|^USA:.*201")),
                    ),
                )
                .and(keyword.select(keyword_text).is_in([
                    "hero",
                    "martial-arts",
                    "hand-to-hand-combat",
                ]))
                .and(production_year.gt(2010)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.select(roletype_text).eq("actress"))
                    .and(person.with(gender.eq("f").and(person_name.rx(r"An")).and(alias))),
            )
            .select(
                character
                    .select(character_text)
                    .and(person.select(person_name)),
            )
            .and(title),
        )
}

fn q24b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Character {
        text: character_text,
        ..
    } = &db.character;
    let Company {
        name: company_name,
        country,
        ..
    } = &db.company;
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
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(
                    country
                        .eq("[us]")
                        .and(company_name.eq("DreamWorks Animation")),
                )
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*201|^USA:.*201")),
                    ),
                )
                .and(keyword.select(keyword_text).is_in([
                    "hero",
                    "martial-arts",
                    "hand-to-hand-combat",
                    "computer-animated-movie",
                ]))
                .and(production_year.gt(2010))
                .and(title.rx(r"^Kung Fu Panda")),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.select(roletype_text).eq("actress"))
                    .and(person.with(gender.eq("f").and(person_name.rx(r"An")).and(alias))),
            )
            .select(
                character
                    .select(character_text)
                    .and(person.select(person_name)),
            )
            .and(title),
        )
}

fn q25a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
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
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            info.with(gf_25ab(db))
                .and(keyword.select(keyword_text).is_in([
                    "murder",
                    "blood",
                    "gore",
                    "death",
                    "female-nudity",
                ])),
        )
        .select(
            info.with(gf_25ab(db))
                .select(info_info)
                .and(
                    data.with(data_ty.select(infotype_text).eq("votes"))
                        .select(data_text),
                )
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()).and(person.with(gender.eq("m"))))
                        .select(person)
                        .select(person_name),
                ),
        )
}

fn q25b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
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
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            info.with(gf_25ab(db))
                .and(keyword.select(keyword_text).is_in([
                    "murder",
                    "blood",
                    "gore",
                    "death",
                    "female-nudity",
                ]))
                .and(production_year.gt(2010))
                .and(title.rx(r"^Vampire")),
        )
        .select(
            info.with(gf_25ab(db))
                .select(info_info)
                .and(
                    data.with(data_ty.select(infotype_text).eq("votes"))
                        .select(data_text),
                )
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()).and(person.with(gender.eq("m"))))
                        .select(person)
                        .select(person_name),
                ),
        )
}

fn q25c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
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
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            info.with(gf_25c(db))
                .and(keyword.select(keyword_text).is_in(kw7())),
        )
        .select(
            info.with(gf_25c(db))
                .select(info_info)
                .and(
                    data.with(data_ty.select(infotype_text).eq("votes"))
                        .select(data_text),
                )
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()).and(person.with(gender.eq("m"))))
                        .select(person)
                        .select(person_name),
                ),
        )
}

fn q26a(db: &'static Job) -> impl Drive<R: Row> {
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
    let Cast {
        person, character, ..
    } = &db.cast;
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
    let Person {
        name: person_name, ..
    } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .eq("cast")
                        .and(status.select(compcasttype_text).rx(r"complete")),
                )
                .and(keyword.select(keyword_text).is_in(kw10()))
                .and(kind.select(kind_text).eq("movie"))
                .and(production_year.gt(2000)),
        )
        .select(
            cast.with(character.select(character_text).rx(r"[Mm]an"))
                .select(
                    character
                        .select(character_text)
                        .and(person.select(person_name)),
                )
                .and(
                    data.with(
                        data_ty
                            .select(infotype_text)
                            .eq("rating")
                            .and(data_text.gt("7.0")),
                    )
                    .select(data_text),
                )
                .and(title),
        )
}

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
    let movie = db.movie.all();
    movie
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

fn q26c(db: &'static Job) -> impl Drive<R: Row> {
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
    let movie = db.movie.all();
    let rd = data
        .with(data_ty.select(infotype_text).eq("rating"))
        .select(data_text);
    movie
        .with(
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .eq("cast")
                        .and(status.select(compcasttype_text).rx(r"complete")),
                )
                .and(keyword.select(keyword_text).is_in(kw10()))
                .and(kind.select(kind_text).eq("movie"))
                .and(production_year.gt(2000)),
        )
        .select(
            cast.with(character.select(character_text).rx(r"[Mm]an"))
                .select(character)
                .select(character_text)
                .and(rd)
                .and(title),
        )
}
