// Run with: cargo run --release --features regen --example runner
//
// Loads a query from data/tpch/parquet with the on-the-fly loader, drives the
// loaded plan, then turns the loaded columns into an in-memory db and drives an
// in-memory plan over them.
//
//   trialLoader.rs — the loader: KeyFilter, ParquetType, ColumnRef, Load
//   fileSchema.rs  — the schema: Catalog, the ColumnRef structs, the Mem structs
//
// `#[path]` keeps the files' names while giving the modules snake_case names.
#[path = "trialLoader.rs"]
mod trial_loader;
#[path = "fileSchema.rs"]
mod file_schema;

use prela::engine::*;

use file_schema::*;
use trial_loader::{KeyFilter, Load, print_rows};

// ===== 11. main: write your query here =========================================

fn main() {
    let mut catalog = Catalog::open("../data/tpch/parquet");
    // Leaked to `&'static` (like src/main.rs does with its db), so the `&ColumnRef`s
    // below borrow for the whole program, as `Filter`'s `F: 'static` requires.
    let db: &'static TpchFiles = Box::leak(Box::new(tpch(&mut catalog)));
    let Region { name: region_name, .. } = &db.region;


    // definining aliases
    let Nation { name: nation_name, region, customers, .. } = &db.nation;
    let Customer { name: customer_name, nation, acctbal, mktsegment, .. } = &db.customer;
    let Part { name: part_name, retailprice, supplies, .. } = &db.part;
    let Partsupp { supplycost, part, .. } = &db.partsupp;
    let Lineitem {
        shipdate, returnflag, status: l_status,
        quantity, extendedprice, discount, tax, ..
    } = &db.lineitem;
    let _ = (region_name, region, customer_name, nation, mktsegment, nation_name, customers, acctbal);
    let _ = (retailprice, part);

    /*load:
  scan lineitem.parquet          l_id -> l_shipdate    kept 9484 of 60175 rows
  scan lineitem.parquet          l_id -> l_discount    kept 2566 of 60175 rows
  scan lineitem.parquet          l_id -> l_quantity    kept 1191 of 60175 rows
  scan lineitem.parquet          l_id -> l_id          kept 1191 of 60175 rows
  scan lineitem.parquet          l_id -> l_extendedprice kept 1191 of 60175 rows
  scan lineitem.parquet          l_id -> l_discount    kept 1191 of 60175 rows*/


    // query plan definition -- from query6 in common.rs
    let q = db.lineitem
        .with(shipdate.during("1994-01-01", "1995-01-01")
         .and(discount.between(0.05, 0.07))
         .and(quantity.lt(24.0)))
        .select(extendedprice.and(discount));

    println!("load:");
    let (loaded, _) = q.load(KeyFilter::All, KeyFilter::All, Vec::new());

    println!("drive (the loaded plan):");
    //print_rows(&loaded);

    // ===== in memory: the loaded columns, under the same names =====
    println!("to_memory:");
    let mem: &'static MemTpch = Box::leak(Box::new(db.to_memory()));
    let MemRegion { name: region_name, .. } = &mem.region;
    let MemNation { name: nation_name, region, customers, .. } = &mem.nation;
    let MemCustomer { name: customer_name, nation, acctbal, mktsegment, .. } = &mem.customer;
    let MemPart { name: part_name, retailprice, supplies, .. } = &mem.part;
    let MemPartsupp { supplycost, part, .. } = &mem.partsupp;
    let MemLineitem {
        shipdate, returnflag, status: l_status,
        quantity, extendedprice, discount, tax, ..
    } = &mem.lineitem;
    let _ = (region_name, region, customer_name, nation, mktsegment, nation_name, customers, acctbal);
    let _ = (retailprice, part);

    // in-memory query plan -- identical to the other one but call on mem instead of db
    let plan = &mem.lineitem
        .with(shipdate.during("1994-01-01", "1995-01-01")
         .and(discount.between(0.05, 0.07))
         .and(quantity.lt(24.0)))
        .select(extendedprice.and(discount));
    

    println!("drive (the in-memory plan):");
    //print_rows(&plan);

    db.clear();
}
