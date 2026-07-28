#!/bin/bash
# One-session capture of every number on the TPC-H scatter: the DuckDB-ST
# baseline and both prela variants, interleaved on the same machine state,
# then the chart.
#
# Why one script rather than three: absolute times on this hardware vary far
# more between sessions (tens of percent — thermals, AC power, background
# load) than the differences being measured, while ratios captured together
# hold. Refreshing one side alone produces a plot whose ratios mean nothing.
# If you only want to re-time prela, re-run this anyway.
#
# Each rep runs duckdb → idiomatic → optimized; the best (lowest warm total)
# rep per variant is what gets written, so a burst of background load costs a
# rep rather than corrupting the capture.
#
# SF=1 only. The scale factor is not a knob here: these scripts regenerate
# DuckDB's dbgen tables but not prela's binary cache, so an SF knob would time
# DuckDB at one scale against whatever cache PRELA_CACHE happens to hold — and
# the checked-in oracles are SF=1 answers, so nothing else verifies anyway.
# Other scales belong in a sweep script that rebuilds the cache per scale.
#
# Recorded but not gated: power state. Low Power Mode slows DuckDB ~2.5x here
# and does not hit both engines equally, so a capture taken under it invents a
# ratio rather than being uniformly slow. The header prints it — check it
# before trusting a ratio against a capture from another session.
#
# Env knobs (defaults shown):
#   REPS=3   THREADS=1   DUCKDB=duckdb
#   PRELA_CACHE=<repo>/cache      — binary cache the suites mmap
#   PLOT=1                        — regenerate tpch_scatter.png afterwards
#   PYTHON=python3                — needs matplotlib; see the note below
#   MAXLOAD=2.5                   — refuse to start above this 1-min load

set -e
BENCH=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$BENCH/../.." && pwd)
REPS=${REPS:-3}
SF=1
THREADS=${THREADS:-1}
PLOT=${PLOT:-1}
PYTHON=${PYTHON:-python3}
MAXLOAD=${MAXLOAD:-2.5}
CACHE=${PRELA_CACHE:-$REPO/cache}
DATA=$BENCH/data

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# The suites mmap a *TPC-H* binary cache. The default ../cache in this checkout
# holds the JOB tables, so without PRELA_CACHE the run panics several minutes
# in, inside a rep, on a missing .bin. Check up front and say what to do.
missing=""
for f in Region_name Lineitem_order Order_status Supplier_nation; do
    [ -f "$CACHE/$f.bin" ] || missing="$missing $f.bin"
done
if [ -n "$missing" ]; then
    echo "no TPC-H cache at: $CACHE" >&2
    echo "  missing:$missing" >&2
    echo "" >&2
    echo "Point PRELA_CACHE at a TPC-H cache, e.g." >&2
    echo "  PRELA_CACHE=/path/to/prela/cache $0" >&2
    echo "or build one from cache/tpch/*.parquet (see benchmarking.md):" >&2
    echo "  cd $REPO/rust && cargo run --release --features regen --bin regen -- tpch ../cache/tpch ../cache" >&2
    exit 1
fi

LOAD1=$(uptime | sed 's/.*load averages*: //' | awk '{print $1}' | tr -d ,)
if [ "$(echo "$LOAD1 > $MAXLOAD" | bc -l)" = 1 ]; then
    echo "refusing to capture at load $LOAD1 (> MAXLOAD=$MAXLOAD)" >&2
    exit 1
fi

echo "=== TPC-H capture: sf=$SF threads=$THREADS reps=$REPS ==="
echo "duckdb:  $(${DUCKDB:-duckdb} --version)"
echo "prela:   $(git -C "$REPO" rev-parse --short HEAD) $(git -C "$REPO" diff --quiet || echo '(dirty)')"
echo "power:   $(pmset -g ps | head -1 | sed -n "s/.*drawing from '\(.*\)'.*/\1/p"), lowpowermode=$(pmset -g | awk '/lowpowermode/ {print $2}')"
echo "load:    $(uptime | sed 's/.*load averages*: //')"
echo "cache:   $CACHE"

cd "$REPO/rust"
cargo build --release

# Pin the suites to the cache we just verified, not whatever was ambient.
export PRELA_CACHE=$CACHE

# Warm total of a prela capture: the run-2 per-query times.
prela_total() {
    awk '/^--- run 2 ---/ {r=1; next}
         r && /^[0-9]+ +(ok|DIFF)/ { for (i=1; i<=NF; i++) if ($i ~ /s$/) { sub(/s$/,"",$i); t+=$i } }
         END { printf "%.4f", t }' "$1"
}
# Warm total of a duckdb capture: every second `Run Time` line.
duck_total() {
    awk '/^Run Time/ { n++; if (n % 2 == 0) { match($0, /real [0-9.]+/); t += substr($0, RSTART+5, RLENGTH-5) } }
         END { printf "%.4f", t }' "$1"
}

for rep in $(seq 1 "$REPS"); do
    echo "--- rep $rep/$REPS ---"

    OUT=$TMP/duckdb_st.$rep.txt THREADS=$THREADS "$BENCH/run_tpch_duck.sh" > /dev/null
    echo "  duckdb     $(duck_total "$TMP/duckdb_st.$rep.txt")s"

    for qs in idiomatic optimized; do
        QS=$qs ./target/release/prela tpch > "$TMP/$qs.$rep.txt" 2>&1
        if grep -q DIFF "$TMP/$qs.$rep.txt"; then
            echo "  $qs: oracle mismatch — refusing to write the capture" >&2
            grep -m5 DIFF "$TMP/$qs.$rep.txt" >&2
            exit 1
        fi
        echo "  $qs  $(prela_total "$TMP/$qs.$rep.txt")s"
    done
done

# Keep the fastest rep of each variant.
echo "--- best of $REPS ---"
for name in duckdb_st idiomatic optimized; do
    best=""; best_t=""
    for rep in $(seq 1 "$REPS"); do
        f=$TMP/$name.$rep.txt
        case $name in
            duckdb_st) t=$(duck_total "$f") ;;
            *)         t=$(prela_total "$f") ;;
        esac
        if [ -z "$best_t" ] || [ "$(echo "$t < $best_t" | bc -l)" = 1 ]; then
            best=$f; best_t=$t
        fi
    done
    cp "$best" "$DATA/$name.txt"
    echo "  $name  ${best_t}s  -> data/$name.txt"
done

if [ "$PLOT" = 1 ]; then
    # No system python here has matplotlib, so fall back to uv, which fetches
    # it into a throwaway env — that keeps the whole capture one command.
    if "$PYTHON" -c "import matplotlib" 2>/dev/null; then
        (cd "$BENCH" && "$PYTHON" plot_tpch.py)
    elif command -v uv > /dev/null; then
        echo "note: $PYTHON has no matplotlib — plotting via uv"
        (cd "$BENCH" && uv run --with matplotlib --python 3.13 python plot_tpch.py)
    else
        echo "note: no matplotlib and no uv — chart not regenerated." >&2
        echo "      install either, then: cd rust/bench && ./plot_tpch.py" >&2
    fi
fi
