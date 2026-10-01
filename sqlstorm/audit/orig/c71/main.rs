mod b152;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b152::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
