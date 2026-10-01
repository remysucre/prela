mod b10;
mod b11;
mod b12;
mod b13;
mod b14;
mod b15;
mod b16;
mod b17;
mod q;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b10::ENTRIES, b11::ENTRIES, b12::ENTRIES, b13::ENTRIES, b14::ENTRIES, b15::ENTRIES, b16::ENTRIES, b17::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
