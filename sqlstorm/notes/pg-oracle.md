# Postgres-oracle queries

86 representatives SQLStorm validated on Postgres and Umbra (they agreed) but not
on DuckDB. They are listed in `$DATA/pg_worklist.txt` (`tools/pg_oracle.py --list`)
and ported in crates c114-c116 (b195-b197). Their oracles are Postgres 18's answers.

    ./tools/pg_load.sh                         # build the so_dba Postgres database (~1 min)
    $PY tools/pg_oracle.py <ids>               # SQL + oracle + serial/parallel stability, like prep.py
    psql -d so_dba                             # poke at it

The database has C collation and America/New_York time, as the DuckDB oracles
do, plus indexes on the foreign-key columns (speed only). Badge 11 is deleted:
DuckDB read that first line of Badges.csv as a header, so the cache and every
DuckDB oracle lack it, and the Postgres copy has to match the cache.

Run on DuckDB here (60 s cap), 48 of the 86 answer: 45 identically to Postgres,
3 differently (21995: an unordered STRING_AGG; 7555 and 26403: ties). 33 time
out, two run out of memory, and 29996 does not parse. 31911 times out on
Postgres too (600 s), so it has no oracle.

Postgres behaviour the ports had to follow (beyond integer `/` truncating,
`DESC` sorting NULLs first, and AVG/SUM/EXTRACT returning numeric, which prints
as a 6-decimal float):

- `ARRAY_AGG(DISTINCT t)` over a `LEFT JOIN UNNEST` of a NULL `Tags` is `{NULL}`,
  so its `ARRAY_LENGTH` is 1, not NULL (28029, 29126, 9600).
- An unqualified column missing from a subquery's table resolves to the outer
  query, as in DuckDB: in 451 `Posts` has no `PostId`, so it means `ps.PostId`.
