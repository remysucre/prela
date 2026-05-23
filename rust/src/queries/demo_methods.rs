// Reference example of the method-chain form. Kept as a registered query so
// `cargo asm` always has a known symbol to inspect.

use crate::data::Data;
use crate::engine::*;
use super::helpers::*;

pub const ENTRIES: &[(&str, &str, fn(&Data) -> String)] = &[
    ("6a/method", "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", q6a_methods),
];

// q6a — movie → (year > 2010) ∧ (keyword == "marvel-...")
//             : (keyword == "marvel-...") × title
//             × (cast → person → name ~ "Downey…")
//
// Operator legend (engine.rs::QueryExt / SetQExt):
//   .o(b)    composition (Query∘Query or SetQ∘Query — same algebra)
//   .k()     keys (Query → SetQ)
//   .x(b)    product (×)
//   .and / .or / .minus  set algebra
//   .eq / .ne / .gt / .lt / .ge / .le / .in_v / .in_s / .rx / .nrx  predicates
pub fn q6a_methods(d: &Data) -> String {
    let kw_marvel = || (&d.movie_keyword).o(&d.keyword_keyword)
                                          .eq("marvel-cinematic-universe");
    let q = d.movie.o(
        (&d.movie_production_year).gt(2010).k()
            .and(kw_marvel().k())
            .o(
                kw_marvel()
                    .x(&d.movie_title)
                    .x((&d.movie_cast).o(
                        (&d.cast_person).o(
                            (&d.person_name).rx(r"Downey.*Robert")))),
            ),
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((kw, title), name)| {
        update(&mut m[0], kw);
        update(&mut m[1], title);
        update(&mut m[2], name);
    });
    fmt3(m)
}
