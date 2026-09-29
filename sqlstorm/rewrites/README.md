# Rewrites — queries made deterministic

A query with `LIMIT` and an `ORDER BY` that is not a total order has no single
right answer, and so does a `ROW_NUMBER()`/`RANK()` whose window `ORDER BY` is
not total — there the tie is observable without any `LIMIT`, because the rank
itself is projected: DuckDB picks arbitrarily among the rows tied at the cut, and so
would any other engine. Comparing a prela port against one arbitrary choice
tests nothing.

A file here replaces `corpus/queries/<id>.sql` when `tools/oracle.py` builds
the oracle. It is the same query with the ordering refined until it is total,
usually by appending a unique id to the `ORDER BY`. The port mirrors the same
tiebreak. Since every total refinement of the original ordering is an equally
valid answer to the original query, this tests the port on a question that
actually has an answer, rather than on DuckDB's coin flip.

Only add a file here when a tie genuinely straddles the `LIMIT` boundary AND
the tied rows differ in a projected column. Most ties are harmless: the tied
rows are identical in everything the query selects, so which one survives
cannot be observed. `notes/blocked.md` records those.

| query | why |
|---|---|
| 17116 | `ORDER BY PH.CreationDate DESC LIMIT 10`; rows 9-11 share an instant and differ in PHT.Name (Initial Tags / Initial Title / Initial Body) |
| 16100 | same query, columns in a different order |
| 16616 | `ORDER BY CommentCount DESC LIMIT 10`; three groups tie at the cut and differ in Title and DisplayName |
| 18612 | same, four groups tie; grouped by DisplayName, Title, CreationDate |
| 11478 | no LIMIT: the tie is inside the window. `ROW_NUMBER() OVER (ORDER BY TotalPosts DESC)` over six post types, where TagWiki and TagWikiExcerpt both have 857 posts, so ranks 3 and 4 are a coin flip and both rows are projected. Refined with `, PostType` |
| 14451 | `ORDER BY PS.Score DESC, PS.ViewCount DESC LIMIT 100`; five posts tie at rank 90 (score 8, no ViewCount) across the cut and differ in every column. Refined with `, PS.PostId` |
| 19637, 15835, 16507, 19034, 15017, 19692, 15136, 15838, 16290, 19002, 19176, 19100 | `JOIN Tags t ON p.Tags LIKE '%' \|\| t.TagName \|\| '%' ... ORDER BY p.<key> DESC LIMIT 10`; one post matches several tags, so the cut falls inside one post's rows and which of its tags survive is a coin flip. Refined with `, p.Id, t.TagName` |
| 6432 | no LIMIT: `ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) <= 10` cuts through a score tie in two post types, so which tied posts survive is a coin flip. Refined with `, p.Id` |
| 19422 | `ORDER BY P.CreationDate DESC LIMIT 10`; one question's four comments straddle the cut and `C.Text` is projected. Refined with `, C.Id` |
| 18677 | `ORDER BY CommentCount DESC LIMIT 10`; four groups tie at 22 across the cut and differ in Title and DisplayName. Refined with `, u.Id, p.Title, p.CreationDate` |
| 17276, 17919 | same, four groups tie at 25; the key has a NULL Title, so refined with `, u.DisplayName, p.CreationDate, p.Title NULLS FIRST` |
| 5603 | not a tie: `AVG(EXTRACT(EPOCH ...))` over 217 groups, where one group's double lands an ulp off a 6-decimal midpoint and DuckDB's own answer flips with `SET threads`. Replaced with the exact-integer mean `SUM(micros)::DOUBLE / COUNT(*) / 1e6`. See `notes/translation-failures.md` 10 |
| 33753 | `STRING_AGG(rb.BadgeName, ', ')` over each user's three latest badges has no `ORDER BY`, so the list order is arbitrary, and `ROW_NUMBER() OVER (... ORDER BY b.Date DESC)` ties on badges awarded at the same instant. Refined with `ORDER BY rb.rn` inside the aggregate and `, b.Id` in the window |
| 10082 | `ORDER BY p.Score DESC, p.ViewCount DESC LIMIT 100`; rows 99-103 are all (Score 10, ViewCount NULL) and are different posts. Refined with `p.Id` |
| 10206 | both `ROW_NUMBER()`s are projected and neither window `ORDER BY` is total: the ViewRank printed for the NULL-ViewCount answers was one of 105k arbitrary choices. Refined with `PostId` in both windows |
| 20479 | `ROW_NUMBER() OVER (PARTITION BY COALESCE(AcceptedAnswerId,-1) ORDER BY Score DESC)` then `WHERE Rank = 1`: the -1 partition holds thousands of posts and the order is not total. Refined with `pd.PostId` |
| 2020 | `STRING_AGG(DISTINCT t.TagName, ', ')` in a correlated subquery, no `ORDER BY`. Added `ORDER BY t.TagName` |
| 22681 | three at once: `ARRAY_AGG(DISTINCT pht.Name)` unordered, a `DENSE_RANK` whose window order is not total, and a final `ORDER BY` under `OFFSET 10 FETCH 20` that is not total. Added `ORDER BY pht.Name`, `p.Id`, and `ps.PostId, ps.TagName` |
| 25191 | `STRING_AGG(T.TagName, ', ')` over a `CROSS JOIN LATERAL UNNEST`, no `ORDER BY`. DuckDB is stable across thread counts here but emits the tags in *reverse* array order, which the SQL does not ask for. Added `ORDER BY T.TagName` |
| 16303 | `STRING_AGG(TagName, ', ')` over all of Tags, no `ORDER BY` (the subquery's `PostId` binds to the neighbouring `C.PostId`, so DuckDB runs it as a LATERAL). Added `ORDER BY Id`; the oracle did not change |
| 3339 | `ORDER BY ups.AvgScore DESC, ups.TotalPosts DESC LIMIT 5` cuts inside one user's ten PopularQuestions rows, which differ in the question. Added `ups.UserId, pq.Score DESC, pq.Id` (and `p.Id` to PopularQuestions' own `LIMIT 10`); the oracle did not change |
| 8762 | `ORDER BY AD.TotalUserUpvotes DESC, PD.ViewCount DESC LIMIT 100` ties across one post's edits; DuckDB's answer moved with `SET threads`. Added `U.Id, PD.PostId, PD.LastActionDate, PD.LastActionComment, PD.PostHistoryTypeId` |
| 11708 | `ORDER BY pi.Score DESC, pi.ViewCount DESC LIMIT 100`: posts without views tie; thread-unstable. Added `pi.PostId` |

## How often this matters at the hard end

Of 40 unported queries sampled at difficulty 15 and above, 21 do not run on
DuckDB at all (see the invalid table in `notes/blocked.md`). 19 run; 18 of
those were checked at `SET threads` 1, 2, 3, 4 and 8, and **13 of the 18 gave
different answers** — almost always an unordered `STRING_AGG`/`ARRAY_AGG` or a
`ROW_NUMBER` whose window `ORDER BY` is not total. Two of the 13 (20479, and
25191 in a different way) looked stable at 1-vs-8 and only broke at 4 threads
or against the array order, so the check needs more than two settings.
Running

    SET threads=1;  -- vs 8, hash the sorted rows

before writing the port is worth more at this end of the corpus than the
tie-at-the-LIMIT check is at the easy end.
| 10835 | no LIMIT: `ROW_NUMBER() OVER (ORDER BY TotalViews DESC)` over all users, most of whom tie at 0 views, and the projected ViewRank of the top scorers falls in those ties. Both windows refined with `, UserId` |
| 2254 | `ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC)` labels one post per type 'Top Post', and several posts tie on the top score; the final `ORDER BY NetScore DESC, rp.CreationDate DESC` also ties at the cut. Refined with `, p.Id` and `, rp.PostId` |
| 29442 | `LIMIT 10` with no `ORDER BY` in the outer query: the `ORDER BY r.UpVotes DESC, r.CommentCount DESC` sits in the CTE, where SQL does not keep it, so which ten rows come back is up to the engine. Moved to the outer query and refined with `, fp.CreationDate` |
| 9451 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) <= 10` over the last 90 days' posts: several posts tie on the score at the tenth place and differ in every projected column. Refined with `, p.Id` |
| 5382 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) <= 10`: recent answers tie on score with no view count at the tenth place, so which answer makes the cut moves with `SET threads`. Refined with `, p.Id`, and the final order with `, tq.PostId` |
| 6732 | `LIMIT 10` with no `ORDER BY` in the outer query: the `ORDER BY tu.Reputation DESC` sits in the FinalStats CTE, where SQL does not keep it. Moved to the outer query and refined with `, UserId` |
| 8514 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) <= 5` over the last three years' positive posts: posts tie on the score at the fifth place and differ in every projected column, so the answer moves with `SET threads`. Refined with `, p.Id` |
| 12550 | `AVG(EXTRACT(EPOCH FROM P.CreationDate))` per user: the printed digits move with `SET threads`. Rewritten to the exact-integer mean `SUM(epoch_us(P.CreationDate))::DOUBLE / COUNT(P.CreationDate) / 1e6` |
| 5214 | no LIMIT: `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY COUNT(c.Id) DESC, SUM(v.VoteTypeId) DESC) <= 5` cuts through a tie, and DuckDB's answer changes with `SET threads`. Refined with `, p.Id` |
| 8164 | `STRING_AGG(rp.Title, '; ')` has no `ORDER BY`, so the list order is arbitrary. Added `ORDER BY rp.PostId` |
| 28282 | no LIMIT: `ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC, p.ViewCount DESC) <= 5` cuts through ties within a tag list, and Title/Body are projected; DuckDB answer moved with threads. Refined with `, p.Id` |
| 28777 | `AVG(EXTRACT(EPOCH FROM (t0 - p.CreationDate)) / 3600)` per user moves with `SET threads`. Rewritten to the exact-integer mean `SUM(epoch_us(t0) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.CreationDate) / 1e6 / 3600` |
| 8409 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) <= 5`: posts of one owner (the ownerless partition) share a creation instant at the fifth place, so which survive moves with `SET threads`. Refined with `, p.Id` |
| 27505 | `LIMIT 10` with no `ORDER BY` in the outer query: the `ORDER BY NetVoteCount DESC` sits in the FilteredPosts CTE, where SQL does not keep it. Moved to the outer query and refined with `, fp.PostId` |
| 6087 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC)` ties within an owner and the final `ORDER BY pvs.UpVotes DESC, pvs.TotalVotes DESC LIMIT 10` ties at the cut (UNSTABLE across threads); both get `, p.Id` / `, bp.PostId` |
| 4860 | tie at the `LIMIT 10` between posts of score 10 (341207, 337299) that differ in every projected column: `, fr.PostId` |
| 9911 | `ORDER BY TotalQuestions DESC, TotalComments DESC LIMIT 10`: every user has TotalQuestions = 1 and several tie on TotalComments at the cut, so which users come back moves with `SET threads`. Refined with `, u.Id` |
| 7920 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) <= 5`: posts of one owner (and the ownerless ones) tie on score and date at the fifth place, so which make the cut moves with `SET threads`. Refined with `, p.Id` |
| 3862 | `ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC)` then `PostRank = 1`: owners with several top-scoring questions pick one arbitrarily (UNSTABLE across threads), and the final `ORDER BY U.Reputation DESC, U.QuestionCount DESC LIMIT 50` is not total. Refined with `, P.Id` and `, U.UserId` |
| 9698 | no LIMIT: the tie is inside the window. `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) <= 5` cuts through owners whose questions tie on ViewCount, so which titles are projected moves with `SET threads`. Refined with `, p.Id` |
| 4565 | no LIMIT inside the window: `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) <= 10` cuts through owners whose questions tie on the score at tenth place, and the tied posts differ in every projected column. Refined with `, p.Id` |
| 2655 | `AVG(EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - ph.CreationDate)))` per (UserId, Comment): the float mean's last digit moves with `SET threads`. Rewritten to the exact-integer mean `SUM(epoch_us(..) - epoch_us(ph.CreationDate))::DOUBLE / COUNT(ph.CreationDate) / 1e6` |
| 3822 | `ORDER BY PS.Score DESC, PS.ViewCount DESC LIMIT 50`: recent answers (no view count) tie on score at the cut and the answer moves with `SET threads`; the name join can also repeat a post. Refined with `, PS.PostId, U.UserId` |
| 26510 | no LIMIT: `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) <= 5` cuts through an owner's questions tied on score and views, which differ in Title and Tags; the answer moved with `SET threads`. Refined with `, p.Id` |
| 8495 | `AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)))` per user over the posts x votes rows: the printed digits move with `SET threads`. Rewritten to the exact-integer mean `SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.CreationDate) / 1e6` |
| 8350 | `LIMIT 10` after `LEFT JOIN Votes v`: the 52 vote rows of post 339077 tie on `Score, CommentCount` at the cut and differ in `MostRecentVoteType` (51 UpMod, 1 AcceptedByOriginator). Refined with `, pp.PostId, v.Id` |
| 3782 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) = 1`: two posts of one user share a CreationDate and differ in the projected PostId. Refined with `, p.Id` |
| 7563 | `ORDER BY Reputation DESC, Score DESC LIMIT 10`: one user's questions tie on Score at the cut and differ in Title, so the answer moves with `SET threads`. PP.Id is carried out of CombinedData as PostId and the order refined with `, PostId` |
| 9646 | `ORDER BY NetVotes DESC, TP.CommentCount DESC LIMIT 10`: three newest-per-owner posts tie at 20 net votes and 20 comments across the cut, and differ in Title and owner. Refined with `, TP.PostId` |
| 3088 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC)` decides the projected PostCategory (`Rank <= 5`), and posts tie on the view count (and on NULL) around the fifth place, so the category moves with `SET threads`. Refined with `, p.Id` |
| 2878 | `ORDER BY tp.Score DESC, tp.Reputation DESC LIMIT 10`: two posts of one owner tie on score 17 and reputation at the tenth place and differ in Title/PostId, so the answer moves with `SET threads`. Refined with `, tp.PostId` |
| 2004 | `ORDER BY tp.NetVotes DESC LIMIT 10`: one post is joined to each of its owner's gold badges, all tied on NetVotes across the cut, and the rows differ in UserBadge. Refined with `, tp.PostId, UserBadge` |
| 2757 | `ORDER BY UserRank LIMIT 100`: users tie on (TotalPosts, TotalViews) and so on RANK across the cut, and differ in every projected column. Refined with `, UserId` |
| 1251 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) <= 10`: posts tie on ViewCount (and NULLs) at the tenth place, and which ones make the cut decides the `CommentCount > 5` survivors, so the answer moves with `SET threads`. Refined with `, p.Id` |
| 25228 | `ORDER BY NetVoteScore DESC LIMIT 50`: hundreds of posts tie on NetVoteScore at the cut and differ in every projected column, so the answer moves with `SET threads`. Refined with `, fp.PostId` |
| 6630 | `ORDER BY ScoreRank, TotalScore DESC LIMIT 50`: users tie on TotalScore (and so on the rank) at the cut and differ in every projected column, so the answer moves with `SET threads`. Refined with `, UserId` |
| 5468 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY <net votes> DESC) <= 5` over the last 30 days: three questions tie on net votes across ranks 4-6 and differ in every projected column, so the answer moves with `SET threads`. Refined with `, p.Id` |
| 8074 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) = 1`: user 2639 has a question and an answer with the same score posted at the same instant, and they differ in the projected Title, so the answer moves with `SET threads`. Refined with `, p.Id` |
| 32784 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) = 1`: owners with several posts at the same top score (often 0) have no single first post, and the tied posts differ in every projected column, so the answer moves with `SET threads`. Refined with `, p.Id`, and the `HistoryRank` window with `, ph.Id` |
| 2811 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) = 1`: 14 owners posted a question and its answer at the same instant, so their first post is either one; one of them has a vote balance of 10, equal to the tenth row at the `LIMIT 10`, and the answer moves with `SET threads`. Refined with `, p.Id` |
| 30218 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) = 1` over the last 30 days: user 1186 posted a question and its answer at the same instant, and PostId and Title are projected, so the row has two right answers. Refined with `, p.Id` |
| 2697 | `ROW_NUMBER() OVER (PARTITION BY CAST(p.CreationDate AS DATE) ORDER BY p.Score DESC) <= 5` feeds a per-owner COUNT: questions of one day tie on the score at fifth place, so QuestionsCount moves with `SET threads`. Refined with `, p.Id` |
| 2513 | `NTILE(5) OVER (ORDER BY Reputation)` puts users tied on reputation in whichever tier the scan order gives, and ReputationTier is projected; TopPosts' `ORDER BY NetScore DESC LIMIT 10` ties at the cut too, so the answer moves with `SET threads`. Refined the NTILE order with `, Id` and TopPosts with `, PS.Id` |
| 1820 | `ORDER BY NetVotes DESC LIMIT 10` inside the TopUsers CTE: users tie on NetVotes at the cut, and which of them get TotalBounties/NetVotes moves with `SET threads`. Refined with `, U.Id` |
| 253 | `ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC)` feeds the projected RecentPostIndicator: some owners have two posts created at the same instant, so which is "Most Recent Post" moves with `SET threads`. Refined with `, P.Id` |
| 2980 | `ORDER BY FPS.TotalScore DESC LIMIT 10`: posts tie on TotalScore at the tenth place and differ in every projected column, so the answer moved with `SET threads`. Refined with `, FPS.PostId` |
| 3332 | `ROW_NUMBER() OVER (ORDER BY u.Reputation DESC)` is projected and numbers users tied on reputation arbitrarily, and the `LIMIT 10` cuts through one post's gold-badge rows, which differ in BadgeId/BadgeName; the answer moved with `SET threads`. Refined the window with `, u.Id` and the final order with `, pr.PostId, bg.Id` |
| 1143 | `ORDER BY ub.GoldBadges DESC, rp.Score DESC LIMIT 10`: users tie on GoldBadges at the tenth place (all with a NULL rp), and differ in every projected column; the answer moved with `SET threads`. Refined with `, ub.UserId, rp.PostId` |
| 8319 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) <= 5`: one owner's questions tie on score and view count at the fifth place and differ in PostId and Title. Refined with `, p.Id` |
| 1050 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC)` numbers the post x vote x comment rows, and which row of an owner is Rank 1 (projected as PostRank) is arbitrary; the `LIMIT 50` also cuts through identical-key rows; the answer moved with `SET threads`. Refined the window with `, p.Id, v.Id, c.Id` and the final order with `, rp.PostId, rp.Rank` |
| 9306 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) <= 10`: five answers tie on score 17 with no view count across the tenth place and differ in the projected owner. Refined with `, p.Id` |
| 1622 | `ORDER BY FR.AvgUpVotes DESC, FR.CommentCount DESC FETCH FIRST 100 ROWS ONLY`: many questions tie at AvgUpVotes 1.0 with the same comment count across the cut and differ in every projected column, so the answer moves with `SET threads`. Refined with `, FR.QuestionId` |
| 1263 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) = 1`: a user's posts tie on the top score and differ in the projected Title and CommentCount, so the answer moves with `SET threads`. Refined with `, p.Id` |
| 34854 | projected `ROW_NUMBER() OVER (ORDER BY fr.TotalBounty DESC, fr.Score DESC)`: posts tie on bounty 0 and score, so FinalRank moves with `SET threads`. FinalResults now carries fp.PostId and the window is refined with `, fr.PostId` |
| 189 | `ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) = 1`: two of one user's recent posts share a CreationDate and differ in the projected PostId. Refined with `, P.Id` |
| 33887 | the `ah` join repeats each post once per question its owner asked, so copies of one post tie in `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC)` and at the final `LIMIT 50` while differing in AnswerStatus; the answer moved with `SET threads`. Refined the window with `, p.Id, ah.AcceptedAnswerId` and the final order with `, ps.PostId, AnswerStatus` |
| 2377 | no LIMIT inside the window: `ROW_NUMBER() OVER (PARTITION BY <status> ORDER BY TotalScore DESC)` ties on TotalScore, and StatusRank is projected and cut at `LIMIT 100`; the answer moved with `SET threads`. Refined with `, UserId` |
| 32545 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) = 1`: a self-answered question and its answer (341207, 341208) share owner and CreationDate, so which is Rank 1 moves with `SET threads`. Refined with `, p.Id` |
| 4705 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC) = 1`: posts of one owner share a LastActivityDate, so which is ActivityRank 1 moves with `SET threads` (81 vs 90 rows). Refined with `, p.Id` |
| 5383 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) <= 5`: posts tie on score and views at the fifth place and differ in every projected column, so the answer moves with `SET threads`. Refined with `, p.Id` |
| 30253 | `ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) = 1`: several history rows of a post share its latest date and differ in UserId, which feeds the projected ReputationRank. Refined with `, ph.Id` |
| 2894 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) <= 10` over the last year's posts: posts tie on the score at the tenth place and differ in every projected column (Rank is projected), so which one makes the cut is arbitrary. Refined with `, p.Id` |
| 8453 | no LIMIT: `ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) <= 10` over the last year's posts cuts through posts tied on the score, which differ in every projected column; DuckDB's answer moved with `SET threads`. Refined with `, p.Id` |
| 25687 | `ORDER BY AvgScore DESC, AvgViewCount DESC LIMIT 50`: the sort keys are per-tag aggregates, so every row of one tag ties and the cut falls among posts of one tag; DuckDB's answer moved with `SET threads`. Refined with `, PostId, tag` |
| 21765 | `FETCH FIRST 100` after `ORDER BY Reputation DESC, UpVoteCount DESC`, where Reputation is a CASE text: two users' answers tie at ("High Reputation", 12) across the cut and differ in DisplayName. R.AnswerId carried into CombinedData, refined with `, AnswerId, LinkTypeDescription` |
| 14246 | `AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)))` per user over the posts x votes rows: the printed digits move with `SET threads`. Rewritten to the exact-integer mean `SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.Id) / 1e6` |
| 637 | `ORDER BY PS.AvgScore DESC, PS.TotalPosts DESC LIMIT 10` over display-name groups: owners tie on (AvgScore, TotalPosts) at the cut and differ in every projected column, so the answer moves with `SET threads`. Refined with `, PS.OwnerDisplayName` |
| 1156 | `AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)))` per user over the posts x votes rows: the printed digits move with `SET threads`. Rewritten to the exact-integer mean `SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.Id) / 1e6` |
| 1615 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) = 1`: an owner's posts tie on CreationDate and differ in the projected Title/Score, so the answer moved with `SET threads`. Refined with `, p.Id`, and the final order with `, tu.UserId` |
| 34294 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) = 1` and `ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 100`: posts tie on Score with no ViewCount at the cut, and the answer moved with `SET threads`. Refined with `, p.Id` in the window and `, rp.PostId` in the final order |
| 6486 | the projected `ROW_NUMBER() OVER (ORDER BY ps.NetScore DESC)` ties at NetScore 99 between two posts (ranks 8 and 9), which differ in PostId. Refined with `, ps.PostId` in the window and the final order |
| 1651 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) = 1` and `LIMIT 100` on `NetVotes DESC, CreationDate DESC`: DuckDB's answer moved with `SET threads` (owners with two posts at the same instant). Refined with `, p.Id` in the window and `, tp.PostId` in the final order |
| 20119 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC)` numbers post x comment rows, and every answer has a NULL ViewCount, so which answer rows get Rank <= 10 (and add to their owner's TopPostCount) is arbitrary. Refined with `, p.Id, c.Id` (the oracle did not change) |
| 2496 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) = 1`: Aaron Bertrand posted a question and its answer at the same instant (342444, 342445), and PostId is projected. Refined with `, p.Id` |
| 99 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) = 1` numbers the post x latest-badge rows, and a user can earn several badges at the same latest instant, so which BadgeClass the first row carries is arbitrary. Refined with `, p.Id, b.Id` (the oracle did not change) |
| 1207 | `ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) <= 5`: owners have several questions tied on score across the fifth place, and the post that joins by id (`ps.UserId = rp.PostId`) moves in and out of the cut (sepupic, Madhavan). Refined with `, p.Id` |
| 468 | `ORDER BY up.Reputation DESC, rp.Score DESC FETCH FIRST 10`: two of Erwin Brandstetter's questions tie at score 14 across the cut and differ in PostId, Title and ViewCount; the answer moved with `SET threads`. Refined with `, rp.PostId, cp.CreationDate, cp.Comment` |
| 582 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) <= 10` numbers post x answer rows, and the answers of one post tie while their owners are projected; the final `ORDER BY tp.ViewCount DESC, us.TotalBountyAwarded DESC LIMIT 10 OFFSET 5` ties too. The answer moved with `SET threads`. Refined with `, p.Id, a.Id` in the window and `, tp.Id, us.UserId` at the end |
| 96 | `ORDER BY AUP.Score DESC, AUP.ViewCount DESC LIMIT 100` over posts x matching tags: posts tie on score and views across the cut and differ in every projected column, so the answer moved with `SET threads`. Refined with `, AUP.PostId, T.Id`, and the rn window with `, P.Id` |
| 4253 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) <= 5` cuts through tied scores, and a post's history rows at its latest date differ in the projected Comment; the answer moved with `SET threads`. Refined with `, p.Id` in the window and `, rp.PostId, rp.LastComment` at the final `LIMIT 10` |
| 868 | `AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)))` per owner: the printed digits move with `SET threads`. Rewritten to the exact-integer mean `SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(*) / 1e6` |
| 8730 | `ORDER BY ar.Score DESC, ar.ViewCount DESC LIMIT 100`: answers with no ViewCount tie on score at the cut and differ in every projected column, so the answer moves with `SET threads`. Refined with `, ar.PostId` |
| 2096 | `ORDER BY Score DESC LIMIT 10` inside TopPosts: hundreds of questions tie on the net recent-vote score at the cut, and the chosen post ids are joined to user ids. Refined with `, P.Id` |
| 24303 | `ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC)`: tag-wiki posts (types 4, 5) all score 0 and tie across the `RankByScore <= 5` cut, and Top5Posts counts which of a user's posts made it. Refined with `, p.Id` |
| 34713 | `ORDER BY m.Score DESC, m.ViewCount DESC LIMIT 100`: recent posts tie at score 1 with no view count across the cut, so which rows come back moves with `SET threads`. Refined with `, m.PostId, m.ClosedDate`, and the RN window with `, p.Id` |
| 32675 | `ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY P.CreationDate DESC)`: a top user posted two posts at the same instant, and the rn = 1 post`s Title, date and score are projected, so the answer moves with `SET threads`. Refined with `, P.Id` |
| 24601 | `LEAD(CreationDate) OVER (ORDER BY CreationDate)` over the post x vote x close rows, cut at `ORDER BY Score DESC, ClosureStatus LIMIT 50`: a post`s rows tie on both, and which of them gets the next post`s date (and which 50 survive) is the engine`s choice. VoteId and CloseId carried through the CTEs, the SELECT * spelled out, and the window and the final order refined with `, PostId, VoteId, CloseId` |
