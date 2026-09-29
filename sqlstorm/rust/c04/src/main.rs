mod b34;
mod b35;
mod b36;
mod b37;
mod b38;
mod b39;
mod b40;
mod b41;
mod b42;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b34::ENTRIES, b35::ENTRIES, b36::ENTRIES, b37::ENTRIES, b38::ENTRIES, b39::ENTRIES, b40::ENTRIES, b41::ENTRIES, b42::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
