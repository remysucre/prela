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

pub type Entry<T = String> = (&'static str, &'static str, Box<dyn Fn(&'static crate::job_schema::Job) -> T>);

pub fn all_queries(db: &'static crate::job_schema::Job) -> Vec<Entry> {
    queries::entries::<helpers::TextOutput>(db)
}

/// The production query plans with typed results for differential comparison.
#[cfg(feature = "test")]
pub fn typed_queries(db: &'static crate::job_schema::Job) -> Vec<Entry<Vec<helpers::Result>>> {
    queries::entries::<helpers::TypedOutput>(db)
}
