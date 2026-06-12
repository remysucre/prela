// queries: queries.jl lines 107..413 (templates 1-5, 11-15, 22 — movie-only)

use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::min_row;
use crate::queries::sets::{murder4, nordic8, nordic10};

pub const ENTRIES: &[super::Entry] = &[
    ("2a",  "'Doc'",                                                                    q2a),
    ("2d",  "& Teller",                                                                 q2d),
    ("3b",  "300: Rise of an Empire",                                                   q3b),
    ("4a",  "5.1 || & Teller 2",                                                        q4a),
    ("13a", "Afghanistan:24 June 2012 || 1.0 || &Me",                                   q13a),
    ("11a", "Churchill Films || followed by || Batman Beyond",                          q11a),
    ("22a", "(empty)",                                                                  q22a),
    ("1a",  "(A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934", q1a),
    ("5a",  "(empty)",                                                                  q5a),
    ("12a", "10th Grade Reunion Films || 8.1 || 3:20",                                  q12a),
    ("14a", "1.0 || $lowdown",                                                          q14a),
    ("1b",  "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2008",          q1b),
    ("2b",  "'Doc'",                                                                    q2b),
    ("2c",  "(empty)",                                                                  q2c),
    ("3a",  "2 Days in New York",                                                       q3a),
    ("3c",  "& Teller 2",                                                               q3c),
    ("4b",  "9.1 || Batman: Arkham City",                                               q4b),
    ("11b", "Filmlance International AB || follows || The Money Man",                   q11b),
    ("13b", "501audio || 1.8 || 5 Time Champion",                                       q13b),
    ("1c",  "(co-production) || Intouchables || 2011",                                  q1c),
    ("1d",  "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2004",          q1d),
    ("4c",  "2.1 || & Teller 2",                                                        q4c),
    ("12b", "$10,000 || Birdemic: Shock and Terror",                                    q12b),
    ("12c", "\"Oh That Gus!\" || 7.1 || $1.11",                                         q12c),
    ("13c", "DL Sites || 1.8 || Champion",                                              q13c),
    ("14b", "6.4 || Of Dolls and Murder",                                               q14b),
    ("14c", "1.0 || $lowdown",                                                          q14c),
    ("22b", "(empty)",                                                                  q22b),
    ("22c", "(empty)",                                                                  q22c),
];

// q2a–q2d differ only in the company country code.
fn q2(cc: &'static str) -> String {
    min_row(movie().in_s(
        keyword().text().eq("character-name-in-title")
            .and(company().country().eq(cc))
    ).title())
}

fn q2a() -> String { q2("[de]") }
fn q2b() -> String { q2("[nl]") }
fn q2c() -> String { q2("[sm]") }
fn q2d() -> String { q2("[us]") }

fn q3b() -> String {
    min_row(movie().in_s(
        keyword().text().rx(r"sequel")
            .and(info().info().eq("Bulgaria")
                .and(production_year().gt(2010)))
    ).title())
}

// q4a–q4c differ only in the year cutoff and rating threshold.
fn q4(year: i64, rating: &'static str) -> String {
    min_row(movie().in_s(
        keyword().text().rx(r"sequel")
            .and(production_year().gt(year))
    ).o(
        data().in_s(
            Data::ty().text().eq("rating")
                .and(Data::text().gt(rating))
        ).text()
        .x(title())
    ))
}

fn q4a() -> String { q4(2005, "5.0") }
fn q4b() -> String { q4(2010, "9.0") }
fn q4c() -> String { q4(1990, "2.0") }

fn q13a() -> String {
    min_row(movie().in_s(
        company().in_s(
            country().eq("[de]")
                .and(Company::ty().text().eq("production companies"))
        )
            .and(kind().text().eq("movie"))
    ).o(
        info().in_s(Info::ty().text().eq("release dates")).info()
        .x(data().in_s(Data::ty().text().eq("rating")).text())
        .x(title())
    ))
}

fn q11a() -> String {
    min_row(movie().in_s(
        keyword().text().eq("sequel")
            .and(production_year().ge(1950)
                .and(production_year().le(2000)))
    ).o(
        company().in_s(
            country().ne("[pl]")
                .and(Company::name().rx(r"Film")
                    .or(Company::name().rx(r"Warner"))
                    .and(Company::ty().text().eq("production companies")))
                .minus(Company::note())
        ).name()
        .x(link().ty().text().rx(r"follow"))
        .x(title())
    ))
}

fn q22a() -> String {
    min_row(movie().in_s(
        info().in_s(
            Info::ty().text().eq("countries")
                .and(Info::info().is_in(["Germany", "German", "USA", "American"]))
        )
            .and(keyword().text().is_in(murder4())
                .and(production_year().gt(2008)
                    .and(kind().text().is_in(["movie", "episode"]))))
    ).o(
        title()
            .x(data().in_s(
                Data::text().lt("7.0")
                    .and(Data::ty().text().eq("rating"))
            ).text())
            .x(company().in_s(
                Company::note().nrx(r"\(USA\)")
                    .and(Company::note().rx(r"\(200.*\)")
                        .and(country().ne("[us]")
                            .and(Company::ty().text().eq("production companies"))))
            ).name())
    ))
}

fn q1a() -> String {
    min_row(movie()
        .in_s(data().ty().text().eq("top 250 rank"))
            .o(
                company().in_s(
                    Company::ty().text().eq("production companies")
                        .and(Company::note().nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")
                            .and(Company::note().rx(r"\(co-production\)")
                                .or(Company::note().rx(r"\(presents\)"))))
                ).note()
                .x(title())
                .x(production_year())
            ))
}

fn q5a() -> String {
    min_row(movie().in_s(
        company().in_s(
            Company::ty().text().eq("production companies")
                .and(Company::note().rx(r"\(theatrical\)")
                    .and(Company::note().rx(r"\(France\)")))
        )
            .and(info().info().is_in(nordic8())
                .and(production_year().gt(2005)))
    ).title())
}

fn q12a() -> String {
    min_row(movie().in_s(
        info().in_s(
            Info::ty().text().eq("genres")
                .and(Info::info().is_in(["Drama", "Horror"]))
        )
            .and(production_year().ge(2005)
                .and(production_year().le(2008)))
    ).o(
        company().in_s(
            country().eq("[us]")
                .and(Company::ty().text().eq("production companies"))
        ).name()
        .x(data().in_s(
            Data::ty().text().eq("rating")
                .and(Data::text().gt("8.0"))
        ).text())
        .x(title())
    ))
}

fn q14a() -> String {
    min_row(movie().in_s(
        keyword().text().is_in(murder4())
            .and(kind().text().eq("movie")
                .and(info().in_s(
                    Info::ty().text().eq("countries")
                        .and(Info::info().is_in(["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"]))
                )
                    .and(production_year().gt(2010))))
    ).o(
        data().in_s(
            Data::ty().text().eq("rating")
                .and(Data::text().lt("8.5"))
        ).text()
        .x(title())
    ))
}

fn q1b() -> String {
    min_row(movie().in_s(
        data().ty().text().eq("bottom 10 rank")
            .and(production_year().ge(2005)
                .and(production_year().le(2010)))
    ).o(
        company().in_s(
            Company::ty().text().eq("production companies")
                .and(Company::note().nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
        ).note()
        .x(title())
        .x(production_year())
    ))
}

// q3a/q3c differ only in the country list and the year cutoff.
fn q3ac(countries: Vec<&'static str>, year: i64) -> String {
    min_row(movie().in_s(
        keyword().text().rx(r"sequel")
            .and(info().info().is_in(countries)
                .and(production_year().gt(year)))
    ).title())
}

fn q3a() -> String { q3ac(nordic8(), 2005) }
fn q3c() -> String {
    q3ac(vec!["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"], 1990)
}

fn q11b() -> String {
    min_row(movie().in_s(
        keyword().text().eq("sequel")
            .and(production_year().eq(1998)
                .and(title().rx(r"Money")))
    ).o(
        company().in_s(
            country().ne("[pl]")
                .and(Company::name().rx(r"Film")
                    .or(Company::name().rx(r"Warner"))
                    .and(Company::ty().text().eq("production companies")))
                .minus(Company::note())
        ).name()
        .x(link().ty().text().rx(r"follows"))
        .x(title())
    ))
}

fn q13b() -> String {
    min_row(movie().in_s(
        kind().text().eq("movie")
            .and(info().ty().text().eq("release dates")
                .and(title().ne("")
                    .and(title().rx(r"Champion")
                        .or(title().rx(r"Loser")))))
    ).o(
        company().in_s(
            country().eq("[us]")
                .and(Company::ty().text().eq("production companies"))
        ).name()
        .x(data().in_s(Data::ty().text().eq("rating")).text())
        .x(title())
    ))
}

fn q1c() -> String {
    min_row(movie().in_s(
        data().ty().text().eq("top 250 rank")
            .and(production_year().gt(2010))
    ).o(
        company().in_s(
            Company::ty().text().eq("production companies")
                .and(Company::note().nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")
                    .and(Company::note().rx(r"\(co-production\)")))
        ).note()
        .x(title())
        .x(production_year())
    ))
}

fn q1d() -> String {
    min_row(movie().in_s(
        data().ty().text().eq("bottom 10 rank")
            .and(production_year().gt(2000))
    ).o(
        company().in_s(
            Company::ty().text().eq("production companies")
                .and(Company::note().nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
        ).note()
        .x(title())
        .x(production_year())
    ))
}

fn q12b() -> String {
    min_row(movie().in_s(
        company().in_s(
            country().eq("[us]")
                .and(Company::ty().text().is_in(["production companies", "distributors"]))
        )
            .and(data().ty().text().eq("bottom 10 rank")
                .and(production_year().gt(2000)
                    .and(title().rx(r"^Birdemic")
                        .or(title().rx(r"Movie")))))
    ).o(
        info().in_s(Info::ty().text().eq("budget")).info()
        .x(title())
    ))
}

fn q12c() -> String {
    min_row(movie().in_s(
        info().in_s(
            Info::ty().text().eq("genres")
                .and(Info::info().is_in(["Drama", "Horror", "Western", "Family"]))
        )
            .and(production_year().ge(2000)
                .and(production_year().le(2010)))
    ).o(
        company().in_s(
            country().eq("[us]")
                .and(Company::ty().text().eq("production companies"))
        ).name()
        .x(data().in_s(
            Data::ty().text().eq("rating")
                .and(Data::text().gt("7.0"))
        ).text())
        .x(title())
    ))
}

fn q13c() -> String {
    min_row(movie().in_s(
        kind().text().eq("movie")
            .and(info().ty().text().eq("release dates")
                .and(title().ne("")
                    .and(title().rx(r"^Champion")
                        .or(title().rx(r"^Loser")))))
    ).o(
        company().in_s(
            country().eq("[us]")
                .and(Company::ty().text().eq("production companies"))
        ).name()
        .x(data().in_s(Data::ty().text().eq("rating")).text())
        .x(title())
    ))
}

fn q14b() -> String {
    min_row(movie().in_s(
        keyword().text().is_in(["murder", "murder-in-title"])
            .and(kind().text().eq("movie")
                .and(info().in_s(
                    Info::ty().text().eq("countries")
                        .and(Info::info().is_in(["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"]))
                )
                    .and(production_year().gt(2010)
                        .and(title().rx(r"murder")
                            .or(title().rx(r"Murder")
                                .or(title().rx(r"Mord")))))))
    ).o(
        data().in_s(
            Data::ty().text().eq("rating")
                .and(Data::text().gt("6.0"))
        ).text()
        .x(title())
    ))
}

fn q14c() -> String {
    min_row(movie().in_s(
        keyword().text().is_in(murder4())
            .and(kind().text().is_in(["movie", "episode"])
                .and(info().in_s(
                    Info::ty().text().eq("countries")
                        .and(Info::info().is_in(nordic10()))
                )
                    .and(production_year().gt(2005))))
    ).o(
        data().in_s(
            Data::ty().text().eq("rating")
                .and(Data::text().lt("8.5"))
        ).text()
        .x(title())
    ))
}

fn q22b() -> String {
    min_row(movie().in_s(
        info().in_s(
            Info::ty().text().eq("countries")
                .and(Info::info().is_in(["Germany", "German", "USA", "American"]))
        )
            .and(keyword().text().is_in(murder4())
                .and(production_year().gt(2009)
                    .and(kind().text().is_in(["movie", "episode"]))))
    ).o(
        title()
            .x(data().in_s(
                Data::text().lt("7.0")
                    .and(Data::ty().text().eq("rating"))
            ).text())
            .x(company().in_s(
                Company::note().nrx(r"\(USA\)")
                    .and(Company::note().rx(r"\(200.*\)")
                        .and(country().ne("[us]")
                            .and(Company::ty().text().eq("production companies"))))
            ).name())
    ))
}

fn q22c() -> String {
    min_row(movie().in_s(
        info().in_s(
            Info::ty().text().eq("countries")
                .and(Info::info().is_in(nordic10()))
        )
            .and(keyword().text().is_in(murder4())
                .and(production_year().gt(2005)
                    .and(kind().text().is_in(["movie", "episode"]))))
    ).o(
        title()
            .x(data().in_s(
                Data::text().lt("8.5")
                    .and(Data::ty().text().eq("rating"))
            ).text())
            .x(company().in_s(
                Company::note().nrx(r"\(USA\)")
                    .and(Company::note().rx(r"\(200.*\)")
                        .and(country().ne("[us]")
                            .and(Company::ty().text().eq("production companies"))))
            ).name())
    ))
}
