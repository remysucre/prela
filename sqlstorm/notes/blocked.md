# Query register

Queries that could not be ported, queries the corpus ships broken, and the
ones with more than one right answer. Batches 00-140 are done and every query
in them matches DuckDB byte for byte; run `./tools/status.sh` for the counts.
34189 below is the only query that was attempted and abandoned.

What prela lacks, and where I mistranslated, live in their own files:

- `limitations.md` — what prela cannot express, or can only express awkwardly.
- `translation-failures.md` — where prela was able and I wrote it wrong.

## Blocked

| query | feature |
| ----- | ------- |
| 34189 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ...)`. prela has no fixpoint: a combinator graph is built once and run once, so there is no way to say "repeat until nothing new". See `limitations.md`. |
| 32009 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ...)`, the same post hierarchy as 34189. On this data it bottoms out at depth 2, but unrolling twice would be exploiting the data, not translating the query. |
| 33648 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.PostId)`, the same post hierarchy as 34189 and 32009: prela has no fixpoint. |
| 31915 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ph ON a.ParentId = ph.PostId WHERE a.PostTypeId = 2)`: the same post hierarchy, no fixpoint in prela. |
| 33932 | `WITH RECURSIVE RecursivePosts AS (questions UNION ALL answers JOIN RecursivePosts rp ON p2.ParentId = rp.PostId)`, read for each user's related-post count: the same post hierarchy, no fixpoint in prela. |
| 30183 | `WITH RECURSIVE UserReputation AS (users UNION ALL users JOIN UserReputation ON U.Id = UR.UserId WHERE Level < 5)`, read at `Level >= 1`: a self-join fixpoint, no fixpoint in prela. |
| 2954 | `LEFT JOIN Users cu ON CAST(cp.CloseReason AS JSON) ->> 'UserId' = CAST(cu.Id AS TEXT)`: the harness has no JSON parser. Liftable: a `flat_map` over `Comment` with a small JSON-object key lookup would express it (DuckDB errors on a comment that is not JSON, so check which comments actually reach the cast). Also a cross join of the two newest posts with the top 10 users of a user x votes x comments x badges product (~218M rows for one user). |
| 32489 | `WITH RECURSIVE PopularPosts AS (... UNION ALL ... JOIN PopularPosts pp ON p.ParentId = pp.Id ...)`: a post hierarchy, no fixpoint in prela. |
| 30244 | `WITH RECURSIVE UserReputation AS (... UNION ALL ... WHERE u.Reputation > 500 * ur.Level)`: each user recurses once per 500 reputation, so the row count is the fixpoint's; no fixpoint in prela. |
| 33470 | `WITH RECURSIVE PostHierarchy`, used by ClosedPosts: no fixpoint in prela (and the ORDER BY is thread-unstable besides). |
| 32939 | `WITH RECURSIVE PostHierarchy` joined back through `Posts q`: no fixpoint in prela. |
| 32626 | `WITH RECURSIVE UserVotes AS (... UNION ALL ... JOIN UserVotes uv ON uv.UserId = u.Id WHERE uv.VoteCount < 10)`: no fixpoint in prela. (The base case happens to be empty on this data, which is why the answer has NULLs, but relying on that would be porting the data.) |
| 34520 | `WITH RECURSIVE PostHierarchy`, whose Level is projected for every post: no fixpoint in prela. |
| 31240 | `WITH RECURSIVE PostHierarchy` with `WHERE ph.Level <= 2` and Level projected: the answers at level 2 come from the recursion. No fixpoint in prela. |
| 31162 | `WITH RECURSIVE PostHierarchy` LEFT JOINed to every post. Nothing from it is projected, but whether the join multiplies a post depends on how often the recursion reaches it, which only a fixpoint (or an assumption about the data) settles. |
| 32343 | `WITH RECURSIVE PopularPosts AS (... UNION ALL ... JOIN PopularPosts pp ON p.AcceptedAnswerId = pp.Id WHERE p.PostTypeId = 2)`, LEFT JOINed for every user: no fixpoint in prela (the recursive step happens to be empty on this data, but relying on that would be porting the data). |
| 33009 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.PostId WHERE ph.Level < 5)` with Level projected for every level: no fixpoint in prela. |
| 32106 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.Id)` with Depth projected and filtered (`Depth <= 5`): no fixpoint in prela. |
| 34585 | `WITH RECURSIVE PopularUsers AS (... UNION ALL ... JOIN PopularUsers pu ON u.Id = (SELECT UserId FROM Votes ... LIMIT 1) WHERE pu.Level < 3)`, whose Level is projected: no fixpoint in prela (and the correlated `LIMIT 1` has no ORDER BY). |
| 32809 | `WITH RECURSIVE PostTree AS (... UNION ALL ... JOIN PostTree pt ON p.ParentId = pt.Id)` LEFT JOINed to every post, with Level projected and sorted on: no fixpoint in prela. |
| 31874 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy h ON p.ParentId = h.PostId)` LEFT JOINed to every recent post, with Level projected: no fixpoint in prela. |
| 33934 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.PostId)` read at `Level = (SELECT MAX(Level) FROM PostHierarchy)`: the deepest level is the fixpoint's; no fixpoint in prela. |
| 32433 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.PostId WHERE p.PostTypeId = 2)`, every level projected and filtered on: no fixpoint in prela. |
| 31932 | `WITH RECURSIVE UserReputationCTE AS (users UNION ALL users JOIN UserReputationCTE UR ON U.Id = UR.UserId WHERE UR.Level < 5)`, every level joined (Reputation + 50 per level): a self-join fixpoint, no fixpoint in prela (as 30183). |
| 34789 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.Id)` LEFT JOINed to every post, with Level projected: no fixpoint in prela. |
| 34876 | `WITH RECURSIVE PostHierarchy` LEFT JOINed to every post, with `MAX(ph.Level)` projected: the depth comes from the recursion, and how often the join multiplies a post depends on it. No fixpoint in prela. |
| 8890 | `LEFT JOIN UserBadges ub ON rp.PostId = (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = ub.UserId LIMIT 1)`: a correlated `LIMIT 1` with no `ORDER BY` picks an arbitrary post per user, and the RankedPosts rows are owned by users with several posts, so which rows get a BadgeCount is DuckDB's scan order, not the query. |
| 31185 | `WITH RECURSIVE UserReputationCTE AS (users UNION ALL users JOIN UserReputationCTE ur ON u.Id = ur.UserId WHERE ur.Depth < 5)`, joined at every depth, so each user's rows are multiplied by the recursion: no fixpoint in prela. |
| 33289 | `WITH RECURSIVE PostHierarchy AS (posts WHERE ParentId IS NULL UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.Id)`, read at `ph.Level = 1` for the reputation score: which posts reach level 1 is the fixpoint's answer; no fixpoint in prela. |
| 34204 | `WITH RECURSIVE PostHierarchy AS (posts WHERE ParentId IS NULL UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.PostId)` with `ph.Level` projected for every post: the levels are the fixpoint's answer; no fixpoint in prela. |
| 30788 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.PostId)` LEFT JOINed through ClosedPosts with Level projected for every closed post: no fixpoint in prela. |
| 29391 | `JOIN Posts P ON T.Id = ANY(string_to_array(P.Tags, '><')::int[])` casts tag names like `<mysql` to INT, which fails on every tagged post; DuckDB answers 0 rows only because ActiveUsers is empty and the cast is never reached, so the empty answer is an artefact of its join order, not of the query. |
| 31150 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.PostId WHERE p.PostTypeId = 2)` LEFT JOINed to every recent post with `ph.Level` projected: the levels are the fixpoint's; no fixpoint in prela. |
| 31229 | `WITH RECURSIVE PostHierarchy AS (posts WHERE ParentId IS NULL UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.PostId)` LEFT JOINed through ClosedPosts with Level projected as ClosedLevel: the levels are the fixpoint's; no fixpoint in prela (the answer is empty today only because RecentPosts reads CURRENT_TIMESTAMP). |
| 33673 | `WITH RECURSIVE PostHierarchy AS (posts WHERE ParentId IS NULL UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.Id)` joined to every post's activity and grouped by `PH.Level`: which posts the recursion reaches, and at what level, is the fixpoint's answer. No fixpoint in prela. |
| 7250 | `LEFT JOIN Badges b ON b.UserId = (SELECT u.Id FROM Users u WHERE u.DisplayName = r.OwnerDisplayName LIMIT 1)`: a correlated `LIMIT 1` with no `ORDER BY` over display names, which are not unique, so which user's badges are summed is the engine's scan order (as 8890). |
| 6640 | `JOIN TopUsers t ON rp.PostId = (SELECT AcceptedAnswerId FROM Posts WHERE AcceptedAnswerId IS NOT NULL AND ParentId = rp.PostId LIMIT 1)`: a correlated `LIMIT 1` with no `ORDER BY` picks an arbitrary child per post, so the query has no single answer wherever two children carry different AcceptedAnswerIds (as 8890). The answer is empty on this data only because no child has an AcceptedAnswerId, which is the data, not the query. |
| 2424 | `LEFT JOIN ClosedPosts CP ON U.UserId = (SELECT OwnerUserId FROM Posts WHERE Title = CP.Title LIMIT 1)`: a correlated `LIMIT 1` with no `ORDER BY`, and 12 closed-post titles are shared by posts of different owners, so which owner a title maps to is DuckDB's scan order (same reason as 8890). |
| 33357 | `WITH RECURSIVE UserReputation AS (users UNION ALL users LEFT JOIN Votes JOIN UserReputation UR ON U.Id = UR.Id WHERE UR.Depth < 3 GROUP BY ...)`, LEFT JOINed at every depth, so each user appears once per level with a reputation the recursion computes: a self-join fixpoint, no fixpoint in prela (as 30183). |
| 21651 | `COALESCE(CAST(ph.Text AS JSON), CAST('{}' AS JSON))` is projected: DuckDB parses and re-serializes the JSON, and the harness has no JSON parser (as 2954). Printing the raw Text would match only because this data is already stored compact. |
| 30583 | `WITH RECURSIVE PostHierarchy AS (questions UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.PostId)`, every level of which becomes a FinalStats row: the rows past the base case are the fixpoint's; no fixpoint in prela. |
| 33230 | `WITH RECURSIVE PostHierarchy AS (posts WHERE ParentId IS NULL UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.Id)`, LEFT JOINed to every post and read at `ph.Level = 1`: which posts the recursion reaches, and at what level, is the fixpoint's answer; no fixpoint in prela. |
| 32329 | `WITH RECURSIVE PostHierarchy AS (questions UNION ALL answers JOIN PostHierarchy ph ON a.ParentId = ph.Id)` aggregated over every level (`COUNT(DISTINCT ph.Id)`, `AVG(... ph.CreationDate)`): the answers come from the recursion; no fixpoint in prela. |
| 4893 | `JOIN CloseReasonTypes ctr ON (ph.Comment::json->>'CloseReasonId')::int = ctr.Id`: the harness has no JSON parser (as 2954). DuckDB answers 0 rows only because PostStatistics (`CURRENT_TIMESTAMP - INTERVAL '1 year'`) is empty, so the JSON join is never reached; that is not an answer to match. |
| 31832 | `WITH RECURSIVE PostHierarchy AS (questions UNION ALL answers JOIN PostHierarchy ph ON a.ParentId = ph.PostId WHERE a.PostTypeId = 2)` with Depth projected and partitioned on for every post: the depth-2 rows come from the recursion; no fixpoint in prela. |
| 22376 | untranslated: `LEFT JOIN UserActivity ua ON ua.UserId = (SELECT u.Id FROM Users u WHERE u.DisplayName = rp.Owner LIMIT 1)`: a correlated `LIMIT 1` with no `ORDER BY` over display names, and the owner "Rick James" of one ranked post is shared by three users, so which user is joined is arbitrary (as 7250). |
| 34077 | `WITH RECURSIVE PostHierarchy AS (posts WHERE ParentId IS NULL UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.Id)` inner-joined to the recent closed posts: which posts it keeps (those reachable from a root) is the fixpoint's answer; no fixpoint in prela. |
| 34568 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ph ON ph.PostId = p2.ParentId)`, and every level is read (rn = 1 per post): prela has no fixpoint. |
| 24907 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.PostId)`, and AggregatedHistory joins every level of it: prela has no fixpoint. |
| 33993 | WITH RECURSIVE PostHierarchy recurses on ParentId and its Level is projected (read beyond the base case); prela has no fixpoint |
| 30603 | WITH RECURSIVE PostHierarchy recurses on ParentId and its Level is projected and filtered (read beyond the base case); prela has no fixpoint |
| 34515 | WITH RECURSIVE PostHierarchy recurses on ParentId and PostLevel is projected (read beyond the base case); prela has no fixpoint |
| 33742 | WITH RECURSIVE PostHierarchy (questions, then answers joined through ParentId) recurses and its level-2 rows reach the output; prela has no fixpoint |
| 30100 | WITH RECURSIVE PostHierarchy recurses on ParentId and ph.Level is projected (read beyond the base case); prela has no fixpoint |
| 32732 | WITH RECURSIVE PostHierarchy (questions, then answers through ParentId) recurses and the WHERE keeps Level = 2 rows; prela has no fixpoint |
| 34888 | WITH RECURSIVE UserActivity joins itself on U.Id = UA.UserId up to ActivityLevel 3, and the levels 2 and 3 copies reach the output (each row three times); prela has no fixpoint |
| 31107 | `WITH RECURSIVE UserReputationHistory AS (users UNION ALL users JOIN UserReputationHistory UH ON U.Id = UH.UserId WHERE UH.Level < 5)`, every level joined: a self-join fixpoint, no fixpoint in prela (as 30183). |
| 31584 | `WITH RECURSIVE PostHierarchy AS (... UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.PostId)` joined on every depth and summed (`SUM(ph.Depth)`): no fixpoint in prela. |
| 30466 | `WITH RECURSIVE TagHierarchy AS (tags UNION ALL tags JOIN TagHierarchy th ON t.Id = th.Id + 1)`, joined on Id so each tag appears once per level: a fixpoint, no fixpoint in prela. |
| 34678 | `WITH RECURSIVE UserReputation AS (users UNION ALL users JOIN UserReputation ur ON u.Id = ur.Id WHERE ur.Depth < 5)`, joined so each user appears once per depth: a self-join fixpoint, no fixpoint in prela (as 30183). |
| 31948 | `WITH RECURSIVE PopularPosts AS (... UNION ALL ... JOIN PopularPosts p ON pp.ParentId = p.Id WHERE pp.PostTypeId = 2)` with every level joined and projected (accumulated Score/ViewCount): no fixpoint in prela. |
| 30455 | `WITH RECURSIVE RecursivePostHierarchy AS (... UNION ALL ... JOIN RecursivePostHierarchy rph ON p.ParentId = rph.Id)` joined on Id with Level projected: every level can match, no fixpoint in prela. |
| 30795 | `WITH RECURSIVE UserHierarchy AS (... UNION ALL ... JOIN UserHierarchy uh ON u.Id = uh.Id + 1)`, a chain of consecutive user ids building a Path string, read beyond the base case: prela has no fixpoint. |
| 31296 | `WITH RECURSIVE UserReputation AS (... UNION ALL ... JOIN UserReputation UR ON U.Id = UR.Id WHERE UR.Level < 5)`: really recurses (every user six times, levels 0-5) and is read beyond the base case; prela has no fixpoint. |
| 34443 | `WITH RECURSIVE TagHierarchy AS (tags UNION ALL ... JOIN TagHierarchy th ON t.ExcerptPostId = th.Id)` LEFT JOINed to every user: which tags the recursion reaches is the fixpoint's answer; no fixpoint in prela (the answer is empty only because RecentEdits reads CURRENT_TIMESTAMP). |
| 32928 | `WITH RECURSIVE PostHierarchy AS (questions UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.Id)` LEFT JOINed ON tu.UserId = ph.Id with `COALESCE(ph.Id, 0)` projected: which ids the recursion reaches is the fixpoint's answer; no fixpoint in prela. |
| 31020 | `WITH RECURSIVE PostHierarchy AS (posts WHERE ParentId IS NULL UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.Id)` read as `MAX(ph.Level) > 0` and projected: the levels are the fixpoint's; no fixpoint in prela. |
| 34809 | `WITH RECURSIVE UserReputationCTE AS (users UNION ALL users JOIN Votes JOIN UserReputationCTE C ON V.PostId IN (posts of C.Id) WHERE U.Reputation > C.Reputation)`, whose `AVG(Reputation)` is projected on every row: the average is over the fixpoint; no fixpoint in prela. |
| 30668 | `WITH RECURSIVE PostHierarchy AS (posts WHERE ParentId IS NULL UNION ALL ... JOIN PostHierarchy PH ON P.ParentId = PH.PostId)`, every row of which becomes a PostStats row: which posts the recursion reaches is the fixpoint's answer; no fixpoint in prela. |
| 30881 | `WITH RECURSIVE RecursivePostTree AS (questions UNION ALL ... JOIN RecursivePostTree pt ON p.ParentId = pt.PostId)`, grouped by `rpt.Level` with Level projected: the levels are the fixpoint's; no fixpoint in prela. |
| 30608 | `WITH RECURSIVE UserReputation AS (users UNION ALL users JOIN UserReputation UR ON U.Id = UR.UserId WHERE U.Reputation > 1000 + UR.Level * 500)` LEFT JOINed to every user: each user appears once per level the recursion reaches, which multiplies the output rows; no fixpoint in prela (as 30244). |
| 30831 | `WITH RECURSIVE PostHierarchy AS (posts WHERE ParentId IS NULL UNION ALL ... JOIN PostHierarchy ph ON p.ParentId = ph.PostId)`, every row of which is output with `ph.Level` projected and sorted on: no fixpoint in prela. |
| 34195 | `WITH RECURSIVE HighRankingUsers AS (users with Reputation > 5000 UNION ALL ... JOIN HighRankingUsers hru ON u.Id = hru.Id + 1)`, LEFT JOINed on DisplayName: a user appears once per run of consecutive ids ending at it, so the join multiplicity is the fixpoint's. No fixpoint in prela. |
| 31396 | `WITH RECURSIVE UserReputationCTE AS (users UNION ALL ... JOIN Users U ON UR.UserId = U.Id WHERE UR.Reputation + U.Reputation < 5000)`, whose accumulated Reputation is projected and filtered for every level: the rows are the fixpoint's; no fixpoint in prela (as 31932). |
| 32893 | `WITH RECURSIVE PostHierarchy AS (posts with no parent UNION ALL ... JOIN PostHierarchy PH ON P.ParentId = PH.PostId)`, read for every question: a question with a ParentId would come in through the recursion. On this data none does, but relying on that would be porting the data. No fixpoint in prela. |
| 30985 | `WITH RECURSIVE UserReputationCTE AS (users UNION ALL users JOIN Votes JOIN UserReputationCTE UR ON V.PostId IN (posts of UR.Id) WHERE UR.Level < 3)`, every level joined to RankedPosts: a self-join fixpoint, no fixpoint in prela (as 30183, 34585). Postgres-oracle query. |

Batches 39-63 cleared everything else that was on this list:

- the twelve substring-join queries (19637, 15835, 16507, 19034, 15017,
  19692, 15136, 15838, 16290, 19002, 19176, 19100) are ported in `b49`
  through `harness::tag_mentions`, which is `flat_map` + `select_where` over
  the 1,233 distinct tag strings instead of a scan of every post against
  every tag. All twelve needed a `rewrites/` entry for a tie at the `LIMIT`.
- 12701 (`STRING_AGG`) turned out to be degenerate: it splits `Tags` on
  `'<>'`, which never occurs, so no tag name ever joins and the aggregate is
  NULL for every row. Ported in `b49`.
- 11573 (`ARRAY_AGG`) does not run on DuckDB; moved to the invalid table.

See `limitations.md` for what is still awkward.

## Invalid queries

Queries the worklist offered that DuckDB refuses to run. Recorded here so
`tools/next.py` stops offering them; they are not prela's problem.

| query | error                                                                                           |
| ----- | ----------------------------------------------------------------------------------------------- |
| 19976 | `ORDER BY p.CreationDate` where CreationDate is neither grouped nor aggregated                  |
| 18269 | `SELECT p.Title` where Title is neither grouped nor aggregated (`GROUP BY p.Id, u.DisplayName`) |
| 9281 | `RANK() OVER (PARTITION BY p.PostTypeId ...)` where PostTypeId is not in the `GROUP BY` |
| 5387 | `ROW_NUMBER() OVER (... ORDER BY p.CreationDate DESC)` where CreationDate is not in the `GROUP BY` |
| 5303 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ...)` where PostTypeId is not in the `GROUP BY` |
| 8480 | `RANK() OVER (PARTITION BY p.PostTypeId ...)` where PostTypeId is not in the `GROUP BY` |
| 7190 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ...)` where PostTypeId is not in the `GROUP BY` |
| 5830 | `RANK() OVER (PARTITION BY p.OwnerUserId ...)` where OwnerUserId is not in the `GROUP BY` |
| 7012 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ...)` where PostTypeId is not in the `GROUP BY` |
| 5182 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ...)` where OwnerUserId is not in the `GROUP BY` |
| 6703 | `SELECT p.Title` where Title is not in the `GROUP BY` |
| 7631 | `RANK() OVER (PARTITION BY p.OwnerUserId ...)` where OwnerUserId is not in the `GROUP BY` |
| 5605 | `RANK() OVER (PARTITION BY p.OwnerUserId ...)` where OwnerUserId is not in the `GROUP BY` |
| 6372 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ...)` where OwnerUserId is not in the `GROUP BY` |
| 8131 | `DENSE_RANK() OVER (... ORDER BY p.Score DESC)` where Score is not in the `GROUP BY` |
| 5470 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ...)` where PostTypeId is not in the `GROUP BY` |
| 25360 | `CARDINALITY(string_to_array(...))`: DuckDB's `CARDINALITY` only takes a MAP |
| 5806 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ...)` where OwnerUserId is not in the `GROUP BY` |
| 6488 | `RANK() OVER (PARTITION BY P.PostTypeId ...)` where PostTypeId is not in the `GROUP BY` |
| 26235 | `ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC)` where CreationDate is not in the `GROUP BY` |
| 7912 | `UNNEST` inside `GROUP BY`: `UNNEST not supported here` |
| 25680 | `UNNEST` in a `SELECT` with `GROUP BY`: `UNNEST not supported here` |
| 14841 | thread-unstable: `ORDER BY` a count that ties across the `LIMIT`, so DuckDB's answer moves with `SET threads` |
| 19307 | thread-unstable: `ORDER BY CommentCount DESC FETCH FIRST 10`, ties at the cut |
| 25070 | `RANK() OVER (PARTITION BY p.PostTypeId ...)` where PostTypeId is not in the `GROUP BY` |
| 25371 | `SELECT rp.Title` where Title is not in the `GROUP BY` (`GROUP BY p.Id, pt.Name, u.DisplayName`) |
| 11293 | `AS t(TagName)` over an `unnest`: DuckDB reads the alias as a struct, `Type VARCHAR with value 'mysql' can't be cast to STRUCT(unnest ...)` |
| 1192 | `UNNEST` inside an `IN (SELECT DISTINCT CAST(UNNEST(...)))`: `UNNEST not supported here` |
| 1815 | `LEFT JOIN` onto a correlated subquery: `Cannot perform non-inner join on subquery!` |
| 25215 | `UNNEST` in a `SELECT` with `GROUP BY`: `UNNEST not supported here` |
| 34670 | `trim(both '{}' FROM unnest(...))` on the struct alias: `No function matches trim(STRUCT(unnest VARCHAR))` |
| 31411 | `SELECT ph.CreationDate` where CreationDate is not in the `GROUP BY` |
| 20329 | `SELECT p.OwnerUserId` where OwnerUserId is not in the `GROUP BY` |
| 20572 | `SELECT u.Reputation` where Reputation is not in the `GROUP BY` |
| 20664 | `LEFT JOIN` onto a correlated subquery: `Cannot perform non-inner join on subquery!` |
| 20804 | same `AS t(TagName)` struct-alias conversion error as 11293 |
| 20940 | `SELECT p.Title` where Title is not in the `GROUP BY` |
| 21015 | same `AS t(TagName)` struct-alias conversion error as 11293 |
| 21577 | `SELECT p.Title` where Title is not in the `GROUP BY` |
| 21824 | same `AS t(TagName)` struct-alias conversion error as 11293 |
| 21853 | `SELECT p.OwnerUserId` where OwnerUserId is not in the `GROUP BY` |
| 21928 | `SELECT p.CreationDate` where CreationDate is not in the `GROUP BY` |
| 22132 | `SELECT p.OwnerUserId` where OwnerUserId is not in the `GROUP BY` |
| 22628 | `SELECT p.Title` where Title is not in the `GROUP BY` |
| 22708 | `UNNEST` in a `SELECT` with `GROUP BY`: `UNNEST not supported here` |
| 10009 | `LEFT JOIN unnest(string_to_array(p.Tags,'<>')) AS tag ON tag IS NOT NULL`: DuckDB reads the alias as a struct, `Type VARCHAR with value 'mysql' can't be cast to STRUCT(unnest ...)` |
| 25201 | `TRIM(UNNEST(...))`: `UNNEST not supported here` |
| 27745 | `UNNEST not supported here` |
| 26700 | `UNNEST not supported here` |
| 25709 | `Cannot perform non-inner join on subquery` |
| 8587 | `ROW_NUMBER() OVER (... ORDER BY p.LastActivityDate DESC)` where LastActivityDate is not in the `GROUP BY` |
| 11573 | `unnest(string_to_array(p.Tags, ',')) AS tag_name ... ON t.TagName = tag_name` compares a VARCHAR to the unnest's STRUCT: `Conversion Error` |
| 16284 | `ORDER BY p.CreationDate` where CreationDate is neither grouped nor aggregated (`GROUP BY p.Id, p.Title, p.Score, u.DisplayName`) |
| 10051 | `PostTypeId` is selected or ordered by but neither grouped nor aggregated |
| 10810 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 11163 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 11454 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 11667 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 12644 | `Tags` is selected or ordered by but neither grouped nor aggregated |
| 12743 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 12857 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 13004 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 13289 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 13606 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 13918 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 14284 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 14320 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 15072 | `PostTypeId` is selected or ordered by but neither grouped nor aggregated |
| 15132 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 15230 | `Score` is selected or ordered by but neither grouped nor aggregated |
| 15247 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 15250 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 15299 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 15422 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 15559 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 15721 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 15807 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 16697 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 16719 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 18075 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 18343 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 18471 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 19536 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 19679 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 19905 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 12485 | `PostTypeId` is selected or ordered by but neither grouped nor aggregated |
| 11124 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 14917 | `Score` is selected or ordered by but neither grouped nor aggregated |
| 10159 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 11211 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 14974 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 13356 | `PostTypeId` is selected or ordered by but neither grouped nor aggregated |
| 10703 | `OwnerUserId` is selected or ordered by but neither grouped nor aggregated |
| 10713 | `LastActivityDate` is selected or ordered by but neither grouped nor aggregated |
| 10841 | `ClosedDate` is selected or ordered by but neither grouped nor aggregated |
| 11018 | `OwnerUserId` is selected or ordered by but neither grouped nor aggregated |
| 11237 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 11463 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 12460 | `BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit ty` |
| 12580 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 12828 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 13582 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 13729 | `CommentCount` is selected or ordered by but neither grouped nor aggregated |
| 13902 | `CommentCount` is selected or ordered by but neither grouped nor aggregated |
| 14035 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 14478 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 14652 | `Title` is selected or ordered by but neither grouped nor aggregated |
| 25659 | `Tags` is selected or ordered by but neither grouped nor aggregated |
| 26819 | `Tags` is selected or ordered by but neither grouped nor aggregated |
| 5705 | `CreationDate` is selected or ordered by but neither grouped nor aggregated |
| 9640 | `DisplayName` is selected or ordered by but neither grouped nor aggregated |
| 9434 | DuckDB binder error: AnswerCount not in GROUP BY |
| 31459 | DuckDB binder error: Title not in GROUP BY |
| 21933 | DuckDB binder error: DisplayName not in GROUP BY |
| 29674 | DuckDB binder error: UNNEST not supported here |
| 14861 | DuckDB binder error: Title not in GROUP BY |
| 27626 | BinderException: Cardinality can only operate on MAPs |
| 11810 | column "OwnerUserId" must appear in the GROUP BY |
| 24620 | column "DisplayName" must appear in the GROUP BY |
| 20657 | column "AnswerCount" must appear in the GROUP BY |
| 14728 | BinderException: trim(STRUCT(unnest VARCHAR)) |
| 26843 | BinderException: UNNEST not supported here |
| 1363 | BinderException: column must appear in the GROUP BY |
| 4238 | BinderException: column must appear in the GROUP BY |
| 2986 | BinderException: column must appear in the GROUP BY |
| 225 | BinderException: column must appear in the GROUP BY |
| 5329 | BinderException: column must appear in the GROUP BY |
| 11955 | BinderException: column must appear in the GROUP BY |
| 13680 | thread-unstable: `AVG` of per-post `AVG(EXTRACT(EPOCH FROM V.CreationDate))` doubles; the last digit moves with `SET threads` (1: ...976099, 2: ...976101, 4: ...976100) |
| 10807 | ConversionException: `LEFT JOIN UNNEST(...) AS tag_array ... t.TagName = tag_array` compares a VARCHAR to the unnest STRUCT |
| 9886 | BinderException: UNNEST not supported here |
| 26034 | BinderException: column "UpVotes" must appear in the GROUP BY clause (ORDER BY UpVotes - DownVotes) |
| 4759 | thread-unstable: per-user `AVG(EXTRACT(EPOCH ...) / 3600)` doubles; the last digit moves with `SET threads` |
| 32216 | BinderException: column "CreationDate" must appear in the GROUP BY clause (ORDER BY p.CreationDate) |
| 13507 | ConversionException: `UNNEST(...) AS tag_array ... t.TagName = tag_array` compares a VARCHAR to the unnest STRUCT |
| 12256 | BinderException: trim(STRUCT(unnest VARCHAR)) |
| 5283 | BinderException: column must appear in the GROUP BY |
| 7199 | BinderException: column must appear in the GROUP BY |
| 9931 | BinderException: column must appear in the GROUP BY |
| 9902 | BinderException: column must appear in the GROUP BY |
| 6483 | BinderException: column must appear in the GROUP BY |
| 12458 | BinderException: column must appear in the GROUP BY |
| 9212 | BinderException: column must appear in the GROUP BY |
| 27148 | BinderException: column must appear in the GROUP BY |
| 5403 | BinderException: column must appear in the GROUP BY |
| 8113 | BinderException: column must appear in the GROUP BY |
| 5336 | BinderException: column must appear in the GROUP BY |
| 8305 | BinderException: column must appear in the GROUP BY |
| 6854 | BinderException: column must appear in the GROUP BY |
| 6424 | BinderException: column must appear in the GROUP BY |
| 7023 | BinderException: column must appear in the GROUP BY |
| 6118 | BinderException: column must appear in the GROUP BY |
| 6722 | BinderException: column must appear in the GROUP BY |
| 5070 | BinderException: column must appear in the GROUP BY |
| 6136 | BinderException: column must appear in the GROUP BY |
| 9028 | BinderException: column must appear in the GROUP BY |
| 7128 | BinderException: column must appear in the GROUP BY |
| 7170 | BinderException: column must appear in the GROUP BY |
| 6393 | BinderException: column must appear in the GROUP BY |
| 8929 | BinderException: column must appear in the GROUP BY |
| 29450 | BinderException: column must appear in the GROUP BY |
| 6110 | BinderException: column must appear in the GROUP BY |
| 9701 | BinderException: column must appear in the GROUP BY |
| 9966 | BinderException: column must appear in the GROUP BY |
| 5929 | BinderException: column must appear in the GROUP BY |
| 8550 | BinderException: column must appear in the GROUP BY |
| 29752 | BinderException: column must appear in the GROUP BY |
| 9799 | BinderException: column must appear in the GROUP BY |
| 26230 | BinderException: column must appear in the GROUP BY |
| 8458 | BinderException: column must appear in the GROUP BY |
| 8188 | BinderException: column must appear in the GROUP BY |
| 1354 | BinderException: column must appear in the GROUP BY |
| 9610 | BinderException: column must appear in the GROUP BY |
| 28578 | BinderException: column must appear in the GROUP BY |
| 8949 | BinderException: column must appear in the GROUP BY |
| 5842 | BinderException: column must appear in the GROUP BY |
| 8414 | BinderException: column must appear in the GROUP BY |
| 9357 | BinderException: column must appear in the GROUP BY |
| 28099 | BinderException: UNNEST not supported here |
| 25498 | BinderException: column must appear in the GROUP BY |
| 8333 | BinderException: column must appear in the GROUP BY |
| 8357 | BinderException: column must appear in the GROUP BY |
| 5435 | BinderException: column must appear in the GROUP BY |
| 6014 | BinderException: column must appear in the GROUP BY |
| 21461 | BinderException: column must appear in the GROUP BY |
| 8564 | BinderException: column must appear in the GROUP BY |
| 9860 | BinderException: column must appear in the GROUP BY |
| 27742 | BinderException: UNNEST not supported here |
| 25580 | BinderException: UNNEST not supported here |
| 25958 | BinderException: UNNEST not supported here |
| 27679 | BinderException: column must appear in the GROUP BY |
| 27846 | BinderException: UNNEST not supported here |
| 28474 | BinderException: UNNEST not supported here |
| 8725 | BinderException: column must appear in the GROUP BY |
| 27267 | BinderException: UNNEST not supported here |
| 3198 | BinderException: column must appear in the GROUP BY |
| 33400 | BinderException: Binder Error: column "Name" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26100 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22525 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25060 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 10302 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 28337 | BinderException: Binder Error: UNNEST not supported here |
| 5369 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7163 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28360 | BinderException: Binder Error: UNNEST not supported here |
| 9569 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8434 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8891 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9039 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9227 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8247 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9105 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6064 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 5764 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5845 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27263 | Binder Error: column "Tags" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5242 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5646 | Binder Error: column "Tags" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7159 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9346 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2559 | Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7695 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5665 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7812 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5892 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6473 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5656 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25098 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6221 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9651 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29307 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27435 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 7407 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5088 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27368 | CatalogException: Catalog Error: Scalar Function with name regexp_substr does not exist! |
| 6768 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9852 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6740 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27372 | BinderException: Binder Error: UNNEST not supported here |
| 6658 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7386 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7986 | BinderException: Binder Error: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8631 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25569 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5297 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8426 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2116 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5814 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8025 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8568 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7018 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8931 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6209 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7829 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4826 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8208 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7549 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7595 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7449 | BinderException: Binder Error: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6152 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6012 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8875 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8957 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 344 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9134 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28329 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5094 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9231 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6381 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 11099 | BinderException: Binder Error: column "AnswerCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5904 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26408 | BinderException: Binder Error: column "Tags" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3392 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6092 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9304 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7363 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 834 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9457 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6355 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5311 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6846 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4767 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8677 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 889 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 7724 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26058 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29512 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29874 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6462 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8041 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26011 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29541 | Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7216 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28288 | Binder Error: UNNEST not supported here |
| 8300 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4871 | Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5732 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6875 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26630 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27935 | Binder Error: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29225 | Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29409 | Catalog Error: Scalar Function with name initcap does not exist! |
| 312 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25620 | Binder Error: UNNEST not supported here |
| 32796 | Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23991 | BinderException: Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8142 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6218 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 9608 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21535 | BinderException: Binder Error: Cardinality can only operate on MAPs |
| 25126 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27185 | BinderException: Binder Error: UNNEST not supported here |
| 2516 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27509 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 1941 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29283 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2035 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 33084 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 31681 | BinderException: Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27107 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 64 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2571 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 33618 | BinderException: Binder Error: column "ClosedDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6188 | ParserException: Parser Error: syntax error at or near "WITH" |
| 597 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 32203 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 513 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 25681 | ParserException: Parser Error: syntax error at or near "." |
| 7956 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 2114 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 27045 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 33254 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 33044 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 32355 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22960 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27457 | BinderException: Binder Error: UNNEST not supported here |
| 28822 | BinderException: Binder Error: UNNEST not supported here |
| 5757 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 1244 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 10000 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27033 | BinderException: Binder Error: UNNEST not supported here |
| 8293 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6802 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8132 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7304 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 2340 | BinderException: Binder Error: UNNEST not supported here |
| 28955 | BinderException: Binder Error: UNNEST not supported here |
| 8788 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5601 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7275 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29187 | BinderException: Binder Error: UNNEST not supported here |
| 2199 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7462 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3052 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 8185 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5287 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6347 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8866 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9979 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8981 | BinderException: Binder Error: UNNEST not supported here |
| 6716 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6562 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 9871 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6559 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 726 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20665 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7382 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 1532 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6520 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8270 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 7224 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7146 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6576 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9922 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9160 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28743 | BinderException: Binder Error: UNNEST not supported here |
| 6648 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7721 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7492 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6868 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8744 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3743 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2346 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9255 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3103 | BinderException: Binder Error: column "LastActivityDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7924 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5194 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6376 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 1350 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6099 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8774 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9898 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9115 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3268 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9936 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7806 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29212 | BinderException: Binder Error: UNNEST not supported here |
| 8826 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5050 | BinderException: Binder Error: column "UpVotes" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8570 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7900 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4138 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7952 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6762 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7804 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9315 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 1890 | BinderException: Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7245 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7898 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9122 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 1492 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23141 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23415 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6354 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5963 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6518 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7035 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2758 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8654 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27853 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3895 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3257 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5683 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26921 | BinderException: Binder Error: UNNEST not supported here |
| 4208 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9389 | BinderException: Binder Error: UNNEST not supported here |
| 8232 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7559 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 362 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8175 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6324 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2412 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7828 | BinderException: Binder Error: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 975 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9994 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 25437 | BinderException: Binder Error: column "Tags" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8618 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 1002 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 337 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2294 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4341 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4769 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29982 | BinderException: Binder Error: column "Tags" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8432 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 739 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3866 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26975 | BinderException: Binder Error: column "ParentId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4837 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8367 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4984 | BinderException: Binder Error: column "LastActivityDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3935 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24159 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28722 | BinderException: Binder Error: UNNEST not supported here |
| 3856 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6761 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2326 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4510 | BinderException: Binder Error: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 33931 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3474 | CatalogException: Catalog Error: Type with name jsonb does not exist! |
| 1481 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 33478 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2628 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9156 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9072 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25784 | BinderException: Binder Error: UNNEST not supported here |
| 7572 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 1303 | BinderException: Binder Error: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28523 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7041 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4736 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 674 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7137 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3947 | BinderException: Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 33500 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 475 | BinderException: Binder Error: column Score must appear in the GROUP BY clause or be used in an aggregate function |
| 25167 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 1748 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 915 | BinderException: Binder Error: column "LastActivityDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7303 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20165 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 34102 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 5412 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26964 | BinderException: Binder Error: UNNEST not supported here |
| 32990 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 656 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29466 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 1683 | BinderException: Binder Error: column "UpVotes" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24035 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8401 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 31961 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4844 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 34634 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21220 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 28449 | BinderException: Binder Error: UNNEST not supported here |
| 25011 | BinderException: Binder Error: UNNEST not supported here |
| 3708 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23593 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6296 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 870 | BinderException: Binder Error: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20205 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 546 | BinderException: Binder Error: column "UpVotes" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 32565 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 810 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4764 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 9957 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28626 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4944 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 31596 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 30955 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20122 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 33831 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 31165 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24667 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29371 | BinderException: Binder Error: Cardinality can only operate on MAPs |
| 34732 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29393 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21330 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24229 | BinderException: Binder Error: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 31469 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 31686 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24553 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 24301 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24837 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 33489 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23571 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 30114 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 31050 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21690 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9493 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 9981 | BinderException: Binder Error: UNNEST not supported here |
| 26365 | BinderException: Binder Error: UNNEST not supported here |
| 26639 | BinderException: Binder Error: UNNEST not supported here |
| 28445 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 5729 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20234 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3563 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28670 | BinderException: Binder Error: UNNEST not supported here |
| 27877 | BinderException: Binder Error: UNNEST not supported here |
| 28108 | BinderException: Binder Error: UNNEST not supported here |
| 25603 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 26631 | BinderException: Binder Error: UNNEST not supported here |
| 1977 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 25258 | BinderException: Binder Error: UNNEST not supported here |
| 27297 | BinderException: Binder Error: UNNEST not supported here |
| 70 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9519 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5105 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5882 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2869 | BinderException: Binder Error: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6457 | BinderException: Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8665 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4124 | BinderException: Binder Error: column "LastActivityDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6143 | BinderException: Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 1561 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 27159 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR), STRING_LITERAL)'. You might need to add explicit type casts. |
| 26903 | BinderException: Binder Error: UNNEST not supported here |
| 856 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 692 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 951 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 142 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 3561 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9869 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4457 | BinderException: Binder Error: column "AnswerCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 289 | BinderException: Binder Error: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 30184 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 1074 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5344 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9007 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6929 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25611 | BinderException: Binder Error: UNNEST not supported here |
| 135 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9006 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5376 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3037 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 240 | BinderException: Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8919 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4282 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 214 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24856 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3696 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4946 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4789 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23605 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22861 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 5870 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24427 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9623 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9076 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3897 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 30156 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4729 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5784 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2836 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 23248 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 31046 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 25770 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 3997 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9703 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27220 | BinderException: Binder Error: UNNEST not supported here |
| 2243 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4436 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 990 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4395 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9741 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 34663 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3920 | BinderException: Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21176 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22166 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 701 | BinderException: Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 31148 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3925 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2991 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24440 | BinderException: Binder Error: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24763 | BinderException: Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4429 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22507 | BinderException: Binder Error: column "Body" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27428 | BinderException: Binder Error: UNNEST not supported here |
| 2711 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4689 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22216 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23333 | BinderException: Binder Error: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 1518 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21375 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 32716 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3816 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21246 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4957 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24116 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20874 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20449 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23882 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20289 | BinderException: Binder Error: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24247 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22284 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22028 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24671 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21792 | BinderException: Binder Error: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3628 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24855 | CatalogException: Catalog Error: Type with name jsonb does not exist! |
| 33482 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 908 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21271 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 34494 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 21820 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21419 | BinderException: Binder Error: column ViewCount must appear in the GROUP BY clause or be used in an aggregate function |
| 21527 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20849 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22932 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24565 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22600 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24777 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22085 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23489 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 44 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 32283 | BinderException: Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21134 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20615 | BinderException: Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24986 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22385 | BinderException: Binder Error: column "ClosedDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23560 | BinderException: Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20115 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23293 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23379 | BinderException: Binder Error: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2042 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25357 | BinderException: Binder Error: UNNEST not supported here |
| 6132 | BinderException: Binder Error: UNNEST not supported here |
| 26491 | BinderException: Binder Error: UNNEST not supported here |
| 29977 | BinderException: Binder Error: UNNEST not supported here |
| 28678 | BinderException: Binder Error: UNNEST not supported here |
| 28665 | BinderException: Binder Error: UNNEST not supported here |
| 27936 | BinderException: Binder Error: No function matches the given name and argument types 'ltrim(STRUCT(unnest VARCHAR), STRING_LITERAL)'. |
| 25340 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) |
| 6902 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4340 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8965 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29385 | BinderException: Binder Error: UNNEST not supported here |
| 26177 | BinderException: Binder Error: UNNEST not supported here |
| 26615 | BinderException: Binder Error: UNNEST not supported here |
| 28232 | BinderException: Binder Error: UNNEST not supported here |
| 3828 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28455 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 873 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29389 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 33982 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8438 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 1102 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 26441 | BinderException: Binder Error: UNNEST not supported here |
| 26781 | BinderException: Binder Error: UNNEST not supported here |
| 21265 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29384 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23282 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2386 | Binder Error: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2218 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 2180 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2225 | Binder Error: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 30783 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24544 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 34961 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 21940 | Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20935 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 968 | Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 30385 | Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22634 | Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3849 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23955 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 22237 | Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22547 | Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20807 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24622 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20483 | Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22062 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 32099 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 20384 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28044 | Binder Error: UNNEST not supported here |
| 28853 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27740 | Binder Error: UNNEST not supported here |
| 27427 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25562 | Binder Error: UNNEST not supported here |
| 5030 | Binder Error: UNNEST not supported here |
| 27839 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 270 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 25441 | Binder Error: UNNEST not supported here |
| 25795 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6081 | Binder Error: UNNEST not supported here |
| 28867 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 9682 | Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8219 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25567 | Binder Error: UNNEST not supported here |
| 29877 | Binder Error: UNNEST not supported here |
| 25982 | Binder Error: UNNEST not supported here |
| 5181 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 531 | Catalog Error: Type with name jsonb does not exist! |
| 1017 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25977 | Binder Error: UNNEST not supported here |
| 25837 | Binder Error: UNNEST not supported here |
| 26147 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29929 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27086 | Binder Error: UNNEST not supported here |
| 4830 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8906 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23082 | Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28593 | Binder Error: UNNEST not supported here |
| 26199 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29163 | BinderException: Binder Error: UNNEST not supported here |
| 1934 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26191 | BinderException: Binder Error: Cardinality can only operate on MAPs |
| 27198 | BinderException: Binder Error: UNNEST not supported here |
| 29554 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27953 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28868 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29068 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 32274 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25048 | BinderException: Binder Error: UNNEST not supported here |
| 25339 | BinderException: Binder Error: column "LastActivityDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28849 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 8995 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8422 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 25717 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 34814 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 29578 | BinderException: Binder Error: column "Tags" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27775 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 32848 | BinderException: Binder Error: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27031 | BinderException: Binder Error: UNNEST not supported here |
| 28312 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 5476 | BinderException: Binder Error: UNNEST not supported here |
| 29745 | BinderException: Binder Error: UNNEST not supported here |
| 7263 | BinderException: Binder Error: UNNEST not supported here |
| 2320 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28842 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28040 | BinderException: Binder Error: UNNEST not supported here |
| 28182 | BinderException: Binder Error: UNNEST not supported here |
| 25119 | BinderException: Binder Error: UNNEST not supported here |
| 5700 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25041 | BinderException: Binder Error: UNNEST not supported here |
| 29797 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 25821 | BinderException: Binder Error: UNNEST not supported here |
| 29885 | BinderException: Binder Error: UNNEST not supported here |
| 26861 | BinderException: Binder Error: UNNEST not supported here |
| 26274 | Binder Error: UNNEST not supported here |
| 29268 | Binder Error: UNNEST not supported here |
| 26212 | Binder Error: UNNEST not supported here |
| 27234 | Binder Error: UNNEST not supported here |
| 25096 | Binder Error: UNNEST not supported here |
| 8560 | Binder Error: UNNEST not supported here |
| 7624 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29556 | Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 32841 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 7269 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28004 | Binder Error: UNNEST not supported here |
| 27353 | Binder Error: UNNEST not supported here |
| 27431 | Binder Error: UNNEST not supported here |
| 29591 | Binder Error: UNNEST not supported here |
| 34942 | Binder Error: UNNEST not supported here |
| 31037 | Binder Error: column "Tags" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29800 | Binder Error: UNNEST not supported here |
| 25452 | Binder Error: UNNEST not supported here |
| 31659 | Binder Error: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25755 | Binder Error: UNNEST not supported here |
| 29641 | Binder Error: UNNEST not supported here |
| 28504 | Binder Error: UNNEST not supported here |
| 7325 | Binder Error: UNNEST not supported here |
| 27729 | Binder Error: UNNEST not supported here |
| 32162 | Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9721 | Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 27103 | Binder Error: UNNEST not supported here |
| 2372 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25173 | Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 27855 | Binder Error: UNNEST not supported here |
| 29032 | Binder Error: UNNEST not supported here |
| 34400 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 32327 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27460 | Binder Error: UNNEST not supported here |
| 31073 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22024 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5709 | Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 21782 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28697 | Binder Error: UNNEST not supported here |
| 27924 | Binder Error: UNNEST not supported here |
| 7643 | Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR), STRING_LITERAL)'. You might need to add explicit type casts. |
| 4307 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5964 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26688 | Binder Error: UNNEST not supported here |
| 29632 | Binder Error: UNNEST not supported here |
| 2812 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27053 | Binder Error: UNNEST not supported here |
| 26863 | Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 3544 | Binder Error: UNNEST not supported here |
| 5255 | Binder Error: UNNEST not supported here |
| 31136 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 25914 | Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 27001 | Binder Error: UNNEST not supported here |
| 27112 | Binder Error: UNNEST not supported here |
| 24422 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5515 | Binder Error: UNNEST not supported here |
| 8471 | Binder Error: UNNEST not supported here |
| 27481 | Binder Error: UNNEST not supported here |
| 27342 | Binder Error: UNNEST not supported here |
| 26169 | Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 25839 | Binder Error: UNNEST not supported here |
| 29994 | Binder Error: UNNEST not supported here |
| 24805 | Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 185 | Binder Error: UNNEST not supported here |
| 342 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 27117 | Binder Error: UNNEST not supported here |
| 8573 | Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3285 | Binder Error: UNNEST not supported here |
| 9303 | Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 24042 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 25445 | Binder Error: UNNEST not supported here |
| 13199 | Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 22468 | Conversion Error: Could not convert string 'User 214700 deleted' to INT32 when casting from source column Comment |
| 26693 | Binder Error: UNNEST not supported here |
| 28829 | Binder Error: UNNEST not supported here |
| 30296 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 27716 | Binder Error: UNNEST not supported here |
| 2613 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 22349 | Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25937 | Binder Error: UNNEST not supported here |
| 20816 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 28873 | Binder Error: UNNEST not supported here |
| 22961 | Binder Error: column "Tags" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24592 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 7147 | Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3508 | Binder Error: UNNEST not supported here |
| 2139 | Binder Error: UNNEST not supported here |
| 29940 | ParserException: syntax error at or near "." (`at` is a reserved alias) |
| 25058 | BinderException: UNNEST not supported here |
| 32843 | BinderException: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function |
| 20097 | NotImplementedException: Cannot perform non-inner join on subquery! |
| 26927 | BinderException: UNNEST not supported here |
| 982 | CatalogException: Type with name jsonb does not exist! |
| 26496 | BinderException: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function |
| 22066 | BinderException: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function |
| 29172 | NotImplementedException: Cannot perform non-inner join on subquery! |
| 28617 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) |
| 9513 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29093 | BinderException: UNNEST not supported here |
| 27536 | BinderException: UNNEST not supported here |
| 22498 | BinderException: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21710 | BinderException: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26271 | BinderException: UNNEST not supported here |
| 23526 | BinderException: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28666 | BinderException: UNNEST not supported here |
| 22016 | BinderException: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22232 | BinderException: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26683 | BinderException: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 30120 | BinderException: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28000 | BinderException: UNNEST not supported here |
| 30572 | BinderException: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21880 | ConversionException: Type VARCHAR with value '{}' can't be cast to the destination type VARCHAR[] |
| 26335 | BinderException: UNNEST not supported here |
| 1270 | BinderException: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28630 | BinderException: UNNEST not supported here |
| 26940 | BinderException: UNNEST not supported here |
| 33721 | NotImplementedException: Cannot perform non-inner join on subquery! |
| 21892 | BinderException: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20853 | BinderException: column "Body" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22867 | BinderException: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 32860 | BinderException: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29980 | BinderException: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22010 | NotImplementedException: Cannot perform non-inner join on subquery! |
| 21926 | NotImplementedException: Cannot perform non-inner join on subquery! |
| 22773 | BinderException: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 5730 | BinderException: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28250 | BinderException: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26895 | BinderException: UNNEST not supported here |
| 21321 | BinderException: column "Tags" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23304 | BinderException: column "Body" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26792 | BinderException: UNNEST not supported here |
| 26142 | BinderException: UNNEST not supported here |
| 25319 | BinderException: UNNEST not supported here |
| 23162 | BinderException: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29430 | BinderException: UNNEST not supported here |
| 25861 | BinderException: UNNEST not supported here |
| 6515 | BinderException: UNNEST not supported here |
| 28528 | BinderException: Binder Error: UNNEST not supported here |
| 8645 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 8248 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 8386 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 7155 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 7394 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8346 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26480 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 26370 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 25209 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 26089 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 29975 | NotImplementedException: Not implemented Error: Cannot perform non-inner join on subquery! |
| 29543 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25301 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 9177 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28385 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28958 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR), STRING_LITERAL)'. You might need to add explicit type casts. |
| 27114 | BinderException: Binder Error: UNNEST not supported here |
| 29151 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 26854 | BinderException: Binder Error: UNNEST not supported here |
| 26828 | BinderException: Binder Error: UNNEST not supported here |
| 9926 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27062 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 27023 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 28686 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 30565 | BinderException: Binder Error: column "LastActivityDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 31890 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 6614 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 9000 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 22371 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27754 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25111 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2487 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 25162 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 31986 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25570 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR), STRING_LITERAL)'. You might need to add explicit type casts. |
| 25939 | BinderException: Binder Error: column "Tags" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28227 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26552 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 27896 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 5058 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 34479 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25408 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26947 | BinderException: Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4084 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 23848 | ConversionException: Conversion Error: Type VARCHAR with value '{}' can't be cast to the destination type VARCHAR[] |
| 27391 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 20195 | BinderException: Binder Error: Join condition for non-inner LATERAL JOIN must be a comparison between the left and right side |
| 25859 | BinderException: Binder Error: UNNEST not supported here |
| 20450 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 8405 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 26048 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 4094 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR), STRING_LITERAL)'. You might need to add explicit type casts. |
| 7506 | BinderException: Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29685 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21421 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR), STRING_LITERAL)'. You might need to add explicit type casts. |
| 28195 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 34576 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR), STRING_LITERAL)'. You might need to add explicit type casts. |
| 28309 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 29197 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 32526 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 3874 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 33375 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 558 | BinderException: Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22558 | BinderException: Binder Error: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 34639 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 33891 | BinderException: Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22290 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 21521 | BinderException: Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 5675 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 8177 | ConversionException: Conversion Error: Type VARCHAR with value 'mysql' can't be cast to the destination type STRUCT(unnest VARCHAR) when casting from source column TagName |
| 24666 | BinderException: Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24122 | BinderException: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 32230 | BinderException: UNNEST not supported here |
| 33679 | BinderException: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23140 | BinderException: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26867 | BinderException: UNNEST not supported here |
| 24468 | BinderException: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24699 | BinderException: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22856 | BinderException: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20182 | NotImplementedException: Cannot perform non-inner join on subquery! |
| 24604 | BinderException: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 21174 | NotImplementedException: Cannot perform non-inner join on subquery! |
| 6896 | Binder Error: UNNEST not supported here |
| 29875 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3834 | Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 4659 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28499 | Binder Error: UNNEST not supported here |
| 26911 | Binder Error: UNNEST not supported here |
| 2967 | Binder Error: UNNEST not supported here |
| 27153 | Binder Error: UNNEST not supported here |
| 2453 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28943 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27708 | Binder Error: UNNEST not supported here |
| 29161 | Binder Error: UNNEST not supported here |
| 24742 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 25728 | Binder Error: UNNEST not supported here |
| 27077 | Binder Error: UNNEST not supported here |
| 23217 | Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 22163 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26151 | Binder Error: UNNEST not supported here |
| 28198 | Binder Error: UNNEST not supported here |
| 27264 | Binder Error: UNNEST not supported here |
| 24744 | Not implemented Error: Cannot perform non-inner join on subquery! |
| 34437 | Binder Error: UNNEST not supported here |
| 23793 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 23281 | Binder Error: column "AcceptedAnswerId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24424 | Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3011 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20975 | Binder Error: column "Reputation" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 24526 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20978 | Binder Error: UNNEST not supported here |
| 22786 | Binder Error: column "Score" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 3226 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 20998 | Binder Error: column "OwnerUserId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 28037 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2301 | Binder Error: column "DisplayName" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27999 | Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR))'. You might need to add explicit type casts. |
| 28303 | Binder Error: No function matches the given name and argument types 'trim(STRUCT(unnest VARCHAR), STRING_LITERAL)'. You might need to add explicit type casts. |
| 27635 | Binder Error: column "CreationDate" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 27886 | Binder Error: column "PostTypeId" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 29768 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 26194 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 2184 | Binder Error: UNNEST not supported here |
| 34092 | Binder Error: Cannot compare values of type VARCHAR and STRUCT(unnest VARCHAR) in IN/ANY/ALL clause - an explicit cast is required |
| 26680 | Binder Error: column "ViewCount" must appear in the GROUP BY clause or must be part of an aggregate function. |
| 30192 | Binder Error: column "Title" must appear in the GROUP BY clause or must be part of an aggregate function. |

The rows from 9281 to 8587 were offered by the window-function batches
(39-63): 25 of them. 17 put a column in a window's `PARTITION BY` or
`ORDER BY` (or the select list) that is missing from the `GROUP BY`, 6 use
`UNNEST` where DuckDB does not allow it, and 2 are other binder errors.
11573 came off the old blocked list.

19976 and 18269 got through because `tools/select.py` misread `valid_queries.csv`. Its
`systems` column is not a flat list of engines that ran the query — it is a
PARTITION of the engines into groups that agreed on the result:

    3.sql      [["umbra", "duckdb", "postgres"]]      all three agreed
    19976.sql  [["postgres", "umbra"], ["duckdb"]]    duckdb disagreed

and disagreement includes refusing the query. The filter tested `"duckdb" in
r["systems"]` as a substring, which accepts both. Fixed in `select.py`: the
column is parsed as JSON, and each query records how many engines matched
duckdb (`agree`) and whether the engines split at all (`split`).

**Fixed in both the code and the data.** For a long time the fix was only in
`select.py`: `index.json` had never been regenerated after it, so not one of
its 15,914 entries carried an `agree` or a `split` key, `next.py`'s
`e.get("split")` was always false, and the `[SPLIT]` mark had never once been
printed. Batches 00-63 were all ported without the screen.

`select.py` has now been re-run and `index.json` rebuilt, so every entry
carries both keys and `[SPLIT]` prints:

    python3 -c "import json; i=json.load(open('.../index.json'));
                print(sum('split' in e for e in i))"    # 15914

The regenerated data gives 2,285 split and 1,923 duckdb-alone of the 10,222
duckdb-valid queries, the same figures this file already quoted from
`valid_queries.csv` — they are now reaching the tool. Note that the 723
already-ported queries were chosen before the screen worked, so they remain
unscreened; 18269 was caught by the oracle refusing to run, not by the screen.

## No oracle

DuckDB could not produce an answer, so there is nothing to compare a port
against.

| query | why |
| ----- | --- |
| 5986 | out of memory after 70 GB of temp files: `JOIN ActiveBadges ab ON rp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = ab.UserId)` is a correlated subquery in a join condition |
| 11174 | did not finish: killed after 90 s in the foreground, and a background run started in the same session never finished either |
| 3352 | TIMEOUT after 300s (also after 90s): the JOIN ON tu.UserId = (correlated LIMIT 1 subquery) is evaluated per row |
| 26304 | TIMEOUT after 90s; at 300s DuckDB ran out of memory (28.5 GiB): `JOIN Posts p ON t.Id = p.Id JOIN Badges b ... JOIN Users u` per tag, then `pt.TagName = ANY(STRING_TO_ARRAY(rp.Tags, ', '))` |
| 24379 | TIMEOUT after 90s in tools/prep.py |
| 31911 | Postgres-oracle query (`notes/pg-oracle.md`): Postgres times out after 600 s, and DuckDB after 60 s |

## Engine disagreement predicts the ambiguous queries

Of the 10,222 duckdb-validated queries, 2,285 have the engines splitting into
more than one group, and 1,923 have duckdb alone against the other two. A
split means the query has more than one defensible answer — nearly always a
`LIMIT` over an `ORDER BY` that is not a total order, where each engine keeps
different tied rows.

This is a far better determinism screen than checking each batch against
DuckDB by hand — once `index.json` carries the flags (see above; it does
not yet). Of the 360 queries ported through batch 31, 15 are flagged as split,
and **all four that needed a `rewrites/` entry are among those 15** — no
rewrite was needed for any query the engines agreed on. `tools/next.py` now
prints `[SPLIT]` so the tie check happens before the port rather than after a
DIFF.

## The corpus is templates, not queries

Positions 11–40 are thirty spellings of ONE shape:

    SELECT <some of p.Id, p.Title, p.Score, p.CreationDate, p.Body, u.DisplayName>
    FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
    WHERE p.PostTypeId = 1
    ORDER BY <p.Score | p.CreationDate> DESC LIMIT 10

The text-level dedup in `tools/select.py` cannot see this: the queries differ
in alias case (`p` vs `P`), join direction, projection order, and trailing
whitespace. `select.py` now also computes a SHAPE key from DuckDB's parse
tree with the cosmetics stripped (aliases, source positions, column
qualifiers) and the projection list sorted.

That collapses less than hoped — 10,222 DuckDB-valid queries have 9,483
distinct shapes, only 7% — because these thirty do genuinely differ: they
project different SETS of columns, not just different orders. But the shape
key still earns its place, because the worklist is now ordered by shape, so
a batch is one relational plan with ten formatters rather than ten unrelated
queries. `tools/next.py` puts the ids that share a shape with something
already ported at the front for the same reason.

The practical consequence: the low-difficulty tier is cheap per query but
not free, and the per-query cost is the projection, not the plan.

## Features ahead, from the corpus-wide scan

Share of the 18,251 queries using each. Verdicts are one word here; the
detail is in `limitations.md`.

| feature                              | share              | status                                                    |
| ------------------------------------ | ------------------ | --------------------------------------------------------- |
| ORDER BY / GROUP BY / LIMIT / HAVING | 99 / 90 / 48 / 5 % | ok — sorting and top-N are host Rust                      |
| LEFT / OUTER JOIN                    | 87 %               | ok — `get` to project, `dense_fold_outer` to aggregate    |
| CTE / subquery                       | 71 / 70 %          | ok when it just names a sub-plan; correlated ones untried |
| CASE                                 | 62 %               | ok — `.map`, or a branch in the fold                      |
| window functions                     | 54 %               | ok — `.window(row_number/rank/..., order, cmp)`           |
| NULL handling                        | ~46 % use COALESCE | ok — absence is the null                                  |
| INTERVAL / DATE_TRUNC / EXTRACT      | 38 %               | ok — `harness::time`                                      |
| DISTINCT                             | 37 %               | partial — only per group                                  |
| STRING_AGG / ARRAY_AGG               | 15 %               | awkward — `buf_fold` to a leaked `&'static str` / slice   |
| LATERAL / UNNEST                     | 8 %                | ok for tags, pre-exploded; general unnest untried         |
| UNION / EXCEPT / INTERSECT           | 1.4 %              | ok — `union` / `minus`                                    |

## Order-by ties at the LIMIT cut

If two rows tie on the sort key and the LIMIT falls between them, the query
has no single answer:

    Posts(Id, Title, Score): (1,'a',5) (2,'b',3) (3,'c',3)
    SELECT Title FROM Posts ORDER BY Score DESC LIMIT 2;
    -- 'a', then EITHER 'b' OR 'c'. Both are correct answers.

A tie only matters when it straddles the cut AND the tied rows differ in a
column the query projects. 17061 ties three ways at positions 9-11, but those
three rows are identical in every projected column, so which two survive
cannot be observed. 17116 and 16100 tie on rows that differ in `PHT.Name`,
which they do project — those needed a `rewrites/` entry.

17445 is another unobservable one. Four rows tie at the cut, all four the
same question joined to four of its comments, and DuckDB keeps two:

    SELECT P.Title, P.CreationDate, U.DisplayName, C.Score ...
    ORDER BY P.CreationDate DESC LIMIT 10

    SELECT P.Id, C.Id, C.Score FROM Posts P ... WHERE
      P.CreationDate = TIMESTAMP '2024-09-30 14:29:59.927'
    342699 | 666024 | 0
    342699 | 666027 | 0
    342699 | 666028 | 0
    342699 | 666031 | 0

`C.Id` is not projected and every projected column agrees, so which two
survive cannot be observed. No rewrite.

Batches 96-105 added six more, each a tie at the cut between rows of one post
that differ in a projected child column, each fixed by extending the ORDER BY
to a total order on the ids: 16744 and 17299 (`, p.Id, t.TagName`), 10486
(`, p.Id, t.TagName`), 12544 (`, p.Id, ph.Id`), 12928 (`, p.Id, vt.Name`), and
13037, whose correlated `ORDER BY B.Date DESC LIMIT 1` picks among badges
earned at the same instant (`, B.Id`). Three more are float `AVG`s whose
printed digits move with `SET threads` (12732, 10694, 13410); like 5603 they
are rewritten to the exact-integer mean `SUM(micros)::DOUBLE / COUNT(..) / 1e6`.

Batches 106-113 added five float means of the same kind (14185, 10023, 13043,
11058, and 10428, whose `AVG(COALESCE(EXTRACT(EPOCH FROM ..), 0))` runs over
the joined rows and so becomes `SUM(..)::DOUBLE / COUNT(*) / 1e6`), and four
ties at the cut where every tied row has a NULL `ViewCount` or `TotalViews`
and the rows differ in the id: 11962 (`, PostId`), 10318 and 10418
(`, ps.PostId`), 10254 (`, UPS.UserId`).

Batches 114-123 added four ties at the cut between rows that share the sort
key and differ in the id: 11373 (one post, several PostHistory rows,
`, ph.Id`), 12370 (one post grouped by several history types,
`, PS.PostHistoryTypeId`), 12717 (`, p.Id`) and 12920 (`, ps.PostId`); and one
float mean, 11727, whose `AVG(EXTRACT(EPOCH FROM P.CreationDate))` becomes
`SUM(epoch_us(P.CreationDate))::DOUBLE / COUNT(P.CreationDate) / 1e6`.

Batches 124-140 added eleven ties at the cut, each fixed by extending the ORDER
BY to the ids: 14019 (`, PS.PostId`), 14776 (`, p.PostId`), 25231 (inside the
`PopularPosts` CTE, `, p.Id`), 13255, 13658, 10762, 7667 (`, PostId`), 20652
(`, F.UserId, F.CloseReasonCount`), 8140 (the name join repeats a post:
`, PostId, BadgeCount, HighestBadgeClass`), and 13186 (several history groups of
one post share its `LastActivityDate`: `, US.UserId, PA.PostId, PA.HistoryDate,
PA.PostHistoryTypeId, PA.UserDisplayName`). Eleven more are float means of
epoch seconds rewritten to `SUM(epoch_us(..))::DOUBLE / COUNT(..) / 1e6`
(divided further where the query divides): 13946, 13355, 10137, 10766, 10226,
14369, 14795, 10006, 3381, 25142, 29019.

Do not hunt these by hand: `valid_queries.csv` already flags them, because a
query with more than one answer is exactly one the engines disagree on. See
"Engine disagreement predicts the ambiguous queries" above.

## Parts of a query left out of the port

Pieces that cannot change the result, so the port does not compute them:
11887 (`LEFT JOIN VoteTypes vt ON vt.Id = (correlated LIMIT 1)` matches at most
one row and `vt` is never read), 9025 (`LEFT JOIN Posts a ON p.Id =
a.AcceptedAnswerId` only adds a GROUP BY key that is constant per question),
10719, 13449 (a CTE the final SELECT never references), 10992 (a
`CURRENT_TIMESTAMP` column that is not in the output), 11391 (a `LEFT JOIN` of a
one-row-per-user CTE that no aggregate reads).

## Other queries left unported

None open. Every query this section used to list (26548, 4710, 16303, 14888,
10806, 14171, 14612, 6006, 9648, 27717, 27183, 29656, 3076, 3760, and the
9812 ... 21342 slice) has since been ported; the port's comment says how.
Queries that cannot be ported go in the tables above, so `next.py` skips them.

## Host-Rust tricks audit (c00-c67)

After batch 159 every crate was audited for filters, joins and groupings done
in host Rust. 140 ports in c00-c67 were fixed; all still match DuckDB and none
had to be blocked:

- 49 (c01-c03) joined two aggregates by hand with `find`/`binary_search`; now
  one fold, or `.and((&distinct).opt())`.
- 29 ran a WHERE/HAVING as `v.into_iter().filter(..)`/`retain` on drained rows;
  12 as an `if` around `push` inside `.drive`. Now `.filt`/`.with`, or
  `drain(rel(v).filt(..))` after a host rank.
- 7 wrote a LEFT JOIN as a `probe` plus an `any` flag; now `.opt()`.
- 46 crossed in host Rust: 34 through `kit::lex_top` (a nested loop), now a
  local `cross_top` that drives `.cross` (narrowing the leading side in prela
  to the rows that can reach the LIMIT); 12 more nested loops, now `.cross`
  or `.cross(left_all(..))`, including 10046.
- A handful of other hand joins: 6959, 26210 (probe inside drive), 29079,
  22681 (tag string split in host), 2254 (`contains`), 11701, 12132, 12643,
  13923 (`.get` of a one-row side), 10460 (a query per partition in a loop),
  7007, 11576.

Batches 149-181 were checked the same way: nine post-rank WHEREs (30151,
13195, 9950, 9369, 5934, 9663, 7969, 4970, 3054) and seven `lex_top` calls
(9732, 30657, 4654, 4262, 2525, 3741, 29984) were moved into prela. `lex_top`
is no longer called anywhere.

Left as they are: about 115 `.get(key)` probes, while formatting a row, of a
fold keyed by that row's own id. Like projecting a nullable column, such a
probe cannot drop or duplicate a row.

## Ports to redo: counts multiplied instead of driving the product

Done. Batches 134-140 (crates c55-c61) were written before the no-tricks rule,
and many of their ports computed a multi-`LEFT JOIN` aggregate by multiplying
per-child counts (`a + x * c.max(1)`) instead of driving the product. All 122
listed ports, plus 12 unlisted ones with the same trick (11158, 13361, 9632,
11632, 13186, 9744, 5134, 7673, 14666, 30104, 8635, and the shared
`named_qa`/`pnq` helpers), now fold over `.opt().and(...)` products. Seven
ports that filtered in a host `Vec` (9972, 6605, 967, 24889, 2360, 8239, 3112)
now filter in prela. None had to be blocked; the slowest, 8635, takes 27 s.

Left as they were, because the arithmetic is the query's own: 2074, 5861,
4768, 6212, 24824, 20370, 6275 (`r * n > sum` is `x > AVG(x)` compared
exactly) and 9396 (the SQL's own score formula).
