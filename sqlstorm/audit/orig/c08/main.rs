mod b67;
mod b68;
mod b69;
mod b70;
mod b71;
mod b72;
mod b73;
mod b74;
mod b75;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [b67::ENTRIES, b68::ENTRIES, b69::ENTRIES, b70::ENTRIES, b71::ENTRIES, b72::ENTRIES, b73::ENTRIES, b74::ENTRIES, b75::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
