// All 113 JOB queries — one entry per Julia _q(name, oracle) block —
// plus the method-chain demo. See queries.rs.
//
// Each query takes the loaded database and opens by destructuring the
// entities it touches (`let Movie { title, keyword, .. } = &db.movie;`).
// The bindings are the relations themselves, so the combinators hang off
// them directly. `db` is `&'static` (main leaks it once), which is why the
// plans these functions return carry no lifetime and `ENTRIES` can stay a
// `const` array of fn pointers.

pub mod helpers;
mod queries;
pub mod sets;

pub type Entry = crate::Entry<crate::job_schema::Job>;

pub fn all_queries() -> Vec<Entry> {
    queries::ENTRIES.to_vec()
}
