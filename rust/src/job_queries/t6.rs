// queries: 27a–33c (queries.jl lines 1114–1394)

use super::helpers::{film_or_warner_co, follow_link};
use crate::engine::*;
use crate::job_queries::helpers::{Row, min_row};
use crate::job_queries::sets::{
    genre6, kw7, link3, murder4, nordic9, nordic10, voice3, voice4, writer5,
};
use crate::job_schema::*;

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

fn dt_28ac(db: &'static Job) -> impl Query<R = Id<Data>, D = Id<Movie>> + Drive + Probe {
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
            .and(data_text.lt("8.5")),
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

// Conjunct trees (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_horror(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
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
        .and(info_info.is_in(["Horror", "Thriller"]))
}

fn gf_genre6(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
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

fn qlink_33b(db: &'static Job) -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
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
        movielink_ty.select(linktype_text).rx(r"follow").and(
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
                    .and(production_year.eq(2007)),
            ),
        ),
    )
}

fn qlink_33c(db: &'static Job) -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
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
                    .is_in(["tv series", "episode"])
                    .and(company)
                    .and(
                        data.with(
                            data_ty
                                .select(infotype_text)
                                .eq("rating")
                                .and(data_text.lt("3.5")),
                        ),
                    )
                    .and(production_year.ge(2000))
                    .and(production_year.le(2010)),
            ),
        ),
    )
}

pub const ENTRIES: &[super::Entry] = &[
    (
        "27a",
        "Det Danske Filminstitut || followed by || Spår i mörker",
        |db| min_row(q27a(db)),
    ),
    (
        "27b",
        "Filmlance International AB || followed by || Vita nätter",
        |db| min_row(q27b(db)),
    ),
    (
        "27c",
        "Det Danske Filminstitut || followed by || Spår i mörker",
        |db| min_row(q27c(db)),
    ),
    ("28a", "01 Distribuzione || 2.9 || (#1.1)", |db| {
        min_row(q28a(db))
    }),
    ("28b", "20th Century Fox || 6.6 || (#1.1)", |db| {
        min_row(q28b(db))
    }),
    ("28c", "01 Distribuzione || 1.9 || (#1.1)", |db| {
        min_row(q28c(db))
    }),
    ("29a", "Queen || Andrews, Julie || Shrek 2", |db| {
        min_row(q29a(db))
    }),
    ("29b", "Queen || Andrews, Julie || Shrek 2", |db| {
        min_row(q29b(db))
    }),
    ("29c", "Lola || Andrews, Julie || Hoodwinked!", |db| {
        min_row(q29c(db))
    }),
    (
        "30a",
        "Horror || 100356 || 16 Blocks || Abrams, J.J.",
        |db| min_row(q30a(db)),
    ),
    (
        "30b",
        "Horror || 194782 || Freddy vs. Jason || Shannon, Damian",
        |db| min_row(q30b(db)),
    ),
    ("30c", "Action || 100356 || $ || Abernathy, Lewis", |db| {
        min_row(q30c(db))
    }),
    (
        "31a",
        "Horror || 1040 || 2001 Maniacs || Agnew, Jim",
        |db| min_row(q31a(db)),
    ),
    (
        "31b",
        "Horror || 129755 || Saw || Bousman, Darren Lynn",
        |db| min_row(q31b(db)),
    ),
    ("31c", "Action || 1008 || 11:14 || Abraham, Brad", |db| {
        min_row(q31c(db))
    }),
    ("32a", "(empty)", |db| min_row(q32a(db))),
    (
        "32b",
        "alternate language version of || 12 oz. Mouse || 'Angel': Season 2 Overview",
        |db| min_row(q32b(db)),
    ),
    (
        "33a",
        "495 Productions || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila",
        |db| min_row(q33a(db)),
    ),
    (
        "33b",
        "MTV Netherlands || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila",
        |db| min_row(q33b(db)),
    ),
    (
        "33c",
        "2BE || 495 Productions || 1.3 || 1.0 || A Double Shot at Love || A Double Shot at Love",
        |db| min_row(q33c(db)),
    ),
];

fn q27a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        complete_cast,
        link,
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
                .and(production_year.between(1950, 2000))
                .and(info.select(info_info.is_in(["Sweden", "Germany", "Swedish", "German"])))
                .and(
                    complete_cast.select(
                        subject
                            .select(compcasttype_text)
                            .is_in(["cast", "crew"])
                            .and(status.select(compcasttype_text).eq("complete")),
                    ),
                )
                .and(follow_link(db)),
        )
        .select(
            film_or_warner_co(db)
                .select(company_name)
                .and(follow_link(db))
                .and(title),
        )
}

fn q27b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        complete_cast,
        link,
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
                .and(production_year.eq(1998))
                .and(info.select(info_info.is_in(["Sweden", "Germany", "Swedish", "German"])))
                .and(
                    complete_cast.select(
                        subject
                            .select(compcasttype_text)
                            .is_in(["cast", "crew"])
                            .and(status.select(compcasttype_text).eq("complete")),
                    ),
                )
                .and(follow_link(db)),
        )
        .select(
            film_or_warner_co(db)
                .select(company_name)
                .and(follow_link(db))
                .and(title),
        )
}

fn q27c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        complete_cast,
        link,
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
                .and(production_year.between(1950, 2010))
                .and(info.select(info_info.is_in(nordic9())))
                .and(
                    complete_cast.select(
                        subject
                            .select(compcasttype_text)
                            .eq("cast")
                            .and(status.select(compcasttype_text).rx(r"^complete")),
                    ),
                )
                .and(follow_link(db)),
        )
        .select(
            film_or_warner_co(db)
                .select(company_name)
                .and(follow_link(db))
                .and(title),
        )
}

fn q28a(db: &'static Job) -> impl Drive<R: Row> {
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
    let movie = db.movie.all();
    movie
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
                            .and(info_info.is_in(nordic10())),
                    ),
                )
                .and(dt_28ac(db))
                .and(keyword.select(keyword_text).is_in(murder4()))
                .and(kind.select(kind_text).is_in(["movie", "episode"]))
                .and(production_year.gt(2000)),
        )
        .select(
            co_28(db)
                .select(company_name)
                .and(dt_28ac(db).select(data_text))
                .and(title),
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
    let movie = db.movie.all();
    movie
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

fn q28c(db: &'static Job) -> impl Drive<R: Row> {
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
    let movie = db.movie.all();
    movie
        .with(
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .eq("cast")
                        .and(status.select(compcasttype_text).eq("complete")),
                )
                .and(co_28(db))
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("countries")
                            .and(info_info.is_in(nordic10())),
                    ),
                )
                .and(dt_28ac(db))
                .and(keyword.select(keyword_text).is_in(murder4()))
                .and(kind.select(kind_text).is_in(["movie", "episode"]))
                .and(production_year.gt(2005)),
        )
        .select(
            co_28(db)
                .select(company_name)
                .and(dt_28ac(db).select(data_text))
                .and(title),
        )
}

fn q29a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        cast,
        info,
        complete_cast,
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
    let CompCastType {
        text: compcasttype_text,
        ..
    } = &db.comp_cast_type;
    let Company { country, .. } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
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
        bio,
        ..
    } = &db.person;
    let PersonInfo {
        ty: personinfo_ty, ..
    } = &db.person_info;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
        .with(
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .eq("cast")
                        .and(status.select(compcasttype_text).eq("complete+verified")),
                )
                .and(company.select(country).eq("[us]"))
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*200|^USA:.*200")),
                    ),
                )
                .and(keyword.select(keyword_text).eq("computer-animation"))
                .and(title.eq("Shrek 2"))
                .and(production_year.ge(2000))
                .and(production_year.le(2010)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice3())
                    .and(role.select(roletype_text).eq("actress"))
                    .and(character.select(character_text).eq("Queen"))
                    .and(
                        person.with(
                            gender
                                .eq("f")
                                .and(person_name.rx(r"An"))
                                .and(alias)
                                .and(bio.select(personinfo_ty.select(infotype_text).eq("trivia"))),
                        ),
                    ),
            )
            .select(
                character
                    .select(character_text)
                    .and(person.select(person_name)),
            )
            .and(title),
        )
}

fn q29b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        cast,
        info,
        complete_cast,
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
    let CompCastType {
        text: compcasttype_text,
        ..
    } = &db.comp_cast_type;
    let Company { country, .. } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
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
        bio,
        ..
    } = &db.person;
    let PersonInfo {
        ty: personinfo_ty, ..
    } = &db.person_info;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
        .with(
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .eq("cast")
                        .and(status.select(compcasttype_text).eq("complete+verified")),
                )
                .and(company.select(country).eq("[us]"))
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("release dates")
                            .and(info_info.rx(r"^USA:.*200")),
                    ),
                )
                .and(keyword.select(keyword_text).eq("computer-animation"))
                .and(title.eq("Shrek 2"))
                .and(production_year.ge(2000))
                .and(production_year.le(2005)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice3())
                    .and(role.select(roletype_text).eq("actress"))
                    .and(character.select(character_text).eq("Queen"))
                    .and(
                        person.with(
                            gender
                                .eq("f")
                                .and(person_name.rx(r"An"))
                                .and(alias)
                                .and(bio.select(personinfo_ty.select(infotype_text).eq("height"))),
                        ),
                    ),
            )
            .select(
                character
                    .select(character_text)
                    .and(person.select(person_name)),
            )
            .and(title),
        )
}

fn q29c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        cast,
        info,
        complete_cast,
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
    let CompCastType {
        text: compcasttype_text,
        ..
    } = &db.comp_cast_type;
    let Company { country, .. } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
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
        bio,
        ..
    } = &db.person;
    let PersonInfo {
        ty: personinfo_ty, ..
    } = &db.person_info;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
        .with(
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .eq("cast")
                        .and(status.select(compcasttype_text).eq("complete+verified")),
                )
                .and(company.select(country).eq("[us]"))
                .and(
                    info.select(
                        info_ty
                            .select(infotype_text)
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*200|^USA:.*200")),
                    ),
                )
                .and(keyword.select(keyword_text).eq("computer-animation"))
                .and(production_year.ge(2000))
                .and(production_year.le(2010)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.select(roletype_text).eq("actress"))
                    .and(
                        person.with(
                            gender
                                .eq("f")
                                .and(person_name.rx(r"An"))
                                .and(alias)
                                .and(bio.select(personinfo_ty.select(infotype_text).eq("trivia"))),
                        ),
                    ),
            )
            .select(
                character
                    .select(character_text)
                    .and(person.select(person_name)),
            )
            .and(title),
        )
}

fn q30a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        cast,
        info,
        data,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
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
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .is_in(["cast", "crew"])
                        .and(status.select(compcasttype_text).eq("complete+verified")),
                )
                .and(info.with(gf_horror(db)))
                .and(keyword.select(keyword_text).is_in(kw7()))
                .and(production_year.gt(2000)),
        )
        .select(
            info.with(gf_horror(db))
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

fn q30b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        cast,
        info,
        data,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
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
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .is_in(["cast", "crew"])
                        .and(status.select(compcasttype_text).eq("complete+verified")),
                )
                .and(info.with(gf_horror(db)))
                .and(keyword.select(keyword_text).is_in(kw7()))
                .and(production_year.gt(2000))
                .and(title.rx(r"Freddy|Jason|^Saw")),
        )
        .select(
            info.with(gf_horror(db))
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

fn q30c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        cast,
        info,
        data,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
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
            complete_cast
                .select(
                    subject
                        .select(compcasttype_text)
                        .eq("cast")
                        .and(status.select(compcasttype_text).eq("complete+verified")),
                )
                .and(info.with(gf_genre6(db)))
                .and(keyword.select(keyword_text).is_in(kw7())),
        )
        .select(
            info.with(gf_genre6(db))
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

fn q31a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        company,
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
    let Company {
        name: company_name, ..
    } = &db.company;
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
            company
                .select(company_name)
                .rx(r"^Lionsgate")
                .and(info.with(gf_horror(db)))
                .and(keyword.select(keyword_text).is_in(kw7())),
        )
        .select(
            info.with(gf_horror(db))
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

fn q31b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
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
    let Company {
        name: company_name,
        note: company_note,
        ..
    } = &db.company;
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
            company
                .select(
                    company_name
                        .rx(r"^Lionsgate")
                        .and(company_note.rx(r"\(Blu-ray\)")),
                )
                .and(info.with(gf_horror(db)))
                .and(keyword.select(keyword_text).is_in(kw7()))
                .and(production_year.gt(2000))
                .and(title.rx(r"Freddy|Jason|^Saw")),
        )
        .select(
            info.with(gf_horror(db))
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

fn q31c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        company,
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
    let Company {
        name: company_name, ..
    } = &db.company;
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
        name: person_name, ..
    } = &db.person;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(company_name)
                .rx(r"^Lionsgate")
                .and(info.with(gf_genre6(db)))
                .and(keyword.select(keyword_text).is_in(kw7())),
        )
        .select(
            info.with(gf_genre6(db))
                .select(info_info)
                .and(
                    data.with(data_ty.select(infotype_text).eq("votes"))
                        .select(data_text),
                )
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()))
                        .select(person)
                        .select(person_name),
                ),
        )
}

// q32a/q32b differ only in the keyword constant.
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
    let movie = db.movie.all();
    movie
        .with(link.and(keyword.select(keyword_text).eq(kw)))
        .select(
            link.select(movielink_ty)
                .select(linktype_text)
                .and(title)
                .and(link.select(target).select(title)),
        )
}

fn q32a(db: &'static Job) -> impl Drive<R: Row> {
    q32(db, "10,000-mile-club")
}
fn q32b(db: &'static Job) -> impl Drive<R: Row> {
    q32(db, "character-name-in-title")
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
    let movie = db.movie.all();
    movie
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

fn q33b(db: &'static Job) -> impl Drive<R: Row> {
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
    let movie = db.movie.all();
    movie
        .with(
            kind.select(kind_text)
                .eq("tv series")
                .and(company.select(country).eq("[nl]"))
                .and(qlink_33b(db)),
        )
        .select(
            company
                .with(country.eq("[nl]"))
                .select(company_name)
                .and(
                    qlink_33b(db)
                        .select(target)
                        .select(company)
                        .select(company_name),
                )
                .and(
                    data.with(data_ty.select(infotype_text).eq("rating"))
                        .select(data_text),
                )
                .and(
                    qlink_33b(db).select(target).select(
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
                .and(qlink_33b(db).select(target).select(title)),
        )
}

fn q33c(db: &'static Job) -> impl Drive<R: Row> {
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
    let movie = db.movie.all();
    movie
        .with(
            kind.select(kind_text)
                .is_in(["tv series", "episode"])
                .and(company.select(country).ne("[us]"))
                .and(qlink_33c(db)),
        )
        .select(
            company
                .with(country.ne("[us]"))
                .select(company_name)
                .and(
                    qlink_33c(db)
                        .select(target)
                        .select(company)
                        .select(company_name),
                )
                .and(
                    data.with(data_ty.select(infotype_text).eq("rating"))
                        .select(data_text),
                )
                .and(
                    qlink_33c(db).select(target).select(
                        data.with(
                            data_ty
                                .select(infotype_text)
                                .eq("rating")
                                .and(data_text.lt("3.5")),
                        )
                        .select(data_text),
                    ),
                )
                .and(title)
                .and(qlink_33c(db).select(target).select(title)),
        )
}
