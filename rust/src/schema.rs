// `schema!` — declarative typed schemas over the v2 binary cache.
//
// One declaration generates, per entity:
//   - a unit-struct entity tag (`pub struct Movie;`) — the phantom `E` of
//     `Id<E>`, so cross-entity compositions are COMPILE errors;
//   - a cols struct (same ident, inside the schema's module) holding the
//     entity's typed columns, loaded by the generated `init(cache_dir)`
//     from `<Entity>_<field>.bin` via the src/cache.rs v2 readers
//     (id-valued columns are bulk-reinterpreted to `Id<T>` through the
//     `repr(transparent)` layout — no per-element conversion);
//   - entity-qualified accessors for EVERY field
//     (`Info::info()`, `Info::ty()` → `&'static VecRel<…, Id<Info>>`);
//   - a BARE accessor fn (`production_year()`) for fields marked `pub` —
//     explicit, like Julia's selective `@expose`, because macro_rules
//     cannot detect cross-entity name collisions;
//   - a bare universe accessor (`pub fn movie() -> Universe<Id<Movie>>`)
//     when the entity is declared `Movie(movies)` — explicit because
//     macro_rules cannot lowercase idents. Universe size = the entity's
//     first column's key count;
//   - a NAVIGATION extension trait (`Movie(movies) / MovieNav` — the name
//     is explicit because macro_rules cannot concatenate idents) with one
//     method per field, blanket-implemented for every query whose value
//     type is the entity's id:
//       trait MovieNav: Query<R = Id<Movie>> + Sized {
//           fn title(self) -> Compose<Self, &'static …> { … }  // per field
//       }
//       impl<Q: Query<R = Id<Movie>> + Sized> MovieNav for Q {}
//     so `cast().person().name()` spells the compose chain
//     `cast().get(Cast::person()).get(Person::name())`. Coherence is safe:
//     same-named methods on different entities' nav traits have disjoint
//     receivers (a query's `R` equals exactly ONE `Id<E>`), so method
//     resolution always finds a single applicable trait. Predicate ROOTS
//     stay accessor calls (bare or `Entity::field()`); everything after
//     the root navigates.
//
// Field types: `str` → `VecRel<&'static str, Id<E>>`; `i64`/`f64` →
// `VecRel<i64/f64, Id<E>>`; a bare entity ident `Kind` →
// `VecRel<Id<Kind>, Id<E>>`; `Multi<T>` → `MultiRel<…, Id<E>>` likewise.
// The literal idents `str|i64|f64` are matched before the entity-ident
// case, so an entity cannot be named `str`/`i64`/`f64`.
//
// Filenames are `concat!(stringify!(Entity), "_", stringify!(field))`,
// verbatim — field names are filenames (no raw-keyword underscore
// trimming; spell keyword-ish fields differently, e.g. `ty` not `type`).
//
// The declaration is also regen's source of truth for WHAT the cache must
// contain: the macro emits a `pub const MANIFEST: &[(&str, &str, u32)]`
// of (entity, field, cache kind — src/format.rs `KIND_*`). regen's
// parquet→cache TRANSFORMATION logic (FK joins, multi-table splits) stays
// hand-written — it is parquet-specific — but after writing, regen checks
// its output file set and header kinds against the manifest, so a column
// regen produces that the schema doesn't declare (or vice versa, or with
// the wrong physical kind) fails the regen run loudly.

macro_rules! schema {
    // ===== entry: SCHEMA_MOD / StorageStruct / init_fn : entities... =====
    ( $mod_:ident / $store:ident / $init:ident :
      $( $Ent:ident $( ( $uni:ident ) )? / $Nav:ident { $($body:tt)* } )* ) => {

        $(
            #[allow(dead_code)]
            pub struct $Ent;
        )*

        /// Generated storage: one cols struct per entity, filled by `init`.
        #[allow(non_snake_case, dead_code)]
        pub struct $store {
            $( pub $Ent: $mod_::$Ent, )*
        }

        #[allow(non_snake_case, dead_code)]
        pub mod $mod_ {
            pub static STORE: ::std::sync::OnceLock<super::$store> =
                ::std::sync::OnceLock::new();
            $( $crate::schema::schema!(@colstruct $Ent; $($body)*); )*
        }

        /// Load every column from `<cache_dir>/<Entity>_<field>.bin`.
        #[allow(dead_code)]
        pub fn $init(cache_dir: &::std::path::Path) {
            let loaded = $store {
                $( $Ent: $crate::schema::schema!(@initent cache_dir; $mod_; $Ent; $($body)*), )*
            };
            if $mod_::STORE.set(loaded).is_err() {
                panic!(concat!(stringify!($init), ": schema already initialized"));
            }
        }

        $( $crate::schema::schema!(@uni $mod_; $Ent; [$($uni)?]; $($body)*); )*
        $( $crate::schema::schema!(@fns $mod_; $Ent; $($body)*); )*
        $( $crate::schema::schema!(@nav $Ent; $Nav; [] $($body)*); )*

        $crate::schema::schema!(@manifest [] $( $Ent { $($body)* } )*);
    };

    // ===== manifest: (entity, field, cache kind) for every column ========
    // Accumulator muncher over entities × fields. Field names are used
    // verbatim — `<Entity>_<field>` IS the cache filename.
    (@manifest [$($acc:tt)*]) => {
        /// Generated (entity, field, `format::KIND_*`) manifest — the file
        /// list + physical kinds this schema loads, consumed by `regen` to
        /// verify the cache it writes.
        #[allow(dead_code)]
        pub const MANIFEST: &[(&str, &str, u32)] = &[$($acc)*];
    };
    (@manifest [$($acc:tt)*] $Ent:ident { $($body:tt)* } $($rest:tt)*) => {
        $crate::schema::schema!(@manifest_fields $Ent [$($acc)*] { $($body)* } $($rest)*);
    };
    (@manifest_fields $Ent:ident [$($acc:tt)*] { } $($rest:tt)*) => {
        $crate::schema::schema!(@manifest [$($acc)*] $($rest)*);
    };
    (@manifest_fields $Ent:ident [$($acc:tt)*] { pub $($body:tt)* } $($rest:tt)*) => {
        $crate::schema::schema!(@manifest_fields $Ent [$($acc)*] { $($body)* } $($rest)*);
    };
    (@manifest_fields $Ent:ident [$($acc:tt)*]
      { $f:ident : $t1:tt $(< $t2:tt >)? $(, $($body:tt)*)? } $($rest:tt)*) => {
        $crate::schema::schema!(@manifest_fields $Ent
            [$($acc)* (stringify!($Ent), stringify!($f),
                       $crate::schema::schema!(@kind $t1 $(($t2))?)),]
            { $($($body)*)? } $($rest)*);
    };

    // ===== field type → cache kind ========================================
    (@kind str) => { $crate::format::KIND_DENSE_STR };
    (@kind i64) => { $crate::format::KIND_DENSE_I64 };
    (@kind f64) => { $crate::format::KIND_DENSE_F64 };
    (@kind Multi (str)) => { $crate::format::KIND_CSR_STR };
    (@kind Multi ($T:ident)) => { $crate::format::KIND_CSR_WORDS };
    (@kind $T:ident) => { $crate::format::KIND_DENSE_I64 };

    // ===== cols struct (inside the schema module; entity tags are super::*)
    // Accumulator muncher: `$f:ident` would ambiguously match a leading
    // `pub` (the ident fragment matches keywords), so `pub` is stripped by
    // its own rule before the field rule runs.
    (@colstruct $Ent:ident; $($body:tt)*) => {
        $crate::schema::schema!(@colstruct_acc $Ent [] $($body)*);
    };
    (@colstruct_acc $Ent:ident [$($acc:tt)*]) => {
        pub struct $Ent { $($acc)* }
    };
    (@colstruct_acc $Ent:ident [$($acc:tt)*] pub $($rest:tt)*) => {
        $crate::schema::schema!(@colstruct_acc $Ent [$($acc)*] $($rest)*);
    };
    (@colstruct_acc $Ent:ident [$($acc:tt)*]
      $f:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        $crate::schema::schema!(@colstruct_acc $Ent
            [$($acc)* pub $f: $crate::schema::schema!(@colty (super); $Ent; $t1 $(($t2))?),]
            $($($rest)*)?);
    };

    // ===== field type → physical column type ============================
    (@colty ($($p:ident)?); $E:ident; str) =>
        { $crate::engine::VecRel<&'static str, $crate::engine::Id<$($p::)? $E>> };
    (@colty ($($p:ident)?); $E:ident; i64) =>
        { $crate::engine::VecRel<i64, $crate::engine::Id<$($p::)? $E>> };
    (@colty ($($p:ident)?); $E:ident; f64) =>
        { $crate::engine::VecRel<f64, $crate::engine::Id<$($p::)? $E>> };
    (@colty ($($p:ident)?); $E:ident; Multi (str)) =>
        { $crate::engine::MultiRel<&'static str, $crate::engine::Id<$($p::)? $E>> };
    (@colty ($($p:ident)?); $E:ident; Multi (i64)) =>
        { $crate::engine::MultiRel<i64, $crate::engine::Id<$($p::)? $E>> };
    (@colty ($($p:ident)?); $E:ident; Multi ($T:ident)) =>
        { $crate::engine::MultiRel<$crate::engine::Id<$($p::)? $T>, $crate::engine::Id<$($p::)? $E>> };
    (@colty ($($p:ident)?); $E:ident; $T:ident) =>
        { $crate::engine::VecRel<$crate::engine::Id<$($p::)? $T>, $crate::engine::Id<$($p::)? $E>> };

    // ===== per-entity struct literal for init ============================
    (@initent $dir:ident; $mod_:ident; $Ent:ident; $($body:tt)*) => {
        $crate::schema::schema!(@initent_acc $dir; $mod_; $Ent; [] $($body)*)
    };
    (@initent_acc $dir:ident; $mod_:ident; $Ent:ident; [$($acc:tt)*]) => {
        $mod_::$Ent { $($acc)* }
    };
    (@initent_acc $dir:ident; $mod_:ident; $Ent:ident; [$($acc:tt)*] pub $($rest:tt)*) => {
        $crate::schema::schema!(@initent_acc $dir; $mod_; $Ent; [$($acc)*] $($rest)*)
    };
    (@initent_acc $dir:ident; $mod_:ident; $Ent:ident; [$($acc:tt)*]
      $f:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        $crate::schema::schema!(@initent_acc $dir; $mod_; $Ent;
            [$($acc)* $f: $crate::schema::schema!(@load $dir;
                concat!(stringify!($Ent), "_", stringify!($f));
                $t1 $(($t2))?),]
            $($($rest)*)?)
    };

    // ===== field type → cache reader =====================================
    (@load $dir:ident; $name:expr; str) => { $crate::cache::load_strs_in($dir, $name) };
    (@load $dir:ident; $name:expr; i64) => { $crate::cache::load_i64_in($dir, $name) };
    (@load $dir:ident; $name:expr; f64) => { $crate::cache::load_f64_in($dir, $name) };
    (@load $dir:ident; $name:expr; Multi (str)) => { $crate::cache::load_multi_strs_in($dir, $name) };
    (@load $dir:ident; $name:expr; Multi (i64)) => { $crate::cache::load_multi_i64_in($dir, $name) };
    (@load $dir:ident; $name:expr; Multi ($T:ident)) => { $crate::cache::load_multi_ids_in($dir, $name) };
    (@load $dir:ident; $name:expr; $T:ident) => { $crate::cache::load_ids_in($dir, $name) };

    // ===== universe accessor, sized by the FIRST declared field ==========
    (@uni $mod_:ident; $Ent:ident; []; $($rest:tt)*) => {};
    (@uni $mod_:ident; $Ent:ident; [$uni:ident]; pub $($rest:tt)*) => {
        $crate::schema::schema!(@uni $mod_; $Ent; [$uni]; $($rest)*);
    };
    (@uni $mod_:ident; $Ent:ident; [$uni:ident];
      $ff:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        /// Generated universe accessor — identity relation over the
        /// entity's dense id space (size = first column's key count).
        #[allow(dead_code)]
        #[inline]
        pub fn $uni() -> $crate::engine::Universe<$crate::engine::Id<$Ent>> {
            $crate::engine::Universe::new(
                $mod_::STORE.get().expect("schema not initialized").$Ent.$ff.n_keys())
        }
    };

    // ===== navigation trait: one compose method per field ================
    // Accumulator muncher (like @colstruct): trait items can't be emitted
    // incrementally into an open `trait { … }`, so the methods accumulate
    // as tts and the trait + blanket impl are emitted at the end.
    (@nav $Ent:ident; $Nav:ident; [$($acc:tt)*]) => {
        /// Generated navigation trait — for any query valued in this
        /// entity's ids, one method per field composing with that field's
        /// column (`q.title()` ≡ `q.get(Movie::title())`). Blanket-implemented;
        /// same-named methods on other entities' nav traits don't clash
        /// because the receivers' `R = Id<E>` bounds are disjoint.
        #[allow(dead_code)]
        pub trait $Nav:
            $crate::engine::Query<R = $crate::engine::Id<$Ent>> + Sized
        {
            $($acc)*
        }
        impl<Q: $crate::engine::Query<R = $crate::engine::Id<$Ent>> + Sized> $Nav for Q {}
    };
    (@nav $Ent:ident; $Nav:ident; [$($acc:tt)*] pub $($rest:tt)*) => {
        $crate::schema::schema!(@nav $Ent; $Nav; [$($acc)*] $($rest)*);
    };
    (@nav $Ent:ident; $Nav:ident; [$($acc:tt)*]
      $f:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        $crate::schema::schema!(@nav $Ent; $Nav;
            [$($acc)*
             #[allow(dead_code)]
             #[inline]
             fn $f(self) -> $crate::engine::Compose<
                 Self, &'static $crate::schema::schema!(@colty (); $Ent; $t1 $(($t2))?)>
             {
                 $crate::engine::Compose { a: self, b: <$Ent>::$f() }
             }]
            $($($rest)*)?);
    };

    // ===== accessors: qualified for every field, bare for `pub` fields ===
    (@fns $mod_:ident; $Ent:ident; ) => {};
    (@fns $mod_:ident; $Ent:ident;
      pub $f:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        impl $Ent {
            #[allow(dead_code)]
            #[inline]
            pub fn $f() -> &'static $crate::schema::schema!(@colty (); $Ent; $t1 $(($t2))?) {
                &$mod_::STORE.get().expect("schema not initialized").$Ent.$f
            }
        }
        #[allow(dead_code)]
        #[inline]
        pub fn $f() -> &'static $crate::schema::schema!(@colty (); $Ent; $t1 $(($t2))?) {
            <$Ent>::$f()
        }
        $crate::schema::schema!(@fns $mod_; $Ent; $($($rest)*)?);
    };
    (@fns $mod_:ident; $Ent:ident;
      $f:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        impl $Ent {
            #[allow(dead_code)]
            #[inline]
            pub fn $f() -> &'static $crate::schema::schema!(@colty (); $Ent; $t1 $(($t2))?) {
                &$mod_::STORE.get().expect("schema not initialized").$Ent.$f
            }
        }
        $crate::schema::schema!(@fns $mod_; $Ent; $($($rest)*)?);
    };
}

pub(crate) use schema;

// ===== tests — a tiny schema over a generated v2 cache dir ===============

#[cfg(test)]
mod tests {
    use crate::engine::*;
    use crate::format::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    // Three entities exercising every field-type arm: dense str, dense
    // i64, FK (entity ident), Multi<entity>, Multi<str>.
    schema! { TESTS / TestSchema / test_init:
        Film(film) / FilmNav { pub ftitle: str, pub year: i64, genre: Genre, tags: Multi<Tag> }
        Genre / GenreNav { gname: str, ty: str }
        Tag / TagNav { tag: str, films: Multi<Film> }
    }

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
        (header(KIND_CSR_WORDS, (offsets.len() - 1) as u64, vals.len() as u64), payload)
    }

    #[test]
    fn schema_macro_loads_types_and_navigates() {
        let dir = std::env::temp_dir()
            .join(format!("prela_schema_test_{}", std::process::id()));
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

        // universe size = first column's key count
        assert_eq!(film().n, 2);

        // typed composition across three entities, in navigation form:
        // a predicate ROOT is an accessor (qualified `Film::genre()`, bare
        // `year()` for pub fields); every later hop is a nav method
        // (`.gname()` ≡ `.get(Genre::gname())` via the generated GenreNav).
        let q = film()
            .when(Film::genre().gname().eq("horror"))
            .when(year().lt(1990))
            .ftitle();
        let mut got = Vec::new();
        q.drive(|_, t| got.push(t));
        assert_eq!(got, vec!["Alien"]);

        // Multi<entity> column + nav through Tag's tag column
        let q = film().when(Film::tags().tag().eq("noir")).ftitle();
        let mut got = Vec::new();
        q.drive(|_, t| got.push(t));
        assert_eq!(got, vec!["Blade"]);

        // same-named nav methods on different entities resolve by the
        // receiver's value type: Tag::films() is Film-valued, so `.year()`
        // picks FilmNav; the chain then navigates Film → Genre → gname.
        let mut got = Vec::new();
        Tag::films().year().probe(Id::new(0), |y| got.push(y));
        assert_eq!(got, vec![1979, 1998]);
        let mut got = Vec::new();
        Tag::films().genre().gname().probe(Id::new(1), |g| got.push(g));
        assert_eq!(got, vec!["drama"]);

        // field names are filenames verbatim (`ty` → Genre_ty.bin)
        let mut got = Vec::new();
        Genre::ty().probe(Id::new(0), |v| got.push(v));
        assert_eq!(got, vec!["main"]);

        // typed ids round-trip the bulk reinterpret: Film_genre words → Id<Genre>
        let mut got = Vec::new();
        Film::genre().probe(Id::<Film>::new(1), |g| got.push(g));
        assert_eq!(got, vec![Id::<Genre>::new(0)]);

        // the generated manifest names every column with its cache kind
        assert_eq!(MANIFEST, &[
            ("Film", "ftitle", KIND_DENSE_STR),
            ("Film", "year", KIND_DENSE_I64),
            ("Film", "genre", KIND_DENSE_I64),
            ("Film", "tags", KIND_CSR_WORDS),
            ("Genre", "gname", KIND_DENSE_STR),
            ("Genre", "ty", KIND_DENSE_STR),
            ("Tag", "tag", KIND_DENSE_STR),
            ("Tag", "films", KIND_CSR_WORDS),
        ]);
    }
}
