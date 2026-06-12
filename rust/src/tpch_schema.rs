// The typed TPC-H schema — the `schema!` declaration of the TPC-H cache.
// `pub` marks the fields whose names are unique across this schema and
// don't collide with the universe accessors (so e.g. `Lineitem.part` and
// `Order.customer` stay qualified). Each `Entity / EntityNav` pair names
// the generated navigation trait (see src/schema.rs). Dates are pre-parsed
// yyyymmdd i64.

use crate::schema::schema;

schema! { TPCH / TpchSchema / tpch_init:
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
    Order(orders) / OrderNav {
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
