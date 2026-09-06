// All 113 JOB queries, plus the method-chain demo. See queries.rs.
//
// `entries` destructures the loaded database's entities once
// (`let Movie { title, keyword, .. } = &db.movie;`) and defines every query
// as a closure over those bindings. The bindings are the relations
// themselves, so the combinators hang off them directly. `db` is `&'static`
// (main leaks it once), which is why the closures can be boxed as `'static`
// runners and the plans they return carry no lifetime.

pub mod helpers;
mod queries;
pub mod sets;

pub type Entry = (&'static str, &'static str, Box<dyn Fn(&'static crate::job_schema::Job) -> String>);

pub fn all_queries(db: &'static crate::job_schema::Job) -> Vec<Entry> {
    queries::entries(db)
}
