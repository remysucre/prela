use harness::prelude::*;

// Thirty-six queries written with CTEs. Every one of them is a single GROUP BY
// wrapped in projections: the CTE names the aggregates, an optional second CTE
// adds `ROW_NUMBER() OVER (ORDER BY x)` and the outer SELECT keeps `Rank <= n`.
// Nothing crosses a CTE boundary, so the port is the same grouped query it
// would be without them, with `ROW_NUMBER() ... WHERE Rank <= n` read as
// `ORDER BY x LIMIT n` and `FETCH FIRST n ROWS ONLY` as `LIMIT n`.
//
// Five group posts, thirty-one group users. The SQL above each `fn` is the
// query as the corpus ships it.

fn posts(db: &'static So, only_q: bool, outer: bool, joins: &str) -> Vec<(Id<Post>, Agg)> {
    posts_with_counts(db, only_q, outer, joins)
}

fn u(db: &'static So, joins: &str, by: &str, n: usize, cols: &[&str]) -> String {
    user_rows(db, joins, false, by, n, cols)
}

/// `WHERE u.Reputation > n`, which is on a grouped column and so can be
/// applied to the grouped rows.
fn rep_over(db: &'static So, joins: &str, min: i64, by: &str, n: usize, cols: &[&str]) -> String {
    render_users(db, users_where(db, joins, false, UserWhere::RepGt(min)), joins, by, n, cols)
}

/// `WHERE u.CreationDate >= '2020-01-01'`, likewise.
fn joined_since(db: &'static So, joins: &str, cut: i64, by: &str, n: usize, cols: &[&str]) -> String {
    render_users(db, users_where(db, joins, false, UserWhere::CreatedGe(cut)), joins, by, n, cols)
}

// --- the five that group posts ---------------------------------------------

// WITH Benchmark AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName ) SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, VoteCount FROM Benchmark ORDER BY Score DESC, ViewCount DESC LIMIT 100
fn q10105(db: &'static So) -> String {
    render_posts(db, posts(db, false, true, "cv"), "score,views", 100,
        &["id", "title", "created", "views", "score", "owner", "#c", "#v"])
}

// WITH PostDetails AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName, u.Reputation ), TopPosts AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostDetails ) SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerDisplayName, OwnerReputation, UpVotes, DownVotes FROM TopPosts WHERE Rank <= 10
fn q11966(db: &'static So) -> String {
    render_posts(db, posts(db, false, true, "cv"), "score,views", 10,
        &["id", "title", "created", "score", "views", "answers", "#cx", "owner", "rep", "#up", "#down"])
}

// WITH PostMetrics AS ( SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.FavoriteCount, U.DisplayName AS OwnerDisplayName, U.Reputation AS OwnerReputation, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.FavoriteCount, U.DisplayName, U.Reputation ) SELECT PM.* FROM PostMetrics PM ORDER BY PM.Score DESC
fn q13083(db: &'static So) -> String {
    render_posts(db, posts(db, false, false, "v"), "score", 0,
        &["id", "title", "score", "views", "answers", "comments", "favorites", "owner", "rep", "#vx", "#up", "#down"])
}

// WITH PostStatistics AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, COUNT(c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName, u.Reputation ) SELECT ps.* FROM PostStatistics ps ORDER BY ps.ViewCount DESC
fn q13802(db: &'static So) -> String {
    render_posts(db, posts_where(db, false, true, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "views", 0,
        &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "rep", "#cx", "#up", "#down"])
}

// WITH PopularPosts AS ( SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, COUNT(C.Id) AS CommentCount FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.Score, P.ViewCount, U.DisplayName ORDER BY P.Score DESC, P.ViewCount DESC LIMIT 100 ) SELECT PP.* FROM PopularPosts PP
fn q13981(db: &'static So) -> String {
    render_posts(db, posts(db, true, true, "c"), "score,views", 100,
        &["id", "title", "score", "views", "owner", "#cx"])
}

// --- the thirty-one that group users ---------------------------------------

// WITH UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewsPerPost, AVG(p.Score) AS AvgScorePerPost FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName ) SELECT ... FROM UserPostStats ORDER BY TotalPosts DESC LIMIT 10
fn q10204(db: &'static So) -> String {
    u(db, "", "#rows", 10, &["uid", "name", "#rows", "#q", "#a", "views_sum", "score_sum", "views_avg", "score_avg"])
}

// WITH UserStatistics AS ( SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(c.Id) AS TotalComments, COUNT(v.Id) AS TotalVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.Reputation ) SELECT ... FROM UserStatistics ORDER BY TotalPosts DESC, TotalComments DESC, TotalVotes DESC LIMIT 100
fn q10269(db: &'static So) -> String {
    u(db, "cv", "#n,#cx,#vx", 100, &["uid", "rep", "#n", "#cx", "#vx"])
}

// WITH UserPostCounts AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation ) SELECT ... FROM UserPostCounts UPC ORDER BY UPC.Reputation DESC, UPC.PostCount DESC LIMIT 10
fn q10363(db: &'static So) -> String {
    u(db, "", "rep,#rows", 10, &["uid", "name", "rep", "#rows"])
}

// WITH UserActivity AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(p.ViewCount) AS TotalViewCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation ) SELECT ... FROM UserActivity ORDER BY Reputation DESC, TotalPosts DESC LIMIT 100
fn q10430(db: &'static So) -> String {
    u(db, "cv", "rep,#n", 100, &["uid", "name", "rep", "#n", "#c", "#up", "#down", "#q", "#a", "views_sum"])
}

// WITH UserPostCounts AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikiCount, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AverageScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserPostCounts ) SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViews, AverageScore FROM TopUsers WHERE Rank <= 10
fn q10590(db: &'static So) -> String {
    u(db, "", "#rows", 10, &["uid", "name", "#rows", "#q", "#a", "views_sum", "score_avg"])
}

// WITH UserStats AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats ) SELECT ... FROM TopUsers WHERE Rank <= 10 ORDER BY Reputation DESC
fn q10821(db: &'static So) -> String {
    u(db, "v", "rep", 10, &["uid", "name", "rep", "#n", "#q", "#a", "#up", "#down"])
}

// WITH UserPostStats AS ( SELECT u.Id AS UserId, COUNT(p.Id) AS TotalPosts, COALESCE(AVG(p.Score), 0) AS AveragePostScore, COUNT(v.Id) AS TotalVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id ) SELECT ups.UserId, ups.TotalPosts, ups.AveragePostScore, ups.TotalVotes FROM UserPostStats ups ORDER BY ups.TotalPosts DESC
fn q11412(db: &'static So) -> String {
    u(db, "v", "#rows", 0, &["uid", "#rows", "score_avg0", "#vx"])
}

// WITH UserPostStats AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserPostStats ) SELECT ... FROM TopUsers WHERE Rank <= 10
fn q11432(db: &'static So) -> String {
    u(db, "v", "#rows", 10, &["uid", "name", "#rows", "#q", "#a", "#up", "#down"])
}

// WITH UserStats AS ( SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats ) SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount FROM TopUsers WHERE Rank <= 10
fn q11502(db: &'static So) -> String {
    u(db, "", "rep", 10, &["uid", "rep", "#n", "#q", "#a"])
}

// WITH UserActivity AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserActivity ) SELECT ... FROM TopUsers WHERE Rank <= 10
fn q11840(db: &'static So) -> String {
    u(db, "cv", "#rows", 10, &["uid", "name", "#rows", "#up", "#down", "#c"])
}

// WITH UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, AVG(u.Reputation) AS AverageReputation FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName ) SELECT ... FROM UserPostStats ORDER BY TotalPosts DESC LIMIT 100
fn q11853(db: &'static So) -> String {
    u(db, "v", "#rows", 100, &["uid", "name", "#rows", "#up", "#down", "rep_avg"])
}

// WITH UserActivity AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.CreationDate >= '2020-01-01' GROUP BY u.Id, u.DisplayName ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserActivity ) SELECT ... FROM TopUsers WHERE Rank <= 10
fn q11976(db: &'static So) -> String {
    joined_since(db, "v", date(2020, 1, 1), "#n", 10, &["uid", "name", "#n", "#q", "#a", "#up", "#down"])
}

// WITH UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(v.BountyAmount) AS TotalBountyAmount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats ) SELECT ... FROM TopUsers WHERE Rank <= 10
fn q12003(db: &'static So) -> String {
    u(db, "v", "#rows", 10, &["uid", "name", "#rows", "#q", "#a", "bounty_sum", "#up", "#down"])
}

// WITH UserStats AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats ) SELECT ... FROM TopUsers WHERE Rank <= 10
fn q12098(db: &'static So) -> String {
    u(db, "cv", "rep", 10, &["uid", "name", "rep", "#n", "#c", "#up", "#down"])
}

// WITH UserStats AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation ) SELECT ... FROM UserStats ORDER BY Reputation DESC FETCH FIRST 10 ROWS ONLY
fn q12501(db: &'static So) -> String {
    u(db, "v", "rep", 10, &["uid", "name", "rep", "#rows", "#up", "#down"])
}

// WITH UserReputation AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(COALESCE(p.Score, 0)) AS AvgScore, SUM(p.ViewCount) AS TotalViews FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Ranking FROM UserReputation ) SELECT ... FROM TopUsers WHERE Ranking <= 10 ORDER BY Ranking
fn q12692(db: &'static So) -> String {
    u(db, "", "rep", 10, &["uid", "name", "rep", "#rows", "#q", "#a", "score_avg_all", "views_sum"])
}

// WITH UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers, AVG(p.Score) AS AverageScore, SUM(p.ViewCount) AS TotalViews FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS UserRank FROM UserPostStats ) SELECT ... FROM TopUsers WHERE UserRank <= 10
fn q13060(db: &'static So) -> String {
    u(db, "", "#rows", 10, &["uid", "name", "#rows", "#q", "#a", "#acc", "score_avg", "views_sum"])
}

// WITH UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.ViewCount) AS TotalViews, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation ) SELECT ... FROM UserPostStats ORDER BY Reputation DESC, TotalPosts DESC FETCH FIRST 100 ROWS ONLY
fn q13229(db: &'static So) -> String {
    u(db, "v", "rep,#rows", 100, &["uid", "name", "rep", "#rows", "#q", "#a", "views_sum", "#up", "#down"])
}

// WITH UserStats AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserStats ) SELECT ... FROM TopUsers WHERE Rank <= 10
fn q13650(db: &'static So) -> String {
    u(db, "", "score_sum", 10, &["uid", "name", "#n", "score_sum0", "views_sum0", "#acc"])
}

// WITH UserStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, AVG(p.Score) AS AvgPostScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName ) SELECT ... FROM UserStats u ORDER BY u.TotalPosts DESC, u.TotalComments DESC
fn q13664(db: &'static So) -> String {
    u(db, "cv", "#n,#c", 0, &["uid", "name", "#n", "#c", "#up", "#down", "score_avg"])
}

// WITH UserActivity AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, COUNT(DISTINCT v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName ) SELECT ... FROM UserActivity ORDER BY TotalVotes DESC, TotalPosts DESC
fn q13876(db: &'static So) -> String {
    u(db, "cv", "#v,#n", 0, &["uid", "name", "#n", "#c", "#v", "#up", "#down"])
}

// WITH UserStats AS ( SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.Reputation ) SELECT ... FROM UserStats us WHERE us.Reputation > 1000 ORDER BY us.Reputation DESC
fn q13951(db: &'static So) -> String {
    rep_over(db, "v", 1000, "rep", 0, &["uid", "rep", "#n", "#q", "#a", "#up", "#down"])
}

// WITH UserPostStats AS ( SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.Reputation ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserPostStats ) SELECT ... FROM TopUsers WHERE Rank <= 10
fn q14391(db: &'static So) -> String {
    u(db, "cv", "#n", 10, &["uid", "rep", "#n", "#c", "#up", "#down"])
}

// WITH UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, AVG(COALESCE(p.Score, 0)) AS AvgScorePerPost, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewsPerPost FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName ) SELECT u.DisplayName, u.PostCount, u.QuestionsCount, u.AnswersCount, u.TotalScore, u.TotalViews, u.AvgScorePerPost, u.AvgViewsPerPost FROM UserPostStats u ORDER BY u.TotalScore DESC LIMIT 10
fn q14912(db: &'static So) -> String {
    u(db, "", "score_sum", 10, &["name", "#rows", "#q", "#a", "score_sum0", "views_sum0", "score_avg_all", "views_avg_all"])
}

// WITH UserStats AS ( SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats ) SELECT ... FROM TopUsers WHERE Rank <= 10 ORDER BY Rank
fn q14919(db: &'static So) -> String {
    u(db, "cv", "rep", 10, &["uid", "rep", "#n", "#c", "#up", "#down"])
}

// WITH UserStatistics AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalUpVotes DESC) AS Ranking FROM UserStatistics ) SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalUpVotes, tu.TotalDownVotes FROM TopUsers tu WHERE tu.Ranking <= 10 ORDER BY tu.TotalUpVotes DESC
fn q5321(db: &'static So) -> String {
    rep_over(db, "v", 0, "#up", 10, &["name", "#n", "#q", "#a", "#up", "#down"])
}

// WITH UserSummaries AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserSummaries ) SELECT u.DisplayName, u.TotalPosts, u.QuestionCount, u.AnswerCount, u.AcceptedAnswers, u.Upvotes, u.Downvotes FROM TopUsers u WHERE u.Rank <= 10 ORDER BY u.Rank
fn q6036(db: &'static So) -> String {
    rep_over(db, "v", 100, "#n", 10, &["name", "#n", "#q", "#a", "#acc", "#up", "#down"])
}

// WITH UserStats AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation ), RankedUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats ) SELECT RU.DisplayName, RU.Reputation, RU.PostCount, RU.QuestionCount, RU.AnswerCount, RU.UpVotes, RU.DownVotes FROM RankedUsers RU WHERE RU.Rank <= 10 ORDER BY RU.Reputation DESC
fn q6148(db: &'static So) -> String {
    u(db, "v", "rep", 10, &["name", "rep", "#n", "#q", "#a", "#up", "#down"])
}

// WITH UserEngagement AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserEngagement ) SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.UpVotes, tu.DownVotes FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Reputation DESC
fn q7633(db: &'static So) -> String {
    u(db, "v", "rep", 10, &["name", "rep", "#n", "#q", "#a", "#up", "#down"])
}

// WITH UserStats AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, AVG(COALESCE(P.Score, 0)) AS AvgScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserStats ) SELECT U.UserId, U.DisplayName, U.Reputation, U.PostCount, U.QuestionCount, U.AnswerCount, U.UpvoteCount, U.DownvoteCount, U.AvgScore FROM TopUsers U WHERE U.Rank <= 10 ORDER BY U.Reputation DESC, U.PostCount DESC
fn q7972(db: &'static So) -> String {
    u(db, "v", "rep,#n", 10, &["uid", "name", "rep", "#n", "#q", "#a", "#up", "#down", "score_avg_all"])
}

// WITH UserActivity AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, MAX(p.CreationDate) AS LastPostDate, COUNT(DISTINCT c.Id) AS CommentCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName, u.Reputation ), TopUsers AS ( SELECT ..., ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserActivity ) SELECT t.UserId, t.DisplayName, t.Reputation, t.PostCount, t.QuestionCount, t.AnswerCount, t.LastPostDate, t.CommentCount FROM TopUsers t WHERE t.UserRank <= 10 ORDER BY t.Reputation DESC
fn q9795(db: &'static So) -> String {
    u(db, "c", "rep", 10, &["uid", "name", "rep", "#n", "#q", "#a", "created_max", "#c"])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10105", q10105),
    ("10204", q10204),
    ("10269", q10269),
    ("10363", q10363),
    ("10430", q10430),
    ("10590", q10590),
    ("10821", q10821),
    ("11412", q11412),
    ("11432", q11432),
    ("11502", q11502),
    ("11840", q11840),
    ("11853", q11853),
    ("11966", q11966),
    ("11976", q11976),
    ("12003", q12003),
    ("12098", q12098),
    ("12501", q12501),
    ("12692", q12692),
    ("13060", q13060),
    ("13083", q13083),
    ("13229", q13229),
    ("13650", q13650),
    ("13664", q13664),
    ("13802", q13802),
    ("13876", q13876),
    ("13951", q13951),
    ("13981", q13981),
    ("14391", q14391),
    ("14912", q14912),
    ("14919", q14919),
    ("5321", q5321),
    ("6036", q6036),
    ("6148", q6148),
    ("7633", q7633),
    ("7972", q7972),
    ("9795", q9795),
];
