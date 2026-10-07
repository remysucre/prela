use ohdsi::schema::{self, Db};

static ENTRIES: &[harness::run::Entry<Db>] = &[
    ("c1106", ohdsi::queries::c1106::q),
    ("c255", ohdsi::queries::c255::q),
    ("c27", ohdsi::queries::c27::q),
    ("c29", ohdsi::queries::c29::q),
    ("c862", ohdsi::queries::c862::q),
];

fn main() {
    harness::run("ohdsi", schema::load, ENTRIES)
}
