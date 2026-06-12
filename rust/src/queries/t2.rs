// queries: queries.jl lines ~381-588 (22d, 5b, 5c, 15a-d, 11c-d, 13d, 6a-f)

use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::min_row;
use crate::queries::sets::{kw8, murder4, nordic10};

pub const ENTRIES: &[super::Entry] = &[
    ("22d", "(#1.1) || 2.0 || 13 Productions", q22d),
    ("5b",  "(empty)", q5b),
    ("5c",  "11,830,420", q5c),
    ("15a", "USA:1 June 2007 || Battlestar Galactica: The Resistance", q15a),
    ("15b", "USA:27 April 2007 || RoboCop vs Terminator", q15b),
    ("15c", "USA:1 April 2003 || 24: Day Six - Debrief", q15c),
    ("15d", "(Not So) Instant Photo || 06/05", q15d),
    ("11c", "20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24", q11c),
    ("11d", "13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun", q11d),
    ("13d", "\"O\" Films || 1.0 || #54 Meets #47", q13d),
    ("6a",  "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", q6a),
    ("6b",  "based-on-comic || The Avengers 2 || Downey Jr., Robert", q6b),
    ("6c",  "marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert", q6c),
    ("6d",  "based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert", q6d),
    ("6e",  "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", q6e),
    ("6f",  "based-on-comic || & Teller 2 || \"Steff\", Stefanie Oxmann Mcgaha", q6f),
];

fn q22d() -> String {
    min_row(movies().in_s(
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
                country().ne("[us]")
                    .and(Company::ty().text().eq("production companies"))
            ).name())
    ))
}

fn q5b() -> String {
    min_row(movies().in_s(
        company().in_s(
            Company::ty().text().eq("production companies")
                .and(Company::note().rx(r"\(VHS\)")
                    .and(Company::note().rx(r"\(USA\)")
                        .and(Company::note().rx(r"\(1994\)"))))
        )
            .and(info().info().is_in(["USA", "America"])
                .and(production_year().gt(2010)))
    ).title())
}

fn q5c() -> String {
    min_row(movies().in_s(
        company().in_s(
            Company::ty().text().eq("production companies")
                .and(Company::note().nrx(r"\(TV\)")
                    .and(Company::note().rx(r"\(USA\)")))
        )
            .and(info().info().is_in(nordic10())
                .and(production_year().gt(1990)))
    ).title())
}

fn q15a() -> String {
    min_row(movies().in_s(
        production_year().gt(2000)
            .and(company().in_s(
                country().eq("[us]")
                    .and(Company::note().rx(r"\(200.*\)")
                        .and(Company::note().rx(r"\(worldwide\)")))
            )
                .and(keyword()
                    .and(aka())))
    ).o(
        info().in_s(
            Info::ty().text().eq("release dates")
                .and(Info::info().rx(r"^USA:.* 200")
                    .and(Info::note().rx(r"internet")))
        ).info()
        .x(title())
    ))
}

fn q15b() -> String {
    min_row(movies().in_s(
        company().in_s(
            country().eq("[us]")
                .and(Company::name().eq("YouTube")
                    .and(Company::note().rx(r"\(200.*\)")
                        .and(Company::note().rx(r"\(worldwide\)"))))
        )
            .and(keyword()
                .and(aka()
                    .and(production_year().ge(2005)
                        .and(production_year().le(2010)))))
    ).o(
        info().in_s(
            Info::ty().text().eq("release dates")
                .and(Info::info().rx(r"^USA:.* 200")
                    .and(Info::note().rx(r"internet")))
        ).info()
        .x(title())
    ))
}

fn q15c() -> String {
    min_row(movies().in_s(
        company().country().eq("[us]")
            .and(keyword()
                .and(aka()
                    .and(production_year().gt(1990))))
    ).o(
        info().in_s(
            Info::ty().text().eq("release dates")
                .and(Info::info().rx(r"^USA:.* 199")
                    .or(Info::info().rx(r"^USA:.* 200"))
                    .and(Info::note().rx(r"internet")))
        ).info()
        .x(title())
    ))
}

fn q15d() -> String {
    min_row(movies().in_s(
        company().country().eq("[us]")
            .and(keyword()
                .and(info().in_s(
                    Info::ty().text().eq("release dates")
                        .and(Info::note().rx(r"internet"))
                )
                    .and(production_year().gt(1990))))
    ).o(
        aka().text()
            .x(title())
    ))
}

fn q11c() -> String {
    min_row(movies().in_s(
        keyword().text().is_in(["sequel", "revenge", "based-on-novel"])
            .and(production_year().gt(1950)
                .and(link()))
    ).o(
        company().in_s(
            country().ne("[pl]")
                .and(Company::name().rx(r"^20th Century Fox")
                    .or(Company::name().rx(r"^Twentieth Century Fox"))
                    .and(Company::ty().text().ne("production companies")
                        .and(Company::note())))
        ).o(Company::name().x(Company::note()))
        .x(title())
    ))
}

fn q11d() -> String {
    min_row(movies().in_s(
        keyword().text().is_in(["sequel", "revenge", "based-on-novel"])
            .and(production_year().gt(1950)
                .and(link()))
    ).o(
        company().in_s(
            country().ne("[pl]")
                .and(Company::ty().text().ne("production companies")
                    .and(Company::note()))
        ).o(Company::name().x(Company::note()))
        .x(title())
    ))
}

fn q13d() -> String {
    min_row(movies().in_s(
        kind().text().eq("movie")
            .and(info().ty().text().eq("release dates"))
    ).o(
        company().in_s(
            country().eq("[us]")
                .and(Company::ty().text().eq("production companies"))
        ).name()
        .x(data().in_s(Data::ty().text().eq("rating")).text())
        .x(title())
    ))
}

// q6a/c/e share the marvel-cinematic-universe keyword and q6b/d the kw8
// list; within each pair only the year cutoff varies.
fn q6_marvel(year: i64) -> String {
    let kw = || keyword().text().eq("marvel-cinematic-universe");
    let downey = cast().person().name().rx(r"Downey.*Robert");
    min_row(movies().in_s(production_year().gt(year).and(kw()))
        .o(kw().x(title()).x(downey)))
}

fn q6_comic(year: i64) -> String {
    let kw = || keyword().text().is_in(kw8());
    let downey = cast().person().name().rx(r"Downey.*Robert");
    min_row(movies().in_s(production_year().gt(year).and(kw()))
        .o(kw().x(title()).x(downey)))
}

fn q6a() -> String { q6_marvel(2010) }
fn q6b() -> String { q6_comic(2014) }
fn q6c() -> String { q6_marvel(2014) }
fn q6d() -> String { q6_comic(2000) }
fn q6e() -> String { q6_marvel(2000) }

fn q6f() -> String {
    let kw = || keyword().text().is_in(kw8());
    let cast_name = cast().person().name();
    min_row(movies().in_s(production_year().gt(2000).and(kw()))
        .o(kw().x(title()).x(cast_name)))
}
