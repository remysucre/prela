// queries: 27a–33c (queries.jl lines 1114–1394)

use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::{min_row, Row};
use crate::queries::sets::{genre6, kw7, link3, murder4, nordic9, nordic10, voice3, voice4, writer5};
use super::helpers::{film_or_warner_co, follow_link};

fn co_28() -> impl Query<R = Id<Company>, D = Id<Movie>> + Drive + Probe {
    company.when(country.ne("[us]")
            .and(Company::note.nrx(r"\(USA\)"))
            .and(Company::note.rx(r"\(200.*\)")))
}

fn dt_28ac() -> impl Query<R = Id<Data>, D = Id<Movie>> + Drive + Probe {
    data.when(Data::ty.text().eq("rating")
         .and(Data::text.lt("8.5")))
}

fn dt_28b() -> impl Query<R = Id<Data>, D = Id<Movie>> + Drive + Probe {
    data.when(Data::ty.text().eq("rating")
         .and(Data::text.gt("6.5")))
}

// Conjunct trees (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_horror() -> impl Query<D = Id<Info>> + Probe {
    Info::ty.text().eq("genres")
        .and(Info::info.is_in(["Horror", "Thriller"]))
}

fn gf_genre6() -> impl Query<D = Id<Info>> + Probe {
    Info::ty.text().eq("genres")
        .and(Info::info.is_in(genre6()))
}

fn qlink_33a() -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    link.when(MovieLink::ty.text().is_in(link3())
         .and(target.when(kind.text().eq("tv series")
                     .and(company)
                     .and(data.when(Data::ty.text().eq("rating")
                               .and(Data::text.lt("3.0"))))
                     .and(production_year.ge(2005))
                     .and(production_year.le(2008)))))
}

fn qlink_33b() -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    link.when(MovieLink::ty.text().rx(r"follow")
         .and(target.when(kind.text().eq("tv series")
                     .and(company)
                     .and(data.when(Data::ty.text().eq("rating")
                               .and(Data::text.lt("3.0"))))
                     .and(production_year.eq(2007)))))
}

fn qlink_33c() -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    link.when(MovieLink::ty.text().is_in(link3())
         .and(target.when(kind.text().is_in(["tv series", "episode"])
                     .and(company)
                     .and(data.when(Data::ty.text().eq("rating")
                               .and(Data::text.lt("3.5"))))
                     .and(production_year.ge(2000))
                     .and(production_year.le(2010)))))
}

pub const ENTRIES: &[super::Entry] = &[
    ("27a", "Det Danske Filminstitut || followed by || Spår i mörker", || min_row(q27a())),
    ("27b", "Filmlance International AB || followed by || Vita nätter", || min_row(q27b())),
    ("27c", "Det Danske Filminstitut || followed by || Spår i mörker", || min_row(q27c())),
    ("28a", "01 Distribuzione || 2.9 || (#1.1)", || min_row(q28a())),
    ("28b", "20th Century Fox || 6.6 || (#1.1)", || min_row(q28b())),
    ("28c", "01 Distribuzione || 1.9 || (#1.1)", || min_row(q28c())),
    ("29a", "Queen || Andrews, Julie || Shrek 2", || min_row(q29a())),
    ("29b", "Queen || Andrews, Julie || Shrek 2", || min_row(q29b())),
    ("29c", "Lola || Andrews, Julie || Hoodwinked!", || min_row(q29c())),
    ("30a", "Horror || 100356 || 16 Blocks || Abrams, J.J.", || min_row(q30a())),
    ("30b", "Horror || 194782 || Freddy vs. Jason || Shannon, Damian", || min_row(q30b())),
    ("30c", "Action || 100356 || $ || Abernathy, Lewis", || min_row(q30c())),
    ("31a", "Horror || 1040 || 2001 Maniacs || Agnew, Jim", || min_row(q31a())),
    ("31b", "Horror || 129755 || Saw || Bousman, Darren Lynn", || min_row(q31b())),
    ("31c", "Action || 1008 || 11:14 || Abraham, Brad", || min_row(q31c())),
    ("32a", "(empty)", || min_row(q32a())),
    ("32b", "alternate language version of || 12 oz. Mouse || 'Angel': Season 2 Overview", || min_row(q32b())),
    ("33a", "495 Productions || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila", || min_row(q33a())),
    ("33b", "MTV Netherlands || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila", || min_row(q33b())),
    ("33c", "2BE || 495 Productions || 1.3 || 1.0 || A Double Shot at Love || A Double Shot at Love", || min_row(q33c())),
];

fn q27a() -> impl Drive<R: Row> {
    movie.when(complete_cast.select(subject.text().is_in(["cast", "crew"])
                               .and(status.text().eq("complete")))
          .and(film_or_warner_co())
          .and(keyword.text().eq("sequel"))
          .and(follow_link())
          .and(info.select(Info::info.is_in(["Sweden", "Germany", "Swedish", "German"])))
          .and(production_year.ge(1950))
          .and(production_year.le(2000)))
       .select(film_or_warner_co().name()
          .and(follow_link())
          .and(title))
}

fn q27b() -> impl Drive<R: Row> {
    movie.when(complete_cast.select(subject.text().is_in(["cast", "crew"])
                               .and(status.text().eq("complete")))
          .and(film_or_warner_co())
          .and(keyword.text().eq("sequel"))
          .and(follow_link())
          .and(info.select(Info::info.is_in(["Sweden", "Germany", "Swedish", "German"])))
          .and(production_year.eq(1998)))
       .select(film_or_warner_co().name()
          .and(follow_link())
          .and(title))
}

fn q27c() -> impl Drive<R: Row> {
    movie.when(complete_cast.select(subject.text().eq("cast")
                               .and(status.text().rx(r"^complete")))
          .and(film_or_warner_co())
          .and(keyword.text().eq("sequel"))
          .and(follow_link())
          .and(info.select(Info::info.is_in(nordic9())))
          .and(production_year.ge(1950))
          .and(production_year.le(2010)))
       .select(film_or_warner_co().name()
          .and(follow_link())
          .and(title))
}

fn q28a() -> impl Drive<R: Row> {
    movie.when(complete_cast.select(subject.text().eq("crew")
                               .and(status.text().ne("complete+verified")))
          .and(co_28())
          .and(info.select(Info::ty.text().eq("countries")
                      .and(Info::info.is_in(nordic10()))))
          .and(dt_28ac())
          .and(keyword.text().is_in(murder4()))
          .and(kind.text().is_in(["movie", "episode"]))
          .and(production_year.gt(2000)))
       .select(co_28().name()
          .and(dt_28ac().text())
          .and(title))
}

fn q28b() -> impl Drive<R: Row> {
    movie.when(complete_cast.select(subject.text().eq("crew")
                               .and(status.text().ne("complete+verified")))
          .and(co_28())
          .and(info.select(Info::ty.text().eq("countries")
                      .and(Info::info.is_in(["Sweden", "Germany", "Swedish", "German"]))))
          .and(dt_28b())
          .and(keyword.text().is_in(murder4()))
          .and(kind.text().is_in(["movie", "episode"]))
          .and(production_year.gt(2005)))
       .select(co_28().name()
          .and(dt_28b().text())
          .and(title))
}

fn q28c() -> impl Drive<R: Row> {
    movie.when(complete_cast.select(subject.text().eq("cast")
                               .and(status.text().eq("complete")))
          .and(co_28())
          .and(info.select(Info::ty.text().eq("countries")
                      .and(Info::info.is_in(nordic10()))))
          .and(dt_28ac())
          .and(keyword.text().is_in(murder4()))
          .and(kind.text().is_in(["movie", "episode"]))
          .and(production_year.gt(2005)))
       .select(co_28().name()
          .and(dt_28ac().text())
          .and(title))
}

fn q29a() -> impl Drive<R: Row> {
    movie.when(complete_cast.select(subject.text().eq("cast")
                               .and(status.text().eq("complete+verified")))
          .and(company.country().eq("[us]"))
          .and(info.select(Info::ty.text().eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*200")
                       .or(Info::info.rx(r"^USA:.*200")))))
          .and(keyword.text().eq("computer-animation"))
          .and(title.eq("Shrek 2"))
          .and(production_year.ge(2000))
          .and(production_year.le(2010)))
       .select(cast
             .when(Cast::note.is_in(voice3())
              .and(role.text().eq("actress"))
              .and(character.text().eq("Queen"))
              .and(person.when(gender.eq("f")
                          .and(Person::name.rx(r"An"))
                          .and(alias)
                          .and(bio.select(PersonInfo::ty.text().eq("trivia"))))))
             .select(character.text()
                .and(person.name()))
          .and(title))
}

fn q29b() -> impl Drive<R: Row> {
    movie.when(complete_cast.select(subject.text().eq("cast")
                               .and(status.text().eq("complete+verified")))
          .and(company.country().eq("[us]"))
          .and(info.select(Info::ty.text().eq("release dates")
                      .and(Info::info.rx(r"^USA:.*200"))))
          .and(keyword.text().eq("computer-animation"))
          .and(title.eq("Shrek 2"))
          .and(production_year.ge(2000))
          .and(production_year.le(2005)))
       .select(cast
             .when(Cast::note.is_in(voice3())
              .and(role.text().eq("actress"))
              .and(character.text().eq("Queen"))
              .and(person.when(gender.eq("f")
                          .and(Person::name.rx(r"An"))
                          .and(alias)
                          .and(bio.select(PersonInfo::ty.text().eq("height"))))))
             .select(character.text()
                .and(person.name()))
          .and(title))
}

fn q29c() -> impl Drive<R: Row> {
    movie.when(complete_cast.select(subject.text().eq("cast")
                               .and(status.text().eq("complete+verified")))
          .and(company.country().eq("[us]"))
          .and(info.select(Info::ty.text().eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*200")
                       .or(Info::info.rx(r"^USA:.*200")))))
          .and(keyword.text().eq("computer-animation"))
          .and(production_year.ge(2000))
          .and(production_year.le(2010)))
       .select(cast
             .when(Cast::note.is_in(voice4())
              .and(role.text().eq("actress"))
              .and(person.when(gender.eq("f")
                          .and(Person::name.rx(r"An"))
                          .and(alias)
                          .and(bio.select(PersonInfo::ty.text().eq("trivia"))))))
             .select(character.text()
                .and(person.name()))
          .and(title))
}

fn q30a() -> impl Drive<R: Row> {
    movie.when(complete_cast.select(subject.text().is_in(["cast", "crew"])
                               .and(status.text().eq("complete+verified")))
          .and(info.when(gf_horror()))
          .and(keyword.text().is_in(kw7()))
          .and(production_year.gt(2000)))
       .select(info.when(gf_horror()).info()
          .and(data.when(Data::ty.text().eq("votes")).text())
          .and(title)
          .and(cast.when(Cast::note.is_in(writer5())
                    .and(person.when(gender.eq("m")))).person().name()))
}

fn q30b() -> impl Drive<R: Row> {
    movie.when(complete_cast.select(subject.text().is_in(["cast", "crew"])
                               .and(status.text().eq("complete+verified")))
          .and(info.when(gf_horror()))
          .and(keyword.text().is_in(kw7()))
          .and(production_year.gt(2000))
          .and(title.rx(r"Freddy").or(title.rx(r"Jason")).or(title.rx(r"^Saw"))))
       .select(info.when(gf_horror()).info()
          .and(data.when(Data::ty.text().eq("votes")).text())
          .and(title)
          .and(cast.when(Cast::note.is_in(writer5())
                    .and(person.when(gender.eq("m")))).person().name()))
}

fn q30c() -> impl Drive<R: Row> {
    movie.when(complete_cast.select(subject.text().eq("cast")
                               .and(status.text().eq("complete+verified")))
          .and(info.when(gf_genre6()))
          .and(keyword.text().is_in(kw7())))
       .select(info.when(gf_genre6()).info()
          .and(data.when(Data::ty.text().eq("votes")).text())
          .and(title)
          .and(cast.when(Cast::note.is_in(writer5())
                    .and(person.when(gender.eq("m")))).person().name()))
}

fn q31a() -> impl Drive<R: Row> {
    movie.when(company.name().rx(r"^Lionsgate")
          .and(info.when(gf_horror()))
          .and(keyword.text().is_in(kw7())))
       .select(info.when(gf_horror()).info()
          .and(data.when(Data::ty.text().eq("votes")).text())
          .and(title)
          .and(cast.when(Cast::note.is_in(writer5())
                    .and(person.when(gender.eq("m")))).person().name()))
}

fn q31b() -> impl Drive<R: Row> {
    movie.when(company.select(Company::name.rx(r"^Lionsgate")
                         .and(Company::note.rx(r"\(Blu-ray\)")))
          .and(info.when(gf_horror()))
          .and(keyword.text().is_in(kw7()))
          .and(production_year.gt(2000))
          .and(title.rx(r"Freddy").or(title.rx(r"Jason")).or(title.rx(r"^Saw"))))
       .select(info.when(gf_horror()).info()
          .and(data.when(Data::ty.text().eq("votes")).text())
          .and(title)
          .and(cast.when(Cast::note.is_in(writer5())
                    .and(person.when(gender.eq("m")))).person().name()))
}

fn q31c() -> impl Drive<R: Row> {
    movie.when(company.name().rx(r"^Lionsgate")
          .and(info.when(gf_genre6()))
          .and(keyword.text().is_in(kw7())))
       .select(info.when(gf_genre6()).info()
          .and(data.when(Data::ty.text().eq("votes")).text())
          .and(title)
          .and(cast.when(Cast::note.is_in(writer5())).person().name()))
}

// q32a/q32b differ only in the keyword constant.
fn q32(kw: &'static str) -> impl Drive<R: Row> {
    movie.when(keyword.text().eq(kw)
          .and(link))
       .select(link.ty().text()
          .and(title)
          .and(link.target().title()))
}

fn q32a() -> impl Drive<R: Row> { q32("10,000-mile-club") }
fn q32b() -> impl Drive<R: Row> { q32("character-name-in-title") }

fn q33a() -> impl Drive<R: Row> {
    movie.when(kind.text().eq("tv series")
          .and(company.country().eq("[us]"))
          .and(qlink_33a()))
       .select(company.when(country.eq("[us]")).name()
          .and(qlink_33a().target().company().name())
          .and(data.when(Data::ty.text().eq("rating")).text())
          .and(qlink_33a().target().select(data.when(Data::ty.text().eq("rating")
                                                .and(Data::text.lt("3.0"))).text()))
          .and(title)
          .and(qlink_33a().target().title()))
}

fn q33b() -> impl Drive<R: Row> {
    movie.when(kind.text().eq("tv series")
          .and(company.country().eq("[nl]"))
          .and(qlink_33b()))
       .select(company.when(country.eq("[nl]")).name()
          .and(qlink_33b().target().company().name())
          .and(data.when(Data::ty.text().eq("rating")).text())
          .and(qlink_33b().target().select(data.when(Data::ty.text().eq("rating")
                                                .and(Data::text.lt("3.0"))).text()))
          .and(title)
          .and(qlink_33b().target().title()))
}

fn q33c() -> impl Drive<R: Row> {
    movie.when(kind.text().is_in(["tv series", "episode"])
          .and(company.country().ne("[us]"))
          .and(qlink_33c()))
       .select(company.when(country.ne("[us]")).name()
          .and(qlink_33c().target().company().name())
          .and(data.when(Data::ty.text().eq("rating")).text())
          .and(qlink_33c().target().select(data.when(Data::ty.text().eq("rating")
                                                .and(Data::text.lt("3.5"))).text()))
          .and(title)
          .and(qlink_33c().target().title()))
}
