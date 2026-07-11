// The TPC-H schema declared via the proc macro (`schema_proc!`) instead of
// `schema!` — the declaration is IDENTICAL, only the macro differs (and no
// `#![recursion_limit]` bump is needed: there is no macro recursion).
// Demonstration only; nothing uses this module — main.rs runs tpch_schema.rs.

use crate::schema_proc::schema_proc;

schema_proc! {
    TPCHP / TpchProcSchema / tpch_proc_init:
    Region(region) / RegionNav { name: str, comment: str }
    Nation(nation) / NationNav { name: str, region: Region, comment: str }
    Supplier(supplier) / SupplierNav {
        name: str,
        address: str,
        nation: Nation,
        phone: str,
        acctbal: f64,
        comment: str,
    }
    Customer(customer) / CustomerNav {
        name: str,
        address: str,
        nation: Nation,
        phone: str,
        acctbal: f64,
        pub mktsegment: str,
        comment: str,
    }
    Part(part) / PartNav {
        name: str,
        pub mfgr: str,
        pub brand: str,
        pub ty: str,
        pub size: i64,
        pub container: str,
        pub retailprice: f64,
        comment: str,
    }
    PartSupp(partsupp) / PartSuppNav {
        part: Part,
        supplier: Supplier,
        pub availqty: i64,
        pub supplycost: f64,
        comment: str,
    }
    Order(orders sparse) / OrderNav {
        customer: Customer,
        status: str,
        pub totalprice: f64,
        pub date: i64,
        pub priority: str,
        pub clerk: str,
        pub shippriority: i64,
        comment: str,
    }
    Lineitem(lineitem) / LineitemNav {
        pub order: Order,
        part: Part,
        supplier: Supplier,
        pub number: i64,
        pub quantity: f64,
        pub extendedprice: f64,
        pub discount: f64,
        pub tax: f64,
        pub returnflag: str,
        status: str,
        pub shipdate: i64,
        pub commitdate: i64,
        pub receiptdate: i64,
        pub shipinstruct: str,
        pub shipmode: str,
        comment: str,
    }
}
