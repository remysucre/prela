// queries: queries.jl lines 107..413 (templates 1-5, 11-15, 22 — movie-only)
use crate::data::Data;
use crate::engine::*;
use super::helpers::*;
use super::sets::*;

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
fn q2(d: &Data, country: &'static str) -> String {
    let q = d.movie.in_s(
        (&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title")
            .and((&d.movie_company).o((&d.company_country).eq(country)))
    ).o(&d.movie_title);
    min_row(q)
}

fn q2a(d: &Data) -> String { q2(d, "[de]") }
fn q2b(d: &Data) -> String { q2(d, "[nl]") }
fn q2c(d: &Data) -> String { q2(d, "[sm]") }
fn q2d(d: &Data) -> String { q2(d, "[us]") }

fn q3b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_keyword).o(&d.keyword_keyword).rx(r"sequel")
            .and(
                (&d.movie_info).o((&d.info_info).eq("Bulgaria"))
                    .and((&d.movie_production_year).gt(2010))
            )
    ).o(&d.movie_title);
    min_row(q)
}

// q4a–q4c differ only in the year cutoff and rating threshold.
fn q4(d: &Data, year: i64, rating: &'static str) -> String {
    let q = d.movie.in_s(
        (&d.movie_keyword).o(&d.keyword_keyword).rx(r"sequel")
            .and((&d.movie_production_year).gt(year))
    ).o(
        (&d.movie_data).in_s(
            (&d.data_type).o(&d.infotype_info).eq("rating")
                .and((&d.data_data).gt(rating))
        ).o(&d.data_data)
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q4a(d: &Data) -> String { q4(d, 2005, "5.0") }
fn q4b(d: &Data) -> String { q4(d, 2010, "9.0") }
fn q4c(d: &Data) -> String { q4(d, 1990, "2.0") }

fn q13a(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[de]")
                .and((&d.company_type).o(&d.companytype_kind).eq("production companies"))
        )
            .and((&d.movie_kind).o(&d.kind_kind).eq("movie"))
    ).o(
        (&d.movie_info).in_s((&d.info_type).o(&d.infotype_info).eq("release dates")).o(&d.info_info)
        .x(
            (&d.movie_data).in_s((&d.data_type).o(&d.infotype_info).eq("rating")).o(&d.data_data)
        )
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q11a(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_keyword).o(&d.keyword_keyword).eq("sequel")
            .and(
                (&d.movie_production_year).ge(1950)
                    .and((&d.movie_production_year).le(2000))
            )
    ).o(
        (&d.movie_company).in_s(
            (&d.company_country).ne("[pl]")
                .and(
                    (&d.company_name).rx(r"Film")
                        .or((&d.company_name).rx(r"Warner"))
                        .and((&d.company_type).o(&d.companytype_kind).eq("production companies"))
                )
                .minus(&d.company_note)
        ).o(&d.company_name)
        .x(
            (&d.movie_link).o((&d.movielink_type).o(&d.linktype_link).rx(r"follow"))
        )
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q22a(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_info).in_s(
            (&d.info_type).o(&d.infotype_info).eq("countries")
                .and((&d.info_info).in_v(vec!["Germany","German","USA","American"]))
        )
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4())
                    .and(
                        (&d.movie_production_year).gt(2008)
                            .and((&d.movie_kind).o(&d.kind_kind).in_v(vec!["movie","episode"]))
                    )
            )
    ).o(
        (&d.movie_title)
            .x(
                (&d.movie_data).in_s(
                    (&d.data_data).lt("7.0")
                        .and((&d.data_type).o(&d.infotype_info).eq("rating"))
                ).o(&d.data_data)
            )
            .x(
                (&d.movie_company).in_s(
                    (&d.company_note).nrx(r"\(USA\)")
                        .and(
                            (&d.company_note).rx(r"\(200.*\)")
                                .and(
                                    (&d.company_country).ne("[us]")
                                        .and((&d.company_type).o(&d.companytype_kind).eq("production companies"))
                                )
                        )
                ).o(&d.company_name)
            )
    );
    min_row(q)
}

fn q1a(d: &Data) -> String {
    let q = d.movie
        .in_s((&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("top 250 rank")))
            .o(
                (&d.movie_company).in_s(
                    (&d.company_type).o(&d.companytype_kind).eq("production companies")
                        .and(
                            (&d.company_note).nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")
                                .and(
                                    (&d.company_note).rx(r"\(co-production\)")
                                        .or((&d.company_note).rx(r"\(presents\)"))
                                )
                        )
                ).o(&d.company_note)
                .x(&d.movie_title)
                .x(&d.movie_production_year)
            );
    min_row(q)
}

fn q5a(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).in_s(
            (&d.company_type).o(&d.companytype_kind).eq("production companies")
                .and(
                    (&d.company_note).rx(r"\(theatrical\)")
                        .and((&d.company_note).rx(r"\(France\)"))
                )
        )
            .and(
                (&d.movie_info).o((&d.info_info).in_v(nordic8()))
                    .and((&d.movie_production_year).gt(2005))
            )
    ).o(&d.movie_title);
    min_row(q)
}

fn q12a(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_info).in_s(
            (&d.info_type).o(&d.infotype_info).eq("genres")
                .and((&d.info_info).in_v(vec!["Drama","Horror"]))
        )
            .and(
                (&d.movie_production_year).ge(2005)
                    .and((&d.movie_production_year).le(2008))
            )
    ).o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]")
                .and((&d.company_type).o(&d.companytype_kind).eq("production companies"))
        ).o(&d.company_name)
        .x(
            (&d.movie_data).in_s(
                (&d.data_type).o(&d.infotype_info).eq("rating")
                    .and((&d.data_data).gt("8.0"))
            ).o(&d.data_data)
        )
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q14a(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4())
            .and(
                (&d.movie_kind).o(&d.kind_kind).eq("movie")
                    .and(
                        (&d.movie_info).in_s(
                            (&d.info_type).o(&d.infotype_info).eq("countries")
                                .and((&d.info_info).in_v(vec!["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"]))
                        )
                            .and((&d.movie_production_year).gt(2010))
                    )
            )
    ).o(
        (&d.movie_data).in_s(
            (&d.data_type).o(&d.infotype_info).eq("rating")
                .and((&d.data_data).lt("8.5"))
        ).o(&d.data_data)
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q1b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("bottom 10 rank"))
            .and(
                (&d.movie_production_year).ge(2005)
                    .and((&d.movie_production_year).le(2010))
            )
    ).o(
        (&d.movie_company).in_s(
            (&d.company_type).o(&d.companytype_kind).eq("production companies")
                .and((&d.company_note).nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
        ).o(&d.company_note)
        .x(&d.movie_title)
        .x(&d.movie_production_year)
    );
    min_row(q)
}

// q3a/q3c differ only in the country list and the year cutoff.
fn q3ac(d: &Data, countries: Vec<&'static str>, year: i64) -> String {
    let q = d.movie.in_s(
        (&d.movie_keyword).o(&d.keyword_keyword).rx(r"sequel")
            .and(
                (&d.movie_info).o((&d.info_info).in_v(countries))
                    .and((&d.movie_production_year).gt(year))
            )
    ).o(&d.movie_title);
    min_row(q)
}

fn q3a(d: &Data) -> String { q3ac(d, nordic8(), 2005) }
fn q3c(d: &Data) -> String {
    q3ac(d, vec!["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"], 1990)
}

fn q11b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_keyword).o(&d.keyword_keyword).eq("sequel")
            .and(
                (&d.movie_production_year).eq(1998)
                    .and((&d.movie_title).rx(r"Money"))
            )
    ).o(
        (&d.movie_company).in_s(
            (&d.company_country).ne("[pl]")
                .and(
                    (&d.company_name).rx(r"Film")
                        .or((&d.company_name).rx(r"Warner"))
                        .and((&d.company_type).o(&d.companytype_kind).eq("production companies"))
                )
                .minus(&d.company_note)
        ).o(&d.company_name)
        .x(
            (&d.movie_link).o((&d.movielink_type).o(&d.linktype_link).rx(r"follows"))
        )
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q13b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_kind).o(&d.kind_kind).eq("movie")
            .and(
                (&d.movie_info).o((&d.info_type).o(&d.infotype_info).eq("release dates"))
                    .and(
                        (&d.movie_title).ne("")
                            .and(
                                (&d.movie_title).rx(r"Champion")
                                    .or((&d.movie_title).rx(r"Loser"))
                            )
                    )
            )
    ).o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]")
                .and((&d.company_type).o(&d.companytype_kind).eq("production companies"))
        ).o(&d.company_name)
        .x(
            (&d.movie_data).in_s((&d.data_type).o(&d.infotype_info).eq("rating")).o(&d.data_data)
        )
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q1c(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("top 250 rank"))
            .and((&d.movie_production_year).gt(2010))
    ).o(
        (&d.movie_company).in_s(
            (&d.company_type).o(&d.companytype_kind).eq("production companies")
                .and(
                    (&d.company_note).nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")
                        .and((&d.company_note).rx(r"\(co-production\)"))
                )
        ).o(&d.company_note)
        .x(&d.movie_title)
        .x(&d.movie_production_year)
    );
    min_row(q)
}

fn q1d(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("bottom 10 rank"))
            .and((&d.movie_production_year).gt(2000))
    ).o(
        (&d.movie_company).in_s(
            (&d.company_type).o(&d.companytype_kind).eq("production companies")
                .and((&d.company_note).nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
        ).o(&d.company_note)
        .x(&d.movie_title)
        .x(&d.movie_production_year)
    );
    min_row(q)
}

fn q12b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]")
                .and((&d.company_type).o(&d.companytype_kind).in_v(vec!["production companies","distributors"]))
        )
            .and(
                (&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("bottom 10 rank"))
                    .and(
                        (&d.movie_production_year).gt(2000)
                            .and(
                                (&d.movie_title).rx(r"^Birdemic")
                                    .or((&d.movie_title).rx(r"Movie"))
                            )
                    )
            )
    ).o(
        (&d.movie_info).in_s((&d.info_type).o(&d.infotype_info).eq("budget")).o(&d.info_info)
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q12c(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_info).in_s(
            (&d.info_type).o(&d.infotype_info).eq("genres")
                .and((&d.info_info).in_v(vec!["Drama","Horror","Western","Family"]))
        )
            .and(
                (&d.movie_production_year).ge(2000)
                    .and((&d.movie_production_year).le(2010))
            )
    ).o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]")
                .and((&d.company_type).o(&d.companytype_kind).eq("production companies"))
        ).o(&d.company_name)
        .x(
            (&d.movie_data).in_s(
                (&d.data_type).o(&d.infotype_info).eq("rating")
                    .and((&d.data_data).gt("7.0"))
            ).o(&d.data_data)
        )
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q13c(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_kind).o(&d.kind_kind).eq("movie")
            .and(
                (&d.movie_info).o((&d.info_type).o(&d.infotype_info).eq("release dates"))
                    .and(
                        (&d.movie_title).ne("")
                            .and(
                                (&d.movie_title).rx(r"^Champion")
                                    .or((&d.movie_title).rx(r"^Loser"))
                            )
                    )
            )
    ).o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]")
                .and((&d.company_type).o(&d.companytype_kind).eq("production companies"))
        ).o(&d.company_name)
        .x(
            (&d.movie_data).in_s((&d.data_type).o(&d.infotype_info).eq("rating")).o(&d.data_data)
        )
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q14b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_keyword).o(&d.keyword_keyword).in_v(vec!["murder","murder-in-title"])
            .and(
                (&d.movie_kind).o(&d.kind_kind).eq("movie")
                    .and(
                        (&d.movie_info).in_s(
                            (&d.info_type).o(&d.infotype_info).eq("countries")
                                .and((&d.info_info).in_v(vec!["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"]))
                        )
                            .and(
                                (&d.movie_production_year).gt(2010)
                                    .and(
                                        (&d.movie_title).rx(r"murder")
                                            .or(
                                                (&d.movie_title).rx(r"Murder")
                                                    .or((&d.movie_title).rx(r"Mord"))
                                            )
                                    )
                            )
                    )
            )
    ).o(
        (&d.movie_data).in_s(
            (&d.data_type).o(&d.infotype_info).eq("rating")
                .and((&d.data_data).gt("6.0"))
        ).o(&d.data_data)
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q14c(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4())
            .and(
                (&d.movie_kind).o(&d.kind_kind).in_v(vec!["movie","episode"])
                    .and(
                        (&d.movie_info).in_s(
                            (&d.info_type).o(&d.infotype_info).eq("countries")
                                .and((&d.info_info).in_v(nordic10()))
                        )
                            .and((&d.movie_production_year).gt(2005))
                    )
            )
    ).o(
        (&d.movie_data).in_s(
            (&d.data_type).o(&d.infotype_info).eq("rating")
                .and((&d.data_data).lt("8.5"))
        ).o(&d.data_data)
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q22b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_info).in_s(
            (&d.info_type).o(&d.infotype_info).eq("countries")
                .and((&d.info_info).in_v(vec!["Germany","German","USA","American"]))
        )
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4())
                    .and(
                        (&d.movie_production_year).gt(2009)
                            .and((&d.movie_kind).o(&d.kind_kind).in_v(vec!["movie","episode"]))
                    )
            )
    ).o(
        (&d.movie_title)
            .x(
                (&d.movie_data).in_s(
                    (&d.data_data).lt("7.0")
                        .and((&d.data_type).o(&d.infotype_info).eq("rating"))
                ).o(&d.data_data)
            )
            .x(
                (&d.movie_company).in_s(
                    (&d.company_note).nrx(r"\(USA\)")
                        .and(
                            (&d.company_note).rx(r"\(200.*\)")
                                .and(
                                    (&d.company_country).ne("[us]")
                                        .and((&d.company_type).o(&d.companytype_kind).eq("production companies"))
                                )
                        )
                ).o(&d.company_name)
            )
    );
    min_row(q)
}

fn q22c(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_info).in_s(
            (&d.info_type).o(&d.infotype_info).eq("countries")
                .and((&d.info_info).in_v(nordic10()))
        )
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4())
                    .and(
                        (&d.movie_production_year).gt(2005)
                            .and((&d.movie_kind).o(&d.kind_kind).in_v(vec!["movie","episode"]))
                    )
            )
    ).o(
        (&d.movie_title)
            .x(
                (&d.movie_data).in_s(
                    (&d.data_data).lt("8.5")
                        .and((&d.data_type).o(&d.infotype_info).eq("rating"))
                ).o(&d.data_data)
            )
            .x(
                (&d.movie_company).in_s(
                    (&d.company_note).nrx(r"\(USA\)")
                        .and(
                            (&d.company_note).rx(r"\(200.*\)")
                                .and(
                                    (&d.company_country).ne("[us]")
                                        .and((&d.company_type).o(&d.companytype_kind).eq("production companies"))
                                )
                        )
                ).o(&d.company_name)
            )
    );
    min_row(q)
}
