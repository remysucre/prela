#!/bin/bash
# Run canonical JOB queries against single-threaded DuckDB, writing
# data/job_duck.txt in the format plot_job.py expects.
#
# Prerequisites:
#   - DuckDB CLI at $DUCKDB (default /Users/remywang/.duckdb/cli/latest/duckdb)
#   - JOB DB at $DBFILE  (default /tmp/ddb_bench/job.duckdb,
#     build once via parquet ingest — see README below)
#   - Canonical query SQLs at $QDIR (default ~/projects/join-order-benchmark)
#
# Build the DB once:
#   mkdir -p /tmp/ddb_bench
#   $DUCKDB /tmp/ddb_bench/job.duckdb <<SQL
#     CREATE TABLE aka_name AS SELECT * FROM '~/projects/jobdata/parquet/aka_name.parquet';
#     ... (one CREATE TABLE per parquet file in ~/projects/jobdata/parquet/)
#   SQL

set -e
DUCKDB=${DUCKDB:-/Users/remywang/.duckdb/cli/latest/duckdb}
DBFILE=${DBFILE:-/tmp/ddb_bench/job.duckdb}
QDIR=${QDIR:-$HOME/projects/join-order-benchmark}
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
QNAMES=$SCRIPT_DIR/data/job_qnames.txt
RAW=$SCRIPT_DIR/data/job_duck_canonical.txt
OUT=$SCRIPT_DIR/data/job_duck.txt

> $RAW
for q in $(cat $QNAMES); do
    QFILE=$QDIR/$q.sql
    if [ ! -f $QFILE ]; then
        echo "$q MISSING" | tee -a $RAW
        continue
    fi
    output=$($DUCKDB $DBFILE <<DDB 2>&1
PRAGMA threads=1;
.timer on
.read $QFILE
.read $QFILE
DDB
)
    timings=($(echo "$output" | grep -oE "real [0-9.]+" | awk '{print $2}'))
    cold=${timings[0]:-NA}
    warm=${timings[1]:-NA}
    echo "$q cold=$cold warm=$warm" | tee -a $RAW
done

# Convert to plot_job.py format (cold+warm pairs of "Run Time (s): real X.XXX")
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
' $RAW $QNAMES > $OUT

echo "---"
echo "wrote $OUT ($(wc -l < $OUT) lines, expected 226)"
awk -F'warm=' '/warm=/ {split($2, a, " "); s+=a[1]; n++} END {printf "Warm total: %.2f s in %d queries\n", s, n}' $RAW
