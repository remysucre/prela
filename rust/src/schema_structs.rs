// The struct/field schema — the "absolutely minimal, no macros, no traits of
// our own" design, written to see exactly HOW it breaks.
//
// Design in one line: an entity is a struct, a column is a field holding the
// loaded relation. There is no `Col`, no `ColType`, no `Spec`, no registry,
// no `init`, no `Entity` trait, no `cols!`. Loading is a struct literal.
//
//     pub struct Films {
//         pub title: VecRel<Str, Id<Films>>,
//         pub genre: VecRel<Id<Genres>, Id<Films>>,
//     }
//
// The entity struct doubles as its own id tag: `Id<Films>` phantoms over the
// very struct that holds the columns, so no separate ZST is needed.
//
// Everything below compiles and passes. The interesting part is the
// `// BREAKS:` blocks — real code, commented out, each with the compiler
// error it produces. See `mod breakage` at the bottom for the catalogue.

use crate::cache;
use crate::engine::{DictTable, Id, Key, MultiRel, Universe, VecRel};
use std::path::Path;

/// Same spelling as the other designs, so declarations read alike.
pub type Str = &'static str;

// ===== the entities ======================================================
// Each is a plain struct of loaded relations plus a hand-written `load`.
// Note what is NOT here: any indirection between "column" and "relation".
// The field IS the relation. That is the whole design.

pub struct Films {
    pub title: VecRel<Str, Id<Films>>,
    pub year: VecRel<i64, Id<Films>>,
    pub rating: VecRel<f64, Id<Films>>,
    /// FK into a DENSE entity: range is already `Id<Genres>`, so navigation
    /// needs no hop — `Ident` would have been the identity anyway.
    pub genre: VecRel<Id<Genres>, Id<Films>>,
    pub tags: MultiRel<Id<Tags>, Id<Films>>,
    /// FK into a DICT entity: stores EXTERNAL keys (205, 100, ...), not rows.
    /// The key→row hop has nowhere to live on a bare field — see `Db::studio`.
    pub studio: VecRel<Key<Studios>, Id<Films>>,
}

impl Films {
    pub fn load(dir: &Path) -> Self {
        Films {
            title: cache::load_strs_in(dir, "Film_title"),
            year: cache::load_i64_in(dir, "Film_year"),
            rating: cache::load_f64_in(dir, "Film_rating"),
            genre: cache::load_ids_in(dir, "Film_genre"),
            tags: cache::load_multi_ids_in(dir, "Film_tags"),
            // No `load_fk_in` here: that helper is generic over `EntityKind`,
            // a TRAIT — and this design has none. So the dict FK is loaded as
            // raw i64 and mapped by hand. First crack: "no traits" means no
            // shared vocabulary for "what does this entity store as its FK".
            studio: VecRel::new(
                cache::load_i64_in::<Id<Films>>(dir, "Film_studio")
                    .values
                    .into_iter()
                    .map(|k| Key::new(k as u64))
                    .collect(),
            ),
        }
    }

    /// The universe. A METHOD, not an associated fn: it needs `self` to know
    /// how many rows there are, because the count lives in a loaded column.
    pub fn all(&self) -> Universe<Id<Films>> {
        Universe::new(self.title.n_keys())
    }
}

pub struct Genres {
    pub name: VecRel<Str, Id<Genres>>,
    pub kind: VecRel<Str, Id<Genres>>,
}

impl Genres {
    pub fn load(dir: &Path) -> Self {
        Genres {
            name: cache::load_strs_in(dir, "Genre_name"),
            kind: cache::load_strs_in(dir, "Genre_kind"),
        }
    }
    pub fn all(&self) -> Universe<Id<Genres>> {
        Universe::new(self.name.n_keys())
    }
}

pub struct Tags {
    pub text: VecRel<Str, Id<Tags>>,
    pub films: MultiRel<Id<Films>, Id<Tags>>,
}

impl Tags {
    pub fn load(dir: &Path) -> Self {
        Tags {
            text: cache::load_strs_in(dir, "Tag_text"),
            films: cache::load_multi_ids_in(dir, "Tag_films"),
        }
    }
}

pub struct Studios {
    pub id: VecRel<i64, Id<Studios>>,
    pub name: VecRel<Str, Id<Studios>>,
}

impl Studios {
    pub fn load(dir: &Path) -> Self {
        Studios {
            id: cache::load_i64_in(dir, "Studio_id"),
            name: cache::load_strs_in(dir, "Studio_name"),
        }
    }
}

// ===== the database ======================================================
// Every entity in one value, because a query that navigates touches more
// than one entity and fields can only be reached through an instance.

pub struct Db {
    pub films: Films,
    pub genres: Genres,
    pub tags: Tags,
    pub studios: Studios,
    /// Studios' ID → ROW column (external key 205 → row 1).
    ///
    /// EVERY entity conceptually has one of these. For a dense entity the
    /// external id IS the row, so it is the identity and compiles away to
    /// nothing — which is why `Films`, `Genres` and `Tags` have no such field
    /// here. Only `Studios`, whose ids are sparse (100/205/9899), needs it
    /// materialized as a real `DictTable`.
    ///
    /// Note the asymmetry this design forces: in the token version the ID→Row
    /// hop lives INSIDE the column's resolution (`Compose<_, Ident<E>>` for
    /// dense, `Compose<_, &DictTable<E>>` for dict), so both cases are written
    /// and read identically. With bare fields it has no home on the column, so
    /// it is hoisted to the `Db` and spliced in by hand — and only for the
    /// dict case. The no-op does not "compile away" so much as go MISSING,
    /// which is why dense and dict queries end up spelled differently.
    pub studio_table: DictTable<Studios>,
}

impl Db {
    pub fn load(dir: &Path) -> Self {
        let studios = Studios::load(dir);
        let studio_table = DictTable::from_i64(&studios.id.values);
        Db {
            films: Films::load(dir),
            genres: Genres::load(dir),
            tags: Tags::load(dir),
            studios,
            studio_table,
        }
    }
}

// ===== how it breaks =====================================================
//
// 1. MOVE OUT OF BORROW. Every `QueryExt` combinator takes `self` by value,
//    and `VecRel` is not `Copy` (it owns a `Vec`). From a `&Db` you cannot
//    move a field out, so every column reference needs a manual `&`:
//
//        db.films.year.lt(1990)      // error[E0507]: cannot move out of
//                                    //   `db.films.year` which is behind
//                                    //   a shared reference
//        (&db.films.year).lt(1990)   // ok — `&VecRel` is Copy and is a Query
//
//    The token design never hits this: `Film::year` already hands back a
//    `Copy` `&'static VecRel`. Here the `&` is on the programmer, forever.
//
// 2. THE ID→ROW COLUMN HAS NO HOME. Every entity conceptually has an ID→Row
//    column; for a dense entity it is the identity and vanishes. A bare field
//    cannot carry it, so the dense case silently has nothing while the dict
//    case needs a hand-spliced `DictTable` — the two are spelled differently
//    at every call site. Concretely, the dict FK's range is `Key<Studios>`
//    while `studios.name`'s domain is `Id<Studios>`, so they do not compose:
//
//        (&db.films.studio).select(&db.studios.name)   // error[E0271]:
//                                    //   type mismatch resolving
//                                    //   `<&VecRel<Str, Id<Studios>> as Query>::D
//                                    //    == Key<Studios>`
//        (&db.films.studio).select(&db.studio_table).select(&db.studios.name)  // ok
//
// 3. BORROWED, NOT 'static. A plan built from `&db` borrows `db`, so its type
//    carries that lifetime. It cannot outlive the borrow, be returned from a
//    function that owns the `Db`, or be stored in a `static`. The token
//    design's plans hold `&'static` and are lifetime-free.
//
// 4. INSTANCE PLUMBING. `Films::all` is `&self`, and every query site needs
//    `&db` threaded in. There is no way to write a free-standing query
//    constant or a `fn q17() -> String` that reads globals, as the existing
//    benchmark runners do — they would all grow a `db: &Db` parameter.
//
// 5. NO MANIFEST, NO SPEC. Nothing enumerates the columns, so regen has no
//    generated `(entity, field, kind)` list to verify its cache against, and
//    a typo'd filename in `load` is a RUNTIME panic, not a compile error.
//
// 6. EAGER. `Db::load` reads every column of every entity whether or not a
//    query touches it. (Softenable with `OnceCell` fields, at the cost of the
//    struct literal's simplicity — and of `&` ergonomics getting worse still.)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::*;
    use crate::format::{HEADER_LEN, align8, header};
    use crate::format::{KIND_CSR_WORDS, KIND_DENSE_F64, KIND_DENSE_I64, KIND_DENSE_STR};
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    // ----- the same synthetic v2 cache the token sandbox uses -------------

    fn write_v2(dir: &PathBuf, name: &str, head: [u8; HEADER_LEN], payload: &[u8]) {
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
        (
            header(KIND_DENSE_STR, vals.len() as u64, off as u64),
            payload,
        )
    }

    fn dense_words(vals: &[u64]) -> ([u8; HEADER_LEN], Vec<u8>) {
        let mut payload = Vec::new();
        for v in vals {
            payload.extend_from_slice(&v.to_le_bytes());
        }
        (header(KIND_DENSE_I64, vals.len() as u64, 0), payload)
    }

    fn dense_f64(vals: &[f64]) -> ([u8; HEADER_LEN], Vec<u8>) {
        let mut payload = Vec::new();
        for v in vals {
            payload.extend_from_slice(&v.to_le_bytes());
        }
        (header(KIND_DENSE_F64, vals.len() as u64, 0), payload)
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

    /// Built ONCE: both tests share the dir, and rewriting the files while a
    /// sibling test has them mmap'd is a torn read.
    fn build_cache() -> &'static PathBuf {
        static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        DIR.get_or_init(build_cache_once)
    }

    fn build_cache_once() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("prela_structs_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let (h, p) = dense_str(&["Alien", "Blade", "Solaris"]);
        write_v2(&dir, "Film_title", h, &p);
        let (h, p) = dense_words(&[1979, 1998, 1972]);
        write_v2(&dir, "Film_year", h, &p);
        let (h, p) = dense_f64(&[8.5, 7.1, 8.1]);
        write_v2(&dir, "Film_rating", h, &p);
        let (h, p) = dense_words(&[1, 0, 1]);
        write_v2(&dir, "Film_genre", h, &p);
        let (h, p) = csr_words(&[0, 1, 3, 3], &[0, 0, 1]);
        write_v2(&dir, "Film_tags", h, &p);
        let (h, p) = dense_words(&[205, 100, 205]);
        write_v2(&dir, "Film_studio", h, &p);

        let (h, p) = dense_str(&["action", "horror"]);
        write_v2(&dir, "Genre_name", h, &p);
        let (h, p) = dense_str(&["main", "sub"]);
        write_v2(&dir, "Genre_kind", h, &p);

        let (h, p) = dense_str(&["cult", "noir"]);
        write_v2(&dir, "Tag_text", h, &p);
        let (h, p) = csr_words(&[0, 2, 3], &[0, 1, 1]);
        write_v2(&dir, "Tag_films", h, &p);

        let (h, p) = dense_words(&[100, 205, 9899]);
        write_v2(&dir, "Studio_id", h, &p);
        let (h, p) = dense_str(&["Warner", "A24", "Mubi"]);
        write_v2(&dir, "Studio_name", h, &p);

        dir
    }

    fn collect<Q: IntoQuery>(q: Q) -> Vec<<Q::Q as Query>::R>
    where
        Q::Q: Drive,
    {
        let mut out = Vec::new();
        q.iq().drive(|_, r| out.push(r));
        out
    }

    #[test]
    fn structs_sandbox() {
        let db = Db::load(&build_cache());

        // universe — a method on the instance, not an associated fn
        assert_eq!(db.films.all().n, 3);
        assert_eq!(db.genres.all().n, 2);

        // scalar columns. note the `&`: without it this is E0507.
        assert_eq!(collect(&db.films.title), vec!["Alien", "Blade", "Solaris"]);
        assert_eq!(collect(&db.films.rating), vec![8.5, 7.1, 8.1]);

        // filter + restrict. `&` on every column reference.
        assert_eq!(
            collect(
                db.films
                    .all()
                    .with((&db.films.year).lt(1990))
                    .select(&db.films.title)
            ),
            vec!["Alien", "Solaris"]
        );

        // DENSE FK nav works exactly as well as the token design: the field's
        // range is already `Id<Genres>`, and the token's `Ident` hop was the
        // identity anyway. Nothing is lost here.
        assert_eq!(
            collect((&db.films.genre).select(&db.genres.name)),
            vec!["horror", "action", "horror"]
        );
        assert_eq!(
            collect(
                db.films
                    .all() // films is a struct of cols, not a col, so can't directly call
                    // combinators (doesn't impl Query, etc.)
                    .with((&db.films.genre).select(&db.genres.name).eq("horror"))
                    .select(&db.films.title)
            ),
            vec!["Alien", "Solaris"]
        );

        // Multi columns, both directions — also fine.
        assert_eq!(
            collect((&db.films.tags).select(&db.tags.text)),
            vec!["cult", "cult", "noir"]
        );
        assert_eq!(
            collect((&db.tags.films).select(&db.films.year)),
            vec![1979, 1998, 1998]
        );

        // DICT FK: the hop must be spliced in BY HAND. This is breakage #2 —
        // the extra `.select(&db.studio_table)` is what the token hid.
        assert_eq!(
            collect(
                (&db.films.studio)
                    .select(&db.studio_table)
                    .select(&db.studios.name)
            ),
            vec!["A24", "Warner", "A24"]
        );

        // the column really does store un-followed external keys
        assert_eq!(
            collect(&db.films.studio),
            vec![Key::<Studios>::new(205), Key::new(100), Key::new(205)]
        );

        // The dict table maps ID → ROW, not row → ID. Pinned explicitly,
        // since the test data is asymmetric enough to catch an inversion:
        // Studio_id = [100, 205, 9899], so external key 205 is ROW 1.
        // Inverted, 205 would land on row 2 ("Mubi") or miss entirely.
        let rows = collect((&db.films.studio).select(&db.studio_table));
        assert_eq!(
            rows,
            vec![Id::<Studios>::new(1), Id::new(0), Id::new(1)],
            "DictTable must map external id → row"
        );

        // cross-entity mistakes ARE still compile errors — the phantom on
        // `Id<E>` does that work, and fields carry it as well as tokens do:
        //   (&db.films.genre).select(&db.tags.text)  // expected Id<Genres>,
        //                                            //   found Id<Tags>
    }

    /// Breakage #3, as a real signature: a plan built from a `&Db` borrows it.
    /// This compiles only because the lifetime is threaded explicitly; the
    /// token design needs no lifetime at all.
    fn horror_titles(db: &Db) -> Vec<Str> {
        collect(
            db.films
                .all()
                .with((&db.films.genre).select(&db.genres.name).eq("horror"))
                .select(&db.films.title),
        )
    }

    #[test]
    fn plans_are_borrowed_not_static() {
        let db = Db::load(&build_cache());
        assert_eq!(horror_titles(&db), vec!["Alien", "Solaris"]);

        // BREAKS: a plan cannot outlive the Db it borrows, so this signature
        // is impossible — there is no way to build the query and hand it back
        // while the `Db` is owned locally:
        //
        //     fn make_plan() -> impl Drive {
        //         let db = Db::load(&build_cache());
        //         db.films.all().select(&db.films.title)
        //     }
        //     // error[E0597]: `db.films.title` does not live long enough
        //     //   (verified: the borrow is dropped at end of fn, but the
        //     //    returned `impl Drive` still holds it)
        //
        // The token design's equivalent is just `Film::all().select(Film::title)`,
        // which is `'static` and returns fine.
    }
}
