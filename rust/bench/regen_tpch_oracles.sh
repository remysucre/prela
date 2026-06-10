#!/bin/bash
# Regenerate the 14 file-loaded TPCH oracles checked in under oracles/tpch/,
# consumed by both julia/tpch_queries_*.jl and rust/src/tpch/:
# Q19/Q11/Q17/Q13/Q7/Q8/Q9/Q18/Q22/Q16/Q15/Q2/Q20/Q21.
#
# Runs each canonical TPCH SQL against the parquet cache at
# DBDIR (single-threaded, so float sums are deterministic), formats
# numeric fields to %.2f, and writes Q$N.txt to OUTDIR with no trailing
# newline (so the `read(..., String) == got` comparison matches).
#
# Env knobs (defaults shown):
#   DUCKDB=/Users/remywang/.duckdb/cli/latest/duckdb
#   DBDIR=<repo>/cache/tpch
#   QDIR=/Users/remywang/projects/duckdb/extension/tpch/dbgen/queries
#   OUTDIR=<repo>/oracles/tpch
#
# Q1 / Q3-6 / Q10 / Q12 / Q14 use inline oracles in the query files and
# are not handled here. Q9 has two 1-cent Float64 drifts which the loaders
# patch in-place — the raw DuckDB values are written here.

set -e
REPO=$(cd "$(dirname "$0")/../.." && pwd)
DUCKDB=${DUCKDB:-/Users/remywang/.duckdb/cli/latest/duckdb}
DBDIR=${DBDIR:-$REPO/cache/tpch}
QDIR=${QDIR:-/Users/remywang/projects/duckdb/extension/tpch/dbgen/queries}
OUTDIR=${OUTDIR:-$REPO/oracles/tpch}
mkdir -p $OUTDIR

# Format every decimal field as %.2f to match Julia's _fmt(Float64).
fmt='BEGIN { FS="|"; OFS="|" }
     { for (i=1; i<=NF; i++) if ($i ~ /^-?[0-9]+\.[0-9]+$/) $i = sprintf("%.2f", $i); print }'

# Cast varchar date columns to DATE so canonical TPCH SQL (which uses
# CAST(... AS date) and extract(year FROM ...)) works against the parquet.
VIEWS="
CREATE VIEW lineitem AS SELECT * EXCLUDE (l_shipdate, l_commitdate, l_receiptdate),
    CAST(l_shipdate AS DATE) AS l_shipdate,
    CAST(l_commitdate AS DATE) AS l_commitdate,
    CAST(l_receiptdate AS DATE) AS l_receiptdate
    FROM read_parquet('$DBDIR/lineitem.parquet');
CREATE VIEW part     AS SELECT * FROM read_parquet('$DBDIR/part.parquet');
CREATE VIEW partsupp AS SELECT * FROM read_parquet('$DBDIR/partsupp.parquet');
CREATE VIEW supplier AS SELECT * FROM read_parquet('$DBDIR/supplier.parquet');
CREATE VIEW nation   AS SELECT * FROM read_parquet('$DBDIR/nation.parquet');
CREATE VIEW region   AS SELECT * FROM read_parquet('$DBDIR/region.parquet');
CREATE VIEW orders   AS SELECT * EXCLUDE (o_orderdate),
    CAST(o_orderdate AS DATE) AS o_orderdate
    FROM read_parquet('$DBDIR/orders.parquet');
CREATE VIEW customer AS SELECT * FROM read_parquet('$DBDIR/customer.parquet');
"

for q in 19 11 17 13 7 8 9 18 22 16 15 2 20 21; do
    sql=$(printf "%s/q%02d.sql" "$QDIR" "$q")
    if [ ! -f "$sql" ]; then
        echo "MISSING SQL for Q$q: $sql"
        continue
    fi
    $DUCKDB <<EOF | awk "$fmt" | perl -e 'local $/; my $s = <STDIN>; $s =~ s/\n$//; print $s' > $OUTDIR/Q$q.txt
PRAGMA threads=1;
$VIEWS
.mode list
.separator |
.headers off
.read $sql
EOF
    lines=$(wc -l < $OUTDIR/Q$q.txt)
    echo "Q$q  $lines lines  ($(head -1 $OUTDIR/Q$q.txt | head -c 60))"
done
