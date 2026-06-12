// PULL-PROTOCOL PORT of the idiomatic TPC-H queries (common.rs) — IDENTICAL
// plans, pull-side consumption spellings only:
//   .fold(           → .fold_p(
//   .count_distinct( → .count_distinct_p(
//   .collect()       → .collect_p()
//   .unwrap_fold(i, op) → .iter().fold(i, |a, (_, v)| op(a, v))
//   q.drive(|k, v| rows.push((k, v))) → rows.extend(q.iter())
//   other drive sinks → .iter().for_each(...)
// Oracles come from the base registry via `with_overrides`, so the pass
// criterion is byte-equality with the same strings the push suite checks.
// Suite: `prela tpch-pull` (QS is ignored — idiomatic only).

#![allow(clippy::too_many_lines)]

use std::collections::HashMap;

use crate::engine::*;
use crate::pull::*;
use crate::tpch_data::{TpchData, fmt_yyyymmdd};

use super::common::{self, f, join_lines};

// ---------- Q1 — pricing summary report ----------

fn q1(d: &TpchData) -> String {
    let live = d.lineitem.in_s((&d.li_shipdate).le(19980902));
    let scan = live.o(
        (&d.li_quantity).x(&d.li_extendedprice).x(&d.li_discount).x(&d.li_tax)
    );
    let group_key = (&d.li_returnflag).x(&d.li_status);
    let grouped = scan.group_by(group_key)
        .fold_p((0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0_i64),
              |(qty, ext, di, dp, chg, n), (((q, e), dc), tx)| {
                  let dp_inc = e * (1.0 - dc);
                  let chg_inc = dp_inc * (1.0 + tx);
                  (qty + q, ext + e, di + dc, dp + dp_inc, chg + chg_inc, n + 1)
              });
    let mut rows: Vec<((&str, &str), (f64, f64, f64, f64, f64, i64))> = Vec::new();
    rows.extend(grouped.iter());
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    join_lines(rows.iter().map(|(k, (qty, ext, di, dp, chg, n))| {
        let nf = *n as f64;
        format!("{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                k.0, k.1, f(*qty), f(*ext), f(*dp), f(*chg),
                f(qty / nf), f(ext / nf), f(di / nf), n)
    }))
}

// ---------- Q6 — forecasting revenue change (scalar) ----------

fn q6(d: &TpchData) -> String {
    let live = d.lineitem
        .in_s((&d.li_shipdate).during(19940101, 19950101))
        .in_s((&d.li_discount).between(0.05, 0.07))
        .in_s((&d.li_quantity).lt(24.0));
    let sum = live.o((&d.li_extendedprice).x(&d.li_discount))
        .iter().fold(0.0, |acc, (_, (e, dc))| acc + e * dc);
    f(sum)
}

// ---------- Q14 — promo revenue ratio ----------

fn q14(d: &TpchData) -> String {
    let live = d.lineitem.in_s((&d.li_shipdate).during(19950901, 19951001));
    let scan = live.o(
        (&d.li_extendedprice).x(&d.li_discount).x((&d.li_part).o(&d.pa_type))
    );
    let (promo, total) = scan.iter().fold((0.0, 0.0), |(p, t), (_, ((e, dc), ty))| {
        let dp = e * (1.0 - dc);
        (p + if ty.starts_with("PROMO") { dp } else { 0.0 }, t + dp)
    });
    f(100.0 * promo / total)
}

// ---------- Q3 — shipping priority ----------

fn q3(d: &TpchData) -> String {
    let item = (&d.lineitem)
        .in_s((&d.li_shipdate).gt(19950315))
        .in_s((&d.li_order).o(&d.ord_date).lt(19950315))
        .in_s((&d.li_order).o((&d.ord_customer).o(&d.cu_mktsegment)).eq("BUILDING"))
        .o((&d.li_extendedprice).x(&d.li_discount));
    let revenue = item.group_by(&d.li_order)
        .fold_p(0.0_f64, |a, (e, dc)| a + e * (1.0 - dc));
    let mut rows: Vec<(usize, f64)> = Vec::new();
    rows.extend(revenue.iter());
    rows.sort_by(|a, b| {
        let (oa, ob) = (a.0, b.0);
        b.1.partial_cmp(&a.1).unwrap()
            .then_with(|| d.ord_date.values[oa].cmp(&d.ord_date.values[ob]))
    });
    rows.truncate(10);
    join_lines(rows.iter().map(|(o, r)| {
        let oi = *o;
        // natural orderkey = internal id + 1 (formatting edge only)
        format!("{}|{}|{}|{}", o + 1, f(*r), fmt_yyyymmdd(d.ord_date.values[oi]), d.ord_shippriority.values[oi])
    }))
}

// ---------- Q4 — order priority checking ----------

fn q4(d: &TpchData) -> String {
    let bad_li_order = d.lineitem
        .in_s((&d.li_commitdate).x(&d.li_receiptdate).filt(|(c, r)| c < r))
        .o(&d.li_order);
    let live_orders = d.orders.in_s(
        (&d.ord_date).during(19930701, 19931001)
    );
    let live = live_orders.in_s(bad_li_order.collect_p::<MatSet<_>>());
    let counts = live.o(&d.ord_priority).inv().fold_p(0_i64, |a, _| a + 1);
    let mut rows: Vec<(&str, i64)> = Vec::new();
    rows.extend(counts.iter());
    rows.sort_by(|a, b| a.0.cmp(b.0));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// ---------- Q5 — local supplier volume ----------

fn q5(d: &TpchData) -> String {
    let c_nation = (&d.li_order).o(&d.ord_customer).o(&d.cu_nation);
    let s_nation = (&d.li_supplier).o(&d.su_nation);
    let live = (&d.lineitem)
        .in_s((&d.li_order).o(&d.ord_date).during(19940101, 19950101))
        .in_s((&s_nation).o((&d.na_region).o(&d.re_name)).eq("ASIA"))
        .in_s((&c_nation).x(&s_nation).filt(|(c, s)| c == s));
    let groups = (&live).o((&s_nation).o(&d.na_name));
    let scan = (&live).o((&d.li_extendedprice).x(&d.li_discount));
    let result = scan.group_by(groups).fold_p(0.0_f64, |a, (e, dc)| a + e * (1.0 - dc));
    let mut rows: Vec<(&str, f64)> = Vec::new();
    rows.extend(result.iter());
    rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, f(*v))))
}

// ---------- Q7 — volume shipping between nation pairs ----------

fn q7(d: &TpchData) -> String {
    let snat = (&d.li_supplier).o((&d.su_nation).o(&d.na_name));
    let cnat = (&d.li_order).o((&d.ord_customer).o((&d.cu_nation).o(&d.na_name)));
    let live = (&d.lineitem)
        .in_s((&d.li_shipdate).between(19950101, 19961231))
        .in_s((&snat).x(&cnat).filt(|(s, c)| {
            (s == "FRANCE" && c == "GERMANY") || (s == "GERMANY" && c == "FRANCE")
        }));
    let sname = (&live).o(&snat);
    let cname = (&live).o(&cnat);
    let year = (&live).o(&d.li_shipdate).map(|d: i64| d / 10000);
    let groups = sname.x(cname).x(year);
    let scan = (&live).o((&d.li_extendedprice).x(&d.li_discount));
    let result = scan.group_by(groups).fold_p(0.0_f64, |a, (e, dc)| a + e * (1.0 - dc));
    let mut rows: Vec<(((&str, &str), i64), f64)> = Vec::new();
    rows.extend(result.iter());
    rows.sort_by(|a, b| (a.0).0.0.cmp(&(b.0).0.0)
        .then((a.0).0.1.cmp(&(b.0).0.1))
        .then((a.0).1.cmp(&(b.0).1)));
    join_lines(rows.iter().map(|(k, v)| {
        format!("{}|{}|{}|{}", k.0.0, k.0.1, k.1, f(*v))
    }))
}

// ---------- Q8 — market share for BRAZIL ----------

fn q8(d: &TpchData) -> String {
    let live = (&d.lineitem)
        .in_s((&d.li_part).o(&d.pa_type).eq("ECONOMY ANODIZED STEEL"))
        .in_s((&d.li_order).o((&d.ord_customer).o((&d.cu_nation).o((&d.na_region).o(&d.re_name))))
             .eq("AMERICA"))
        .in_s((&d.li_order).o(&d.ord_date).between(19950101, 19961231));
    let year = (&live).o((&d.li_order).o(&d.ord_date)).map(|d: i64| d / 10000);
    let snat_name = (&live).o((&d.li_supplier).o((&d.su_nation).o(&d.na_name)));
    let scan = (&live).o((&d.li_extendedprice).x(&d.li_discount)).x(snat_name);
    let pair_fold = scan.group_by(year).fold_p((0.0_f64, 0.0_f64), |(b, t), ((e, dc), nm)| {
        let v = e * (1.0 - dc);
        (b + if nm == "BRAZIL" { v } else { 0.0 }, t + v)
    });
    let ratio = pair_fold.map(|(b, t)| b / t);
    let mut rows: Vec<(i64, f64)> = Vec::new();
    rows.extend(ratio.iter());
    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, f(*v))))
}

// ---------- Q9 — product type profit measure ----------

fn q9(d: &TpchData) -> String {
    // 2-key index: (part, supp) → supplycost via Prod-Inv → Compose → mat.
    let sc: HashIdx<_, _> = (&d.ps_part).x(&d.ps_supplier).inv().o(&d.ps_supplycost).collect_p();
    let live = (&d.lineitem)
        .in_s((&d.li_part).o(&d.pa_name).filt(|n: &str| n.contains("green")));
    let sname = (&live).o((&d.li_supplier).o((&d.su_nation).o(&d.na_name)));
    let year  = (&live).o((&d.li_order).o(&d.ord_date)).map(|d: i64| d / 10000);
    let groups = sname.x(year);
    let cost_per_li = (&d.li_part).x(&d.li_supplier).o(&sc);
    let scan = (&live).o(
        (&d.li_extendedprice).x(&d.li_discount).x(&d.li_quantity).x(cost_per_li)
    );
    let result = scan.group_by(groups).fold_p(0.0_f64, |a, (((e, dc), q), cost)| {
        a + e * (1.0 - dc) - cost * q
    });
    let mut rows: Vec<((&str, i64), f64)> = Vec::new();
    rows.extend(result.iter());
    rows.sort_by(|a, b| a.0.0.cmp(b.0.0).then_with(|| b.0.1.cmp(&a.0.1)));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}|{}", k.0, k.1, f(*v))))
}

// ---------- Q10 — returned-item reporting ----------

fn q10(d: &TpchData) -> String {
    let live = d.lineitem
        .in_s((&d.li_returnflag).eq("R"))
        .in_s((&d.li_order).o(&d.ord_date).during(19931001, 19940101));
    let revenue = live.o((&d.li_extendedprice).x(&d.li_discount))
        .group_by((&d.li_order).o(&d.ord_customer))
        .fold_p(0.0_f64, |a, (e, dc)| a + e * (1.0 - dc));
    let mut rows: Vec<(usize, f64)> = Vec::new();
    rows.extend(revenue.iter());
    rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    rows.truncate(20);
    join_lines(rows.iter().map(|(c, r)| {
        let ci = *c;
        // natural custkey = internal id + 1
        format!("{}|{}|{}|{}|{}|{}|{}|{}",
                c + 1, d.cu_name.values[ci], f(*r), f(d.cu_acctbal.values[ci]),
                d.na_name.values[d.cu_nation.values[ci]],
                d.cu_address.values[ci], d.cu_phone.values[ci], d.cu_comment.values[ci])
    }))
}

// ---------- Q11 — important stock ----------

fn q11(d: &TpchData) -> String {
    let live_ps = (&d.partsupp).in_s(
        (&d.ps_supplier).o((&d.su_nation).o(&d.na_name).eq("GERMANY"))
    );
    let value_per_part = (&live_ps).o((&d.ps_supplycost).x(&d.ps_availqty))
        .group_by((&live_ps).o(&d.ps_part))
        .fold_p(0.0, |a, (c, q)| a + c * (q as f64));
    // Scalar-subquery escape: drive the fold once into a total, derive threshold.
    let total = value_per_part.iter().fold(0.0, |a, (_, v)| a + v);
    let threshold = 0.0001 * total;
    let filtered = value_per_part.gt(threshold);
    let mut rows: Vec<(usize, f64)> = Vec::new();
    rows.extend(filtered.iter());
    rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    // natural partkey = internal id + 1
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k + 1, f(*v))))
}

// ---------- Q12 — shipping modes and order priority ----------

fn q12(d: &TpchData) -> String {
    let live = (&d.lineitem)
        .in_s((&d.li_shipmode).in_v(vec!["MAIL", "SHIP"]))
        .in_s((&d.li_commitdate).x(&d.li_receiptdate).filt(|(c, r)| c < r))
        .in_s((&d.li_shipdate).x(&d.li_commitdate).filt(|(s, c)| s < c))
        .in_s((&d.li_receiptdate).during(19940101, 19950101));
    let scan = (&live).o(&d.li_shipmode);
    let prio = (&live).o((&d.li_order).o(&d.ord_priority));
    let result = prio.group_by(scan).fold_p((0_i64, 0_i64), |(h, l), pr| {
        let is_high = pr == "1-URGENT" || pr == "2-HIGH";
        if is_high { (h + 1, l) } else { (h, l + 1) }
    });
    let mut rows: Vec<(&str, (i64, i64))> = Vec::new();
    rows.extend(result.iter());
    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, (h, l))| format!("{}|{}|{}", k, h, l)))
}

// ---------- Q13 — customer distribution (LEFT JOIN) ----------

fn q13(d: &TpchData) -> String {
    let live_orders = (&d.orders)
        .in_s((&d.ord_customer).ne(NO_ID))   // skip sparse orderkey gaps (hole fill NO_ID)
        .in_s((&d.ord_comment).nrx("special.*requests"));
    let count_per_cust = (&live_orders).o(&d.ord_date)
        .group_by((&live_orders).o(&d.ord_customer))
        .fold_p(0_i64, |a, _| a + 1);
    let mut dist: HashMap<i64, i64> = HashMap::new();
    let mut n_with = 0i64;
    count_per_cust.iter().for_each(|(_, c)| { *dist.entry(c).or_insert(0) += 1; n_with += 1; });
    // LEFT JOIN zero-default: customers with no qualifying orders contribute to c_count=0.
    dist.insert(0, d.customer.n as i64 - n_with);
    let mut rows: Vec<_> = dist.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| b.0.cmp(a.0)));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// ---------- Q15 — top supplier ----------

fn q15(d: &TpchData) -> String {
    let live = d.lineitem.in_s((&d.li_shipdate).during(19960101, 19960401));
    let revenue = (&live).o((&d.li_extendedprice).x(&d.li_discount))
        .group_by(&d.li_supplier)
        .fold_p(0.0_f64, |a, (e, dc)| a + e * (1.0 - dc));
    let max_rev = revenue.iter().fold(0.0, |a, (_, v)| f64::max(a, v));
    let mut rows: Vec<(usize, f64)> = Vec::new();
    revenue.iter().for_each(|(k, v)| if v == max_rev { rows.push((k, v)) });
    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, v)| {
        let i = *k;
        // natural suppkey = internal id + 1
        format!("{}|{}|{}|{}|{}", k + 1, d.su_name.values[i], d.su_address.values[i],
                d.su_phone.values[i], f(*v))
    }))
}

// ---------- Q16 — distinct supplier count ----------

fn q16(d: &TpchData) -> String {
    let live_ps = (&d.partsupp)
        .in_s((&d.ps_part).o(&d.pa_brand).ne("Brand#45"))
        .in_s((&d.ps_part).o(&d.pa_type).filt(|s: &str| !s.starts_with("MEDIUM POLISHED")))
        .in_s((&d.ps_part).o(&d.pa_size).in_v(vec![49, 14, 23, 45, 19, 3, 36, 9]))
        .in_s((&d.ps_supplier).o(&d.su_comment).nrx("Customer.*Complaints"));
    let group = (&live_ps).o((&d.ps_part).o((&d.pa_brand).x(&d.pa_type).x(&d.pa_size)));
    let supp  = (&live_ps).o(&d.ps_supplier);
    let counts = supp.group_by(group).count_distinct_p();
    let mut rows: Vec<(((&str, &str), i64), i64)> = Vec::new();
    rows.extend(counts.iter());
    rows.sort_by(|a, b| b.1.cmp(&a.1)
        .then(a.0.0.0.cmp(&b.0.0.0))
        .then(a.0.0.1.cmp(&b.0.0.1))
        .then(a.0.1.cmp(&b.0.1)));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}|{}|{}", k.0.0, k.0.1, k.1, v)))
}

// ---------- Q17 — small-quantity order revenue ----------

fn q17(d: &TpchData) -> String {
    // Per-part (sum_q, count) → 0.2 * avg in one fused fold.
    let threshold_per_part = (&d.li_quantity).group_by(&d.li_part)
        .fold_p((0.0_f64, 0_i64), |(s, n), q| (s + q, n + 1))
        .map(|(s, n)| 0.2 * s / n as f64);
    // Materialize so the cross-col compare doesn't re-fold per row.
    let tpp: HashIdx<_, _> = threshold_per_part.collect_p();
    let live = (&d.lineitem)
        .in_s((&d.li_part).o(&d.pa_brand).eq("Brand#23"))
        .in_s((&d.li_part).o(&d.pa_container).eq("MED BOX"))
        .in_s((&d.li_quantity).x((&d.li_part).o(&tpp))
             .filt(|(q, t)| q < t));
    let sum = live.o(&d.li_extendedprice)
        .iter().fold(0.0_f64, |a, (_, e)| a + e);
    f(sum / 7.0)
}

// ---------- Q18 — large volume customer ----------

fn q18(d: &TpchData) -> String {
    let sum_qty = (&d.li_quantity).group_by(&d.li_order).fold_p(0.0_f64, |a, q| a + q);
    let big = sum_qty.gt(300.0);
    let mut rows: Vec<(usize, f64)> = Vec::new();
    rows.extend(big.iter());
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

// ---------- Q19 — discounted revenue ----------

fn q19(d: &TpchData) -> String {
    // 3-branch disjunctive predicate folded into a single closure-filter.
    let pred = (&d.li_part).o((&d.pa_brand).x(&d.pa_container).x(&d.pa_size))
        .x(&d.li_quantity)
        .filt(|(((br, ct), sz), q)| {
            let in_v = |arr: &[&str], s: &str| arr.iter().any(|&a| a == s);
            (br == "Brand#12" && in_v(&["SM CASE","SM BOX","SM PACK","SM PKG"], ct)
                && q >= 1.0 && q <= 11.0 && sz >= 1 && sz <= 5)
            || (br == "Brand#23" && in_v(&["MED BAG","MED BOX","MED PKG","MED PACK"], ct)
                && q >= 10.0 && q <= 20.0 && sz >= 1 && sz <= 10)
            || (br == "Brand#34" && in_v(&["LG CASE","LG BOX","LG PACK","LG PKG"], ct)
                && q >= 20.0 && q <= 30.0 && sz >= 1 && sz <= 15)
        });
    let live = (&d.lineitem)
        .in_s((&d.li_shipmode).in_v(vec!["AIR", "AIR REG"]))
        .in_s((&d.li_shipinstruct).eq("DELIVER IN PERSON"))
        .in_s(pred);
    let sum = live.o((&d.li_extendedprice).x(&d.li_discount))
        .iter().fold(0.0_f64, |a, (_, (e, dc))| a + e * (1.0 - dc));
    f(sum)
}

// ---------- Q20 — potential part promotion ----------

fn q20(d: &TpchData) -> String {
    let live_li = d.lineitem.in_s((&d.li_shipdate).during(19940101, 19950101));
    let sum_qty = (&live_li).o(&d.li_quantity)
        .group_by((&live_li).o((&d.li_part).x(&d.li_supplier)))
        .fold_p(0.0_f64, |a, q| a + q);
    let threshold = (&d.ps_part).x(&d.ps_supplier).o(&sum_qty).map(|s| 0.5 * s);
    let qual_ps = (&d.partsupp)
        .in_s((&d.ps_part).o(&d.pa_name).filt(|n: &str| n.starts_with("forest")))
        .in_s((&d.ps_availqty).map(|q| q as f64).x(threshold).filt(|(a, t)| a > t));
    let canada_supps = (&d.supplier).in_s(
        (&d.su_nation).o(&d.na_name).eq("CANADA")
    );
    let qual_supps: MatSet<_> = qual_ps.o(&d.ps_supplier).collect_p();
    let target = canada_supps.in_s(qual_supps);
    let mut rows: Vec<(&str, &str)> = Vec::new();
    target.o((&d.su_name).x(&d.su_address)).iter().for_each(|(_, (n, a))| rows.push((n, a)));
    rows.sort_by(|a, b| a.0.cmp(b.0));
    join_lines(rows.iter().map(|(n, a)| format!("{}|{}", n, a)))
}

// ---------- Q21 — suppliers who kept orders waiting ----------

fn q21(d: &TpchData) -> String {
    let late = d.lineitem.in_s(
        (&d.li_commitdate).x(&d.li_receiptdate).filt(|(c, r)| c < r)
    );
    let multi_supp = (&d.li_supplier).group_by(&d.li_order).count_distinct_p().gt(1);
    let only_late = (&late).o(&d.li_supplier)
        .group_by((&late).o(&d.li_order))
        .count_distinct_p().eq(1);
    let saudi = (&d.supplier).and(
        (&d.su_nation).o(&d.na_name).eq("SAUDI ARABIA")
    );
    let f_ords = (&d.orders).and((&d.ord_status).eq("F"));
    let qualifying = (&late)
        .in_s((&d.li_supplier).in_s(saudi))
        .in_s((&d.li_order).in_s(f_ords.and(multi_supp).and(only_late)));
    let counts = qualifying.group_by(&d.li_supplier).fold_p(0_i64, |a, _| a + 1);
    let mut rows: Vec<(usize, i64)> = Vec::new();
    rows.extend(counts.iter());
    let mut named: Vec<(&str, i64)> = rows.iter()
        .map(|(s, c)| (d.su_name.values[*s], *c)).collect();
    named.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    named.truncate(100);
    join_lines(named.iter().map(|(n, c)| format!("{}|{}", n, c)))
}

// ---------- Q22 — global sales opportunity ----------

fn q22(d: &TpchData) -> String {
    let prefix = (&d.cu_phone).map(|p: &str| &p[..2]);
    let codes = vec!["13","31","23","29","30","18","17"];
    let prefix_ok = (&d.customer).in_s((&prefix).in_v(codes));
    let pos = (&prefix_ok).in_s((&d.cu_acctbal).gt(0.0));
    let (sum_p, cnt_p) = pos.o(&d.cu_acctbal)
        .iter().fold((0.0_f64, 0_i64), |(s, n), (_, v)| (s + v, n + 1));
    let avg = sum_p / cnt_p as f64;
    let custs_with_orders: MatSet<_> = (&d.ord_customer).collect_p();
    let target = (&prefix_ok).in_s((&d.cu_acctbal).gt(avg))
        .minus(custs_with_orders);
    let counts = target.group_by(&prefix)
        .fold_p((0_i64, 0.0_f64), |(cnt, sm), c| {
            let ab = d.cu_acctbal.values[c];
            (cnt + 1, sm + ab)
        });
    let mut rows: Vec<(&str, (i64, f64))> = Vec::new();
    rows.extend(counts.iter());
    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, (cnt, sm))| format!("{}|{}|{}", k, cnt, f(*sm))))
}

// ---------- Q2 — minimum-cost supplier per part ----------

fn q2(d: &TpchData) -> String {
    let eu_ps = (&d.partsupp).in_s(
        (&d.ps_supplier).o((&d.su_nation).o((&d.na_region).o(&d.re_name))).eq("EUROPE")
    );
    let min_per_part = (&eu_ps).o(&d.ps_supplycost)
        .group_by((&eu_ps).o(&d.ps_part))
        .fold_p(f64::INFINITY, |a, c| if c < a { c } else { a });
    let target = (&eu_ps)
        .in_s((&d.ps_part).o(&d.pa_size).eq(15))
        .in_s((&d.ps_part).o(&d.pa_type).filt(|s: &str| s.ends_with("BRASS")))
        .in_s((&d.ps_supplycost).x((&d.ps_part).o(&min_per_part))
             .filt(|(c, m)| c == m));
    // Project per PS row → (acct, sname, nname, pkey, mfgr, addr, phone, comm)
    let mut rows: Vec<(f64, &str, &str, usize, &str, &str, &str, &str)> = Vec::new();
    target.iter().for_each(|(psi, _)| {
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

// ---------- registry ----------

/// Full registry with every runner swapped to its pull port; oracles stay
/// the base registry's, so passing means byte-equality with the push suite.
pub fn queries() -> Vec<super::Entry> {
    common::with_overrides(&[
        ("1",  q1),  ("2",  q2),  ("3",  q3),  ("4",  q4),  ("5",  q5),
        ("6",  q6),  ("7",  q7),  ("8",  q8),  ("9",  q9),  ("10", q10),
        ("11", q11), ("12", q12), ("13", q13), ("14", q14), ("15", q15),
        ("16", q16), ("17", q17), ("18", q18), ("19", q19), ("20", q20),
        ("21", q21), ("22", q22),
    ])
}
