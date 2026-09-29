mod b101;
mod b102;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b101::ENTRIES, b102::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
