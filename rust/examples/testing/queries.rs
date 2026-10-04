// The test queries, from examples/testingPlan.md.
//
// Each query's prela expression is written once, in a function that takes
// the driving key set(s) and the columns it reads. The baseline passes the
// whole cached schema's keys and columns; the tree version passes the
// reduced ones built from the Sink. That way both run the exact same
// expression with the same thresholds, every exact .with() included.
use prela::engine::*;
use prela::join_tree::*;
use prela::loader::{Col, Str};
use prela::tpch_schema::{Customer, Lineitem, Nation, Order, Part, PartSupp, Region, Supplier};

use crate::helpers::*;

pub struct Query {
    pub name: &'static str,
    pub tables: Vec<&'static str>,            // files the query touches, for the baseline statistic
    pub baseline: fn() -> Vec<String>,        // formatted rows, sorted
    pub tree: fn() -> (Vec<String>, usize),   // formatted rows, sorted + rows pushed into the sink
}

pub fn all_queries() -> Vec<Query> {
    vec![
        Query {
            name: "q_customer_filter",
            tables: vec!["customer"],
            baseline: customer_filter_baseline,
            tree: customer_filter_tree,
        },
        Query {
            name: "q_asia_suppliers",
            tables: vec!["supplier", "nation", "region"],
            baseline: asia_suppliers_baseline,
            tree: asia_suppliers_tree,
        },
        Query {
            name: "q_nation_big_customers",
            tables: vec!["nation", "customer"],
            baseline: nation_big_customers_baseline,
            tree: nation_big_customers_tree,
        },
        Query {
            name: "q_nation_two_children",
            tables: vec!["nation", "supplier", "customer"],
            baseline: nation_two_children_baseline,
            tree: nation_two_children_tree,
        },
        Query {
            name: "q_lineitem_germany",
            tables: vec!["lineitem", "orders", "customer", "nation"],
            baseline: lineitem_germany_baseline,
            tree: lineitem_germany_tree,
        },
        Query {
            name: "q_partsupp_branching",
            tables: vec!["partsupp", "part", "supplier", "nation", "region"],
            baseline: partsupp_branching_baseline,
            tree: partsupp_branching_tree,
        },
    ]
}

// ===== 1. q_customer_filter: single node, no edges ==========================

const Q1_SEGMENT: &str = "BUILDING";
const Q1_HIGH: f64 = 9500.0;
const Q1_LOW: f64 = -900.0;

fn customer_filter(
    customer: impl Drive<D = Id<Customer>, R = Id<Customer>>,
    name: &Col<Customer, Str>,
    acctbal: &Col<Customer, f64>,
    mktsegment: &Col<Customer, Str>,
) -> Vec<String> {
    let mut rows = Vec::new();
    customer
        .with(mktsegment.eq(Q1_SEGMENT))
        .with(acctbal.ge(Q1_HIGH).or(acctbal.le(Q1_LOW)))
        .select(name.and(acctbal))
        .drive(|c, (n, b)| rows.push(format!("{} {n} {b}", c.0)));
    sorted(rows)
}

fn customer_filter_baseline() -> Vec<String> {
    let db = db();
    let Customer { name, acctbal, mktsegment, .. } = &db.customer;
    customer_filter(db.customer.id, name, acctbal, mktsegment)
}

fn customer_filter_tree() -> (Vec<String>, usize) {
    let acctbal_or = PredicateExpr::Or(vec![
        cmp("c_acctbal", CmpOp::GtEq, Literal::F64(Q1_HIGH)),
        cmp("c_acctbal", CmpOp::LtEq, Literal::F64(Q1_LOW)),
    ]);
    let customer = node(
        "customer",
        vec![cmp("c_mktsegment", CmpOp::Eq, Literal::Str(Q1_SEGMENT.into())), acctbal_or],
        &["c_custkey", "c_name", "c_acctbal", "c_mktsegment"],
        vec![],
    );
    let mut sink = Sink::default();
    traverse(&customer, None, &mut sink);

    let b = batches(&sink, "customer");
    let c = ids(b, "c_custkey");
    let n = domain(&c);
    let rows = customer_filter(
        key(n, &c),
        &val_col(n, &c, strs(b, "c_name")),
        &val_col(n, &c, floats(b, "c_acctbal")),
        &val_col(n, &c, strs(b, "c_mktsegment")),
    );
    (rows, sink_rows(&sink))
}

// ===== 2. q_asia_suppliers: chain of 3, parent holds the FK =================

const Q2_MIN_ACCTBAL: f64 = 5000.0;
const Q2_REGION: &str = "ASIA";

fn asia_suppliers(
    supplier: impl Drive<D = Id<Supplier>, R = Id<Supplier>>,
    s_name: &Col<Supplier, Str>,
    s_acctbal: &Col<Supplier, f64>,
    s_nation: &Col<Supplier, Id<Nation>>,
    n_region: &Col<Nation, Id<Region>>,
    r_name: &Col<Region, Str>,
) -> Vec<String> {
    let mut rows = Vec::new();
    supplier
        .with(s_acctbal.ge(Q2_MIN_ACCTBAL))
        .with(s_nation.select(n_region).select(r_name).eq(Q2_REGION))
        .select(s_name.and(s_acctbal))
        .drive(|s, (n, b)| rows.push(format!("{} {n} {b}", s.0)));
    sorted(rows)
}

fn asia_suppliers_baseline() -> Vec<String> {
    let db = db();
    let Supplier { name: s_name, acctbal: s_acctbal, nation: s_nation, .. } = &db.supplier;
    let Nation { region: n_region, .. } = &db.nation;
    let Region { name: r_name, .. } = &db.region;
    asia_suppliers(db.supplier.id, s_name, s_acctbal, s_nation, n_region, r_name)
}

fn asia_suppliers_tree() -> (Vec<String>, usize) {
    let region = node(
        "region",
        vec![cmp("r_name", CmpOp::Eq, Literal::Str(Q2_REGION.into()))],
        &["r_regionkey", "r_name"],
        vec![],
    );
    let nation = node(
        "nation",
        vec![],
        &["n_nationkey", "n_regionkey"],
        vec![edge(region, "n_regionkey", "r_regionkey")],
    );
    let supplier = node(
        "supplier",
        vec![cmp("s_acctbal", CmpOp::GtEq, Literal::F64(Q2_MIN_ACCTBAL))],
        &["s_suppkey", "s_name", "s_acctbal", "s_nationkey"],
        vec![edge(nation, "s_nationkey", "n_nationkey")],
    );
    let mut sink = Sink::default();
    traverse(&supplier, None, &mut sink);

    let sb = batches(&sink, "supplier");
    let s = ids(sb, "s_suppkey");
    let ns = domain(&s);
    let nb = batches(&sink, "nation");
    let n = ids(nb, "n_nationkey");
    let nn = domain(&n);
    let rb = batches(&sink, "region");
    let r = ids(rb, "r_regionkey");
    let nr = domain(&r);
    let rows = asia_suppliers(
        key(ns, &s),
        &val_col(ns, &s, strs(sb, "s_name")),
        &val_col(ns, &s, floats(sb, "s_acctbal")),
        &id_col(ns, &s, &ids(sb, "s_nationkey")),
        &id_col(nn, &n, &ids(nb, "n_regionkey")),
        &val_col(nr, &r, strs(rb, "r_name")),
    );
    (rows, sink_rows(&sink))
}

// ===== 3. q_nation_big_customers: child holds the FK (regression) ===========
// The existing baseline_query.rs / tree_query.rs query.

const Q3_MIN_ACCTBAL: f64 = 9900.0;

fn nation_big_customers(
    customer: impl Drive<D = Id<Customer>, R = Id<Customer>>,
    nation: impl Drive<D = Id<Nation>, R = Id<Nation>>,
    acctbal: &Col<Customer, f64>,
    c_nation: &Col<Customer, Id<Nation>>,
    n_region: &Col<Nation, Id<Region>>,
) -> Vec<String> {
    let nations: MatSet<_> = customer
        .with(acctbal.ge(Q3_MIN_ACCTBAL))
        .select(c_nation)
        .collect();
    let mut rows = Vec::new();
    nation
        .with(&nations)
        .select(n_region)
        .drive(|n, r| rows.push(format!("{} {}", n.0, r.0)));
    sorted(rows)
}

fn nation_big_customers_baseline() -> Vec<String> {
    let db = db();
    let Customer { acctbal, nation: c_nation, .. } = &db.customer;
    let Nation { region: n_region, .. } = &db.nation;
    nation_big_customers(db.customer.id, db.nation.id, acctbal, c_nation, n_region)
}

fn nation_big_customers_tree() -> (Vec<String>, usize) {
    let customer = node(
        "customer",
        vec![cmp("c_acctbal", CmpOp::GtEq, Literal::F64(Q3_MIN_ACCTBAL))],
        &["c_custkey", "c_acctbal", "c_nationkey"],
        vec![],
    );
    let nation = node(
        "nation",
        vec![],
        &["n_nationkey", "n_regionkey"],
        vec![edge(customer, "n_nationkey", "c_nationkey")],
    );
    let mut sink = Sink::default();
    traverse(&nation, None, &mut sink);

    let cb = batches(&sink, "customer");
    let c = ids(cb, "c_custkey");
    let nc = domain(&c);
    let nb = batches(&sink, "nation");
    let n = ids(nb, "n_nationkey");
    let nn = domain(&n);
    let rows = nation_big_customers(
        key(nc, &c),
        key(nn, &n),
        &val_col(nc, &c, floats(cb, "c_acctbal")),
        &id_col(nc, &c, &ids(cb, "c_nationkey")),
        &id_col(nn, &n, &ids(nb, "n_regionkey")),
    );
    (rows, sink_rows(&sink))
}

// ===== 4. q_nation_two_children: two sibling children =======================

const Q4_MIN_SUPPLIER_ACCTBAL: f64 = 9000.0;
const Q4_MIN_CUSTOMER_ACCTBAL: f64 = 9500.0;

#[allow(clippy::too_many_arguments)]
fn nation_two_children(
    supplier: impl Drive<D = Id<Supplier>, R = Id<Supplier>>,
    customer: impl Drive<D = Id<Customer>, R = Id<Customer>>,
    nation: impl Drive<D = Id<Nation>, R = Id<Nation>>,
    s_acctbal: &Col<Supplier, f64>,
    s_nation: &Col<Supplier, Id<Nation>>,
    acctbal: &Col<Customer, f64>,
    c_nation: &Col<Customer, Id<Nation>>,
    n_region: &Col<Nation, Id<Region>>,
    n_name: &Col<Nation, Str>,
) -> Vec<String> {
    let sup_nations: MatSet<_> = supplier
        .with(s_acctbal.ge(Q4_MIN_SUPPLIER_ACCTBAL))
        .select(s_nation)
        .collect();
    let cust_nations: MatSet<_> = customer
        .with(acctbal.ge(Q4_MIN_CUSTOMER_ACCTBAL))
        .select(c_nation)
        .collect();
    let mut rows = Vec::new();
    nation
        .with(&sup_nations)
        .with(&cust_nations)
        .select(n_region.and(n_name))
        .drive(|n, (r, name)| rows.push(format!("{} {} {name}", n.0, r.0)));
    sorted(rows)
}

fn nation_two_children_baseline() -> Vec<String> {
    let db = db();
    let Supplier { acctbal: s_acctbal, nation: s_nation, .. } = &db.supplier;
    let Customer { acctbal, nation: c_nation, .. } = &db.customer;
    let Nation { region: n_region, name: n_name, .. } = &db.nation;
    nation_two_children(
        db.supplier.id, db.customer.id, db.nation.id,
        s_acctbal, s_nation, acctbal, c_nation, n_region, n_name,
    )
}

fn nation_two_children_tree() -> (Vec<String>, usize) {
    let supplier = node(
        "supplier",
        vec![cmp("s_acctbal", CmpOp::GtEq, Literal::F64(Q4_MIN_SUPPLIER_ACCTBAL))],
        &["s_suppkey", "s_acctbal", "s_nationkey"],
        vec![],
    );
    let customer = node(
        "customer",
        vec![cmp("c_acctbal", CmpOp::GtEq, Literal::F64(Q4_MIN_CUSTOMER_ACCTBAL))],
        &["c_custkey", "c_acctbal", "c_nationkey"],
        vec![],
    );
    let nation = node(
        "nation",
        vec![],
        &["n_nationkey", "n_regionkey", "n_name"],
        vec![
            edge(supplier, "n_nationkey", "s_nationkey"),
            edge(customer, "n_nationkey", "c_nationkey"),
        ],
    );
    let mut sink = Sink::default();
    traverse(&nation, None, &mut sink);

    let sb = batches(&sink, "supplier");
    let s = ids(sb, "s_suppkey");
    let ns = domain(&s);
    let cb = batches(&sink, "customer");
    let c = ids(cb, "c_custkey");
    let nc = domain(&c);
    let nb = batches(&sink, "nation");
    let n = ids(nb, "n_nationkey");
    let nn = domain(&n);
    let rows = nation_two_children(
        key(ns, &s),
        key(nc, &c),
        key(nn, &n),
        &val_col(ns, &s, floats(sb, "s_acctbal")),
        &id_col(ns, &s, &ids(sb, "s_nationkey")),
        &val_col(nc, &c, floats(cb, "c_acctbal")),
        &id_col(nc, &c, &ids(cb, "c_nationkey")),
        &id_col(nn, &n, &ids(nb, "n_regionkey")),
        &val_col(nn, &n, strs(nb, "n_name")),
    );
    (rows, sink_rows(&sink))
}

// ===== 5. q_lineitem_germany: largest table, sparse keys, 4 levels ==========

const Q5_MIN_QUANTITY: f64 = 45.0;
const Q5_SHIPMODE: &str = "AIR";
const Q5_NATION: &str = "GERMANY";

#[allow(clippy::too_many_arguments)]
fn lineitem_germany(
    lineitem: impl Drive<D = Id<Lineitem>, R = Id<Lineitem>>,
    l_quantity: &Col<Lineitem, f64>,
    l_shipmode: &Col<Lineitem, Str>,
    l_extendedprice: &Col<Lineitem, f64>,
    l_discount: &Col<Lineitem, f64>,
    l_order: &Col<Lineitem, Id<Order>>,
    o_customer: &Col<Order, Id<Customer>>,
    c_nation: &Col<Customer, Id<Nation>>,
    n_name: &Col<Nation, Str>,
) -> Vec<String> {
    let mut rows = Vec::new();
    lineitem
        .with(l_quantity.ge(Q5_MIN_QUANTITY))
        .with(l_shipmode.eq(Q5_SHIPMODE))
        .with(l_order.select(o_customer).select(c_nation).select(n_name).eq(Q5_NATION))
        .select(l_extendedprice.and(l_discount))
        .drive(|l, (p, d)| rows.push(format!("{} {p} {d}", l.0)));
    sorted(rows)
}

fn lineitem_germany_baseline() -> Vec<String> {
    let db = db();
    let Lineitem {
        quantity: l_quantity,
        shipmode: l_shipmode,
        extendedprice: l_extendedprice,
        discount: l_discount,
        order: l_order,
        ..
    } = &db.lineitem;
    let Order { customer: o_customer, .. } = &db.order;
    let Customer { nation: c_nation, .. } = &db.customer;
    let Nation { name: n_name, .. } = &db.nation;
    lineitem_germany(
        db.lineitem.id,
        l_quantity, l_shipmode, l_extendedprice, l_discount,
        l_order, o_customer, c_nation, n_name,
    )
}

fn lineitem_germany_tree() -> (Vec<String>, usize) {
    let nation = node(
        "nation",
        vec![cmp("n_name", CmpOp::Eq, Literal::Str(Q5_NATION.into()))],
        &["n_nationkey", "n_name"],
        vec![],
    );
    let customer = node(
        "customer",
        vec![],
        &["c_custkey", "c_nationkey"],
        vec![edge(nation, "c_nationkey", "n_nationkey")],
    );
    let orders = node(
        "orders",
        vec![],
        &["o_orderkey", "o_custkey"],
        vec![edge(customer, "o_custkey", "c_custkey")],
    );
    let lineitem = node(
        "lineitem",
        vec![
            cmp("l_quantity", CmpOp::GtEq, Literal::F64(Q5_MIN_QUANTITY)),
            cmp("l_shipmode", CmpOp::Eq, Literal::Str(Q5_SHIPMODE.into())),
        ],
        &["l_id", "l_orderkey", "l_quantity", "l_shipmode", "l_extendedprice", "l_discount"],
        vec![edge(orders, "l_orderkey", "o_orderkey")],
    );
    let mut sink = Sink::default();
    traverse(&lineitem, None, &mut sink);

    let lb = batches(&sink, "lineitem");
    let l = ids(lb, "l_id");
    let nl = domain(&l);
    let ob = batches(&sink, "orders");
    let o = ids(ob, "o_orderkey");
    let no = domain(&o);
    let cb = batches(&sink, "customer");
    let c = ids(cb, "c_custkey");
    let nc = domain(&c);
    let nb = batches(&sink, "nation");
    let n = ids(nb, "n_nationkey");
    let nn = domain(&n);
    let rows = lineitem_germany(
        key(nl, &l),
        &val_col(nl, &l, floats(lb, "l_quantity")),
        &val_col(nl, &l, strs(lb, "l_shipmode")),
        &val_col(nl, &l, floats(lb, "l_extendedprice")),
        &val_col(nl, &l, floats(lb, "l_discount")),
        &id_col(nl, &l, &ids(lb, "l_orderkey")),
        &id_col(no, &o, &ids(ob, "o_custkey")),
        &id_col(nc, &c, &ids(cb, "c_nationkey")),
        &val_col(nn, &n, strs(nb, "n_name")),
    );
    (rows, sink_rows(&sink))
}

// ===== 6. q_partsupp_branching: branching tree, In, range via And ===========

const Q6_MAX_SUPPLYCOST: f64 = 50.0;
const Q6_BRANDS: [&str; 2] = ["Brand#12", "Brand#23"];
const Q6_MIN_SIZE: i64 = 5;
const Q6_MAX_SIZE: i64 = 20;
const Q6_REGION: &str = "EUROPE";

#[allow(clippy::too_many_arguments)]
fn partsupp_branching(
    partsupp: impl Drive<D = Id<PartSupp>, R = Id<PartSupp>>,
    ps_supplycost: &Col<PartSupp, f64>,
    ps_availqty: &Col<PartSupp, i64>,
    ps_part: &Col<PartSupp, Id<Part>>,
    ps_supplier: &Col<PartSupp, Id<Supplier>>,
    p_brand: &Col<Part, Str>,
    p_size: &Col<Part, i64>,
    s_nation: &Col<Supplier, Id<Nation>>,
    n_region: &Col<Nation, Id<Region>>,
    r_name: &Col<Region, Str>,
) -> Vec<String> {
    let mut rows = Vec::new();
    partsupp
        .with(ps_supplycost.le(Q6_MAX_SUPPLYCOST))
        .with(ps_part.with(p_brand.in_v(Q6_BRANDS.to_vec()))
                     .with(p_size.ge(Q6_MIN_SIZE))
                     .with(p_size.le(Q6_MAX_SIZE)))
        .with(ps_supplier.select(s_nation).select(n_region).select(r_name).eq(Q6_REGION))
        .select(ps_availqty.and(ps_supplycost))
        .drive(|ps, (q, c)| rows.push(format!("{} {q} {c}", ps.0)));
    sorted(rows)
}

fn partsupp_branching_baseline() -> Vec<String> {
    let db = db();
    let PartSupp {
        supplycost: ps_supplycost,
        availqty: ps_availqty,
        part: ps_part,
        supplier: ps_supplier,
        ..
    } = &db.partsupp;
    let Part { brand: p_brand, size: p_size, .. } = &db.part;
    let Supplier { nation: s_nation, .. } = &db.supplier;
    let Nation { region: n_region, .. } = &db.nation;
    let Region { name: r_name, .. } = &db.region;
    partsupp_branching(
        db.partsupp.id,
        ps_supplycost, ps_availqty, ps_part, ps_supplier,
        p_brand, p_size, s_nation, n_region, r_name,
    )
}

fn partsupp_branching_tree() -> (Vec<String>, usize) {
    let brands = Q6_BRANDS.iter().map(|b| Literal::Str(b.to_string())).collect();
    let part = node(
        "part",
        vec![
            PredicateExpr::In { col: "p_brand".into(), values: brands },
            cmp("p_size", CmpOp::GtEq, Literal::I32(Q6_MIN_SIZE as i32)),
            cmp("p_size", CmpOp::LtEq, Literal::I32(Q6_MAX_SIZE as i32)),
        ],
        &["p_partkey", "p_brand", "p_size"],
        vec![],
    );
    let region = node(
        "region",
        vec![cmp("r_name", CmpOp::Eq, Literal::Str(Q6_REGION.into()))],
        &["r_regionkey", "r_name"],
        vec![],
    );
    let nation = node(
        "nation",
        vec![],
        &["n_nationkey", "n_regionkey"],
        vec![edge(region, "n_regionkey", "r_regionkey")],
    );
    let supplier = node(
        "supplier",
        vec![],
        &["s_suppkey", "s_nationkey"],
        vec![edge(nation, "s_nationkey", "n_nationkey")],
    );
    let partsupp = node(
        "partsupp",
        vec![cmp("ps_supplycost", CmpOp::LtEq, Literal::F64(Q6_MAX_SUPPLYCOST))],
        &["ps_id", "ps_partkey", "ps_suppkey", "ps_availqty", "ps_supplycost"],
        vec![
            edge(part, "ps_partkey", "p_partkey"),
            edge(supplier, "ps_suppkey", "s_suppkey"),
        ],
    );
    let mut sink = Sink::default();
    traverse(&partsupp, None, &mut sink);

    let psb = batches(&sink, "partsupp");
    let ps = ids(psb, "ps_id");
    let nps = domain(&ps);
    let pb = batches(&sink, "part");
    let p = ids(pb, "p_partkey");
    let np = domain(&p);
    let sb = batches(&sink, "supplier");
    let s = ids(sb, "s_suppkey");
    let ns = domain(&s);
    let nb = batches(&sink, "nation");
    let n = ids(nb, "n_nationkey");
    let nn = domain(&n);
    let rb = batches(&sink, "region");
    let r = ids(rb, "r_regionkey");
    let nr = domain(&r);
    let rows = partsupp_branching(
        key(nps, &ps),
        &val_col(nps, &ps, floats(psb, "ps_supplycost")),
        &val_col(nps, &ps, ints(psb, "ps_availqty")),
        &id_col(nps, &ps, &ids(psb, "ps_partkey")),
        &id_col(nps, &ps, &ids(psb, "ps_suppkey")),
        &val_col(np, &p, strs(pb, "p_brand")),
        &val_col(np, &p, ints(pb, "p_size")),
        &id_col(ns, &s, &ids(sb, "s_nationkey")),
        &id_col(nn, &n, &ids(nb, "n_regionkey")),
        &val_col(nr, &r, strs(rb, "r_name")),
    );
    (rows, sink_rows(&sink))
}
