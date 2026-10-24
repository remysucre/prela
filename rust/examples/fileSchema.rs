// The schema of data/tpch/parquet, in both forms:
//   - the files: one ColumnRef per column, checked against the Parquet footers
//     (what queries are written over and `.load()`ed), and
//   - in memory: what the ColumnRefs' sinks become after loading (`to_memory`).
//
// Part of the `runner` example (examples/runner.rs); the loader itself is in
// trialLoader.rs.
//
// Sections:
//   5.  Catalog           — reads footers, hands out checked ColumnRefs
//   6.  Schema            — every file and column in data/tpch/parquet
//   6b. In-memory schema  — MemRegion, ..., MemTpch: what the sinks become
//   6c. to_memory / clear
use std::collections::HashMap;
use std::fs::File;

use arrow::datatypes::SchemaRef;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use prela::engine::*;
use prela::loader::{Set, SparseKey, Str};

use crate::trial_loader::{ColumnRef, ParquetType, leak_str};

// ===== 5. Catalog: footers -> checked ColumnRefs ==============================

pub struct Catalog {
    dir: String,
    footers: HashMap<String, SchemaRef>,
}

impl Catalog {
    pub fn open(dir: &str) -> Self {
        Catalog { dir: dir.to_string(), footers: HashMap::new() }
    }

    fn path(&self, file: &str) -> String {
        format!("{}/{file}.parquet", self.dir)
    }

    // Each file's footer is read once; only the schema is kept.
    fn footer(&mut self, file: &str) -> SchemaRef {
        if !self.footers.contains_key(file) {
            let path = self.path(file);
            let f = File::open(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
            let builder = ParquetRecordBatchReaderBuilder::try_new(f).unwrap();
            self.footers.insert(file.to_string(), builder.schema().clone());
        }
        self.footers[file].clone()
    }

    /// key_col -> val_col in one file, checked against its footer.
    pub fn col<E: 'static, T: ParquetType>(
        &mut self,
        file: &str,
        key_col: &'static str,
        val_col: &'static str,
    ) -> ColumnRef<E, T> {
        let schema = self.footer(file);
        check::<Id<E>>(&schema, file, key_col);
        check::<T>(&schema, file, val_col);
        ColumnRef::new(leak_str(&self.path(file)), key_col, val_col)
    }

    /// The identity relation over the file's key: key_col -> key_col.
    pub fn key<E: 'static>(&mut self, file: &str, key_col: &'static str) -> ColumnRef<E, Id<E>> {
        self.col(file, key_col, key_col)
    }
}

fn check<T: ParquetType>(schema: &SchemaRef, file: &str, col: &str) {
    let field = schema
        .field_with_name(col)
        .unwrap_or_else(|_| panic!("{file}.parquet has no column {col}"));
    assert!(
        T::accepts(field.data_type()),
        "{file}.parquet: column {col} is {:?}, declared {}",
        field.data_type(),
        std::any::type_name::<T>()
    );
}

// ===== 6. Schema: data/tpch/parquet ============================================
//
// Every column of every file, under prela's tpch_schema field names. Each
// foreign key also has a reverse FK on the entity it points to (`Nation.customers`
// is `c_nationkey -> c_custkey`, read from customer.parquet).
//
// Dates are declared as `Str`: the files store them as "YYYY-MM-DD" strings
// (tpch_schema's in-memory dates are yyyymmdd i64, parsed by regen).
//
// Rule: every column has its file's primary key on one side. An entity's own
// attributes are keyed by its id (`p_partkey -> p_name`), and a reverse FK ends
// at the row's id (`ps_partkey -> ps_id`). So a (key, value) pair is exactly one
// file row, which is what lets `union` merge two loads of a column by comparing
// pairs. Do not add a shortcut that skips the row's id (e.g. a link table's
// `movie_id -> company_id`): two different rows could then give the same pair.

#[derive(IntoQuery)]
pub struct Region {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub name: ColumnRef<Self, Str>,
    pub comment: ColumnRef<Self, Str>,
    pub nations: ColumnRef<Self, Id<Nation>>, // reverse FK: read from nation.parquet
}

#[derive(IntoQuery)]
pub struct Nation {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub name: ColumnRef<Self, Str>,
    pub region: ColumnRef<Self, Id<Region>>,
    pub comment: ColumnRef<Self, Str>,
    pub customers: ColumnRef<Self, Id<Customer>>, // reverse FK: read from customer.parquet
    pub suppliers: ColumnRef<Self, Id<Supplier>>, // reverse FK: read from supplier.parquet
}

#[derive(IntoQuery)]
pub struct Supplier {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub name: ColumnRef<Self, Str>,
    pub address: ColumnRef<Self, Str>,
    pub nation: ColumnRef<Self, Id<Nation>>,
    pub phone: ColumnRef<Self, Str>,
    pub acctbal: ColumnRef<Self, f64>,
    pub comment: ColumnRef<Self, Str>,
    pub supplies: ColumnRef<Self, Id<Partsupp>>, // reverse FK: read from partsupp.parquet
    pub lineitems: ColumnRef<Self, Id<Lineitem>>, // reverse FK: read from lineitem.parquet
}

#[derive(IntoQuery)]
pub struct Customer {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub name: ColumnRef<Self, Str>,
    pub address: ColumnRef<Self, Str>,
    pub nation: ColumnRef<Self, Id<Nation>>,
    pub phone: ColumnRef<Self, Str>,
    pub acctbal: ColumnRef<Self, f64>,
    pub mktsegment: ColumnRef<Self, Str>,
    pub comment: ColumnRef<Self, Str>,
    pub orders: ColumnRef<Self, Id<Order>>, // reverse FK: read from orders.parquet
}

#[derive(IntoQuery)]
pub struct Part {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub name: ColumnRef<Self, Str>,
    pub mfgr: ColumnRef<Self, Str>,
    pub brand: ColumnRef<Self, Str>,
    /// `type` is a keyword; the file column is `p_type`.
    pub ty: ColumnRef<Self, Str>,
    pub size: ColumnRef<Self, i64>,
    pub container: ColumnRef<Self, Str>,
    pub retailprice: ColumnRef<Self, f64>,
    pub comment: ColumnRef<Self, Str>,
    pub supplies: ColumnRef<Self, Id<Partsupp>>, // reverse FK: read from partsupp.parquet
    pub lineitems: ColumnRef<Self, Id<Lineitem>>, // reverse FK: read from lineitem.parquet
}

#[derive(IntoQuery)]
pub struct Partsupp {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub part: ColumnRef<Self, Id<Part>>,
    pub supplier: ColumnRef<Self, Id<Supplier>>,
    pub availqty: ColumnRef<Self, i64>,
    pub supplycost: ColumnRef<Self, f64>,
    pub comment: ColumnRef<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Order {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub customer: ColumnRef<Self, Id<Customer>>,
    pub status: ColumnRef<Self, Str>,
    pub totalprice: ColumnRef<Self, f64>,
    pub date: ColumnRef<Self, Str>,
    pub priority: ColumnRef<Self, Str>,
    pub clerk: ColumnRef<Self, Str>,
    pub shippriority: ColumnRef<Self, i64>,
    pub comment: ColumnRef<Self, Str>,
    pub lineitems: ColumnRef<Self, Id<Lineitem>>, // reverse FK: read from lineitem.parquet
}

#[derive(IntoQuery)]
pub struct Lineitem {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub order: ColumnRef<Self, Id<Order>>,
    pub part: ColumnRef<Self, Id<Part>>,
    pub supplier: ColumnRef<Self, Id<Supplier>>,
    pub number: ColumnRef<Self, i64>,
    pub quantity: ColumnRef<Self, f64>,
    pub extendedprice: ColumnRef<Self, f64>,
    pub discount: ColumnRef<Self, f64>,
    pub tax: ColumnRef<Self, f64>,
    pub returnflag: ColumnRef<Self, Str>,
    pub status: ColumnRef<Self, Str>,
    pub shipdate: ColumnRef<Self, Str>,
    pub commitdate: ColumnRef<Self, Str>,
    pub receiptdate: ColumnRef<Self, Str>,
    pub shipinstruct: ColumnRef<Self, Str>,
    pub shipmode: ColumnRef<Self, Str>,
    pub comment: ColumnRef<Self, Str>,
}

pub struct TpchFiles {
    pub region: Region,
    pub nation: Nation,
    pub supplier: Supplier,
    pub customer: Customer,
    pub part: Part,
    pub partsupp: Partsupp,
    pub order: Order,
    pub lineitem: Lineitem,
}

pub fn tpch(c: &mut Catalog) -> TpchFiles {
    TpchFiles {
        region: Region {
            id: c.key("region", "r_regionkey"),
            name: c.col("region", "r_regionkey", "r_name"),
            comment: c.col("region", "r_regionkey", "r_comment"),
            nations: c.col("nation", "n_regionkey", "n_nationkey"), // reverse FK
        },
        nation: Nation {
            id: c.key("nation", "n_nationkey"),
            name: c.col("nation", "n_nationkey", "n_name"),
            region: c.col("nation", "n_nationkey", "n_regionkey"),
            comment: c.col("nation", "n_nationkey", "n_comment"),
            customers: c.col("customer", "c_nationkey", "c_custkey"), // reverse FK
            suppliers: c.col("supplier", "s_nationkey", "s_suppkey"), // reverse FK
        },
        supplier: Supplier {
            id: c.key("supplier", "s_suppkey"),
            name: c.col("supplier", "s_suppkey", "s_name"),
            address: c.col("supplier", "s_suppkey", "s_address"),
            nation: c.col("supplier", "s_suppkey", "s_nationkey"),
            phone: c.col("supplier", "s_suppkey", "s_phone"),
            acctbal: c.col("supplier", "s_suppkey", "s_acctbal"),
            comment: c.col("supplier", "s_suppkey", "s_comment"),
            supplies: c.col("partsupp", "ps_suppkey", "ps_id"), // reverse FK
            lineitems: c.col("lineitem", "l_suppkey", "l_id"), // reverse FK
        },
        customer: Customer {
            id: c.key("customer", "c_custkey"),
            name: c.col("customer", "c_custkey", "c_name"),
            address: c.col("customer", "c_custkey", "c_address"),
            nation: c.col("customer", "c_custkey", "c_nationkey"),
            phone: c.col("customer", "c_custkey", "c_phone"),
            acctbal: c.col("customer", "c_custkey", "c_acctbal"),
            mktsegment: c.col("customer", "c_custkey", "c_mktsegment"),
            comment: c.col("customer", "c_custkey", "c_comment"),
            orders: c.col("orders", "o_custkey", "o_orderkey"), // reverse FK
        },
        part: Part {
            id: c.key("part", "p_partkey"),
            name: c.col("part", "p_partkey", "p_name"),
            mfgr: c.col("part", "p_partkey", "p_mfgr"),
            brand: c.col("part", "p_partkey", "p_brand"),
            ty: c.col("part", "p_partkey", "p_type"),
            size: c.col("part", "p_partkey", "p_size"),
            container: c.col("part", "p_partkey", "p_container"),
            retailprice: c.col("part", "p_partkey", "p_retailprice"),
            comment: c.col("part", "p_partkey", "p_comment"),
            supplies: c.col("partsupp", "ps_partkey", "ps_id"), // reverse FK
            lineitems: c.col("lineitem", "l_partkey", "l_id"), // reverse FK
        },
        partsupp: Partsupp {
            id: c.key("partsupp", "ps_id"),
            part: c.col("partsupp", "ps_id", "ps_partkey"),
            supplier: c.col("partsupp", "ps_id", "ps_suppkey"),
            availqty: c.col("partsupp", "ps_id", "ps_availqty"),
            supplycost: c.col("partsupp", "ps_id", "ps_supplycost"),
            comment: c.col("partsupp", "ps_id", "ps_comment"),
        },
        order: Order {
            id: c.key("orders", "o_orderkey"),
            customer: c.col("orders", "o_orderkey", "o_custkey"),
            status: c.col("orders", "o_orderkey", "o_orderstatus"),
            totalprice: c.col("orders", "o_orderkey", "o_totalprice"),
            date: c.col("orders", "o_orderkey", "o_orderdate"),
            priority: c.col("orders", "o_orderkey", "o_orderpriority"),
            clerk: c.col("orders", "o_orderkey", "o_clerk"),
            shippriority: c.col("orders", "o_orderkey", "o_shippriority"),
            comment: c.col("orders", "o_orderkey", "o_comment"),
            lineitems: c.col("lineitem", "l_orderkey", "l_id"), // reverse FK
        },
        lineitem: Lineitem {
            id: c.key("lineitem", "l_id"),
            order: c.col("lineitem", "l_id", "l_orderkey"),
            part: c.col("lineitem", "l_id", "l_partkey"),
            supplier: c.col("lineitem", "l_id", "l_suppkey"),
            number: c.col("lineitem", "l_id", "l_linenumber"),
            quantity: c.col("lineitem", "l_id", "l_quantity"),
            extendedprice: c.col("lineitem", "l_id", "l_extendedprice"),
            discount: c.col("lineitem", "l_id", "l_discount"),
            tax: c.col("lineitem", "l_id", "l_tax"),
            returnflag: c.col("lineitem", "l_id", "l_returnflag"),
            status: c.col("lineitem", "l_id", "l_linestatus"),
            shipdate: c.col("lineitem", "l_id", "l_shipdate"),
            commitdate: c.col("lineitem", "l_id", "l_commitdate"),
            receiptdate: c.col("lineitem", "l_id", "l_receiptdate"),
            shipinstruct: c.col("lineitem", "l_id", "l_shipinstruct"),
            shipmode: c.col("lineitem", "l_id", "l_shipmode"),
            comment: c.col("lineitem", "l_id", "l_comment"),
        },
    }
}

// ===== 6b. In-memory schema: what the sinks become =============================
//
// One struct per table, with the same field names as section 6, so a query
// over the loaded data reads the same. Write the entity tag explicitly
// (`Region`), never `Self`: inside `MemRegion`, `Self` is `MemRegion`, and the
// ids must be the same `Id<Region>` that the ColumnRefs load.
// `id` is a SparseKey: only the loaded ids exist, and a drive skips the gaps.

#[derive(IntoQuery)]
pub struct MemRegion {
    #[primary_key]
    pub id: SparseKey<Region>,
    pub name: Set<Region, Str>,
    pub comment: Set<Region, Str>,
    pub nations: Set<Region, Id<Nation>>,
}

#[derive(IntoQuery)]
pub struct MemNation {
    #[primary_key]
    pub id: SparseKey<Nation>,
    pub name: Set<Nation, Str>,
    pub region: Set<Nation, Id<Region>>,
    pub comment: Set<Nation, Str>,
    pub customers: Set<Nation, Id<Customer>>,
    pub suppliers: Set<Nation, Id<Supplier>>,
}

#[derive(IntoQuery)]
pub struct MemSupplier {
    #[primary_key]
    pub id: SparseKey<Supplier>,
    pub name: Set<Supplier, Str>,
    pub address: Set<Supplier, Str>,
    pub nation: Set<Supplier, Id<Nation>>,
    pub phone: Set<Supplier, Str>,
    pub acctbal: Set<Supplier, f64>,
    pub comment: Set<Supplier, Str>,
    pub supplies: Set<Supplier, Id<Partsupp>>,
    pub lineitems: Set<Supplier, Id<Lineitem>>,
}

#[derive(IntoQuery)]
pub struct MemCustomer {
    #[primary_key]
    pub id: SparseKey<Customer>,
    pub name: Set<Customer, Str>,
    pub address: Set<Customer, Str>,
    pub nation: Set<Customer, Id<Nation>>,
    pub phone: Set<Customer, Str>,
    pub acctbal: Set<Customer, f64>,
    pub mktsegment: Set<Customer, Str>,
    pub comment: Set<Customer, Str>,
    pub orders: Set<Customer, Id<Order>>,
}

#[derive(IntoQuery)]
pub struct MemPart {
    #[primary_key]
    pub id: SparseKey<Part>,
    pub name: Set<Part, Str>,
    pub mfgr: Set<Part, Str>,
    pub brand: Set<Part, Str>,
    pub ty: Set<Part, Str>,
    pub size: Set<Part, i64>,
    pub container: Set<Part, Str>,
    pub retailprice: Set<Part, f64>,
    pub comment: Set<Part, Str>,
    pub supplies: Set<Part, Id<Partsupp>>,
    pub lineitems: Set<Part, Id<Lineitem>>,
}

#[derive(IntoQuery)]
pub struct MemPartsupp {
    #[primary_key]
    pub id: SparseKey<Partsupp>,
    pub part: Set<Partsupp, Id<Part>>,
    pub supplier: Set<Partsupp, Id<Supplier>>,
    pub availqty: Set<Partsupp, i64>,
    pub supplycost: Set<Partsupp, f64>,
    pub comment: Set<Partsupp, Str>,
}

#[derive(IntoQuery)]
pub struct MemOrder {
    #[primary_key]
    pub id: SparseKey<Order>,
    pub customer: Set<Order, Id<Customer>>,
    pub status: Set<Order, Str>,
    pub totalprice: Set<Order, f64>,
    pub date: Set<Order, Str>,
    pub priority: Set<Order, Str>,
    pub clerk: Set<Order, Str>,
    pub shippriority: Set<Order, i64>,
    pub comment: Set<Order, Str>,
    pub lineitems: Set<Order, Id<Lineitem>>,
}

#[derive(IntoQuery)]
pub struct MemLineitem {
    #[primary_key]
    pub id: SparseKey<Lineitem>,
    pub order: Set<Lineitem, Id<Order>>,
    pub part: Set<Lineitem, Id<Part>>,
    pub supplier: Set<Lineitem, Id<Supplier>>,
    pub number: Set<Lineitem, i64>,
    pub quantity: Set<Lineitem, f64>,
    pub extendedprice: Set<Lineitem, f64>,
    pub discount: Set<Lineitem, f64>,
    pub tax: Set<Lineitem, f64>,
    pub returnflag: Set<Lineitem, Str>,
    pub status: Set<Lineitem, Str>,
    pub shipdate: Set<Lineitem, Str>,
    pub commitdate: Set<Lineitem, Str>,
    pub receiptdate: Set<Lineitem, Str>,
    pub shipinstruct: Set<Lineitem, Str>,
    pub shipmode: Set<Lineitem, Str>,
    pub comment: Set<Lineitem, Str>,
}

pub struct MemTpch {
    pub region: MemRegion,
    pub nation: MemNation,
    pub supplier: MemSupplier,
    pub customer: MemCustomer,
    pub part: MemPart,
    pub partsupp: MemPartsupp,
    pub order: MemOrder,
    pub lineitem: MemLineitem,
}

// ===== 6c. to_memory / clear ===================================================

impl TpchFiles {
    /// The loaded columns as an in-memory db. An entity's `id` is every key
    /// loaded by any of its columns. Unloaded columns come back empty, with a
    /// warning. The sinks are left as they are; only `clear()` resets them.
    pub fn to_memory(&self) -> MemTpch {
        MemTpch {
            region: MemRegion {
                id: mem_key(
                    &[
                        self.region.id.keys(),
                        self.region.name.keys(),
                        self.region.comment.keys(),
                        self.region.nations.keys(),
                    ]
                    .concat(),
                ),
                name: self.region.name.loaded("region.name"),
                comment: self.region.comment.loaded("region.comment"),
                nations: self.region.nations.loaded("region.nations"),
            },
            nation: MemNation {
                id: mem_key(
                    &[
                        self.nation.id.keys(),
                        self.nation.name.keys(),
                        self.nation.region.keys(),
                        self.nation.comment.keys(),
                        self.nation.customers.keys(),
                        self.nation.suppliers.keys(),
                    ]
                    .concat(),
                ),
                name: self.nation.name.loaded("nation.name"),
                region: self.nation.region.loaded("nation.region"),
                comment: self.nation.comment.loaded("nation.comment"),
                customers: self.nation.customers.loaded("nation.customers"),
                suppliers: self.nation.suppliers.loaded("nation.suppliers"),
            },
            supplier: MemSupplier {
                id: mem_key(
                    &[
                        self.supplier.id.keys(),
                        self.supplier.name.keys(),
                        self.supplier.address.keys(),
                        self.supplier.nation.keys(),
                        self.supplier.phone.keys(),
                        self.supplier.acctbal.keys(),
                        self.supplier.comment.keys(),
                        self.supplier.supplies.keys(),
                        self.supplier.lineitems.keys(),
                    ]
                    .concat(),
                ),
                name: self.supplier.name.loaded("supplier.name"),
                address: self.supplier.address.loaded("supplier.address"),
                nation: self.supplier.nation.loaded("supplier.nation"),
                phone: self.supplier.phone.loaded("supplier.phone"),
                acctbal: self.supplier.acctbal.loaded("supplier.acctbal"),
                comment: self.supplier.comment.loaded("supplier.comment"),
                supplies: self.supplier.supplies.loaded("supplier.supplies"),
                lineitems: self.supplier.lineitems.loaded("supplier.lineitems"),
            },
            customer: MemCustomer {
                id: mem_key(
                    &[
                        self.customer.id.keys(),
                        self.customer.name.keys(),
                        self.customer.address.keys(),
                        self.customer.nation.keys(),
                        self.customer.phone.keys(),
                        self.customer.acctbal.keys(),
                        self.customer.mktsegment.keys(),
                        self.customer.comment.keys(),
                        self.customer.orders.keys(),
                    ]
                    .concat(),
                ),
                name: self.customer.name.loaded("customer.name"),
                address: self.customer.address.loaded("customer.address"),
                nation: self.customer.nation.loaded("customer.nation"),
                phone: self.customer.phone.loaded("customer.phone"),
                acctbal: self.customer.acctbal.loaded("customer.acctbal"),
                mktsegment: self.customer.mktsegment.loaded("customer.mktsegment"),
                comment: self.customer.comment.loaded("customer.comment"),
                orders: self.customer.orders.loaded("customer.orders"),
            },
            part: MemPart {
                id: mem_key(
                    &[
                        self.part.id.keys(),
                        self.part.name.keys(),
                        self.part.mfgr.keys(),
                        self.part.brand.keys(),
                        self.part.ty.keys(),
                        self.part.size.keys(),
                        self.part.container.keys(),
                        self.part.retailprice.keys(),
                        self.part.comment.keys(),
                        self.part.supplies.keys(),
                        self.part.lineitems.keys(),
                    ]
                    .concat(),
                ),
                name: self.part.name.loaded("part.name"),
                mfgr: self.part.mfgr.loaded("part.mfgr"),
                brand: self.part.brand.loaded("part.brand"),
                ty: self.part.ty.loaded("part.ty"),
                size: self.part.size.loaded("part.size"),
                container: self.part.container.loaded("part.container"),
                retailprice: self.part.retailprice.loaded("part.retailprice"),
                comment: self.part.comment.loaded("part.comment"),
                supplies: self.part.supplies.loaded("part.supplies"),
                lineitems: self.part.lineitems.loaded("part.lineitems"),
            },
            partsupp: MemPartsupp {
                id: mem_key(
                    &[
                        self.partsupp.id.keys(),
                        self.partsupp.part.keys(),
                        self.partsupp.supplier.keys(),
                        self.partsupp.availqty.keys(),
                        self.partsupp.supplycost.keys(),
                        self.partsupp.comment.keys(),
                    ]
                    .concat(),
                ),
                part: self.partsupp.part.loaded("partsupp.part"),
                supplier: self.partsupp.supplier.loaded("partsupp.supplier"),
                availqty: self.partsupp.availqty.loaded("partsupp.availqty"),
                supplycost: self.partsupp.supplycost.loaded("partsupp.supplycost"),
                comment: self.partsupp.comment.loaded("partsupp.comment"),
            },
            order: MemOrder {
                id: mem_key(
                    &[
                        self.order.id.keys(),
                        self.order.customer.keys(),
                        self.order.status.keys(),
                        self.order.totalprice.keys(),
                        self.order.date.keys(),
                        self.order.priority.keys(),
                        self.order.clerk.keys(),
                        self.order.shippriority.keys(),
                        self.order.comment.keys(),
                        self.order.lineitems.keys(),
                    ]
                    .concat(),
                ),
                customer: self.order.customer.loaded("order.customer"),
                status: self.order.status.loaded("order.status"),
                totalprice: self.order.totalprice.loaded("order.totalprice"),
                date: self.order.date.loaded("order.date"),
                priority: self.order.priority.loaded("order.priority"),
                clerk: self.order.clerk.loaded("order.clerk"),
                shippriority: self.order.shippriority.loaded("order.shippriority"),
                comment: self.order.comment.loaded("order.comment"),
                lineitems: self.order.lineitems.loaded("order.lineitems"),
            },
            lineitem: MemLineitem {
                id: mem_key(
                    &[
                        self.lineitem.id.keys(),
                        self.lineitem.order.keys(),
                        self.lineitem.part.keys(),
                        self.lineitem.supplier.keys(),
                        self.lineitem.number.keys(),
                        self.lineitem.quantity.keys(),
                        self.lineitem.extendedprice.keys(),
                        self.lineitem.discount.keys(),
                        self.lineitem.tax.keys(),
                        self.lineitem.returnflag.keys(),
                        self.lineitem.status.keys(),
                        self.lineitem.shipdate.keys(),
                        self.lineitem.commitdate.keys(),
                        self.lineitem.receiptdate.keys(),
                        self.lineitem.shipinstruct.keys(),
                        self.lineitem.shipmode.keys(),
                        self.lineitem.comment.keys(),
                    ]
                    .concat(),
                ),
                order: self.lineitem.order.loaded("lineitem.order"),
                part: self.lineitem.part.loaded("lineitem.part"),
                supplier: self.lineitem.supplier.loaded("lineitem.supplier"),
                number: self.lineitem.number.loaded("lineitem.number"),
                quantity: self.lineitem.quantity.loaded("lineitem.quantity"),
                extendedprice: self.lineitem.extendedprice.loaded("lineitem.extendedprice"),
                discount: self.lineitem.discount.loaded("lineitem.discount"),
                tax: self.lineitem.tax.loaded("lineitem.tax"),
                returnflag: self.lineitem.returnflag.loaded("lineitem.returnflag"),
                status: self.lineitem.status.loaded("lineitem.status"),
                shipdate: self.lineitem.shipdate.loaded("lineitem.shipdate"),
                commitdate: self.lineitem.commitdate.loaded("lineitem.commitdate"),
                receiptdate: self.lineitem.receiptdate.loaded("lineitem.receiptdate"),
                shipinstruct: self.lineitem.shipinstruct.loaded("lineitem.shipinstruct"),
                shipmode: self.lineitem.shipmode.loaded("lineitem.shipmode"),
                comment: self.lineitem.comment.loaded("lineitem.comment"),
            },
        }
    }

    /// Empty every sink, before loading the next query.
    pub fn clear(&self) {
        self.region.id.clear();
        self.region.name.clear();
        self.region.comment.clear();
        self.region.nations.clear();

        self.nation.id.clear();
        self.nation.name.clear();
        self.nation.region.clear();
        self.nation.comment.clear();
        self.nation.customers.clear();
        self.nation.suppliers.clear();

        self.supplier.id.clear();
        self.supplier.name.clear();
        self.supplier.address.clear();
        self.supplier.nation.clear();
        self.supplier.phone.clear();
        self.supplier.acctbal.clear();
        self.supplier.comment.clear();
        self.supplier.supplies.clear();
        self.supplier.lineitems.clear();

        self.customer.id.clear();
        self.customer.name.clear();
        self.customer.address.clear();
        self.customer.nation.clear();
        self.customer.phone.clear();
        self.customer.acctbal.clear();
        self.customer.mktsegment.clear();
        self.customer.comment.clear();
        self.customer.orders.clear();

        self.part.id.clear();
        self.part.name.clear();
        self.part.mfgr.clear();
        self.part.brand.clear();
        self.part.ty.clear();
        self.part.size.clear();
        self.part.container.clear();
        self.part.retailprice.clear();
        self.part.comment.clear();
        self.part.supplies.clear();
        self.part.lineitems.clear();

        self.partsupp.id.clear();
        self.partsupp.part.clear();
        self.partsupp.supplier.clear();
        self.partsupp.availqty.clear();
        self.partsupp.supplycost.clear();
        self.partsupp.comment.clear();

        self.order.id.clear();
        self.order.customer.clear();
        self.order.status.clear();
        self.order.totalprice.clear();
        self.order.date.clear();
        self.order.priority.clear();
        self.order.clerk.clear();
        self.order.shippriority.clear();
        self.order.comment.clear();
        self.order.lineitems.clear();

        self.lineitem.id.clear();
        self.lineitem.order.clear();
        self.lineitem.part.clear();
        self.lineitem.supplier.clear();
        self.lineitem.number.clear();
        self.lineitem.quantity.clear();
        self.lineitem.extendedprice.clear();
        self.lineitem.discount.clear();
        self.lineitem.tax.clear();
        self.lineitem.returnflag.clear();
        self.lineitem.status.clear();
        self.lineitem.shipdate.clear();
        self.lineitem.commitdate.clear();
        self.lineitem.receiptdate.clear();
        self.lineitem.shipinstruct.clear();
        self.lineitem.shipmode.clear();
        self.lineitem.comment.clear();
    }
}

// The ids of E present in memory, as a SparseKey over 0..=max id with only
// these ids valid (as tree_query.rs's `key`). The Bitset is leaked because
// SparseUniverse holds a `&'static`.
fn mem_key<E: 'static>(ids: &[usize]) -> SparseKey<E> {
    let n = ids.iter().max().map_or(0, |&m| m + 1);
    let mut valid: Bitset<Id<E>> = Bitset::empty(Universe::<Id<E>>::new(n));
    for &i in ids {
        valid.set(Id::new(i));
    }
    SparseUniverse::new(n, Box::leak(Box::new(valid)))
}
