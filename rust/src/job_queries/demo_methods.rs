// Reference example of the method-chain form. Kept as a registered query so
// `cargo asm` always has a known symbol to inspect.

use crate::engine::*;
use crate::job_queries::helpers::{Row, min_row};
use crate::job_schema::*;

pub const ENTRIES: &[super::Entry] = &[(
    "6a/method",
    "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert",
    |db| min_row(q6a_methods(db)),
)];

// q6a — movie : (year > 2010) ∧ (keyword == "marvel-...")
//             → (keyword == "marvel-...") × title
//             × (cast → person → name ~ "Downey…")
//
// Operator legend (engine.rs::QueryExt; everything roots on IntoQuery, so
// the destructured columns — `keyword`, `title`, `cast` — mix freely with
// plan nodes):
//   .select(b)    composition (a set is an identity relation, so set∘Query is
//            the same Compose — no keyset projection), and also how you
//            navigate: `cast.select(person).select(person_name)` walks
//            Movie → Cast → Person → name, one column per hop
//   .select(keyword_text)  the ID→label hop that `Primary` used to elide
//            behind `keyword.eq(..)`; with no global store to read the
//            primary column from, it is written out
//   .and(b)    product (×)
//   .and     ∧ — alias for the product; conjunct trees are consumed via
//            the flat short-circuit `member` (restriction = `.with`)
//   .or      ∨ — probe-only membership union (drive with `.union`)
//   .minus   value-bearing difference (key-based member test)
//   .with    restriction (Julia `:`) — keep rows whose value is a member
//   .eq / .ne / .gt / .lt / .ge / .le / .is_in / .rx / .nrx  predicates
pub fn q6a_methods(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Keyword {
        text: keyword_text, ..
    } = &db.keyword;
    let Person {
        name: person_name, ..
    } = &db.person;
    let movie = db.movie.all();
    let kw_marvel = || keyword.select(keyword_text).eq("marvel-cinematic-universe");
    let q = movie
        .with(production_year.gt(2010).and(kw_marvel()))
        .select(
            kw_marvel().and(title).and(
                cast.select(person)
                    .select(person_name)
                    .rx(r"Downey.*Robert"),
            ),
        );
    q
}
