mod b94;
mod b95;
mod b96;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b94::ENTRIES, b95::ENTRIES, b96::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
