// ===== 0. Header ============================================================
//
// Part of the `runner` example: cargo run --release --features regen --example runner
// (runner.rs has `main`; the schema is in fileSchema.rs).
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
// Sections (5, 6, 6b, 6c are in fileSchema.rs; 11, main, is in runner.rs):
//   1. KeyFilter          — the filters passed DOWN (and the Blooms passed UP)
//   3. ParquetType        — Rust value type <-> Arrow column type
//   4. ColumnRef          — the leaf: (file, key column, value column) + its sink
//   7. Load               — the fourth mode
//   8. Load for ColumnRef — the only I/O
//   9. Load for Compose / Restrict / Filter
//  10. Helpers
//  12. KeyLoad            — the key-only mode for with-arguments (exact Bitsets)
use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::{Arc, OnceLock};

use arrow::array::{Array, ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray};
use arrow::compute::cast;
use arrow::datatypes::{DataType, Field};
use prela::engine::*;
use prela::join_tree::{Bloom, ColumnFilter};
use prela::loader::Str;

use std::fs::File;
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::{
    ArrowPredicate, ArrowPredicateFn, ArrowReaderOptions, ParquetRecordBatchReader,
    ParquetRecordBatchReaderBuilder, RowFilter, RowSelection, RowSelector,
};

use parquet::arrow::{ProjectionMask, RowNumber};

const MISSING: u32 = u32::MAX;

// ===== 1. KeyFilter: filters passed down, Blooms passed up ====================

// A filter on one column of join keys. `All` lets every key through; `Blooms`
// lets a key through only if EVERY Bloom in the list may contain it, so ANDing
// two filters is concatenating their lists (Blooms of different sizes cannot
// be ANDed bitwise). `Exact` lets through exactly the ids in its set.
#[derive(Clone)]
pub enum KeyFilter {
    All,
    Blooms(Vec<Arc<Bloom>>),
    /// An exact set of raw key ids. Untyped (`Bitset` = `Bitset<usize>`),
    /// because a KeyFilter is passed to columns of any entity.
    Exact(Arc<Bitset>),
}

impl KeyFilter {
    pub fn and(self, other: KeyFilter) -> KeyFilter {
        match (self, other) {
            (KeyFilter::All, f) | (f, KeyFilter::All) => f,
            (KeyFilter::Blooms(mut a), KeyFilter::Blooms(b)) => {
                a.extend(b);
                KeyFilter::Blooms(a)
            }
            // Both exact: the ids in both.
            (KeyFilter::Exact(a), KeyFilter::Exact(b)) => KeyFilter::Exact(Arc::new(intersect(&a, &b))),
            // Exact and Blooms: keep the exact ids that every Bloom may contain.
            // Sound: it only drops ids a Bloom has proved absent.
            (KeyFilter::Exact(e), KeyFilter::Blooms(bs)) | (KeyFilter::Blooms(bs), KeyFilter::Exact(e)) => {
                let mut kept: Bitset = Bitset { bs: vec![0; e.bs.len()], n: e.n, _d: PhantomData };
                e.drive(|k, _| {
                    if bs.iter().all(|b| b.has(k as i64)) {
                        kept.set(k);
                    }
                });
                KeyFilter::Exact(Arc::new(kept))
            }
        }
    }

    /// A typed exact set as a filter: drops the type, moves the words (no copy).
    pub fn exact<D: Dense>(b: Bitset<D>) -> KeyFilter {
        KeyFilter::Exact(Arc::new(Bitset { bs: b.bs, n: b.n, _d: PhantomData }))
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

    // One ColumnFilter per Bloom (or one for the exact set), probed with
    // `column` during the scan.
    fn push_filters(&self, column: &str, out: &mut Vec<ColumnFilter>) {
        match self {
            KeyFilter::All => {}
            KeyFilter::Blooms(blooms) => {
                for b in blooms {
                    out.push(ColumnFilter::bloom(column, b.clone()));
                }
            }
            KeyFilter::Exact(set) => out.push(exact_filter(column, set.clone())),
        }
    }
    
    fn to_selection(&self) -> Option<RowSelection> {
        let KeyFilter::Exact(set) = self else { return None };
        let mut runs = Vec::new();
        let mut i = 0;
        while i < set.n {
            let keep = set.member(i);
            let mut j = i;
            while j < set.n && set.member(j) == keep {
                j += 1;
            }
            runs.push(if keep { RowSelector::select(j - i) } else { RowSelector::skip(j - i) });
            i = j;
        }
        Some(RowSelection::from(runs))
    }

    /// A filter on rows of ANOTHER file, tested on `column`, which stores that
    /// file's ids: look each id up in `index`, then test its row.
    fn push_via(&self, column: &'static str, index: &'static [u32], out: &mut Vec<ColumnFilter>) {
        let KeyFilter::Exact(set) = self else { return };
        let set = set.clone();
        out.push(ColumnFilter {
            columns: vec![column.to_string()],
            eval: Box::new(move |batch| {
                let ids = ints(batch.column_by_name(column).unwrap());
                Ok(ids.into_iter().map(|id| Some(row_in(index, id).is_some_and(|r| set.member(r)))).collect())
            }),
        })
    }
}

// ===== 3. ParquetType: Rust value type <-> Arrow column type ===================

// `PartialEq` so `union` can tell whether a loaded row is already in a sink.
pub trait ParquetType: Copy + PartialEq + 'static {
    /// Startup check against the footer.
    fn accepts(t: &DataType) -> bool;
    /// Load: Arrow column -> Rust values, appended to `out`.
    fn from_array(a: &ArrayRef, out: &mut Vec<Option<Self>>);
    /// Whether values of this type are join keys, so a Bloom over them is useful.
    const IS_KEY: bool = false;
    /// The value as a join key (raw file value, widened to i64).
    fn as_key(self) -> Option<i64> {
        None
    }
    /// Filter: one bool per row, `p` on the row's value. Nothing is kept, so by
    /// default it just decodes (fine for numbers: copied, not allocated).
    fn test_array(a: &ArrayRef, p: &Pred<Self>) -> BooleanArray {
        let mut vals = Vec::new();
        Self::from_array(a, &mut vals);
        vals.into_iter().map(|v| v.map(|v| p(v))).collect()
    }

    fn from_row(_row: usize) -> Self {
        unreachable!("only an id column points at rows")
    }
}

impl ParquetType for i64 {
    fn accepts(t: &DataType) -> bool {
        matches!(t, DataType::Int32 | DataType::Int64)
    }
    fn from_array(a: &ArrayRef, out: &mut Vec<Option<Self>>) {
        out.extend(ints(a));
}
}

impl ParquetType for f64 {
    fn accepts(t: &DataType) -> bool {
        matches!(t, DataType::Float64)
    }
    fn from_array(a: &ArrayRef, out: &mut Vec<Option<Self>>) {
        out.extend(a.as_any().downcast_ref::<Float64Array>().unwrap().iter());
}
}

impl ParquetType for Str {
    fn accepts(t: &DataType) -> bool {
        matches!(t, DataType::Utf8)
    }
    fn from_array(a: &ArrayRef, out: &mut Vec<Option<Self>>) {
        let strings = a.as_any().downcast_ref::<StringArray>().unwrap();
        out.extend(strings.iter().map(|s| s.map(leak_str)));
    }
    // Test the strings where they are (in the batch) instead of leaking a copy
    // of each, as `from_array` must for strings that are kept.
    fn test_array(a: &ArrayRef, p: &Pred<Self>) -> BooleanArray {
        let strings = a.as_any().downcast_ref::<StringArray>().unwrap();
        strings
            .iter() // Option<&str>: None for a NULL, which stays None = row not selected
            .map(|s| {
                s.map(|s| {
                    // SAFETY: `s` borrows from the batch, which outlives this call, and
                    // `p` only reads it and returns (eq, rx, is_in, ...); it never keeps it.
                    let s: &'static str = unsafe { std::mem::transmute::<&str, &'static str>(s) };
                    p(s)
                })
            })
            .collect()
    }
}

// An id is the raw key value from the file, used directly as the dense index
// (c_custkey 1..1500 -> slots 0..1500, slot 0 empty), so every column of an
// entity lines up without any shift.
impl<E: 'static> ParquetType for Id<E> {
    fn accepts(t: &DataType) -> bool {
        matches!(t, DataType::Int32 | DataType::Int64)
    }
    fn from_array(a: &ArrayRef, out: &mut Vec<Option<Self>>) {
        out.extend(ints(a).into_iter().map(|k| k.map(|k| Id::new(k as usize))));
    }
    const IS_KEY: bool = true;
    fn as_key(self) -> Option<i64> {
        Some(self.0 as i64)
    }
    fn from_row(row: usize) -> Self {
        Id::new(row)
    }
}


/// id -> row of one file: `rows[id]` is the row holding that id (MISSING if none).
/// Read on first use and kept: it never changes. A Vec, not a HashMap: ids are dense.
pub struct RowIndex {
    path: &'static str,
    rows: OnceLock<Vec<u32>>,
}

impl RowIndex {
    pub fn new(path: String) -> &'static RowIndex {
        Box::leak(Box::new(RowIndex { path: leak_str(&path), rows: OnceLock::new() }))
    }

    fn rows(&'static self) -> &'static [u32] {
        self.rows.get_or_init(|| {
            let mut rows = Vec::new();
            for batch in scan_rows(self.path, Vec::new(), None, &["id".to_string()]) {
                let batch = batch.unwrap();
                let ids = ints(batch.column_by_name("id").unwrap());
                let nums = ints(batch.column_by_name("row_number").unwrap());
                for (id, row) in ids.into_iter().zip(nums) {
                    let id = id.unwrap() as usize;
                    if id >= rows.len() {
                        rows.resize(id + 1, MISSING);
                    }
                    rows[id] = row.unwrap() as u32;
                }
            }
            rows
        })
    }
}

fn row_in(index: &[u32], id: Option<i64>) -> Option<usize> {
    let row = *index.get(id? as usize)?;
    (row != MISSING).then_some(row as usize)
}

// ===== 4. ColumnRef: the leaf ==================================================

/// Where a column's loaded rows accumulate, across every occurrence of the
/// column in the loaded query. `None` = not loaded since the last `clear()`.
pub type Sink<E, T> = RefCell<Option<MultiRel<Id<E>, T>>>;


#[derive(Clone, Copy)]
pub enum Side {
    Row,                    // this file's row number (the column is the file's `id`): read nothing
    Via(&'static RowIndex), // an id of another file: read the column, look up the row
    Value,                  // a plain value (title, note, ...): read the column
}

pub struct ColumnRef<E: 'static, T: Copy + 'static> {
    pub path: &'static str,
    pub key_col: &'static str,
    pub val_col: &'static str,
    pub key: Side,
    pub val: Side,
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

impl<E: 'static, T: Copy + 'static> ColumnRef<E, T> {

    pub fn new(path: &'static str, key_col: &'static str, val_col: &'static str) -> Self {
        let side = |col: &str| if col == "id" { Side::Row } else { Side::Value };
        ColumnRef {
            path, key_col, val_col,
            key: side(key_col),
            val: side(val_col),
            sink: Box::leak(Box::new(RefCell::new(None))),
            _t: PhantomData,
        }
    }
    pub fn key_via(mut self, index: &'static RowIndex) -> Self {
        self.key = Side::Via(index);
        self
    }
    pub fn val_via(mut self, index: &'static RowIndex) -> Self {
        self.val = Side::Via(index);
        self
    }

}

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

    fn filters(&self, key_filter: KeyFilter, val_filter: KeyFilter, preds: Vec<Pred<T>>)
        -> (Vec<ColumnFilter>, Option<RowSelection>) {
        let mut on_rows = KeyFilter::All;
        let mut filters = Vec::new();
        for (side, col, f) in [(self.key, self.key_col, key_filter), (self.val, self.val_col, val_filter)] {
            match side {
                Side::Row => on_rows = on_rows.and(f),
                Side::Via(index) => f.push_via(col, index.rows(), &mut filters),
                Side::Value => assert!(!T::IS_KEY, "{col} stores ids: give it .val_via(..) in the schema"),
            }
        }
        for p in preds {
            filters.push(to_predicate(self.val_col, p));
        }
        (filters, on_rows.to_selection())
    }
}

impl<E: 'static, T: Copy + 'static> Query for ColumnRef<E, T> {
    type D = Id<E>;
    type R = T;
}




// ===== 7. Load: the fourth mode ================================================

// Like `Drive`, one case per node. `load` scans what the node needs under the
// two filters (on its key column and its value column) and returns
//   - the same node over in-memory relations (`Out`), and
//   - (keys Bloom, values Bloom): which of its keys / values survived.
pub trait Load: Query {
    type Out: Query<D = Self::D, R = Self::R> ;
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
// `test_array` reads `Str` values in place, so testing leaks nothing.
fn to_predicate<T: ParquetType>(column: &'static str, p: Pred<T>) -> ColumnFilter {
    ColumnFilter {
        columns: vec![column.to_string()], // decode only this column to evaluate me
        eval: Box::new(move |batch| Ok(T::test_array(batch.column_by_name(column).unwrap(), &p))),
    }
}

impl<E: 'static, T: ParquetType> Load for ColumnRef<E, T> {
    type Out = MultiRel<Id<E>, T>;
    fn load(self, key_filter: KeyFilter, val_filter: KeyFilter, preds: Vec<Pred<T>>) -> (Self::Out, (KeyFilter, KeyFilter)) {
        let (filters, selection) = self.filters(key_filter, val_filter, preds);
        let selective = !filters.is_empty() || selection.is_some();

        // Read only the columns a side is stored in; a Row side is `row_number`, which always comes.
        let mut cols = Vec::new();
        if !matches!(self.key, Side::Row) { cols.push(self.key_col.to_string()); }
        if !matches!(self.val, Side::Row) { cols.push(self.val_col.to_string()); }

        let mut keys: Vec<usize> = Vec::new();
        let mut vals: Vec<T> = Vec::new();
        for batch in scan_rows(self.path, filters, selection, &cols) {
            let batch = batch.unwrap();
            let ks = rows_of(&batch, self.key, self.key_col);
            let vs: Vec<Option<T>> = match self.val {
                Side::Value => {
                    let mut v = Vec::new();
                    T::from_array(batch.column_by_name(self.val_col).unwrap(), &mut v);
                    v
                }
                _ => rows_of(&batch, self.val, self.val_col).into_iter().map(|r| r.map(T::from_row)).collect(),
            };
            for (k, v) in ks.into_iter().zip(vs) {
                if let (Some(k), Some(v)) = (k, v) {
                    keys.push(k);
                    vals.push(v);
                }
            }
        }

        // From here on as before, only now in rows.
        let blooms = if selective {
            let kb = KeyFilter::exact(exact_from_keys::<usize>(&keys));
            let vb = if T::IS_KEY {
                let rows: Vec<usize> = vals.iter().filter_map(|v| v.as_key()).map(|k| k as usize).collect();
                KeyFilter::exact(exact_from_keys::<usize>(&rows))
            } else {
                KeyFilter::All
            };
            (kb, vb)
        } else {
            (KeyFilter::All, KeyFilter::All)
        };

        let n = keys.iter().max().map_or(0, |&m| m + 1);
        let rel = MultiRel::from_pairs(n, keys.into_iter().zip(vals));
        
        let mut slot = self.sink.borrow_mut();
        let merged = match slot.take() {
            None => MultiRel::from_csr(rel.offsets, rel.v), 
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

// movie.select(title).eq("A")
impl<A, B> Load for ComposeRight<A, B>
where
    A: Load,
    B: Load<D = A::R>,
    B::Out: Probe,
{
    type Out = Compose<A::Out, B::Out>;
    fn load(
        self,
        key_filter: KeyFilter,
        val_filter: KeyFilter,
        preds: Vec<Pred<Self::R>>,
    ) -> (Self::Out, (KeyFilter, KeyFilter)) {
        // doing a rhs load first 
        let (b, (bk, bv)) = self.b.load(KeyFilter::All, val_filter, preds);
        let (a, (ak, _av)) = self.a.load(key_filter, bk, Vec::new());
        //let (a, (ak, av)) = self.a.load(key_filter, KeyFilter::All, Vec::new()); // top-down: a first
        //let (b, (_bk, bv)) = self.b.load(av, val_filter, preds); // b's KEYS ⊆ a's VALUES; preds follow the values
        (Compose { a, b }, (ak, bv)) // my D-side is a's, my R-side is b's
    }
}

impl<A, B> Load for Compose<A, B>
where
    A: Load,
    B: Load<D = A::R>,
    B::Out: Probe,
{
    type Out = Compose<A::Out, B::Out>;
    fn load(
        self,
        key_filter: KeyFilter,
        val_filter: KeyFilter,
        preds: Vec<Pred<Self::R>>,
    ) -> (Self::Out, (KeyFilter, KeyFilter)) {
        // doing a rhs load first 
        let (b, (bk, bv)) = self.b.load(KeyFilter::All, val_filter, preds);
        let (a, (ak, _av)) = self.a.load(key_filter, bk, Vec::new());
        //let (a, (ak, av)) = self.a.load(key_filter, KeyFilter::All, Vec::new()); // top-down: a first
        //let (b, (_bk, bv)) = self.b.load(av, val_filter, preds); // b's KEYS ⊆ a's VALUES; preds follow the values
        (Compose { a, b }, (ak, bv)) // my D-side is a's, my R-side is b's
    }
}

impl<A, B> Load for ComposeLeft<A, B>
where
    A: Load,
    B: Load<D = A::R>,
    B::Out: Probe,
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

// A `Lookup` field used by reference (`let Movie { kind, .. } = &db.movie;`,
// jobFileSchema.rs). Compose isn't Copy, but its halves are (two ColumnRefs):
// copy them into an owned Compose and load that.
impl<A, B> Load for &Compose<A, B>
where
    A: Load + Copy,
    B: Load<D = A::R> + Copy,
    B::Out: Probe,
{
    type Out = Compose<A::Out, B::Out>;
    fn load(
        self,
        key_filter: KeyFilter,
        val_filter: KeyFilter,
        preds: Vec<Pred<Self::R>>,
    ) -> (Self::Out, (KeyFilter, KeyFilter)) {
        if preds.is_empty() && matches!(val_filter, KeyFilter::All) {
        // nothing to filter the names by: follow the keys (top-down)
        ComposeLeft { a: self.a, b: self.b }.load(key_filter, val_filter, preds)
    } else {
        // a predicate on the name: find the few ids first (bottom-up)
        Compose { a: self.a, b: self.b }.load(key_filter, val_filter, preds)
    }
    }
}

impl<A, B> Load for &ComposeLeft<A, B>
where
    A: Load + Copy,
    B: Load<D = A::R> + Copy,
    B::Out: Probe,
{
    type Out = Compose<A::Out, B::Out>;
    fn load(
        self,
        key_filter: KeyFilter,
        val_filter: KeyFilter,
        preds: Vec<Pred<Self::R>>,
    ) -> (Self::Out, (KeyFilter, KeyFilter)) {
        ComposeLeft { a: self.a, b: self.b }.load(key_filter, val_filter, preds)
    }
}

impl<A, B> Load for &ComposeRight<A, B>
where
    A: Load + Copy,
    B: Load<D = A::R> + Copy,
    B::Out: Probe,
{
    type Out = Compose<A::Out, B::Out>;
    fn load(
        self,
        key_filter: KeyFilter,
        val_filter: KeyFilter,
        preds: Vec<Pred<Self::R>>,
    ) -> (Self::Out, (KeyFilter, KeyFilter)) {
        ComposeRight { a: self.a, b: self.b }.load(key_filter, val_filter, preds)
    }
}

// a.with(b) — a: D -> M, b: M -> X; keep a's pairs whose value ∈ dom b.
// b in key-only mode (exact, nothing materialized), then a filtered by it.
// b's set is kept as Restrict's b and the final drive re-checks it, which
// matters when a's value path doesn't push filters down (Prod, Map).
impl<A, B> Load for Restrict<A, B>
where
    A: Load,
    A::R: Dense,
    B: KeyLoad<D = A::R>,
{
    type Out = Restrict<A::Out, Bitset<A::R>>;
    fn load(
        self,
        key_filter: KeyFilter,
        val_filter: KeyFilter,
        preds: Vec<Pred<Self::R>>,
    ) -> (Self::Out, (KeyFilter, KeyFilter)) {
        let kb = self.b.keyload(KeyFilter::All, KeyFilter::All, Vec::new()); // the with-argument FIRST
        // Two owners (the scan's filter closure and the returned plan), so one copy of the words.
        let filter = KeyFilter::Exact(Arc::new(Bitset { bs: kb.bs.clone(), n: kb.n, _d: PhantomData }));
        let (a, blooms) = self.a.load(key_filter, val_filter.and(filter), preds); // b's KEYS filter a's VALUES
        (Restrict { a, b: kb }, blooms) // result is a's columns
    }
}



impl<K,S> Load for GroupBy<K,S>
where
    K: Load,
    S: Load<R = K::D>,
    K::R: Eq + std::hash::Hash,
    K::Out: Probe,
{
    type Out = GroupBy<K::Out, S::Out>;
    fn load(self, key_filter:KeyFilter, val_filter:KeyFilter, preds:Vec<Pred<Self::R>>) -> (Self::Out, (KeyFilter, KeyFilter)){
        let (key, (key_bloom, val_bloom)) = self.key.load(KeyFilter::All, key_filter, Vec::new());
        let (set, (keyset_bloom, valset_bloom)) = self.set.load(KeyFilter::All, val_filter.and(key_bloom.clone()), preds);

        (GroupBy{ key, set}, (val_bloom, valset_bloom))
    }
}

impl<Q> Load for InvStream<Q>
where 
Q: Load,
Q::R: Eq + std::hash::Hash,
{
    type Out = InvStream<Q::Out>;
    fn load (self, key_filter:KeyFilter, val_filter:KeyFilter, preds:Vec<Pred<Self::R>>) -> (Self::Out, (KeyFilter, KeyFilter)){
        
        let (rel, (keyb, valb)) = self.q.load(val_filter, key_filter, Vec::new());

        (InvStream{q:rel}, (valb, keyb))
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

impl<A, B> Load for Prod<A, B>
where
    A: Load,
    B: Load<D = A::D>,
    B::Out: Probe,
{
    type Out = Prod<A::Out, B::Out>;
    fn load(
        self,
        key_filter: KeyFilter,
        _val_filter: KeyFilter,     // values are (x, y) pairs: fits neither side's scan
        _preds: Vec<Pred<Self::R>>, // likewise; the final drive applies them exactly
    ) -> (Self::Out, (KeyFilter, KeyFilter)) {
        let (a, (ak, _)) = self.a.load(key_filter.clone(), KeyFilter::All, Vec::new());
        let (b, (bk, _)) = self.b.load(key_filter.and(ak.clone()), KeyFilter::All, Vec::new()); 

        let ab_k = ak.and(bk);
        (Prod { a, b }, (ab_k, KeyFilter::All)) 
    }
}

impl<A, B> Load for ProdRight<A, B>
where
    A: Load,
    B: Load<D = A::D>,
    B::Out: Probe,
{
    type Out = Prod<A::Out, B::Out>;
    fn load(
        self,
        key_filter: KeyFilter,
        _val_filter: KeyFilter,     // values are (x, y) pairs: fits neither side's scan
        _preds: Vec<Pred<Self::R>>, // likewise; the final drive applies them exactly
    ) -> (Self::Out, (KeyFilter, KeyFilter)) {
        // and itself isn't really symmetric in context so we provide this method we can use explicitly 
        // when there is an end predicate on the right
        let (b, (bk, _)) = self.b.load(key_filter.clone(), KeyFilter::All, Vec::new()); 
        let (a, (ak, _)) = self.a.load(key_filter.and(bk.clone()), KeyFilter::All, Vec::new());

        let ab_k = ak.and(bk);
        (Prod { a, b }, (ab_k, KeyFilter::All)) 
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
pub fn leak_str(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

// Two loads of the same column as one, each (key, value) pair once. Exact by
// the schema rule (fileSchema.rs, section 6): a pair is one file row, so an equal pair is the
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

// Drive `q` and print its rows sorted, then the count. Any key type, so it also
// prints a group_by's (tuple-keyed) rows.
pub fn print_rows<Q>(q: &Q)
where
    Q: Drive,
    Q::D: std::fmt::Debug + PartialOrd,
    Q::R: std::fmt::Debug + PartialOrd,
{
    let mut rows = Vec::new();
    q.drive(|k, v| rows.push((k, v)));
    rows.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    //for (k, v) in &rows {
    //    println!("  {k:?} -> {v:?}");
    //}
    println!("{} rows", rows.len());
}

// The Exact counterpart of `ColumnFilter::bloom`: one bit test per row.
// Built from ColumnFilter's pub fields, so join_tree.rs is unchanged.
fn exact_filter(column: &str, set: Arc<Bitset>) -> ColumnFilter {
    let name = column.to_string();
    ColumnFilter {
        columns: vec![name.clone()],
        eval: Box::new(move |batch| {
            let keys = keys_i64(batch.column_by_name(&name).unwrap());
            Ok(keys.iter().map(|&k| Some(k >= 0 && set.member(k as usize))).collect())
        }),
    }
}


// The ids in both sets.
fn intersect(a: &Bitset, b: &Bitset) -> Bitset {
    let bs = a.bs.iter().zip(&b.bs).map(|(x, y)| x & y).collect();
    Bitset { bs, n: a.n.min(b.n), _d: PhantomData }
}

// ===== 12. KeyLoad: the key-only mode, for with-arguments =====================
//
// `a.with(b)` only needs to know WHICH keys of b survive, not b's rows. So b is
// interpreted in this mode: nothing is materialized and no sink is filled; each
// node returns the exact set of its surviving keys, which becomes both the scan
// filter for a's values and the `b` of the loaded `Restrict` (a Bitset is an
// identity relation, so it is a valid stump).
//
// Definition, exactly:
//   keyload(q, kf, vf, preds) = { d ∈ kf : ∃r. (d, r) ∈ q ∧ r ∈ vf ∧ preds(r) }
// Rule: val_filter is All or Exact, never Blooms (a Bloom would let through
// values that don't match, so the result would not be exact). key_filter may
// be anything: a Bloom only adds keys that really are in q.
pub trait KeyLoad: Query<D: Dense> {
    fn keyload(self, key_filter: KeyFilter, val_filter: KeyFilter, preds: Vec<Pred<Self::R>>)
        -> Bitset<Self::D>;
}

impl<E: 'static, T: ParquetType> KeyLoad for ColumnRef<E, T> {
    fn keyload(self, key_filter: KeyFilter, val_filter: KeyFilter, preds: Vec<Pred<T>>) -> Bitset<Id<E>> {
        let (filters, selection) = self.filters(key_filter, val_filter, preds);
        // A Row key reads no column at all: just the selection, the predicates and row_number.
        let cols = match self.key {
            Side::Row => vec![],
            _ => vec![self.key_col.to_string()],
        };
        let mut keys = Vec::new();
        for batch in scan_rows(self.path, filters, selection, &cols) {
            keys.extend(rows_of(&batch.unwrap(), self.key, self.key_col).into_iter().flatten());
        }
        exact_from_keys(&keys)
    }
}


impl<E: 'static, T: ParquetType> KeyLoad for &ColumnRef<E, T> {
    fn keyload(self, key_filter: KeyFilter, val_filter: KeyFilter, preds: Vec<Pred<T>>) -> Bitset<Id<E>> {
        (*self).keyload(key_filter, val_filter, preds)
    }
}

// a.select(b): the keys of b passing vf/preds, then the keys of a whose value is one of them.
impl<A, B> KeyLoad for Compose<A, B>
where
    A: KeyLoad,
    B: KeyLoad<D = A::R>,
{
    fn keyload(self, key_filter: KeyFilter, val_filter: KeyFilter, preds: Vec<Pred<Self::R>>) -> Bitset<A::D> {
        let kb = self.b.keyload(KeyFilter::All, val_filter, preds);
        self.a.keyload(key_filter, KeyFilter::exact(kb), Vec::new())
    }
}

impl<A, B> KeyLoad for &Compose<A, B>
where
    A: KeyLoad + Copy,
    B: KeyLoad<D = A::R> + Copy,
{
    fn keyload(self, key_filter: KeyFilter, val_filter: KeyFilter, preds: Vec<Pred<Self::R>>) -> Bitset<A::D> {
        Compose { a: self.a, b: self.b }.keyload(key_filter, val_filter, preds)
    }
}

// a.with(b) nested inside a with-argument: b's keys narrow a's values.
impl<A, B> KeyLoad for Restrict<A, B>
where
    A: KeyLoad,
    B: KeyLoad<D = A::R>,
{
    fn keyload(self, key_filter: KeyFilter, val_filter: KeyFilter, preds: Vec<Pred<Self::R>>) -> Bitset<A::D> {
        let kb = self.b.keyload(KeyFilter::All, KeyFilter::All, Vec::new());
        self.a.keyload(key_filter, val_filter.and(KeyFilter::exact(kb)), preds)
    }
}

// a.eq(..) / .rx(..) / ...: add the predicate and pass it down to the leaf.
impl<A, F> KeyLoad for Filter<A, F>
where
    A: KeyLoad,
    A::R: 'static,
    F: Fn(A::R) -> bool + Send + Sync + 'static,
{
    fn keyload(self, key_filter: KeyFilter, val_filter: KeyFilter, mut preds: Vec<Pred<Self::R>>) -> Bitset<A::D> {
        preds.push(Arc::new(self.p));
        self.a.keyload(key_filter, val_filter, preds)
    }
}

// a.and(b): the keys in both. Exact only without predicates on the (x, y) pair,
// which no JOB query has; anything else needs a load+drive fallback.
impl<A, B> KeyLoad for Prod<A, B>
where
    A: KeyLoad,
    B: KeyLoad<D = A::D>,
{
    fn keyload(self, key_filter: KeyFilter, val_filter: KeyFilter, preds: Vec<Pred<Self::R>>) -> Bitset<A::D> {
        assert!(
            preds.is_empty() && matches!(val_filter, KeyFilter::All),
            "KeyLoad for Prod: a predicate on (x, y) pairs needs a load+drive fallback"
        );
        let ka = self.a.keyload(key_filter.clone(), KeyFilter::All, Vec::new());
        // b scanned under a's exact keys, so its keys are already ⊆ a's.
        self.b.keyload(key_filter.and(KeyFilter::exact(ka)), KeyFilter::All, Vec::new())
    }
}




//==== IO related stuff 
fn rows_of(batch: &RecordBatch, side: Side, column: &str) -> Vec<Option<usize>> {
    match side {
        Side::Row => ints(batch.column_by_name("row_number").unwrap())
            .into_iter().map(|r| r.map(|r| r as usize)).collect(),
        Side::Via(index) => {
            let index = index.rows();
            ints(batch.column_by_name(column).unwrap()).into_iter().map(|id| row_in(index, id)).collect()
        }
        Side::Value => panic!("{column} is a plain value, not a key: give it .key_via(..) in the schema"),
    }
}

/// Integers as i64 (any width), None for NULL. Replaces keys_i64, which
/// turned a NULL into 0 — harmless as an id, but row 0 is a real row.
fn ints(col: &ArrayRef) -> Vec<Option<i64>> {
    let wide = cast(col, &DataType::Int64).unwrap();
    wide.as_any().downcast_ref::<Int64Array>().unwrap().iter().collect()
}

fn exact_from_keys<D: Dense>(keys: &[usize]) -> Bitset<D> {
    let n = keys.iter().max().map_or(0, |&m| m + 1);
    let mut b = Bitset { bs: vec![0; n.div_ceil(64)], n, _d: PhantomData };
    for &k in keys {
        b.set(D::from_idx(k));
    }
    b
}

pub fn scan_rows(path: &str, filters: Vec<ColumnFilter>, rows: Option<RowSelection>, cols: &[String])
    -> ParquetRecordBatchReader {
    let file = File::open(path).expect(path);
    // `row_number`: each row's position in the file, added to every batch.
    let row_number = Arc::new(Field::new("row_number", DataType::Int64, false).with_extension_type(RowNumber));
    let options = ArrowReaderOptions::new().with_virtual_columns(vec![row_number]).unwrap();
    let mut builder = ParquetRecordBatchReaderBuilder::try_new_with_options(file, options).unwrap();

    let mut predicates: Vec<Box<dyn ArrowPredicate>> = Vec::new();
    for ColumnFilter { columns, mut eval } in filters {
        let m = mask(&builder, &columns);
        predicates.push(Box::new(ArrowPredicateFn::new(m, move |batch: RecordBatch| eval(&batch))));
    }
    if let Some(rows) = rows {
        builder = builder.with_row_selection(rows);
    }
    let out = mask(&builder, cols);
    builder.with_projection(out).with_row_filter(RowFilter::new(predicates)).build().unwrap()
}

fn mask(builder: &ParquetRecordBatchReaderBuilder<File>, cols: &[String]) -> ProjectionMask {
    let idx = cols.iter().map(|c| builder.schema().index_of(c).unwrap());
    ProjectionMask::roots(builder.parquet_schema(), idx)
}
