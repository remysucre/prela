mod b99;
mod b100;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b99::ENTRIES, b100::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
