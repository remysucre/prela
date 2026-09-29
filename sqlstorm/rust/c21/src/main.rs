mod b97;
mod b98;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b97::ENTRIES, b98::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
