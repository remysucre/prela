mod b150;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b150::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
