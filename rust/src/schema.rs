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
//   - a paren-free leaf HANDLE per field: a ZST named by the field, living
//     in `<mod>::<Nav>` (the nav-trait ident doubles as the per-entity
//     handle namespace — macro_rules cannot concatenate idents), that
//     implements `engine::IntoQuery<Q = &'static VecRel/MultiRel<…>>`. Its
//     `iq` fetches the column from the OnceLock store ONCE, at plan
//     construction — the built plan contains only the `&'static` leaf, so
//     hot loops are identical to hand-built plans;
//   - the QUALIFIED spelling for EVERY field as an associated const on the
//     entity tag (`impl Info { pub const ty: TPCH-internal handle type }`),
//     so `Info::ty` is a value usable wherever a relation is expected;
//   - a BARE re-export of the handle (`pub use …::production_year;`) for
//     fields marked `pub` — explicit, like Julia's selective `@expose`,
//     because macro_rules cannot detect cross-entity name collisions.
//     CAVEAT (unit structs in patterns): a bare handle in scope captures
//     any same-named BINDING pattern — `let kind = …`, a closure param
//     `|part, _|`, a match arm — which then fails to compile ("interpreted
//     as a unit struct, not a new binding"). Rename such locals. This also
//     means `assert_eq!`/`assert_ne!` break in any module that glob-imports
//     a schema exporting bare `kind` (the core macros internally bind
//     `let kind`): test modules import schema names selectively;
//   - a bare universe HANDLE (`pub struct movie;` with
//     `IntoQuery<Q = Universe<Id<Movie>>>`) when the entity is declared
//     `Movie(movie)` — explicit because macro_rules cannot lowercase
//     idents. Universe size = the entity's first column's key count,
//     resolved at `iq` time;
//   - a NAVIGATION extension trait (`Movie(movie) / MovieNav`) with one
//     method per field, blanket-implemented for everything that RESOLVES
//     (via `IntoQuery`) to a query whose value type is the entity's id:
//       trait MovieNav: IntoQuery + Sized
//       where Self::Q: Query<R = Id<Movie>> {
//           fn title(self) -> Compose<Self::Q, &'static …> { … } // per field
//       }
//     so `cast.person().name()` spells the compose chain
//     `cast.select(Cast::person).select(Person::name)` — the leaf handle roots
//     the chain paren-free, and every later hop is a nav method. Coherence
//     is safe: same-named methods on different entities' nav traits have
//     disjoint receivers (a resolved query's `R` equals exactly ONE
//     `Id<E>`), so method resolution always finds a single applicable
//     trait. Predicate ROOTS are bare handles (`keyword`) or qualified
//     consts (`Entity::field`); everything after the root navigates.
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
            $( $crate::schema::schema!(@handlemod $Ent; $Nav; $($body)*); )*
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
        $( $crate::schema::schema!(@consts $mod_; $Ent; $Nav; $($body)*); )*
        $( $crate::schema::schema!(@nav $mod_; $Ent; $Nav; [] $($body)*); )*

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
    // The parenthesized prefix is a `::`-joined path back to the scope
    // holding the entity tags: `()` at the invocation scope, `(super)` from
    // inside the schema module, `(super super)` from a handle module.
    (@colty ($($p:ident)*); $E:ident; str) =>
        { $crate::engine::VecRel<&'static str, $crate::engine::Id<$($p::)* $E>> };
    (@colty ($($p:ident)*); $E:ident; i64) =>
        { $crate::engine::VecRel<i64, $crate::engine::Id<$($p::)* $E>> };
    (@colty ($($p:ident)*); $E:ident; f64) =>
        { $crate::engine::VecRel<f64, $crate::engine::Id<$($p::)* $E>> };
    (@colty ($($p:ident)*); $E:ident; Multi (str)) =>
        { $crate::engine::MultiRel<&'static str, $crate::engine::Id<$($p::)* $E>> };
    (@colty ($($p:ident)*); $E:ident; Multi (i64)) =>
        { $crate::engine::MultiRel<i64, $crate::engine::Id<$($p::)* $E>> };
    (@colty ($($p:ident)*); $E:ident; Multi ($T:ident)) =>
        { $crate::engine::MultiRel<$crate::engine::Id<$($p::)* $T>, $crate::engine::Id<$($p::)* $E>> };
    (@colty ($($p:ident)*); $E:ident; $T:ident) =>
        { $crate::engine::VecRel<$crate::engine::Id<$($p::)* $T>, $crate::engine::Id<$($p::)* $E>> };

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

    // ===== universe handle, sized by the FIRST declared field ============
    (@uni $mod_:ident; $Ent:ident; []; $($rest:tt)*) => {};
    (@uni $mod_:ident; $Ent:ident; [$uni:ident]; pub $($rest:tt)*) => {
        $crate::schema::schema!(@uni $mod_; $Ent; [$uni]; $($rest)*);
    };
    (@uni $mod_:ident; $Ent:ident; [$uni:ident];
      $ff:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        /// Generated universe HANDLE — resolves (via `IntoQuery`) to the
        /// identity relation over the entity's dense id space (size = first
        /// column's key count, read at plan-construction time).
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Clone, Copy)]
        pub struct $uni;
        impl $crate::engine::IntoQuery for $uni {
            type Q = $crate::engine::Universe<$crate::engine::Id<$Ent>>;
            #[inline]
            fn iq(self) -> Self::Q {
                $crate::engine::Universe::new(
                    $mod_::STORE.get().expect("schema not initialized").$Ent.$ff.n_keys())
            }
        }
    };

    // ===== navigation trait: one compose method per field ================
    // Accumulator muncher (like @colstruct): trait items can't be emitted
    // incrementally into an open `trait { … }`, so the methods accumulate
    // as tts and the trait + blanket impl are emitted at the end.
    (@nav $mod_:ident; $Ent:ident; $Nav:ident; [$($acc:tt)*]) => {
        /// Generated navigation trait — for anything resolving (via
        /// `IntoQuery`) to a query valued in this entity's ids, one method
        /// per field composing with that field's column (`q.title()` ≡
        /// `q.select(Movie::title)`). Blanket-implemented; same-named methods
        /// on other entities' nav traits don't clash because the resolved
        /// receivers' `R = Id<E>` bounds are disjoint.
        #[allow(dead_code)]
        pub trait $Nav: $crate::engine::IntoQuery + Sized
        where Self::Q: $crate::engine::Query<R = $crate::engine::Id<$Ent>>
        {
            $($acc)*
        }
        impl<T: $crate::engine::IntoQuery + Sized> $Nav for T
        where T::Q: $crate::engine::Query<R = $crate::engine::Id<$Ent>> {}
    };
    (@nav $mod_:ident; $Ent:ident; $Nav:ident; [$($acc:tt)*] pub $($rest:tt)*) => {
        $crate::schema::schema!(@nav $mod_; $Ent; $Nav; [$($acc)*] $($rest)*);
    };
    (@nav $mod_:ident; $Ent:ident; $Nav:ident; [$($acc:tt)*]
      $f:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        $crate::schema::schema!(@nav $mod_; $Ent; $Nav;
            [$($acc)*
             #[allow(dead_code)]
             #[inline]
             fn $f(self) -> $crate::engine::Compose<
                 Self::Q, &'static $crate::schema::schema!(@colty (); $Ent; $t1 $(($t2))?)>
             {
                 $crate::engine::Compose {
                     a: self.iq(),
                     b: &$mod_::STORE.get().expect("schema not initialized").$Ent.$f,
                 }
             }]
            $($($rest)*)?);
    };

    // ===== leaf handles: one paren-free ZST per field =====================
    // The nav-trait ident doubles as the per-entity handle namespace (a
    // module inside the schema module), because macro_rules cannot mint
    // fresh idents: `Movie.title`'s handle type is `<mod>::MovieNav::title`.
    // The PUBLIC spellings — `Movie::title` (assoc const, every field) and
    // bare `title` (re-export, `pub` fields) — are generated by @consts.
    (@handlemod $Ent:ident; $Nav:ident; $($body:tt)*) => {
        /// Generated per-field leaf handles — ZSTs resolving (via
        /// `IntoQuery::iq`, one OnceLock fetch at plan construction) to the
        /// `&'static` column relation. Internal: spell them `Entity::field`
        /// or (for `pub` fields) bare `field`.
        #[allow(non_camel_case_types, dead_code)]
        pub mod $Nav {
            $crate::schema::schema!(@handle_acc $Ent; $($body)*);
        }
    };
    (@handle_acc $Ent:ident; ) => {};
    (@handle_acc $Ent:ident; pub $($rest:tt)*) => {
        $crate::schema::schema!(@handle_acc $Ent; $($rest)*);
    };
    (@handle_acc $Ent:ident;
      $f:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        #[derive(Clone, Copy)]
        pub struct $f;
        impl $crate::engine::IntoQuery for $f {
            type Q = &'static $crate::schema::schema!(@colty (super super); $Ent; $t1 $(($t2))?);
            #[inline]
            fn iq(self) -> Self::Q {
                &super::STORE.get().expect("schema not initialized").$Ent.$f
            }
        }
        $crate::schema::schema!(@handle_acc $Ent; $($($rest)*)?);
    };

    // ===== public handle spellings: qualified const for every field, =====
    // ===== bare re-export for `pub` fields ================================
    (@consts $mod_:ident; $Ent:ident; $Nav:ident; ) => {};
    (@consts $mod_:ident; $Ent:ident; $Nav:ident;
      pub $f:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        impl $Ent {
            #[allow(non_upper_case_globals, dead_code)]
            pub const $f: $mod_::$Nav::$f = $mod_::$Nav::$f;
        }
        #[allow(unused_imports)]
        pub use $mod_::$Nav::$f;
        $crate::schema::schema!(@consts $mod_; $Ent; $Nav; $($($rest)*)?);
    };
    (@consts $mod_:ident; $Ent:ident; $Nav:ident;
      $f:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        impl $Ent {
            #[allow(non_upper_case_globals, dead_code)]
            pub const $f: $mod_::$Nav::$f = $mod_::$Nav::$f;
        }
        $crate::schema::schema!(@consts $mod_; $Ent; $Nav; $($($rest)*)?);
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
        Tag::films.genre().gname().probe(Id::new(1), |g| got.push(g));
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
