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

use crate::engine::{Dense, DenseFold, Fold, ParDrive};
use ahash::AHashMap as HashMap;
use chili::{Scope, ThreadPool};
use std::hash::Hash;
use std::marker::PhantomData;

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
