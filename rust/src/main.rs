use prela::engine::IntoQuery;
use prela::{Entry, job_schema, queries, tpch, tpch_schema};

/// Cache directory the suites mmap from — `../cache` by default, overridable
/// with `PRELA_CACHE` (e.g. to point at a different scale factor's cache).
fn cache_dir() -> std::path::PathBuf {
    std::env::var_os("PRELA_CACHE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("../cache"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let suite = args.get(1).map(|s| s.as_str()).unwrap_or("job");

    // `prela <suite> par` runs the JOB min_row scans on chili's global pool.
    // (min_row is order-independent, so output stays byte-identical.)
    if args.iter().any(|a| a == "par") {
        prela::par::PARALLEL.store(true, std::sync::atomic::Ordering::Relaxed);
        eprintln!("PARALLEL mode: {} threads",
                  std::thread::available_parallelism().map(|c| c.get()).unwrap_or(1));
    }

    match suite {
        "tpch" => run_tpch(),
        _ => run_job(),
    }
}

/// Warm-up pass (round 1, faults the mmap'd columns in), then a measured pass
/// (round 2) that times each query as the MIN of K consecutive runs. The
/// best-of-K matters at SF=10: a single interleaved sample measures each query
/// half-cold because the previous queries evicted its working set; running one
/// query K times back-to-back reaches warm steady state (matching how DuckDB
/// is timed — 2nd of a cold/warm pair). Per-query reporting is suite-specific.
fn run_suite(
    qs: &[Entry],
    on_pass: impl Fn(usize, &str, f64, &str),
    on_diff: impl Fn(&str, f64, &str, &str),
) {
    const K: usize = 7;
    eprintln!("--- run 1 ---");
    let t = std::time::Instant::now();
    let mut ok = 0usize;
    for (_, oracle, f) in qs {
        if f() == *oracle { ok += 1; }
    }
    eprintln!("run 1: {}/{} ok  total {:.2}s", ok, qs.len(), t.elapsed().as_secs_f32());

    eprintln!("--- run 2 ---  (best-of-{K} warm per query)");
    let mut ok = 0usize;
    let mut warm_sum = 0f64;
    for (name, oracle, f) in qs {
        let mut best = f64::INFINITY;
        let mut got = String::new();
        for _ in 0..K {
            let q_t = std::time::Instant::now();
            got = f();
            best = best.min(q_t.elapsed().as_secs_f64());
        }
        warm_sum += best;
        if got == *oracle {
            ok += 1;
            on_pass(2, name, best, &got);
        } else {
            on_diff(name, best, &got, oracle);
        }
    }
    eprintln!("run 2: {}/{} ok  warm-sum {:.2}s", ok, qs.len(), warm_sum);
}

fn run_job() {
    let t = std::time::Instant::now();
    job_schema::job_init(&cache_dir());
    eprintln!("load: {:.2}s  (movie n={}, person n={})",
              t.elapsed().as_secs_f32(),
              job_schema::movie.iq().n, job_schema::persons.iq().n);

    let qs = queries::all_queries();
    eprintln!("{} queries registered", qs.len());

    run_suite(&qs,
        |round, name, dt, got| {
            if round == 2 || dt > 0.5 {
                println!("{:<5} ok  {:>8.4}s  {}", name, dt, got);
            }
        },
        |name, dt, got, oracle| {
            println!("{:<5} DIFF {:>8.4}s  {}", name, dt, got);
            println!("        oracle: {oracle}");
        });
}

fn run_tpch() {
    let t = std::time::Instant::now();
    tpch_schema::tpch_init(&cache_dir());
    eprintln!("load: {:.2}s  (li n={}, ord n={}, ps n={})",
              t.elapsed().as_secs_f32(),
              tpch_schema::lineitem.iq().n, tpch_schema::orders.iq().n,
              tpch_schema::partsupp.iq().n);

    // QS=idiomatic|optimized (default optimized)
    let variant = std::env::var("QS").unwrap_or_else(|_| "optimized".to_string());
    let qs = match variant.as_str() {
        "idiomatic" => tpch::idiomatic::queries(),
        "optimized" => tpch::optimized::queries(),
        other => panic!("unknown QS variant: {other:?} (use idiomatic|optimized)"),
    };
    eprintln!("{} TPC-H queries registered ({} variant)", qs.len(), variant);

    run_suite(&qs,
        |_, name, dt, _| println!("{:<5} ok    {:>8.4}s", name, dt),
        |name, dt, got, oracle| {
            println!("{:<5} DIFF  {:>8.4}s", name, dt);
            // Write to /tmp for offline diff
            let _ = std::fs::write(format!("/tmp/tpch_got_{name}.txt"), got);
            let _ = std::fs::write(format!("/tmp/tpch_oracle_{name}.txt"), oracle);
            let got_lines: Vec<_> = got.lines().collect();
            let oracle_lines: Vec<_> = oracle.lines().collect();
            for i in 0..got_lines.len().max(oracle_lines.len()) {
                let g = got_lines.get(i).unwrap_or(&"");
                let o = oracle_lines.get(i).unwrap_or(&"");
                if g != o {
                    println!("        line {}:", i + 1);
                    println!("          got:    {g:?}");
                    println!("          oracle: {o:?}");
                    if i >= 3 { break; }
                }
            }
        });
}
