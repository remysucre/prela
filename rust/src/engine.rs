// Top-down CPS engine — eager physical state, compile-time access modes.
//
// One trait family mirrors prela's Driven/Probed access modes:
//
//   Query    { type D; type R; }              — a binary relation D → R
//   Drive:  Query + fn drive(&self, k)        — can be scanned (k(d, r) per pair)
//   Probe:  Query + fn probe / probe_any /    — can be looked up by key;
//                  fn member                  member(x) = probe_any(x, |_| true)
//
// There is no separate key-set family: a set IS an identity relation D → D
// (Julia's `Unary{D} <: Query{D, D}`). Set-shaped nodes (Universe, Bitset,
// MatSet, Disj) emit `(x, x)` from drive and yield `x` from
// probe iff member — so they compose, product and left-compose like any
// other relation, with no keyset projection in between. Membership is part
// of the relation protocol: `member(q, x)` defaults to
// `probe_any(x, |_| true)` for ANY probe-able query and is overridden where
// a direct test is cheaper (Bitset bit-test, Universe bound check, MatSet
// hash lookup).
//
// Set algebra mirrors Julia exactly: `∧`/`.and` is an ALIAS for the product
// (`member(Prod)` short-circuits flat across the legs without building the
// pair value); `-`/`.minus` is the value-bearing `Diff` (key-based test);
// `∨`/`.or` is the probe-only membership union `Disj` (driving it is a
// compile error); the enumerable bag union is the separate `Union` node.
//
// A node implements exactly the modes it supports, with bounds that propagate
// the mode rule through the plan (a Compose drives its lhs and probes its
// rhs, etc.), so a mode error is a *compile* error — rustc performs the same
// lowering prela's `prepare` does with `Driven()`/`Probed()`, at type-check
// time. Stream/index pairs are separate types, chosen by the query author:
//
//   .inv()                     → InvStream  (drive-only: flips pairs, no state)
//   .collect::<HashIdx<_,_>>() → HashIdx    (probe-only: eager HashMap<D, SVec<R>>)
//   .collect::<MatSet<_>>()    → MatSet     (probe-only membership set)
//   s.group_by(key)            → GroupBy    (drive-only)
//   .fold(...)                 → Fold       (cache; both modes)
//
// NO HIDDEN MATERIALIZATION: a drive-only node in probe position is a compile
// error, and the fix is an explicit `collect` whose target type names the
// physical structure (the `FromQuery` mirror of Iterator's `FromIterator`) —
// every index/set build is visible in the query text.
//
// State is EAGER: every index/cache-holding node builds its state in its
// constructor, from already-built children, and holds it in plain
// concretely-typed fields — no OnceCell, no interior mutability (everything
// is Sync for free). Construction = prela's `prepare`; the monomorphized
// `drive` = the staged scan. Continuations are generic FnMut closures, so
// each query type monomorphizes into a fused loop nest at `cargo build` time.
// All non-leaf operators are #[inline(always)]; the runtime cost matches a
// hand-rolled imperative loop.

use ahash::{AHashMap as HashMap, AHashSet as HashSet};
use regex::Regex;
use smallvec::SmallVec;
use std::hash::Hash;

/// Default inline capacity for the probe-index buckets. Most TPC-H
/// foreign-key relations are 1:1 or 1:few (e.g. lineitems-per-order ≈ 4),
/// so this size keeps the common case inline + heap-free.
type SVec<T> = SmallVec<[T; 4]>;

// ===== the mode traits ==================================================

pub trait Query {
    type D: Copy + Eq + Hash;
    type R: Copy;
}

pub trait Drive: Query {
    fn drive<K: FnMut(Self::D, Self::R)>(&self, k: K);
}

pub trait Probe: Query {
    fn probe<K: FnMut(Self::R)>(&self, x: Self::D, k: K);
    fn probe_any<K: FnMut(Self::R) -> bool>(&self, x: Self::D, k: K) -> bool;
    /// Domain-membership test — "is `x` in the domain of this relation?".
    /// The default is the universal definition (`probe_any` with a
    /// trivially-true continuation, which short-circuits at the first
    /// value); leaves with a cheaper direct test override it.
    #[inline(always)]
    fn member(&self, x: Self::D) -> bool {
        self.probe_any(x, |_| true)
    }
}

// blanket: &T inherits T's modes.
impl<T: Query + ?Sized> Query for &T { type D = T::D; type R = T::R; }
impl<T: Drive + ?Sized> Drive for &T {
    #[inline(always)]
    fn drive<K: FnMut(T::D, T::R)>(&self, k: K) { (**self).drive(k); }
}
impl<T: Probe + ?Sized> Probe for &T {
    #[inline(always)]
    fn probe<K: FnMut(T::R)>(&self, x: T::D, k: K) { (**self).probe(x, k); }
    #[inline(always)]
    fn probe_any<K: FnMut(T::R) -> bool>(&self, x: T::D, k: K) -> bool {
        (**self).probe_any(x, k)
    }
    #[inline(always)]
    fn member(&self, x: T::D) -> bool { (**self).member(x) }
}

// ===== leaf storage =====================================================
// Entity ids are 0-based `usize`: a universe of size n has ids 0..n-1,
// indexing its dense columns directly. (The binary cache is 1-based — Julia
// writes it — and the loaders shift at the load edge: internal id = cache
// id − 1.) Ids are opaque dense indexes, so the id domain type is `usize`;
// scalar value columns (years, sizes, counts, …) stay `i64`/`f64`.
//
// `NO_ID` is the missing-id sentinel (FK hole fill, "none seen yet" fold
// states): it fails every `i < len` / `.get` bounds check, so a hole probes
// to nothing for free.
//
// `VecRel<R>` — total 1:1 relation; entity-id → R (one value per id).
// Keys with no pair keep the fill value (`R::default()` for `from_pairs`).
// INVARIANT: an FK-valued column over a gappy key space (holes that a query
// can drive or probe, e.g. TPC-H ord_customer over the sparse orderkey
// domain) must use `from_pairs_fill` with fill `NO_ID` — a default-0 hole
// would alias entity 0, which is a live id.
// `MultiRel<R>` — multi-valued / partial; dense forward index Vec<Vec<R>>
// addressed by .id; empty slot for missing keys.

pub const NO_ID: usize = usize::MAX;

pub struct VecRel<R: Copy> {
    pub values: Vec<R>,
}

pub struct MultiRel<R: Copy> {
    pub fwd: Vec<Vec<R>>,
}

impl<R: Copy> VecRel<R> {
    pub fn from_pairs_fill(n: usize, fill: R, pairs: impl IntoIterator<Item = (usize, R)>) -> Self {
        let mut values = vec![fill; n];
        for (k, v) in pairs {
            values[k] = v;
        }
        VecRel { values }
    }
}

impl<R: Copy + Default> VecRel<R> {
    pub fn from_pairs(n: usize, pairs: impl IntoIterator<Item = (usize, R)>) -> Self {
        Self::from_pairs_fill(n, R::default(), pairs)
    }
}

impl<R: Copy> MultiRel<R> {
    pub fn from_pairs(n: usize, pairs: impl IntoIterator<Item = (usize, R)>) -> Self {
        let mut fwd: Vec<Vec<R>> = (0..n).map(|_| Vec::new()).collect();
        for (k, v) in pairs {
            if k < n {
                fwd[k].push(v);
            }
        }
        MultiRel { fwd }
    }
}

// Probe policy: drive loops iterate, so they use safe iterators (bounds-
// check-free by construction). Probe indexes by *data* (a foreign key);
// with `usize` keys there is no cast, and `.get` IS the single bounds
// check — a missing-key sentinel (`NO_ID` = usize::MAX) or any
// out-of-universe id fails it, so "missing key emits nothing" and bounds
// safety are the same one check. No `unsafe` needed.
impl<R: Copy> Query for VecRel<R> { type D = usize; type R = R; }
impl<R: Copy> Drive for VecRel<R> {
    #[inline(always)]
    fn drive<K: FnMut(usize, R)>(&self, mut k: K) {
        for (i, &v) in self.values.iter().enumerate() {
            k(i, v);
        }
    }
}
impl<R: Copy> Probe for VecRel<R> {
    #[inline(always)]
    fn probe<K: FnMut(R)>(&self, x: usize, mut k: K) {
        if let Some(&v) = self.values.get(x) {
            k(v);
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(R) -> bool>(&self, x: usize, mut k: K) -> bool {
        self.values.get(x).is_some_and(|&v| k(v))
    }
}

impl<R: Copy> Query for MultiRel<R> { type D = usize; type R = R; }
impl<R: Copy> Drive for MultiRel<R> {
    #[inline(always)]
    fn drive<K: FnMut(usize, R)>(&self, mut k: K) {
        for (i, vs) in self.fwd.iter().enumerate() {
            for &v in vs { k(i, v); }
        }
    }
}
impl<R: Copy> Probe for MultiRel<R> {
    #[inline(always)]
    fn probe<K: FnMut(R)>(&self, x: usize, mut k: K) {
        if let Some(vs) = self.fwd.get(x) {
            for &v in vs { k(v); }
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(R) -> bool>(&self, x: usize, mut k: K) -> bool {
        self.fwd.get(x).is_some_and(|vs| vs.iter().any(|&v| k(v)))
    }
}

// ===== Universe — identity relation over the dense entity ids ===========

#[derive(Copy, Clone)]
pub struct Universe { pub n: usize }

impl Query for Universe { type D = usize; type R = usize; }
impl Drive for Universe {
    #[inline(always)]
    fn drive<K: FnMut(usize, usize)>(&self, mut k: K) {
        for i in 0..self.n { k(i, i); }
    }
}
impl Probe for Universe {
    #[inline(always)]
    fn probe<K: FnMut(usize)>(&self, x: usize, mut k: K) {
        if x < self.n { k(x); }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(usize) -> bool>(&self, x: usize, mut k: K) -> bool {
        x < self.n && k(x)
    }
    #[inline(always)]
    fn member(&self, x: usize) -> bool { x < self.n }
}

// ===== Compose: a: D → M, b: M → R  ⟹  Compose: D → R ===================
// Mode rule: the rhs is always probed; the lhs carries the Compose's mode.

pub struct Compose<A, B> { pub a: A, pub b: B }

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

// ===== Filter (relation × scalar predicate) =============================
// The predicate is a plain closure `Fn(A::R) -> bool`, held directly — no
// predicate trait layer (Julia: `Filter(a, pred)` with any callable). Every
// comparison combinator below (`.eq`, `.gt`, `.rx`, …) is a captured-closure
// form of `.filt`.

pub struct Filter<A, F> { pub a: A, pub p: F }

impl<A: Query, F> Query for Filter<A, F> {
    type D = A::D;
    type R = A::R;
}
impl<A: Drive, F: Fn(A::R) -> bool> Drive for Filter<A, F> {
    #[inline(always)]
    fn drive<K: FnMut(A::D, A::R)>(&self, mut k: K) {
        self.a.drive(|x, v| if (self.p)(v) { k(x, v); });
    }
}
impl<A: Probe, F: Fn(A::R) -> bool> Probe for Filter<A, F> {
    #[inline(always)]
    fn probe<K: FnMut(A::R)>(&self, x: A::D, mut k: K) {
        self.a.probe(x, |v| if (self.p)(v) { k(v); });
    }
    #[inline(always)]
    fn probe_any<K: FnMut(A::R) -> bool>(&self, x: A::D, mut k: K) -> bool {
        self.a.probe_any(x, |v| (self.p)(v) && k(v))
    }
}

// ===== Restrict (relation × relation — `a : b`) =========================
// Keeps a's pairs (a's VALUE flows through) where the value is a `member`
// of b; b is consumed via `member` only (julia-engine branch, interp.jl:
// `drive(n::Restrict, k) = drive(n.a, (x, m) -> member(n.b, m) && k(x, m))`,
// probe/probe_any analogous). No `member` override: the defaulted
// `probe_any(x, |_| true)` already reduces to
// `a.probe_any(x, |v| b.member(v))`, which is the optimal form.

pub struct Restrict<A, B> { pub a: A, pub b: B }

impl<A: Query, B: Query<D = A::R>> Query for Restrict<A, B> {
    type D = A::D;
    type R = A::R;
}
impl<A: Drive, B: Probe<D = A::R>> Drive for Restrict<A, B> {
    #[inline(always)]
    fn drive<K: FnMut(A::D, A::R)>(&self, mut k: K) {
        self.a.drive(|x, v| if self.b.member(v) { k(x, v); });
    }
}
impl<A: Probe, B: Probe<D = A::R>> Probe for Restrict<A, B> {
    #[inline(always)]
    fn probe<K: FnMut(A::R)>(&self, x: A::D, mut k: K) {
        self.a.probe(x, |v| if self.b.member(v) { k(v); });
    }
    #[inline(always)]
    fn probe_any<K: FnMut(A::R) -> bool>(&self, x: A::D, mut k: K) -> bool {
        self.a.probe_any(x, |v| self.b.member(v) && k(v))
    }
}

// ===== Diff / Disj / Union — set algebra ================================
// Conjunction needs no node at all: `∧` IS the product (Julia:
// `∧(a, b) = ⊗(a, b)`), and `member(Prod)` short-circuits flat across the
// legs without building the pair value — see Prod below. The remaining set
// operators take ANY member-capable rhs (no projection of a value-bearing
// operand to a "keyset" node).

/// `a - b` — Julia's value-bearing minus: keyed on `a`'s DOMAIN, drive and
/// probe pass `a`'s `(x, v)` pairs through unchanged, skipping keys that
/// are members of `b` (julia-engine interp.jl `drive(n::Diff, k) =
/// drive(n.a, (x, y) -> member(n.b, x) || k(x, y))`). For an identity `a`
/// this degenerates to the plain set difference (emits `(x, x)`).
pub struct Diff<A, B> { pub a: A, pub b: B }
impl<A: Query, B: Query<D = A::D>> Query for Diff<A, B> { type D = A::D; type R = A::R; }
impl<A: Drive, B: Probe<D = A::D>> Drive for Diff<A, B> {
    #[inline(always)]
    fn drive<K: FnMut(A::D, A::R)>(&self, mut k: K) {
        self.a.drive(|x, v| if !self.b.member(x) { k(x, v); });
    }
}
impl<A: Probe, B: Probe<D = A::D>> Probe for Diff<A, B> {
    #[inline(always)]
    fn probe<K: FnMut(A::R)>(&self, x: A::D, k: K) {
        if !self.b.member(x) { self.a.probe(x, k); }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(A::R) -> bool>(&self, x: A::D, k: K) -> bool {
        !self.b.member(x) && self.a.probe_any(x, k)
    }
    #[inline(always)]
    fn member(&self, x: A::D) -> bool { self.a.member(x) && !self.b.member(x) }
}

/// `∨` — PROBE-ONLY membership union (julia-engine interp.jl: "driving a union
/// (dedup-while-emitting) is the one operation that would need its lhs both
/// driven and probed, so it lives elsewhere"). There is deliberately NO
/// `Drive` impl — driving a `Disj` is a compile error. Enumerate a union
/// with `Union` (bag-concat) instead, materializing first if the sink does
/// not dedup.
pub struct Disj<A, B> { pub a: A, pub b: B }
impl<A: Query, B: Query<D = A::D>> Query for Disj<A, B> { type D = A::D; type R = A::D; }
impl<A: Probe, B: Probe<D = A::D>> Probe for Disj<A, B> {
    #[inline(always)]
    fn probe<K: FnMut(A::D)>(&self, x: A::D, mut k: K) {
        if self.member(x) { k(x); }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(A::D) -> bool>(&self, x: A::D, mut k: K) -> bool {
        self.member(x) && k(x)
    }
    #[inline(always)]
    fn member(&self, x: A::D) -> bool { self.a.member(x) || self.b.member(x) }
}

/// Enumerable BAG union: drive `a` fully, then `b` fully — NO dedup and no
/// membership pretense (drive-only; no `Probe`). The legs must agree on
/// domain AND value type. A key in both legs is emitted by both, so feed a
/// `Union` only to deduping sinks (`Bitset::over`,
/// `.collect::<MatSet<_>>()`, …) or collect into a set first when
/// duplicates would change results.
/// Julia leaves this node as a design note next to `drive(::Disj)`; Rust
/// implements it. Built with `.union(b)`.
pub struct Union<A, B> { pub a: A, pub b: B }
impl<A: Query, B: Query<D = A::D, R = A::R>> Query for Union<A, B> { type D = A::D; type R = A::R; }
impl<A: Drive, B: Drive<D = A::D, R = A::R>> Drive for Union<A, B> {
    #[inline(always)]
    fn drive<K: FnMut(A::D, A::R)>(&self, mut k: K) {
        self.a.drive(&mut k);
        self.b.drive(k);
    }
}

// ===== Prod (× / ⊗, and ∧) — binary; n-ary by nesting ===================
// Mode rule: like Compose — drive the first leg, probe the rest.
// `∧` is an alias for `⊗` (Julia algebra.jl): a conjunction IS a product,
// consumed in member position via the flat short-circuit `member` override
// below, which never builds the pair value (Julia's `_prod_member`).

pub struct Prod<A, B> { pub a: A, pub b: B }

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
    /// Flat short-circuit AND of the per-leg `member`s — the conj-position
    /// fast path (Julia `_prod_member`). Unlike the default (which threads
    /// the pair through `probe_any`), no pair value is ever built.
    #[inline(always)]
    fn member(&self, x: A::D) -> bool { self.a.member(x) && self.b.member(x) }
}

// ===== InvStream — `q'` in drive position: flip pairs, no state =========

pub struct InvStream<Q> { pub q: Q }

impl<Q: Query> Query for InvStream<Q> where Q::R: Eq + Hash {
    type D = Q::R;
    type R = Q::D;
}
impl<Q: Drive> Drive for InvStream<Q> where Q::R: Eq + Hash {
    #[inline(always)]
    fn drive<K: FnMut(Q::R, Q::D)>(&self, mut k: K) {
        self.q.drive(|d, r| k(r, d));
    }
}

// ===== FromQuery / collect — explicit materialization ======================
// The relation mirror of `FromIterator`/`Iterator::collect`: `q.collect()`
// drives `q` once into the physical structure named by the target type
// (turbofish or `let` annotation). This is the ONLY way a stream becomes
// probe-side state, so every materialization is visible in the query text.
// `Bitset` deliberately does not implement `FromQuery`: it needs the universe
// size `n` — part of the physical choice — so it keeps the explicit
// `Bitset::over(universe, q)` constructor.

pub trait FromQuery<Q: Drive>: Sized {
    fn from_rel(q: Q) -> Self;
}

// ===== HashIdx — THE probe-side physical node ===========================
// An eager `HashMap<K, SVec<V>>` with probe access — the probed form of a
// materialized forward index (`.collect::<HashIdx<_, _>>()`).

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

impl<K: Copy + Eq + Hash, V: Copy> Query for HashIdx<K, V> { type D = K; type R = V; }
impl<K: Copy + Eq + Hash, V: Copy> Probe for HashIdx<K, V> {
    #[inline(always)]
    fn probe<F: FnMut(V)>(&self, x: K, mut k: F) {
        if let Some(vs) = self.idx.get(&x) {
            for &v in vs { k(v); }
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

// ===== MatSet — materialized membership set (probe-only identity) ====

pub struct MatSet<D: Copy + Eq + Hash> { pub set: HashSet<D> }
/// Drive the input and collect the VALUE slot. Identity relations send
/// their keys through the value slot, so one impl materializes a set's
/// keys and a value-bearing query's values alike.
impl<Q: Drive> FromQuery<Q> for MatSet<Q::R>
where Q::R: Eq + Hash {
    fn from_rel(q: Q) -> Self {
        let mut set = HashSet::new();
        q.drive(|_, v| { set.insert(v); });
        MatSet { set }
    }
}
impl<D: Copy + Eq + Hash> Query for MatSet<D> { type D = D; type R = D; }
impl<D: Copy + Eq + Hash> Probe for MatSet<D> {
    #[inline(always)]
    fn probe<K: FnMut(D)>(&self, x: D, mut k: K) {
        if self.set.contains(&x) { k(x); }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(D) -> bool>(&self, x: D, mut k: K) -> bool {
        self.set.contains(&x) && k(x)
    }
    #[inline(always)]
    fn member(&self, x: D) -> bool { self.set.contains(&x) }
}

// ===== Bitset — `Vec<u64>`-backed dense identity relation ===============
//
// Drop-in replacement for `MatSet` when the membership domain is a
// dense `0..n`: trades the HashSet's hash+probe for one bit-test.
// `drive` enumerates set bits via word-scan + `trailing_zeros` so
// iteration cost is proportional to popcount, not the universe size.
// `set` rejects keys ≥ n (`NO_ID` hole sentinels), so padding bits in the
// last word stay 0 and `member`/`drive` can trust the words as-is.

// Not a `FromQuery` target: the universe size `n` is part of the physical
// choice, so construction stays explicit via `Bitset::over(universe, q)`.
pub struct Bitset { pub bs: Vec<u64>, pub n: usize }

impl Bitset {
    pub fn empty(u: Universe) -> Self {
        Bitset { bs: vec![0u64; u.n.div_ceil(64)], n: u.n }
    }
    /// A bitset over `u`, driven from `q`: set a bit at each emitted VALUE.
    /// Identity relations send their keys through the value slot, so one
    /// constructor bit-sets a set's keys and a value-bearing query's values
    /// alike (julia-engine plan.jl `build_bitset`). Out-of-universe values —
    /// including `NO_ID` hole fills — are dropped by the `set` guard.
    pub fn over<Q: Drive<R = usize>>(u: Universe, q: &Q) -> Self {
        let mut b = Self::empty(u);
        q.drive(|_, c| b.set(c));
        b
    }
    #[inline] pub fn set(&mut self, x: usize) {
        if x < self.n {
            self.bs[x / 64] |= 1u64 << (x % 64);
        }
    }
}

impl Query for Bitset { type D = usize; type R = usize; }
impl Drive for Bitset {
    #[inline]
    fn drive<K: FnMut(usize, usize)>(&self, mut k: K) {
        for (wi, &w) in self.bs.iter().enumerate() {
            let mut w = w;
            while w != 0 {
                let b = w.trailing_zeros() as usize;
                let x = wi * 64 + b;
                k(x, x);
                w &= w - 1;
            }
        }
    }
}
impl Probe for Bitset {
    #[inline]
    fn probe<K: FnMut(usize)>(&self, x: usize, mut k: K) {
        if self.member(x) { k(x); }
    }
    #[inline]
    fn probe_any<K: FnMut(usize) -> bool>(&self, x: usize, mut k: K) -> bool {
        self.member(x) && k(x)
    }
    #[inline]
    fn member(&self, x: usize) -> bool {
        self.bs.get(x / 64).is_some_and(|&w| (w >> (x % 64)) & 1 == 1)
    }
}

// ===== GroupBy (Julia `r ← s`) — drive src, probe key per row ===========
// For src: Drive<D, SV> and key: Probe<D, RK>, produces a drive-only
// RK → SV: each src pair is re-keyed by `key`'s value at the same d.
// Method spelling is receiver-first on the DRIVEN side: `s.group_by(r)` —
// Julia's `←` argument order is an infix-surface artifact.

pub struct GroupBy<S, R> { pub src: S, pub key: R }

impl<S: Query, R: Query<D = S::D>> Query for GroupBy<S, R> where R::R: Eq + Hash {
    type D = R::R;
    type R = S::R;
}
impl<S: Drive, R: Probe<D = S::D>> Drive for GroupBy<S, R> where R::R: Eq + Hash {
    #[inline(always)]
    fn drive<K: FnMut(R::R, S::R)>(&self, mut k: K) {
        self.src.drive(|d, sv| self.key.probe(d, |rk| k(rk, sv)));
    }
}

// ===== Fold (`▷`) — per-key reduce into an eager cache ==================
// One physical type serves foldl (`.fold`) and the buffered whole-group
// reduce (`.buf_fold`, Julia's BufFold; `.count_distinct` is an instance)
// — they differ only in how the cache is filled (julia-engine interp.jl FoldP).

pub struct Fold<D: Copy + Eq + Hash, S: Copy> {
    pub cache: HashMap<D, S>,
}

impl<D: Copy + Eq + Hash, S: Copy> Fold<D, S> {
    /// Per-key foldl: cache[d] = op(op(init, v1), v2)…
    pub fn build<Q, OP>(q: Q, init: S, op: OP) -> Self
    where Q: Drive<D = D>, OP: Fn(S, Q::R) -> S {
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
    where Q: Drive<D = D>, F: Fn(SVec<Q::R>) -> S {
        let mut buf: HashMap<D, SVec<Q::R>> = HashMap::new();
        q.drive(|d, v| buf.entry(d).or_default().push(v));
        Fold { cache: buf.into_iter().map(|(d, vs)| (d, f(vs))).collect() }
    }
}

impl<D: Copy + Eq + Hash, S: Copy> Query for Fold<D, S> { type D = D; type R = S; }
impl<D: Copy + Eq + Hash, S: Copy> Drive for Fold<D, S> {
    #[inline(always)]
    fn drive<K: FnMut(D, S)>(&self, mut k: K) {
        for (&d, &s) in &self.cache { k(d, s); }
    }
}
impl<D: Copy + Eq + Hash, S: Copy> Probe for Fold<D, S> {
    #[inline(always)]
    fn probe<K: FnMut(S)>(&self, x: D, mut k: K) {
        if let Some(&s) = self.cache.get(&x) { k(s); }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(S) -> bool>(&self, x: D, mut k: K) -> bool {
        match self.cache.get(&x) { Some(&s) => k(s), None => false }
    }
}

// ===== DenseFold — `▷` with dense id-keyed array cache ==================
//
// Drop-in replacement for `Fold` when `D = usize` and the key range is a
// known, dense `0..n`. Backing store is `Vec<S>` (one slot per key) plus a
// parallel `Vec<bool>` presence map. Avoids HashMap probe + entry alloc on
// every reduce step; for Q1 (≤6 group keys via packed byte index), Q2 / Q20
// (per-part), Q18 (per-order), the gain is ~5-10× over `Fold`.

pub struct DenseFold<S: Copy> {
    pub vals: Vec<S>,
    pub seen: Vec<bool>,
}

impl<S: Copy> DenseFold<S> {
    pub fn build<Q, OP>(q: Q, n: usize, init: S, op: OP) -> Self
    where Q: Drive<D = usize>, OP: Fn(S, Q::R) -> S {
        let mut vals = vec![init; n];
        let mut seen = vec![false; n];
        q.drive(|d, v| {
            if let Some(s) = vals.get_mut(d) {
                *s = op(*s, v);
                seen[d] = true;
            }
        });
        DenseFold { vals, seen }
    }
}

impl<S: Copy> Query for DenseFold<S> { type D = usize; type R = S; }
impl<S: Copy> Drive for DenseFold<S> {
    #[inline(always)]
    fn drive<K: FnMut(usize, S)>(&self, mut k: K) {
        for (i, (&v, &seen)) in self.vals.iter().zip(&self.seen).enumerate() {
            if seen {
                k(i, v);
            }
        }
    }
}
impl<S: Copy> Probe for DenseFold<S> {
    #[inline(always)]
    fn probe<K: FnMut(S)>(&self, x: usize, mut k: K) {
        if let Some(&v) = self.vals.get(x) {
            if self.seen[x] { k(v); }
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(S) -> bool>(&self, x: usize, mut k: K) -> bool {
        self.vals.get(x).is_some_and(|&v| self.seen[x] && k(v))
    }
}

// ===== Map (`↦ f`) — per-row lambda =====================================

pub struct Map<Q, F, S: Copy> {
    pub q: Q,
    pub f: F,
    _phantom: std::marker::PhantomData<S>,
}

impl<Q: Query, F: Fn(Q::R) -> S, S: Copy> Map<Q, F, S> {
    pub fn new(q: Q, f: F) -> Self { Map { q, f, _phantom: std::marker::PhantomData } }
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

// ===== operators (method-only surface) ==================================
// Constructors are mode-agnostic (they just build the node; the node's
// trait impls carry the mode bounds) EXCEPT the eager physical nodes, whose
// constructors drive their input right here — those require `Self: Drive`
// and consume their input, exactly like prela's `build_*` inside `prepare`.

pub trait QueryExt: Query + Sized {
    /// Compose two queries (bridge type = self's value type).
    #[inline(always)]
    fn o<B: Query<D = Self::R>>(self, b: B) -> Compose<Self, B> { Compose { a: self, b } }

    /// Postfix adjoint in drive position — streams flipped pairs, no state.
    #[inline(always)]
    fn inv(self) -> InvStream<Self> where Self::R: Eq + Hash { InvStream { q: self } }

    /// `∧` — alias for the product (Julia: `∧(a, b) = ⊗(a, b)`). In member
    /// position (a conjunct tree fed to `.in_s`, `.minus`'s rhs, …) the
    /// `member` override short-circuits flat across the legs without
    /// building pair values; in drive/probe position it IS `⊗` and emits
    /// nested-pair values — restrict-then-project is `a.in_s(p).o(b)`.
    #[inline(always)]
    fn and<B: Query<D = Self::D>>(self, b: B) -> Prod<Self, B> { Prod { a: self, b } }

    /// `∨` — probe-only membership union (`member` = a OR b). Driving it is
    /// a compile error; enumerate with `.union(b)` instead.
    #[inline(always)]
    fn or<B: Query<D = Self::D>>(self, b: B) -> Disj<Self, B> { Disj { a: self, b } }

    /// `-` — value-bearing difference: self's pairs whose KEY is not a
    /// member of `b` (identity self ⟹ plain set difference).
    #[inline(always)]
    fn minus<B: Query<D = Self::D>>(self, b: B) -> Diff<Self, B> { Diff { a: self, b } }

    /// Cartesian product (× / ⊗).
    #[inline(always)]
    fn x<B: Query<D = Self::D>>(self, b: B) -> Prod<Self, B> { Prod { a: self, b } }

    /// Enumerable bag union — drive self fully, then `b` fully, NO dedup
    /// (the drive-position complement of the probe-only `.or`). Feed it to
    /// deduping sinks, or collect it into a `MatSet` when duplicates would matter.
    #[allow(dead_code)] // no suite query drives a union today (every `∨` is member-position); kept as the sanctioned enumerable form, exercised by unit tests
    #[inline(always)]
    fn union<B: Query<D = Self::D, R = Self::R>>(self, b: B) -> Union<Self, B> { Union { a: self, b } }

    // Predicate filters — all captured-closure forms of `filt`.
    #[inline(always)] fn eq(self, v: Self::R) -> Filter<Self, impl Fn(Self::R) -> bool>
        where Self::R: PartialEq { self.filt(move |x| x == v) }
    #[inline(always)] fn ne(self, v: Self::R) -> Filter<Self, impl Fn(Self::R) -> bool>
        where Self::R: PartialEq { self.filt(move |x| x != v) }
    #[inline(always)] fn gt(self, v: Self::R) -> Filter<Self, impl Fn(Self::R) -> bool>
        where Self::R: PartialOrd { self.filt(move |x| x > v) }
    #[inline(always)] fn lt(self, v: Self::R) -> Filter<Self, impl Fn(Self::R) -> bool>
        where Self::R: PartialOrd { self.filt(move |x| x < v) }
    #[inline(always)] fn ge(self, v: Self::R) -> Filter<Self, impl Fn(Self::R) -> bool>
        where Self::R: PartialOrd { self.filt(move |x| x >= v) }
    #[inline(always)] fn le(self, v: Self::R) -> Filter<Self, impl Fn(Self::R) -> bool>
        where Self::R: PartialOrd { self.filt(move |x| x <= v) }
    #[inline(always)] fn in_v(self, vs: Vec<Self::R>) -> Filter<Self, impl Fn(Self::R) -> bool>
        where Self::R: PartialEq { self.filt(move |x| vs.iter().any(|&v| v == x)) }
    /// Restriction `a : b` — keep self's pairs whose VALUE is a `member` of
    /// `s` (any probe-able relation). Builds the dedicated `Restrict` node,
    /// node-for-node with Julia.
    #[inline(always)] fn in_s<S: Probe<D = Self::R>>(self, s: S) -> Restrict<Self, S>
        { Restrict { a: self, b: s } }
    #[inline(always)] fn rx(self, re: &str) -> Filter<Self, impl Fn(Self::R) -> bool>
        where Self: Query<R = &'static str> {
        let re = Regex::new(re).unwrap();
        self.filt(move |s| re.is_match(s))
    }
    #[inline(always)] fn nrx(self, re: &str) -> Filter<Self, impl Fn(Self::R) -> bool>
        where Self: Query<R = &'static str> {
        let re = Regex::new(re).unwrap();
        self.filt(move |s| !re.is_match(s))
    }
    /// Closure-predicate filter — for things like cross-column compares.
    #[inline(always)] fn filt<F: Fn(Self::R) -> bool>(self, f: F) -> Filter<Self, F>
        { Filter { a: self, p: f } }
    /// Half-open range `[lo, hi)` — Julia `during(lo, hi)`.
    #[inline(always)] fn during(self, lo: Self::R, hi: Self::R) -> Filter<Self, impl Fn(Self::R) -> bool>
        where Self::R: PartialOrd { self.filt(move |x| x >= lo && x < hi) }
    /// Closed range `[lo, hi]` — Julia `lo..hi`.
    #[inline(always)] fn between(self, lo: Self::R, hi: Self::R) -> Filter<Self, impl Fn(Self::R) -> bool>
        where Self::R: PartialOrd { self.filt(move |x| x >= lo && x <= hi) }

    /// Materialize — drive self once into the physical structure named by
    /// the target type (`FromQuery`, the relation mirror of `FromIterator`):
    /// `.collect::<HashIdx<_, _>>()` for a forward index, `.collect::<
    /// MatSet<_>>()` for a membership set (Julia: `prepare` probing a
    /// `Materialized`). The type annotation IS the visible physical choice.
    #[inline(always)]
    fn collect<T: FromQuery<Self>>(self) -> T where Self: Drive { T::from_rel(self) }

    /// Julia's `r ← s` in drive position — drives self, probes `key` per
    /// row, emits (key-value, self-value). (With sets now identity
    /// relations, grouping by a set is just this general form: the set's
    /// key flows through the value slot.)
    #[inline(always)]
    fn group_by<R: Query<D = Self::D>>(self, key: R) -> GroupBy<Self, R>
    where R::R: Eq + Hash { GroupBy { src: self, key } }

    /// `▷ (op, init)` — per-key foldl into an eager cache.
    #[inline(always)]
    fn fold<OP: Fn(S, Self::R) -> S, S: Copy>(self, init: S, op: OP) -> Fold<Self::D, S>
    where Self: Drive { Fold::build(self, init, op) }

    /// `▷ f` with a callable — per-key whole-multiset reduce (Julia's
    /// BufFold): buffer each group, then cache `f(group)`. For reducers
    /// that need the whole group (count-distinct, median, …) rather than
    /// foldl's streaming `(S, R) -> S` shape.
    #[inline(always)]
    fn buf_fold<F: Fn(SVec<Self::R>) -> S, S: Copy>(self, f: F) -> Fold<Self::D, S>
    where Self: Drive { Fold::build_buf(self, f) }

    /// `▷ (op, init)` with a dense id-keyed `Vec<S>` cache. Use when the
    /// key range is known to be `0..n` (`n` slots) and small/dense enough
    /// that a `Vec<S>` of size `n` beats the HashMap path of `fold`.
    #[inline(always)]
    fn dense_fold<OP: Fn(S, Self::R) -> S, S: Copy>(self, n: usize, init: S, op: OP)
        -> DenseFold<S>
    where Self: Drive<D = usize> { DenseFold::build(self, n, init, op) }

    /// Count-distinct — the `length ∘ unique` instance of `.buf_fold`. The
    /// closure sorts + dedups the per-key SVec on finalization — much
    /// faster than a HashSet per group for the typical small-group case.
    #[inline(always)]
    fn count_distinct(self) -> Fold<Self::D, i64>
    where Self: Drive, Self::R: Ord {
        self.buf_fold(|mut vs| { vs.sort_unstable(); vs.dedup(); vs.len() as i64 })
    }

    /// `↦ f` — per-row map.
    #[inline(always)]
    fn map<F: Fn(Self::R) -> S, S: Copy>(self, f: F) -> Map<Self, F, S> {
        Map::new(self, f)
    }

    /// `⊵ (op, init)` — no-group foldl. Drives the whole query, returns scalar.
    #[inline(always)]
    fn unwrap_fold<OP: Fn(S, Self::R) -> S, S: Copy>(&self, init: S, op: OP) -> S
    where Self: Drive {
        let mut acc = init;
        self.drive(|_, v| acc = op(acc, v));
        acc
    }
}
impl<Q: Query> QueryExt for Q {}

// ===== tests — tiny inline data, every node in every mode ===============

#[cfg(test)]
mod tests {
    use super::*;

    // films: 0 → 10, 1 → 20, 2 → 30 (VecRel); cast: 0 → {7, 8}, 2 → {7} (MultiRel)
    // Values are id-typed (usize) so they can feed compose/restrict domains.
    fn films() -> VecRel<usize> { VecRel::from_pairs(3, [(0, 10), (1, 20), (2, 30)]) }
    fn cast() -> MultiRel<usize> { MultiRel::from_pairs(3, [(0, 7), (0, 8), (2, 7)]) }

    fn drive_all<Q: Drive>(q: &Q) -> Vec<(Q::D, Q::R)>
    where Q::D: Ord, Q::R: Ord {
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
        assert_eq!(drive_all(&(&c).o(&f)), vec![]); // cast values 7,8 not film keys <3
        assert_eq!(drive_all(&(&f).filt(|v| v > 15)), vec![(1, 20), (2, 30)]);
        let u = Universe { n: 2 };
        assert_eq!(drive_all(&u.o(&f)), vec![(0, 10), (1, 20)]);
        assert_eq!(drive_all(&(&f).x(&f)), vec![(0, (10, 10)), (1, (20, 20)), (2, (30, 30))]);
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
        // group film-values by cast-person (Julia `cast ← films`): films
        // driven, cast probed for the key — for film d, value f(d), key =
        // each cast member of d.
        let grouped = (&f).group_by(&c);
        assert_eq!(drive_all(&grouped), vec![(7, 10), (7, 30), (8, 10)]);
        // fold: count films per person
        let counts = (&f).group_by(&c).fold(0i64, |a, _| a + 1);
        assert_eq!(drive_all(&counts), vec![(7, 2), (8, 1)]);
        // dense fold over person ids 0..9
        let dcounts = (&f).group_by(&c).dense_fold(9, 0i64, |a, _| a + 1);
        assert_eq!(drive_all(&dcounts), vec![(7, 2), (8, 1)]);
        // buf_fold: whole-group reduce the foldl shape can't express —
        // person 7 saw film values {10, 30} (range 20), person 8 {10}
        let range = (&f).group_by(&c).buf_fold(|vs| {
            let mn = *vs.iter().min().unwrap();
            let mx = *vs.iter().max().unwrap();
            (mx - mn) as i64
        });
        assert_eq!(drive_all(&range), vec![(7, 20), (8, 0)]);
        // median via buf_fold (an order-statistic — needs the whole group)
        let med = (&f).group_by(&c).buf_fold(|mut vs| {
            vs.sort_unstable();
            vs[vs.len() / 2]
        });
        assert_eq!(drive_all(&med), vec![(7, 30), (8, 10)]);
        // count_distinct = buf_fold's `length ∘ unique` instance; the
        // duplicate (7, 10) row collapses
        let cd = (&f).group_by(&c).union((&f).group_by(&c).filt(|v| v == 10)).count_distinct();
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
        let u = Universe { n: 2 };
        assert!(u.member(1) && !u.member(2));
        let ms: MatSet<_> = (&f).collect(); // value-set of films: {10, 20, 30}
        assert!(ms.member(10) && !ms.member(11));
    }

    #[test]
    fn identity_sets_and_bitset() {
        let c = cast();
        let u3 = Universe { n: 3 };
        // films-with-cast as an identity relation: restrict the universe by
        // membership in cast (Julia's `a : b`, the Restrict node).
        let people = u3.in_s(&c);
        assert_eq!(drive_all(&people), vec![(0, 0), (2, 2)]);
        assert!(people.member(0) && !people.member(1));
        // identity sets send keys through the value slot, so one FromQuery /
        // from_drive impl serves sets and value-bearing queries alike
        let ms: MatSet<_> = (&people).collect();
        assert!(ms.member(0) && !ms.member(1));
        let b = Bitset::over(Universe { n: 3 }, &people);
        assert!(b.member(0) && !b.member(1) && b.member(2));
        assert!(!b.member(NO_ID) && !b.member(3));
        let vb = Bitset::over(Universe { n: 9 }, &c); // values of cast: {7, 8}
        assert!(vb.member(7) && vb.member(8) && !vb.member(0));
        // restrict/diff over Universe — drive emits (x, x)
        let u2 = Universe { n: 2 };
        assert_eq!(drive_all(&u2.in_s(&ms)), vec![(0, 0)]);
        assert_eq!(drive_all(&u2.minus(&ms)), vec![(1, 1)]);
        // ∨ is PROBE-ONLY (no Drive impl — `drive_all(&u2.or(&b))` would be
        // a compile error by design; `.union` is the enumerable form):
        // probe yields x iff member of either leg.
        let mut got = Vec::new();
        u2.or(&b).probe(2, |x| got.push(x));
        u2.or(&b).probe(5, |x| got.push(x));
        assert_eq!(got, vec![2]);
        assert!(u2.or(&b).member(2) && !u2.or(&b).member(5));
        assert!(u2.minus(&ms).member(1) && !u2.minus(&ms).member(0));
        // identity composes like any relation
        let f = films();
        assert_eq!(drive_all(&(&people).o(&f)), vec![(0, 10), (2, 30)]);
    }

    #[test]
    fn restrict_keeps_lhs_value() {
        // b maps a's values (10, 20) to DIFFERENT values (99, 88); the
        // restriction must pass a's pairs through untouched — the membership
        // test is on a's VALUE against b's DOMAIN, and b's values never flow.
        let f = films(); // 0 → 10, 1 → 20, 2 → 30
        let b = MultiRel::from_pairs(31, [(10, 99usize), (20, 88)]);
        let r = (&f).in_s(&b);
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
        let r = (&f).in_s(&b);
        assert!(r.member(0) && r.member(1));
        assert!(!r.member(2));     // 30 fails the membership test
        assert!(!r.member(3));     // outside a's domain entirely
        // multi-valued a: film 0 has cast {7, 8}; restrict by value-set {8}
        let c = cast();
        let only8 = MultiRel::from_pairs(9, [(8, 0usize)]);
        let rc = (&c).in_s(&only8);
        assert!(rc.member(0) && !rc.member(2)); // film 2's only cast is 7
    }

    #[test]
    fn prod_member_is_flat_short_circuit_and() {
        let f = films();
        let c = cast();
        // ∧ = ⊗: member is the flat AND of the per-leg members
        let conj = (&f).filt(|v| v > 15).and(&c);
        assert!(conj.member(2));   // film 2: 30 > 15, has cast
        assert!(!conj.member(1));  // film 1: no cast row
        assert!(!conj.member(0));  // film 0: 10 fails the filter
        // short-circuit: a false first leg never consults the second
        let never = (&f).filt(|_| false);
        let trap = (&f).filt(|_| -> bool { panic!("second leg must not be probed") });
        assert!(!(&never).and(&trap).member(1));
        // in drive position ∧ IS the product: pair values, lhs multiplicity
        assert_eq!(drive_all(&(&c).and(&f)),
                   vec![(0, (7, 10)), (0, (8, 10)), (2, (7, 30))]);
    }

    #[test]
    fn diff_is_value_bearing_and_key_based() {
        let c = cast();
        let u1 = Universe { n: 1 }; // key set {0}
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
        let u2 = Universe { n: 2 };
        // duplicates are preserved: each leg emits all its rows
        assert_eq!(drive_all(&(&c).union(&c)),
                   vec![(0, 7), (0, 7), (0, 8), (0, 8), (2, 7), (2, 7)]);
        // identity legs: the overlap is emitted twice; a deduping sink
        // (Bitset / MatSet collect) collapses the bag back to a set
        let both = u2.union(Universe { n: 1 });
        assert_eq!(drive_all(&both), vec![(0, 0), (0, 0), (1, 1)]);
        let b = Bitset::over(Universe { n: 2 }, &both);
        assert!(b.member(0) && b.member(1));
    }

    #[test]
    fn collect_set_restrict_and_map() {
        let f = films();
        let u = Universe { n: 31 };
        // Julia's `⩘`: the universe 0..31 restricted by films' collected
        // value-set {10, 20, 30}
        let w = u.in_s((&f).collect::<MatSet<_>>());
        assert_eq!(drive_all(&w), vec![(10, 10), (20, 20), (30, 30)]);
        assert!(w.member(10) && !w.member(11));
        assert_eq!(drive_all(&(&f).map(|v| v * 2)), vec![(0, 20), (1, 40), (2, 60)]);
    }
}
