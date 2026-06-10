// queries: queries.jl lines 107..413 (templates 1-5, 11-15, 22 — movie-only)
use crate::data::Data;
use crate::engine::*;
use super::helpers::*;
use super::sets::*;

pub const ENTRIES: &[(&str, &str, fn(&Data) -> String)] = &[
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

fn q2a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k()
            .and((&d.movie_company).o((&d.company_country).eq("[de]")).k())
            .o(&d.movie_title)
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, v| update(&mut m, v));
    fmt1(m)
}

fn q2d(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k()
            .and((&d.movie_company).o((&d.company_country).eq("[us]")).k())
            .o(&d.movie_title)
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, v| update(&mut m, v));
    fmt1(m)
}

fn q3b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).rx(r"sequel").k()
            .and(
                (&d.movie_info).o((&d.info_info).eq("Bulgaria")).k()
                    .and((&d.movie_production_year).gt(2010).k())
            )
            .o(&d.movie_title)
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, v| update(&mut m, v));
    fmt1(m)
}

fn q4a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).rx(r"sequel").k()
            .and((&d.movie_production_year).gt(2005).k())
            .o(
                (&d.movie_data).o(
                    (&d.data_type).o(&d.infotype_info).eq("rating").k()
                        .and((&d.data_data).gt("5.0").k())
                        .o(&d.data_data)
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (a, b)| { update(&mut m[0], a); update(&mut m[1], b); });
    fmt2(m)
}

fn q13a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[de]").k()
                .and((&d.company_type).o(&d.companytype_kind).eq("production companies").k())
        ).k()
            .and((&d.movie_kind).o(&d.kind_kind).eq("movie").k())
            .o(
                (&d.movie_info).o(
                    (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                        .o(&d.info_info)
                )
                .x(
                    (&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .o(&d.data_data)
                    )
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut m[2], c); });
    fmt3(m)
}

fn q11a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).eq("sequel").k()
            .and(
                (&d.movie_production_year).ge(1950).k()
                    .and((&d.movie_production_year).le(2000).k())
            )
            .o(
                (&d.movie_company).o(
                    (&d.company_country).ne("[pl]").k()
                        .and(
                            (&d.company_name).rx(r"Film").k()
                                .or((&d.company_name).rx(r"Warner").k())
                                .and((&d.company_type).o(&d.companytype_kind).eq("production companies").k())
                        )
                        .minus((&d.company_note).k())
                        .o(&d.company_name)
                )
                .x(
                    (&d.movie_link).o(
                        (&d.movielink_type).o(&d.linktype_link).rx(r"follow").k()
                            .o((&d.movielink_type).o(&d.linktype_link))
                    )
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut m[2], c); });
    fmt3(m)
}

fn q22a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_info).in_s(
            (&d.info_type).o(&d.infotype_info).eq("countries").k()
                .and((&d.info_info).in_v(vec!["Germany","German","USA","American"]).k())
        ).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4()).k()
                    .and(
                        (&d.movie_production_year).gt(2008).k()
                            .and((&d.movie_kind).o(&d.kind_kind).in_v(vec!["movie","episode"]).k())
                    )
            )
            .o(
                (&d.movie_title)
                    .x(
                        (&d.movie_data).o(
                            (&d.data_data).lt("7.0").k()
                                .and((&d.data_type).o(&d.infotype_info).eq("rating").k())
                                .o(&d.data_data)
                        )
                    )
                    .x(
                        (&d.movie_company).o(
                            (&d.company_note).nrx(r"\(USA\)").k()
                                .and(
                                    (&d.company_note).rx(r"\(200.*\)").k()
                                        .and(
                                            (&d.company_country).ne("[us]").k()
                                                .and((&d.company_type).o(&d.companytype_kind).eq("production companies").k())
                                        )
                                )
                                .o(&d.company_name)
                        )
                    )
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut m[2], c); });
    fmt3(m)
}

fn q1a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("top 250 rank")).k()
            .o(
                (&d.movie_company).o(
                    (&d.company_type).o(&d.companytype_kind).eq("production companies").k()
                        .and(
                            (&d.company_note).nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)").k()
                                .and(
                                    (&d.company_note).rx(r"\(co-production\)").k()
                                        .or((&d.company_note).rx(r"\(presents\)").k())
                                )
                        )
                        .o(&d.company_note)
                )
                .x(&d.movie_title)
                .x(&d.movie_production_year)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    let mut my: Option<i64> = None;
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut my, c); });
    if m[0].is_none() { return "(empty)".into(); }
    format!("{} || {} || {}", m[0].unwrap(), m[1].unwrap(), my.unwrap())
}

fn q5a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_type).o(&d.companytype_kind).eq("production companies").k()
                .and(
                    (&d.company_note).rx(r"\(theatrical\)").k()
                        .and((&d.company_note).rx(r"\(France\)").k())
                )
        ).k()
            .and(
                (&d.movie_info).o((&d.info_info).in_v(nordic8())).k()
                    .and((&d.movie_production_year).gt(2005).k())
            )
            .o(&d.movie_title)
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, v| update(&mut m, v));
    fmt1(m)
}

fn q12a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_info).in_s(
            (&d.info_type).o(&d.infotype_info).eq("genres").k()
                .and((&d.info_info).in_v(vec!["Drama","Horror"]).k())
        ).k()
            .and(
                (&d.movie_production_year).ge(2005).k()
                    .and((&d.movie_production_year).le(2008).k())
            )
            .o(
                (&d.movie_company).o(
                    (&d.company_country).eq("[us]").k()
                        .and((&d.company_type).o(&d.companytype_kind).eq("production companies").k())
                        .o(&d.company_name)
                )
                .x(
                    (&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .and((&d.data_data).gt("8.0").k())
                            .o(&d.data_data)
                    )
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut m[2], c); });
    fmt3(m)
}

fn q14a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4()).k()
            .and(
                (&d.movie_kind).o(&d.kind_kind).eq("movie").k()
                    .and(
                        (&d.movie_info).in_s(
                            (&d.info_type).o(&d.infotype_info).eq("countries").k()
                                .and((&d.info_info).in_v(vec!["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"]).k())
                        ).k()
                            .and((&d.movie_production_year).gt(2010).k())
                    )
            )
            .o(
                (&d.movie_data).o(
                    (&d.data_type).o(&d.infotype_info).eq("rating").k()
                        .and((&d.data_data).lt("8.5").k())
                        .o(&d.data_data)
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (a, b)| { update(&mut m[0], a); update(&mut m[1], b); });
    fmt2(m)
}

fn q1b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("bottom 10 rank")).k()
            .and(
                (&d.movie_production_year).ge(2005).k()
                    .and((&d.movie_production_year).le(2010).k())
            )
            .o(
                (&d.movie_company).o(
                    (&d.company_type).o(&d.companytype_kind).eq("production companies").k()
                        .and((&d.company_note).nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)").k())
                        .o(&d.company_note)
                )
                .x(&d.movie_title)
                .x(&d.movie_production_year)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    let mut my: Option<i64> = None;
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut my, c); });
    if m[0].is_none() { return "(empty)".into(); }
    format!("{} || {} || {}", m[0].unwrap(), m[1].unwrap(), my.unwrap())
}

fn q2b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k()
            .and((&d.movie_company).o((&d.company_country).eq("[nl]")).k())
            .o(&d.movie_title)
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, v| update(&mut m, v));
    fmt1(m)
}

fn q2c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k()
            .and((&d.movie_company).o((&d.company_country).eq("[sm]")).k())
            .o(&d.movie_title)
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, v| update(&mut m, v));
    fmt1(m)
}

fn q3a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).rx(r"sequel").k()
            .and(
                (&d.movie_info).o((&d.info_info).in_v(nordic8())).k()
                    .and((&d.movie_production_year).gt(2005).k())
            )
            .o(&d.movie_title)
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, v| update(&mut m, v));
    fmt1(m)
}

fn q3c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).rx(r"sequel").k()
            .and(
                (&d.movie_info).o((&d.info_info).in_v(vec!["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"])).k()
                    .and((&d.movie_production_year).gt(1990).k())
            )
            .o(&d.movie_title)
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, v| update(&mut m, v));
    fmt1(m)
}

fn q4b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).rx(r"sequel").k()
            .and((&d.movie_production_year).gt(2010).k())
            .o(
                (&d.movie_data).o(
                    (&d.data_type).o(&d.infotype_info).eq("rating").k()
                        .and((&d.data_data).gt("9.0").k())
                        .o(&d.data_data)
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (a, b)| { update(&mut m[0], a); update(&mut m[1], b); });
    fmt2(m)
}

fn q11b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).eq("sequel").k()
            .and(
                (&d.movie_production_year).eq(1998).k()
                    .and((&d.movie_title).rx(r"Money").k())
            )
            .o(
                (&d.movie_company).o(
                    (&d.company_country).ne("[pl]").k()
                        .and(
                            (&d.company_name).rx(r"Film").k()
                                .or((&d.company_name).rx(r"Warner").k())
                                .and((&d.company_type).o(&d.companytype_kind).eq("production companies").k())
                        )
                        .minus((&d.company_note).k())
                        .o(&d.company_name)
                )
                .x(
                    (&d.movie_link).o(
                        (&d.movielink_type).o(&d.linktype_link).rx(r"follows").k()
                            .o((&d.movielink_type).o(&d.linktype_link))
                    )
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut m[2], c); });
    fmt3(m)
}

fn q13b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_kind).o(&d.kind_kind).eq("movie").k()
            .and(
                (&d.movie_info).o((&d.info_type).o(&d.infotype_info).eq("release dates")).k()
                    .and(
                        (&d.movie_title).ne("").k()
                            .and(
                                (&d.movie_title).rx(r"Champion").k()
                                    .or((&d.movie_title).rx(r"Loser").k())
                            )
                    )
            )
            .o(
                (&d.movie_company).o(
                    (&d.company_country).eq("[us]").k()
                        .and((&d.company_type).o(&d.companytype_kind).eq("production companies").k())
                        .o(&d.company_name)
                )
                .x(
                    (&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .o(&d.data_data)
                    )
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut m[2], c); });
    fmt3(m)
}

fn q1c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("top 250 rank")).k()
            .and((&d.movie_production_year).gt(2010).k())
            .o(
                (&d.movie_company).o(
                    (&d.company_type).o(&d.companytype_kind).eq("production companies").k()
                        .and(
                            (&d.company_note).nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)").k()
                                .and((&d.company_note).rx(r"\(co-production\)").k())
                        )
                        .o(&d.company_note)
                )
                .x(&d.movie_title)
                .x(&d.movie_production_year)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    let mut my: Option<i64> = None;
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut my, c); });
    if m[0].is_none() { return "(empty)".into(); }
    format!("{} || {} || {}", m[0].unwrap(), m[1].unwrap(), my.unwrap())
}

fn q1d(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("bottom 10 rank")).k()
            .and((&d.movie_production_year).gt(2000).k())
            .o(
                (&d.movie_company).o(
                    (&d.company_type).o(&d.companytype_kind).eq("production companies").k()
                        .and((&d.company_note).nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)").k())
                        .o(&d.company_note)
                )
                .x(&d.movie_title)
                .x(&d.movie_production_year)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    let mut my: Option<i64> = None;
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut my, c); });
    if m[0].is_none() { return "(empty)".into(); }
    format!("{} || {} || {}", m[0].unwrap(), m[1].unwrap(), my.unwrap())
}

fn q4c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).rx(r"sequel").k()
            .and((&d.movie_production_year).gt(1990).k())
            .o(
                (&d.movie_data).o(
                    (&d.data_type).o(&d.infotype_info).eq("rating").k()
                        .and((&d.data_data).gt("2.0").k())
                        .o(&d.data_data)
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (a, b)| { update(&mut m[0], a); update(&mut m[1], b); });
    fmt2(m)
}

fn q12b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]").k()
                .and((&d.company_type).o(&d.companytype_kind).in_v(vec!["production companies","distributors"]).k())
        ).k()
            .and(
                (&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("bottom 10 rank")).k()
                    .and(
                        (&d.movie_production_year).gt(2000).k()
                            .and(
                                (&d.movie_title).rx(r"^Birdemic").k()
                                    .or((&d.movie_title).rx(r"Movie").k())
                            )
                    )
            )
            .o(
                (&d.movie_info).o(
                    (&d.info_type).o(&d.infotype_info).eq("budget").k()
                        .o(&d.info_info)
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (a, b)| { update(&mut m[0], a); update(&mut m[1], b); });
    fmt2(m)
}

fn q12c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_info).in_s(
            (&d.info_type).o(&d.infotype_info).eq("genres").k()
                .and((&d.info_info).in_v(vec!["Drama","Horror","Western","Family"]).k())
        ).k()
            .and(
                (&d.movie_production_year).ge(2000).k()
                    .and((&d.movie_production_year).le(2010).k())
            )
            .o(
                (&d.movie_company).o(
                    (&d.company_country).eq("[us]").k()
                        .and((&d.company_type).o(&d.companytype_kind).eq("production companies").k())
                        .o(&d.company_name)
                )
                .x(
                    (&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .and((&d.data_data).gt("7.0").k())
                            .o(&d.data_data)
                    )
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut m[2], c); });
    fmt3(m)
}

fn q13c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_kind).o(&d.kind_kind).eq("movie").k()
            .and(
                (&d.movie_info).o((&d.info_type).o(&d.infotype_info).eq("release dates")).k()
                    .and(
                        (&d.movie_title).ne("").k()
                            .and(
                                (&d.movie_title).rx(r"^Champion").k()
                                    .or((&d.movie_title).rx(r"^Loser").k())
                            )
                    )
            )
            .o(
                (&d.movie_company).o(
                    (&d.company_country).eq("[us]").k()
                        .and((&d.company_type).o(&d.companytype_kind).eq("production companies").k())
                        .o(&d.company_name)
                )
                .x(
                    (&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .o(&d.data_data)
                    )
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut m[2], c); });
    fmt3(m)
}

fn q14b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).in_v(vec!["murder","murder-in-title"]).k()
            .and(
                (&d.movie_kind).o(&d.kind_kind).eq("movie").k()
                    .and(
                        (&d.movie_info).in_s(
                            (&d.info_type).o(&d.infotype_info).eq("countries").k()
                                .and((&d.info_info).in_v(vec!["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"]).k())
                        ).k()
                            .and(
                                (&d.movie_production_year).gt(2010).k()
                                    .and(
                                        (&d.movie_title).rx(r"murder").k()
                                            .or(
                                                (&d.movie_title).rx(r"Murder").k()
                                                    .or((&d.movie_title).rx(r"Mord").k())
                                            )
                                    )
                            )
                    )
            )
            .o(
                (&d.movie_data).o(
                    (&d.data_type).o(&d.infotype_info).eq("rating").k()
                        .and((&d.data_data).gt("6.0").k())
                        .o(&d.data_data)
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (a, b)| { update(&mut m[0], a); update(&mut m[1], b); });
    fmt2(m)
}

fn q14c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4()).k()
            .and(
                (&d.movie_kind).o(&d.kind_kind).in_v(vec!["movie","episode"]).k()
                    .and(
                        (&d.movie_info).in_s(
                            (&d.info_type).o(&d.infotype_info).eq("countries").k()
                                .and((&d.info_info).in_v(nordic10()).k())
                        ).k()
                            .and((&d.movie_production_year).gt(2005).k())
                    )
            )
            .o(
                (&d.movie_data).o(
                    (&d.data_type).o(&d.infotype_info).eq("rating").k()
                        .and((&d.data_data).lt("8.5").k())
                        .o(&d.data_data)
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (a, b)| { update(&mut m[0], a); update(&mut m[1], b); });
    fmt2(m)
}

fn q22b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_info).in_s(
            (&d.info_type).o(&d.infotype_info).eq("countries").k()
                .and((&d.info_info).in_v(vec!["Germany","German","USA","American"]).k())
        ).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4()).k()
                    .and(
                        (&d.movie_production_year).gt(2009).k()
                            .and((&d.movie_kind).o(&d.kind_kind).in_v(vec!["movie","episode"]).k())
                    )
            )
            .o(
                (&d.movie_title)
                    .x(
                        (&d.movie_data).o(
                            (&d.data_data).lt("7.0").k()
                                .and((&d.data_type).o(&d.infotype_info).eq("rating").k())
                                .o(&d.data_data)
                        )
                    )
                    .x(
                        (&d.movie_company).o(
                            (&d.company_note).nrx(r"\(USA\)").k()
                                .and(
                                    (&d.company_note).rx(r"\(200.*\)").k()
                                        .and(
                                            (&d.company_country).ne("[us]").k()
                                                .and((&d.company_type).o(&d.companytype_kind).eq("production companies").k())
                                        )
                                )
                                .o(&d.company_name)
                        )
                    )
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut m[2], c); });
    fmt3(m)
}

fn q22c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_info).in_s(
            (&d.info_type).o(&d.infotype_info).eq("countries").k()
                .and((&d.info_info).in_v(nordic10()).k())
        ).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4()).k()
                    .and(
                        (&d.movie_production_year).gt(2005).k()
                            .and((&d.movie_kind).o(&d.kind_kind).in_v(vec!["movie","episode"]).k())
                    )
            )
            .o(
                (&d.movie_title)
                    .x(
                        (&d.movie_data).o(
                            (&d.data_data).lt("8.5").k()
                                .and((&d.data_type).o(&d.infotype_info).eq("rating").k())
                                .o(&d.data_data)
                        )
                    )
                    .x(
                        (&d.movie_company).o(
                            (&d.company_note).nrx(r"\(USA\)").k()
                                .and(
                                    (&d.company_note).rx(r"\(200.*\)").k()
                                        .and(
                                            (&d.company_country).ne("[us]").k()
                                                .and((&d.company_type).o(&d.companytype_kind).eq("production companies").k())
                                        )
                                )
                                .o(&d.company_name)
                        )
                    )
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((a, b), c)| { update(&mut m[0], a); update(&mut m[1], b); update(&mut m[2], c); });
    fmt3(m)
}
