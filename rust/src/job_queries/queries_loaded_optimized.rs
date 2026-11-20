#![cfg_attr(rustfmt, rustfmt_skip)]
// Hand-optimised loads of the JOB queries (`prela job_loaded_optim`, feature
// `job_loaded_optim`). Run and read out exactly like `job_loaded`:
//
//     PRELA_ONLY=1a,6d ROUNDS=2 cargo run --release --features job_loaded_optim -- job_loaded_optim
//
// 1a–16c are optimised (section "Optimised plans" below): each query is split
// into a LOAD PLAN, one tree over the files that decides what is checked while
// reading, and an IN-MEMORY PLAN, the query over the loaded rows. 16d–33c are
// queries_loaded.rs's, unchanged (in `entries`), as is the timing readout:
// load = reading the files into memory, exec = the query over memory.
//
// queries_loaded.rs's description, which still holds for the unchanged queries:
//
// The queries are queries.rs's, character for character; only their leaves
// differ. The columns destructured at the top of `entries` are `ColumnRef`s
// over the files (job_file_schema.rs), so each query closure builds a tree
// that `.load()` reads and reduces. The one other change is in the
// registry: `min_row(q1a())` is `min_row(loaded(q1a()))`, which loads the
// tree and then drives the loaded result.

use std::cell::Cell;
use std::time::Instant;

use crate::engine::*;
use crate::job_queries::helpers::{Row, min_row};
use crate::job_queries::sets::{
    genre6, horror2, kw7, kw8, kw10, link3, murder4, nordic8, nordic10, voice3, voice4, writer5,
};
use crate::job_file_schema::*;
use crate::trial_loader::{KeyFilter, Load};

/// `job_queries::Entry`, over the files instead of the cache.
pub type Entry = (&'static str, &'static str, Box<dyn Fn(&'static JobFiles) -> String>);

thread_local! {
    /// Seconds the running query has spent in `loaded` (its scans), for `entry`.
    static LOAD_SECS: Cell<f64> = const { Cell::new(0.0) };
}

/// As queries.rs's `entry`, plus:
///   - each run prints its time split in two:
///       load — the `.load()` (scans + building the in-memory relations);
///       exec — the rest of the run: building the plan, then `min_row`'s drive
///              over the loaded relations. This is what main.rs's per-query
///              time measures for `job` (which drives the cache instead).
///   - the columns' sinks are cleared after each run, so loaded rows don't pile
///     up across the 113 queries (outside the split; main.rs's time includes it).
fn entry(name: &'static str, oracle: &'static str, q: impl Fn() -> String + 'static) -> Entry {
    (name, oracle, Box::new(move |db: &'static JobFiles| {
        let t = Instant::now();
        let got = q();
        let total = t.elapsed().as_secs_f64();
        let load = LOAD_SECS.with(|s| s.replace(0.0));
        println!("{name:<5} load {load:>8.4}s  exec {:>8.4}s", total - load);
        db.clear();
        got
    }))
}

/// Load a query's tree from the files, unreduced from above, and return the
/// loaded (in-memory) plan for `min_row` to drive. Its time goes to `LOAD_SECS`.
fn loaded<Q: Load>(q: Q) -> Q::Out {
    let t = Instant::now();
    let out = q.load(KeyFilter::All, KeyFilter::All, Vec::new()).0;
    LOAD_SECS.with(|s| s.set(s.get() + t.elapsed().as_secs_f64()));
    out
}

/// The result of a query that can't be loaded yet: its tree has a node with no
/// `Load` impl (`what`). Such a query's closure is marked
/// `#[cfg(any())] // pending: <what>` (never compiled); once the impl exists,
/// drop that line and restore its entry to `min_row(loaded(qN()))`.
fn pending(what: &str) -> String {
    format!("(pending: {what})")
}

// ===== Optimised plans: 1a–16c ==================================================
//
// Each query is split in two, and each half is a list in query order:
//
//   load_plan(db, "1a")  ONE tree over the files, `.load()`ed. It decides which
//                        conditions are checked while reading, and in which
//                        order, and fills the columns' sinks with every row the
//                        answer needs.
//   mem_plan(mem, "1a")  the query over `db.to_memory()` (those sinks): it
//                        checks what the load left to it and returns the answer.
//
// `optimised` runs the two, timed like queries_loaded.rs: load = load plan +
// to_memory, exec = the in-memory plan. The oracle is queries_loaded.rs's.
//
// How the load plans are chosen (the numbers in the comments are the
// selectivities in data/imdb/parquet):
//
//   1. A root `with` is read key-only and exactly, so `mem.movie` (the movies
//      the load kept) already satisfies it: the in-memory plan drops it.
//   2. `select_left` (top-down) when what is already loaded is small, e.g. the
//      250 movies of 1a; `select` (bottom-up) when the deep predicate is the
//      selective part, e.g. the 2 Downeys of 6d.
//   3. In `.and`, the leg that keeps the fewest movies goes first: the later
//      legs are read only for its movies. Only a filter or a bottom-up `select`
//      narrows them, though: a `select_left` leg passes on the keys of its
//      first hop (e.g. every movie with a company row), so it goes later.
//   4. A `with` inside the select is read key-only over the WHOLE file (it gets
//      no filter from above). When the rows above it are few, the load plan
//      reads its columns top-down instead (`x.select_left(pred.and(..))`), and
//      the in-memory plan restores it as `x.with(<that column>)`.
//   5. A predicate checked while loading is not checked again in memory: each
//      column is loaded once per plan, so its sink holds only the rows that
//      passed. (A key-only `with` argument fills no sink at all.)
//   6. A cheap but unselective root predicate (year > 2000: 55% of movies) is
//      read for the few movies the select leaves rather than for all 2.5M, and
//      checked in memory as `with(production_year)`.

/// Load a plan's tree. Its in-memory result is not used: the sinks are.
fn load<Q: Load>(q: Q) {
    q.load(KeyFilter::All, KeyFilter::All, Vec::new());
}

/// Run `f` as part of the query's load time (`LOAD_SECS`).
fn timed_load<T>(f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let out = f();
    LOAD_SECS.with(|s| s.set(s.get() + t.elapsed().as_secs_f64()));
    out
}

/// `q`'s load plan, then its in-memory plan over what was loaded. The
/// in-memory db is leaked to `'static`, which the query's `Filter`s need.
fn optimised(db: &'static JobFiles, q: &str) -> String {
    let mem: &'static MemJob = timed_load(|| {
        load_plan(db, q);
        Box::leak(Box::new(db.to_memory()))
    });
    mem_plan(mem, q)
}

const PROD: &str = "production companies";
const MGM: &str = r"\(as Metro-Goldwyn-Mayer Pictures\)";
const CNIT: &str = "character-name-in-title";
const MARVEL: &str = "marvel-cinematic-universe";
const DOWNEY: &str = r"Downey.*Robert";

// ----- Load plans ----------------------------------------------------------------

fn load_plan(db: &'static JobFiles, q: &str) {
    let Movie { title, kind, production_year, episode_nr, keyword, aka,
                company, cast, info, data, link, linked_by, .. } = &db.movie;
    let Cast { person, role, note: cast_note, character, .. } = &db.cast;
    let Person { name: person_name, gender, alias, bio, name_pcode_cf, .. } = &db.person;
    let Company { name: company_name, country, note: company_note, ty: company_ty, .. } = &db.company;
    let Info { info: info_info, ty: info_ty, note: info_note, .. } = &db.info;
    let Data { text: data_text, ty: data_ty, .. } = &db.data;
    let PersonInfo { info: personinfo_info, ty: personinfo_ty, note: personinfo_note, .. } = &db.person_info;
    let MovieLink { ty: link_ty, .. } = &db.movie_link;

    match q {
        // Root: 250 movies; their 8.3k company rows, top-down (the note regex
        // then runs on those rows only, not on all 2.6M).
        "1a" => load(db.movie.with(data.select(data_ty).eq("top 250 rank"))
                             .select_left(company.select_left(company_note.nrx(MGM).rx(r"\(co-production\)|\(presents\)")
                                                              .and(company_ty.eq(PROD)))
                                          .and(title)
                                          .and(production_year))),
        // Root: 10 movies, 120 company rows. The year first: it keeps fewest.
        "1b" => load(db.movie.with(data.select(data_ty).eq("bottom 10 rank"))
                             .select_left(production_year.between(2005, 2010)
                                          .and(company.select_left(company_note.nrx(MGM).and(company_ty.eq(PROD))))
                                          .and(title))),
        "1c" => load(db.movie.with(data.select(data_ty).eq("top 250 rank"))
                             .select_left(production_year.gt(2010)
                                          .and(company.select_left(company_note.nrx(MGM).rx(r"\(co-production\)")
                                                                   .and(company_ty.eq(PROD))))
                                          .and(title))),
        "1d" => load(db.movie.with(data.select(data_ty).eq("bottom 10 rank"))
                             .select_left(production_year.gt(2000)
                                          .and(company.select_left(company_note.nrx(MGM).and(company_ty.eq(PROD))))
                                          .and(title))),

        // The with first (key-only), then only its movies' titles (the tree
        // plan reads all 2.5M titles first). Its smaller conjunct leads.
        "2a" => load(db.movie.with(keyword.eq(CNIT).and(company.select(country).eq("[de]"))).select_left(title)),
        "2b" => load(db.movie.with(company.select(country).eq("[nl]").and(keyword.eq(CNIT))).select_left(title)),
        "2c" => load(db.movie.with(company.select(country).eq("[sm]").and(keyword.eq(CNIT))).select_left(title)),
        "2d" => load(db.movie.with(keyword.eq(CNIT).and(company.select(country).eq("[us]"))).select_left(title)),

        // Keyword and year leave 1.5k / 309 / 6.1k movies; their movie_info rows
        // (69k / 18k / 205k) are read top-down, instead of testing all 14.8M
        // info strings key-only.
        "3a" => load(db.movie.with(keyword.rx("sequel").and(production_year.gt(2005)))
                             .select_left(info.select_left(info_info.is_in(nordic8())).and(title))),
        "3b" => load(db.movie.with(keyword.rx("sequel").and(production_year.gt(2010)))
                             .select_left(info.select_left(info_info.eq("Bulgaria")).and(title))),
        "3c" => load(db.movie.with(keyword.rx("sequel").and(production_year.gt(1990)))
                             .select_left(info.select_left(info_info.is_in(["Sweden", "Norway", "Germany", "Denmark", "Swedish",
                                                                            "Denish", "Norwegian", "German", "USA", "American"]))
                                          .and(title))),

        // queries.rs's rx_dict is a regex on a dictionary column: here, `rx`.
        // Keyword and year: 1.5k / 309 / 6.1k movies, then their ratings top-down.
        "4a" => load(db.movie.with(keyword.rx("sequel").and(production_year.gt(2005)))
                             .select_left(data.select_left(data_ty.eq("rating").and(data_text.gt("5.0"))).and(title))),
        "4b" => load(db.movie.with(keyword.rx("sequel").and(production_year.gt(2010)))
                             .select_left(data.select_left(data_ty.eq("rating").and(data_text.gt("9.0"))).and(title))),
        "4c" => load(db.movie.with(keyword.rx("sequel").and(production_year.gt(1990)))
                             .select_left(data.select_left(data_ty.eq("rating").and(data_text.gt("2.0"))).and(title))),

        // The company conjunct is the selective one (0 / 0 / 783 movies) and
        // reads only movie_companies; the info test moves to the select, top-down.
        "5a" => load(db.movie.with(company.with(company_ty.eq(PROD).and(company_note.rx(r"\(theatrical\)").rx(r"\(France\)")))
                                   .and(production_year.gt(2005)))
                             .select_left(info.select_left(info_info.is_in(nordic8())).and(title))),
        "5b" => load(db.movie.with(company.with(company_ty.eq(PROD).and(company_note.rx(r"\(VHS\)").rx(r"\(USA\)").rx(r"\(1994\)")))
                                   .and(production_year.gt(2010)))
                             .select_left(info.select_left(info_info.is_in(["USA", "America"])).and(title))),
        "5c" => load(db.movie.with(company.with(company_ty.eq(PROD).and(company_note.nrx(r"\(TV\)").rx(r"\(USA\)")))
                                   .and(production_year.gt(1990)))
                             .select_left(info.select_left(info_info.is_in(nordic10())).and(title))),

        // marvel-c-u: 14 movies, so it leads; the year (15%) is read for those
        // only; the cast chain top-down from the 11 left (the Downey regex then
        // runs on a few names, not on 4.2M).
        "6a" => load(db.movie.select(keyword.eq(MARVEL)
                                     .and(production_year.gt(2010))
                                     .and(title)
                                     .and(cast.select_left(person).select_left(person_name).rx(DOWNEY)))),
        // year > 2014: 438 movies, so the root leads; the rest top-down.
        "6b" => load(db.movie.with(production_year.gt(2014))
                             .select_left(keyword.is_in(kw8())
                                          .and(title)
                                          .and(cast.select_left(person).select_left(person_name).rx(DOWNEY)))),
        "6c" => load(db.movie.with(production_year.gt(2014))
                             .select_left(keyword.eq(MARVEL)
                                          .and(title)
                                          .and(cast.select_left(person).select_left(person_name).rx(DOWNEY)))),
        // Downey is the selective part (2 persons, 486 cast rows, 433 movies):
        // bottom-up from the name; then kw8 and the year for those movies.
        "6d" => load(db.movie.select(cast.select(person).select(person_name).rx(DOWNEY)
                                     .and(keyword.is_in(kw8()))
                                     .and(production_year.gt(2000))
                                     .and(title))),
        "6e" => load(db.movie.select(keyword.eq(MARVEL)
                                     .and(production_year.gt(2000))
                                     .and(title)
                                     .and(cast.select_left(person).select_left(person_name).rx(DOWNEY)))),
        // No name filter: kw8 and year (11.8k movies), then their 584k cast rows.
        "6f" => load(db.movie.select(keyword.is_in(kw8())
                                     .and(production_year.gt(2000))
                                     .and(title)
                                     .and(cast.select_left(person).select_left(person_name)))),

        // Root: 820 / 247 movies; their cast rows' persons top-down. The bio test
        // leads, bottom-up (only 64 persons have a mini biography by Volker
        // Boehm), so the other person tests are read for those few persons. The
        // gender/name disjunction can't be loaded: gender and name are, and the
        // `or` runs in memory.
        "7a" => load(db.movie.with(linked_by.select(link_ty).eq("features").and(production_year.between(1980, 1995)))
                             .select_left(cast.select_left(person)
                                              .select_left(bio.select(personinfo_note.eq("Volker Boehm")
                                                                      .and(personinfo_ty.eq("mini biography")))
                                                           .and(alias.rx(r"a"))
                                                           .and(name_pcode_cf.between("A", "F"))
                                                           .and(gender)
                                                           .and(person_name))
                                          .and(title))),
        "7b" => load(db.movie.with(linked_by.select(link_ty).eq("features").and(production_year.between(1980, 1984)))
                             .select_left(cast.select_left(person)
                                              .select_left(bio.select(personinfo_note.eq("Volker Boehm")
                                                                      .and(personinfo_ty.eq("mini biography")))
                                                           .and(alias.rx(r"a"))
                                                           .and(name_pcode_cf.rx(r"^D"))
                                                           .and(gender.eq("m"))
                                                           .and(person_name))
                                          .and(title))),
        // Root: 8k movies, 228k persons. No bio filter is selective here (any
        // note), and the bios are long texts: the bio leg goes last, top-down,
        // so they are read only for the persons that passed the alias and code.
        "7c" => load(db.movie.with(linked_by.select(link_ty).is_in(["references", "referenced in", "features", "featured in"])
                                   .and(production_year.between(1980, 2010)))
                             .select_left(cast.select_left(person)
                                              .select_left(alias.rx(r"a|^A")
                                                           .and(name_pcode_cf.between("A", "F"))
                                                           .and(gender)
                                                           .and(person_name)
                                                           .and(bio.select_left(personinfo_note
                                                                                .and(personinfo_ty.eq("mini biography"))
                                                                                .and(personinfo_info)))))),

        // Root: 36k jp movies. The cast `with` (93k voice-English rows, role
        // first: an int column) is key-only, so m.cast reads just those rows;
        // persons and names top-down from them.
        "8a" => load(db.movie.with(company.with(country.eq("[jp]").and(company_note.rx(r"\(Japan\)").nrx(r"\(USA\)"))))
                             .select_left(cast.with(role.eq("actress").and(cast_note.eq("(voice: English version)")))
                                              .select_left(person.select_left(person_name.rx(r"Yo").nrx(r"Yu").and(alias)))
                                          .and(title))),
        // Root: 3.2k movies, and the title regex keeps 39 movies of all 2.5M:
        // it leads, and the cast tests run top-down on those few movies' rows.
        "8b" => load(db.movie.with(company.with(country.eq("[jp]")
                                                .and(company_note.rx(r"\(Japan\)").nrx(r"\(USA\)").rx(r"\(2006\)|\(2007\)")))
                                   .and(production_year.between(2006, 2007)))
                             .select_left(title.rx(r"^One Piece|^Dragon Ball Z")
                                          .and(cast.select_left(cast_note.eq("(voice: English version)")
                                                                .and(role.eq("actress"))
                                                                .and(person.select_left(person_name.rx(r"Yo").nrx(r"Yu").and(alias)))))) ),
        // Root: 572k [us] movies. The role `with` is key-only, so m.cast reads only
        // writer rows (836k of us movies), then persons and aliases top-down.
        "8c" => load(db.movie.with(company.select(country).eq("[us]"))
                             .select_left(cast.with(role.eq("writer")).select_left(person).select_left(alias)
                                          .and(title))),
        "8d" => load(db.movie.with(company.select(country).eq("[us]"))
                             .select_left(cast.with(role.eq("costume designer")).select_left(person).select_left(alias)
                                          .and(title))),

        // Root: 120k / 40k / 572k / 572k movies. The cast `with` (276k / 223k
        // voice-actress rows) is key-only, role first; m.cast then reads only
        // those rows of the root's movies (21k / 5k / 84k / 84k); the person
        // tests top-down from them, then the character of the cast rows left.
        "9a" => load(db.movie.with(company.with(country.eq("[us]").and(company_note.rx(r"\(USA\)|\(worldwide\)")))
                                   .and(production_year.between(2005, 2015)))
                             .select_left(cast.with(role.eq("actress").and(cast_note.is_in(voice4())))
                                              .select_left(person.select_left(gender.eq("f").and(person_name.rx(r"Ang")).and(alias))
                                                           .and(character))
                                          .and(title))),
        "9b" => load(db.movie.with(company.with(country.eq("[us]").and(company_note.rx(r"\(200.*\)").rx(r"\(USA\)|\(worldwide\)")))
                                   .and(production_year.between(2007, 2010)))
                             .select_left(cast.with(role.eq("actress").and(cast_note.eq("(voice)")))
                                              .select_left(person.select_left(gender.eq("f").and(person_name.rx(r"Angel")).and(alias))
                                                           .and(character))
                                          .and(title))),
        "9c" => load(db.movie.with(company.select(country).eq("[us]"))
                             .select_left(cast.with(role.eq("actress").and(cast_note.is_in(voice4())))
                                              .select_left(person.select_left(gender.eq("f").and(person_name.rx(r"An")).and(alias))
                                                           .and(character))
                                          .and(title))),
        "9d" => load(db.movie.with(company.select(country).eq("[us]"))
                             .select_left(cast.with(role.eq("actress").and(cast_note.is_in(voice4())))
                                              .select_left(person.select_left(gender.eq("f").and(alias).and(person_name))
                                                           .and(character))
                                          .and(title))),

        // Root: 3k / 817 movies, 170k / 37k cast rows: the note regex runs on
        // those rows (top-down), not on all 36M notes.
        "10a" => load(db.movie.with(company.select(country).eq("[ru]").and(production_year.gt(2005)))
                              .select_left(cast.select_left(cast_note.rx(r"\(voice\)").rx(r"\(uncredited\)")
                                                            .and(role.eq("actor"))
                                                            .and(character))
                                           .and(title))),
        "10b" => load(db.movie.with(company.select(country).eq("[ru]").and(production_year.gt(2010)))
                              .select_left(cast.select_left(cast_note.rx(r"\(producer\)")
                                                            .and(role.eq("actor"))
                                                            .and(character))
                                           .and(title))),
        // Root: 316k movies (too many cast rows to go top-down): the (producer)
        // `with` is key-only, m.cast reads its 342k rows of root movies, and
        // only 6 of those have a character.
        "10c" => load(db.movie.with(company.select(country).eq("[us]").and(production_year.gt(1990)))
                              .select_left(cast.with(cast_note.rx(r"\(producer\)")).select_left(character)
                                           .and(title))),

        // Root: 26 / 5 movies. The company rows are read top-down. A movie's
        // company rows WITHOUT a note are wanted (`minus(company_note)`), so the
        // note is loaded (NULL = no row) and the `minus` runs in memory; that leg
        // goes last so that it narrows no other leg's movies.
        "11a" => load(db.movie.with(link.and(keyword.eq("sequel")).and(production_year.between(1950, 2000)))
                              .select_left(link.select_left(link_ty.rx(r"follow"))
                                           .and(title)
                                           .and(company.select_left(company_name.rx(r"Film|Warner")
                                                                    .and(company_ty.eq(PROD))
                                                                    .and(country.ne("[pl]"))
                                                                    .and(company_note))))),
        "11b" => load(db.movie.with(link.and(keyword.eq("sequel")).and(production_year.eq(1998)))
                              .select_left(title.rx(r"Money")
                                           .and(link.select_left(link_ty.rx(r"follows")))
                                           .and(company.select_left(company_name.rx(r"Film|Warner")
                                                                    .and(company_ty.eq(PROD))
                                                                    .and(country.ne("[pl]"))
                                                                    .and(company_note))))),
        // Root: 181 movies; their company rows top-down.
        "11c" => load(db.movie.with(link.and(keyword.is_in(["sequel", "revenge", "based-on-novel"])).and(production_year.gt(1950)))
                              .select_left(company.select_left(company_name.rx(r"^20th Century Fox|^Twentieth Century Fox")
                                                               .and(company_ty.ne(PROD))
                                                               .and(country.ne("[pl]"))
                                                               .and(company_note))
                                           .and(title))),
        "11d" => load(db.movie.with(link.and(keyword.is_in(["sequel", "revenge", "based-on-novel"])).and(production_year.gt(1950)))
                              .select_left(company.select_left(company_ty.ne(PROD)
                                                               .and(country.ne("[pl]"))
                                                               .and(company_name)
                                                               .and(company_note))
                                           .and(title))),

        // Root: 36k / 102k movies. The rating leg leads, bottom-up (rating > 8.0:
        // 16k movies of all 2.5M), so that the company leg is read only for the
        // root movies it leaves.
        "12a" => load(db.movie.with(info.with(info_ty.eq("genres").and(info_info.is_in(["Drama", "Horror"])))
                                    .and(production_year.between(2005, 2008)))
                              .select_left(data.select(data_ty.eq("rating").and(data_text.gt("8.0")))
                                           .and(company.select_left(country.eq("[us]").and(company_ty.eq(PROD)).and(company_name)))
                                           .and(title))),
        // Root: 6 movies (bottom 10 rank leads); the title regex, then the budget
        // rows of what is left, top-down (not all 14.8M info rows key-only).
        "12b" => load(db.movie.with(data.select(data_ty).eq("bottom 10 rank")
                                    .and(production_year.gt(2000))
                                    .and(company.with(country.eq("[us]").and(company_ty.is_in([PROD, "distributors"])))))
                              .select_left(title.rx(r"^Birdemic|Movie")
                                           .and(info.select_left(info_ty.eq("budget").and(info_info))))),
        "12c" => load(db.movie.with(info.with(info_ty.eq("genres").and(info_info.is_in(["Drama", "Horror", "Western", "Family"])))
                                    .and(production_year.between(2000, 2010)))
                              .select_left(data.select(data_ty.eq("rating").and(data_text.gt("7.0")))
                                           .and(company.select_left(country.eq("[us]").and(company_ty.eq(PROD)).and(company_name)))
                                           .and(title))),

        // Root: 24.6k movies. Their release dates: the info type is an int
        // column, so the `with` is key-only and m.info reads just the 79k
        // release-date rows of root movies.
        "13a" => load(db.movie.with(company.with(country.eq("[de]").and(company_ty.eq(PROD))).and(kind.eq("movie")))
                              .select_left(data.select_left(data_ty.eq("rating").and(data_text))
                                           .and(info.with(info_ty.eq("release dates")).select_left(info_info))
                                           .and(title))),
        // The title regex keeps 295 / 58 of the 663k movies: it leads, and the
        // release-date test moves from the root to them (top-down).
        "13b" => load(db.movie.with(kind.eq("movie"))
                              .select_left(title.ne("").rx(r"Champion|Loser")
                                           .and(info.select_left(info_ty.eq("release dates")))
                                           .and(company.select_left(country.eq("[us]").and(company_ty.eq(PROD)).and(company_name)))
                                           .and(data.select_left(data_ty.eq("rating").and(data_text))))),
        "13c" => load(db.movie.with(kind.eq("movie"))
                              .select_left(title.ne("").rx(r"^Champion|^Loser")
                                           .and(info.select_left(info_ty.eq("release dates")))
                                           .and(company.select_left(country.eq("[us]").and(company_ty.eq(PROD)).and(company_name)))
                                           .and(data.select_left(data_ty.eq("rating").and(data_text))))),
        // Root: 465k movies; the result is large (59k movies), so all of it is read.
        "13d" => load(db.movie.with(kind.eq("movie").and(info.select(info_ty).eq("release dates")))
                              .select_left(data.select_left(data_ty.eq("rating").and(data_text))
                                           .and(company.select_left(country.eq("[us]").and(company_ty.eq(PROD)).and(company_name)))
                                           .and(title))),

        // Keyword, year and kind: 1.9k / ~1k / 7k movies; their country rows top-down.
        "14a" => load(db.movie.with(keyword.is_in(murder4()).and(production_year.gt(2010)).and(kind.eq("movie")))
                              .select_left(info.select_left(info_ty.eq("countries")
                                                            .and(info_info.is_in(["Sweden", "Norway", "Germany", "Denmark", "Swedish",
                                                                                  "Denish", "Norwegian", "German", "USA", "American"])))
                                           .and(data.select_left(data_ty.eq("rating").and(data_text.lt("8.5"))))
                                           .and(title))),
        "14b" => load(db.movie.with(keyword.is_in(["murder", "murder-in-title"]).and(production_year.gt(2010)).and(kind.eq("movie")))
                              .select_left(title.rx(r"murder|Murder|Mord")
                                           .and(info.select_left(info_ty.eq("countries")
                                                                 .and(info_info.is_in(["Sweden", "Norway", "Germany", "Denmark", "Swedish",
                                                                                       "Denish", "Norwegian", "German", "USA", "American"]))))
                                           .and(data.select_left(data_ty.eq("rating").and(data_text.gt("6.0")))))),
        "14c" => load(db.movie.with(keyword.is_in(murder4()).and(production_year.gt(2005)).and(kind.is_in(["movie", "episode"])))
                              .select_left(info.select_left(info_ty.eq("countries").and(info_info.is_in(nordic10())))
                                           .and(data.select_left(data_ty.eq("rating").and(data_text.lt("8.5"))))
                                           .and(title))),

        // Root: 2.4k / 17 movies; their release dates top-down.
        "15a" => load(db.movie.with(company.with(country.eq("[us]").and(company_note.rx(r"\(200.*\)").rx(r"\(worldwide\)")))
                                    .and(aka).and(keyword).and(production_year.gt(2000)))
                              .select_left(info.select_left(info_note.rx(r"internet")
                                                            .and(info_ty.eq("release dates"))
                                                            .and(info_info.rx(r"^USA:.* 200")))
                                           .and(title))),
        "15b" => load(db.movie.with(company.with(company_name.eq("YouTube")
                                                 .and(country.eq("[us]"))
                                                 .and(company_note.rx(r"\(200.*\)").rx(r"\(worldwide\)")))
                                    .and(aka).and(keyword).and(production_year.between(2005, 2010)))
                              .select_left(info.select_left(info_note.rx(r"internet")
                                                            .and(info_ty.eq("release dates"))
                                                            .and(info_info.rx(r"^USA:.* 200")))
                                           .and(title))),
        // Root: 28k movies (2.8M info rows: too many top-down). The internet
        // note keeps 10.7k info rows: key-only, then m.info reads just those.
        "15c" => load(db.movie.with(company.select(country).eq("[us]").and(aka).and(keyword).and(production_year.gt(1990)))
                              .select_left(info.with(info_note.rx(r"internet"))
                                               .select_left(info_ty.eq("release dates").and(info_info.rx(r"^USA:.* 199|^USA:.* 200")))
                                           .and(title))),
        // The internet release dates (10.6k movies) lead the root.
        "15d" => load(db.movie.with(info.with(info_note.rx(r"internet").and(info_ty.eq("release dates")))
                                    .and(company.select(country).eq("[us]"))
                                    .and(keyword)
                                    .and(production_year.gt(1990)))
                              .select_left(aka.and(title))),

        // Root: 17 / 24.6k / 3.9k movies; their cast, persons, aliases top-down
        // (the tree plan starts from all 901k aliases).
        "16a" => load(db.movie.with(keyword.eq(CNIT).and(episode_nr.ge(50).lt(100)).and(company.select(country).eq("[us]")))
                              .select_left(cast.select_left(person).select_left(alias).and(title))),
        "16b" => load(db.movie.with(keyword.eq(CNIT).and(company.select(country).eq("[us]")))
                              .select_left(cast.select_left(person).select_left(alias).and(title))),
        "16c" => load(db.movie.with(keyword.eq(CNIT).and(episode_nr.lt(100)).and(company.select(country).eq("[us]")))
                              .select_left(cast.select_left(person).select_left(alias).and(title))),

        _ => unreachable!("no load plan for {q}"),
    }
}

// ----- In-memory plans -----------------------------------------------------------
//
// queries.rs's query, over the loaded rows, minus what the load already
// checked: the root `with`s (rule 1) and every loaded predicate (rule 5). A
// `with` the load turned top-down comes back as `with(<loaded column>)`.

fn mem_plan(mem: &'static MemJob, q: &str) -> String {
    let MemMovie { title, production_year, keyword, aka, company, cast, info, data, link, .. } = &mem.movie;
    let MemCast { person, role, note: cast_note, character, .. } = &mem.cast;
    let MemPerson { name: person_name, gender, alias, bio, name_pcode_cf, .. } = &mem.person;
    let MemCompany { name: company_name, country, note: company_note, ty: company_ty, .. } = &mem.company;
    let MemInfo { info: info_info, ty: info_ty, note: info_note, .. } = &mem.info;
    let MemData { text: data_text, ty: data_ty, .. } = &mem.data;
    let MemPersonInfo { info: personinfo_info, ty: personinfo_ty, note: personinfo_note, .. } = &mem.person_info;
    let MemMovieLink { ty: link_ty, .. } = &mem.movie_link;

    match q {
        "1a" | "1b" | "1c" | "1d" =>
            min_row(mem.movie.select(company.with(company_ty).select(company_note)
                                     .and(title)
                                     .and(production_year))),

        "2a" | "2b" | "2c" | "2d" => min_row(mem.movie.select(title)),

        "3a" | "3b" | "3c" => min_row(mem.movie.with(info.select(info_info)).select(title)),

        "4a" | "4b" | "4c" => min_row(mem.movie.select(data.with(data_ty).select(data_text).and(title))),

        "5a" | "5b" | "5c" => min_row(mem.movie.with(info.select(info_info)).select(title)),

        "6a" | "6d" | "6e" | "6f" =>
            min_row(mem.movie.with(production_year)
                             .select(keyword.and(title).and(cast.select(person).select(person_name)))),
        "6b" | "6c" => min_row(mem.movie.select(keyword.and(title).and(cast.select(person).select(person_name)))),

        "7a" => min_row(mem.movie.select(cast.select(person.with(alias
                                                                 .and(name_pcode_cf)
                                                                 .and(gender.eq("m").or(gender.eq("f").and(person_name.rx(r"^B"))))
                                                                 .and(bio.with(personinfo_ty.and(personinfo_note)))))
                                              .select(person_name)
                                         .and(title))),
        "7b" => min_row(mem.movie.select(cast.select(person.with(alias
                                                                 .and(name_pcode_cf)
                                                                 .and(gender)
                                                                 .and(bio.with(personinfo_ty.and(personinfo_note)))))
                                              .select(person_name)
                                         .and(title))),
        "7c" => min_row(mem.movie.select(cast.select(person.with(alias
                                                                 .and(name_pcode_cf)
                                                                 .and(gender.eq("m").or(gender.eq("f").and(person_name.rx(r"^A"))))))
                                              .select(person_name.and(bio.with(personinfo_ty.and(personinfo_note))
                                                                         .select(personinfo_info))))),

        "8a" => min_row(mem.movie.select(cast.select(person.with(person_name)).select(alias).and(title))),
        "8b" => min_row(mem.movie.select(cast.with(cast_note.and(role)).select(person.with(person_name)).select(alias)
                                         .and(title))),
        "8c" | "8d" => min_row(mem.movie.select(cast.select(person).select(alias).and(title))),

        "9a" => min_row(mem.movie.select(cast.select(person.with(gender.and(person_name)).select(alias)
                                                     .and(character))
                                         .and(title))),
        "9b" | "9c" | "9d" =>
            min_row(mem.movie.select(cast.select(person.with(gender).select(alias.and(person_name))
                                                 .and(character))
                                     .and(title))),

        "10a" | "10b" => min_row(mem.movie.select(cast.with(cast_note.and(role)).select(character).and(title))),
        "10c" => min_row(mem.movie.select(cast.select(character).and(title))),

        "11a" | "11b" =>
            min_row(mem.movie.select(company.with(company_name.and(company_ty).and(country).minus(company_note))
                                            .select(company_name)
                                     .and(link.select(link_ty))
                                     .and(title))),
        "11c" | "11d" =>
            min_row(mem.movie.select(company.with(company_ty.and(country)).select(company_name.and(company_note))
                                     .and(title))),

        "12a" | "12c" =>
            min_row(mem.movie.select(company.with(country.and(company_ty)).select(company_name)
                                     .and(data.with(data_ty).select(data_text))
                                     .and(title))),
        "12b" => min_row(mem.movie.select(info.with(info_ty).select(info_info).and(title))),

        "13a" => min_row(mem.movie.select(info.select(info_info)
                                          .and(data.with(data_ty).select(data_text))
                                          .and(title))),
        "13b" | "13c" =>
            min_row(mem.movie.with(info.select(info_ty))
                             .select(title
                                     .and(company.with(country.and(company_ty)).select(company_name))
                                     .and(data.with(data_ty).select(data_text)))),
        "13d" =>
            min_row(mem.movie.select(company.with(country.and(company_ty)).select(company_name)
                                     .and(data.with(data_ty).select(data_text))
                                     .and(title))),

        "14a" | "14b" | "14c" =>
            min_row(mem.movie.with(info.with(info_ty).select(info_info))
                             .select(data.with(data_ty).select(data_text).and(title))),

        "15a" | "15b" => min_row(mem.movie.select(info.with(info_note.and(info_ty)).select(info_info).and(title))),
        "15c" => min_row(mem.movie.select(info.with(info_ty).select(info_info).and(title))),
        "15d" => min_row(mem.movie.select(aka.and(title))),

        "16a" | "16b" | "16c" => min_row(mem.movie.select(cast.select(person).select(alias).and(title))),

        _ => unreachable!("no in-memory plan for {q}"),
    }
}

// 16d–33c: queries_loaded.rs's tree plans, unchanged; then the registry.
pub fn entries(db: &'static JobFiles) -> Vec<Entry> {
    let Movie { title, kind, production_year, episode_nr, keyword,
                company, cast, info, data, complete_cast, link, .. } = &db.movie;
    let Cast { person, role, note: cast_note, character, .. } = &db.cast;
    let Person { name: person_name, gender, alias, bio, .. } = &db.person;
    let Company { name: company_name, country, note: company_note, ty: company_ty, .. } = &db.company;
    let Info { info: info_info, ty: info_ty, note: info_note, .. } = &db.info;
    let Data { text: data_text, ty: data_ty, .. } = &db.data;
    let PersonInfo { ty: personinfo_ty, .. } = &db.person_info;
    let MovieLink { target, ty: link_ty, .. } = &db.movie_link;
    let CompleteCast { status, subject, .. } = &db.complete_cast;

    let q16d = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(keyword.eq("character-name-in-title"))
                      .and(episode_nr.ge(5).lt(100)))
                .select(cast.select(person).select(alias)
                        .and(title))
    };

    let q17a = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(person_name).rx(r"^B"))
    };

    let q17b = move || {
        db.movie.with(company
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(person_name).rx(r"^Z"))
    };

    let q17c = move || {
        db.movie.with(company
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(person_name).rx(r"^X"))
    };

    let q17d = move || {
        db.movie.with(company
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(person_name).rx(r"Bert"))
    };

    let q17e = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(person_name))
    };

    let q17f = move || {
        db.movie.with(company
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(person_name).rx(r"B"))
    };

    let q18a = move || {
        // `budget` leads the with although the select repeats it: the test is
        // cheap and cuts the drive to movies with a budget before the cast
        // walk and its name regex.
        let budget = || info.with(info_ty.eq("budget")).select(info_info);
        db.movie.with(budget()
                      .and(cast.with(cast_note.is_in(["(producer)", "(executive producer)"])
                                     .and(person.with(gender.eq("m").and(person_name.rx(r"Tim")))))))
                .select(budget()
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title))
    };

    #[cfg(any())] // pending: Diff
    let q18b = move || {
        // The genre test leads the with although the select repeats it: it is
        // cheap and cuts the drive before the cast walk.
        let gf = || info_ty.eq("genres").and(info_info.is_in(horror2())).minus(info_note);
        db.movie.with(info.with(gf())
                      .and(production_year.between(2008, 2014))
                      .and(cast.with(cast_note.is_in(writer5())
                                     .and(person.select(gender).eq("f")))))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("rating")).select(data_text.gt("8.0")))
                        .and(title))
    };

    let q18c = move || {
        // The genre test leads the with although the select repeats it: it is
        // cheap and cuts the drive before the cast walk.
        let gf = || info_ty.eq("genres").and(info_info.is_in(genre6()));
        db.movie.with(info.with(gf())
                      .and(cast.with(cast_note.is_in(writer5())
                                     .and(person.select(gender).eq("m")))))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title))
    };

    let q19a = move || {
        db.movie.with(company.with(country.eq("[us]")
                                   .and(company_note.rx(r"\(USA\)|\(worldwide\)")))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*200|^USA:.*200"))))
                      .and(production_year.between(2005, 2009)))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress"))
                                  .and(character))
                            .select(person.with(gender.eq("f").and(alias)).select(person_name.rx(r"Ang")))
                        .and(title))
    };

    let q19b = move || {
        db.movie.with(company.with(country.eq("[us]")
                                   .and(company_note.rx(r"\(200.*\)").rx(r"\(USA\)|\(worldwide\)")))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*2007|^USA:.*2008"))))
                      .and(production_year.between(2007, 2008)))
                .select(cast.with(cast_note.eq("(voice)")
                                  .and(role.eq("actress"))
                                  .and(character))
                            .select(person.with(gender.eq("f").and(alias)).select(person_name.rx(r"Angel")))
                        .and(title.rx(r"Kung.*Fu.*Panda")))
    };

    let q19c = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*200|^USA:.*200"))))
                      .and(production_year.gt(2000)))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress"))
                                  .and(character))
                            .select(person.with(gender.eq("f").and(alias)).select(person_name.rx(r"An")))
                        .and(title))
    };

    let q19d = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(info.select(info_ty).eq("release dates"))
                      .and(production_year.gt(2000)))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress"))
                                  .and(character))
                            .select(person.with(gender.eq("f").and(alias)).select(person_name))
                        .and(title))
    };

    let q20a = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.rx(r"complete")))
                      .and(keyword.is_in(kw8()))
                      .and(kind.eq("movie"))
                      .and(production_year.gt(1950))
                      .and(cast.select(character).nrx(r"Sherlock").rx(r"Tony.*Stark|Iron.*Man")))
                .select(title)
    };

    let q20b = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.rx(r"complete")))
                      .and(keyword.is_in(kw8()))
                      .and(kind.eq("movie"))
                      .and(production_year.gt(2000))
                      .and(cast.with(character.nrx(r"Sherlock").rx(r"Tony.*Stark|Iron.*Man")
                                     .and(person.select(person_name).rx(r"Downey.*Robert")))))
                .select(title)
    };

    let q20c = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.rx(r"complete")))
                      .and(keyword.is_in(kw10()))
                      .and(kind.eq("movie"))
                      .and(production_year.gt(2000)))
                .select(cast.with(character.rx(r"[Mm]an")).select(person).select(person_name)
                        .and(title))
    };

    // The company and link-type tests live only in the select: it is a join,
    // so a movie without a Film/Warner company or a "follow" link simply
    // probes to no row, and repeating either test in the `with` would only
    // add work. The `with` is ordered cheapest-and-most-selective first, and
    // bare `link` leads although the select's link test makes it redundant:
    // only 6.4k of the 2.5M movies have a link, and the test is a
    // CSR-emptiness check rather than a walk of the movie's keyword ids.
    // Leading with the keyword test instead is 8x slower, and leading with
    // the `follow` regex over the links is 20% slower (q21a–c, q27a–c).
    #[cfg(any())] // pending: Diff
    let q21a = move || {
        db.movie.with(link.and(keyword.eq("sequel"))
                      .and(production_year.between(1950, 2000))
                      .and(info.select(info_info).is_in(nordic8())))
                .select(company.with(country.ne("[pl]")
                                     .and(company_name.rx(r"Film|Warner"))
                                     .and(company_ty.eq("production companies"))
                                     .minus(company_note))
                               .select(company_name)
                        .and(link.select(link_ty).rx(r"follow"))
                        .and(title))
    };

    #[cfg(any())] // pending: Diff
    let q21b = move || {
        db.movie.with(link.and(keyword.eq("sequel"))
                      .and(production_year.between(2000, 2010))
                      .and(info.select(info_info).is_in(["Germany", "German"])))
                .select(company.with(country.ne("[pl]")
                                     .and(company_name.rx(r"Film|Warner"))
                                     .and(company_ty.eq("production companies"))
                                     .minus(company_note))
                               .select(company_name)
                        .and(link.select(link_ty).rx(r"follow"))
                        .and(title))
    };

    #[cfg(any())] // pending: Diff
    let q21c = move || {
        db.movie.with(link.and(keyword.eq("sequel"))
                      .and(production_year.between(1950, 2010))
                      .and(info.select(info_info).is_in(nordic9())))
                .select(company.with(country.ne("[pl]")
                                     .and(company_name.rx(r"Film|Warner"))
                                     .and(company_ty.eq("production companies"))
                                     .minus(company_note))
                               .select(company_name)
                        .and(link.select(link_ty).rx(r"follow"))
                        .and(title))
    };

    let q22a = move || {
        db.movie.with(info.with(info_ty.eq("countries")
                                .and(info_info.is_in(["Germany", "German", "USA", "American"])))
                      .and(keyword.is_in(murder4()))
                      .and(production_year.gt(2008))
                      .and(kind.is_in(["movie", "episode"])))
                .select(title
                        .and(data.with(data_ty.eq("rating")).select(data_text.lt("7.0")))
                        .and(company.with(company_note.nrx(r"\(USA\)").rx(r"\(200.*\)")
                                          .and(country.ne("[us]"))
                                          .and(company_ty.eq("production companies")))
                                    .select(company_name)))
    };

    let q22b = move || {
        db.movie.with(info.with(info_ty.eq("countries")
                                .and(info_info.is_in(["Germany", "German", "USA", "American"])))
                      .and(keyword.is_in(murder4()))
                      .and(production_year.gt(2009))
                      .and(kind.is_in(["movie", "episode"])))
                .select(title
                        .and(data.with(data_ty.eq("rating")).select(data_text.lt("7.0")))
                        .and(company.with(company_note.nrx(r"\(USA\)").rx(r"\(200.*\)")
                                          .and(country.ne("[us]"))
                                          .and(company_ty.eq("production companies")))
                                    .select(company_name)))
    };

    let q22c = move || {
        db.movie.with(info.with(info_ty.eq("countries")
                                .and(info_info.is_in(nordic10())))
                      .and(keyword.is_in(murder4()))
                      .and(production_year.gt(2005))
                      .and(kind.is_in(["movie", "episode"])))
                .select(title
                        .and(data.with(data_ty.eq("rating")).select(data_text.lt("8.5")))
                        .and(company.with(company_note.nrx(r"\(USA\)").rx(r"\(200.*\)")
                                          .and(country.ne("[us]"))
                                          .and(company_ty.eq("production companies")))
                                    .select(company_name)))
    };

    let q22d = move || {
        db.movie.with(info.with(info_ty.eq("countries").and(info_info.is_in(nordic10())))
                      .and(keyword.is_in(murder4()))
                      .and(production_year.gt(2005))
                      .and(kind.is_in(["movie", "episode"])))
                .select(title
                        .and(data.with(data_ty.eq("rating")).select(data_text.lt("8.5")))
                        .and(company.with(country.ne("[us]")
                                          .and(company_ty.eq("production companies")))
                                    .select(company_name)))
    };

    let q23a = move || {
        db.movie.with(complete_cast.select(status).eq("complete+verified")
                      .and(company.select(country).eq("[us]"))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_note.rx(r"internet"))
                                     .and(info_info.rx(r"^USA:.* 199|^USA:.* 200"))))
                      .and(keyword)
                      .and(production_year.gt(2000)))
                .select(kind.eq("movie").and(title))
    };

    let q23b = move || {
        db.movie.with(complete_cast.select(status).eq("complete+verified")
                      .and(company.select(country).eq("[us]"))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_note.rx(r"internet"))
                                     .and(info_info.rx(r"^USA:.* 200"))))
                      .and(keyword.is_in(["nerd", "loner", "alienation", "dignity"]))
                      .and(production_year.gt(2000)))
                .select(kind.eq("movie").and(title))
    };

    let q23c = move || {
        db.movie.with(complete_cast.select(status).eq("complete+verified")
                      .and(company.select(country).eq("[us]"))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_note.rx(r"internet"))
                                     .and(info_info.rx(r"^USA:.* 199|^USA:.* 200"))))
                      .and(keyword)
                      .and(production_year.gt(1990)))
                .select(kind.is_in(["movie", "tv movie", "video movie", "video game"]).and(title))
    };

    let q24a = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*201|^USA:.*201"))))
                      .and(keyword.is_in(["hero", "martial-arts", "hand-to-hand-combat"]))
                      .and(production_year.gt(2010)))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress")))
                            .select(character
                                    .and(person.with(gender.eq("f").and(alias)).select(person_name.rx(r"An"))))
                        .and(title))
    };

    let q24b = move || {
        db.movie.with(company.with(country.eq("[us]")
                                   .and(company_name.eq("DreamWorks Animation")))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*201|^USA:.*201"))))
                      .and(keyword.is_in(["hero", "martial-arts", "hand-to-hand-combat", "computer-animated-movie"]))
                      .and(production_year.gt(2010)))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress")))
                            .select(character
                                    .and(person.with(gender.eq("f").and(alias)).select(person_name.rx(r"An"))))
                        .and(title.rx(r"^Kung Fu Panda")))
    };

    let q25a = move || {
        db.movie.with(keyword.is_in(["murder", "blood", "gore", "death", "female-nudity"]))
                .select(info.with(info_ty.eq("genres").and(info_info.eq("Horror"))).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5()))
                                 .select(person.with(gender.eq("m"))).select(person_name)))
    };

    let q25b = move || {
        db.movie.with(keyword.is_in(["murder", "blood", "gore", "death", "female-nudity"])
                      .and(production_year.gt(2010)))
                .select(info.with(info_ty.eq("genres").and(info_info.eq("Horror"))).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title.rx(r"^Vampire"))
                        .and(cast.with(cast_note.is_in(writer5()))
                                 .select(person.with(gender.eq("m"))).select(person_name)))
    };

    let q25c = move || {
        db.movie.with(keyword.is_in(kw7()))
                .select(info.with(info_ty.eq("genres").and(info_info.is_in(genre6()))).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5()))
                                 .select(person.with(gender.eq("m"))).select(person_name)))
    };

    let q26a = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.rx(r"complete")))
                      .and(keyword.is_in(kw10()))
                      .and(kind.eq("movie"))
                      .and(production_year.gt(2000)))
                .select(cast.select(character.rx(r"[Mm]an").and(person.select(person_name)))
                        .and(data.with(data_ty.eq("rating")).select(data_text.gt("7.0")))
                        .and(title))
    };

    let q26b = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.rx(r"complete")))
                      .and(keyword.is_in(["superhero", "marvel-comics", "based-on-comic", "fight"]))
                      .and(kind.eq("movie"))
                      .and(production_year.gt(2005)))
                .select(cast.select(character.rx(r"[Mm]an"))
                        .and(data.with(data_ty.eq("rating")).select(data_text.gt("8.0")))
                        .and(title))
    };

    let q26c = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.rx(r"complete")))
                      .and(keyword.is_in(kw10()))
                      .and(kind.eq("movie"))
                      .and(production_year.gt(2000)))
                .select(cast.select(character.rx(r"[Mm]an"))
                        .and(data.with(data_ty.eq("rating")).select(data_text))
                        .and(title))
    };

    #[cfg(any())] // pending: Diff
    let q27a = move || {
        db.movie.with(link.and(keyword.eq("sequel"))
                      .and(production_year.between(1950, 2000))
                      .and(info.select(info_info).is_in(["Sweden", "Germany", "Swedish", "German"]))
                      .and(complete_cast.with(subject.is_in(["cast", "crew"]).and(status.eq("complete")))))
                .select(company.with(country.ne("[pl]")
                                     .and(company_name.rx(r"Film|Warner"))
                                     .and(company_ty.eq("production companies"))
                                     .minus(company_note))
                               .select(company_name)
                        .and(link.select(link_ty).rx(r"follow"))
                        .and(title))
    };

    #[cfg(any())] // pending: Diff
    let q27b = move || {
        db.movie.with(link.and(keyword.eq("sequel"))
                      .and(production_year.eq(1998))
                      .and(info.select(info_info).is_in(["Sweden", "Germany", "Swedish", "German"]))
                      .and(complete_cast.with(subject.is_in(["cast", "crew"]).and(status.eq("complete")))))
                .select(company.with(country.ne("[pl]")
                                     .and(company_name.rx(r"Film|Warner"))
                                     .and(company_ty.eq("production companies"))
                                     .minus(company_note))
                               .select(company_name)
                        .and(link.select(link_ty).rx(r"follow"))
                        .and(title))
    };

    #[cfg(any())] // pending: Diff
    let q27c = move || {
        db.movie.with(link.and(keyword.eq("sequel"))
                      .and(production_year.between(1950, 2010))
                      .and(info.select(info_info).is_in(nordic9()))
                      .and(complete_cast.with(subject.eq("cast").and(status.rx(r"^complete")))))
                .select(company.with(country.ne("[pl]")
                                     .and(company_name.rx(r"Film|Warner"))
                                     .and(company_ty.eq("production companies"))
                                     .minus(company_note))
                               .select(company_name)
                        .and(link.select(link_ty).rx(r"follow"))
                        .and(title))
    };

    let q28a = move || {
        db.movie.with(complete_cast.with(subject.eq("crew").and(status.ne("complete+verified")))
                      .and(info.with(info_ty.eq("countries").and(info_info.is_in(nordic10()))))
                      .and(keyword.is_in(murder4()))
                      .and(kind.is_in(["movie", "episode"]))
                      .and(production_year.gt(2000)))
                .select(company.with(country.ne("[us]")
                                     .and(company_note.nrx(r"\(USA\)").rx(r"\(200.*\)")))
                               .select(company_name)
                        .and(data.with(data_ty.eq("rating")).select(data_text.lt("8.5")))
                        .and(title))
    };

    let q28b = move || {
        db.movie.with(complete_cast.with(subject.eq("crew").and(status.ne("complete+verified")))
                      .and(info.with(info_ty.eq("countries").and(info_info.is_in(["Sweden", "Germany", "Swedish", "German"]))))
                      .and(keyword.is_in(murder4()))
                      .and(kind.is_in(["movie", "episode"]))
                      .and(production_year.gt(2005)))
                .select(company.with(country.ne("[us]")
                                     .and(company_note.nrx(r"\(USA\)").rx(r"\(200.*\)")))
                               .select(company_name)
                        .and(data.with(data_ty.eq("rating")).select(data_text.gt("6.5")))
                        .and(title))
    };

    let q28c = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.eq("complete")))
                      .and(info.with(info_ty.eq("countries").and(info_info.is_in(nordic10()))))
                      .and(keyword.is_in(murder4()))
                      .and(kind.is_in(["movie", "episode"]))
                      .and(production_year.gt(2005)))
                .select(company.with(country.ne("[us]")
                                     .and(company_note.nrx(r"\(USA\)").rx(r"\(200.*\)")))
                               .select(company_name)
                        .and(data.with(data_ty.eq("rating")).select(data_text.lt("8.5")))
                        .and(title))
    };

    let q29a = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.eq("complete+verified")))
                      .and(company.select(country).eq("[us]"))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*200|^USA:.*200"))))
                      .and(keyword.eq("computer-animation"))
                      .and(production_year.between(2000, 2010)))
                .select(cast.with(cast_note.is_in(voice3())
                                  .and(role.eq("actress")))
                            .select(character.eq("Queen")
                                    .and(person.with(gender.eq("f")
                                                     .and(alias)
                                                     .and(bio.select(personinfo_ty).eq("trivia")))
                                               .select(person_name.rx(r"An"))))
                        .and(title.eq("Shrek 2")))
    };

    let q29b = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.eq("complete+verified")))
                      .and(company.select(country).eq("[us]"))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^USA:.*200"))))
                      .and(keyword.eq("computer-animation"))
                      .and(production_year.between(2000, 2005)))
                .select(cast.with(cast_note.is_in(voice3())
                                  .and(role.eq("actress")))
                            .select(character.eq("Queen")
                                    .and(person.with(gender.eq("f")
                                                     .and(alias)
                                                     .and(bio.select(personinfo_ty).eq("height")))
                                               .select(person_name.rx(r"An"))))
                        .and(title.eq("Shrek 2")))
    };

    let q29c = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.eq("complete+verified")))
                      .and(company.select(country).eq("[us]"))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*200|^USA:.*200"))))
                      .and(keyword.eq("computer-animation"))
                      .and(production_year.between(2000, 2010)))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress")))
                            .select(character
                                    .and(person.with(gender.eq("f")
                                                     .and(alias)
                                                     .and(bio.select(personinfo_ty).eq("trivia")))
                                               .select(person_name.rx(r"An"))))
                        .and(title))
    };

    let q30a = move || {
        db.movie.with(complete_cast.with(subject.is_in(["cast", "crew"]).and(status.eq("complete+verified")))
                      .and(keyword.is_in(kw7()))
                      .and(production_year.gt(2000)))
                .select(info.with(info_ty.eq("genres").and(info_info.is_in(horror2()))).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5()))
                                 .select(person.with(gender.eq("m"))).select(person_name)))
    };

    let q30b = move || {
        db.movie.with(complete_cast.with(subject.is_in(["cast", "crew"]).and(status.eq("complete+verified")))
                      .and(keyword.is_in(kw7()))
                      .and(production_year.gt(2000)))
                .select(info.with(info_ty.eq("genres").and(info_info.is_in(horror2()))).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title.rx(r"Freddy|Jason|^Saw"))
                        .and(cast.with(cast_note.is_in(writer5()))
                                 .select(person.with(gender.eq("m"))).select(person_name)))
    };

    let q30c = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.eq("complete+verified")))
                      .and(keyword.is_in(kw7())))
                .select(info.with(info_ty.eq("genres").and(info_info.is_in(genre6()))).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5()))
                                 .select(person.with(gender.eq("m"))).select(person_name)))
    };

    let q31a = move || {
        db.movie.with(company.select(company_name).rx(r"^Lionsgate")
                      .and(keyword.is_in(kw7())))
                .select(info.with(info_ty.eq("genres").and(info_info.is_in(horror2()))).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5()))
                                 .select(person.with(gender.eq("m"))).select(person_name)))
    };

    let q31b = move || {
        db.movie.with(company.with(company_name.rx(r"^Lionsgate")
                                   .and(company_note.rx(r"\(Blu-ray\)")))
                      .and(keyword.is_in(kw7()))
                      .and(production_year.gt(2000)))
                .select(info.with(info_ty.eq("genres").and(info_info.is_in(horror2()))).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title.rx(r"Freddy|Jason|^Saw"))
                        .and(cast.with(cast_note.is_in(writer5()))
                                 .select(person.with(gender.eq("m"))).select(person_name)))
    };

    let q31c = move || {
        db.movie.with(company.select(company_name).rx(r"^Lionsgate")
                      .and(keyword.is_in(kw7())))
                .select(info.with(info_ty.eq("genres").and(info_info.is_in(genre6()))).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5())).select(person).select(person_name)))
    };

    let q32a = move || {
        // Bare `link` leads the with although the select repeats it: only 6.4k
        // movies have one, and the test is a CSR-emptiness check, so it cuts
        // the drive before the keyword walk.
        db.movie.with(link.and(keyword.eq("10,000-mile-club")))
                .select(link.select(link_ty.and(target.select(title)))
                        .and(title))
    };

    let q32b = move || {
        // Bare `link` leads the with although the select repeats it: only 6.4k
        // movies have one, and the test is a CSR-emptiness check, so it cuts
        // the drive before the keyword walk.
        db.movie.with(link.and(keyword.eq("character-name-in-title")))
                .select(link.select(link_ty.and(target.select(title)))
                        .and(title))
    };

    let q33a = move || {
        db.movie.with(kind.eq("tv series"))
                .select(company.with(country.eq("[us]")).select(company_name)
                        .and(data.with(data_ty.eq("rating")).select(data_text))
                        .and(title)
                        .and(link.with(link_ty.is_in(link3()))
                                 .select(target.with(kind.eq("tv series")
                                                     .and(production_year.between(2005, 2008))))
                                 .select(company.select(company_name)
                                         .and(data.with(data_ty.eq("rating")).select(data_text.lt("3.0")))
                                         .and(title))))
    };

    let q33b = move || {
        db.movie.with(kind.eq("tv series"))
                .select(company.with(country.eq("[nl]")).select(company_name)
                        .and(data.with(data_ty.eq("rating")).select(data_text))
                        .and(title)
                        .and(link.with(link_ty.rx(r"follow"))
                                 .select(target.with(kind.eq("tv series")
                                                     .and(production_year.eq(2007))))
                                 .select(company.select(company_name)
                                         .and(data.with(data_ty.eq("rating")).select(data_text.lt("3.0")))
                                         .and(title))))
    };

    let q33c = move || {
        db.movie.with(kind.is_in(["tv series", "episode"]))
                .select(company.with(country.ne("[us]")).select(company_name)
                        .and(data.with(data_ty.eq("rating")).select(data_text))
                        .and(title)
                        .and(link.with(link_ty.is_in(link3()))
                                 .select(target.with(kind.is_in(["tv series", "episode"])
                                                     .and(production_year.between(2000, 2010))))
                                 .select(company.select(company_name)
                                         .and(data.with(data_ty.eq("rating")).select(data_text.lt("3.5")))
                                         .and(title))))
    };

    vec![
        // 1a–16c: optimised (load plan + in-memory plan, see above).
        entry("1a", "(A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934", move || optimised(db, "1a")),
        entry("1b", "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2008", move || optimised(db, "1b")),
        entry("1c", "(co-production) || Intouchables || 2011", move || optimised(db, "1c")),
        entry("1d", "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2004", move || optimised(db, "1d")),
        entry("2a", "'Doc'", move || optimised(db, "2a")),
        entry("2b", "'Doc'", move || optimised(db, "2b")),
        entry("2c", "(empty)", move || optimised(db, "2c")),
        entry("2d", "& Teller", move || optimised(db, "2d")),
        entry("3a", "2 Days in New York", move || optimised(db, "3a")),
        entry("3b", "300: Rise of an Empire", move || optimised(db, "3b")),
        entry("3c", "& Teller 2", move || optimised(db, "3c")),
        entry("4a", "5.1 || & Teller 2", move || optimised(db, "4a")),
        entry("4b", "9.1 || Batman: Arkham City", move || optimised(db, "4b")),
        entry("4c", "2.1 || & Teller 2", move || optimised(db, "4c")),
        entry("5a", "(empty)", move || optimised(db, "5a")),
        entry("5b", "(empty)", move || optimised(db, "5b")),
        entry("5c", "11,830,420", move || optimised(db, "5c")),
        entry("6a", "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", move || optimised(db, "6a")),
        entry("6b", "based-on-comic || The Avengers 2 || Downey Jr., Robert", move || optimised(db, "6b")),
        entry("6c", "marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert", move || optimised(db, "6c")),
        entry("6d", "based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert", move || optimised(db, "6d")),
        entry("6e", "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", move || optimised(db, "6e")),
        entry("6f", "based-on-comic || & Teller 2 || \"Steff\", Stefanie Oxmann Mcgaha", move || optimised(db, "6f")),
        entry("7a", "Antonioni, Michelangelo || Dressed to Kill", move || optimised(db, "7a")),
        entry("7b", "De Palma, Brian || Dressed to Kill", move || optimised(db, "7b")),
        entry("7c",
              "50 Cent || \"Boo\" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as \"Boo\" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.",
              move || optimised(db, "7c")),
        entry("8a", "Chambers, Linda || .hack//Quantum", move || optimised(db, "8a")),
        entry("8b", "Chambers, Linda || Dragon Ball Z: Shin Budokai", move || optimised(db, "8b")),
        entry("8c", "\"A.J.\" || #1 Cheerleader Camp", move || optimised(db, "8c")),
        entry("8d", "\"Jenny from the Block\" || #1 Cheerleader Camp", move || optimised(db, "8d")),
        entry("9a", "AJ || Airport Announcer || Blue Harvest", move || optimised(db, "9a")),
        entry("9b", "AJ || Bassett, Angela || Airport Announcer || Blue Harvest", move || optimised(db, "9b")),
        entry("9c", "'Annette' || Alborg, Ana Esther || 2nd Balladeer || (1975-01-20)", move || optimised(db, "9c")),
        entry("9d", "!!!, Toy || Aaron, Caroline || \"Cockamamie's\" Salesgirl || $15,000.00 Error", move || optimised(db, "9d")),
        entry("10a", "Actor || 12 Rounds", move || optimised(db, "10a")),
        entry("10b", "(empty)", move || optimised(db, "10b")),
        entry("10c", "Himself || Evil Eyes: Behind the Scenes", move || optimised(db, "10c")),
        entry("11a", "Churchill Films || followed by || Batman Beyond", move || optimised(db, "11a")),
        entry("11b", "Filmlance International AB || follows || The Money Man", move || optimised(db, "11b")),
        entry("11c", "20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24", move || optimised(db, "11c")),
        entry("11d", "13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun", move || optimised(db, "11d")),
        entry("12a", "10th Grade Reunion Films || 8.1 || 3:20", move || optimised(db, "12a")),
        entry("12b", "$10,000 || Birdemic: Shock and Terror", move || optimised(db, "12b")),
        entry("12c", "\"Oh That Gus!\" || 7.1 || $1.11", move || optimised(db, "12c")),
        entry("13a", "Afghanistan:24 June 2012 || 1.0 || &Me", move || optimised(db, "13a")),
        entry("13b", "5 Time Champion || 501audio || 1.8", move || optimised(db, "13b")),
        entry("13c", "Champion || DL Sites || 1.8", move || optimised(db, "13c")),
        entry("13d", "\"O\" Films || 1.0 || #54 Meets #47", move || optimised(db, "13d")),
        entry("14a", "1.0 || $lowdown", move || optimised(db, "14a")),
        entry("14b", "6.4 || Of Dolls and Murder", move || optimised(db, "14b")),
        entry("14c", "1.0 || $lowdown", move || optimised(db, "14c")),
        entry("15a", "USA:1 June 2007 || Battlestar Galactica: The Resistance", move || optimised(db, "15a")),
        entry("15b", "USA:27 April 2007 || RoboCop vs Terminator", move || optimised(db, "15b")),
        entry("15c", "USA:1 April 2003 || 24: Day Six - Debrief", move || optimised(db, "15c")),
        entry("15d", "(Not So) Instant Photo || 06/05", move || optimised(db, "15d")),
        entry("16a", "Adams, Stan || Carol Burnett vs. Anthony Perkins", move || optimised(db, "16a")),
        entry("16b", "!!!, Toy || & Teller", move || optimised(db, "16b")),
        entry("16c", "\"Brooklyn\" Tony Danza || (#1.5)", move || optimised(db, "16c")),
        // 16d–33c: queries_loaded.rs's tree plans, unchanged.
        entry("16d", "\"Brooklyn\" Tony Danza || (#1.5)", move || min_row(loaded(q16d()))),
        entry("17a", "B, Khaz", move || min_row(loaded(q17a()))),
        entry("17b", "Z'Dar, Robert", move || min_row(loaded(q17b()))),
        entry("17c", "X'Volaitis, John", move || min_row(loaded(q17c()))),
        entry("17d", "Abrahamsson, Bertil", move || min_row(loaded(q17d()))),
        entry("17e", "$hort, Too", move || min_row(loaded(q17e()))),
        entry("17f", "'El Galgo PornoStar', Blanquito", move || min_row(loaded(q17f()))),
        entry("18a", "$1,000 || 10 || 40 Days and 40 Nights", move || min_row(loaded(q18a()))),
        entry("18b", "Horror || 8.1 || Agorable", move || pending("Diff")),
        entry("18c", "Action || 10 || #PostModem", move || min_row(loaded(q18c()))),
        entry("19a", "Angeline, Moriah || Blue Harvest", move || min_row(loaded(q19a()))),
        entry("19b", "Jolie, Angelina || Kung Fu Panda", move || min_row(loaded(q19b()))),
        entry("19c", "Alborg, Ana Esther || .hack//Akusei heni vol. 2", move || min_row(loaded(q19c()))),
        entry("19d", "Aaron, Caroline || $9.99", move || min_row(loaded(q19d()))),
        entry("20a", "Disaster Movie", move || min_row(loaded(q20a()))),
        entry("20b", "Iron Man", move || min_row(loaded(q20b()))),
        entry("20c", "Abell, Alistair || ...And Then I...", move || min_row(loaded(q20c()))),
        entry("21a", "Det Danske Filminstitut || followed by || Der Serienkiller - Klinge des Todes", move || pending("Diff")),
        entry("21b", "Filmlance International AB || followed by || Hämndens pris", move || pending("Diff")),
        entry("21c", "Churchill Films || followed by || Batman Beyond", move || pending("Diff")),
        entry("22a", "(empty)", move || min_row(loaded(q22a()))),
        entry("22b", "(empty)", move || min_row(loaded(q22b()))),
        entry("22c", "(empty)", move || min_row(loaded(q22c()))),
        entry("22d", "(#1.1) || 2.0 || 13 Productions", move || min_row(loaded(q22d()))),
        entry("23a", "movie || The Analysts", move || min_row(loaded(q23a()))),
        entry("23b", "movie || The Big Mope", move || min_row(loaded(q23b()))),
        entry("23c", "movie || Dirt Merchant", move || min_row(loaded(q23c()))),
        entry("24a", "Additional Voices || Baker, Andrea || Baiohazâdo 6", move || min_row(loaded(q24a()))),
        entry("24b", "Tigress || Jolie, Angelina || Kung Fu Panda 2", move || min_row(loaded(q24b()))),
        entry("25a", "Horror || 10 || -- And Now the Screaming Starts! || Abdallah, Damon", move || min_row(loaded(q25a()))),
        entry("25b", "Horror || 138 || Vampire Boys || Campbell, Jeremiah", move || min_row(loaded(q25b()))),
        entry("25c", "Action || 10 || $ || Aakeson, Kim Fupz", move || min_row(loaded(q25c()))),
        entry("26a", "'Agua' Man || Acereda, Hermie || 7.1 || 3:10 to Yuma", move || min_row(loaded(q26a()))),
        entry("26b", "Bank Manager || 8.2 || Inception", move || min_row(loaded(q26b()))),
        entry("26c", "'Agua' Man || 1.9 || 12 Rounds", move || min_row(loaded(q26c()))),
        entry("27a", "Det Danske Filminstitut || followed by || Spår i mörker", move || pending("Diff")),
        entry("27b", "Filmlance International AB || followed by || Vita nätter", move || pending("Diff")),
        entry("27c", "Det Danske Filminstitut || followed by || Spår i mörker", move || pending("Diff")),
        entry("28a", "01 Distribuzione || 2.9 || (#1.1)", move || min_row(loaded(q28a()))),
        entry("28b", "20th Century Fox || 6.6 || (#1.1)", move || min_row(loaded(q28b()))),
        entry("28c", "01 Distribuzione || 1.9 || (#1.1)", move || min_row(loaded(q28c()))),
        entry("29a", "Queen || Andrews, Julie || Shrek 2", move || min_row(loaded(q29a()))),
        entry("29b", "Queen || Andrews, Julie || Shrek 2", move || min_row(loaded(q29b()))),
        entry("29c", "Lola || Andrews, Julie || Hoodwinked!", move || min_row(loaded(q29c()))),
        entry("30a", "Horror || 100356 || 16 Blocks || Abrams, J.J.", move || min_row(loaded(q30a()))),
        entry("30b", "Horror || 194782 || Freddy vs. Jason || Shannon, Damian", move || min_row(loaded(q30b()))),
        entry("30c", "Action || 100356 || $ || Abernathy, Lewis", move || min_row(loaded(q30c()))),
        entry("31a", "Horror || 1040 || 2001 Maniacs || Agnew, Jim", move || min_row(loaded(q31a()))),
        entry("31b", "Horror || 129755 || Saw || Bousman, Darren Lynn", move || min_row(loaded(q31b()))),
        entry("31c", "Action || 1008 || 11:14 || Abraham, Brad", move || min_row(loaded(q31c()))),
        entry("32a", "(empty)", move || min_row(loaded(q32a()))),
        entry("32b", "alternate language version of || 'Angel': Season 2 Overview || 12 oz. Mouse", move || min_row(loaded(q32b()))),
        entry("33a", "495 Productions || 3.3 || A Double Shot at Love || 495 Productions || 2.7 || A Shot at Love with Tila Tequila", move || min_row(loaded(q33a()))),
        entry("33b", "MTV Netherlands || 3.3 || A Double Shot at Love || 495 Productions || 2.7 || A Shot at Love with Tila Tequila", move || min_row(loaded(q33b()))),
        entry("33c", "2BE || 1.3 || A Double Shot at Love || 495 Productions || 1.0 || A Double Shot at Love", move || min_row(loaded(q33c()))),
        entry("6a/method", "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", move || min_row(loaded(q6a_methods(db)))),
    ]
}

// ===== Reference example of the method-chain form. Kept as a registered
// query so `cargo asm` always has a known symbol to inspect. =====

// q6a — movie : (year > 2010) ∧ (keyword == "marvel-...")
//             → (keyword == "marvel-...") × title
//             × (cast → person → name ~ "Downey…")
//
// Operator legend (engine.rs::QueryExt; everything roots on IntoQuery, so
// the destructured columns — `keyword`, `title`, `cast` — mix freely with
// plan nodes):
//   .select(b)    composition (a set is an identity relation, so set∘Query is
//            the same Compose — no keyset projection), and also how you
//            navigate: `cast.select(person).select(name)` walks
//            Movie → Cast → Person → name, one column per hop
//            Lookup tables are not hops: `kind`, `keyword`, `role`, `ty`
//            are dictionary-encoded string columns (`Dict`/`DictSet`),
//            so `kind.eq("movie")` compares the string directly
//   .and(b)    product (×)
//   .and     ∧ — alias for the product; conjunct trees are consumed via
//            the flat short-circuit `member` (restriction = `.with`)
//   .or      ∨ — probe-only membership union (drive with `.union`)
//   .minus   value-bearing difference (key-based member test)
//   .with    restriction — keep rows whose value is a member
//   .eq / .ne / .gt / .lt / .ge / .le / .is_in / .rx / .nrx  predicates
pub fn q6a_methods(db: &'static JobFiles) -> impl Load<R: Row, Out: Drive> {
    let Movie { title, production_year, keyword, cast, .. } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Person { name, .. } = &db.person;
    let kw_marvel = || keyword.eq("marvel-cinematic-universe");
    let q = db.movie.with(production_year.gt(2010)
                          .and(kw_marvel()))
                    .select(kw_marvel()
                            .and(title)
                            .and(cast.select(person).select(name).rx(r"Downey.*Robert")));
    q
}
