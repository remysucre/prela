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
# Env knobs (defaults shown):
#   REPS=3   SF=1   THREADS=1   DUCKDB=duckdb
#   PRELA_CACHE=<repo>/cache      — binary cache the suites mmap
#   PLOT=1                        — regenerate tpch_scatter.png afterwards
#   PYTHON=python3                — needs matplotlib; see the note below
#   MAXLOAD=2.5                   — refuse to start above this 1-min load
#   ALLOW_BATTERY=0               — set 1 to capture on battery anyway

set -e
BENCH=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$BENCH/../.." && pwd)
REPS=${REPS:-3}
SF=${SF:-1}
THREADS=${THREADS:-1}
PLOT=${PLOT:-1}
PYTHON=${PYTHON:-python3}
MAXLOAD=${MAXLOAD:-2.5}
ALLOW_BATTERY=${ALLOW_BATTERY:-0}
DATA=$BENCH/data

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# Power state gate. Running on battery — or with Low Power Mode on — throttles
# this machine by roughly 2.5x, and it does not hit both engines equally, so a
# capture taken that way silently invents a ratio. This has burned a capture
# before; the gate is not optional politeness.
POWER=$(pmset -g ps | head -1 | sed -n "s/.*drawing from '\(.*\)'.*/\1/p")
LPM=$(pmset -g | awk '/lowpowermode/ {print $2}')
if [ "$ALLOW_BATTERY" != 1 ]; then
    if [ "$POWER" != "AC Power" ]; then
        echo "refusing to capture on '$POWER' — plug in (ALLOW_BATTERY=1 to override)" >&2
        exit 1
    fi
    if [ "$LPM" = 1 ]; then
        echo "refusing to capture with Low Power Mode on — turn it off in" >&2
        echo "System Settings > Battery (ALLOW_BATTERY=1 to override)" >&2
        exit 1
    fi
fi

LOAD1=$(uptime | sed 's/.*load averages*: //' | awk '{print $1}' | tr -d ,)
if [ "$(echo "$LOAD1 > $MAXLOAD" | bc -l)" = 1 ]; then
    echo "refusing to capture at load $LOAD1 (> MAXLOAD=$MAXLOAD)" >&2
    exit 1
fi

echo "=== TPC-H capture: sf=$SF threads=$THREADS reps=$REPS ==="
echo "duckdb:  $(${DUCKDB:-duckdb} --version)"
echo "prela:   $(git -C "$REPO" rev-parse --short HEAD) $(git -C "$REPO" diff --quiet || echo '(dirty)')"
echo "power:   $POWER, lowpowermode=$LPM"
echo "load:    $(uptime | sed 's/.*load averages*: //')"
echo "cache:   ${PRELA_CACHE:-$REPO/cache}"

cd "$REPO/rust"
cargo build --release

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

    OUT=$TMP/duckdb_st.$rep.txt SF=$SF THREADS=$THREADS "$BENCH/run_tpch_duck.sh" > /dev/null
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
    # matplotlib is not on the system python here; `uv run --with matplotlib
    # --python 3.13 python plot_tpch.py` works without installing anything.
    if "$PYTHON" -c "import matplotlib" 2>/dev/null; then
        (cd "$BENCH" && "$PYTHON" plot_tpch.py)
    else
        echo "note: $PYTHON has no matplotlib — chart not regenerated." >&2
        echo "      cd rust/bench && uv run --with matplotlib --python 3.13 python plot_tpch.py" >&2
    fi
fi
