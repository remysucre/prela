mod b81;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b81::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
