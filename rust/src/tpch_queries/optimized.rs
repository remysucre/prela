use super::common::{f, fmt_yyyymmdd, join_lines, with_overrides};
use crate::engine::*;
use crate::tpch_schema::*;
use std::collections::HashMap;

pub fn queries() -> Vec<super::Entry> {
    with_overrides(&[
        ("1", q1),
        ("2", q2),
        ("4", q4),
        ("9", q9),
        ("12", q12),
        ("13", q13),
        ("17", q17),
        ("18", q18),
        ("21", q21),
        ("22", q22),
    ])
}

// Optimizations:
// Instesd of grouping by a `(returnflag: str, status: str)` tuple, we encode the tuples into a `usize`
// before grouping, thus reducing the required allocations.
// In addition, since the maximal value of this `usize` is 281, we can treat the `group_by` index as
// a dense key over a universe of size 282 and use a `dense_fold` instead of a `fold`.
fn q1(db: &'static Tpch) -> String {
    let Lineitem {
        shipdate, returnflag, status: l_status,
        quantity, extendedprice, discount, tax, ..
    } = &db.lineitem;

    let mut rows: Vec<(usize, (f64, f64, f64, f64, f64, i64))> = Vec::new();

    db.lineitem
        .with(shipdate.le(19980902))
        .group_by(
            returnflag
                .and(l_status)
                .map(|(rf, ls): (&str, &str)| {
                    ((rf.as_bytes()[0].wrapping_sub(b'A') as usize) << 4)
                        | (ls.as_bytes()[0].wrapping_sub(b'F') as usize)
                }),
        )
        .select(quantity.and(extendedprice).and(discount).and(tax))
        .dense_fold(
            282,
            (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0_i64),
            |(qty, ext, di, dp, chg, n), (((q, e), dc), tx)| {
                let dp_inc = e * (1.0 - dc);
                let chg_inc = dp_inc * (1.0 + tx);
                (qty + q, ext + e, di + dc, dp + dp_inc, chg + chg_inc, n + 1)
            },
        )
        .drive(|k, v| rows.push((k, v)));

    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, (qty, ext, di, dp, chg, n))| {
        let rf = (((*k >> 4) as u8).wrapping_add(b'A')) as char;
        let ls = (((*k & 0xF) as u8).wrapping_add(b'F')) as char;
        let nf = *n as f64;
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            rf, ls, f(*qty), f(*ext), f(*dp), f(*chg),
            f(qty / nf), f(ext / nf), f(di / nf), n
        )
    }))
}

// Optimizations:
// We replace the `fold` over the `part -> cost` relation with a `dense_fold`.
fn q2(db: &'static Tpch) -> String {
    let PartSupp { part: ps_part, supplier: ps_supplier, supplycost, .. } = &db.partsupp;
    let Part { size, ty: p_ty, mfgr, .. } = &db.part;
    let Supplier { acctbal: s_acctbal, name: s_name, nation: s_nation,
                   address: s_address, phone: s_phone, comment: s_comment, .. } = &db.supplier;
    let Nation { name: n_name, region: n_region, .. } = &db.nation;
    let Region { name: r_name, .. } = &db.region;

    let mut rows: Vec<(f64, &str, &str, Id<Part>, &str, &str, &str, &str)> = Vec::new();

    let eu = || ps_supplier.select(s_nation).select(n_region).select(r_name).eq("EUROPE");

    let min_per_part = db.partsupp
        .with(eu())
        .group_by(ps_part)
        .select(supplycost)
        .dense_fold(db.part.key.n, f64::INFINITY, |a, c| if c < a { c } else { a });

    db.partsupp
        .with(eu())
        .with(
            ps_part
                .with(size.eq(15).and(p_ty.filt(|s: &str| s.ends_with("BRASS"))))
                .and(
                    supplycost
                        .and(ps_part.select(&min_per_part))
                        .filt(|(c, m)| c == m),
                ),
        )
        .select(
            ps_supplier
                .select(
                    s_acctbal
                        .and(s_name)
                        .and(s_nation.select(n_name))
                        .and(s_address)
                        .and(s_phone)
                        .and(s_comment),
                )
                .and(ps_part)
                .and(ps_part.select(mfgr)),
        )
        .drive(
            |_, (((((((acct, sname), nname), addr), phone), comm), pkey), mfg)| {
                rows.push((acct, sname, nname, pkey, mfg, addr, phone, comm))
            },
        );
    rows.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap()
            .then(a.2.cmp(b.2))
            .then(a.1.cmp(b.1))
            .then(a.3.cmp(&b.3))
    });
    rows.truncate(100);
    // natural partkey = internal id + 1
    join_lines(rows.iter().map(|r| {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            f(r.0), r.1, r.2, r.3.idx() + 1, r.4, r.5, r.6, r.7
        )
    }))
}

// Optimizations:
// Use `Bitset` instead of `MatSet` to materialize late orders.
fn q4(db: &'static Tpch) -> String {
    let Lineitem { commitdate, receiptdate, order: l_order, .. } = &db.lineitem;
    let Order { date: o_date, priority, .. } = &db.order;

    let mut rows: Vec<(&str, i64)> = Vec::new();
    db.order
        .with(o_date.during(19930701, 19931001))
        .with(Bitset::over(
            &db.order,
            db.lineitem
                .with(commitdate.and(receiptdate).filt(|(c, r)| c < r))
                .select(l_order),
        ))
        .select(priority)
        .inv()
        .fold(0_i64, |a, _| a + 1)
        .drive(|k, v| rows.push((k, v)));
    rows.sort_by(|a, b| a.0.cmp(b.0));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// Optimizations:
// Materialize green parts into a `Bitset` instead of a `HashIdx`.
// Materialize nation names after the `fold`.
fn q9(db: &'static Tpch) -> String {
    let Lineitem { part: l_part, supplier: l_supplier, order: l_order,
                   extendedprice, discount, quantity, .. } = &db.lineitem;
    let PartSupp { part: ps_part, supplier: ps_supplier, supplycost, .. } = &db.partsupp;
    let Part { name: p_name, .. } = &db.part;
    let Order { date: o_date, .. } = &db.order;
    let Supplier { nation: s_nation, .. } = &db.supplier;

    let mut rows: Vec<((&str, i64), f64)> = Vec::new();

    let green_parts = Bitset::over(
        &db.part,
        &db.part.with(p_name.filt(|n: &str| n.contains("green"))),
    );

    let sc: HashIdx<_, _> = db.partsupp
        .group_by(ps_part.select(&green_parts).and(ps_supplier))
        .select(supplycost)
        .collect();

    db.lineitem
        .with(l_part.select(&green_parts))
        .group_by(
            l_supplier
                .select(s_nation)
                .and(l_order.select(o_date).map(|d: i64| d / 10_000)),
        )
        .select(
            extendedprice
                .and(discount)
                .and(quantity)
                .and(l_part.and(l_supplier).select(&sc)),
        )
        .fold(0.0_f64, |a, (((e, dc), q), cost)| {
            a + e * (1.0 - dc) - cost * q
        })
        .drive(|(id, d), v| rows.push(((db.nation.name.v[id.idx()], d), v)));

    rows.sort_by(|((n1, y1), _), ((n2, y2), _)| n1.cmp(n2).then(y2.cmp(y1)));
    join_lines(
        rows.iter()
            .map(|((n, y), v)| format!("{}|{}|{}", n, y, f(*v))),
    )
}

// Optimizations
// Order conjuncts by selectivity.
fn q12(db: &'static Tpch) -> String {
    let Lineitem { receiptdate, shipmode, shipdate, commitdate, order: l_order, .. } = &db.lineitem;
    let Order { priority, .. } = &db.order;

    let result = db.lineitem
        .with(
            receiptdate
                .during(19940101, 19950101)
                .and(shipmode.is_in(["MAIL", "SHIP"]))
                .and(shipdate.and(commitdate).filt(|(s, c)| s < c))
                .and(commitdate.and(receiptdate).filt(|(c, r)| c < r)),
        )
        .group_by(shipmode)
        .select(l_order)
        .select(priority)
        .fold((0_i64, 0_i64), |(h, l), pr| {
            let is_high = pr == "1-URGENT" || pr == "2-HIGH";
            if is_high { (h + 1, l) } else { (h, l + 1) }
        });

    let mut rows: Vec<(&str, (i64, i64))> = Vec::new();
    result.drive(|k, v| rows.push((k, v)));
    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, (h, l))| format!("{}|{}|{}", k, h, l)))
}

// Optimizations:
// Use `memchr` instead of default regex engine.
// Skip `NONE` customers, so use `dense_fold` instead of `dense_fold_outer`. Recover bin
// 0 as the difference between total number of customers and customers with special orders.
fn q13(db: &'static Tpch) -> String {
    use memchr::memmem;
    let Order { customer: o_customer, comment: o_comment, .. } = &db.order;

    let f_special = memmem::Finder::new("special");
    let mut dist: HashMap<i64, i64> = HashMap::new();
    let mut n_with = 0i64;
    let n_cust = db.customer.key.n;

    db.order
        .with(
            o_customer
                .filt(|c| c != Dense::NONE)
                .and(
                    o_comment.filt(move |c: &str| match f_special.find(c.as_bytes()) {
                        Some(p) => !c[p + "special".len()..].contains("requests"),
                        None => true,
                    }),
                ),
        )
        .group_by(o_customer)
        .dense_fold(n_cust, 0_i64, |a, _| a + 1)
        .drive(|_, c| {
            *dist.entry(c).or_insert(0) += 1;
            n_with += 1;
        });
    dist.insert(0, n_cust as i64 - n_with);

    let mut rows: Vec<_> = dist.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| b.0.cmp(a.0)));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// Optimizations:
// Restrict by qualifying parts before materializing the threshold `HashIdx`.
// Use `Bitset` over qualifying parts to represent qualifying line items.
// NOTE: materializing the `Bitset` once into `qual_parts` is somehow slower than materializing
// twice as needed.
// NOTE: materializing the `Bitset` is somehow faster than composing/restricting to the right
// attributes, as the original idiomatic query does.
fn q17(db: &'static Tpch) -> String {
    let Lineitem { part: l_part, quantity, extendedprice, .. } = &db.lineitem;
    let Part { brand, container, .. } = &db.part;

    let qual_parts = || {
        Bitset::over(
            &db.part,
            &db.part.with(brand.eq("Brand#23").and(container.eq("MED BOX"))),
        )
    };

    let tpp: HashIdx<_, _> = db.lineitem
        .group_by(l_part.with(qual_parts())) // restrict to qual parts
        .select(quantity)
        .fold((0.0_f64, 0_i64), |(s, n), q| (s + q, n + 1))
        .map(|(s, n)| 0.2 * s / n as f64)
        .collect();

    let sum = db.lineitem
        .with(
            l_part
                .with(qual_parts())
                .and(
                    quantity
                        .and(l_part.select(&tpp))
                        .filt(|(q, t)| q < t),
                ),
        )
        .select(extendedprice)
        .unwrap_fold(0.0_f64, |a, e| a + e);

    f(sum / 7.0)
}

// Optimizations:
// Use `dense_fold` instead of `fold` on the dense order keys.
fn q18(db: &'static Tpch) -> String {
    let Lineitem { order: l_order, quantity, .. } = &db.lineitem;
    let Order { totalprice, date: o_date, customer: o_customer, .. } = &db.order;
    let Customer { name: c_name, .. } = &db.customer;

    let mut rows: Vec<(Id<Order>, (f64, ((f64, i64), (&str, Id<Customer>))))> = Vec::new();

    db.lineitem
        .group_by(l_order)
        .select(quantity)
        .dense_fold(db.order.key.n, 0.0_f64, |a, q| a + q)
        .gt(300.0)
        .and(
            totalprice
                .and(o_date)
                .and(o_customer.select(c_name).and(o_customer)),
        )
        .drive(|k, v| rows.push((k, v)));

    rows.sort_by(|(_, (_, ((tp1, dt1), _))), (_, (_, ((tp2, dt2), _)))| {
        tp2.partial_cmp(tp1).unwrap().then(dt1.cmp(dt2))
    });
    rows.truncate(100);
    join_lines(rows.iter().map(|(o, (sum_q, ((tp, dt), (name, cust))))| {
        // natural custkey / orderkey = internal id + 1
        format!(
            "{}|{}|{}|{}|{}|{}",
            name, cust.idx() + 1, o.idx() + 1,
            fmt_yyyymmdd(*dt), f(*tp), f(*sum_q)
        )
    }))
}

// Per-order supplier state for q21, packed into one `u32`: the low 31 bits
// hold `supplier id + 1` (0 = "no supplier seen yet"), the top bit is the
// "more than one distinct supplier" flag. Two of these are all q21 needs per
// order, so its `dense_fold` state is 8 bytes rather than the 48 that the
// natural `((Option<Id<Supplier>>, bool), (Option<Id<Supplier>>, bool))`
// tuple occupies.
const Q21_MULTI: u32 = 1 << 31;
const Q21_ID: u32 = !Q21_MULTI;

#[inline(always)]
fn q21_note(slot: u32, s: Id<Supplier>) -> u32 {
    debug_assert!(s.0 + 1 < Q21_MULTI as usize, "supplier id needs the flag bit");
    let id = s.0 as u32 + 1;
    if slot == 0 {
        id
    } else if slot & Q21_ID != id {
        slot | Q21_MULTI
    } else {
        slot
    }
}

// Optimizations:
// Cheap `dense_fold` over state tracking whether 0, 1, or more suppliers seen per order instead
// of expensive `group_by` + `count`.
// Single pass over data to capture both (i) orders with > 1 suppliers and (ii) orders with exactly 1 late supplier.
// `select` into `Bitset` instead of `with` to restrict to SA suppliers.
// The fold state is bit-packed (see `q21_note`): `orders` is a *sparse*
// universe — 6M slots for 1.5M live orders at SF=1 — so the `dense_fold`
// array is 6M × size_of::<S>() regardless of how many orders exist. At the
// natural 48-byte tuple that is a 288 MB array to zero and stream; at 8
// bytes it is 48 MB.
// The scan leads with the SAUDI ARABIA supplier bitset instead of the date
// comparison, so the commit/receipt columns are only read for the ~4% of
// lineitems whose supplier can possibly qualify (`Restrict` short-circuits
// left to right).
fn q21(db: &'static Tpch) -> String {
    let Lineitem { order: l_order, supplier: l_supplier, commitdate, receiptdate, .. } = &db.lineitem;
    let Order { status: o_status, .. } = &db.order;
    let Supplier { name: s_name, nation: s_nation, .. } = &db.supplier;
    let Nation { name: n_name, .. } = &db.nation;

    let mut rows: Vec<(&str, i64)> = Vec::new();

    // track overall suppliers and late suppliers
    let track = |(all, late): (u32, u32), ((s, c), r): ((Id<Supplier>, i64), i64)| {
        // if the line is late, update the late-supplier state too
        (q21_note(all, s), if c < r { q21_note(late, s) } else { late })
    };

    let state = db.lineitem
        .group_by(l_order)
        .select(l_supplier
           .and(commitdate)
           .and(receiptdate))
        .dense_fold(db.order.key.n, (0, 0), track);

    let saudi = Bitset::over(
        &db.supplier,
        &db.supplier.with(s_nation.select(n_name).eq("SAUDI ARABIA")),
    );

    db.lineitem
        .with(l_supplier.select(&saudi))
        .with(commitdate.and(receiptdate).filt(|(c, r)| c < r))
        .with(l_order.select(o_status.eq("F").and(
            state.filt(|(all, late)| {
                // order has > 1 supplier, and exactly one distinct late one
                // (which must be this line's, since this line is late)
                all & Q21_MULTI != 0 && late != 0 && late & Q21_MULTI == 0
            }),
        )))
        .group_by(l_supplier)
        .fold(0_i64, |a, _| a + 1)
        .and(s_name)
        .drive(|_, (c, n)| rows.push((n, c)));

    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    rows.truncate(100);
    join_lines(rows.iter().map(|(n, c)| format!("{}|{}", n, c)))
}

// Optimizations:
// `Bitset` instead of `MatSet` to materialize customers with orders.
fn q22(db: &'static Tpch) -> String {
    let Customer { phone: c_phone, acctbal: c_acctbal, .. } = &db.customer;
    let Order { customer: o_customer, .. } = &db.order;

    let prefix = c_phone.map(|p: &str| &p[..2]);
    let codes = ["13", "31", "23", "29", "30", "18", "17"];

    let (sum_p, cnt_p) = db.customer
        .with((&prefix).is_in(codes))
        .with(c_acctbal.gt(0.0))
        .select(c_acctbal)
        .unwrap_fold((0.0_f64, 0_i64), |(s, n), v| (s + v, n + 1));
    let avg = sum_p / cnt_p as f64;

    let mut rows: Vec<(&str, (i64, f64))> = Vec::new();

    db.customer
        .with((&prefix).is_in(codes))
        .with(c_acctbal.gt(avg))
        .minus(Bitset::over(&db.customer, o_customer))
        .group_by(&prefix)
        .select(c_acctbal)
        .fold((0_i64, 0.0_f64), |(cnt, sm), ab| {
            (cnt + 1, sm + ab)
        })
        .drive(|k, v| rows.push((k, v)));

    rows.sort_by_key(|r| r.0);
    join_lines(
        rows.iter()
            .map(|(k, (cnt, sm))| format!("{}|{}|{}", k, cnt, f(*sm))),
    )
}
