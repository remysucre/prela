# Audit of the SQLStorm → prela port (2026-09-30 – 10-05)

## Scope

All 9,123 ported queries in `rust/c00`–`c113` were audited (every one has a DuckDB oracle;
none is Postgres-only). Crates `c114`+ (the Postgres-only queries, still being ported) were not.

Each query was read next to its SQL and checked for (1) sanity — it computes the SQL from the
data and doesn't hardwire or lean on facts true only of this data; (2) idiom — every relational
step is a prela combinator, host Rust only sorts, cuts, formats and evaluates scalar UDFs;
(3) tricks — arithmetic or bookkeeping standing in for relational work; (4) speed against
DuckDB. The rules, with every clarification added along the way, are in `audit/RUBRIC.md`
(first pass) and `audit/SWEEP.md` (second pass over crates audited before a rule existed).
Every rewrite was run against its oracle; a rewrite whose oracle has 0 or 1 rows was also
checked against DuckDB with a constant loosened until rows came back ("relaxed check").

## Results

Per-query records: `audit/review/cNN.jsonl` (one line per id), plus `sweep_*.jsonl`,
`ties.jsonl`, `optimize.jsonl`. The pre-audit sources are in `audit/orig/`.

|                                                       | queries |
| ----------------------------------------------------- | ------- |
| left unchanged (sane, idiomatic, no trick)            | 4,495   |
| rewritten                                             | 4,628   |
| — unidiomatic before                                  | 3,963   |
| — suspect (wrong in general, right only on this data) | 1,246   |
| — trick                                               | 85      |
| cannot be written idiomatically                       | 0       |
| matching the oracle at the end                        | 9,119   |

Final full run: 9,119 ok, 4 DIFF. The four (21857, 22180, 21670, 9509) use `CURRENT_DATE`;
their oracles were built on an earlier day and the cutoff has since moved past the data. Each
was checked against a fresh or date-pinned DuckDB run and matches.

### What was unidiomatic

Almost all of it was window functions done in host Rust — `kit::ranked`, `top_per`,
`per_group`, rank-by-`enumerate`, `cross_top`, `lex_top`, `two_ranks` — now `.window(row_number
| rank | dense_rank | lag | lead)` (with `whole(..)` for no PARTITION BY) and `.filt`. The rest:
COUNT(DISTINCT) by sort+dedup (`distinct_some`) → `count_distinct`; `left_all` and host cross
loops → `.opt()` on a `()`-keyed index / `.cross`; `.get()` probes inside filters → joined
columns; WHEREs on drained Vecs → `.filt`; UNIONs assembled in a Vec → `.union`.

### What was wrong (suspect), by frequency

1. Joins/partitions/COUNT(DISTINCT) on a raw id (`p.OwnerUserId = b.UserId`, `PARTITION BY
OwnerUserId`, `COUNT(DISTINCT v.UserId)`) done through a foreign-key edge, which drops
   ids that point at nothing (`.opt()` merges them into the NULL group).
2. GROUP BY a name (TagName, pt.Name, DisplayName) done by entity id; names aren't declared
   unique.
3. Nullable columns joined as non-null (`.and(view_count)`), silently dropping NULL rows —
   including the shared `views::questions`.
4. Clauses dropped because they filter nothing on this data (WHEREs, LIMITs, LEFT JOINs ported
   as inner, splits on separators that never occur, LIKE wildcards in names that never occur).
5. Aggregates over empty input printing 0/sentinels instead of NULL; ungrouped aggregates
   producing no row instead of one.
6. Smaller: NULL sort order, f32 vs f64 casts, hardwired vote/post-type ids for names,
   wrong tuple positions, CONCAT over NULL.

Several of these were real wrong answers the oracle couldn't see because its result was empty;
the relaxed checks caught them (e.g. 3377, 121, 23578, 3837, 2373).

### Tricks

Mostly candidates picked with a host-computed threshold ("the tenth-highest reputation",
`top_n(..).last()`), now a window `RANK() <= N`; a few totals stitched beside rows instead of
joined, and tie-breaks relying on hash order.

## Ties

20 candidate ties were checked by flipping the id tie-break in DuckDB; 11 are real (the oracle
depends on DuckDB's pick) and now have `rewrites/<id>.sql` with a total order and a rebuilt
oracle: 14653, 8009, 4776, 26105, 12331, 2023, 1946, 7191, 9543, 29615, 20230.
`rewrites/` also holds ~50 older files with no row in `rewrites/README.md`'s table.

## Harness changes

- `views::questions` / `by_views`: ViewCount nullable, NULLs last.
- `views::group_posts`: COUNT(DISTINCT b.Id) per group via `count_distinct` (it summed
  per-owner badge counts per post, double-counting shared owners).
- `views::users_where`: the inner-join filter is a `.filt`, not a host `if`.
- `views::tag_mentions`: exact LIKE for names containing `<`, `>`, `%`, `_`; `kit::like` added.
- `kit::user_distinct_posts`: a fold instead of sort+dedup.
- Removed `top_per`, `per_group`, `two_ranks`, `top_score_users`, `lex_top` (no callers left).
  `ranked`, `left_all`, `distinct_some` remain only because `c114`+ still use them.

Still faulty and avoided rather than fixed: `kit::tag_stats` groups by tag id (right only for
`GROUP BY t.Id`); `views::stats_fold`/`stats_with` join "b" goes through the owner edge (right
only when the post is inner-joined to Users anyway); `views::user_stats_fold` is slow (see below).

## Speed

Every query timed in a fresh process (prela, one thread) and in DuckDB (8 threads, 60 s cap):
total 2,146 s vs 3,864 s; median ratio 1.0; prela faster on 4,527 queries. DuckDB timed out on
10002 (prela 3.7 s).

52 candidates (prela > 5× DuckDB-8t and > 0.5 s, or > 5 s and > 2×) were re-timed against
1-thread DuckDB (`audit/slow.tsv`); 41 were targeted (`audit/review/optimize.jsonl`), each
re-verified. All but one are now faster than 1-thread DuckDB; e.g. 29933 10.2 s → 0.017 s
(DuckDB 0.10), the 44 s Users × Posts × Comments × Votes × Badges family → ~1.35 s (DuckDB
12–17 s). The main cause was `views::user_stats_fold`/`stats_fold`: a HashMap fold of a
300–560-byte struct computing ~30 aggregates (including a body character count) over every
product row, whatever the query reads. Replaced locally by folds of only the needed columns
(`dense_fold` where it helps), plus restricting before the product and pick-first where the
SQL's own order allows. No count multiplication: every product row is still driven.

Still slow: **24609** (c102), 3.0 s vs 1.6 s for 1-thread DuckDB. 2.3 s is a LEAD window over
3.76M rows in one partition (the author name joins Users twice, multiplying rows); LEAD must
see every row before the WHERE/LIMIT, and the cost is gather + single-threaded sort.

The 11 remaining candidates were within ~2× of 1-thread DuckDB and were left.

## Known limitations left

- `views::tag_list` slices bytes `1..len-1`, assuming every Tags string is `<…>`-wrapped
  (true of all rows here; a malformed string would mis-split or panic).
- Queries whose empty oracle no constant can make non-empty (listed per id in the review
  files, e.g. 2499, 7428, 380, 20565) are verified only on the empty case.
- `CURRENT_DATE`/`NOW()` oracles drift with the clock (4 already have).
- Dead-code warnings: unused `user_counts` wrappers in c36/c38/c40 after the speed pass.
