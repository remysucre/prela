#!/bin/sh
# Run from prela/rust: sh examples/compare_tree.sh
# Runs the baseline and the join-tree versions of the engine_query.rs query.
# An empty diff means the two produced identical results.
set -e
cargo run --release --features regen --example baseline_query
cargo run --release --features regen --example tree_query
diff out_baseline.txt out_tree.txt && echo "identical"
