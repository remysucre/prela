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
pub type QFn = fn(&Data) -> String;
pub type Entry = (&'static str, &'static str, QFn);

pub fn all_queries() -> Vec<Entry> {
    let mut v: Vec<Entry> = Vec::new();
    v.extend_from_slice(t1::ENTRIES);
    v.extend_from_slice(t2::ENTRIES);
    v.extend_from_slice(t3::ENTRIES);
    v.extend_from_slice(t4::ENTRIES);
    v.extend_from_slice(t5::ENTRIES);
    v.extend_from_slice(t6::ENTRIES);
    v.extend_from_slice(demo_methods::ENTRIES);
    v
}
