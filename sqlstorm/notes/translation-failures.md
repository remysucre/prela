# Translation failures

Cases where prela could express the query and I wrote it wrong. Every one
gave a plausible-looking wrong answer, not an error. All were caught by the
oracle, and all are consequences of "missing value = row not there".

Every SQL answer below was run through DuckDB.

## 1. Projecting a missing value drops the whole row

    Posts(Id, Title, Score): (1,'a',5) (2,NULL,3) (3,'c',1)
    SELECT Id, Title FROM Posts ORDER BY Score DESC LIMIT 2

    right           wrong
    1 | a           1 | a
    2 | NULL        3 | c      <- post 2 fell out, so post 3 moved up

    db.post.select(origid.and(title))     // wrong: no title, no row
    ... ostr(get(title, p))               // right: keep row, look title up

The damage is not a misprinted NULL — it is a different row in the output.
Hit in batch 07 (16776) and again in batch 20 (19599).

## 2. A join is also a filter

    Posts(Id, OwnerUserId): (1,1) (2,1) (3,NULL)

    SELECT COUNT(*) FROM Posts                                    -- 3
    SELECT COUNT(*) FROM Posts p JOIN Users u ON p.OwnerUserId=u.Id  -- 2

Count over `db.post` when the SQL joins Users is too high. Count over
`db.post.with(owner_user)` instead. In the corpus: 140,494 vs 136,737.
Hit in batch 15 (12931, 16324).

## 3. A LEFT JOIN multiplies rows

    Users(1,'ann')   Posts(1,ann) (2,ann)   Votes on post 1: three

    SELECT COUNT(p.Id) FROM Users u
      LEFT JOIN Posts p ON u.Id=p.OwnerUserId
      LEFT JOIN Votes v ON p.Id=v.PostId     -- 4, not 2

ann has 2 posts. Post 1 is counted once per vote (3), post 2 once (1).
Anything counted after a join counts joined rows, not table rows.
Hit in batch 29 (16665: 2,539 vs 18,148).

## 4. Folding a missing value loses whole groups

    Votes(Id, PostId, Bounty): (1,1,NULL) (2,1,50) (3,1,NULL) (4,3,NULL)

    SELECT PostId, SUM(Bounty) FROM Votes GROUP BY PostId
    1 | 50
    3 | NULL          <- prela produces no group here at all

Post 3 has a vote but no bounty, so grouping on post while reading Bounty
never sees it. Build the groups from a column that is always there, then
attach the aggregate. Hit in batch 29 (16665).

## 5. Averaging through floats loses the last digit

    four durations in microseconds:
    413745141256484, 322009732330750, 194197737026088, 191004529612240

    DuckDB                        280239285.056391
    add as float seconds          280239285.056390   <- one off
    add as whole microseconds     280239285.056391

Each conversion to seconds rounds before the addition. Add the exact
integers and divide once at the end. Needs `i128` at corpus scale.
Hit in batch 25 (10119).

## 6. Dividing by a group that has no rows is NULL, not zero

    Posts(Id, PostTypeId): 24 ModeratorNomination posts, none of them voted on

    SELECT COUNT(p.Id) AS TotalPosts, SUM(vs.VoteCount) AS TotalVotes,
           CASE WHEN TotalPosts > 0 THEN TotalVotes / TotalPosts ELSE 0 END
    ModeratorNomination | 24 | NULL | NULL

    right                          wrong
    ... | \N | \N                 ... | \N | 0.000000

The `CASE` guards the DIVISOR, so it looks like the zero branch covers the
empty group — but the empty group makes the NUMERATOR null, and `NULL / 24`
is NULL. `nullable(sum, n)` was already used for the SUM itself and then the
division computed from the raw `0` accumulator anyway. Anything derived from
an aggregate has to be guarded by the same emptiness test as the aggregate.
Hit in batch 36 (14985).

## 7. A NULL partition key is its own partition

    Posts(Id, OwnerUserId): (1,1) (2,1) (3,NULL) (4,NULL)

    ROW_NUMBER() OVER (PARTITION BY OwnerUserId ORDER BY Id)
    post 3 -> 1, post 4 -> 2        <- the ownerless posts rank among themselves

Grouping rows by the partition key with `HashIdx` keyed on `Id<User>` drops
every ownerless row, so those rank values never exist and the rows vanish from
the result. 6,651 posts have no owner. Same cause as the missing-group gap in
`limitations.md`: fold the leftovers as one extra partition.

## 8. EXTRACT(EPOCH ...) is already a float, so sum it as one

    AVG(EXTRACT(EPOCH FROM (NextEditDate - EditDate)))   -- per post

    DuckDB                               24133620.633312
    add whole microseconds, divide once  24133620.633313   <- one off
    add float seconds per row            24133620.633312

The opposite of 5. There the SQL averaged exact values, so the port had to add
exact integers. Here `EXTRACT(EPOCH ...)` turns each interval into a double
before `AVG` sees it, so DuckDB is adding rounded floats, and the port has to
round the same way to match. Hit in batch 50 (12800).

## 9. ORDER BY ... LIMIT is not a rank filter

    SELECT ..., RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS r
    FROM ... ORDER BY Score DESC, ViewCount DESC LIMIT 100

    right   100 rows
    wrong   105 rows    <- ported as .with(rank.le(100))

`rank <= 100` keeps every row tied at rank 90 and later; `LIMIT 100` cuts in
the middle of that tie. The port has to rank for the projected column and
count rows separately for the cut. The tie at the cut also made the query
ambiguous, so it needed a rewrite as well. Hit in batch 45 (14451).

## 10. A float aggregate has no single answer in its last digit

    AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)))  -- per post type

    DuckDB                       30895258.450773
    naive f64 fold, id order     30895258.450772   <- one off

The per-row value is right (`us as f64 / 1e6` is exactly what DuckDB
computes) and so is the group; the drift is in the 104,437 additions. Naive
summation gives 30895258.450772475 in id order and 30895258.450772744 in join
order, and neither reaches DuckDB.

DuckDB is not doing a serial fold at all. Its float `SUM`/`AVG` is a tree
reduction over vectors and threads, so the answer moves with the thread count:

    SET threads=1  ->  30895258.450773243
    SET threads=2  ->  30895258.450773135
    SET threads=4  ->  30895258.450773053      (also 8, and the default)
    exact (fsum)   ->  30895258.450773038

All of them are nearer the exact sum than a naive fold is, so compensated
summation reproduces DuckDB far more often than a plain `+=`:

    fn kahan((s, err): (f64, f64), x: f64) -> (f64, f64) {
        let y = x - err;
        let t = s + y;
        (t, (t - s) - y)
    }

`(f64, f64)` is `Copy`, so it drops straight into `fold`. That is what 11087
uses, and it matches — the value is far from a 6-decimal boundary, so every
thread count prints the same `30895258.450773`.

**But it is luck, not a rule.** 5603 averages the same way over 217 groups,
and there Kahan is wrong on one of them while a naive fold is wrong on a
different one — because DuckDB's answer for that group lands one ulp off a
6-decimal midpoint, and the printed digit flips:

    SUM over 16 rows, group ('Lennart - Slava Ukraini', NULL, 2)
    threads=1,2   14550018.8559375     -> prints 14550018.855937
    threads=4,8   14550018.855937501   -> prints 14550018.855938
    exact (fsum)  14550018.8559375     -> prints 14550018.855937

The oracle itself changes with `SET threads`, so the query has no single
answer to port against. That is the same situation as a tie at a `LIMIT`, and
it gets the same treatment: `rewrites/5603.sql` replaces the float `AVG` with
an exact-integer mean, `SUM(micros)::DOUBLE / COUNT(*) / 1e6`, which has one
rounding instead of an accumulation and is identical at every thread count.

The rule to take from this: sum exact integers and divide once whenever the
SQL lets you. Reach for Kahan only when the SQL really does average a float
per row (8 above), and expect it to be right only to within an ulp. Hit in
batch 64 (11087) and batch 69 (5603).

## 11. COUNT of a table alias counts the unmatched rows too

    SELECT COUNT(c), COUNT(c.y), COUNT(*)
    FROM (SELECT 1 AS x) a LEFT JOIN (SELECT 1 AS y WHERE false) c ON true
    1 | 0 | 1

`COUNT(C)` names the whole row of `C`, and DuckDB gives an unmatched LEFT JOIN
a struct of NULLs rather than a NULL struct, so it is never NULL and the count
is `COUNT(*)`. Ported first as `COUNT(C.Id)`, which is `c.is_some()` summed,
and off by one on every post with no comment. Hit in batch 98 (10316). Hit again in
batch 115 (11091): the Community user's group came out 0 against 210.

## 12. COALESCE inside AVG counts the row a LEFT JOIN kept

    SELECT AVG(COALESCE(p.s, 0)), COUNT(p.s)
    FROM (SELECT 1 AS t) pt LEFT JOIN (SELECT 2 AS t, 5 AS s) p ON p.t = pt.t
    0.0 | 0

A post type with no posts still has its one NULL row, and `COALESCE` turns it
into a 0 that `AVG` then counts: the average is 0.0, not NULL. Folding only the
matched rows gave NULL for every empty type. The divisor is the joined rows,
not the posts. Hit in batch 104 (12134).

## 13. A multi-valued GROUP BY key does not filter what the group sees

    SELECT p.Id, ph.PostHistoryTypeId, COUNT(ph.Id)
    FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId
    GROUP BY p.Id, ph.PostHistoryTypeId

Grouping posts by `Ident.and(history_of.select(type_id))` emits one key per
history row, but the `select(history_of.opt())` under it is probed at the post,
so every key counted all the post's history: 6 where DuckDB has 1. The group
has to be built over the history rows themselves, keyed by `(post, type)`, and
the posts join to those groups. Hit in batch 119 (12370).

## 14. A per-user fold over joined rows counts rows, not posts

    SELECT u.Id, COUNT(DISTINCT p.Id), SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END)
    FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
                 LEFT JOIN Votes v ON p.Id = v.PostId
    GROUP BY u.Id

The harness fold that walks each user's posts × votes (`user_stats_fold`)
has to count every joined row, because `SUM(CASE ...)` over the product does.
Its row count is therefore `COUNT(p.Id)`, not `COUNT(DISTINCT p.Id)`: a user
with one post and five votes gets 5. The distinct count is a separate fold
(`user_distinct`). Eleven ports in batch 134 printed the row count as
`PostCount` (13744, 14055, 10849, 12610, ...) before the oracle caught it.

## 15. An AVG over a LEFT JOIN is weighted by the joined rows

    SELECT p.OwnerUserId, COUNT(c.Id), AVG(p.ViewCount)
    FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.OwnerUserId

`AVG(p.ViewCount)` here averages over post × comment rows, so a post with
four comments counts four times. Folding one row per post gave the unweighted
mean (23262.67 where DuckDB has 28854.88). Hit in batch 136 (7609); 28584 had
the same shape in a `COUNT(c.Id)` beside a `PostLinks` join.

## 16. COALESCE with CURRENT_TIMESTAMP moves the answer by the DST hour

    AVG(EXTRACT(EPOCH FROM (COALESCE(p.LastActivityDate, CURRENT_TIMESTAMP) - p.CreationDate)))

    DuckDB                          7059110.186238     (Answer)
    wall-clock difference           7059101.499746
    both ends as UTC instants       7059110.211862

`LastActivityDate` is never NULL, so the COALESCE looks like a no-op. It is
not: `CURRENT_TIMESTAMP` is a TIMESTAMPTZ, so both arguments become
TIMESTAMPTZ in the session zone (America/New_York on this machine), and the
subtraction is done by the ICU extension. ICU does not subtract instants
either: it counts whole days on the local calendar first and only then the
elapsed time of the remainder, so the DST hour shows up only when it falls in
that last partial day (197 of the 246,673 posts). `harness::time::tz_sub`
reproduces it, and was checked against DuckDB on every post before any port
used it. Hit in batch 142 (12537, 11688); 210 corpus files mention
`CURRENT_TIMESTAMP`.

## 17. DATE_PART('day', ...) of a negative interval truncates toward zero

    DATE_PART('day', TIMESTAMP '2024-10-01 12:34:56' - LastActivityDate)
    LastActivityDate = 2024-10-01 14:00:00   (after the reference instant)

    right   0
    wrong  -1        <- floor division

The interval is -1:25:04; its day field is 0, not -1. Ported first with
`div_euclid(DAY_US)`, which floors. Plain `/` truncates like DuckDB. Hit in
batch 144 (20141).

## 18. Kahan is wrong where the group is small

    AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)) / 60)   -- per user

    DuckDB        202309.058182
    Kahan         202309.058183
    naive fold    202309.058182

The mirror image of 10. A user's handful of posts is summed by one thread in
order, so DuckDB's answer is the naive left-to-right sum; compensation moves
it. Kahan earned its keep on the post-type groups of 10 (a hundred thousand
rows each, split across threads); per-user and per-post averages want the
plain fold. Hit in batch 143 (14171) and 144 (9812).

## 19. CURRENT_DATE is the date in the session zone, not in UTC

    SELECT CURRENT_DATE    -- run at 21:05 EDT on 2026-09-28
    DuckDB     2026-09-28
    harness    2026-09-29     <- current_date() truncated the UTC instant

DuckDB's session zone here is America/New_York, so between 20:00 and midnight
Eastern the UTC date is a day ahead. 21857 gave 2 rows against DuckDB's 3.
`harness::views::current_date` now truncates `utc_to_ny(now)`, so every port
that uses it is right at any hour. Hit in batch 181 (21857).

## 20. A COALESCE sentinel can be a real id

    COALESCE(ans.OwnerUserId, -1) AS AcceptedAnswerUserId
    ... LEFT JOIN UserReputationTotales u ON q.AcceptedAnswerUserId = u.UserId

    right   16      <- -1 is the Community user, whose total is 16
    wrong   \N      <- ported as "no accepted answer, so no match"

The sentinel is an ordinary value to the join, and on this data it names a
row. It has to go through the raw-id join like any other id. Hit in batch
174 (31862).

## 21. EXISTS is a semi-join, not a join

    CASE WHEN EXISTS (SELECT 1 FROM TagsWithHighCount t WHERE position(t.TagName IN rp.Title) > 0)

    right   10 rows
    wrong   27 rows     <- title.select(&trending).opt()

Joining the title to the matching tags emits one row per tag it contains.
`EXISTS` only asks whether there is one: `Ident::<Post>::new().with(title.select(&trending)).opt()`.
Hit in batch 172 (22537).

## 22. EXTRACT(EPOCH ...) of a timestamp difference adds whole days first

    EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - TIMESTAMP '2024-09-28 15:17:03.083')) / 3600

    DuckDB                         69.298033     (epoch 249472.91700000002)
    micros as f64 / 1e6 / 3600     69.298032     (epoch 249472.917)

The difference is an INTERVAL of 2 days and 76672.917 s, and the epoch is
`days * 86400 + micros / 1e6` in doubles, which is one ulp above the exact
value; divided by 3600 it lands on the other side of a 6-decimal midpoint.
The port computes `(us / DAY_US) as f64 * 86400.0 + (us % DAY_US) as f64 / 1e6`.
Hit in b192 (23002).
