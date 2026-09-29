mod b151;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b151::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
