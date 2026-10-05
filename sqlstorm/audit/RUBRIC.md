# Audit rubric: SQLStorm → prela ports

You are auditing every port in ONE crate (`sqlstorm/rust/cNN`). Read every
`fn qNNNN` (and every local helper it calls) next to the SQL in the comment
above it. Judge by reading, not by grep. Background, read once:
`sqlstorm/README.md` ("The three moving parts"), `notes/agent-prompt.md`
("Writing a port"), `notes/limitations.md` (the window and re-key sections),
`notes/translation-failures.md`. The prela API is `rust/src/engine.rs`
(`QueryExt`), shared helpers in `sqlstorm/rust/harness/src/{views,kit}.rs`.

## The four checks per query

### 1. Sanity
The port computes what the SQL asks, from the data. Fail it (`suspect`) if it:
* hardwires output: literal result values, ids/strings/counts copied from the
  oracle, special-casing particular ids, an `if` that patches a row;
* drops or alters a clause because it happens not to matter on this data
  (a WHERE that "never filters", a join assumed to always match, a LIMIT
  removed because the result is short, NULL handling skipped because the
  column "has no NULLs" when the schema column is a `Set`) — data-specific
  knowledge is hardwiring. Most common case found so far: a nullable column
  (a `Set` in `harness/src/schema.rs`, e.g. Posts.ViewCount) joined with
  `.and(col)`/`.select(col)` where SQL only projects it, silently dropping the
  NULL rows — it needs `.opt()` and `oint`/`ostr`, and `DESC` sorts NULLs last;
* matches the oracle only by luck (e.g. wrong semantics that coincide here).
A suspect port gets fixed (rewritten correctly) like an unidiomatic one.

### 2. Idiomatic
Every relational step goes through prela combinators: `with`/`minus`
(semi/anti join, IS [NOT] NULL), `select` (join/compose), `and` (product on a
shared key), `opt` (LEFT JOIN / nullable projection), `cross`, `union`,
`filt`/`eq`/`gt`/`is_in`/... (WHERE), `group_by` + `fold`/`buf_fold`/
`dense_fold[_outer]` (GROUP BY + aggregates), `count_distinct`, `window`
(ROW_NUMBER/RANK/DENSE_RANK/LAG/LEAD, with `whole(q)` as the single partition
when there is no PARTITION BY), `map`/`flat_map` (computed values),
`select_where`, `inv`, `.collect()` into `HashIdx`/`MatSet` (indexes,
materialised CTEs).

Host Rust is allowed ONLY for:
* the final sort and `LIMIT`/`OFFSET` cut (`top_n`, `sort_by`, `truncate`,
  `take`/`skip` on the final sorted rows) — also an ORDER BY ... LIMIT taken
  early to pick which rows to drive further ("pick first") is fine when it is
  exactly the SQL's ORDER BY/LIMIT;
* scalar UDFs inside closures (arithmetic, CASE, COALESCE, string and date
  functions, casts) and the step function of a fold;
* formatting output (`row`, `rows`, `V::*`, `post_fields`, `ucols`, ...) and
  projecting output columns of the final rows with `.get(key)`.

Classify as UNIDIOMATIC (err on this side when unsure):
* a filter/WHERE/HAVING done on a host `Vec` (`.iter().filter`,
  `into_iter().filter`, `filter_map` that drops rows, `take_while`, `retain`,
  `find`, `any`/`all`/`position`, `if` inside a loop that skips rows);
* a join or lookup done in host: `.get()`/`.member()` probes used to decide
  which rows survive, to join, or inside a `filt` closure to reach another
  table; a `Vec` indexed by id; any HashMap/BTreeMap/HashSet;
* grouping/aggregation in host (loops accumulating, sorting then scanning
  runs, `distinct_some`/sort+dedup to count distinct when `count_distinct`
  or a fold would do);
* window functions computed in host: `kit::ranked`, `top_per`, `per_group`,
  `top_score_users`, `two_ranks`, `lex_top`, hand-rolled rank/row-number/lag
  loops — prela has `.window(row_number|rank|dense_rank|lag|lead, order,
  asc|desc)`. A rank filter (`WHERE rn <= n`) then goes through `.filt` on
  the window's output;
* `left_all`, `cross_top`, or any other host construction of a relation that
  a combinator could express;
* `drain(..)` then host work then `rel(..)` where the host work is anything
  but sort/cut.

### 3. Tricks / hacks
Arithmetic or bookkeeping standing in for relational work: multiplying counts
instead of driving the product, subtracting to emulate an anti-join,
precomputed totals reused in a way the SQL doesn't say, sentinels that only
work on this data, relying on hash/insertion order, etc. The one accepted
identity: COUNT(DISTINCT child.Id) per group = sum of per-parent child counts
when each child belongs to exactly one parent row in the group (state it in
the review when relied on).

## What to do

* Idiomatic, sane, no trick: record it, change nothing.
* Otherwise: rewrite the port in idiomatic prela, in place in the batch file
  (keep the SQL comment; update/remove any `//` note that no longer holds).
  Then run it: `cd sqlstorm/rust && SQLSTORM_ONLY=<ids> cargo run -q --release -p cNN`
  and it must print `ok` for every id the fn serves (several ids can share
  one fn — check ENTRIES). Note the runner's time before and after.
* If an idiomatic version is impossible, leave the existing port, and record
  exactly what prela lacks (specific operator / capability) and why the
  workaround is needed. "It was easier" is not a reason.
* If an idiomatic rewrite is much slower (>2x and >0.2 s), keep the rewrite
  anyway but note both times.

Rules:
* Do NOT edit `harness/` (views.rs, kit.rs, ...) or `rust/src/` — other
  agents run against them concurrently. If a harness helper is itself
  unidiomatic or wrong, record which one; you may write an idiomatic/correct
  local helper in your crate instead. Known wrong: `views::questions` and
  `by_created`/`by_score`/`by_views` join ViewCount as non-null (drops NULL
  rows); a corrected copy exists in `rust/c01/src/q.rs` — copy it into your
  crate if you need it. `views::type_aggs`/`by_count` group by type name only
  (wrong for `GROUP BY pt.Id`).
* Only edit files in your own crate. No git commits. No new comments beyond
  the existing SQL comment style (a short `//` note only where a reader would
  be surprised).
* Every claim of `ok` must come from an actual run. Finish with the whole
  crate passing: `cargo run -q --release -p cNN` → 0 diff.

## Output

Write `sqlstorm/audit/review/cNN.jsonl`, one JSON object per ENTRIES id:

    {"id": "12345", "fn": "q12345", "sanity": "ok" | "suspect",
     "idiomatic": true | false, "trick": true | false,
     "issues": "short: what was unidiomatic/suspect/tricky (empty if none)",
     "action": "none" | "rewritten" | "cannot",
     "cannot_why": "what prela lacks (only for cannot)",
     "t_before": 0.012, "t_after": 0.010,   // runner seconds; t_after null if not rewritten
     "verified": true}                     // ok against oracle after any change

Then reply with a summary: counts per category, the list of `cannot` ids
with reasons, any harness helpers you found unidiomatic, and anything odd.

## Before editing

Run the whole crate once before touching anything and save it:
`cd sqlstorm/rust && cargo run -q --release -p cNN > ../audit/prela_out/cNN.before.txt 2>&1`
— that is your `t_before` source (and the baseline: it must be 0 diff).

## Empty oracles

Many oracles have 0 rows, so `ok` proves little for a rewrite. When you
rewrite a query whose oracle has 0 (or 1) rows, also check it on a relaxed
version: change one constant (date cutoff, threshold, LIMIT) in a copy of
the SQL until DuckDB returns rows, run it with
`/Users/paultalma/projects/sqlstorm_data/venv/bin/python` (format with
`tools/oracle.py`'s `fmt`; db `/Users/paultalma/projects/sqlstorm_data/so_dba.duckdb`,
read_only), make the same change temporarily in the port, compare (sorted
multiset), then restore the original constant and re-run. Record
`"relaxed": "<what you changed>, <n> rows, match"` in the jsonl. If no
single constant gives rows, say so.

## Known harness pitfalls (found by earlier crates)

* `views::tag_mentions` is case-sensitive: right for LIKE, wrong for ILIKE
  (c05 has a local `tag_mentions_ci`).
* Vote/post-type names in SQL (`vt.Name = 'UpMod'`) hardwired as ids is
  suspect; filter on `vtype_name`/`ptype_name`.
* `COUNT(DISTINCT p.OwnerUserId)` must count the raw `owner_user_id`, not the
  `owner_user` edge (which drops dangling ids).
* GROUP BY a name (`t.TagName`, `pt.Name`, `u.DisplayName`) ported as a group
  by the entity id, or vice versa, is suspect: the schema declares no name
  unique. Group by exactly what the SQL groups by.

## Clarifications

* `ROW_NUMBER() OVER (ORDER BY x)` (no PARTITION BY) filtered by `rn <= n`
  with `rn` not projected is exactly `ORDER BY x LIMIT n`; porting it as a
  sort + cut is fine. With PARTITION BY, or when the rank is projected or
  RANK/DENSE_RANK semantics matter, use `.window`.
* Scratch files: use `/tmp/aud_cNN/` (your crate name), never a shared dir.
* `views::group_posts`'s "#b" (COUNT(DISTINCT b.Id)) sums per-owner badge
  counts per post, so it double-counts when posts in a group share an owner:
  any port reading it is suspect.
* Aggregates over zero rows print NULL (SUM/MIN/MAX/AVG), not 0 or a
  sentinel; COUNT prints 0.
* A join SQL does on raw ids (e.g. `p.OwnerUserId = b.UserId`,
  `pl.RelatedPostId = t.ExcerptPostId`) done through an edge, which drops
  dangling ids, is suspect even if nothing dangles in this data: use the raw
  `*_id` columns.
* An uncorrelated scalar subquery in SELECT computed once (`fold_flat`) and
  printed beside each row is acceptable (always exactly one value). A
  CTE/derived table that is JOINed (may have 0 rows) must be joined
  (`.cross`/`.and` over a one-row relation), not stitched in.
* `views::stats_fold` join "b" goes through the owner edge (raw-id issue).
* For DuckDB runs involving CURRENT_DATE/CURRENT_TIMESTAMP, first
  `SET TimeZone='America/New_York'` (what the oracles assumed).
* A port that matches only because its tie-break at a LIMIT cut happens to
  agree with DuckDB's pick (tied rows differ in a projected column): record
  "tie" in issues; don't write the rewrite file.
* `kit::tag_stats` groups by tag id: wrong for `GROUP BY t.TagName`.
* An aggregate with no GROUP BY yields exactly ONE row even over empty input.
  A `whole(..).fold(..)` (or any `()`-keyed fold) crossed into the result
  yields NO row when its input is empty — wrong. Drive a one-row relation
  and join the fold with `.and(fold.opt())` (NULL aggregates, COUNT 0), as
  in c31/c33.
* Picking candidates with a host-computed threshold (e.g. "the tenth-highest
  reputation", `top_n(..).last()`) counts as a trick even when sound: use a
  window rank (`RANK() <= N` on the leading key) instead. Shell note: `cp` is
  aliased to `cp -i`; use `/bin/cp -f`.
* Keep every foreground command under ~5 minutes (a 600 s no-progress
  watchdog kills agents): `timeout 240` on DuckDB relaxed checks; if a relaxed
  query is too slow, record that and move on.
