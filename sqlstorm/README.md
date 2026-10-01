# SQLStorm → prela

Porting the [SQLStorm](https://github.com/SQL-Storm/SQLStorm) StackOverflow
query corpus to prela, and checking every port against DuckDB on the same
data. Self-contained: nothing here is referenced by the prela crate, and the
only thing outside this directory is the data, which lives out of the repo at
`/Users/paultalma/projects/sqlstorm_data` (override with `SQLSTORM_DATA`).

    sqlstorm_data/
      corpus/            SQLStorm v1.0 stackoverflow: 18,251 .sql + its csvs
      stackoverflow_dba/ the 1 GB dba dump (13 csv files)
      so_dba.duckdb      that dump loaded, via SQLStorm's own schema+copy
      cache/             the prela binary cache — 107 .bin columns
      oracles/<id>.txt   DuckDB's answer, canonical format
      worklist.txt       every query, easiest first
      index.json         per query: features, difficulty, duplicates
      venv/              python duckdb, for the tools

## Start here

    ./tools/status.sh

prints where the port stands — counts, which crate has room, the next batch,
and whether the tree is dirty — derived from the files, so it cannot go stale
the way a paragraph does.

The 86 queries that only Postgres and Umbra validated have Postgres oracles
instead; `notes/pg-oracle.md` has their tools and what differs.

The loop, from this directory:

    PY=/Users/paultalma/projects/sqlstorm_data/venv/bin/python

    ./tools/next.py 10                 # pick a batch (system python is fine)
    $PY tools/prep.py <ids>            # SQL + DuckDB's answers + stability, 90 s cap each
    # write the ports into a scratch file new.rs, each with its SQL in a comment
    python3 tools/addq.py rust/cNN/src/bNNN.rs new.rs   # append + register in ENTRIES
    cd rust && SQLSTORM_ONLY=<ids> cargo run -q --release -p cNN   # compare
    ./tools/run_all.sh                 # all crates, before stopping

`tools/oracle.py <ids>` builds the oracles without the extras. A query DuckDB
refuses goes in the invalid table with `python3 tools/block.py <id> "<error>"`.

Batch numbering is global (`b00`…). Early crates hold ten batches of ten;
since b141 each batch is ~100 queries in a crate of its own. Start a new one
with `tools/newcrate.sh cNN bNNN` (creates the crate, its `main.rs`, an empty
batch, and adds it to `rust/Cargo.toml`). The current last crate is shown by
`tools/status.sh`.

Before writing a batch, for each query:

* Is it marked `[SPLIT]`? Then the engines disagree, which nearly always means
  a tie at a `LIMIT`. If the tied rows differ in a projected column it needs a
  `rewrites/<id>.sql`. See `notes/blocked.md`.
* Which columns are nullable? Those are `Set`s, and they are the source of
  every silent wrong answer so far — read `notes/translation-failures.md` once
  before the first batch.

When something cannot be expressed, record it in `notes/limitations.md` and
add the id to the table in `notes/blocked.md` (which is what stops `next.py`
offering it again), then move on.

## Writing the mechanical batches

Most of the corpus is the same few queries written out again and again: the
same `FROM`, the same `GROUP BY`, a permuted select list and a different
column alias. Two tools say which is which:

    ./tools/families.py <ids...>   # histogram the FROM/GROUP/ORDER skeletons
    ./tools/qspec.py <ids...>      # one query's FROM/WHERE/GROUP/SELECT/ORDER, aliases folded

The queries themselves are written by hand, but only once per shape. A shape
lives in `rust/harness/src/views.rs` as one view — `posts_with_counts` for the
queries that group posts, `users_with_counts` for the ones that group users —
and each query is a single call naming its join set, its sort and its columns:

    fn q19213(db: &'static So) -> String {
        post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#cx"])
    }

So a family of twenty queries that differ only in which columns they project
is twenty one-line functions over one implementation, not twenty copies of it.
When one of them turns out to differ in something that matters — a WHERE, a
join that hangs off `u.Id` rather than `p.Id`, a second ORDER BY key — it gets
written out in full in its batch file, next to the others.

Both views *join* the children rather than counting them: `.and` is SQL's
product, probing each child at the parent and emitting every combination, and
`.opt()` turns "no children" into the one `None` row a LEFT JOIN keeps. That
is why the join set has to be named — it says which children are in the
product — and it is why `COUNT(c.Id)` and `COUNT(DISTINCT c.Id)` differ: the
first is summed over the joined rows, the second comes from a second fold over
one row per parent.

The product is the whole cost of these queries, and prela has no optimiser to
avoid it. `notes/limitations.md` has the measurements.

A `WHERE` is a `PostWhere` or a `UserWhere` passed to the view, which builds
it into the relation the group and the join are taken over — not a filter on
the result.

CTEs need no special handling. Every one in this corpus is a projection over a
single `GROUP BY`, with an optional `ROW_NUMBER() OVER (ORDER BY x)` and a
`WHERE Rank <= n` on top, which is `ORDER BY x LIMIT n`. Nothing crosses a CTE
boundary, so the port is the grouped query it would be without them.

Each batch still has to match the oracle before it counts, and a `DIFF` is the
signal to go read the query properly: the shape vocabulary only covers what is
already known to be expressible, and it does not check for ties at the `LIMIT`
or for the thread-instability described in `rewrites/README.md`.

## The three moving parts

**Data.** `tools/build_cache.py` turns the DuckDB database into prela's cache
format (`rust/src/format.rs`) directly — no parquet hop and no changes to
`regen`. It absorbs every load-time transformation:

* ids are remapped to a dense `0..n-1` key ordered by the SQL `Id`. The
  original is kept as `origid`, so a query projecting `Id` prints what SQL
  prints.
* a foreign key becomes an edge to the target's dense id. Its raw SQL value
  is kept too, as `<field>_id`: a FK can be NOT NULL and still dangle
  (`Votes.PostId` points outside the dba subset 113k times), and then the
  edge is absent while SQL still has a value to project and count.
* timestamps are epoch microseconds — exactly DuckDB's TIMESTAMP, so
  comparisons against literals are exact (`harness::time`).
* `Posts.Tags` (a `<sql-server><t-sql>` string) is exploded into a real CSR
  edge `Post.tags -> Tag`, which turns the whole LATERAL-unnest-on-tags
  family into a native column.

It also generates `rust/harness/src/schema.rs`, so the schema cannot drift
from the cache.

**NULL is absence.** A column that is NULL anywhere becomes a `Set` (CSR,
zero-or-one entry per key) rather than a dense `Col`. Nothing else is needed:

| SQL | prela |
|---|---|
| `WHERE c IS NOT NULL` | `.with(c)` |
| `WHERE c IS NULL` | `.minus(c)` |
| `JOIN ... ON t.fk = u.Id` | `.select(fk.select(..))` — absent keys drop out |
| `SUM(c)` skipping nulls | the fold only ever sees the present ones |
| `SELECT c` (projection!) | `c.get(key)` — see below |

The one place absence is *not* what you want is a projection: `SELECT p.Title`
has to emit the row with NULL, where `.select(title)` would drop it. That is a
probe from inside the drive — `title.get(pid) -> Option<Str>` (`Probe::get`,
which prela ships), then `ostr(..)` to make it a field.

Whether a column is dense or a set is decided from the DATA, not from the
declared nullability, and `cache/columns.json` records the call and the null
count for each of the 107 columns.

**Comparison.** `tools/oracle.py` runs the original SQL through DuckDB and
writes the rows in a canonical text format; `harness::fmt` produces the same
format from Rust, and the runner compares the two as a sorted multiset. Format:
fields TAB-separated, rows newline-separated, `\N` for NULL, floats at six
decimals, timestamps `YYYY-MM-DD HH:MM:SS.ffffff`, strings with
backslash/tab/newline escaped so a field can never break the framing.

## Running

From `rust/`:

    cargo run -q -p c00                    # one crate
    SQLSTORM_ONLY=15070 cargo run -q -p c00        # one query
    SQLSTORM_ONLY=15070 SQLSTORM_OUT=/tmp/o cargo run -q -p c00   # dump its rows

From here:

    ./tools/run_all.sh                     # every crate
    ./tools/status.sh                      # where the port stands

`cargo run -p cNN` exits nonzero if any query in it differs. `tools/next.py`
and `tools/status.sh` run on any python3; `oracle.py`, `select.py` and
`build_cache.py` need the venv, which has duckdb.

## Batch size: 10 queries per file, 100 per crate

Compile cost is linear in queries per crate, measured on this schema at
about **7 ms each** plus ~0.15 s of fixed per-crate overhead:

| queries in one crate | rebuild |
|---|---|
| 10 | 0.23 s |
| 50 | 0.50 s |
| 100 | 0.88 s |
| 400 | 2.92 s |
| 1600 | 11.75 s |

There is no superlinear blowup, so the crate boundary is not about avoiding
one — it is about balancing two overheads. A crate per batch of ten would
mean ~1,000 crates for the DuckDB-valid corpus, and 1,000 × 0.15 s of cargo
overhead is more than the ~72 s the queries themselves cost; it would also
mean 1,000 binaries each re-mmapping the cache. One crate of 10,000 would
rebuild in ~75 s for a one-line edit.

So: the CRATE is 100 queries (`c00`, `c01`, …) — about a second to rebuild —
and the FILE is a batch of ten (`c00/src/b00.rs` … `b09.rs`), which is the
unit of work and of review. Modules in a crate compile together, so the file
split costs nothing; it just keeps a batch readable and reviewable on its own.

## Order of work

`tools/select.py` collapses exact duplicates (18,251 → 15,914 distinct),
keeps what SQLStorm validated on DuckDB (10,222), and sorts by a weighted
count of the SQL features each query uses — window functions and
LATERAL/UNNEST cost most, GROUP BY and ORDER BY nothing, since those are host
Rust after the plan. It also groups by SHAPE (DuckDB's parse tree with
aliases and projection order normalized away), so the queries in a batch are
spellings of one plan rather than ten unrelated ports.

`tools/next.py` prints the next unported ids in that order — it reads what is
already ported out of the batch crates themselves, and what is blocked out of
`notes/blocked.md`, so it cannot drift:

    ./tools/next.py 10          # the next batch, siblings of a working shape
    ./tools/next.py 10 --new    # the next batch, ten distinct shapes

Default order finishes a shape before starting the next one; `--new` takes
one query per unseen shape. Use `--new` while the point is to find out what
prela is missing, since a batch of siblings exercises one plan ten times and
tells you nothing new. Use the default to sweep up the spellings once a shape
is known to work. Batches 00-05 ran in default order, 06 onward with `--new`.

The 5,692 distinct queries SQLStorm did not mark DuckDB-valid are deferred,
not dropped — they still have to be looked at to finish the corpus.

## Status

`./tools/status.sh` is the live answer. As of batch 194 (c113): 9,123 queries
ported and matching DuckDB, 75 blocked on a missing feature (nearly all
recursive CTEs), 1,042 the corpus ships broken (DuckDB refuses them), 5 with no
oracle, 224 with a rewritten oracle. The DuckDB-valid worklist is exhausted:
`next.py` offers no more candidates. What remains is the 5,692 queries
SQLStorm did not mark DuckDB-valid (see "Order of work").

The ports in batches 134-140 that multiplied per-child counts have been
redone, and every crate has been audited for filters and joins done in host
Rust; see `notes/blocked.md`.

The whole `sqlstorm/` directory is untracked in git (on `main`), by request:
nothing has been committed. Nothing under `rust/` outside `sqlstorm/` has
been touched by the port.

## Notes

* `notes/agent-prompt.md` — the handoff: the rules and the working loop for
  an agent continuing the port. Read it before the first batch.
* `notes/blocked.md` — the register of queries that could not be ported, the
  invalid ones, and the determinism screen.
* `notes/limitations.md` — what prela cannot express, and what it can express
  only awkwardly for want of an operator. Minimal example each.
* `notes/translation-failures.md` — where prela was perfectly able and the
  translation was wrong anyway. Minimal example each.
* `harness/src/views.rs` — the derived tables the corpus keeps asking for
  (the questions-joined-to-users row, the per-post-type aggregate roll-up).
  `type_aggs` is memoised, so the per-query time the runner prints for a
  query that uses it is not that query's cost. The runner measures
  correctness, not performance.
* Determinism: a query with `LIMIT` over a non-total `ORDER BY` has no single
  right answer. `tools/next.py` marks these `[SPLIT]` from SQLStorm's own
  cross-engine data, so they are spotted before porting rather than after a
  DIFF. When the tied rows differ in a projected column, `rewrites/<id>.sql`
  holds the same query ordered totally, and the port mirrors the tiebreak.
  See `notes/blocked.md` and `rewrites/README.md`.
