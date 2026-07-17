// queries: queries.jl lines 757-856 (templates 16-18)

use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::{min_row, Row};
use crate::queries::sets::{genre6, writer5};

pub const ENTRIES: &[super::Entry] = &[
    ("16a", "Adams, Stan || Carol Burnett vs. Anthony Perkins", || min_row(q16a())),
    ("16b", "!!!, Toy || & Teller", || min_row(q16b())),
    ("16c", "\"Brooklyn\" Tony Danza || (#1.5)", || min_row(q16c())),
    ("16d", "\"Brooklyn\" Tony Danza || (#1.5)", || min_row(q16d())),
    ("17a", "B, Khaz", || min_row(q17a())),
    ("17b", "Z'Dar, Robert", || min_row(q17b())),
    ("17c", "X'Volaitis, John", || min_row(q17c())),
    ("17d", "Abrahamsson, Bertil", || min_row(q17d())),
    ("17e", "$hort, Too", || min_row(q17e())),
    ("17f", "'El Galgo PornoStar', Blanquito", || min_row(q17f())),
    ("18a", "$1,000 || 10 || 40 Days and 40 Nights", || min_row(q18a())),
    ("18b", "Horror || 8.1 || Agorable", || min_row(q18b())),
    ("18c", "Action || 10 || #PostModem", || min_row(q18c())),
];

// q16a/q16d differ only in the episode_nr lower bound.
fn q16ad(lo: i64) -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(keyword.eq("character-name-in-title"))
          .and(episode_nr.ge(lo))
          .and(episode_nr.lt(100)))
       .select(cast.person().alias().text()
          .and(title))
}

fn q16a() -> impl Drive<R: Row> { q16ad(50) }
fn q16d() -> impl Drive<R: Row> { q16ad(5) }

fn q16b() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(keyword.eq("character-name-in-title")))
       .select(cast.person().alias().text()
          .and(title))
}

fn q16c() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(keyword.eq("character-name-in-title"))
          .and(episode_nr.lt(100)))
       .select(cast.person().alias().text()
          .and(title))
}

fn q17a() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(keyword.eq("character-name-in-title")))
       .select(cast.person().rx(r"^B"))
}

// q17b/c/d/f differ only in the person-name regex.
fn q17_any_co(re: &str) -> impl Drive<R: Row> {
    movie.with(company
          .and(keyword.eq("character-name-in-title")))
       .select(cast.person().rx(re))
}

fn q17b() -> impl Drive<R: Row> { q17_any_co(r"^Z") }
fn q17c() -> impl Drive<R: Row> { q17_any_co(r"^X") }
fn q17d() -> impl Drive<R: Row> { q17_any_co(r"Bert") }
fn q17f() -> impl Drive<R: Row> { q17_any_co(r"B") }

fn q17e() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(keyword.eq("character-name-in-title")))
       .select(cast.person().name())
}

fn ib_18a() -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe + Member {
    info.with(Info::ty.eq("budget")).info()
}

fn q18a() -> impl Drive<R: Row> {
    movie.with(ib_18a()
          .and(cast.select(Cast::note.is_in(["(producer)", "(executive producer)"])
                      .and(person.select(gender.eq("m")
                                    .and(Person::name.rx(r"Tim")))))))
       .select(ib_18a()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title))
}

// Conjunct/diff tree (∧ = Prod, - = Diff) — consumed via `member` only, so
// the value type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_18b() -> impl Query<D = Id<Info>> + Probe + Member {
    Info::ty.eq("genres")
        .and(Info::info.is_in(["Horror", "Thriller"]))
        .minus(Info::note)
}

fn q18b() -> impl Drive<R: Row> {
    movie.with(info.with(gf_18b())
          .and(production_year.ge(2008))
          .and(production_year.le(2014))
          .and(cast.select(Cast::note.is_in(writer5())
                      .and(person.select(gender.eq("f"))))))
       .select(info.with(gf_18b()).info()
          .and(data.with(Data::ty.eq("rating")
                    .and(Data::text.gt("8.0"))).text())
          .and(title))
}

fn gf_18c() -> impl Query<D = Id<Info>> + Probe + Member {
    Info::ty.eq("genres")
        .and(Info::info.is_in(genre6()))
}

fn q18c() -> impl Drive<R: Row> {
    movie.with(info.with(gf_18c())
          .and(cast.select(Cast::note.is_in(writer5())
                      .and(person.select(gender.eq("m"))))))
       .select(info.with(gf_18c()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title))
}
