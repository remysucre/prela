// Regenerate the binary cache (format — see src/format.rs) from parquet:
//
//   cargo run --release --features regen --bin regen -- job  [../../jobdata/parquet] [../cache]
//   cargo run --release --features regen --bin regen -- tpch [../cache/tpch] [../cache]
//
// regen absorbs ALL load-time transformation: ids are shifted to 0-based
// here, FK holes are filled with NO_ID, dates are parsed to yyyymmdd i64,
// strings are laid out as offsets+bytes, and multi-valued columns are
// built into CSR — the engine loaders (src/cache.rs) just mmap and bulk
// copy/slice.
//
// TPC-H expects clean DuckDB-exported parquet: all-BIGINT integer columns,
// DOUBLE for money, VARCHAR for strings (dates pre-formatted as ISO
// yyyy-mm-dd). It runs per-field passes through parquet (column
// projection) and writes each dense column immediately. JOB reads the IMDb
// parquet export as-is and feeds normalized rows to the shared `job_shred`
// core also used by differential tests. That core computes entity universe
// sizes and finalizes each column to its dense/CSR layout.

use prela::cache_writer::{StringColumn as ColS, WordColumn as ColW};
use prela::format::*;
use prela::job_shred::JobShredder;

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use arrow::array::{Array, Float64Array, Int32Array, Int64Array, LargeStringArray, StringArray};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ProjectionMask;
use parquet::arrow::arrow_reader::{ParquetRecordBatchReader, ParquetRecordBatchReaderBuilder};

macro_rules! t {
    ($s:expr) => {{
        let start = std::time::Instant::now();
        $s;
        eprintln!("    {:.2}s", start.elapsed().as_secs_f32());
    }};
}

// ===== schema manifest check =============================================
// The struct schemas (src/job_schema.rs, src/tpch_schema.rs) are the source
// of truth for WHAT the cache contains: regen records every file it writes
// and, after a run, checks the set against the schema's `manifest()` — name
// for name (field names are filenames, verbatim) and header kind for kind.
// Drift in either direction fails the regen run loudly.
//
// `manifest()` is produced by running the schema's own `build` body against
// a recording loader that reads nothing (src/loader.rs), so the list cannot
// fall out of step with the structs the engine will actually load.

fn verify_manifest(
    cache_dir: &Path,
    written: &[String],
    manifest: Vec<(String, u32)>,
    suite: &str,
) {
    let expected: HashMap<String, u32> = manifest.into_iter().collect();
    for name in written {
        assert!(
            expected.contains_key(name),
            "{suite}: regen wrote {name}.bin but the schema does not declare it"
        );
    }
    for (name, kind) in &expected {
        assert!(
            written.iter().any(|w| w == name),
            "{suite}: the schema declares {name} but regen did not write it"
        );
        let path = cache_dir.join(format!("{name}.bin"));
        let mut head = [0u8; HEADER_LEN];
        File::open(&path)
            .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut head))
            .unwrap_or_else(|e| panic!("{suite}: read {path:?}: {e}"));
        parse_header(&head, *kind, &format!("{path:?}")); // panics on kind mismatch
    }
    eprintln!(
        "{suite}: schema manifest verified — {} columns",
        expected.len()
    );
}

// ===== column buffers ====================================================
// Pairs are buffered with INTERNAL (0-based) keys; values are 8-byte
// words — a 0-based id, a raw i64 bit pattern, or an f64 bit pattern
// (the finalize call's kind says which). Dense scatter is
// last-write-wins and panics on a key outside the universe
// (`VecRel::from_pairs`); CSR drops out-of-universe keys and keeps
// per-key stream order (`MultiRel::from_pairs`).

fn internal_key(key: i64) -> usize {
    usize::try_from(key).expect("internal key must be nonnegative and fit usize")
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
    let pos = indices
        .iter()
        .map(|i| sorted.binary_search(i).unwrap())
        .collect();
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
        panic!(
            "column {pos}: expected Int32/Int64, got {:?}",
            col.data_type()
        )
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
        panic!(
            "column {pos}: expected Utf8/LargeUtf8, got {:?}",
            col.data_type()
        )
    }
}

// ======================== TPC-H ========================
//
// All columns are dense; every parquet row contributes a pair (NULL
// strings become ""). Key spaces: suppkey /
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
    d(0) * 10_000_000
        + d(1) * 1_000_000
        + d(2) * 100_000
        + d(3) * 10_000
        + d(5) * 1000
        + d(6) * 100
        + d(8) * 10
        + d(9)
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
                    col.push(
                        internal_key(k.get(i).unwrap() + key_delta),
                        (v.get(i).unwrap() + delta) as u64,
                    );
                }
            }
            TVal::I64 => {
                let v = int_col(&batch, pos[1]);
                for i in 0..batch.num_rows() {
                    col.push(
                        internal_key(k.get(i).unwrap() + key_delta),
                        v.get(i).unwrap() as u64,
                    );
                }
            }
            TVal::F64 => {
                kind = KIND_DENSE_F64;
                let v = batch
                    .column(pos[1])
                    .as_any()
                    .downcast_ref::<Float64Array>()
                    .unwrap_or_else(|| panic!("{val}: expected Float64"));
                for i in 0..batch.num_rows() {
                    col.push(
                        internal_key(k.get(i).unwrap() + key_delta),
                        v.value(i).to_bits(),
                    );
                }
            }
            TVal::Date => {
                let v = str_col(&batch, pos[1]);
                for i in 0..batch.num_rows() {
                    let d = parse_yyyymmdd(v.get(i).unwrap_or(""));
                    col.push(internal_key(k.get(i).unwrap() + key_delta), d as u64);
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
            col.push(
                internal_key(k.get(i).unwrap() + key_delta),
                v.get(i).unwrap_or(""),
            );
        }
    }
    let n = col.n_from_keys();
    col.write_dense(out, n);
}

fn run_tpch(parquet_dir: &Path, cache_dir: &Path) {
    std::fs::create_dir_all(cache_dir).unwrap();

    let written = std::cell::RefCell::new(Vec::<String>::new());
    let p = |name: &str| parquet_dir.join(format!("{name}.parquet"));
    let o = |name: &str| {
        written.borrow_mut().push(name.to_string());
        cache_dir.join(format!("{name}.bin"))
    };
    use TVal::*;

    // Region / Nation: 0-based keys in the parquet (internal = raw).
    eprintln!("region");
    t!(tpch_dense_str(
        &o("Region_name"),
        &p("region"),
        "r_regionkey",
        "r_name",
        0
    ));
    t!(tpch_dense_str(
        &o("Region_comment"),
        &p("region"),
        "r_regionkey",
        "r_comment",
        0
    ));

    eprintln!("nation");
    t!(tpch_dense_str(
        &o("Nation_name"),
        &p("nation"),
        "n_nationkey",
        "n_name",
        0
    ));
    t!(tpch_dense(
        &o("Nation_region"),
        &p("nation"),
        "n_nationkey",
        "n_regionkey",
        0,
        Id { delta: 0 }
    ));
    t!(tpch_dense_str(
        &o("Nation_comment"),
        &p("nation"),
        "n_nationkey",
        "n_comment",
        0
    ));

    // Supplier (1-based suppkey)
    eprintln!("supplier");
    t!(tpch_dense_str(
        &o("Supplier_name"),
        &p("supplier"),
        "s_suppkey",
        "s_name",
        -1
    ));
    t!(tpch_dense_str(
        &o("Supplier_address"),
        &p("supplier"),
        "s_suppkey",
        "s_address",
        -1
    ));
    t!(tpch_dense(
        &o("Supplier_nation"),
        &p("supplier"),
        "s_suppkey",
        "s_nationkey",
        -1,
        Id { delta: 0 }
    ));
    t!(tpch_dense_str(
        &o("Supplier_phone"),
        &p("supplier"),
        "s_suppkey",
        "s_phone",
        -1
    ));
    t!(tpch_dense(
        &o("Supplier_acctbal"),
        &p("supplier"),
        "s_suppkey",
        "s_acctbal",
        -1,
        F64
    ));
    t!(tpch_dense_str(
        &o("Supplier_comment"),
        &p("supplier"),
        "s_suppkey",
        "s_comment",
        -1
    ));

    // Customer
    eprintln!("customer");
    t!(tpch_dense_str(
        &o("Customer_name"),
        &p("customer"),
        "c_custkey",
        "c_name",
        -1
    ));
    t!(tpch_dense_str(
        &o("Customer_address"),
        &p("customer"),
        "c_custkey",
        "c_address",
        -1
    ));
    t!(tpch_dense(
        &o("Customer_nation"),
        &p("customer"),
        "c_custkey",
        "c_nationkey",
        -1,
        Id { delta: 0 }
    ));
    t!(tpch_dense_str(
        &o("Customer_phone"),
        &p("customer"),
        "c_custkey",
        "c_phone",
        -1
    ));
    t!(tpch_dense(
        &o("Customer_acctbal"),
        &p("customer"),
        "c_custkey",
        "c_acctbal",
        -1,
        F64
    ));
    t!(tpch_dense_str(
        &o("Customer_mktsegment"),
        &p("customer"),
        "c_custkey",
        "c_mktsegment",
        -1
    ));
    t!(tpch_dense_str(
        &o("Customer_comment"),
        &p("customer"),
        "c_custkey",
        "c_comment",
        -1
    ));

    // Part
    eprintln!("part");
    t!(tpch_dense_str(
        &o("Part_name"),
        &p("part"),
        "p_partkey",
        "p_name",
        -1
    ));
    t!(tpch_dense_str(
        &o("Part_mfgr"),
        &p("part"),
        "p_partkey",
        "p_mfgr",
        -1
    ));
    t!(tpch_dense_str(
        &o("Part_brand"),
        &p("part"),
        "p_partkey",
        "p_brand",
        -1
    ));
    t!(tpch_dense_str(
        &o("Part_ty"),
        &p("part"),
        "p_partkey",
        "p_type",
        -1
    ));
    t!(tpch_dense(
        &o("Part_size"),
        &p("part"),
        "p_partkey",
        "p_size",
        -1,
        I64
    ));
    t!(tpch_dense_str(
        &o("Part_container"),
        &p("part"),
        "p_partkey",
        "p_container",
        -1
    ));
    t!(tpch_dense(
        &o("Part_retailprice"),
        &p("part"),
        "p_partkey",
        "p_retailprice",
        -1,
        F64
    ));
    t!(tpch_dense_str(
        &o("Part_comment"),
        &p("part"),
        "p_partkey",
        "p_comment",
        -1
    ));

    // PartSupp (synthetic ps_id 1..N; part/supplier FKs 1-based)
    eprintln!("partsupp");
    t!(tpch_dense(
        &o("PartSupp_part"),
        &p("partsupp"),
        "ps_id",
        "ps_partkey",
        -1,
        Id { delta: -1 }
    ));
    t!(tpch_dense(
        &o("PartSupp_supplier"),
        &p("partsupp"),
        "ps_id",
        "ps_suppkey",
        -1,
        Id { delta: -1 }
    ));
    t!(tpch_dense(
        &o("PartSupp_availqty"),
        &p("partsupp"),
        "ps_id",
        "ps_availqty",
        -1,
        I64
    ));
    t!(tpch_dense(
        &o("PartSupp_supplycost"),
        &p("partsupp"),
        "ps_id",
        "ps_supplycost",
        -1,
        F64
    ));
    t!(tpch_dense_str(
        &o("PartSupp_comment"),
        &p("partsupp"),
        "ps_id",
        "ps_comment",
        -1
    ));

    // Orders (sparse orderkey — dense files carry the holes)
    eprintln!("orders");
    t!(tpch_dense(
        &o("Order_customer"),
        &p("orders"),
        "o_orderkey",
        "o_custkey",
        -1,
        Id { delta: -1 }
    ));
    t!(tpch_dense_str(
        &o("Order_status"),
        &p("orders"),
        "o_orderkey",
        "o_orderstatus",
        -1
    ));
    t!(tpch_dense(
        &o("Order_totalprice"),
        &p("orders"),
        "o_orderkey",
        "o_totalprice",
        -1,
        F64
    ));
    t!(tpch_dense(
        &o("Order_date"),
        &p("orders"),
        "o_orderkey",
        "o_orderdate",
        -1,
        Date
    ));
    t!(tpch_dense_str(
        &o("Order_priority"),
        &p("orders"),
        "o_orderkey",
        "o_orderpriority",
        -1
    ));
    t!(tpch_dense_str(
        &o("Order_clerk"),
        &p("orders"),
        "o_orderkey",
        "o_clerk",
        -1
    ));
    t!(tpch_dense(
        &o("Order_shippriority"),
        &p("orders"),
        "o_orderkey",
        "o_shippriority",
        -1,
        I64
    ));
    t!(tpch_dense_str(
        &o("Order_comment"),
        &p("orders"),
        "o_orderkey",
        "o_comment",
        -1
    ));

    // Lineitem (synthetic l_id 1..N)
    eprintln!("lineitem");
    t!(tpch_dense(
        &o("Lineitem_order"),
        &p("lineitem"),
        "l_id",
        "l_orderkey",
        -1,
        Id { delta: -1 }
    ));
    t!(tpch_dense(
        &o("Lineitem_part"),
        &p("lineitem"),
        "l_id",
        "l_partkey",
        -1,
        Id { delta: -1 }
    ));
    t!(tpch_dense(
        &o("Lineitem_supplier"),
        &p("lineitem"),
        "l_id",
        "l_suppkey",
        -1,
        Id { delta: -1 }
    ));
    t!(tpch_dense(
        &o("Lineitem_number"),
        &p("lineitem"),
        "l_id",
        "l_linenumber",
        -1,
        I64
    ));
    t!(tpch_dense(
        &o("Lineitem_quantity"),
        &p("lineitem"),
        "l_id",
        "l_quantity",
        -1,
        F64
    ));
    t!(tpch_dense(
        &o("Lineitem_extendedprice"),
        &p("lineitem"),
        "l_id",
        "l_extendedprice",
        -1,
        F64
    ));
    t!(tpch_dense(
        &o("Lineitem_discount"),
        &p("lineitem"),
        "l_id",
        "l_discount",
        -1,
        F64
    ));
    t!(tpch_dense(
        &o("Lineitem_tax"),
        &p("lineitem"),
        "l_id",
        "l_tax",
        -1,
        F64
    ));
    t!(tpch_dense_str(
        &o("Lineitem_returnflag"),
        &p("lineitem"),
        "l_id",
        "l_returnflag",
        -1
    ));
    t!(tpch_dense_str(
        &o("Lineitem_status"),
        &p("lineitem"),
        "l_id",
        "l_linestatus",
        -1
    ));
    t!(tpch_dense(
        &o("Lineitem_shipdate"),
        &p("lineitem"),
        "l_id",
        "l_shipdate",
        -1,
        Date
    ));
    t!(tpch_dense(
        &o("Lineitem_commitdate"),
        &p("lineitem"),
        "l_id",
        "l_commitdate",
        -1,
        Date
    ));
    t!(tpch_dense(
        &o("Lineitem_receiptdate"),
        &p("lineitem"),
        "l_id",
        "l_receiptdate",
        -1,
        Date
    ));
    t!(tpch_dense_str(
        &o("Lineitem_shipinstruct"),
        &p("lineitem"),
        "l_id",
        "l_shipinstruct",
        -1
    ));
    t!(tpch_dense_str(
        &o("Lineitem_shipmode"),
        &p("lineitem"),
        "l_id",
        "l_shipmode",
        -1
    ));
    t!(tpch_dense_str(
        &o("Lineitem_comment"),
        &p("lineitem"),
        "l_id",
        "l_comment",
        -1
    ));

    verify_manifest(
        cache_dir,
        &written.into_inner(),
        prela::tpch_schema::manifest(),
        "tpch",
    );
}

// ======================== JOB ========================
//
// Pair semantics:
//
//   - pairs are emitted in parquet row order;
//   - a pair is skipped iff its key or its value is NULL (per-column
//     independence: a cast_info row with a NULL note still contributes to
//     the other cast columns);
//   - Company name/country come from a company_name lookup Dict
//     (last-write-wins on duplicate keys; lookup misses skip the pair);
//   - ids are 1-based in the parquet; the −1 shift to internal ids
//     happens HERE (push sites), not at engine load time;
//   - entity universe sizes come from the max ids, which is what dense
//     hole-filling and CSR out-of-range dropping are measured against.

fn visit_job_ids(
    parquet: &Path,
    key_idx: usize,
    value_idx: usize,
    mut visit: impl FnMut(Option<i64>, Option<i64>),
) {
    let (reader, pos) = open_cols(parquet, &[key_idx, value_idx]);
    for batch in reader {
        let batch = batch.unwrap();
        let key = int_col(&batch, pos[0]);
        let value = int_col(&batch, pos[1]);
        for row in 0..batch.num_rows() {
            visit(key.get(row), value.get(row));
        }
    }
}

fn visit_job_strings(
    parquet: &Path,
    key_idx: usize,
    value_idx: usize,
    mut visit: impl FnMut(Option<i64>, Option<&str>),
) {
    let (reader, pos) = open_cols(parquet, &[key_idx, value_idx]);
    for batch in reader {
        let batch = batch.unwrap();
        let key = int_col(&batch, pos[0]);
        let value = str_col(&batch, pos[1]);
        for row in 0..batch.num_rows() {
            visit(key.get(row), value.get(row));
        }
    }
}

/// Read normalized JOB Parquet rows through the same shredder used by the
/// generated differential fixtures.
fn read_job(parquet_dir: &Path) -> JobShredder {
    let p = |name: &str| parquet_dir.join(format!("{name}.parquet"));
    let mut shred = JobShredder::default();

    eprintln!("title");
    t!({
        let (reader, pos) = open_cols(&p("title"), &[0, 1, 3, 4, 9]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let title = str_col(&batch, pos[1]);
            let kind = int_col(&batch, pos[2]);
            let year = int_col(&batch, pos[3]);
            let episode = int_col(&batch, pos[4]);
            for row in 0..batch.num_rows() {
                shred.title(
                    id.get(row),
                    title.get(row),
                    kind.get(row),
                    year.get(row),
                    episode.get(row),
                );
            }
        }
    });

    eprintln!(
        "kind_type / role_type / char_name / info_type / link_type / comp_cast_type / company_type"
    );
    t!({
        visit_job_strings(&p("kind_type"), 0, 1, |id, value| {
            shred.kind_type(id, value)
        });
        visit_job_strings(&p("role_type"), 0, 1, |id, value| {
            shred.role_type(id, value)
        });
        visit_job_strings(&p("char_name"), 0, 1, |id, value| {
            shred.char_name(id, value)
        });
        visit_job_strings(&p("info_type"), 0, 1, |id, value| {
            shred.info_type(id, value)
        });
        visit_job_strings(&p("link_type"), 0, 1, |id, value| {
            shred.link_type(id, value)
        });
        visit_job_strings(&p("comp_cast_type"), 0, 1, |id, value| {
            shred.comp_cast_type(id, value)
        });
        visit_job_strings(&p("company_type"), 0, 1, |id, value| {
            shred.company_type(id, value)
        });
    });

    eprintln!("keyword");
    t!({
        visit_job_strings(&p("keyword"), 0, 1, |id, value| shred.keyword(id, value));
        visit_job_ids(&p("movie_keyword"), 1, 2, |movie, keyword| {
            shred.movie_keyword(movie, keyword)
        });
    });

    eprintln!("company");
    t!({
        let (reader, pos) = open_cols(&p("company_name"), &[0, 1, 2]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let name = str_col(&batch, pos[1]);
            let country = str_col(&batch, pos[2]);
            for row in 0..batch.num_rows() {
                shred.company_name(id.get(row), name.get(row), country.get(row));
            }
        }

        let (reader, pos) = open_cols(&p("movie_companies"), &[0, 1, 2, 3, 4]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let movie = int_col(&batch, pos[1]);
            let company = int_col(&batch, pos[2]);
            let company_type = int_col(&batch, pos[3]);
            let note = str_col(&batch, pos[4]);
            for row in 0..batch.num_rows() {
                shred.movie_company(
                    id.get(row),
                    movie.get(row),
                    company.get(row),
                    company_type.get(row),
                    note.get(row),
                );
            }
        }
    });

    eprintln!("movie_info");
    t!({
        let (reader, pos) = open_cols(&p("movie_info"), &[0, 1, 2, 3, 4]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let movie = int_col(&batch, pos[1]);
            let info_type = int_col(&batch, pos[2]);
            let info = str_col(&batch, pos[3]);
            let note = str_col(&batch, pos[4]);
            for row in 0..batch.num_rows() {
                shred.movie_info(
                    id.get(row),
                    movie.get(row),
                    info_type.get(row),
                    info.get(row),
                    note.get(row),
                );
            }
        }
    });

    eprintln!("movie_info_idx");
    t!({
        let (reader, pos) = open_cols(&p("movie_info_idx"), &[0, 1, 2, 3]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let movie = int_col(&batch, pos[1]);
            let info_type = int_col(&batch, pos[2]);
            let info = str_col(&batch, pos[3]);
            for row in 0..batch.num_rows() {
                shred.movie_info_idx(
                    id.get(row),
                    movie.get(row),
                    info_type.get(row),
                    info.get(row),
                );
            }
        }
    });

    eprintln!("movie_link");
    t!({
        let (reader, pos) = open_cols(&p("movie_link"), &[0, 1, 2, 3]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let movie = int_col(&batch, pos[1]);
            let target = int_col(&batch, pos[2]);
            let link_type = int_col(&batch, pos[3]);
            for row in 0..batch.num_rows() {
                shred.movie_link(
                    id.get(row),
                    movie.get(row),
                    target.get(row),
                    link_type.get(row),
                );
            }
        }
    });

    eprintln!("aka_title");
    t!({
        let (reader, pos) = open_cols(&p("aka_title"), &[0, 1, 2]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let movie = int_col(&batch, pos[1]);
            let title = str_col(&batch, pos[2]);
            for row in 0..batch.num_rows() {
                shred.aka_title(id.get(row), movie.get(row), title.get(row));
            }
        }
    });

    eprintln!("name");
    t!({
        let (reader, pos) = open_cols(&p("name"), &[0, 1, 4, 5]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let name = str_col(&batch, pos[1]);
            let gender = str_col(&batch, pos[2]);
            let name_pcode = str_col(&batch, pos[3]);
            for row in 0..batch.num_rows() {
                shred.name(
                    id.get(row),
                    name.get(row),
                    gender.get(row),
                    name_pcode.get(row),
                );
            }
        }
    });

    eprintln!("aka_name");
    t!({
        let (reader, pos) = open_cols(&p("aka_name"), &[0, 1, 2]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let person = int_col(&batch, pos[1]);
            let name = str_col(&batch, pos[2]);
            for row in 0..batch.num_rows() {
                shred.aka_name(id.get(row), person.get(row), name.get(row));
            }
        }
    });

    eprintln!("complete_cast");
    t!({
        let (reader, pos) = open_cols(&p("complete_cast"), &[0, 1, 2, 3]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let movie = int_col(&batch, pos[1]);
            let subject = int_col(&batch, pos[2]);
            let status = int_col(&batch, pos[3]);
            for row in 0..batch.num_rows() {
                shred.complete_cast(
                    id.get(row),
                    movie.get(row),
                    subject.get(row),
                    status.get(row),
                );
            }
        }
    });

    eprintln!("person_info");
    t!({
        let (reader, pos) = open_cols(&p("person_info"), &[0, 1, 2, 3, 4]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let person = int_col(&batch, pos[1]);
            let info_type = int_col(&batch, pos[2]);
            let info = str_col(&batch, pos[3]);
            let note = str_col(&batch, pos[4]);
            for row in 0..batch.num_rows() {
                shred.person_info(
                    id.get(row),
                    person.get(row),
                    info_type.get(row),
                    info.get(row),
                    note.get(row),
                );
            }
        }
    });

    eprintln!("cast_info");
    t!({
        let (reader, pos) = open_cols(&p("cast_info"), &[0, 1, 2, 3, 4, 6]);
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let person = int_col(&batch, pos[1]);
            let movie = int_col(&batch, pos[2]);
            let character = int_col(&batch, pos[3]);
            let note = str_col(&batch, pos[4]);
            let role = int_col(&batch, pos[5]);
            for row in 0..batch.num_rows() {
                shred.cast_info(
                    id.get(row),
                    person.get(row),
                    movie.get(row),
                    character.get(row),
                    note.get(row),
                    role.get(row),
                );
            }
        }
    });

    shred
}

fn run_job(parquet_dir: &Path, cache_dir: &Path) {
    let written = read_job(parquet_dir).write_cache(cache_dir);
    let _ = std::fs::remove_file(cache_dir.join("Cast_movie.bin"));
    verify_manifest(cache_dir, &written, prela::job_schema::manifest(), "job");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("job") => {
            let parquet_dir = PathBuf::from(
                args.get(2)
                    .map(|s| s.as_str())
                    .unwrap_or("../../jobdata/parquet"),
            );
            let cache_dir = PathBuf::from(args.get(3).map(|s| s.as_str()).unwrap_or("../cache"));
            run_job(&parquet_dir, &cache_dir);
        }
        Some("tpch") => {
            let parquet_dir =
                PathBuf::from(args.get(2).map(|s| s.as_str()).unwrap_or("../cache/tpch"));
            let cache_dir = PathBuf::from(args.get(3).map(|s| s.as_str()).unwrap_or("../cache"));
            run_tpch(&parquet_dir, &cache_dir);
        }
        _ => {
            eprintln!("usage: regen job|tpch [parquet_dir] [cache_dir]");
            std::process::exit(1);
        }
    }
    eprintln!("done.");
}
