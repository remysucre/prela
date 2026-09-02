use prela::{Entry, job_queries, job_schema, tpch_queries, tpch_schema};

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

    match suite {
        "tpch" => run_tpch(),
        _ => run_job(),
    }
}

/// Two timed rounds over a query suite: run every query, diff against its
/// oracle, report ok-counts. Per-query reporting is suite-specific.
///
/// `db` is `&'static` because the runners are fn pointers in `const`
/// tables — see `Entry`.
fn run_suite<D>(
    db: &'static D,
    qs: &[Entry<D>],
    on_pass: impl Fn(usize, &str, f64, &str),
    on_diff: impl Fn(&str, f64, &str, &str),
) {
    // ROUNDS=<n> overrides the default two timed rounds (used by bench scripts).
    let rounds: usize = std::env::var("ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);
    // PRELA_DUMP=<dir> writes every query's full output there, oracle match
    // or not — the differential check across a refactor needs the text even
    // when the cache's scale factor doesn't match the checked-in oracles.
    let dump = std::env::var_os("PRELA_DUMP").map(std::path::PathBuf::from);
    if let Some(d) = &dump {
        let _ = std::fs::create_dir_all(d);
    }
    // PRELA_ONLY=<a,b,c> restricts the run to those query names, and
    // PRELA_ITERS=<n> runs each of them n times per round, reporting the mean
    // at nanosecond precision on its own `bench` line. The normal per-query
    // line is printed to 0.1 ms, which is coarser than the fastest JOB
    // queries; this is how you measure one of those without the grid.
    let only: Option<Vec<String>> = std::env::var("PRELA_ONLY")
        .ok()
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect());
    let iters: usize = std::env::var("PRELA_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    for round in 1..=rounds {
        eprintln!("--- run {round} ---");
        let mut ok = 0usize;
        let t = std::time::Instant::now();
        for (name, oracle, f) in qs {
            if let Some(keep) = &only {
                if !keep.iter().any(|k| k == name) {
                    continue;
                }
            }
            let q_t = std::time::Instant::now();
            let mut got = f(db);
            for _ in 1..iters {
                got = f(db);
            }
            let dt = q_t.elapsed().as_secs_f64() / iters as f64;
            if iters > 1 {
                println!("{:<5} bench {:.9}s  (mean of {} iters)", name, dt, iters);
            }
            if let Some(d) = &dump {
                let _ = std::fs::write(d.join(format!("{name}.txt")), &got);
            }
            if got == *oracle {
                ok += 1;
                on_pass(round, name, dt, &got);
            } else {
                on_diff(name, dt, &got, oracle);
            }
        }
        eprintln!(
            "run {round}: {}/{} ok  total {:.2}s",
            ok,
            qs.len(),
            t.elapsed().as_secs_f32()
        );
    }
}

fn run_job() {
    let t = std::time::Instant::now();
    let db: &'static job_schema::Job = Box::leak(Box::new(job_schema::load(&cache_dir())));
    eprintln!(
        "load: {:.2}s  (movie n={}, person n={})",
        t.elapsed().as_secs_f32(),
        db.movie.key.n,
        db.person.key.n
    );

    let qs = job_queries::all_queries();
    eprintln!("{} queries registered", qs.len());

    run_suite(
        db,
        &qs,
        |round, name, dt, got| {
            if round == 2 || dt > 0.5 {
                println!("{:<5} ok  {:>8.4}s  {}", name, dt, got);
            }
        },
        |name, dt, got, oracle| {
            println!("{:<5} DIFF {:>8.4}s  {}", name, dt, got);
            println!("        oracle: {oracle}");
        },
    );
}

fn run_tpch() {
    let t = std::time::Instant::now();
    // Leaked, once: `Entry`'s runners are fn pointers, and plans built from
    // the columns hold `&'static` references (which is what the engine's
    // `Compose`/`Filter` types expect). The cache mmap is leaked anyway.
    let db: &'static tpch_schema::Tpch = Box::leak(Box::new(tpch_schema::load(&cache_dir())));
    eprintln!(
        "load: {:.2}s  (li n={}, ord n={}, ps n={})",
        t.elapsed().as_secs_f32(),
        db.lineitem.key.n,
        db.order.key.n,
        db.partsupp.key.n
    );

    // QS=idiomatic|optimized (default optimized)
    let variant = std::env::var("QS").unwrap_or_else(|_| "optimized".to_string());
    let qs = match variant.as_str() {
        "idiomatic" => tpch_queries::idiomatic::queries(),
        "optimized" => tpch_queries::optimized::queries(),
        other => panic!("unknown QS variant: {other:?} (use idiomatic|optimized)"),
    };
    eprintln!(
        "{} TPC-H queries registered ({} variant)",
        qs.len(),
        variant
    );

    run_suite(
        db,
        &qs,
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
                    if i >= 3 {
                        break;
                    }
                }
            }
        },
    );
}
