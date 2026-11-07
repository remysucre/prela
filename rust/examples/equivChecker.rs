// Run with: cargo run --release --features regen --example equivChecker
//
// Checks, on data/tpch/parquet, whether the stump loader (LeftRightLoader.rs)
// makes a `.with` safe to DROP from the in-memory plan. For each query:
//
//   ref     — the old loader (trialLoader.rs): load the full query, drive it.
//   stump   — the stump loader: load the full query, drive it.
//             Must equal ref: it checks the stump loader itself.
//   no-with — the stump loader's sinks -> `to_memory()`, then the SAME query
//             with every `.with(..)` removed, driven in memory.
//             Equals ref only if the loaded columns already are exactly the
//             with-survivors.
//
// Each query uses each column at most once (no sink merging).
// The rows are written to target/equiv/<query>.{ref,stump,nowith}.txt and diffed.
#![allow(unused, dead_code)]

use std::fmt::Debug;

use prela::engine::*;

// Each loader with its own copy of the schema: the schema's ColumnRef type
// comes from the loader next to it (`super::trial_loader` in fileSchema.rs).
// `#[path = "."]` keeps the inner paths relative to examples/ (not examples/old/).
#[path = "."]
mod old {
    #[path = "trialLoader.rs"]
    pub mod trial_loader;
    #[path = "fileSchema.rs"]
    pub mod file_schema;
}
#[path = "."]
mod stump {
    #[path = "LeftRightLoader.rs"]
    pub mod trial_loader;
    #[path = "fileSchema.rs"]
    pub mod file_schema;
}

// ===== The queries =============================================================
//
// Each query is written ONCE, as a macro over a db: `$m` is `full` or `nowith`,
// and `maybe_with!($m, a, b)` is `a.with(b)` or just `a`. The same text then
// builds the plan over both loaders' ColumnRefs and over the in-memory db.
// Columns are bound as references (`&db.x.y`), which works for both the Copy
// ColumnRefs and the in-memory Sets.

macro_rules! maybe_with {
    (full, $a:expr, $b:expr) => { $a.with($b) };
    (nowith, $a:expr, $b:expr) => { $a };
}

// TPC-H Q1 shape. Not selective: ~98% of lineitems ship by 1998-09-02.
macro_rules! pricing {
    ($m:ident, $db:expr) => {{
        let db = $db;
        let shipdate = &db.lineitem.shipdate;
        let returnflag = &db.lineitem.returnflag;
        let status = &db.lineitem.status;
        let quantity = &db.lineitem.quantity;
        let extendedprice = &db.lineitem.extendedprice;
        let discount = &db.lineitem.discount;
        let tax = &db.lineitem.tax;
        maybe_with!($m, (&db.lineitem), shipdate.le("1998-09-02"))
            .group_by(returnflag.and(status))
            .select(quantity.and(extendedprice).and(discount).and(tax))
    }};
}

// TPC-H Q6 shape: a 3-way Prod of filters as the with-argument. ~2% of lineitems.
macro_rules! forecast {
    ($m:ident, $db:expr) => {{
        let db = $db;
        let shipdate = &db.lineitem.shipdate;
        let discount = &db.lineitem.discount;
        let quantity = &db.lineitem.quantity;
        let extendedprice = &db.lineitem.extendedprice;
        maybe_with!($m, (&db.lineitem),
            shipdate.during("1994-01-01", "1995-01-01")
                .and(discount.between(0.05, 0.07))
                .and(quantity.lt(24.0)))
            .select(extendedprice)
    }};
}

// Cross-file with-argument, two hops: orders -> customer -> nation -> name.
// 1 nation of 25: ~4% of the 15,000 orders.
macro_rules! german_orders {
    ($m:ident, $db:expr) => {{
        let db = $db;
        let customer = &db.order.customer;
        let c_nation = &db.customer.nation;
        let n_name = &db.nation.name;
        let totalprice = &db.order.totalprice;
        maybe_with!($m, (&db.order), customer.select(c_nation).select(n_name).eq("GERMANY"))
            .select(totalprice)
    }};
}

// A with nested inside the with-argument (KeyLoad for Restrict), ANDed with a
// local filter (KeyLoad for Prod). 1 brand of 25 x 1 shipmode of 7: ~0.6%.
macro_rules! brand_air {
    ($m:ident, $db:expr) => {{
        let db = $db;
        let l_part = &db.lineitem.part;
        let brand = &db.part.brand;
        let shipmode = &db.lineitem.shipmode;
        let quantity = &db.lineitem.quantity;
        let extendedprice = &db.lineitem.extendedprice;
        maybe_with!($m, (&db.lineitem),
            maybe_with!($m, l_part, brand.eq("Brand#12")).and(shipmode.eq("AIR")))
            .select(quantity.and(extendedprice))
    }};
}

// A multi-valued with-argument through a reverse FK: parts with at least one
// French supplier. 4 suppliers per part, 1 nation of 25: ~15% of parts.
macro_rules! french_supplied_parts {
    ($m:ident, $db:expr) => {{
        let db = $db;
        let supplies = &db.part.supplies;
        let ps_supplier = &db.partsupp.supplier;
        let s_nation = &db.supplier.nation;
        let n_name = &db.nation.name;
        let p_name = &db.part.name;
        let retailprice = &db.part.retailprice;
        maybe_with!($m, (&db.part),
            supplies.select(ps_supplier).select(s_nation).select(n_name).eq("FRANCE"))
            .select(p_name.and(retailprice))
    }};
}

// Two withs: one on the root, one inside the select (the Load for Restrict
// path of an inner node). 5 of 25 nations, then customers with acctbal > 9000 (~9%).
macro_rules! rich_european_customers {
    ($m:ident, $db:expr) => {{
        let db = $db;
        let n_region = &db.nation.region;
        let r_name = &db.region.name;
        let customers = &db.nation.customers;
        let acctbal = &db.customer.acctbal;
        let c_name = &db.customer.name;
        maybe_with!($m, (&db.nation), n_region.select(r_name).eq("EUROPE"))
            .select(maybe_with!($m, customers, acctbal.gt(9000.0)).select(c_name))
    }};
}

// TPC-H Q3 shape: a with on the root and a with whose `a` is a FK (the
// with-argument restricts the customer VALUE). ~1.3% of orders x 20% of customers.
macro_rules! building_march_orders {
    ($m:ident, $db:expr) => {{
        let db = $db;
        let date = &db.order.date;
        let customer = &db.order.customer;
        let mktsegment = &db.customer.mktsegment;
        maybe_with!($m, (&db.order), date.during("1995-03-01", "1995-04-01"))
            .select(maybe_with!($m, customer, mktsegment.eq("BUILDING")))
    }};
}

// ===== Running and diffing =====================================================

/// Drive `q`; one line per (key, value) row, sorted (a multiset, so the order
/// rows come out in doesn't matter).
fn rows<Q: Drive>(q: &Q) -> Vec<String>
where
    Q::D: Debug,
    Q::R: Debug,
{
    let mut out = Vec::new();
    q.drive(|d, r| out.push(format!("{d:?} -> {r:?}")));
    out.sort();
    out
}

/// Write `rows` to target/equiv/<name>.<tag>.txt.
fn save(name: &str, tag: &str, rows: &[String]) {
    let dir = std::path::Path::new("target/equiv");
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(format!("{name}.{tag}.txt")), rows.join("\n")).unwrap();
}

/// `got` against `want`, both sorted: "same", or the counts of extra and
/// missing rows plus a few examples of each. Returns whether they match.
fn diff(label: &str, want: &[String], got: &[String]) -> bool {
    let (mut i, mut j) = (0, 0);
    let (mut extra, mut missing) = (Vec::new(), Vec::new());
    while i < want.len() || j < got.len() {
        if j == got.len() || (i < want.len() && want[i] < got[j]) {
            missing.push(&want[i]);
            i += 1;
        } else if i == want.len() || got[j] < want[i] {
            extra.push(&got[j]);
            j += 1;
        } else {
            i += 1;
            j += 1;
        }
    }
    if extra.is_empty() && missing.is_empty() {
        println!("    {label:<8} same ({} rows)", got.len());
        return true;
    }
    println!(
        "    {label:<8} DIFF: {} rows, {} extra, {} missing",
        got.len(),
        extra.len(),
        missing.len()
    );
    for r in extra.iter().take(3) {
        println!("               + {r}");
    }
    for r in missing.iter().take(3) {
        println!("               - {r}");
    }
    false
}

// One query through all three paths. A macro rather than a fn: the query is a
// macro, and each path builds it over a different db type.
macro_rules! check {
    ($q:ident, $old_db:expr, $new_db:expr, $results:expr) => {{
        let name = stringify!($q);
        println!("\n===== {name} =====");

        println!("  [ref] old loader:");
        let (loaded, _) = old::trial_loader::Load::load(
            $q!(full, $old_db),
            old::trial_loader::KeyFilter::All,
            old::trial_loader::KeyFilter::All,
            Vec::new(),
        );
        let reference = rows(&loaded);
        $old_db.clear();

        println!("  [stump] stump loader:");
        let (loaded, _) = stump::trial_loader::Load::load(
            $q!(full, $new_db),
            stump::trial_loader::KeyFilter::All,
            stump::trial_loader::KeyFilter::All,
            Vec::new(),
        );
        let stumped = rows(&loaded);

        println!("  [no-with] stump sinks, in memory, withs removed:");
        let mem: &'static stump::file_schema::MemTpch = Box::leak(Box::new($new_db.to_memory()));
        let nowith = rows(&$q!(nowith, mem));
        $new_db.clear();

        save(name, "ref", &reference);
        save(name, "stump", &stumped);
        save(name, "nowith", &nowith);
        println!("  vs ref ({} rows):", reference.len());
        let stump_ok = diff("stump", &reference, &stumped);
        let nowith_ok = diff("no-with", &reference, &nowith);
        $results.push((name, reference.len(), stump_ok, nowith_ok));
    }};
}

fn main() {
    let dir = "../data/tpch/parquet";
    // Leaked to `&'static`, as `Filter`'s `F: 'static` requires (see runner.rs).
    let old_db: &'static old::file_schema::TpchFiles =
        Box::leak(Box::new(old::file_schema::tpch(&mut old::file_schema::Catalog::open(dir))));
    let new_db: &'static stump::file_schema::TpchFiles =
        Box::leak(Box::new(stump::file_schema::tpch(&mut stump::file_schema::Catalog::open(dir))));

    let mut results = Vec::new();
    check!(pricing, old_db, new_db, results);
    check!(forecast, old_db, new_db, results);
    check!(german_orders, old_db, new_db, results);
    check!(brand_air, old_db, new_db, results);
    check!(french_supplied_parts, old_db, new_db, results);
    check!(rich_european_customers, old_db, new_db, results);
    check!(building_march_orders, old_db, new_db, results);

    println!("\n===== summary (rows in target/equiv/) =====");
    println!("{:<26} {:>8}  {:<6} {:<7}", "query", "ref rows", "stump", "no-with");
    for (name, n, stump_ok, nowith_ok) in &results {
        let mark = |ok: &bool| if *ok { "same" } else { "DIFF" };
        println!("{name:<26} {n:>8}  {:<6} {:<7}", mark(stump_ok), mark(nowith_ok));
    }
}
