use crate::engine::{Id, IntoQuery};
use crate::loader::{Col, Key, Loader, SparseKey, Str, sparse_key};
use std::path::Path;

// =====================================================================
// Entities
// =====================================================================
//
// An entity struct reads like a SQL table: `id` is the primary key — the
// identity column over the entity's ids, its id space sized at load — and
// every other field is a column keyed by those ids, with `Self` naming the
// entity. `#[derive(IntoQuery)]` implements `IntoQuery for &Self` by
// returning the `#[primary_key]` field, and `QueryExt` is
// blanket-implemented over `IntoQuery`, so an entity drives directly:
// `db.part.select(..)` resolves by autoref on `&Part`. Reach for
// `db.part.id` itself when you want the `Universe`, e.g. for its `.n`.
//
// `Order` is the exception: TPC-H leaves the orderkey range gappy, so its
// primary key is a `SparseKey` — `0..n` with the holes masked out, so a drive
// never emits one. Its `.n` counts slots INCLUDING holes (about 4× the live
// orders), which is what a `dense_fold` over orders is sized by.

#[derive(IntoQuery)]
pub struct Region {
    #[primary_key]
    pub id: Key<Self>,
    pub name: Col<Self, Str>,
    pub comment: Col<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Nation {
    #[primary_key]
    pub id: Key<Self>,
    pub name: Col<Self, Str>,
    pub region: Col<Self, Id<Region>>,
    pub comment: Col<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Supplier {
    #[primary_key]
    pub id: Key<Self>,
    pub name: Col<Self, Str>,
    pub address: Col<Self, Str>,
    pub nation: Col<Self, Id<Nation>>,
    pub phone: Col<Self, Str>,
    pub acctbal: Col<Self, f64>,
    pub comment: Col<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Customer {
    #[primary_key]
    pub id: Key<Self>,
    pub name: Col<Self, Str>,
    pub address: Col<Self, Str>,
    pub nation: Col<Self, Id<Nation>>,
    pub phone: Col<Self, Str>,
    pub acctbal: Col<Self, f64>,
    pub mktsegment: Col<Self, Str>,
    pub comment: Col<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Part {
    #[primary_key]
    pub id: Key<Self>,
    pub name: Col<Self, Str>,
    pub mfgr: Col<Self, Str>,
    pub brand: Col<Self, Str>,
    /// `type` is a keyword; the cache file is `Part_ty.bin`.
    pub ty: Col<Self, Str>,
    pub size: Col<Self, i64>,
    pub container: Col<Self, Str>,
    pub retailprice: Col<Self, f64>,
    pub comment: Col<Self, Str>,
}

#[derive(IntoQuery)]
pub struct PartSupp {
    #[primary_key]
    pub id: Key<Self>,
    pub part: Col<Self, Id<Part>>,
    pub supplier: Col<Self, Id<Supplier>>,
    pub availqty: Col<Self, i64>,
    pub supplycost: Col<Self, f64>,
    pub comment: Col<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Order {
    #[primary_key]
    pub id: SparseKey<Self>,
    pub customer: Col<Self, Id<Customer>>,
    pub status: Col<Self, Str>,
    pub totalprice: Col<Self, f64>,
    pub date: Col<Self, i64>,
    pub priority: Col<Self, Str>,
    pub clerk: Col<Self, Str>,
    pub shippriority: Col<Self, i64>,
    pub comment: Col<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Lineitem {
    #[primary_key]
    pub id: Key<Self>,
    pub order: Col<Self, Id<Order>>,
    pub part: Col<Self, Id<Part>>,
    pub supplier: Col<Self, Id<Supplier>>,
    pub number: Col<Self, i64>,
    pub quantity: Col<Self, f64>,
    pub extendedprice: Col<Self, f64>,
    pub discount: Col<Self, f64>,
    pub tax: Col<Self, f64>,
    pub returnflag: Col<Self, Str>,
    pub status: Col<Self, Str>,
    pub shipdate: Col<Self, i64>,
    pub commitdate: Col<Self, i64>,
    pub receiptdate: Col<Self, i64>,
    pub shipinstruct: Col<Self, Str>,
    pub shipmode: Col<Self, Str>,
    pub comment: Col<Self, Str>,
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
            id: l.key("Region_name"),
            name: l.strs("Region_name"),
            comment: l.strs("Region_comment"),
        },
        nation: Nation {
            id: l.key("Nation_name"),
            name: l.strs("Nation_name"),
            region: l.ids("Nation_region"),
            comment: l.strs("Nation_comment"),
        },
        supplier: Supplier {
            id: l.key("Supplier_name"),
            name: l.strs("Supplier_name"),
            address: l.strs("Supplier_address"),
            nation: l.ids("Supplier_nation"),
            phone: l.strs("Supplier_phone"),
            acctbal: l.f64s("Supplier_acctbal"),
            comment: l.strs("Supplier_comment"),
        },
        customer: Customer {
            id: l.key("Customer_name"),
            name: l.strs("Customer_name"),
            address: l.strs("Customer_address"),
            nation: l.ids("Customer_nation"),
            phone: l.strs("Customer_phone"),
            acctbal: l.f64s("Customer_acctbal"),
            mktsegment: l.strs("Customer_mktsegment"),
            comment: l.strs("Customer_comment"),
        },
        part: Part {
            id: l.key("Part_name"),
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
            id: l.key("PartSupp_part"),
            part: l.ids("PartSupp_part"),
            supplier: l.ids("PartSupp_supplier"),
            availqty: l.i64s("PartSupp_availqty"),
            supplycost: l.f64s("PartSupp_supplycost"),
            comment: l.strs("PartSupp_comment"),
        },
        order: {
            // A hole is a slot whose customer FK regen filled with NO_ID;
            // the id space is that column's domain with those masked out.
            let customer = l.ids("Order_customer");
            Order {
                id: sparse_key(&customer),
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
            id: l.key("Lineitem_order"),
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

    /// Compile-only: every entity drives directly off its primary key, and
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
