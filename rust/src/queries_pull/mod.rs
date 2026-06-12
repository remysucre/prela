// Pull-protocol ports of all JOB queries — IDENTICAL plans to src/queries/,
// only the consumption spelling changes (`min_row` → `min_row_pull`).
// Suite: `prela job-pull`.

pub mod helpers;
pub use crate::queries::sets;

mod demo_methods;
mod t1;
mod t2;
mod t3;
mod t4;
mod t5;
mod t6;

use crate::data::Data;
pub type Entry = crate::Entry<Data>;

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
