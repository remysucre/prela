use prela::engine::*;
use prela::tpch_schema::{self, Customer, Nation};

fn main() {
let db = tpch_schema::load(std::path::Path::new("../cache"));
let Customer { acctbal, nation: c_nation, .. } = &db.customer;
let Nation { region, .. } = &db.nation;

// 1. nations that have at least one customer with acctbal >= 800, as a membership set
let nations: MatSet<_> = db.customer
    .with(acctbal.ge(800.0))
    .select(c_nation)
    .collect();

// 2. semi-join: keep only those nations, mapped to (nationkey, regionkey)
let q = db.nation.with(&nations).select(region);

let mut rows: Vec<(usize, usize)> = Vec::new();
q.drive(|nk, rk| rows.push((nk.0, rk.0)));   // Id<E>(pub usize, ..), so .0 is the raw key
rows.sort();

for (i, (nk, rk)) in rows.iter().take(5).enumerate() {
    println!("{i}: {nk} {rk}");
}

}

