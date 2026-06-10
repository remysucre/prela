// All 113 JOB queries — one entry per Julia _q(name, oracle) block.
// Each chunk file owns one slice of queries.jl; ALL stitches them together.

pub mod helpers;
pub mod sets;
pub mod demo_methods;

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
