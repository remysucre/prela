// queries: 19a-26c (queries.jl lines 859-1111)

use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::min_row;
use crate::queries::sets::{genre6, kw7, kw8, kw10, nordic8, nordic9, voice4, writer5};
use super::helpers::{film_or_warner_co, follow_link};

fn k_23ab() -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    kind().text().eq("movie")
}

fn k_23c() -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    kind().text()
        .is_in(["movie", "tv movie", "video movie", "video game"])
}

// Conjunct trees (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_25ab() -> impl Query<D = Id<Info>> + Probe {
    Info::ty().text().eq("genres")
        .and(Info::info().eq("Horror"))
}

fn gf_25c() -> impl Query<D = Id<Info>> + Probe {
    Info::ty().text().eq("genres")
        .and(Info::info().is_in(genre6()))
}

pub const ENTRIES: &[super::Entry] = &[
    ("19a", "Angeline, Moriah || Blue Harvest", q19a),
    ("19b", "Jolie, Angelina || Kung Fu Panda", q19b),
    ("19c", "Alborg, Ana Esther || .hack//Akusei heni vol. 2", q19c),
    ("19d", "Aaron, Caroline || $9.99", q19d),
    ("20a", "Disaster Movie", q20a),
    ("20b", "Iron Man", q20b),
    ("20c", "Abell, Alistair || ...And Then I...", q20c),
    ("21a", "Det Danske Filminstitut || followed by || Der Serienkiller - Klinge des Todes", q21a),
    ("21b", "Filmlance International AB || followed by || Hämndens pris", q21b),
    ("21c", "Churchill Films || followed by || Batman Beyond", q21c),
    ("23a", "movie || The Analysts", q23a),
    ("23b", "movie || The Big Mope", q23b),
    ("23c", "movie || Dirt Merchant", q23c),
    ("24a", "Additional Voices || Baker, Andrea || Baiohazâdo 6", q24a),
    ("24b", "Tigress || Jolie, Angelina || Kung Fu Panda 2", q24b),
    ("25a", "Horror || 10 || -- And Now the Screaming Starts! || Abdallah, Damon", q25a),
    ("25b", "Horror || 138 || Vampire Boys || Campbell, Jeremiah", q25b),
    ("25c", "Action || 10 || $ || Aakeson, Kim Fupz", q25c),
    ("26a", "'Agua' Man || Acereda, Hermie || 7.1 || 3:10 to Yuma", q26a),
    ("26b", "Bank Manager || 8.2 || Inception", q26b),
    ("26c", "'Agua' Man || 1.9 || 12 Rounds", q26c),
];

fn q19a() -> String {
    min_row(movies().in_s(
        company().in_s(
            country().eq("[us]")
                .and(Company::note().rx(r"\(USA\)")
                    .or(Company::note().rx(r"\(worldwide\)")))
        )
            .and(info().in_s(
                Info::ty().text().eq("release dates")
                    .and(Info::info().rx(r"^Japan:.*200")
                        .or(Info::info().rx(r"^USA:.*200")))
            )
                .and(production_year().ge(2005)
                    .and(production_year().le(2009))))
    ).o(
        cast().in_s(
            Cast::note().is_in(voice4())
                .and(role().text().eq("actress")
                    .and(character()
                        .and(person().in_s(
                            gender().eq("f")
                                .and(Person::name().rx(r"Ang")
                                    .and(alias()))
                        ))))
        ).person().name()
            .x(title())
    ))
}

fn q19b() -> String {
    min_row(movies().in_s(
        company().in_s(
            country().eq("[us]")
                .and(Company::note().rx(r"\(200.*\)")
                    .and(Company::note().rx(r"\(USA\)")
                        .or(Company::note().rx(r"\(worldwide\)"))))
        )
            .and(info().in_s(
                Info::ty().text().eq("release dates")
                    .and(Info::info().rx(r"^Japan:.*2007")
                        .or(Info::info().rx(r"^USA:.*2008")))
            )
                .and(production_year().ge(2007)
                    .and(production_year().le(2008)
                        .and(title().rx(r"Kung.*Fu.*Panda")))))
    ).o(
        cast().in_s(
            Cast::note().eq("(voice)")
                .and(role().text().eq("actress")
                    .and(character()
                        .and(person().in_s(
                            gender().eq("f")
                                .and(Person::name().rx(r"Angel")
                                    .and(alias()))
                        ))))
        ).person().name()
            .x(title())
    ))
}

fn q19c() -> String {
    min_row(movies().in_s(
        company().country().eq("[us]")
            .and(info().in_s(
                Info::ty().text().eq("release dates")
                    .and(Info::info().rx(r"^Japan:.*200")
                        .or(Info::info().rx(r"^USA:.*200")))
            )
                .and(production_year().gt(2000)))
    ).o(
        cast().in_s(
            Cast::note().is_in(voice4())
                .and(role().text().eq("actress")
                    .and(character()
                        .and(person().in_s(
                            gender().eq("f")
                                .and(Person::name().rx(r"An")
                                    .and(alias()))
                        ))))
        ).person().name()
            .x(title())
    ))
}

fn q19d() -> String {
    min_row(movies().in_s(
        company().country().eq("[us]")
            .and(info().ty().text().eq("release dates")
                .and(production_year().gt(2000)))
    ).o(
        cast().in_s(
            Cast::note().is_in(voice4())
                .and(role().text().eq("actress")
                    .and(character()
                        .and(person().in_s(
                            gender().eq("f")
                                .and(alias())
                        ))))
        ).person().name()
            .x(title())
    ))
}

fn q20a() -> String {
    min_row(movies().in_s(
        complete_cast().in_s(
            subject().text().eq("cast")
                .and(status().text().rx(r"complete"))
        )
            .and(keyword().text().is_in(kw8())
                .and(kind().text().eq("movie")
                    .and(production_year().gt(1950)
                        .and(cast().o(
                            character().in_s(
                                Character::text().nrx(r"Sherlock")
                                    .and(Character::text().rx(r"Tony.*Stark")
                                        .or(Character::text().rx(r"Iron.*Man")))
                            )
                        )))))
    ).title())
}

fn q20b() -> String {
    min_row(movies().in_s(
        complete_cast().in_s(
            subject().text().eq("cast")
                .and(status().text().rx(r"complete"))
        )
            .and(keyword().text().is_in(kw8())
                .and(kind().text().eq("movie")
                    .and(production_year().gt(2000)
                        .and(cast().in_s(
                            character().in_s(
                                Character::text().nrx(r"Sherlock")
                                    .and(Character::text().rx(r"Tony.*Stark")
                                        .or(Character::text().rx(r"Iron.*Man")))
                            )
                                .and(person().name().rx(r"Downey.*Robert"))
                        )))))
    ).title())
}

fn q20c() -> String {
    min_row(movies().in_s(
        complete_cast().in_s(
            subject().text().eq("cast")
                .and(status().text().rx(r"complete"))
        )
            .and(keyword().text().is_in(kw10())
                .and(kind().text().eq("movie")
                    .and(production_year().gt(2000))))
    ).o(
        cast().in_s(character().text().rx(r"[Mm]an"))
            .person().name()
            .x(title())
    ))
}

// q21a/b/c differ only in the country list and year range.
fn q21(countries: Vec<&'static str>, ylo: i64, yhi: i64) -> String {
    min_row(movies().in_s(
        film_or_warner_co()
            .and(keyword().text().eq("sequel")
                .and(follow_link()
                    .and(info().info().is_in(countries)
                        .and(production_year().ge(ylo)
                            .and(production_year().le(yhi))))))
    ).o(
        film_or_warner_co().name()
            .x(follow_link().ty().text())
            .x(title())
    ))
}

fn q21a() -> String { q21(nordic8(), 1950, 2000) }
fn q21b() -> String { q21(vec!["Germany", "German"], 2000, 2010) }
fn q21c() -> String { q21(nordic9(), 1950, 2010) }

fn q23a() -> String {
    min_row(movies().in_s(
        complete_cast().status().text().eq("complete+verified")
            .and(company().country().eq("[us]")
                .and(info().in_s(
                    Info::ty().text().eq("release dates")
                        .and(Info::note().rx(r"internet")
                            .and(Info::info().rx(r"^USA:.* 199")
                                .or(Info::info().rx(r"^USA:.* 200"))))
                )
                    .and(k_23ab()
                        .and(keyword()
                            .and(production_year().gt(2000))))))
    ).o(k_23ab().x(title())))
}

fn q23b() -> String {
    min_row(movies().in_s(
        complete_cast().status().text().eq("complete+verified")
            .and(company().country().eq("[us]")
                .and(info().in_s(
                    Info::ty().text().eq("release dates")
                        .and(Info::note().rx(r"internet")
                            .and(Info::info().rx(r"^USA:.* 200")))
                )
                    .and(k_23ab()
                        .and(keyword().text()
                            .is_in(["nerd", "loner", "alienation", "dignity"])
                            .and(production_year().gt(2000))))))
    ).o(k_23ab().x(title())))
}

fn q23c() -> String {
    min_row(movies().in_s(
        complete_cast().status().text().eq("complete+verified")
            .and(company().country().eq("[us]")
                .and(info().in_s(
                    Info::ty().text().eq("release dates")
                        .and(Info::note().rx(r"internet")
                            .and(Info::info().rx(r"^USA:.* 199")
                                .or(Info::info().rx(r"^USA:.* 200"))))
                )
                    .and(k_23c()
                        .and(keyword()
                            .and(production_year().gt(1990))))))
    ).o(k_23c().x(title())))
}

fn q24a() -> String {
    min_row(movies().in_s(
        company().country().eq("[us]")
            .and(info().in_s(
                Info::ty().text().eq("release dates")
                    .and(Info::info().rx(r"^Japan:.*201")
                        .or(Info::info().rx(r"^USA:.*201")))
            )
                .and(keyword().text()
                    .is_in(["hero", "martial-arts", "hand-to-hand-combat"])
                    .and(production_year().gt(2010))))
    ).o(
        cast().in_s(
            Cast::note().is_in(voice4())
                .and(role().text().eq("actress")
                    .and(person().in_s(
                        gender().eq("f")
                            .and(Person::name().rx(r"An")
                                .and(alias()))
                    )))
        ).o(
            character().text()
                .x(person().name())
        )
            .x(title())
    ))
}

fn q24b() -> String {
    min_row(movies().in_s(
        company().in_s(
            country().eq("[us]")
                .and(Company::name().eq("DreamWorks Animation"))
        )
            .and(info().in_s(
                Info::ty().text().eq("release dates")
                    .and(Info::info().rx(r"^Japan:.*201")
                        .or(Info::info().rx(r"^USA:.*201")))
            )
                .and(keyword().text()
                    .is_in(["hero", "martial-arts", "hand-to-hand-combat", "computer-animated-movie"])
                    .and(production_year().gt(2010)
                        .and(title().rx(r"^Kung Fu Panda")))))
    ).o(
        cast().in_s(
            Cast::note().is_in(voice4())
                .and(role().text().eq("actress")
                    .and(person().in_s(
                        gender().eq("f")
                            .and(Person::name().rx(r"An")
                                .and(alias()))
                    )))
        ).o(
            character().text()
                .x(person().name())
        )
            .x(title())
    ))
}

fn q25a() -> String {
    min_row(movies().in_s(
        info().in_s(gf_25ab())
            .and(keyword().text()
                .is_in(["murder", "blood", "gore", "death", "female-nudity"]))
    ).o(
        info().in_s(gf_25ab()).info()
            .x(data().in_s(Data::ty().text().eq("votes")).text())
            .x(title())
            .x(cast().in_s(
                Cast::note().is_in(writer5())
                    .and(person().gender().eq("m"))
            ).person().name())
    ))
}

fn q25b() -> String {
    min_row(movies().in_s(
        info().in_s(gf_25ab())
            .and(keyword().text()
                .is_in(["murder", "blood", "gore", "death", "female-nudity"])
                .and(production_year().gt(2010)
                    .and(title().rx(r"^Vampire"))))
    ).o(
        info().in_s(gf_25ab()).info()
            .x(data().in_s(Data::ty().text().eq("votes")).text())
            .x(title())
            .x(cast().in_s(
                Cast::note().is_in(writer5())
                    .and(person().gender().eq("m"))
            ).person().name())
    ))
}

fn q25c() -> String {
    min_row(movies().in_s(
        info().in_s(gf_25c())
            .and(keyword().text().is_in(kw7()))
    ).o(
        info().in_s(gf_25c()).info()
            .x(data().in_s(Data::ty().text().eq("votes")).text())
            .x(title())
            .x(cast().in_s(
                Cast::note().is_in(writer5())
                    .and(person().gender().eq("m"))
            ).person().name())
    ))
}

fn q26a() -> String {
    min_row(movies().in_s(
        complete_cast().in_s(
            subject().text().eq("cast")
                .and(status().text().rx(r"complete"))
        )
            .and(keyword().text().is_in(kw10())
                .and(kind().text().eq("movie")
                    .and(production_year().gt(2000))))
    ).o(
        cast().in_s(character().text().rx(r"[Mm]an"))
            .o(
                character().text()
                    .x(person().name())
            )
            .x(data().in_s(
                Data::ty().text().eq("rating")
                    .and(Data::text().gt("7.0"))
            ).text())
            .x(title())
    ))
}

fn q26b() -> String {
    min_row(movies().in_s(
        complete_cast().in_s(
            subject().text().eq("cast")
                .and(status().text().rx(r"complete"))
        )
            .and(keyword().text()
                .is_in(["superhero", "marvel-comics", "based-on-comic", "fight"])
                .and(kind().text().eq("movie")
                    .and(production_year().gt(2005))))
    ).o(
        cast().in_s(character().text().rx(r"[Mm]an"))
            .character().text()
            .x(data().in_s(
                Data::ty().text().eq("rating")
                    .and(Data::text().gt("8.0"))
            ).text())
            .x(title())
    ))
}

fn q26c() -> String {
    let rd = data().in_s(Data::ty().text().eq("rating")).text();
    min_row(movies().in_s(
        complete_cast().in_s(
            subject().text().eq("cast")
                .and(status().text().rx(r"complete"))
        )
            .and(keyword().text().is_in(kw10())
                .and(kind().text().eq("movie")
                    .and(production_year().gt(2000))))
    ).o(
        cast().in_s(character().text().rx(r"[Mm]an"))
            .character().text()
            .x(rd)
            .x(title())
    ))
}
