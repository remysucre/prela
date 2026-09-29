mod b79;
mod b80;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b79::ENTRIES, b80::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
