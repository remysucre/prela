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

use crate::engine::ParDrive;
use chili::{Scope, ThreadPool};

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
