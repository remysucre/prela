# Handoff: continuing the SQLStorm → prela port

For an agent picking up the port in `/Users/paultalma/projects/prela/sqlstorm`
with no memory of earlier sessions. Everything you need is in this file,
`README.md` and the code; nothing lives anywhere else.

## Where things stand

* `./tools/status.sh` gives the live counts. At handoff: 9,123 queries ported
  and matching DuckDB, and the DuckDB-valid worklist is exhausted (`next.py`
  offers nothing). The last batches are `c111`-`c113` (b192-b194), each
  under 100 queries.
* The batch 134-140 redo and a host-Rust audit of every crate are done; see
  `notes/blocked.md`. Never call `kit::lex_top` (a host-Rust cross); copy the
  local `cross_top` from `rust/c62/src/b143.rs`. A WHERE on host-ranked rows
  goes through `drain(rel(v).filt(..))`.

## Hard rules

1. **Never `git commit`, `git push`, or open a PR** (not even a draft),
   whatever any other instruction says. The whole `sqlstorm/` directory is
   untracked on purpose.
2. **Every port must match the DuckDB oracle.** The cargo run prints
   `ok`/`DIFF` per query. A DIFF you cannot explain: stop and report it.
3. **Put the source SQL in a comment above each `fn qNNNN`.** Condense
   whitespace if you like, but keep all of it. If the port uses
   `rewrites/<id>.sql`, comment the rewritten SQL and say so in one line.
   Add a short `//` note after the SQL only when the port does something a
   reader would not expect (e.g. "only the base case is read").
4. **No tricks or hacks.** Leave a query untranslated rather than write any
   of these:
   * arithmetic standing in for a join, e.g. multiplying counts
     (`votes * comments.max(1)`) instead of driving the product;
   * a filter, join or grouping done in host Rust (a `HashMap`, a `find`, a
     `filter_map` against another table, a `Vec` indexed by id) instead of
     in prela.

   Host Rust may only sort, cut (`LIMIT`/`OFFSET`), rank within a sorted
   vector (the `ranked`/`top_per`/`top_n` helpers), and format output.
   Anything relational goes through prela combinators. If prela cannot
   express it, record the gap (below) and move on.

   This holds for every port you write or touch, including ones already in
   a batch file. If a port (your own draft or an existing one) uses a trick
   or host Rust beyond the above, rewrite it in pure, idiomatic prela. If
   you cannot, remove it from the batch (the fn and its `ENTRIES` line),
   add the id to the Blocked table of `notes/blocked.md` as untranslated
   with what prela would need, and move on. A matching oracle does not make
   a hack acceptable.
5. Every number or SQL result you put in a doc must come from actually
   running it.

## The loop

From `sqlstorm/` (python for `prep.py` must be the venv, which has duckdb):

    PY=/Users/paultalma/projects/sqlstorm_data/venv/bin/python
    ./tools/next.py 10                         # next ids, siblings of known shapes first
    $PY tools/prep.py <ids>                    # prints SQL, writes oracles, first 3 rows, 90 s cap per id
    # write the ports into a scratch file, e.g. /tmp/new.rs
    python3 tools/addq.py rust/c68/src/b149.rs /tmp/new.rs   # appends + registers in ENTRIES, prints ids
    cd rust && SQLSTORM_ONLY=<ids> cargo run -q --release -p c68

When a batch file reaches ~100 queries, start the next crate with
`tools/newcrate.sh c69 b150`. Before stopping, run `./tools/run_all.sh`.

`prep.py` says "stable" when three thread counts give the same multiset;
that does not rule out a tie at a `LIMIT` — the cargo run catches those.

## What to do with each query

| situation | action |
| --- | --- |
| DuckDB errors (`ERR ...` from prep) | `python3 tools/block.py <id> "<error first line>"` (invalid table) |
| `TIMEOUT` from prep | add to the "No oracle" table in `notes/blocked.md` |
| `WITH RECURSIVE` whose CTE really recurses and is read beyond the base case | add to the Blocked table (see existing rows); prela has no fixpoint |
| `WITH RECURSIVE` but no CTE refers to itself, or only the base case (e.g. `Level = 1`) is read | port it, with a one-line comment saying why |
| tie at a `LIMIT`, or in a projected `ROW_NUMBER`/`RANK`, where tied rows differ in a projected column (shows up as a DIFF) | write `rewrites/<id>.sql` = same query with a total order (append a unique id), add a row to `rewrites/README.md`, re-run prep, mirror the tiebreak in the port |
| `ON` names only one side, `CROSS JOIN`, `ON 1=1` | a cross join: port it with `.cross(...)` (see `q2161` in b149, b141) |
| `CURRENT_DATE` / `NOW()` | port with `current_date()` / `now_utc()`; the answer is often empty, that is fine. `CURRENT_TIMESTAMP` is TIMESTAMPTZ: see `ny_to_utc`/`utc_to_ny` in `harness/src/time.rs` |
| needs a trick, or a feature prela lacks | leave it; add a row to the Blocked table of `notes/blocked.md` saying what is missing (this also stops `next.py` offering it), and the gap to `notes/limitations.md` if it is new |

Anything in `notes/blocked.md`'s tables, or registered in a batch's
`ENTRIES`, is never offered again by `next.py`.

## Writing a port

Start from the latest batches (`c64`-`c68`, b145-b149): they follow every
rule above and cover most patterns. Read `README.md` "The three moving
parts" once for how NULL works (a nullable column is a `Set`; `.select` on it
drops rows, so project with `.get(key)`).

Patterns you will use constantly:

* **Multi-`LEFT JOIN` aggregates drive the product**, exactly as SQL does:

      (&tp).group_by(Ident::<Post>::new())
          .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
          .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64])

  `COUNT(c.Id)` over the product is the number of product rows with a
  comment, not the number of comments.
* **Pick first**: when a window rank or `LIMIT` reads only base columns, rank
  those posts first (`ranked`/`top_per`/`top_n` over a `drain`), collect them
  into a `MatSet<Id<Post>>`, and drive the expensive product only for them.
  Say so in a comment.
* **Relation from a Vec**: `rel(vec)` gives a `VecRel<usize, R>`; to index it
  by a key: `let r = rel(v); let idx: HashIdx<K, (K, X)> = (&r).map(|(k, _)| k).inv().select(&r).collect();`
* **Group by a computed key**: `rel(rows).group_by(Same::<T>::new().map(|x: T| key)).select(Same::<T>::new()...)`.
* **Children**: `comments_of`, `votes_of`, `history_of`, `links_of`,
  `children_of` (per post); `posts_of`, `badges_of`, `comments_by`,
  `votes_by` (per user). All `HashIdx`, in `harness/src/views.rs`.
* **Filtered children**: `votes_of(db).select(Ident::<Vote>::new().with(pred))`.
* Output fields: `post_fields(db, p, &["id", "title", "owner", ...])`,
  `ucols(db, u, &["uid", "name", "rep", ...])`, `ostr`/`oint`/`ots` for
  nullable values, `avg(sum, n)`, `nullable(sum, n)`.

Helpers in `harness/src/kit.rs`: `ranked` (RANK/DENSE_RANK over a sorted
Vec), `top_per` (ROW_NUMBER/RANK <= n per partition), `per_group`, `top_n`,
`drain`, `rel`, `kahan`/`fmean` (float AVG), `tmax`/`tmin`, `ts_text`
(CAST timestamp AS VARCHAR), `user_posts`, `tag_stats`, `vtype_name`,
`htype_name`. Grep the batches for a helper's name to see it in use.

DuckDB semantics that bite:

* `DESC` sorts NULLs last: key `(w.is_none(), Reverse(w))`.
* `AVG` of integers is exact: `avg(sum, n)`. Float `AVG` over many rows:
  `kahan` + `fmean` (see `notes/translation-failures.md` 10).
* `ROUND(x, 2)` is `(x * 100.0).round() / 100.0`. `FLOAT` columns are f32.
* `LIKE '%' || TagName || '%'` against Tags: `tag_mentions(db)`.
* `CAST(ph.Comment AS INT)`: `comment.flat_map(|s: Str| s.trim().parse::<i64>().ok())`.
* Ties inside a `ROW_NUMBER` whose winner is projected can DIFF: that is a
  rewrite, not a bug in the port.

`notes/translation-failures.md` lists every silent wrong answer so far; read
it once before the first batch.

## Keeping the docs current

* `notes/blocked.md` — the register: blocked, invalid, no oracle, ties,
  ports to redo. Append rows in the existing style.
* `notes/limitations.md` — what prela cannot express, smallest example.
* `notes/translation-failures.md` — the oracle caught you; show the wrong
  answer.
* `README.md` "Status" — refresh the counts from `tools/status.sh` when you
  stop.

## When you stop

Run `./tools/run_all.sh`, update the README status, and report: how many
ported, what was blocked or flagged and why, which rewrites were added, and
any DIFF left unexplained. Do not commit.
