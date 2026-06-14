// Heartbeat-scheduled (chili) parallel driver over the `ParDrive` spine.
//
// The engine is push-CPS: a query is one fused `drive` from the root down a
// nest of probe continuations, with no iterator to hand a data-parallel
// runtime. We parallelize the only axis that drives — the root scan — with a
// divide-and-conquer recursion over the root row window: split in half, `join`
// the halves (chili promotes the split to another worker only when its
// heartbeat fires — near-free otherwise), and merge the two partial sinks with
// the sink's monoid. A 1-1 step bottoms out as the sequential base case (zero
// `join`s, zero overhead); only fat windows actually fork.
//
// The driver is generic over the sink: `mk` drives a leaf window into a fresh
// partial, `merge` is the associative combine. `par_min_row` (in
// queries::helpers) and the fold builders specialize it.

use crate::engine::{Dense, DenseFold, Drive, Fold, ParDrive, Probe, Query, QueryExt};
use ahash::AHashMap as HashMap;
use chili::{Scope, ThreadPool};
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global switch: when set, the `p*` fold dispatchers run on chili's global
/// pool; otherwise they fall back to the sequential combinators. The bench
/// binary flips it; the verification suite leaves it off. (JOB's `min_row`
/// reads the same flag — see queries::helpers.)
pub static PARALLEL: AtomicBool = AtomicBool::new(false);
/// Leaf window for the TPC-H fold scans (lineitem-scale roots).
const GRAIN: usize = 16_384;

#[inline]
pub fn is_parallel() -> bool { PARALLEL.load(Ordering::Relaxed) }

/// Divide-and-conquer over the root window `[lo, hi)`. Leaves (≤ `grain` rows)
/// build a partial sink with `mk`; internal nodes `join` their halves and
/// `merge`. The plan `q` is shared by reference across workers (read-only
/// during the scan — only the sink accumulates, privately per branch).
pub fn par_reduce<Q, S, MK, MG>(
    s: &mut Scope,
    q: &Q,
    lo: usize,
    hi: usize,
    grain: usize,
    mk: &MK,
    merge: &MG,
) -> S
where
    Q: ParDrive + Sync,
    S: Send,
    MK: Fn(&Q, usize, usize) -> S + Sync,
    MG: Fn(S, S) -> S + Sync,
{
    if hi - lo <= grain {
        mk(q, lo, hi)
    } else {
        let mid = lo + (hi - lo) / 2;
        let (a, b) = s.join(
            |s| par_reduce(s, q, lo, mid, grain, mk, merge),
            |s| par_reduce(s, q, mid, hi, grain, mk, merge),
        );
        merge(a, b)
    }
}

/// Run the whole root scan of `q` in parallel on `pool`, reducing with
/// (`mk`, `merge`). `mk(q, lo, hi)` must drive exactly the window `[lo, hi)`
/// (via `q.drive_range`) into a fresh partial; `merge` must be associative.
pub fn par_run<Q, S, MK, MG>(pool: &ThreadPool, q: &Q, grain: usize, mk: MK, merge: MG) -> S
where
    Q: ParDrive + Sync,
    S: Send,
    MK: Fn(&Q, usize, usize) -> S + Sync,
    MG: Fn(S, S) -> S + Sync,
{
    let n = q.rows();
    let mut scope = pool.scope();
    par_reduce(&mut scope, q, 0, n, grain, &mk, &merge)
}

/// Parallel global reduction — the `unwrap_fold` (no group-by) sink. Each
/// worker folds its window with `op` into a private accumulator, then the
/// partials combine with `combine`. For an associative-commutative aggregate
/// (sum, min, max, count) this is byte-identical to the sequential fold.
/// `op` need not equal `combine`: `op` absorbs a ROW into the state, `combine`
/// merges two STATES (e.g. q6 revenue: op = `acc + e*dc`, combine = `+`).
pub fn par_unwrap_fold<Q, S, OP, CB>(
    pool: &ThreadPool,
    q: &Q,
    grain: usize,
    init: S,
    op: OP,
    combine: CB,
) -> S
where
    Q: ParDrive + Sync,
    S: Copy + Send + Sync,
    OP: Fn(S, Q::R) -> S + Sync,
    CB: Fn(S, S) -> S + Sync,
{
    par_run(
        pool,
        q,
        grain,
        |q, lo, hi| {
            let mut acc = init;
            q.drive_range(lo, hi, |_, v| acc = op(acc, v));
            acc
        },
        |a, b| combine(a, b),
    )
}

/// Parallel per-key fold — the `.group_by(k).fold(init, op)` sink. Each worker
/// folds its window into a private `HashMap<key, state>` (entries only for the
/// keys it touches), then the partial maps merge: shared keys `combine`, new
/// keys insert. Returns the same eager `Fold` the sequential path builds, so
/// downstream drive/probe/sort is unchanged.
///
/// `op` absorbs a row into a key's state; `combine` merges two states for the
/// same key. As with `par_unwrap_fold`, `init` must be `combine`'s identity.
/// HashMap partials scale with *touched keys per window*, not the key space —
/// so fine grain (skew balancing) stays cheap at any cardinality.
pub fn par_fold<Q, S, OP, CB>(
    pool: &ThreadPool,
    q: &Q,
    grain: usize,
    init: S,
    op: OP,
    combine: CB,
) -> Fold<Q::D, S>
where
    Q: ParDrive + Sync,
    Q::D: Eq + Hash + Send + Sync,
    S: Copy + Send + Sync,
    OP: Fn(S, Q::R) -> S + Sync,
    CB: Fn(S, S) -> S + Sync,
{
    let cache = par_run(
        pool,
        q,
        grain,
        |q, lo, hi| {
            let mut m: HashMap<Q::D, S> = HashMap::new();
            q.drive_range(lo, hi, |d, v| {
                let s = m.entry(d).or_insert(init);
                *s = op(*s, v);
            });
            m
        },
        |a, b| {
            // Merge the smaller map into the larger to bound entry churn.
            let (mut big, small) = if a.len() >= b.len() { (a, b) } else { (b, a) };
            for (d, sv) in small {
                big.entry(d).and_modify(|s| *s = combine(*s, sv)).or_insert(sv);
            }
            big
        },
    );
    Fold { cache }
}

/// Parallel per-key fold into a DENSE `Vec<S>` cache — the `.group_by(k)
/// .dense_fold(n, init, op)` sink. Each worker folds its window into a private
/// `vec![init; n]` + presence map, then partials merge elementwise. Returns the
/// same `DenseFold` the sequential path builds.
///
/// Trade-off vs `par_fold`: each leaf allocates an n-slot vector and each merge
/// is O(n), so the win holds only while `n` is SMALL/dense (q1's 288, by-nation
/// 25, …). For high-cardinality keys (per-customer/part) the per-leaf alloc and
/// O(n·leaves) merge dominate — use `par_fold` (HashMap, scales with touched
/// keys) there, or a coarse grain here. `init` must be `combine`'s identity.
pub fn par_dense_fold<Q, S, OP, CB>(
    pool: &ThreadPool,
    q: &Q,
    grain: usize,
    n: usize,
    init: S,
    op: OP,
    combine: CB,
) -> DenseFold<S, Q::D>
where
    Q: ParDrive + Sync,
    Q::D: Dense,
    S: Copy + Send + Sync,
    OP: Fn(S, Q::R) -> S + Sync,
    CB: Fn(S, S) -> S + Sync,
{
    let (vals, seen) = par_run(
        pool,
        q,
        grain,
        |q, lo, hi| {
            let mut vals = vec![init; n];
            let mut seen = vec![false; n];
            q.drive_range(lo, hi, |d, v| {
                let i = d.idx();
                if let Some(s) = vals.get_mut(i) {
                    *s = op(*s, v);
                    seen[i] = true;
                }
            });
            (vals, seen)
        },
        |(mut va, mut sa), (vb, sb)| {
            for i in 0..va.len() {
                if sb[i] {
                    va[i] = if sa[i] { combine(va[i], vb[i]) } else { vb[i] };
                    sa[i] = true;
                }
            }
            (va, sa)
        },
    );
    DenseFold { vals, seen, _d: PhantomData }
}

// ===== flag-dispatched fold combinators (TPC-H) ========================
// One call site, two behaviors: sequential (the tuned combinator) when the
// PARALLEL flag is off, parallel (par_*) when on. The combiner is supplied
// always — unused in the sequential branch, the monoid merge in the parallel
// one. The pre-fold plan `q` is `ParDrive` (lineitem/partsupp/orders/customer
// scan spines). Return type is identical across branches so one query body
// compiles for both.

/// Per-key fold into a HashMap-backed `Fold`. The high-cardinality choice
/// (per-order/part/customer): partial maps scale with touched keys, not the
/// key space — so memory stays bounded where a dense `Vec` would blow up.
pub fn pfold<Q, S, OP, CB>(q: Q, init: S, op: OP, combine: CB) -> Fold<Q::D, S>
where
    Q: ParDrive + Sync,
    Q::D: Eq + Hash + Send + Sync,
    S: Copy + Send + Sync,
    OP: Fn(S, Q::R) -> S + Sync,
    CB: Fn(S, S) -> S + Sync,
{
    if is_parallel() {
        par_fold(ThreadPool::global(), &q, GRAIN, init, op, combine)
    } else {
        q.fold(init, op)
    }
}

/// Per-key fold into a dense `Vec`-backed `DenseFold`. Use ONLY for small key
/// spaces (q1's 288 packed groups): the parallel build allocates an n-slot
/// vector per leaf, so n must stay tiny.
pub fn pdense_fold<Q, S, OP, CB>(q: Q, n: usize, init: S, op: OP, combine: CB) -> DenseFold<S, Q::D>
where
    Q: ParDrive + Sync,
    Q::D: Dense,
    S: Copy + Send + Sync,
    OP: Fn(S, Q::R) -> S + Sync,
    CB: Fn(S, S) -> S + Sync,
{
    if is_parallel() {
        par_dense_fold(ThreadPool::global(), &q, GRAIN, n, init, op, combine)
    } else {
        q.dense_fold(n, init, op)
    }
}

/// Global (no-group) fold to a single state.
pub fn punwrap_fold<Q, S, OP, CB>(q: Q, init: S, op: OP, combine: CB) -> S
where
    Q: ParDrive + Sync,
    S: Copy + Send + Sync,
    OP: Fn(S, Q::R) -> S + Sync,
    CB: Fn(S, S) -> S + Sync,
{
    if is_parallel() {
        par_unwrap_fold(ThreadPool::global(), &q, GRAIN, init, op, combine)
    } else {
        q.unwrap_fold(init, op)
    }
}

/// Parallel aggregation for input ALREADY SORTED by the group key. TPC-H
/// lineitem is stored in orderkey order, so `group_by(order)` emits keys
/// non-decreasing — a streaming reduce of consecutive-equal keys yields a
/// key-sorted run with O(distinct-keys) memory, no hash and no dense Vec. Two
/// adjacent runs merge as a monoid: the boundary group (last of left == first
/// of right) combines, the rest concatenates. This is "sort-based aggregation"
/// made free by pre-sorted data — the radix/streaming answer to the dense-fold
/// memory blowup. Returns the key-sorted (key, agg) pairs.
///
/// CORRECTNESS depends on the drive emitting keys in non-decreasing order; if
/// a key recurs non-adjacently the result is wrong. Use only where the scan is
/// clustered by the group key (verify with `is_sorted_by_key`).
pub fn par_sorted_fold<Q, S, OP, CB>(
    pool: &ThreadPool,
    q: &Q,
    grain: usize,
    init: S,
    op: OP,
    combine: CB,
) -> Vec<(Q::D, S)>
where
    Q: ParDrive + Sync,
    Q::D: Send + Sync,
    S: Copy + Send + Sync,
    OP: Fn(S, Q::R) -> S + Sync,
    CB: Fn(S, S) -> S + Sync,
{
    par_run(
        pool,
        q,
        grain,
        |q, lo, hi| {
            let mut run: Vec<(Q::D, S)> = Vec::new();
            q.drive_range(lo, hi, |d, v| match run.last_mut() {
                Some((k, s)) if *k == d => *s = op(*s, v),
                _ => run.push((d, op(init, v))),
            });
            run
        },
        |mut a, b| {
            if b.is_empty() {
                return a;
            }
            let mut bi = b.into_iter();
            let first = bi.next().unwrap();
            match a.last_mut() {
                Some((k, s)) if *k == first.0 => *s = combine(*s, first.1),
                _ => a.push(first),
            }
            a.extend(bi);
            a
        },
    )
}

/// A key-sorted `(D, R)` relation with binary-search `probe` — the probe-able
/// form of `par_sorted_fold`'s output. Memory is one Vec of the *distinct*
/// keys (vs a dense n-slot table), so it carries a parallel sorted-run
/// aggregation into probe position (q21's per-order state) without the dense
/// blowup. Keys are dense ids, compared by index.
pub struct SortedRel<D: Dense, R: Copy> {
    pub pairs: Vec<(D, R)>,
}
impl<D: Dense, R: Copy> Query for SortedRel<D, R> {
    type D = D;
    type R = R;
}
impl<D: Dense, R: Copy> Probe for SortedRel<D, R> {
    #[inline(always)]
    fn probe<K: FnMut(R)>(&self, x: D, mut k: K) {
        if let Ok(i) = self.pairs.binary_search_by_key(&x.idx(), |(d, _)| d.idx()) {
            k(self.pairs[i].1);
        }
    }
    #[inline(always)]
    fn probe_any<K: FnMut(R) -> bool>(&self, x: D, mut k: K) -> bool {
        match self.pairs.binary_search_by_key(&x.idx(), |(d, _)| d.idx()) {
            Ok(i) => k(self.pairs[i].1),
            Err(_) => false,
        }
    }
}
impl<D: Dense, R: Copy> Drive for SortedRel<D, R> {
    #[inline(always)]
    fn drive<K: FnMut(D, R)>(&self, mut k: K) {
        for &(d, r) in &self.pairs {
            k(d, r);
        }
    }
}

/// Flag-dispatched sorted-run fold returning a probe-able `SortedRel`. Parallel
/// path uses `par_sorted_fold`; sequential path streaming-reduces in one pass.
/// Both require the scan clustered by the (dense) group key.
pub fn psorted_fold<Q, S, OP, CB>(q: Q, init: S, op: OP, combine: CB) -> SortedRel<Q::D, S>
where
    Q: ParDrive + Sync,
    Q::D: Dense + Send + Sync,
    S: Copy + Send + Sync,
    OP: Fn(S, Q::R) -> S + Sync,
    CB: Fn(S, S) -> S + Sync,
{
    let pairs = if is_parallel() {
        par_sorted_fold(ThreadPool::global(), &q, GRAIN, init, op, combine)
    } else {
        let mut run: Vec<(Q::D, S)> = Vec::new();
        q.drive(|d, v| match run.last_mut() {
            Some((k, s)) if *k == d => *s = op(*s, v),
            _ => run.push((d, op(init, v))),
        });
        run
    };
    SortedRel { pairs }
}
