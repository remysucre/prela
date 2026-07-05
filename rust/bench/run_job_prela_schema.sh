#!/bin/bash
# JOB schema-fairness check: does Prela's one schema-level denormalization
# (Company = movie_companies ⋈ company_name, inlined at regen) explain any
# of its JOB advantage? Build TWO single-threaded DuckDB databases from the
# same parquet — canonical schema, and "prela schema" with the merged
# movie_companies — rewrite the affected queries, verify all 113 results
# are byte-identical across the two, then time both suites with the same
# protocol as run_job_duck.sh (per-query session, threads=1, cold+warm).
#
# Writes data/job_prela_schema.txt (per-query timings for both schemas).
# Captured result on the baseline machine (2026-07-04, duckdb 1.5.3):
# canonical 15.44 s warm total, prela schema 15.97 s — the denormalized
# schema is a net LOSS for DuckDB (see benchmarking.md).
#
# Env knobs (defaults shown):
#   DUCKDB=duckdb
#   PQDIR=~/projects/jobdata/parquet
#   QDIR=~/projects/join-order-benchmark
#   WORK=/tmp/ddb_bench/prela_schema

set -e
DUCKDB=${DUCKDB:-duckdb}
PQDIR=${PQDIR:-$HOME/projects/jobdata/parquet}
QDIR=${QDIR:-$HOME/projects/join-order-benchmark}
WORK=${WORK:-/tmp/ddb_bench/prela_schema}
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
QNAMES=$SCRIPT_DIR/data/job_qnames.txt
OUT=$SCRIPT_DIR/data/job_prela_schema.txt
mkdir -p $WORK

TABLES="aka_name aka_title cast_info char_name comp_cast_type company_type
        complete_cast info_type keyword kind_type link_type movie_info_idx
        movie_info movie_keyword movie_link name person_info role_type title"

# ---- build the two DBs (skipped if present) ----
if [ ! -f $WORK/job_canonical.duckdb ]; then
    { echo "PRAGMA threads=4;"
      for t in $TABLES company_name movie_companies; do
          echo "CREATE TABLE $t AS SELECT * FROM '$PQDIR/$t.parquet';"
      done
    } | $DUCKDB $WORK/job_canonical.duckdb
    echo "built job_canonical.duckdb"
fi
if [ ! -f $WORK/job_prela.duckdb ]; then
    # identical, except movie_companies carries company name/country inline
    # (LEFT JOIN: a NULL country stays NULL, matching regen's skipped pushes)
    # and company_name is gone — exactly the entity regen builds.
    { echo "PRAGMA threads=4;"
      for t in $TABLES; do
          echo "CREATE TABLE $t AS SELECT * FROM '$PQDIR/$t.parquet';"
      done
      cat <<SQL
CREATE TABLE movie_companies AS
SELECT mc.id, mc.movie_id, mc.company_type_id, mc.note,
       cn.name AS company_name, cn.country_code AS company_country
FROM '$PQDIR/movie_companies.parquet' mc
LEFT JOIN '$PQDIR/company_name.parquet' cn ON mc.company_id = cn.id;
SQL
    } | $DUCKDB $WORK/job_prela.duckdb
    echo "built job_prela.duckdb"
fi

# ---- rewrite queries + verify result parity ----
# (emits BOTH sets: queries_canon is canonical modulo the semantics-neutral
# `at` → `att` alias rename — AT is reserved in duckdb 1.5.3)
python3 $SCRIPT_DIR/rewrite_job_prela_schema.py $QDIR $WORK/queries_canon $WORK/queries_prela

fail=0
for q in $(cat $QNAMES); do
    $DUCKDB $WORK/job_canonical.duckdb -list -noheader < $WORK/queries_canon/$q.sql > $WORK/canon_out.txt 2>&1 \
        || { echo "QUERY FAILED on canonical: $q"; exit 1; }
    $DUCKDB $WORK/job_prela.duckdb -list -noheader < $WORK/queries_prela/$q.sql > $WORK/prela_out.txt 2>&1 \
        || { echo "QUERY FAILED on prela schema: $q"; exit 1; }
    diff -q $WORK/canon_out.txt $WORK/prela_out.txt > /dev/null || { echo "RESULT DIFF: $q"; fail=1; }
done
if [ $fail -ne 0 ]; then echo "parity check FAILED — not timing"; exit 1; fi
echo "all 113 results match"

# ---- time both suites (threads=1, per-query session, cold+warm) ----
time_suite() { # $1=db $2=qdir $3=label
    for q in $(cat $QNAMES); do
        output=$($DUCKDB $1 <<DDB 2>&1
PRAGMA threads=1;
.timer on
.read $2/$q.sql
.read $2/$q.sql
DDB
)
        timings=($(echo "$output" | grep -oE "real [0-9.]+" | awk '{print $2}'))
        echo "$q $3_cold=${timings[0]:-NA} $3_warm=${timings[1]:-NA}"
    done
}

> $OUT
time_suite $WORK/job_canonical.duckdb $WORK/queries_canon canon > $WORK/canon_times.txt
time_suite $WORK/job_prela.duckdb     $WORK/queries_prela prela > $WORK/prela_times.txt
paste -d' ' $WORK/canon_times.txt <(cut -d' ' -f2- $WORK/prela_times.txt) > $OUT

awk '{for (i=2; i<=NF; i++) { split($i, kv, "="); tot[kv[1]] += kv[2] }}
     END { printf "canonical warm total: %.2f s   prela-schema warm total: %.2f s\n",
           tot["canon_warm"], tot["prela_warm"] }' $OUT
echo "wrote $OUT"
