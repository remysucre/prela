// queries: queries.jl lines 757-856 (templates 16-18)

use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::min_row;
use crate::queries::sets::{genre6, writer5};

pub const ENTRIES: &[super::Entry] = &[
    ("16a", "Adams, Stan || Carol Burnett vs. Anthony Perkins", q16a),
    ("16b", "!!!, Toy || & Teller", q16b),
    ("16c", "\"Brooklyn\" Tony Danza || (#1.5)", q16c),
    ("16d", "\"Brooklyn\" Tony Danza || (#1.5)", q16d),
    ("17a", "B, Khaz", q17a),
    ("17b", "Z'Dar, Robert", q17b),
    ("17c", "X'Volaitis, John", q17c),
    ("17d", "Abrahamsson, Bertil", q17d),
    ("17e", "$hort, Too", q17e),
    ("17f", "'El Galgo PornoStar', Blanquito", q17f),
    ("18a", "$1,000 || 10 || 40 Days and 40 Nights", q18a),
    ("18b", "Horror || 8.1 || Agorable", q18b),
    ("18c", "Action || 10 || #PostModem", q18c),
];

// q16a/q16d differ only in the episode_nr lower bound.
fn q16ad(lo: i64) -> String {
    min_row(movie()
        .when(company().country().eq("[us]")
              .and(keyword().text().eq("character-name-in-title"))
              .and(episode_nr().ge(lo))
              .and(episode_nr().lt(100)))
        .get(cast().person().alias().text()
             .and(title())))
}

fn q16a() -> String { q16ad(50) }
fn q16d() -> String { q16ad(5) }

fn q16b() -> String {
    min_row(movie()
        .when(company().country().eq("[us]")
              .and(keyword().text().eq("character-name-in-title")))
        .get(cast().person().alias().text()
             .and(title())))
}

fn q16c() -> String {
    min_row(movie()
        .when(company().country().eq("[us]")
              .and(keyword().text().eq("character-name-in-title"))
              .and(episode_nr().lt(100)))
        .get(cast().person().alias().text()
             .and(title())))
}

fn q17a() -> String {
    min_row(movie()
        .when(company().country().eq("[us]")
              .and(keyword().text().eq("character-name-in-title")))
        .get(cast().person().name().rx(r"^B")))
}

// q17b/c/d/f differ only in the person-name regex.
fn q17_any_co(re: &str) -> String {
    min_row(movie()
        .when(company()
              .and(keyword().text().eq("character-name-in-title")))
        .get(cast().person().name().rx(re)))
}

fn q17b() -> String { q17_any_co(r"^Z") }
fn q17c() -> String { q17_any_co(r"^X") }
fn q17d() -> String { q17_any_co(r"Bert") }
fn q17f() -> String { q17_any_co(r"B") }

fn q17e() -> String {
    min_row(movie()
        .when(company().country().eq("[us]")
              .and(keyword().text().eq("character-name-in-title")))
        .get(cast().person().name()))
}

fn ib_18a() -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    info().when(Info::ty().text().eq("budget")).info()
}

fn q18a() -> String {
    min_row(movie()
        .when(ib_18a()
              .and(cast().get(Cast::note().is_in(["(producer)", "(executive producer)"])
                              .and(person().get(gender().eq("m")
                                                .and(Person::name().rx(r"Tim")))))))
        .get(ib_18a()
             .and(data().when(Data::ty().text().eq("votes")).text())
             .and(title())))
}

// Conjunct/diff tree (∧ = Prod, - = Diff) — consumed via `member` only, so
// the value type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_18b() -> impl Query<D = Id<Info>> + Probe {
    Info::ty().text().eq("genres")
        .and(Info::info().is_in(["Horror", "Thriller"]))
        .minus(Info::note())
}

fn q18b() -> String {
    min_row(movie()
        .when(info().when(gf_18b())
              .and(production_year().ge(2008))
              .and(production_year().le(2014))
              .and(cast().get(Cast::note().is_in(writer5())
                              .and(person().get(gender().eq("f"))))))
        .get(info().when(gf_18b()).info()
             .and(data().when(Data::ty().text().eq("rating")
                              .and(Data::text().gt("8.0"))).text())
             .and(title())))
}

fn gf_18c() -> impl Query<D = Id<Info>> + Probe {
    Info::ty().text().eq("genres")
        .and(Info::info().is_in(genre6()))
}

fn q18c() -> String {
    min_row(movie()
        .when(info().when(gf_18c())
              .and(cast().get(Cast::note().is_in(writer5())
                              .and(person().get(gender().eq("m"))))))
        .get(info().when(gf_18c()).info()
             .and(data().when(Data::ty().text().eq("votes")).text())
             .and(title())))
}
