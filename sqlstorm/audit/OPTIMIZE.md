# Speed pass

`audit/slow.tsv` lists the queries where prela (fresh process, single
thread) is slow against DuckDB: prela seconds, DuckDB at 8 threads and at 1
thread, and the ratio to 1-thread DuckDB. Your targets are given in the
prompt.

For each target:

1. Read the port and its SQL. Find where the time goes (time pieces with
   `std::time::Instant` in a scratch copy if needed; run with
   `cd sqlstorm/rust && SQLSTORM_ONLY=<id> cargo run -q --release -p cNN`).
2. Try to make it faster while staying idiomatic per `audit/RUBRIC.md`
   (relational work in prela combinators; host Rust only for sort/cut,
   UDFs, formatting). Legitimate levers: restrict before joining (push a
   WHERE/semi-join below the product), pick-first when the SQL's own ORDER
   BY/LIMIT or a window rank reads only base columns, collect a reused
   sub-relation once into a `HashIdx`/`MatSet`, avoid re-driving a `.cross`
   right side, drop columns/joins that nothing reads, choose the cheaper
   side to drive, use dense folds.
   NOT allowed: arithmetic standing in for a join (multiplying per-child
   counts instead of driving the product), host filters/joins, anything
   that only works on this data.
3. Verify the modified query only: it must print `ok`. If its oracle has 0
   or 1 rows, also run the relaxed check from RUBRIC.md ("Empty oracles").
   Re-time it in a fresh process (3 runs, take the min).
4. Append a line to `audit/review/optimize.jsonl`:
   `{"id", "crate", "before": s, "after": s, "duck1": s, "change": "...",
   "verified": true, "remaining": "why it is still slow, if it is"}` and
   update that id's `t_after` in `audit/review/cNN.jsonl`.

If it cannot be made meaningfully faster idiomatically, say exactly why in
`remaining` (e.g. "SQL's FROM is the Users x Posts x Comments x Votes x
Badges product and every aggregate reads it; prela has no optimiser to
avoid materialising it, DuckDB's hash aggregate is vectorised and
parallel"). Don't commit; edit only the targets' functions (and helpers
only they use); `/bin/cp -f`; keep commands under ~5 minutes.
