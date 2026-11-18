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

// This is a bloom filter implementation for now which we will use as the structure that we want 
struct Bloom(Vec<u64>) {
    filter_col: usize; // it should know whichc column it's built on--assume its a single one for now
    path: &str; // it should know what file it's built on 
};

// when we construct a Bloom filter we should already know what the file and column is built on 
// so it's okay to couple IO with just keeping track of membership
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
    /* 
    our bloom filter will also know what column it's built on so that we can just can just 
    call its predicateFn method to get the function whenever we want
    */
    fn predicateFn(&self, idx_col: usize) -> impl FnMut(RecordBatch) -> Result<BooleanArray> {
        move |batch: RecordBatch| {
        let keys = batch.column(idx_col).as_any().downcast_ref::<Int32Array>().unwrap();
        Ok(keys.iter().map(|k| Some(bloom.has(k.unwrap()))).collect())
    }
    }
    // effectively the idea is that after we populate the Bloom filter we can easily just retrieve the 
    // mask that interacts with Parquet directly
    fn getFilter(&self) -> Box<dyn ArrowPredicate> {
        let file = File::open(self.path).unwrap();
        let options = ArrowReaderOptions::new().with_page_index(true);
        let builder = ParquetRecordBatchReaderBuilder::try_new_with_options(file, options).unwrap();

        let idx = |name: &str| builder.schema().index_of(name).unwrap();
        let filter_mask = ProjectionMask::roots(builder.parquet_schema(), [idx(self.filter_col)]);

        Box::new(ArrowPredicateFn::new(filter_mask, self.predicateFn(idx(self.filter_col))));
    }
}

struct Predicate{
    // should be a similar kind of struct as Bloom which can handle the internals of acting or combing
    // multiple columns so that we can externally just get one single filter from it
}


// that way we can get the vector from just calling each filter and then doing filter = ROwFilter::new(filter array)

