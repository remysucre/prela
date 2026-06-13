// queries: queries.jl lines ~381-588 (22d, 5b, 5c, 15a-d, 11c-d, 13d, 6a-f)

use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::{min_row, Row};
use crate::queries::sets::{kw8, murder4, nordic10};

pub const ENTRIES: &[super::Entry] = &[
    ("22d", "(#1.1) || 2.0 || 13 Productions", || min_row(q22d())),
    ("5b",  "(empty)", || min_row(q5b())),
    ("5c",  "11,830,420", || min_row(q5c())),
    ("15a", "USA:1 June 2007 || Battlestar Galactica: The Resistance", || min_row(q15a())),
    ("15b", "USA:27 April 2007 || RoboCop vs Terminator", || min_row(q15b())),
    ("15c", "USA:1 April 2003 || 24: Day Six - Debrief", || min_row(q15c())),
    ("15d", "(Not So) Instant Photo || 06/05", || min_row(q15d())),
    ("11c", "20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24", || min_row(q11c())),
    ("11d", "13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun", || min_row(q11d())),
    ("13d", "\"O\" Films || 1.0 || #54 Meets #47", || min_row(q13d())),
    ("6a",  "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", || min_row(q6a())),
    ("6b",  "based-on-comic || The Avengers 2 || Downey Jr., Robert", || min_row(q6b())),
    ("6c",  "marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert", || min_row(q6c())),
    ("6d",  "based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert", || min_row(q6d())),
    ("6e",  "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", || min_row(q6e())),
    ("6f",  "based-on-comic || & Teller 2 || \"Steff\", Stefanie Oxmann Mcgaha", || min_row(q6f())),
];

fn q22d() -> impl Drive<R: Row> {
    movie
        .when(info.get(Info::ty.text().eq("countries")
                         .and(Info::info.is_in(nordic10())))
              .and(keyword.text().is_in(murder4()))
              .and(production_year.gt(2005))
              .and(kind.text().is_in(["movie", "episode"])))
        .get(title
             .and(data.when(Data::text.lt("8.5")
                              .and(Data::ty.text().eq("rating"))).text())
             .and(company.when(country.ne("[us]")
                                 .and(Company::ty.text().eq("production companies"))).name()))
}

fn q5b() -> impl Drive<R: Row> {
    movie
        .when(company.get(Company::ty.text().eq("production companies")
                            .and(Company::note.rx(r"\(VHS\)"))
                            .and(Company::note.rx(r"\(USA\)"))
                            .and(Company::note.rx(r"\(1994\)")))
              .and(info.info().is_in(["USA", "America"]))
              .and(production_year.gt(2010)))
        .title()
}

fn q5c() -> impl Drive<R: Row> {
    movie
        .when(company.get(Company::ty.text().eq("production companies")
                            .and(Company::note.nrx(r"\(TV\)"))
                            .and(Company::note.rx(r"\(USA\)")))
              .and(info.info().is_in(nordic10()))
              .and(production_year.gt(1990)))
        .title()
}

fn q15a() -> impl Drive<R: Row> {
    movie
        .when(production_year.gt(2000)
              .and(company.get(country.eq("[us]")
                                 .and(Company::note.rx(r"\(200.*\)"))
                                 .and(Company::note.rx(r"\(worldwide\)"))))
              .and(keyword)
              .and(aka))
        .get(info.when(Info::ty.text().eq("release dates")
                         .and(Info::info.rx(r"^USA:.* 200"))
                         .and(Info::note.rx(r"internet"))).info()
             .and(title))
}

fn q15b() -> impl Drive<R: Row> {
    movie
        .when(company.get(country.eq("[us]")
                            .and(Company::name.eq("YouTube"))
                            .and(Company::note.rx(r"\(200.*\)"))
                            .and(Company::note.rx(r"\(worldwide\)")))
              .and(keyword)
              .and(aka)
              .and(production_year.ge(2005))
              .and(production_year.le(2010)))
        .get(info.when(Info::ty.text().eq("release dates")
                         .and(Info::info.rx(r"^USA:.* 200"))
                         .and(Info::note.rx(r"internet"))).info()
             .and(title))
}

fn q15c() -> impl Drive<R: Row> {
    movie
        .when(company.country().eq("[us]")
              .and(keyword)
              .and(aka)
              .and(production_year.gt(1990)))
        .get(info.when(Info::ty.text().eq("release dates")
                         .and(Info::info.rx(r"^USA:.* 199")
                              .or(Info::info.rx(r"^USA:.* 200")))
                         .and(Info::note.rx(r"internet"))).info()
             .and(title))
}

fn q15d() -> impl Drive<R: Row> {
    movie
        .when(company.country().eq("[us]")
              .and(keyword)
              .and(info.get(Info::ty.text().eq("release dates")
                              .and(Info::note.rx(r"internet"))))
              .and(production_year.gt(1990)))
        .get(aka.text()
             .and(title))
}

fn q11c() -> impl Drive<R: Row> {
    movie
        .when(keyword.text().is_in(["sequel", "revenge", "based-on-novel"])
              .and(production_year.gt(1950))
              .and(link))
        .get(company.when(country.ne("[pl]")
                            .and(Company::name.rx(r"^20th Century Fox")
                                 .or(Company::name.rx(r"^Twentieth Century Fox")))
                            .and(Company::ty.text().ne("production companies"))
                            .and(Company::note)).get(Company::name.and(Company::note))
             .and(title))
}

fn q11d() -> impl Drive<R: Row> {
    movie
        .when(keyword.text().is_in(["sequel", "revenge", "based-on-novel"])
              .and(production_year.gt(1950))
              .and(link))
        .get(company.when(country.ne("[pl]")
                            .and(Company::ty.text().ne("production companies"))
                            .and(Company::note)).get(Company::name.and(Company::note))
             .and(title))
}

fn q13d() -> impl Drive<R: Row> {
    movie
        .when(kind.text().eq("movie")
              .and(info.ty().text().eq("release dates")))
        .get(company.when(country.eq("[us]")
                            .and(Company::ty.text().eq("production companies"))).name()
             .and(data.when(Data::ty.text().eq("rating")).text())
             .and(title))
}

// q6a/c/e share the marvel-cinematic-universe keyword and q6b/d the kw8
// list; within each pair only the year cutoff varies.
fn q6_marvel(year: i64) -> impl Drive<R: Row> {
    let kw = || keyword.text().eq("marvel-cinematic-universe");
    let downey = cast.person().name().rx(r"Downey.*Robert");
    movie
        .when(production_year.gt(year).and(kw()))
        .get(kw().and(title).and(downey))
}

fn q6_comic(year: i64) -> impl Drive<R: Row> {
    let kw = || keyword.text().is_in(kw8());
    let downey = cast.person().name().rx(r"Downey.*Robert");
    movie
        .when(production_year.gt(year).and(kw()))
        .get(kw().and(title).and(downey))
}

fn q6a() -> impl Drive<R: Row> { q6_marvel(2010) }
fn q6b() -> impl Drive<R: Row> { q6_comic(2014) }
fn q6c() -> impl Drive<R: Row> { q6_marvel(2014) }
fn q6d() -> impl Drive<R: Row> { q6_comic(2000) }
fn q6e() -> impl Drive<R: Row> { q6_marvel(2000) }

fn q6f() -> impl Drive<R: Row> {
    let kw = || keyword.text().is_in(kw8());
    let cast_name = cast.person().name();
    movie
        .when(production_year.gt(2000).and(kw()))
        .get(kw().and(title).and(cast_name))
}
