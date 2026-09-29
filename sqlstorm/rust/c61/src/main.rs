mod b140;
mod b141;
mod b142;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b140::ENTRIES, b141::ENTRIES, b142::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
