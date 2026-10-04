// Shared by every test query: where the data lives, short constructors for
// join trees, and the Sink -> prela relation conversion that tree_query.rs
// used to do by hand.
use std::sync::OnceLock;

use arrow::array::{Array, Float64Array, Int64Array, StringArray};
use arrow::compute::cast;
use arrow::datatypes::DataType;
use arrow::record_batch::RecordBatch;
use prela::engine::*;
use prela::join_tree::*;
use prela::loader::{Col, Str};
use prela::tpch_schema::{self, Tpch};

// Paths are relative to prela/rust, where cargo runs the example.
const PARQUET_DIR: &str = "../data/tpch/parquet";
const CACHE_DIR: &str = "../cache";

pub fn path(table: &str) -> String {
    format!("{PARQUET_DIR}/{table}.parquet")
}

// The whole cached schema for the baselines, loaded once and shared.
pub fn db() -> &'static Tpch {
    static DB: OnceLock<Tpch> = OnceLock::new();
    DB.get_or_init(|| tpch_schema::load(std::path::Path::new(CACHE_DIR)))
}

// ===== building join trees ================================================

pub fn names(cols: &[&str]) -> Vec<String> {
    cols.iter().map(|c| c.to_string()).collect()
}

// Every node materializes for now, since the prela query still probes each table.
pub fn node(table: &str, predicates: Vec<PredicateExpr>, out_columns: &[&str], children: Vec<Edge>) -> Node {
    Node {
        path: path(table),
        children,
        predicates,
        out_columns: names(out_columns),
        materialize: true,
    }
}

pub fn edge(child: Node, parent_key: &str, child_key: &str) -> Edge {
    Edge {
        child,
        parent_key: parent_key.to_string(),
        child_key: child_key.to_string(),
    }
}

pub fn cmp(col: &str, op: CmpOp, literal: Literal) -> PredicateExpr {
    PredicateExpr::Cmp { col: col.to_string(), op, literal }
}

// ===== Sink -> prela relations ============================================

// One table's batches from the sink. A table where no row survived has no
// entry, which is just an empty table.
pub fn batches<'a>(sink: &'a Sink, table: &str) -> &'a [RecordBatch] {
    sink.tables.get(&path(table)).map_or(&[], |b| b.as_slice())
}

// Rows pushed into the sink over the whole traversal: the tree's "rows loaded".
pub fn sink_rows(sink: &Sink) -> usize {
    sink.tables.values().flatten().map(|b| b.num_rows()).sum()
}

// One integer column across all of a table's batches, widened to i64
// (some keys are Int64 in the files, others Int32).
pub fn ints(batches: &[RecordBatch], name: &str) -> Vec<i64> {
    let mut out = Vec::new();
    for b in batches {
        let col = cast(b.column_by_name(name).unwrap(), &DataType::Int64).unwrap();
        out.extend(col.as_any().downcast_ref::<Int64Array>().unwrap().values().iter());
    }
    out
}

pub fn floats(batches: &[RecordBatch], name: &str) -> Vec<f64> {
    let mut out = Vec::new();
    for b in batches {
        let col = b.column_by_name(name).unwrap();
        out.extend(col.as_any().downcast_ref::<Float64Array>().unwrap().values().iter());
    }
    out
}

// Prela's string columns hold &'static str (they borrow from the leaked
// cache mmap), so the loaded strings are leaked too. Fine for a test run.
pub fn strs(batches: &[RecordBatch], name: &str) -> Vec<Str> {
    let mut out = Vec::new();
    for b in batches {
        let col = b.column_by_name(name).unwrap();
        for s in col.as_any().downcast_ref::<StringArray>().unwrap().iter() {
            out.push(&*Box::leak(s.unwrap().to_string().into_boxed_str()));
        }
    }
    out
}

// The shift regen applies when it builds the cache: nationkey and regionkey
// are 0-based in the files, every other key (custkey, ps_id, l_orderkey, ...)
// is 1-based. This holds for primary and foreign keys alike.
fn key_shift(col: &str) -> i64 {
    if col.ends_with("nationkey") || col.ends_with("regionkey") { 0 } else { -1 }
}

// A key column as prela ids, with the cache's shift applied.
pub fn ids(batches: &[RecordBatch], col: &str) -> Vec<usize> {
    let shift = key_shift(col);
    ints(batches, col).iter().map(|&k| (k + shift) as usize).collect()
}

// Same rule as the cache loader (regen's n_from_keys): max id + 1, here over
// the loaded rows only. Ids past it were never loaded, so nothing probes them.
pub fn domain(ids: &[usize]) -> usize {
    ids.iter().max().map_or(0, |&m| m + 1)
}

// VecRel is dense, so every slot below its length counts as a member. Drive
// from a SparseUniverse over the ids actually loaded instead, the way the
// loader handles Order's gappy ids. The Bitset is leaked because
// SparseUniverse holds a &'static, like loader::sparse_key.
pub fn key<E: 'static>(n: usize, ids: &[usize]) -> SparseUniverse<Id<E>> {
    let mut valid = Bitset::empty(Universe::<Id<E>>::new(n));
    for &i in ids {
        valid.set(Id::new(i));
    }
    SparseUniverse::new(n, Box::leak(Box::new(valid)))
}

// A foreign-key column; `targets` are already shifted (from `ids`). Slots for
// rows that were not loaded hold NO_ID, like regen's hole fill, so a hop
// through them reaches nothing.
pub fn id_col<E: 'static, T: 'static>(n: usize, ids: &[usize], targets: &[usize]) -> Col<E, Id<T>> {
    let mut v = vec![Id::<T>::new(NO_ID); n];
    for (&i, &t) in ids.iter().zip(targets) {
        v[i] = Id::new(t);
    }
    VecRel::new(v)
}

// A value column. Slots for rows that were not loaded hold R's default (0,
// 0.0, ""). Only a hop through a foreign key can reach such a slot, and only
// when a Bloom false positive let in a parent row whose child row was never
// loaded. So a predicate that the default satisfies (say `le(20)` on 0)
// could let that row through, and the test would FAIL. No query here relies
// on such a predicate alone: q_partsupp_branching's `p_size.le(..)` is
// ANDed with a brand check that "" never passes.
pub fn val_col<E: 'static, R: Copy + Default>(n: usize, ids: &[usize], vals: Vec<R>) -> Col<E, R> {
    VecRel::from_pairs(n, ids.iter().copied().zip(vals))
}

pub fn sorted(mut rows: Vec<String>) -> Vec<String> {
    rows.sort();
    rows
}
