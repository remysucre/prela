// Column loading for struct-declared schemas.
//
// A schema is a set of plain Rust structs whose fields hold the relations
// themselves (src/job_schema.rs, src/tpch_schema.rs). An entity struct
// reads like a SQL table: `#[primary_key] id: Key<Self>` is the identity
// column over its ids, the rest are its columns with `Self` as domain, and
// `#[derive(IntoQuery)]` makes `&Entity` a query over the primary key.
// Filling one in is a struct literal, one call per field:
//
//     fn build(l: &mut Loader) -> Tpch {
//         Tpch { region: Region {
//             id: l.key("Region_name"),
//             name: l.strs("Region_name"), ..
//         }, .. }
//     }
//
// `Loader` exists for two reasons beyond terseness.
//
// FIRST, it keeps the declared type and the cache reader together. Each
// method names one physical column kind and delegates to the matching
// reader in src/cache.rs; picking the wrong method is caught at load, by
// `parse_header`, with the file name in the message.
//
// SECOND, it RECORDS what it was asked for. Run the same `build` body
// against `Loader::probing()` and nothing touches disk — every method
// returns an empty relation — but `manifest()` comes back holding one
// `(file, kind)` per column. That list is what `regen` verifies its output
// against (src/bin/regen.rs), and it cannot drift from the schema, because
// it is produced by the schema's own loading code rather than declared
// alongside it.
//
// This replaces the `MANIFEST` const that `schema!` used to generate.

use crate::cache;
use crate::engine::{Bitset, Dense, DictMultiRel, DictRel, Id, MultiRel, SparseUniverse, Universe, VecRel};
use crate::format::{KIND_CSR_STR, KIND_CSR_WORDS, KIND_DENSE_F64, KIND_DENSE_I64, KIND_DENSE_STR};
use std::path::Path;


/// A dense string column's value type — the cache bytes are leaked, so the
/// `&str`s borrow from the mmap and live for the program.
pub type Str = &'static str;

/// A scalar column of entity `E`. Spelled entity-first so a field reads
/// `Col<Lineitem, i64>` where the bare `VecRel<i64, Id<Lineitem>>` buries
/// the owner at the end.
pub type Col<E, T> = VecRel<T, Id<E>>;
/// A set-valued (CSR) column of entity `E`.
pub type Set<E, T> = MultiRel<T, Id<E>>;
/// The primary key of a densely-addressed entity `E`: its ids, `0..n`.
pub type Key<E> = Universe<Id<E>>;
/// The primary key of an entity whose id range has holes.
pub type SparseKey<E> = SparseUniverse<Id<E>>;
/// A dictionary-encoded column of entity `E`: what SQL normalises into a
/// lookup table (`kind_type`) is here just a `T`-valued column, stored as
/// codes plus a table.
pub type Dict<E, T> = DictRel<T, Id<E>>;
/// A set-valued dictionary-encoded column of entity `E`.
pub type DictSet<E, T> = DictMultiRel<T, Id<E>>;

/// A dense payload type the cache can hold, with its physical kind and
/// reader. This is what lets `l.dict(..)` pick the table reader from the
/// declared field type.
pub trait Scalar: Copy + 'static {
    const KIND: u32;
    fn load<D: Dense>(dir: &Path, name: &str) -> VecRel<Self, D>;
}
impl Scalar for Str {
    const KIND: u32 = KIND_DENSE_STR;
    fn load<D: Dense>(dir: &Path, name: &str) -> VecRel<Self, D> {
        cache::load_strs_in(dir, name)
    }
}
impl Scalar for i64 {
    const KIND: u32 = KIND_DENSE_I64;
    fn load<D: Dense>(dir: &Path, name: &str) -> VecRel<Self, D> {
        cache::load_i64_in(dir, name)
    }
}
impl Scalar for f64 {
    const KIND: u32 = KIND_DENSE_F64;
    fn load<D: Dense>(dir: &Path, name: &str) -> VecRel<Self, D> {
        cache::load_f64_in(dir, name)
    }
}

/// Reads columns and remembers which ones it read.
///
/// `new(dir)` loads for real; `probing()` reads nothing and returns empty
/// relations, so a `build` body run against it costs only the recording.
pub struct Loader<'a> {
    dir: Option<&'a Path>,
    seen: Vec<(String, u32)>,
}

impl<'a> Loader<'a> {
    pub fn new(dir: &'a Path) -> Self {
        Loader {
            dir: Some(dir),
            seen: Vec::new(),
        }
    }

    /// Record-only mode: no file is opened and every column comes back
    /// empty. Only `manifest()` is meaningful afterwards.
    pub fn probing() -> Self {
        Loader {
            dir: None,
            seen: Vec::new(),
        }
    }

    /// One `(file, kind)` per column asked for, in declaration order.
    /// `file` is the cache basename without `.bin` — `Lineitem_shipdate`.
    pub fn manifest(self) -> Vec<(String, u32)> {
        self.seen
    }

    #[inline]
    fn note(&mut self, name: &str, kind: u32) -> Option<&'a Path> {
        // A dictionary shared by several columns (`InfoType_text` under
        // `Info.ty`, `Data.ty`, `PersonInfo.ty`) is read more than once but
        // is one cache file; the manifest lists each file once.
        match self.seen.iter().find(|(n, _)| n == name) {
            Some((_, k)) => assert_eq!(*k, kind, "{name} declared with two different kinds"),
            None => self.seen.push((name.to_string(), kind)),
        }
        self.dir
    }

    // ===== the entity's primary key =====================================

    /// The primary key of a dense entity: `Key<E>` sized by
    /// the domain of column `name` — any dense column of `E` will do; by
    /// convention its value column. Only the header is read, and nothing is
    /// recorded: the column is already in the manifest under its own load.
    pub fn key<E: 'static>(&self, name: &str) -> Key<E> {
        match self.dir {
            Some(dir) => Universe::new(cache::n_dom_in(dir, name)),
            None => Universe::new(0),
        }
    }

    // ===== dense columns ================================================

    pub fn strs<E: 'static>(&mut self, name: &str) -> Col<E, Str> {
        match self.note(name, KIND_DENSE_STR) {
            Some(dir) => cache::load_strs_in(dir, name),
            None => VecRel::new(Vec::new()),
        }
    }

    pub fn i64s<E: 'static>(&mut self, name: &str) -> Col<E, i64> {
        match self.note(name, KIND_DENSE_I64) {
            Some(dir) => cache::load_i64_in(dir, name),
            None => VecRel::new(Vec::new()),
        }
    }

    pub fn f64s<E: 'static>(&mut self, name: &str) -> Col<E, f64> {
        match self.note(name, KIND_DENSE_F64) {
            Some(dir) => cache::load_f64_in(dir, name),
            None => VecRel::new(Vec::new()),
        }
    }

    /// A foreign key into `T`. The cache words are 0-based `Id<T>` with
    /// `NO_ID` in the holes, bulk-reinterpreted through `Id`'s
    /// `repr(transparent)` layout — no per-element conversion.
    ///
    /// Every entity in both suites is densely addressed, so an FK column
    /// composes straight onto any column of `T`: it is a
    /// `Query<D = Id<E>, R = Id<T>>` and `T`'s columns are keyed by
    /// `Id<T>`. There is no table hop to insert and none to spell.
    pub fn ids<T: 'static, E: 'static>(&mut self, name: &str) -> Col<E, Id<T>> {
        match self.note(name, KIND_DENSE_I64) {
            Some(dir) => cache::load_ids_in(dir, name),
            None => VecRel::new(Vec::new()),
        }
    }

    // ===== dictionary-encoded columns ===================================

    /// One entry per id: `codes` is a dense word column of codes into
    /// `table` (`l.dict("Movie_kind", "Kind_text")`). The table's payload
    /// type comes from the declared field type.
    pub fn dict<E: 'static, T: Scalar>(&mut self, codes: &str, table: &str) -> Dict<E, T> {
        let c = match self.note(codes, KIND_DENSE_I64) {
            Some(dir) => cache::load_words_in(dir, codes),
            None => VecRel::new(Vec::new()),
        };
        DictRel::new(c, self.table(table))
    }

    /// Zero or more entries per id: `codes` is a CSR word column.
    pub fn multi_dict<E: 'static, T: Scalar>(&mut self, codes: &str, table: &str) -> DictSet<E, T> {
        let c = match self.note(codes, KIND_CSR_WORDS) {
            Some(dir) => cache::load_multi_words_in(dir, codes),
            None => empty_csr(),
        };
        DictMultiRel::new(c, self.table(table))
    }

    /// The table of a dictionary column, keyed by code.
    fn table<T: Scalar>(&mut self, name: &str) -> VecRel<T, usize> {
        match self.note(name, T::KIND) {
            Some(dir) => T::load(dir, name),
            None => VecRel::new(Vec::new()),
        }
    }

    // ===== set-valued (CSR) columns =====================================

    pub fn multi_ids<T: 'static, E: 'static>(&mut self, name: &str) -> Set<E, Id<T>> {
        match self.note(name, KIND_CSR_WORDS) {
            Some(dir) => cache::load_multi_ids_in(dir, name),
            None => empty_csr(),
        }
    }

    pub fn multi_i64<E: 'static>(&mut self, name: &str) -> Set<E, i64> {
        match self.note(name, KIND_CSR_WORDS) {
            Some(dir) => cache::load_multi_i64_in(dir, name),
            None => empty_csr(),
        }
    }

    pub fn multi_strs<E: 'static>(&mut self, name: &str) -> Set<E, Str> {
        match self.note(name, KIND_CSR_STR) {
            Some(dir) => cache::load_multi_strs_in(dir, name),
            None => empty_csr(),
        }
    }
}

/// A CSR relation over zero keys. `MultiRel` borrows `&'static` slices (in
/// production they point into the leaked mmap); an empty slice literal is
/// already `'static` for any element type, so probe mode allocates nothing.
fn empty_csr<R: Copy + 'static, D: Dense>() -> MultiRel<R, D> {
    MultiRel::from_csr(&[0u32], &[])
}

/// Validity mask for a SPARSE entity — one whose dense id space carries
/// holes, like TPC-H `Order` over the gappy orderkey range. A slot is live
/// iff its foreign key is a real target rather than `NO_ID`.
///
/// The result is leaked because `SparseUniverse` holds `&'static Bitset`
/// (engine.rs:1069): a lifetime there would propagate into every query
/// type built over it. Callers cache it in a `OnceLock` so it is built
/// once per process.
/// The primary key of an entity whose id range has holes:
/// `SparseKey<E>` over the slots `0..fk.n_dom()`, masked to those
/// whose foreign key `fk` is a real target (`NO_ID` marks a hole). One pass
/// over `fk` at load; the mask is leaked because `SparseUniverse` holds
/// `&'static Bitset` (the same reason `MultiRel` holds `&'static` slices).
pub fn sparse_key<T: Dense, E: 'static>(fk: &VecRel<T, Id<E>>) -> SparseKey<E> {
    let mask: &'static Bitset<Id<E>> = Box::leak(Box::new(Bitset::validity(&fk.v)));
    SparseUniverse::new(fk.n_dom(), mask)
}

// =====================================================================
// Tests — a toy struct schema over a synthetic cache.
// =====================================================================
//
// These replace the two `schema!` tests that went away with the macro
// (loading every field-type arm, navigating across entities, and the
// manifest matching the declaration exactly). The schema below is written
// the way a real one is: `#[derive(IntoQuery)]` structs and one `build`.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Drive, IntoQuery, QueryExt};
    use crate::format::{HEADER_LEN, align8, header};
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    // ----- the toy schema ------------------------------------------------

    #[derive(IntoQuery)]
    pub struct Film {
        #[primary_key]
        pub id: Key<Self>,
        pub ftitle: Col<Self, Str>,
        pub year: Col<Self, i64>,
        pub genre: Col<Self, Id<Genre>>,
        pub tags: Set<Self, Id<Tag>>,
    }
    #[derive(IntoQuery)]
    pub struct Genre {
        #[primary_key]
        pub id: Key<Self>,
        pub gname: Col<Self, Str>,
        pub ty: Col<Self, Str>,
    }
    #[derive(IntoQuery)]
    pub struct Tag {
        #[primary_key]
        pub id: Key<Self>,
        pub tag: Col<Self, Str>,
        pub films: Set<Self, Id<Film>>,
    }
    pub struct Toy {
        pub film: Film,
        pub genre: Genre,
        pub tag: Tag,
    }

    fn build(l: &mut Loader) -> Toy {
        Toy {
            film: Film {
                id: l.key("Film_ftitle"),
                ftitle: l.strs("Film_ftitle"),
                year: l.i64s("Film_year"),
                genre: l.ids("Film_genre"),
                tags: l.multi_ids("Film_tags"),
            },
            genre: Genre {
                id: l.key("Genre_gname"),
                gname: l.strs("Genre_gname"),
                ty: l.strs("Genre_ty"),
            },
            tag: Tag {
                id: l.key("Tag_tag"),
                tag: l.strs("Tag_tag"),
                films: l.multi_ids("Tag_films"),
            },
        }
    }

    fn load(dir: &Path) -> Toy {
        build(&mut Loader::new(dir))
    }

    fn manifest() -> Vec<(String, u32)> {
        let mut l = Loader::probing();
        let _ = build(&mut l);
        l.manifest()
    }

    // ----- synthetic cache files (same writers the schema! tests used) ---

    fn write_col(dir: &PathBuf, name: &str, head: [u8; HEADER_LEN], payload: &[u8]) {
        let mut f = File::create(dir.join(format!("{name}.bin"))).unwrap();
        f.write_all(&head).unwrap();
        f.write_all(payload).unwrap();
    }

    fn dense_str(vals: &[&str]) -> ([u8; HEADER_LEN], Vec<u8>) {
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
        (header(KIND_DENSE_STR, vals.len() as u64, off as u64), payload)
    }

    fn dense_words(vals: &[u64]) -> ([u8; HEADER_LEN], Vec<u8>) {
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
            header(KIND_CSR_WORDS, (offsets.len() - 1) as u64, vals.len() as u64),
            payload,
        )
    }

    /// `tag` keeps concurrently-running tests off each other's files.
    fn fixture(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("prela_loader_test_{}_{tag}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let (h, p) = dense_str(&["Alien", "Blade", "Solaris"]);
        write_col(&dir, "Film_ftitle", h, &p);
        let (h, p) = dense_words(&[1979, 1998, 1972]);
        write_col(&dir, "Film_year", h, &p);
        // rows 0,2 → genre 1 ("scifi"); row 1 → genre 0 ("action")
        let (h, p) = dense_words(&[1, 0, 1]);
        write_col(&dir, "Film_genre", h, &p);
        // film 0 → tags {0,1}; film 1 → {1}; film 2 → {}
        let (h, p) = csr_words(&[0, 2, 3, 3], &[0, 1, 1]);
        write_col(&dir, "Film_tags", h, &p);

        let (h, p) = dense_str(&["action", "scifi"]);
        write_col(&dir, "Genre_gname", h, &p);
        let (h, p) = dense_str(&["broad", "narrow"]);
        write_col(&dir, "Genre_ty", h, &p);

        let (h, p) = dense_str(&["cult", "classic"]);
        write_col(&dir, "Tag_tag", h, &p);
        let (h, p) = csr_words(&[0, 1, 3], &[0, 0, 1]);
        write_col(&dir, "Tag_films", h, &p);

        dir
    }

    /// Every field-type arm loads, and composing across entities works:
    /// dense str, dense i64, an FK into another entity, and CSR ids.
    #[test]
    fn struct_schema_loads_types_and_composes() {
        let db = load(&fixture("compose"));
        let Film {
            id: _,
            ftitle,
            year,
            genre,
            tags,
        } = &db.film;
        let Genre { gname, ty: gty, .. } = &db.genre;
        let Tag { tag, films, .. } = &db.tag;

        // primary keys are sized off the value columns
        assert_eq!(db.film.id.n, 3);
        assert_eq!(db.genre.id.n, 2);
        assert_eq!(db.tag.id.n, 2);

        assert_eq!(ftitle.n_dom(), 3);
        assert_eq!(year.n_dom(), 3);
        assert_eq!(gname.n_dom(), 2);

        // FK compose: film → genre name, no hop to spell.
        let mut got: Vec<(Str, Str)> = Vec::new();
        db.film
            .select(ftitle.and(genre.select(gname)))
            .drive(|_, row| got.push(row));
        assert_eq!(
            got,
            [("Alien", "scifi"), ("Blade", "action"), ("Solaris", "scifi")]
        );

        // Two hops, and a scalar filter on the way.
        let mut broad: Vec<Str> = Vec::new();
        db.film
            .with(genre.select(gty).eq("broad"))
            .select(ftitle)
            .drive(|_, t| broad.push(t));
        assert_eq!(broad, ["Blade"]);

        // CSR fan-out: one row per (film, tag) pair.
        let mut pairs: Vec<(Str, Str)> = Vec::new();
        db.film
            .with(year.lt(1990))
            .select(ftitle.and(tags.select(tag)))
            .drive(|_, row| pairs.push(row));
        assert_eq!(pairs, [("Alien", "cult"), ("Alien", "classic")]);

        // The reverse CSR edge, driven from the other entity.
        let mut back: Vec<(Str, Str)> = Vec::new();
        db.tag
            .select(tag.and(films.select(ftitle)))
            .drive(|_, row| back.push(row));
        assert_eq!(
            back,
            [("cult", "Alien"), ("classic", "Alien"), ("classic", "Blade")]
        );
    }

    /// The manifest is exactly the declaration, in order, with each
    /// column's physical kind — and probe mode reads no files, so it
    /// works with no cache present at all.
    #[test]
    fn manifest_matches_the_declaration() {
        assert_eq!(
            manifest(),
            vec![
                ("Film_ftitle".to_string(), KIND_DENSE_STR),
                ("Film_year".to_string(), KIND_DENSE_I64),
                ("Film_genre".to_string(), KIND_DENSE_I64),
                ("Film_tags".to_string(), KIND_CSR_WORDS),
                ("Genre_gname".to_string(), KIND_DENSE_STR),
                ("Genre_ty".to_string(), KIND_DENSE_STR),
                ("Tag_tag".to_string(), KIND_DENSE_STR),
                ("Tag_films".to_string(), KIND_CSR_WORDS),
            ]
        );
    }

    /// A column read at the wrong declared type fails loudly, naming the
    /// file — the check that used to live in the macro's `@load` arms.
    #[test]
    #[should_panic(expected = "cache/loader mismatch")]
    fn wrong_declared_type_fails_loudly() {
        let dir = fixture("wrongtype");
        let mut l = Loader::new(&dir);
        // Film_year is DENSE_I64; asking for f64 must not silently work.
        let _: Col<Film, f64> = l.f64s("Film_year");
    }
}
