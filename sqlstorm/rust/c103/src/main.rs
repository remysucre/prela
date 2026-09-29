mod b184;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b184::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
