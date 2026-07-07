use prela::engine::IntoQuery;
use prela::{Entry, job_schema, queries, tpch, tpch_schema};

/// Cache directory the suites mmap from — `../cache` by default, overridable
/// with `PRELA_CACHE` (e.g. to point at a different scale factor's cache).
fn cache_dir() -> std::path::PathBuf {
    std::env::var_os("PRELA_CACHE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("../cache"))
}

/// How to aggregate repeated timing samples for a query — set via `STAT`.
#[derive(Clone, Copy)]
enum Stat {
    Min,
    Median,
}

impl Stat {
    fn from_env() -> Self {
        match std::env::var("STAT").as_deref() {
            Ok("median") => Stat::Median,
            Ok("min") | Err(_) => Stat::Min,
            Ok(other) => panic!("unknown STAT {other:?} (use min|median)"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Stat::Min => "min",
            Stat::Median => "median",
        }
    }

    /// `times` need not be sorted; sorted in place.
    fn reduce(self, times: &mut [f64]) -> f64 {
        times.sort_by(f64::total_cmp);
        match self {
            Stat::Min => times[0],
            Stat::Median => times[times.len() / 2],
        }
    }
}

/// Repetitions per query for the timed (round-2) pass — set via `REPS`
/// (default 1, i.e. the old single-shot behavior). Values above 1 rerun
/// each query `REPS` times and reduce with `STAT` (min|median) instead of
/// reporting a single noisy wall-clock sample.
fn reps_from_env() -> usize {
    std::env::var("REPS")
        .ok()
        .map(|s| s.parse().unwrap_or_else(|_| panic!("REPS must be a positive integer, got {s:?}")))
        .unwrap_or(1)
        .max(1)
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
/// Round 1 is a single warm-up pass (untimed for aggregation purposes).
/// Round 2 times each query `reps` times and reduces the samples with
/// `stat` — `reps == 1` recovers the old single-shot-per-round behavior.
fn run_suite(
    qs: &[Entry],
    reps: usize,
    stat: Stat,
    on_pass: impl Fn(usize, &str, f64, &str),
    on_diff: impl Fn(&str, f64, &str, &str),
) {
    for round in 1..=2 {
        eprintln!("--- run {round} ---");
        let mut ok = 0usize;
        let t = std::time::Instant::now();
        for (name, oracle, f) in qs {
            let query_reps = if round == 1 { 1 } else { reps };
            let mut times = Vec::with_capacity(query_reps);
            let mut got = String::new();
            for _ in 0..query_reps {
                let q_t = std::time::Instant::now();
                got = f();
                times.push(q_t.elapsed().as_secs_f64());
            }
            let dt = stat.reduce(&mut times);
            if got == *oracle {
                ok += 1;
                on_pass(round, name, dt, &got);
            } else {
                on_diff(name, dt, &got, oracle);
            }
        }
        eprintln!(
            "run {round}: {}/{} ok  total {:.2}s ({} reps, {})",
            ok,
            qs.len(),
            t.elapsed().as_secs_f32(),
            if round == 1 { 1 } else { reps },
            stat.label()
        );
    }
}

fn run_job() {
    let t = std::time::Instant::now();
    job_schema::job_init(&cache_dir());
    eprintln!(
        "load: {:.2}s  (movie n={}, person n={})",
        t.elapsed().as_secs_f32(),
        job_schema::movie.iq().n,
        job_schema::persons.iq().n
    );

    let qs = queries::all_queries();
    eprintln!("{} queries registered", qs.len());

    run_suite(
        &qs,
        reps_from_env(),
        Stat::from_env(),
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
    tpch_schema::tpch_init(&cache_dir());
    eprintln!(
        "load: {:.2}s  (li n={}, ord n={}, ps n={})",
        t.elapsed().as_secs_f32(),
        tpch_schema::lineitem.iq().n,
        tpch_schema::orders.iq().n,
        tpch_schema::partsupp.iq().n
    );

    let reps = reps_from_env();
    let stat = Stat::from_env();

    // QS=idiomatic|optimized|idiomatic_optimized|all (default idiomatic_optimized)
    let variant = std::env::var("QS").unwrap_or_else(|_| "idiomatic_optimized".to_string());
    if variant == "all" {
        run_tpch_all(reps, stat);
        return;
    }
    let qs = match variant.as_str() {
        "idiomatic" => tpch::idiomatic::queries(),
        "optimized" => tpch::optimized::queries(),
        "idiomatic_optimized" => tpch::idiomatic_optimized::queries(),
        other => panic!("unknown QS variant: {other:?} (use idiomatic|optimized|idiomatic_optimized|all)"),
    };
    eprintln!(
        "{} TPC-H queries registered ({} variant, REPS={reps}, STAT={})",
        qs.len(),
        variant,
        stat.label()
    );

    run_suite(
        &qs,
        reps,
        stat,
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

/// Runs all three TPC-H variants (idiomatic, optimized, idiomatic_optimized)
/// **in a single process**, with query execution interleaved across variants
/// query-by-query, rep-by-rep — `id1/variantA, id1/variantB, id1/variantC,
/// id2/variantA, ...`, repeated `reps` times.
///
/// This matters: launching each variant as a separate process (the old
/// `QS=<variant>` workflow, one invocation per variant redirected to a file)
/// confounds the comparison with cross-process noise — cold code/page
/// caches, allocator warm-up, OS scheduling and thermal/frequency state all
/// differ between launches, and at the ~10-100ms scale of these queries that
/// noise can swamp real differences between variants (a variant can look
/// 15% slower simply because its process happened to launch first). Sharing
/// one process and one warm-up pass, then rotating evenly through all
/// variants for every timed rep, cancels out drift that would otherwise
/// bias whichever variant runs first/last.
///
/// Writes `data/{variant}_{stat}{reps}.txt` (matching the format `bench/plot_tpch.py`
/// already parses) into `TPCH_OUT_DIR` (default `../bench/data`).
fn run_tpch_all(reps: usize, stat: Stat) {
    let out_dir = std::env::var_os("TPCH_OUT_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("bench/data"));

    let variants: [(&str, Vec<Entry>); 3] = [
        ("idiomatic", tpch::idiomatic::queries()),
        ("optimized", tpch::optimized::queries()),
        ("idiomatic_optimized", tpch::idiomatic_optimized::queries()),
    ];

    // Canonical query id order, taken from the idiomatic (full-coverage) registry.
    let ids: Vec<&str> = variants[0].1.iter().map(|(name, _, _)| *name).collect();
    for (vname, qs) in &variants {
        let vids: Vec<&str> = qs.iter().map(|(name, _, _)| *name).collect();
        assert_eq!(&vids, &ids, "variant {vname} registers a different query id set");
    }
    let find = |qs: &[Entry], id: &str| -> Entry { *qs.iter().find(|(name, _, _)| *name == id).unwrap() };

    eprintln!(
        "{} TPC-H queries registered (3 variants interleaved in one process, REPS={reps}, STAT={})",
        ids.len(),
        stat.label()
    );

    // Round 1: single warm-up pass per (variant, query), also checks correctness.
    eprintln!("--- run 1 ---");
    let mut round1: Vec<Vec<(f64, bool)>> = Vec::with_capacity(variants.len());
    for (_, qs) in &variants {
        let mut times = Vec::with_capacity(ids.len());
        for id in &ids {
            let (_, oracle, f) = find(qs, id);
            let t0 = std::time::Instant::now();
            let got = f();
            times.push((t0.elapsed().as_secs_f64(), got == oracle));
        }
        round1.push(times);
    }
    for (vi, (vname, _)) in variants.iter().enumerate() {
        let ok = round1[vi].iter().filter(|(_, ok)| *ok).count();
        eprintln!("run 1 [{vname}]: {ok}/{} ok", ids.len());
    }

    // Round 2: timed. Interleaved query-by-query and variant-by-variant so no
    // variant systematically absorbs process-launch costs, but each (query,
    // variant)'s `reps` repetitions run back-to-back — matching the original
    // per-process harness's cache locality (repeated hits keep that query's
    // working set hot) instead of scattering repeats across unrelated queries,
    // which would evict caches between reps and bias every variant slower
    // alike (that was tried and produced a uniform, spurious slowdown).
    eprintln!("--- run 2 ---");
    let mut times: Vec<Vec<Vec<f64>>> =
        variants.iter().map(|_| ids.iter().map(|_| Vec::with_capacity(reps)).collect()).collect();
    let t = std::time::Instant::now();
    for (qi, id) in ids.iter().enumerate() {
        for (vi, (_, qs)) in variants.iter().enumerate() {
            let (_, _, f) = find(qs, id);
            for _ in 0..reps {
                let t0 = std::time::Instant::now();
                let _ = f();
                times[vi][qi].push(t0.elapsed().as_secs_f64());
            }
        }
    }
    eprintln!("run 2: total {:.2}s ({reps} reps, {})", t.elapsed().as_secs_f32(), stat.label());

    std::fs::create_dir_all(&out_dir).expect("create TPCH_OUT_DIR");
    let suffix = format!("{}{reps}", stat.label());
    for (vi, (vname, _)) in variants.iter().enumerate() {
        let mut out = String::new();
        out.push_str("--- run 1 ---\n");
        let mut ok1 = 0usize;
        for (qi, id) in ids.iter().enumerate() {
            let (dt, is_ok) = round1[vi][qi];
            let tag = if is_ok { "ok" } else { "DIFF" };
            if is_ok {
                ok1 += 1;
            }
            out.push_str(&format!("{id:<5} {tag:<4}  {dt:>8.4}s\n"));
        }
        out.push_str(&format!("run 1: {ok1}/{} ok\n", ids.len()));
        out.push_str("--- run 2 ---\n");
        let mut ok2 = 0usize;
        for (qi, id) in ids.iter().enumerate() {
            let mut ts = times[vi][qi].clone();
            let dt = stat.reduce(&mut ts);
            let is_ok = round1[vi][qi].1;
            let tag = if is_ok { "ok" } else { "DIFF" };
            if is_ok {
                ok2 += 1;
            }
            out.push_str(&format!("{id:<5} {tag:<4}  {dt:>8.4}s\n"));
        }
        out.push_str(&format!("run 2: {ok2}/{} ok  ({reps} reps, {})\n", ids.len(), stat.label()));

        let path = out_dir.join(format!("{vname}_{suffix}.txt"));
        std::fs::write(&path, &out).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
        eprintln!("wrote {}", path.display());
        print!("{out}");
    }
}
