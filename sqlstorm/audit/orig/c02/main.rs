mod b18;
mod b19;
mod b20;
mod b21;
mod b22;
mod b23;
mod b24;
mod b25;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b18::ENTRIES, b19::ENTRIES, b20::ENTRIES, b21::ENTRIES, b22::ENTRIES, b23::ENTRIES, b24::ENTRIES, b25::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
