// All 113 JOB queries — one entry per Julia _q(name, oracle) block —
// plus the method-chain demo. Queries read the typed schema's global
// `OnceLock` store (src/job_schema.rs), so runners take no data argument
// — call `job_schema::job_init` once before running.

pub mod helpers;
pub mod sets;
pub mod demo_methods;

mod queries;

pub type Entry = crate::Entry;

pub fn all_queries() -> Vec<Entry> {
    [
        queries::ENTRIES,
        demo_methods::ENTRIES,
    ].concat()
}
