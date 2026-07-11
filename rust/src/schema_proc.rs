// `schema_proc!` — the proc-macro twin of `schema!` (src/schema.rs): same
// surface syntax, same generated items, but implemented as a real parser +
// AST walk in the `prela-schema-proc` crate (schema-proc/src/lib.rs) instead
// of macro_rules tt-munchers. Kept alongside the original for comparison;
// nothing in the crate invokes it outside its own tests.
//
// NOTE: proc macros have no `$crate`, so the generated code spells paths
// `crate::engine`/`crate::cache`/`crate::format` — usable only inside prela.

pub use prela_schema_proc::schema_proc;

// ===== tests — the same tiny schemas as schema.rs, via the proc macro =====

#[cfg(test)]
mod tests {
    use crate::engine::*;
    use crate::format::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    // Three entities exercising every field-type arm: dense str, dense
    // i64, FK (entity ident), Multi<entity>, Multi<str>.
    crate::schema_proc::schema_proc! {
        TESTS / TestSchema / test_init:
        Film(film) / FilmNav { pub ftitle: str, pub year: i64, genre: Genre, tags: Multi<Tag> }
        Genre / GenreNav { gname: str, ty: str }
        Tag / TagNav { tag: str, films: Multi<Film> }
    }

    pub(super) fn write_v2(dir: &PathBuf, name: &str, head: [u8; HEADER_LEN], payload: &[u8]) {
        let mut f = File::create(dir.join(format!("{name}.bin"))).unwrap();
        f.write_all(&head).unwrap();
        f.write_all(payload).unwrap();
    }

    pub(super) fn dense_str(vals: &[&str]) -> ([u8; HEADER_LEN], Vec<u8>) {
        let mut payload = Vec::new();
        let mut off = 0u32;
        payload.extend_from_slice(&off.to_le_bytes());
        for v in vals {
            off += v.len() as u32;
            payload.extend_from_slice(&off.to_le_bytes());
        }
        for v in vals {
            payload.extend_from_slice(v.as_bytes());
        }
        (
            header(KIND_DENSE_STR, vals.len() as u64, off as u64),
            payload,
        )
    }

    pub(super) fn dense_words(vals: &[u64]) -> ([u8; HEADER_LEN], Vec<u8>) {
        let mut payload = Vec::new();
        for v in vals {
            payload.extend_from_slice(&v.to_le_bytes());
        }
        (header(KIND_DENSE_I64, vals.len() as u64, 0), payload)
    }

    fn csr_words(offsets: &[u32], vals: &[u64]) -> ([u8; HEADER_LEN], Vec<u8>) {
        let mut payload = Vec::new();
        for o in offsets {
            payload.extend_from_slice(&o.to_le_bytes());
        }
        payload.resize(align8(HEADER_LEN + payload.len()) - HEADER_LEN, 0);
        for v in vals {
            payload.extend_from_slice(&v.to_le_bytes());
        }
        (
            header(
                KIND_CSR_WORDS,
                (offsets.len() - 1) as u64,
                vals.len() as u64,
            ),
            payload,
        )
    }

    #[test]
    fn schema_proc_macro_loads_types_and_navigates() {
        let dir =
            std::env::temp_dir().join(format!("prela_schema_proc_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        // films: 0 "Alien" 1979 genre 1 tags {0}, 1 "Blade" 1998 genre 0 tags {0, 1}
        let (h, p) = dense_str(&["Alien", "Blade"]);
        write_v2(&dir, "Film_ftitle", h, &p);
        let (h, p) = dense_words(&[1979, 1998]);
        write_v2(&dir, "Film_year", h, &p);
        let (h, p) = dense_words(&[1, 0]);
        write_v2(&dir, "Film_genre", h, &p);
        let (h, p) = csr_words(&[0, 1, 3], &[0, 0, 1]);
        write_v2(&dir, "Film_tags", h, &p);
        // genres: 0 "drama"/"main", 1 "horror"/"sub"
        let (h, p) = dense_str(&["drama", "horror"]);
        write_v2(&dir, "Genre_gname", h, &p);
        let (h, p) = dense_str(&["main", "sub"]);
        write_v2(&dir, "Genre_ty", h, &p);
        // tags: 0 "cult" films {0, 1}, 1 "noir" films {1}
        let (h, p) = dense_str(&["cult", "noir"]);
        write_v2(&dir, "Tag_tag", h, &p);
        let (h, p) = csr_words(&[0, 2, 3], &[0, 1, 1]);
        write_v2(&dir, "Tag_films", h, &p);

        test_init(&dir);

        // universe size = first column's key count (the universe HANDLE
        // resolves to the `Universe` value via `iq`)
        assert_eq!(film.iq().n, 2);

        // typed composition across three entities, in navigation form:
        // a predicate ROOT is a paren-free handle (qualified `Film::genre`,
        // bare `year` for pub fields); every later hop is a nav method
        // (`.gname()` ≡ `.select(Genre::gname)` via the generated GenreNav).
        let q = film
            .with(Film::genre.gname().eq("horror"))
            .with(year.lt(1990))
            .ftitle();
        let mut got = Vec::new();
        q.drive(|_, t| got.push(t));
        assert_eq!(got, vec!["Alien"]);

        // primary-field ELISION: `Film::genre.eq("horror")` auto-navigates to
        // Genre's primary (gname) — identical result to the explicit
        // `.gname().eq(..)` above. Genre is `Primary` (first field `gname:
        // str`); the scalar `year.lt(1990)` is the identity (Field) case.
        let q = film
            .with(Film::genre.eq("horror"))
            .with(year.lt(1990))
            .ftitle();
        let mut got = Vec::new();
        q.drive(|_, t| got.push(t));
        assert_eq!(got, vec!["Alien"]);

        // Multi<entity> column + nav through Tag's tag column
        let q = film.with(Film::tags.tag().eq("noir")).ftitle();
        let mut got = Vec::new();
        q.drive(|_, t| got.push(t));
        assert_eq!(got, vec!["Blade"]);

        // same-named nav methods on different entities resolve by the
        // receiver's RESOLVED value type: Tag::films is Film-valued, so
        // `.year()` picks FilmNav; the chain navigates Film → Genre → gname.
        let mut got = Vec::new();
        Tag::films.year().probe(Id::new(0), |y| got.push(y));
        assert_eq!(got, vec![1979, 1998]);
        let mut got = Vec::new();
        Tag::films
            .genre()
            .gname()
            .probe(Id::new(1), |g| got.push(g));
        assert_eq!(got, vec!["drama"]);

        // field names are filenames verbatim (`ty` → Genre_ty.bin); a
        // handle in leaf (non-chain) position resolves explicitly via `iq`
        let mut got = Vec::new();
        Genre::ty.iq().probe(Id::new(0), |v| got.push(v));
        assert_eq!(got, vec!["main"]);

        // typed ids round-trip the bulk reinterpret: Film_genre words → Id<Genre>
        let mut got = Vec::new();
        Film::genre.iq().probe(Id::<Film>::new(1), |g| got.push(g));
        assert_eq!(got, vec![Id::<Genre>::new(0)]);

        // the generated manifest names every column with its cache kind
        assert_eq!(
            MANIFEST,
            &[
                ("Film", "ftitle", KIND_DENSE_STR),
                ("Film", "year", KIND_DENSE_I64),
                ("Film", "genre", KIND_DENSE_I64),
                ("Film", "tags", KIND_CSR_WORDS),
                ("Genre", "gname", KIND_DENSE_STR),
                ("Genre", "ty", KIND_DENSE_STR),
                ("Tag", "tag", KIND_DENSE_STR),
                ("Tag", "films", KIND_CSR_WORDS),
            ]
        );
    }
}

// A schema with a NON-DENSE (`dict`) entity — see schema.rs dict_tests.
#[cfg(test)]
mod dict_tests {
    use super::tests::{dense_str, dense_words, write_v2};
    use crate::engine::*;

    crate::schema_proc::schema_proc! { DICTT / DictSchema / dictt_init:
        Movie(movie) / MovieNav { studio: Studio, year: i64 }
        Studio(studio dict) / StudioNav { id: i64, sname: str }
    }

    #[test]
    fn dict_entity_loads_and_navigates() {
        let dir = std::env::temp_dir().join(format!("prela_dict_proc_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        // Studio: EXTERNAL ids 100/205/9899 at dense rows 0/1/2; names. The id
        // column is what the DictTable inverts (external id → row).
        let (h, p) = dense_words(&[100, 205, 9899]);
        write_v2(&dir, "Studio_id", h, &p);
        let (h, p) = dense_str(&["Warner", "A24", "Mubi"]);
        write_v2(&dir, "Studio_sname", h, &p);
        // Movie.studio: FK storing the external KEYS (205, 100) — NOT row ids.
        let (h, p) = dense_words(&[205, 100]);
        write_v2(&dir, "Movie_studio", h, &p);
        let (h, p) = dense_words(&[2008, 1999]);
        write_v2(&dir, "Movie_year", h, &p);

        dictt_init(&dir);

        // movie.studio().sname() — `.studio()` crosses Studio's DictTable (built
        // lazily from `Studio.id`), then `.sname()` reads the column. The FK is
        // a non-dense Key, resolved to a row by the table.
        let q = movie.studio().sname();
        let mut got = Vec::new();
        q.drive(|m, n| got.push((m.idx(), n)));
        got.sort();
        // movie 0 → studio key 205 → row 1 → "A24"; movie 1 → key 100 → row 0 → "Warner"
        assert_eq!(got, vec![(0, "A24"), (1, "Warner")]);

        // the FK column genuinely stores a non-dense Key (not a row Id): the raw
        // handle `Movie::studio` resolves to the Key column, un-followed.
        let mut keys = Vec::new();
        movie
            .select(Movie::studio)
            .drive(|m, k: Key<Studio>| keys.push((m.idx(), k.0)));
        keys.sort();
        assert_eq!(keys, vec![(0, 205), (1, 100)]);
    }
}
