// queries: queries.jl lines 107..413 (templates 1-5, 11-15, 22 — movie-only)

use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::{min_row, Row};
use crate::queries::sets::{murder4, nordic8, nordic10};

pub const ENTRIES: &[super::Entry] = &[
    ("2a",  "'Doc'",                                                                    || min_row(q2a())),
    ("2d",  "& Teller",                                                                 || min_row(q2d())),
    ("3b",  "300: Rise of an Empire",                                                   || min_row(q3b())),
    ("4a",  "5.1 || & Teller 2",                                                        || min_row(q4a())),
    ("13a", "Afghanistan:24 June 2012 || 1.0 || &Me",                                   || min_row(q13a())),
    ("11a", "Churchill Films || followed by || Batman Beyond",                          || min_row(q11a())),
    ("22a", "(empty)",                                                                  || min_row(q22a())),
    ("1a",  "(A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934", || min_row(q1a())),
    ("5a",  "(empty)",                                                                  || min_row(q5a())),
    ("12a", "10th Grade Reunion Films || 8.1 || 3:20",                                  || min_row(q12a())),
    ("14a", "1.0 || $lowdown",                                                          || min_row(q14a())),
    ("1b",  "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2008",          || min_row(q1b())),
    ("2b",  "'Doc'",                                                                    || min_row(q2b())),
    ("2c",  "(empty)",                                                                  || min_row(q2c())),
    ("3a",  "2 Days in New York",                                                       || min_row(q3a())),
    ("3c",  "& Teller 2",                                                               || min_row(q3c())),
    ("4b",  "9.1 || Batman: Arkham City",                                               || min_row(q4b())),
    ("11b", "Filmlance International AB || follows || The Money Man",                   || min_row(q11b())),
    ("13b", "501audio || 1.8 || 5 Time Champion",                                       || min_row(q13b())),
    ("1c",  "(co-production) || Intouchables || 2011",                                  || min_row(q1c())),
    ("1d",  "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2004",          || min_row(q1d())),
    ("4c",  "2.1 || & Teller 2",                                                        || min_row(q4c())),
    ("12b", "$10,000 || Birdemic: Shock and Terror",                                    || min_row(q12b())),
    ("12c", "\"Oh That Gus!\" || 7.1 || $1.11",                                         || min_row(q12c())),
    ("13c", "DL Sites || 1.8 || Champion",                                              || min_row(q13c())),
    ("14b", "6.4 || Of Dolls and Murder",                                               || min_row(q14b())),
    ("14c", "1.0 || $lowdown",                                                          || min_row(q14c())),
    ("22b", "(empty)",                                                                  || min_row(q22b())),
    ("22c", "(empty)",                                                                  || min_row(q22c())),
];

// q2a–q2d differ only in the company country code.
fn q2(cc: &'static str) -> impl Drive<R: Row> {
    movie
        .when(keyword.text().eq("character-name-in-title")
         .and(company.country().eq(cc)))
        .title()
}

fn q2a() -> impl Drive<R: Row> { q2("[de]") }
fn q2b() -> impl Drive<R: Row> { q2("[nl]") }
fn q2c() -> impl Drive<R: Row> { q2("[sm]") }
fn q2d() -> impl Drive<R: Row> { q2("[us]") }

fn q3b() -> impl Drive<R: Row> {
    movie
        .when(keyword.text().rx(r"sequel")
         .and(info.info().eq("Bulgaria"))
         .and(production_year.gt(2010)))
        .title()
}

// q4a–q4c differ only in the year cutoff and rating threshold.
fn q4(year: i64, rating: &'static str) -> impl Drive<R: Row> {
    movie
        .when(keyword.text().rx(r"sequel")
         .and(production_year.gt(year)))
        .select(data.when(Data::ty.text().eq("rating")
                  .and(Data::text.gt(rating))).text()
         .and(title))
}

fn q4a() -> impl Drive<R: Row> { q4(2005, "5.0") }
fn q4b() -> impl Drive<R: Row> { q4(2010, "9.0") }
fn q4c() -> impl Drive<R: Row> { q4(1990, "2.0") }

fn q13a() -> impl Drive<R: Row> {
    movie
        .when(company.select(country.eq("[de]")
                      .and(Company::ty.text().eq("production companies")))
         .and(kind.text().eq("movie")))
        .select(info.when(Info::ty.text().eq("release dates")).info()
         .and(data.when(Data::ty.text().eq("rating")).text())
         .and(title))
}

fn q11a() -> impl Drive<R: Row> {
    movie
        .when(keyword.text().eq("sequel")
         .and(production_year.ge(1950))
         .and(production_year.le(2000)))
        .select(company.when(country.ne("[pl]")
                     .and(Company::name.rx(r"Film").or(Company::name.rx(r"Warner")))
                     .and(Company::ty.text().eq("production companies"))
                     .minus(Company::note)).name()
         .and(link.ty().text().rx(r"follow"))
         .and(title))
}

fn q22a() -> impl Drive<R: Row> {
    movie
        .when(info.select(Info::ty.text().eq("countries")
                   .and(Info::info.is_in(["Germany", "German", "USA", "American"])))
         .and(keyword.text().is_in(murder4()))
         .and(production_year.gt(2008))
         .and(kind.text().is_in(["movie", "episode"])))
        .select(title
         .and(data.when(Data::text.lt("7.0")
                   .and(Data::ty.text().eq("rating"))).text())
         .and(company.when(Company::note.nrx(r"\(USA\)")
                      .and(Company::note.rx(r"\(200.*\)"))
                      .and(country.ne("[us]"))
                      .and(Company::ty.text().eq("production companies"))).name()))
}

fn q1a() -> impl Drive<R: Row> {
    movie
        .when(data.ty().text().eq("top 250 rank"))
        .select(company.when(Company::ty.text().eq("production companies")
                     .and(Company::note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
                     .and(Company::note.rx(r"\(co-production\)")
                      .or(Company::note.rx(r"\(presents\)")))).note()
         .and(title)
         .and(production_year))
}

fn q5a() -> impl Drive<R: Row> {
    movie
        .when(company.select(Company::ty.text().eq("production companies")
                      .and(Company::note.rx(r"\(theatrical\)"))
                      .and(Company::note.rx(r"\(France\)")))
         .and(info.info().is_in(nordic8()))
         .and(production_year.gt(2005)))
        .title()
}

fn q12a() -> impl Drive<R: Row> {
    movie
        .when(info.select(Info::ty.text().eq("genres")
                   .and(Info::info.is_in(["Drama", "Horror"])))
         .and(production_year.ge(2005))
         .and(production_year.le(2008)))
        .select(company.when(country.eq("[us]")
                     .and(Company::ty.text().eq("production companies"))).name()
         .and(data.when(Data::ty.text().eq("rating")
                   .and(Data::text.gt("8.0"))).text())
         .and(title))
}

fn q14a() -> impl Drive<R: Row> {
    movie
        .when(keyword.text().is_in(murder4())
         .and(kind.text().eq("movie"))
         .and(info.select(Info::ty.text().eq("countries")
                   .and(Info::info.is_in(["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"]))))
         .and(production_year.gt(2010)))
        .select(data.when(Data::ty.text().eq("rating")
                  .and(Data::text.lt("8.5"))).text()
         .and(title))
}

fn q1b() -> impl Drive<R: Row> {
    movie
        .when(data.ty().text().eq("bottom 10 rank")
         .and(production_year.ge(2005))
         .and(production_year.le(2010)))
        .select(company.when(Company::ty.text().eq("production companies")
                     .and(Company::note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))).note()
         .and(title)
         .and(production_year))
}

// q3a/q3c differ only in the country list and the year cutoff.
fn q3ac(countries: Vec<&'static str>, year: i64) -> impl Drive<R: Row> {
    movie
        .when(keyword.text().rx(r"sequel")
         .and(info.info().is_in(countries))
         .and(production_year.gt(year)))
        .title()
}

fn q3a() -> impl Drive<R: Row> { q3ac(nordic8(), 2005) }
fn q3c() -> impl Drive<R: Row> {
    q3ac(vec!["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"], 1990)
}

fn q11b() -> impl Drive<R: Row> {
    movie
        .when(keyword.text().eq("sequel")
         .and(production_year.eq(1998))
         .and(title.rx(r"Money")))
        .select(company.when(country.ne("[pl]")
                     .and(Company::name.rx(r"Film").or(Company::name.rx(r"Warner")))
                     .and(Company::ty.text().eq("production companies"))
                     .minus(Company::note)).name()
         .and(link.ty().text().rx(r"follows"))
         .and(title))
}

fn q13b() -> impl Drive<R: Row> {
    movie
        .when(kind.text().eq("movie")
         .and(info.ty().text().eq("release dates"))
         .and(title.ne(""))
         .and(title.rx(r"Champion").or(title.rx(r"Loser"))))
        .select(company.when(country.eq("[us]")
                     .and(Company::ty.text().eq("production companies"))).name()
         .and(data.when(Data::ty.text().eq("rating")).text())
         .and(title))
}

fn q1c() -> impl Drive<R: Row> {
    movie
        .when(data.ty().text().eq("top 250 rank")
         .and(production_year.gt(2010)))
        .select(company.when(Company::ty.text().eq("production companies")
                     .and(Company::note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
                     .and(Company::note.rx(r"\(co-production\)"))).note()
         .and(title)
         .and(production_year))
}

fn q1d() -> impl Drive<R: Row> {
    movie
        .when(data.ty().text().eq("bottom 10 rank")
         .and(production_year.gt(2000)))
        .select(company.when(Company::ty.text().eq("production companies")
                     .and(Company::note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))).note()
         .and(title)
         .and(production_year))
}

fn q12b() -> impl Drive<R: Row> {
    movie
        .when(company.select(country.eq("[us]")
                      .and(Company::ty.text().is_in(["production companies", "distributors"])))
         .and(data.ty().text().eq("bottom 10 rank"))
         .and(production_year.gt(2000))
         .and(title.rx(r"^Birdemic").or(title.rx(r"Movie"))))
        .select(info.when(Info::ty.text().eq("budget")).info()
         .and(title))
}

fn q12c() -> impl Drive<R: Row> {
    movie
        .when(info.select(Info::ty.text().eq("genres")
                   .and(Info::info.is_in(["Drama", "Horror", "Western", "Family"])))
         .and(production_year.ge(2000))
         .and(production_year.le(2010)))
        .select(company.when(country.eq("[us]")
                     .and(Company::ty.text().eq("production companies"))).name()
         .and(data.when(Data::ty.text().eq("rating")
                   .and(Data::text.gt("7.0"))).text())
         .and(title))
}

fn q13c() -> impl Drive<R: Row> {
    movie
        .when(kind.text().eq("movie")
         .and(info.ty().text().eq("release dates"))
         .and(title.ne(""))
         .and(title.rx(r"^Champion").or(title.rx(r"^Loser"))))
        .select(company.when(country.eq("[us]")
                     .and(Company::ty.text().eq("production companies"))).name()
         .and(data.when(Data::ty.text().eq("rating")).text())
         .and(title))
}

fn q14b() -> impl Drive<R: Row> {
    movie
        .when(keyword.text().is_in(["murder", "murder-in-title"])
         .and(kind.text().eq("movie"))
         .and(info.select(Info::ty.text().eq("countries")
                   .and(Info::info.is_in(["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"]))))
         .and(production_year.gt(2010))
         .and(title.rx(r"murder").or(title.rx(r"Murder")).or(title.rx(r"Mord"))))
        .select(data.when(Data::ty.text().eq("rating")
                  .and(Data::text.gt("6.0"))).text()
         .and(title))
}

fn q14c() -> impl Drive<R: Row> {
    movie
        .when(keyword.text().is_in(murder4())
         .and(kind.text().is_in(["movie", "episode"]))
         .and(info.select(Info::ty.text().eq("countries")
                   .and(Info::info.is_in(nordic10()))))
         .and(production_year.gt(2005)))
        .select(data.when(Data::ty.text().eq("rating")
                  .and(Data::text.lt("8.5"))).text()
         .and(title))
}

fn q22b() -> impl Drive<R: Row> {
    movie
        .when(info.select(Info::ty.text().eq("countries")
                   .and(Info::info.is_in(["Germany", "German", "USA", "American"])))
         .and(keyword.text().is_in(murder4()))
         .and(production_year.gt(2009))
         .and(kind.text().is_in(["movie", "episode"])))
        .select(title
         .and(data.when(Data::text.lt("7.0")
                   .and(Data::ty.text().eq("rating"))).text())
         .and(company.when(Company::note.nrx(r"\(USA\)")
                      .and(Company::note.rx(r"\(200.*\)"))
                      .and(country.ne("[us]"))
                      .and(Company::ty.text().eq("production companies"))).name()))
}

fn q22c() -> impl Drive<R: Row> {
    movie
        .when(info.select(Info::ty.text().eq("countries")
                   .and(Info::info.is_in(nordic10())))
         .and(keyword.text().is_in(murder4()))
         .and(production_year.gt(2005))
         .and(kind.text().is_in(["movie", "episode"])))
        .select(title
         .and(data.when(Data::text.lt("8.5")
                   .and(Data::ty.text().eq("rating"))).text())
         .and(company.when(Company::note.nrx(r"\(USA\)")
                      .and(Company::note.rx(r"\(200.*\)"))
                      .and(country.ne("[us]"))
                      .and(Company::ty.text().eq("production companies"))).name()))
}
