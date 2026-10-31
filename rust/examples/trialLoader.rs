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
use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::Arc;

use arrow::array::{Array, ArrayRef, Float64Array, Int64Array, StringArray};
use arrow::compute::cast;
use arrow::datatypes::DataType;
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

impl<E: 'static, T: Copy + 'static> ColumnRef<E, T> {
    /// A column with its own, empty sink. Called by `Catalog::col` (fileSchema.rs)
    /// after it has checked the columns against the file's footer.
    pub fn new(path: &'static str, key_col: &'static str, val_col: &'static str) -> Self {
        ColumnRef {
            path,
            key_col,
            val_col,
            sink: Box::leak(Box::new(RefCell::new(None))), // one sink per column
            _t: PhantomData,
        }
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
// movie.select(cast).select(person)
//                    select 
//            select          Person
//        movies      cast


// movie.select(title.eq("A"))
// movie.with(title.eq("A"))

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

// movie.select(title).eq("A")
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
        Compose { a: self.a, b: self.b }.load(key_filter, val_filter, preds)
    }
}

// a.with(b) — a: D -> M, b: M -> X; keep a's pairs whose value ∈ dom b.
impl<A, B> Load for Restrict<A, B>
where
    A: Load,
    B: Load<D = A::R>,
    B::Out: Member,
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

// movie.with(title.eq("A").group_by(year).select(title)
// year.inv().select(title.eq("A"))

// movie.group_by(cast.select(person))
// year.inv().select(title)

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
        // 
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

// Drive `q` and print its rows sorted, then the count.
pub fn print_rows<E: 'static, Q>(q: &Q)
where
    Q: Drive<D = Id<E>>,
    Q::R: std::fmt::Debug + PartialOrd,
{
    let mut rows = Vec::new();
    q.drive(|k, v| rows.push((k.0, v)));
    rows.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    for (k, v) in &rows {
        println!("  {k} -> {v:?}");
    }
    println!("{} rows", rows.len());
}
