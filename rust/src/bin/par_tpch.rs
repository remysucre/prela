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
use prela::par::{par_fold, par_unwrap_fold};
use prela::tpch_schema::*;
use std::num::NonZero;
use std::path::Path;
use std::time::Instant;

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
    tpch_init(Path::new("../cache"));
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

    // q1 — per-key fold (seq = dense_fold, par = HashMap par_fold)
    let (seq1, seq1_t) = best_of(ITERS, || render_q1(q1_grouped().dense_fold(288, INIT, q1_op)));
    println!("\nq1  per-key fold      seq {seq1_t:.4}s   ({} groups)", seq1.lines().count());
    let p1 = q1_grouped();
    let mut float_reordered = false;
    for t in [2usize, 4, 8, cores] {
        let pl = pool(t);
        let (par, par_t) = best_of(ITERS, || render_q1(par_fold(&pl, &p1, grain, INIT, q1_op, q1_combine)));
        // Grouping + COUNT (integer) must be exact; the f64 sums may differ in
        // the last cent because partition-then-combine reorders the additions.
        assert!(keys_counts(&par) == keys_counts(&seq1), "q1 grouping/count diverged");
        float_reordered |= par != seq1;
        println!("    t={t:<2}  {par_t:>8.4}s   {:.2}x", seq1_t / par_t);
    }
    if float_reordered {
        println!("    note: f64 sums differ by ≤1 cent vs sequential (non-associativity);");
        println!("          groups + counts are exact. Byte-exact oracles need a fix.");
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
