// queries: 27a–33c (queries.jl lines 1114–1394)

use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::min_row;
use crate::queries::sets::{genre6, kw7, link3, murder4, nordic9, nordic10, voice3, voice4, writer5};
use super::helpers::{film_or_warner_co, follow_link};

fn co_28() -> impl Query<R = Id<Company>, D = Id<Movie>> + Drive + Probe {
    company().in_s(
        country().ne("[us]")
            .and(Company::note().nrx(r"\(USA\)"))
            .and(Company::note().rx(r"\(200.*\)"))
    )
}

fn dt_28ac() -> impl Query<R = Id<Data>, D = Id<Movie>> + Drive + Probe {
    data().in_s(
        Data::ty().text().eq("rating")
            .and(Data::text().lt("8.5"))
    )
}

fn dt_28b() -> impl Query<R = Id<Data>, D = Id<Movie>> + Drive + Probe {
    data().in_s(
        Data::ty().text().eq("rating")
            .and(Data::text().gt("6.5"))
    )
}

// Conjunct trees (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_horror() -> impl Query<D = Id<Info>> + Probe {
    Info::ty().text().eq("genres")
        .and(Info::info().is_in(["Horror", "Thriller"]))
}

fn gf_genre6() -> impl Query<D = Id<Info>> + Probe {
    Info::ty().text().eq("genres")
        .and(Info::info().is_in(genre6()))
}

fn qlink_33a() -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    link().in_s(
        MovieLink::ty().text().is_in(link3())
            .and(target().in_s(
                kind().text().eq("tv series")
                    .and(company())
                    .and(data().in_s(
                        Data::ty().text().eq("rating")
                            .and(Data::text().lt("3.0"))
                    ))
                    .and(production_year().ge(2005))
                    .and(production_year().le(2008))
            ))
    )
}

fn qlink_33b() -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    link().in_s(
        MovieLink::ty().text().rx(r"follow")
            .and(target().in_s(
                kind().text().eq("tv series")
                    .and(company())
                    .and(data().in_s(
                        Data::ty().text().eq("rating")
                            .and(Data::text().lt("3.0"))
                    ))
                    .and(production_year().eq(2007))
            ))
    )
}

fn qlink_33c() -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    link().in_s(
        MovieLink::ty().text().is_in(link3())
            .and(target().in_s(
                kind().text().is_in(["tv series", "episode"])
                    .and(company())
                    .and(data().in_s(
                        Data::ty().text().eq("rating")
                            .and(Data::text().lt("3.5"))
                    ))
                    .and(production_year().ge(2000))
                    .and(production_year().le(2010))
            ))
    )
}

pub const ENTRIES: &[super::Entry] = &[
    ("27a", "Det Danske Filminstitut || followed by || Spår i mörker", q27a),
    ("27b", "Filmlance International AB || followed by || Vita nätter", q27b),
    ("27c", "Det Danske Filminstitut || followed by || Spår i mörker", q27c),
    ("28a", "01 Distribuzione || 2.9 || (#1.1)", q28a),
    ("28b", "20th Century Fox || 6.6 || (#1.1)", q28b),
    ("28c", "01 Distribuzione || 1.9 || (#1.1)", q28c),
    ("29a", "Queen || Andrews, Julie || Shrek 2", q29a),
    ("29b", "Queen || Andrews, Julie || Shrek 2", q29b),
    ("29c", "Lola || Andrews, Julie || Hoodwinked!", q29c),
    ("30a", "Horror || 100356 || 16 Blocks || Abrams, J.J.", q30a),
    ("30b", "Horror || 194782 || Freddy vs. Jason || Shannon, Damian", q30b),
    ("30c", "Action || 100356 || $ || Abernathy, Lewis", q30c),
    ("31a", "Horror || 1040 || 2001 Maniacs || Agnew, Jim", q31a),
    ("31b", "Horror || 129755 || Saw || Bousman, Darren Lynn", q31b),
    ("31c", "Action || 1008 || 11:14 || Abraham, Brad", q31c),
    ("32a", "(empty)", q32a),
    ("32b", "alternate language version of || 12 oz. Mouse || 'Angel': Season 2 Overview", q32b),
    ("33a", "495 Productions || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila", q33a),
    ("33b", "MTV Netherlands || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila", q33b),
    ("33c", "2BE || 495 Productions || 1.3 || 1.0 || A Double Shot at Love || A Double Shot at Love", q33c),
];

fn q27a() -> String {
    min_row(movie().in_s(
        complete_cast().in_s(
            subject().text().is_in(["cast", "crew"])
                .and(status().text().eq("complete"))
        )
            .and(film_or_warner_co())
            .and(keyword().text().eq("sequel"))
            .and(follow_link())
            .and(info().in_s(Info::info().is_in(["Sweden", "Germany", "Swedish", "German"])))
            .and(production_year().ge(1950))
            .and(production_year().le(2000))
    ).o(
        film_or_warner_co().name()
            .x(follow_link().ty().text())
            .x(title())
    ))
}

fn q27b() -> String {
    min_row(movie().in_s(
        complete_cast().in_s(
            subject().text().is_in(["cast", "crew"])
                .and(status().text().eq("complete"))
        )
            .and(film_or_warner_co())
            .and(keyword().text().eq("sequel"))
            .and(follow_link())
            .and(info().in_s(Info::info().is_in(["Sweden", "Germany", "Swedish", "German"])))
            .and(production_year().eq(1998))
    ).o(
        film_or_warner_co().name()
            .x(follow_link().ty().text())
            .x(title())
    ))
}

fn q27c() -> String {
    min_row(movie().in_s(
        complete_cast().in_s(
            subject().text().eq("cast")
                .and(status().text().rx(r"^complete"))
        )
            .and(film_or_warner_co())
            .and(keyword().text().eq("sequel"))
            .and(follow_link())
            .and(info().in_s(Info::info().is_in(nordic9())))
            .and(production_year().ge(1950))
            .and(production_year().le(2010))
    ).o(
        film_or_warner_co().name()
            .x(follow_link().ty().text())
            .x(title())
    ))
}

fn q28a() -> String {
    min_row(movie().in_s(
        complete_cast().in_s(
            subject().text().eq("crew")
                .and(status().text().ne("complete+verified"))
        )
            .and(co_28())
            .and(info().in_s(
                Info::ty().text().eq("countries")
                    .and(Info::info().is_in(nordic10()))
            ))
            .and(dt_28ac())
            .and(keyword().text().is_in(murder4()))
            .and(kind().text().is_in(["movie", "episode"]))
            .and(production_year().gt(2000))
    ).o(
        co_28().name()
            .x(dt_28ac().text())
            .x(title())
    ))
}

fn q28b() -> String {
    min_row(movie().in_s(
        complete_cast().in_s(
            subject().text().eq("crew")
                .and(status().text().ne("complete+verified"))
        )
            .and(co_28())
            .and(info().in_s(
                Info::ty().text().eq("countries")
                    .and(Info::info().is_in(["Sweden", "Germany", "Swedish", "German"]))
            ))
            .and(dt_28b())
            .and(keyword().text().is_in(murder4()))
            .and(kind().text().is_in(["movie", "episode"]))
            .and(production_year().gt(2005))
    ).o(
        co_28().name()
            .x(dt_28b().text())
            .x(title())
    ))
}

fn q28c() -> String {
    min_row(movie().in_s(
        complete_cast().in_s(
            subject().text().eq("cast")
                .and(status().text().eq("complete"))
        )
            .and(co_28())
            .and(info().in_s(
                Info::ty().text().eq("countries")
                    .and(Info::info().is_in(nordic10()))
            ))
            .and(dt_28ac())
            .and(keyword().text().is_in(murder4()))
            .and(kind().text().is_in(["movie", "episode"]))
            .and(production_year().gt(2005))
    ).o(
        co_28().name()
            .x(dt_28ac().text())
            .x(title())
    ))
}

fn q29a() -> String {
    min_row(movie().in_s(
        complete_cast().in_s(
            subject().text().eq("cast")
                .and(status().text().eq("complete+verified"))
        )
            .and(company().country().eq("[us]"))
            .and(info().in_s(
                Info::ty().text().eq("release dates")
                    .and(Info::info().rx(r"^Japan:.*200")
                        .or(Info::info().rx(r"^USA:.*200")))
            ))
            .and(keyword().text().eq("computer-animation"))
            .and(title().eq("Shrek 2"))
            .and(production_year().ge(2000))
            .and(production_year().le(2010))
    ).o(
        cast().in_s(
            Cast::note().is_in(voice3())
                .and(role().text().eq("actress"))
                .and(character().text().eq("Queen"))
                .and(person().in_s(
                    gender().eq("f")
                        .and(Person::name().rx(r"An"))
                        .and(alias())
                        .and(bio().in_s(PersonInfo::ty().text().eq("trivia")))
                ))
        ).o(
            character().text()
                .x(person().name())
        )
        .x(title())
    ))
}

fn q29b() -> String {
    min_row(movie().in_s(
        complete_cast().in_s(
            subject().text().eq("cast")
                .and(status().text().eq("complete+verified"))
        )
            .and(company().country().eq("[us]"))
            .and(info().in_s(
                Info::ty().text().eq("release dates")
                    .and(Info::info().rx(r"^USA:.*200"))
            ))
            .and(keyword().text().eq("computer-animation"))
            .and(title().eq("Shrek 2"))
            .and(production_year().ge(2000))
            .and(production_year().le(2005))
    ).o(
        cast().in_s(
            Cast::note().is_in(voice3())
                .and(role().text().eq("actress"))
                .and(character().text().eq("Queen"))
                .and(person().in_s(
                    gender().eq("f")
                        .and(Person::name().rx(r"An"))
                        .and(alias())
                        .and(bio().in_s(PersonInfo::ty().text().eq("height")))
                ))
        ).o(
            character().text()
                .x(person().name())
        )
        .x(title())
    ))
}

fn q29c() -> String {
    min_row(movie().in_s(
        complete_cast().in_s(
            subject().text().eq("cast")
                .and(status().text().eq("complete+verified"))
        )
            .and(company().country().eq("[us]"))
            .and(info().in_s(
                Info::ty().text().eq("release dates")
                    .and(Info::info().rx(r"^Japan:.*200")
                        .or(Info::info().rx(r"^USA:.*200")))
            ))
            .and(keyword().text().eq("computer-animation"))
            .and(production_year().ge(2000))
            .and(production_year().le(2010))
    ).o(
        cast().in_s(
            Cast::note().is_in(voice4())
                .and(role().text().eq("actress"))
                .and(person().in_s(
                    gender().eq("f")
                        .and(Person::name().rx(r"An"))
                        .and(alias())
                        .and(bio().in_s(PersonInfo::ty().text().eq("trivia")))
                ))
        ).o(
            character().text()
                .x(person().name())
        )
        .x(title())
    ))
}

fn q30a() -> String {
    min_row(movie().in_s(
        complete_cast().in_s(
            subject().text().is_in(["cast", "crew"])
                .and(status().text().eq("complete+verified"))
        )
            .and(info().in_s(gf_horror()))
            .and(keyword().text().is_in(kw7()))
            .and(production_year().gt(2000))
    ).o(
        info().in_s(gf_horror()).info()
            .x(data().in_s(Data::ty().text().eq("votes")).text())
            .x(title())
            .x(cast().in_s(
                Cast::note().is_in(writer5())
                    .and(person().gender().eq("m"))
            ).person().name())
    ))
}

fn q30b() -> String {
    min_row(movie().in_s(
        complete_cast().in_s(
            subject().text().is_in(["cast", "crew"])
                .and(status().text().eq("complete+verified"))
        )
            .and(info().in_s(gf_horror()))
            .and(keyword().text().is_in(kw7()))
            .and(production_year().gt(2000))
            .and(title().rx(r"Freddy")
                .or(title().rx(r"Jason")
                    .or(title().rx(r"^Saw"))))
    ).o(
        info().in_s(gf_horror()).info()
            .x(data().in_s(Data::ty().text().eq("votes")).text())
            .x(title())
            .x(cast().in_s(
                Cast::note().is_in(writer5())
                    .and(person().gender().eq("m"))
            ).person().name())
    ))
}

fn q30c() -> String {
    min_row(movie().in_s(
        complete_cast().in_s(
            subject().text().eq("cast")
                .and(status().text().eq("complete+verified"))
        )
            .and(info().in_s(gf_genre6()))
            .and(keyword().text().is_in(kw7()))
    ).o(
        info().in_s(gf_genre6()).info()
            .x(data().in_s(Data::ty().text().eq("votes")).text())
            .x(title())
            .x(cast().in_s(
                Cast::note().is_in(writer5())
                    .and(person().gender().eq("m"))
            ).person().name())
    ))
}

fn q31a() -> String {
    min_row(movie().in_s(
        company().name().rx(r"^Lionsgate")
            .and(info().in_s(gf_horror()))
            .and(keyword().text().is_in(kw7()))
    ).o(
        info().in_s(gf_horror()).info()
            .x(data().in_s(Data::ty().text().eq("votes")).text())
            .x(title())
            .x(cast().in_s(
                Cast::note().is_in(writer5())
                    .and(person().gender().eq("m"))
            ).person().name())
    ))
}

fn q31b() -> String {
    min_row(movie().in_s(
        company().in_s(
            Company::name().rx(r"^Lionsgate")
                .and(Company::note().rx(r"\(Blu-ray\)"))
        )
            .and(info().in_s(gf_horror()))
            .and(keyword().text().is_in(kw7()))
            .and(production_year().gt(2000))
            .and(title().rx(r"Freddy")
                .or(title().rx(r"Jason")
                    .or(title().rx(r"^Saw"))))
    ).o(
        info().in_s(gf_horror()).info()
            .x(data().in_s(Data::ty().text().eq("votes")).text())
            .x(title())
            .x(cast().in_s(
                Cast::note().is_in(writer5())
                    .and(person().gender().eq("m"))
            ).person().name())
    ))
}

fn q31c() -> String {
    min_row(movie().in_s(
        company().name().rx(r"^Lionsgate")
            .and(info().in_s(gf_genre6()))
            .and(keyword().text().is_in(kw7()))
    ).o(
        info().in_s(gf_genre6()).info()
            .x(data().in_s(Data::ty().text().eq("votes")).text())
            .x(title())
            .x(cast().in_s(Cast::note().is_in(writer5()))
                    .person().name())
    ))
}

// q32a/q32b differ only in the keyword constant.
fn q32(kw: &'static str) -> String {
    min_row(movie().in_s(
        keyword().text().eq(kw)
            .and(link())
    ).o(
        link().ty().text()
            .x(title())
            .x(link().target().title())
    ))
}

fn q32a() -> String { q32("10,000-mile-club") }
fn q32b() -> String { q32("character-name-in-title") }

fn q33a() -> String {
    min_row(movie().in_s(
        kind().text().eq("tv series")
            .and(company().country().eq("[us]"))
            .and(qlink_33a())
    ).o(
        company().in_s(country().eq("[us]")).name()
            .x(qlink_33a().target().company().name())
            .x(data().in_s(Data::ty().text().eq("rating")).text())
            .x(qlink_33a().target().o(data().in_s(
                Data::ty().text().eq("rating")
                    .and(Data::text().lt("3.0"))
            ).text()))
            .x(title())
            .x(qlink_33a().target().title())
    ))
}

fn q33b() -> String {
    min_row(movie().in_s(
        kind().text().eq("tv series")
            .and(company().country().eq("[nl]"))
            .and(qlink_33b())
    ).o(
        company().in_s(country().eq("[nl]")).name()
            .x(qlink_33b().target().company().name())
            .x(data().in_s(Data::ty().text().eq("rating")).text())
            .x(qlink_33b().target().o(data().in_s(
                Data::ty().text().eq("rating")
                    .and(Data::text().lt("3.0"))
            ).text()))
            .x(title())
            .x(qlink_33b().target().title())
    ))
}

fn q33c() -> String {
    min_row(movie().in_s(
        kind().text().is_in(["tv series", "episode"])
            .and(company().country().ne("[us]"))
            .and(qlink_33c())
    ).o(
        company().in_s(country().ne("[us]")).name()
            .x(qlink_33c().target().company().name())
            .x(data().in_s(Data::ty().text().eq("rating")).text())
            .x(qlink_33c().target().o(data().in_s(
                Data::ty().text().eq("rating")
                    .and(Data::text().lt("3.5"))
            ).text()))
            .x(title())
            .x(qlink_33c().target().title())
    ))
}
