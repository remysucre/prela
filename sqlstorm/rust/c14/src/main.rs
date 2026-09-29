mod b83;
mod b84;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b83::ENTRIES, b84::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
