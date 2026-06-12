// queries: queries.jl lines 757-856 (templates 16-18)
use crate::data::Data;
use crate::engine::*;
use super::helpers::*;

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
fn q16ad(d: &Data, lo: i64) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).o((&d.company_country).eq("[us]"))
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title"))
            .and((&d.movie_episode_nr).ge(lo))
            .and((&d.movie_episode_nr).lt(100))
    ).o(
        (&d.movie_cast).o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
            .x(&d.movie_title)
    );
    min_row(q)
}

fn q16a(d: &Data) -> String { q16ad(d, 50) }
fn q16d(d: &Data) -> String { q16ad(d, 5) }

fn q16b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).o((&d.company_country).eq("[us]"))
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title"))
    ).o(
        (&d.movie_cast).o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
            .x(&d.movie_title)
    );
    min_row(q)
}

fn q16c(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).o((&d.company_country).eq("[us]"))
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title"))
            .and((&d.movie_episode_nr).lt(100))
    ).o(
        (&d.movie_cast).o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
            .x(&d.movie_title)
    );
    min_row(q)
}

fn q17a(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).o((&d.company_country).eq("[us]"))
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title"))
    ).o(
        (&d.movie_cast).o((&d.cast_person).o((&d.person_name).rx(r"^B")))
    );
    min_row(q)
}

// q17b/c/d/f differ only in the person-name regex.
fn q17_any_co(d: &Data, re: &str) -> String {
    let q = d.movie.in_s(
        (&d.movie_company)
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title"))
    ).o(
        (&d.movie_cast).o((&d.cast_person).o((&d.person_name).rx(re)))
    );
    min_row(q)
}

fn q17b(d: &Data) -> String { q17_any_co(d, r"^Z") }
fn q17c(d: &Data) -> String { q17_any_co(d, r"^X") }
fn q17d(d: &Data) -> String { q17_any_co(d, r"Bert") }
fn q17f(d: &Data) -> String { q17_any_co(d, r"B") }

fn q17e(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).o((&d.company_country).eq("[us]"))
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title"))
    ).o(
        (&d.movie_cast).o((&d.cast_person).o(&d.person_name))
    );
    min_row(q)
}

fn ib_18a<'d>(d: &'d Data) -> impl Rel<R = &'static str, D = usize> + Drive + Probe + 'd {
    (&d.movie_info).in_s((&d.info_type).o(&d.infotype_info).eq("budget")).o(&d.info_info)
}

fn q18a(d: &Data) -> String {
    let q = d.movie.in_s(
        ib_18a(d)
            .and((&d.movie_cast).in_s(
                (&d.cast_note).in_v(vec!["(producer)", "(executive producer)"])
                    .and((&d.cast_person).in_s(
                        (&d.person_gender).eq("m")
                            .and((&d.person_name).rx(r"Tim"))
                    ))
            ))
    ).o(
        ib_18a(d)
            .x((&d.movie_data).in_s((&d.data_type).o(&d.infotype_info).eq("votes")).o(&d.data_data))
            .x(&d.movie_title)
    );
    min_row(q)
}

// Conjunct/diff tree (∧ = Prod, - = Diff) — consumed via `member` only, so
// the value type stays opaque (`impl Rel<D = usize> + Probe`).
fn gf_18b<'d>(d: &'d Data) -> impl Rel<D = usize> + Probe + 'd {
    (&d.info_type).o(&d.infotype_info).eq("genres")
        .and((&d.info_info).in_v(vec!["Horror", "Thriller"]))
        .minus(&d.info_note)
}

fn q18b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_info).in_s(gf_18b(d))
            .and((&d.movie_production_year).ge(2008))
            .and((&d.movie_production_year).le(2014))
            .and((&d.movie_cast).in_s(
                (&d.cast_note).in_v(super::sets::writer5())
                    .and((&d.cast_person).in_s((&d.person_gender).eq("f")))
            ))
    ).o(
        (&d.movie_info).in_s(gf_18b(d)).o(&d.info_info)
            .x((&d.movie_data).in_s(
                (&d.data_type).o(&d.infotype_info).eq("rating")
                    .and((&d.data_data).gt("8.0"))
            ).o(&d.data_data))
            .x(&d.movie_title)
    );
    min_row(q)
}

fn gf_18c<'d>(d: &'d Data) -> impl Rel<D = usize> + Probe + 'd {
    (&d.info_type).o(&d.infotype_info).eq("genres")
        .and((&d.info_info).in_v(super::sets::genre6()))
}

fn q18c(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_info).in_s(gf_18c(d))
            .and((&d.movie_cast).in_s(
                (&d.cast_note).in_v(super::sets::writer5())
                    .and((&d.cast_person).in_s((&d.person_gender).eq("m")))
            ))
    ).o(
        (&d.movie_info).in_s(gf_18c(d)).o(&d.info_info)
            .x((&d.movie_data).in_s((&d.data_type).o(&d.infotype_info).eq("votes")).o(&d.data_data))
            .x(&d.movie_title)
    );
    min_row(q)
}
