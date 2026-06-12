// The optimized variant — same algebra as the baseline, but hand-encoding
// the plans a stats-driven optimizer (DuckDB's) would pick: dense folds,
// bitset semi-joins, selectivity-ordered conjunctions, SIMD substring
// search. Prela has no optimizer, so the rewrites live in the query text.
//
// Only the rewritten queries are defined here; everything else comes from
// the base registry in common.rs.

#![allow(clippy::too_many_lines)]

use std::collections::HashMap;

use super::common::{f, join_lines, with_overrides};
use crate::engine::*;
use crate::tpch_data::{TpchData, fmt_yyyymmdd};

pub fn queries() -> Vec<super::Entry> {
    with_overrides(&[
        ("1", q1), ("2", q2), ("4", q4), ("9", q9), ("12", q12),
        ("13", q13), ("17", q17), ("18", q18), ("21", q21), ("22", q22),
    ])
}

fn q1(d: &TpchData) -> String {
    // Julia: ((returnflag ⊗ Li.status) ← (lineitem → shipdate <= "..." : qty ⊗ ext ⊗ disc ⊗ tax))
    //        ▷ (cmb, ...) ↦ out
    let live = d.lineitem.in_s((&d.li_shipdate).le(19980902));
    let scan = live.o(
        (&d.li_quantity).x(&d.li_extendedprice).x(&d.li_discount).x(&d.li_tax)
    );
    // Pack (returnflag, status) single-byte values into a small usize index
    // so `dense_fold` can use a `[Acc; 288]`-equivalent dense cache. The
    // packed order `(rf-'A') << 4 | (ls-'F')` preserves the (rf, ls)
    // ascii-pair sort order under integer comparison: rf ∈ {A=0, N=13,
    // R=17}, ls ∈ {F=0, O=9} → max key 281, so ≥282 slots; 288 used.
    let group_key = (&d.li_returnflag).x(&d.li_status)
        .map(|(rf, ls): (&str, &str)| {
            ((rf.as_bytes()[0].wrapping_sub(b'A') as usize) << 4)
                | (ls.as_bytes()[0].wrapping_sub(b'F') as usize)
        });
    let grouped = group_key.lc(scan)
        .dense_fold(288, (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0_i64),
              |(qty, ext, di, dp, chg, n), (((q, e), dc), tx)| {
                  let dp_inc = e * (1.0 - dc);
                  let chg_inc = dp_inc * (1.0 + tx);
                  (qty + q, ext + e, di + dc, dp + dp_inc, chg + chg_inc, n + 1)
              });
    let mut rows: Vec<(usize, (f64, f64, f64, f64, f64, i64))> = Vec::new();
    grouped.drive(|k, v| rows.push((k, v)));
    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, (qty, ext, di, dp, chg, n))| {
        let rf = (((*k >> 4) as u8).wrapping_add(b'A')) as char;
        let ls = (((*k & 0xF) as u8).wrapping_add(b'F')) as char;
        let nf = *n as f64;
        format!("{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                rf, ls, f(*qty), f(*ext), f(*dp), f(*chg),
                f(qty / nf), f(ext / nf), f(di / nf), n)
    }))
}

fn q4(d: &TpchData) -> String {
    // Julia: let live = (lineitem ∧ (commitdate < receiptdate) → Li.order) ⩘
    //                  (orders ∧ (date in during("1993-07-01", "1993-10-01")))
    //        (live → Ord.priority)' ▷ ((a, _) -> a + 1, 0)
    // Dense `Bitset` of orderkeys with a late lineitem replaces the lconj
    // path that lazy-built a HashSet from ~14M late-lineitem orderkeys.
    let bad_li_order = d.lineitem
        .in_s((&d.li_commitdate).x(&d.li_receiptdate).filt(|(c, r)| c < r))
        .o(&d.li_order);
    let is_late = Bitset::from_drive(d.orders.n, &bad_li_order);
    let live = d.orders
        .in_s((&d.ord_date).during(19930701, 19931001))
        .in_s(is_late);
    let counts = live.o(&d.ord_priority).inv().fold(0_i64, |a, _| a + 1);
    let mut rows: Vec<(&str, i64)> = Vec::new();
    counts.drive(|k, v| rows.push((k, v)));
    rows.sort_by(|a, b| a.0.cmp(b.0));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// ---------- Q9 — product type profit measure ----------

fn q9(d: &TpchData) -> String {
    // CP1.3 / CP1.4: group on (nation_id, year) as (usize, i64) — 16-byte
    // integer hash key, not (&str, i64) which costs a string hash + memcmp
    // per collision. Nation name is FD'd by nation_id, looked up at output.
    let sc = (&d.ps_part).x(&d.ps_supplier).inv().o(&d.ps_supplycost).mat_idx();
    // Hoist the `Part.name ~ "green"` predicate out of the 60M-row
    // lineitem scan by materializing the matching part-ids into a `Bitset`
    // (~200K Part rows scanned once). Per lineitem becomes one bit-test.
    let green_parts = Bitset::from_drive(
        d.part.n,
        &(&d.part).in_s((&d.pa_name).filt(|n: &str| n.contains("green"))),
    );
    let live = (&d.lineitem).in_s((&d.li_part).in_s(&green_parts));
    let nation_id = (&live).o((&d.li_supplier).o(&d.su_nation));
    let year      = (&live).o((&d.li_order).o(&d.ord_date)).map(|d: i64| d / 10000);
    let groups = nation_id.x(year);
    let cost_per_li = (&d.li_part).x(&d.li_supplier).o(&sc);
    let scan = (&live).o(
        (&d.li_extendedprice).x(&d.li_discount).x(&d.li_quantity).x(cost_per_li)
    );
    let result = groups.lc(scan).fold(0.0_f64, |a, (((e, dc), q), cost)| {
        a + e * (1.0 - dc) - cost * q
    });
    let mut rows: Vec<((usize, i64), f64)> = Vec::new();
    result.drive(|k, v| rows.push((k, v)));
    rows.sort_by(|a, b| {
        let na = d.na_name.values[a.0.0];
        let nb = d.na_name.values[b.0.0];
        na.cmp(nb).then_with(|| b.0.1.cmp(&a.0.1))
    });
    join_lines(rows.iter().map(|(k, v)| {
        format!("{}|{}|{}", d.na_name.values[k.0], k.1, f(*v))
    }))
}

fn q12(d: &TpchData) -> String {
    // Conjuncts reordered by oracle-known selectivity (most selective first)
    // so each restriction shaves rows off every downstream predicate.
    // The algebra preserves whatever order the user wrote — Prela has no
    // stats-driven optimizer; here we hand-encode the order DuckDB's planner
    // would pick, to show the algebra *can* express the optimal plan.
    //   receiptdate ∈ [1994,1995): ~14%  (most selective)
    //   shipmode IN (MAIL, SHIP):  ~29%
    //   shipdate < commitdate:     ~49%
    //   commit  < receipt:         ~62%  (barely filters; runs last)
    let live = (&d.lineitem)
        .in_s((&d.li_receiptdate).during(19940101, 19950101))
        .in_s((&d.li_shipmode).in_v(vec!["MAIL", "SHIP"]))
        .in_s((&d.li_shipdate).x(&d.li_commitdate).filt(|(s, c)| s < c))
        .in_s((&d.li_commitdate).x(&d.li_receiptdate).filt(|(c, r)| c < r));
    let scan = (&live).o(&d.li_shipmode);
    let prio = (&live).o((&d.li_order).o(&d.ord_priority));
    let result = scan.lc(prio).fold((0_i64, 0_i64), |(h, l), pr| {
        let is_high = pr == "1-URGENT" || pr == "2-HIGH";
        if is_high { (h + 1, l) } else { (h, l + 1) }
    });
    let mut rows: Vec<(&str, (i64, i64))> = Vec::new();
    result.drive(|k, v| rows.push((k, v)));
    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, (h, l))| format!("{}|{}|{}", k, h, l)))
}

// ---------- Q13 — customer distribution (LEFT JOIN) ----------

fn q13(d: &TpchData) -> String {
    // Julia algebraic part: count_per_cust = ((live_orders → Ord.customer) ←
    //                                          (live_orders → date)) ▷ ((a, _) -> a + 1, 0)
    // Then a Julia escape for the LEFT-JOIN zero-default (customer.n - n_with).
    // `nrx("special.*requests")` is regex DFA per row (~100ns × 1.5M).
    // memmem::Finder for "special" is SIMD-backed (~5-10ns); then probe
    // the suffix for "requests" with stdlib `.contains` (also SIMD).
    use memchr::memmem;
    let f_special = memmem::Finder::new("special");
    let live_orders = (&d.orders)
        .in_s((&d.ord_customer).ne(NO_ID))   // skip sparse orderkey gaps (hole fill NO_ID)
        .in_s((&d.ord_comment).filt(move |c: &str| {
            match f_special.find(c.as_bytes()) {
                Some(p) => !c[p + "special".len()..].contains("requests"),
                None => true,
            }
        }));
    let count_per_cust = (&live_orders).o(&d.ord_customer)
        .lc((&live_orders).o(&d.ord_date))
        .fold(0_i64, |a, _| a + 1);
    let mut dist: HashMap<i64, i64> = HashMap::new();
    let mut n_with = 0i64;
    count_per_cust.drive(|_, c| { *dist.entry(c).or_insert(0) += 1; n_with += 1; });
    // LEFT JOIN zero-default: customers with no qualifying orders contribute to c_count=0.
    dist.insert(0, d.customer.n as i64 - n_with);
    let mut rows: Vec<_> = dist.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| b.0.cmp(a.0)));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// ---------- Q17 — small-quantity order revenue ----------

pub(super) fn q17(d: &TpchData) -> String {
    // Semi-join reduction (Boncz CP5.2 / Dreseler §4.8): the threshold
    // avg(quantity) only matters for the ~200 parts that pass the
    // brand+container filter, not all ~200K parts. Identify the qualifying
    // parts up front, restrict the lineitem scan that feeds the avg fold
    // to those parts, then the avg is computed over a tiny slice.
    let qual_part_set = (&d.part)
        .in_s((&d.pa_brand).eq("Brand#23"))
        .in_s((&d.pa_container).eq("MED BOX"))
        .mat_set();
    let live_li = (&d.li_part).in_s(&qual_part_set);
    let threshold_per_part = (&live_li).lc(&d.li_quantity)
        .fold((0.0_f64, 0_i64), |(s, n), q| (s + q, n + 1))
        .map(|(s, n)| 0.2 * s / n as f64);
    let tpp = threshold_per_part.mat_idx();
    let live = d.lineitem
        .in_s(&live_li)
        .in_s((&d.li_quantity).x((&d.li_part).o(&tpp))
             .filt(|(q, t)| q < t));
    let sum = live.o(&d.li_extendedprice)
        .unwrap_fold(0.0_f64, |a, e| a + e);
    f(sum / 7.0)
}

// ---------- Q18 — large volume customer ----------

fn q18(d: &TpchData) -> String {
    let sum_qty = (&d.li_order).lc(&d.li_quantity).dense_fold(d.orders.n, 0.0_f64, |a, q| a + q);
    let big = sum_qty.gt(300.0);
    let mut rows: Vec<(usize, f64)> = Vec::new();
    big.drive(|k, v| rows.push((k, v)));
    rows.sort_by(|a, b| {
        let (oa, ob) = (a.0, b.0);
        d.ord_totalprice.values[ob].partial_cmp(&d.ord_totalprice.values[oa]).unwrap()
            .then_with(|| d.ord_date.values[oa].cmp(&d.ord_date.values[ob]))
    });
    rows.truncate(100);
    join_lines(rows.iter().map(|(o, sum_q)| {
        let oi = *o;
        let cu = d.ord_customer.values[oi];
        let cui = cu;
        // natural custkey / orderkey = internal id + 1
        format!("{}|{}|{}|{}|{}|{}",
                d.cu_name.values[cui], cu + 1, o + 1,
                fmt_yyyymmdd(d.ord_date.values[oi]), f(d.ord_totalprice.values[oi]), f(*sum_q))
    }))
}

// ---------- Q21 — suppliers who kept orders waiting ----------

fn q21(d: &TpchData) -> String {
    // Julia:
    //   late = lineitem ∧ (receiptdate > commitdate)
    //   n_distinct = vs -> length(unique(vs))
    //   multi_supp = askeys((order ← Li.supplier) ▷ n_distinct > 1)
    //   only_late  = askeys(((late : order) ← (late : Li.supplier)) ▷ n_distinct == 1)
    //   qualifying = late ∧ (Li.supplier → saudi) ∧ (order → f_ords ∧ multi_supp ∧ only_late)
    //   (Li.supplier ← qualifying) ▷ ((a, _) -> a + 1, 0) ⊗ Su.name
    let late = d.lineitem.in_s(
        (&d.li_commitdate).x(&d.li_receiptdate).filt(|(c, r)| c < r)
    );
    // `multi_supp`/`only_late` are SET-membership predicates over orderkeys
    // ("more than one distinct supplier" / "exactly one distinct supplier").
    // The Julia `count_distinct() > 1` form materializes a `SVec` of suppliers
    // per group then sort-dedups — needless when we only care whether the
    // count is 0/1/>1. Replace with a constant-state fold tracking
    // `(first_supplier, saw_a_second)` per orderkey; `multi` ⇔ `saw_second`
    // and `only_one` ⇔ `first.is_some() && !saw_second`. "No supplier seen
    // yet" is `None` — supplier ids are 0-based, so 0 is a live id.
    let track = |(first, multi): (Option<usize>, bool), s: usize| match first {
        None => (Some(s), multi),
        Some(f) if f != s => (first, true),
        _ => (first, multi),
    };
    let supp_state = (&d.li_order).lc(&d.li_supplier)
        .dense_fold(d.orders.n, (None, false), track);
    let multi_supp = (&d.orders).in_s(supp_state.filt(|(_, m): (Option<usize>, bool)| m));
    let late_supp_state = (&late).o(&d.li_order)
        .lc((&late).o(&d.li_supplier))
        .dense_fold(d.orders.n, (None, false), track);
    let only_late = (&d.orders).in_s(late_supp_state
        .filt(|(first, multi): (Option<usize>, bool)| first.is_some() && !multi));
    let saudi = (&d.supplier).in_s(
        (&d.su_nation).o(&d.na_name).eq("SAUDI ARABIA")
    );
    let f_ords = (&d.orders).in_s((&d.ord_status).eq("F"));
    // Hoist each per-row membership probe into a dense `Bitset` over its
    // domain — collapses the 5-deep restriction chain on `qualifying` to ~5 ALU ops.
    let saudi_bs      = Bitset::from_drive(d.supplier.n, &saudi);
    let f_ords_bs     = Bitset::from_drive(d.orders.n,   &f_ords);
    let multi_supp_bs = Bitset::from_drive(d.orders.n,   &multi_supp);
    let only_late_bs  = Bitset::from_drive(d.orders.n,   &only_late);
    let qualifying = (&late)
        .in_s((&d.li_supplier).in_s(&saudi_bs))
        .in_s((&d.li_order).in_s(&f_ords_bs))
        .in_s((&d.li_order).in_s(&multi_supp_bs))
        .in_s((&d.li_order).in_s(&only_late_bs));
    let counts = (&d.li_supplier).lc(qualifying).fold(0_i64, |a, _| a + 1);
    let mut rows: Vec<(usize, i64)> = Vec::new();
    counts.drive(|k, v| rows.push((k, v)));
    let mut named: Vec<(&str, i64)> = rows.iter()
        .map(|(s, c)| (d.su_name.values[*s], *c)).collect();
    named.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    named.truncate(100);
    join_lines(named.iter().map(|(n, c)| format!("{}|{}", n, c)))
}

// ---------- Q22 — global sales opportunity ----------

pub(super) fn q22(d: &TpchData) -> String {
    // Julia:
    //   prefix = Cu.phone ↦ (s -> s[1:2])
    //   prefix_ok = customer ∧ (prefix in codes)
    //   avg = unwrap((prefix_ok ∧ (acctbal > 0) → acctbal) ⊵ ... ↦ s/n)
    //   target = (prefix_ok ∧ (acctbal > avg)) - !((orders → Ord.customer)')
    //   (prefix ← (target : acctbal)) ▷ ((cnt, sm), ab) -> (cnt+1, sm+ab)
    let prefix = (&d.cu_phone).map(|p: &str| &p[..2]);
    let codes = vec!["13","31","23","29","30","18","17"];
    let prefix_ok = (&d.customer).in_s((&prefix).in_v(codes));
    let pos = (&prefix_ok).in_s((&d.cu_acctbal).gt(0.0));
    let (sum_p, cnt_p) = pos.o(&d.cu_acctbal)
        .unwrap_fold((0.0_f64, 0_i64), |(s, n), v| (s + v, n + 1));
    let avg = sum_p / cnt_p as f64;
    // Packed bitset over the dense customer universe — replaces the
    // baseline's `mat_set` (a HashSet built from every order's customer)
    // with one bit per customer.
    let custs_with_orders = Bitset::from_drive(d.customer.n, &d.ord_customer);
    let target = (&prefix_ok).in_s((&d.cu_acctbal).gt(avg))
        .minus(custs_with_orders);
    let counts = (&prefix).lc(target)
        .fold((0_i64, 0.0_f64), |(cnt, sm), c| {
            let ab = d.cu_acctbal.values[c];
            (cnt + 1, sm + ab)
        });
    let mut rows: Vec<(&str, (i64, f64))> = Vec::new();
    counts.drive(|k, v| rows.push((k, v)));
    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, (cnt, sm))| format!("{}|{}|{}", k, cnt, f(*sm))))
}

// ---------- Q2 — minimum-cost supplier per part ----------

fn q2(d: &TpchData) -> String {
    // Julia:
    //   eu_ps = partsupp ∧ (PS.supplier → Su.nation → Na.region → Re.name == "EUROPE")
    //   min_per_part = !(((eu_ps → PS.part) ← (eu_ps → supplycost)) ▷ (min, Inf))
    //   target = eu_ps ∧ (PS.part → (size == 15 ∧ type ~ "BRASS$"))
    //                  ∧ (supplycost == (PS.part → min_per_part))
    //   target : (Su.acctbal ⊗ Su.name ⊗ Na.name ⊗ PS.part ⊗ Pa.mfgr
    //             ⊗ Su.address ⊗ Su.phone ⊗ Su.comment)
    let eu_ps = (&d.partsupp).in_s(
        (&d.ps_supplier).o((&d.su_nation).o((&d.na_region).o(&d.re_name))).eq("EUROPE")
    );
    let min_per_part = (&eu_ps).o(&d.ps_part)
        .lc((&eu_ps).o(&d.ps_supplycost))
        .dense_fold(d.part.n, f64::INFINITY, |a, c| if c < a { c } else { a });
    let target = (&eu_ps)
        .in_s((&d.ps_part).o(&d.pa_size).eq(15))
        .in_s((&d.ps_part).o(&d.pa_type).filt(|s: &str| s.ends_with("BRASS")))
        .in_s((&d.ps_supplycost).x((&d.ps_part).o(&min_per_part))
             .filt(|(c, m)| c == m));
    // Project per PS row → (acct, sname, nname, pkey, mfgr, addr, phone, comm)
    let mut rows: Vec<(f64, &str, &str, usize, &str, &str, &str, &str)> = Vec::new();
    target.drive(|psi, _| {
        let pa = d.ps_part.values[psi];
        let su = d.ps_supplier.values[psi];
        rows.push((
            d.su_acctbal.values[su],
            d.su_name.values[su],
            d.na_name.values[d.su_nation.values[su]],
            pa,
            d.pa_mfgr.values[pa],
            d.su_address.values[su],
            d.su_phone.values[su],
            d.su_comment.values[su],
        ));
    });
    rows.sort_by(|a, b| {
        b.0.partial_cmp(&a.0).unwrap()
            .then(a.2.cmp(b.2))
            .then(a.1.cmp(b.1))
            .then(a.3.cmp(&b.3))
    });
    rows.truncate(100);
    // natural partkey = internal id + 1
    join_lines(rows.iter().map(|r| format!("{}|{}|{}|{}|{}|{}|{}|{}",
        f(r.0), r.1, r.2, r.3 + 1, r.4, r.5, r.6, r.7)))
}
