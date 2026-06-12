// `pull-next` mini-suite — the consumption-style axis of the pull-vs-push
// experiment. Six queries spanning shapes (JOB q1a shallow, q2a, q29a deep
// t6 chain; TPC-H q1, q9, q21), each the PULL port with every consuming
// sink swapped from internal iteration (`.fold()`/`.for_each()`, which std
// forwards through `fold`/`try_fold`) to a raw `for` loop (external
// `next()` calls). Plans are byte-identical to the pull suite — only the
// final consumption differs — so the delta isolates the external-iteration
// resumption cost from the adapter-chain cost.

use crate::data::Data;
use crate::engine::*;
use crate::pull::*;
use crate::queries_pull::helpers::min_row_pull_next;
use crate::tpch_data::TpchData;

// Local copies of tpch/common.rs's 1-line formatters (that module is
// private to the tpch tree).
fn tf(x: f64) -> String { format!("{x:.2}") }
fn join_lines(rows: impl IntoIterator<Item = String>) -> String {
    rows.into_iter().collect::<Vec<_>>().join("\n")
}

// ===== JOB next-variants (sink: min_row_pull_next) ======================

// q1a — plan copied verbatim from queries_pull/t1.rs.
fn q1a_next(d: &Data) -> String {
    let q = d.movie
        .in_s((&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("top 250 rank")))
            .o(
                (&d.movie_company).in_s(
                    (&d.company_type).o(&d.companytype_kind).eq("production companies")
                        .and(
                            (&d.company_note).nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")
                                .and(
                                    (&d.company_note).rx(r"\(co-production\)")
                                        .or((&d.company_note).rx(r"\(presents\)"))
                                )
                        )
                ).o(&d.company_note)
                .x(&d.movie_title)
                .x(&d.movie_production_year)
            );
    min_row_pull_next(q)
}

// q2a — plan copied verbatim from queries_pull/t1.rs (q2 with "[de]").
fn q2a_next(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title")
            .and((&d.movie_company).o((&d.company_country).eq("[de]")))
    ).o(&d.movie_title);
    min_row_pull_next(q)
}

// q29a — deep t6 chain, plan copied verbatim from queries_pull/t6.rs.
fn q29a_next(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast")
                .and((&d.completecast_status).o(&d.compcasttype_kind).eq("complete+verified"))
        )
            .and((&d.movie_company).o((&d.company_country).eq("[us]")))
            .and((&d.movie_info).in_s(
                (&d.info_type).o(&d.infotype_info).eq("release dates")
                    .and(
                        (&d.info_info).rx(r"^Japan:.*200")
                            .or((&d.info_info).rx(r"^USA:.*200"))
                    )
            ))
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("computer-animation"))
            .and((&d.movie_title).eq("Shrek 2"))
            .and((&d.movie_production_year).ge(2000))
            .and((&d.movie_production_year).le(2010))
    ).o(
        (&d.movie_cast).in_s(
            (&d.cast_note).in_v(crate::queries::sets::voice3())
                .and((&d.cast_role).o(&d.roletype_role).eq("actress"))
                .and((&d.cast_character).o((&d.character_name).eq("Queen")))
                .and((&d.cast_person).in_s(
                    (&d.person_gender).eq("f")
                        .and((&d.person_name).rx(r"An"))
                        .and(&d.person_aka)
                        .and((&d.person_info).in_s((&d.personinfo_type).o(&d.infotype_info).eq("trivia")))
                ))
        ).o(
            (&d.cast_character).o(&d.character_name)
                .x((&d.cast_person).o(&d.person_name))
        )
        .x(&d.movie_title)
    );
    min_row_pull_next(q)
}

// ===== TPC-H next-variants (sinks: *_next builders + for-loop drains) ===

// Q1 — plan copied verbatim from tpch/idiomatic_pull.rs.
fn tq1_next(d: &TpchData) -> String {
    let live = d.lineitem.in_s((&d.li_shipdate).le(19980902));
    let scan = live.o(
        (&d.li_quantity).x(&d.li_extendedprice).x(&d.li_discount).x(&d.li_tax)
    );
    let group_key = (&d.li_returnflag).x(&d.li_status);
    let grouped = Fold::build_pull_next(scan.group_by(group_key),
        (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0_i64),
        |(qty, ext, di, dp, chg, n), (((q, e), dc), tx)| {
            let dp_inc = e * (1.0 - dc);
            let chg_inc = dp_inc * (1.0 + tx);
            (qty + q, ext + e, di + dc, dp + dp_inc, chg + chg_inc, n + 1)
        });
    let mut rows: Vec<((&str, &str), (f64, f64, f64, f64, f64, i64))> = Vec::new();
    for kv in grouped.iter() {
        rows.push(kv);
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    join_lines(rows.iter().map(|(k, (qty, ext, di, dp, chg, n))| {
        let nf = *n as f64;
        format!("{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                k.0, k.1, tf(*qty), tf(*ext), tf(*dp), tf(*chg),
                tf(qty / nf), tf(ext / nf), tf(di / nf), n)
    }))
}

// Q9 — plan copied verbatim from tpch/idiomatic_pull.rs.
fn tq9_next(d: &TpchData) -> String {
    let sc: HashIdx<_, _> =
        collect_hash_idx_next((&d.ps_part).x(&d.ps_supplier).inv().o(&d.ps_supplycost));
    let live = (&d.lineitem)
        .in_s((&d.li_part).o(&d.pa_name).filt(|n: &str| n.contains("green")));
    let sname = (&live).o((&d.li_supplier).o((&d.su_nation).o(&d.na_name)));
    let year  = (&live).o((&d.li_order).o(&d.ord_date)).map(|d: i64| d / 10000);
    let groups = sname.x(year);
    let cost_per_li = (&d.li_part).x(&d.li_supplier).o(&sc);
    let scan = (&live).o(
        (&d.li_extendedprice).x(&d.li_discount).x(&d.li_quantity).x(cost_per_li)
    );
    let result = Fold::build_pull_next(scan.group_by(groups), 0.0_f64,
        |a, (((e, dc), q), cost)| a + e * (1.0 - dc) - cost * q);
    let mut rows: Vec<((&str, i64), f64)> = Vec::new();
    for kv in result.iter() {
        rows.push(kv);
    }
    rows.sort_by(|a, b| a.0.0.cmp(b.0.0).then_with(|| b.0.1.cmp(&a.0.1)));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}|{}", k.0, k.1, tf(*v))))
}

// Q21 — plan copied verbatim from tpch/idiomatic_pull.rs.
fn tq21_next(d: &TpchData) -> String {
    // count_distinct's whole-group closure, spelled out so the buffered
    // build goes through the raw-next() builder.
    let n_distinct = |mut vs: smallvec::SmallVec<[usize; 4]>| {
        vs.sort_unstable(); vs.dedup(); vs.len() as i64
    };
    let late = d.lineitem.in_s(
        (&d.li_commitdate).x(&d.li_receiptdate).filt(|(c, r)| c < r)
    );
    let multi_supp = Fold::build_buf_pull_next(
        (&d.li_supplier).group_by(&d.li_order), n_distinct).gt(1);
    let only_late = Fold::build_buf_pull_next(
        (&late).o(&d.li_supplier).group_by((&late).o(&d.li_order)), n_distinct).eq(1);
    let saudi = (&d.supplier).and(
        (&d.su_nation).o(&d.na_name).eq("SAUDI ARABIA")
    );
    let f_ords = (&d.orders).and((&d.ord_status).eq("F"));
    let qualifying = (&late)
        .in_s((&d.li_supplier).in_s(saudi))
        .in_s((&d.li_order).in_s(f_ords.and(multi_supp).and(only_late)));
    let counts = Fold::build_pull_next(
        qualifying.group_by(&d.li_supplier), 0_i64, |a, _| a + 1);
    let mut rows: Vec<(usize, i64)> = Vec::new();
    for kv in counts.iter() {
        rows.push(kv);
    }
    let mut named: Vec<(&str, i64)> = rows.iter()
        .map(|(s, c)| (d.su_name.values[*s], *c)).collect();
    named.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    named.truncate(100);
    join_lines(named.iter().map(|(n, c)| format!("{}|{}", n, c)))
}

// ===== registries ========================================================
// Each subset query appears TWICE: once with the pull suite's fold-style
// runner (looked up from the existing registry, so it is the exact same
// monomorphized function the `job-pull`/`tpch-pull` suites time) and once
// with the raw-next() variant above — both inside the same 6-query process,
// so cache context is identical and the fold-vs-next delta is clean.

fn job_entry(name: &str) -> crate::Entry<Data> {
    *crate::queries_pull::all_queries().iter()
        .find(|e| e.0 == name).expect("unknown JOB query")
}

fn tpch_entry(name: &str) -> crate::Entry<TpchData> {
    *crate::tpch::idiomatic_pull::queries().iter()
        .find(|e| e.0 == name).expect("unknown TPC-H query")
}

pub fn job_entries() -> Vec<crate::Entry<Data>> {
    let fold = |n| job_entry(n);
    let (e1, e2, e29) = (fold("1a"), fold("2a"), fold("29a"));
    vec![
        ("1a/fold",  e1.1,  e1.2),
        ("1a/next",  e1.1,  q1a_next as fn(&Data) -> String),
        ("2a/fold",  e2.1,  e2.2),
        ("2a/next",  e2.1,  q2a_next),
        ("29a/fold", e29.1, e29.2),
        ("29a/next", e29.1, q29a_next),
    ]
}

pub fn tpch_entries() -> Vec<crate::Entry<TpchData>> {
    let fold = |n| tpch_entry(n);
    let (e1, e9, e21) = (fold("1"), fold("9"), fold("21"));
    vec![
        ("1/fold",  e1.1,  e1.2),
        ("1/next",  e1.1,  tq1_next as fn(&TpchData) -> String),
        ("9/fold",  e9.1,  e9.2),
        ("9/next",  e9.1,  tq9_next),
        ("21/fold", e21.1, e21.2),
        ("21/next", e21.1, tq21_next),
    ]
}
