use crate::engine::{Bitset, Id, IntoQuery, SparseUniverse, Universe};
use crate::loader::{Col, Loader, Str, sparse_mask};
use std::path::Path;
use std::sync::OnceLock;

// =====================================================================
// Entities
// =====================================================================

pub struct Region {
    pub name: Col<Region, Str>,
    pub comment: Col<Region, Str>,
}

pub struct Nation {
    pub name: Col<Nation, Str>,
    pub region: Col<Nation, Id<Region>>,
    pub comment: Col<Nation, Str>,
}

pub struct Supplier {
    pub name: Col<Supplier, Str>,
    pub address: Col<Supplier, Str>,
    pub nation: Col<Supplier, Id<Nation>>,
    pub phone: Col<Supplier, Str>,
    pub acctbal: Col<Supplier, f64>,
    pub comment: Col<Supplier, Str>,
}

pub struct Customer {
    pub name: Col<Customer, Str>,
    pub address: Col<Customer, Str>,
    pub nation: Col<Customer, Id<Nation>>,
    pub phone: Col<Customer, Str>,
    pub acctbal: Col<Customer, f64>,
    pub mktsegment: Col<Customer, Str>,
    pub comment: Col<Customer, Str>,
}

pub struct Part {
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

pub struct PartSupp {
    pub part: Col<PartSupp, Id<Part>>,
    pub supplier: Col<PartSupp, Id<Supplier>>,
    pub availqty: Col<PartSupp, i64>,
    pub supplycost: Col<PartSupp, f64>,
    pub comment: Col<PartSupp, Str>,
}

pub struct Order {
    pub customer: Col<Order, Id<Customer>>,
    pub status: Col<Order, Str>,
    pub totalprice: Col<Order, f64>,
    pub date: Col<Order, i64>,
    pub priority: Col<Order, Str>,
    pub clerk: Col<Order, Str>,
    pub shippriority: Col<Order, i64>,
    pub comment: Col<Order, Str>,
    /// Derived, not loaded: which order slots are live. The one non-column
    /// field on an entity — `Order::all()` needs it, and `all()` only gets
    /// `&self`. Built on first use and leaked, because `SparseUniverse`
    /// holds `&'static Bitset` (the same reason `MultiRel` holds `&'static`
    /// slices).
    valid: OnceLock<&'static Bitset<Id<Order>>>,
}

pub struct Lineitem {
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
// Universes
// =====================================================================

impl Region {
    #[inline]
    pub fn all(&self) -> Universe<Id<Region>> {
        Universe::new(self.name.n_keys())
    }
}
impl Nation {
    #[inline]
    pub fn all(&self) -> Universe<Id<Nation>> {
        Universe::new(self.name.n_keys())
    }
}
impl Supplier {
    #[inline]
    pub fn all(&self) -> Universe<Id<Supplier>> {
        Universe::new(self.name.n_keys())
    }
}
impl Customer {
    #[inline]
    pub fn all(&self) -> Universe<Id<Customer>> {
        Universe::new(self.name.n_keys())
    }
}
impl Part {
    #[inline]
    pub fn all(&self) -> Universe<Id<Part>> {
        Universe::new(self.name.n_keys())
    }
}
impl PartSupp {
    #[inline]
    pub fn all(&self) -> Universe<Id<PartSupp>> {
        Universe::new(self.part.n_keys())
    }
}
impl Lineitem {
    #[inline]
    pub fn all(&self) -> Universe<Id<Lineitem>> {
        Universe::new(self.order.n_keys())
    }
}

impl Order {
    /// Size of the id space INCLUDING holes — TPC-H leaves the orderkey
    /// range gappy, so this is about 4× the number of live orders. It is
    /// what a `dense_fold` over orders must be sized by.
    #[inline]
    pub fn n_slots(&self) -> usize {
        self.customer.n_keys()
    }

    /// The drivable universe of orders: `0..n_slots` with the orderkey gaps
    /// masked out, so a drive never emits a hole. Unlike every other
    /// entity's `all()` this is a `SparseUniverse`, and the mask costs one
    /// pass over `Order_customer` the first time it is asked for.
    #[inline]
    pub fn all(&self) -> SparseUniverse<Id<Order>> {
        // A hole is a slot whose customer FK regen filled with NO_ID.
        let mask = self.valid.get_or_init(|| sparse_mask(&self.customer));
        SparseUniverse::new(self.n_slots(), mask)
    }
}

// =====================================================================
// Entity-as-query
// =====================================================================
//
// `QueryExt` is blanket-implemented over `IntoQuery`, so teaching an entity
// struct to become its own universe is enough to make it drivable directly:
// `db.part.select(..)` is `db.part.all().select(..)`, resolved by autoref on
// `&Part`. `all()` remains the explicit spelling — and the one to reach for
// when you want the `Universe` itself, e.g. for its `.n`.
//
// No coherence clash with `impl<Q: Query> IntoQuery for Q`: an entity struct
// is not itself a `Query`.

macro_rules! entity_query {
    ($($E:ident => $U:ident),* $(,)?) => {$(
        impl IntoQuery for &$E {
            type Q = $U<Id<$E>>;
            #[inline(always)]
            fn iq(self) -> Self::Q {
                self.all()
            }
        }
    )*};
}

entity_query! {
    Region => Universe,
    Nation => Universe,
    Supplier => Universe,
    Customer => Universe,
    Part => Universe,
    PartSupp => Universe,
    Order => SparseUniverse,
    Lineitem => Universe,
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
            name: l.strs("Region_name"),
            comment: l.strs("Region_comment"),
        },
        nation: Nation {
            name: l.strs("Nation_name"),
            region: l.ids("Nation_region"),
            comment: l.strs("Nation_comment"),
        },
        supplier: Supplier {
            name: l.strs("Supplier_name"),
            address: l.strs("Supplier_address"),
            nation: l.ids("Supplier_nation"),
            phone: l.strs("Supplier_phone"),
            acctbal: l.f64s("Supplier_acctbal"),
            comment: l.strs("Supplier_comment"),
        },
        customer: Customer {
            name: l.strs("Customer_name"),
            address: l.strs("Customer_address"),
            nation: l.ids("Customer_nation"),
            phone: l.strs("Customer_phone"),
            acctbal: l.f64s("Customer_acctbal"),
            mktsegment: l.strs("Customer_mktsegment"),
            comment: l.strs("Customer_comment"),
        },
        part: Part {
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
            part: l.ids("PartSupp_part"),
            supplier: l.ids("PartSupp_supplier"),
            availqty: l.i64s("PartSupp_availqty"),
            supplycost: l.f64s("PartSupp_supplycost"),
            comment: l.strs("PartSupp_comment"),
        },
        order: Order {
            customer: l.ids("Order_customer"),
            status: l.strs("Order_status"),
            totalprice: l.f64s("Order_totalprice"),
            date: l.i64s("Order_date"),
            priority: l.strs("Order_priority"),
            clerk: l.strs("Order_clerk"),
            shippriority: l.i64s("Order_shippriority"),
            comment: l.strs("Order_comment"),
            valid: OnceLock::new(),
        },
        lineitem: Lineitem {
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

    /// Compile-only: every entity drives directly, without `.all()`, and
    /// `Order` does it through its sparse universe. The body never runs —
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
