// ===== 0. Header ============================================================
//
// Run with: cargo run --release --features regen --example trialLoader
//
// An on-the-fly loader that interprets prela's own query AST (examples/trialLoader.md).
// A query is written over `ColumnRef` leaves exactly as it would be over the
// in-memory schema. `.load(..)` walks it like `.drive` does, one case per node
// (Compose, Restrict, Filter): each leaf is scanned from Parquet the moment it
// is visited, filtered by the Blooms known so far, and every node returns its
// in-memory self plus a pair of Blooms (keys, values) for its parent. The
// result is the same AST over `MultiRel`s, which is then driven as usual.
//
// Each leaf also keeps its loaded rows in its column's sink (merged when a
// column is loaded more than once), and `db.to_memory()` gathers the sinks into
// an in-memory db, so the loaded data can be queried with a different plan.
//
// Sections:
//   1. KeyFilter          — the filters passed DOWN (and the Blooms passed UP)
//   3. ParquetType        — Rust value type <-> Arrow column type
//   4. ColumnRef          — the leaf: (file, key column, value column) + its sink
//   5. Catalog            — reads footers, hands out checked ColumnRefs
//   6. Schema             — Region, Nation, Customer, Part, Partsupp over data/tpch/parquet
//   6b. In-memory schema  — MemRegion, ..., MemTpch: what the sinks become
//   6c. to_memory / clear
//   7. Load               — the fourth mode
//   8. Load for ColumnRef — the only I/O
//   9. Load for Compose / Restrict / Filter
//  10. Helpers
//  11. main               — write your query here
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::marker::PhantomData;
use std::sync::Arc;

use arrow::array::{Array, ArrayRef, Float64Array, Int64Array, StringArray};
use arrow::compute::cast;
use arrow::datatypes::{DataType, SchemaRef};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use prela::engine::*;
use prela::join_tree::{Bloom, ColumnFilter, scan};
use prela::loader::{Set, SparseKey, Str};

// ===== 1. KeyFilter: filters passed down, Blooms passed up ====================

// A filter on one column of join keys. `All` lets every key through; `Blooms`
// lets a key through only if EVERY Bloom in the list may contain it, so ANDing
// two filters is concatenating their lists (Blooms of different sizes cannot
// be ANDed bitwise).
#[derive(Clone)]
pub enum KeyFilter {
    All,
    Blooms(Vec<Arc<Bloom>>),
}

impl KeyFilter {
    pub fn and(self, other: KeyFilter) -> KeyFilter {
        match (self, other) {
            (KeyFilter::All, f) | (f, KeyFilter::All) => f,
            (KeyFilter::Blooms(mut a), KeyFilter::Blooms(b)) => {
                a.extend(b);
                KeyFilter::Blooms(a)
            }
        }
    }

    // A Bloom over `keys`. `Bloom::finish` gives None when the Bloom is too
    // full to rule anything out, which is the same as no filter at all.
    fn from_keys(capacity: usize, keys: impl IntoIterator<Item = i64>) -> KeyFilter {
        let mut bloom = Bloom::with_capacity(capacity);
        for k in keys {
            bloom.insert(k);
        }
        match bloom.finish() {
            Some(b) => KeyFilter::Blooms(vec![b]),
            None => KeyFilter::All,
        }
    }

    // One ColumnFilter per Bloom, probed with `column` during the scan.
    fn push_filters(&self, column: &str, out: &mut Vec<ColumnFilter>) {
        if let KeyFilter::Blooms(blooms) = self {
            for b in blooms {
                out.push(ColumnFilter::bloom(column, b.clone()));
            }
        }
    }
}

// ===== 3. ParquetType: Rust value type <-> Arrow column type ===================

// `PartialEq` so `union` can tell whether a loaded row is already in a sink.
pub trait ParquetType: Copy + PartialEq + 'static {
    /// Startup check against the footer.
    fn accepts(t: &DataType) -> bool;
    /// Load: Arrow column -> Rust values, appended to `out`.
    fn from_array(a: &ArrayRef, out: &mut Vec<Self>);
    /// Whether values of this type are join keys, so a Bloom over them is useful.
    const IS_KEY: bool = false;
    /// The value as a join key (raw file value, widened to i64).
    fn as_key(self) -> Option<i64> {
        None
    }
}

impl ParquetType for i64 {
    fn accepts(t: &DataType) -> bool {
        matches!(t, DataType::Int32 | DataType::Int64)
    }
    fn from_array(a: &ArrayRef, out: &mut Vec<Self>) {
        out.extend(keys_i64(a));
    }
}

impl ParquetType for f64 {
    fn accepts(t: &DataType) -> bool {
        matches!(t, DataType::Float64)
    }
    fn from_array(a: &ArrayRef, out: &mut Vec<Self>) {
        out.extend(a.as_any().downcast_ref::<Float64Array>().unwrap().values().iter());
    }
}

impl ParquetType for Str {
    fn accepts(t: &DataType) -> bool {
        matches!(t, DataType::Utf8)
    }
    fn from_array(a: &ArrayRef, out: &mut Vec<Self>) {
        let strings = a.as_any().downcast_ref::<StringArray>().unwrap();
        out.extend((0..strings.len()).map(|i| leak_str(strings.value(i))));
    }
}

// An id is the raw key value from the file, used directly as the dense index
// (c_custkey 1..1500 -> slots 0..1500, slot 0 empty), so every column of an
// entity lines up without any shift.
impl<E: 'static> ParquetType for Id<E> {
    fn accepts(t: &DataType) -> bool {
        matches!(t, DataType::Int32 | DataType::Int64)
    }
    fn from_array(a: &ArrayRef, out: &mut Vec<Self>) {
        out.extend(keys_i64(a).into_iter().map(|k| Id::new(k as usize)));
    }
    const IS_KEY: bool = true;
    fn as_key(self) -> Option<i64> {
        Some(self.0 as i64)
    }
}

// ===== 4. ColumnRef: the leaf ==================================================

/// Where a column's loaded rows accumulate, across every occurrence of the
/// column in the loaded query. `None` = not loaded since the last `clear()`.
pub type Sink<E, T> = RefCell<Option<MultiRel<Id<E>, T>>>;

// The relation key_col -> val_col of one Parquet file. It holds names plus a
// REFERENCE to its sink, so it stays Copy (which `#[derive(IntoQuery)]` needs
// for the primary key) and every copy of it, including the temporary one
// inside `load`, fills the same sink. `RefCell` is what allows writing to the
// sink through the shared `&'static TpchFiles`.
pub struct ColumnRef<E: 'static, T: Copy + 'static> {
    pub path: &'static str,
    pub key_col: &'static str,
    pub val_col: &'static str,
    pub sink: &'static Sink<E, T>,
    _t: PhantomData<(E, T)>,
}

// Written by hand: `#[derive(Clone, Copy)]` would demand `E: Copy`, and the
// entity tags (`Nation`, ...) are not.
impl<E: 'static, T: Copy + 'static> Clone for ColumnRef<E, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<E: 'static, T: Copy + 'static> Copy for ColumnRef<E, T> {}

impl<E: 'static, T: ParquetType> ColumnRef<E, T> {
    /// Forget what was loaded (the leaked rows themselves are not freed).
    pub fn clear(&self) {
        *self.sink.borrow_mut() = None;
    }

    /// The loaded rows, or an empty relation plus a warning if never loaded.
    pub fn loaded(&self, label: &str) -> MultiRel<Id<E>, T> {
        match &*self.sink.borrow() {
            Some(rel) => MultiRel::from_csr(rel.offsets, rel.v), // a view, no copy
            None => {
                println!("  warning: {label} ({} -> {}) was not loaded", self.key_col, self.val_col);
                MultiRel::from_pairs(0, [])
            }
        }
    }

    /// The ids with at least one loaded row (empty if never loaded).
    pub fn keys(&self) -> Vec<usize> {
        let mut ids = Vec::new();
        if let Some(rel) = &*self.sink.borrow() {
            rel.drive(|d, _| ids.push(d.idx()));
        }
        ids
    }
}

impl<E: 'static, T: Copy + 'static> Query for ColumnRef<E, T> {
    type D = Id<E>;
    type R = T;
}

// Only here because `QueryExt::with` demands its argument be `Member`
// (engine.rs:2135), and `Filter`/`Compose` are only Member/Probe when their
// children are Probe. An unloaded column cannot answer either question.
impl<E: 'static, T: Copy + 'static> Member for ColumnRef<E, T> {
    fn member(&self, _: Id<E>) -> bool {
        unreachable!("{}.{} is not loaded", self.path, self.val_col)
    }
}
impl<E: 'static, T: Copy + 'static> Probe for ColumnRef<E, T> {
    fn probe<K: FnMut(T)>(&self, _: Id<E>, _: K) {
        unreachable!("{}.{} is not loaded", self.path, self.val_col)
    }
    fn probe_any<K: FnMut(T) -> bool>(&self, _: Id<E>, _: K) -> bool {
        unreachable!("{}.{} is not loaded", self.path, self.val_col)
    }
}

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
        ColumnRef {
            path: leak_str(&self.path(file)),
            key_col,
            val_col,
            sink: Box::leak(Box::new(RefCell::new(None))), // one sink per column
            _t: PhantomData,
        }
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
}

#[derive(IntoQuery)]
pub struct Nation {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub name: ColumnRef<Self, Str>,
    pub region: ColumnRef<Self, Id<Region>>,
    pub customers: ColumnRef<Self, Id<Customer>>, // reverse FK: read from customer.parquet
}

#[derive(IntoQuery)]
pub struct Customer {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub name: ColumnRef<Self, Str>,
    pub nation: ColumnRef<Self, Id<Nation>>,
    pub acctbal: ColumnRef<Self, f64>,
    pub mktsegment: ColumnRef<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Part {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub name: ColumnRef<Self, Str>,
    pub retailprice: ColumnRef<Self, f64>,
    pub supplies: ColumnRef<Self, Id<Partsupp>>, // reverse FK: read from partsupp.parquet
}

#[derive(IntoQuery)]
pub struct Partsupp {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub supplycost: ColumnRef<Self, f64>,
    pub part: ColumnRef<Self, Id<Part>>,
}

pub struct TpchFiles {
    pub region: Region,
    pub nation: Nation,
    pub customer: Customer,
    pub part: Part,
    pub partsupp: Partsupp,
}

pub fn tpch(c: &mut Catalog) -> TpchFiles {
    TpchFiles {
        region: Region {
            id: c.key("region", "r_regionkey"),
            name: c.col("region", "r_regionkey", "r_name"),
        },
        nation: Nation {
            id: c.key("nation", "n_nationkey"),
            name: c.col("nation", "n_nationkey", "n_name"),
            region: c.col("nation", "n_nationkey", "n_regionkey"),
            customers: c.col("customer", "c_nationkey", "c_custkey"), // reverse FK
        },
        customer: Customer {
            id: c.key("customer", "c_custkey"),
            name: c.col("customer", "c_custkey", "c_name"),
            nation: c.col("customer", "c_custkey", "c_nationkey"),
            acctbal: c.col("customer", "c_custkey", "c_acctbal"),
            mktsegment: c.col("customer", "c_custkey", "c_mktsegment"),
        },
        part: Part {
            id: c.key("part", "p_partkey"),
            name: c.col("part", "p_partkey", "p_name"),
            retailprice: c.col("part", "p_partkey", "p_retailprice"),
            supplies: c.col("partsupp", "ps_partkey", "ps_id"), // reverse FK
        },
        partsupp: Partsupp {
            id: c.key("partsupp", "ps_id"),
            supplycost: c.col("partsupp", "ps_id", "ps_supplycost"),
            part: c.col("partsupp", "ps_id", "ps_partkey"),
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
}

#[derive(IntoQuery)]
pub struct MemNation {
    #[primary_key]
    pub id: SparseKey<Nation>,
    pub name: Set<Nation, Str>,
    pub region: Set<Nation, Id<Region>>,
    pub customers: Set<Nation, Id<Customer>>,
}

#[derive(IntoQuery)]
pub struct MemCustomer {
    #[primary_key]
    pub id: SparseKey<Customer>,
    pub name: Set<Customer, Str>,
    pub nation: Set<Customer, Id<Nation>>,
    pub acctbal: Set<Customer, f64>,
    pub mktsegment: Set<Customer, Str>,
}

#[derive(IntoQuery)]
pub struct MemPart {
    #[primary_key]
    pub id: SparseKey<Part>,
    pub name: Set<Part, Str>,
    pub retailprice: Set<Part, f64>,
    pub supplies: Set<Part, Id<Partsupp>>,
}

#[derive(IntoQuery)]
pub struct MemPartsupp {
    #[primary_key]
    pub id: SparseKey<Partsupp>,
    pub supplycost: Set<Partsupp, f64>,
    pub part: Set<Partsupp, Id<Part>>,
}

pub struct MemTpch {
    pub region: MemRegion,
    pub nation: MemNation,
    pub customer: MemCustomer,
    pub part: MemPart,
    pub partsupp: MemPartsupp,
}

// ===== 6c. to_memory / clear ===================================================

impl TpchFiles {
    /// The loaded columns as an in-memory db. An entity's `id` is every key
    /// loaded by any of its columns. Unloaded columns come back empty, with a
    /// warning. The sinks are left as they are; only `clear()` resets them.
    pub fn to_memory(&self) -> MemTpch {
        MemTpch {
            region: MemRegion {
                id: mem_key(&[self.region.id.keys(), self.region.name.keys()].concat()),
                name: self.region.name.loaded("region.name"),
            },
            nation: MemNation {
                id: mem_key(
                    &[
                        self.nation.id.keys(),
                        self.nation.name.keys(),
                        self.nation.region.keys(),
                        self.nation.customers.keys(),
                    ]
                    .concat(),
                ),
                name: self.nation.name.loaded("nation.name"),
                region: self.nation.region.loaded("nation.region"),
                customers: self.nation.customers.loaded("nation.customers"),
            },
            customer: MemCustomer {
                id: mem_key(
                    &[
                        self.customer.id.keys(),
                        self.customer.name.keys(),
                        self.customer.nation.keys(),
                        self.customer.acctbal.keys(),
                        self.customer.mktsegment.keys(),
                    ]
                    .concat(),
                ),
                name: self.customer.name.loaded("customer.name"),
                nation: self.customer.nation.loaded("customer.nation"),
                acctbal: self.customer.acctbal.loaded("customer.acctbal"),
                mktsegment: self.customer.mktsegment.loaded("customer.mktsegment"),
            },
            part: MemPart {
                id: mem_key(
                    &[
                        self.part.id.keys(),
                        self.part.name.keys(),
                        self.part.retailprice.keys(),
                        self.part.supplies.keys(),
                    ]
                    .concat(),
                ),
                name: self.part.name.loaded("part.name"),
                retailprice: self.part.retailprice.loaded("part.retailprice"),
                supplies: self.part.supplies.loaded("part.supplies"),
            },
            partsupp: MemPartsupp {
                id: mem_key(
                    &[self.partsupp.id.keys(), self.partsupp.supplycost.keys(), self.partsupp.part.keys()]
                        .concat(),
                ),
                supplycost: self.partsupp.supplycost.loaded("partsupp.supplycost"),
                part: self.partsupp.part.loaded("partsupp.part"),
            },
        }
    }

    /// Empty every sink, before loading the next query.
    pub fn clear(&self) {
        self.region.id.clear();
        self.region.name.clear();

        self.nation.id.clear();
        self.nation.name.clear();
        self.nation.region.clear();
        self.nation.customers.clear();

        self.customer.id.clear();
        self.customer.name.clear();
        self.customer.nation.clear();
        self.customer.acctbal.clear();
        self.customer.mktsegment.clear();

        self.part.id.clear();
        self.part.name.clear();
        self.part.retailprice.clear();
        self.part.supplies.clear();

        self.partsupp.id.clear();
        self.partsupp.supplycost.clear();
        self.partsupp.part.clear();
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

// ===== 7. Load: the fourth mode ================================================

// Like `Drive`, one case per node. `load` scans what the node needs under the
// two filters (on its key column and its value column) and returns
//   - the same node over in-memory relations (`Out`), and
//   - (keys Bloom, values Bloom): which of its keys / values survived.
pub trait Load: Query {
    type Out: Drive<D = Self::D, R = Self::R> + Probe;
    /// `preds`: closure predicates on this node's VALUES, passed down to the
    /// leaf that owns them, which applies them during its scan.
    fn load(
        self,
        key_filter: KeyFilter,
        val_filter: KeyFilter,
        preds: Vec<Pred<Self::R>>,
    ) -> (Self::Out, (KeyFilter, KeyFilter));
}

// ===== 8. Load for ColumnRef: the only I/O =====================================


/// A value predicate passed down to a leaf: one test on values of type `T`.
/// `Arc`: the Filter node keeps it for the final drive while the scan uses it too.
/// `Send + Sync`: Parquet's reader requires its filters to be `Send`.
pub type Pred<T> = Arc<dyn Fn(T) -> bool + Send + Sync>;

// A closure predicate as a ColumnFilter on `column`: decode the column into
// Rust values and call `p` on each row. Unlike a PredicateExpr this is one call
// per row, but it works for any predicate (eq, ge, rx, filt, ...).
// Caveat: for `Str` columns, `from_array` leaks every string, rejected rows included.
fn to_predicate<T: ParquetType>(column: &'static str, p: Pred<T>) -> ColumnFilter {
    ColumnFilter {
        columns: vec![column.to_string()], // decode only this column to evaluate me
        eval: Box::new(move |batch| {
            let mut vals: Vec<T> = Vec::new();
            T::from_array(batch.column_by_name(column).unwrap(), &mut vals);
            Ok(vals.into_iter().map(|v| Some(p(v))).collect()) // one bool per row
        }),
    }
}

impl<E: 'static, T: ParquetType> Load for ColumnRef<E, T> {
    type Out = MultiRel<Id<E>, T>;
    fn load(self, key_filter: KeyFilter, val_filter: KeyFilter, preds: Vec<Pred<T>>) -> (Self::Out, (KeyFilter, KeyFilter)) {
        let mut filters = Vec::new();

        for p in preds {
            filters.push(to_predicate(self.val_col, p));
        }

        key_filter.push_filters(self.key_col, &mut filters);
        val_filter.push_filters(self.val_col, &mut filters);
        // With no filters every row survives, and a Bloom over all keys rules out nothing.
        let selective = !filters.is_empty();

        let mut cols = vec![self.key_col.to_string()];
        if self.val_col != self.key_col {
            cols.push(self.val_col.to_string());
        }
        let (reader, row_count) = scan(self.path, filters, &cols);

        // Only rows passing all filters arrive here.
        let mut keys: Vec<i64> = Vec::new();
        let mut vals: Vec<T> = Vec::new();
        for batch in reader {
            let batch = batch.unwrap();
            keys.extend(keys_i64(batch.column_by_name(self.key_col).unwrap()));
            T::from_array(batch.column_by_name(self.val_col).unwrap(), &mut vals);
        }
        let file = self.path.rsplit('/').next().unwrap();
        println!(
            "  scan {file:<17} {:>12} -> {:<13} kept {:>4} of {:>4} rows",
            self.key_col,
            self.val_col,
            keys.len(),
            row_count
        );

        let blooms = if selective {
            let kb = KeyFilter::from_keys(row_count, keys.iter().copied());
            let vb = if T::IS_KEY {
                KeyFilter::from_keys(row_count, vals.iter().filter_map(|v| v.as_key()))
            } else {
                KeyFilter::All
            };
            (kb, vb)
        } else {
            (KeyFilter::All, KeyFilter::All)
        };

        let n = keys.iter().max().map_or(0, |&m| m as usize + 1);
        let rel = MultiRel::from_pairs(n, keys.into_iter().map(|k| k as usize).zip(vals));

        // Also keep the rows in this column's sink, for `to_memory`. A column
        // loaded again (another occurrence in the query) is merged, not replaced.
        let mut slot = self.sink.borrow_mut();
        let merged = match slot.take() {
            None => MultiRel::from_csr(rel.offsets, rel.v), // same slices as `rel`, no copy
            Some(old) => union(&old, &rel),
        };
        *slot = Some(merged);
        drop(slot);

        (rel, blooms)
    }
}

// Queries hold leaves by reference (`let Nation { name, .. } = &db.nation;`).
impl<E: 'static, T: ParquetType> Load for &ColumnRef<E, T> {
    type Out = MultiRel<Id<E>, T>;
    fn load(self, key_filter: KeyFilter, val_filter: KeyFilter, preds: Vec<Pred<T>>) -> (Self::Out, (KeyFilter, KeyFilter)) {
        (*self).load(key_filter, val_filter, preds)
    }
}

// ===== 9. Load for Compose / Restrict / Filter =================================

// a.select(b) — a: D -> M, b: M -> R, joined on a.2 = b.1.
impl<A, B> Load for Compose<A, B>
where
    A: Load,
    B: Load<D = A::R>,
{
    type Out = Compose<A::Out, B::Out>;
    fn load(
        self,
        key_filter: KeyFilter,
        val_filter: KeyFilter,
        preds: Vec<Pred<Self::R>>,
    ) -> (Self::Out, (KeyFilter, KeyFilter)) {
        let (a, (ak, av)) = self.a.load(key_filter, KeyFilter::All, Vec::new()); // top-down: a first
        let (b, (_bk, bv)) = self.b.load(av, val_filter, preds); // b's KEYS ⊆ a's VALUES; preds follow the values
        (Compose { a, b }, (ak, bv)) // my D-side is a's, my R-side is b's
    }
}

// a.with(b) — a: D -> M, b: M -> X; keep a's pairs whose value ∈ dom b.
impl<A, B> Load for Restrict<A, B>
where
    A: Load,
    B: Load<D = A::R>,
{
    type Out = Restrict<A::Out, B::Out>;
    fn load(
        self,
        key_filter: KeyFilter,
        val_filter: KeyFilter,
        preds: Vec<Pred<Self::R>>,
    ) -> (Self::Out, (KeyFilter, KeyFilter)) {
        let (b, (bk, _)) = self.b.load(KeyFilter::All, KeyFilter::All, Vec::new()); // the with-argument FIRST
        let (a, (ak, av)) = self.a.load(key_filter, val_filter.and(bk), preds); // b's KEYS filter a's VALUES
        (Restrict { a, b }, (ak, av)) // result is a's columns
    }
}

// a.eq(..) / .ge(..) / .rx(..) / ... — add the closure to the predicates
// collected from above and pass them all down to the leaf that owns our
// values, which applies them during its scan (`to_predicate`). The leaf's
// Blooms then already reflect the predicate, so nothing is driven here.
impl<A, F> Load for Filter<A, F>
where
    A: Load,
    A::R: 'static,
    F: Fn(A::R) -> bool + Send + Sync + 'static,
{
    // The closure now has two owners (the scan and this node), so it is shared
    // through an Arc; `Box<dyn Fn>` is the nameable type that holds it here.
    type Out = Filter<A::Out, Box<dyn Fn(A::R) -> bool>>;
    fn load(
        self,
        key_filter: KeyFilter,
        val_filter: KeyFilter,
        mut preds: Vec<Pred<Self::R>>,
    ) -> (Self::Out, (KeyFilter, KeyFilter)) {
        let p: Pred<A::R> = Arc::new(self.p);
        preds.push(p.clone());
        let (a, blooms) = self.a.load(key_filter, val_filter, preds);
        // Kept for the final drive: a cheap exact re-check of what the scan applied.
        (Filter { a, p: Box::new(move |r| p(r)) }, blooms)
    }
}

// ===== 10. Helpers =============================================================

// Join keys as i64, whatever integer width the file stored them in (the same
// as join_tree's private keys_i64: c_nationkey is Int32, c_custkey Int64).
fn keys_i64(col: &ArrayRef) -> Vec<i64> {
    let wide = cast(col, &DataType::Int64).unwrap();
    wide.as_any().downcast_ref::<Int64Array>().unwrap().values().to_vec()
}

// `Str` is `&'static str`, but Arrow frees its strings with the batch, so each
// one is copied into storage that lives for the whole program.
fn leak_str(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

// Two loads of the same column as one, each (key, value) pair once. Exact by
// the schema rule (section 6): a pair is one file row, so an equal pair is the
// same row loaded twice. (A NaN value would never compare equal; TPC-H has none.)
fn union<E: 'static, T: ParquetType>(old: &MultiRel<Id<E>, T>, new: &MultiRel<Id<E>, T>) -> MultiRel<Id<E>, T> {
    let n = old.n_dom().max(new.n_dom());
    let mut pairs = Vec::new();
    for k in 0..n {
        let mut row: Vec<T> = Vec::new();
        old.probe(Id::new(k), |v| row.push(v));
        new.probe(Id::new(k), |v| {
            if !row.contains(&v) {
                row.push(v);
            }
        });
        pairs.extend(row.into_iter().map(|v| (k, v)));
    }
    MultiRel::from_pairs(n, pairs)
}

// Drive `q` and print its rows sorted, then the count.
fn print_rows<E: 'static, Q>(q: &Q)
where
    Q: Drive<D = Id<E>>,
    Q::R: std::fmt::Display + PartialOrd,
{
    let mut rows = Vec::new();
    q.drive(|k, v| rows.push((k.0, v)));
    rows.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    for (k, v) in &rows {
        println!("  {k} -> {v}");
    }
    println!("{} rows", rows.len());
}

// ===== 11. main: write your query here =========================================

fn main() {
    let mut catalog = Catalog::open("../data/tpch/parquet");
    // Leaked to `&'static` (like src/main.rs does with its db), so the `&ColumnRef`s
    // below borrow for the whole program, as `Filter`'s `F: 'static` requires.
    let db: &'static TpchFiles = Box::leak(Box::new(tpch(&mut catalog)));
    let Region { name: region_name, .. } = &db.region;
    let Nation { name: nation_name, region, customers, .. } = &db.nation;
    let Customer { name: customer_name, nation, acctbal, mktsegment, .. } = &db.customer;
    let Part { name: part_name, retailprice, supplies, .. } = &db.part;
    let Partsupp { supplycost, part, .. } = &db.partsupp;
    let _ = (region_name, region, customer_name, nation, mktsegment, nation_name, customers, acctbal);
    let _ = (retailprice, part);

    // ----- your query -----
    // Nations with a customer whose balance is at least 9000, mapped to their name.
    //let q = db.nation.with(customers.select(acctbal).ge(998.0)).select(nation_name);
    // Parts with some supplier offering them at a supply cost of at least 900, mapped to their name.
    let q = db.part.with(supplies.select(supplycost).ge(900.0)).select(part_name).eq("goldenrod lavender spring chocolate lace");
    //let q = db.part.select(supplies).select(supplycost);
    // ----------------------

    println!("load:");
    let (loaded, _) = q.load(KeyFilter::All, KeyFilter::All, Vec::new());

    println!("drive (the loaded plan):");
    print_rows(&loaded);

    // ===== in memory: the loaded columns, under the same names =====
    println!("to_memory:");
    let mem: &'static MemTpch = Box::leak(Box::new(db.to_memory()));
    let MemRegion { name: region_name, .. } = &mem.region;
    let MemNation { name: nation_name, region, customers, .. } = &mem.nation;
    let MemCustomer { name: customer_name, nation, acctbal, mktsegment, .. } = &mem.customer;
    let MemPart { name: part_name, retailprice, supplies, .. } = &mem.part;
    let MemPartsupp { supplycost, part, .. } = &mem.partsupp;
    let _ = (region_name, region, customer_name, nation, mktsegment, nation_name, customers, acctbal);
    let _ = (retailprice, part);

    // ----- your in-memory plan -----
    // The same expression as `q`, so its rows must match the loaded plan's.
    let plan = mem.part.with(supplies.select(supplycost).ge(900.0)).select(part_name).eq("goldenrod lavender spring chocolate lace");
    // --------------------------------

    println!("drive (the in-memory plan):");
    print_rows(&plan);

    db.clear();
}
