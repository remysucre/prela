// Run with (from prela/rust):
//   cargo run --release --features regen --example testing [-- <query_name>]
//
// For each query (examples/testingPlan.md) it runs the baseline over the
// whole cached schema and the join-tree version over the reduced Sink,
// writes both outputs to examples/testing/out/, and reports PASS when they
// are identical along with how many rows each one loaded.
mod helpers;
mod queries;

use prela::join_tree::row_count;

use helpers::path;
use queries::all_queries;

const OUT_DIR: &str = "examples/testing/out";

fn write_rows(file: &str, rows: &[String]) {
    let text: String = rows.iter().map(|r| format!("{r}\n")).collect();
    std::fs::write(file, text).unwrap();
}

fn main() {
    let only = std::env::args().nth(1);
    let mut queries = all_queries();
    if let Some(name) = &only {
        queries.retain(|q| q.name == *name);
        if queries.is_empty() {
            let known: Vec<_> = all_queries().iter().map(|q| q.name).collect();
            eprintln!("no query named {name}; known: {}", known.join(", "));
            std::process::exit(2);
        }
    }
    std::fs::create_dir_all(OUT_DIR).unwrap();

    // (name, baseline rows loaded, tree rows loaded, output rows, passed)
    let mut summary = Vec::new();
    for q in &queries {
        let base = (q.baseline)();
        let (tree, tree_loaded) = (q.tree)();

        // Always written, so `diff` can be run by hand after a failure.
        let base_file = format!("{OUT_DIR}/out_baseline_{}.txt", q.name);
        let tree_file = format!("{OUT_DIR}/out_tree_{}.txt", q.name);
        write_rows(&base_file, &base);
        write_rows(&tree_file, &tree);

        // The baseline loads every table whole.
        let base_loaded: usize = q.tables.iter().map(|t| row_count(&path(t))).sum();
        // Both are sorted, so equal vectors means equal files.
        let pass = base == tree;
        println!(
            "{}: {}  (output {} vs {} rows; loaded {} of {} rows)",
            q.name,
            if pass { "PASS" } else { "FAIL" },
            base.len(),
            tree.len(),
            tree_loaded,
            base_loaded,
        );
        if !pass {
            println!("  diff {base_file} {tree_file}");
        }
        if base.is_empty() && tree.is_empty() {
            println!("  warning: both outputs are empty, so this PASS proves nothing; loosen the thresholds");
        }
        summary.push((q.name, base_loaded, tree_loaded, base.len(), pass));
    }

    println!();
    println!("{:<24} {:>13} {:>10} {:>7} {:>9}  result", "query", "baseline rows", "tree rows", "ratio", "out rows");
    for &(name, base_loaded, tree_loaded, out, pass) in &summary {
        let ratio = tree_loaded as f64 / base_loaded as f64;
        let result = if pass { "PASS" } else { "FAIL" };
        println!("{name:<24} {base_loaded:>13} {tree_loaded:>10} {ratio:>7.3} {out:>9}  {result}");
    }

    let failed = summary.iter().filter(|s| !s.4).count();
    if failed > 0 {
        println!("\n{failed} of {} queries FAILED", summary.len());
        std::process::exit(1);
    }
}
