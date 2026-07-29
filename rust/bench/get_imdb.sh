#!/bin/bash
# get_imdb.sh — download the JOB/IMDB CSV dump and convert it to parquet at
# data/imdb/parquet/, then build the JOB binary cache at data/cache/.
#
# Usage: ./get_imdb.sh
#
# Source: imdb.tgz, the standard JOB dump from Leis et al., "How Good Are
# Query Optimizers, Really?" (VLDB 2015). Schema (schema.sql) comes from
# github.com/gregrahn/join-order-benchmark, the same canonical JOB checkout
# used by job/run_job_all.sh (QDIR) and job/fairness/run_job_prela_schema.sh.
#
# Env knobs (defaults shown):
#   JOB_REPO_DIR=<repo>/data/join-order-benchmark
#   IMDB_TGZ_URL=https://event.cwi.nl/da/job/imdb.tgz

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$SCRIPT_DIR/../.." && pwd)
DEST=$REPO/data/imdb
CSV_DIR=$DEST/csv
PARQUET_DIR=$DEST/parquet
JOB_REPO_DIR=${JOB_REPO_DIR:-$REPO/data/join-order-benchmark}
# homepages.cwi.nl/~boncz/job/imdb.tgz (the original source) 404s as of 2026;
# event.cwi.nl/da/job/imdb.tgz is the CWI-hosted mirror (also mirrored at
# bonsai.cedardb.com/job/imdb.tgz), same file.
IMDB_TGZ_URL=${IMDB_TGZ_URL:-https://event.cwi.nl/da/job/imdb.tgz}

mkdir -p "$CSV_DIR" "$PARQUET_DIR"

# schema (only schema.sql is needed here; the same checkout's canonical
# query SQL is used separately, via QDIR, by the JOB bench scripts)
if [ ! -f "$JOB_REPO_DIR/schema.sql" ]; then
    echo "cloning join-order-benchmark for schema.sql ..."
    git clone --depth 1 https://github.com/gregrahn/join-order-benchmark.git "$JOB_REPO_DIR"
fi

# raw CSVs (~3.6 GB compressed)
if [ -z "$(ls -A "$CSV_DIR" 2>/dev/null)" ]; then
    echo "downloading $IMDB_TGZ_URL ..."
    curl -L "$IMDB_TGZ_URL" -o "$DEST/imdb.tgz"
    tar xzf "$DEST/imdb.tgz" -C "$CSV_DIR"
    rm "$DEST/imdb.tgz"
fi

# load into DuckDB with the canonical schema, export one parquet per table
echo "loading CSVs into DuckDB and exporting parquet ..."
duckdb <<SQL
.read $JOB_REPO_DIR/schema.sql
$(for f in "$CSV_DIR"/*.csv; do
    t=$(basename "$f" .csv)
    echo "COPY $t FROM '$f' (DELIMITER ',', QUOTE '\"', ESCAPE '\\', HEADER false, NULL '');"
    echo "COPY $t TO '$PARQUET_DIR/$t.parquet' (FORMAT PARQUET);"
done)
SQL
echo "parquet written to $PARQUET_DIR"

# build regen, then the JOB binary cache
cd "$REPO/rust"
cargo build --release --features regen --bin regen
./target/release/regen job "$PARQUET_DIR" "$REPO/data/cache"

echo "done: JOB cache at $REPO/data/cache"
