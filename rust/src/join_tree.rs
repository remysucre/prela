// Join-tree loader (examples/designPlan.md).
//
// One post-order pass over a hand-built join tree. Each node's file is read
// once, as selectively as its own predicates and its children's Bloom filters
// allow, and the surviving rows land in a `Sink`. Prela then runs the real
// query over those reduced relations; the joins themselves stay on the prela
// side. This is the only module that touches Parquet.
//
// Not here yet (on purpose): a top-down pass, joining while scanning,
// ordering filters by statistics, and joins on anything but one ID column.
use std::collections::HashMap;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BooleanArray, Float64Array, Int32Array, Int64Array, Scalar, StringArray,
};
use arrow::compute::cast;
use arrow::compute::kernels::boolean::{and, not, or};
use arrow::compute::kernels::cmp::{eq, gt, gt_eq, lt, lt_eq, neq};
use arrow::datatypes::DataType;
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use parquet::arrow::ProjectionMask;
use parquet::arrow::arrow_reader::{
    ArrowPredicate, ArrowPredicateFn, ArrowReaderOptions, ParquetRecordBatchReader,
    ParquetRecordBatchReaderBuilder, RowFilter,
};

// ===== Bloom ==============================================================

// A tiny Bloom filter over join keys: 3 probes into a bit array (same hashing
// as simple.rs). It may say "yes" for a key never inserted (false positive),
// never "no" for one that was, so prela must still do the exact join.
// Keys are i64 rather than i32: TPC-H's c_custkey is Int64 while the
// nationkeys are Int32, and widening covers both.
pub struct Bloom(Vec<u64>);

impl Bloom {
    // ~10 bits per row gives roughly a 1% false-positive rate with 3 probes.
    // Sized from the footer row count, i.e. as if every row survived.
    pub fn with_capacity(rows: usize) -> Self {
        Bloom(vec![0; (rows * 10).div_ceil(64).max(1)])
    }
    fn probes(&self, key: i64) -> [usize; 3] {
        let mut h = DefaultHasher::new();
        key.hash(&mut h);
        let h = h.finish();
        let (a, b) = (h as u32 as usize, (h >> 32) as usize | 1);
        let bits = self.0.len() * 64;
        [0, 1, 2].map(|i| a.wrapping_add(i * b) % bits)
    }
    pub fn insert(&mut self, key: i64) {
        for p in self.probes(key) {
            self.0[p / 64] |= 1 << (p % 64);
        }
    }
    pub fn insert_all(&mut self, keys: &[i64]) {
        for &k in keys {
            self.insert(k);
        }
    }
    pub fn has(&self, key: i64) -> bool {
        self.probes(key).iter().all(|&p| self.0[p / 64] >> (p % 64) & 1 == 1)
    }
    // Freeze it for sharing. If most bits are set it passes nearly every key,
    // so probing it would cost time and filter nothing.
    pub fn finish(self) -> Option<Arc<Bloom>> {
        let ones: usize = self.0.iter().map(|w| w.count_ones() as usize).sum();
        if ones * 2 > self.0.len() * 64 {
            None
        } else {
            Some(Arc::new(self))
        }
    }
}

// ===== PredicateExpr ======================================================

#[derive(Clone, Copy)]
pub enum CmpOp {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
}

// The literal's type must match the column's Arrow type (e.g. F64 for a
// Float64 column), since the comparison kernels do not coerce.
#[derive(Clone)]
pub enum Literal {
    I32(i32),
    I64(i64),
    F64(f64),
    Str(String),
}

impl Literal {
    fn scalar(&self) -> Scalar<ArrayRef> {
        let array: ArrayRef = match self {
            Literal::I32(v) => Arc::new(Int32Array::from(vec![*v])),
            Literal::I64(v) => Arc::new(Int64Array::from(vec![*v])),
            Literal::F64(v) => Arc::new(Float64Array::from(vec![*v])),
            Literal::Str(v) => Arc::new(StringArray::from(vec![v.as_str()])),
        };
        Scalar::new(array)
    }
}

#[derive(Clone)]
pub enum PredicateExpr {
    Cmp { col: String, op: CmpOp, literal: Literal },
    And(Vec<PredicateExpr>),
    Or(Vec<PredicateExpr>),
    Not(Box<PredicateExpr>),
    In { col: String, values: Vec<Literal> },
}

impl PredicateExpr {
    // Every column the expression reads, each once.
    fn columns(&self) -> Vec<String> {
        let mut cols = Vec::new();
        self.collect_columns(&mut cols);
        cols.sort();
        cols.dedup();
        cols
    }
    fn collect_columns(&self, out: &mut Vec<String>) {
        match self {
            PredicateExpr::Cmp { col, .. } | PredicateExpr::In { col, .. } => out.push(col.clone()),
            PredicateExpr::And(parts) | PredicateExpr::Or(parts) => {
                for p in parts {
                    p.collect_columns(out);
                }
            }
            PredicateExpr::Not(p) => p.collect_columns(out),
        }
    }

    // Evaluate with Arrow's vectorized kernels, one call per node of the
    // expression for the whole batch, not one closure call per row.
    fn eval(&self, batch: &RecordBatch) -> Result<BooleanArray, ArrowError> {
        match self {
            PredicateExpr::Cmp { col, op, literal } => {
                let c = column(batch, col);
                let s = literal.scalar();
                match op {
                    CmpOp::Eq => eq(c, &s),
                    CmpOp::NotEq => neq(c, &s),
                    CmpOp::Lt => lt(c, &s),
                    CmpOp::LtEq => lt_eq(c, &s),
                    CmpOp::Gt => gt(c, &s),
                    CmpOp::GtEq => gt_eq(c, &s),
                }
            }
            PredicateExpr::And(parts) => {
                let mut acc = parts[0].eval(batch)?;
                for p in &parts[1..] {
                    acc = and(&acc, &p.eval(batch)?)?;
                }
                Ok(acc)
            }
            PredicateExpr::Or(parts) => {
                let mut acc = parts[0].eval(batch)?;
                for p in &parts[1..] {
                    acc = or(&acc, &p.eval(batch)?)?;
                }
                Ok(acc)
            }
            PredicateExpr::Not(p) => not(&p.eval(batch)?),
            PredicateExpr::In { col, values } => {
                let c = column(batch, col);
                let mut acc = eq(c, &values[0].scalar())?;
                for v in &values[1..] {
                    acc = or(&acc, &eq(c, &v.scalar())?)?;
                }
                Ok(acc)
            }
        }
    }
}

// The batch handed to a predicate holds only its masked columns, in file
// order, so positions mean nothing here: always look columns up by name.
fn column<'a>(batch: &'a RecordBatch, name: &str) -> &'a ArrayRef {
    batch
        .column_by_name(name)
        .unwrap_or_else(|| panic!("column {name} not in batch"))
}

// Join keys as i64, whatever integer width the file stored them in.
fn keys_i64(col: &ArrayRef) -> Int64Array {
    let wide = cast(col, &DataType::Int64).unwrap();
    wide.as_any().downcast_ref::<Int64Array>().unwrap().clone()
}

// ===== ColumnFilter =======================================================

// "Take a batch, return a boolean per row". Predicates and Bloom filters both
// become one of these, so the scan treats them the same way.
pub struct ColumnFilter {
    pub columns: Vec<String>, // which columns it reads; its ProjectionMask is built from these
    pub eval: Box<dyn FnMut(&RecordBatch) -> Result<BooleanArray, ArrowError> + Send>,
}

impl ColumnFilter {
    pub fn from_expr(expr: PredicateExpr) -> Self {
        ColumnFilter {
            columns: expr.columns(),
            eval: Box::new(move |batch| expr.eval(batch)),
        }
    }

    // The Bloom is finished and immutable, so the closure owns a shared clone.
    pub fn bloom(column: &str, bloom: Arc<Bloom>) -> Self {
        let name = column.to_string();
        ColumnFilter {
            columns: vec![name.clone()],
            eval: Box::new(move |batch| {
                let keys = keys_i64(batch.column_by_name(&name).unwrap());
                Ok(keys.values().iter().map(|&k| Some(bloom.has(k))).collect())
            }),
        }
    }
}

// Node predicates are ANDed. Split nested top-level ANDs apart and regroup
// the conjuncts by the columns they read: one filter per column (set), so each
// gets a narrow mask and later filters only decode rows that already survived.
pub fn compile(predicates: &[PredicateExpr]) -> Vec<ColumnFilter> {
    let mut conjuncts = Vec::new();
    for p in predicates {
        split_and(p.clone(), &mut conjuncts);
    }
    let mut groups: Vec<(Vec<String>, Vec<PredicateExpr>)> = Vec::new();
    for c in conjuncts {
        let cols = c.columns();
        match groups.iter_mut().find(|(g, _)| *g == cols) {
            Some((_, group)) => group.push(c),
            None => groups.push((cols, vec![c])),
        }
    }
    let mut filters = Vec::new();
    for (_, mut group) in groups {
        let expr = if group.len() == 1 { group.pop().unwrap() } else { PredicateExpr::And(group) };
        filters.push(ColumnFilter::from_expr(expr));
    }
    filters
}

fn split_and(p: PredicateExpr, out: &mut Vec<PredicateExpr>) {
    match p {
        PredicateExpr::And(parts) => {
            for q in parts {
                split_and(q, out);
            }
        }
        other => out.push(other),
    }
}

// ===== scan ===============================================================

fn footer_rows(builder: &ParquetRecordBatchReaderBuilder<File>) -> usize {
    builder.metadata().file_metadata().num_rows() as usize
}

// A file's row count from its footer alone, without decoding any data.
pub fn row_count(path: &str) -> usize {
    let builder = ParquetRecordBatchReaderBuilder::try_new(File::open(path).expect(path)).unwrap();
    footer_rows(&builder)
}

// Column names -> a ProjectionMask over the file's root columns.
fn mask(builder: &ParquetRecordBatchReaderBuilder<File>, cols: &[String]) -> ProjectionMask {
    let idx = cols.iter().map(|c| builder.schema().index_of(c).unwrap());
    ProjectionMask::roots(builder.parquet_schema(), idx)
}

// Open `path` and read its footer once. Every filter gets a mask over just
// the columns it reads, and Parquet applies them in order while decoding, so
// only rows passing all of them come out, projected to `cols`. Also returns
// the footer row count, which sizes this node's Bloom.
pub fn scan(
    path: &str,
    filters: Vec<ColumnFilter>,
    cols: &[String],
) -> (ParquetRecordBatchReader, usize) {
    let file = File::open(path).expect(path);
    // The page index lets Parquet skip whole pages that a filter rules out.
    let options = ArrowReaderOptions::new().with_page_index(true);
    let builder = ParquetRecordBatchReaderBuilder::try_new_with_options(file, options).unwrap();
    let row_count = footer_rows(&builder);

    let mut predicates: Vec<Box<dyn ArrowPredicate>> = Vec::new();
    for ColumnFilter { columns, mut eval } in filters {
        let f = move |batch: RecordBatch| eval(&batch);
        predicates.push(Box::new(ArrowPredicateFn::new(mask(&builder, &columns), f)));
    }

    let out_mask = mask(&builder, cols);
    let reader = builder
        .with_projection(out_mask)
        .with_row_filter(RowFilter::new(predicates))
        .build()
        .unwrap();
    (reader, row_count)
}

// ===== The join tree ======================================================

// S JOIN T ON s.parent_key = t.child_key, where S is the node owning this
// edge and T is `child`. The child fills its Bloom with `child_key` values,
// and the parent probes it with `parent_key`.
pub struct Edge {
    pub child: Node,
    pub parent_key: String,
    pub child_key: String,
}

pub struct Node {
    pub path: String, // the Parquet file; its schema comes from the footer
    pub children: Vec<Edge>,
    pub predicates: Vec<PredicateExpr>,
    pub out_columns: Vec<String>, // what we actually need to produce
    pub materialize: bool,        // false if the node is a pure filter
}

// The reduced rows of every materialized node, keyed by file path (so a tree
// that reads the same file twice would merge them; fine for now).
#[derive(Default)]
pub struct Sink {
    pub tables: HashMap<String, Vec<RecordBatch>>,
}

impl Sink {
    pub fn push(&mut self, node: &Node, batch: RecordBatch) {
        self.tables.entry(node.path.clone()).or_default().push(batch);
    }
}

fn project(batch: &RecordBatch, cols: &[String]) -> RecordBatch {
    let idx: Vec<usize> = cols.iter().map(|c| batch.schema().index_of(c).unwrap()).collect();
    batch.project(&idx).unwrap()
}

// Post-order: load the node's reduced rows into `sink`, and return a Bloom
// over `parent_key` for the parent to filter its own scan with. Returns None
// for the root (no parent_key) or when the Bloom would not filter anything.
pub fn traverse(node: &Node, parent_key: Option<&str>, sink: &mut Sink) -> Option<Arc<Bloom>> {
    // Exact and cheap, so they go first in the RowFilter.
    let mut filters = compile(&node.predicates);

    for edge in &node.children {
        if let Some(bloom) = traverse(&edge.child, Some(&edge.child_key), sink) {
            filters.push(ColumnFilter::bloom(&edge.parent_key, bloom));
        }
    }
    // With no filters every row survives, and a Bloom over all keys rules out nothing.
    let selective = !filters.is_empty();

    // All selections are known now, so read this node as selectively as possible.
    let mut cols = node.out_columns.clone();
    cols.extend(parent_key.map(String::from));
    cols.extend(node.children.iter().map(|e| e.parent_key.clone()));
    let (reader, row_count) = scan(&node.path, filters, &cols);

    let mut bloom = match parent_key {
        Some(_) if selective => Some(Bloom::with_capacity(row_count)),
        _ => None,
    };
    for batch in reader {
        // Only rows passing all filters arrive here.
        let batch = batch.unwrap();
        if node.materialize {
            sink.push(node, project(&batch, &node.out_columns));
        }
        if let Some(bloom) = &mut bloom {
            let keys = keys_i64(batch.column_by_name(parent_key.unwrap()).unwrap());
            bloom.insert_all(keys.values());
        }
    }
    bloom.and_then(Bloom::finish)
}
