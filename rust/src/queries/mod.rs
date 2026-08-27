// All 113 JOB queries — one entry per Julia _q(name, oracle) block —
// plus the method-chain demo. Each chunk file owns one slice of
// queries.jl; ALL stitches them together.
//
// Each query takes the loaded database and opens by destructuring the
// entities it touches (`let Movie { title, keyword, .. } = &db.movie;`).
// The bindings are the relations themselves, so the combinators hang off
// them directly. `db` is `&'static` (main leaks it once), which is why the
// plans these functions return carry no lifetime and `ENTRIES` can stay a
// `const` array of fn pointers.

pub mod helpers;
pub mod sets;
pub mod demo_methods;

mod t1;
mod t2;
mod t3;
mod t4;
mod t5;
mod t6;

pub type Entry = crate::Entry<crate::job_schema::Job>;

pub fn all_queries() -> Vec<Entry> {
    [
        t1::ENTRIES,
        t2::ENTRIES,
        t3::ENTRIES,
        t4::ENTRIES,
        t5::ENTRIES,
        t6::ENTRIES,
        demo_methods::ENTRIES,
    ].concat()
}
