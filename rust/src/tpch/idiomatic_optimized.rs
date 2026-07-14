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
fn q1() -> String {
    let mut rows: Vec<(usize, (f64, f64, f64, f64, f64, i64))> = Vec::new();

    lineitem
        .with(shipdate.le(19980902))
        .group_by(
            returnflag
                .and(Lineitem::status)
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
            rf,
            ls,
            f(*qty),
            f(*ext),
            f(*dp),
            f(*chg),
            f(qty / nf),
            f(ext / nf),
            f(di / nf),
            n
        )
    }))
}

// Optimizations:
// We replace the `fold` over the `part -> cost` relation with a `dense_fold`.
fn q2() -> String {
    let mut rows: Vec<(f64, &str, &str, Id<Part>, &str, &str, &str, &str)> = Vec::new();

    let min_per_part = partsupp
        .with(PartSupp::supplier.nation().region().eq("EUROPE"))
        .group_by(PartSupp::part)
        .supplycost()
        .dense_fold(part.iq().n, f64::INFINITY, |a, c| if c < a { c } else { a });

    partsupp
        .with(PartSupp::supplier.nation().region().eq("EUROPE"))
        .with(
            PartSupp::part
                .with(size.eq(15).and(ty.filt(|s: &str| s.ends_with("BRASS"))))
                .and(
                    supplycost
                        .and(PartSupp::part.select(&min_per_part))
                        .filt(|(c, m)| c == m),
                ),
        )
        .select(
            PartSupp::supplier
                .select(
                    Supplier::acctbal
                        .and(Supplier::name)
                        .and(Supplier::nation.name())
                        .and(Supplier::address)
                        .and(Supplier::phone)
                        .and(Supplier::comment),
                )
                .and(PartSupp::part)
                .and(PartSupp::part.mfgr()),
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
            f(r.0),
            r.1,
            r.2,
            r.3.idx() + 1,
            r.4,
            r.5,
            r.6,
            r.7
        )
    }))
}

// Optimizations:
// Use `Bitset` instead of `MatSet` to materialize late orders.
fn q4() -> String {
    let mut rows: Vec<(&str, i64)> = Vec::new();
    orders
        .with(date.during(19930701, 19931001))
        .with(Bitset::over(
            orders,
            lineitem
                .with(commitdate.and(receiptdate).filt(|(c, r)| c < r))
                .order(),
        ))
        .priority()
        .inv()
        .fold(0_i64, |a, _| a + 1)
        .drive(|k, v| rows.push((k, v)));
    rows.sort_by(|a, b| a.0.cmp(b.0));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

// Optimizations:
// Materialize green parts into a `Bitset` instead of a `HashIdx`.
// Materialize nation names after the `fold`.
fn q9() -> String {
    let mut rows: Vec<((&str, i64), f64)> = Vec::new();

    let green_parts = Bitset::over(
        part,
        &part.with(Part::name.filt(|n: &str| n.contains("green"))),
    );

    let sc: HashIdx<_, _> = partsupp
        .group_by(PartSupp::part.select(&green_parts).and(PartSupp::supplier))
        .supplycost()
        .collect();

    lineitem
        .with(Lineitem::part.select(&green_parts))
        .group_by(
            Lineitem::supplier
                .nation()
                .and(order.date().map(|d: i64| d / 10_000)),
        )
        .select(
            extendedprice
                .and(discount)
                .and(quantity)
                .and(Lineitem::part.and(Lineitem::supplier).select(&sc)),
        )
        .fold(0.0_f64, |a, (((e, dc), q), cost)| {
            a + e * (1.0 - dc) - cost * q
        })
        .drive(|(id, d), v| rows.push(((Nation::name.iq().values[id.idx()], d), v)));

    rows.sort_by(|((n1, y1), _), ((n2, y2), _)| n1.cmp(n2).then(y2.cmp(y1)));
    join_lines(
        rows.iter()
            .map(|((n, y), v)| format!("{}|{}|{}", n, y, f(*v))),
    )
}

// Optimizations
// Order conjuncts by selectivity.
fn q12() -> String {
    let result = lineitem
        .with(
            receiptdate
                .during(19940101, 19950101)
                .and(shipmode.is_in(["MAIL", "SHIP"]))
                .and(shipdate.and(commitdate).filt(|(s, c)| s < c))
                .and(commitdate.and(receiptdate).filt(|(c, r)| c < r)),
        )
        .group_by(shipmode)
        .order()
        .priority()
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
fn q13() -> String {
    use memchr::memmem;
    let f_special = memmem::Finder::new("special");
    let mut dist: HashMap<i64, i64> = HashMap::new();
    let mut n_with = 0i64;

    orders
        .with(
            Order::customer
                .filt(|c| c != Dense::NONE)
                .and(
                    Order::comment.filt(move |c: &str| match f_special.find(c.as_bytes()) {
                        Some(p) => !c[p + "special".len()..].contains("requests"),
                        None => true,
                    }),
                ),
        )
        .group_by(Order::customer)
        .dense_fold(customer.iq().n, 0_i64, |a, _| a + 1)
        .drive(|_, c| {
            *dist.entry(c).or_insert(0) += 1;
            n_with += 1;
        });
    dist.insert(0, customer.iq().n as i64 - n_with);

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
fn q17() -> String {
    let tpp: HashIdx<_, _> = lineitem
        .group_by(
            Lineitem::part // restrict to qual parts
                .with(Bitset::over(
                    part,
                    &part.with(brand.eq("Brand#23").and(container.eq("MED BOX"))),
                )),
        )
        .quantity()
        .fold((0.0_f64, 0_i64), |(s, n), q| (s + q, n + 1))
        .map(|(s, n)| 0.2 * s / n as f64)
        .collect();

    let sum = lineitem
        .with(
            Lineitem::part
                .with(Bitset::over(
                    part,
                    &part.with(brand.eq("Brand#23").and(container.eq("MED BOX"))),
                ))
                .and(
                    quantity
                        .and(Lineitem::part.select(&tpp))
                        .filt(|(q, t)| q < t),
                ),
        )
        .extendedprice()
        .unwrap_fold(0.0_f64, |a, e| a + e);

    f(sum / 7.0)
}

// Optimizations:
// Use `dense_fold` instead of `fold` on the dense order keys.
fn q18() -> String {
    let mut rows: Vec<(Id<Order>, (f64, ((f64, i64), (&str, Id<Customer>))))> = Vec::new();

    lineitem
        .group_by(order)
        .quantity()
        .dense_fold(orders.iq().n, 0.0_f64, |a, q| a + q)
        .gt(300.0)
        .and(
            totalprice
                .and(date)
                .and(Order::customer.name().and(Order::customer)),
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
            name,
            cust.idx() + 1,
            o.idx() + 1,
            fmt_yyyymmdd(*dt),
            f(*tp),
            f(*sum_q)
        )
    }))
}

// Optimizations:
// Cheap `dense_fold` over state tracking whether 0, 1, or more suppliers seen per order instead
// of expensive `group_by` + `count`.
// Single pass over data to capture both (i) orders with > 1 suppliers and (ii) orders with exactly 1 late supplier.
// `select` into `Bitset` instead of `with` to restrict to SA suppliers.
fn q21() -> String {
    let mut rows: Vec<(&str, i64)> = Vec::new();

    // track overall suppliers and late suppliers
    let track = |((first, multi), (first_late, multi_late)): (
        (Option<Id<Supplier>>, bool),
        (Option<Id<Supplier>>, bool),
    ),
                 ((s, c), r): ((Id<Supplier>, i64), i64)| {
        let t1 = match first {
            None => (Some(s), multi),
            Some(f) if f != s => (first, true),
            _ => (first, multi),
        };
        // if order is late, update late suppliers state
        let t2 = if c < r {
            match first_late {
                None => (Some(s), multi_late),
                Some(f) if f != s => (first_late, true),
                _ => (first_late, multi_late),
            }
        } else {
            (first_late, multi_late)
        };
        (t1, t2)
    };

    let state = lineitem
        .group_by(order)
        .select(Lineitem::supplier
           .and(commitdate)
           .and(receiptdate))
        .dense_fold(orders.iq().n, ((None, false), (None, false)), track);

    lineitem
        .with(commitdate.and(receiptdate).filt(|(c, r)| c < r))
        .with(
            Lineitem::supplier
                .select(Bitset::over(
                    supplier,
                    &supplier.with(Supplier::nation.eq("SAUDI ARABIA")),
                ))
                .and(order.select(Order::status.eq("F").and(
                    // order has > 1 supp <-> `m_late`
                    state.filt(|((_, m), (f_late, m_late))| m 
                    && 
                    // order has 1 late supp <-> `f_late.is_some() && !m_late`
                    (f_late.is_some() && !m_late)),
                ))),
        )
        .group_by(Lineitem::supplier)
        .fold(0_i64, |a, _| a + 1)
        .and(Supplier::name)
        .drive(|_, (c, n)| rows.push((n, c)));

    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    rows.truncate(100);
    join_lines(rows.iter().map(|(n, c)| format!("{}|{}", n, c)))
}

// Optimizations:
// `Bitset` instead of `MatSet` to materialize customers with orders.
fn q22() -> String {
    let prefix = Customer::phone.map(|p: &str| &p[..2]);
    let codes = ["13", "31", "23", "29", "30", "18", "17"];

    let (sum_p, cnt_p) = customer
        .with((&prefix).is_in(codes))
        .with(Customer::acctbal.gt(0.0))
        .acctbal()
        .unwrap_fold((0.0_f64, 0_i64), |(s, n), v| (s + v, n + 1));
    let avg = sum_p / cnt_p as f64;

    let mut rows: Vec<(&str, (i64, f64))> = Vec::new();

    customer
        .with((&prefix).is_in(codes))
        .with(Customer::acctbal.gt(avg))
        .minus(Bitset::over(customer, Order::customer))
        .group_by(&prefix)
        .acctbal()
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
