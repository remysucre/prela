// Threading experiment — heartbeat-scheduled (chili) parallelization of a
// single root scan, measured against the sequential driver.
//
// The engine is a push-CPS engine: a query is one fused `drive` from the root
// universe down a nest of probe continuations. There is no iterator to hand to
// a data-parallel runtime. So we parallelize the ONLY axis that drives (the
// root scan over movie ids) with a divide-and-conquer recursion: split the id
// window in half, `join` the halves (chili promotes the split to another worker
// only when its heartbeat fires — near-free otherwise), and merge the two
// partial results with the sink's monoid. The sink here is `min_row`'s
// per-column minimum, whose monoid is `Row::col_min` with identity `None`.
//
// The query is q22a (regex-heavy, fold-free) parameterized by its root
// universe, so the same plan builder serves the sequential baseline (full
// 0..n window) and each parallel leaf (a sub-window).

use chili::{Config, Scope, ThreadPool};
use prela::engine::*;
use prela::job_schema::*;
use prela::queries::helpers::{min_row, Row};
use std::num::NonZero;
use std::path::Path;
use std::time::Instant;

/// q22a with its root universe hoisted out, so a windowed sub-universe can be
/// substituted per parallel leaf. Byte-identical plan to `queries::t1::q22a`.
fn q22a(root: Universe<Id<Movie>>) -> impl Drive<R: Row + Send> {
    root.with(
        info.select(
            Info::ty
                .eq("countries")
                .and(Info::info.is_in(["Germany", "German", "USA", "American"])),
        )
        .and(keyword.is_in(["murder", "murder-in-title", "blood", "violence"]))
        .and(production_year.gt(2008))
        .and(kind.is_in(["movie", "episode"])),
    )
    .select(
        title
            .and(
                data.with(Data::text.lt("7.0").and(Data::ty.eq("rating")))
                    .text(),
            )
            .and(
                company
                    .with(
                        Company::note
                            .nrx(r"\(USA\)")
                            .and(Company::note.rx(r"\(200.*\)"))
                            .and(country.ne("[us]"))
                            .and(Company::ty.eq("production companies")),
                    )
                    .name(),
            ),
    )
}

/// q9c with its root universe hoisted out — a compute-bound counterpoint to
/// q22a: a broad `company.country == "[us]"` filter lets most movies through,
/// then each survivor runs a regex (`Person::name ~ "An"`) over its cast, so
/// the per-row work (the `select`) actually fires. Non-empty result.
fn q9c(root: Universe<Id<Movie>>) -> impl Drive<R: Row + Send> {
    root.with(company.country().eq("[us]")).select(
        cast.with(
            Cast::note
                .is_in(["(voice)", "(voice: Japanese version)", "(voice) (uncredited)", "(voice: English version)"])
                .and(role.eq("actress"))
                .and(person.with(gender.eq("f").and(Person::name.rx(r"An")))),
        )
        .select(person.alias().text().and(character.text()).and(person.name()))
        .and(title),
    )
}

/// Divide-and-conquer over the id window `[lo, hi)`. Leaves (≤ `grain` ids)
/// drive the windowed plan sequentially into a per-column-min accumulator;
/// internal nodes `join` their halves and merge with `col_min`. `build` is the
/// root-parameterized plan; reconstructing it per leaf is cheap (struct wraps
/// over `&'static` columns).
fn rec<F, Q>(s: &mut Scope, build: &F, n: usize, lo: usize, hi: usize, grain: usize) -> Option<Q::R>
where
    F: Fn(Universe<Id<Movie>>) -> Q + Sync,
    Q: Drive,
    Q::R: Row + Send,
{
    if hi - lo <= grain {
        let mut acc: Option<Q::R> = None;
        build(Universe::new(n).window(lo, hi)).drive(|_, v| {
            acc = Some(match acc {
                Some(a) => a.col_min(v),
                None => v,
            });
        });
        acc
    } else {
        let mid = lo + (hi - lo) / 2;
        let (a, b) = s.join(
            move |s| rec(s, build, n, lo, mid, grain),
            move |s| rec(s, build, n, mid, hi, grain),
        );
        match (a, b) {
            (Some(a), Some(b)) => Some(a.col_min(b)),
            (a, b) => a.or(b),
        }
    }
}

/// Render an accumulated row exactly as `min_row` does, so results compare
/// byte-for-byte against the sequential baseline.
fn render<R: Row>(row: Option<R>) -> String {
    match row {
        None => "(empty)".into(),
        Some(r) => {
            let mut cols = Vec::new();
            r.push_cols(&mut cols);
            cols.join(" || ")
        }
    }
}

/// Best (min) wall-clock of `iters` runs, plus the (identical) result.
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

/// Sequential baseline + a thread-count sweep for one root-parameterized query.
fn sweep<F, Q>(label: &str, n: usize, cores: usize, grain: usize, build: F)
where
    F: Fn(Universe<Id<Movie>>) -> Q + Sync,
    Q: Drive,
    Q::R: Row + Send,
{
    const ITERS: usize = 9;
    let (seq, seq_t) = best_of(ITERS, || min_row(build(Universe::new(n))));
    println!("\n{label}  (grain {grain})");
    println!("  seq          {seq_t:>8.4}s   {seq}");
    for threads in [2usize, 4, 8, cores] {
        let pool = ThreadPool::with_config(Config {
            thread_count: NonZero::new(threads),
            ..Default::default()
        });
        let (par, par_t) = best_of(ITERS, || {
            let mut scope = pool.scope();
            render(rec(&mut scope, &build, n, 0, n, grain))
        });
        assert!(par == seq, "{label}: parallel diverged:\n  seq: {seq}\n  par: {par}");
        println!("  par t={threads:<2}    {par_t:>8.4}s   {:.2}x", seq_t / par_t);
    }
}

fn main() {
    job_init(Path::new("../cache"));
    let n = movie.iq().n;
    let cores = std::thread::available_parallelism().map(|c| c.get()).unwrap_or(1);
    eprintln!("movie n = {n}  ({cores} cores available)");

    sweep("q22a (filter scan, empty result — memory-latency bound)", n, cores, 1_024, q22a);
    sweep("q9c  (regex over cast — compute bound)", n, cores, 1_024, q9c);

    // Grain sweep at full thread count — finer grain = more steal points, so
    // if scaling improves as grain shrinks the q9c plateau is load skew, not
    // bandwidth.
    println!("\nq9c grain sweep at t={cores}:");
    let (_, seq_t) = best_of(9, || min_row(q9c(Universe::new(n))));
    let pool = ThreadPool::with_config(Config { thread_count: NonZero::new(cores), ..Default::default() });
    for grain in [65_536usize, 16_384, 4_096, 1_024, 256, 64] {
        let (_p, t) = best_of(9, || {
            let mut s = pool.scope();
            render(rec(&mut s, &q9c, n, 0, n, grain))
        });
        println!("  grain {grain:<6}  {t:>8.4}s   {:.2}x", seq_t / t);
    }
}
