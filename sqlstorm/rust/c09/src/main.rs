mod b76;
mod b77;
mod q;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b76::ENTRIES, b77::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
