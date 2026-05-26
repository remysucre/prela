// Top-down CPS engine — the Rust port of Prela.jl.
//
// `drive(k)`/`probe(x,k)`/`probe_any(x,k)` form the CPS protocol; continuations
// are generic FnMut closures, so each query type monomorphizes into a fused
// loop nest at `cargo build` time (the moral equivalent of Julia's JIT, but
// AOT). No `dyn`, no runtime dispatch, no per-row allocation.
//
// `Query<D, R>` is generic over both the domain D and value R:
//   - Leaves (Vec1, Many, Universe) fix D = i64 (entity IDs).
//   - Group-by operators (Fold, BufFold) re-key into arbitrary D types — e.g.
//     `Query<(String, String), f64>` for the Q1 group key.
//
// All non-leaf operators are #[inline(always)]; the runtime cost matches a
// hand-rolled imperative loop.

#![allow(dead_code)]

use regex::Regex;
use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

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

// ===== CPS protocol =====================================================

pub trait Query {
    type D: Copy + Eq + Hash;
    type R: Copy;
    fn drive<K: FnMut(Self::D, Self::R)>(&self, k: K);
    fn probe<K: FnMut(Self::R)>(&self, x: Self::D, k: K);
    fn probe_any<K: FnMut(Self::R) -> bool>(&self, x: Self::D, k: K) -> bool;
}

pub trait SetQ {
    type D: Copy + Eq + Hash;
    fn drivekeys<K: FnMut(Self::D)>(&self, k: K);
    fn member(&self, x: Self::D) -> bool;
}

#[inline(always)]
pub fn member_of<Q: Query>(q: &Q, x: Q::D) -> bool {
    q.probe_any(x, |_| true)
}

// ===== leaf impls =======================================================

impl<R: Copy> Query for Vec1<R> {
    type D = i64;
    type R = R;
    #[inline(always)]
    fn drive<K: FnMut(i64, R)>(&self, mut k: K) {
        for i in 1..self.values.len() {
            k(i as i64, unsafe { *self.values.get_unchecked(i) });
        }
    }
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

impl<R: Copy> Query for Many<R> {
    type D = i64;
    type R = R;
    #[inline(always)]
    fn drive<K: FnMut(i64, R)>(&self, mut k: K) {
        for (i, vs) in self.fwd.iter().enumerate().skip(1) {
            for &v in vs { k(i as i64, v); }
        }
    }
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

// blanket: &Q is itself a Query.
impl<Q: Query + ?Sized> Query for &Q {
    type D = Q::D;
    type R = Q::R;
    #[inline(always)]
    fn drive<K: FnMut(Q::D, Q::R)>(&self, k: K) { (**self).drive(k); }
    #[inline(always)]
    fn probe<K: FnMut(Q::R)>(&self, x: Q::D, k: K) { (**self).probe(x, k); }
    #[inline(always)]
    fn probe_any<K: FnMut(Q::R) -> bool>(&self, x: Q::D, k: K) -> bool { (**self).probe_any(x, k) }
}
impl<S: SetQ + ?Sized> SetQ for &S {
    type D = S::D;
    #[inline(always)]
    fn drivekeys<K: FnMut(S::D)>(&self, k: K) { (**self).drivekeys(k); }
    #[inline(always)]
    fn member(&self, x: S::D) -> bool { (**self).member(x) }
}

// ===== Universe (SetQ over i64) =========================================

#[derive(Copy, Clone)]
pub struct Universe { pub n: i64 }

impl SetQ for Universe {
    type D = i64;
    #[inline(always)]
    fn drivekeys<K: FnMut(i64)>(&self, mut k: K) {
        for i in 1..=self.n { k(i); }
    }
    #[inline(always)]
    fn member(&self, x: i64) -> bool { x >= 1 && x <= self.n }
}

// ===== Compose: a: D → M, b: M → R  ⟹ Compose: D → R =====================

pub struct Compose<A, B> { pub a: A, pub b: B }

impl<A: Query, B: Query<D = A::R>> Query for Compose<A, B>
where A::R: Eq + Hash
{
    type D = A::D;
    type R = B::R;
    #[inline(always)]
    fn drive<K: FnMut(A::D, B::R)>(&self, mut k: K) {
        self.a.drive(|x, m| self.b.probe(m, |r| k(x, r)));
    }
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

pub struct EqP<R: Copy + PartialEq>(pub R);
impl<R: Copy + PartialEq> Pred<R> for EqP<R> {
    #[inline(always)] fn test(&self, v: R) -> bool { v == self.0 }
}
pub struct Ne<R: Copy + PartialEq>(pub R);
impl<R: Copy + PartialEq> Pred<R> for Ne<R> {
    #[inline(always)] fn test(&self, v: R) -> bool { v != self.0 }
}
pub struct Gt<R: Copy + PartialOrd>(pub R);
impl<R: Copy + PartialOrd> Pred<R> for Gt<R> {
    #[inline(always)] fn test(&self, v: R) -> bool { v > self.0 }
}
pub struct Lt<R: Copy + PartialOrd>(pub R);
impl<R: Copy + PartialOrd> Pred<R> for Lt<R> {
    #[inline(always)] fn test(&self, v: R) -> bool { v < self.0 }
}
pub struct Ge<R: Copy + PartialOrd>(pub R);
impl<R: Copy + PartialOrd> Pred<R> for Ge<R> {
    #[inline(always)] fn test(&self, v: R) -> bool { v >= self.0 }
}
pub struct Le<R: Copy + PartialOrd>(pub R);
impl<R: Copy + PartialOrd> Pred<R> for Le<R> {
    #[inline(always)] fn test(&self, v: R) -> bool { v <= self.0 }
}

pub struct InVec<R: Copy + PartialEq>(pub Vec<R>);
impl<R: Copy + PartialEq> Pred<R> for InVec<R> {
    #[inline(always)] fn test(&self, v: R) -> bool { self.0.iter().any(|&x| x == v) }
}

pub struct RegexP(pub Regex);
impl Pred<&'static str> for RegexP {
    #[inline(always)] fn test(&self, v: &'static str) -> bool { self.0.is_match(v) }
}
pub struct NotRegexP(pub Regex);
impl Pred<&'static str> for NotRegexP {
    #[inline(always)] fn test(&self, v: &'static str) -> bool { !self.0.is_match(v) }
}

pub struct InSet<S: SetQ>(pub S);
impl<S: SetQ<D = i64>> Pred<i64> for InSet<S> {
    #[inline(always)] fn test(&self, v: i64) -> bool { self.0.member(v) }
}

/// Closure-typed predicate — used for cross-column compares like
/// `(c_nation ⊗ s_nation).filter(|(c, s)| c == s)`.
pub struct FnP<F>(pub F);
impl<R: Copy, F: Fn(R) -> bool> Pred<R> for FnP<F> {
    #[inline(always)] fn test(&self, v: R) -> bool { (self.0)(v) }
}

// Half-open interval [lo, hi) — for date ranges, the common TPC-H idiom.
pub struct InCO<T: Copy + PartialOrd>(pub T, pub T);
impl<T: Copy + PartialOrd> Pred<T> for InCO<T> {
    #[inline(always)] fn test(&self, v: T) -> bool { v >= self.0 && v < self.1 }
}

// Closed interval [lo, hi]
pub struct InCC<T: Copy + PartialOrd>(pub T, pub T);
impl<T: Copy + PartialOrd> Pred<T> for InCC<T> {
    #[inline(always)] fn test(&self, v: T) -> bool { v >= self.0 && v <= self.1 }
}

impl<A: Query, P: Pred<A::R>> Query for Filter<A, P> {
    type D = A::D;
    type R = A::R;
    #[inline(always)]
    fn drive<K: FnMut(A::D, A::R)>(&self, mut k: K) {
        self.a.drive(|x, v| if self.p.test(v) { k(x, v); });
    }
    #[inline(always)]
    fn probe<K: FnMut(A::R)>(&self, x: A::D, mut k: K) {
        self.a.probe(x, |v| if self.p.test(v) { k(v); });
    }
    #[inline(always)]
    fn probe_any<K: FnMut(A::R) -> bool>(&self, x: A::D, mut k: K) -> bool {
        self.a.probe_any(x, |v| self.p.test(v) && k(v))
    }
}

// ===== Restrict (SetQ : Query) ==========================================

pub struct Restrict<A: SetQ, B: Query> { pub a: A, pub b: B }

impl<A: SetQ, B: Query<D = A::D>> Query for Restrict<A, B> {
    type D = A::D;
    type R = B::R;
    #[inline(always)]
    fn drive<K: FnMut(A::D, B::R)>(&self, mut k: K) {
        self.a.drivekeys(|x| self.b.probe(x, |r| k(x, r)));
    }
    #[inline(always)]
    fn probe<K: FnMut(B::R)>(&self, x: A::D, k: K) {
        if self.a.member(x) { self.b.probe(x, k); }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(B::R) -> bool>(&self, x: A::D, k: K) -> bool {
        self.a.member(x) && self.b.probe_any(x, k)
    }
}

// ===== Keys (Query → SetQ) ==============================================

pub struct Keys<Q: Query> { pub q: Q }

impl<Q: Query> SetQ for Keys<Q> {
    type D = Q::D;
    #[inline(always)]
    fn drivekeys<K: FnMut(Q::D)>(&self, mut k: K) {
        self.q.drive(|x, _| k(x));
    }
    #[inline(always)]
    fn member(&self, x: Q::D) -> bool {
        self.q.probe_any(x, |_| true)
    }
}

// ===== Conj / Disj / SetDiff ============================================

pub struct Conj<A: SetQ, B: SetQ> { pub a: A, pub b: B }
impl<A: SetQ, B: SetQ<D = A::D>> SetQ for Conj<A, B> {
    type D = A::D;
    #[inline(always)]
    fn drivekeys<K: FnMut(A::D)>(&self, mut k: K) {
        self.a.drivekeys(|x| if self.b.member(x) { k(x); });
    }
    #[inline(always)]
    fn member(&self, x: A::D) -> bool { self.a.member(x) && self.b.member(x) }
}

pub struct SetDiff<A: SetQ, B: SetQ> { pub a: A, pub b: B }
impl<A: SetQ, B: SetQ<D = A::D>> SetQ for SetDiff<A, B> {
    type D = A::D;
    #[inline(always)]
    fn drivekeys<K: FnMut(A::D)>(&self, mut k: K) {
        self.a.drivekeys(|x| if !self.b.member(x) { k(x); });
    }
    #[inline(always)]
    fn member(&self, x: A::D) -> bool { self.a.member(x) && !self.b.member(x) }
}

pub struct Disj<A: SetQ, B: SetQ> { pub a: A, pub b: B }
impl<A: SetQ, B: SetQ<D = A::D>> SetQ for Disj<A, B> {
    type D = A::D;
    #[inline(always)]
    fn drivekeys<K: FnMut(A::D)>(&self, mut k: K) {
        self.a.drivekeys(&mut k);
        self.b.drivekeys(|x| if !self.a.member(x) { k(x); });
    }
    #[inline(always)]
    fn member(&self, x: A::D) -> bool { self.a.member(x) || self.b.member(x) }
}

// ===== Prod (×) — binary; n-ary by nesting ==============================

pub struct Prod<A: Query, B: Query> { pub a: A, pub b: B }

impl<A: Query, B: Query<D = A::D>> Query for Prod<A, B> {
    type D = A::D;
    type R = (A::R, B::R);
    #[inline(always)]
    fn drive<K: FnMut(A::D, (A::R, B::R))>(&self, mut k: K) {
        self.a.drive(|x, a| self.b.probe(x, |b| k(x, (a, b))));
    }
    #[inline(always)]
    fn probe<K: FnMut((A::R, B::R))>(&self, x: A::D, mut k: K) {
        self.a.probe(x, |a| self.b.probe(x, |b| k((a, b))));
    }
    #[inline(always)]
    fn probe_any<K: FnMut((A::R, B::R)) -> bool>(&self, x: A::D, mut k: K) -> bool {
        self.a.probe_any(x, |a| self.b.probe_any(x, |b| k((a, b))))
    }
}

// ===== Inv (adjoint) — `q'`. Streaming drive; lazy-cached probe =========
//
// `Inv(q)` for `q : Query<D, R>` is `Query<R, D>`. Drive flips pairs without
// allocation. `probe`/`member`/`drivekeys` lazy-build a `HashMap<R, Vec<D>>`
// on first call and reuse it — so using `q'` on the rhs of a `→` (Compose)
// auto-materializes the inverse index the first time the scan needs it.

pub struct Inv<Q: Query> {
    pub q: Q,
    idx: OnceCell<HashMap<Q::R, Vec<Q::D>>>,
}

impl<Q: Query> Inv<Q> where Q::R: Eq + Hash {
    pub fn new(q: Q) -> Self { Inv { q, idx: OnceCell::new() } }

    fn idx(&self) -> &HashMap<Q::R, Vec<Q::D>> {
        self.idx.get_or_init(|| {
            let mut m: HashMap<Q::R, Vec<Q::D>> = HashMap::new();
            self.q.drive(|d, r| m.entry(r).or_default().push(d));
            m
        })
    }
}

impl<Q: Query> Query for Inv<Q> where Q::R: Eq + Hash {
    type D = Q::R;
    type R = Q::D;
    #[inline(always)]
    fn drive<K: FnMut(Q::R, Q::D)>(&self, mut k: K) {
        self.q.drive(|d, r| k(r, d));
    }
    #[inline(always)]
    fn probe<K: FnMut(Q::D)>(&self, x: Q::R, mut k: K) {
        if let Some(vs) = self.idx().get(&x) {
            for &d in vs { k(d); }
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(Q::D) -> bool>(&self, x: Q::R, mut k: K) -> bool {
        match self.idx().get(&x) {
            Some(vs) => vs.iter().any(|&d| k(d)),
            None => false,
        }
    }
}

// ===== Materialized — lazy-cached Vec + HashMap index ===================
// `!q` materializes once into a Vec<(D, R)> + Dict<D, Vec<R>>. Probes hit
// the dict (O(1)).

pub struct Mat<Q: Query> {
    pub q: Q,
    pairs: OnceCell<Vec<(Q::D, Q::R)>>,
    idx:   OnceCell<HashMap<Q::D, Vec<Q::R>>>,
}

impl<Q: Query> Mat<Q> {
    pub fn new(q: Q) -> Self { Mat { q, pairs: OnceCell::new(), idx: OnceCell::new() } }
    fn pairs(&self) -> &Vec<(Q::D, Q::R)> {
        self.pairs.get_or_init(|| {
            let mut v = Vec::new();
            self.q.drive(|d, r| v.push((d, r)));
            v
        })
    }
    fn idx(&self) -> &HashMap<Q::D, Vec<Q::R>> {
        self.idx.get_or_init(|| {
            let mut m: HashMap<Q::D, Vec<Q::R>> = HashMap::new();
            for &(d, r) in self.pairs() { m.entry(d).or_default().push(r); }
            m
        })
    }
}

impl<Q: Query> Query for Mat<Q> {
    type D = Q::D;
    type R = Q::R;
    #[inline(always)]
    fn drive<K: FnMut(Q::D, Q::R)>(&self, mut k: K) {
        for &(d, r) in self.pairs() { k(d, r); }
    }
    #[inline(always)]
    fn probe<K: FnMut(Q::R)>(&self, x: Q::D, mut k: K) {
        if let Some(vs) = self.idx().get(&x) {
            for &r in vs { k(r); }
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(Q::R) -> bool>(&self, x: Q::D, mut k: K) -> bool {
        match self.idx().get(&x) {
            Some(vs) => vs.iter().any(|&r| k(r)),
            None => false,
        }
    }
}

// ===== MatSet — materialize a SetQ ======================================

pub struct MatSet<S: SetQ> {
    pub s: S,
    keys: OnceCell<Vec<S::D>>,
    set:  OnceCell<HashSet<S::D>>,
}

impl<S: SetQ> MatSet<S> {
    pub fn new(s: S) -> Self { MatSet { s, keys: OnceCell::new(), set: OnceCell::new() } }
    fn keys(&self) -> &Vec<S::D> {
        self.keys.get_or_init(|| {
            let mut v = Vec::new();
            self.s.drivekeys(|x| v.push(x));
            v
        })
    }
    fn set(&self) -> &HashSet<S::D> {
        self.set.get_or_init(|| self.keys().iter().copied().collect())
    }
}

impl<S: SetQ> SetQ for MatSet<S> {
    type D = S::D;
    #[inline(always)]
    fn drivekeys<K: FnMut(S::D)>(&self, mut k: K) {
        for &x in self.keys() { k(x); }
    }
    #[inline(always)]
    fn member(&self, x: S::D) -> bool { self.set().contains(&x) }
}

// ===== LeftCompose (`r ← s`) — drive s, probe r; lazy probe cache =======
//
// For r: Query<D, RK> and s: Query<D, SV>, produces Query<RK, SV>. Driving
// walks s and probes r per row (matches the Julia semantics). For probe
// access (e.g. when used as the rhs of a `→` or in a `−`), lazy-builds a
// `HashMap<RK, Vec<SV>>` on first call.

pub struct LeftCompose<R: Query, S: Query> {
    pub r: R,
    pub s: S,
    idx: OnceCell<HashMap<R::R, Vec<S::R>>>,
}

impl<R: Query, S: Query<D = R::D>> LeftCompose<R, S> where R::R: Eq + Hash {
    pub fn new(r: R, s: S) -> Self { LeftCompose { r, s, idx: OnceCell::new() } }
    fn idx(&self) -> &HashMap<R::R, Vec<S::R>> {
        self.idx.get_or_init(|| {
            let mut m: HashMap<R::R, Vec<S::R>> = HashMap::new();
            self.s.drive(|d, sv| self.r.probe(d, |rk| m.entry(rk).or_default().push(sv)));
            m
        })
    }
}

impl<R: Query, S: Query<D = R::D>> Query for LeftCompose<R, S> where R::R: Eq + Hash {
    type D = R::R;
    type R = S::R;
    #[inline(always)]
    fn drive<K: FnMut(R::R, S::R)>(&self, mut k: K) {
        self.s.drive(|d, sv| self.r.probe(d, |rk| k(rk, sv)));
    }
    #[inline(always)]
    fn probe<K: FnMut(S::R)>(&self, x: R::R, mut k: K) {
        if let Some(vs) = self.idx().get(&x) {
            for &v in vs { k(v); }
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(S::R) -> bool>(&self, x: R::R, mut k: K) -> bool {
        match self.idx().get(&x) {
            Some(vs) => vs.iter().any(|&v| k(v)),
            None => false,
        }
    }
}

// `r ← s` with `s : SetQ<D>` — drive s's keys, probe r per key. SetQ has
// no values, so we re-emit the key as the value (preserving the domain for
// downstream composition). Result Query<RK, D>.

pub struct LeftComposeSet<R: Query, S: SetQ> {
    pub r: R,
    pub s: S,
    idx: OnceCell<HashMap<R::R, Vec<S::D>>>,
}

impl<R: Query, S: SetQ<D = R::D>> LeftComposeSet<R, S> where R::R: Eq + Hash {
    pub fn new(r: R, s: S) -> Self { LeftComposeSet { r, s, idx: OnceCell::new() } }
    fn idx(&self) -> &HashMap<R::R, Vec<S::D>> {
        self.idx.get_or_init(|| {
            let mut m: HashMap<R::R, Vec<S::D>> = HashMap::new();
            self.s.drivekeys(|d| self.r.probe(d, |rk| m.entry(rk).or_default().push(d)));
            m
        })
    }
}

impl<R: Query, S: SetQ<D = R::D>> Query for LeftComposeSet<R, S> where R::R: Eq + Hash {
    type D = R::R;
    type R = S::D;
    #[inline(always)]
    fn drive<K: FnMut(R::R, S::D)>(&self, mut k: K) {
        self.s.drivekeys(|d| self.r.probe(d, |rk| k(rk, d)));
    }
    #[inline(always)]
    fn probe<K: FnMut(S::D)>(&self, x: R::R, mut k: K) {
        if let Some(vs) = self.idx().get(&x) {
            for &v in vs { k(v); }
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(S::D) -> bool>(&self, x: R::R, mut k: K) -> bool {
        match self.idx().get(&x) {
            Some(vs) => vs.iter().any(|&v| k(v)),
            None => false,
        }
    }
}

// ===== LeftConj (`l ⩘ r`) ===============================================
//
// Materializes the *value-set* of `l` (auto-invert, mirroring `←`), then
// drives `r` and member-checks per row. For `l : Query<A, B>` you intersect
// against `r : SetQ<B>` — no need to write `l'` manually.

pub struct LeftConj<L: Query, R: SetQ> {
    pub l: L,
    pub r: R,
    vset: OnceCell<HashSet<L::R>>,
}

impl<L: Query, R: SetQ<D = L::R>> LeftConj<L, R> where L::R: Eq + Hash {
    pub fn new(l: L, r: R) -> Self { LeftConj { l, r, vset: OnceCell::new() } }
    fn vset(&self) -> &HashSet<L::R> {
        self.vset.get_or_init(|| {
            let mut s = HashSet::new();
            self.l.drive(|_, v| { s.insert(v); });
            s
        })
    }
}

impl<L: Query, R: SetQ<D = L::R>> SetQ for LeftConj<L, R> where L::R: Eq + Hash {
    type D = L::R;
    #[inline(always)]
    fn drivekeys<K: FnMut(L::R)>(&self, mut k: K) {
        let vs = self.vset();
        self.r.drivekeys(|x| if vs.contains(&x) { k(x); });
    }
    #[inline(always)]
    fn member(&self, x: L::R) -> bool { self.vset().contains(&x) && self.r.member(x) }
}

// ===== Fold (`▷ (op, init)`) — per-key foldl, lazy-cached ===============

pub struct Fold<Q: Query, OP, S: Copy> {
    pub q: Q,
    pub op: OP,
    pub init: S,
    cache: OnceCell<HashMap<Q::D, S>>,
}

impl<Q: Query, OP: Fn(S, Q::R) -> S, S: Copy> Fold<Q, OP, S> {
    pub fn new(q: Q, op: OP, init: S) -> Self { Fold { q, op, init, cache: OnceCell::new() } }
    fn cache(&self) -> &HashMap<Q::D, S> {
        self.cache.get_or_init(|| {
            let mut m: HashMap<Q::D, S> = HashMap::new();
            self.q.drive(|d, v| {
                let s = m.get(&d).copied().unwrap_or(self.init);
                m.insert(d, (self.op)(s, v));
            });
            m
        })
    }
}

impl<Q: Query, OP: Fn(S, Q::R) -> S, S: Copy> Query for Fold<Q, OP, S> {
    type D = Q::D;
    type R = S;
    #[inline(always)]
    fn drive<K: FnMut(Q::D, S)>(&self, mut k: K) {
        for (&d, &s) in self.cache() { k(d, s); }
    }
    #[inline(always)]
    fn probe<K: FnMut(S)>(&self, x: Q::D, mut k: K) {
        if let Some(&s) = self.cache().get(&x) { k(s); }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(S) -> bool>(&self, x: Q::D, mut k: K) -> bool {
        match self.cache().get(&x) { Some(&s) => k(s), None => false }
    }
}

// ===== BufFold (`▷ f`) — per-key buffered reduce, lazy-cached ===========

pub struct BufFold<Q: Query, F, S: Copy> {
    pub q: Q,
    pub f: F,
    cache: OnceCell<HashMap<Q::D, S>>,
}

impl<Q: Query, F: Fn(&[Q::R]) -> S, S: Copy> BufFold<Q, F, S> {
    pub fn new(q: Q, f: F) -> Self { BufFold { q, f, cache: OnceCell::new() } }
    fn cache(&self) -> &HashMap<Q::D, S> {
        self.cache.get_or_init(|| {
            let mut buf: HashMap<Q::D, Vec<Q::R>> = HashMap::new();
            self.q.drive(|d, v| buf.entry(d).or_default().push(v));
            buf.into_iter().map(|(d, vs)| (d, (self.f)(&vs))).collect()
        })
    }
}

impl<Q: Query, F: Fn(&[Q::R]) -> S, S: Copy> Query for BufFold<Q, F, S> {
    type D = Q::D;
    type R = S;
    #[inline(always)]
    fn drive<K: FnMut(Q::D, S)>(&self, mut k: K) {
        for (&d, &s) in self.cache() { k(d, s); }
    }
    #[inline(always)]
    fn probe<K: FnMut(S)>(&self, x: Q::D, mut k: K) {
        if let Some(&s) = self.cache().get(&x) { k(s); }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(S) -> bool>(&self, x: Q::D, mut k: K) -> bool {
        match self.cache().get(&x) { Some(&s) => k(s), None => false }
    }
}

// ===== Map (`↦ f`) — per-row lambda =====================================

pub struct Map<Q: Query, F, S: Copy> {
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
    #[inline(always)]
    fn drive<K: FnMut(Q::D, S)>(&self, mut k: K) {
        self.q.drive(|d, v| k(d, (self.f)(v)));
    }
    #[inline(always)]
    fn probe<K: FnMut(S)>(&self, x: Q::D, mut k: K) {
        self.q.probe(x, |v| k((self.f)(v)));
    }
    #[inline(always)]
    fn probe_any<K: FnMut(S) -> bool>(&self, x: Q::D, mut k: K) -> bool {
        self.q.probe_any(x, |v| k((self.f)(v)))
    }
}

// ===== Scalar (`⊵ (op, init)`) — no-group foldl =========================
//
// Drives the whole inner query and folds every value into one S. Exposed
// as a method `.unwrap_fold(op, init) -> S` since composing a "single-value
// Query" further isn't useful in Rust.

// ===== operators (method-only surface) ==================================

pub trait QueryExt: Query + Sized {
    /// Compose two queries (bridge type = self's value type).
    #[inline(always)]
    fn o<B: Query<D = Self::R>>(self, b: B) -> Compose<Self, B>
    where Self::R: Eq + Hash { Compose { a: self, b } }

    /// Postfix adjoint — `q'`. Streams pairs; lazy-builds reverse index on probe.
    #[inline(always)]
    fn inv(self) -> Inv<Self> where Self::R: Eq + Hash { Inv::new(self) }

    /// Reify the value type as a SetQ (Keys).
    #[inline(always)]
    fn k(self) -> Keys<Self> { Keys { q: self } }

    /// Cartesian product (× / ⊗).
    #[inline(always)]
    fn x<B: Query<D = Self::D>>(self, b: B) -> Prod<Self, B> { Prod { a: self, b } }

    // Predicate filters.
    #[inline(always)] fn eq(self, v: Self::R) -> Filter<Self, EqP<Self::R>>
        where Self::R: PartialEq { Filter { a: self, p: EqP(v) } }
    #[inline(always)] fn ne(self, v: Self::R) -> Filter<Self, Ne<Self::R>>
        where Self::R: PartialEq { Filter { a: self, p: Ne(v) } }
    #[inline(always)] fn gt(self, v: Self::R) -> Filter<Self, Gt<Self::R>>
        where Self::R: PartialOrd { Filter { a: self, p: Gt(v) } }
    #[inline(always)] fn lt(self, v: Self::R) -> Filter<Self, Lt<Self::R>>
        where Self::R: PartialOrd { Filter { a: self, p: Lt(v) } }
    #[inline(always)] fn ge(self, v: Self::R) -> Filter<Self, Ge<Self::R>>
        where Self::R: PartialOrd { Filter { a: self, p: Ge(v) } }
    #[inline(always)] fn le(self, v: Self::R) -> Filter<Self, Le<Self::R>>
        where Self::R: PartialOrd { Filter { a: self, p: Le(v) } }
    #[inline(always)] fn in_v(self, vs: Vec<Self::R>) -> Filter<Self, InVec<Self::R>>
        where Self::R: PartialEq { Filter { a: self, p: InVec(vs) } }
    #[inline(always)] fn in_s<S: SetQ<D = i64>>(self, s: S) -> Filter<Self, InSet<S>>
        where Self: Query<R = i64> { Filter { a: self, p: InSet(s) } }
    #[inline(always)] fn rx(self, re: &str) -> Filter<Self, RegexP>
        where Self: Query<R = &'static str> { Filter { a: self, p: RegexP(Regex::new(re).unwrap()) } }
    #[inline(always)] fn nrx(self, re: &str) -> Filter<Self, NotRegexP>
        where Self: Query<R = &'static str> { Filter { a: self, p: NotRegexP(Regex::new(re).unwrap()) } }
    /// Closure-predicate filter — for things like cross-column compares.
    #[inline(always)] fn filt<F: Fn(Self::R) -> bool>(self, f: F) -> Filter<Self, FnP<F>>
        { Filter { a: self, p: FnP(f) } }
    /// Half-open range `[lo, hi)` — Julia `during(lo, hi)`.
    #[inline(always)] fn during(self, lo: Self::R, hi: Self::R) -> Filter<Self, InCO<Self::R>>
        where Self::R: PartialOrd { Filter { a: self, p: InCO(lo, hi) } }
    /// Closed range `[lo, hi]` — Julia `lo..hi`.
    #[inline(always)] fn between(self, lo: Self::R, hi: Self::R) -> Filter<Self, InCC<Self::R>>
        where Self::R: PartialOrd { Filter { a: self, p: InCC(lo, hi) } }

    /// Materialize (`!q`).
    #[inline(always)]
    fn mat(self) -> Mat<Self> { Mat::new(self) }

    /// `r ← s` — left-compose. Drives s, probes r per row.
    #[inline(always)]
    fn lc<S: Query<D = Self::D>>(self, s: S) -> LeftCompose<Self, S>
    where Self::R: Eq + Hash { LeftCompose::new(self, s) }

    /// `r ← s` where s is a SetQ — drives s's keys, probes r, value = key.
    #[inline(always)]
    fn lcs<S: SetQ<D = Self::D>>(self, s: S) -> LeftComposeSet<Self, S>
    where Self::R: Eq + Hash { LeftComposeSet::new(self, s) }

    /// `▷ (op, init)` — per-key foldl.
    #[inline(always)]
    fn fold<OP: Fn(S, Self::R) -> S, S: Copy>(self, init: S, op: OP) -> Fold<Self, OP, S> {
        Fold::new(self, op, init)
    }

    /// `▷ f` — per-key buffered reduce.
    #[inline(always)]
    fn buf_fold<F: Fn(&[Self::R]) -> S, S: Copy>(self, f: F) -> BufFold<Self, F, S> {
        BufFold::new(self, f)
    }

    /// `↦ f` — per-row map.
    #[inline(always)]
    fn map<F: Fn(Self::R) -> S, S: Copy>(self, f: F) -> Map<Self, F, S> {
        Map::new(self, f)
    }

    /// `⊵ (op, init)` — no-group foldl. Drives the whole query, returns scalar.
    #[inline(always)]
    fn unwrap_fold<OP: Fn(S, Self::R) -> S, S: Copy>(&self, init: S, op: OP) -> S {
        let mut acc = init;
        self.drive(|_, v| acc = op(acc, v));
        acc
    }
}
impl<Q: Query> QueryExt for Q {}

pub trait SetQExt: SetQ + Sized {
    /// `s : q` — restrict q to s's keys.
    #[inline(always)]
    fn o<B: Query<D = Self::D>>(self, b: B) -> Restrict<Self, B> { Restrict { a: self, b } }

    #[inline(always)]
    fn and<B: SetQ<D = Self::D>>(self, b: B) -> Conj<Self, B> { Conj { a: self, b } }

    #[inline(always)]
    fn or<B: SetQ<D = Self::D>>(self, b: B) -> Disj<Self, B> { Disj { a: self, b } }

    #[inline(always)]
    fn minus<B: SetQ<D = Self::D>>(self, b: B) -> SetDiff<Self, B> { SetDiff { a: self, b } }

    /// Materialize the set.
    #[inline(always)]
    fn mat_set(self) -> MatSet<Self> { MatSet::new(self) }

    /// `l ⩘ r` — left-driving wedge.
    #[inline(always)]
    fn lconj<R: SetQ<D = Self::D>>(self, _r: R) -> Self {
        // SetQ form of left-conj: same as plain `and` for SetQ inputs.
        // (The Query-on-left form is on QueryExt::lconj.)
        unimplemented!("use QueryExt::lconj for Query on lhs; SetQ ⩘ SetQ is just `and`")
    }
}
impl<S: SetQ> SetQExt for S {}

// Extra QueryExt method for ⩘ when LHS is a Query (auto-invert).
pub trait LConjExt: Query + Sized where Self::R: Eq + Hash {
    #[inline(always)]
    fn lconj<R: SetQ<D = Self::R>>(self, r: R) -> LeftConj<Self, R> {
        LeftConj::new(self, r)
    }
}
impl<Q: Query> LConjExt for Q where Q::R: Eq + Hash {}
