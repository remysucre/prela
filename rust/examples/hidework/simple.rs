// Run with: cargo run --release --features regen --example simple
// SELECT n_nationkey, n_regionkey
// FROM nation JOIN customer ON n_nationkey = c_nationkey
// WHERE c_acctbal >= 800.0          (as a semi-join: each nation appears once)
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};

use arrow::array::{Array, BooleanArray, Float64Array, Int32Array};
use arrow::compute::kernels::cmp::gt_eq;
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use parquet::arrow::ProjectionMask;
use parquet::arrow::arrow_reader::{
    ArrowPredicateFn, ArrowReaderOptions, ParquetRecordBatchReader,
    ParquetRecordBatchReaderBuilder, RowFilter,
};

// A tiny Bloom filter over i32 keys: 3 probes into a bit array.
// It may say "yes" for a key that was never inserted (false positive),
// but never "no" for a key that was. Make `bits` bigger to lower that rate.
struct Bloom(Vec<u64>);
impl Bloom {
    fn new(bits: usize) -> Self {
        Bloom(vec![0; bits / 64])
    }
    fn probes(&self, key: i32) -> [usize; 3] {
        let mut h = DefaultHasher::new();
        key.hash(&mut h);
        let h = h.finish();
        let (a, b) = (h as u32 as usize, (h >> 32) as usize | 1);
        let bits = self.0.len() * 64;
        [0, 1, 2].map(|i| a.wrapping_add(i * b) % bits)
    }
    fn insert(&mut self, key: i32) {
        for p in self.probes(key) {
            self.0[p / 64] |= 1 << (p % 64);
        }
    }
    fn has(&self, key: i32) -> bool {
        self.probes(key).iter().all(|&p| self.0[p / 64] >> (p % 64) & 1 == 1)
    }
}

// Read the `out` columns of a parquet file, keeping only rows where `pred` is true
// for the single column `filter_col`. Parquet applies the filter while decoding.
fn scan(
    path: &str,
    filter_col: &str,
    pred: impl FnMut(RecordBatch) -> Result<BooleanArray, ArrowError> + Send + 'static,
    out: &[&str],
) -> ParquetRecordBatchReader {
    let file = File::open(path).unwrap();
    // The page index lets Parquet skip whole pages that the filter rules out.
    let options = ArrowReaderOptions::new().with_page_index(true);
    let builder = ParquetRecordBatchReaderBuilder::try_new_with_options(file, options).unwrap();

    let idx = |name: &str| builder.schema().index_of(name).unwrap();
    let filter_mask = ProjectionMask::roots(builder.parquet_schema(), [idx(filter_col)]);
    let out_mask = ProjectionMask::roots(builder.parquet_schema(), out.iter().map(|c| idx(c)));

    let filter = RowFilter::new(vec![Box::new(ArrowPredicateFn::new(filter_mask, pred))]);
    builder.with_projection(out_mask).with_row_filter(filter).build().unwrap()
}

fn main() {
    let dir = "../data/tpch/parquet";

    // Step 1: customers with acctbal >= 800 -> put their nationkeys in a Bloom filter.
    // Change this predicate to filter customers on anything else.
    let rich = |batch: RecordBatch| gt_eq(batch.column(0), &Float64Array::new_scalar(800.0));
    let mut bloom = Bloom::new(1 << 16);
    for batch in scan(&format!("{dir}/customer.parquet"), "c_acctbal", rich, &["c_nationkey"]) {
        let batch = batch.unwrap();
        let keys = batch.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
        keys.iter().for_each(|k| bloom.insert(k.unwrap()));
    }

    // Step 2: read nation, keeping only rows whose nationkey is in the Bloom filter.
    let in_bloom = move |batch: RecordBatch| {
        let keys = batch.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
        Ok(keys.iter().map(|k| Some(bloom.has(k.unwrap()))).collect())
    };
    let mut rows: Vec<(i32, i32)> = Vec::new();
    for batch in scan(&format!("{dir}/nation.parquet"), "n_nationkey", in_bloom, &["n_nationkey", "n_regionkey"]) {
        let batch = batch.unwrap();
        let nation = batch.column_by_name("n_nationkey").unwrap().as_any().downcast_ref::<Int32Array>().unwrap();
        let region = batch.column_by_name("n_regionkey").unwrap().as_any().downcast_ref::<Int32Array>().unwrap();
        for i in 0..batch.num_rows() {
            rows.push((nation.value(i), region.value(i)));
        }
    }

    // Sort and print the first 5 (nationkey, regionkey).
    rows.sort();
    for (n, (nationkey, regionkey)) in rows.iter().take(5).enumerate() {
        println!("{n}: {nationkey} {regionkey}");
    }
}
