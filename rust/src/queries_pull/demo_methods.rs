// PULL-PROTOCOL PORT of src/queries/demo_methods.rs — identical plans, pull sinks.
// Reference example of the method-chain form. Kept as a registered query so
// `cargo asm` always has a known symbol to inspect.

use crate::data::Data;
use crate::engine::*;
use super::helpers::*;

pub const ENTRIES: &[super::Entry] = &[
    ("6a/method", "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", q6a_methods),
];

// q6a — movie : (year > 2010) ∧ (keyword == "marvel-...")
//             → (keyword == "marvel-...") × title
//             × (cast → person → name ~ "Downey…")
//
// Operator legend (engine.rs::QueryExt):
//   .o(b)    composition (a set is an identity relation, so set∘Query is
//            the same Compose — no keyset projection)
//   .x(b)    product (×)
//   .and     ∧ — alias for the product; conjunct trees are consumed via
//            the flat short-circuit `member` (restriction = `.in_s`)
//   .or      ∨ — probe-only membership union (drive with `.union`)
//   .minus   value-bearing difference (key-based member test)
//   .in_s    restriction (Julia `:`) — keep rows whose value is a member
//   .eq / .ne / .gt / .lt / .ge / .le / .in_v / .rx / .nrx  predicates
pub fn q6a_methods(d: &Data) -> String {
    let kw_marvel = || (&d.movie_keyword).o(&d.keyword_keyword)
                                          .eq("marvel-cinematic-universe");
    let q = d.movie.in_s(
        (&d.movie_production_year).gt(2010)
            .and(kw_marvel()),
    ).o(
        kw_marvel()
            .x(&d.movie_title)
            .x((&d.movie_cast).o(
                (&d.cast_person).o(
                    (&d.person_name).rx(r"Downey.*Robert")))),
    );
    min_row_pull(q)
}
