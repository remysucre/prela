// Top-down CPS engine — eager physical state, compile-time access modes.
//
// Two trait families mirror prela's Driven/Probed access modes:
//
//   Rel    { type D; type R; }              — a binary relation D → R
//   Drive:  Rel + fn drive(&self, k)        — can be scanned (k(d, r) per pair)
//   Probe:  Rel + fn probe / probe_any      — can be looked up by key
//
// and for key-sets (the Unary side of the Julia algebra):
//
//   KeySet    { type D; }
//   DriveKeys: KeySet + fn drivekeys        — can enumerate members
//   Member:    KeySet + fn member           — can test membership
//
// A node implements exactly the modes it supports, with bounds that propagate
// the mode rule through the plan (a Compose drives its lhs and probes its
// rhs, etc.), so a mode error is a *compile* error — rustc performs the same
// lowering prela's `prepare` does with `Driven()`/`Probed()`, at type-check
// time. Stream/index pairs are separate types, chosen by the query author:
//
//   .inv()      → InvStream   (drive-only: flips pairs, no state)
//   .mat_idx()  → HashIdx     (probe-only: eager HashMap<D, SVec<R>>)
//   .lc(s)      → LCStream    (drive-only)
//   .fold(...)  → Fold        (cache; both modes)
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

pub trait Rel {
    type D: Copy + Eq + Hash;
    type R: Copy;
}

pub trait Drive: Rel {
    fn drive<K: FnMut(Self::D, Self::R)>(&self, k: K);
}

pub trait Probe: Rel {
    fn probe<K: FnMut(Self::R)>(&self, x: Self::D, k: K);
    fn probe_any<K: FnMut(Self::R) -> bool>(&self, x: Self::D, k: K) -> bool;
}

pub trait KeySet {
    type D: Copy + Eq + Hash;
}

pub trait DriveKeys: KeySet {
    fn drivekeys<K: FnMut(Self::D)>(&self, k: K);
}

pub trait Member: KeySet {
    fn member(&self, x: Self::D) -> bool;
}

// blanket: &T inherits T's modes.
impl<T: Rel + ?Sized> Rel for &T { type D = T::D; type R = T::R; }
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
}
impl<S: KeySet + ?Sized> KeySet for &S { type D = S::D; }
impl<S: DriveKeys + ?Sized> DriveKeys for &S {
    #[inline(always)]
    fn drivekeys<K: FnMut(S::D)>(&self, k: K) { (**self).drivekeys(k); }
}
impl<S: Member + ?Sized> Member for &S {
    #[inline(always)]
    fn member(&self, x: S::D) -> bool { (**self).member(x) }
}

// ===== leaf storage =====================================================
// `Vec1<R>` — total 1:1 relation; entity-id → R (one value per id). The
// vector is 1-indexed (slot 0 is a sentinel).
// `Many<R>` — multi-valued / partial; dense forward index Vec<Vec<R>>
// addressed by .id; empty slot for missing keys.

pub struct Vec1<R: Copy> {
    pub values: Vec<R>,
}

pub struct Many<R: Copy> {
    pub fwd: Vec<Vec<R>>,
}

impl<R: Copy + Default> Vec1<R> {
    pub fn from_pairs(n: usize, pairs: impl IntoIterator<Item = (i64, R)>) -> Self {
        let mut values = vec![R::default(); n + 1];
        for (k, v) in pairs {
            values[k as usize] = v;
        }
        Vec1 { values }
    }
}

impl<R: Copy> Many<R> {
    pub fn from_pairs(n: usize, pairs: impl IntoIterator<Item = (i64, R)>) -> Self {
        let mut fwd: Vec<Vec<R>> = (0..=n).map(|_| Vec::new()).collect();
        for (k, v) in pairs {
            if k >= 1 && (k as usize) <= n {
                fwd[k as usize].push(v);
            }
        }
        Many { fwd }
    }
}

// Unsafe policy: drive loops iterate, so they use safe iterators (bounds-
// check-free by construction). Probe loads index by *data* (a foreign key);
// the manual `1 <= i < len` range check IS the "missing key emits nothing"
// semantics, and the `get_unchecked` after it guarantees the optimizer
// doesn't re-check through the i64→usize cast.
impl<R: Copy> Rel for Vec1<R> { type D = i64; type R = R; }
impl<R: Copy> Drive for Vec1<R> {
    #[inline(always)]
    fn drive<K: FnMut(i64, R)>(&self, mut k: K) {
        for (i, &v) in self.values.iter().enumerate().skip(1) {
            k(i as i64, v);
        }
    }
}
impl<R: Copy> Probe for Vec1<R> {
    #[inline(always)]
    fn probe<K: FnMut(R)>(&self, x: i64, mut k: K) {
        let i = x as usize;
        if i >= 1 && i < self.values.len() {
            k(unsafe { *self.values.get_unchecked(i) });
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(R) -> bool>(&self, x: i64, mut k: K) -> bool {
        let i = x as usize;
        i >= 1 && i < self.values.len() && k(unsafe { *self.values.get_unchecked(i) })
    }
}

impl<R: Copy> Rel for Many<R> { type D = i64; type R = R; }
impl<R: Copy> Drive for Many<R> {
    #[inline(always)]
    fn drive<K: FnMut(i64, R)>(&self, mut k: K) {
        for (i, vs) in self.fwd.iter().enumerate().skip(1) {
            for &v in vs { k(i as i64, v); }
        }
    }
}
impl<R: Copy> Probe for Many<R> {
    #[inline(always)]
    fn probe<K: FnMut(R)>(&self, x: i64, mut k: K) {
        let i = x as usize;
        if i >= 1 && i < self.fwd.len() {
            for &v in unsafe { self.fwd.get_unchecked(i) } { k(v); }
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(R) -> bool>(&self, x: i64, mut k: K) -> bool {
        let i = x as usize;
        if i >= 1 && i < self.fwd.len() {
            for &v in unsafe { self.fwd.get_unchecked(i) } {
                if k(v) { return true; }
            }
        }
        false
    }
}

// ===== Universe (KeySet over i64) =======================================

#[derive(Copy, Clone)]
pub struct Universe { pub n: i64 }

impl KeySet for Universe { type D = i64; }
impl DriveKeys for Universe {
    #[inline(always)]
    fn drivekeys<K: FnMut(i64)>(&self, mut k: K) {
        for i in 1..=self.n { k(i); }
    }
}
impl Member for Universe {
    #[inline(always)]
    fn member(&self, x: i64) -> bool { x >= 1 && x <= self.n }
}

// ===== Compose: a: D → M, b: M → R  ⟹  Compose: D → R ===================
// Mode rule: the rhs is always probed; the lhs carries the Compose's mode.

pub struct Compose<A, B> { pub a: A, pub b: B }

impl<A: Rel, B: Rel<D = A::R>> Rel for Compose<A, B> {
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

// ===== Filter (predicates) ==============================================

pub struct Filter<A, P> { pub a: A, pub p: P }

pub trait Pred<R> { fn test(&self, v: R) -> bool; }

/// Membership-in-a-KeySet predicate. Kept nominal (rather than a closure)
/// so it can hold the set by value without naming its type.
pub struct InSet<S: Member>(pub S);
impl<S: Member<D = i64>> Pred<i64> for InSet<S> {
    #[inline(always)] fn test(&self, v: i64) -> bool { self.0.member(v) }
}

/// Closure predicate — every comparison combinator below desugars to this.
pub struct FnP<F>(pub F);
impl<R: Copy, F: Fn(R) -> bool> Pred<R> for FnP<F> {
    #[inline(always)] fn test(&self, v: R) -> bool { (self.0)(v) }
}

impl<A: Rel, P> Rel for Filter<A, P> {
    type D = A::D;
    type R = A::R;
}
impl<A: Drive, P: Pred<A::R>> Drive for Filter<A, P> {
    #[inline(always)]
    fn drive<K: FnMut(A::D, A::R)>(&self, mut k: K) {
        self.a.drive(|x, v| if self.p.test(v) { k(x, v); });
    }
}
impl<A: Probe, P: Pred<A::R>> Probe for Filter<A, P> {
    #[inline(always)]
    fn probe<K: FnMut(A::R)>(&self, x: A::D, mut k: K) {
        self.a.probe(x, |v| if self.p.test(v) { k(v); });
    }
    #[inline(always)]
    fn probe_any<K: FnMut(A::R) -> bool>(&self, x: A::D, mut k: K) -> bool {
        self.a.probe_any(x, |v| self.p.test(v) && k(v))
    }
}

// ===== Restrict (KeySet : Query) ========================================

pub struct Restrict<A: KeySet, B> { pub a: A, pub b: B }

impl<A: KeySet, B: Rel<D = A::D>> Rel for Restrict<A, B> {
    type D = A::D;
    type R = B::R;
}
impl<A: DriveKeys, B: Probe<D = A::D>> Drive for Restrict<A, B> {
    #[inline(always)]
    fn drive<K: FnMut(A::D, B::R)>(&self, mut k: K) {
        self.a.drivekeys(|x| self.b.probe(x, |r| k(x, r)));
    }
}
impl<A: Member, B: Probe<D = A::D>> Probe for Restrict<A, B> {
    #[inline(always)]
    fn probe<K: FnMut(B::R)>(&self, x: A::D, k: K) {
        if self.a.member(x) { self.b.probe(x, k); }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(B::R) -> bool>(&self, x: A::D, k: K) -> bool {
        self.a.member(x) && self.b.probe_any(x, k)
    }
}

// ===== Keys (Query → KeySet) ============================================

pub struct Keys<Q> { pub q: Q }

impl<Q: Rel> KeySet for Keys<Q> { type D = Q::D; }
impl<Q: Drive> DriveKeys for Keys<Q> {
    #[inline(always)]
    fn drivekeys<K: FnMut(Q::D)>(&self, mut k: K) {
        self.q.drive(|x, _| k(x));
    }
}
impl<Q: Probe> Member for Keys<Q> {
    #[inline(always)]
    fn member(&self, x: Q::D) -> bool {
        self.q.probe_any(x, |_| true)
    }
}

// ===== Conj / Disj / SetDiff ============================================

pub struct Conj<A, B> { pub a: A, pub b: B }
impl<A: KeySet, B: KeySet<D = A::D>> KeySet for Conj<A, B> { type D = A::D; }
impl<A: DriveKeys, B: Member<D = A::D>> DriveKeys for Conj<A, B> {
    #[inline(always)]
    fn drivekeys<K: FnMut(A::D)>(&self, mut k: K) {
        self.a.drivekeys(|x| if self.b.member(x) { k(x); });
    }
}
impl<A: Member, B: Member<D = A::D>> Member for Conj<A, B> {
    #[inline(always)]
    fn member(&self, x: A::D) -> bool { self.a.member(x) && self.b.member(x) }
}

pub struct SetDiff<A, B> { pub a: A, pub b: B }
impl<A: KeySet, B: KeySet<D = A::D>> KeySet for SetDiff<A, B> { type D = A::D; }
impl<A: DriveKeys, B: Member<D = A::D>> DriveKeys for SetDiff<A, B> {
    #[inline(always)]
    fn drivekeys<K: FnMut(A::D)>(&self, mut k: K) {
        self.a.drivekeys(|x| if !self.b.member(x) { k(x); });
    }
}
impl<A: Member, B: Member<D = A::D>> Member for SetDiff<A, B> {
    #[inline(always)]
    fn member(&self, x: A::D) -> bool { self.a.member(x) && !self.b.member(x) }
}

pub struct Disj<A, B> { pub a: A, pub b: B }
impl<A: KeySet, B: KeySet<D = A::D>> KeySet for Disj<A, B> { type D = A::D; }
impl<A: DriveKeys + Member, B: DriveKeys<D = A::D>> DriveKeys for Disj<A, B> {
    #[inline(always)]
    fn drivekeys<K: FnMut(A::D)>(&self, mut k: K) {
        self.a.drivekeys(&mut k);
        self.b.drivekeys(|x| if !self.a.member(x) { k(x); });
    }
}
impl<A: Member, B: Member<D = A::D>> Member for Disj<A, B> {
    #[inline(always)]
    fn member(&self, x: A::D) -> bool { self.a.member(x) || self.b.member(x) }
}

// ===== Prod (×) — binary; n-ary by nesting ==============================
// Mode rule: like Compose — drive the first leg, probe the rest.

pub struct Prod<A, B> { pub a: A, pub b: B }

impl<A: Rel, B: Rel<D = A::D>> Rel for Prod<A, B> {
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

// ===== InvStream — `q'` in drive position: flip pairs, no state =========

pub struct InvStream<Q> { pub q: Q }

impl<Q: Rel> Rel for InvStream<Q> where Q::R: Eq + Hash {
    type D = Q::R;
    type R = Q::D;
}
impl<Q: Drive> Drive for InvStream<Q> where Q::R: Eq + Hash {
    #[inline(always)]
    fn drive<K: FnMut(Q::R, Q::D)>(&self, mut k: K) {
        self.q.drive(|d, r| k(r, d));
    }
}

// ===== HashIdx — THE probe-side physical node ===========================
// An eager `HashMap<K, SVec<V>>` with probe access — the probed form of a
// materialized forward index (`.mat_idx()`).

pub struct HashIdx<K: Copy + Eq + Hash, V: Copy> {
    pub idx: HashMap<K, SVec<V>>,
}

impl<K: Copy + Eq + Hash, V: Copy> HashIdx<K, V> {
    /// Forward index: bucket q's values by key. (`.mat_idx()`)
    pub fn mat<Q: Drive<D = K, R = V>>(q: Q) -> Self {
        let mut m: HashMap<K, SVec<V>> = HashMap::new();
        q.drive(|d, r| m.entry(d).or_default().push(r));
        HashIdx { idx: m }
    }
}

impl<K: Copy + Eq + Hash, V: Copy> Rel for HashIdx<K, V> { type D = K; type R = V; }
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

// ===== MatSetSet — materialized membership key-set ======================

pub struct MatSetSet<D: Copy + Eq + Hash> { pub set: HashSet<D> }
impl<D: Copy + Eq + Hash> MatSetSet<D> {
    pub fn build<S: DriveKeys<D = D>>(s: S) -> Self {
        let mut set = HashSet::new();
        s.drivekeys(|x| { set.insert(x); });
        MatSetSet { set }
    }
}
impl<D: Copy + Eq + Hash> KeySet for MatSetSet<D> { type D = D; }
impl<D: Copy + Eq + Hash> Member for MatSetSet<D> {
    #[inline(always)]
    fn member(&self, x: D) -> bool { self.set.contains(&x) }
}

// ===== Bitset — `Vec<u64>`-backed dense KeySet<D = i64> =================
//
// Drop-in replacement for `MatSetSet` when the membership domain is `i64`
// in `1..=n` and dense: trades the HashSet's hash+probe for one bit-test.
// `drivekeys` enumerates set bits via word-scan + `trailing_zeros` so
// iteration cost is proportional to popcount, not the universe size.

pub struct Bitset { pub bs: Vec<u64>, pub n: i64 }

impl Bitset {
    pub fn empty(n: i64) -> Self {
        Bitset { bs: vec![0u64; (n as usize / 64) + 1], n }
    }
    /// Build by driving a `Drive<R = i64>` and setting bits at each emitted value.
    pub fn from_drive<Q: Drive<R = i64>>(n: i64, q: &Q) -> Self {
        let mut b = Self::empty(n);
        q.drive(|_, c| b.set(c));
        b
    }
    /// Build by driving a `DriveKeys<D = i64>` and setting bits at each key.
    pub fn from_setq<S: DriveKeys<D = i64>>(n: i64, s: &S) -> Self {
        let mut b = Self::empty(n);
        s.drivekeys(|c| b.set(c));
        b
    }
    #[inline] pub fn set(&mut self, x: i64) {
        let c = x as usize;
        if c >= 1 && c <= self.n as usize {
            unsafe { *self.bs.get_unchecked_mut(c / 64) |= 1u64 << (c % 64); }
        }
    }
}

impl KeySet for Bitset { type D = i64; }
impl DriveKeys for Bitset {
    #[inline]
    fn drivekeys<K: FnMut(i64)>(&self, mut k: K) {
        for (wi, &w) in self.bs.iter().enumerate() {
            let mut w = w;
            while w != 0 {
                let b = w.trailing_zeros() as usize;
                let c = (wi * 64 + b) as i64;
                if c >= 1 && c <= self.n { k(c); }
                w &= w - 1;
            }
        }
    }
}
impl Member for Bitset {
    #[inline]
    fn member(&self, x: i64) -> bool {
        let c = x as usize;
        c >= 1 && c <= self.n as usize
            && unsafe { (*self.bs.get_unchecked(c / 64) >> (c % 64)) & 1 == 1 }
    }
}

// ===== LCStream (`r ← s`) — drive s, probe r per row ====================
// For r: Probe<D, RK> and s: Drive<D, SV>, produces a drive-only RK → SV.

pub struct LCStream<R, S> { pub r: R, pub s: S }

impl<R: Rel, S: Rel<D = R::D>> Rel for LCStream<R, S> where R::R: Eq + Hash {
    type D = R::R;
    type R = S::R;
}
impl<R: Probe, S: Drive<D = R::D>> Drive for LCStream<R, S> where R::R: Eq + Hash {
    #[inline(always)]
    fn drive<K: FnMut(R::R, S::R)>(&self, mut k: K) {
        self.s.drive(|d, sv| self.r.probe(d, |rk| k(rk, sv)));
    }
}

// `r ← s` with `s : KeySet` — drive s's keys, probe r per key. The key is
// re-emitted as the value (preserving the domain for downstream composition).

pub struct LCSetStream<R, S> { pub r: R, pub s: S }

impl<R: Rel, S: KeySet<D = R::D>> Rel for LCSetStream<R, S> where R::R: Eq + Hash {
    type D = R::R;
    type R = S::D;
}
impl<R: Probe, S: DriveKeys<D = R::D>> Drive for LCSetStream<R, S> where R::R: Eq + Hash {
    #[inline(always)]
    fn drive<K: FnMut(R::R, S::D)>(&self, mut k: K) {
        self.s.drivekeys(|d| self.r.probe(d, |rk| k(rk, d)));
    }
}

// ===== LeftConj (`l ⩘ r`) ===============================================
// Materializes the *value-set* of `l` eagerly (auto-invert, mirroring `←`),
// then intersects with `r`: drivekeys drives `r` filtered by the set;
// member checks both.

pub struct LeftConj<D: Copy + Eq + Hash, R> {
    pub vset: HashSet<D>,
    pub r: R,
}

impl<D: Copy + Eq + Hash, R: KeySet<D = D>> LeftConj<D, R> {
    pub fn build<L: Drive<R = D>>(l: L, r: R) -> Self {
        let mut vset = HashSet::new();
        l.drive(|_, v| { vset.insert(v); });
        LeftConj { vset, r }
    }
}

impl<D: Copy + Eq + Hash, R: KeySet<D = D>> KeySet for LeftConj<D, R> { type D = D; }
impl<D: Copy + Eq + Hash, R: DriveKeys<D = D>> DriveKeys for LeftConj<D, R> {
    #[inline(always)]
    fn drivekeys<K: FnMut(D)>(&self, mut k: K) {
        self.r.drivekeys(|x| if self.vset.contains(&x) { k(x); });
    }
}
impl<D: Copy + Eq + Hash, R: Member<D = D>> Member for LeftConj<D, R> {
    #[inline(always)]
    fn member(&self, x: D) -> bool { self.vset.contains(&x) && self.r.member(x) }
}

// ===== Fold (`▷`) — per-key reduce into an eager cache ==================
// One physical type serves foldl (`.fold`) and count-distinct
// (`.count_distinct`) — they differ only in how the cache is filled.

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
}

impl<D: Copy + Eq + Hash> Fold<D, i64> {
    /// Specialized count-distinct: per-key sort+dedup of an SVec — much
    /// faster than a HashSet per group for the typical small-group case.
    pub fn build_count_distinct<Q>(q: Q) -> Self
    where Q: Drive<D = D>, Q::R: Ord {
        let mut buf: HashMap<D, SVec<Q::R>> = HashMap::new();
        q.drive(|d, v| buf.entry(d).or_default().push(v));
        Fold {
            cache: buf.into_iter().map(|(d, mut vs)| {
                vs.sort_unstable();
                vs.dedup();
                (d, vs.len() as i64)
            }).collect(),
        }
    }
}

impl<D: Copy + Eq + Hash, S: Copy> Rel for Fold<D, S> { type D = D; type R = S; }
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

// ===== DenseFold — `▷` with dense i64-keyed array cache =================
//
// Drop-in replacement for `Fold` when `D = i64` and the key range is a
// known, dense `1..=n`. Backing store is `Vec<S>` (one slot per key) plus a
// parallel `Vec<bool>` presence map. Avoids HashMap probe + entry alloc on
// every reduce step; for Q1 (≤6 group keys via packed byte index), Q2 / Q20
// (per-part), Q18 (per-order), the gain is ~5-10× over `Fold`.

pub struct DenseFold<S: Copy> {
    pub vals: Vec<S>,
    pub seen: Vec<bool>,
}

impl<S: Copy> DenseFold<S> {
    pub fn build<Q, OP>(q: Q, n: i64, init: S, op: OP) -> Self
    where Q: Drive<D = i64>, OP: Fn(S, Q::R) -> S {
        let sz = (n as usize) + 1;
        let mut vals = vec![init; sz];
        let mut seen = vec![false; sz];
        q.drive(|d, v| {
            let i = d as usize;
            if i < sz {
                let s = unsafe { *vals.get_unchecked(i) };
                unsafe {
                    *vals.get_unchecked_mut(i) = op(s, v);
                    *seen.get_unchecked_mut(i) = true;
                }
            }
        });
        DenseFold { vals, seen }
    }
}

impl<S: Copy> Rel for DenseFold<S> { type D = i64; type R = S; }
impl<S: Copy> Drive for DenseFold<S> {
    #[inline(always)]
    fn drive<K: FnMut(i64, S)>(&self, mut k: K) {
        for (i, (&v, &seen)) in self.vals.iter().zip(&self.seen).enumerate() {
            if seen {
                k(i as i64, v);
            }
        }
    }
}
impl<S: Copy> Probe for DenseFold<S> {
    #[inline(always)]
    fn probe<K: FnMut(S)>(&self, x: i64, mut k: K) {
        let i = x as usize;
        if i < self.vals.len() && unsafe { *self.seen.get_unchecked(i) } {
            k(unsafe { *self.vals.get_unchecked(i) });
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(S) -> bool>(&self, x: i64, mut k: K) -> bool {
        let i = x as usize;
        i < self.vals.len() && unsafe { *self.seen.get_unchecked(i) }
            && k(unsafe { *self.vals.get_unchecked(i) })
    }
}

// ===== Map (`↦ f`) — per-row lambda =====================================

pub struct Map<Q, F, S: Copy> {
    pub q: Q,
    pub f: F,
    _phantom: std::marker::PhantomData<S>,
}

impl<Q: Rel, F: Fn(Q::R) -> S, S: Copy> Map<Q, F, S> {
    pub fn new(q: Q, f: F) -> Self { Map { q, f, _phantom: std::marker::PhantomData } }
}

impl<Q: Rel, F: Fn(Q::R) -> S, S: Copy> Rel for Map<Q, F, S> {
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

pub trait QueryExt: Rel + Sized {
    /// Compose two queries (bridge type = self's value type).
    #[inline(always)]
    fn o<B: Rel<D = Self::R>>(self, b: B) -> Compose<Self, B> { Compose { a: self, b } }

    /// Postfix adjoint in drive position — streams flipped pairs, no state.
    #[inline(always)]
    fn inv(self) -> InvStream<Self> where Self::R: Eq + Hash { InvStream { q: self } }

    /// Reify the key set.
    #[inline(always)]
    fn k(self) -> Keys<Self> { Keys { q: self } }

    /// Cartesian product (× / ⊗).
    #[inline(always)]
    fn x<B: Rel<D = Self::D>>(self, b: B) -> Prod<Self, B> { Prod { a: self, b } }

    // Predicate filters — all captured-closure forms of `filt`.
    #[inline(always)] fn eq(self, v: Self::R) -> Filter<Self, impl Pred<Self::R>>
        where Self::R: PartialEq { self.filt(move |x| x == v) }
    #[inline(always)] fn ne(self, v: Self::R) -> Filter<Self, impl Pred<Self::R>>
        where Self::R: PartialEq { self.filt(move |x| x != v) }
    #[inline(always)] fn gt(self, v: Self::R) -> Filter<Self, impl Pred<Self::R>>
        where Self::R: PartialOrd { self.filt(move |x| x > v) }
    #[inline(always)] fn lt(self, v: Self::R) -> Filter<Self, impl Pred<Self::R>>
        where Self::R: PartialOrd { self.filt(move |x| x < v) }
    #[inline(always)] fn ge(self, v: Self::R) -> Filter<Self, impl Pred<Self::R>>
        where Self::R: PartialOrd { self.filt(move |x| x >= v) }
    #[inline(always)] fn le(self, v: Self::R) -> Filter<Self, impl Pred<Self::R>>
        where Self::R: PartialOrd { self.filt(move |x| x <= v) }
    #[inline(always)] fn in_v(self, vs: Vec<Self::R>) -> Filter<Self, impl Pred<Self::R>>
        where Self::R: PartialEq { self.filt(move |x| vs.iter().any(|&v| v == x)) }
    #[inline(always)] fn in_s<S: Member<D = i64>>(self, s: S) -> Filter<Self, InSet<S>>
        where Self: Rel<R = i64> { Filter { a: self, p: InSet(s) } }
    #[inline(always)] fn rx(self, re: &str) -> Filter<Self, impl Pred<Self::R>>
        where Self: Rel<R = &'static str> {
        let re = Regex::new(re).unwrap();
        self.filt(move |s| re.is_match(s))
    }
    #[inline(always)] fn nrx(self, re: &str) -> Filter<Self, impl Pred<Self::R>>
        where Self: Rel<R = &'static str> {
        let re = Regex::new(re).unwrap();
        self.filt(move |s| !re.is_match(s))
    }
    /// Closure-predicate filter — for things like cross-column compares.
    #[inline(always)] fn filt<F: Fn(Self::R) -> bool>(self, f: F) -> Filter<Self, FnP<F>>
        { Filter { a: self, p: FnP(f) } }
    /// Half-open range `[lo, hi)` — Julia `during(lo, hi)`.
    #[inline(always)] fn during(self, lo: Self::R, hi: Self::R) -> Filter<Self, impl Pred<Self::R>>
        where Self::R: PartialOrd { self.filt(move |x| x >= lo && x < hi) }
    /// Closed range `[lo, hi]` — Julia `lo..hi`.
    #[inline(always)] fn between(self, lo: Self::R, hi: Self::R) -> Filter<Self, impl Pred<Self::R>>
        where Self::R: PartialOrd { self.filt(move |x| x >= lo && x <= hi) }

    /// Materialize in probe position (`!q` probed) — eager forward index.
    #[inline(always)]
    fn mat_idx(self) -> HashIdx<Self::D, Self::R> where Self: Drive { HashIdx::mat(self) }

    /// `r ← s` in drive position — drives s, probes r per row.
    #[inline(always)]
    fn lc<S: Rel<D = Self::D>>(self, s: S) -> LCStream<Self, S>
    where Self::R: Eq + Hash { LCStream { r: self, s } }

    /// `r ← s` where s is a KeySet — drives s's keys, probes r, value = key.
    #[inline(always)]
    fn lcs<S: KeySet<D = Self::D>>(self, s: S) -> LCSetStream<Self, S>
    where Self::R: Eq + Hash { LCSetStream { r: self, s } }

    /// `l ⩘ r` — left-driving wedge: materialize l's value-set, intersect r.
    #[inline(always)]
    fn lconj<R: KeySet<D = Self::R>>(self, r: R) -> LeftConj<Self::R, R>
    where Self: Drive, Self::R: Eq + Hash { LeftConj::build(self, r) }

    /// `▷ (op, init)` — per-key foldl into an eager cache.
    #[inline(always)]
    fn fold<OP: Fn(S, Self::R) -> S, S: Copy>(self, init: S, op: OP) -> Fold<Self::D, S>
    where Self: Drive { Fold::build(self, init, op) }

    /// `▷ (op, init)` with a dense i64-keyed `Vec<S>` cache. Use when the
    /// key range is known to be `1..=n` and small/dense enough that
    /// `Vec<S>` of size `n+1` beats the HashMap path of `fold`.
    #[inline(always)]
    fn dense_fold<OP: Fn(S, Self::R) -> S, S: Copy>(self, n: i64, init: S, op: OP)
        -> DenseFold<S>
    where Self: Drive<D = i64> { DenseFold::build(self, n, init, op) }

    /// Specialized count-distinct fold — sorts + dedups the per-key SVec on
    /// finalization, avoiding the HashSet alloc per group.
    #[inline(always)]
    fn count_distinct(self) -> Fold<Self::D, i64>
    where Self: Drive, Self::R: Ord { Fold::build_count_distinct(self) }

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
impl<Q: Rel> QueryExt for Q {}

pub trait SetExt: KeySet + Sized {
    /// `s : q` — restrict q to s's keys.
    #[inline(always)]
    fn o<B: Rel<D = Self::D>>(self, b: B) -> Restrict<Self, B> { Restrict { a: self, b } }

    #[inline(always)]
    fn and<B: KeySet<D = Self::D>>(self, b: B) -> Conj<Self, B> { Conj { a: self, b } }

    #[inline(always)]
    fn or<B: KeySet<D = Self::D>>(self, b: B) -> Disj<Self, B> { Disj { a: self, b } }

    #[inline(always)]
    fn minus<B: KeySet<D = Self::D>>(self, b: B) -> SetDiff<Self, B> { SetDiff { a: self, b } }

    /// Materialize in member position — eager membership HashSet.
    #[inline(always)]
    fn mat_set(self) -> MatSetSet<Self::D> where Self: DriveKeys { MatSetSet::build(self) }
}
impl<S: KeySet> SetExt for S {}

// ===== tests — tiny inline data, every node in every mode ===============

#[cfg(test)]
mod tests {
    use super::*;

    // films: 1 → 10, 2 → 20, 3 → 30 (Vec1); cast: 1 → {7, 8}, 3 → {7} (Many)
    fn films() -> Vec1<i64> { Vec1::from_pairs(3, [(1, 10), (2, 20), (3, 30)]) }
    fn cast() -> Many<i64> { Many::from_pairs(3, [(1, 7), (1, 8), (3, 7)]) }

    fn drive_all<Q: Drive>(q: &Q) -> Vec<(Q::D, Q::R)>
    where Q::D: Ord, Q::R: Ord {
        let mut v = Vec::new();
        q.drive(|d, r| v.push((d, r)));
        v.sort();
        v
    }

    #[test]
    fn leaves() {
        assert_eq!(drive_all(&films()), vec![(1, 10), (2, 20), (3, 30)]);
        assert_eq!(drive_all(&cast()), vec![(1, 7), (1, 8), (3, 7)]);
        let f = films();
        let mut got = Vec::new();
        f.probe(2, |v| got.push(v));
        assert_eq!(got, vec![20]);
        assert!(cast().probe_any(3, |_| true) && !cast().probe_any(2, |_| true));
    }

    #[test]
    fn compose_filter_restrict_prod() {
        let f = films();
        let c = cast();
        // cast ∘ (films probed at cast values)? — compose cast: i64→i64 with films
        assert_eq!(drive_all(&(&c).o(&f)), vec![]); // cast values 7,8 not film keys ≤3
        assert_eq!(drive_all(&(&f).filt(|v| v > 15)), vec![(2, 20), (3, 30)]);
        let u = Universe { n: 2 };
        assert_eq!(drive_all(&u.o(&f)), vec![(1, 10), (2, 20)]);
        assert_eq!(drive_all(&(&f).x(&f)), vec![(1, (10, 10)), (2, (20, 20)), (3, (30, 30))]);
    }

    #[test]
    fn inv_stream() {
        let f = films();
        assert_eq!(drive_all(&(&f).inv()), vec![(10, 1), (20, 2), (30, 3)]);
    }

    #[test]
    fn mat_idx() {
        let f = films();
        let idx = (&f).filt(|v| v > 10).mat_idx();
        let mut got = Vec::new();
        idx.probe(3, |v| got.push(v));
        assert_eq!(got, vec![30]);
        assert!(!idx.probe_any(99, |_| true));
    }

    #[test]
    fn lc_and_folds() {
        let f = films();
        let c = cast();
        // group film-values by cast-person: lc(cast ← films)... r=cast probed,
        // s=films driven: for film d, value f(d), key = each cast member of d.
        let grouped = (&c).lc(&f);
        assert_eq!(drive_all(&grouped), vec![(7, 10), (7, 30), (8, 10)]);
        // fold: count films per person
        let counts = (&c).lc(&f).fold(0i64, |a, _| a + 1);
        assert_eq!(drive_all(&counts), vec![(7, 2), (8, 1)]);
        // dense fold over person ids 1..=8
        let dcounts = (&c).lc(&f).dense_fold(8, 0i64, |a, _| a + 1);
        assert_eq!(drive_all(&dcounts), vec![(7, 2), (8, 1)]);
        // count_distinct
        let cd = (&c).lc(&f).count_distinct();
        assert_eq!(drive_all(&cd), vec![(7, 2), (8, 1)]);
        // scalar
        assert_eq!((&f).unwrap_fold(0i64, |a, v| a + v), 60);
    }

    #[test]
    fn sets_and_bitset() {
        let c = cast();
        let people = (&c).k(); // keyset of cast = films with cast
        let mut keys = Vec::new();
        people.drivekeys(|x| keys.push(x));
        keys.sort(); keys.dedup();
        assert_eq!(keys, vec![1, 3]);
        let ms = (&people).mat_set();
        assert!(ms.member(1) && !ms.member(2));
        let b = Bitset::from_setq(3, &people);
        assert!(b.member(1) && !b.member(2) && b.member(3));
        // conj/disj/diff over Universe
        let u2 = Universe { n: 2 };
        let mut got = Vec::new();
        u2.and(&ms).drivekeys(|x| got.push(x));
        assert_eq!(got, vec![1]);
        let mut got = Vec::new();
        u2.minus(&ms).drivekeys(|x| got.push(x));
        assert_eq!(got, vec![2]);
        let mut got = Vec::new();
        u2.or(&b).drivekeys(|x| { got.push(x); });
        got.sort();
        assert_eq!(got, vec![1, 2, 3]);
    }

    #[test]
    fn lconj_and_map() {
        let f = films();
        let u = Universe { n: 30 };
        // values of films (10, 20, 30) intersected with universe 1..=30
        let w = (&f).lconj(&u);
        let mut got = Vec::new();
        w.drivekeys(|x| got.push(x));
        got.sort();
        assert_eq!(got, vec![10, 20, 30]);
        assert!(w.member(10) && !w.member(11));
        assert_eq!(drive_all(&(&f).map(|v| v * 2)), vec![(1, 20), (2, 40), (3, 60)]);
    }
}
