#!/bin/bash
# setup_tpch.sh — generate TPC-H via DuckDB's dbgen, export to
# data/tpch/parquet/, then build the TPC-H binary cache at data/cache/.
#
# Usage: SF=1 ./setup_tpch.sh
#
# Env knobs (defaults shown):
#   SF=1   — TPC-H scale factor (dbgen; SF=10 needs ~32 GB RAM)

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$SCRIPT_DIR/../.." && pwd)
SF=${SF:-1}
PARQUET_DIR=$REPO/data/tpch/parquet

mkdir -p "$PARQUET_DIR"

echo "generating TPC-H sf=$SF via DuckDB dbgen ..."
duckdb <<EOF
INSTALL tpch; LOAD tpch;
CALL dbgen(sf = $SF);
COPY (SELECT CAST(row_number() OVER () AS BIGINT) AS ps_id,
             CAST(ps_partkey AS BIGINT) AS ps_partkey,
             CAST(ps_suppkey AS BIGINT) AS ps_suppkey,
             CAST(ps_availqty AS BIGINT) AS ps_availqty,
             CAST(ps_supplycost AS DOUBLE) AS ps_supplycost,
             ps_comment
        FROM partsupp)
  TO '$PARQUET_DIR/partsupp.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(row_number() OVER () AS BIGINT) AS l_id,
             CAST(l_orderkey AS BIGINT) AS l_orderkey,
             CAST(l_partkey AS BIGINT) AS l_partkey,
             CAST(l_suppkey AS BIGINT) AS l_suppkey,
             CAST(l_linenumber AS BIGINT) AS l_linenumber,
             CAST(l_quantity AS DOUBLE) AS l_quantity,
             CAST(l_extendedprice AS DOUBLE) AS l_extendedprice,
             CAST(l_discount AS DOUBLE) AS l_discount,
             CAST(l_tax AS DOUBLE) AS l_tax,
             l_returnflag, l_linestatus,
             strftime(l_shipdate, '%Y-%m-%d') AS l_shipdate,
             strftime(l_commitdate, '%Y-%m-%d') AS l_commitdate,
             strftime(l_receiptdate, '%Y-%m-%d') AS l_receiptdate,
             l_shipinstruct, l_shipmode, l_comment
        FROM lineitem)
  TO '$PARQUET_DIR/lineitem.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(r_regionkey AS BIGINT) AS r_regionkey,
             r_name, r_comment FROM region) TO '$PARQUET_DIR/region.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(n_nationkey AS BIGINT) AS n_nationkey, n_name,
             CAST(n_regionkey AS BIGINT) AS n_regionkey, n_comment
        FROM nation) TO '$PARQUET_DIR/nation.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(s_suppkey AS BIGINT) AS s_suppkey, s_name, s_address,
             CAST(s_nationkey AS BIGINT) AS s_nationkey, s_phone,
             CAST(s_acctbal AS DOUBLE) AS s_acctbal, s_comment
        FROM supplier) TO '$PARQUET_DIR/supplier.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(c_custkey AS BIGINT) AS c_custkey, c_name, c_address,
             CAST(c_nationkey AS BIGINT) AS c_nationkey, c_phone,
             CAST(c_acctbal AS DOUBLE) AS c_acctbal, c_mktsegment, c_comment
        FROM customer) TO '$PARQUET_DIR/customer.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(p_partkey AS BIGINT) AS p_partkey, p_name, p_mfgr, p_brand,
             p_type, CAST(p_size AS BIGINT) AS p_size, p_container,
             CAST(p_retailprice AS DOUBLE) AS p_retailprice, p_comment
        FROM part) TO '$PARQUET_DIR/part.parquet' (FORMAT PARQUET);
COPY (SELECT CAST(o_orderkey AS BIGINT) AS o_orderkey,
             CAST(o_custkey AS BIGINT) AS o_custkey, o_orderstatus,
             CAST(o_totalprice AS DOUBLE) AS o_totalprice,
             strftime(o_orderdate, '%Y-%m-%d') AS o_orderdate,
             o_orderpriority, o_clerk,
             CAST(o_shippriority AS BIGINT) AS o_shippriority, o_comment
        FROM orders) TO '$PARQUET_DIR/orders.parquet' (FORMAT PARQUET);
EOF
echo "parquet written to $PARQUET_DIR"

cd "$REPO/rust"
cargo build --release --features regen --bin regen
./target/release/regen tpch "$PARQUET_DIR" "$REPO/data/cache"

echo "done: TPC-H cache at $REPO/data/cache"
