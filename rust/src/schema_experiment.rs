// `cols!` — the macro-minimal schema surface (experiment).
//
// The old `schema!` (src/schema.rs, ~470 macro lines) generates, per field,
// a fresh ZST handle type, a typed cols-struct slot, a loader call, a nav
// method and a manifest entry. This module replaces all of that with ONE
// generic handle type plus two traits, so a schema becomes plain Rust:
//
//     pub struct Customer;
//     #[allow(non_upper_case_globals)]
//     impl Customer {
//         pub const name:   Col<Self, Str>        = Col::new("name");
//         pub const nation: Col<Self, Id<Nation>> = Col::new("nation");
//     }
//     cols!(Customer: name, nation);
//
// The only thing Rust cannot do for us is REFLECT over an impl block's
// consts, so `cols!` exists purely to enumerate them into `Entity::COLUMNS`.
// It never sees a type — that is `ColType`'s job — so it structurally cannot
// grow back into `schema!`.
//
// Surface: qualified handles only (`Customer::name`). No nav methods, no
// bare handles; chain with `.select(..)` (`Customer::nation.select(Nation::name)`).
//
// Layering:
//   - `ColType` maps a DECLARED column type (`i64`, `f64`, `Str`,
//     `Id<E>`, `Multi<..>`) to its physical storage, its cache kind, its
//     loader, and the query it resolves to. This one trait absorbs the old
//     macro's `@colty` / `@kind` / `@load` arms AND the FK table hop — the
//     `Id<T>` impl composes `T::table()`, which is `Ident` (inlines away) for
//     a dense entity and a `DictTable` probe for a `dict` one, so both are
//     covered with no extra declaration syntax.
//   - `Col<E, T>` is the handle: `Copy`, const-constructible, `IntoQuery`.
//     Its `iq()` resolves the column ONCE at plan construction; built plans
//     hold only `&'static` relations, exactly as before.
//   - `Spec` is a fully type-ERASED (no `E`, no `T`) column descriptor
//     carrying a monomorphized loader fn pointer. A `&[Spec]` is plain data,
//     so it serves as both the eager-init work list and regen's manifest.
//   - `init` walks the specs once and FREEZES the registry into a
//     `OnceLock<HashMap>`. Because loading is eager, the registry is
//     read-only afterwards — no locking anywhere on the query path.

use crate::cache;
use crate::engine::{
    Compose, EntityKind, Id, Ident, IntoQuery, MultiRel, Query, Universe, VecRel,
};
use crate::format::{KIND_CSR_STR, KIND_CSR_WORDS, KIND_DENSE_F64, KIND_DENSE_I64, KIND_DENSE_STR};
use std::any::Any;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::OnceLock;

/// A dense string column's value type — spelled `Str` so declarations read
/// `Col<Customer, Str>` rather than `Col<Customer, &'static str>`.
pub type Str = &'static str;

/// Declared type of a set-valued (CSR) column: `Col<Film, Multi<Id<Tag>>>`.
#[allow(dead_code)]
pub struct Multi<T>(PhantomData<T>);

// ===== the registry ======================================================
// Built once by `init`, immutable thereafter. Keyed by (entity, field) —
// a tuple rather than a joined `"Film_title"` because const string
// concatenation isn't available; the joined form is only ever needed as a
// FILENAME, which `init` formats at load time.

/// One loaded column: the erased relation plus its key count (which sizes
/// the entity's universe — the erased value alone can't report it).
struct Loaded {
    data: &'static (dyn Any + Send + Sync),
    n_keys: usize,
}

static REGISTRY: OnceLock<HashMap<(&'static str, &'static str), Loaded>> = OnceLock::new();

fn registry() -> &'static HashMap<(&'static str, &'static str), Loaded> {
    REGISTRY
        .get()
        .expect("schema not initialized — call schema_experiment::init first")
}

// ===== ColType — declared type → storage, kind, loader, resolved query ====

/// How a declared column type is stored, read, and turned into a query.
/// One impl per column SHAPE (five of them), written once and generic over
/// the owning entity — where `schema!` needed a macro arm per shape per use.
pub trait ColType: 'static {
    /// Physical relation held in the registry.
    type Store<E: Entity>: Any + Send + Sync;
    /// What a handle of this type resolves to at plan-construction time.
    type Out<E: Entity>: Query<D = Id<E>>;
    /// `format::KIND_*` this column's cache file must carry.
    const KIND: u32;
    fn load<E: Entity>(dir: &Path, file: &str) -> Self::Store<E>;
    fn n_keys<E: Entity>(store: &Self::Store<E>) -> usize;
    fn resolve<E: Entity>(store: &'static Self::Store<E>) -> Self::Out<E>;
}

macro_rules! scalar_coltype {
    ($t:ty, $kind:expr, $load:path) => {
        impl ColType for $t {
            type Store<E: Entity> = VecRel<$t, Id<E>>;
            type Out<E: Entity> = &'static VecRel<$t, Id<E>>;
            const KIND: u32 = $kind;
            #[inline]
            fn load<E: Entity>(dir: &Path, file: &str) -> Self::Store<E> {
                $load(dir, file)
            }
            #[inline]
            fn n_keys<E: Entity>(s: &Self::Store<E>) -> usize {
                s.n_keys()
            }
            #[inline]
            fn resolve<E: Entity>(s: &'static Self::Store<E>) -> Self::Out<E> {
                s
            }
        }
    };
}

// (A local 4-line helper, not schema-facing: the three scalar impls differ
// only in type/kind/loader. Spelling them out costs ~45 duplicated lines.)
scalar_coltype!(i64, KIND_DENSE_I64, cache::load_i64_in);
scalar_coltype!(f64, KIND_DENSE_F64, cache::load_f64_in);
scalar_coltype!(Str, KIND_DENSE_STR, cache::load_strs_in);

/// Foreign key into entity `T`. The stored value is `T::Fk` — `Id<T>` for a
/// dense entity, `Key<T>` for a `dict` one — and resolving composes `T`'s
/// entity table, so a dense FK is a bare column compose (`Ident` inlines to
/// nothing) and a dict FK gains the dictionary probe. Same declaration.
impl<T> ColType for Id<T>
where
    T: EntityKind,
    T::Fk: Send + Sync + 'static,
    T::Table: 'static,
{
    type Store<E: Entity> = VecRel<T::Fk, Id<E>>;
    type Out<E: Entity> = Compose<&'static VecRel<T::Fk, Id<E>>, T::Table>;
    const KIND: u32 = KIND_DENSE_I64;
    #[inline]
    fn load<E: Entity>(dir: &Path, file: &str) -> Self::Store<E> {
        cache::load_fk_in::<T, _>(dir, file)
    }
    #[inline]
    fn n_keys<E: Entity>(s: &Self::Store<E>) -> usize {
        s.n_keys()
    }
    #[inline]
    fn resolve<E: Entity>(s: &'static Self::Store<E>) -> Self::Out<E> {
        Compose {
            a: s,
            b: T::table(),
        }
    }
}

/// Set-valued FK. Multi columns always store dense `Id<T>` (regen writes row
/// ids), so unlike the scalar FK there is no table hop.
impl<T: Send + Sync + 'static> ColType for Multi<Id<T>> {
    type Store<E: Entity> = MultiRel<Id<T>, Id<E>>;
    type Out<E: Entity> = &'static MultiRel<Id<T>, Id<E>>;
    const KIND: u32 = KIND_CSR_WORDS;
    #[inline]
    fn load<E: Entity>(dir: &Path, file: &str) -> Self::Store<E> {
        cache::load_multi_ids_in::<T, _>(dir, file)
    }
    #[inline]
    fn n_keys<E: Entity>(s: &Self::Store<E>) -> usize {
        s.n_keys()
    }
    #[inline]
    fn resolve<E: Entity>(s: &'static Self::Store<E>) -> Self::Out<E> {
        s
    }
}

impl ColType for Multi<i64> {
    type Store<E: Entity> = MultiRel<i64, Id<E>>;
    type Out<E: Entity> = &'static MultiRel<i64, Id<E>>;
    const KIND: u32 = KIND_CSR_WORDS;
    #[inline]
    fn load<E: Entity>(dir: &Path, file: &str) -> Self::Store<E> {
        cache::load_multi_i64_in(dir, file)
    }
    #[inline]
    fn n_keys<E: Entity>(s: &Self::Store<E>) -> usize {
        s.n_keys()
    }
    #[inline]
    fn resolve<E: Entity>(s: &'static Self::Store<E>) -> Self::Out<E> {
        s
    }
}

impl ColType for Multi<Str> {
    type Store<E: Entity> = MultiRel<Str, Id<E>>;
    type Out<E: Entity> = &'static MultiRel<Str, Id<E>>;
    const KIND: u32 = KIND_CSR_STR;
    #[inline]
    fn load<E: Entity>(dir: &Path, file: &str) -> Self::Store<E> {
        cache::load_multi_strs_in(dir, file)
    }
    #[inline]
    fn n_keys<E: Entity>(s: &Self::Store<E>) -> usize {
        s.n_keys()
    }
    #[inline]
    fn resolve<E: Entity>(s: &'static Self::Store<E>) -> Self::Out<E> {
        s
    }
}

// ===== Spec — the erased column descriptor ================================

/// A column, with every type parameter erased: plain data plus a
/// monomorphized loader. `init` CALLS `load`; regen READS `(entity, field,
/// kind)` — one structure, both jobs.
pub struct Spec {
    pub entity: &'static str,
    pub field: &'static str,
    pub kind: u32,
    load: fn(&Path, &str) -> Loaded,
}

/// The monomorphized loader a `Spec` carries: read the column, leak it to
/// `&'static`, erase it to `dyn Any`.
fn load_erased<E: Entity, T: ColType>(dir: &Path, file: &str) -> Loaded {
    let store: T::Store<E> = T::load::<E>(dir, file);
    let n_keys = T::n_keys::<E>(&store);
    Loaded {
        data: Box::leak(Box::new(store)) as &'static (dyn Any + Send + Sync),
        n_keys,
    }
}

// ===== Col — the one handle type =========================================

/// A column handle: `Customer::name`. Carries the owning entity and the
/// declared column type in phantoms, so composing across mismatched entities
/// is a COMPILE error exactly as with the generated ZSTs.
pub struct Col<E, T> {
    field: &'static str,
    _p: PhantomData<fn() -> (E, T)>,
}

// Manual, like `Id<E>`: derive would bound the phantoms.
impl<E, T> Copy for Col<E, T> {}
impl<E, T> Clone for Col<E, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<E, T> Col<E, T> {
    /// `Col::new("name")` — the field name is the cache filename's suffix.
    pub const fn new(field: &'static str) -> Self {
        Col {
            field,
            _p: PhantomData,
        }
    }
}

impl<E: Entity, T: ColType> Col<E, T> {
    /// Erase this column to a `Spec`. Const, so `cols!` can build
    /// `Entity::COLUMNS` as a `const`.
    pub const fn spec(self) -> Spec {
        Spec {
            entity: E::NAME,
            field: self.field,
            kind: T::KIND,
            load: load_erased::<E, T>,
        }
    }

    /// The loaded relation. One hash lookup + a checked downcast, paid at
    /// plan construction; the plan itself holds the `&'static`.
    pub fn rel(self) -> &'static T::Store<E> {
        let l = registry().get(&(E::NAME, self.field)).unwrap_or_else(|| {
            panic!(
                "column {}_{} not registered — add `{}` to its cols!(..) list",
                E::NAME,
                self.field,
                self.field
            )
        });
        l.data.downcast_ref::<T::Store<E>>().unwrap_or_else(|| {
            panic!(
                "column {}_{} loaded at a different type than declared",
                E::NAME,
                self.field
            )
        })
    }
}

impl<E: Entity, T: ColType> IntoQuery for Col<E, T> {
    type Q = T::Out<E>;
    #[inline]
    fn iq(self) -> Self::Q {
        T::resolve::<E>(self.rel())
    }
}

// ===== Entity ============================================================

/// An entity tag. `cols!` writes this impl; everything else about the entity
/// (the tag struct, the column consts) is hand-written plain Rust.
pub trait Entity: Sized + Send + Sync + 'static {
    /// Filename prefix — `<NAME>_<field>.bin`.
    const NAME: &'static str;
    /// Every column of this entity, erased. The FIRST entry sizes `all()`.
    const COLUMNS: &'static [Spec];

    /// The entity's universe: drive over `0..n`, sized by the first declared
    /// column's key count (as `schema!`'s universe handle was).
    fn all() -> Universe<Id<Self>> {
        let c = Self::COLUMNS
            .first()
            .expect("entity declares no columns — cols!(E: ..) needs at least one");
        let l = registry()
            .get(&(c.entity, c.field))
            .expect("schema not initialized for this entity");
        Universe::new(l.n_keys)
    }
}

/// A DENSELY addressed entity (external id == row index). `cols!` implements
/// it; a `dict` entity opts out (`cols!(E dict: ..)`) and writes its own
/// `EntityKind` over `Key`/`DictTable`.
pub trait DenseEntity: Entity {}

impl<E: DenseEntity> EntityKind for E {
    type Fk = Id<E>;
    type Table = Ident<E>;
    #[inline(always)]
    fn table() -> Self::Table {
        Ident::new()
    }
}

/// Object-safe view of an entity, so a schema is a plain
/// `&[&dyn Schema]` — no macro needed to enumerate entities.
pub trait Schema: Send + Sync {
    fn columns(&self) -> &'static [Spec];
}
impl<E: Entity> Schema for E {
    fn columns(&self) -> &'static [Spec] {
        E::COLUMNS
    }
}

// ===== init / manifest ===================================================

/// Eagerly load every column of every entity from `<dir>/<Entity>_<field>.bin`
/// and freeze the registry. Panics if called twice.
pub fn init(dir: &Path, schema: &[&dyn Schema]) {
    let mut map: HashMap<(&'static str, &'static str), Loaded> = HashMap::new();
    for ent in schema {
        for c in ent.columns() {
            let file = format!("{}_{}", c.entity, c.field);
            let loaded = (c.load)(dir, &file);
            if map.insert((c.entity, c.field), loaded).is_some() {
                panic!("duplicate column {file} in schema");
            }
        }
    }
    if REGISTRY.set(map).is_err() {
        panic!("schema already initialized");
    }
}

/// `(entity, field, kind)` for every declared column — what regen verifies
/// its cache output against. Same `Spec` list `init` walks.
pub fn manifest(schema: &[&dyn Schema]) -> Vec<(&'static str, &'static str, u32)> {
    schema
        .iter()
        .flat_map(|e| e.columns().iter().map(|c| (c.entity, c.field, c.kind)))
        .collect()
}

// ===== cols! =============================================================

/// Enumerate an entity's column consts — the one thing Rust can't reflect.
///
/// ```text
/// cols!(Film: title, year, genre);   // dense (the common case)
/// cols!(Studio dict: id, name);      // non-dense; write EntityKind by hand
/// ```
///
/// The macro never mentions a type, concatenates no idents, and has no
/// accumulator munchers: it is two flat rules over `$( $Ent::$f.spec() ),*`.
// (Used only by the sandbox until the JOB/TPC-H schemas port over.)
#[allow(unused_macros)]
macro_rules! cols {
    ($Ent:ident : $($f:ident),+ $(,)?) => {
        $crate::schema_experiment::cols!(@ent $Ent : $($f),+);
        impl $crate::schema_experiment::DenseEntity for $Ent {}
    };
    ($Ent:ident dict : $($f:ident),+ $(,)?) => {
        $crate::schema_experiment::cols!(@ent $Ent : $($f),+);
    };
    (@ent $Ent:ident : $($f:ident),+) => {
        impl $crate::schema_experiment::Entity for $Ent {
            const NAME: &'static str = stringify!($Ent);
            const COLUMNS: &'static [$crate::schema_experiment::Spec] =
                &[ $( $Ent::$f.spec() ),+ ];
        }
    };
}

#[allow(unused_imports)]
pub(crate) use cols;

// ===== sandbox ===========================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::*;
    use crate::format::{align8, header, HEADER_LEN};
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    // ----- the schema, in plain Rust -------------------------------------

    pub struct Film;
    pub struct Genre;
    pub struct Tag;
    pub struct Studio;

    #[allow(non_upper_case_globals)]
    impl Film {
        pub const title: Col<Self, Str> = Col::new("title");
        pub const year: Col<Self, i64> = Col::new("year");
        pub const rating: Col<Self, f64> = Col::new("rating");
        pub const genre: Col<Self, Id<Genre>> = Col::new("genre");
        pub const tags: Col<Self, Multi<Id<Tag>>> = Col::new("tags");
        pub const studio: Col<Self, Id<Studio>> = Col::new("studio");
        /// Declared but deliberately LEFT OUT of `cols!` below — the
        /// forgot-to-list failure mode, asserted in `unlisted_column_panics`.
        pub const unlisted: Col<Self, i64> = Col::new("unlisted");
    }
    cols!(Film: title, year, rating, genre, tags, studio);

    #[allow(non_upper_case_globals)]
    impl Genre {
        pub const name: Col<Self, Str> = Col::new("name");
        pub const kind: Col<Self, Str> = Col::new("kind");
    }
    cols!(Genre: name, kind);

    #[allow(non_upper_case_globals)]
    impl Tag {
        pub const text: Col<Self, Str> = Col::new("text");
        pub const films: Col<Self, Multi<Id<Film>>> = Col::new("films");
    }
    cols!(Tag: text, films);

    // A NON-DENSE entity: addressed by external ids (100/205/9899), not rows.
    // `cols!(.. dict: ..)` skips the DenseEntity impl; the EntityKind impl is
    // hand-written, reading the id column through the ordinary handle.
    #[allow(non_upper_case_globals)]
    impl Studio {
        pub const id: Col<Self, i64> = Col::new("id");
        pub const name: Col<Self, Str> = Col::new("name");
    }
    cols!(Studio dict: id, name);

    impl EntityKind for Studio {
        type Fk = Key<Studio>;
        type Table = &'static DictTable<Studio>;
        fn table() -> Self::Table {
            static T: OnceLock<DictTable<Studio>> = OnceLock::new();
            T.get_or_init(|| DictTable::from_i64(&Studio::id.rel().values))
        }
    }

    const SANDBOX: &[&dyn Schema] = &[&Film, &Genre, &Tag, &Studio];

    // ----- synthetic v2 cache --------------------------------------------

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

    // Films: 0 Alien/1979/8.5/horror/{cult}/A24,
    //        1 Blade/1998/7.1/action/{cult,noir}/Warner,
    //        2 Solaris/1972/8.1/horror/{}/A24
    fn build_cache() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("prela_cols_test_{}", std::process::id()));
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
        // FK into the dict entity: EXTERNAL keys, not rows.
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

    // `init` is one-shot and global; `Once` makes it safe from either test.
    static SETUP: std::sync::Once = std::sync::Once::new();
    fn setup() {
        SETUP.call_once(|| init(&build_cache(), SANDBOX));
    }

    #[test]
    fn cols_sandbox() {
        setup();

        // universe, sized off the first declared column
        assert_eq!(Film::all().n, 3);
        assert_eq!(Genre::all().n, 2);

        // scalar columns: a handle drives on its own
        assert_eq!(collect(Film::title), vec!["Alien", "Blade", "Solaris"]);
        assert_eq!(collect(Film::rating), vec![8.5, 7.1, 8.1]);

        // filter + restrict, rooted at the universe
        assert_eq!(
            collect(Film::all().with(Film::year.lt(1990)).select(Film::title)),
            vec!["Alien", "Solaris"]
        );

        // FK navigation into a DENSE entity — `Ident` hop, so this is just a
        // column compose. Chaining is `.select`, per the design (no nav methods).
        assert_eq!(
            collect(Film::genre.select(Genre::name)),
            vec!["horror", "action", "horror"]
        );
        assert_eq!(
            collect(
                Film::all()
                    .with(Film::genre.select(Genre::name).eq("horror"))
                    .select(Film::title)
            ),
            vec!["Alien", "Solaris"]
        );

        // the raw FK still reads as typed ids
        assert_eq!(
            collect(Film::genre.rel()),
            vec![Id::<Genre>::new(1), Id::new(0), Id::new(1)]
        );

        // Multi column, and navigation THROUGH it
        assert_eq!(
            collect(Film::tags.select(Tag::text)),
            vec!["cult", "cult", "noir"]
        );
        assert_eq!(
            collect(
                Film::all()
                    .with(Film::tags.select(Tag::text).eq("noir"))
                    .select(Film::title)
            ),
            vec!["Blade"]
        );
        // ...and back the other way: Tag::films is Film-valued
        assert_eq!(collect(Tag::films.select(Film::year)), vec![1979, 1998, 1998]);

        // NON-DENSE entity: the same `Col<_, Id<Studio>>` declaration routes
        // through the DictTable, external key → row → name.
        assert_eq!(
            collect(Film::studio.select(Studio::name)),
            vec!["A24", "Warner", "A24"]
        );
        // the column itself genuinely stores un-followed external Keys
        assert_eq!(
            collect(Film::studio.rel()),
            vec![Key::<Studio>::new(205), Key::new(100), Key::new(205)]
        );

        // cross-entity mistakes stay COMPILE errors — this does not compile:
        //   Film::genre.select(Tag::text)   // expected Id<Genre>, found Id<Tag>

        // the manifest regen verifies against falls out of the same Specs
        assert_eq!(
            manifest(SANDBOX),
            vec![
                ("Film", "title", KIND_DENSE_STR),
                ("Film", "year", KIND_DENSE_I64),
                ("Film", "rating", KIND_DENSE_F64),
                ("Film", "genre", KIND_DENSE_I64),
                ("Film", "tags", KIND_CSR_WORDS),
                ("Film", "studio", KIND_DENSE_I64),
                ("Genre", "name", KIND_DENSE_STR),
                ("Genre", "kind", KIND_DENSE_STR),
                ("Tag", "text", KIND_DENSE_STR),
                ("Tag", "films", KIND_CSR_WORDS),
                ("Studio", "id", KIND_DENSE_I64),
                ("Studio", "name", KIND_DENSE_STR),
            ]
        );
    }

    // The one failure mode `cols!` introduces: a const the list forgets is
    // never loaded, and says so on first touch.
    #[test]
    #[should_panic(expected = "not registered")]
    fn unlisted_column_panics() {
        setup();
        let _ = Film::unlisted.iq();
    }
}
