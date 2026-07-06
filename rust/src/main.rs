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

    // QS=idiomatic|optimized|optimized_idiomatic (default optimized_idiomatic)
    let variant = std::env::var("QS").unwrap_or_else(|_| "optimized_idiomatic".to_string());
    let qs = match variant.as_str() {
        "idiomatic" => tpch::idiomatic::queries(),
        "optimized" => tpch::optimized::queries(),
        "optimized_idiomatic" => tpch::optimized_idiomatic::queries(),
        other => panic!("unknown QS variant: {other:?} (use idiomatic|optimized)"),
    };
    let reps = reps_from_env();
    let stat = Stat::from_env();
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
