// Regenerate the binary cache (format v2 — see src/format.rs) from parquet:
//
//   cargo run --release --features regen --bin regen -- job  [../../jobdata/parquet] [../cache]
//   cargo run --release --features regen --bin regen -- tpch [../cache/tpch] [../cache]
//
// regen absorbs ALL load-time transformation: ids are shifted to 0-based
// here, FK holes are filled with NO_ID, dates are parsed to yyyymmdd i64,
// strings are laid out as offsets+bytes, and multi-valued columns are
// built into CSR — the engine loaders (src/cache.rs) just mmap and bulk
// copy/slice. (The v1 pair-stream format, inherited from the retired
// Julia engine — julia-engine branch — is gone.)
//
// TPC-H expects clean DuckDB-exported parquet: all-BIGINT integer columns,
// DOUBLE for money, VARCHAR for strings (dates pre-formatted as ISO
// yyyy-mm-dd). It runs per-field passes through parquet (column
// projection) and writes each dense column immediately. JOB reads the
// imdb parquet export as-is (INT32 ids, nullable columns), reproducing
// the pair semantics of the retired Julia loader (julia-engine branch,
// JOB.jl `load_all!`): one pass per source table buffers every derived
// column's pairs in RAM; once all tables are read, the entity universe
// sizes are computed (the same max-id formulas the v1 runtime loader
// used) and each column is finalized to its dense/CSR layout and written.

#[path = "../format.rs"]
mod format;

use format::*;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use arrow::array::{Array, Float64Array, Int32Array, Int64Array, LargeStringArray, StringArray};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::{ParquetRecordBatchReader, ParquetRecordBatchReaderBuilder};
use parquet::arrow::ProjectionMask;

macro_rules! t {
    ($s:expr) => {{
        let start = std::time::Instant::now();
        $s;
        eprintln!("    {:.2}s", start.elapsed().as_secs_f32());
    }};
}

// ===== file writing ======================================================

fn out_file(path: &Path, kind: u32, n: usize, m: usize) -> BufWriter<File> {
    let mut f = BufWriter::new(File::create(path).unwrap());
    f.write_all(&header(kind, n as u64, m as u64)).unwrap();
    f
}

fn write_words(f: &mut BufWriter<File>, words: &[u64]) {
    let bytes =
        unsafe { std::slice::from_raw_parts(words.as_ptr() as *const u8, words.len() * 8) };
    f.write_all(bytes).unwrap();
}

fn write_u32s(f: &mut BufWriter<File>, vals: &[u32]) {
    let bytes =
        unsafe { std::slice::from_raw_parts(vals.as_ptr() as *const u8, vals.len() * 4) };
    f.write_all(bytes).unwrap();
}

/// Zero-pad from the end of a (n+1)×u32 offsets section to the 8-aligned
/// start of a CSR words payload.
fn pad_after_offsets(f: &mut BufWriter<File>, n: usize) {
    let end = HEADER_LEN + (n + 1) * 4;
    f.write_all(&vec![0u8; align8(end) - end]).unwrap();
}

// ===== column buffers ====================================================
// Pairs are buffered with INTERNAL (0-based) u32 keys; values are 8-byte
// words — a 0-based id, a raw i64 bit pattern, or an f64 bit pattern
// (the finalize call's kind says which). The finalizers reproduce the v1
// runtime loader's semantics exactly: dense scatter is last-write-wins
// and panics on a key outside the universe (v1 `VecRel::from_pairs`);
// CSR drops out-of-universe keys and keeps per-key stream order
// (v1 `MultiRel::from_pairs`).

fn internal_key(k: i64) -> u32 {
    debug_assert!((0..u32::MAX as i64).contains(&k), "key {k} out of u32 range");
    k as u32
}

/// Word-valued column buffer.
struct ColW {
    pairs: Vec<(u32, u64)>,
}

impl ColW {
    fn new() -> Self {
        ColW { pairs: Vec::new() }
    }
    /// Push with an INTERNAL (already 0-based) key.
    fn push(&mut self, k: i64, word: u64) {
        self.pairs.push((internal_key(k), word));
    }
    /// Universe-size contribution of the keys: max internal key + 1.
    fn n_from_keys(&self) -> usize {
        self.pairs.iter().map(|&(k, _)| k as usize + 1).max().unwrap_or(0)
    }
    /// Universe-size contribution of id-valued words: max id + 1.
    fn n_from_vals(&self) -> usize {
        self.pairs.iter().map(|&(_, v)| v as usize + 1).max().unwrap_or(0)
    }

    /// Dense column: one word per id 0..n, holes = `fill`, last write wins.
    fn write_dense(self, out: &Path, n: usize, kind: u32, fill: u64) {
        let mut arr = vec![fill; n];
        for (k, v) in self.pairs {
            arr[k as usize] = v; // out-of-universe key = data anomaly: panic
        }
        let mut f = out_file(out, kind, n, 0);
        write_words(&mut f, &arr);
    }

    /// CSR multi column (kind 3).
    fn write_csr(self, out: &Path, n: usize) {
        let mut offsets = vec![0u32; n + 1];
        let mut m = 0usize;
        for &(k, _) in &self.pairs {
            if (k as usize) < n {
                offsets[k as usize + 1] += 1;
                m += 1;
            }
        }
        for i in 1..=n {
            offsets[i] += offsets[i - 1];
        }
        let mut next = offsets.clone();
        let mut values = vec![0u64; m];
        for &(k, v) in &self.pairs {
            if (k as usize) < n {
                let p = &mut next[k as usize];
                values[*p as usize] = v;
                *p += 1;
            }
        }
        let mut f = out_file(out, KIND_CSR_WORDS, n, m);
        write_u32s(&mut f, &offsets);
        pad_after_offsets(&mut f, n);
        write_words(&mut f, &values);
    }
}

/// String-valued column buffer (keys + arena of bytes).
struct ColS {
    keys: Vec<u32>,
    offs: Vec<u32>, // len = keys.len() + 1
    bytes: Vec<u8>,
}

impl ColS {
    fn new() -> Self {
        ColS { keys: Vec::new(), offs: vec![0], bytes: Vec::new() }
    }
    /// Push with an INTERNAL (already 0-based) key.
    fn push(&mut self, k: i64, s: &str) {
        self.keys.push(internal_key(k));
        self.bytes.extend_from_slice(s.as_bytes());
        self.offs.push(u32::try_from(self.bytes.len()).expect("string column > 4 GB"));
    }
    fn n_from_keys(&self) -> usize {
        self.keys.iter().map(|&k| k as usize + 1).max().unwrap_or(0)
    }
    fn str_at(&self, j: usize) -> &[u8] {
        &self.bytes[self.offs[j] as usize..self.offs[j + 1] as usize]
    }

    /// Dense string column (kind 2): one string per id, holes = "".
    fn write_dense(self, out: &Path, n: usize) {
        let mut chosen = vec![u32::MAX; n];
        for (j, &k) in self.keys.iter().enumerate() {
            chosen[k as usize] = j as u32; // last write wins; OOU key panics
        }
        let mut offsets = Vec::with_capacity(n + 1);
        let mut data = Vec::new();
        offsets.push(0u32);
        for &c in &chosen {
            if c != u32::MAX {
                data.extend_from_slice(self.str_at(c as usize));
            }
            offsets.push(u32::try_from(data.len()).expect("string column > 4 GB"));
        }
        let mut f = out_file(out, KIND_DENSE_STR, n, data.len());
        write_u32s(&mut f, &offsets);
        f.write_all(&data).unwrap();
    }

    /// CSR string column (kind 4).
    fn write_csr(self, out: &Path, n: usize) {
        let mut row_off = vec![0u32; n + 1];
        let mut m = 0usize;
        for &k in &self.keys {
            if (k as usize) < n {
                row_off[k as usize + 1] += 1;
                m += 1;
            }
        }
        for i in 1..=n {
            row_off[i] += row_off[i - 1];
        }
        // stable scatter of string indices into CSR order
        let mut next = row_off.clone();
        let mut order = vec![0u32; m];
        for (j, &k) in self.keys.iter().enumerate() {
            if (k as usize) < n {
                let p = &mut next[k as usize];
                order[*p as usize] = j as u32;
                *p += 1;
            }
        }
        let mut str_off = Vec::with_capacity(m + 1);
        let mut data = Vec::new();
        str_off.push(0u32);
        for &j in &order {
            data.extend_from_slice(self.str_at(j as usize));
            str_off.push(u32::try_from(data.len()).expect("string column > 4 GB"));
        }
        let mut f = out_file(out, KIND_CSR_STR, n, m);
        write_u32s(&mut f, &row_off);
        write_u32s(&mut f, &str_off);
        f.write_all(&data).unwrap();
    }
}

// ===== parquet access ====================================================

/// Open a parquet file projecting the given 0-based column indices.
/// Returns the reader plus, for each requested index, its position within
/// the projected batches (projection preserves file order, not request
/// order).
fn open_cols(path: &Path, indices: &[usize]) -> (ParquetRecordBatchReader, Vec<usize>) {
    let file = File::open(path).unwrap_or_else(|e| panic!("open {path:?}: {e}"));
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
    let schema = builder.parquet_schema();
    let mut sorted: Vec<usize> = indices.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mask = ProjectionMask::roots(schema, sorted.iter().copied());
    let pos = indices.iter().map(|i| sorted.binary_search(i).unwrap()).collect();
    (builder.with_projection(mask).build().unwrap(), pos)
}

/// Like `open_cols` but by column NAME (the TPC-H parquet has stable names).
fn open_named(path: &Path, columns: &[&str]) -> (ParquetRecordBatchReader, Vec<usize>) {
    let file = File::open(path).unwrap_or_else(|e| panic!("open {path:?}: {e}"));
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
    let schema = builder.parquet_schema();
    let indices: Vec<usize> = columns
        .iter()
        .map(|name| {
            (0..schema.num_columns())
                .find(|&i| schema.column(i).name() == *name)
                .unwrap_or_else(|| panic!("column {name} not found in {path:?}"))
        })
        .collect();
    drop(builder);
    open_cols(path, &indices)
}

enum IntCol<'a> {
    I32(&'a Int32Array),
    I64(&'a Int64Array),
}

impl IntCol<'_> {
    fn get(&self, i: usize) -> Option<i64> {
        match self {
            IntCol::I32(a) => (!a.is_null(i)).then(|| a.value(i) as i64),
            IntCol::I64(a) => (!a.is_null(i)).then(|| a.value(i)),
        }
    }
}

fn int_col<'a>(batch: &'a RecordBatch, pos: usize) -> IntCol<'a> {
    let col = batch.column(pos);
    if let Some(a) = col.as_any().downcast_ref::<Int32Array>() {
        IntCol::I32(a)
    } else if let Some(a) = col.as_any().downcast_ref::<Int64Array>() {
        IntCol::I64(a)
    } else {
        panic!("column {pos}: expected Int32/Int64, got {:?}", col.data_type())
    }
}

enum StrCol<'a> {
    Utf8(&'a StringArray),
    Large(&'a LargeStringArray),
}

impl StrCol<'_> {
    fn get(&self, i: usize) -> Option<&str> {
        match self {
            StrCol::Utf8(a) => (!a.is_null(i)).then(|| a.value(i)),
            StrCol::Large(a) => (!a.is_null(i)).then(|| a.value(i)),
        }
    }
}

fn str_col<'a>(batch: &'a RecordBatch, pos: usize) -> StrCol<'a> {
    let col = batch.column(pos);
    if let Some(a) = col.as_any().downcast_ref::<StringArray>() {
        StrCol::Utf8(a)
    } else if let Some(a) = col.as_any().downcast_ref::<LargeStringArray>() {
        StrCol::Large(a)
    } else {
        panic!("column {pos}: expected Utf8/LargeUtf8, got {:?}", col.data_type())
    }
}

// ======================== TPC-H ========================
//
// All columns are dense; every parquet row contributes a pair (NULL
// strings become "", matching the v1 writer). Key spaces: suppkey /
// custkey / partkey / orderkey / synthetic ps_id / l_id are 1-based in
// the parquet (internal = raw − 1); regionkey / nationkey are 0-based
// (internal = raw). The orderkey space is sparse — the dense scatter
// fills the holes (NO_ID for FKs, 0/"" otherwise).

/// "YYYY-MM-DD" → packed i64 YYYYMMDD (numeric compare preserves lexical
/// order). The runtime never parses dates — only this regen path does.
fn parse_yyyymmdd(s: &str) -> i64 {
    if s.is_empty() {
        return 0;
    }
    let b = s.as_bytes();
    assert_eq!(b.len(), 10, "bad date {s:?}");
    let d = |i: usize| (b[i] - b'0') as i64;
    d(0) * 10_000_000 + d(1) * 1_000_000 + d(2) * 100_000 + d(3) * 10_000
        + d(5) * 1000 + d(6) * 100
        + d(8) * 10 + d(9)
}

/// What a TPC-H value column holds → how it becomes an 8-byte word.
enum TVal {
    Id { delta: i64 }, // FK: internal id = raw + delta
    I64,
    F64,
    Date,
}

/// One per-field pass: read (key, val), buffer, write the dense column.
/// `key_delta` maps the raw key to the internal id (−1 for 1-based keys,
/// 0 for regionkey/nationkey).
fn tpch_dense(out: &Path, parquet: &Path, key: &str, val: &str, key_delta: i64, tv: TVal) {
    let (reader, pos) = open_named(parquet, &[key, val]);
    let mut col = ColW::new();
    let mut kind = KIND_DENSE_I64;
    let mut fill = 0u64;
    for batch in reader {
        let batch = batch.unwrap();
        let k = int_col(&batch, pos[0]);
        match &tv {
            TVal::Id { delta } => {
                fill = NO_ID_WORD;
                let v = int_col(&batch, pos[1]);
                for i in 0..batch.num_rows() {
                    col.push(k.get(i).unwrap() + key_delta, (v.get(i).unwrap() + delta) as u64);
                }
            }
            TVal::I64 => {
                let v = int_col(&batch, pos[1]);
                for i in 0..batch.num_rows() {
                    col.push(k.get(i).unwrap() + key_delta, v.get(i).unwrap() as u64);
                }
            }
            TVal::F64 => {
                kind = KIND_DENSE_F64;
                let v = batch.column(pos[1]).as_any().downcast_ref::<Float64Array>()
                    .unwrap_or_else(|| panic!("{val}: expected Float64"));
                for i in 0..batch.num_rows() {
                    col.push(k.get(i).unwrap() + key_delta, v.value(i).to_bits());
                }
            }
            TVal::Date => {
                let v = str_col(&batch, pos[1]);
                for i in 0..batch.num_rows() {
                    let d = parse_yyyymmdd(v.get(i).unwrap_or(""));
                    col.push(k.get(i).unwrap() + key_delta, d as u64);
                }
            }
        }
    }
    let n = col.n_from_keys();
    col.write_dense(out, n, kind, fill);
}

/// One per-field pass for a dense string column.
fn tpch_dense_str(out: &Path, parquet: &Path, key: &str, val: &str, key_delta: i64) {
    let (reader, pos) = open_named(parquet, &[key, val]);
    let mut col = ColS::new();
    for batch in reader {
        let batch = batch.unwrap();
        let k = int_col(&batch, pos[0]);
        let v = str_col(&batch, pos[1]);
        for i in 0..batch.num_rows() {
            col.push(k.get(i).unwrap() + key_delta, v.get(i).unwrap_or(""));
        }
    }
    let n = col.n_from_keys();
    col.write_dense(out, n);
}

fn run_tpch(parquet_dir: &Path, cache_dir: &Path) {
    std::fs::create_dir_all(cache_dir).unwrap();

    let p = |name: &str| parquet_dir.join(format!("{name}.parquet"));
    let o = |name: &str| cache_dir.join(format!("{name}.bin"));
    use TVal::*;

    // Region / Nation: 0-based keys in the parquet (internal = raw).
    eprintln!("region");
    t!(tpch_dense_str(&o("Region_name"),    &p("region"), "r_regionkey", "r_name",    0));
    t!(tpch_dense_str(&o("Region_comment"), &p("region"), "r_regionkey", "r_comment", 0));

    eprintln!("nation");
    t!(tpch_dense_str(&o("Nation_name"),    &p("nation"), "n_nationkey", "n_name",    0));
    t!(tpch_dense(&o("Nation_region"),      &p("nation"), "n_nationkey", "n_regionkey", 0, Id { delta: 0 }));
    t!(tpch_dense_str(&o("Nation_comment"), &p("nation"), "n_nationkey", "n_comment", 0));

    // Supplier (1-based suppkey)
    eprintln!("supplier");
    t!(tpch_dense_str(&o("Supplier_name"),    &p("supplier"), "s_suppkey", "s_name",    -1));
    t!(tpch_dense_str(&o("Supplier_address"), &p("supplier"), "s_suppkey", "s_address", -1));
    t!(tpch_dense(&o("Supplier_nation"),      &p("supplier"), "s_suppkey", "s_nationkey", -1, Id { delta: 0 }));
    t!(tpch_dense_str(&o("Supplier_phone"),   &p("supplier"), "s_suppkey", "s_phone",   -1));
    t!(tpch_dense(&o("Supplier_acctbal"),     &p("supplier"), "s_suppkey", "s_acctbal", -1, F64));
    t!(tpch_dense_str(&o("Supplier_comment"), &p("supplier"), "s_suppkey", "s_comment", -1));

    // Customer
    eprintln!("customer");
    t!(tpch_dense_str(&o("Customer_name"),       &p("customer"), "c_custkey", "c_name",       -1));
    t!(tpch_dense_str(&o("Customer_address"),    &p("customer"), "c_custkey", "c_address",    -1));
    t!(tpch_dense(&o("Customer_nation"),         &p("customer"), "c_custkey", "c_nationkey", -1, Id { delta: 0 }));
    t!(tpch_dense_str(&o("Customer_phone"),      &p("customer"), "c_custkey", "c_phone",      -1));
    t!(tpch_dense(&o("Customer_acctbal"),        &p("customer"), "c_custkey", "c_acctbal",    -1, F64));
    t!(tpch_dense_str(&o("Customer_mktsegment"), &p("customer"), "c_custkey", "c_mktsegment", -1));
    t!(tpch_dense_str(&o("Customer_comment"),    &p("customer"), "c_custkey", "c_comment",    -1));

    // Part
    eprintln!("part");
    t!(tpch_dense_str(&o("Part_name"),      &p("part"), "p_partkey", "p_name",      -1));
    t!(tpch_dense_str(&o("Part_mfgr"),      &p("part"), "p_partkey", "p_mfgr",      -1));
    t!(tpch_dense_str(&o("Part_brand"),     &p("part"), "p_partkey", "p_brand",     -1));
    t!(tpch_dense_str(&o("Part_type"),      &p("part"), "p_partkey", "p_type",      -1));
    t!(tpch_dense(&o("Part_size"),          &p("part"), "p_partkey", "p_size",      -1, I64));
    t!(tpch_dense_str(&o("Part_container"), &p("part"), "p_partkey", "p_container", -1));
    t!(tpch_dense(&o("Part_retailprice"),   &p("part"), "p_partkey", "p_retailprice", -1, F64));
    t!(tpch_dense_str(&o("Part_comment"),   &p("part"), "p_partkey", "p_comment",   -1));

    // PartSupp (synthetic ps_id 1..N; part/supplier FKs 1-based)
    eprintln!("partsupp");
    t!(tpch_dense(&o("PartSupp_part"),       &p("partsupp"), "ps_id", "ps_partkey",   -1, Id { delta: -1 }));
    t!(tpch_dense(&o("PartSupp_supplier"),   &p("partsupp"), "ps_id", "ps_suppkey",   -1, Id { delta: -1 }));
    t!(tpch_dense(&o("PartSupp_availqty"),   &p("partsupp"), "ps_id", "ps_availqty",  -1, I64));
    t!(tpch_dense(&o("PartSupp_supplycost"), &p("partsupp"), "ps_id", "ps_supplycost", -1, F64));
    t!(tpch_dense_str(&o("PartSupp_comment"), &p("partsupp"), "ps_id", "ps_comment",   -1));

    // Orders (sparse orderkey — dense files carry the holes)
    eprintln!("orders");
    t!(tpch_dense(&o("Order_customer"),     &p("orders"), "o_orderkey", "o_custkey",     -1, Id { delta: -1 }));
    t!(tpch_dense_str(&o("Order_status"),   &p("orders"), "o_orderkey", "o_orderstatus", -1));
    t!(tpch_dense(&o("Order_totalprice"),   &p("orders"), "o_orderkey", "o_totalprice",  -1, F64));
    t!(tpch_dense(&o("Order_date"),         &p("orders"), "o_orderkey", "o_orderdate",   -1, Date));
    t!(tpch_dense_str(&o("Order_priority"), &p("orders"), "o_orderkey", "o_orderpriority", -1));
    t!(tpch_dense_str(&o("Order_clerk"),    &p("orders"), "o_orderkey", "o_clerk",       -1));
    t!(tpch_dense(&o("Order_shippriority"), &p("orders"), "o_orderkey", "o_shippriority", -1, I64));
    t!(tpch_dense_str(&o("Order_comment"),  &p("orders"), "o_orderkey", "o_comment",     -1));

    // Lineitem (synthetic l_id 1..N)
    eprintln!("lineitem");
    t!(tpch_dense(&o("Lineitem_order"),         &p("lineitem"), "l_id", "l_orderkey",     -1, Id { delta: -1 }));
    t!(tpch_dense(&o("Lineitem_part"),          &p("lineitem"), "l_id", "l_partkey",      -1, Id { delta: -1 }));
    t!(tpch_dense(&o("Lineitem_supplier"),      &p("lineitem"), "l_id", "l_suppkey",      -1, Id { delta: -1 }));
    t!(tpch_dense(&o("Lineitem_number"),        &p("lineitem"), "l_id", "l_linenumber",   -1, I64));
    t!(tpch_dense(&o("Lineitem_quantity"),      &p("lineitem"), "l_id", "l_quantity",     -1, F64));
    t!(tpch_dense(&o("Lineitem_extendedprice"), &p("lineitem"), "l_id", "l_extendedprice", -1, F64));
    t!(tpch_dense(&o("Lineitem_discount"),      &p("lineitem"), "l_id", "l_discount",     -1, F64));
    t!(tpch_dense(&o("Lineitem_tax"),           &p("lineitem"), "l_id", "l_tax",          -1, F64));
    t!(tpch_dense_str(&o("Lineitem_returnflag"), &p("lineitem"), "l_id", "l_returnflag",  -1));
    t!(tpch_dense_str(&o("Lineitem_status"),    &p("lineitem"), "l_id", "l_linestatus",   -1));
    t!(tpch_dense(&o("Lineitem_shipdate"),      &p("lineitem"), "l_id", "l_shipdate",     -1, Date));
    t!(tpch_dense(&o("Lineitem_commitdate"),    &p("lineitem"), "l_id", "l_commitdate",   -1, Date));
    t!(tpch_dense(&o("Lineitem_receiptdate"),   &p("lineitem"), "l_id", "l_receiptdate",  -1, Date));
    t!(tpch_dense_str(&o("Lineitem_shipinstruct"), &p("lineitem"), "l_id", "l_shipinstruct", -1));
    t!(tpch_dense_str(&o("Lineitem_shipmode"),  &p("lineitem"), "l_id", "l_shipmode",     -1));
    t!(tpch_dense_str(&o("Lineitem_comment"),   &p("lineitem"), "l_id", "l_comment",      -1));
}

// ======================== JOB ========================
//
// Pair semantics preserved from the retired Julia loader (julia-engine
// branch, JOB.jl `load_all!`):
//
//   - pairs are emitted in parquet row order;
//   - a pair is skipped iff its key or its value is NULL (per-column
//     independence: a cast_info row with a NULL note still contributes to
//     the other cast columns);
//   - Company name/country come from a company_name lookup Dict
//     (last-write-wins on duplicate keys; lookup misses skip the pair);
//   - ids are 1-based in the parquet; the −1 shift to internal ids
//     happens HERE (push sites), not at engine load time;
//   - entity universe sizes use the same max-id formulas the v1 runtime
//     loader used, so dense hole-filling and CSR out-of-range dropping
//     reproduce the v1 in-memory state bit-for-bit.

/// All JOB column buffers, filled by the table passes below and written
/// out once the universe sizes are known.
#[derive(Default)]
struct Job {
    // title → Movie
    movie_title: Option<ColS>,
    movie_kind: Option<ColW>,
    movie_production_year: Option<ColW>,
    movie_episode_nr: Option<ColW>,
    // keyword / movie_keyword
    keyword_keyword: Option<ColS>,
    movie_keyword: Option<ColW>,
    // kind_type / role_type / char_name / comp_cast_type / info_type / link_type
    kind_kind: Option<ColS>,
    roletype_role: Option<ColS>,
    character_name: Option<ColS>,
    compcasttype_kind: Option<ColS>,
    infotype_info: Option<ColS>,
    linktype_link: Option<ColS>,
    companytype_kind: Option<ColS>,
    // movie_companies → Company
    movie_company: Option<ColW>,
    company_name: Option<ColS>,
    company_country: Option<ColS>,
    company_note: Option<ColS>,
    company_type: Option<ColW>,
    // movie_info → Info
    movie_info: Option<ColW>,
    info_info: Option<ColS>,
    info_type: Option<ColW>,
    info_note: Option<ColS>,
    // movie_info_idx → Data
    movie_data: Option<ColW>,
    data_data: Option<ColS>,
    data_type: Option<ColW>,
    // movie_link → MovieLink
    movie_link: Option<ColW>,
    movie_linked_by: Option<ColW>,
    movielink_target: Option<ColW>,
    movielink_type: Option<ColW>,
    // aka_title / aka_name
    movie_aka: Option<ColW>,
    akatitle_title: Option<ColS>,
    person_aka: Option<ColW>,
    akaname_name: Option<ColS>,
    // name → Person
    person_name: Option<ColS>,
    person_gender: Option<ColS>,
    person_name_pcode: Option<ColS>,
    // complete_cast
    movie_complete_cast: Option<ColW>,
    completecast_subject: Option<ColW>,
    completecast_status: Option<ColW>,
    // person_info → PersonInfo
    person_info: Option<ColW>,
    personinfo_type: Option<ColW>,
    personinfo_info: Option<ColS>,
    personinfo_note: Option<ColS>,
    // cast_info → Cast
    movie_cast: Option<ColW>,
    cast_person: Option<ColW>,
    cast_character: Option<ColW>,
    cast_role: Option<ColW>,
    cast_note: Option<ColS>,
}

/// Two int columns (1-based ids) → ColW of internal-id pairs. Pair
/// emitted iff both values are non-NULL.
fn job_ids(parquet: &Path, key_idx: usize, val_idx: usize) -> ColW {
    let (reader, pos) = open_cols(parquet, &[key_idx, val_idx]);
    let mut w = ColW::new();
    for batch in reader {
        let batch = batch.unwrap();
        let k = int_col(&batch, pos[0]);
        let v = int_col(&batch, pos[1]);
        for i in 0..batch.num_rows() {
            if let (Some(k), Some(v)) = (k.get(i), v.get(i)) {
                w.push(k - 1, (v - 1) as u64);
            }
        }
    }
    w
}

/// Int key column (1-based id) + string column → ColS. Same rule.
fn job_strs(parquet: &Path, key_idx: usize, val_idx: usize) -> ColS {
    let (reader, pos) = open_cols(parquet, &[key_idx, val_idx]);
    let mut w = ColS::new();
    for batch in reader {
        let batch = batch.unwrap();
        let k = int_col(&batch, pos[0]);
        let v = str_col(&batch, pos[1]);
        for i in 0..batch.num_rows() {
            if let (Some(k), Some(v)) = (k.get(i), v.get(i)) {
                w.push(k - 1, v);
            }
        }
    }
    w
}

fn read_job(parquet_dir: &Path) -> Job {
    let p = |name: &str| parquet_dir.join(format!("{name}.parquet"));
    let mut j = Job::default();

    // ---- title (Movie): id, title, kind_id(3), production_year(4), episode_nr(9) ----
    eprintln!("title");
    t!({
        let (reader, pos) = open_cols(&p("title"), &[0, 1, 3, 4, 9]);
        let mut title = ColS::new();
        let mut kind = ColW::new();
        let mut year = ColW::new();
        let mut epnr = ColW::new();
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let ti = str_col(&batch, pos[1]);
            let kd = int_col(&batch, pos[2]);
            let py = int_col(&batch, pos[3]);
            let ep = int_col(&batch, pos[4]);
            for i in 0..batch.num_rows() {
                let Some(id) = id.get(i) else { continue };
                let id = id - 1;
                if let Some(v) = ti.get(i) { title.push(id, v); }
                if let Some(v) = kd.get(i) { kind.push(id, (v - 1) as u64); }
                if let Some(v) = py.get(i) { year.push(id, v as u64); }
                if let Some(v) = ep.get(i) { epnr.push(id, v as u64); }
            }
        }
        j.movie_title = Some(title);
        j.movie_kind = Some(kind);
        j.movie_production_year = Some(year);
        j.movie_episode_nr = Some(epnr);
    });

    // ---- small lookup tables ----
    eprintln!("kind_type / role_type / char_name / info_type / link_type / comp_cast_type / company_type");
    t!({
        j.kind_kind = Some(job_strs(&p("kind_type"), 0, 1));
        j.roletype_role = Some(job_strs(&p("role_type"), 0, 1));
        j.character_name = Some(job_strs(&p("char_name"), 0, 1));
        j.infotype_info = Some(job_strs(&p("info_type"), 0, 1));
        j.linktype_link = Some(job_strs(&p("link_type"), 0, 1));
        j.compcasttype_kind = Some(job_strs(&p("comp_cast_type"), 0, 1));
        j.companytype_kind = Some(job_strs(&p("company_type"), 0, 1));
    });

    // ---- keyword + movie_keyword ----
    eprintln!("keyword");
    t!({
        j.keyword_keyword = Some(job_strs(&p("keyword"), 0, 1));
        j.movie_keyword = Some(job_ids(&p("movie_keyword"), 1, 2));
    });

    // ---- company_name + movie_companies → Company ----
    // Company entities are movie_companies rows, with name/country joined
    // from company_name via id lookup (miss ⇒ skip the pair).
    eprintln!("company");
    t!({
        let mut cn_name: HashMap<i64, String> = HashMap::new();
        let mut cn_country: HashMap<i64, String> = HashMap::new();
        let (reader, pos) = open_cols(&p("company_name"), &[0, 1, 2]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let nm = str_col(&batch, pos[1]);
            let cc = str_col(&batch, pos[2]);
            for i in 0..batch.num_rows() {
                let Some(id) = id.get(i) else { continue };
                if let Some(v) = nm.get(i) { cn_name.insert(id, v.to_string()); }
                if let Some(v) = cc.get(i) { cn_country.insert(id, v.to_string()); }
            }
        }

        // movie_companies: id, movie_id, company_id, company_type_id, note
        let (reader, pos) = open_cols(&p("movie_companies"), &[0, 1, 2, 3, 4]);
        let mut mv_co = ColW::new();
        let mut co_name = ColS::new();
        let mut co_country = ColS::new();
        let mut co_type = ColW::new();
        let mut co_note = ColS::new();
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let mv = int_col(&batch, pos[1]);
            let co = int_col(&batch, pos[2]);
            let ct = int_col(&batch, pos[3]);
            let nt = str_col(&batch, pos[4]);
            for i in 0..batch.num_rows() {
                let Some(cid) = id.get(i) else { continue };
                let cid = cid - 1;
                if let Some(mid) = mv.get(i) { mv_co.push(mid - 1, cid as u64); }
                if let Some(cn) = co.get(i) {
                    if let Some(nm) = cn_name.get(&cn) { co_name.push(cid, nm); }
                    if let Some(cc) = cn_country.get(&cn) { co_country.push(cid, cc); }
                }
                if let Some(v) = ct.get(i) { co_type.push(cid, (v - 1) as u64); }
                if let Some(v) = nt.get(i) { co_note.push(cid, v); }
            }
        }
        j.movie_company = Some(mv_co);
        j.company_name = Some(co_name);
        j.company_country = Some(co_country);
        j.company_type = Some(co_type);
        j.company_note = Some(co_note);
    });

    // ---- movie_info → Info: id, movie_id, info_type_id, info, note ----
    eprintln!("movie_info");
    t!({
        let (reader, pos) = open_cols(&p("movie_info"), &[0, 1, 2, 3, 4]);
        let mut mv_info = ColW::new();
        let mut info_type = ColW::new();
        let mut info_text = ColS::new();
        let mut info_note = ColS::new();
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let mv = int_col(&batch, pos[1]);
            let ty = int_col(&batch, pos[2]);
            let tx = str_col(&batch, pos[3]);
            let nt = str_col(&batch, pos[4]);
            for i in 0..batch.num_rows() {
                let Some(iid) = id.get(i) else { continue };
                let iid = iid - 1;
                if let Some(mid) = mv.get(i) { mv_info.push(mid - 1, iid as u64); }
                if let Some(v) = ty.get(i) { info_type.push(iid, (v - 1) as u64); }
                if let Some(v) = tx.get(i) { info_text.push(iid, v); }
                if let Some(v) = nt.get(i) { info_note.push(iid, v); }
            }
        }
        j.movie_info = Some(mv_info);
        j.info_type = Some(info_type);
        j.info_info = Some(info_text);
        j.info_note = Some(info_note);
    });

    // ---- movie_info_idx → Data: id, movie_id, info_type_id, info ----
    eprintln!("movie_info_idx");
    t!({
        let (reader, pos) = open_cols(&p("movie_info_idx"), &[0, 1, 2, 3]);
        let mut mv_data = ColW::new();
        let mut data_type = ColW::new();
        let mut data_text = ColS::new();
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let mv = int_col(&batch, pos[1]);
            let ty = int_col(&batch, pos[2]);
            let dx = str_col(&batch, pos[3]);
            for i in 0..batch.num_rows() {
                let Some(did) = id.get(i) else { continue };
                let did = did - 1;
                if let Some(mid) = mv.get(i) { mv_data.push(mid - 1, did as u64); }
                if let Some(v) = ty.get(i) { data_type.push(did, (v - 1) as u64); }
                if let Some(v) = dx.get(i) { data_text.push(did, v); }
            }
        }
        j.movie_data = Some(mv_data);
        j.data_type = Some(data_type);
        j.data_data = Some(data_text);
    });

    // ---- movie_link → MovieLink: id, movie_id, linked_movie_id, link_type_id ----
    eprintln!("movie_link");
    t!({
        let (reader, pos) = open_cols(&p("movie_link"), &[0, 1, 2, 3]);
        let mut mv_link = ColW::new();
        let mut mv_linked_by = ColW::new();
        let mut ml_target = ColW::new();
        let mut ml_type = ColW::new();
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let sr = int_col(&batch, pos[1]);
            let tg = int_col(&batch, pos[2]);
            let lt = int_col(&batch, pos[3]);
            for i in 0..batch.num_rows() {
                let Some(mlid) = id.get(i) else { continue };
                let mlid = mlid - 1;
                if let Some(src) = sr.get(i) { mv_link.push(src - 1, mlid as u64); }
                if let Some(tgt) = tg.get(i) {
                    mv_linked_by.push(tgt - 1, mlid as u64);
                    ml_target.push(mlid, (tgt - 1) as u64);
                }
                if let Some(v) = lt.get(i) { ml_type.push(mlid, (v - 1) as u64); }
            }
        }
        j.movie_link = Some(mv_link);
        j.movie_linked_by = Some(mv_linked_by);
        j.movielink_target = Some(ml_target);
        j.movielink_type = Some(ml_type);
    });

    // ---- aka_title: id, movie_id, title ----
    eprintln!("aka_title");
    t!({
        j.movie_aka = Some(job_ids(&p("aka_title"), 1, 0));
        j.akatitle_title = Some(job_strs(&p("aka_title"), 0, 2));
    });

    // ---- name (Person): id, name, gender(4), name_pcode_cf(5) ----
    eprintln!("name");
    t!({
        j.person_name = Some(job_strs(&p("name"), 0, 1));
        j.person_gender = Some(job_strs(&p("name"), 0, 4));
        j.person_name_pcode = Some(job_strs(&p("name"), 0, 5));
    });

    // ---- aka_name: id, person_id, name ----
    eprintln!("aka_name");
    t!({
        j.akaname_name = Some(job_strs(&p("aka_name"), 0, 2));
        j.person_aka = Some(job_ids(&p("aka_name"), 1, 0));
    });

    // ---- complete_cast: id, movie_id, subject_id, status_id ----
    eprintln!("complete_cast");
    t!({
        let (reader, pos) = open_cols(&p("complete_cast"), &[0, 1, 2, 3]);
        let mut mv_cc = ColW::new();
        let mut cc_subject = ColW::new();
        let mut cc_status = ColW::new();
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let mv = int_col(&batch, pos[1]);
            let sj = int_col(&batch, pos[2]);
            let st = int_col(&batch, pos[3]);
            for i in 0..batch.num_rows() {
                let Some(ccid) = id.get(i) else { continue };
                let ccid = ccid - 1;
                if let Some(mid) = mv.get(i) { mv_cc.push(mid - 1, ccid as u64); }
                if let Some(v) = sj.get(i) { cc_subject.push(ccid, (v - 1) as u64); }
                if let Some(v) = st.get(i) { cc_status.push(ccid, (v - 1) as u64); }
            }
        }
        j.movie_complete_cast = Some(mv_cc);
        j.completecast_subject = Some(cc_subject);
        j.completecast_status = Some(cc_status);
    });

    // ---- person_info → PersonInfo: id, person_id, info_type_id, info, note ----
    eprintln!("person_info");
    t!({
        let (reader, pos) = open_cols(&p("person_info"), &[0, 1, 2, 3, 4]);
        let mut pe_info = ColW::new();
        let mut pi_type = ColW::new();
        let mut pi_info = ColS::new();
        let mut pi_note = ColS::new();
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let pe = int_col(&batch, pos[1]);
            let ty = int_col(&batch, pos[2]);
            let inf = str_col(&batch, pos[3]);
            let nt = str_col(&batch, pos[4]);
            for i in 0..batch.num_rows() {
                let Some(piid) = id.get(i) else { continue };
                let piid = piid - 1;
                if let Some(pid) = pe.get(i) { pe_info.push(pid - 1, piid as u64); }
                if let Some(v) = ty.get(i) { pi_type.push(piid, (v - 1) as u64); }
                if let Some(v) = inf.get(i) { pi_info.push(piid, v); }
                if let Some(v) = nt.get(i) { pi_note.push(piid, v); }
            }
        }
        j.person_info = Some(pe_info);
        j.personinfo_type = Some(pi_type);
        j.personinfo_info = Some(pi_info);
        j.personinfo_note = Some(pi_note);
    });

    // ---- cast_info (Cast) — the big one (~36M rows) ----
    // Columns: id, person_id, movie_id, person_role_id, note, nr_order, role_id(6).
    // (v1 also wrote a Cast_movie file; no loader ever read it — dropped.)
    eprintln!("cast_info");
    t!({
        let (reader, pos) = open_cols(&p("cast_info"), &[0, 1, 2, 3, 4, 6]);
        let mut movie_cast = ColW::new();
        let mut cast_person = ColW::new();
        let mut cast_character = ColW::new();
        let mut cast_role = ColW::new();
        let mut cast_note = ColS::new();
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let pe = int_col(&batch, pos[1]);
            let mv = int_col(&batch, pos[2]);
            let ch = int_col(&batch, pos[3]);
            let nt = str_col(&batch, pos[4]);
            let ro = int_col(&batch, pos[5]);
            for i in 0..batch.num_rows() {
                let Some(cid) = id.get(i) else { continue };
                let cid = cid - 1;
                if let Some(m) = mv.get(i) { movie_cast.push(m - 1, cid as u64); }
                if let Some(v) = pe.get(i) { cast_person.push(cid, (v - 1) as u64); }
                if let Some(v) = ch.get(i) { cast_character.push(cid, (v - 1) as u64); }
                if let Some(v) = ro.get(i) { cast_role.push(cid, (v - 1) as u64); }
                if let Some(v) = nt.get(i) { cast_note.push(cid, v); }
            }
        }
        j.movie_cast = Some(movie_cast);
        j.cast_person = Some(cast_person);
        j.cast_character = Some(cast_character);
        j.cast_role = Some(cast_role);
        j.cast_note = Some(cast_note);
    });

    j
}

fn run_job(parquet_dir: &Path, cache_dir: &Path) {
    std::fs::create_dir_all(cache_dir).unwrap();
    let o = |name: &str| cache_dir.join(format!("{name}.bin"));

    let mut j = read_job(parquet_dir);
    macro_rules! take {
        ($f:ident) => { j.$f.take().unwrap() };
    }

    // ---- entity sizes (same max-id formulas as the v1 runtime loader) ----
    eprintln!("universe sizes");
    let n_movie = j.movie_title.as_ref().unwrap().n_from_keys()
        .max(j.movielink_target.as_ref().unwrap().n_from_vals());
    let n_person = j.person_name.as_ref().unwrap().n_from_keys();
    let n_cast = j.cast_person.as_ref().unwrap().n_from_keys();
    let n_keyword = j.keyword_keyword.as_ref().unwrap().n_from_keys()
        .max(j.movie_keyword.as_ref().unwrap().n_from_vals());
    let n_kind = j.kind_kind.as_ref().unwrap().n_from_keys()
        .max(j.movie_kind.as_ref().unwrap().n_from_vals());
    let n_roletype = j.roletype_role.as_ref().unwrap().n_from_keys()
        .max(j.cast_role.as_ref().unwrap().n_from_vals());
    let n_character = j.character_name.as_ref().unwrap().n_from_keys()
        .max(j.cast_character.as_ref().unwrap().n_from_vals());
    let n_company = j.company_name.as_ref().unwrap().n_from_keys()
        .max(j.company_country.as_ref().unwrap().n_from_keys())
        .max(j.company_note.as_ref().unwrap().n_from_keys())
        .max(j.company_type.as_ref().unwrap().n_from_keys())
        .max(j.movie_company.as_ref().unwrap().n_from_vals());
    let n_comptype = j.companytype_kind.as_ref().unwrap().n_from_keys()
        .max(j.company_type.as_ref().unwrap().n_from_vals());
    let n_info = j.info_info.as_ref().unwrap().n_from_keys()
        .max(j.info_type.as_ref().unwrap().n_from_keys())
        .max(j.info_note.as_ref().unwrap().n_from_keys())
        .max(j.movie_info.as_ref().unwrap().n_from_vals());
    let n_infotype = j.infotype_info.as_ref().unwrap().n_from_keys()
        .max(j.info_type.as_ref().unwrap().n_from_vals());
    let n_data = j.data_data.as_ref().unwrap().n_from_keys()
        .max(j.data_type.as_ref().unwrap().n_from_keys())
        .max(j.movie_data.as_ref().unwrap().n_from_vals());
    let n_pinfo = j.personinfo_info.as_ref().unwrap().n_from_keys()
        .max(j.personinfo_type.as_ref().unwrap().n_from_keys())
        .max(j.personinfo_note.as_ref().unwrap().n_from_keys())
        .max(j.person_info.as_ref().unwrap().n_from_vals());
    let n_akaname = j.akaname_name.as_ref().unwrap().n_from_keys()
        .max(j.person_aka.as_ref().unwrap().n_from_vals());
    let n_akatitle = j.akatitle_title.as_ref().unwrap().n_from_keys()
        .max(j.movie_aka.as_ref().unwrap().n_from_vals());
    let n_mlink = j.movielink_target.as_ref().unwrap().n_from_keys()
        .max(j.movielink_type.as_ref().unwrap().n_from_keys())
        .max(j.movie_link.as_ref().unwrap().n_from_vals())
        .max(j.movie_linked_by.as_ref().unwrap().n_from_vals());
    let n_ltype = j.linktype_link.as_ref().unwrap().n_from_keys()
        .max(j.movielink_type.as_ref().unwrap().n_from_vals());
    let n_ccast = j.completecast_status.as_ref().unwrap().n_from_keys()
        .max(j.completecast_subject.as_ref().unwrap().n_from_keys())
        .max(j.movie_complete_cast.as_ref().unwrap().n_from_vals());
    let n_ccktype = j.compcasttype_kind.as_ref().unwrap().n_from_keys()
        .max(j.completecast_status.as_ref().unwrap().n_from_vals())
        .max(j.completecast_subject.as_ref().unwrap().n_from_vals());
    eprintln!("    movie {n_movie}  person {n_person}  cast {n_cast}");

    // ---- finalize + write (drops each buffer as it goes) ----
    eprintln!("write");
    t!({
        // Movie
        take!(movie_title).write_dense(&o("Movie_title"), n_movie);
        take!(movie_kind).write_dense(&o("Movie_kind"), n_movie, KIND_DENSE_I64, NO_ID_WORD);
        take!(movie_production_year).write_csr(&o("Movie_production_year"), n_movie);
        take!(movie_episode_nr).write_csr(&o("Movie_episode_nr"), n_movie);
        take!(movie_keyword).write_csr(&o("Movie_keyword"), n_movie);
        take!(movie_company).write_csr(&o("Movie_company"), n_movie);
        take!(movie_cast).write_csr(&o("Movie_cast"), n_movie);
        take!(movie_info).write_csr(&o("Movie_info"), n_movie);
        take!(movie_data).write_csr(&o("Movie_data"), n_movie);
        take!(movie_complete_cast).write_csr(&o("Movie_complete_cast"), n_movie);
        take!(movie_link).write_csr(&o("Movie_link"), n_movie);
        take!(movie_linked_by).write_csr(&o("Movie_linked_by"), n_movie);
        take!(movie_aka).write_csr(&o("Movie_aka"), n_movie);

        // Cast
        take!(cast_person).write_dense(&o("Cast_person"), n_cast, KIND_DENSE_I64, NO_ID_WORD);
        take!(cast_role).write_dense(&o("Cast_role"), n_cast, KIND_DENSE_I64, NO_ID_WORD);
        take!(cast_note).write_csr(&o("Cast_note"), n_cast);
        take!(cast_character).write_csr(&o("Cast_character"), n_cast);

        // Person
        take!(person_name).write_dense(&o("Person_name"), n_person);
        take!(person_gender).write_csr(&o("Person_gender"), n_person);
        take!(person_aka).write_csr(&o("Person_aka"), n_person);
        take!(person_info).write_csr(&o("Person_info"), n_person);
        take!(person_name_pcode).write_csr(&o("Person_name_pcode_cf"), n_person);

        // lookup entities
        take!(keyword_keyword).write_dense(&o("Keyword_keyword"), n_keyword);
        take!(kind_kind).write_dense(&o("Kind_kind"), n_kind);
        take!(roletype_role).write_dense(&o("RoleType_role"), n_roletype);
        take!(character_name).write_dense(&o("Character_name"), n_character);

        // Company
        take!(company_country).write_csr(&o("Company_country"), n_company);
        take!(company_name).write_dense(&o("Company_name"), n_company);
        take!(company_note).write_csr(&o("Company_note"), n_company);
        take!(company_type).write_dense(&o("Company_type"), n_company, KIND_DENSE_I64, NO_ID_WORD);
        take!(companytype_kind).write_dense(&o("CompanyType_kind"), n_comptype);

        // Info / Data / PersonInfo
        take!(info_info).write_dense(&o("Info_info"), n_info);
        take!(info_type).write_dense(&o("Info_type"), n_info, KIND_DENSE_I64, NO_ID_WORD);
        take!(info_note).write_csr(&o("Info_note"), n_info);
        take!(infotype_info).write_dense(&o("InfoType_info"), n_infotype);
        take!(data_data).write_dense(&o("Data_data"), n_data);
        take!(data_type).write_dense(&o("Data_type"), n_data, KIND_DENSE_I64, NO_ID_WORD);
        take!(personinfo_info).write_dense(&o("PersonInfo_info"), n_pinfo);
        take!(personinfo_type).write_dense(&o("PersonInfo_type"), n_pinfo, KIND_DENSE_I64, NO_ID_WORD);
        take!(personinfo_note).write_csr(&o("PersonInfo_note"), n_pinfo);

        // Aka
        take!(akaname_name).write_dense(&o("AkaName_name"), n_akaname);
        take!(akatitle_title).write_dense(&o("AkaTitle_title"), n_akatitle);

        // MovieLink / LinkType
        take!(movielink_target).write_dense(&o("MovieLink_target"), n_mlink, KIND_DENSE_I64, NO_ID_WORD);
        take!(movielink_type).write_dense(&o("MovieLink_type"), n_mlink, KIND_DENSE_I64, NO_ID_WORD);
        take!(linktype_link).write_dense(&o("LinkType_link"), n_ltype);

        // CompleteCast / CompCastType
        take!(completecast_status).write_dense(&o("CompleteCast_status"), n_ccast, KIND_DENSE_I64, NO_ID_WORD);
        take!(completecast_subject).write_dense(&o("CompleteCast_subject"), n_ccast, KIND_DENSE_I64, NO_ID_WORD);
        take!(compcasttype_kind).write_dense(&o("CompCastType_kind"), n_ccktype);
    });

    // v1 leftovers that v2 never reads (Cast_movie was write-only even in v1)
    let _ = std::fs::remove_file(o("Cast_movie"));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("job") => {
            let parquet_dir = PathBuf::from(args.get(2).map(|s| s.as_str()).unwrap_or("../../jobdata/parquet"));
            let cache_dir   = PathBuf::from(args.get(3).map(|s| s.as_str()).unwrap_or("../cache"));
            run_job(&parquet_dir, &cache_dir);
        }
        Some("tpch") => {
            let parquet_dir = PathBuf::from(args.get(2).map(|s| s.as_str()).unwrap_or("../cache/tpch"));
            let cache_dir   = PathBuf::from(args.get(3).map(|s| s.as_str()).unwrap_or("../cache"));
            run_tpch(&parquet_dir, &cache_dir);
        }
        _ => {
            eprintln!("usage: regen job|tpch [parquet_dir] [cache_dir]");
            std::process::exit(1);
        }
    }
    eprintln!("done.");
}
