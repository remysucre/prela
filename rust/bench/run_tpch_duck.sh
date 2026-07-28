#!/bin/bash
# TPC-H single-threaded DuckDB baseline — writes the `data/duckdb_st.txt`
# that plot_tpch.py reads as the x-axis of the scatter.
#
# Times the canonical TPCH SQL (DuckDB's own `tpch_queries()`, so the text
# tracks the installed extension) against *native dbgen tables* rather than
# the parquet cache: reading parquet would time arrow decoding instead of
# DuckDB's execution, and the baseline is meant to be fair to DuckDB. Each
# query runs twice in one session; the second (warm) timing is the one
# plot_tpch.py keeps.
#
# Prefer `run_tpch_all.sh`, which drives this and both prela variants in a
# single session — absolute times on this hardware move far more between
# sessions than the differences being measured.
#
# SF=1 only, deliberately: prela's side of the comparison reads a prebuilt
# binary cache that nothing here regenerates, so a scale-factor knob would
# only ever desynchronise the two engines. Other scales belong in a sweep
# script that rebuilds the cache per scale.
#
# Env knobs (defaults shown):
#   DUCKDB=duckdb          THREADS=1
#   OUT=<repo>/rust/bench/data/duckdb_st.txt

set -e
REPO=$(cd "$(dirname "$0")/../.." && pwd)
DUCKDB=${DUCKDB:-duckdb}
SF=1
THREADS=${THREADS:-1}
OUT=${OUT:-$REPO/rust/bench/data/duckdb_st.txt}

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# Pull the 22 canonical queries out of the tpch extension, one file each.
# Every one is a single statement (q15 uses a CTE, not a view), which is what
# makes the two-timings-per-query pairing below reliable.
for n in $(seq 1 22); do
    $DUCKDB -noheader -list \
        -c "LOAD tpch; SELECT query FROM tpch_queries() WHERE query_nr=$n;" \
        > "$TMP/q$n.sql"
    [ -s "$TMP/q$n.sql" ] || { echo "empty SQL for Q$n" >&2; exit 1; }
done

# One session: build the tables, then time each query cold + warm. `.bail on`
# aborts on the first error — an errored query would otherwise time as ~0s and
# silently flatter the baseline.
{
    echo ".bail on"
    echo "INSTALL tpch; LOAD tpch;"
    echo "SET threads=$THREADS;"
    echo "CALL dbgen(sf=$SF);"
    echo ".mode list"
    echo ".headers off"
    echo ".timer on"
    for n in $(seq 1 22); do
        echo ".read $TMP/q$n.sql"
        echo ".read $TMP/q$n.sql"
    done
} > "$TMP/run.sql"

# Run first, filter second — piping straight into grep would hide a non-zero
# duckdb exit behind grep's. Keep only the timings: the result rows run to
# ~40k lines (q16 alone returns thousands) and this file is checked in.
$DUCKDB < "$TMP/run.sql" > "$TMP/raw.txt"
grep '^Run Time' "$TMP/raw.txt" > "$TMP/out.txt" || true

# Gate: exactly two timings per query, or plot_tpch.py's positional parse
# (timer_lines[1::2]) silently attributes times to the wrong queries.
got=$(grep -c '^Run Time' "$TMP/out.txt" || true)
if [ "$got" -ne 44 ]; then
    echo "expected 44 'Run Time' lines, got $got — refusing to write $OUT" >&2
    exit 1
fi

mv "$TMP/out.txt" "$OUT"
echo "wrote $OUT  ($($DUCKDB --version), sf=$SF, threads=$THREADS)"
awk '/^Run Time/ {
        n++
        if (n % 2 == 0) { match($0, /real [0-9.]+/); t += substr($0, RSTART+5, RLENGTH-5) }
     }
     END { printf "  warm total %.3fs over %d queries\n", t, n/2 }' "$OUT"
