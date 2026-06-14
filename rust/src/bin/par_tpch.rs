// Threading bench (TPC-H) — the fold sink, parallelized via a combiner.
//
// q6 is the cleanest aggregate: a filtered global revenue SUM (no group-by),
// so the sink is a single f64 with monoid (+, 0.0). The pre-fold plan
// `lineitem.with(...).select(extendedprice.and(discount))` is `ParDrive`
// (Compose -> Restrict -> Universe), so each worker folds its lineitem window
// into a private partial and the partials add up. `op` absorbs a row
// (acc + e*dc); `combine` merges two partial sums (+).
//
// This exercises the combiner-fold path on a real high-volume scan
// (6M lineitems at SF=1). Per-key (group-by) folds are a separate question —
// see the note printed at the end.

use prela::engine::*;
use prela::par::par_unwrap_fold;
use prela::tpch_schema::*;
use std::num::NonZero;
use std::path::Path;
use std::time::Instant;

/// q6's pre-fold plan: filtered lineitems projected to (extendedprice, discount).
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

fn main() {
    tpch_init(Path::new("../cache"));
    let cores = std::thread::available_parallelism().map(|c| c.get()).unwrap_or(1);
    eprintln!("lineitem n = {}  ({cores} cores)", lineitem.iq().n);

    const ITERS: usize = 9;
    let (seq, seq_t) = best_of(ITERS, || {
        q6_rows().unwrap_fold(0.0, |acc, (e, dc)| acc + e * dc)
    });
    println!("\nq6  global revenue sum   seq {seq_t:.4}s   = {seq:.2}");

    let grain = 16_384;
    let plan = q6_rows();
    for threads in [2usize, 4, 8, cores] {
        let pool = chili::ThreadPool::with_config(chili::Config {
            thread_count: NonZero::new(threads),
            ..Default::default()
        });
        let (par, par_t) = best_of(ITERS, || {
            par_unwrap_fold(&pool, &plan, grain, 0.0, |acc, (e, dc)| acc + e * dc, |a, b| a + b)
        });
        // f64 sum reorders under partitioning — assert agreement to the cent.
        assert!((par - seq).abs() < 0.01, "q6 parallel diverged: seq {seq:.2} par {par:.2}");
        println!("    t={threads:<2}  {par_t:>8.4}s   {:.2}x", seq_t / par_t);
    }
}
