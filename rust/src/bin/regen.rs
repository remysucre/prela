// Regenerate the binary cache from parquet. Two modes:
//
//   cargo run --release --features regen --bin regen -- tpch [/tmp/tpch5_clean] [../cache]
//   cargo run --release --features regen --bin regen -- job  [../../jobdata/parquet] [../cache]
//
// (A bare `regen <parquet_dir> <cache_dir>` with no mode keyword is the
// legacy TPC-H invocation and still works.)
//
// TPC-H expects clean DuckDB-exported parquet: all-BIGINT integer columns,
// DOUBLE for money, VARCHAR for strings (dates pre-formatted as ISO
// yyyy-mm-dd). JOB reads the imdb parquet export as-is (INT32 ids,
// nullable columns) and reproduces the transformations of the retired
// Julia loader (julia-engine branch, JOB.jl `load_all!`) byte-for-byte.
//
// Writes <cache_dir>/<Entity>_<field>.bin files in the cache format, which
// is defined and produced here (originally by Julia's cache.jl — see the
// julia-engine branch for the historic implementation):
//   bits  — [u64 n][n × (i64,i64)]
//   str   — [u64 n][n × i64 keys][(n+1) × u32 offsets][bytes]
//
// Keys/values are written as the raw 1-based ids found in the parquet; the
// runtime loader (src/cache.rs) shifts to 0-based internal ids at read time.
//
// TPC-H runs per-field passes through parquet (column projection — touches
// only the two needed columns). Streaming: never holds more than one
// parquet row group in RAM, so SF=10+ on a 32GB machine is fine. JOB runs
// one pass per source table, fanning out to all derived .bin files;
// bits files stream to disk, string files buffer in RAM.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
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

fn open_parquet(path: &Path, columns: &[&str]) -> parquet::arrow::arrow_reader::ParquetRecordBatchReader {
    let file = File::open(path).unwrap_or_else(|e| panic!("open {path:?}: {e}"));
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
    let schema = builder.parquet_schema();
    let indices: Vec<usize> = columns.iter()
        .map(|name| {
            (0..schema.num_columns())
                .find(|&i| schema.column(i).name() == *name)
                .unwrap_or_else(|| panic!("column {name} not found in {path:?}"))
        })
        .collect();
    let mask = ProjectionMask::roots(schema, indices);
    builder.with_projection(mask).build().unwrap()
}

fn write_bits_header(f: &mut BufWriter<File>, n: u64) {
    f.write_all(&n.to_le_bytes()).unwrap();
}

/// Write a `<entity>_<field>.bin` of i64-pair format (D, R both i64).
/// `key_shift` is added to every key (used for Region/Nation +1).
fn write_bits(
    out: &Path,
    parquet: &Path,
    key_col: &str,
    val_col: &str,
    key_shift: i64,
    val_shift: i64,
) {
    let mut reader = open_parquet(parquet, &[key_col, val_col]);
    let mut buf: Vec<i64> = Vec::new();
    while let Some(batch) = reader.next() {
        let batch = batch.unwrap();
        let k = batch.column(0).as_any().downcast_ref::<Int64Array>()
            .unwrap_or_else(|| panic!("{key_col}: expected Int64"));
        let v = batch.column(1).as_any().downcast_ref::<Int64Array>()
            .unwrap_or_else(|| panic!("{val_col}: expected Int64"));
        for i in 0..batch.num_rows() {
            buf.push(k.value(i) + key_shift);
            buf.push(v.value(i) + val_shift);
        }
    }
    let n = (buf.len() / 2) as u64;
    let mut f = BufWriter::new(File::create(out).unwrap());
    write_bits_header(&mut f, n);
    let bytes = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 8) };
    f.write_all(bytes).unwrap();
}

/// f64-valued field — store as i64 bit pattern so the runtime's
/// `f64::from_bits` reads it back.
fn write_f64(out: &Path, parquet: &Path, key_col: &str, val_col: &str, key_shift: i64) {
    let mut reader = open_parquet(parquet, &[key_col, val_col]);
    let mut buf: Vec<i64> = Vec::new();
    while let Some(batch) = reader.next() {
        let batch = batch.unwrap();
        let k = batch.column(0).as_any().downcast_ref::<Int64Array>()
            .unwrap_or_else(|| panic!("{key_col}: expected Int64"));
        let v = batch.column(1).as_any().downcast_ref::<Float64Array>()
            .unwrap_or_else(|| panic!("{val_col}: expected Float64"));
        for i in 0..batch.num_rows() {
            buf.push(k.value(i) + key_shift);
            buf.push(v.value(i).to_bits() as i64);
        }
    }
    let n = (buf.len() / 2) as u64;
    let mut f = BufWriter::new(File::create(out).unwrap());
    write_bits_header(&mut f, n);
    let bytes = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 8) };
    f.write_all(bytes).unwrap();
}

/// String-valued field — header + keys[] + offsets[] + bytes[].
fn write_strs(out: &Path, parquet: &Path, key_col: &str, val_col: &str, key_shift: i64) {
    let mut reader = open_parquet(parquet, &[key_col, val_col]);
    let mut keys: Vec<i64> = Vec::new();
    let mut bytes: Vec<u8> = Vec::new();
    let mut offsets: Vec<u32> = vec![0];
    while let Some(batch) = reader.next() {
        let batch = batch.unwrap();
        let k = batch.column(0).as_any().downcast_ref::<Int64Array>()
            .unwrap_or_else(|| panic!("{key_col}: expected Int64"));
        let v = batch.column(1).as_any().downcast_ref::<StringArray>()
            .unwrap_or_else(|| panic!("{val_col}: expected String"));
        for i in 0..batch.num_rows() {
            keys.push(k.value(i) + key_shift);
            let s = if v.is_null(i) { "" } else { v.value(i) };
            bytes.extend_from_slice(s.as_bytes());
            offsets.push(bytes.len() as u32);
        }
    }
    let n = keys.len() as u64;
    let mut f = BufWriter::new(File::create(out).unwrap());
    write_bits_header(&mut f, n);
    let k_bytes = unsafe { std::slice::from_raw_parts(keys.as_ptr() as *const u8, keys.len() * 8) };
    f.write_all(k_bytes).unwrap();
    let o_bytes = unsafe { std::slice::from_raw_parts(offsets.as_ptr() as *const u8, offsets.len() * 4) };
    f.write_all(o_bytes).unwrap();
    f.write_all(&bytes).unwrap();
}

fn run_tpch(parquet_dir: &Path, cache_dir: &Path) {
    std::fs::create_dir_all(cache_dir).unwrap();

    let p = |name: &str| parquet_dir.join(format!("{name}.parquet"));
    let o = |name: &str| cache_dir.join(format!("{name}.bin"));

    // Region: keys 0..4 shifted +1
    eprintln!("region");
    t!(write_strs(&o("Region_name"),    &p("region"), "r_regionkey", "r_name",    1));
    t!(write_strs(&o("Region_comment"), &p("region"), "r_regionkey", "r_comment", 1));

    // Nation: keys shifted +1, region FK shifted +1
    eprintln!("nation");
    t!(write_strs(&o("Nation_name"),    &p("nation"), "n_nationkey", "n_name",    1));
    t!(write_bits(&o("Nation_region"),  &p("nation"), "n_nationkey", "n_regionkey", 1, 1));
    t!(write_strs(&o("Nation_comment"), &p("nation"), "n_nationkey", "n_comment", 1));

    // Supplier
    eprintln!("supplier");
    t!(write_strs(&o("Supplier_name"),    &p("supplier"), "s_suppkey", "s_name",    0));
    t!(write_strs(&o("Supplier_address"), &p("supplier"), "s_suppkey", "s_address", 0));
    t!(write_bits(&o("Supplier_nation"),  &p("supplier"), "s_suppkey", "s_nationkey", 0, 1));
    t!(write_strs(&o("Supplier_phone"),   &p("supplier"), "s_suppkey", "s_phone",   0));
    t!(write_f64 (&o("Supplier_acctbal"), &p("supplier"), "s_suppkey", "s_acctbal", 0));
    t!(write_strs(&o("Supplier_comment"), &p("supplier"), "s_suppkey", "s_comment", 0));

    // Customer
    eprintln!("customer");
    t!(write_strs(&o("Customer_name"),       &p("customer"), "c_custkey", "c_name",       0));
    t!(write_strs(&o("Customer_address"),    &p("customer"), "c_custkey", "c_address",    0));
    t!(write_bits(&o("Customer_nation"),     &p("customer"), "c_custkey", "c_nationkey", 0, 1));
    t!(write_strs(&o("Customer_phone"),      &p("customer"), "c_custkey", "c_phone",      0));
    t!(write_f64 (&o("Customer_acctbal"),    &p("customer"), "c_custkey", "c_acctbal",    0));
    t!(write_strs(&o("Customer_mktsegment"), &p("customer"), "c_custkey", "c_mktsegment", 0));
    t!(write_strs(&o("Customer_comment"),    &p("customer"), "c_custkey", "c_comment",    0));

    // Part
    eprintln!("part");
    t!(write_strs(&o("Part_name"),        &p("part"), "p_partkey", "p_name",      0));
    t!(write_strs(&o("Part_mfgr"),        &p("part"), "p_partkey", "p_mfgr",      0));
    t!(write_strs(&o("Part_brand"),       &p("part"), "p_partkey", "p_brand",     0));
    t!(write_strs(&o("Part_type"),        &p("part"), "p_partkey", "p_type",      0));
    t!(write_bits(&o("Part_size"),        &p("part"), "p_partkey", "p_size",      0, 0));
    t!(write_strs(&o("Part_container"),   &p("part"), "p_partkey", "p_container", 0));
    t!(write_f64 (&o("Part_retailprice"), &p("part"), "p_partkey", "p_retailprice", 0));
    t!(write_strs(&o("Part_comment"),     &p("part"), "p_partkey", "p_comment",   0));

    // PartSupp (synthetic ps_id 1..N)
    eprintln!("partsupp");
    t!(write_bits(&o("PartSupp_part"),       &p("partsupp"), "ps_id", "ps_partkey",   0, 0));
    t!(write_bits(&o("PartSupp_supplier"),   &p("partsupp"), "ps_id", "ps_suppkey",   0, 0));
    t!(write_bits(&o("PartSupp_availqty"),   &p("partsupp"), "ps_id", "ps_availqty",  0, 0));
    t!(write_f64 (&o("PartSupp_supplycost"), &p("partsupp"), "ps_id", "ps_supplycost", 0));
    t!(write_strs(&o("PartSupp_comment"),    &p("partsupp"), "ps_id", "ps_comment",    0));

    // Orders
    eprintln!("orders");
    t!(write_bits(&o("Order_customer"),     &p("orders"), "o_orderkey", "o_custkey",     0, 0));
    t!(write_strs(&o("Order_status"),       &p("orders"), "o_orderkey", "o_orderstatus", 0));
    t!(write_f64 (&o("Order_totalprice"),   &p("orders"), "o_orderkey", "o_totalprice",  0));
    t!(write_strs(&o("Order_date"),         &p("orders"), "o_orderkey", "o_orderdate",   0));
    t!(write_strs(&o("Order_priority"),     &p("orders"), "o_orderkey", "o_orderpriority", 0));
    t!(write_strs(&o("Order_clerk"),        &p("orders"), "o_orderkey", "o_clerk",       0));
    t!(write_bits(&o("Order_shippriority"), &p("orders"), "o_orderkey", "o_shippriority", 0, 0));
    t!(write_strs(&o("Order_comment"),      &p("orders"), "o_orderkey", "o_comment",     0));

    // Lineitem (synthetic l_id 1..N)
    eprintln!("lineitem");
    t!(write_bits(&o("Lineitem_order"),         &p("lineitem"), "l_id", "l_orderkey",     0, 0));
    t!(write_bits(&o("Lineitem_part"),          &p("lineitem"), "l_id", "l_partkey",      0, 0));
    t!(write_bits(&o("Lineitem_supplier"),      &p("lineitem"), "l_id", "l_suppkey",      0, 0));
    t!(write_bits(&o("Lineitem_number"),        &p("lineitem"), "l_id", "l_linenumber",   0, 0));
    t!(write_f64 (&o("Lineitem_quantity"),      &p("lineitem"), "l_id", "l_quantity",     0));
    t!(write_f64 (&o("Lineitem_extendedprice"), &p("lineitem"), "l_id", "l_extendedprice", 0));
    t!(write_f64 (&o("Lineitem_discount"),      &p("lineitem"), "l_id", "l_discount",     0));
    t!(write_f64 (&o("Lineitem_tax"),           &p("lineitem"), "l_id", "l_tax",          0));
    t!(write_strs(&o("Lineitem_returnflag"),    &p("lineitem"), "l_id", "l_returnflag",   0));
    t!(write_strs(&o("Lineitem_status"),        &p("lineitem"), "l_id", "l_linestatus",   0));
    t!(write_strs(&o("Lineitem_shipdate"),      &p("lineitem"), "l_id", "l_shipdate",     0));
    t!(write_strs(&o("Lineitem_commitdate"),    &p("lineitem"), "l_id", "l_commitdate",   0));
    t!(write_strs(&o("Lineitem_receiptdate"),   &p("lineitem"), "l_id", "l_receiptdate",  0));
    t!(write_strs(&o("Lineitem_shipinstruct"),  &p("lineitem"), "l_id", "l_shipinstruct", 0));
    t!(write_strs(&o("Lineitem_shipmode"),      &p("lineitem"), "l_id", "l_shipmode",     0));
    t!(write_strs(&o("Lineitem_comment"),       &p("lineitem"), "l_id", "l_comment",      0));
}

// ======================== JOB ========================
//
// Faithful port of the retired Julia loader (julia-engine branch,
// julia-engine branch JOB.jl `load_all!` + cache.jl). Semantics to preserve:
//
//   - pairs are emitted in parquet row order;
//   - a pair is skipped iff its key or its value is NULL (per-column
//     independence: a cast_info row with a NULL note still contributes to
//     the other cast columns);
//   - Company name/country come from a company_name lookup Dict
//     (last-write-wins on duplicate keys; lookup misses skip the pair);
//   - ids are written raw (1-based, as stored in the parquet) — the
//     runtime loader does the 0-based shift.

/// Open a parquet file projecting the given 0-based column indices.
/// Returns the reader plus, for each requested index, its position within
/// the projected batches (projection preserves file order, not request
/// order).
fn job_open(path: &Path, indices: &[usize]) -> (ParquetRecordBatchReader, Vec<usize>) {
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

/// Incremental bits-format writer: placeholder count up front, patched on
/// finish — so multi-gigabyte columns (cast_info) never sit in RAM.
struct BitsFile {
    f: BufWriter<File>,
    n: u64,
}

impl BitsFile {
    fn create(path: &Path) -> Self {
        let mut f = BufWriter::new(File::create(path).unwrap());
        f.write_all(&0u64.to_le_bytes()).unwrap();
        BitsFile { f, n: 0 }
    }
    fn push(&mut self, k: i64, v: i64) {
        self.f.write_all(&k.to_le_bytes()).unwrap();
        self.f.write_all(&v.to_le_bytes()).unwrap();
        self.n += 1;
    }
    fn finish(mut self) {
        self.f.seek(SeekFrom::Start(0)).unwrap();
        self.f.write_all(&self.n.to_le_bytes()).unwrap();
        self.f.flush().unwrap();
    }
}

/// Str-format writer. Keys and offsets precede the bytes in the file, so
/// everything is buffered and written on finish.
struct StrFile {
    path: PathBuf,
    keys: Vec<i64>,
    offsets: Vec<u32>,
    bytes: Vec<u8>,
}

impl StrFile {
    fn create(path: &Path) -> Self {
        StrFile { path: path.to_path_buf(), keys: Vec::new(), offsets: vec![0], bytes: Vec::new() }
    }
    fn push(&mut self, k: i64, s: &str) {
        self.keys.push(k);
        self.bytes.extend_from_slice(s.as_bytes());
        self.offsets.push(self.bytes.len() as u32);
    }
    fn finish(self) {
        let n = self.keys.len() as u64;
        let mut f = BufWriter::new(File::create(&self.path).unwrap());
        f.write_all(&n.to_le_bytes()).unwrap();
        let k_bytes = unsafe {
            std::slice::from_raw_parts(self.keys.as_ptr() as *const u8, self.keys.len() * 8)
        };
        f.write_all(k_bytes).unwrap();
        let o_bytes = unsafe {
            std::slice::from_raw_parts(self.offsets.as_ptr() as *const u8, self.offsets.len() * 4)
        };
        f.write_all(o_bytes).unwrap();
        f.write_all(&self.bytes).unwrap();
    }
}

/// Two int columns → bits file. Pair emitted iff both values are non-NULL
/// (Julia `_load!`).
fn job_bits(out: &Path, parquet: &Path, key_idx: usize, val_idx: usize) {
    let (reader, pos) = job_open(parquet, &[key_idx, val_idx]);
    let mut w = BitsFile::create(out);
    for batch in reader {
        let batch = batch.unwrap();
        let k = int_col(&batch, pos[0]);
        let v = int_col(&batch, pos[1]);
        for i in 0..batch.num_rows() {
            if let (Some(k), Some(v)) = (k.get(i), v.get(i)) {
                w.push(k, v);
            }
        }
    }
    w.finish();
}

/// Int key column + string column → str file. Same both-non-NULL rule.
fn job_strs(out: &Path, parquet: &Path, key_idx: usize, val_idx: usize) {
    let (reader, pos) = job_open(parquet, &[key_idx, val_idx]);
    let mut w = StrFile::create(out);
    for batch in reader {
        let batch = batch.unwrap();
        let k = int_col(&batch, pos[0]);
        let v = str_col(&batch, pos[1]);
        for i in 0..batch.num_rows() {
            if let (Some(k), Some(v)) = (k.get(i), v.get(i)) {
                w.push(k, v);
            }
        }
    }
    w.finish();
}

fn run_job(parquet_dir: &Path, cache_dir: &Path) {
    std::fs::create_dir_all(cache_dir).unwrap();

    let p = |name: &str| parquet_dir.join(format!("{name}.parquet"));
    let o = |name: &str| cache_dir.join(format!("{name}.bin"));

    // ---- title (Movie): id, title, kind_id(3), production_year(4), episode_nr(9) ----
    eprintln!("title");
    t!({
        let (reader, pos) = job_open(&p("title"), &[0, 1, 3, 4, 9]);
        let mut title = StrFile::create(&o("Movie_title"));
        let mut kind = BitsFile::create(&o("Movie_kind"));
        let mut year = BitsFile::create(&o("Movie_production_year"));
        let mut epnr = BitsFile::create(&o("Movie_episode_nr"));
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let ti = str_col(&batch, pos[1]);
            let kd = int_col(&batch, pos[2]);
            let py = int_col(&batch, pos[3]);
            let ep = int_col(&batch, pos[4]);
            for i in 0..batch.num_rows() {
                let Some(id) = id.get(i) else { continue };
                if let Some(v) = ti.get(i) { title.push(id, v); }
                if let Some(v) = kd.get(i) { kind.push(id, v); }
                if let Some(v) = py.get(i) { year.push(id, v); }
                if let Some(v) = ep.get(i) { epnr.push(id, v); }
            }
        }
        title.finish(); kind.finish(); year.finish(); epnr.finish();
    });

    // ---- kind_type ----
    eprintln!("kind_type");
    t!(job_strs(&o("Kind_kind"), &p("kind_type"), 0, 1));

    // ---- keyword + movie_keyword ----
    eprintln!("keyword");
    t!(job_strs(&o("Keyword_keyword"), &p("keyword"), 0, 1));
    t!(job_bits(&o("Movie_keyword"), &p("movie_keyword"), 1, 2));

    // ---- company_name + company_type + movie_companies → Company ----
    // Company entities are movie_companies rows, with name/country joined
    // from company_name via id lookup (miss ⇒ skip the pair).
    eprintln!("company");
    t!(job_strs(&o("CompanyType_kind"), &p("company_type"), 0, 1));
    t!({
        let mut cn_name: HashMap<i64, String> = HashMap::new();
        let mut cn_country: HashMap<i64, String> = HashMap::new();
        let (reader, pos) = job_open(&p("company_name"), &[0, 1, 2]);
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
        let (reader, pos) = job_open(&p("movie_companies"), &[0, 1, 2, 3, 4]);
        let mut mv_co = BitsFile::create(&o("Movie_company"));
        let mut co_name = StrFile::create(&o("Company_name"));
        let mut co_country = StrFile::create(&o("Company_country"));
        let mut co_type = BitsFile::create(&o("Company_type"));
        let mut co_note = StrFile::create(&o("Company_note"));
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let mv = int_col(&batch, pos[1]);
            let co = int_col(&batch, pos[2]);
            let ct = int_col(&batch, pos[3]);
            let nt = str_col(&batch, pos[4]);
            for i in 0..batch.num_rows() {
                let Some(cid) = id.get(i) else { continue };
                if let Some(mid) = mv.get(i) { mv_co.push(mid, cid); }
                if let Some(cn) = co.get(i) {
                    if let Some(nm) = cn_name.get(&cn) { co_name.push(cid, nm); }
                    if let Some(cc) = cn_country.get(&cn) { co_country.push(cid, cc); }
                }
                if let Some(v) = ct.get(i) { co_type.push(cid, v); }
                if let Some(v) = nt.get(i) { co_note.push(cid, v); }
            }
        }
        mv_co.finish(); co_name.finish(); co_country.finish(); co_type.finish(); co_note.finish();
    });

    // ---- info_type ----
    eprintln!("info_type");
    t!(job_strs(&o("InfoType_info"), &p("info_type"), 0, 1));

    // ---- movie_info → Info: id, movie_id, info_type_id, info, note ----
    eprintln!("movie_info");
    t!({
        let (reader, pos) = job_open(&p("movie_info"), &[0, 1, 2, 3, 4]);
        let mut mv_info = BitsFile::create(&o("Movie_info"));
        let mut info_type = BitsFile::create(&o("Info_type"));
        let mut info_text = StrFile::create(&o("Info_info"));
        let mut info_note = StrFile::create(&o("Info_note"));
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let mv = int_col(&batch, pos[1]);
            let ty = int_col(&batch, pos[2]);
            let tx = str_col(&batch, pos[3]);
            let nt = str_col(&batch, pos[4]);
            for i in 0..batch.num_rows() {
                let Some(iid) = id.get(i) else { continue };
                if let Some(mid) = mv.get(i) { mv_info.push(mid, iid); }
                if let Some(v) = ty.get(i) { info_type.push(iid, v); }
                if let Some(v) = tx.get(i) { info_text.push(iid, v); }
                if let Some(v) = nt.get(i) { info_note.push(iid, v); }
            }
        }
        mv_info.finish(); info_type.finish(); info_text.finish(); info_note.finish();
    });

    // ---- movie_info_idx → Data: id, movie_id, info_type_id, info ----
    eprintln!("movie_info_idx");
    t!({
        let (reader, pos) = job_open(&p("movie_info_idx"), &[0, 1, 2, 3]);
        let mut mv_data = BitsFile::create(&o("Movie_data"));
        let mut data_type = BitsFile::create(&o("Data_type"));
        let mut data_text = StrFile::create(&o("Data_data"));
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let mv = int_col(&batch, pos[1]);
            let ty = int_col(&batch, pos[2]);
            let dx = str_col(&batch, pos[3]);
            for i in 0..batch.num_rows() {
                let Some(did) = id.get(i) else { continue };
                if let Some(mid) = mv.get(i) { mv_data.push(mid, did); }
                if let Some(v) = ty.get(i) { data_type.push(did, v); }
                if let Some(v) = dx.get(i) { data_text.push(did, v); }
            }
        }
        mv_data.finish(); data_type.finish(); data_text.finish();
    });

    // ---- link_type + movie_link → MovieLink: id, movie_id, linked_movie_id, link_type_id ----
    eprintln!("movie_link");
    t!(job_strs(&o("LinkType_link"), &p("link_type"), 0, 1));
    t!({
        let (reader, pos) = job_open(&p("movie_link"), &[0, 1, 2, 3]);
        let mut mv_link = BitsFile::create(&o("Movie_link"));
        let mut mv_linked_by = BitsFile::create(&o("Movie_linked_by"));
        let mut ml_target = BitsFile::create(&o("MovieLink_target"));
        let mut ml_type = BitsFile::create(&o("MovieLink_type"));
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let sr = int_col(&batch, pos[1]);
            let tg = int_col(&batch, pos[2]);
            let lt = int_col(&batch, pos[3]);
            for i in 0..batch.num_rows() {
                let Some(mlid) = id.get(i) else { continue };
                if let Some(src) = sr.get(i) { mv_link.push(src, mlid); }
                if let Some(tgt) = tg.get(i) {
                    mv_linked_by.push(tgt, mlid);
                    ml_target.push(mlid, tgt);
                }
                if let Some(v) = lt.get(i) { ml_type.push(mlid, v); }
            }
        }
        mv_link.finish(); mv_linked_by.finish(); ml_target.finish(); ml_type.finish();
    });

    // ---- aka_title: id, movie_id, title ----
    eprintln!("aka_title");
    t!(job_bits(&o("Movie_aka"), &p("aka_title"), 1, 0));
    t!(job_strs(&o("AkaTitle_title"), &p("aka_title"), 0, 2));

    // ---- name (Person): id, name, gender(4), name_pcode_cf(5) ----
    eprintln!("name");
    t!(job_strs(&o("Person_name"), &p("name"), 0, 1));
    t!(job_strs(&o("Person_gender"), &p("name"), 0, 4));
    t!(job_strs(&o("Person_name_pcode_cf"), &p("name"), 0, 5));

    // ---- char_name (Character) ----
    eprintln!("char_name");
    t!(job_strs(&o("Character_name"), &p("char_name"), 0, 1));

    // ---- role_type ----
    eprintln!("role_type");
    t!(job_strs(&o("RoleType_role"), &p("role_type"), 0, 1));

    // ---- aka_name: id, person_id, name ----
    eprintln!("aka_name");
    t!(job_strs(&o("AkaName_name"), &p("aka_name"), 0, 2));
    t!(job_bits(&o("Person_aka"), &p("aka_name"), 1, 0));

    // ---- comp_cast_type ----
    eprintln!("comp_cast_type");
    t!(job_strs(&o("CompCastType_kind"), &p("comp_cast_type"), 0, 1));

    // ---- complete_cast: id, movie_id, subject_id, status_id ----
    eprintln!("complete_cast");
    t!({
        let (reader, pos) = job_open(&p("complete_cast"), &[0, 1, 2, 3]);
        let mut mv_cc = BitsFile::create(&o("Movie_complete_cast"));
        let mut cc_subject = BitsFile::create(&o("CompleteCast_subject"));
        let mut cc_status = BitsFile::create(&o("CompleteCast_status"));
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let mv = int_col(&batch, pos[1]);
            let sj = int_col(&batch, pos[2]);
            let st = int_col(&batch, pos[3]);
            for i in 0..batch.num_rows() {
                let Some(ccid) = id.get(i) else { continue };
                if let Some(mid) = mv.get(i) { mv_cc.push(mid, ccid); }
                if let Some(v) = sj.get(i) { cc_subject.push(ccid, v); }
                if let Some(v) = st.get(i) { cc_status.push(ccid, v); }
            }
        }
        mv_cc.finish(); cc_subject.finish(); cc_status.finish();
    });

    // ---- person_info → PersonInfo: id, person_id, info_type_id, info, note ----
    eprintln!("person_info");
    t!({
        let (reader, pos) = job_open(&p("person_info"), &[0, 1, 2, 3, 4]);
        let mut pe_info = BitsFile::create(&o("Person_info"));
        let mut pi_type = BitsFile::create(&o("PersonInfo_type"));
        let mut pi_info = StrFile::create(&o("PersonInfo_info"));
        let mut pi_note = StrFile::create(&o("PersonInfo_note"));
        for batch in reader {
            let batch = batch.unwrap();
            let id = int_col(&batch, pos[0]);
            let pe = int_col(&batch, pos[1]);
            let ty = int_col(&batch, pos[2]);
            let inf = str_col(&batch, pos[3]);
            let nt = str_col(&batch, pos[4]);
            for i in 0..batch.num_rows() {
                let Some(piid) = id.get(i) else { continue };
                if let Some(pid) = pe.get(i) { pe_info.push(pid, piid); }
                if let Some(v) = ty.get(i) { pi_type.push(piid, v); }
                if let Some(v) = inf.get(i) { pi_info.push(piid, v); }
                if let Some(v) = nt.get(i) { pi_note.push(piid, v); }
            }
        }
        pe_info.finish(); pi_type.finish(); pi_info.finish(); pi_note.finish();
    });

    // ---- cast_info (Cast) — the big one (~36M rows) ----
    // Columns: id, person_id, movie_id, person_role_id, note, nr_order, role_id(6).
    eprintln!("cast_info");
    t!({
        let (reader, pos) = job_open(&p("cast_info"), &[0, 1, 2, 3, 4, 6]);
        let mut cast_movie = BitsFile::create(&o("Cast_movie"));
        let mut movie_cast = BitsFile::create(&o("Movie_cast"));
        let mut cast_person = BitsFile::create(&o("Cast_person"));
        let mut cast_character = BitsFile::create(&o("Cast_character"));
        let mut cast_role = BitsFile::create(&o("Cast_role"));
        let mut cast_note = StrFile::create(&o("Cast_note"));
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
                if let Some(m) = mv.get(i) {
                    cast_movie.push(cid, m);
                    movie_cast.push(m, cid);
                }
                if let Some(v) = pe.get(i) { cast_person.push(cid, v); }
                if let Some(v) = ch.get(i) { cast_character.push(cid, v); }
                if let Some(v) = ro.get(i) { cast_role.push(cid, v); }
                if let Some(v) = nt.get(i) { cast_note.push(cid, v); }
            }
        }
        cast_movie.finish(); movie_cast.finish(); cast_person.finish();
        cast_character.finish(); cast_role.finish(); cast_note.finish();
    });
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
            let parquet_dir = PathBuf::from(args.get(2).map(|s| s.as_str()).unwrap_or("/tmp/tpch5_clean"));
            let cache_dir   = PathBuf::from(args.get(3).map(|s| s.as_str()).unwrap_or("../cache"));
            run_tpch(&parquet_dir, &cache_dir);
        }
        // Legacy invocation: bare positional args = TPC-H.
        _ => {
            let parquet_dir = PathBuf::from(args.get(1).map(|s| s.as_str()).unwrap_or("/tmp/tpch5_clean"));
            let cache_dir   = PathBuf::from(args.get(2).map(|s| s.as_str()).unwrap_or("../cache"));
            run_tpch(&parquet_dir, &cache_dir);
        }
    }
    eprintln!("done.");
}
