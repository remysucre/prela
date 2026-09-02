use crate::engine::{Id, IntoQuery, SparseUniverse, Universe};
use crate::loader::{Col, Loader, Str, sparse_key};
use std::path::Path;

// =====================================================================
// Entities
// =====================================================================
//
// An entity struct reads like a SQL table: the first field `key` is the
// identity column over the entity's ids (its id space, sized at load), and
// every other field is a column keyed by those ids. `#[derive(IntoQuery)]`
// implements `IntoQuery for &Self` by returning `key`, and `QueryExt` is
// blanket-implemented over `IntoQuery`, so an entity drives directly:
// `db.part.select(..)` resolves by autoref on `&Part`. Reach for
// `db.part.key` itself when you want the `Universe`, e.g. for its `.n`.
//
// `Order` is the exception: TPC-H leaves the orderkey range gappy, so its
// key is a `SparseUniverse` — `0..n` with the holes masked out, so a drive
// never emits one. Its `.n` counts slots INCLUDING holes (about 4× the live
// orders), which is what a `dense_fold` over orders is sized by.

#[derive(IntoQuery)]
pub struct Region {
    pub key: Universe<Id<Region>>,
    pub name: Col<Region, Str>,
    pub comment: Col<Region, Str>,
}

#[derive(IntoQuery)]
pub struct Nation {
    pub key: Universe<Id<Nation>>,
    pub name: Col<Nation, Str>,
    pub region: Col<Nation, Id<Region>>,
    pub comment: Col<Nation, Str>,
}

#[derive(IntoQuery)]
pub struct Supplier {
    pub key: Universe<Id<Supplier>>,
    pub name: Col<Supplier, Str>,
    pub address: Col<Supplier, Str>,
    pub nation: Col<Supplier, Id<Nation>>,
    pub phone: Col<Supplier, Str>,
    pub acctbal: Col<Supplier, f64>,
    pub comment: Col<Supplier, Str>,
}

#[derive(IntoQuery)]
pub struct Customer {
    pub key: Universe<Id<Customer>>,
    pub name: Col<Customer, Str>,
    pub address: Col<Customer, Str>,
    pub nation: Col<Customer, Id<Nation>>,
    pub phone: Col<Customer, Str>,
    pub acctbal: Col<Customer, f64>,
    pub mktsegment: Col<Customer, Str>,
    pub comment: Col<Customer, Str>,
}

#[derive(IntoQuery)]
pub struct Part {
    pub key: Universe<Id<Part>>,
    pub name: Col<Part, Str>,
    pub mfgr: Col<Part, Str>,
    pub brand: Col<Part, Str>,
    /// `type` is a keyword; the cache file is `Part_ty.bin`.
    pub ty: Col<Part, Str>,
    pub size: Col<Part, i64>,
    pub container: Col<Part, Str>,
    pub retailprice: Col<Part, f64>,
    pub comment: Col<Part, Str>,
}

#[derive(IntoQuery)]
pub struct PartSupp {
    pub key: Universe<Id<PartSupp>>,
    pub part: Col<PartSupp, Id<Part>>,
    pub supplier: Col<PartSupp, Id<Supplier>>,
    pub availqty: Col<PartSupp, i64>,
    pub supplycost: Col<PartSupp, f64>,
    pub comment: Col<PartSupp, Str>,
}

#[derive(IntoQuery)]
pub struct Order {
    pub key: SparseUniverse<Id<Order>>,
    pub customer: Col<Order, Id<Customer>>,
    pub status: Col<Order, Str>,
    pub totalprice: Col<Order, f64>,
    pub date: Col<Order, i64>,
    pub priority: Col<Order, Str>,
    pub clerk: Col<Order, Str>,
    pub shippriority: Col<Order, i64>,
    pub comment: Col<Order, Str>,
}

#[derive(IntoQuery)]
pub struct Lineitem {
    pub key: Universe<Id<Lineitem>>,
    pub order: Col<Lineitem, Id<Order>>,
    pub part: Col<Lineitem, Id<Part>>,
    pub supplier: Col<Lineitem, Id<Supplier>>,
    pub number: Col<Lineitem, i64>,
    pub quantity: Col<Lineitem, f64>,
    pub extendedprice: Col<Lineitem, f64>,
    pub discount: Col<Lineitem, f64>,
    pub tax: Col<Lineitem, f64>,
    pub returnflag: Col<Lineitem, Str>,
    pub status: Col<Lineitem, Str>,
    pub shipdate: Col<Lineitem, i64>,
    pub commitdate: Col<Lineitem, i64>,
    pub receiptdate: Col<Lineitem, i64>,
    pub shipinstruct: Col<Lineitem, Str>,
    pub shipmode: Col<Lineitem, Str>,
    pub comment: Col<Lineitem, Str>,
}

// =====================================================================
// The database
// =====================================================================

pub struct Tpch {
    pub region: Region,
    pub nation: Nation,
    pub supplier: Supplier,
    pub customer: Customer,
    pub part: Part,
    pub partsupp: PartSupp,
    pub order: Order,
    pub lineitem: Lineitem,
}

// =====================================================================
// Loading
// =====================================================================

fn build(l: &mut Loader) -> Tpch {
    Tpch {
        region: Region {
            key: l.key("Region_name"),
            name: l.strs("Region_name"),
            comment: l.strs("Region_comment"),
        },
        nation: Nation {
            key: l.key("Nation_name"),
            name: l.strs("Nation_name"),
            region: l.ids("Nation_region"),
            comment: l.strs("Nation_comment"),
        },
        supplier: Supplier {
            key: l.key("Supplier_name"),
            name: l.strs("Supplier_name"),
            address: l.strs("Supplier_address"),
            nation: l.ids("Supplier_nation"),
            phone: l.strs("Supplier_phone"),
            acctbal: l.f64s("Supplier_acctbal"),
            comment: l.strs("Supplier_comment"),
        },
        customer: Customer {
            key: l.key("Customer_name"),
            name: l.strs("Customer_name"),
            address: l.strs("Customer_address"),
            nation: l.ids("Customer_nation"),
            phone: l.strs("Customer_phone"),
            acctbal: l.f64s("Customer_acctbal"),
            mktsegment: l.strs("Customer_mktsegment"),
            comment: l.strs("Customer_comment"),
        },
        part: Part {
            key: l.key("Part_name"),
            name: l.strs("Part_name"),
            mfgr: l.strs("Part_mfgr"),
            brand: l.strs("Part_brand"),
            ty: l.strs("Part_ty"),
            size: l.i64s("Part_size"),
            container: l.strs("Part_container"),
            retailprice: l.f64s("Part_retailprice"),
            comment: l.strs("Part_comment"),
        },
        partsupp: PartSupp {
            key: l.key("PartSupp_part"),
            part: l.ids("PartSupp_part"),
            supplier: l.ids("PartSupp_supplier"),
            availqty: l.i64s("PartSupp_availqty"),
            supplycost: l.f64s("PartSupp_supplycost"),
            comment: l.strs("PartSupp_comment"),
        },
        order: {
            // A hole is a slot whose customer FK regen filled with NO_ID;
            // the key is that column's domain with those masked out.
            let customer = l.ids("Order_customer");
            Order {
                key: sparse_key(&customer),
                customer,
                status: l.strs("Order_status"),
                totalprice: l.f64s("Order_totalprice"),
                date: l.i64s("Order_date"),
                priority: l.strs("Order_priority"),
                clerk: l.strs("Order_clerk"),
                shippriority: l.i64s("Order_shippriority"),
                comment: l.strs("Order_comment"),
            }
        },
        lineitem: Lineitem {
            key: l.key("Lineitem_order"),
            order: l.ids("Lineitem_order"),
            part: l.ids("Lineitem_part"),
            supplier: l.ids("Lineitem_supplier"),
            number: l.i64s("Lineitem_number"),
            quantity: l.f64s("Lineitem_quantity"),
            extendedprice: l.f64s("Lineitem_extendedprice"),
            discount: l.f64s("Lineitem_discount"),
            tax: l.f64s("Lineitem_tax"),
            returnflag: l.strs("Lineitem_returnflag"),
            status: l.strs("Lineitem_status"),
            shipdate: l.i64s("Lineitem_shipdate"),
            commitdate: l.i64s("Lineitem_commitdate"),
            receiptdate: l.i64s("Lineitem_receiptdate"),
            shipinstruct: l.strs("Lineitem_shipinstruct"),
            shipmode: l.strs("Lineitem_shipmode"),
            comment: l.strs("Lineitem_comment"),
        },
    }
}

pub fn load(dir: &Path) -> Tpch {
    build(&mut Loader::new(dir))
}

pub fn manifest() -> Vec<(String, u32)> {
    let mut l = Loader::probing();
    let _ = build(&mut l);
    l.manifest()
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Drive, QueryExt};

    /// Compile-only: every entity drives directly off its `key`, and
    /// `Order` does it through its sparse one. The body never runs —
    /// the point is that it type-checks.
    #[allow(dead_code)]
    fn entities_are_queries(db: &Tpch) {
        db.region.select(&db.region.name).drive(|_, _: Str| {});
        db.nation.select(&db.nation.name).drive(|_, _: Str| {});
        db.supplier.select(&db.supplier.acctbal).drive(|_, _: f64| {});
        db.customer.select(&db.customer.name).drive(|_, _: Str| {});
        db.part.select(&db.part.size).drive(|_, _: i64| {});
        db.partsupp
            .select(&db.partsupp.supplycost)
            .drive(|_, _: f64| {});
        db.order.select(&db.order.totalprice).drive(|_, _: f64| {});
        db.lineitem.select(&db.lineitem.quantity).drive(|_, _: f64| {});

        // Composing across entities still works from the bare struct.
        db.lineitem
            .select((&db.lineitem.order).select(&db.order.totalprice))
            .drive(|_, _: f64| {});
    }
}
