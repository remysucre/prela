// Query crate c00 — batches b00..b09, one module per batch of ten.
//
// The crate is the compilation unit and the file is the unit of work:
// ~7ms of compile per query means a 100-query crate rebuilds in about a
// second, while a crate per batch of ten would spend more time on cargo's
// per-crate overhead than on the queries themselves (see README).

mod b00;
mod b01;
mod b02;
mod b03;
mod b04;
mod b05;
mod b06;
mod b07;
mod b08;
mod b09;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b00::ENTRIES, b01::ENTRIES, b02::ENTRIES, b03::ENTRIES, b04::ENTRIES, b05::ENTRIES, b06::ENTRIES, b07::ENTRIES, b08::ENTRIES, b09::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
