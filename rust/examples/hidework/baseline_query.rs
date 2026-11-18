// Run with: cargo run --release --features regen --example baseline_query
// The engine_query.rs query over the whole cached schema. Writes every result
// row, sorted, to out_baseline.txt for comparison with tree_query.
use prela::engine::*;
use prela::tpch_schema::{self, Customer, Nation};

fn main() {
    let db = tpch_schema::load(std::path::Path::new("../cache"));
    let Customer { acctbal, nation: c_nation, .. } = &db.customer;
    let Nation { region, .. } = &db.nation;

    // 1. nations that have at least one customer with acctbal >= 800, as a membership set
    let nations: MatSet<_> = db.customer
        .with(acctbal.ge(9900.0))
        .select(c_nation)
        .collect();

    // 2. semi-join: keep only those nations, mapped to (nationkey, regionkey)
    let q = db.nation.with(&nations).select(region);

    let mut rows: Vec<(usize, usize)> = Vec::new();
    q.drive(|nk, rk| rows.push((nk.0, rk.0)));
    rows.sort();

    // Same line format as tree_query, so the two files diff cleanly.
    let out: String = rows.iter().map(|(nk, rk)| format!("{nk} {rk}\n")).collect();
    std::fs::write("out_baseline.txt", out).unwrap();
    println!("baseline: {} rows -> out_baseline.txt", rows.len());
}
