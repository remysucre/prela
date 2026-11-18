# TPC-H Parquet Data Cheat Sheet

Row counts, schema, and min/max (or distinct-value count for strings) for
every file in `data/tpch/parquet`, as read directly from the Parquet footers
with `pyarrow` (not from the derived cache in `../cache`).

## region (5 rows)
| column | type | min | max |
|---|---|---|---|
| r_regionkey | int32 | 0 | 4 |
| r_name | string | — | 5 distinct |
| r_comment | string | — | 5 distinct |

## nation (25 rows)
| column | type | min | max |
|---|---|---|---|
| n_nationkey | int32 | 0 | 24 |
| n_name | string | — | 25 distinct |
| n_regionkey | int32 | 0 | 4 |
| n_comment | string | — | 25 distinct |

## supplier (100 rows)
| column | type | min | max |
|---|---|---|---|
| s_suppkey | int64 | 1 | 100 |
| s_name | string | — | 100 distinct |
| s_address | string | — | 100 distinct |
| s_nationkey | int32 | 0 | 24 |
| s_phone | string | — | 100 distinct |
| s_acctbal | double | -966.20 | 9915.24 |
| s_comment | string | — | 100 distinct |

## customer (1,500 rows)
| column | type | min | max |
|---|---|---|---|
| c_custkey | int64 | 1 | 1500 |
| c_name | string | — | 1500 distinct |
| c_address | string | — | 1500 distinct |
| c_nationkey | int32 | 0 | 24 |
| c_phone | string | — | 1500 distinct |
| c_acctbal | double | -994.79 | 9987.71 |
| c_mktsegment | string | — | 5 distinct |
| c_comment | string | — | 1500 distinct |

## part (2,000 rows)
| column | type | min | max |
|---|---|---|---|
| p_partkey | int64 | 1 | 2000 |
| p_name | string | — | 2000 distinct |
| p_mfgr | string | — | 5 distinct |
| p_brand | string | — | 25 distinct |
| p_type | string | — | 150 distinct |
| p_size | int32 | 1 | 50 |
| p_container | string | — | 40 distinct |
| p_retailprice | double | 901.00 | 1900.99 |
| p_comment | string | — | 1962 distinct |

## partsupp (8,000 rows)
| column | type | min | max |
|---|---|---|---|
| ps_id | int64 | 1 | 8000 |
| ps_partkey | int64 | 1 | 2000 |
| ps_suppkey | int64 | 1 | 100 |
| ps_availqty | int64 | 3 | 9998 |
| ps_supplycost | double | 1.05 | 999.99 |
| ps_comment | string | — | 7998 distinct |

## orders (15,000 rows)
| column | type | min | max |
|---|---|---|---|
| o_orderkey | int64 | 1 | 60000 |
| o_custkey | int64 | 1 | 1499 |
| o_orderstatus | string | — | 3 distinct |
| o_totalprice | double | 874.89 | 466001.28 |
| o_orderdate | string (YYYY-MM-DD) | — | 2401 distinct |
| o_orderpriority | string | — | 5 distinct |
| o_clerk | string | — | 1000 distinct |
| o_shippriority | int32 | 0 | 0 (constant) |
| o_comment | string | — | 14984 distinct |

**Note:** `o_orderkey` goes up to 60000 but there are only 15,000 rows — the
orderkey space is gappy (TPC-H only generates every 4th key), which is why
`tpch_schema::Order` uses `SparseKey` instead of `Key`
(`src/tpch_schema.rs`, `id: SparseKey<Self>`).

## lineitem (60,175 rows)
| column | type | min | max |
|---|---|---|---|
| l_id | int64 | 1 | 60175 |
| l_orderkey | int64 | 1 | 60000 |
| l_partkey | int64 | 1 | 2000 |
| l_suppkey | int64 | 1 | 100 |
| l_linenumber | int64 | 1 | 7 |
| l_quantity | double | 1.0 | 50.0 |
| l_extendedprice | double | 904.00 | 94949.50 |
| l_discount | double | 0.0 | 0.1 |
| l_tax | double | 0.0 | 0.08 |
| l_returnflag | string | — | 3 distinct |
| l_linestatus | string | — | 2 distinct |
| l_shipdate | string (YYYY-MM-DD) | — | 2518 distinct |
| l_commitdate | string (YYYY-MM-DD) | — | 2460 distinct |
| l_receiptdate | string (YYYY-MM-DD) | — | 2529 distinct |
| l_shipinstruct | string | — | 4 distinct |
| l_shipmode | string | — | 7 distinct |
| l_comment | string | — | 58516 distinct |

## Key-space notes relevant to `join_tree.rs` / `regen`

- `region`/`nation` keys are 0-based in Parquet already (`key_delta: 0` in
  `src/bin/regen.rs`).
- Everything else (`s_suppkey`, `c_custkey`, `p_partkey`,
  `ps_id`/`ps_partkey`/`ps_suppkey`, `o_orderkey`/`o_custkey`,
  `l_id`/`l_orderkey`/`l_partkey`/`l_suppkey`) is 1-based in Parquet, shifted
  by `-1` on load.
- `o_custkey` maxing at 1499 while `c_custkey` goes to 1500 means at least one
  customer (custkey 1500) has placed no orders — worth knowing if you write a
  join-tree test over `orders`/`customer`.
- Date columns (`o_orderdate`, `l_shipdate`, `l_commitdate`,
  `l_receiptdate`) are stored as `YYYY-MM-DD` strings in Parquet; `regen`
  parses them into packed `YYYYMMDD` integers (`parse_yyyymmdd`,
  `src/bin/regen.rs`) before caching, so a `PredicateExpr::Cmp` against one
  of these columns straight from Parquet needs a string comparison, not a
  numeric one, unless you replicate that parse yourself.

## How this was generated

```python
import pyarrow.parquet as pq
import pyarrow.compute as pc

for name in ["region", "nation", "supplier", "customer", "part",
             "partsupp", "orders", "lineitem"]:
    table = pq.ParquetFile(f"{name}.parquet").read()
    for col in table.schema.names:
        arr = table.column(col)
        # numeric -> pc.min(arr) / pc.max(arr); string -> pc.count_distinct(arr)
```
Run from `data/tpch/parquet/`.
