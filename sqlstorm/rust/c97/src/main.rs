mod b178;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b178::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
