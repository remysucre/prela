// Threading bench — heartbeat-scheduled (chili) parallel root scan vs the
// sequential driver, across a spread of JOB query shapes.
//
// With the `ParDrive` spine in engine.rs, queries need no rewriting: the plan
// is built normally and shared by reference across workers (`par_min_row`),
// while the sequential baseline drives the same plan via `min_row`. Each query
// is a `fn() -> impl ParDrive` so the harness can rebuild it per measurement.

use prela::engine::*;
use prela::job_schema::*;
use prela::queries::helpers::{min_row, par_min_row, Row};
use std::num::NonZero;
use std::path::Path;
use std::time::Instant;

// ===== a spread of JOB queries (byte-identical to queries::t1/t3/t4) =====

/// Pure filter scan, empty result — random semijoin probes, memory-latency
/// bound (the hard case for scaling).
fn q22a() -> impl ParDrive<R: Row + Send> {
    movie
        .with(
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
                .and(data.with(Data::text.lt("7.0").and(Data::ty.eq("rating"))).text())
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

/// Regex over each US movie's cast — the per-row `select` fires, compute-bound.
fn q9c() -> impl ParDrive<R: Row + Send> {
    movie.with(company.country().eq("[us]")).select(
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

/// Like q9c, no name regex — broader survivor set, more projection work.
fn q9d() -> impl ParDrive<R: Row + Send> {
    movie.with(company.country().eq("[us]")).select(
        cast.with(
            Cast::note
                .is_in(["(voice)", "(voice: Japanese version)", "(voice) (uncredited)", "(voice: English version)"])
                .and(role.eq("actress"))
                .and(person.with(gender.eq("f"))),
        )
        .select(person.alias().text().and(person.name()).and(character.text()))
        .and(title),
    )
}

/// Year-bounded with a company-note regex in the filter — different filter mix.
fn q9a() -> impl ParDrive<R: Row + Send> {
    movie
        .with(
            company
                .select(country.eq("[us]").and(Company::note.rx(r"\(USA\)").or(Company::note.rx(r"\(worldwide\)"))))
                .and(production_year.ge(2005))
                .and(production_year.le(2015)),
        )
        .select(
            cast.with(
                Cast::note
                    .is_in(["(voice)", "(voice: Japanese version)", "(voice) (uncredited)", "(voice: English version)"])
                    .and(role.eq("actress"))
                    .and(person.with(gender.eq("f").and(Person::name.rx(r"Ang")))),
            )
            .select(person.alias().text().and(character.text()))
            .and(title),
        )
}

/// keyword + episode-range filter, cast→person→aka projection.
fn q16a() -> impl ParDrive<R: Row + Send> {
    movie
        .with(
            company
                .country()
                .eq("[us]")
                .and(keyword.eq("character-name-in-title"))
                .and(episode_nr.ge(50))
                .and(episode_nr.lt(100)),
        )
        .select(cast.person().alias().text().and(title))
}

// ===== harness ===========================================================

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

/// Sequential baseline + a thread-count sweep for one query.
fn sweep<Q>(label: &str, make: impl Fn() -> Q, cores: usize, grain: usize)
where
    Q: ParDrive + Sync,
    Q::R: Row + Send,
{
    const ITERS: usize = 9;
    let (seq, seq_t) = best_of(ITERS, || min_row(make()));
    println!("\n{label}   seq {seq_t:.4}s");
    let plan = make();
    for threads in [2usize, 4, 8, cores] {
        let pool = chili::ThreadPool::with_config(chili::Config {
            thread_count: NonZero::new(threads),
            ..Default::default()
        });
        let (par, par_t) = best_of(ITERS, || par_min_row(&pool, &plan, grain));
        assert!(par == seq, "{label}: parallel diverged:\n  seq: {seq}\n  par: {par}");
        println!("    t={threads:<2}  {par_t:>8.4}s   {:.2}x", seq_t / par_t);
    }
}

fn main() {
    job_init(Path::new("../cache"));
    let cores = std::thread::available_parallelism().map(|c| c.get()).unwrap_or(1);
    eprintln!("movie n = {}  ({cores} cores)", movie.iq().n);

    let grain = 1_024;
    sweep("q22a  filter scan, empty (latency-bound)", q22a, cores, grain);
    sweep("q9a   regex filter + cast regex        ", q9a, cores, grain);
    sweep("q9c   cast regex (compute-bound)       ", q9c, cores, grain);
    sweep("q9d   cast, no name regex              ", q9d, cores, grain);
    sweep("q16a  keyword + episode range          ", q16a, cores, grain);
}
