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
fn q1() -> String {
    let grouped = lineitem
        .with(shipdate.le(19980902))
        .select(quantity.and(extendedprice).and(discount).and(tax))
        .group_by(
            returnflag
                .and(Lineitem::status)
                .map(|(rf, ls): (&str, &str)| {
                    ((rf.as_bytes()[0].wrapping_sub(b'A') as usize) << 4)
                        | (ls.as_bytes()[0].wrapping_sub(b'F') as usize)
                }),
        )
        .dense_fold(
            282,
            (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0_i64),
            |(qty, ext, di, dp, chg, n), (((q, e), dc), tx)| {
                let dp_inc = e * (1.0 - dc);
                let chg_inc = dp_inc * (1.0 + tx);
                (qty + q, ext + e, di + dc, dp + dp_inc, chg + chg_inc, n + 1)
            },
        );

    let mut rows: Vec<(usize, (f64, f64, f64, f64, f64, i64))> = Vec::new();
    grouped.drive(|k, v| rows.push((k, v)));
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

fn q2() -> String {
    let mut rows: Vec<(f64, &str, &str, Id<Part>, &str, &str, &str, &str)> = Vec::new();

    let min_per_part = partsupp
        .with(PartSupp::supplier.nation().region().eq("EUROPE"))
        .supplycost()
        .group_by(PartSupp::part)
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
fn q9() -> String {
    let mut rows: Vec<((Id<Nation>, i64), f64)> = Vec::new();

    let green_parts = Bitset::over(
        part,
        &part.with(Part::name.filt(|n: &str| n.contains("green"))),
    );

    let sc: HashIdx<_, _> = supplycost
        .group_by(PartSupp::part.select(&green_parts).and(PartSupp::supplier))
        .collect();

    let result = lineitem
        .with(Lineitem::part.select(&green_parts))
        .select(
            extendedprice
                .and(discount)
                .and(quantity)
                .and(Lineitem::part.and(Lineitem::supplier).select(&sc)),
        )
        .group_by(
            Lineitem::supplier
                .nation()
                .and(order.date().map(|d: i64| d / 10_000)),
        )
        .fold(0.0_f64, |a, (((e, dc), q), cost)| {
            a + e * (1.0 - dc) - cost * q
        });

    result.drive(|k, v| rows.push((k, v)));
    rows.sort_by(|a, b| {
        let na = Nation::name.iq().values[a.0.0.idx()];
        let nb = Nation::name.iq().values[b.0.0.idx()];
        na.cmp(nb).then_with(|| b.0.1.cmp(&a.0.1))
    });
    join_lines(
        rows.iter()
            .map(|(k, v)| format!("{}|{}|{}", Nation::name.iq().values[k.0.idx()], k.1, f(*v))),
    )
}
fn q12() -> String {
    let result = lineitem
        .with(
            receiptdate
                .during(19940101, 19950101)
                .and(shipmode.is_in(["MAIL", "SHIP"]))
                .and(shipdate.and(commitdate).filt(|(s, c)| s < c))
                .and(commitdate.and(receiptdate).filt(|(c, r)| c < r)),
        )
        .select(order.priority())
        .group_by(shipmode)
        .fold((0_i64, 0_i64), |(h, l), pr| {
            let is_high = pr == "1-URGENT" || pr == "2-HIGH";
            if is_high { (h + 1, l) } else { (h, l + 1) }
        });

    let mut rows: Vec<(&str, (i64, i64))> = Vec::new();
    result.drive(|k, v| rows.push((k, v)));
    rows.sort_by_key(|r| r.0);
    join_lines(rows.iter().map(|(k, (h, l))| format!("{}|{}|{}", k, h, l)))
}

fn q13() -> String {
    use memchr::memmem;
    let f_special = memmem::Finder::new("special");
    let mut dist: HashMap<i64, i64> = HashMap::new();
    let mut n_with = 0i64;

    orders
        // restrict to non-NONE customers with non-special requests
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
        // count orders per customer
        .group_by(Order::customer)
        .dense_fold(customer.iq().n, 0_i64, |a, _| a + 1)
        // build histogram
        .drive(|_, c| {
            *dist.entry(c).or_insert(0) += 1;
            n_with += 1; // count nonzero bins
        });

    // add bin 0
    dist.insert(0, customer.iq().n as i64 - n_with);

    let mut rows: Vec<_> = dist.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| b.0.cmp(a.0)));
    join_lines(rows.iter().map(|(k, v)| format!("{}|{}", k, v)))
}

fn q17() -> String {
    let tpp: HashIdx<_, _> = quantity
        .group_by(
            Lineitem::part // restrict to qual parts
                .with(Bitset::over(
                    part,
                    &part.with(brand.eq("Brand#23").and(container.eq("MED BOX"))),
                )),
        )
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
        .select(extendedprice)
        .unwrap_fold(0.0_f64, |a, e| a + e);

    f(sum / 7.0)
}

fn q18() -> String {
    let mut rows: Vec<(Id<Order>, f64)> = Vec::new();
    quantity
        .group_by(order)
        .dense_fold(orders.iq().n, 0.0_f64, |a, q| a + q)
        .gt(300.0)
        .drive(|k, v| rows.push((k, v)));

    rows.sort_by(|a, b| {
        let (oa, ob) = (a.0.idx(), b.0.idx());
        totalprice.iq().values[ob]
            .partial_cmp(&totalprice.iq().values[oa])
            .unwrap()
            .then_with(|| date.iq().values[oa].cmp(&date.iq().values[ob]))
    });
    rows.truncate(100);

    join_lines(rows.iter().map(|(o, sum_q)| {
        let oi = o.idx();
        let cu = Order::customer.iq().values[oi];
        let cui = cu.idx();
        // natural custkey / orderkey = internal id + 1
        format!(
            "{}|{}|{}|{}|{}|{}",
            Customer::name.iq().values[cui],
            cui + 1,
            oi + 1,
            fmt_yyyymmdd(date.iq().values[oi]),
            f(totalprice.iq().values[oi]),
            f(*sum_q)
        )
    }))
}
fn q21() -> String {
    let mut rows: Vec<(&str, i64)> = Vec::new();

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

    let state = Lineitem::supplier
        .and(commitdate)
        .and(receiptdate)
        .group_by(order)
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
                    state.filt(|((_, m), (f_late, m_late))| m && (f_late.is_some() && !m_late)),
                ))),
        )
        .group_by(Lineitem::supplier)
        .fold(0_i64, |a, _| a + 1)
        .drive(|k, v| rows.push((Supplier::name.iq().values[k.idx()], v)));
    // TODO: may be able to get names more efficiently here?

    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    rows.truncate(100);
    join_lines(rows.iter().map(|(n, c)| format!("{}|{}", n, c)))
}

fn q22() -> String {
    let prefix = Customer::phone.map(|p: &str| &p[..2]);
    let codes = ["13", "31", "23", "29", "30", "18", "17"];

    let (sum_p, cnt_p) = customer
        .with((&prefix).is_in(codes))
        .with(Customer::acctbal.gt(0.0))
        .select(Customer::acctbal)
        .unwrap_fold((0.0_f64, 0_i64), |(s, n), v| (s + v, n + 1));
    let avg = sum_p / cnt_p as f64;

    let mut rows: Vec<(&str, (i64, f64))> = Vec::new();

    customer
        .with((&prefix).is_in(codes))
        .with(Customer::acctbal.gt(avg))
        .minus(Bitset::over(customer, Order::customer))
        .group_by(&prefix)
        .fold((0_i64, 0.0_f64), |(cnt, sm), c| {
            let ab = Customer::acctbal.iq().values[c.idx()];
            (cnt + 1, sm + ab)
        })
        .drive(|k, v| rows.push((k, v)));
    rows.sort_by_key(|r| r.0);
    join_lines(
        rows.iter()
            .map(|(k, (cnt, sm))| format!("{}|{}|{}", k, cnt, f(*sm))),
    )
}
