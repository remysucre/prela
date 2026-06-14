#!/bin/bash
# JOB against multi-threaded DuckDB (PRAGMA threads=$THREADS, default all cores).
# Emits data/job_duck_mt.txt in plot format (cold+warm "Run Time" pairs per q).
set -e
DUCKDB=${DUCKDB:-/Users/remywang/.duckdb/cli/latest/duckdb}
DBFILE=${DBFILE:-/tmp/ddb_bench/job.duckdb}
QDIR=${QDIR:-$HOME/projects/join-order-benchmark}
THREADS=${THREADS:-0}   # 0 = DuckDB default (all cores)
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
QNAMES=$SCRIPT_DIR/data/job_qnames.txt
OUT=$SCRIPT_DIR/data/job_duck_mt.txt
> $OUT
PRAGMA_THREADS=""
[ "$THREADS" != "0" ] && PRAGMA_THREADS="PRAGMA threads=$THREADS;"
for q in $(cat $QNAMES); do
    QFILE=$QDIR/$q.sql
    [ -f $QFILE ] || { echo "Run Time (s): real NA user 0.0 sys 0.0" >> $OUT; echo "Run Time (s): real NA user 0.0 sys 0.0" >> $OUT; continue; }
    out=$($DUCKDB $DBFILE 2>&1 <<DDB
$PRAGMA_THREADS
.mode trash
.timer on
.read $QFILE
.read $QFILE
DDB
)
    echo "$out" | grep -oE "Run Time \(s\): real [0-9.]+" | head -2 | sed 's/$/ user 0.0 sys 0.0/' >> $OUT
done
echo "wrote $OUT ($(grep -c 'Run Time' $OUT) lines)"
