// queries: 19a-26c (queries.jl lines 859-1111)

use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::{min_row, Row};
use crate::queries::sets::{genre6, kw7, kw8, kw10, nordic8, nordic9, voice4, writer5};
use super::helpers::{film_or_warner_co, follow_link};

fn k_23ab() -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe + Member {
    kind.eq("movie")
}

fn k_23c() -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe + Member {
    kind.text()
        .is_in(["movie", "tv movie", "video movie", "video game"])
}

// Conjunct trees (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_25ab() -> impl Query<D = Id<Info>> + Probe + Member {
    Info::ty.eq("genres")
        .and(Info::info.eq("Horror"))
}

fn gf_25c() -> impl Query<D = Id<Info>> + Probe + Member {
    Info::ty.eq("genres")
        .and(Info::info.is_in(genre6()))
}

pub const ENTRIES: &[super::Entry] = &[
    ("19a", "Angeline, Moriah || Blue Harvest", || min_row(q19a())),
    ("19b", "Jolie, Angelina || Kung Fu Panda", || min_row(q19b())),
    ("19c", "Alborg, Ana Esther || .hack//Akusei heni vol. 2", || min_row(q19c())),
    ("19d", "Aaron, Caroline || $9.99", || min_row(q19d())),
    ("20a", "Disaster Movie", || min_row(q20a())),
    ("20b", "Iron Man", || min_row(q20b())),
    ("20c", "Abell, Alistair || ...And Then I...", || min_row(q20c())),
    ("21a", "Det Danske Filminstitut || followed by || Der Serienkiller - Klinge des Todes", || min_row(q21a())),
    ("21b", "Filmlance International AB || followed by || Hämndens pris", || min_row(q21b())),
    ("21c", "Churchill Films || followed by || Batman Beyond", || min_row(q21c())),
    ("23a", "movie || The Analysts", || min_row(q23a())),
    ("23b", "movie || The Big Mope", || min_row(q23b())),
    ("23c", "movie || Dirt Merchant", || min_row(q23c())),
    ("24a", "Additional Voices || Baker, Andrea || Baiohazâdo 6", || min_row(q24a())),
    ("24b", "Tigress || Jolie, Angelina || Kung Fu Panda 2", || min_row(q24b())),
    ("25a", "Horror || 10 || -- And Now the Screaming Starts! || Abdallah, Damon", || min_row(q25a())),
    ("25b", "Horror || 138 || Vampire Boys || Campbell, Jeremiah", || min_row(q25b())),
    ("25c", "Action || 10 || $ || Aakeson, Kim Fupz", || min_row(q25c())),
    ("26a", "'Agua' Man || Acereda, Hermie || 7.1 || 3:10 to Yuma", || min_row(q26a())),
    ("26b", "Bank Manager || 8.2 || Inception", || min_row(q26b())),
    ("26c", "'Agua' Man || 1.9 || 12 Rounds", || min_row(q26c())),
];

fn q19a() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[us]")
                         .and(Company::note.rx(r"\(USA\)")
                          .or(Company::note.rx(r"\(worldwide\)"))))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*200")
                       .or(Info::info.rx(r"^USA:.*200")))))
          .and(production_year.ge(2005))
          .and(production_year.le(2009)))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(character)
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"Ang"))
                          .and(alias))))
             .person().name()
          .and(title))
}

fn q19b() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[us]")
                         .and(Company::note.rx(r"\(200.*\)"))
                         .and(Company::note.rx(r"\(USA\)")
                          .or(Company::note.rx(r"\(worldwide\)"))))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*2007")
                       .or(Info::info.rx(r"^USA:.*2008")))))
          .and(production_year.ge(2007))
          .and(production_year.le(2008))
          .and(title.rx(r"Kung.*Fu.*Panda")))
       .select(cast
             .with(Cast::note.eq("(voice)")
              .and(role.eq("actress"))
              .and(character)
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"Angel"))
                          .and(alias))))
             .person().name()
          .and(title))
}

fn q19c() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*200")
                       .or(Info::info.rx(r"^USA:.*200")))))
          .and(production_year.gt(2000)))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(character)
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"An"))
                          .and(alias))))
             .person().name()
          .and(title))
}

fn q19d() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(info.ty().eq("release dates"))
          .and(production_year.gt(2000)))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(character)
              .and(person.with(gender.eq("f")
                          .and(alias))))
             .person().name()
          .and(title))
}

fn q20a() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"complete")))
          .and(keyword.is_in(kw8()))
          .and(kind.eq("movie"))
          .and(production_year.gt(1950))
          .and(cast.select(character.select(Character::text.nrx(r"Sherlock")
                                       .and(Character::text.rx(r"Tony.*Stark")
                                        .or(Character::text.rx(r"Iron.*Man")))))))
        .title()
}

fn q20b() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"complete")))
          .and(keyword.is_in(kw8()))
          .and(kind.eq("movie"))
          .and(production_year.gt(2000))
          .and(cast.select(character.select(Character::text.nrx(r"Sherlock")
                                       .and(Character::text.rx(r"Tony.*Stark")
                                        .or(Character::text.rx(r"Iron.*Man"))))
                      .and(person.rx(r"Downey.*Robert")))))
        .title()
}

fn q20c() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"complete")))
          .and(keyword.is_in(kw10()))
          .and(kind.eq("movie"))
          .and(production_year.gt(2000)))
       .select(cast.with(character.rx(r"[Mm]an"))
             .person().name()
          .and(title))
}

// q21a/b/c differ only in the country list and year range.
fn q21(countries: Vec<&'static str>, ylo: i64, yhi: i64) -> impl Drive<R: Row> {
    movie.with(film_or_warner_co()
          .and(keyword.eq("sequel"))
          .and(follow_link())
          .and(info.is_in(countries))
          .and(production_year.ge(ylo))
          .and(production_year.le(yhi)))
       .select(film_or_warner_co().name()
          .and(follow_link())
          .and(title))
}

fn q21a() -> impl Drive<R: Row> { q21(nordic8(), 1950, 2000) }
fn q21b() -> impl Drive<R: Row> { q21(vec!["Germany", "German"], 2000, 2010) }
fn q21c() -> impl Drive<R: Row> { q21(nordic9(), 1950, 2010) }

fn q23a() -> impl Drive<R: Row> {
    movie.with(complete_cast.status().eq("complete+verified")
          .and(company.country().eq("[us]"))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::note.rx(r"internet"))
                      .and(Info::info.rx(r"^USA:.* 199")
                       .or(Info::info.rx(r"^USA:.* 200")))))
          .and(k_23ab())
          .and(keyword)
          .and(production_year.gt(2000)))
       .select(k_23ab().and(title))
}

fn q23b() -> impl Drive<R: Row> {
    movie.with(complete_cast.status().eq("complete+verified")
          .and(company.country().eq("[us]"))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::note.rx(r"internet"))
                      .and(Info::info.rx(r"^USA:.* 200"))))
          .and(k_23ab())
          .and(keyword.is_in(["nerd", "loner", "alienation", "dignity"]))
          .and(production_year.gt(2000)))
       .select(k_23ab().and(title))
}

fn q23c() -> impl Drive<R: Row> {
    movie.with(complete_cast.status().eq("complete+verified")
          .and(company.country().eq("[us]"))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::note.rx(r"internet"))
                      .and(Info::info.rx(r"^USA:.* 199")
                       .or(Info::info.rx(r"^USA:.* 200")))))
          .and(k_23c())
          .and(keyword)
          .and(production_year.gt(1990)))
       .select(k_23c().and(title))
}

fn q24a() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*201")
                       .or(Info::info.rx(r"^USA:.*201")))))
          .and(keyword.is_in(["hero", "martial-arts", "hand-to-hand-combat"]))
          .and(production_year.gt(2010)))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"An"))
                          .and(alias))))
             .select(character.text()
                .and(person.name()))
          .and(title))
}

fn q24b() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[us]")
                         .and(Company::name.eq("DreamWorks Animation")))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*201")
                       .or(Info::info.rx(r"^USA:.*201")))))
          .and(keyword.is_in(["hero", "martial-arts", "hand-to-hand-combat", "computer-animated-movie"]))
          .and(production_year.gt(2010))
          .and(title.rx(r"^Kung Fu Panda")))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"An"))
                          .and(alias))))
             .select(character.text()
                .and(person.name()))
          .and(title))
}

fn q25a() -> impl Drive<R: Row> {
    movie.with(info.with(gf_25ab())
          .and(keyword.is_in(["murder", "blood", "gore", "death", "female-nudity"])))
       .select(info.with(gf_25ab()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title)
          .and(cast.with(Cast::note.is_in(writer5())
                    .and(person.with(gender.eq("m")))).person().name()))
}

fn q25b() -> impl Drive<R: Row> {
    movie.with(info.with(gf_25ab())
          .and(keyword.is_in(["murder", "blood", "gore", "death", "female-nudity"]))
          .and(production_year.gt(2010))
          .and(title.rx(r"^Vampire")))
       .select(info.with(gf_25ab()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title)
          .and(cast.with(Cast::note.is_in(writer5())
                    .and(person.with(gender.eq("m")))).person().name()))
}

fn q25c() -> impl Drive<R: Row> {
    movie.with(info.with(gf_25c())
          .and(keyword.is_in(kw7())))
       .select(info.with(gf_25c()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title)
          .and(cast.with(Cast::note.is_in(writer5())
                    .and(person.with(gender.eq("m")))).person().name()))
}

fn q26a() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"complete")))
          .and(keyword.is_in(kw10()))
          .and(kind.eq("movie"))
          .and(production_year.gt(2000)))
       .select(cast.with(character.rx(r"[Mm]an"))
             .select(character.text()
                .and(person.name()))
          .and(data.with(Data::ty.eq("rating")
                    .and(Data::text.gt("7.0"))).text())
          .and(title))
}

fn q26b() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"complete")))
          .and(keyword.is_in(["superhero", "marvel-comics", "based-on-comic", "fight"]))
          .and(kind.eq("movie"))
          .and(production_year.gt(2005)))
       .select(cast.with(character.rx(r"[Mm]an"))
             .character().text()
          .and(data.with(Data::ty.eq("rating")
                    .and(Data::text.gt("8.0"))).text())
          .and(title))
}

fn q26c() -> impl Drive<R: Row> {
    let rd = data.with(Data::ty.eq("rating")).text();
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"complete")))
          .and(keyword.is_in(kw10()))
          .and(kind.eq("movie"))
          .and(production_year.gt(2000)))
       .select(cast.with(character.rx(r"[Mm]an"))
             .character().text()
          .and(rd)
          .and(title))
}
