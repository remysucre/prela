#!/bin/bash
# Run every query crate and summarize. Exits nonzero if any query differs.
#
# Release, because the queries whose FROM is a product of several LEFT JOINs
# drive that product, and unoptimised that is fifteen times slower for no
# change in the answers. See notes/limitations.md, "No optimiser".
#
#   ./tools/run_all.sh
set -uo pipefail
cd "$(dirname "$0")/../rust"

cargo build --release 2>&1 | grep -E '^error' && exit 1
fail=0
for c in c[0-9]*; do
    out=$(./target/release/"$c" 2>&1)
    echo "$out" | grep -E 'DIFF|SKIP' && fail=1
    echo "$c: $(echo "$out" | tail -1)"
done
exit $fail
