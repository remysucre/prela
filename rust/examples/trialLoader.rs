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
// Sections:
//   1. KeyFilter          — the filters passed DOWN (and the Blooms passed UP)
//   3. ParquetType        — Rust value type <-> Arrow column type
//   4. ColumnRef          — the leaf: (file, key column, value column)
//   5. Catalog            — reads footers, hands out checked ColumnRefs
//   6. Schema             — Region, Nation, Customer over data/tpch/parquet
//   7. Load               — the fourth mode
//   8. Load for ColumnRef — the only I/O
//   9. Load for Compose / Restrict / Filter
//  10. Helpers
//  11. main               — write your query here
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
use prela::loader::Str;

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

pub trait ParquetType: Copy + 'static {
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

// The relation key_col -> val_col of one Parquet file. It holds only names, so
// it is Copy (which `#[derive(IntoQuery)]` needs for the primary key) and
// loading never mutates it: the loaded data is returned, not stored here.
pub struct ColumnRef<E: 'static, T> {
    pub path: &'static str,
    pub key_col: &'static str,
    pub val_col: &'static str,
    _t: PhantomData<(E, T)>,
}

// Written by hand: `#[derive(Clone, Copy)]` would demand `E: Copy`, and the
// entity tags (`Nation`, ...) are not.
impl<E: 'static, T> Clone for ColumnRef<E, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<E: 'static, T> Copy for ColumnRef<E, T> {}

impl<E: 'static, T: Copy> Query for ColumnRef<E, T> {
    type D = Id<E>;
    type R = T;
}

// Only here because `QueryExt::with` demands its argument be `Member`
// (engine.rs:2135), and `Filter`/`Compose` are only Member/Probe when their
// children are Probe. An unloaded column cannot answer either question.
impl<E: 'static, T: Copy> Member for ColumnRef<E, T> {
    fn member(&self, _: Id<E>) -> bool {
        unreachable!("{}.{} is not loaded", self.path, self.val_col)
    }
}
impl<E: 'static, T: Copy> Probe for ColumnRef<E, T> {
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
        ColumnRef { path: leak_str(&self.path(file)), key_col, val_col, _t: PhantomData }
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

// ===== 7. Load: the fourth mode ================================================

// Like `Drive`, one case per node. `load` scans what the node needs under the
// two filters (on its key column and its value column) and returns
//   - the same node over in-memory relations (`Out`), and
//   - (keys Bloom, values Bloom): which of its keys / values survived.
pub trait Load: Query {
    type Out: Drive<D = Self::D, R = Self::R> + Probe;
    fn load(self, key_filter: KeyFilter, val_filter: KeyFilter) -> (Self::Out, (KeyFilter, KeyFilter));
}

// ===== 8. Load for ColumnRef: the only I/O =====================================

impl<E: 'static, T: ParquetType> Load for ColumnRef<E, T> {
    type Out = MultiRel<Id<E>, T>;
    fn load(self, key_filter: KeyFilter, val_filter: KeyFilter) -> (Self::Out, (KeyFilter, KeyFilter)) {
        let mut filters = Vec::new();
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
        (rel, blooms)
    }
}

// Queries hold leaves by reference (`let Nation { name, .. } = &db.nation;`).
impl<E: 'static, T: ParquetType> Load for &ColumnRef<E, T> {
    type Out = MultiRel<Id<E>, T>;
    fn load(self, key_filter: KeyFilter, val_filter: KeyFilter) -> (Self::Out, (KeyFilter, KeyFilter)) {
        (*self).load(key_filter, val_filter)
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
    fn load(self, key_filter: KeyFilter, val_filter: KeyFilter) -> (Self::Out, (KeyFilter, KeyFilter)) {
        let (a, (ak, av)) = self.a.load(key_filter, KeyFilter::All); // top-down: a first
        let (b, (_bk, bv)) = self.b.load(av, val_filter); // b's KEYS ⊆ a's VALUES
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
    fn load(self, key_filter: KeyFilter, val_filter: KeyFilter) -> (Self::Out, (KeyFilter, KeyFilter)) {
        let (b, (bk, _)) = self.b.load(KeyFilter::All, KeyFilter::All); // the with-argument FIRST
        let (a, (ak, av)) = self.a.load(key_filter, val_filter.and(bk)); // b's KEYS filter a's VALUES
        (Restrict { a, b }, (ak, av)) // result is a's columns
    }
}

// a.eq(..) / .ge(..) / .rx(..) / ... — the predicate is an opaque closure, so it
// cannot reach the scan. Load the child unfiltered by it, then drive the
// filtered node once in memory to get exact Blooms over what passes.
impl<A, F> Load for Filter<A, F>
where
    A: Load,
    F: Fn(A::R) -> bool,
    A::D: ParquetType,
    A::R: ParquetType,
{
    type Out = Filter<A::Out, F>;
    fn load(self, key_filter: KeyFilter, val_filter: KeyFilter) -> (Self::Out, (KeyFilter, KeyFilter)) {
        let (a, _) = self.a.load(key_filter, val_filter);
        let out = Filter { a, p: self.p };

        let mut keys = Vec::new();
        let mut vals = Vec::new();
        out.drive(|d, r| {
            keys.extend(d.as_key());
            vals.extend(r.as_key());
        });
        let kb = KeyFilter::from_keys(keys.len(), keys);
        let vb = if A::R::IS_KEY { KeyFilter::from_keys(vals.len(), vals) } else { KeyFilter::All };
        (out, (kb, vb))
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

// ===== 11. main: write your query here =========================================

fn main() {
    let mut catalog = Catalog::open("../data/tpch/parquet");
    let db = tpch(&mut catalog);
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
    //let q = db.part.with(supplies.select(supplycost).ge(900.0)).select(part_name);
    let q = db.part.select(supplies).select(supplycost);
    // ----------------------

    println!("load:");
    let (loaded, _) = q.load(KeyFilter::All, KeyFilter::All);

    println!("drive:");
    let mut rows = Vec::new();
    loaded.drive(|k, v| rows.push((k.0, v)));
    rows.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    for (k, v) in &rows {
        println!("  {k} -> {v}");
    }
    println!("{} rows", rows.len());
}
