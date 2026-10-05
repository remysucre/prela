# Second pass: rules added to the rubric after early crates were audited

The crates in your range were audited before some rules existed. Re-check
every port in your range against ONLY these rules (they are also in
RUBRIC.md, end), fix what fails, verify (`ok` against the oracle; relaxed
check for 0/1-row oracles), and append a line per changed id to
`audit/review/sweep.jsonl` (same fields as the review jsonl, plus
`"rule": "<which>"`). Also update that id's line in `audit/review/cNN.jsonl`
(set action/issues/t_after) so the crate review stays the source of truth.

1. **raw-id**: a join / IS NOT NULL / COUNT(DISTINCT) / GROUP BY / PARTITION
   BY that SQL does on a raw id column (OwnerUserId, UserId, PostId,
   AcceptedAnswerId, ParentId, RelatedPostId, ...) done through an edge
   (`owner_user`, `user`, `post`, `accepted_answer`, `parent`, `badges_of`
   via owner, `stats_fold` join "b", `badges_per_user` probed by owner...).
   Edges drop dangling ids, and `edge.opt()` merges them into the NULL group.
   Use the raw `*_id` columns. (Exception: the edge is fine when the row is
   later inner-joined to the target table anyway — say so.)
2. **name-vs-id**: GROUP BY / PARTITION BY / JOIN on a name (TagName,
   pt.Name, DisplayName, lt.Name) done by entity id (incl. `kit::tag_stats`,
   `views::type_aggs`/`by_count` where SQL says pt.Id), or vice versa.
3. **ungrouped-agg**: an aggregate with no GROUP BY must yield exactly one
   row even on empty input; a `()`-keyed fold `.cross`ed in yields none.
   Use a one-row relation `.and(fold.opt())`. Also SUM/MIN/MAX over zero
   rows print NULL.
4. **threshold**: candidates picked with a host-computed threshold
   (`top_n(..).last()`, "tenth-highest", `div_ceil`) → window `RANK() <= N`.
5. **host-window**: any remaining `ranked`, `top_per`, `per_group`,
   `cross_top`, `left_all`, `distinct_some`, `user_distinct_posts`,
   enumerate-as-ROW_NUMBER → `.window` / `count_distinct` / `.opt()`.
6. **data-assumption**: anything justified by "never happens in this data"
   (no commas in Tags, names unique, ids never dangle, ...).

Keep foreground commands under ~5 min (`timeout 240` on DuckDB), use
`/bin/cp -f`, never leave a relaxed constant in the source, edit only your
crates, never edit harness/ or rust/src, never commit. Each crate must end
0 diff (`cargo run -q --release -p cNN`), except known CURRENT_DATE drift.
Summary: per rule, the ids changed.
