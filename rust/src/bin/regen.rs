// Regenerate the binary cache from clean DuckDB-exported parquet.
//
// Usage:
//   cargo run --release --features regen --bin regen -- /tmp/tpch5_clean /Users/remywang/projects/prela/cache
//
// Expects parquet files with all-BIGINT integer columns, DOUBLE for money,
// VARCHAR for strings (date columns pre-formatted as ISO yyyy-mm-dd).
//
// Writes <cache_dir>/<Entity>_<field>.bin files in the format the runtime
// loader expects (see julia/cache.jl):
//   bits  — [u64 n][n × (i64,i64)]
//   str   — [u64 n][n × i64 keys][(n+1) × u32 offsets][bytes]
//
// Per-field passes through parquet (column projection — touches only the
// two needed columns). Streaming: never holds more than one parquet row
// group in RAM, so SF=10+ on a 32GB machine is fine.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use arrow::array::{Array, Float64Array, Int64Array, StringArray};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ProjectionMask;

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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let parquet_dir = PathBuf::from(args.get(1).map(|s| s.as_str()).unwrap_or("/tmp/tpch5_clean"));
    let cache_dir   = PathBuf::from(args.get(2).map(|s| s.as_str()).unwrap_or("../cache"));
    std::fs::create_dir_all(&cache_dir).unwrap();

    let p = |name: &str| parquet_dir.join(format!("{name}.parquet"));
    let o = |name: &str| cache_dir.join(format!("{name}.bin"));

    macro_rules! t { ($s:expr) => { { let start = std::time::Instant::now(); $s; eprintln!("    {:.2}s", start.elapsed().as_secs_f32()); } }; }

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

    eprintln!("done.");
}
