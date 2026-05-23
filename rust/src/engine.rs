// Top-down CPS engine — the Rust port of Prela.jl.
//
// `drive(k)`/`probe(x,k)`/`probe_any(x,k)` form the CPS protocol; continuations
// are generic FnMut closures, so each query type monomorphizes into a fused
// loop nest at `cargo build` time (the moral equivalent of Julia's JIT, but
// AOT). No `dyn`, no runtime dispatch, no per-row allocation.

#![allow(dead_code)]

use regex::Regex;

// ===== leaf storage =====================================================
// `Vec1<R>` — total 1:1 relation; entity-id → R (one value per id). The
// vector is 1-indexed (slot 0 is a sentinel).
// `Many<R>` — multi-valued / partial; dense forward index Vec<Vec<R>>
// addressed by .id; empty slot for missing keys.

pub struct Vec1<R: Copy> {
    pub values: Vec<R>,           // values[id] is the one value for id
}

pub struct Many<R: Copy> {
    pub fwd: Vec<Vec<R>>,         // fwd[id] is the list of values for id
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
    type R: Copy;
    fn drive<K: FnMut(i64, Self::R)>(&self, k: K);
    fn probe<K: FnMut(Self::R)>(&self, x: i64, k: K);
    fn probe_any<K: FnMut(Self::R) -> bool>(&self, x: i64, k: K) -> bool;
}

pub trait SetQ {
    fn drivekeys<K: FnMut(i64)>(&self, k: K);
    fn member(&self, x: i64) -> bool;
}

#[inline(always)]
pub fn member_of<Q: Query>(q: &Q, x: i64) -> bool {
    q.probe_any(x, |_| true)
}

// ===== leaf impls =======================================================

impl<R: Copy> Query for Vec1<R> {
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
    type R = R;
    #[inline(always)]
    fn drive<K: FnMut(i64, R)>(&self, mut k: K) {
        for (i, vs) in self.fwd.iter().enumerate().skip(1) {
            for &v in vs {
                k(i as i64, v);
            }
        }
    }
    #[inline(always)]
    fn probe<K: FnMut(R)>(&self, x: i64, mut k: K) {
        let i = x as usize;
        if i >= 1 && i < self.fwd.len() {
            for &v in unsafe { self.fwd.get_unchecked(i) } {
                k(v);
            }
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(R) -> bool>(&self, x: i64, mut k: K) -> bool {
        let i = x as usize;
        if i >= 1 && i < self.fwd.len() {
            for &v in unsafe { self.fwd.get_unchecked(i) } {
                if k(v) {
                    return true;
                }
            }
        }
        false
    }
}

// blanket: a reference to a Query/SetQ is itself a Query/SetQ. Lets queries
// borrow leaves so the same dataset can host many queries.

impl<Q: Query + ?Sized> Query for &Q {
    type R = Q::R;
    #[inline(always)]
    fn drive<K: FnMut(i64, Q::R)>(&self, k: K) { (**self).drive(k); }
    #[inline(always)]
    fn probe<K: FnMut(Q::R)>(&self, x: i64, k: K) { (**self).probe(x, k); }
    #[inline(always)]
    fn probe_any<K: FnMut(Q::R) -> bool>(&self, x: i64, k: K) -> bool { (**self).probe_any(x, k) }
}
impl<S: SetQ + ?Sized> SetQ for &S {
    #[inline(always)]
    fn drivekeys<K: FnMut(i64)>(&self, k: K) { (**self).drivekeys(k); }
    #[inline(always)]
    fn member(&self, x: i64) -> bool { (**self).member(x) }
}

// ===== Universe (SetQ) ==================================================

#[derive(Copy, Clone)]
pub struct Universe { pub n: i64 }

impl SetQ for Universe {
    #[inline(always)]
    fn drivekeys<K: FnMut(i64)>(&self, mut k: K) {
        for i in 1..=self.n { k(i); }
    }
    #[inline(always)]
    fn member(&self, x: i64) -> bool { x >= 1 && x <= self.n }
}

// ===== Compose ==========================================================
// a: domain→i64, b: i64→R  ⟹  Compose: domain→R

pub struct Compose<A, B> { pub a: A, pub b: B }

impl<A, B> Query for Compose<A, B>
where A: Query<R = i64>, B: Query, B::R: Copy
{
    type R = B::R;
    #[inline(always)]
    fn drive<K: FnMut(i64, B::R)>(&self, mut k: K) {
        self.a.drive(|x, m| self.b.probe(m, |r| k(x, r)));
    }
    #[inline(always)]
    fn probe<K: FnMut(B::R)>(&self, x: i64, mut k: K) {
        self.a.probe(x, |m| self.b.probe(m, |r| k(r)));
    }
    #[inline(always)]
    fn probe_any<K: FnMut(B::R) -> bool>(&self, x: i64, mut k: K) -> bool {
        self.a.probe_any(x, |m| self.b.probe_any(m, |r| k(r)))
    }
}

// ===== Filter (predicates) ==============================================

pub struct Filter<A, P> { pub a: A, pub p: P }

pub trait Pred<R> {
    fn test(&self, v: R) -> bool;
}

pub struct Eq<R: Copy + PartialEq>(pub R);
impl<R: Copy + PartialEq> Pred<R> for Eq<R> {
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

// `InSet(s)`: test that the value lives in the given SetQ — used to express
// `r → s` (Filter a Query r by membership of its value in a set).
pub struct InSet<S: SetQ>(pub S);
impl<S: SetQ> Pred<i64> for InSet<S> {
    #[inline(always)] fn test(&self, v: i64) -> bool { self.0.member(v) }
}

impl<A, P> Query for Filter<A, P>
where A: Query, P: Pred<A::R>
{
    type R = A::R;
    #[inline(always)]
    fn drive<K: FnMut(i64, A::R)>(&self, mut k: K) {
        self.a.drive(|x, v| if self.p.test(v) { k(x, v); });
    }
    #[inline(always)]
    fn probe<K: FnMut(A::R)>(&self, x: i64, mut k: K) {
        self.a.probe(x, |v| if self.p.test(v) { k(v); });
    }
    #[inline(always)]
    fn probe_any<K: FnMut(A::R) -> bool>(&self, x: i64, mut k: K) -> bool {
        self.a.probe_any(x, |v| self.p.test(v) && k(v))
    }
}

// ===== Restrict (SetQ : Query) ==========================================
// a: SetQ over domain, b: Query domain→R ⟹ restrict b to a's keys.

pub struct Restrict<A: SetQ, B: Query> { pub a: A, pub b: B }

impl<A: SetQ, B: Query> Query for Restrict<A, B> {
    type R = B::R;
    #[inline(always)]
    fn drive<K: FnMut(i64, B::R)>(&self, mut k: K) {
        self.a.drivekeys(|x| self.b.probe(x, |r| k(x, r)));
    }
    #[inline(always)]
    fn probe<K: FnMut(B::R)>(&self, x: i64, k: K) {
        if self.a.member(x) { self.b.probe(x, k); }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(B::R) -> bool>(&self, x: i64, k: K) -> bool {
        self.a.member(x) && self.b.probe_any(x, k)
    }
}

// ===== Keys (Query → SetQ) ==============================================

pub struct Keys<Q: Query> { pub q: Q }

impl<Q: Query> SetQ for Keys<Q> {
    #[inline(always)]
    fn drivekeys<K: FnMut(i64)>(&self, mut k: K) {
        self.q.drive(|x, _| k(x));
    }
    #[inline(always)]
    fn member(&self, x: i64) -> bool {
        self.q.probe_any(x, |_| true)
    }
}

// ===== Conj (SetQ ∧ SetQ) ===============================================

pub struct Conj<A: SetQ, B: SetQ> { pub a: A, pub b: B }
impl<A: SetQ, B: SetQ> SetQ for Conj<A, B> {
    #[inline(always)]
    fn drivekeys<K: FnMut(i64)>(&self, mut k: K) {
        self.a.drivekeys(|x| if self.b.member(x) { k(x); });
    }
    #[inline(always)]
    fn member(&self, x: i64) -> bool { self.a.member(x) && self.b.member(x) }
}

// ===== SetDiff (SetQ - SetQ) ============================================

pub struct SetDiff<A: SetQ, B: SetQ> { pub a: A, pub b: B }
impl<A: SetQ, B: SetQ> SetQ for SetDiff<A, B> {
    #[inline(always)]
    fn drivekeys<K: FnMut(i64)>(&self, mut k: K) {
        self.a.drivekeys(|x| if !self.b.member(x) { k(x); });
    }
    #[inline(always)]
    fn member(&self, x: i64) -> bool { self.a.member(x) && !self.b.member(x) }
}

// ===== Disj (SetQ ∨ SetQ) ===============================================

pub struct Disj<A: SetQ, B: SetQ> { pub a: A, pub b: B }
impl<A: SetQ, B: SetQ> SetQ for Disj<A, B> {
    #[inline(always)]
    fn drivekeys<K: FnMut(i64)>(&self, mut k: K) {
        self.a.drivekeys(&mut k);
        self.b.drivekeys(|x| if !self.a.member(x) { k(x); });
    }
    #[inline(always)]
    fn member(&self, x: i64) -> bool { self.a.member(x) || self.b.member(x) }
}

// ===== Prod (×) — binary, n-ary by nesting ==============================

pub struct Prod<A: Query, B: Query> { pub a: A, pub b: B }

impl<A: Query, B: Query> Query for Prod<A, B> {
    type R = (A::R, B::R);
    #[inline(always)]
    fn drive<K: FnMut(i64, (A::R, B::R))>(&self, mut k: K) {
        self.a.drive(|x, a| self.b.probe(x, |b| k(x, (a, b))));
    }
    #[inline(always)]
    fn probe<K: FnMut((A::R, B::R))>(&self, x: i64, mut k: K) {
        self.a.probe(x, |a| self.b.probe(x, |b| k((a, b))));
    }
    #[inline(always)]
    fn probe_any<K: FnMut((A::R, B::R)) -> bool>(&self, x: i64, mut k: K) -> bool {
        self.a.probe_any(x, |a| self.b.probe_any(x, |b| k((a, b))))
    }
}

// ===== operators (method-only surface) ==================================
//
// All operators are methods on the `QueryExt` / `SetQExt` extension traits.
// `.o` unifies "compose two Queries" (bridge = value) with "restrict a SetQ
// by a Query" (bridge = key) — they're the same algebraic operation
// (composition of relations), and Rust trait dispatch picks the right impl
// from the receiver's trait. No type implements both Query and SetQ, so the
// dispatch is unambiguous.
//
// All methods are `#[inline(always)]` and resolve to the same struct
// constructions a hand-written tree would produce — zero runtime cost.
// The traits are not object-safe (methods are `Self`-generic in their
// inputs); we only ever use static dispatch.
//
// Receivers move `self`. For a borrowed leaf use
// `(&d.foo).method(...)` — the parens are needed because `&` binds looser
// than `.` and Rust won't auto-borrow a `self` (by-value) receiver.

// `.o` unifies "compose two Queries" (bridge = value) with "restrict a SetQ by
// a Query" (bridge = key). They're the same algebraic op — composition of
// relations — and trait dispatch picks the right one because no type
// implements both Query and SetQ.

pub trait QueryExt: Query + Sized {
    #[inline(always)] fn o<B: Query>(self, b: B) -> Compose<Self, B>
        where Self: Query<R = i64> { Compose { a: self, b } }
    #[inline(always)] fn k(self) -> Keys<Self> { Keys { q: self } }
    #[inline(always)] fn x<B: Query>(self, b: B) -> Prod<Self, B> { Prod { a: self, b } }
    #[inline(always)] fn eq(self, v: Self::R) -> Filter<Self, Eq<Self::R>>
        where Self::R: PartialEq { Filter { a: self, p: Eq(v) } }
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
    #[inline(always)] fn in_s<S: SetQ>(self, s: S) -> Filter<Self, InSet<S>>
        where Self: Query<R = i64> { Filter { a: self, p: InSet(s) } }
    #[inline(always)] fn rx(self, re: &str) -> Filter<Self, RegexP>
        where Self: Query<R = &'static str> { Filter { a: self, p: RegexP(Regex::new(re).unwrap()) } }
    #[inline(always)] fn nrx(self, re: &str) -> Filter<Self, NotRegexP>
        where Self: Query<R = &'static str> { Filter { a: self, p: NotRegexP(Regex::new(re).unwrap()) } }
}
impl<Q: Query> QueryExt for Q {}

pub trait SetQExt: SetQ + Sized {
    #[inline(always)] fn o<B: Query>(self, b: B) -> Restrict<Self, B> { Restrict { a: self, b } }
    #[inline(always)] fn and<B: SetQ>(self, b: B) -> Conj<Self, B> { Conj { a: self, b } }
    #[inline(always)] fn or<B: SetQ>(self, b: B) -> Disj<Self, B> { Disj { a: self, b } }
    #[inline(always)] fn minus<B: SetQ>(self, b: B) -> SetDiff<Self, B> { SetDiff { a: self, b } }
}
impl<S: SetQ> SetQExt for S {}
