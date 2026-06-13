// The duckdb-cheat variant — bounds what hand-rolled SQL-shaped code can
// do, so it skips the algebra wherever a raw loop or a dense array wins.
// The raw loops index the typed columns' `.values` directly — `Id<E>` is
// `repr(transparent)` over `usize`, so `.idx()` is the only ceremony.
//
// Only the rewritten queries are defined here; everything else comes from
// the base registry in common.rs. q17/q22 are shared with the optimized
// variant (the algebraic rewrite is already the hand-rolled plan).

#![allow(clippy::too_many_lines)]

use ahash::AHashMap as HashMap;

use super::common::{f, fmt_yyyymmdd, join_lines, with_overrides};
use super::optimized;
use crate::engine::*;
use crate::tpch_schema::*;

pub fn queries() -> Vec<super::Entry> {
    with_overrides(&[
        ("1", q1), ("2", q2), ("4", q4), ("8", q8), ("9", q9), ("12", q12),
        ("13", q13), ("17", optimized::q17), ("18", q18), ("20", q20),
        ("21", q21), ("22", optimized::q22),
    ])
}

fn q1() -> String {
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

    let (li_shipdate, li_returnflag, li_status) = (shipdate.iq(), returnflag.iq(), Lineitem::status.iq());
    let (li_quantity, li_extendedprice, li_discount, li_tax) =
        (quantity.iq(), extendedprice.iq(), discount.iq(), tax.iq());
    for li in 0..lineitem.iq().n {
        if li_shipdate.values[li] > 19980902 { continue; }
        let rf = li_returnflag.values[li].as_bytes()[0];
        let ls = li_status.values[li].as_bytes()[0];
        let idx = ((rf.wrapping_sub(b'A')) as usize) << 4
                | ((ls.wrapping_sub(b'F')) as usize);
        let q  = li_quantity.values[li];
        let e  = li_extendedprice.values[li];
        let dc = li_discount.values[li];
        let tx = li_tax.values[li];
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

fn q4() -> String {
    // ddbcheat: instead of collecting a MatSet from ~14M late-lineitem
    // orderkeys then intersecting with ~750K date-filtered orders, build a
    // packed bitset of "has-late-lineitem" indexed by orderkey in one scan.
    // The chained restriction short-circuits against the date predicate
    // first; only ~750K orderkeys hit the bit test.
    let mut is_late = Bitset::empty(orders);
    let (li_commitdate, li_receiptdate, li_order) = (commitdate.iq(), receiptdate.iq(), order.iq());
    for li in 0..lineitem.iq().n {
        if li_commitdate.values[li] < li_receiptdate.values[li] {
            is_late.set(li_order.values[li]);
        }
    }
    let live = orders
        .when(date.during(19930701, 19931001))
        .when(is_late);
    let counts = live.get(priority).inv().fold(0_i64, |a, _| a + 1);
    let mut rows: Vec<(&str, i64)> = Vec::new();
    counts.drive(|k, v| rows.push((k, v)));
    rows.sort_by(|a, b| a.0.cmp(b.0));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// ---------- Q8 — market share for BRAZIL ----------

fn q8() -> String {
    // ddbcheat: hoist the 4-hop `li_order → ord_customer → cu_nation →
    // na_region → re_name == "AMERICA"` chain into a Vec<bool> over the
    // dense order domain. One bool index per lineitem instead of 4 array
    // reads + a string compare.
    let n_ord = orders.iq().n;
    let mut ord_is_america: Vec<bool> = vec![false; n_ord];
    for o in 0..n_ord {
        let cust = Order::customer.iq().values[o];
        if cust == Dense::NONE { continue; }   // sparse orderkey gap (hole fill NO_ID)
        let na = Customer::nation.iq().values[cust.idx()];
        let re = Nation::region.iq().values[na.idx()];
        ord_is_america[o] = Region::name.iq().values[re.idx()] == "AMERICA";
    }
    let live = lineitem
        .when(Lineitem::part.ty().eq("ECONOMY ANODIZED STEEL")
              .and(order.filt(move |o: Id<Order>| ord_is_america[o.idx()]))
              .and(order.date().between(19950101, 19961231)));
    let year = (&live).order().date().map(|d: i64| d / 10000);
    let snat_name = (&live).supplier().nation().name();
    let scan = (&live).get(extendedprice.and(discount)).and(snat_name);
    let pair_fold = scan.group_by(year).fold((0.0_f64, 0.0_f64), |(b, t), ((e, dc), nm)| {
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

fn q9() -> String {
    // ddbcheat (CP4.3): DuckDB's plan filters `part WHERE contains(p_name,
    // 'green')` to ~4K parts before touching lineitem. Our algebra calls
    // `str::contains("green")` once per lineitem (60M × TwoWaySearcher
    // ≈ 7s, the dominant cost in Q9). Materialize the predicate per part_id
    // as a dense Vec<bool> — one array index per row instead.
    let n_part = part.iq().n;
    let mut pa_is_green = vec![false; n_part];
    for i in 0..n_part {
        pa_is_green[i] = Part::name.iq().values[i].contains("green");
    }
    let sc: HashIdx<_, _> = PartSupp::part.and(PartSupp::supplier).inv().supplycost().collect();
    let live = lineitem
        .when(Lineitem::part.filt(move |p: Id<Part>| pa_is_green[p.idx()]));
    let nation_id = (&live).supplier().nation();
    let year      = (&live).order().date().map(|d: i64| d / 10000);
    let groups = nation_id.and(year);
    let cost_per_li = Lineitem::part.and(Lineitem::supplier).get(&sc);
    let scan = (&live).get(
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
    // ddbcheat (CP4.2d Evaluation Order in Conjunctions): predicates reordered
    // by selectivity. The conjunction's member check is a flat &&-short-circuit
    // left to right, so the leftmost filter sees every row. The 1-year date
    // range (~14%) is by far the most selective; do it first.
    //   receiptdate ∈ [1994,1995): ~14%   (most selective)
    //   shipmode IN (MAIL, SHIP):  ~29%
    //   shipdate < commitdate:     ~49%
    //   commit < receipt:          ~62%   (barely filters)
    let live = lineitem
        .when(receiptdate.during(19940101, 19950101)
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
    // ddbcheat: regex `special.*requests` runs ~100ns/call via DFA, ×15M
    // orders = the bulk of Q13. Use memchr's Finder (built once, ~5–10ns/call
    // via NEON SIMD) plus a precomputed Bitset over qualifying orders so
    // the hot path is a single bit test, not a regex evaluation.
    use memchr::memmem;
    let f_special  = memmem::Finder::new("special");
    let f_requests = memmem::Finder::new("requests");
    const SPECIAL_LEN: usize = 7;
    let mut not_special = Bitset::empty(orders);
    let (ord_customer, ord_comment) = (Order::customer.iq(), Order::comment.iq());
    for o in 0..orders.iq().n {
        if ord_customer.values[o] == Dense::NONE { continue; }   // skip sparse gaps (hole fill NO_ID)
        let bytes = ord_comment.values[o].as_bytes();
        let has_pattern = if let Some(pos) = f_special.find(bytes) {
            f_requests.find(&bytes[pos + SPECIAL_LEN..]).is_some()
        } else { false };
        if !has_pattern { not_special.set(Id::new(o)); }
    }

    let live_orders = orders.when(not_special);
    let count_per_cust = (&live_orders).date()
        .group_by((&live_orders).customer())
        .fold(0_i64, |a, _| a + 1);
    let mut dist: HashMap<i64, i64> = HashMap::new();
    let mut n_with = 0i64;
    count_per_cust.drive(|_, c| { *dist.entry(c).or_insert(0) += 1; n_with += 1; });
    dist.insert(0, customer.iq().n as i64 - n_with);
    let mut rows: Vec<_> = dist.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| b.0.cmp(a.0)));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// ---------- Q18 — large volume customer ----------

fn q18() -> String {
    // ddbcheat: replace `HashMap<order, f64>` fold with a dense `Vec<f32>`
    // indexed by orderkey. n_orders ≈ 60M → 240MB; sums are ≤ ~350 per
    // order (≤7 lineitems × ≤50 qty each) so f32's 7-digit precision is
    // ample. Skips 60M HashMap upserts.
    let n_ord = orders.iq().n;
    let mut sum_qty: Vec<f32> = vec![0.0; n_ord];
    let (li_order, li_quantity) = (order.iq(), quantity.iq());
    for li in 0..lineitem.iq().n {
        let o = li_order.values[li].idx();
        sum_qty[o] += li_quantity.values[li] as f32;
    }
    let mut rows: Vec<(usize, f64)> = Vec::new();
    for o in 0..n_ord {
        let s = sum_qty[o];
        if s > 300.0 { rows.push((o, s as f64)); }
    }
    rows.sort_by(|a, b| {
        let (oa, ob) = (a.0, b.0);
        totalprice.iq().values[ob].partial_cmp(&totalprice.iq().values[oa]).unwrap()
            .then_with(|| date.iq().values[oa].cmp(&date.iq().values[ob]))
    });
    rows.truncate(100);
    join_lines(rows.iter().map(|(o, sum_q)| {
        let oi = *o;
        let cu = Order::customer.iq().values[oi];
        let cui = cu.idx();
        // natural custkey / orderkey = internal id + 1
        format!("{}|{}|{}|{}|{}|{}",
                Customer::name.iq().values[cui], cui + 1, oi + 1,
                fmt_yyyymmdd(date.iq().values[oi]), f(totalprice.iq().values[oi]), f(*sum_q))
    }))
}

// ---------- Q20 — potential part promotion ----------

fn q20() -> String {
    // ddbcheat: dbgen lays out PartSupp consecutively, 4 PS rows per part:
    // the ps ids for part p are p*4 + {0,1,2,3} (0-based). So we can resolve
    // (part, supp) → ps_id with a 4-element linear probe — no HashMap, no
    // allocation per lookup. Verified at SF=10 against the cache layout.
    //
    // Pre-filter optimization: sum_qty is only consulted for ps_ids whose
    // part passes the "forest" name filter (~5% of parts). Precompute the
    // forest predicate per part as a dense Vec<bool> and skip non-forest
    // lineitems in the drive — drops ~95% of the layout-probe work.
    let n_part = part.iq().n;
    let mut pa_is_forest = vec![false; n_part];
    for i in 0..n_part {
        pa_is_forest[i] = Part::name.iq().values[i].starts_with("forest");
    }
    let n_ps = partsupp.iq().n;
    // NaN sentinel: ps_ids that never see a live forest lineitem stay NaN, so
    // the subsequent `availqty > threshold` test returns false for them
    // (matches the original Compose-probe-miss semantics).
    let mut sum_qty: Vec<f64> = vec![f64::NAN; n_ps];
    let live_li = lineitem.when(shipdate.during(19940101, 19950101));
    let (li_part, li_supplier, li_quantity) =
        (Lineitem::part.iq(), Lineitem::supplier.iq(), quantity.iq());
    let ps_supplier = PartSupp::supplier.iq();
    live_li.drive(|li, _| {
        let li = li.idx();
        let pa = li_part.values[li];
        if !pa_is_forest[pa.idx()] { return; }
        let supp = li_supplier.values[li];
        let base = pa.idx() * 4;
        for k in 0..4 {
            let psi = base + k;
            if ps_supplier.values[psi] == supp {
                let s = &mut sum_qty[psi];
                *s = if s.is_nan() { li_quantity.values[li] }
                     else { *s + li_quantity.values[li] };
                break;
            }
        }
    });
    let sum_qty_v: VecRel<f64, Id<PartSupp>> = VecRel::new(sum_qty);
    let threshold = (&sum_qty_v).map(|s| 0.5 * s);
    let qual_ps = partsupp
        .when(PartSupp::part.name().filt(|n: &str| n.starts_with("forest"))
              .and(availqty.map(|q| q as f64).and(threshold).filt(|(a, t)| a > t)));
    let canada_supps = supplier.when(Supplier::nation.name().eq("CANADA"));
    let qual_supps: MatSet<_> = qual_ps.supplier().collect();
    let target = canada_supps.when(qual_supps);
    let mut rows: Vec<(&str, &str)> = Vec::new();
    target.get(Supplier::name.and(Supplier::address)).drive(|_, (n, a)| rows.push((n, a)));
    rows.sort_by(|a, b| a.0.cmp(b.0));
    join_lines(rows.iter().map(|(n, a)| format!("{}|{}", n, a)))
}

// ---------- Q21 — suppliers who kept orders waiting ----------

fn q21() -> String {
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
    let n_ord  = orders.iq().n;
    let n_supp = supplier.iq().n;
    let mut first_supp: Vec<usize> = vec![NO_ID; n_ord];
    let mut multi:      Vec<bool>  = vec![false; n_ord];
    let mut late_first: Vec<usize> = vec![NO_ID; n_ord];
    let mut late_multi: Vec<bool>  = vec![false; n_ord];
    let (li_order, li_supplier) = (order.iq(), Lineitem::supplier.iq());
    let (li_commitdate, li_receiptdate) = (commitdate.iq(), receiptdate.iq());
    for li in 0..lineitem.iq().n {
        let ord = li_order.values[li].idx();
        let sup = li_supplier.values[li].idx();
        if !multi[ord] {
            let prev = first_supp[ord];
            if prev == NO_ID    { first_supp[ord] = sup; }
            else if prev != sup { multi[ord] = true; }
        }
        let is_late = li_commitdate.values[li] < li_receiptdate.values[li];
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
        let nation_id = Supplier::nation.iq().values[s];
        is_saudi[s] = Nation::name.iq().values[nation_id.idx()] == "SAUDI ARABIA";
    }
    let mut is_f_ord: Vec<bool> = vec![false; n_ord];
    for o in 0..n_ord {
        is_f_ord[o] = Order::status.iq().values[o] == "F";
    }

    let qualifying = lineitem
        // Conjunct order: late filter (cross-col) → Saudi (1 byte) → F-order
        // (1 byte) → multi (1 byte) → only-late (2 reads). Cheap-and-selective
        // first; the member check short-circuits left to right.
        .when(commitdate.and(receiptdate).filt(|(c, r)| c < r)
              .and(Lineitem::supplier.filt(move |s: Id<Supplier>| is_saudi[s.idx()]))
              .and(order.filt(move |o: Id<Order>| is_f_ord[o.idx()]))
              .and(order.filt(move |o: Id<Order>| multi[o.idx()]))
              .and(order.and(Lineitem::supplier).filt(move |(o, s): (Id<Order>, Id<Supplier>)| {
                  !late_multi[o.idx()] && late_first[o.idx()] == s.idx()
              })));
    let counts = qualifying.group_by(Lineitem::supplier).fold(0_i64, |a, _| a + 1);
    let mut rows: Vec<(Id<Supplier>, i64)> = Vec::new();
    counts.drive(|k, v| rows.push((k, v)));
    let mut named: Vec<(&str, i64)> = rows.iter()
        .map(|(s, c)| (Supplier::name.iq().values[s.idx()], *c)).collect();
    named.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    named.truncate(100);
    join_lines(named.iter().map(|(n, c)| format!("{}|{}", n, c)))
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
    let eu_ps = partsupp.when(PartSupp::supplier.nation().region().name().eq("EUROPE"));
    // ddbcheat: partkey is a dense i64 0..N, so min-per-key fits in a
    // Vec<f64> indexed by partkey — no hash, no allocation per insert. The
    // a generic `.fold()` builds an AHashMap<i64,f64> which is ~2× the
    // wall-clock for this fold's hot loop.
    let n_part = part.iq().n;
    let mut min_per_part: Vec<f64> = vec![f64::INFINITY; n_part];
    let (ps_part, ps_supplycost) = (PartSupp::part.iq(), supplycost.iq());
    (&eu_ps).drive(|ps, _| {
        let p = ps_part.values[ps.idx()].idx();
        let c = ps_supplycost.values[ps.idx()];
        if c < min_per_part[p] { min_per_part[p] = c; }
    });
    let target = (&eu_ps)
        .when(PartSupp::part.size().eq(15)
              .and(PartSupp::part.ty().filt(|s: &str| s.ends_with("BRASS")))
              .and(supplycost.and(PartSupp::part)
                   .filt(move |(c, p): (f64, Id<Part>)| c == min_per_part[p.idx()])));
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
