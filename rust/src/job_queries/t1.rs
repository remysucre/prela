// queries: queries.jl lines 107..413 (templates 1-5, 11-15, 22 — movie-only)

use crate::engine::*;
use crate::job_queries::helpers::{Row, kw_rx, min_row};
use crate::job_queries::sets::{murder4, nordic8, nordic10};
use crate::job_schema::*;

pub const ENTRIES: &[super::Entry] = &[
    ("2a", "'Doc'", |db| min_row(q2a(db))),
    ("2d", "& Teller", |db| min_row(q2d(db))),
    ("3b", "300: Rise of an Empire", |db| min_row(q3b(db))),
    ("4a", "5.1 || & Teller 2", |db| min_row(q4a(db))),
    ("13a", "Afghanistan:24 June 2012 || 1.0 || &Me", |db| {
        min_row(q13a(db))
    }),
    (
        "11a",
        "Churchill Films || followed by || Batman Beyond",
        |db| min_row(q11a(db)),
    ),
    ("22a", "(empty)", |db| min_row(q22a(db))),
    (
        "1a",
        "(A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934",
        |db| min_row(q1a(db)),
    ),
    ("5a", "(empty)", |db| min_row(q5a(db))),
    ("12a", "10th Grade Reunion Films || 8.1 || 3:20", |db| {
        min_row(q12a(db))
    }),
    ("14a", "1.0 || $lowdown", |db| min_row(q14a(db))),
    (
        "1b",
        "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2008",
        |db| min_row(q1b(db)),
    ),
    ("2b", "'Doc'", |db| min_row(q2b(db))),
    ("2c", "(empty)", |db| min_row(q2c(db))),
    ("3a", "2 Days in New York", |db| min_row(q3a(db))),
    ("3c", "& Teller 2", |db| min_row(q3c(db))),
    ("4b", "9.1 || Batman: Arkham City", |db| min_row(q4b(db))),
    (
        "11b",
        "Filmlance International AB || follows || The Money Man",
        |db| min_row(q11b(db)),
    ),
    ("13b", "501audio || 1.8 || 5 Time Champion", |db| {
        min_row(q13b(db))
    }),
    ("1c", "(co-production) || Intouchables || 2011", |db| {
        min_row(q1c(db))
    }),
    (
        "1d",
        "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2004",
        |db| min_row(q1d(db)),
    ),
    ("4c", "2.1 || & Teller 2", |db| min_row(q4c(db))),
    ("12b", "$10,000 || Birdemic: Shock and Terror", |db| {
        min_row(q12b(db))
    }),
    ("12c", "\"Oh That Gus!\" || 7.1 || $1.11", |db| {
        min_row(q12c(db))
    }),
    ("13c", "DL Sites || 1.8 || Champion", |db| min_row(q13c(db))),
    ("14b", "6.4 || Of Dolls and Murder", |db| min_row(q14b(db))),
    ("14c", "1.0 || $lowdown", |db| min_row(q14c(db))),
    ("22b", "(empty)", |db| min_row(q22b(db))),
    ("22c", "(empty)", |db| min_row(q22c(db))),
];

// q2a–q2d differ only in the company country code.
fn q2(db: &'static Job, cc: &'static str) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        company,
        ..
    } = &db.movie;
    let Company { country, .. } = &db.company;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    db.movie
        .with(
            keyword
                .select(keyword_text)
                .eq("character-name-in-title")
                .and(company.select(country).eq(cc)),
        )
        .select(title)
}

fn q2a(db: &'static Job) -> impl Drive<R: Row> {
    q2(db, "[de]")
}
fn q2b(db: &'static Job) -> impl Drive<R: Row> {
    q2(db, "[nl]")
}
fn q2c(db: &'static Job) -> impl Drive<R: Row> {
    q2(db, "[sm]")
}
fn q2d(db: &'static Job) -> impl Drive<R: Row> {
    q2(db, "[us]")
}

fn q3b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        ..
    } = &db.movie;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    db.movie
        .with(
            keyword
                .select(keyword_text)
                .rx(r"sequel")
                .and(info.select(info_info).eq("Bulgaria"))
                .and(production_year.gt(2010)),
        )
        .select(title)
}

// q4a–q4c differ only in the year cutoff and rating threshold.
fn q4(db: &'static Job, year: i64, rating: &'static str) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        data,
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
    db.movie
        .with(
            keyword
                .with(kw_rx(db, r"sequel"))
                .and(production_year.gt(year)),
        )
        .select(
            data.with(
                data_ty
                    .select(infotype_text)
                    .eq("rating")
                    .and(data_text.gt(rating)),
            )
            .select(data_text)
            .and(title),
        )
}

fn q4a(db: &'static Job) -> impl Drive<R: Row> {
    q4(db, 2005, "5.0")
}
fn q4b(db: &'static Job) -> impl Drive<R: Row> {
    q4(db, 2010, "9.0")
}
fn q4c(db: &'static Job) -> impl Drive<R: Row> {
    q4(db, 1990, "2.0")
}

fn q13a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
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
    let Kind {
        text: kind_text, ..
    } = &db.kind;
    db.movie
        .with(
            company
                .select(
                    country.eq("[de]").and(
                        company_ty
                            .select(companytype_text)
                            .eq("production companies"),
                    ),
                )
                .and(kind.select(kind_text).eq("movie")),
        )
        .select(
            info.with(info_ty.select(infotype_text).eq("release dates"))
                .select(info_info)
                .and(
                    data.with(data_ty.select(infotype_text).eq("rating"))
                        .select(data_text),
                )
                .and(title),
        )
}

fn q11a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let LinkType {
        text: linktype_text,
        ..
    } = &db.link_type;
    let MovieLink {
        ty: movielink_ty, ..
    } = &db.movie_link;
    db.movie
        .with(
            link.and(keyword.select(keyword_text).eq("sequel"))
                .and(production_year.between(1950, 2000)),
        )
        .select(
            company
                .with(
                    country
                        .ne("[pl]")
                        .and(company_name.rx(r"Film|Warner"))
                        .and(
                            company_ty
                                .select(companytype_text)
                                .eq("production companies"),
                        )
                        .minus(company_note),
                )
                .select(company_name)
                .and(
                    link.select(movielink_ty)
                        .select(linktype_text)
                        .rx(r"follow"),
                )
                .and(title),
        )
}

fn q22a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
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
            info.select(
                info_ty
                    .select(infotype_text)
                    .eq("countries")
                    .and(info_info.is_in(["Germany", "German", "USA", "American"])),
            )
            .and(keyword.select(keyword_text).is_in(murder4()))
            .and(production_year.gt(2008))
            .and(kind.select(kind_text).is_in(["movie", "episode"])),
        )
        .select(
            title
                .and(
                    data.with(
                        data_text
                            .lt("7.0")
                            .and(data_ty.select(infotype_text).eq("rating")),
                    )
                    .select(data_text),
                )
                .and(
                    company
                        .with(
                            company_note
                                .nrx(r"\(USA\)")
                                .and(company_note.rx(r"\(200.*\)"))
                                .and(country.ne("[us]"))
                                .and(
                                    company_ty
                                        .select(companytype_text)
                                        .eq("production companies"),
                                ),
                        )
                        .select(company_name),
                ),
        )
}

fn q1a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Data { ty: data_ty, .. } = &db.data;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    db.movie
        .with(
            data.select(data_ty)
                .select(infotype_text)
                .eq("top 250 rank"),
        )
        .select(
            company
                .with(
                    company_ty
                        .select(companytype_text)
                        .eq("production companies")
                        .and(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
                        .and(company_note.rx(r"\(co-production\)|\(presents\)")),
                )
                .select(company_note)
                .and(title)
                .and(production_year),
        )
}

fn q5a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        info,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Info {
        info: info_info, ..
    } = &db.info;
    db.movie
        .with(
            company
                .select(
                    company_ty
                        .select(companytype_text)
                        .eq("production companies")
                        .and(company_note.rx(r"\(theatrical\)"))
                        .and(company_note.rx(r"\(France\)")),
                )
                .and(info.select(info_info).is_in(nordic8()))
                .and(production_year.gt(2005)),
        )
        .select(title)
}

fn q12a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
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
    db.movie
        .with(
            info.select(
                info_ty
                    .select(infotype_text)
                    .eq("genres")
                    .and(info_info.is_in(["Drama", "Horror"])),
            )
            .and(production_year.ge(2005))
            .and(production_year.le(2008)),
        )
        .select(
            company
                .with(
                    country.eq("[us]").and(
                        company_ty
                            .select(companytype_text)
                            .eq("production companies"),
                    ),
                )
                .select(company_name)
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

fn q14a(db: &'static Job) -> impl Drive<R: Row> {
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
                .and(kind.select(kind_text).eq("movie"))
                .and(
                    info.select(info_ty.select(infotype_text).eq("countries").and(
                        info_info.is_in([
                            "Sweden",
                            "Norway",
                            "Germany",
                            "Denmark",
                            "Swedish",
                            "Denish",
                            "Norwegian",
                            "German",
                            "USA",
                            "American",
                        ]),
                    )),
                )
                .and(production_year.gt(2010)),
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

fn q1b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Data { ty: data_ty, .. } = &db.data;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    db.movie
        .with(
            data.select(data_ty)
                .select(infotype_text)
                .eq("bottom 10 rank")
                .and(production_year.ge(2005))
                .and(production_year.le(2010)),
        )
        .select(
            company
                .with(
                    company_ty
                        .select(companytype_text)
                        .eq("production companies")
                        .and(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")),
                )
                .select(company_note)
                .and(title)
                .and(production_year),
        )
}

// q3a/q3c differ only in the country list and the year cutoff.
fn q3ac(db: &'static Job, countries: Vec<&'static str>, year: i64) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        ..
    } = &db.movie;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    db.movie
        .with(
            keyword
                .select(keyword_text)
                .rx(r"sequel")
                .and(info.select(info_info).is_in(countries))
                .and(production_year.gt(year)),
        )
        .select(title)
}

fn q3a(db: &'static Job) -> impl Drive<R: Row> {
    q3ac(db, nordic8(), 2005)
}
fn q3c(db: &'static Job) -> impl Drive<R: Row> {
    q3ac(
        db,
        vec![
            "Sweden",
            "Norway",
            "Germany",
            "Denmark",
            "Swedish",
            "Denish",
            "Norwegian",
            "German",
            "USA",
            "American",
        ],
        1990,
    )
}

fn q11b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let LinkType {
        text: linktype_text,
        ..
    } = &db.link_type;
    let MovieLink {
        ty: movielink_ty, ..
    } = &db.movie_link;
    db.movie
        .with(
            link.and(keyword.select(keyword_text).eq("sequel"))
                .and(production_year.eq(1998))
                .and(title.rx(r"Money")),
        )
        .select(
            company
                .with(
                    country
                        .ne("[pl]")
                        .and(company_name.rx(r"Film|Warner"))
                        .and(
                            company_ty
                                .select(companytype_text)
                                .eq("production companies"),
                        )
                        .minus(company_note),
                )
                .select(company_name)
                .and(
                    link.select(movielink_ty)
                        .select(linktype_text)
                        .rx(r"follows"),
                )
                .and(title),
        )
}

fn q13b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info { ty: info_ty, .. } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Kind {
        text: kind_text, ..
    } = &db.kind;
    db.movie
        .with(
            kind.select(kind_text)
                .eq("movie")
                .and(
                    info.select(info_ty)
                        .select(infotype_text)
                        .eq("release dates"),
                )
                .and(title.ne(""))
                .and(title.rx(r"Champion|Loser")),
        )
        .select(
            company
                .with(
                    country.eq("[us]").and(
                        company_ty
                            .select(companytype_text)
                            .eq("production companies"),
                    ),
                )
                .select(company_name)
                .and(
                    data.with(data_ty.select(infotype_text).eq("rating"))
                        .select(data_text),
                )
                .and(title),
        )
}

fn q1c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Data { ty: data_ty, .. } = &db.data;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    db.movie
        .with(
            data.select(data_ty)
                .select(infotype_text)
                .eq("top 250 rank")
                .and(production_year.gt(2010)),
        )
        .select(
            company
                .with(
                    company_ty
                        .select(companytype_text)
                        .eq("production companies")
                        .and(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
                        .and(company_note.rx(r"\(co-production\)")),
                )
                .select(company_note)
                .and(title)
                .and(production_year),
        )
}

fn q1d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Data { ty: data_ty, .. } = &db.data;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    db.movie
        .with(
            data.select(data_ty)
                .select(infotype_text)
                .eq("bottom 10 rank")
                .and(production_year.gt(2000)),
        )
        .select(
            company
                .with(
                    company_ty
                        .select(companytype_text)
                        .eq("production companies")
                        .and(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")),
                )
                .select(company_note)
                .and(title)
                .and(production_year),
        )
}

fn q12b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Data { ty: data_ty, .. } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    db.movie
        .with(
            company
                .select(
                    country.eq("[us]").and(
                        company_ty
                            .select(companytype_text)
                            .is_in(["production companies", "distributors"]),
                    ),
                )
                .and(
                    data.select(data_ty)
                        .select(infotype_text)
                        .eq("bottom 10 rank"),
                )
                .and(production_year.gt(2000))
                .and(title.rx(r"^Birdemic|Movie")),
        )
        .select(
            info.with(info_ty.select(infotype_text).eq("budget"))
                .select(info_info)
                .and(title),
        )
}

fn q12c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
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
    db.movie
        .with(
            info.select(
                info_ty
                    .select(infotype_text)
                    .eq("genres")
                    .and(info_info.is_in(["Drama", "Horror", "Western", "Family"])),
            )
            .and(production_year.ge(2000))
            .and(production_year.le(2010)),
        )
        .select(
            company
                .with(
                    country.eq("[us]").and(
                        company_ty
                            .select(companytype_text)
                            .eq("production companies"),
                    ),
                )
                .select(company_name)
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

fn q13c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info { ty: info_ty, .. } = &db.info;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let Kind {
        text: kind_text, ..
    } = &db.kind;
    db.movie
        .with(
            kind.select(kind_text)
                .eq("movie")
                .and(
                    info.select(info_ty)
                        .select(infotype_text)
                        .eq("release dates"),
                )
                .and(title.ne(""))
                .and(title.rx(r"^Champion|^Loser")),
        )
        .select(
            company
                .with(
                    country.eq("[us]").and(
                        company_ty
                            .select(companytype_text)
                            .eq("production companies"),
                    ),
                )
                .select(company_name)
                .and(
                    data.with(data_ty.select(infotype_text).eq("rating"))
                        .select(data_text),
                )
                .and(title),
        )
}

fn q14b(db: &'static Job) -> impl Drive<R: Row> {
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
                .is_in(["murder", "murder-in-title"])
                .and(kind.select(kind_text).eq("movie"))
                .and(
                    info.select(info_ty.select(infotype_text).eq("countries").and(
                        info_info.is_in([
                            "Sweden",
                            "Norway",
                            "Germany",
                            "Denmark",
                            "Swedish",
                            "Denish",
                            "Norwegian",
                            "German",
                            "USA",
                            "American",
                        ]),
                    )),
                )
                .and(production_year.gt(2010))
                .and(title.rx(r"murder|Murder|Mord")),
        )
        .select(
            data.with(
                data_ty
                    .select(infotype_text)
                    .eq("rating")
                    .and(data_text.gt("6.0")),
            )
            .select(data_text)
            .and(title),
        )
}

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

fn q22b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
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
            info.select(
                info_ty
                    .select(infotype_text)
                    .eq("countries")
                    .and(info_info.is_in(["Germany", "German", "USA", "American"])),
            )
            .and(keyword.select(keyword_text).is_in(murder4()))
            .and(production_year.gt(2009))
            .and(kind.select(kind_text).is_in(["movie", "episode"])),
        )
        .select(
            title
                .and(
                    data.with(
                        data_text
                            .lt("7.0")
                            .and(data_ty.select(infotype_text).eq("rating")),
                    )
                    .select(data_text),
                )
                .and(
                    company
                        .with(
                            company_note
                                .nrx(r"\(USA\)")
                                .and(company_note.rx(r"\(200.*\)"))
                                .and(country.ne("[us]"))
                                .and(
                                    company_ty
                                        .select(companytype_text)
                                        .eq("production companies"),
                                ),
                        )
                        .select(company_name),
                ),
        )
}

fn q22c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let CompanyType {
        text: companytype_text,
        ..
    } = &db.company_type;
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
            info.select(
                info_ty
                    .select(infotype_text)
                    .eq("countries")
                    .and(info_info.is_in(nordic10())),
            )
            .and(keyword.select(keyword_text).is_in(murder4()))
            .and(production_year.gt(2005))
            .and(kind.select(kind_text).is_in(["movie", "episode"])),
        )
        .select(
            title
                .and(
                    data.with(
                        data_text
                            .lt("8.5")
                            .and(data_ty.select(infotype_text).eq("rating")),
                    )
                    .select(data_text),
                )
                .and(
                    company
                        .with(
                            company_note
                                .nrx(r"\(USA\)")
                                .and(company_note.rx(r"\(200.*\)"))
                                .and(country.ne("[us]"))
                                .and(
                                    company_ty
                                        .select(companytype_text)
                                        .eq("production companies"),
                                ),
                        )
                        .select(company_name),
                ),
        )
}
