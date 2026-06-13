// The optimized variant — same algebra as the baseline, but hand-encoding
// the plans a stats-driven optimizer (DuckDB's) would pick: dense folds,
// bitset semi-joins, selectivity-ordered conjunctions, SIMD substring
// search. Prela has no optimizer, so the rewrites live in the query text.
//
// Only the rewritten queries are defined here; everything else comes from
// the base registry in common.rs.

#![allow(clippy::too_many_lines)]

use std::collections::HashMap;

use super::common::{f, fmt_yyyymmdd, join_lines, with_overrides};
use crate::engine::*;
use crate::tpch_schema::*;

pub fn queries() -> Vec<super::Entry> {
    with_overrides(&[
        ("1", q1), ("2", q2), ("4", q4), ("9", q9), ("12", q12),
        ("13", q13), ("17", q17), ("18", q18), ("21", q21), ("22", q22),
    ])
}

fn q1() -> String {
    // Julia: ((returnflag ⊗ Li.status) ← (lineitem → shipdate <= "..." : qty ⊗ ext ⊗ disc ⊗ tax))
    //        ▷ (cmb, ...) ↦ out
    let live = lineitem.with(shipdate.le(19980902));
    let scan = live.select(
        quantity.and(extendedprice).and(discount).and(tax)
    );
    // Pack (returnflag, status) single-byte values into a small usize index
    // so `dense_fold` can use a `[Acc; 288]`-equivalent dense cache. The
    // packed order `(rf-'A') << 4 | (ls-'F')` preserves the (rf, ls)
    // ascii-pair sort order under integer comparison: rf ∈ {A=0, N=13,
    // R=17}, ls ∈ {F=0, O=9} → max key 281, so ≥282 slots; 288 used.
    let group_key = returnflag.and(Lineitem::status)
        .map(|(rf, ls): (&str, &str)| {
            ((rf.as_bytes()[0].wrapping_sub(b'A') as usize) << 4)
                | (ls.as_bytes()[0].wrapping_sub(b'F') as usize)
        });
    let grouped = scan.group_by(group_key)
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

fn q4() -> String {
    // Julia: let live = (lineitem ∧ (commitdate < receiptdate) → Li.order) ⩘
    //                  (orders ∧ (date in during("1993-07-01", "1993-10-01")))
    //        (live → Ord.priority)' ▷ ((a, _) -> a + 1, 0)
    // Dense `Bitset` of orderkeys with a late lineitem replaces the MatSet
    // path that lazy-built a HashSet from ~14M late-lineitem orderkeys.
    let bad_li_order = lineitem
        .with(commitdate.and(receiptdate).filt(|(c, r)| c < r))
        .order();
    let is_late = Bitset::over(orders, &bad_li_order);
    let live = orders
        .with(date.during(19930701, 19931001))
        .with(is_late);
    let counts = live.priority().inv().fold(0_i64, |a, _| a + 1);
    let mut rows: Vec<(&str, i64)> = Vec::new();
    counts.drive(|k, v| rows.push((k, v)));
    rows.sort_by(|a, b| a.0.cmp(b.0));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// ---------- Q9 — product type profit measure ----------

fn q9() -> String {
    // CP1.3 / CP1.4: group on (nation_id, year) as (Id<Nation>, i64) — 16-byte
    // integer hash key, not (&str, i64) which costs a string hash + memcmp
    // per collision. Nation name is FD'd by nation_id, looked up at output.
    let sc: HashIdx<_, _> = PartSupp::part.and(PartSupp::supplier).inv().supplycost().collect();
    // Hoist the `Part.name ~ "green"` predicate out of the 60M-row
    // lineitem scan by materializing the matching part-ids into a `Bitset`
    // (~200K Part rows scanned once). Per lineitem becomes one bit-test.
    let green_parts = Bitset::over(
        part,
        &part.with(Part::name.filt(|n: &str| n.contains("green"))),
    );
    let live = lineitem.with(Lineitem::part.select(&green_parts));
    let nation_id = (&live).supplier().nation();
    let year      = (&live).order().date().map(|d: i64| d / 10000);
    let groups = nation_id.and(year);
    let cost_per_li = Lineitem::part.and(Lineitem::supplier).select(&sc);
    let scan = (&live).select(
        extendedprice.and(discount).and(quantity).and(cost_per_li)
    );
    let result = scan.group_by(groups).fold(0.0_f64, |a, (((e, dc), q), cost)| {
        a + e * (1.0 - dc) - cost * q
    });
    let mut rows: Vec<((Id<Nation>, i64), f64)> = Vec::new();
    result.drive(|k, v| rows.push((k, v)));
    rows.sort_by(|a, b| {
        let na = Nation::name.iq().values[a.0.0.idx()];
        let nb = Nation::name.iq().values[b.0.0.idx()];
        na.cmp(nb).then_with(|| b.0.1.cmp(&a.0.1))
    });
    join_lines(rows.iter().map(|(k, v)| {
        format!("{}|{}|{}", Nation::name.iq().values[k.0.idx()], k.1, f(*v))
    }))
}

fn q12() -> String {
    // Conjuncts reordered by oracle-known selectivity (most selective first)
    // so each conjunct shaves rows off every downstream predicate — the
    // member check short-circuits left to right.
    // The algebra preserves whatever order the user wrote — Prela has no
    // stats-driven optimizer; here we hand-encode the order DuckDB's planner
    // would pick, to show the algebra *can* express the optimal plan.
    //   receiptdate ∈ [1994,1995): ~14%  (most selective)
    //   shipmode IN (MAIL, SHIP):  ~29%
    //   shipdate < commitdate:     ~49%
    //   commit  < receipt:         ~62%  (barely filters; runs last)
    let live = lineitem
        .with(receiptdate.during(19940101, 19950101)
         .and(shipmode.is_in(["MAIL", "SHIP"]))
         .and(shipdate.and(commitdate).filt(|(s, c)| s < c))
         .and(commitdate.and(receiptdate).filt(|(c, r)| c < r)));
    let scan = (&live).shipmode();
    let prio = (&live).order().priority();
    let result = prio.group_by(scan).fold((0_i64, 0_i64), |(h, l), pr| {
        let is_high = pr == "1-URGENT" || pr == "2-HIGH";
        if is_high { (h + 1, l) } else { (h, l + 1) }
    });
    let mut rows: Vec<(&str, (i64, i64))> = Vec::new();
    result.drive(|k, v| rows.push((k, v)));
    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, (h, l))| format!("{}|{}|{}", k, h, l)))
}

// ---------- Q13 — customer distribution (LEFT JOIN) ----------

fn q13() -> String {
    // Julia algebraic part: count_per_cust = ((live_orders → Ord.customer) ←
    //                                          (live_orders → date)) ▷ ((a, _) -> a + 1, 0)
    // Then a Julia escape for the LEFT-JOIN zero-default (customer.iq().n - n_with).
    // `nrx("special.*requests")` is regex DFA per row (~100ns × 1.5M).
    // memmem::Finder for "special" is SIMD-backed (~5-10ns); then probe
    // the suffix for "requests" with stdlib `.contains` (also SIMD).
    use memchr::memmem;
    let f_special = memmem::Finder::new("special");
    let live_orders = orders
        .with(Order::customer.filt(|c| c != Dense::NONE)    // skip sparse orderkey gaps (hole fill NO_ID)
         .and(Order::comment.filt(move |c: &str| {
                  match f_special.find(c.as_bytes()) {
                      Some(p) => !c[p + "special".len()..].contains("requests"),
                      None => true,
                  }
              })));
    // `customer` is a dense 0..n id domain, so the per-customer order count
    // folds into a `Vec<i64>` indexed by customer id (DenseFold) rather than
    // a HashMap keyed by id (Fold) — one array bump per order, no hashing,
    // across all ~15M live orders. Same hoist as q9/q15/q21's dense folds.
    let count_per_cust = (&live_orders).date()
        .group_by((&live_orders).customer())
        .dense_fold(customer.iq().n, 0_i64, |a, _| a + 1);
    let mut dist: HashMap<i64, i64> = HashMap::new();
    let mut n_with = 0i64;
    count_per_cust.drive(|_, c| { *dist.entry(c).or_insert(0) += 1; n_with += 1; });
    // LEFT JOIN zero-default: customers with no qualifying orders contribute to c_count=0.
    dist.insert(0, customer.iq().n as i64 - n_with);
    let mut rows: Vec<_> = dist.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| b.0.cmp(a.0)));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// ---------- Q17 — small-quantity order revenue ----------

pub(super) fn q17() -> String {
    // Semi-join reduction (Boncz CP5.2 / Dreseler §4.8): the threshold
    // avg(quantity) only matters for the ~200 parts that pass the
    // brand+container filter, not all ~200K parts. Identify the qualifying
    // parts up front, restrict the lineitem scan that feeds the avg fold
    // to those parts, then the avg is computed over a tiny slice.
    // Dense `Bitset` of qualifying part-ids (not a HashSet): membership is
    // bit-tested on every lineitem in both the avg fold and the outer scan
    // (~12M probes), so a bitmask beats a hash lookup — same hoist as q9's
    // `green_parts` / q21's predicate bitsets.
    let qual_parts = Bitset::over(part,
        &part.with(brand.eq("Brand#23").and(container.eq("MED BOX"))));
    let live_li = Lineitem::part.with(&qual_parts);
    let threshold_per_part = quantity.group_by(&live_li)
        .fold((0.0_f64, 0_i64), |(s, n), q| (s + q, n + 1))
        .map(|(s, n)| 0.2 * s / n as f64);
    let tpp: HashIdx<_, _> = threshold_per_part.collect();
    let live = lineitem
        .with((&live_li)
         .and(quantity.and(Lineitem::part.select(&tpp)).filt(|(q, t)| q < t)));
    let sum = live.select(extendedprice)
        .unwrap_fold(0.0_f64, |a, e| a + e);
    f(sum / 7.0)
}

// ---------- Q18 — large volume customer ----------

fn q18() -> String {
    let sum_qty = quantity.group_by(order).dense_fold(orders.iq().n, 0.0_f64, |a, q| a + q);
    let big = sum_qty.gt(300.0);
    let mut rows: Vec<(Id<Order>, f64)> = Vec::new();
    big.drive(|k, v| rows.push((k, v)));
    rows.sort_by(|a, b| {
        let (oa, ob) = (a.0.idx(), b.0.idx());
        totalprice.iq().values[ob].partial_cmp(&totalprice.iq().values[oa]).unwrap()
            .then_with(|| date.iq().values[oa].cmp(&date.iq().values[ob]))
    });
    rows.truncate(100);
    join_lines(rows.iter().map(|(o, sum_q)| {
        let oi = o.idx();
        let cu = Order::customer.iq().values[oi];
        let cui = cu.idx();
        // natural custkey / orderkey = internal id + 1
        format!("{}|{}|{}|{}|{}|{}",
                Customer::name.iq().values[cui], cui + 1, oi + 1,
                fmt_yyyymmdd(date.iq().values[oi]), f(totalprice.iq().values[oi]), f(*sum_q))
    }))
}

// ---------- Q21 — suppliers who kept orders waiting ----------

fn q21() -> String {
    // Julia:
    //   late = lineitem ∧ (receiptdate > commitdate)
    //   n_distinct = vs -> length(unique(vs))
    //   multi_supp = askeys((order ← Li.supplier) ▷ n_distinct > 1)
    //   only_late  = askeys(((late : order) ← (late : Li.supplier)) ▷ n_distinct == 1)
    //   qualifying = late ∧ (Li.supplier → saudi) ∧ (order → f_ords ∧ multi_supp ∧ only_late)
    //   (Li.supplier ← qualifying) ▷ ((a, _) -> a + 1, 0) ⊗ Su.name
    let late = lineitem.with(
        commitdate.and(receiptdate).filt(|(c, r)| c < r)
    );
    // `multi_supp`/`only_late` are SET-membership predicates over orderkeys
    // ("more than one distinct supplier" / "exactly one distinct supplier").
    // The Julia `count_distinct() > 1` form materializes a `SVec` of suppliers
    // per group then sort-dedups — needless when we only care whether the
    // count is 0/1/>1. Replace with a constant-state fold tracking
    // `(first_supplier, saw_a_second)` per orderkey; `multi` ⇔ `saw_second`
    // and `only_one` ⇔ `first.is_some() && !saw_second`. "No supplier seen
    // yet" is `None` — supplier ids are 0-based, so 0 is a live id.
    let track = |(first, multi): (Option<Id<Supplier>>, bool), s: Id<Supplier>| match first {
        None => (Some(s), multi),
        Some(f) if f != s => (first, true),
        _ => (first, multi),
    };
    let supp_state = Lineitem::supplier.group_by(order)
        .dense_fold(orders.iq().n, (None, false), track);
    let late_supp_state = (&late).select(Lineitem::supplier)
        .group_by((&late).select(order))
        .dense_fold(orders.iq().n, (None, false), track);
    // Only the SAUDI suppliers' late lineitems can qualify, so `saudi` is the
    // selective FIRST conjunct — precompute it as a Bitset (bit-tested per
    // late lineitem; supplier.n is tiny). The order predicates (status F,
    // >1 distinct supplier, exactly-one-late) ride behind it: rather than
    // materialize each as an order-wide Bitset (three 6M-row build scans),
    // probe the status column and the dense-fold states directly — they fire
    // only on the handful of saudi-late survivors the `.and` short-circuits to.
    let saudi_bs   = Bitset::over(supplier, &supplier.with(Supplier::nation.eq("SAUDI ARABIA")));
    let f_ord      = Order::status.eq("F");
    let multi_supp = supp_state.filt(|(_, m): (Option<Id<Supplier>>, bool)| m);
    let only_late  = late_supp_state.filt(|(f, m): (Option<Id<Supplier>>, bool)| f.is_some() && !m);
    let qualifying = (&late)
        .with(Lineitem::supplier.select(&saudi_bs)
         .and(order.select((&f_ord).and(&multi_supp).and(&only_late))));
    let counts = qualifying.group_by(Lineitem::supplier).fold(0_i64, |a, _| a + 1);
    let mut rows: Vec<(Id<Supplier>, i64)> = Vec::new();
    counts.drive(|k, v| rows.push((k, v)));
    let mut named: Vec<(&str, i64)> = rows.iter()
        .map(|(s, c)| (Supplier::name.iq().values[s.idx()], *c)).collect();
    named.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    named.truncate(100);
    join_lines(named.iter().map(|(n, c)| format!("{}|{}", n, c)))
}

// ---------- Q22 — global sales opportunity ----------

pub(super) fn q22() -> String {
    // Julia:
    //   prefix = Cu.phone ↦ (s -> s[1:2])
    //   prefix_ok = customer ∧ (prefix in codes)
    //   avg = unwrap((prefix_ok ∧ (acctbal > 0) → acctbal) ⊵ ... ↦ s/n)
    //   target = (prefix_ok ∧ (acctbal > avg)) - !((orders → Ord.customer)')
    //   (prefix ← (target : acctbal)) ▷ ((cnt, sm), ab) -> (cnt+1, sm+ab)
    let prefix = Customer::phone.map(|p: &str| &p[..2]);
    let codes = ["13","31","23","29","30","18","17"];
    let prefix_ok = customer.with((&prefix).is_in(codes));
    let pos = (&prefix_ok).with(Customer::acctbal.gt(0.0));
    let (sum_p, cnt_p) = pos.select(Customer::acctbal)
        .unwrap_fold((0.0_f64, 0_i64), |(s, n), v| (s + v, n + 1));
    let avg = sum_p / cnt_p as f64;
    // Packed bitset over the dense customer universe — replaces the
    // baseline's collected `MatSet` (a HashSet built from every order's customer)
    // with one bit per customer.
    let custs_with_orders = Bitset::over(customer, Order::customer);
    let target = (&prefix_ok).with(Customer::acctbal.gt(avg))
        .minus(custs_with_orders);
    let counts = target.group_by(&prefix)
        .fold((0_i64, 0.0_f64), |(cnt, sm), c| {
            let ab = Customer::acctbal.iq().values[c.idx()];
            (cnt + 1, sm + ab)
        });
    let mut rows: Vec<(&str, (i64, f64))> = Vec::new();
    counts.drive(|k, v| rows.push((k, v)));
    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, (cnt, sm))| format!("{}|{}|{}", k, cnt, f(*sm))))
}

// ---------- Q2 — minimum-cost supplier per part ----------

fn q2() -> String {
    // Julia:
    //   eu_ps = partsupp ∧ (PS.supplier → Su.nation → Na.region → Re.name == "EUROPE")
    //   min_per_part = !(((eu_ps → PS.part) ← (eu_ps → supplycost)) ▷ (min, Inf))
    //   target = eu_ps ∧ (PS.part → (size == 15 ∧ type ~ "BRASS$"))
    //                  ∧ (supplycost == (PS.part → min_per_part))
    //   target : (Su.acctbal ⊗ Su.name ⊗ Na.name ⊗ PS.part ⊗ Pa.mfgr
    //             ⊗ Su.address ⊗ Su.phone ⊗ Su.comment)
    let eu_ps = partsupp.with(PartSupp::supplier.nation().region().eq("EUROPE"));
    let min_per_part = (&eu_ps).supplycost()
        .group_by((&eu_ps).part())
        .dense_fold(part.iq().n, f64::INFINITY, |a, c| if c < a { c } else { a });
    let target = (&eu_ps)
        .with(PartSupp::part.size().eq(15)
         .and(PartSupp::part.ty().filt(|s: &str| s.ends_with("BRASS")))
         .and(supplycost.and(PartSupp::part.select(&min_per_part)).filt(|(c, m)| c == m)));
    // Project per PS row → (acct, sname, nname, pkey, mfgr, addr, phone, comm)
    let mut rows: Vec<(f64, &str, &str, Id<Part>, &str, &str, &str, &str)> = Vec::new();
    target.drive(|psi, _| {
        let pa = PartSupp::part.iq().values[psi.idx()];
        let su = PartSupp::supplier.iq().values[psi.idx()].idx();
        rows.push((
            Supplier::acctbal.iq().values[su],
            Supplier::name.iq().values[su],
            Nation::name.iq().values[Supplier::nation.iq().values[su].idx()],
            pa,
            mfgr.iq().values[pa.idx()],
            Supplier::address.iq().values[su],
            Supplier::phone.iq().values[su],
            Supplier::comment.iq().values[su],
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
        f(r.0), r.1, r.2, r.3.idx() + 1, r.4, r.5, r.6, r.7)))
}
