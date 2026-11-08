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

use prela::engine::*;

// The stump loader is the default; the old one is used by its full path (old::...).
use stump::file_schema::*;
use stump::trial_loader::{KeyFilter, Load, print_rows, KeyLoad};

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


    let q = db.lineitem.with(shipdate.le("1998-09-02"))
            .group_by(returnflag.and(l_status)) 
          .select(quantity.and(extendedprice).and(discount).and(tax));

    // The same query again for the old loader. It needs its own db: old::file_schema's
    // columns are the old loader's ColumnRef type, which only has the old Load.
    // (A query is also used up by .load, so `q` can't be loaded twice anyway.)
    let old_db: &'static old::file_schema::TpchFiles =
        Box::leak(Box::new(old::file_schema::tpch(&mut old::file_schema::Catalog::open("../data/tpch/parquet"))));
    let ol = &old_db.lineitem;
    let q_old = old_db.lineitem.with(ol.shipdate.le("1998-09-02"))
            .group_by(ol.returnflag.and(ol.status))
          .select(ol.quantity.and(ol.extendedprice).and(ol.discount).and(ol.tax));


    println!("load: trialLoader");
    let (loaded, _) = old::trial_loader::Load::load(
        q_old, old::trial_loader::KeyFilter::All, old::trial_loader::KeyFilter::All, Vec::new());

    println!("load: stump");
    let (loaded_stump, _) = q.load(KeyFilter::All, KeyFilter::All, Vec::new());


    println!("trial_Loader:");
    let mut t = std::time::Instant::now();

    print_rows(&loaded);
    println!("Exec time: {}", t.elapsed().as_secs_f64());

    t = std::time::Instant::now();
    println!("stump:");
    print_rows(&loaded_stump);
    println!("Exec time: {}", t.elapsed().as_secs_f64());


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


    let q_nowith = (&mem.lineitem)
            .group_by(returnflag.and(l_status)) 
          .select(quantity.and(extendedprice).and(discount).and(tax));
    

    println!("nowith plan :");
    t = std::time::Instant::now();
    print_rows(&q_nowith);
    println!("Exec time: {}", t.elapsed().as_secs_f64());

    db.clear();
}
