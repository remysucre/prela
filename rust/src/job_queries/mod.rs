// All 113 JOB queries, plus the method-chain demo. See queries.rs.
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

/// Evaluate a query against a borrowed JOB and return only owned cells.
///
/// Query plans currently encode the database borrow as `'static` so the
/// production registry can remain a const table of plain function pointers.
/// The extension is contained here: the plan is fully consumed before this
/// function returns, and text cells are copied into owned `String`s.
#[cfg(feature = "test")]
pub fn differential(
    name: &str,
    db: &crate::job_schema::Job,
) -> Result<Vec<helpers::Result>, String> {
    // SAFETY: every query is consumed synchronously by `min_result`, and its
    // only borrowed output type (`&str`) is copied into `String` before the
    // private query dispatcher returns.
    let db: &'static crate::job_schema::Job = unsafe { &*std::ptr::from_ref(db) };
    queries::differential(name, db)
}
