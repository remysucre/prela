// The duckdb-cheat variant — bounds what hand-rolled SQL-shaped code can
// do, so it skips the algebra wherever a raw loop or a dense array wins.
//
// Only the rewritten queries are defined here; everything else comes from
// the base registry in common.rs. q17/q22 are shared with the optimized
// variant (the algebraic rewrite is already the hand-rolled plan).

#![allow(clippy::too_many_lines)]

use ahash::AHashMap as HashMap;

use super::common::{f, join_lines, with_overrides};
use super::optimized;
use crate::engine::*;
use crate::tpch_data::{TpchData, fmt_yyyymmdd};

pub fn queries() -> Vec<super::Entry> {
    with_overrides(&[
        ("1", q1), ("2", q2), ("4", q4), ("8", q8), ("9", q9), ("12", q12),
        ("13", q13), ("17", optimized::q17), ("18", q18), ("20", q20),
        ("21", q21), ("22", optimized::q22),
    ])
}

fn q1(d: &TpchData) -> String {
    // ddbcheat (CP1.3 Small Group-By Keys): the group key (returnflag,
    // linestatus) has ≤ 6 distinct values across the entire dataset. Skip
    // the HashMap-keyed fold entirely and aggregate into a fixed-size
    // array, indexed by packing the two single-byte fields:
    //   idx = ((rf - b'A') << 4) | (ls - b'F')
    // Range upper-bound: rf ∈ {A=0, N=13, R=17}, ls ∈ {F=0, O=9} → max 281.
    // A [Acc; 288] table is sparse but the 4 hot indices stay cache-warm.
    type Acc = (f64, f64, f64, f64, f64, i64);
    let mut acc: [Acc; 288] = [(0.0, 0.0, 0.0, 0.0, 0.0, 0_i64); 288];
    let mut seen: [bool; 288] = [false; 288];

    for li in 0..d.lineitem.n {
        if d.li_shipdate.values[li] > 19980902 { continue; }
        let rf = d.li_returnflag.values[li].as_bytes()[0];
        let ls = d.li_status.values[li].as_bytes()[0];
        let idx = ((rf.wrapping_sub(b'A')) as usize) << 4
                | ((ls.wrapping_sub(b'F')) as usize);
        let q  = d.li_quantity.values[li];
        let e  = d.li_extendedprice.values[li];
        let dc = d.li_discount.values[li];
        let tx = d.li_tax.values[li];
        let dp  = e * (1.0 - dc);
        let chg = dp * (1.0 + tx);
        let a = &mut acc[idx];
        a.0 += q; a.1 += e; a.2 += dc; a.3 += dp; a.4 += chg; a.5 += 1;
        seen[idx] = true;
    }

    let mut rows: Vec<((u8, u8), Acc)> = Vec::new();
    for idx in 0..288 {
        if !seen[idx] { continue; }
        let rf = (idx >> 4) as u8 + b'A';
        let ls = (idx & 0xF) as u8 + b'F';
        rows.push(((rf, ls), acc[idx]));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    join_lines(rows.iter().map(|(k, (qty, ext, di, dp, chg, n))| {
        let nf = *n as f64;
        let rf = std::str::from_utf8(std::slice::from_ref(&k.0)).unwrap();
        let ls = std::str::from_utf8(std::slice::from_ref(&k.1)).unwrap();
        format!("{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                rf, ls, f(*qty), f(*ext), f(*dp), f(*chg),
                f(qty / nf), f(ext / nf), f(di / nf), n)
    }))
}

fn q4(d: &TpchData) -> String {
    // ddbcheat: instead of `lconj` building a HashSet from ~14M late-lineitem
    // orderkeys then intersecting with ~750K date-filtered orders, build a
    // packed bitset of "has-late-lineitem" indexed by orderkey in one scan.
    // The Conj-and short-circuits against the date predicate first; only
    // ~750K orderkeys hit the bit test.
    let mut is_late = Bitset::empty(d.orders.n);
    for li in 0..d.lineitem.n {
        if d.li_commitdate.values[li] < d.li_receiptdate.values[li] {
            is_late.set(d.li_order.values[li]);
        }
    }
    let live = d.orders
        .and((&d.ord_date).during(19930701, 19931001).k())
        .and(is_late);
    let counts = live.o(&d.ord_priority).inv().fold(0_i64, |a, _| a + 1);
    let mut rows: Vec<(&str, i64)> = Vec::new();
    counts.drive(|k, v| rows.push((k, v)));
    rows.sort_by(|a, b| a.0.cmp(b.0));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// ---------- Q8 — market share for BRAZIL ----------

fn q8(d: &TpchData) -> String {
    // ddbcheat: hoist the 4-hop `li_order → ord_customer → cu_nation →
    // na_region → re_name == "AMERICA"` chain into a Vec<bool> over the
    // dense order domain. One bool index per lineitem instead of 4 array
    // reads + a string compare.
    let n_ord = d.orders.n;
    let mut ord_is_america: Vec<bool> = vec![false; n_ord];
    for o in 0..n_ord {
        let cust = d.ord_customer.values[o];
        if cust == NO_ID { continue; }   // sparse orderkey gap (hole fill NO_ID)
        let nation = d.cu_nation.values[cust];
        let region = d.na_region.values[nation];
        ord_is_america[o] = d.re_name.values[region] == "AMERICA";
    }
    let live = (&d.lineitem)
        .and((&d.li_part).o(&d.pa_type).eq("ECONOMY ANODIZED STEEL").k())
        .and((&d.li_order).filt(move |o: usize| ord_is_america[o]).k())
        .and((&d.li_order).o(&d.ord_date).between(19950101, 19961231).k());
    let year = (&live).o((&d.li_order).o(&d.ord_date)).map(|d: i64| d / 10000);
    let snat_name = (&live).o((&d.li_supplier).o((&d.su_nation).o(&d.na_name)));
    let scan = (&live).o((&d.li_extendedprice).x(&d.li_discount)).x(snat_name);
    let pair_fold = year.lc(scan).fold((0.0_f64, 0.0_f64), |(b, t), ((e, dc), nm)| {
        let v = e * (1.0 - dc);
        (b + if nm == "BRAZIL" { v } else { 0.0 }, t + v)
    });
    let ratio = pair_fold.map(|(b, t)| b / t);
    let mut rows: Vec<(i64, f64)> = Vec::new();
    ratio.drive(|k, v| rows.push((k, v)));
    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, f(*v))))
}

// ---------- Q9 — product type profit measure ----------

fn q9(d: &TpchData) -> String {
    // ddbcheat (CP4.3): DuckDB's plan filters `part WHERE contains(p_name,
    // 'green')` to ~4K parts before touching lineitem. Our algebra calls
    // `str::contains("green")` once per lineitem (60M × TwoWaySearcher
    // ≈ 7s, the dominant cost in Q9). Materialize the predicate per part_id
    // as a dense Vec<bool> — one array index per row instead.
    let n_part = d.part.n;
    let mut pa_is_green = vec![false; n_part];
    for i in 0..n_part {
        pa_is_green[i] = d.pa_name.values[i].contains("green");
    }
    let sc = (&d.ps_part).x(&d.ps_supplier).inv().o(&d.ps_supplycost).mat_idx();
    let live = (&d.lineitem)
        .and((&d.li_part).filt(move |p: usize| pa_is_green[p]).k());
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
    // ddbcheat (CP4.2d Evaluation Order in Conjunctions): predicates reordered
    // by selectivity. Conj/`.and` chains evaluate via &&-short-circuiting
    // member checks, so the leftmost filter sees every row. The 1-year date
    // range (~14%) is by far the most selective; do it first.
    //   receiptdate ∈ [1994,1995): ~14%   (most selective)
    //   shipmode IN (MAIL, SHIP):  ~29%
    //   shipdate < commitdate:     ~49%
    //   commit < receipt:          ~62%   (barely filters)
    let live = (&d.lineitem)
        .and((&d.li_receiptdate).during(19940101, 19950101).k())
        .and((&d.li_shipmode).in_v(vec!["MAIL", "SHIP"]).k())
        .and((&d.li_shipdate).x(&d.li_commitdate).filt(|(s, c)| s < c).k())
        .and((&d.li_commitdate).x(&d.li_receiptdate).filt(|(c, r)| c < r).k());
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
    // ddbcheat: regex `special.*requests` runs ~100ns/call via DFA, ×15M
    // orders = the bulk of Q13. Use memchr's Finder (built once, ~5–10ns/call
    // via NEON SIMD) plus a precomputed Bitset over qualifying orders so
    // the hot path is a single bit test, not a regex evaluation.
    use memchr::memmem;
    let f_special  = memmem::Finder::new("special");
    let f_requests = memmem::Finder::new("requests");
    const SPECIAL_LEN: usize = 7;
    let mut not_special = Bitset::empty(d.orders.n);
    for o in 0..d.orders.n {
        if d.ord_customer.values[o] == NO_ID { continue; }   // skip sparse gaps (hole fill NO_ID)
        let bytes = d.ord_comment.values[o].as_bytes();
        let has_pattern = if let Some(pos) = f_special.find(bytes) {
            f_requests.find(&bytes[pos + SPECIAL_LEN..]).is_some()
        } else { false };
        if !has_pattern { not_special.set(o); }
    }

    let live_orders = (&d.orders).and(not_special);
    let count_per_cust = (&live_orders).o(&d.ord_customer)
        .lc((&live_orders).o(&d.ord_date))
        .fold(0_i64, |a, _| a + 1);
    let mut dist: HashMap<i64, i64> = HashMap::new();
    let mut n_with = 0i64;
    count_per_cust.drive(|_, c| { *dist.entry(c).or_insert(0) += 1; n_with += 1; });
    dist.insert(0, d.customer.n as i64 - n_with);
    let mut rows: Vec<_> = dist.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| b.0.cmp(a.0)));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// ---------- Q18 — large volume customer ----------

fn q18(d: &TpchData) -> String {
    // ddbcheat: replace `HashMap<order, f64>` fold with a dense `Vec<f32>`
    // indexed by orderkey. n_orders ≈ 60M → 240MB; sums are ≤ ~350 per
    // order (≤7 lineitems × ≤50 qty each) so f32's 7-digit precision is
    // ample. Skips 60M HashMap upserts.
    let n_ord = d.orders.n;
    let mut sum_qty: Vec<f32> = vec![0.0; n_ord];
    for li in 0..d.lineitem.n {
        let o = d.li_order.values[li];
        sum_qty[o] += d.li_quantity.values[li] as f32;
    }
    let mut rows: Vec<(usize, f64)> = Vec::new();
    for o in 0..n_ord {
        let s = sum_qty[o];
        if s > 300.0 { rows.push((o, s as f64)); }
    }
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

// ---------- Q20 — potential part promotion ----------

fn q20(d: &TpchData) -> String {
    // ddbcheat: dbgen lays out PartSupp consecutively, 4 PS rows per part:
    // the ps ids for part p are p*4 + {0,1,2,3} (0-based). So we can resolve
    // (part, supp) → ps_id with a 4-element linear probe — no HashMap, no
    // allocation per lookup. Verified at SF=10 against the cache layout.
    //
    // Pre-filter optimization: sum_qty is only consulted for ps_ids whose
    // part passes the "forest" name filter (~5% of parts). Precompute the
    // forest predicate per part as a dense Vec<bool> and skip non-forest
    // lineitems in the drive — drops ~95% of the layout-probe work.
    let n_part = d.part.n;
    let mut pa_is_forest = vec![false; n_part];
    for i in 0..n_part {
        pa_is_forest[i] = d.pa_name.values[i].starts_with("forest");
    }
    let n_ps = d.partsupp.n;
    // NaN sentinel: ps_ids that never see a live forest lineitem stay NaN, so
    // the subsequent `availqty > threshold` test returns false for them
    // (matches the original Compose-probe-miss semantics).
    let mut sum_qty: Vec<f64> = vec![f64::NAN; n_ps];
    let live_li = d.lineitem.and((&d.li_shipdate).during(19940101, 19950101).k());
    live_li.drivekeys(|li| {
        let part = d.li_part.values[li];
        if !pa_is_forest[part] { return; }
        let supp = d.li_supplier.values[li];
        let base = part * 4;
        for k in 0..4 {
            let psi = base + k;
            if d.ps_supplier.values[psi] == supp {
                let s = &mut sum_qty[psi];
                *s = if s.is_nan() { d.li_quantity.values[li] }
                     else { *s + d.li_quantity.values[li] };
                break;
            }
        }
    });
    let sum_qty_v = Vec1 { values: sum_qty };
    let threshold = (&sum_qty_v).map(|s| 0.5 * s);
    let qual_ps = (&d.partsupp)
        .and((&d.ps_part).o(&d.pa_name).filt(|n: &str| n.starts_with("forest")).k())
        .and((&d.ps_availqty).map(|q| q as f64).x(threshold).filt(|(a, t)| a > t).k());
    let canada_supps = (&d.supplier).and(
        (&d.su_nation).o(&d.na_name).eq("CANADA").k()
    );
    let target = qual_ps.o(&d.ps_supplier).lconj(canada_supps);
    let mut rows: Vec<(&str, &str)> = Vec::new();
    target.o((&d.su_name).x(&d.su_address)).drive(|_, (n, a)| rows.push((n, a)));
    rows.sort_by(|a, b| a.0.cmp(b.0));
    join_lines(rows.iter().map(|(n, a)| format!("{}|{}", n, a)))
}

// ---------- Q21 — suppliers who kept orders waiting ----------

fn q21(d: &TpchData) -> String {
    // ddbcheat: replace the two LeftCompose lazy-HashMap builds
    // (ord_to_supp + late_ord_to_supp, ~100M inserts of SVec<supp>) with
    // dense per-order summary state computed in a single pass:
    //   first_supp[o] / multi[o]            — for the EXISTS-other-supp test
    //   late_first[o] / late_multi[o]       — for the NOT-EXISTS-other-late-supp test
    // The bounded 2-state-per-order representation is sufficient: we only need
    // "≥ 2 distinct suppliers" and "exactly one distinct late supp, and it's
    // mine", neither of which requires enumerating the full set.
    // The no-supplier-yet sentinel is NO_ID: supplier ids are 0-based, so 0
    // is a live id (and NO_ID keeps the arrays at 8 bytes per order, unlike
    // Option<usize>).
    let n_ord  = d.orders.n;
    let n_supp = d.supplier.n;
    let mut first_supp: Vec<usize> = vec![NO_ID; n_ord];
    let mut multi:      Vec<bool>  = vec![false; n_ord];
    let mut late_first: Vec<usize> = vec![NO_ID; n_ord];
    let mut late_multi: Vec<bool>  = vec![false; n_ord];
    for li in 0..d.lineitem.n {
        let ord = d.li_order.values[li];
        let sup = d.li_supplier.values[li];
        if !multi[ord] {
            let prev = first_supp[ord];
            if prev == NO_ID    { first_supp[ord] = sup; }
            else if prev != sup { multi[ord] = true; }
        }
        let is_late = d.li_commitdate.values[li] < d.li_receiptdate.values[li];
        if is_late && !late_multi[ord] {
            let prev = late_first[ord];
            if prev == NO_ID    { late_first[ord] = sup; }
            else if prev != sup { late_multi[ord] = true; }
        }
    }

    // Cheap membership checks via dense Vec<bool>, same trick as Q9's
    // pa_is_green: one byte per supplier / order instead of multi-hop probe
    // chains in the per-row qualifying loop.
    let mut is_saudi: Vec<bool> = vec![false; n_supp];
    for s in 0..n_supp {
        let nation_id = d.su_nation.values[s];
        is_saudi[s] = d.na_name.values[nation_id] == "SAUDI ARABIA";
    }
    let mut is_f_ord: Vec<bool> = vec![false; n_ord];
    for o in 0..n_ord {
        is_f_ord[o] = d.ord_status.values[o] == "F";
    }

    let qualifying = d.lineitem
        // Order: late filter (cross-col) → Saudi (1 byte) → F-order (1 byte)
        // → multi (1 byte) → only-late (2 reads). Cheap-and-selective first.
        .and((&d.li_commitdate).x(&d.li_receiptdate).filt(|(c, r)| c < r).k())
        .and((&d.li_supplier).filt(move |s: usize| is_saudi[s]).k())
        .and((&d.li_order).filt(move |o: usize| is_f_ord[o]).k())
        .and((&d.li_order).filt(move |o: usize| multi[o]).k())
        .and((&d.li_order).x(&d.li_supplier).filt(move |(o, s)| {
            !late_multi[o] && late_first[o] == s
        }).k());
    let counts = (&d.li_supplier).lcs(qualifying).fold(0_i64, |a, _| a + 1);
    let mut rows: Vec<(usize, i64)> = Vec::new();
    counts.drive(|k, v| rows.push((k, v)));
    let mut named: Vec<(&str, i64)> = rows.iter()
        .map(|(s, c)| (d.su_name.values[*s], *c)).collect();
    named.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    named.truncate(100);
    join_lines(named.iter().map(|(n, c)| format!("{}|{}", n, c)))
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
    let eu_ps = (&d.partsupp).and(
        (&d.ps_supplier).o((&d.su_nation).o((&d.na_region).o(&d.re_name))).eq("EUROPE").k()
    );
    // ddbcheat: partkey is a dense i64 0..N, so min-per-key fits in a
    // Vec<f64> indexed by partkey — no hash, no allocation per insert. The
    // generic `.fold().mat()` builds an AHashMap<i64,f64> which is ~2× the
    // wall-clock for this fold's hot loop.
    let n_part = d.part.n;
    let mut min_per_part: Vec<f64> = vec![f64::INFINITY; n_part];
    (&eu_ps).drivekeys(|ps| {
        let p = d.ps_part.values[ps];
        let c = d.ps_supplycost.values[ps];
        if c < min_per_part[p] { min_per_part[p] = c; }
    });
    let target = (&eu_ps)
        .and((&d.ps_part).o(&d.pa_size).eq(15).k())
        .and((&d.ps_part).o(&d.pa_type).filt(|s: &str| s.ends_with("BRASS")).k())
        .and((&d.ps_supplycost).x(&d.ps_part)
             .filt(move |(c, p)| c == min_per_part[p]).k());
    // Project per PS row → (acct, sname, nname, pkey, mfgr, addr, phone, comm)
    let mut rows: Vec<(f64, &str, &str, usize, &str, &str, &str, &str)> = Vec::new();
    target.drivekeys(|psi| {
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
