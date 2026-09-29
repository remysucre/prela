mod b173;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b173::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
