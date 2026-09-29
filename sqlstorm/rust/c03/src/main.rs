mod b26;
mod b27;
mod b28;
mod b29;
mod b30;
mod b31;
mod b32;
mod b33;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b26::ENTRIES, b27::ENTRIES, b28::ENTRIES, b29::ENTRIES, b30::ENTRIES, b31::ENTRIES, b32::ENTRIES, b33::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
