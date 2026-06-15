// Threading bench (TPC-H) — the fold sinks, parallelized via combiners.
//
// q6: a filtered global revenue SUM (no group-by) — sink is one f64, monoid
//     (+, 0.0). `par_unwrap_fold`.
// q1: the canonical per-key aggregate — group by a packed (returnflag, status)
//     byte key, 6-tuple of running aggregates per group. `par_fold` builds a
//     HashMap partial per worker window and merges them (shared keys combine,
//     new keys insert), returning the same eager `Fold` the sequential path
//     builds. The sequential baseline uses the tuned `dense_fold`.
//
// Both pre-fold plans are `ParDrive` (q1's is GroupBy -> Compose -> Restrict
// -> Universe), so the lineitem scan splits across workers and the partials
// merge up the join tree.

use prela::engine::*;
use prela::par::{par_bitset, par_dense_fold, par_fold, par_sorted_fold, par_unwrap_fold};
use prela::tpch_schema::*;
use std::num::NonZero;
use std::time::Instant;

// ===== q18 — sorted-run aggregation (radix-free, data pre-sorted by key) ==
// q18 groups lineitem by order; lineitem is stored in orderkey order, so the
// scan emits keys non-decreasing and a streaming reduce gives a key-sorted run
// with no hash and no 60M-slot dense Vec. Compare sequential dense_fold vs
// par_sorted_fold for the per-order quantity sum.

/// Top-100 order ids (by sum_qty>300, ordered as q18 does) — the comparable
/// part of q18's output, from a list of (order, sum_qty) pairs.
fn q18_top100(mut rows: Vec<(Id<Order>, f64)>) -> Vec<usize> {
    rows.retain(|(_, s)| *s > 300.0);
    rows.sort_by(|a, b| {
        let (oa, ob) = (a.0.idx(), b.0.idx());
        totalprice.iq().values[ob]
            .partial_cmp(&totalprice.iq().values[oa])
            .unwrap()
            .then_with(|| date.iq().values[oa].cmp(&date.iq().values[ob]))
    });
    rows.truncate(100);
    rows.iter().map(|(o, _)| o.idx()).collect()
}

/// Sequential dense_fold → (order, sum) pairs.
fn q18_seq() -> Vec<(Id<Order>, f64)> {
    let fold = quantity.group_by(order).dense_fold(orders.iq().n, 0.0_f64, |a, q| a + q);
    let mut rows = Vec::new();
    fold.drive(|k, v| rows.push((k, v)));
    rows
}

// ===== q6 — global revenue sum ===========================================

fn q6_rows() -> impl ParDrive<R = (f64, f64)> {
    lineitem
        .with(
            shipdate
                .during(19940101, 19950101)
                .and(discount.between(0.05, 0.07))
                .and(quantity.lt(24.0)),
        )
        .select(extendedprice.and(discount))
}

// ===== q1 — per-key aggregate ============================================

/// (sum_qty, sum_ext, sum_disc, sum_disc_price, sum_charge, count)
type Acc = (f64, f64, f64, f64, f64, i64);
const INIT: Acc = (0.0, 0.0, 0.0, 0.0, 0.0, 0);

/// Absorb one lineitem into a group's running aggregates.
fn q1_op(a: Acc, r: (((f64, f64), f64), f64)) -> Acc {
    let (qty, ext, di, dp, chg, n) = a;
    let (((q, e), dc), tx) = r;
    let dp_inc = e * (1.0 - dc);
    let chg_inc = dp_inc * (1.0 + tx);
    (qty + q, ext + e, di + dc, dp + dp_inc, chg + chg_inc, n + 1)
}
/// Merge two groups' partial aggregates (elementwise +). Identity is INIT.
fn q1_combine(a: Acc, b: Acc) -> Acc {
    (a.0 + b.0, a.1 + b.1, a.2 + b.2, a.3 + b.3, a.4 + b.4, a.5 + b.5)
}

/// q1's pre-fold plan: filtered lineitems grouped by the packed (rf, ls) key.
fn q1_grouped() -> impl ParDrive<D = usize, R = (((f64, f64), f64), f64)> {
    let scan = lineitem
        .with(shipdate.le(19980902))
        .select(quantity.and(extendedprice).and(discount).and(tax));
    let group_key = returnflag.and(Lineitem::status).map(|(rf, ls): (&str, &str)| {
        ((rf.as_bytes()[0].wrapping_sub(b'A') as usize) << 4)
            | (ls.as_bytes()[0].wrapping_sub(b'F') as usize)
    });
    scan.group_by(group_key)
}

/// Render a folded q1 result (works for both DenseFold and the parallel Fold).
fn render_q1(folded: impl Drive<D = usize, R = Acc>) -> String {
    let mut rows: Vec<(usize, Acc)> = Vec::new();
    folded.drive(|k, v| rows.push((k, v)));
    rows.sort_by_key(|r| r.0);
    rows.iter()
        .map(|(k, (qty, ext, di, dp, chg, n))| {
            let rf = (((*k >> 4) as u8).wrapping_add(b'A')) as char;
            let ls = (((*k & 0xF) as u8).wrapping_add(b'F')) as char;
            let nf = *n as f64;
            let ff = |x: f64| format!("{x:.2}");
            format!(
                "{rf}|{ls}|{}|{}|{}|{}|{}|{}|{}|{n}",
                ff(*qty), ff(*ext), ff(*dp), ff(*chg), ff(qty / nf), ff(ext / nf), ff(di / nf)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ===== harness ===========================================================

fn best_of<T>(iters: usize, mut f: impl FnMut() -> T) -> (T, f64) {
    let mut best = f64::INFINITY;
    let mut out = None;
    for _ in 0..iters {
        let t = Instant::now();
        let r = f();
        best = best.min(t.elapsed().as_secs_f64());
        out = Some(r);
    }
    (out.unwrap(), best)
}

fn pool(threads: usize) -> chili::ThreadPool {
    chili::ThreadPool::with_config(chili::Config {
        thread_count: NonZero::new(threads),
        ..Default::default()
    })
}

fn main() {
    // Honor PRELA_CACHE (e.g. an SF=10 cache) like the main binary does.
    let cache = std::env::var_os("PRELA_CACHE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("../cache"));
    tpch_init(&cache);
    let cores = std::thread::available_parallelism().map(|c| c.get()).unwrap_or(1);
    eprintln!("lineitem n = {}  ({cores} cores)", lineitem.iq().n);
    const ITERS: usize = 9;
    let grain = 16_384;

    // q6 — global sum
    let (seq6, seq6_t) = best_of(ITERS, || q6_rows().unwrap_fold(0.0, |a, (e, dc)| a + e * dc));
    println!("\nq6  global sum        seq {seq6_t:.4}s   = {seq6:.2}");
    let p6 = q6_rows();
    for t in [2usize, 4, 8, cores] {
        let pl = pool(t);
        let (par, par_t) = best_of(ITERS, || {
            par_unwrap_fold(&pl, &p6, grain, 0.0, |a, (e, dc)| a + e * dc, |a, b| a + b)
        });
        assert!((par - seq6).abs() < 0.01, "q6 diverged: {seq6:.2} vs {par:.2}");
        println!("    t={t:<2}  {par_t:>8.4}s   {:.2}x", seq6_t / par_t);
    }

    // q1 — per-key fold. Sequential baseline = tuned dense_fold. Two parallel
    // sinks: HashMap partials (par_fold) and dense partials (par_dense_fold).
    // q1's key space is tiny (288 slots, 4 live groups) — the dense case.
    let (seq1, seq1_t) = best_of(ITERS, || render_q1(q1_grouped().dense_fold(288, INIT, q1_op)));
    println!("\nq1  per-key fold      seq {seq1_t:.4}s   ({} groups)", seq1.lines().count());
    let p1 = q1_grouped();

    println!("  par_fold (HashMap partials):");
    for t in [2usize, 4, 8, cores] {
        let pl = pool(t);
        let (par, par_t) = best_of(ITERS, || render_q1(par_fold(&pl, &p1, grain, INIT, q1_op, q1_combine)));
        assert!(keys_counts(&par) == keys_counts(&seq1), "q1 grouping/count diverged");
        println!("    t={t:<2}  {par_t:>8.4}s   {:.2}x", seq1_t / par_t);
    }
    println!("  par_dense_fold (dense partials, n=288):");
    for t in [2usize, 4, 8, cores] {
        let pl = pool(t);
        let (par, par_t) = best_of(ITERS, || render_q1(par_dense_fold(&pl, &p1, grain, 288, INIT, q1_op, q1_combine)));
        assert!(keys_counts(&par) == keys_counts(&seq1), "q1 grouping/count diverged");
        println!("    t={t:<2}  {par_t:>8.4}s   {:.2}x", seq1_t / par_t);
    }
    println!("  (f64 sums differ ≤1 cent vs sequential — reordering; groups+counts exact)");

    // q18 — per-order sum, sorted-run aggregation. Verify the scan is clustered
    // by the group key (lineitem in orderkey order), then compare sequential
    // dense_fold vs par_sorted_fold (streaming reduce + monoid merge of runs).
    let mut descents = 0usize;
    let mut prev = 0usize;
    quantity.group_by(order).drive(|k, _| {
        if k.idx() < prev { descents += 1; }
        prev = k.idx();
    });
    println!("\nq18 per-order sum   scan clustered by order: {} descents (0 = fully sorted)", descents);

    let seq_top = q18_top100(q18_seq());
    let (_, seq18_t) = best_of(ITERS, || q18_top100(q18_seq()));
    println!("  seq dense_fold        {seq18_t:.4}s");
    let pre = quantity.group_by(order);
    for t in [2usize, 4, 8, cores] {
        let pl = pool(t);
        let (top, par_t) = best_of(ITERS, || {
            q18_top100(par_sorted_fold(&pl, &pre, grain, 0.0_f64, |a, q| a + q, |a, b| a + b))
        });
        assert!(top == seq_top, "q18 sorted-fold top100 diverged");
        println!("  par_sorted_fold t={t:<2} {par_t:>8.4}s   {:.2}x", seq18_t / par_t);
    }

    // q13 scan-cost decomposition (sequential, warm) — where does the
    // full-orders scan spend its time? Iterate-all (60M slots) vs +hole-skip
    // (customer != NONE) vs +memmem comment filter.
    use memchr::memmem;
    let count_all = |c: &mut u64| *c += 1;
    let (n_all, t_all) = best_of(5, || { let mut c = 0u64; orders.iq().drive(|_, _| count_all(&mut c)); c });
    let (n_live, t_live) = best_of(5, || {
        let mut c = 0u64;
        orders.with(Order::customer.filt(|x| x != Dense::NONE)).drive(|_, _| count_all(&mut c));
        c
    });
    let (n_full, t_full) = best_of(5, || {
        let f = memmem::Finder::new("special");
        let mut c = 0u64;
        orders
            .with(Order::customer.filt(|x| x != Dense::NONE).and(Order::comment.filt(move |s: &str| match f.find(s.as_bytes()) {
                Some(p) => !s[p + 7..].contains("requests"),
                None => true,
            })))
            .drive(|_, _| count_all(&mut c));
        c
    });
    println!("\nq13 scan decomposition (warm, seq):");
    println!("  iterate 60M slots         {t_all:.4}s  ({n_all} rows)");
    println!("  + hole-skip (cust!=NONE)  {t_live:.4}s  ({n_live} live, +{:.4}s)", t_live - t_all);
    println!("  + memmem comment filter   {t_full:.4}s  ({n_full} kept,  +{:.4}s)", t_full - t_live);

    // q4 phase 1 (is_late bitset build) scaling — bandwidth wall or headroom?
    let bad_li_order = lineitem
        .with(commitdate.and(receiptdate).filt(|(c, r)| c < r))
        .order();
    println!("\nq4 phase1 is_late build scaling:");
    let (_, t1) = best_of(5, || {
        let pl = pool(1);
        par_bitset(&pl, orders.iq().n, &bad_li_order)
    });
    for t in [1usize, 2, 4, 8, cores] {
        let pl = pool(t);
        let (_, pt) = best_of(5, || par_bitset(&pl, orders.iq().n, &bad_li_order));
        println!("  t={t:<2}  {pt:.4}s  ({:.2}x, {:.1} GB/s)", t1 / pt, 1.44 / pt);
    }

    // Does the comment scan itself parallelize? Count survivors via
    // par_unwrap_fold at increasing thread counts.
    println!("q13 comment-scan parallelism (count survivors):");
    let f2 = memmem::Finder::new("special").into_owned();
    let live = orders.with(
        Order::customer.filt(|x| x != Dense::NONE).and(Order::comment.filt(move |s: &str| {
            match f2.find(s.as_bytes()) {
                Some(p) => !s[p + 7..].contains("requests"),
                None => true,
            }
        })),
    );
    for t in [1usize, 2, 4, 8, cores] {
        let pl = pool(t);
        let (cnt, pt) = best_of(5, || par_unwrap_fold(&pl, &live, 16_384, 0_i64, |a, _| a + 1, |a, b| a + b));
        println!("  t={t:<2}  {pt:.4}s  ({cnt} survivors, {:.2}x)", t_full / pt);
    }
}

/// (returnflag, status, count) per row — the integer-exact part of q1's output.
fn keys_counts(s: &str) -> Vec<(String, String, String)> {
    s.lines()
        .map(|l| {
            let p: Vec<&str> = l.split('|').collect();
            (p[0].into(), p[1].into(), p[p.len() - 1].into())
        })
        .collect()
}
