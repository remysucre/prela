# Prela limitations

Things prela can only do awkwardly, and the sharp edges in what it has.

One thing in the corpus has turned out to be strictly inexpressible: the
recursive CTE below, which needs a fixpoint. Everything else is expressible,
sometimes awkwardly. Three times I recorded a gap as impossible and it was
not: substring joins, windows with a PARTITION BY and cross joins all work,
and all three now have operators. Assume the same of anything added here until it has been tried.

# Not expressible

## No fixpoint, so no recursive CTE

    WITH RECURSIVE PostHierarchy AS (
        SELECT p.Id, p.ParentId, 0 AS Depth FROM Posts p WHERE p.PostTypeId = 1
        UNION ALL
        SELECT p.Id, p.ParentId, ph.Depth + 1
        FROM Posts p JOIN PostHierarchy ph ON p.ParentId = ph.PostId)

A prela query is a combinator graph built once and driven once. Nothing in it
says "run this again on what you just produced, until nothing new comes out",
which is what `WITH RECURSIVE` is. `union` joins two relations that both
already exist; it cannot feed its own output back in.

This corpus is shallow enough to hide the gap: the closure bottoms out at
depth 1 (104,437 questions, then their 140,494 answers, and no post has an
answer as its parent), so two hand-written levels would match. That is
exploiting the data, not translating the query, and it would be silently
wrong on a forum that nests replies. 34189 is blocked for this, and
batches 124-140 left 33648, 31915 unported for the same reason.

It is the only query so far that needs an operator prela does not have at all,
as opposed to one it spells awkwardly.

# Expressible, but only awkwardly

## A window is keyed by its partition, not by the row

    movie.group_by(kind).select(year.and(title)).window(rank, |(y, _)| y, desc)
        // kind -> ((year, title), rank)

`window` is `gather` then `flat_map`: each partition's values are collected
into a cell, sorted, and handed to the window function, and the result stays
keyed by the partition. Nothing in it knows what a row _is_ — a query that
needs the row selects it, with `Ident` for an entity's id or `Same` for any
other key:

    let rn = owned(db).group_by(post_type)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    (&rn).filt(|(_, n)| n <= 10).map(|((p, _), _)| p)   // the top ten posts, as a relation of posts

Three consequences for a port, each seen many times over in c04–c08:

- **Only the row and the order columns go into the window.** Anything that can
  drop a row — an inner-join `select`, a `Set` column without `.opt()` —
  would drop it _before_ the rank is taken and change the ranks. It goes
  after the window, through the row the value carries. A query that prints
  the rank and also needs such a join carries it as `.opt()` in the window's
  value and cuts the `None`s afterwards.
- **Windows chain.** A second `window` over the first one's result sees the
  same partitions and the rows in the order they came in (a window function
  annotates rows, it does not reorder them), so two ranks over one set of rows
  are one window after another, each reading the other's value.
- **Joining a rank back by row is a re-key.** When a rank has to meet
  something keyed by the row that cannot ride in the value — a join that
  multiplies rows, or a partition computed from other windows' outputs
  (28962) — the ranked rows are re-keyed with the idiom under "No way to
  re-key a relation" below.

## A LEFT JOIN of two children is `.and`, but the rows have to be collected to rank them

    SELECT p.Id, COUNT(c.Id), COUNT(v.Id)
    FROM Posts p LEFT JOIN Comments c ON ... LEFT JOIN Votes v ON ...
    GROUP BY p.Id

Two LEFT JOINs off one parent multiply: a post with 3 comments and 2 votes is
6 rows, so `COUNT(c.Id)` is 6. `.and` already is that join. `Prod::probe`
probes both sides at the same key and emits every pair, and `.opt()` turns "no
children" into one `None`, so

    comments_of(db).opt().and(votes_of(db).opt())

yields exactly SQL's joined rows for a post, and a fold over them counts
`is_some()` for `COUNT(c.Id)`. `harness::engagement` is that fold, and
`group_posts` and `users_where` are the same thing over a named set of
children — between them they are most of the port. `COUNT(DISTINCT c.Id)` does
_not_ want the product, so it comes from a second fold over one row per post:
each child row belongs to exactly one parent, so the group's distinct count is
the sum of its parents' own counts.

Both sides of the `.and` have to be probeable, and the obvious spelling of
"the comments of a post", `(&db.comment.post).inv()`, is not: `inv` returns an
`InvStream`, which is `Drive` only, and `.opt()` and the right side of `.and`
both want `Probe`. So the child side is collected into a `HashIdx`, which is
probeable — that is all `harness::comments_of` and friends are, one memoised
index per child table. A hash join has to build that index anyway, so the
materialisation is not extra work; the cost is that it lives outside the
query, as a hand-written static, rather than as something the plan asks for.

Without a `GROUP BY`, a window numbers those joined rows, not the posts, so
the rows have to exist as values to be ranked:

    let joined: MatSet<(Id<Post>, Option<Id<Comment>>, Option<Id<Vote>>)> = base
        .select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt()))
        .map(|((p, c), v)| (p, c, v))
        .collect();
    let rn = (&joined)
        .group_by((&post_of).select(post_type))
        .select(Same::new().and((&post_of).select(creation_date)))
        .window(row_number, |(t, cd)| (cd, t), ..);

(22599, 2703, 3804.) It works, but it is a materialisation, every post column
has to be reached back through `post_of = joined.map(|(p, _, _)| p)`, and the
`MatSet` drives in hash order, so the row itself goes into the window's order
to break ties.

## No way to re-key a relation

`map` changes the value of a relation and never its key. So a fold keyed by
a tuple cannot be turned into one keyed by a part of that tuple, which is what
a join on part of a composite key needs:

    GROUP BY u.DisplayName, u.Reputation     -- key (name, rep)
    ... JOIN Users u2 ON u2.DisplayName = g.DisplayName

The port copies the key into the fold's value, collects `(name, rep, ..)`
tuples into a `MatSet`, and inverts a `map` over that (`q28706`, `q25989`):

    let tops: MatSet<(Str, i64, i64)> = tu.and(top.le(10)).map(|(a, _)| a).collect();
    let by_name: HashIdx<Str, _> = tops.map(|(dn, _, _)| dn).inv().collect();

Three steps and a materialisation for what is one `.rekey(|(dn, _)| dn)`.

## A substring join goes through the distinct strings

    JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%'

There is no index for "contains", so some side has to be scanned.
`select_where` is that scan as an operator: it probes a key by testing every
row of the right side. Scanning every post against every tag is
246,673 x 1,232. Instead `harness::tag_mentions` splits each post's tag list
with `flat_map`, collects the 1,233 distinct tags, runs `select_where` on
those (1,233 x 1,232 tests), and joins the posts back through that index,
keeping each (post, tag) pair once. This relies on a tag name never
containing `<` or `>`, so a name can only match inside one of the post's
tags.

The twelve formerly blocked queries and seven later ones on the same join
(9466, 28776, 7485, 8028, 28124, 25446, 28706) match DuckDB, at about 0.15 s
each in a release build — most of it building the pair set, which each query
builds again, since `tag_mentions` is not memoised. Measure this in release:
the runner's dev profile builds the batch crates at `opt-level = 0`, where
the same queries take 0.9-1.9 s. DuckDB needs 6.8 s on this join, so the
split-and-index approach is the one place prela is dramatically ahead. The
general case, with nothing like a tag list to split, is still one
`select_where` scan per probe.

## Collecting a group into a string or a list

`buf_fold` wants a `Copy` result, so `STRING_AGG` and `ARRAY_AGG` leak their
result: `Box::leak(parts.join(", ").into_boxed_str())` for a string, a leaked
slice for a list (`q2354`, `q8000`, `q33753`). Both work. The order inside
the aggregate is the problem: without an `ORDER BY` in the aggregate, DuckDB's
order is arbitrary, and 33753 needed a rewrite that adds one.

## Solved by `.opt()`: missing groups, unaggregated LEFT JOINs, divisors

Three gaps recorded here before batch 39 all came from "a missing value drops
the row", and `.opt()` (a missing value becomes `None`) closes all three.
Batches 00-38 have since been migrated too, so the whole port uses it instead
of the old workarounds:

- **NULL group.** `group_by(tags_str.opt())` puts the untagged questions in
  one `None` group, which is SQL's NULL group; a `PARTITION BY p.Tags` window
  is the same call (29270, 5989). Before, it was one extra `.minus(key)` pass
  per nullable key.
- **LEFT JOIN without GROUP BY.** `select(history_idx.opt())` emits one row
  per match and one `None` row where there is none (12878, 25507, 26640).
  Before, it was a hand-written `any` flag.
- **Different divisors in one fold.** `score.and(view_count.opt())` keeps the
  row, and the fold counts `v.is_some()` for `AVG(ViewCount)` (11570, 5006).
  Before, it was a second pass plus a host-Rust `find` to join the two.

- **Aggregates of differing nullability in one `GROUP BY`.** `SELECT
COUNT(p.Id), AVG(p.ViewCount)` used to need one fold per nullability class
  — folding `view_count` directly drops the posts that have none — and then a
  host-Rust `Vec::find` to stitch the passes back together on the group key.
  `score.and(view_count.opt())` keeps every row in one pass and the fold
  counts `v.is_some()` for the second divisor. `views.rs::compute_type_aggs`
  was eight passes this way and is now one.

What `.opt()` does not fix: an `.opt()` key in DRIVE position cannot produce
keys that have no rows at all, so `dense_fold_outer` is still the outer-join
aggregate over users. Nor does it merge a `count_distinct()` into a `fold`,
which is a different operator — the two remaining host-side merges in the
port (`b26`, `b27`) are both a fold looking up a `count_distinct` result.

## GROUP BY a child's column groups the materialised joined rows

    SELECT p.Id, v.VoteTypeId, COUNT(v.Id)
    FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId
    GROUP BY p.Id, v.VoteTypeId                                -- 10965

`group_by` probes its key at the row being grouped, and here the row is a
post, but the key is not a function of the post: one post has a group per
vote type. So the joined rows have to exist as values before they can be
grouped, the same materialisation as ranking joined rows above:

    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> =
        base.select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let vote_of = (&j).flat_map(|(_, v)| v);
    (&j).group_by((&post_of).and((&vote_of).select(vote_type_id).opt()))
        .select((&vote_of).opt())
        .fold(0, |n, v| n + v.is_some() as i64)

The ids in the tuple make each joined row distinct, so the set loses nothing,
and any other child the query joins hangs off `post_of` inside the fold.
Batches 96-105 use it for every GROUP BY that names a column of Votes,
Comments or PostHistory beside the post (15203, 16138, 19881, 10965, 12928,
14649, 10892, 13730, 12750, 11252, ...). It works; it is also a relation the
plan should have been able to group without building first.

## An uncorrelated scalar subquery is a separate query

    SELECT (SELECT COUNT(*) FROM Posts), (SELECT COUNT(*) FROM Users)   -- 13558

Each subquery is an aggregate with no group, which prela computes with
`fold_flat` into a host value, and the one output row is those values side by
side. The same goes for a constant column beside a grouped query, `SELECT
pt.Name, COUNT(p.Id), (SELECT COUNT(*) FROM Badges) ... GROUP BY pt.Name`
(10247): the grouped rows come from prela and the scalar is printed next to
each. That is how an uncorrelated scalar subquery is evaluated anywhere — once,
then substituted — but the combining happens in host Rust, not in a prela
plan. Flagged rather than hidden: 25 of the batch 99-105 ports are this shape
(13558, 10247, 10771, 14973, 13816, ...). They could now be written in prela:
`whole(..).fold(..)` is a one-row relation keyed by `()`, and `.cross` puts it
beside the grouped rows (see "A cross join is `.cross`" below); b141 does
this for one-row CTEs.

## No aggregate without a group

    SELECT COUNT(DISTINCT OwnerUserId) FROM Posts

`count_distinct` only exists per group, so a whole-table count has to invent
a group: `.group_by(creation_date.map(|_| 0))`. Works; reads like a trick.

## Sorting and top-N are outside the language

Agreed up front, but recorded for completeness: every `ORDER BY` and `LIMIT`
in the port is host Rust after the plan, so no ported query is end to end in
prela.

## A float aggregate is only reproducible to within an ulp

DuckDB's `SUM`/`AVG` over `DOUBLE` is a tree reduction over vectors and
threads, not a serial fold, so its last bit moves with `SET threads`. Usually
this is invisible, because the value is far from a 6-decimal boundary. When it
is not, the query has no single answer and needs a `rewrites/` entry, the same
as a tie at a `LIMIT`. `notes/translation-failures.md` 10 has the numbers.

The practical rule: if the SQL averages something exact, sum exact integers
and divide once — `SUM(micros)::DOUBLE / COUNT(*) / 1e6` is order-independent
where a float fold is not.

That rule has no form for an average of averages. 12687 takes `AVG` over posts
of a per-post `AVG(EXTRACT(EPOCH ..))`; DuckDB's answer moves with `SET
threads`, and the inner means are not integers, so there is no exact sum to
rewrite it to. Left unported, as is 13680 (the same shape over the joined
vote rows of each post). 10017 and 10185 have the same shape but their
answers hold still across thread counts, and a naive `f64` sum matches them.

## Two aggregate shapes over one group means two folds

    SELECT pt.Name, COUNT(p.Id), AVG(p.Score), COUNT(DISTINCT p.OwnerUserId)

`count_distinct` is a `buf_fold` and the rest is a `fold`, and a fold carries
one accumulator, so the group has to be built twice and the two results joined
by probing one from the other:

    let main = base.group_by(k).select(score).fold(..);
    let uniq = base.group_by(k).select(owner_user).count_distinct();
    main.and((&uniq).opt()).drive(..)

It works and reads well enough (13456, 12038, 10616), but the grouping is done
twice, and the `.opt()` is needed because a group whose every row has a NULL
in the distinct column has no entry in the second fold at all.

## No optimiser, so independent fan-outs cost their product

A query is its plan, which cuts both ways. When several LEFT JOINs hang off
the same key and do not constrain each other, the SQL `FROM` is their product,
and prela drives that product, because that is what the query says.

    SELECT u.Id, COUNT(DISTINCT p.Id), COUNT(DISTINCT c.Id),
           SUM(v.BountyAmount), SUM(u.UpVotes)
    FROM Users u LEFT JOIN Posts p    ON u.Id = p.OwnerUserId
                 LEFT JOIN Comments c ON u.Id = c.UserId
                 LEFT JOIN Votes v    ON u.Id = v.UserId
    GROUP BY u.Id                                          -- 10002

That `FROM` is 1,076,543,949 rows on the dba dump. The translation is
`posts_of(db).opt().and(comments_by(db).opt()).and(votes_by(db).opt())` folded
per user, and it drives all of them.

Nothing in the aggregate needs the product. Each `COUNT(DISTINCT)` undoes one
fan-out and is the plain count; each `SUM` is the un-crossed sum times the
sizes of the other fan-outs, so three `dense_fold_outer`s over 267k users and
a little arithmetic at the drive site give the same rows in a fiftieth of a
second.

This port used to do that, everywhere the shape appeared, and no longer does.
`COUNT(c.Id)` written as `nc * max(1,nv) * max(1,nb)` is not a translation of
the query, it is a hand-written plan for it — and a port that replaces a join
with its closed form stops being evidence that the join is expressible at all.
The arithmetic is worth knowing, because it is the rewrite an optimiser would
find and prela has no optimiser to find it, but it belongs in this note rather
than in the port.

| query | FROM rows | DuckDB 1 thr | prela  |
| ----- | --------- | ------------ | ------ |
| 10002 | 1.08 e9   | 85.7 s       | 11.6 s |
| 10046 | 7.6 e8    | 53.5 s       | 9.0 s  |
| 10079 | 3.1 e8    | 19.6 s       | 5.0 s  |

Both engines pay for the product; neither avoids it. DuckDB really does grind
it out, and gets no benefit from eight threads on 10046. prela drives the same
rows about seven times faster, which is the more interesting result and the
one the factored version was hiding: the cost of having no optimiser is real,
but on this shape it is not what makes the query slow.

prela timings are the release build, single threaded, on 246k posts / 267k
users. Unoptimised these three take 189 s, 116 s and 55 s — about fifteen
times slower — which is why `tools/run_all.sh` builds the suite with
`--release`.

## A child that is not joined is an empty relation, not a filtered one

The grouped views take a join set, so the same code serves a query that joins
Comments and one that does not. The tempting spelling of "not joined" is a
filter that always fails:

    comments_of(db).filt(|_| false).opt()          // wrong

It gives the right answer — no comment ever passes, so `.opt()` yields the one
`None` row that not joining the table would — and it is quadratically slow.
`Filter` decides per _value_, so probing it still walks the whole child list
before discarding it, and a table the query never mentions costs its own size
on every row of the product it is not part of. On 10046 that was the
difference between two minutes and no answer in forty.

The fix is to make the relation empty rather than its values unacceptable:
`child_index!` defines a second accessor that returns an empty `HashIdx` of
the same type, so probing is one failed lookup. Same rows, same types, and the
cost of a child the query does not join is zero rather than its cardinality.

## A join between two entities' ids goes through `origid`

The cache renumbers every table `0..n-1`, so `Id<Post>` and `Id<User>` are not
SQL ids and are not comparable to each other. A query that joins them anyway

    FROM UserPostCount up LEFT JOIN PostVoteCount pv ON up.UserId = pv.PostId

is meaningless but legal, and appears three times in this corpus (10036,
10011, 10033). The translation is an index on the raw id, which the cache
keeps beside the dense one:

    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    db.user.select((&db.user.origid).select(&pidx).opt())

Cheap and entirely in prela, but worth knowing the renumbering is not free:
any join that is not along a declared foreign key has to be rebuilt this way.

## A cross join is `.cross`, and is skipped when its output is trivial

`.and(..)` is `Prod`, which probes the right side _at the left side's key_: it
pairs a row with the other side's values for the same entity. SQL also asks
for the product of two relations that share no key, whenever it comma-joins
two independently grouped subqueries:

    WITH PostCounts AS (SELECT pt.Name, COUNT(*) FROM Posts p JOIN PostTypes pt ... GROUP BY pt.Name),
         UserReps   AS (SELECT u.Reputation, COUNT(*) FROM Posts p JOIN Users u ... GROUP BY u.Reputation)
    SELECT * FROM PostCounts pc, UserReps ur                      -- 14979

`a.cross(b)` is that product. It drives `a` and, for each of its rows, all of
`b`, and the result is keyed by the pair `(ka, kb)`:

    let pc = db.post.group_by(type_name).select(..).fold(..);        // Str -> ..
    let ur = db.post.group_by(owner_user.select(reputation)).fold(..); // i64 -> ..
    (&pc).cross((&ur).filt(|m| m > 0))                              // (Str, i64) -> (.., ..)

The output is |a| x |b| rows, so no plan does less work than this nested loop
when the rows are projected. `b` is re-driven once per row of `a`, so it
should be a materialised relation (a `Fold`, a `HashIdx`), not a lazy plan
that would be recomputed each time.

Three spellings in the corpus all reduce to it:

- a comma join, `CROSS JOIN`, or `JOIN .. ON TRUE` (14979, 11485, 13476,
  11785, 14747, 10056, 10239);
- `JOIN .. ON <predicate on one side only>`, which is a filter on that side
  and then a cross (14979, 10393), or on an uncorrelated scalar, which is the
  same after computing the scalar (10441: `ON TU.PostCount = (SELECT MAX ..)`);
- a one-row side, an aggregate with no `GROUP BY`, which is
  `whole(..).fold(..)`, a relation keyed by `()`, crossed like any other
  (11485, 13476, 11785, 14747, 10056).

A `LIMIT`ed CTE on one side is sorted and cut in host Rust like any other
`ORDER BY .. LIMIT`, and the surviving rows go back in as a `VecRel` keyed by
rank so they can be crossed (11485, 11785).

**When the output is trivial the cross is skipped.** If what consumes the
product aggregates one side straight back out, the answer does not need the
pairs. 10384 is `PostStats ps LEFT JOIN UserStats us ON ps.TotalPosts > 0
GROUP BY ps.*` with `COUNT(DISTINCT us.UserId)` and `AVG(us.AverageReputation)`:
every post type gets the aggregates of all of UserStats, so those are computed
once and printed beside each type, instead of driving 6 x 267k pairs and
folding them away. This is the one place the port deliberately writes a plan
rather than translating the join, and it is only done where the result is a
single value per row of the other side.

10046 (`b73`) predates `.cross` and still does its cross join as a nested loop
in host Rust.

# Sharp edges in what exists

`dense_fold_outer` is the outer-join aggregate — it gives every key a value,
so a user with no posts still counts 0:

    owner_user.inv().dense_fold_outer(db.user.id.n, 0, |a,_| a+1)

It assumes `0..n` is exactly the set of ids. True here because the cache
renumbers every table from zero, but with a sparse domain it invents rows for
ids that do not exist. The port depends on this in about ten places.

## `cross` re-drives its right side for every left row

`a.cross(b)` is a nested loop: for each row of `a` it drives `b` from the
start. When `b` is a filter over a large relation, the filter runs again for
every left row:

    (&post_stats).cross((&user_votes).filt(|a| a[0] == most))   // 246k x 267k
    rel(drain((&user_votes).filt(|a| a[0] == most)))              // then cross that

The first form took 158 s on 7217 for a right side of one row; materialising
it first took 0.09 s. Nothing in the plan says "drive this once", so the
query has to be written that way.

## A correlated `LIMIT 1` is an arg-max fold

    LEFT JOIN UserVoteSummary uvs
      ON ps.PostId = (SELECT v.PostId FROM Votes v WHERE v.UserId = uvs.UserId
                      ORDER BY v.CreationDate DESC LIMIT 1)

There is no correlated subquery in prela; the subquery is a per-user fold
that keeps the row with the largest key, which is then joined like any
other derived table. The fold has to say what it does with a tie (the SQL
does not), so the ports record one and warn. 4710, 26548.

## DuckDB binds a derived table's unknown column to its neighbour

    FROM UserVoteSummary U ...
    JOIN (SELECT UserId, AVG(Score) FROM Posts GROUP BY UserId) S ON U.UserId = S.UserId

Posts has no UserId. DuckDB resolves it to `U.UserId` and runs S as a
LATERAL subquery — one row per U row, the average over all posts — so the
join condition is always true. 16303 does the same through `C.PostId`. The
ports translate what DuckDB runs, and say so in the comment above them.

## No range probe on an equality key plus an ordered column

    (SELECT COUNT(*) FROM Posts p
      WHERE p.OwnerUserId = a.UserId AND p.CreationDate < a.CreationDate)   -- 23639

`select_gt` probes a sorted index on one column, but nothing combines an
equality key with a range on a second column (an index on `(owner, date)`
probed with `owner = x AND date < y`). The port runs `select_where` against
every (owner, date, post), a full scan per probe row: 100 rows x 246k posts,
about 1 s. A composite sorted index with a prefix-equality range probe would
make it one binary search per row.
