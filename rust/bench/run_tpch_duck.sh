#!/bin/bash
# Run the 22 canonical TPC-H queries against single-threaded DuckDB, writing
# the `Run Time (s): real …` cold/warm pairs that plot_tpch.py expects.
#
# The baseline is timed against native dbgen tables (not the parquet cache),
# matching the checked-in data/duckdb_st.txt: Prela's binary cache is
# schema-isomorphic to them, so no counterpart run is needed (see
# benchmarking.md, "Schema fairness").
#
# Env knobs (defaults shown):
#   DUCKDB=duckdb                          DuckDB CLI
#   DBFILE=/tmp/ddb_bench/tpch_sf1.duckdb  built via CALL dbgen if missing
#   SF=1                                   scale factor used when building
#   QDIR=<tmp>/tpch_queries                canonical SQL; dumped from the
#                                          tpch extension's tpch_queries()
#   OUT=<script>/data/duckdb_st.txt        refuses to clobber unless FORCE=1
#
# Each query runs twice in one DuckDB process with PRAGMA threads=1;
# plot_tpch.py reads the second (warm) time of each pair.

set -e
DUCKDB=${DUCKDB:-duckdb}
SF=${SF:-1}
DBFILE=${DBFILE:-/tmp/ddb_bench/tpch_sf$SF.duckdb}
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
OUT=${OUT:-$SCRIPT_DIR/data/duckdb_st.txt}

if [ -e "$OUT" ] && [ "${FORCE:-0}" != "1" ]; then
    echo "refusing to overwrite existing $OUT (set FORCE=1 to allow)" >&2
    exit 1
fi
mkdir -p "$(dirname "$OUT")"

# Build the DB once (native dbgen tables).
if [ ! -f "$DBFILE" ]; then
    echo "building $DBFILE (sf=$SF) ..."
    mkdir -p "$(dirname "$DBFILE")"
    $DUCKDB "$DBFILE" -c "INSTALL tpch; LOAD tpch; CALL dbgen(sf = $SF);" > /dev/null
fi

# Dump the canonical queries from the tpch extension unless QDIR is provided.
if [ -z "$QDIR" ]; then
    QDIR=$(dirname "$DBFILE")/tpch_queries
    mkdir -p "$QDIR"
    $DUCKDB -c "LOAD tpch; COPY (SELECT query_nr, query FROM tpch_queries() ORDER BY query_nr)
                TO '$QDIR/queries.csv' (FORMAT CSV, HEADER false);" > /dev/null
    python3 - "$QDIR" <<'PY'
import csv, pathlib, sys
d = pathlib.Path(sys.argv[1])
for nr, q in csv.reader(open(d / "queries.csv")):
    (d / f"q{int(nr):02d}.sql").write_text(q.strip() + "\n")
PY
fi

> "$OUT"
for q in $(seq 1 22); do
    QFILE=$(printf "%s/q%02d.sql" "$QDIR" "$q")
    if [ ! -f "$QFILE" ]; then
        echo "MISSING SQL for Q$q: $QFILE" >&2
        exit 1
    fi
    output=$($DUCKDB "$DBFILE" <<DDB 2>&1
PRAGMA threads=1;
.mode trash
.timer on
.read $QFILE
.read $QFILE
DDB
)
    timings=($(echo "$output" | grep -oE "real [0-9.]+" | awk '{print $2}'))
    cold=${timings[0]:-NA}
    warm=${timings[1]:-NA}
    printf "Run Time (s): real %s user 0.0 sys 0.0\n" "$cold" >> "$OUT"
    printf "Run Time (s): real %s user 0.0 sys 0.0\n" "$warm" >> "$OUT"
    echo "Q$q  cold=$cold  warm=$warm"
done

echo "---"
echo "wrote $OUT ($(wc -l < "$OUT") lines, expected 44)"
awk 'NR%2==0 {match($0, /real [0-9.]+/); s += substr($0, RSTART+5, RLENGTH-5)}
     END {printf "Warm total: %.3f s over %d queries\n", s, NR/2}' "$OUT"
