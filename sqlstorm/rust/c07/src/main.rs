mod b59;
mod b60;
mod b61;
mod b62;
mod b63;
mod b64;
mod b65;
mod b66;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b59::ENTRIES, b60::ENTRIES, b61::ENTRIES, b62::ENTRIES, b63::ENTRIES, b64::ENTRIES, b65::ENTRIES, b66::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
