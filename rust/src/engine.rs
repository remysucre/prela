use ahash::{AHashMap as HashMap, AHashSet as HashSet};
use regex::Regex;
use smallvec::SmallVec;
use std::hash::Hash;
use std::marker::PhantomData;

/// Default inline capacity for the probe-index buckets. Most TPC-H
/// foreign-key relations are 1:1 or 1:few (e.g. lineitems-per-order ≈ 4),
/// so this size keeps the common case inline + heap-free.
type SVec<T> = SmallVec<[T; 4]>;

// ==================
// MODE TRAITS
// ==================

pub trait Query {
    type D: Copy + Eq + Hash;
    type R: Copy;
}

pub trait Drive: Query {
    fn drive<K: FnMut(Self::D, Self::R)>(&self, k: K);
}

pub trait Member: Query {
    fn member(&self, x: Self::D) -> bool;
}

pub trait Probe: Member {
    fn probe<K: FnMut(Self::R)>(&self, x: Self::D, k: K);
    fn probe_any<K: FnMut(Self::R) -> bool>(&self, x: Self::D, k: K) -> bool;
}

impl<T: Probe> Member for T {
    #[inline(always)]
    fn member(&self, x: Self::D) -> bool {
        self.probe_any(x, |_| true)
    }
}

pub trait IntoQuery {
    type Q: Query;
    fn iq(self) -> Self::Q;
}
impl<Q: Query> IntoQuery for Q {
    type Q = Q;
    #[inline(always)]
    fn iq(self) -> Q {
        self
    }
}

/// The domain type of what `T` resolves to (`<T::Q as Query>::D`).
pub type DOf<T> = <<T as IntoQuery>::Q as Query>::D;
/// The value type of what `T` resolves to (`<T::Q as Query>::R`).
pub type ROf<T> = <<T as IntoQuery>::Q as Query>::R;

// blanket: &T inherits T's modes.
impl<T: Query + ?Sized> Query for &T {
    type D = T::D;
    type R = T::R;
}
impl<T: Drive + ?Sized> Drive for &T {
    #[inline(always)]
    fn drive<K: FnMut(T::D, T::R)>(&self, k: K) {
        (**self).drive(k);
    }
}
impl<T: Probe + ?Sized> Probe for &T {
    #[inline(always)]
    fn probe<K: FnMut(T::R)>(&self, x: T::D, k: K) {
        (**self).probe(x, k);
    }
    #[inline(always)]
    fn probe_any<K: FnMut(T::R) -> bool>(&self, x: T::D, k: K) -> bool {
        (**self).probe_any(x, k)
    }
}

// ==================
// LEAF STORAGE
// ==================

// Entity ids are 0-based `usize`: a universe of size n has ids 0..n-1,
// indexing its dense columns directly. (The cache stores these final
// physical layouts — 0-based, `NO_ID` holes baked in; see src/format.rs.)
// Ids are opaque dense indexes, so the id domain type is `usize`;
// scalar value columns (years, sizes, counts, …) stay `i64`/`f64`.
//
// `NO_ID` is the missing-id sentinel (FK hole fill, "none seen yet" fold
// states): it fails every `i < len` / `.get` bounds check, so a hole probes
// to nothing for free.
//
// `VecRel<R>` — total 1:1 relation; entity-id → R (one value per id).
// INVARIANT: an FK-valued column over a gappy key space (holes that a query
// can drive or probe, e.g. TPC-H ord_customer over the sparse orderkey
// domain) holds `NO_ID` in the holes — a default-0 hole would alias entity
// 0, which is a live id. (regen bakes the fill in; non-FK holes are
// `Default`: 0 / 0.0 / "".)
// `MultiRel<R>` — multi-valued / partial; CSR over the dense key space:
// row i = `values[offsets[i]..offsets[i+1]]`, empty range for missing
// keys. The slices are `&'static` — in production they point into the
// leaked cache mmap (zero-copy); `from_pairs` (unit tests) leaks two small
// Vecs to the same effect.

pub const NO_ID: usize = usize::MAX;

// ===== Dense domains: untyped `usize` or phantom-typed `Id<E>` ==========
// `Dense` abstracts "an id indexing a dense 0..n universe". The dense nodes
// are generic over it: `D = usize` is the untyped default (existing loaders
// and queries, unchanged); `D = Id<E>` carries a phantom entity tag so that
// composing through mismatched entities is a COMPILE error — e.g. with
// `movie_keyword: Query<D = Id<Movie>, R = Id<Keyword>>` and
// `person_name: Query<D = Id<Person>>`, `movie_keyword.select(person_name)`
// fails to type-check (expected `Id<Keyword>`, found `Id<Person>`).

pub trait Dense: Copy + Eq + Hash + 'static {
    /// Missing-id sentinel for this domain — fails every bounds check.
    const NONE: Self;
    fn idx(self) -> usize;
    fn from_idx(i: usize) -> Self;
}

impl Dense for usize {
    const NONE: usize = NO_ID;
    #[inline(always)]
    fn idx(self) -> usize {
        self
    }
    #[inline(always)]
    fn from_idx(i: usize) -> usize {
        i
    }
}

/// Phantom-typed entity id (the Julia engine's `ID{E}`). `repr(transparent)`
/// over `usize`, so typed id columns can be reinterpreted in bulk from the
/// cache's word arrays.
#[repr(transparent)]
pub struct Id<E: 'static>(pub usize, pub PhantomData<E>);

impl<E> Id<E> {
    #[inline(always)]
    pub fn new(i: usize) -> Self {
        Id(i, PhantomData)
    }
}
// Manual impls: `derive` would wrongly require bounds on the phantom `E`.
impl<E> Copy for Id<E> {}
impl<E> Clone for Id<E> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}
impl<E> PartialEq for Id<E> {
    #[inline(always)]
    fn eq(&self, o: &Self) -> bool {
        self.0 == o.0
    }
}
impl<E> Eq for Id<E> {}
impl<E> Hash for Id<E> {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.0.hash(h)
    }
}
// Ids are dense indexes; index order is the (arbitrary but total) order
// used by sinks like `count_distinct`'s sort+dedup.
impl<E> PartialOrd for Id<E> {
    #[inline(always)]
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl<E> Ord for Id<E> {
    #[inline(always)]
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.cmp(&o.0)
    }
}
impl<E> std::fmt::Debug for Id<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Id({})", self.0)
    }
}
impl<E: 'static> Dense for Id<E> {
    const NONE: Self = Id(NO_ID, PhantomData);
    #[inline(always)]
    fn idx(self) -> usize {
        self.0
    }
    #[inline(always)]
    fn from_idx(i: usize) -> Self {
        Id::new(i)
    }
}

// ===== primary-field elision (Julia's `==` on an entity value) ==========
// A comparison on an entity-valued query auto-navigates to the entity's
// PRIMARY (first-declared) scalar field before comparing — `keyword.eq("x")`
// ≡ `keyword.text().eq("x")`. The dispatch is on the RECEIVER's value type,
// not the argument: `Field` carries a GAT holding the (concrete) elided
// QUERY — `Q` for a scalar, `Compose<Q, &Primary>` for an `Id<E>` — while the
// comparator's closure stays method-level RPITIT. (The earlier dead end put
// the closure in the GAT, which is impossible; keeping the GAT query-only is
// the way through.) Scalar columns elide to identity, so explicit navigation
// (`kind.text().eq(..)`) keeps working unchanged.

/// An entity tag whose first-declared field is scalar — `schema!` emits this.
/// `'static` is load-bearing: `Id<E: 'static>`, and `Col`/`primary` are
/// `&'static`.
pub trait Primary: 'static + Sized {
    type Scalar: Copy;
    type Col: Query<D = Id<Self>, R = Self::Scalar>;
    fn primary() -> &'static Self::Col;
}

/// A relation's VALUE type, and how a comparison on it elides: scalars are the
/// identity; entity ids navigate to their primary column.
pub trait Field: Copy {
    type Scalar: Copy;
    type Elided<Q: Query<R = Self>>: Query<R = Self::Scalar>;
    fn elide<Q: Query<R = Self>>(q: Q) -> Self::Elided<Q>;
}
impl Field for i64 {
    type Scalar = i64;
    type Elided<Q: Query<R = i64>> = Q;
    #[inline(always)]
    fn elide<Q: Query<R = i64>>(q: Q) -> Q {
        q
    }
}
impl Field for f64 {
    type Scalar = f64;
    type Elided<Q: Query<R = f64>> = Q;
    #[inline(always)]
    fn elide<Q: Query<R = f64>>(q: Q) -> Q {
        q
    }
}
impl Field for &'static str {
    type Scalar = &'static str;
    type Elided<Q: Query<R = &'static str>> = Q;
    #[inline(always)]
    fn elide<Q: Query<R = &'static str>>(q: Q) -> Q {
        q
    }
}
// Untyped escape hatch: `usize`-valued columns (the pre-schema loaders) keep
// `.eq` available, identity-elided.
impl Field for usize {
    type Scalar = usize;
    type Elided<Q: Query<R = usize>> = Q;
    #[inline(always)]
    fn elide<Q: Query<R = usize>>(q: Q) -> Q {
        q
    }
}
impl<E: Primary> Field for Id<E> {
    type Scalar = E::Scalar;
    type Elided<Q: Query<R = Id<E>>> = Compose<Q, &'static E::Col>;
    #[inline(always)]
    fn elide<Q: Query<R = Id<E>>>(q: Q) -> Compose<Q, &'static E::Col> {
        Compose {
            a: q,
            b: E::primary(),
        }
    }
}

/// The elided value type a comparison on `T` ends up comparing.
pub type Sc<T> = <ROf<T> as Field>::Scalar;
/// The query a comparison on `T` filters, after primary elision.
pub type Elided<T> = <ROf<T> as Field>::Elided<<T as IntoQuery>::Q>;

pub struct VecRel<R: Copy, D: Dense = usize> {
    pub values: Vec<R>,
    pub _d: PhantomData<D>,
}

impl<R: Copy, D: Dense> VecRel<R, D> {
    pub fn new(values: Vec<R>) -> Self {
        VecRel {
            values,
            _d: PhantomData,
        }
    }
    pub fn n_keys(&self) -> usize {
        self.values.len()
    }
}

pub struct MultiRel<R: Copy + 'static, D: Dense = usize> {
    pub _d: PhantomData<D>,
    /// CSR row offsets, length n+1 (u32: every cached column's value count
    /// fits, and half-width offsets halve the footprint of sparse rows).
    pub offsets: &'static [u32],
    /// All rows' values, concatenated in key order.
    pub values: &'static [R],
}

#[cfg(test)]
impl<R: Copy + Default, D: Dense> VecRel<R, D> {
    pub fn from_pairs(n: usize, pairs: impl IntoIterator<Item = (usize, R)>) -> Self {
        let mut values = vec![R::default(); n];
        for (k, v) in pairs {
            values[k] = v;
        }
        VecRel::new(values)
    }
}

impl<R: Copy + 'static, D: Dense> MultiRel<R, D> {
    /// Wrap existing CSR arrays (the cache loaders' zero-copy path).
    pub fn from_csr(offsets: &'static [u32], values: &'static [R]) -> Self {
        assert!(!offsets.is_empty(), "CSR offsets must have length n+1");
        assert_eq!(*offsets.last().unwrap() as usize, values.len());
        MultiRel {
            offsets,
            values,
            _d: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn n_keys(&self) -> usize {
        self.offsets.len() - 1
    }

    /// Build CSR from a pair stream — small-data constructor for unit
    /// tests (the backing Vecs are leaked). Pairs with `k >= n` are
    /// dropped; per-key value order follows the stream.
    #[cfg(test)]
    pub fn from_pairs(n: usize, pairs: impl IntoIterator<Item = (usize, R)>) -> Self {
        let mut buckets: Vec<Vec<R>> = (0..n).map(|_| Vec::new()).collect();
        for (k, v) in pairs {
            if k < n {
                buckets[k].push(v);
            }
        }
        let mut offsets = Vec::with_capacity(n + 1);
        let mut values = Vec::new();
        offsets.push(0u32);
        for b in &buckets {
            values.extend_from_slice(b);
            offsets.push(values.len() as u32);
        }
        MultiRel {
            offsets: Vec::leak(offsets),
            values: Vec::leak(values),
            _d: PhantomData,
        }
    }

    /// Row slice for key `x` — empty for missing/out-of-universe keys
    /// (`NO_ID` included: it fails the `x < n` check like any other
    /// out-of-range id).
    #[inline(always)]
    fn row(&self, x: usize) -> &'static [R] {
        // `x < len-1`, not `x+1 < len`: x = NO_ID must not overflow.
        if x < self.offsets.len() - 1 {
            &self.values[self.offsets[x] as usize..self.offsets[x + 1] as usize]
        } else {
            &[]
        }
    }
}

impl<R: Copy, D: Dense> Query for VecRel<R, D> {
    type D = D;
    type R = R;
}
impl<R: Copy, D: Dense> Drive for VecRel<R, D> {
    #[inline(always)]
    fn drive<K: FnMut(D, R)>(&self, mut k: K) {
        for (i, &v) in self.values.iter().enumerate() {
            k(D::from_idx(i), v);
        }
    }
}
impl<R: Copy, D: Dense> Probe for VecRel<R, D> {
    #[inline(always)]
    fn probe<K: FnMut(R)>(&self, x: D, mut k: K) {
        if let Some(&v) = self.values.get(x.idx()) {
            k(v);
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(R) -> bool>(&self, x: D, mut k: K) -> bool {
        self.values.get(x.idx()).is_some_and(|&v| k(v))
    }
}

impl<R: Copy, D: Dense> Query for MultiRel<R, D> {
    type D = D;
    type R = R;
}
impl<R: Copy, D: Dense> Drive for MultiRel<R, D> {
    #[inline(always)]
    fn drive<K: FnMut(D, R)>(&self, mut k: K) {
        for (i, w) in self.offsets.windows(2).enumerate() {
            for &v in &self.values[w[0] as usize..w[1] as usize] {
                k(D::from_idx(i), v);
            }
        }
    }
}
impl<R: Copy, D: Dense> Probe for MultiRel<R, D> {
    #[inline(always)]
    fn probe<K: FnMut(R)>(&self, x: D, mut k: K) {
        for &v in self.row(x.idx()) {
            k(v);
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(R) -> bool>(&self, x: D, mut k: K) -> bool {
        self.row(x.idx()).iter().any(|&v| k(v))
    }
}

// ===== Universe — identity relation over the dense entity ids ===========

#[derive(Copy, Clone)]
pub struct Universe<D: Dense = usize> {
    pub n: usize,
    pub _d: PhantomData<D>,
}

impl<D: Dense> Universe<D> {
    pub fn new(n: usize) -> Self {
        Universe { n, _d: PhantomData }
    }
}

impl<D: Dense> Query for Universe<D> {
    type D = D;
    type R = D;
}
impl<D: Dense> Drive for Universe<D> {
    #[inline(always)]
    fn drive<K: FnMut(D, D)>(&self, mut k: K) {
        for i in 0..self.n {
            let d = D::from_idx(i);
            k(d, d);
        }
    }
}
impl<D: Dense> Probe for Universe<D> {
    #[inline(always)]
    fn probe<K: FnMut(D)>(&self, x: D, mut k: K) {
        if x.idx() < self.n {
            k(x);
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(D) -> bool>(&self, x: D, mut k: K) -> bool {
        x.idx() < self.n && k(x)
    }
}

// ===== Ident — the identity ENTITY TABLE `Id<E> → Id<E>` ================
// The `id → row` map for a DENSE entity, where the external id IS the row.

pub struct Ident<E: 'static>(pub PhantomData<E>);
impl<E> Ident<E> {
    #[inline(always)]
    pub fn new() -> Self {
        Ident(PhantomData)
    }
}
impl<E> Default for Ident<E> {
    #[inline(always)]
    fn default() -> Self {
        Ident(PhantomData)
    }
}
impl<E: 'static> Query for Ident<E> {
    type D = Id<E>;
    type R = Id<E>;
}
impl<E: 'static> Probe for Ident<E> {
    #[inline(always)]
    fn probe<K: FnMut(Id<E>)>(&self, x: Id<E>, mut k: K) {
        k(x);
    }
    #[inline(always)]
    fn probe_any<K: FnMut(Id<E>) -> bool>(&self, x: Id<E>, mut k: K) -> bool {
        k(x)
    }
}

// ===== non-dense entities: Key<E> + DictTable =============================
// The general entity table, for an entity whose external ids are NOT a dense
// `0..n`.

/// External, possibly non-dense / non-contiguous id of entity `E` — distinct
/// from `Id<E>` (the dense ROW index). Manual `Copy/Eq/Hash` like `Id<E>`
/// (`derive` would wrongly bound the phantom `E`).
#[repr(transparent)]
pub struct Key<E: 'static>(pub u64, pub PhantomData<E>);
impl<E> Key<E> {
    #[inline(always)]
    pub fn new(k: u64) -> Self {
        Key(k, PhantomData)
    }
}
impl<E> Copy for Key<E> {}
impl<E> Clone for Key<E> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}
impl<E> PartialEq for Key<E> {
    #[inline(always)]
    fn eq(&self, o: &Self) -> bool {
        self.0 == o.0
    }
}
impl<E> Eq for Key<E> {}
impl<E> Hash for Key<E> {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.0.hash(h)
    }
}
impl<E> std::fmt::Debug for Key<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Key({})", self.0)
    }
}

pub struct DictTable<E: 'static> {
    map: HashMap<Key<E>, Id<E>>,
}
impl<E: 'static> DictTable<E> {
    pub fn from_keys(keys: &[u64]) -> Self {
        DictTable {
            map: keys
                .iter()
                .enumerate()
                .map(|(r, &k)| (Key::new(k), Id::from_idx(r)))
                .collect(),
        }
    }
    /// Build from the entity's i64 external-id column (`row → external id`),
    /// the inverse `external id → row`. Used by the schema macro at load.
    pub fn from_i64(keys: &[i64]) -> Self {
        DictTable {
            map: keys
                .iter()
                .enumerate()
                .map(|(r, &k)| (Key::new(k as u64), Id::from_idx(r)))
                .collect(),
        }
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}
impl<E: 'static> Query for DictTable<E> {
    type D = Key<E>;
    type R = Id<E>;
}
impl<E: 'static> Probe for DictTable<E> {
    #[inline]
    fn probe<K: FnMut(Id<E>)>(&self, x: Key<E>, mut k: K) {
        if let Some(&r) = self.map.get(&x) {
            k(r);
        }
    }
    #[inline]
    fn probe_any<K: FnMut(Id<E>) -> bool>(&self, x: Key<E>, mut k: K) -> bool {
        self.map.get(&x).is_some_and(|&r| k(r))
    }
}

/// How an entity is ADDRESSED:
///   - DENSE  (default) — `Fk = Id<Self>`, `Table = Ident` (inlines away).
///   - NON-DENSE (`dict`)— `Fk = Key<Self>`, `Table = &'static DictTable`.
pub trait EntityKind: Sized + 'static {
    type Fk: Copy + Eq + Hash;
    type Table: Probe<D = Self::Fk, R = Id<Self>>;
    fn table() -> Self::Table;
}

// ====================
// COMBINATOR NODES
// ====================

// ===== Compose ===================
// Mode rule: the rhs is always probed; the lhs carries the Compose's mode.

pub struct Compose<A, B> {
    pub a: A,
    pub b: B,
}

impl<A: Query, B: Query<D = A::R>> Query for Compose<A, B> {
    type D = A::D;
    type R = B::R;
}
impl<A: Drive, B: Probe<D = A::R>> Drive for Compose<A, B> {
    #[inline(always)]
    fn drive<K: FnMut(A::D, B::R)>(&self, mut k: K) {
        self.a.drive(|x, m| self.b.probe(m, |r| k(x, r)));
    }
}
impl<A: Probe, B: Probe<D = A::R>> Probe for Compose<A, B> {
    #[inline(always)]
    fn probe<K: FnMut(B::R)>(&self, x: A::D, mut k: K) {
        self.a.probe(x, |m| self.b.probe(m, |r| k(r)));
    }
    #[inline(always)]
    fn probe_any<K: FnMut(B::R) -> bool>(&self, x: A::D, mut k: K) -> bool {
        self.a.probe_any(x, |m| self.b.probe_any(m, |r| k(r)))
    }
}

// ===== Filter =============================

pub struct Filter<A, F> {
    pub a: A,
    pub p: F,
}

impl<A: Query, F> Query for Filter<A, F> {
    type D = A::D;
    type R = A::R;
}
impl<A: Drive, F: Fn(A::R) -> bool> Drive for Filter<A, F> {
    #[inline(always)]
    fn drive<K: FnMut(A::D, A::R)>(&self, mut k: K) {
        self.a.drive(|x, v| {
            if (self.p)(v) {
                k(x, v);
            }
        });
    }
}
impl<A: Probe, F: Fn(A::R) -> bool> Probe for Filter<A, F> {
    #[inline(always)]
    fn probe<K: FnMut(A::R)>(&self, x: A::D, mut k: K) {
        self.a.probe(x, |v| {
            if (self.p)(v) {
                k(v);
            }
        });
    }
    #[inline(always)]
    fn probe_any<K: FnMut(A::R) -> bool>(&self, x: A::D, mut k: K) -> bool {
        self.a.probe_any(x, |v| (self.p)(v) && k(v))
    }
}

// ===== Restrict =========================
// Keeps a's pairs where the value is a `member` of b;

pub struct Restrict<A, B> {
    pub a: A,
    pub b: B,
}

impl<A: Query, B: Query<D = A::R>> Query for Restrict<A, B> {
    type D = A::D;
    type R = A::R;
}
impl<A: Drive, B: Member<D = A::R>> Drive for Restrict<A, B> {
    #[inline(always)]
    fn drive<K: FnMut(A::D, A::R)>(&self, mut k: K) {
        self.a.drive(|x, v| {
            if self.b.member(v) {
                k(x, v);
            }
        });
    }
}
impl<A: Probe, B: Probe<D = A::R>> Probe for Restrict<A, B> {
    #[inline(always)]
    fn probe<K: FnMut(A::R)>(&self, x: A::D, mut k: K) {
        self.a.probe(x, |v| {
            if self.b.member(v) {
                k(v);
            }
        });
    }
    #[inline(always)]
    fn probe_any<K: FnMut(A::R) -> bool>(&self, x: A::D, mut k: K) -> bool {
        self.a.probe_any(x, |v| self.b.member(v) && k(v))
    }
}

// ===== Diff / Disj / Union — set algebra ================================

pub struct Diff<A, B> {
    pub a: A,
    pub b: B,
}
impl<A: Query, B: Query<D = A::D>> Query for Diff<A, B> {
    type D = A::D;
    type R = A::R;
}
impl<A: Drive, B: Probe<D = A::D>> Drive for Diff<A, B> {
    #[inline(always)]
    fn drive<K: FnMut(A::D, A::R)>(&self, mut k: K) {
        self.a.drive(|x, v| {
            if !self.b.member(x) {
                k(x, v);
            }
        });
    }
}
impl<A: Probe, B: Probe<D = A::D>> Probe for Diff<A, B> {
    #[inline(always)]
    fn probe<K: FnMut(A::R)>(&self, x: A::D, k: K) {
        if !self.b.member(x) {
            self.a.probe(x, k);
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(A::R) -> bool>(&self, x: A::D, k: K) -> bool {
        !self.b.member(x) && self.a.probe_any(x, k)
    }
}

pub struct Disj<A, B> {
    pub a: A,
    pub b: B,
}
impl<A: Query, B: Query<D = A::D>> Query for Disj<A, B> {
    type D = A::D;
    type R = (A::R, B::R);
}
impl<A: Member, B: Member<D = A::D>> Member for Disj<A, B> {
    #[inline(always)]
    fn member(&self, x: A::D) -> bool {
        self.a.member(x) || self.b.member(x)
    }
}

pub struct Union<A, B> {
    pub a: A,
    pub b: B,
}
impl<A: Query, B: Query<D = A::D, R = A::R>> Query for Union<A, B> {
    type D = A::D;
    type R = A::R;
}
impl<A: Drive, B: Drive<D = A::D, R = A::R>> Drive for Union<A, B> {
    #[inline(always)]
    fn drive<K: FnMut(A::D, A::R)>(&self, mut k: K) {
        self.a.drive(&mut k);
        self.b.drive(k);
    }
}

// ===== Prod ===================

pub struct Prod<A, B> {
    pub a: A,
    pub b: B,
}

impl<A: Query, B: Query<D = A::D>> Query for Prod<A, B> {
    type D = A::D;
    type R = (A::R, B::R);
}
impl<A: Drive, B: Probe<D = A::D>> Drive for Prod<A, B> {
    #[inline(always)]
    fn drive<K: FnMut(A::D, (A::R, B::R))>(&self, mut k: K) {
        self.a.drive(|x, a| self.b.probe(x, |b| k(x, (a, b))));
    }
}
impl<A: Probe, B: Probe<D = A::D>> Probe for Prod<A, B> {
    #[inline(always)]
    fn probe<K: FnMut((A::R, B::R))>(&self, x: A::D, mut k: K) {
        self.a.probe(x, |a| self.b.probe(x, |b| k((a, b))));
    }
    #[inline(always)]
    fn probe_any<K: FnMut((A::R, B::R)) -> bool>(&self, x: A::D, mut k: K) -> bool {
        self.a.probe_any(x, |a| self.b.probe_any(x, |b| k((a, b))))
    }
}

// ===== InvStream — `q'` in drive position =========

pub struct InvStream<Q> {
    pub q: Q,
}

impl<Q: Query> Query for InvStream<Q>
where
    Q::R: Eq + Hash,
{
    type D = Q::R;
    type R = Q::D;
}
impl<Q: Drive> Drive for InvStream<Q>
where
    Q::R: Eq + Hash,
{
    #[inline(always)]
    fn drive<K: FnMut(Q::R, Q::D)>(&self, mut k: K) {
        self.q.drive(|d, r| k(r, d));
    }
}

// ===== FromQuery / collect — explicit materialization ======================
// The relation mirror of `FromIterator`/`Iterator::collect`: `q.collect()`
// drives `q` once into the physical structure named by the target type
// (turbofish or `let` annotation).

pub trait FromQuery<Q: Drive>: Sized {
    fn from_rel(q: Q) -> Self;
}

// ===== HashIdx  ===========================

pub struct HashIdx<K: Copy + Eq + Hash, V: Copy> {
    pub idx: HashMap<K, SVec<V>>,
}

/// Forward index: bucket q's values by key.
impl<Q: Drive> FromQuery<Q> for HashIdx<Q::D, Q::R> {
    fn from_rel(q: Q) -> Self {
        let mut m: HashMap<Q::D, SVec<Q::R>> = HashMap::new();
        q.drive(|d, r| m.entry(d).or_default().push(r));
        HashIdx { idx: m }
    }
}

impl<K: Copy + Eq + Hash, V: Copy> Query for HashIdx<K, V> {
    type D = K;
    type R = V;
}
impl<K: Copy + Eq + Hash, V: Copy> Probe for HashIdx<K, V> {
    #[inline(always)]
    fn probe<F: FnMut(V)>(&self, x: K, mut k: F) {
        if let Some(vs) = self.idx.get(&x) {
            for &v in vs {
                k(v);
            }
        }
    }
    #[inline(always)]
    fn probe_any<F: FnMut(V) -> bool>(&self, x: K, mut k: F) -> bool {
        match self.idx.get(&x) {
            Some(vs) => vs.iter().any(|&v| k(v)),
            None => false,
        }
    }
}

// ===== MatSet — materialized membership set ====

pub struct MatSet<D: Copy + Eq + Hash> {
    pub set: HashSet<D>,
}

impl<Q: Drive> FromQuery<Q> for MatSet<Q::R>
where
    Q::R: Eq + Hash,
{
    fn from_rel(q: Q) -> Self {
        let mut set = HashSet::new();
        q.drive(|_, v| {
            set.insert(v);
        });
        MatSet { set }
    }
}
impl<D: Copy + Eq + Hash> Query for MatSet<D> {
    type D = D;
    type R = D;
}
impl<D: Copy + Eq + Hash> Probe for MatSet<D> {
    #[inline(always)]
    fn probe<K: FnMut(D)>(&self, x: D, mut k: K) {
        if self.set.contains(&x) {
            k(x);
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(D) -> bool>(&self, x: D, mut k: K) -> bool {
        self.set.contains(&x) && k(x)
    }
}

// ===== Bitset — `Vec<u64>`-backed dense identity relation ===============
//
// Drop-in replacement for `MatSet` when the membership domain is a
// dense `0..n`: trades the HashSet's hash+probe for one bit-test.

pub struct Bitset<D: Dense = usize> {
    pub bs: Vec<u64>,
    pub n: usize,
    pub _d: PhantomData<D>,
}

impl<D: Dense> Bitset<D> {
    pub fn empty<U: IntoQuery>(u: U) -> Self
    where
        U::Q: UnivSize<D>,
    {
        let n = u.iq().univ_n();
        Bitset {
            bs: vec![0u64; n.div_ceil(64)],
            n,
            _d: PhantomData,
        }
    }

    /// A bitset over `u`, driven from `q`: set a bit at each emitted VALUE.
    pub fn over<U: IntoQuery, Q: IntoQuery>(u: U, q: Q) -> Self
    where
        U::Q: UnivSize<D>,
        Q::Q: Drive<R = D>,
    {
        let mut b = Self::empty(u);
        q.iq().drive(|_, c| b.set(c));
        b
    }
    #[inline]
    pub fn set(&mut self, x: D) {
        let i = x.idx();
        if i < self.n {
            self.bs[i / 64] |= 1u64 << (i % 64);
        }
    }

    /// Direct bit test. INHERENT method, deliberately shadowing trait
    /// `member` to avoid infinite mutual recursion.
    #[inline(always)]
    pub fn member(&self, x: D) -> bool {
        let i = x.idx();
        i < self.n && (self.bs[i / 64] >> (i % 64)) & 1 != 0
    }
}

impl<D: Dense> Query for Bitset<D> {
    type D = D;
    type R = D;
}
impl<D: Dense> Drive for Bitset<D> {
    #[inline]
    fn drive<K: FnMut(D, D)>(&self, mut k: K) {
        for (wi, &w) in self.bs.iter().enumerate() {
            let mut w = w;
            while w != 0 {
                let b = w.trailing_zeros() as usize;
                let x = D::from_idx(wi * 64 + b);
                k(x, x);
                w &= w - 1;
            }
        }
    }
}
impl<D: Dense> Probe for Bitset<D> {
    #[inline]
    fn probe<K: FnMut(D)>(&self, x: D, mut k: K) {
        if self.member(x) {
            k(x);
        }
    }
    #[inline]
    fn probe_any<K: FnMut(D) -> bool>(&self, x: D, mut k: K) -> bool {
        self.member(x) && k(x)
    }
}

impl<D: Dense> Bitset<D> {
    /// Validity mask for a SPARSE entity: bit set for each slot `0..fk.len()`
    /// whose foreign key is a real target (`!= NONE`).
    pub fn validity<T: Dense>(fk: &[T]) -> Self {
        let mut b = Self::empty(Universe::<D>::new(fk.len()));
        for (i, &v) in fk.iter().enumerate() {
            if v != T::NONE {
                b.set(D::from_idx(i));
            }
        }
        b
    }
}

// ===== SparseUniverse — `Universe` with a validity mask =================
// A dense id space `0..n` that carries holes (e.g. `orders` over sparse
// orderkeys).
//
// It is a SEPARATE TYPE from `Universe` on purpose: drive dispatch is at
// compile time, so the dense `Universe::drive` loop is untouched — no shared
// branch to de-optimise its closure inlining.

pub struct SparseUniverse<D: Dense> {
    pub n: usize,
    pub valid: &'static Bitset<D>,
}
impl<D: Dense> SparseUniverse<D> {
    pub fn new(n: usize, valid: &'static Bitset<D>) -> Self {
        SparseUniverse { n, valid }
    }
}
impl<D: Dense> Query for SparseUniverse<D> {
    type D = D;
    type R = D;
}
impl<D: Dense> Drive for SparseUniverse<D> {
    #[inline(always)]
    fn drive<K: FnMut(D, D)>(&self, k: K) {
        self.valid.drive(k);
    }
}
impl<D: Dense> Probe for SparseUniverse<D> {
    #[inline(always)]
    fn probe<K: FnMut(D)>(&self, x: D, mut k: K) {
        if x.idx() < self.n {
            k(x);
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(D) -> bool>(&self, x: D, mut k: K) -> bool {
        x.idx() < self.n && k(x)
    }
}

pub trait UnivSize<D: Dense> {
    fn univ_n(&self) -> usize;
}
impl<D: Dense> UnivSize<D> for Universe<D> {
    #[inline]
    fn univ_n(&self) -> usize {
        self.n
    }
}
impl<D: Dense> UnivSize<D> for SparseUniverse<D> {
    #[inline]
    fn univ_n(&self) -> usize {
        self.n
    }
}

// ===== GroupBy ==============

pub struct GroupBy<S, R> {
    pub set: S,
    pub key: R,
}

impl<S: Query, R: Query<D = S::R>> Query for GroupBy<S, R>
where
    R::R: Eq + Hash,
{
    type D = R::R;
    type R = S::R;
}
impl<S: Drive, R: Probe<D = S::R>> Drive for GroupBy<S, R>
where
    R::R: Eq + Hash,
{
    #[inline(always)]
    fn drive<K: FnMut(R::R, S::R)>(&self, mut k: K) {
        self.set.drive(|_, x| self.key.probe(x, |rk| k(rk, x)));
    }
}

// ===== Fold  ==================

pub struct Fold<D: Copy + Eq + Hash, S: Copy> {
    pub cache: HashMap<D, S>,
}

impl<D: Copy + Eq + Hash, S: Copy> Fold<D, S> {
    pub fn build<Q, OP>(q: Q, init: S, op: OP) -> Self
    where
        Q: Drive<D = D>,
        OP: Fn(S, Q::R) -> S,
    {
        let mut m: HashMap<D, S> = HashMap::new();
        q.drive(|d, v| {
            let s = m.entry(d).or_insert(init);
            *s = op(*s, v);
        });
        Fold { cache: m }
    }

    /// Whole-multiset reduce (Julia's BufFold — julia-engine plan.jl
    /// `build_buffold`): buffer every group into an `SVec`, then compute
    /// each cache entry as `f(vs)` over the whole group. For reducers that
    /// don't fit foldl's `(S, R) -> S` shape — count-distinct, median, … —
    /// where `build` is the per-key foldl.
    pub fn build_buf<Q, F>(q: Q, f: F) -> Self
    where
        Q: Drive<D = D>,
        F: Fn(SVec<Q::R>) -> S,
    {
        let mut buf: HashMap<D, SVec<Q::R>> = HashMap::new();
        q.drive(|d, v| buf.entry(d).or_default().push(v));
        Fold {
            cache: buf.into_iter().map(|(d, vs)| (d, f(vs))).collect(),
        }
    }
}

impl<D: Copy + Eq + Hash, S: Copy> Query for Fold<D, S> {
    type D = D;
    type R = S;
}
impl<D: Copy + Eq + Hash, S: Copy> Drive for Fold<D, S> {
    #[inline(always)]
    fn drive<K: FnMut(D, S)>(&self, mut k: K) {
        for (&d, &s) in &self.cache {
            k(d, s);
        }
    }
}
impl<D: Copy + Eq + Hash, S: Copy> Probe for Fold<D, S> {
    #[inline(always)]
    fn probe<K: FnMut(S)>(&self, x: D, mut k: K) {
        if let Some(&s) = self.cache.get(&x) {
            k(s);
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(S) -> bool>(&self, x: D, mut k: K) -> bool {
        match self.cache.get(&x) {
            Some(&s) => k(s),
            None => false,
        }
    }
}

// ===== DenseFold ==================
//
// Drop-in replacement for `Fold` when `D = usize` and the key range is a
// known, dense `0..n`. Backing store is `Vec<S>` (one slot per key) plus a
// parallel `Vec<bool>` presence map. Avoids HashMap probe + entry alloc on
// every reduce step; for Q1 (≤6 group keys via packed byte index), Q2 / Q20
// (per-part), Q18 (per-order), the gain is ~5-10× over `Fold`.

pub struct DenseFold<S: Copy, D: Dense = usize> {
    pub vals: Vec<S>,
    pub seen: Vec<bool>,
    /// When set, `drive`/`probe` emit the seeded `init` for keys that never
    /// matched (left-outer-join aggregate); otherwise only `seen` keys.
    pub emit_all: bool,
    pub _d: PhantomData<D>,
}

impl<S: Copy, D: Dense> DenseFold<S, D> {
    pub fn build<Q, OP>(q: Q, n: usize, init: S, op: OP) -> Self
    where
        Q: Drive<D = D>,
        OP: Fn(S, Q::R) -> S,
    {
        Self::build_with(q, n, init, op, false)
    }

    /// Like `build`, but the result emits the identity `init` for keys that
    /// never matched — the left-outer-join aggregate. Correct ONLY when
    /// `0..n` is exactly the key universe (every slot a real key); with a
    /// sparse/packed key space this fabricates rows for absent keys.
    pub fn build_outer<Q, OP>(q: Q, n: usize, init: S, op: OP) -> Self
    where
        Q: Drive<D = D>,
        OP: Fn(S, Q::R) -> S,
    {
        Self::build_with(q, n, init, op, true)
    }

    fn build_with<Q, OP>(q: Q, n: usize, init: S, op: OP, emit_all: bool) -> Self
    where
        Q: Drive<D = D>,
        OP: Fn(S, Q::R) -> S,
    {
        let mut vals = vec![init; n];
        let mut seen = vec![false; n];
        q.drive(|d, v| {
            if let Some(s) = vals.get_mut(d.idx()) {
                *s = op(*s, v);
                seen[d.idx()] = true;
            }
        });
        DenseFold {
            vals,
            seen,
            emit_all,
            _d: PhantomData,
        }
    }
}

impl<S: Copy, D: Dense> Query for DenseFold<S, D> {
    type D = D;
    type R = S;
}
impl<S: Copy, D: Dense> Drive for DenseFold<S, D> {
    #[inline(always)]
    fn drive<K: FnMut(D, S)>(&self, mut k: K) {
        for (i, (&v, &seen)) in self.vals.iter().zip(&self.seen).enumerate() {
            if self.emit_all || seen {
                k(D::from_idx(i), v);
            }
        }
    }
}
impl<S: Copy, D: Dense> Probe for DenseFold<S, D> {
    #[inline(always)]
    fn probe<K: FnMut(S)>(&self, x: D, mut k: K) {
        if let Some(&v) = self.vals.get(x.idx()) {
            if self.emit_all || self.seen[x.idx()] {
                k(v);
            }
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(S) -> bool>(&self, x: D, mut k: K) -> bool {
        self.vals
            .get(x.idx())
            .is_some_and(|&v| (self.emit_all || self.seen[x.idx()]) && k(v))
    }
}

// ===== Map =====================================

pub struct Map<Q, F, S: Copy> {
    pub q: Q,
    pub f: F,
    _phantom: std::marker::PhantomData<S>,
}

impl<Q: Query, F: Fn(Q::R) -> S, S: Copy> Map<Q, F, S> {
    pub fn new(q: Q, f: F) -> Self {
        Map {
            q,
            f,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<Q: Query, F: Fn(Q::R) -> S, S: Copy> Query for Map<Q, F, S> {
    type D = Q::D;
    type R = S;
}
impl<Q: Drive, F: Fn(Q::R) -> S, S: Copy> Drive for Map<Q, F, S> {
    #[inline(always)]
    fn drive<K: FnMut(Q::D, S)>(&self, mut k: K) {
        self.q.drive(|d, v| k(d, (self.f)(v)));
    }
}
impl<Q: Probe, F: Fn(Q::R) -> S, S: Copy> Probe for Map<Q, F, S> {
    #[inline(always)]
    fn probe<K: FnMut(S)>(&self, x: Q::D, mut k: K) {
        self.q.probe(x, |v| k((self.f)(v)));
    }
    #[inline(always)]
    fn probe_any<K: FnMut(S) -> bool>(&self, x: Q::D, mut k: K) -> bool {
        self.q.probe_any(x, |v| k((self.f)(v)))
    }
}

// ==================
// OPERATORS
// ==================

// Constructors are mode-agnostic (they just build the node; the node's
// trait impls carry the mode bounds) EXCEPT the eager physical nodes, whose
// constructors drive their input right here — those require `Self: Drive`
// and consume their input, exactly like prela's `build_*` inside `prepare`.

pub trait QueryExt: IntoQuery + Sized {
    #[inline(always)]
    fn select<B: IntoQuery>(self, b: B) -> Compose<Self::Q, B::Q>
    where
        B::Q: Query<D = ROf<Self>>,
    {
        Compose {
            a: self.iq(),
            b: b.iq(),
        }
    }

    #[inline(always)]
    fn inv(self) -> InvStream<Self::Q>
    where
        ROf<Self>: Eq + Hash,
    {
        InvStream { q: self.iq() }
    }

    #[inline(always)]
    fn and<B: IntoQuery>(self, b: B) -> Prod<Self::Q, B::Q>
    where
        B::Q: Query<D = DOf<Self>>,
    {
        Prod {
            a: self.iq(),
            b: b.iq(),
        }
    }

    #[inline(always)]
    fn or<B: IntoQuery>(self, b: B) -> Disj<Self::Q, B::Q>
    where
        B::Q: Query<D = DOf<Self>>,
    {
        Disj {
            a: self.iq(),
            b: b.iq(),
        }
    }

    #[inline(always)]
    fn minus<B: IntoQuery>(self, b: B) -> Diff<Self::Q, B::Q>
    where
        B::Q: Query<D = DOf<Self>>,
    {
        Diff {
            a: self.iq(),
            b: b.iq(),
        }
    }

    #[inline(always)]
    fn union<B: IntoQuery>(self, b: B) -> Union<Self::Q, B::Q>
    where
        B::Q: Query<D = DOf<Self>, R = ROf<Self>>,
    {
        Union {
            a: self.iq(),
            b: b.iq(),
        }
    }

    // Predicate filters — all elide the primary field (scalar = identity)
    // then compare; see the `Field`/`Primary` traits above.
    #[inline(always)]
    fn eq(self, v: Sc<Self>) -> Filter<Elided<Self>, impl Fn(Sc<Self>) -> bool>
    where
        ROf<Self>: Field,
        Sc<Self>: PartialEq,
    {
        Filter {
            a: <ROf<Self> as Field>::elide(self.iq()),
            p: move |x| x == v,
        }
    }
    #[inline(always)]
    fn ne(self, v: Sc<Self>) -> Filter<Elided<Self>, impl Fn(Sc<Self>) -> bool>
    where
        ROf<Self>: Field,
        Sc<Self>: PartialEq,
    {
        Filter {
            a: <ROf<Self> as Field>::elide(self.iq()),
            p: move |x| x != v,
        }
    }
    #[inline(always)]
    fn gt(self, v: Sc<Self>) -> Filter<Elided<Self>, impl Fn(Sc<Self>) -> bool>
    where
        ROf<Self>: Field,
        Sc<Self>: PartialOrd,
    {
        Filter {
            a: <ROf<Self> as Field>::elide(self.iq()),
            p: move |x| x > v,
        }
    }
    #[inline(always)]
    fn lt(self, v: Sc<Self>) -> Filter<Elided<Self>, impl Fn(Sc<Self>) -> bool>
    where
        ROf<Self>: Field,
        Sc<Self>: PartialOrd,
    {
        Filter {
            a: <ROf<Self> as Field>::elide(self.iq()),
            p: move |x| x < v,
        }
    }
    #[inline(always)]
    fn ge(self, v: Sc<Self>) -> Filter<Elided<Self>, impl Fn(Sc<Self>) -> bool>
    where
        ROf<Self>: Field,
        Sc<Self>: PartialOrd,
    {
        Filter {
            a: <ROf<Self> as Field>::elide(self.iq()),
            p: move |x| x >= v,
        }
    }
    #[inline(always)]
    fn le(self, v: Sc<Self>) -> Filter<Elided<Self>, impl Fn(Sc<Self>) -> bool>
    where
        ROf<Self>: Field,
        Sc<Self>: PartialOrd,
    {
        Filter {
            a: <ROf<Self> as Field>::elide(self.iq()),
            p: move |x| x <= v,
        }
    }
    #[inline(always)]
    fn in_v(self, vs: Vec<Sc<Self>>) -> Filter<Elided<Self>, impl Fn(Sc<Self>) -> bool>
    where
        ROf<Self>: Field,
        Sc<Self>: PartialEq,
    {
        Filter {
            a: <ROf<Self> as Field>::elide(self.iq()),
            p: move |x| vs.iter().any(|&v| v == x),
        }
    }
    /// `in_v` over any `IntoIterator`
    #[inline(always)]
    fn is_in<I: IntoIterator<Item = Sc<Self>>>(
        self,
        vs: I,
    ) -> Filter<Elided<Self>, impl Fn(Sc<Self>) -> bool>
    where
        ROf<Self>: Field,
        Sc<Self>: PartialEq,
    {
        let vs: Vec<Sc<Self>> = vs.into_iter().collect();
        Filter {
            a: <ROf<Self> as Field>::elide(self.iq()),
            p: move |x| vs.iter().any(|&v| v == x),
        }
    }

    #[inline(always)]
    fn with<S: IntoQuery>(self, s: S) -> Restrict<Self::Q, S::Q>
    where
        S::Q: Probe<D = ROf<Self>>,
    {
        Restrict {
            a: self.iq(),
            b: s.iq(),
        }
    }

    #[inline(always)]
    fn rx(self, re: &str) -> Filter<Elided<Self>, impl Fn(&'static str) -> bool>
    where
        ROf<Self>: Field<Scalar = &'static str>,
    {
        let re = Regex::new(re).unwrap();
        Filter {
            a: <ROf<Self> as Field>::elide(self.iq()),
            p: move |s| re.is_match(s),
        }
    }

    #[inline(always)]
    fn nrx(self, re: &str) -> Filter<Elided<Self>, impl Fn(&'static str) -> bool>
    where
        ROf<Self>: Field<Scalar = &'static str>,
    {
        let re = Regex::new(re).unwrap();
        Filter {
            a: <ROf<Self> as Field>::elide(self.iq()),
            p: move |s| !re.is_match(s),
        }
    }

    #[inline(always)]
    fn filt<F: Fn(ROf<Self>) -> bool>(self, f: F) -> Filter<Self::Q, F> {
        Filter { a: self.iq(), p: f }
    }

    /// Half-open range `[lo, hi)` — Julia `during(lo, hi)`.
    #[inline(always)]
    fn during(self, lo: Sc<Self>, hi: Sc<Self>) -> Filter<Elided<Self>, impl Fn(Sc<Self>) -> bool>
    where
        ROf<Self>: Field,
        Sc<Self>: PartialOrd,
    {
        Filter {
            a: <ROf<Self> as Field>::elide(self.iq()),
            p: move |x| x >= lo && x < hi,
        }
    }

    /// Closed range `[lo, hi]` — Julia `lo..hi`.
    #[inline(always)]
    fn between(self, lo: Sc<Self>, hi: Sc<Self>) -> Filter<Elided<Self>, impl Fn(Sc<Self>) -> bool>
    where
        ROf<Self>: Field,
        Sc<Self>: PartialOrd,
    {
        Filter {
            a: <ROf<Self> as Field>::elide(self.iq()),
            p: move |x| x >= lo && x <= hi,
        }
    }

    /// Materialize into the physical structure named by
    /// the target type
    #[inline(always)]
    fn collect<T: FromQuery<Self::Q>>(self) -> T
    where
        Self::Q: Drive,
    {
        T::from_rel(self.iq())
    }

    #[inline(always)]
    fn group_by<R: IntoQuery>(self, key: R) -> GroupBy<Self::Q, R::Q>
    where
        R::Q: Query<D = ROf<Self>>,
        ROf<R>: Eq + Hash,
    {
        GroupBy {
            set: self.iq(),
            key: key.iq(),
        }
    }

    #[inline(always)]
    fn fold<OP: Fn(S, ROf<Self>) -> S, S: Copy>(self, init: S, op: OP) -> Fold<DOf<Self>, S>
    where
        Self::Q: Drive,
    {
        Fold::build(self.iq(), init, op)
    }

    #[inline(always)]
    fn buf_fold<F: Fn(SVec<ROf<Self>>) -> S, S: Copy>(self, f: F) -> Fold<DOf<Self>, S>
    where
        Self::Q: Drive,
    {
        Fold::build_buf(self.iq(), f)
    }

    #[inline(always)]
    fn dense_fold<OP: Fn(S, ROf<Self>) -> S, S: Copy>(
        self,
        n: usize,
        init: S,
        op: OP,
    ) -> DenseFold<S, DOf<Self>>
    where
        Self::Q: Drive,
        DOf<Self>: Dense,
    {
        DenseFold::build(self.iq(), n, init, op)
    }

    /// Left-outer-join aggregate: like `dense_fold`, but every key in `0..n`
    /// is emitted.
    #[inline(always)]
    fn dense_fold_outer<OP: Fn(S, ROf<Self>) -> S, S: Copy>(
        self,
        n: usize,
        init: S,
        op: OP,
    ) -> DenseFold<S, DOf<Self>>
    where
        Self::Q: Drive,
        DOf<Self>: Dense,
    {
        DenseFold::build_outer(self.iq(), n, init, op)
    }

    /// Count-distinct — the `length ∘ unique` instance of `.buf_fold`. The
    /// closure sorts + dedups the per-key SVec on finalization — much
    /// faster than a HashSet per group for the typical small-group case.
    #[inline(always)]
    fn count_distinct(self) -> Fold<DOf<Self>, i64>
    where
        Self::Q: Drive,
        ROf<Self>: Ord,
    {
        self.buf_fold(|mut vs| {
            vs.sort_unstable();
            vs.dedup();
            vs.len() as i64
        })
    }

    #[inline(always)]
    fn map<F: Fn(ROf<Self>) -> S, S: Copy>(self, f: F) -> Map<Self::Q, F, S> {
        Map::new(self.iq(), f)
    }

    #[inline(always)]
    fn unwrap_fold<OP: Fn(S, ROf<Self>) -> S, S: Copy>(self, init: S, op: OP) -> S
    where
        Self::Q: Drive,
    {
        let mut acc = init;
        self.iq().drive(|_, v| acc = op(acc, v));
        acc
    }
}
impl<T: IntoQuery> QueryExt for T {}

// ===== tests — tiny inline data, every node in every mode ===============

#[cfg(test)]
mod tests {
    use super::*;

    // films: 0 → 10, 1 → 20, 2 → 30 (VecRel); cast: 0 → {7, 8}, 2 → {7} (MultiRel)
    // Values are id-typed (usize) so they can feed compose/restrict domains.
    fn films() -> VecRel<usize> {
        VecRel::from_pairs(3, [(0, 10), (1, 20), (2, 30)])
    }
    fn cast() -> MultiRel<usize> {
        MultiRel::from_pairs(3, [(0, 7), (0, 8), (2, 7)])
    }

    fn drive_all<Q: Drive>(q: &Q) -> Vec<(Q::D, Q::R)>
    where
        Q::D: Ord,
        Q::R: Ord,
    {
        let mut v = Vec::new();
        q.drive(|d, r| v.push((d, r)));
        v.sort();
        v
    }

    #[test]
    fn leaves() {
        assert_eq!(drive_all(&films()), vec![(0, 10), (1, 20), (2, 30)]);
        assert_eq!(drive_all(&cast()), vec![(0, 7), (0, 8), (2, 7)]);
        let f = films();
        let mut got = Vec::new();
        f.probe(1, |v| got.push(v));
        assert_eq!(got, vec![20]);
        assert!(cast().probe_any(2, |_| true) && !cast().probe_any(1, |_| true));
        assert!(!f.probe_any(NO_ID, |_| true) && !f.probe_any(3, |_| true));
    }

    #[test]
    fn compose_filter_prod() {
        let f = films();
        let c = cast();
        // cast ∘ (films probed at cast values)? — compose cast: i64→i64 with films
        assert_eq!(drive_all(&(&c).select(&f)), vec![]); // cast values 7,8 not film keys <3
        assert_eq!(drive_all(&(&f).filt(|v| v > 15)), vec![(1, 20), (2, 30)]);
        let u = Universe::new(2);
        assert_eq!(drive_all(&u.select(&f)), vec![(0, 10), (1, 20)]);
        assert_eq!(
            drive_all(&(&f).and(&f)),
            vec![(0, (10, 10)), (1, (20, 20)), (2, (30, 30))]
        );
    }

    #[test]
    fn inv_stream() {
        let f = films();
        assert_eq!(drive_all(&(&f).inv()), vec![(10, 0), (20, 1), (30, 2)]);
    }

    #[test]
    fn collect_hash_idx() {
        let f = films();
        let idx = (&f).filt(|v| v > 10).collect::<HashIdx<_, _>>();
        let mut got = Vec::new();
        idx.probe(2, |v| got.push(v));
        assert_eq!(got, vec![30]);
        assert!(!idx.probe_any(99, |_| true));
    }

    #[test]
    fn group_by_and_folds() {
        let f = films();
        let c = cast();
        let fs = Universe::new(3); // the film entity set (receiver is row-valued)
        // group films by cast-person (`c.inv().with(fs.inv())`): the film
        // SET is driven, cast probed for the key — emits (person, film-row);
        // film values come by navigation after the grouping.
        let grouped = fs.group_by(&c).select(&f);
        assert_eq!(drive_all(&grouped), vec![(7, 10), (7, 30), (8, 10)]);
        // fold straight off the grouped set: count films per person
        let counts = fs.group_by(&c).fold(0i64, |a, _| a + 1);
        assert_eq!(drive_all(&counts), vec![(7, 2), (8, 1)]);
        // dense fold over person ids 0..9
        let dcounts = fs.group_by(&c).dense_fold(9, 0i64, |a, _| a + 1);
        assert_eq!(drive_all(&dcounts), vec![(7, 2), (8, 1)]);
        // buf_fold: whole-group reduce the foldl shape can't express —
        // person 7 saw film values {10, 30} (range 20), person 8 {10}
        let range = fs.group_by(&c).select(&f).buf_fold(|vs| {
            let mn = *vs.iter().min().unwrap();
            let mx = *vs.iter().max().unwrap();
            (mx - mn) as i64
        });
        assert_eq!(drive_all(&range), vec![(7, 20), (8, 0)]);
        // median via buf_fold (an order-statistic — needs the whole group)
        let med = fs.group_by(&c).select(&f).buf_fold(|mut vs| {
            vs.sort_unstable();
            vs[vs.len() / 2]
        });
        assert_eq!(drive_all(&med), vec![(7, 30), (8, 10)]);
        // count_distinct = buf_fold's `length ∘ unique` instance; the
        // duplicate (7, 10) row collapses
        let cd = fs
            .group_by(&c)
            .select(&f)
            .union(fs.group_by(&c).select(&f).filt(|v| v == 10))
            .count_distinct();
        assert_eq!(drive_all(&cd), vec![(7, 2), (8, 1)]);
        // scalar
        assert_eq!((&f).unwrap_fold(0usize, |a, v| a + v), 60);
    }

    #[test]
    fn member_default_and_overrides() {
        let f = films();
        let c = cast();
        // default member = probe_any(x, |_| true) on leaves and chains
        assert!(f.member(1) && !f.member(3) && !f.member(NO_ID));
        assert!(c.member(0) && !c.member(1));
        assert!((&f).filt(|v| v > 15).member(1) && !(&f).filt(|v| v > 15).member(0));
        // overrides: Universe bound check, MatSet hash, Bitset bit test
        let u = Universe::new(2);
        assert!(u.member(1) && !u.member(2));
        let ms: MatSet<_> = (&f).collect(); // value-set of films: {10, 20, 30}
        assert!(ms.member(10) && !ms.member(11));
    }

    #[test]
    fn identity_sets_and_bitset() {
        let c = cast();
        let u3 = Universe::new(3);
        // films-with-cast as an identity relation: restrict the universe by
        // membership in cast (Julia's `a : b`, the Restrict node).
        let people = u3.with(&c);
        assert_eq!(drive_all(&people), vec![(0, 0), (2, 2)]);
        assert!(people.member(0) && !people.member(1));
        // identity sets send keys through the value slot, so one FromQuery /
        // from_drive impl serves sets and value-bearing queries alike
        let ms: MatSet<_> = (&people).collect();
        assert!(ms.member(0) && !ms.member(1));
        let b = Bitset::over(Universe::new(3), &people);
        assert!(b.member(0) && !b.member(1) && b.member(2));
        assert!(!b.member(NO_ID) && !b.member(3));
        let vb = Bitset::over(Universe::new(9), &c); // values of cast: {7, 8}
        assert!(vb.member(7) && vb.member(8) && !vb.member(0));
        // restrict/diff over Universe — drive emits (x, x)
        let u2 = Universe::new(2);
        assert_eq!(drive_all(&u2.with(&ms)), vec![(0, 0)]);
        assert_eq!(drive_all(&u2.minus(&ms)), vec![(1, 1)]);
        // ∨ is MEMBER-ONLY (no Drive/Probe impl — `drive_all(&u2.or(&b))`
        // and `u2.or(&b).probe(…)` would be compile errors by design;
        // `.union` is the enumerable form): member iff member of either leg.
        assert!(u2.or(&b).member(2) && !u2.or(&b).member(5));
        assert!(u2.minus(&ms).member(1) && !u2.minus(&ms).member(0));
        // identity composes like any relation
        let f = films();
        assert_eq!(drive_all(&(&people).select(&f)), vec![(0, 10), (2, 30)]);
    }

    #[test]
    fn restrict_keeps_lhs_value() {
        // b maps a's values (10, 20) to DIFFERENT values (99, 88); the
        // restriction must pass a's pairs through untouched — the membership
        // test is on a's VALUE against b's DOMAIN, and b's values never flow.
        let f = films(); // 0 → 10, 1 → 20, 2 → 30
        let b = MultiRel::from_pairs(31, [(10, 99usize), (20, 88)]);
        let r = (&f).with(&b);
        assert_eq!(drive_all(&r), vec![(0, 10), (1, 20)]); // not (0, 99) …
        // probe keeps a's value too
        let mut got = Vec::new();
        r.probe(1, |v| got.push(v));
        r.probe(2, |v| got.push(v)); // 30 ∉ dom(b): filtered out
        assert_eq!(got, vec![20]);
    }

    #[test]
    fn restrict_member() {
        // member(Restrict, x) = a has some value at x that is a member of b
        // (the defaulted probe_any(x, |_| true) path)
        let f = films();
        let b = MultiRel::from_pairs(31, [(10, 99usize), (20, 88)]);
        let r = (&f).with(&b);
        assert!(r.member(0) && r.member(1));
        assert!(!r.member(2)); // 30 fails the membership test
        assert!(!r.member(3)); // outside a's domain entirely
        // multi-valued a: film 0 has cast {7, 8}; restrict by value-set {8}
        let c = cast();
        let only8 = MultiRel::from_pairs(9, [(8, 0usize)]);
        let rc = (&c).with(&only8);
        assert!(rc.member(0) && !rc.member(2)); // film 2's only cast is 7
    }

    #[test]
    fn prod_member_is_flat_short_circuit_and() {
        let f = films();
        let c = cast();
        // ∧ = ⊗: member is the flat AND of the per-leg members
        let conj = (&f).filt(|v| v > 15).and(&c);
        assert!(conj.member(2)); // film 2: 30 > 15, has cast
        assert!(!conj.member(1)); // film 1: no cast row
        assert!(!conj.member(0)); // film 0: 10 fails the filter
        // short-circuit: a false first leg never consults the second
        let never = (&f).filt(|_| false);
        let trap = (&f).filt(|_| -> bool { panic!("second leg must not be probed") });
        assert!(!(&never).and(&trap).member(1));
        // in drive position ∧ IS the product: pair values, lhs multiplicity
        assert_eq!(
            drive_all(&(&c).and(&f)),
            vec![(0, (7, 10)), (0, (8, 10)), (2, (7, 30))]
        );
    }

    #[test]
    fn diff_is_value_bearing_and_key_based() {
        let c = cast();
        let u1 = Universe::new(1); // key set {0}
        // value-bearing lhs: pairs pass through with their VALUES; the
        // exclusion test is on the KEY (film id), not the value
        let dd = (&c).minus(u1);
        assert_eq!(drive_all(&dd), vec![(2, 7)]);
        assert!(dd.member(2) && !dd.member(0) && !dd.member(1));
        let mut got = Vec::new();
        dd.probe(2, |v| got.push(v));
        dd.probe(0, |v| got.push(v));
        assert_eq!(got, vec![7]);
    }

    #[test]
    fn union_is_bag_concat() {
        let c = cast();
        let u2 = Universe::new(2);
        // duplicates are preserved: each leg emits all its rows
        assert_eq!(
            drive_all(&(&c).union(&c)),
            vec![(0, 7), (0, 7), (0, 8), (0, 8), (2, 7), (2, 7)]
        );
        // identity legs: the overlap is emitted twice; a deduping sink
        // (Bitset / MatSet collect) collapses the bag back to a set
        let both = u2.union(Universe::new(1));
        assert_eq!(drive_all(&both), vec![(0, 0), (0, 0), (1, 1)]);
        let b = Bitset::over(Universe::new(2), &both);
        assert!(b.member(0) && b.member(1));
    }

    #[test]
    fn typed_ids_compose() {
        struct M;
        struct K;
        // typed fixture columns: movie → kind id, kind → name
        let mk: VecRel<Id<K>, Id<M>> = VecRel::new(vec![Id::new(1), Id::new(0), Id::new(1)]);
        let kname: VecRel<&'static str, Id<K>> = VecRel::new(vec!["alpha", "beta"]);
        // compose through the typed bridge (Id<K> = Id<K>) — the shape the
        // schema!-generated nav methods build (`q.kname()` ≡ `q.select(kname)`)
        let mut got = Vec::new();
        (&mk).select(&kname).drive(|m, n| got.push((m.0, n)));
        assert_eq!(got, vec![(0, "beta"), (1, "alpha"), (2, "beta")]);
        let mut got = Vec::new();
        (&mk)
            .select(&kname)
            .eq("beta")
            .drive(|m, n| got.push((m.0, n)));
        assert_eq!(got, vec![(0, "beta"), (2, "beta")]);
        // member position through a typed universe restriction
        let u: Universe<Id<M>> = Universe::new(3);
        let live = u.with((&mk).select(&kname).eq("alpha"));
        assert!(live.member(Id::new(1)) && !live.member(Id::new(0)));
        assert!(!live.member(Id::NONE));
        // typed ids work as fold/group keys (Eq + Hash + Ord)
        let counts = (&mk).inv().fold(0i64, |a, _| a + 1);
        let mut got = Vec::new();
        counts.drive(|k: Id<K>, c| got.push((k.0, c)));
        got.sort();
        assert_eq!(got, vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn collect_set_restrict_and_map() {
        let f = films();
        let u = Universe::new(31);
        // Julia's `⩘`: the universe 0..31 restricted by films' collected
        // value-set {10, 20, 30}
        let w = u.with((&f).collect::<MatSet<_>>());
        assert_eq!(drive_all(&w), vec![(10, 10), (20, 20), (30, 30)]);
        assert!(w.member(10) && !w.member(11));
        assert_eq!(
            drive_all(&(&f).map(|v| v * 2)),
            vec![(0, 20), (1, 40), (2, 60)]
        );
    }

    // ===== non-dense entities (Key<E> + DictTable) =======================

    // An FK into a NON-DENSE entity stores a `Key`; navigation crosses the
    // entity's `DictTable` (Key→row) before reading columns. The whole chain is
    // the stock `.select` combinator — only the entity table differs from the
    // dense `Ident` case.
    #[test]
    fn nondense_entity_navigation() {
        struct Movie;
        struct Person;
        // Person: non-dense external ids {100,205,9899} → rows {0,1,2}; names.
        let person_table = DictTable::<Person>::from_keys(&[100, 205, 9899]);
        let person_name: VecRel<&str, Id<Person>> = VecRel {
            values: vec!["Nolan", "Kubrick", "Tarkovsky"],
            _d: PhantomData,
        };
        // Movie.director : FK storing the EXTERNAL person key.
        let director: VecRel<Key<Person>, Id<Movie>> = VecRel {
            values: vec![Key::new(205), Key::new(100), Key::new(9899)],
            _d: PhantomData,
        };
        let movies = Universe::<Id<Movie>>::new(3);

        // movie → director(Key) → person(row) → name
        let q = (&movies)
            .select(&director)
            .select(&person_table)
            .select(&person_name);
        let mut got = Vec::new();
        q.drive(|m, n| got.push((m.idx(), n)));
        got.sort();
        assert_eq!(got, vec![(0, "Kubrick"), (1, "Nolan"), (2, "Tarkovsky")]);

        // A DANGLING key (no such person) drops out via the table's probe miss;
        // the table works in `with` (semijoin) position too.
        let director2: VecRel<Key<Person>, Id<Movie>> = VecRel {
            values: vec![Key::new(205), Key::new(404), Key::new(9899)],
            _d: PhantomData,
        };
        let live = (&movies).with((&director2).select(&person_table));
        let mut kept = Vec::new();
        live.drive(|m, _| kept.push(m.idx()));
        kept.sort();
        assert_eq!(kept, vec![0, 2]); // movie 1 (dangling 404) dropped
    }

    // Aggregation through a non-dense entity: filter is implicit, navigate the
    // FK into Person, group by the navigated country, fold — all stock
    // combinators over a `Key` domain (no `Dense` needed for `group_by`).
    #[test]
    fn nondense_group_by_through_table() {
        struct Movie;
        struct Person;
        let person_table = DictTable::<Person>::from_keys(&[100, 205, 9899]);
        let country: VecRel<&str, Id<Person>> = VecRel {
            values: vec!["US", "UK", "RU"],
            _d: PhantomData,
        };
        let director: VecRel<Key<Person>, Id<Movie>> = VecRel {
            values: vec![Key::new(205), Key::new(100), Key::new(9899), Key::new(100)],
            _d: PhantomData,
        };
        let movies = Universe::<Id<Movie>>::new(4);

        let dir_country = (&director).select(&person_table).select(&country);
        let counts = (&movies).group_by(dir_country).fold(0_i64, |a, _| a + 1);
        let mut rows: Vec<(&str, i64)> = Vec::new();
        counts.drive(|k, v| rows.push((k, v)));
        rows.sort();
        // director keys 205(UK) 100(US) 9899(RU) 100(US) → US=2, UK=1, RU=1
        assert_eq!(rows, vec![("RU", 1), ("UK", 1), ("US", 2)]);
    }

    // The entity table is pluggable: an Ident (dense, Key==Id) and a DictTable
    // (non-dense) both drive the same navigation to the same result.
    #[test]
    fn dense_ident_matches_nondense_dict() {
        struct Movie;
        struct Person;
        let name: VecRel<&str, Id<Person>> = VecRel {
            values: vec!["Nolan", "Kubrick"],
            _d: PhantomData,
        };
        let movies = Universe::<Id<Movie>>::new(2);

        // dense: FK stores the row Id directly; entity table is Ident.
        let fk_dense: VecRel<Id<Person>, Id<Movie>> = VecRel {
            values: vec![Id::from_idx(1), Id::from_idx(0)],
            _d: PhantomData,
        };
        let mut d = Vec::new();
        (&movies)
            .select(&fk_dense)
            .select(Ident::<Person>::new())
            .select(&name)
            .drive(|m, n| d.push((m.idx(), n)));
        d.sort();

        // non-dense: same logical mapping via external keys + a DictTable.
        let table = DictTable::<Person>::from_keys(&[100, 205]); // row0=key100, row1=key205
        let fk_keys: VecRel<Key<Person>, Id<Movie>> = VecRel {
            values: vec![Key::new(205), Key::new(100)],
            _d: PhantomData,
        };
        let mut nd = Vec::new();
        (&movies)
            .select(&fk_keys)
            .select(&table)
            .select(&name)
            .drive(|m, n| nd.push((m.idx(), n)));
        nd.sort();

        assert_eq!(d, nd);
        assert_eq!(d, vec![(0, "Kubrick"), (1, "Nolan")]);
    }
}
