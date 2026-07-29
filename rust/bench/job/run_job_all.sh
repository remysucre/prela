#!/bin/bash
# Measures DuckDB and Prela performance on JOB.
# Produces scatter plot.
# One-session capture of every number on the JOB scatter: the DuckDB-ST
# baseline and the prela suite, interleaved on the same machine state, then
# the chart.
#
# Env vars (=default):
#   REPS=2   DUCKDB=duckdb
#   DBFILE=/tmp/ddb_bench/job.duckdb   — built from PARQUET on first run
#   PARQUET=<repo>/data/imdb/parquet   — from get_imdb.sh
#   QDIR=<repo>/../join-order-benchmark — from get_imdb.sh (JOB_REPO_DIR)
#   PRELA_CACHE=<repo>/data/cache      — binary cache the suite mmaps
#   PLOT=1   PYTHON=python3            — see the matplotlib note below

set -e
JOB=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$JOB/../../.." && pwd)
REPS=${REPS:-2}
PLOT=${PLOT:-1}
PYTHON=${PYTHON:-python3}
CACHE=${PRELA_CACHE:-$REPO/data/cache}
DATA=$JOB/data
DUCKDB=${DUCKDB:-duckdb}
DBFILE=${DBFILE:-/tmp/ddb_bench/job.duckdb}
PARQUET=${PARQUET:-$REPO/data/imdb/parquet}
QDIR=${QDIR:-$REPO/data/join-order-benchmark}
QNAMES=$DATA/job_qnames.txt

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

missing=""
for f in Cast_person Company_name AkaName_text Character_text; do
    [ -f "$CACHE/$f.bin" ] || missing="$missing $f.bin"
done
if [ -n "$missing" ]; then
    echo "no JOB cache at: $CACHE" >&2
    echo "  missing:$missing" >&2
    echo "" >&2
    echo "Point PRELA_CACHE at a JOB cache, or build one:" >&2
    echo "  ./get_imdb.sh   (from rust/bench/)" >&2
    exit 1
fi

if [ ! -f "$QDIR/1a.sql" ]; then
    echo "no canonical JOB queries at: $QDIR" >&2
    echo "  clone https://github.com/gregrahn/join-order-benchmark, or set QDIR" >&2
    exit 1
fi

echo "=== JOB capture: reps=$REPS ==="
echo "duckdb:  $(${DUCKDB:-duckdb} --version)"
echo "prela:   $(git -C "$REPO" rev-parse --short HEAD) $(git -C "$REPO" diff --quiet || echo '(dirty)')"
echo "cache:   $CACHE"

cd "$REPO/rust"
cargo build --release
export PRELA_CACHE=$CACHE

# Prela total
prela_total() {
    awk '/^--- run 2 ---/ {r=1; next}
         r && /^[^ ]+ +(ok|DIFF)/ { for (i=1; i<=NF; i++) if ($i ~ /^[0-9.]+s$/) { sub(/s$/,"",$i); t+=$i } }
         END { printf "%.4f", t }' "$1"
}
# DuckDB total
duck_total() {
    awk '/^Run Time/ { n++; if (n % 2 == 0) { match($0, /real [0-9.]+/); t += substr($0, RSTART+5, RLENGTH-5) } }
         END { printf "%.4f", t }' "$1"
}

# Build DuckDB db
if [ ! -f "$DBFILE" ]; then
    if [ ! -d "$PARQUET" ]; then
        echo "no JOB database at $DBFILE and no parquet at $PARQUET to build it from" >&2
        exit 1
    fi
    echo "building $DBFILE from $PARQUET (one-time, ~4 GB) ..."
    mkdir -p "$(dirname "$DBFILE")"
    for f in "$PARQUET"/*.parquet; do
        t=$(basename "$f" .parquet)
        echo "CREATE TABLE $t AS SELECT * FROM read_parquet('$f');"
    done | $DUCKDB "$DBFILE"
fi

# `at` -> `att`: AT is reserved from duckdb 1.5.x on
# note: we use perl, not sed, for the replace
STAGE=$TMP/queries
mkdir -p "$STAGE"
for q in $(cat "$QNAMES"); do
    [ -f "$QDIR/$q.sql" ] || continue
    perl -pe 's/\bAS\s+at\b/AS att/g; s/\bat\./att./g' "$QDIR/$q.sql" >"$STAGE/$q.sql"
done

# One duckdb session per query, threads=1, run twice. $1=out $2=raw
capture_duck() {
    out=$1
    raw=$2
    >"$raw"
    for q in $(cat "$QNAMES"); do
        if [ ! -f "$STAGE/$q.sql" ]; then
            echo "$q MISSING" >>"$raw"
            continue
        fi
        output=$(
            $DUCKDB "$DBFILE" <<DDB 2>&1
PRAGMA threads=1;
.timer on
.read $STAGE/$q.sql
.read $STAGE/$q.sql
DDB
        )
        timings=($(echo "$output" | grep -oE "real [0-9.]+" | awk '{print $2}'))
        echo "$q cold=${timings[0]:-NA} warm=${timings[1]:-NA}" >>"$raw"
    done

    # check for bad queries
    bad=$(awk '$0 ~ /cold=(NA|0\.000)/ || $0 ~ /warm=(NA|0\.000)/ || /MISSING/ {print $1}' "$raw")
    if [ -n "$bad" ]; then
        echo "queries with no valid timing: $(echo $bad | tr '\n' ' ')" >&2
        exit 1
    fi

    # cold+warm "Run Time (s): real X" pairs, paired positionally with QNAMES.
    awk -F'[ =]' '
        NR==FNR { canonical[$1] = $0; next }
        {
            q = $0
            if (q in canonical) {
                split(canonical[q], parts, " ")
                cold = ""; warm = ""
                for (i in parts) {
                    if (parts[i] ~ /^cold=/) { sub(/^cold=/, "", parts[i]); cold = parts[i] }
                    if (parts[i] ~ /^warm=/) { sub(/^warm=/, "", parts[i]); warm = parts[i] }
                }
                print "Run Time (s): real " cold " user 0.0 sys 0.0"
                print "Run Time (s): real " warm " user 0.0 sys 0.0"
            }
        }
    ' "$raw" "$QNAMES" >"$out"
}

for rep in $(seq 1 "$REPS"); do
    echo "--- rep $rep/$REPS ---"

    capture_duck "$TMP/job_duck.$rep.txt" "$TMP/job_duck_canonical.$rep.txt"
    echo "  duckdb  $(duck_total "$TMP/job_duck.$rep.txt")s"

    ./target/release/prela >"$TMP/job_prela.$rep.txt" 2>&1
    if grep -q DIFF "$TMP/job_prela.$rep.txt"; then
        echo "  prela: oracle mismatch — refusing to write the capture" >&2
        grep -m5 DIFF "$TMP/job_prela.$rep.txt" >&2
        exit 1
    fi
    echo "  prela   $(prela_total "$TMP/job_prela.$rep.txt")s"
done

# Keep the fastest rep of each side. job_duck_canonical.txt is the raw
# cold=/warm= intermediate for the same rep, kept so the pair stays coherent.
echo "--- best of $REPS ---"
best_rep=""
best_t=""
for rep in $(seq 1 "$REPS"); do
    t=$(duck_total "$TMP/job_duck.$rep.txt")
    if [ -z "$best_t" ] || [ "$(echo "$t < $best_t" | bc -l)" = 1 ]; then
        best_rep=$rep
        best_t=$t
    fi
done
cp "$TMP/job_duck.$best_rep.txt" "$DATA/job_duck.txt"
cp "$TMP/job_duck_canonical.$best_rep.txt" "$DATA/job_duck_canonical.txt"
echo "  duckdb  ${best_t}s  -> data/job_duck.txt (+ job_duck_canonical.txt)"

best_rep=""
best_t=""
for rep in $(seq 1 "$REPS"); do
    t=$(prela_total "$TMP/job_prela.$rep.txt")
    if [ -z "$best_t" ] || [ "$(echo "$t < $best_t" | bc -l)" = 1 ]; then
        best_rep=$rep
        best_t=$t
    fi
done
cp "$TMP/job_prela.$best_rep.txt" "$DATA/job_prela.txt"
echo "  prela   ${best_t}s  -> data/job_prela.txt"

if [ "$PLOT" = 1 ]; then
    if "$PYTHON" -c "import matplotlib" 2>/dev/null; then
        (cd "$JOB" && "$PYTHON" plot_job.py)
    elif command -v uv >/dev/null; then
        echo "note: $PYTHON has no matplotlib. Using uv"
        (cd "$JOB" && uv run --with matplotlib --python 3.13 python plot_job.py)
    else
        echo "note: no matplotlib and no uv — chart not regenerated." >&2
        echo "      install either, then: cd rust/bench/job && ./plot_job.py" >&2
    fi
fi
