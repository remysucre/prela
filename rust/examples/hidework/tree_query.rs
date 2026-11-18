// Run with: cargo run --release --features regen --example tree_query
// The engine_query.rs query, but the tables are loaded through the join-tree
// pass (src/join_tree.rs) instead of the whole cached schema. Writes every
// result row, sorted, to out_tree.txt for comparison with baseline_query.
use arrow::array::{Array, Float64Array, Int64Array};
use arrow::compute::cast;
use arrow::datatypes::DataType;
use arrow::record_batch::RecordBatch;
use prela::engine::*;
use prela::join_tree::*;
use prela::loader::Col;
use prela::tpch_schema::{Customer, Nation, Region};

fn main() {
    let dir = "../data/tpch/parquet";
    let customer_path = format!("{dir}/customer.parquet");
    let nation_path = format!("{dir}/nation.parquet");

    // nation.with(customers with acctbal >= 800).select(region), decomposed by
    // hand. Customer is the child: its Bloom over c_nationkey filters the
    // nation scan. Both materialize, since the prela query below still
    // probes customer (out_columns include each table's own key for that).
    let customer = Node {
        path: customer_path.clone(),
        children: vec![],
        predicates: vec![PredicateExpr::Cmp {
            col: "c_acctbal".into(),
            op: CmpOp::GtEq,
            literal: Literal::F64(9900.0),
        }],
        out_columns: names(&["c_custkey", "c_acctbal", "c_nationkey"]),
        materialize: true,
    };
    let nation = Node {
        path: nation_path.clone(),
        children: vec![Edge {
            child: customer,
            parent_key: "n_nationkey".into(),
            child_key: "c_nationkey".into(),
        }],
        predicates: vec![],
        out_columns: names(&["n_nationkey", "n_regionkey"]),
        materialize: true,
    };

    let mut sink = Sink::default();
    traverse(&nation, None, &mut sink);

    // Sink -> prela relations. Ids get the same shift regen applies when it
    // builds the cache (custkey is 1-based; nationkey and regionkey are
    // 0-based), so the printed ids match the baseline's.
    let cust = &sink.tables[&customer_path];
    let c_ids: Vec<usize> = ints(cust, "c_custkey").iter().map(|&k| (k - 1) as usize).collect();
    let n_cust = domain(&c_ids);
    let cust_key = key::<Customer>(n_cust, &c_ids);
    let acctbal: Col<Customer, f64> =
        VecRel::from_pairs(n_cust, c_ids.iter().copied().zip(floats(cust, "c_acctbal")));
    let c_nation: Col<Customer, Id<Nation>> = id_col(n_cust, &c_ids, &ints(cust, "c_nationkey"));

    let nat = &sink.tables[&nation_path];
    let n_ids: Vec<usize> = ints(nat, "n_nationkey").iter().map(|&k| k as usize).collect();
    let n_nat = domain(&n_ids);
    let nation_key = key::<Nation>(n_nat, &n_ids);
    let region: Col<Nation, Id<Region>> = id_col(n_nat, &n_ids, &ints(nat, "n_regionkey"));
    println!("tree: loaded {} customers, {} nations", c_ids.len(), n_ids.len());

    // The baseline query, driven from the reduced key sets instead of the
    // full entities. The exact .with() still runs, which removes any Bloom
    // false positives that reached the sink.
    let nations: MatSet<_> = cust_key
        .with((&acctbal).ge(800.0))
        .select(&c_nation)
        .collect();
    let q = nation_key.with(&nations).select(&region);

    let mut rows: Vec<(usize, usize)> = Vec::new();
    q.drive(|nk, rk| rows.push((nk.0, rk.0)));
    rows.sort();

    // Same line format as baseline_query, so the two files diff cleanly.
    let out: String = rows.iter().map(|(nk, rk)| format!("{nk} {rk}\n")).collect();
    std::fs::write("out_tree.txt", out).unwrap();
    println!("tree: {} rows -> out_tree.txt", rows.len());
}

fn names(cols: &[&str]) -> Vec<String> {
    cols.iter().map(|c| c.to_string()).collect()
}

// Same rule as the cache loader (regen's n_from_keys): max id + 1, here over
// the loaded rows only. Ids past it were never loaded, so nothing probes them.
fn domain(ids: &[usize]) -> usize {
    ids.iter().max().map_or(0, |&m| m + 1)
}

// VecRel is dense, so every slot below its length counts as a member. Drive
// from a SparseUniverse over the ids actually loaded instead, the way the
// loader handles Order's gappy ids. The Bitset is leaked because
// SparseUniverse holds a &'static, like loader::sparse_key.
fn key<E: 'static>(n: usize, ids: &[usize]) -> SparseUniverse<Id<E>> {
    let mut valid = Bitset::empty(Universe::<Id<E>>::new(n));
    for &i in ids {
        valid.set(Id::new(i));
    }
    SparseUniverse::new(n, Box::leak(Box::new(valid)))
}

// A foreign-key column. Slots for rows that were not loaded hold NO_ID, like
// regen's hole fill, so they can never match a real id.
fn id_col<E: 'static, T: 'static>(n: usize, ids: &[usize], vals: &[i64]) -> Col<E, Id<T>> {
    let mut v = vec![Id::<T>::new(NO_ID); n];
    for (&i, &x) in ids.iter().zip(vals) {
        v[i] = Id::new(x as usize);
    }
    VecRel::new(v)
}

// One integer column across all of a table's batches, widened to i64
// (c_custkey is Int64 in the files, the nationkeys are Int32).
fn ints(batches: &[RecordBatch], name: &str) -> Vec<i64> {
    let mut out = Vec::new();
    for b in batches {
        let col = cast(b.column_by_name(name).unwrap(), &DataType::Int64).unwrap();
        out.extend(col.as_any().downcast_ref::<Int64Array>().unwrap().values().iter());
    }
    out
}

fn floats(batches: &[RecordBatch], name: &str) -> Vec<f64> {
    let mut out = Vec::new();
    for b in batches {
        let col = b.column_by_name(name).unwrap();
        out.extend(col.as_any().downcast_ref::<Float64Array>().unwrap().values().iter());
    }
    out
}
