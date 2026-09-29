mod b88;
mod b89;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b88::ENTRIES, b89::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
