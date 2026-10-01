mod b43;
mod b44;
mod b45;
mod b46;
mod b47;
mod b48;
mod b49;
mod b50;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b43::ENTRIES, b44::ENTRIES, b45::ENTRIES, b46::ENTRIES, b47::ENTRIES, b48::ENTRIES, b49::ENTRIES, b50::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
