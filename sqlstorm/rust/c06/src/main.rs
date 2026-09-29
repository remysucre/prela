mod b51;
mod b52;
mod b53;
mod b54;
mod b55;
mod b56;
mod b57;
mod b58;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [
        b51::ENTRIES,
        b52::ENTRIES,
        b53::ENTRIES,
        b54::ENTRIES,
        b55::ENTRIES,
        b56::ENTRIES,
        b57::ENTRIES,
        b58::ENTRIES,
    ] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
