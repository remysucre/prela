mod b85;
mod b86;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b85::ENTRIES, b86::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
