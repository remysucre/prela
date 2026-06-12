// Pull (external-iterator) protocol — the experiment counterpart to the
// push/CPS engine in engine.rs.
//
// Same node types, same eager state, second access protocol:
//
//   Iterate:   Query + fn iter()        — pull mirror of Drive
//   ProbeIter: Query + fn probe_iter /  — pull mirror of Probe;
//              fn member_p              member_p(x) = probe_iter(x).next().is_some()
//
// Every node implements exactly the modes its push twin does (Disj stays
// probe-only, Union/InvStream/GroupBy stay drive-only), with the same mode
// rule propagated through the combinators — a pull-mode error is the same
// compile error the push protocol gives.
//
// FAITHFULNESS RULE: the pull hot path never calls into push `drive` /
// `probe` / `probe_any`. Membership inside pull combinators goes through
// `member_p`; leaves with a cheap direct test (Universe bound check, Bitset
// bit-test, MatSet hash lookup) override it with the same test push's
// `member` uses, and their `probe_iter` is the member-gated `once`-shape so
// the defaulted `.next().is_some()` inlines to the direct test anyway.
//
// Pipeline breakers get pull-side builders (`build_pull`, `collect_p`, …)
// that consume their input via `.fold()`/`.for_each()` — internal iteration,
// which std's adapters forward through `fold`/`try_fold`; this is pull's
// best shot at matching the push loop nests. The `*_next` builder variants
// consume via a raw `for` loop (external `next()` calls) to isolate the
// resumption-state cost — see the `pull-next` suite.

use ahash::{AHashMap as HashMap, AHashSet as HashSet};
use smallvec::SmallVec;
use std::hash::Hash;

use crate::engine::*;

/// Same inline bucket capacity as the push side (engine.rs `SVec`).
type SVec<T> = SmallVec<[T; 4]>;

// ===== the pull mode traits =============================================

pub trait Iterate: Query {
    fn iter(&self) -> impl Iterator<Item = (Self::D, Self::R)> + '_;
}

pub trait ProbeIter: Query {
    fn probe_iter(&self, x: Self::D) -> impl Iterator<Item = Self::R> + '_;
    /// Domain-membership test — pull mirror of `Probe::member`. The default
    /// is the universal definition (first probe result short-circuits);
    /// leaves with a cheaper direct test override it.
    #[inline(always)]
    fn member_p(&self, x: Self::D) -> bool {
        self.probe_iter(x).next().is_some()
    }
}

// blanket: &T inherits T's pull modes.
impl<T: Iterate + ?Sized> Iterate for &T {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (T::D, T::R)> + '_ { (**self).iter() }
}
impl<T: ProbeIter + ?Sized> ProbeIter for &T {
    #[inline(always)]
    fn probe_iter(&self, x: T::D) -> impl Iterator<Item = T::R> + '_ {
        (**self).probe_iter(x)
    }
    #[inline(always)]
    fn member_p(&self, x: T::D) -> bool { (**self).member_p(x) }
}

// ===== leaves ===========================================================
// Probe policy mirrors push: `.get` is the single bounds check, so missing
// keys (`NO_ID` holes, out-of-universe ids) yield empty iterators for free.

impl<R: Copy> Iterate for VecRel<R> {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (usize, R)> + '_ {
        self.values.iter().copied().enumerate()
    }
}
impl<R: Copy> ProbeIter for VecRel<R> {
    #[inline(always)]
    fn probe_iter(&self, x: usize) -> impl Iterator<Item = R> + '_ {
        self.values.get(x).copied().into_iter()
    }
}

impl<R: Copy> Iterate for MultiRel<R> {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (usize, R)> + '_ {
        self.fwd.iter().enumerate()
            .flat_map(|(i, vs)| vs.iter().copied().map(move |v| (i, v)))
    }
}
impl<R: Copy> ProbeIter for MultiRel<R> {
    #[inline(always)]
    fn probe_iter(&self, x: usize) -> impl Iterator<Item = R> + '_ {
        self.fwd.get(x).into_iter().flatten().copied()
    }
}

impl Iterate for Universe {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        (0..self.n).map(|i| (i, i))
    }
}
impl ProbeIter for Universe {
    #[inline(always)]
    fn probe_iter(&self, x: usize) -> impl Iterator<Item = usize> + '_ {
        (x < self.n).then_some(x).into_iter()
    }
    #[inline(always)]
    fn member_p(&self, x: usize) -> bool { x < self.n }
}

// ===== Compose ==========================================================

impl<A: Iterate, B: ProbeIter<D = A::R>> Iterate for Compose<A, B> {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (A::D, B::R)> + '_ {
        self.a.iter()
            .flat_map(move |(x, m)| self.b.probe_iter(m).map(move |r| (x, r)))
    }
}
impl<A: ProbeIter, B: ProbeIter<D = A::R>> ProbeIter for Compose<A, B> {
    #[inline(always)]
    fn probe_iter(&self, x: A::D) -> impl Iterator<Item = B::R> + '_ {
        self.a.probe_iter(x).flat_map(move |m| self.b.probe_iter(m))
    }
}

// ===== Filter ===========================================================

impl<A: Iterate, F: Fn(A::R) -> bool> Iterate for Filter<A, F> {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (A::D, A::R)> + '_ {
        self.a.iter().filter(move |&(_, v)| (self.p)(v))
    }
}
impl<A: ProbeIter, F: Fn(A::R) -> bool> ProbeIter for Filter<A, F> {
    #[inline(always)]
    fn probe_iter(&self, x: A::D) -> impl Iterator<Item = A::R> + '_ {
        self.a.probe_iter(x).filter(move |&v| (self.p)(v))
    }
}

// ===== Restrict (`a : b`) — b consumed via member_p only ================

impl<A: Iterate, B: ProbeIter<D = A::R>> Iterate for Restrict<A, B> {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (A::D, A::R)> + '_ {
        self.a.iter().filter(move |&(_, v)| self.b.member_p(v))
    }
}
impl<A: ProbeIter, B: ProbeIter<D = A::R>> ProbeIter for Restrict<A, B> {
    #[inline(always)]
    fn probe_iter(&self, x: A::D) -> impl Iterator<Item = A::R> + '_ {
        self.a.probe_iter(x).filter(move |&v| self.b.member_p(v))
    }
}

// ===== Diff / Disj / Union ==============================================

impl<A: Iterate, B: ProbeIter<D = A::D>> Iterate for Diff<A, B> {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (A::D, A::R)> + '_ {
        self.a.iter().filter(move |&(x, _)| !self.b.member_p(x))
    }
}
impl<A: ProbeIter, B: ProbeIter<D = A::D>> ProbeIter for Diff<A, B> {
    #[inline(always)]
    fn probe_iter(&self, x: A::D) -> impl Iterator<Item = A::R> + '_ {
        (!self.b.member_p(x))
            .then(|| self.a.probe_iter(x))
            .into_iter()
            .flatten()
    }
    #[inline(always)]
    fn member_p(&self, x: A::D) -> bool { self.a.member_p(x) && !self.b.member_p(x) }
}

// `∨` stays probe-only in the pull protocol too (no Iterate impl).
impl<A: ProbeIter, B: ProbeIter<D = A::D>> ProbeIter for Disj<A, B> {
    #[inline(always)]
    fn probe_iter(&self, x: A::D) -> impl Iterator<Item = A::D> + '_ {
        self.member_p(x).then_some(x).into_iter()
    }
    #[inline(always)]
    fn member_p(&self, x: A::D) -> bool { self.a.member_p(x) || self.b.member_p(x) }
}

// Bag union stays drive-only: chain a then b.
impl<A: Iterate, B: Iterate<D = A::D, R = A::R>> Iterate for Union<A, B> {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (A::D, A::R)> + '_ {
        self.a.iter().chain(self.b.iter())
    }
}

// ===== Prod (× / ⊗, and ∧) — probe b per a-value, like push =============

impl<A: Iterate, B: ProbeIter<D = A::D>> Iterate for Prod<A, B> {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (A::D, (A::R, B::R))> + '_ {
        self.a.iter()
            .flat_map(move |(x, a)| self.b.probe_iter(x).map(move |b| (x, (a, b))))
    }
}
impl<A: ProbeIter, B: ProbeIter<D = A::D>> ProbeIter for Prod<A, B> {
    #[inline(always)]
    fn probe_iter(&self, x: A::D) -> impl Iterator<Item = (A::R, B::R)> + '_ {
        self.a.probe_iter(x)
            .flat_map(move |a| self.b.probe_iter(x).map(move |b| (a, b)))
    }
    /// Flat short-circuit AND — the conj-position fast path, mirroring
    /// push's `member` override (no pair value is built).
    #[inline(always)]
    fn member_p(&self, x: A::D) -> bool { self.a.member_p(x) && self.b.member_p(x) }
}

// ===== InvStream — drive-only flip ======================================

impl<Q: Iterate> Iterate for InvStream<Q> where Q::R: Eq + Hash {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (Q::R, Q::D)> + '_ {
        self.q.iter().map(|(d, r)| (r, d))
    }
}

// ===== HashIdx / MatSet — probe-only physical nodes =====================

impl<K: Copy + Eq + Hash, V: Copy> ProbeIter for HashIdx<K, V> {
    #[inline(always)]
    fn probe_iter(&self, x: K) -> impl Iterator<Item = V> + '_ {
        self.idx.get(&x).into_iter().flatten().copied()
    }
}

impl<D: Copy + Eq + Hash> ProbeIter for MatSet<D> {
    #[inline(always)]
    fn probe_iter(&self, x: D) -> impl Iterator<Item = D> + '_ {
        self.set.contains(&x).then_some(x).into_iter()
    }
    #[inline(always)]
    fn member_p(&self, x: D) -> bool { self.set.contains(&x) }
}

// ===== Bitset ===========================================================

/// Set-bit iterator over one word — the pull mirror of `Bitset::drive`'s
/// word scan (`trailing_zeros` + clear-lowest-bit).
struct WordBits { w: u64, base: usize }
impl Iterator for WordBits {
    type Item = usize;
    #[inline(always)]
    fn next(&mut self) -> Option<usize> {
        if self.w == 0 { return None; }
        let b = self.w.trailing_zeros() as usize;
        self.w &= self.w - 1;
        Some(self.base + b)
    }
}

impl Iterate for Bitset {
    #[inline]
    fn iter(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.bs.iter().enumerate()
            .flat_map(|(wi, &w)| WordBits { w, base: wi * 64 }.map(|x| (x, x)))
    }
}
impl ProbeIter for Bitset {
    #[inline]
    fn probe_iter(&self, x: usize) -> impl Iterator<Item = usize> + '_ {
        self.member_p(x).then_some(x).into_iter()
    }
    #[inline]
    fn member_p(&self, x: usize) -> bool {
        self.bs.get(x / 64).is_some_and(|&w| (w >> (x % 64)) & 1 == 1)
    }
}

// ===== GroupBy — drive-only =============================================

impl<S: Iterate, R: ProbeIter<D = S::D>> Iterate for GroupBy<S, R> where R::R: Eq + Hash {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (R::R, S::R)> + '_ {
        self.src.iter()
            .flat_map(move |(d, sv)| self.key.probe_iter(d).map(move |rk| (rk, sv)))
    }
}

// ===== Fold / DenseFold — caches with pull access ========================

impl<D: Copy + Eq + Hash, S: Copy> Iterate for Fold<D, S> {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (D, S)> + '_ {
        self.cache.iter().map(|(&d, &s)| (d, s))
    }
}
impl<D: Copy + Eq + Hash, S: Copy> ProbeIter for Fold<D, S> {
    #[inline(always)]
    fn probe_iter(&self, x: D) -> impl Iterator<Item = S> + '_ {
        self.cache.get(&x).copied().into_iter()
    }
}

impl<S: Copy> Iterate for DenseFold<S> {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (usize, S)> + '_ {
        self.vals.iter().zip(&self.seen).enumerate()
            .filter_map(|(i, (&v, &seen))| seen.then_some((i, v)))
    }
}
impl<S: Copy> ProbeIter for DenseFold<S> {
    #[inline(always)]
    fn probe_iter(&self, x: usize) -> impl Iterator<Item = S> + '_ {
        self.vals.get(x)
            .and_then(|&v| self.seen[x].then_some(v))
            .into_iter()
    }
}

// ===== Map ==============================================================

impl<Q: Iterate, F: Fn(Q::R) -> S, S: Copy> Iterate for Map<Q, F, S> {
    #[inline(always)]
    fn iter(&self) -> impl Iterator<Item = (Q::D, S)> + '_ {
        self.q.iter().map(move |(d, v)| (d, (self.f)(v)))
    }
}
impl<Q: ProbeIter, F: Fn(Q::R) -> S, S: Copy> ProbeIter for Map<Q, F, S> {
    #[inline(always)]
    fn probe_iter(&self, x: Q::D) -> impl Iterator<Item = S> + '_ {
        self.q.probe_iter(x).map(move |v| (self.f)(v))
    }
}

// ===== pull-side builders for the pipeline breakers =====================
// Same physical state as the push constructors; only the consumption
// protocol differs. `.for_each()`/`.fold()` consume via internal iteration
// (std forwards through `fold`/`try_fold`); the `*_next` variants consume
// via a raw `for` loop (external `next()` calls) for the consumption-style
// axis of the experiment.

impl<D: Copy + Eq + Hash, S: Copy> Fold<D, S> {
    /// Pull twin of `Fold::build` — per-key foldl, consumed via `.for_each()`.
    pub fn build_pull<Q, OP>(q: Q, init: S, op: OP) -> Self
    where Q: Iterate<D = D>, OP: Fn(S, Q::R) -> S {
        let mut m: HashMap<D, S> = HashMap::new();
        q.iter().for_each(|(d, v)| {
            let s = m.entry(d).or_insert(init);
            *s = op(*s, v);
        });
        Fold { cache: m }
    }

    /// Pull twin of `Fold::build_buf` — whole-group buffered reduce.
    pub fn build_buf_pull<Q, F>(q: Q, f: F) -> Self
    where Q: Iterate<D = D>, F: Fn(SVec<Q::R>) -> S {
        let mut buf: HashMap<D, SVec<Q::R>> = HashMap::new();
        q.iter().for_each(|(d, v)| buf.entry(d).or_default().push(v));
        Fold { cache: buf.into_iter().map(|(d, vs)| (d, f(vs))).collect() }
    }

    /// `build_pull` with raw-`next()` consumption (a `for` loop) — the
    /// consumption-style axis. Used only by the `pull-next` suite.
    pub fn build_pull_next<Q, OP>(q: Q, init: S, op: OP) -> Self
    where Q: Iterate<D = D>, OP: Fn(S, Q::R) -> S {
        let mut m: HashMap<D, S> = HashMap::new();
        for (d, v) in q.iter() {
            let s = m.entry(d).or_insert(init);
            *s = op(*s, v);
        }
        Fold { cache: m }
    }

    /// `build_buf_pull` with raw-`next()` consumption. `pull-next` only.
    pub fn build_buf_pull_next<Q, F>(q: Q, f: F) -> Self
    where Q: Iterate<D = D>, F: Fn(SVec<Q::R>) -> S {
        let mut buf: HashMap<D, SVec<Q::R>> = HashMap::new();
        for (d, v) in q.iter() {
            buf.entry(d).or_default().push(v);
        }
        Fold { cache: buf.into_iter().map(|(d, vs)| (d, f(vs))).collect() }
    }
}

impl<S: Copy> DenseFold<S> {
    /// Pull twin of `DenseFold::build`.
    // No idiomatic JOB/TPC-H port needs a dense fold (that's the optimized
    // variant, out of scope); kept for protocol parity, exercised by tests.
    #[allow(dead_code)]
    pub fn build_pull<Q, OP>(q: Q, n: usize, init: S, op: OP) -> Self
    where Q: Iterate<D = usize>, OP: Fn(S, Q::R) -> S {
        let mut vals = vec![init; n];
        let mut seen = vec![false; n];
        q.iter().for_each(|(d, v)| {
            if let Some(s) = vals.get_mut(d) {
                *s = op(*s, v);
                seen[d] = true;
            }
        });
        DenseFold { vals, seen }
    }
}

impl Bitset {
    /// Pull twin of `Bitset::over`.
    // No idiomatic JOB/TPC-H port builds a bitset (optimized variant only);
    // kept for protocol parity, exercised by tests.
    #[allow(dead_code)]
    pub fn over_pull<Q: Iterate<R = usize>>(u: Universe, q: &Q) -> Self {
        let mut b = Self::empty(u);
        q.iter().for_each(|(_, c)| b.set(c));
        b
    }
}

/// Pull twin of `FromQuery` — `q.collect_p::<T>()` materializes via
/// internal iteration over `q.iter()`.
pub trait FromQueryPull<Q: Iterate>: Sized {
    fn from_rel_pull(q: Q) -> Self;
}

impl<Q: Iterate> FromQueryPull<Q> for HashIdx<Q::D, Q::R> {
    fn from_rel_pull(q: Q) -> Self {
        let mut m: HashMap<Q::D, SVec<Q::R>> = HashMap::new();
        q.iter().for_each(|(d, r)| m.entry(d).or_default().push(r));
        HashIdx { idx: m }
    }
}

impl<Q: Iterate> FromQueryPull<Q> for MatSet<Q::R>
where Q::R: Eq + Hash {
    fn from_rel_pull(q: Q) -> Self {
        let mut set = HashSet::new();
        q.iter().for_each(|(_, v)| { set.insert(v); });
        MatSet { set }
    }
}

/// `HashIdx` collect with raw-`next()` consumption. `pull-next` only.
pub fn collect_hash_idx_next<Q: Iterate>(q: Q) -> HashIdx<Q::D, Q::R> {
    let mut m: HashMap<Q::D, SVec<Q::R>> = HashMap::new();
    for (d, r) in q.iter() {
        m.entry(d).or_default().push(r);
    }
    HashIdx { idx: m }
}

// ===== PullExt — pull spellings of the QueryExt sinks ===================
// The mode-agnostic constructors (`.o`, `.x`, `.in_s`, `.group_by`, …) are
// shared with the push side via QueryExt; only the consuming sinks get a
// `_p` spelling here.

pub trait PullExt: Query + Sized {
    /// `▷ (op, init)` — pull twin of `.fold`.
    #[inline(always)]
    fn fold_p<OP: Fn(S, Self::R) -> S, S: Copy>(self, init: S, op: OP) -> Fold<Self::D, S>
    where Self: Iterate { Fold::build_pull(self, init, op) }

    /// `▷ f` — pull twin of `.buf_fold`.
    #[inline(always)]
    fn buf_fold_p<F: Fn(SVec<Self::R>) -> S, S: Copy>(self, f: F) -> Fold<Self::D, S>
    where Self: Iterate { Fold::build_buf_pull(self, f) }

    /// Pull twin of `.dense_fold`.
    // No idiomatic port needs it (optimized variant only); kept for
    // protocol parity, exercised by unit tests.
    #[allow(dead_code)]
    #[inline(always)]
    fn dense_fold_p<OP: Fn(S, Self::R) -> S, S: Copy>(self, n: usize, init: S, op: OP)
        -> DenseFold<S>
    where Self: Iterate<D = usize> { DenseFold::build_pull(self, n, init, op) }

    /// Pull twin of `.count_distinct` — same `length ∘ unique` closure.
    #[inline(always)]
    fn count_distinct_p(self) -> Fold<Self::D, i64>
    where Self: Iterate, Self::R: Ord {
        self.buf_fold_p(|mut vs| { vs.sort_unstable(); vs.dedup(); vs.len() as i64 })
    }

    /// Pull twin of `.collect` — materialize via `FromQueryPull`.
    #[inline(always)]
    fn collect_p<T: FromQueryPull<Self>>(self) -> T where Self: Iterate {
        T::from_rel_pull(self)
    }
}
impl<Q: Query> PullExt for Q {}

// ===== tests — pull semantics against the push fixtures =================

#[cfg(test)]
mod tests {
    use super::*;

    fn films() -> VecRel<usize> { VecRel::from_pairs(3, [(0, 10), (1, 20), (2, 30)]) }
    fn cast() -> MultiRel<usize> { MultiRel::from_pairs(3, [(0, 7), (0, 8), (2, 7)]) }

    fn iter_all<Q: Iterate>(q: &Q) -> Vec<(Q::D, Q::R)>
    where Q::D: Ord, Q::R: Ord {
        let mut v: Vec<_> = q.iter().collect();
        v.sort();
        v
    }

    /// Pull drive must equal push drive on every node shape.
    fn push_all<Q: crate::engine::Drive>(q: &Q) -> Vec<(Q::D, Q::R)>
    where Q::D: Ord, Q::R: Ord {
        let mut v = Vec::new();
        q.drive(|d, r| v.push((d, r)));
        v.sort();
        v
    }

    #[test]
    fn compose_iterate_matches_push() {
        let f = films();
        let c = cast();
        let u = Universe { n: 2 };
        // identity ∘ relation, relation ∘ relation, deep chain
        assert_eq!(iter_all(&u.o(&f)), push_all(&u.o(&f)));
        assert_eq!(iter_all(&(&c).o(&f)), push_all(&(&c).o(&f)));
        let person_of_film = MultiRel::from_pairs(31, [(10, 1usize), (30, 0), (30, 2)]);
        let chain = (&f).o(&person_of_film).o(&f);
        assert_eq!(iter_all(&chain), push_all(&chain));
        assert_eq!(iter_all(&chain), vec![(0, 20), (2, 10), (2, 30)]);
        // probe_iter side of compose
        let got: Vec<_> = (&f).o(&person_of_film).probe_iter(2).collect();
        assert_eq!(got, vec![0, 2]);
    }

    #[test]
    fn prod_probe_iter_pairs_per_key() {
        let f = films(); // 0→10, 1→20, 2→30
        let c = cast();  // 0→{7,8}, 2→{7}
        // probe at a shared key: every (a, b) pair at that key, a-major
        let got: Vec<_> = (&c).x(&f).probe_iter(0).collect();
        assert_eq!(got, vec![(7, 10), (8, 10)]);
        // iterate matches push (lhs multiplicity, b probed per a-value)
        assert_eq!(iter_all(&(&c).and(&f)), push_all(&(&c).and(&f)));
        assert_eq!(iter_all(&(&c).and(&f)),
                   vec![(0, (7, 10)), (0, (8, 10)), (2, (7, 30))]);
        // member_p is the flat short-circuit AND, no pair built
        let conj = (&f).filt(|v| v > 15).and(&c);
        assert!(conj.member_p(2) && !conj.member_p(1) && !conj.member_p(0));
        let never = (&f).filt(|_| false);
        let trap = (&f).filt(|_| -> bool { panic!("second leg must not be probed") });
        assert!(!(&never).and(&trap).member_p(1));
    }

    #[test]
    fn restrict_member_semantics() {
        let f = films();
        let b = MultiRel::from_pairs(31, [(10, 99usize), (20, 88)]);
        let r = (&f).in_s(&b);
        // a's value flows through; b consumed via member_p only
        assert_eq!(iter_all(&r), vec![(0, 10), (1, 20)]);
        assert_eq!(iter_all(&r), push_all(&r));
        assert!(r.member_p(0) && r.member_p(1) && !r.member_p(2) && !r.member_p(3));
        let got: Vec<_> = r.probe_iter(1).chain(r.probe_iter(2)).collect();
        assert_eq!(got, vec![20]);
        // leaf member_p overrides agree with push member
        let u = Universe { n: 2 };
        assert!(u.member_p(1) && !u.member_p(2));
        let ms: MatSet<_> = (&f).collect_p();
        assert!(ms.member_p(10) && !ms.member_p(11));
    }

    #[test]
    fn diff_disj_union_pull() {
        let c = cast();
        let u1 = Universe { n: 1 };
        let u2 = Universe { n: 2 };
        let dd = (&c).minus(u1);
        assert_eq!(iter_all(&dd), vec![(2, 7)]);
        assert!(dd.member_p(2) && !dd.member_p(0) && !dd.member_p(1));
        let got: Vec<_> = dd.probe_iter(2).chain(dd.probe_iter(0)).collect();
        assert_eq!(got, vec![7]);
        let f = films();
        let ms: MatSet<_> = (&f).collect_p();
        assert!(u2.or(&ms).member_p(1) && u2.or(&ms).member_p(10) && !u2.or(&ms).member_p(5));
        let got: Vec<_> = u2.or(&ms).probe_iter(10).collect();
        assert_eq!(got, vec![10]);
        assert_eq!(iter_all(&(&c).union(&c)), push_all(&(&c).union(&c)));
        let b = Bitset::over_pull(Universe { n: 9 }, &c); // values {7, 8}
        assert!(b.member_p(7) && b.member_p(8) && !b.member_p(0) && !b.member_p(NO_ID));
        assert_eq!(iter_all(&b), vec![(7, 7), (8, 8)]);
    }

    #[test]
    fn pull_folds_match_push() {
        let f = films();
        let c = cast();
        let push = (&f).group_by(&c).fold(0i64, |a, _| a + 1);
        let pull = (&f).group_by(&c).fold_p(0i64, |a, _| a + 1);
        assert_eq!(iter_all(&pull), push_all(&push));
        let dpull = (&f).group_by(&c).dense_fold_p(9, 0i64, |a, _| a + 1);
        assert_eq!(iter_all(&dpull), vec![(7, 2), (8, 1)]);
        let got: Vec<_> = dpull.probe_iter(7).chain(dpull.probe_iter(3)).collect();
        assert_eq!(got, vec![2]);
        let cd = (&f).group_by(&c).union((&f).group_by(&c).filt(|v| v == 10)).count_distinct_p();
        assert_eq!(iter_all(&cd), vec![(7, 2), (8, 1)]);
        let next = Fold::build_pull_next((&f).group_by(&c), 0i64, |a, _| a + 1);
        assert_eq!(iter_all(&next), vec![(7, 2), (8, 1)]);
        let bnext = Fold::build_buf_pull_next((&f).group_by(&c), |vs| vs.len() as i64);
        assert_eq!(iter_all(&bnext), vec![(7, 2), (8, 1)]);
        let idx = collect_hash_idx_next((&f).inv());
        let got: Vec<_> = idx.probe_iter(30).collect();
        assert_eq!(got, vec![2]);
        let idx2: HashIdx<_, _> = (&f).map(|v| v * 2).collect_p();
        let got: Vec<_> = idx2.probe_iter(1).collect();
        assert_eq!(got, vec![40]);
    }
}
