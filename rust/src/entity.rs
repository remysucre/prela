//! PROTOTYPE — decomposing tables into `id → row` (an ENTITY TABLE) plus
//! `row → attr` (dense columns), so entities with non-dense / non-contiguous
//! external ids work with the same combinator surface as the dense engine.
//!
//! The dense engine fuses two things: a value's external IDENTITY and its
//! STORAGE ADDRESS (`Id<E>` is both the key and the `Vec` index). That fusion
//! is what gives O(1) column access — but it assumes ids are a dense `0..n`.
//!
//! Decompose them:
//!   - `Key<E>`        — the external id (arbitrary, non-dense).
//!   - `Id<E>`         — the dense ROW index (as today), addresses columns.
//!   - an ENTITY TABLE — the relation `Key<E> → Id<E>` (addressing layer).
//!   - columns         — `VecRel<_, Id<E>>` i.e. `Id<E> → attr` (storage).
//!
//! Navigating a foreign key `c: Key<S>` of `r` into `S`'s attribute `x`:
//!   `r.select(c)            // Row<R> → Key<S>     (read the FK)`
//!   ` .select(s_table)      // Key<S> → Id<S>      (the id→row hop)`
//!   ` .select(s_x)          // Id<S>  → x          (dense column)`
//!
//! Two entity-table shapes carry the whole generalisation:
//!   - `DictTable<E>` — non-dense: a `HashMap<Key, Id>` lookup.
//!   - `Ident<E>`     — dense (`Key == Id`): the identity relation, a
//!                      pass-through that `Compose` inlines away, so dense
//!                      navigation pays NOTHING for the indirection.
//!
//! Crucially the generic combinators already accept this: `Query::D` is only
//! `Copy + Eq + Hash` (no `Dense` bound), so `Compose`/`Probe` thread a
//! non-dense `Key<E>` domain fine — only the dense leaves need `Dense`.

use std::collections::HashMap;
use std::marker::PhantomData;

use crate::engine::*;

/// External, possibly non-dense / non-contiguous id of entity `E` — distinct
/// from `Id<E>`, which is the dense ROW index that addresses columns.
/// Manual `Copy/Eq/Hash` (like `Id<E>`): `derive` would wrongly bound `E`.
pub struct Key<E: 'static>(pub u64, pub PhantomData<E>);
impl<E> Key<E> {
    #[inline(always)]
    pub fn new(k: u64) -> Self { Key(k, PhantomData) }
}
impl<E> Copy for Key<E> {}
impl<E> Clone for Key<E> { #[inline(always)] fn clone(&self) -> Self { *self } }
impl<E> PartialEq for Key<E> { #[inline(always)] fn eq(&self, o: &Self) -> bool { self.0 == o.0 } }
impl<E> Eq for Key<E> {}
impl<E> std::hash::Hash for Key<E> {
    #[inline(always)] fn hash<H: std::hash::Hasher>(&self, h: &mut H) { self.0.hash(h) }
}
impl<E> std::fmt::Debug for Key<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "Key({})", self.0) }
}

// ===== entity tables: the `id → row` addressing layer ====================

/// Entity table for a NON-DENSE entity: external `Key<E>` → dense `Id<E>` row.
/// The one place the addressing scheme lives; columns stay dense `VecRel`.
pub struct DictTable<E: 'static> {
    map: HashMap<Key<E>, Id<E>>,
}
impl<E: 'static> DictTable<E> {
    /// `keys[row]` = the external id assigned to dense row `row` (`0..keys.len()`).
    pub fn new(keys: &[u64]) -> Self {
        DictTable {
            map: keys.iter().enumerate().map(|(r, &k)| (Key::new(k), Id::from_idx(r))).collect(),
        }
    }
    pub fn len(&self) -> usize { self.map.len() }
    pub fn is_empty(&self) -> bool { self.map.is_empty() }
}
impl<E: 'static> Query for DictTable<E> { type D = Key<E>; type R = Id<E>; }
impl<E: 'static> Probe for DictTable<E> {
    #[inline]
    fn probe<F: FnMut(Id<E>)>(&self, x: Key<E>, mut f: F) {
        if let Some(&r) = self.map.get(&x) { f(r); }
    }
    #[inline]
    fn probe_any<F: FnMut(Id<E>) -> bool>(&self, x: Key<E>, mut f: F) -> bool {
        self.map.get(&x).is_some_and(|&r| f(r))
    }
    #[inline]
    fn member(&self, x: Key<E>) -> bool { self.map.contains_key(&x) }
}

/// Entity table for a non-dense entity as a SORTED key array with binary
/// search — the cache-friendlier, order-preserving alternative to `DictTable`'s
/// hash map (your "btree instead of vectors"). Same `Key<E> → Id<E>` interface,
/// so navigation is identical; only the addressing data structure differs.
pub struct SortedTable<E: 'static> {
    sorted: Vec<(u64, Id<E>)>,   // (external key, row), sorted by key
}
impl<E: 'static> SortedTable<E> {
    pub fn new(keys: &[u64]) -> Self {
        let mut sorted: Vec<(u64, Id<E>)> =
            keys.iter().enumerate().map(|(r, &k)| (k, Id::from_idx(r))).collect();
        sorted.sort_by_key(|&(k, _)| k);
        SortedTable { sorted }
    }
    #[inline]
    fn lookup(&self, k: u64) -> Option<Id<E>> {
        self.sorted.binary_search_by_key(&k, |&(k, _)| k).ok().map(|i| self.sorted[i].1)
    }
}
impl<E: 'static> Query for SortedTable<E> { type D = Key<E>; type R = Id<E>; }
impl<E: 'static> Probe for SortedTable<E> {
    #[inline] fn probe<F: FnMut(Id<E>)>(&self, x: Key<E>, mut f: F) {
        if let Some(r) = self.lookup(x.0) { f(r); }
    }
    #[inline] fn probe_any<F: FnMut(Id<E>) -> bool>(&self, x: Key<E>, mut f: F) -> bool {
        self.lookup(x.0).is_some_and(|r| f(r))
    }
    #[inline] fn member(&self, x: Key<E>) -> bool { self.lookup(x.0).is_some() }
}

/// Entity table for a DENSE entity — the external id IS the row, so the table
/// is the identity relation `Id<E> → Id<E>`. `probe` is a pass-through, so
/// `Compose<_, Ident>` inlines to its left operand: dense navigation pays
/// nothing for the `id → row` hop. (This is the compile-time-dispatch lesson
/// from `SparseUniverse`: the trivial case is a distinct type that vanishes.)
pub struct Ident<E: 'static>(pub PhantomData<E>);
impl<E> Default for Ident<E> { fn default() -> Self { Ident(PhantomData) } }
impl<E> Ident<E> { #[inline(always)] pub fn new() -> Self { Ident(PhantomData) } }
impl<E: 'static> Query for Ident<E> { type D = Id<E>; type R = Id<E>; }
impl<E: 'static> Probe for Ident<E> {
    #[inline(always)] fn probe<F: FnMut(Id<E>)>(&self, x: Id<E>, mut f: F) { f(x); }
    #[inline(always)] fn probe_any<F: FnMut(Id<E>) -> bool>(&self, x: Id<E>, mut f: F) -> bool { f(x) }
    #[inline(always)] fn member(&self, _x: Id<E>) -> bool { true }
}

// ===== scalar `Index`: `table[key]` is one address translation ===========
// `Index` is the operator that genuinely fits (it has the table AND the key),
// unlike `Deref`. It's the SCALAR escape — resolve ONE id — for use inside a
// fold/drive closure or output formatting. The relational `.select`/`.as_`
// chain is the same translation done set-at-a-time over a whole column.
//
//   cols[ table[ fk[row] ] ]   ≡   physical_mem[ page_table[ vaddr ] ]
//     │      │      └ read FK:        Row<R> → Key<S>
//     │      └─────── translate:      Key<S> → Id<S>   (the page table)
//     └────────────── read attr:      Id<S>  → x

/// `table[key]` → the row (panics on a dangling key, like `Vec`/`HashMap`).
impl<E: 'static> std::ops::Index<Key<E>> for DictTable<E> {
    type Output = Id<E>;
    #[inline] fn index(&self, key: Key<E>) -> &Id<E> { &self.map[&key] }
}

/// `column[row]` → the attribute. (A dense column is just `Id → attr`.)
impl<R: Copy, D: Dense> std::ops::Index<D> for VecRel<R, D> {
    type Output = R;
    #[inline] fn index(&self, d: D) -> &R { &self.values[d.idx()] }
}

// ===== ergonomics: `.as_(table)` reads as "this id, as an entity row" ====
// Alias of `select` that names the entity-table crossing. The schema-generated
// attribute navs (`.name()`, …) would bake this hop in front of the column so
// it never surfaces for single-attribute reads; `.as_(E)` is the explicit form
// used to SHARE the hop across several columns (`x.as_(E).select(a.and(b))`).

pub trait Nav: IntoQuery + Sized {
    #[inline(always)]
    fn as_<T: IntoQuery>(self, table: T) -> Compose<Self::Q, T::Q>
    where T::Q: Query<D = ROf<Self>> { self.select(table) }
}
impl<T: IntoQuery> Nav for T {}

// ===== Resolve: "deref" on a COLUMN — auto-follow FKs, like Rust's `.` =====
//
// Putting the deref on the column (not the value type) is what dodges the
// `f64 ∉ Eq+Hash` wall: a scalar column's deref is the identity *function*
// (it returns ITSELF), so no `Probe<D = f64>` is ever built. An FK column's
// deref crosses its target entity table.
//
// A `Field` exposes its column two ways:
//   raw      — the stored relation (`Id<E> → A`, or `Id<E> → Key<S>` for an FK)
//   resolved — raw for scalars; `raw ∘ table` for FKs (`Id<E> → Id<S>`)
// `.s(c)` selects `resolved` (auto-deref, the `.`-behaviour); `.at(c)` selects
// `raw` (the FK's `Key`, un-followed). For a DENSE target the table is `Ident`,
// so `resolved == raw` and `.s == .at` at zero cost (see the composes-away
// test). The schema macro would emit one `Field` impl per declared field —
// identity `resolved` for scalars, table-crossing `resolved` for FKs — so the
// two impls never overlap.

pub trait Column {
    type Raw: IntoQuery;
    type Resolved: IntoQuery;
    fn raw(self) -> Self::Raw;
    fn resolved(self) -> Self::Resolved;
}

/// Scalar column `Id<E> → A` — `resolved` is the identity (returns itself).
pub struct Col<'a, A: Copy, E: 'static>(pub &'a VecRel<A, Id<E>>);
impl<'a, A: Copy, E: 'static> Column for Col<'a, A, E> {
    type Raw = &'a VecRel<A, Id<E>>;
    type Resolved = &'a VecRel<A, Id<E>>;
    #[inline] fn raw(self) -> Self::Raw { self.0 }
    #[inline] fn resolved(self) -> Self::Resolved { self.0 }
}

/// Foreign-key column `Id<E> → Key<S>` — `resolved` crosses S's entity table,
/// yielding `Id<E> → Id<S>`. (Table bundled here; the real schema fetches it
/// from the global store by the FK's target type `S`.)
pub struct Fk<'a, S: 'static, E: 'static> {
    pub col: &'a VecRel<Key<S>, Id<E>>,
    pub table: &'a DictTable<S>,
}
impl<'a, S: 'static, E: 'static> Column for Fk<'a, S, E> {
    type Raw = &'a VecRel<Key<S>, Id<E>>;
    type Resolved = Compose<&'a VecRel<Key<S>, Id<E>>, &'a DictTable<S>>;
    #[inline] fn raw(self) -> Self::Raw { self.col }
    #[inline] fn resolved(self) -> Self::Resolved { Compose { a: self.col, b: self.table } }
}

/// `.s(c)` — select a column, auto-deref'd (FKs followed into their entity).
/// `.at(c)` — select the RAW column (an FK stays its `Key`).
pub trait Navigate: IntoQuery + Sized {
    #[inline]
    fn s<F: Column>(self, c: F) -> Compose<Self::Q, <F::Resolved as IntoQuery>::Q>
    where <F::Resolved as IntoQuery>::Q: Query<D = ROf<Self>> {
        Compose { a: self.iq(), b: c.resolved().iq() }
    }
    #[inline]
    fn at<F: Column>(self, c: F) -> Compose<Self::Q, <F::Raw as IntoQuery>::Q>
    where <F::Raw as IntoQuery>::Q: Query<D = ROf<Self>> {
        Compose { a: self.iq(), b: c.raw().iq() }
    }
}
impl<T: IntoQuery> Navigate for T {}

// ===== demonstration ======================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Tag types for two entities.
    struct Movie;
    struct Person;

    fn col<R: Copy, D: Dense>(values: Vec<R>) -> VecRel<R, D> {
        VecRel { values, _d: PhantomData }
    }

    fn drive_sorted<Q: Drive>(q: &Q) -> Vec<(usize, Q::R)>
    where Q::D: Dense, Q::R: Ord {
        let mut v = Vec::new();
        q.drive(|d, r| v.push((d.idx(), r)));
        v.sort();
        v
    }

    // Person stored DECOMPOSED: external ids {100, 205, 9899} (non-dense) map
    // to dense rows {0, 1, 2}; the name column is addressed by row.
    //   movie 0 → director key 205 (Kubrick)
    //   movie 1 → director key 100 (Nolan)
    //   movie 2 → director key 9899 (Tarkovsky)
    #[test]
    fn navigate_into_nondense_entity() {
        let person_table = DictTable::<Person>::new(&[100, 205, 9899]);
        let person_name = col::<&str, Id<Person>>(vec!["Nolan", "Kubrick", "Tarkovsky"]);
        let director = col::<Key<Person>, Id<Movie>>(
            vec![Key::new(205), Key::new(100), Key::new(9899)]);
        let movies = Universe::<Id<Movie>>::new(3);

        // movie → director(Key) → person(row) → name — one combinator chain.
        let q = movies
            .select(&director)        // Row<Movie> → Key<Person>
            .as_(&person_table)       // Key<Person> → Id<Person>  (the id→row hop)
            .select(&person_name);    // Id<Person>  → name
        assert_eq!(drive_sorted(&q),
            vec![(0, "Kubrick"), (1, "Nolan"), (2, "Tarkovsky")]);
    }

    // Same query shape against a DENSE person entity (external id == row), so
    // the entity table is `Ident` — and the result is identical to skipping
    // the hop entirely, demonstrating `Compose<_, Ident>` is a no-op.
    #[test]
    fn dense_entity_table_composes_away() {
        let person_name = col::<&str, Id<Person>>(vec!["Nolan", "Kubrick", "Tarkovsky"]);
        // dense FK: movie → person ROW directly (id == row)
        let director = col::<Id<Person>, Id<Movie>>(
            vec![Id::from_idx(1), Id::from_idx(0), Id::from_idx(2)]);
        let movies = Universe::<Id<Movie>>::new(3);

        let with_hop = movies.select(&director).as_(Ident::<Person>::new()).select(&person_name);
        let no_hop   = movies.select(&director).select(&person_name);
        assert_eq!(drive_sorted(&with_hop), drive_sorted(&no_hop));
        assert_eq!(drive_sorted(&with_hop),
            vec![(0, "Kubrick"), (1, "Nolan"), (2, "Tarkovsky")]);
    }

    // The SCALAR form of the same navigation: `cols[table[fk[row]]]`, reading
    // exactly like a memory access through a page table. This is what you'd
    // write inside a fold/output closure that has a single row in hand.
    #[test]
    fn indexing_is_scalar_address_translation() {
        let person_table = DictTable::<Person>::new(&[100, 205, 9899]);
        let person_name = col::<&str, Id<Person>>(vec!["Nolan", "Kubrick", "Tarkovsky"]);
        let director = col::<Key<Person>, Id<Movie>>(
            vec![Key::new(205), Key::new(100), Key::new(9899)]);

        let m = Id::<Movie>::from_idx(0);
        //          read attr ───┐   translate ──┐   read FK ┐
        let name = person_name[person_table[director[m]]];
        assert_eq!(name, "Kubrick");

        // DENSE entity (id == row): the middle translation simply isn't there —
        // an identity-mapped address space needs no page table.
        let dense_director = col::<Id<Person>, Id<Movie>>(
            vec![Id::from_idx(1), Id::from_idx(0), Id::from_idx(2)]);
        let name2 = person_name[dense_director[m]];   // no page-table hop
        assert_eq!(name2, "Kubrick");   // movie 0 → row 1, same as the dict form
    }

    // `.s()` auto-derefs an FK column (follows it into the entity) and reads a
    // plain column uniformly — Rust-`.` behaviour; `.at()` gives the raw `Key`.
    #[test]
    fn resolve_navigation() {
        let person_table = DictTable::<Person>::new(&[100, 205, 9899]);
        let person_name = col::<&str, Id<Person>>(vec!["Nolan", "Kubrick", "Tarkovsky"]);
        let director = col::<Key<Person>, Id<Movie>>(
            vec![Key::new(205), Key::new(100), Key::new(9899)]);
        let movies = Universe::<Id<Movie>>::new(3);

        // .s(fk) follows director into Person; .s(plain) reads name — same verb.
        let names = movies
            .s(Fk { col: &director, table: &person_table })   // movie → Id<Person>
            .s(Col(&person_name));                            // Id<Person> → name
        assert_eq!(drive_sorted(&names),
            vec![(0, "Kubrick"), (1, "Nolan"), (2, "Tarkovsky")]);

        // .at(fk) gives the RAW foreign key — the Key, un-followed.
        let keys = movies.at(Fk { col: &director, table: &person_table });
        let mut got = Vec::new();
        keys.drive(|m, k| got.push((m.idx(), k.0)));
        got.sort();
        assert_eq!(got, vec![(0, 205), (1, 100), (2, 9899)]);
    }

    // A FULL query over a non-dense entity: filter + FK navigation + group_by +
    // fold all the way through. Proves the entity-table decomposition threads
    // the whole combinator surface, not just `select`.
    //   "count movies released after 2000, grouped by their director's country"
    #[test]
    fn full_aggregation_over_nondense_entity() {
        // Person: non-dense ids 100/205/9899 → rows 0/1/2, with a country.
        let person_table = DictTable::<Person>::new(&[100, 205, 9899]);
        let person_country = col::<&str, Id<Person>>(vec!["US", "UK", "RU"]);
        // Movies: director FK (Key<Person>) + release year.
        let director = col::<Key<Person>, Id<Movie>>(vec![
            Key::new(205), Key::new(100), Key::new(9899), Key::new(100), Key::new(205)]);
        let year = col::<i64, Id<Movie>>(vec![1999, 2008, 2010, 2014, 2001]);
        let movies = Universe::<Id<Movie>>::new(5);

        // group key: movie → director (FK, auto-deref'd) → country
        let dir_country = Fk { col: &director, table: &person_table }
            .resolved()                  // movie → Id<Person>  (crossed the table)
            .select(&person_country);    // movie → country

        let counts = movies
            .with((&year).gt(2000))                       // filter: released after 2000
            .group_by(dir_country)                        // by director's country
            .fold(0_i64, |a, _| a + 1);                   // count

        let mut rows: Vec<(&str, i64)> = Vec::new();
        counts.drive(|k, v| rows.push((k, v)));
        rows.sort();
        // post-2000: m1(US) m2(RU) m3(US) m4(UK)  →  US=2, UK=1, RU=1
        assert_eq!(rows, vec![("RU", 1), ("UK", 1), ("US", 2)]);
    }

    // Multi-hop navigation across TWO non-dense entities — movie → director
    // (Person) → employer (Company) → name. Each `.s(Fk)` crosses one page
    // table; they nest, so the decomposition composes across entity boundaries.
    #[test]
    fn multi_hop_across_two_nondense_entities() {
        struct Company;
        // Company: non-dense ids 7/42 → rows 0/1, names.
        let company_table = DictTable::<Company>::new(&[7, 42]);
        let company_name = col::<&str, Id<Company>>(vec!["Warner", "A24"]);
        // Person: non-dense ids 100/205/9899, each with an employer FK.
        let person_table = DictTable::<Person>::new(&[100, 205, 9899]);
        let person_employer = col::<Key<Company>, Id<Person>>(
            vec![Key::new(42), Key::new(7), Key::new(42)]);
        // Movie: director FK into Person.
        let director = col::<Key<Person>, Id<Movie>>(
            vec![Key::new(205), Key::new(100), Key::new(9899)]);
        let movies = Universe::<Id<Movie>>::new(3);

        let q = movies
            .s(Fk { col: &director, table: &person_table })          // → Id<Person>
            .s(Fk { col: &person_employer, table: &company_table })  // → Id<Company>
            .s(Col(&company_name));                                  // → company name
        // m0→dir205→emp7→Warner; m1→dir100→emp42→A24; m2→dir9899→emp42→A24
        assert_eq!(drive_sorted(&q),
            vec![(0, "Warner"), (1, "A24"), (2, "A24")]);
    }

    // The addressing layer is PLUGGABLE: the exact same navigation runs over a
    // hash (`DictTable`) or a sorted/binary-search (`SortedTable`) entity table
    // — only the id→row data structure differs. (Ident is the third, for dense.)
    #[test]
    fn pluggable_addressing_hash_vs_sorted() {
        let names = col::<&str, Id<Person>>(vec!["Nolan", "Kubrick", "Tarkovsky"]);
        let director = col::<Key<Person>, Id<Movie>>(
            vec![Key::new(205), Key::new(100), Key::new(9899)]);
        let movies = Universe::<Id<Movie>>::new(3);

        let hash_table = DictTable::<Person>::new(&[100, 205, 9899]);
        let sorted_table = SortedTable::<Person>::new(&[100, 205, 9899]);
        // identical chain — read FK, cross the entity table, read name — with
        // only the table's data structure swapped under `.select(table)`.
        let via_hash   = movies.select(&director).select(&hash_table).select(&names);
        let via_sorted = movies.select(&director).select(&sorted_table).select(&names);
        assert_eq!(drive_sorted(&via_hash), drive_sorted(&via_sorted));
        assert_eq!(drive_sorted(&via_hash),
            vec![(0, "Kubrick"), (1, "Nolan"), (2, "Tarkovsky")]);
    }

    // Grouping by an FK needs NO entity table: the external `Key<S>` is already
    // `Eq + Hash`, so it's a valid group key directly (`.at`, un-deref'd). The
    // table is only needed to DEREF into the entity's columns. ("group_by an FK
    // yields external ids" — the corner flagged in the design discussion.)
    #[test]
    fn group_by_raw_foreign_key() {
        let director = col::<Key<Person>, Id<Movie>>(vec![
            Key::new(205), Key::new(100), Key::new(9899), Key::new(100), Key::new(205)]);
        let movies = Universe::<Id<Movie>>::new(5);

        let counts = movies.group_by(&director).fold(0_i64, |a, _| a + 1);
        let mut rows: Vec<(u64, i64)> = Vec::new();
        counts.drive(|k, v| rows.push((k.0, v)));
        rows.sort();
        assert_eq!(rows, vec![(100, 2), (205, 2), (9899, 1)]);
    }

    // The entity table is also a `Probe`, so it works in PROBE position too —
    // e.g. a semijoin keeping movies whose director key is a real person.
    #[test]
    fn entity_table_as_membership() {
        let person_table = DictTable::<Person>::new(&[100, 205, 9899]);
        // movie 1's director (404) is a dangling key — no such person.
        let director = col::<Key<Person>, Id<Movie>>(
            vec![Key::new(205), Key::new(404), Key::new(9899)]);
        let movies = Universe::<Id<Movie>>::new(3);

        // keep movies whose director resolves to a real person row
        let live = movies.with((&director).select(&person_table));
        let mut got: Vec<usize> = Vec::new();
        live.drive(|m, _| got.push(m.idx()));
        got.sort();
        assert_eq!(got, vec![0, 2]); // movie 1 (dangling 404) dropped
    }
}

// ===== what the schema macro would EMIT ===================================
// The end-user payoff: generated per-field handles + nav traits, so a query
// reads `movie.director().name()` and never mentions `Fk`/`Col`/`.s`. An FK
// nav FUSES the page-table crossing (so chaining lands you on the entity);
// a scalar nav is a plain select. For a DENSE entity the fused table is
// `Ident` and the nav is byte-identical to today's dense engine.
#[cfg(test)]
mod generated {
    use super::*;

    // entity tags
    struct Movie;
    struct Person;

    // The schema store (what `tpch_init`/`job_init` build): dense columns +
    // the one entity table for the non-dense entity (Person).
    struct Store {
        director: VecRel<Key<Person>, Id<Movie>>,  // Movie.director : FK → Person
        p_name:   VecRel<&'static str, Id<Person>>, // Person.name    : scalar
        p_table:  DictTable<Person>,                // Person's id→row table
    }
    static STORE: std::sync::OnceLock<Store> = std::sync::OnceLock::new();
    #[inline] fn store() -> &'static Store { STORE.get().expect("schema not initialized") }

    // Generated nav trait for Movie — one method per field. The FK field fuses
    // the resolve; a scalar field would be `self.select(&store().<col>)`.
    trait MovieNav: IntoQuery + Sized where Self::Q: Query<R = Id<Movie>> {
        #[inline]
        fn director(self) -> Compose<Self::Q,
            Compose<&'static VecRel<Key<Person>, Id<Movie>>, &'static DictTable<Person>>> {
            self.s(Fk { col: &store().director, table: &store().p_table })
        }
    }
    impl<T: IntoQuery> MovieNav for T where T::Q: Query<R = Id<Movie>> {}

    // Generated nav trait for Person.
    trait PersonNav: IntoQuery + Sized where Self::Q: Query<R = Id<Person>> {
        #[inline]
        fn name(self) -> Compose<Self::Q, &'static VecRel<&'static str, Id<Person>>> {
            self.select(&store().p_name)
        }
    }
    impl<T: IntoQuery> PersonNav for T where T::Q: Query<R = Id<Person>> {}

    #[test]
    fn generated_navs_read_cleanly() {
        STORE.set(Store {
            director: VecRel { values: vec![Key::new(205), Key::new(100)], _d: PhantomData },
            p_name:   VecRel { values: vec!["Nolan", "Kubrick"], _d: PhantomData },
            p_table:  DictTable::new(&[100, 205]),   // person ids 100,205 → rows 0,1
        }).ok();

        // the payoff — FK auto-deref'd by `.director()`, no Fk/Col/.s in sight:
        let q = (Universe::<Id<Movie>>::new(2)).director().name();
        let mut got = Vec::new();
        q.drive(|m, n| got.push((m.idx(), n)));
        got.sort();
        // m0 → dir 205 → person row 1 → "Kubrick";  m1 → dir 100 → row 0 → "Nolan"
        assert_eq!(got, vec![(0, "Kubrick"), (1, "Nolan")]);
    }
}
