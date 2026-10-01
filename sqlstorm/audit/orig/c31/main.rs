mod b110;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b110::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
