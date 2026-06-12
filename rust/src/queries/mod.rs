// All 113 JOB queries — one entry per Julia _q(name, oracle) block —
// plus the method-chain demo. Each chunk file owns one slice of
// queries.jl; ALL stitches them together. Queries read the typed schema's
// global `OnceLock` store (src/job_schema.rs), so runners take no data
// argument — call `job_schema::job_init` once before running.

pub mod helpers;
pub mod sets;
pub mod demo_methods;

mod t1;
mod t2;
mod t3;
mod t4;
mod t5;
mod t6;

pub type Entry = crate::Entry;

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
