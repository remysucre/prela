use harness::prelude::*;
use std::cmp::Reverse;


// ORDER BY (a key, b key) LIMIT n over a CROSS JOIN, the product driven by prela's `.cross`. The order leads with the a key, so an a row
// with k rows strictly ahead of it starts at position k * |b| + 1: only a rows whose key is at most that of the row at index (n - 1) / |b|
// can reach the first n. That cut-off key is picked first and the a side filtered on it in prela, as for a rank read from base columns.
fn cross_top<A: Copy, B: Copy, KA: Ord, KB: Ord>(a: Vec<A>, ka: impl Fn(&A) -> KA, b: Vec<B>, kb: impl Fn(&B) -> KB, n: usize) -> Vec<(A, B)> {
    let n = if n == usize::MAX { 0 } else { n };
    let a = top_n(a, &ka, 0);
    let ra = rel(a);
    let rb = rel(b);
    let v = if n > 0 && !rb.v.is_empty() && (n - 1) / rb.v.len() + 1 < ra.v.len() {
        let cut = ka(&ra.v[(n - 1) / rb.v.len()]);
        drain((&ra).filt(|x| ka(&x) <= cut).cross(&rb))
    } else {
        drain((&ra).cross(&rb))
    };
    top_n(v, |(_, (x, y))| (ka(x), kb(y)), n).into_iter().map(|x| x.1).collect()
}

/// Per user over `Users LEFT JOIN Posts LEFT JOIN Badges` (the product):
/// [rows with a post, score sum, gold, silver, bronze, class sum, badge id sum, rows].
fn user_posts_badges(db: &'static So) -> Fold<Id<User>, [i64; 8]> {
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.score).opt().and(badges_of(db).select((&db.badge.class).and(&db.badge.origid)).opt()))
        .fold([0i64; 8], |a, (s, b)| {
            let c = b.map(|b| b.0);
            [
                a[0] + s.is_some() as i64,
                a[1] + s.unwrap_or(0),
                a[2] + (c == Some(1)) as i64,
                a[3] + (c == Some(2)) as i64,
                a[4] + (c == Some(3)) as i64,
                a[5] + c.unwrap_or(0),
                a[6] + b.map_or(0, |b| b.1),
                a[7] + 1,
            ]
        })
}

fn user_distinct_posts(db: &'static So) -> Fold<Id<User>, i64> {
    db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some)
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//            SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.Reputation),
// PostTypesCount AS (
//     SELECT PT.Id AS PostTypeId, PT.Name AS PostTypeName, COUNT(P.Id) AS TotalPosts, SUM(P.Score) AS TotalScore
//     FROM PostTypes PT LEFT JOIN Posts P ON PT.Id = P.PostTypeId GROUP BY PT.Id, PT.Name)
// SELECT U.UserId, U.Reputation, U.PostCount, U.AnswerCount, U.QuestionCount, U.CommentCount, PT.PostTypeId, PT.PostTypeName, PT.TotalPosts, PT.TotalScore
// FROM UserStats U CROSS JOIN PostTypesCount PT ORDER BY U.Reputation DESC, PT.TotalPosts DESC LIMIT 100;
fn q12820(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt())))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + c.is_some() as i64]);
    let users = drain(user_distinct_posts(db).iq().and((&us).opt()));
    let types = drain(&type_left_posts(db));
    let v = cross_top(users, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), types, |&(_, b)| Reverse(b[0]), 100);
    rows(v.iter().map(|&((u, (n, a)), (t, b))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(db.post_type.origid.get(t).unwrap()), tname(db, t), V::I(b[0]), nullable(b[1], b[0])]);
        row(f)
    }))
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount,
//            AVG(p.ViewCount) AS AverageViewCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount,
//            SUM(CASE WHEN p.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN 1 ELSE 0 END) AS OldPostCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// PostTypeCount AS (SELECT pt.Id AS PostTypeId, COUNT(p.Id) AS PostCount FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Id)
// SELECT u.DisplayName, ups.PostCount, ups.PositiveScoreCount, ups.AverageViewCount, ups.AcceptedAnswerCount, ups.OldPostCount, pt.PostTypeId,
//        pt.PostCount AS PostTypePostCount
// FROM UserPostStats ups JOIN Users u ON ups.UserId = u.Id JOIN PostTypeCount pt ON ups.PostCount > 0 ORDER BY ups.PostCount DESC, u.DisplayName;
fn q13277(db: &'static So) -> String {
    let old = ts(2023, 10, 1, 12, 34, 56);
    let Post { score, view_count, accepted_answer_id, creation_date, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(accepted_answer_id.opt()).and(creation_date)))
        .fold([0i64; 6], |a, (((s, v), x), d)| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4] + x.is_some() as i64, a[5] + (d < old) as i64]);
    let pt = type_left_posts(db);
    let mut out = Vec::new();
    (&ups).cross(&pt).drive(|(u, t), (a, b)| {
        out.push(row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(a[4]), V::I(a[5]), V::I(db.post_type.origid.get(t).unwrap()), V::I(b[0])]))
    });
    rows(out)
}

// WITH PostStatistics AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount, AVG(COALESCE(p.Score, 0)) AS AvgScore,
//            SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers, SUM(COALESCE(p.CommentCount, 0)) AS TotalComments
//     FROM Posts p INNER JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStatistics AS (
//     SELECT u.DisplayName, COUNT(p.Id) AS PostsCount, AVG(COALESCE(u.Reputation, 0)) AS AvgReputation,
//            SUM(CASE WHEN b.Date IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName)
// SELECT ps.PostType, ps.TotalPosts, ps.AvgViewCount, ps.AvgScore, ps.TotalAnswers, ps.TotalComments, us.DisplayName AS UserName,
//        us.PostsCount, us.AvgReputation, us.TotalBadges
// FROM PostStatistics ps CROSS JOIN UserStatistics us ORDER BY ps.TotalPosts DESC, us.PostsCount DESC;
fn q13698(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let us = db
        .user
        .group_by(&db.user.display_name)
        .select((&db.user.reputation).and(posts_of(db).opt()).and(badges_of(db).select(&db.badge.date).opt()))
        .fold([0i64; 4], |a, ((r, p), d)| [a[0] + 1, a[1] + p.is_some() as i64, a[2] + r, a[3] + d.is_some() as i64]);
    let mut out = Vec::new();
    (&ps).cross(&us).drive(|(t, name), (a, b)| {
        out.push(row(vec![V::S(t), V::I(a[0]), avg(a[3], a[0]), avg(a[1], a[0]), V::I(a[5]), V::I(a[6]), V::S(name), V::I(b[1]), avg(b[2], b[0]), V::I(b[3])]))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostTypeName, COUNT(p.Id) AS TotalPosts, AVG(CASE WHEN p.PostTypeId = 1 THEN p.Score END) AS AverageQuestionScore,
//            SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
//     FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     LEFT JOIN (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPostsByUser, SUM(COALESCE(b.Id, 0)) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation)
// SELECT ps.PostTypeName, ps.TotalPosts, ps.AverageQuestionScore, ps.TotalComments, us.Reputation, us.TotalPostsByUser, us.TotalBadges
// FROM PostStats ps JOIN UserStats us ON us.TotalPostsByUser > 0 ORDER BY ps.TotalPosts DESC;
fn q13872(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post).fold(0i64, |n, _| n + 1);
    let ps = db
        .post
        .group_by(ptype_name(db))
        .select((&db.post.post_type_id).and(&db.post.score).and((&cc).opt()))
        .fold([0i64; 4], |a, ((t, s), c)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + if t == 1 { s } else { 0 }, a[3] + c.unwrap_or(0)]);
    let upb = user_posts_badges(db);
    let dp = user_distinct_posts(db);
    let mut out = Vec::new();
    (&ps).cross((&dp).filt(|n| n > 0).and(&upb)).drive(|(t, u), (a, (n, b))| {
        out.push(row(vec![V::S(t), V::I(a[0]), avg(a[2], a[1]), V::I(a[3]), user_col(db, u, "rep"), V::I(n), V::I(b[6])]))
    });
    rows(out)
}

// WITH UserEngagement AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments,
//            SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY U.Id, U.DisplayName),
// PostStats AS (
//     SELECT P.PostTypeId, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore,
//            AVG(P.AnswerCount) AS AvgAnswerCount, AVG(P.CommentCount) AS AvgCommentCount
//     FROM Posts P GROUP BY P.PostTypeId)
// SELECT UE.UserId, UE.DisplayName, UE.TotalPosts, UE.TotalComments, UE.TotalVotes, UE.AcceptedAnswers, PS.PostTypeId, PS.PostCount,
//        PS.TotalViews, PS.TotalScore, PS.AvgAnswerCount, PS.AvgCommentCount
// FROM UserEngagement UE JOIN PostStats PS ON UE.TotalPosts > 0 ORDER BY UE.TotalPosts DESC, PS.PostCount DESC;
fn q13041(db: &'static So) -> String {
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.accepted_answer_id).opt().and(comments_of(db).opt()).and(votes_of(db).opt())))
        .fold((0i64, 0i64), |(v, a), ((x, _), y)| (v + y.is_some() as i64, a + x.is_some() as i64));
    let dp = user_distinct_posts(db);
    let dc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).count_distinct();
    let Post { view_count, score, answer_count, comment_count, .. } = &db.post;
    let ps = db
        .post
        .group_by(&db.post.post_type_id)
        .select(view_count.opt().and(score).and(answer_count.opt()).and(comment_count))
        .fold([0i64; 7], |a, (((v, s), an), c)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s, a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c]);
    let mut out = Vec::new();
    (&dp).filt(|n| n > 0).and(&ue).and((&dc).opt()).cross(&ps).drive(|(u, t), (((n, (v, a)), c), b)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c.unwrap_or(0)), V::I(v), V::I(a), V::I(t), V::I(b[0]), nullable(b[2], b[1]), V::I(b[3]), avg(b[5], b[4]), avg(b[6], b[0])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserCounts AS (
//     SELECT Users.Id AS UserId, COUNT(DISTINCT Posts.Id) AS PostCount, SUM(Posts.Score) AS TotalScore,
//            SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId LEFT JOIN Votes ON Posts.Id = Votes.PostId GROUP BY Users.Id),
// BadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// PostStatistics AS (
//     SELECT PostTypes.Name AS PostTypeName, COUNT(Posts.Id) AS TotalPosts, SUM(Posts.ViewCount) AS TotalViews, AVG(Posts.Score) AS AverageScore
//     FROM Posts JOIN PostTypes ON Posts.PostTypeId = PostTypes.Id GROUP BY PostTypes.Name)
// SELECT U.UserId, U.PostCount, U.TotalScore, U.UpVotes, U.DownVotes, COALESCE(B.BadgeCount, 0) AS BadgeCount, P.PostTypeName, P.TotalPosts,
//        P.TotalViews, P.AverageScore
// FROM UserCounts U LEFT JOIN BadgeCounts B ON U.UserId = B.UserId LEFT JOIN PostStatistics P ON P.TotalPosts > 0
// ORDER BY U.TotalScore DESC, U.PostCount DESC;
fn q13130(db: &'static So) -> String {
    let uc = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, t)) => [a[0] + 1, a[1] + s, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    let dp = user_distinct_posts(db);
    let bc = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let ps = stats_by_type(db);
    let p = left_all(drain((&ps).filt(|a| a[0] > 0)));
    let mut out = Vec::new();
    (&uc).and(&dp).and((&bc).opt()).cross(&p).drive(|(u, _), (((a, n), b), x)| {
        let mut f = vec![user_col(db, u, "uid"), V::I(n), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(b.unwrap_or(0))];
        f.extend(match x {
            Some((t, c)) => [V::S(t), V::I(c[0]), nullable(c[3], c[2]), avg(c[1], c[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStatistics AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, COUNT(CASE WHEN p.ViewCount IS NOT NULL THEN 1 END) AS PostsWithViews,
//            AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViews, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS QuestionsWithAcceptedAnswers
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStatistics AS (
//     SELECT u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPostsByUser, SUM(p.ViewCount) AS TotalViewsByUser, SUM(p.Score) AS TotalScoreByUser,
//            AVG(p.Score) AS AverageScoreByUser
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.DisplayName)
// SELECT ps.*, us.* FROM PostStatistics ps FULL OUTER JOIN UserStatistics us ON TRUE ORDER BY ps.TotalPosts DESC, us.TotalPostsByUser DESC;
//
// With both sides non-empty a FULL JOIN ON TRUE is the cross product.
fn q14408(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let acc = db.post.group_by(ptype_name(db)).select((&db.post.accepted_answer_id).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let us = db
        .user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score)).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((v, s)) => [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s],
            None => a,
        });
    let dp = db.user.group_by(&db.user.display_name).select(posts_of(db).opt()).buf_fold(distinct_some);
    let mut out = Vec::new();
    (&ps).and(&acc).cross((&us).and(&dp)).drive(|(t, name), ((a, x), (b, n))| {
        out.push(row(vec![V::S(t), V::I(a[0]), V::I(a[2]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(x), V::S(name), V::I(n), nullable(b[2], b[1]), nullable(b[3], b[0]), avg(b[3], b[0])]))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT p.PostTypeId, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(p.ViewCount) AS TotalViews,
//            AVG(COALESCE(p.Score, 0)) AS AvgScore, AVG(COALESCE(p.ViewCount, 0)) AS AvgViews, MIN(p.CreationDate) AS EarliestPostDate,
//            MAX(p.CreationDate) AS LatestPostDate
//     FROM Posts p GROUP BY p.PostTypeId),
// UserStats AS (
//     SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, ... Silver, Bronze
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT p.*, u.* FROM PostStats p LEFT JOIN UserStats u ON u.PostsCreated > 0 ORDER BY p.PostTypeId, u.PostsCreated DESC;
fn q11151(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let ps = db
        .post
        .group_by(&db.post.post_type_id)
        .select(score.and(view_count.opt()).and(creation_date))
        .fold([0, 0, 0, 0, i64::MAX, i64::MIN], |a, ((s, v), d)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4].min(d), a[5].max(d)]);
    let upb = user_posts_badges(db);
    let us = left_all(drain(user_distinct_posts(db).iq().filt(|n| n > 0).and(&upb)));
    let mut out = Vec::new();
    (&ps).cross(&us).drive(|(t, _), (a, x)| {
        let mut f = vec![V::I(t), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[1], a[0]), avg(a[3], a[0]), tmin(a[4]), tmax(a[5])];
        f.extend(match x {
            Some((u, (n, b))) => [user_col(db, u, "uid"), V::I(n), V::I(b[2]), V::I(b[3]), V::I(b[4])],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//            SUM(CASE WHEN p.Score <= 0 THEN 1 ELSE 0 END) AS NegativeScorePosts, AVG(p.ViewCount) AS AvgViewCount, AVG(p.AnswerCount) AS AvgAnswerCount,
//            AVG(p.CommentCount) AS AvgCommentCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(b.Class, 0)) AS TotalBadges, AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName)
// SELECT ps.PostType, ps.TotalPosts, ps.PositiveScorePosts, ps.NegativeScorePosts, ps.AvgViewCount, ps.AvgAnswerCount, ps.AvgCommentCount,
//        us.DisplayName AS TopUser, us.TotalPosts AS UserPostCount, us.TotalBadges, us.AvgReputation
// FROM PostStats ps LEFT JOIN UserStats us ON us.TotalPosts = (SELECT MAX(TotalPosts) FROM UserStats) ORDER BY ps.TotalPosts DESC;
fn q10607(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let us = db
        .user
        .group_by(&db.user.display_name)
        .select((&db.user.reputation).and(posts_of(db).opt()).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, ((r, _), c)| [a[0] + 1, a[1] + c.unwrap_or(0), a[2] + r]);
    let dp = db.user.group_by(&db.user.display_name).select(posts_of(db).opt()).buf_fold(distinct_some);
    let most = (&dp).fold_flat(i64::MIN, |m, n| m.max(n));
    let top = left_all(drain((&dp).filt(|n| n == most).and(&us)));
    let mut out = Vec::new();
    (&ps).cross(&top).drive(|(t, _), (a, x)| {
        let mut f = vec![V::S(t), V::I(a[0]), V::I(a[9]), V::I(a[0] - a[9]), avg(a[3], a[2]), avg(a[5], a[4]), avg(a[6], a[0])];
        f.extend(match x {
            Some((name, (n, b))) => [V::S(name), V::I(n), V::I(b[1]), avg(b[2], b[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH UserStatistics AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//            SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, ... Silver, Bronze
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostTypesStatistics AS (
//     SELECT pt.Name AS PostTypeName, COUNT(p.Id) AS PostCount, AVG(p.ViewCount) AS AvgViewCount, AVG(p.AnswerCount) AS AvgAnswerCount,
//            SUM(p.Score) AS TotalScore
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Name)
// SELECT u.DisplayName, u.PostCount, u.TotalScore, u.GoldBadges, u.SilverBadges, u.BronzeBadges, pt.PostTypeName, pt.PostCount AS PostTypeCount,
//        pt.AvgViewCount, pt.AvgAnswerCount, pt.TotalScore AS PostTypeScore
// FROM UserStatistics u CROSS JOIN PostTypesStatistics pt ORDER BY u.TotalScore DESC, pt.TotalScore DESC;
fn q12942(db: &'static So) -> String {
    let upb = user_posts_badges(db);
    let dp = user_distinct_posts(db);
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let pts = db
        .post_type
        .group_by(&db.post_type.name)
        .select((&of_type).select((&db.post.view_count).opt().and((&db.post.answer_count).opt()).and(&db.post.score)).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((v, an), s)) => [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + an.is_some() as i64, a[4] + an.unwrap_or(0), a[5] + s],
            None => a,
        });
    let mut out = Vec::new();
    (&upb).and(&dp).cross(&pts).drive(|(u, t), ((a, n), b)| {
        out.push(row(vec![user_col(db, u, "name"), V::I(n), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::S(t), V::I(b[0]), avg(b[2], b[1]), avg(b[4], b[3]), nullable(b[5], b[0])]))
    });
    rows(out)
}


// WITH PostCounts AS (
//     SELECT PostTypeId, COUNT(*) AS TotalPosts, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
//     FROM Posts GROUP BY PostTypeId),
// UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS TotalBadges, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (
//     SELECT p.Id AS PostId, COUNT(ph.Id) AS RevisionCount, MAX(ph.CreationDate) AS LastRevisionDate
//     FROM PostHistory ph INNER JOIN Posts p ON ph.PostId = p.Id GROUP BY p.Id)
// SELECT pc.PostTypeId, pc.TotalPosts, pc.TotalQuestions, pc.TotalAnswers, us.UserId, us.DisplayName, us.TotalBadges, us.TotalBountyAmount,
//        phs.RevisionCount, phs.LastRevisionDate
// FROM PostCounts pc JOIN UserStats us ON us.UserId IS NOT NULL JOIN PostHistoryStats phs ON phs.PostId = pc.PostTypeId
// ORDER BY pc.TotalPosts DESC, us.TotalBadges DESC;
//
// The last join matches a post Id against a post type Id, as written.
fn q13305(db: &'static So) -> String {
    let pc = db.post.group_by(&db.post.post_type_id).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let phs = db.post_history.group_by((&db.post_history.post).select(&db.post.origid)).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64), |(b, s), (x, v)| (b + x.is_some() as i64, s + v.flatten().unwrap_or(0)));
    let mut out = Vec::new();
    (&pc).and(&phs).cross(&us).drive(|(t, u), ((a, (n, m)), (b, s))| {
        let mut f = vec![V::I(t), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(b), V::I(s), V::I(n), tmax(m)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AvgScore, SUM(p.ViewCount) AS TotalViews,
//            SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, SUM(v.BountyAmount) AS TotalBounty, COUNT(DISTINCT v.PostId) AS TotalVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// VoteStats AS (SELECT vt.Name AS VoteType, COUNT(v.Id) AS TotalVotes, AVG(v.BountyAmount) AS AvgBounty
//               FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY vt.Name)
// SELECT ps.PostType, ps.TotalPosts, ps.AvgScore, ps.TotalViews, ps.TotalAcceptedAnswers, us.DisplayName AS UserDisplayName, us.TotalBounty,
//        us.TotalVotes, vs.VoteType, vs.TotalVotes AS VoteCount, vs.AvgBounty
// FROM PostStats ps JOIN UserStats us ON us.TotalVotes > 0 JOIN VoteStats vs ON vs.TotalVotes > 0 ORDER BY ps.TotalPosts DESC;
fn q11167(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let acc = db.post.group_by(ptype_name(db)).select((&db.post.accepted_answer_id).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt())).fold((0i64, 0i64), |(n, s), b| (n + b.is_some() as i64, s + b.unwrap_or(0)));
    let dv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.post_id)).count_distinct();
    let vs = db.vote.group_by(vtype_name(db)).select((&db.vote.bounty_amount).opt()).fold([0i64; 3], |a, b| [a[0] + 1, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]);
    let users = rel(drain((&dv).filt(|n| n > 0).and(&us)));
    let mut out = Vec::new();
    (&ps).and(&acc).cross(&users).cross((&vs).filt(|a| a[0] > 0)).drive(|((t, _), vt), (((a, x), (u, (n, (bn, b)))), c)| {
        out.push(row(vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(x), user_col(db, u, "name"), nullable(b, bn), V::I(n), V::S(vt), V::I(c[0]), avg(c[2], c[1])]))
    });
    rows(out)
}

// WITH RecentPosts AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Author, pt.Name AS PostType, COUNT(DISTINCT c.Id) AS CommentCount,
//            COUNT(DISTINCT a.Id) AS AnswerCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, pt.Name),
// TopUsers AS (
//     SELECT u.Id, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Author, rp.PostType, rp.CommentCount, rp.AnswerCount, tu.DisplayName AS TopUser, tu.UpVotes, tu.DownVotes
// FROM RecentPosts rp JOIN TopUsers tu ON tu.UpVotes > 50 ORDER BY rp.CreationDate DESC, rp.CommentCount DESC;
fn q8459(db: &'static So) -> String {
    let recent = || owned(db).with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let dc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(distinct_some);
    let da = recent().group_by(Ident::<Post>::new()).select(answers_of(db).opt()).buf_fold(distinct_some);
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt())
        .fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let top = rel(drain((&tu).filt(|(u, _)| u > 50)));
    let mut out = Vec::new();
    (&dc).and(&da).cross(&top).drive(|(p, _), ((c, a), (u, (up, dn)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "type"]);
        f.extend([V::I(c), V::I(a), user_col(db, u, "name"), V::I(up), V::I(dn)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostCounts AS (
//     SELECT PostTypeId, COUNT(*) AS TotalPosts, COUNT(DISTINCT OwnerUserId) AS UniqueUsers, SUM(CASE WHEN Score > 0 THEN 1 ELSE 0 END) AS PositiveScores,
//            SUM(CASE WHEN AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Posts GROUP BY PostTypeId),
// UserStatistics AS (
//     SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN p.Id IS NOT NULL THEN 1 ELSE 0 END) AS PostCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.Reputation),
// TopPosts AS (
//     SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerName
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id ORDER BY p.Score DESC LIMIT 10)
// SELECT pc.PostTypeId, pc.TotalPosts, pc.UniqueUsers, pc.PositiveScores, pc.AcceptedAnswers, us.Reputation, us.BadgeCount, us.PostCount,
//        tp.Title, tp.Score, tp.ViewCount, tp.OwnerName
// FROM PostCounts pc JOIN UserStatistics us ON us.PostCount > 0 CROSS JOIN TopPosts tp;
fn q12495(db: &'static So) -> String {
    let pc = db
        .post
        .group_by(&db.post.post_type_id)
        .select((&db.post.score).and((&db.post.accepted_answer_id).opt()))
        .fold([0i64; 3], |a, (s, x)| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + x.is_some() as i64]);
    let uu = db.post.group_by(&db.post.post_type_id).select(&db.post.owner_user_id).count_distinct();
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).opt()))
        .fold((0i64, 0i64), |(b, p), (x, y)| (b + x.is_some() as i64, p + y.is_some() as i64));
    let users = rel(drain((&us).filt(|(_, p)| p > 0)));
    let tp = rel(top_n(drain(owned(db).select(&db.post.score)), |&(_, s)| Reverse(s), 10));
    let mut out = Vec::new();
    (&pc).and((&uu).opt()).cross(&users).cross(&tp).drive(|((t, _), _), (((a, o), (u, (b, n))), (p, _))| {
        let mut f = vec![V::I(t)];
        f.extend([V::I(a[0]), V::I(o.unwrap_or(0)), V::I(a[1]), V::I(a[2]), user_col(db, u, "rep"), V::I(b), V::I(n)]);
        f.extend(post_fields(db, p, &["title", "score", "views", "owner"]));
        out.push(row(f))
    });
    rows(out)
}

// WITH RecentPosts AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount,
//            COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// TopUsers AS (
//     SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(b.Class), 0) AS TotalBadges, COALESCE(SUM(p.Score), 0) AS TotalScore
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName
//     ORDER BY TotalBadges DESC, TotalScore DESC LIMIT 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes, tu.DisplayName AS TopUser, tu.TotalBadges, tu.TotalScore
// FROM RecentPosts rp JOIN TopUsers tu ON rp.Score > 50 ORDER BY rp.Score DESC, rp.CommentCount DESC;
fn q9251(db: &'static So) -> String {
    let rp = db
        .post
        .with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with((&db.post.score).gt(50))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let upb = user_posts_badges(db);
    let tu = rel(top_n(drain(&upb), |&(_, a)| (Reverse(a[5]), Reverse(a[1])), 10));
    let mut out = Vec::new();
    (&rp).cross(&tu).drive(|(p, _), (a, (u, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), user_col(db, u, "name"), V::I(b[5]), V::I(b[1])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH TagStats AS (
//     SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(COALESCE(p.Score, 0)) AS AverageScore
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// UserEngagement AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//            SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//            COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     GROUP BY u.Id, u.DisplayName)
// SELECT ts.TagName, ts.PostCount, ts.QuestionCount, ts.AnswerCount, ts.AverageScore, ue.DisplayName, ue.PostCount AS UserPostCount,
//        ue.TotalViews, ue.UpVotes, ue.DownVotes, ue.CommentCount
// FROM TagStats ts JOIN UserEngagement ue ON ts.PostCount > 0 ORDER BY ts.AverageScore DESC, ue.TotalViews DESC LIMIT 10;
fn q26254(db: &'static So) -> String {
    let ts_ = tag_stats(db);
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some(((w, t), _)) => [a[0] + w.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
            None => a,
        });
    let dp = user_distinct_posts(db);
    let dc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).count_distinct();
    let tags = drain((&ts_).filt(|a| a[0] > 0));
    let users = drain((&ue).and(&dp).and((&dc).opt()));
    let v = cross_top(tags, |&(_, a)| Reverse(fkey(a[3] as f64 / a[0] as f64)), users, |&(_, ((a, _), _))| Reverse(a[0]), 10);
    rows(v.iter().map(|&((t, a), (u, ((b, n), c)))| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[4]), V::I(a[5]), avg(a[3], a[0])];
        f.extend([user_col(db, u, "name"), V::I(n), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.Reputation, U.DisplayName, U.Views, U.UpVotes, U.DownVotes, COUNT(DISTINCT P.Id) AS TotalPosts,
//            COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions, COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers,
//            SUM(COALESCE(P.Score, 0)) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.Reputation, U.DisplayName, U.Views, U.UpVotes, U.DownVotes),
// TopTags AS (SELECT T.TagName, COUNT(PL.PostId) AS LinkCount FROM Tags T JOIN PostLinks PL ON T.Id = PL.RelatedPostId
//             GROUP BY T.TagName ORDER BY LinkCount DESC LIMIT 10),
// PopularBadges AS (SELECT B.Name, COUNT(B.Id) AS BadgeCount, U.Id AS UserId FROM Badges B JOIN Users U ON B.UserId = U.Id
//                   GROUP BY B.Name, U.Id ORDER BY BadgeCount DESC LIMIT 5)
// SELECT US.UserId, US.DisplayName, US.Reputation, US.Views, US.TotalPosts, US.TotalQuestions, US.TotalAnswers, US.TotalScore, TT.TagName,
//        PB.Name AS PopularBadge
// FROM UserStats US CROSS JOIN TopTags TT LEFT JOIN PopularBadges PB ON US.UserId = PB.UserId
// WHERE US.Reputation > 1000 ORDER BY US.Reputation DESC, US.TotalScore DESC;
fn q7346(db: &'static So) -> String {
    let ups = user_posts(db);
    let tids: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let tt = db.post_link.group_by((&db.post_link.related_post_id).select(&tids).select(&db.tag.tag_name)).fold(0i64, |n, _| n + 1);
    let tt = rel(top_n(drain(&tt), |&(_, n)| Reverse(n), 10));
    let pb = db.badge.group_by((&db.badge.name).and(&db.badge.user)).fold(0i64, |n, _| n + 1);
    let pb = rel(top_n(drain(&pb), |&(_, n)| Reverse(n), 5));
    let pb_keys: MatSet<(Str, Id<User>)> = (&pb).map(|(k, _)| k).collect();
    let pb_by: HashIdx<Id<User>, (Str, Id<User>)> = (&pb_keys).map(|(_, u)| u).inv().collect();
    let mut out = Vec::new();
    db.user
        .with((&db.user.reputation).gt(1000))
        .select(Ident::<User>::new().and(&ups).and((&pb_by).opt()))
        .cross(&tt)
        .drive(|_, (((u, a), b), (t, _))| {
            let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
            f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::S(t), ostr(b.map(|(n, _)| n))]);
            out.push(row(f))
        });
    rows(out)
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(vote_count) AS TotalVotes,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN p.AnswerCount ELSE 0 END) AS QuestionsAnswered, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven,
//            COUNT(DISTINCT ph.Id) AS PostEdits
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(Id) AS vote_count FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// PostSummary AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AverageScore, SUM(p.ViewCount) AS TotalViews
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY pt.Name)
// SELECT ua.DisplayName, ua.PostCount, ua.TotalVotes, ua.QuestionsAnswered, ua.AnswersGiven, ua.PostEdits, ps.PostType, ps.PostCount AS TotalPostsOfType,
//        ps.AverageScore, ps.TotalViews
// FROM UserActivity ua CROSS JOIN PostSummary ps ORDER BY ua.TotalVotes DESC, ua.PostCount DESC, ps.TotalViews DESC LIMIT 100;
fn q8487(db: &'static So) -> String {
    let vc = db.vote.group_by(&db.vote.post).fold(0i64, |n, _| n + 1);
    let Post { post_type_id, answer_count, .. } = &db.post;
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(answer_count.opt()).and((&vc).opt()).and(history_of(db).opt())))
        .fold([0i64; 6], |a, (((t, an), v), _)| {
            let q = if t == 1 { an } else { Some(0) };
            [a[0] + v.is_some() as i64, a[1] + v.unwrap_or(0), a[2] + q.is_some() as i64, a[3] + q.unwrap_or(0), a[4] + (t == 2) as i64, 0]
        });
    let dp = user_distinct_posts(db);
    let dh = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(history_of(db))).count_distinct();
    let Post { score, view_count, creation_date, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)))
        .group_by(ptype_name(db))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, v)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]);
    let users = drain((&ua).and(&dp).and((&dh).opt()));
    let types = drain(&ps);
    let v = cross_top(users, |&(_, ((a, n), _))| (a[0] == 0, Reverse(a[1]), Reverse(n)), types, |&(_, b)| (b[2] == 0, Reverse(b[3])), 100);
    rows(v.iter().map(|&((u, ((a, n), h)), (t, b))| {
        row(vec![user_col(db, u, "name"), V::I(n), nullable(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4]), V::I(h.unwrap_or(0)), V::S(t), V::I(b[0]), avg(b[1], b[0]), nullable(b[3], b[2])])
    }))
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//            SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
//            SUM(u.UpVotes) AS UpVotes, SUM(u.DownVotes) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.PostTypeId, COUNT(*) AS TotalPosts, AVG(p.Score) AS AvgScore, SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.PostTypeId),
// BadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT ua.UserId, ua.DisplayName, ua.PostCount, ua.PositivePosts, ua.NegativePosts, ua.TotalComments, ua.UpVotes, ua.DownVotes, bc.BadgeCount,
//        ps.PostTypeId, ps.TotalPosts, ps.AvgScore, ps.TotalViews
// FROM UserActivity ua LEFT JOIN BadgeCounts bc ON ua.UserId = bc.UserId LEFT JOIN PostStatistics ps ON ua.PostCount > 0 ORDER BY ua.PostCount DESC;
fn q10859(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post).fold(0i64, |n, _| n + 1);
    let User { up_votes, down_votes, .. } = &db.user;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db).select((&db.post.score).and((&cc).opt())).opt()))
        .fold([0i64; 5], |a, ((u, d), p)| match p {
            Some((s, c)) => [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + c.unwrap_or(0), a[3] + u, a[4] + d],
            None => [a[0], a[1], a[2], a[3] + u, a[4] + d],
        });
    let dp = user_distinct_posts(db);
    let bc = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let ps = db
        .post
        .group_by(&db.post.post_type_id)
        .select((&db.post.score).and((&db.post.view_count).opt()))
        .fold([0i64; 4], |a, (s, v)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]);
    let head = |u: Id<User>, a: [i64; 5], n: i64, b: Option<i64>| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), oint(b)]);
        f
    };
    let mut out = Vec::new();
    (&ua).and(&dp).and((&bc).opt()).filt(|((_, n), _)| n > 0).cross(&ps).drive(|(u, t), (((a, n), b), p)| {
        let mut f = head(u, a, n, b);
        f.extend([V::I(t), V::I(p[0]), avg(p[1], p[0]), nullable(p[3], p[2])]);
        out.push(row(f))
    });
    (&ua).and(&dp).and((&bc).opt()).filt(|((_, n), _)| n == 0).drive(|u, ((a, n), b)| {
        let mut f = head(u, a, n, b);
        f.extend([V::Null, V::Null, V::Null, V::Null]);
        out.push(row(f))
    });
    rows(out)
}


// WITH PostStatistics AS (
//     SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//            COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, MAX(p.CreationDate) AS LatestActivityDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title),
// TopUsers AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName ORDER BY AvgReputation DESC LIMIT 10),
// PostDetails AS (
//     SELECT ps.PostId, ps.Title, ps.CommentCount, ps.UpVotes, ps.DownVotes, ps.LatestActivityDate, tu.DisplayName AS TopUser
//     FROM PostStatistics ps JOIN TopUsers tu ON ps.UpVotes > 5)
// SELECT pd.Title, pd.CommentCount, pd.UpVotes, pd.DownVotes, pd.LatestActivityDate, pd.TopUser FROM PostDetails pd
// WHERE pd.LatestActivityDate BETWEEN TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND TIMESTAMP '2024-10-01 12:34:56'
// ORDER BY pd.UpVotes DESC, pd.CommentCount DESC;
fn q9834(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select((&db.post.creation_date).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0, 0, 0, i64::MIN], |a, ((d, c), t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3].max(d)]);
    let tu = db.user.group_by(Ident::<User>::new()).select((&db.user.reputation).and(badges_of(db).opt())).fold((0i64, 0i64), |(n, r), (x, _)| (n + 1, r + x));
    let tu = rel(top_n(drain(&tu), |&(_, (n, r))| Reverse(fkey(r as f64 / n as f64)), 10));
    let lo = add_days(now, -30);
    let mut out = Vec::new();
    (&ps).filt(move |a| a[1] > 5 && a[3] >= lo && a[3] <= now).cross(&tu).drive(|(p, _), (a, (u, _))| {
        out.push(row(vec![title(db, p), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::T(a[3]), user_col(db, u, "name")]))
    });
    rows(out)
}

// WITH UserVotes AS (
//     SELECT v.UserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, uv.VoteCount, uv.UpVotes, uv.DownVotes
//              FROM Users u JOIN UserVotes uv ON u.Id = uv.UserId ORDER BY uv.VoteCount DESC LIMIT 10),
// PostStats AS (
//     SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.CreationDate, COALESCE(a.UserCount, 0) AS ActiveUsers
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(DISTINCT UserId) AS UserCount FROM Comments GROUP BY PostId) a ON p.Id = a.PostId
//     ORDER BY p.ViewCount DESC LIMIT 10)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.AnswerCount, ps.CommentCount,
//        ps.CreationDate, ps.ActiveUsers
// FROM TopUsers tu JOIN PostStats ps ON ps.AnswerCount > 0 ORDER BY tu.VoteCount DESC, ps.ViewCount DESC;
fn q10964(db: &'static So) -> String {
    let uv = db.vote.group_by(&db.vote.user).fold(0i64, |n, _| n + 1);
    let tu = rel(top_n(drain(&uv), |&(_, n)| Reverse(n), 10));
    let au = db.comment.group_by(&db.comment.post).select(&db.comment.user_id).count_distinct();
    let ps = rel(top_n(drain(db.post.select((&db.post.view_count).opt())), |&(_, v)| (v.is_none(), Reverse(v)), 10));
    let mut out = Vec::new();
    (&tu).cross((&ps).map(|(p, _)| p).select(Ident::<Post>::new().with((&db.post.answer_count).gt(0)).and((&au).opt()))).drive(|_, ((u, _), (p, a))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "answers", "comments", "created"]));
        f.push(V::I(a.unwrap_or(0)));
        out.push(row(f))
    });
    rows(out)
}

// WITH TagStats AS (
//     SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.ViewCount > 1000 THEN 1 ELSE 0 END) AS PopularPostsCount,
//            AVG(U.Reputation) AS AverageUserReputation
//     FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' LEFT JOIN Users U ON P.OwnerUserId = U.Id GROUP BY T.TagName),
// PopularTags AS (SELECT TagName FROM TagStats WHERE PopularPostsCount > 5),
// MostActiveUsers AS (
//     SELECT U.DisplayName, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, AVG(P.Score) AS AveragePostScore
//     FROM Users U JOIN Posts P ON P.OwnerUserId = U.Id WHERE U.Reputation > 1000 GROUP BY U.DisplayName ORDER BY PostCount DESC LIMIT 10),
// CombinedStats AS (
//     SELECT PT.TagName, AU.DisplayName, AU.PostCount, AU.TotalViews, AU.AveragePostScore, TS.AverageUserReputation
//     FROM PopularTags PT JOIN TagStats TS ON PT.TagName = TS.TagName JOIN MostActiveUsers AU ON AU.TotalViews > 10000)
// SELECT CT.TagName, CT.DisplayName, CT.PostCount, CT.TotalViews, CT.AveragePostScore, CT.AverageUserReputation
// FROM CombinedStats CT ORDER BY CT.AveragePostScore DESC, CT.TotalViews DESC;
fn q26232(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select((&db.post.view_count).opt().and((&db.post.owner_user).select(&db.user.reputation).opt())))
        .fold([0i64; 3], |a, (v, r)| [a[0] + (v.unwrap_or(0) > 1000) as i64, a[1] + r.is_some() as i64, a[2] + r.unwrap_or(0)]);
    let au = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(&db.user.display_name)
        .select(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score)))
        .fold([0i64; 3], |a, (v, s)| [a[0] + 1, a[1] + v.unwrap_or(0), a[2] + s]);
    let au = rel(top_n(drain(&au), |&(_, a)| Reverse(a[0]), 10));
    let mut out = Vec::new();
    (&ts_).filt(|a| a[0] > 5).cross((&au).filt(|(_, a)| a[1] > 10000)).drive(|(t, _), (a, (name, b))| {
        out.push(row(vec![V::S(t), V::S(name), V::I(b[0]), V::I(b[1]), avg(b[2], b[0]), avg(a[2], a[1])]))
    });
    rows(out)
}

// WITH UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// BadgeStats AS (SELECT b.UserId, COUNT(*) AS BadgeCount, ... GoldBadges, SilverBadges, BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT u.UserId, u.DisplayName, COALESCE(u.PostCount, 0) AS TotalPosts, ..., COALESCE(b.BronzeBadges, 0) AS TotalBronzeBadges
// FROM UserStats u FULL OUTER JOIN BadgeStats b ON u.UserId = b.UserId ORDER BY TotalPosts DESC, TotalUpVotes DESC LIMIT 100;
//
// Badges.UserId never dangles, so the FULL JOIN is a LEFT JOIN.
fn q12662(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 4], |a, (t, v)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]);
    let dp = user_distinct_posts(db);
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain((&dp).and((&us).opt()).and((&bs).opt()));
    let v = top_n(v, |&(_, ((n, a), _))| (Reverse(n), Reverse(a.map_or(0, |a| a[2]))), 100);
    rows(v.iter().map(|&(u, ((n, a), b))| {
        let a = a.unwrap_or([0; 4]);
        let b = b.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.iter().chain(b.iter()).map(|&x| V::I(x)));
        row(f)
    }))
}

// WITH UserBadges AS (
//     SELECT U.Id AS UserId, U.DisplayName, B.Class, COUNT(B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, B.Class),
// PostStats AS (
//     SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(P.Score) AS AverageScore
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// RecentVotes AS (
//     SELECT V.UserId, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes V WHERE V.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY V.UserId)
// SELECT UB.UserId, UB.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount, PS.TotalPosts, PS.TotalQuestions, PS.TotalAnswers,
//        PS.AverageScore, RV.TotalVotes, RV.UpVotes, RV.DownVotes
// FROM UserBadges UB FULL OUTER JOIN PostStats PS ON UB.UserId = PS.OwnerUserId FULL OUTER JOIN RecentVotes RV ON UB.UserId = RV.UserId
// WHERE COALESCE(PS.TotalPosts, 0) + COALESCE(RV.TotalVotes, 0) > 0
// ORDER BY COALESCE(UB.BadgeCount, 0) DESC, PS.TotalPosts DESC NULLS LAST;
//
// Both FULL JOINs are on UB.UserId, so the rows are the UserBadges rows with
// their user's posts and votes, then the PostStats groups and the
// RecentVotes groups that no user matches (the NULL owner and voter among
// them), each alone.
fn q3658(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let ub = db.badge.group_by((&db.badge.user).and(&db.badge.class)).fold(0i64, |n, _| n + 1);
    let Post { owner_user_id, post_type_id, score, creation_date, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(now, -1)))
        .group_by(owner_user_id.opt())
        .select(post_type_id.and(score))
        .fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let rv = db
        .vote
        .with((&db.vote.creation_date).ge(add_months(now, -1)))
        .group_by((&db.vote.user_id).opt())
        .select(&db.vote.vote_type_id)
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let users: MatSet<Option<i64>> = (&db.user.origid).map(Some).collect();
    let pf = |p: Option<[i64; 4]>| match p {
        Some(p) => [V::I(p[0]), V::I(p[1]), V::I(p[2]), avg(p[3], p[0])],
        None => [V::Null, V::Null, V::Null, V::Null],
    };
    let vf = |r: Option<[i64; 3]>| match r {
        Some(r) => [V::I(r[0]), V::I(r[1]), V::I(r[2])],
        None => [V::Null, V::Null, V::Null],
    };
    let ws = (&db.user.origid)
        .map(Some)
        .select((&ps).opt().and((&rv).opt()))
        .filt(|(p, r): (Option<[i64; 4]>, Option<[i64; 3]>)| p.map_or(0, |p| p[0]) + r.map_or(0, |r| r[0]) > 0);
    let keys: MatSet<((Id<User>, i64), i64)> = whole(&ub).select(Same::new().and(&ub)).collect();
    let by_user: HashIdx<Id<User>, ((Id<User>, i64), i64)> = (&keys).map(|((u, _), _)| u).inv().collect();
    let mut out = Vec::new();
    db.user.select(Ident::<User>::new().and((&by_user).map(|(_, n)| n)).and(&ws)).drive(|_, ((u, n), (p, r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(pf(p));
        f.extend(vf(r));
        out.push(row(f))
    });
    db.user.minus(badges_of(db)).select(Ident::<User>::new().and(&ws)).drive(|_, (u, (p, r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(0));
        f.extend(pf(p));
        f.extend(vf(r));
        out.push(row(f))
    });
    (&ps).minus(&users).drive(|_, p| {
        let mut f = vec![V::Null, V::Null, V::I(0)];
        f.extend(pf(Some(p)));
        f.extend(vf(None));
        out.push(row(f))
    });
    (&rv).minus(&users).drive(|_, r| {
        let mut f = vec![V::Null, V::Null, V::I(0)];
        f.extend(pf(None));
        f.extend(vf(Some(r)));
        out.push(row(f))
    });
    rows(out)
}

// WITH TagStatistics AS (
//     SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, AVG(u.Reputation) AS AvgUserReputation
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY t.TagName),
// RecentActivity AS (
//     SELECT ph.PostId, COUNT(DISTINCT ph.UserId) AS EditorCount, MAX(ph.CreationDate) AS LastEditDate
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId),
// MergedData AS (
//     SELECT ts.TagName, ts.PostCount, ts.AnswerCount, ts.QuestionCount, ts.AvgUserReputation, ra.EditorCount, ra.LastEditDate
//     FROM TagStatistics ts LEFT JOIN RecentActivity ra ON ra.PostId = (
//         SELECT p.Id FROM Posts p WHERE p.Tags LIKE '%' || ts.TagName || '%' ORDER BY p.LastActivityDate DESC LIMIT 1))
// SELECT TagName, PostCount, QuestionCount, AnswerCount, AvgUserReputation, EditorCount, LastEditDate FROM MergedData
// ORDER BY PostCount DESC, AvgUserReputation DESC;
//
// The correlated subquery is, per tag, the tagged post with the latest
// LastActivityDate: an arg-max fold over the tag's posts.
fn q26548(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).with((&db.post.creation_date).ge(add_years(date(2024, 10, 1), -1))).select((&db.post.post_type_id).and((&db.post.owner_user).select(&db.user.reputation))))
        .fold([0i64; 4], |a, (t, r)| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + r]);
    let latest = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select(Ident::<Post>::new().and(&db.post.last_activity_date)))
        .fold((i64::MIN, None, false), |(m, p, tie): (i64, Option<Id<Post>>, bool), (q, d)| {
            if d > m { (d, Some(q), false) } else if d == m { (m, p, true) } else { (m, p, tie) }
        });
    let PostHistory { post_history_type_id, creation_date, .. } = &db.post_history;
    let ra = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(&db.post_history.post).select(creation_date).fold(i64::MIN, |m, d| m.max(d));
    let re = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(&db.post_history.post).select(&db.post_history.user_id).count_distinct();
    let mut out = Vec::new();
    let chosen = (&latest).map(|(_, p, tie): (i64, Option<Id<Post>>, bool)| {
        if tie {
            eprintln!("tie in the correlated LIMIT 1");
        }
        p.unwrap()
    });
    (&ts_).and(chosen.select((&ra).and((&re).opt()).opt())).drive(|t, (a, r)| {
        let (e, d) = match r {
            Some((d, e)) => (V::I(e.unwrap_or(0)), V::T(d)),
            None => (V::Null, V::Null),
        };
        out.push(row(vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[2]), V::I(a[1]), avg(a[3], a[0]), e, d]))
    });
    rows(out)
}

// WITH PostStatistics AS (
//     SELECT P.PostTypeId, COUNT(P.Id) AS PostCount, AVG(P.Score) AS AvgScore, SUM(P.ViewCount) AS TotalViews, SUM(P.AnswerCount) AS TotalAnswers,
//            SUM(P.CommentCount) AS TotalComments, SUM(P.FavoriteCount) AS TotalFavorites, MAX(P.CreationDate) AS LastPostDate
//     FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.PostTypeId),
// UserEngagement AS (
//     SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//            SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation)
// SELECT PST.PostTypeId, PST.PostCount, PST.AvgScore, PST.TotalViews, PST.TotalAnswers, PST.TotalComments, PST.TotalFavorites, PST.LastPostDate,
//        SUM(UE.TotalPosts) AS TotalUserPosts, AVG(UE.Reputation) AS AvgUserReputation, SUM(UE.TotalUpVotes) AS TotalUserUpVotes,
//        SUM(UE.TotalDownVotes) AS TotalUserDownVotes
// FROM PostStatistics PST JOIN UserEngagement UE ON PST.PostCount > 0
// GROUP BY PST.PostTypeId, PST.PostCount, PST.AvgScore, PST.TotalViews, PST.TotalAnswers, PST.TotalComments, PST.TotalFavorites, PST.LastPostDate
// ORDER BY PST.PostTypeId;
fn q12020(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, favorite_count, creation_date, .. } = &db.post;
    let pst = db
        .post
        .with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)))
        .group_by(&db.post.post_type_id)
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()).and(creation_date))
        .fold([0, 0, 0, 0, 0, 0, 0, 0, 0, i64::MIN], |a, (((((s, v), an), c), f), d)| {
            [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c, a[7] + f.is_some() as i64, a[8] + f.unwrap_or(0), a[9].max(d)]
        });
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let dp = user_distinct_posts(db);
    let tot = db
        .user
        .select((&db.user.reputation).and(&dp).and((&ue).opt()))
        .fold_flat([0i64; 5], |a, ((r, n), v)| {
            let v = v.unwrap_or((0, 0));
            [a[0] + n, a[1] + r, a[2] + 1, a[3] + v.0, a[4] + v.1]
        });
    let mut out = Vec::new();
    (&pst).filt(|a| a[0] > 0).drive(|t, a| {
        out.push(row(vec![
            V::I(t),
            V::I(a[0]),
            avg(a[1], a[0]),
            nullable(a[3], a[2]),
            nullable(a[5], a[4]),
            V::I(a[6]),
            nullable(a[8], a[7]),
            tmax(a[9]),
            V::I(tot[0]),
            avg(tot[1], tot[2]),
            V::I(tot[3]),
            V::I(tot[4]),
        ]))
    });
    rows(out)
}

// WITH TagStatistics AS (
//     SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, AVG(P.Score) AS AverageScore,
//            COUNT(DISTINCT C.Id) AS CommentCount
//     FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY T.TagName),
// UserEngagement AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT V.PostId) AS VotesGiven, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostHistoryDetails AS (
//     SELECT P.Id AS PostId, MAX(CASE WHEN PH.PostHistoryTypeId = 2 THEN PH.CreationDate END) AS FirstBodyEdit,
//            MAX(CASE WHEN PH.PostHistoryTypeId = 4 THEN PH.CreationDate END) AS FirstTitleEdit, SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount
//     FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.Id)
// SELECT TS.TagName, TS.PostCount, TS.TotalViews, TS.AverageScore, TS.CommentCount, UE.DisplayName, UE.VotesGiven, UE.UpVotes, UE.DownVotes,
//        PHD.FirstBodyEdit, PHD.FirstTitleEdit, PHD.CloseCount
// FROM TagStatistics TS JOIN UserEngagement UE ON UE.UpVotes > 10 OR UE.DownVotes > 10 JOIN PostHistoryDetails PHD ON PHD.CloseCount > 0
// ORDER BY TS.TotalViews DESC, TS.AverageScore DESC LIMIT 25;
fn q27106(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let tposts = || (&by_tag).map(|(p, _)| p);
    let ts_ = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select(tposts().select((&db.post.view_count).opt().and(&db.post.score).and(comments_of(db).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some(((v, s), _)) => [a[0] + v.unwrap_or(0), a[1] + 1, a[2] + s],
            None => a,
        });
    let tp = db.tag.group_by(Ident::<Tag>::new()).select(tposts().opt()).buf_fold(distinct_some);
    let tc = db.tag.group_by(Ident::<Tag>::new()).select(tposts().select(comments_of(db)).opt()).buf_fold(distinct_some);
    let ue = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id)).fold((0i64, 0i64), |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64));
    let uvd = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.post_id)).count_distinct();
    let PostHistory { post_history_type_id, creation_date, .. } = &db.post_history;
    let phd = db
        .post
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(post_history_type_id.and(creation_date)).opt())
        .fold((i64::MIN, i64::MIN, 0i64), |(b, t, c), h| match h {
            Some((k, d)) => (if k == 2 { b.max(d) } else { b }, if k == 4 { t.max(d) } else { t }, c + (k == 10) as i64),
            None => (b, t, c),
        });
    let users = rel(drain((&ue).filt(|(u, d)| u > 10 || d > 10).and((&uvd).opt())));
    let closed = rel(drain((&phd).filt(|(_, _, c)| c > 0)));
    let v = drain((&ts_).and(&tp).and(&tc).cross(&users).cross(&closed));
    let v = top_n(v, |&(_, (((((a, _), _), _)), _))| (Reverse(a[0]), a[1] == 0, Reverse(fkey(a[2] as f64 / a[1].max(1) as f64))), 25);
    rows(v.iter().map(|&(((t, _), _), ((((a, n), c), (u, ((up, dn), g))), (_, (b, ti, k))))| {
        row(vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(n), V::I(a[0]), avg(a[2], a[1]), V::I(c), user_col(db, u, "name"), V::I(g.unwrap_or(0)), V::I(up), V::I(dn), tmax(b), tmax(ti), V::I(k)])
    }))
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//            COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END), 0) AS WikiCount,
//            COALESCE(SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END), 0) AS AcceptedAnswerCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PopularTags AS (SELECT T.TagName, COUNT(P.Id) AS PostCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%'
//                 GROUP BY T.TagName ORDER BY PostCount DESC LIMIT 10),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//                       COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId)
// SELECT US.UserId, US.DisplayName, US.Reputation, US.QuestionCount, US.AnswerCount, US.WikiCount, US.AcceptedAnswerCount, UB.GoldBadges,
//        UB.SilverBadges, UB.BronzeBadges, PT.TagName AS PopularTag
// FROM UserStats US LEFT JOIN UserBadges UB ON US.UserId = UB.UserId CROSS JOIN PopularTags PT WHERE US.Reputation > 1000
// ORDER BY US.Reputation DESC, US.QuestionCount DESC, PT.PostCount DESC;
fn q25034(db: &'static So) -> String {
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&db.post.accepted_answer_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, x)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (t == 1 && x.is_some()) as i64],
            None => a,
        });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ts_ = tag_stats(db);
    let pt = rel(top_n(drain((&ts_).filt(|a| a[0] > 0)), |&(_, a)| Reverse(a[0]), 10));
    let mut out = Vec::new();
    (&us).and((&ub).opt()).cross(&pt).drive(|(u, _), ((a, b), (t, _))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.iter().map(|&x| V::I(x)));
        f.extend(match b {
            Some(b) => [V::I(b[0]), V::I(b[1]), V::I(b[2])],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(db.tag.tag_name.get(t).unwrap()));
        out.push(row(f))
    });
    rows(out)
}

// WITH UserVoteSummary AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
//            SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostInteractionSummary AS (
//     SELECT p.Id AS PostId, p.Title, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//            COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(c.Id) AS CommentCount,
//            COUNT(DISTINCT ph.UserId) AS EditorsCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     GROUP BY p.Id, p.Title),
// CombinedSummary AS (
//     SELECT uvs.UserId, uvs.DisplayName, uvs.TotalVotes, uvs.UpVotesCount, uvs.DownVotesCount, pis.PostId, pis.Title AS PostTitle, pis.TotalUpVotes,
//            pis.TotalDownVotes, pis.CommentCount, pis.EditorsCount
//     FROM UserVoteSummary uvs JOIN PostInteractionSummary pis ON pis.TotalUpVotes > 5)
// SELECT UserId, DisplayName, TotalVotes, UpVotesCount, DownVotesCount, PostId, PostTitle, TotalUpVotes, TotalDownVotes, CommentCount, EditorsCount
// FROM CombinedSummary WHERE TotalVotes > 10 ORDER BY TotalVotes DESC, UpVotesCount DESC;
fn q8746(db: &'static So) -> String {
    let uvs = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id))
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let pis = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 3], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let eds = db.post.group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.user_id)).count_distinct();
    let users = rel(drain((&uvs).filt(|a| a[0] > 10)));
    let mut out = Vec::new();
    (&pis).filt(|a| a[0] > 5).and((&eds).opt()).cross(&users).drive(|(p, _), ((a, e), (u, b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(e.unwrap_or(0))]);
        out.push(row(f))
    });
    rows(out)
}


// WITH PostStats AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount,
//            SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//            MAX(CASE WHEN v.VoteTypeId = 2 THEN v.CreationDate END) AS LastUpVoteDate, MAX(CASE WHEN v.VoteTypeId = 3 THEN v.CreationDate END) AS LastDownVoteDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2023-01-01'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, ... GoldBadges, SilverBadges, BronzeBadges
//                FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT ps.*, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM PostStats ps JOIN Users u ON ps.PostId IN (SELECT DISTINCT OwnerUserId FROM Posts WHERE OwnerUserId IS NOT NULL)
// JOIN UserBadges ub ON u.Id = ub.UserId ORDER BY ps.ViewCount DESC, ps.Score DESC;
//
// The first join condition only tests the post (its Id against the owner
// ids), so each surviving post pairs with every user.
fn q6607(db: &'static So) -> String {
    let owners: MatSet<i64> = (&db.post.owner_user_id).collect();
    let ps = db
        .post
        .with((&db.post.creation_date).ge(date(2023, 1, 1)))
        .with((&db.post.origid).with(&owners))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type_id).and(&db.vote.creation_date)).opt()))
        .fold([0, 0, 0, i64::MIN, i64::MIN], |a, (c, v)| match v {
            Some((t, d)) => [a[0] + c.is_some() as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, if t == 2 { a[3].max(d) } else { a[3] }, if t == 3 { a[4].max(d) } else { a[4] }],
            None => [a[0] + c.is_some() as i64, a[1], a[2], a[3], a[4]],
        });
    let vu = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let mut out = Vec::new();
    (&ps).and((&vu).opt()).cross(&ub).drive(|(p, _), ((a, n), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(n.unwrap_or(0)), V::I(a[1]), V::I(a[2]), tmax(a[3]), tmax(a[4]), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserBadges AS (SELECT U.Id AS UserId, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, ... SilverBadges, BronzeBadges
//                     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostStats AS (
//     SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//            SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews, AVG(EXTRACT(EPOCH FROM (P.CreationDate - U.CreationDate)) / 86400) AS AverageAccountAge
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id GROUP BY P.OwnerUserId),
// UserPerformance AS (
//     SELECT UB.UserId, COALESCE(UB.GoldBadges, 0) AS GoldBadges, ..., COALESCE(PS.TotalScore, 0) AS Score, COALESCE(PS.TotalViews, 0) AS Views, PS.AverageAccountAge
//     FROM UserBadges UB FULL OUTER JOIN PostStats PS ON UB.UserId = PS.OwnerUserId)
// SELECT UP.UserId, U.DisplayName, UP.GoldBadges, UP.SilverBadges, UP.BronzeBadges, UP.QuestionsPosted, UP.AnswersPosted, UP.Score, UP.Views, UP.AverageAccountAge
// FROM UserPerformance UP JOIN Users U ON UP.UserId = U.Id WHERE U.Reputation > 100 ORDER BY UP.Score DESC, U.Reputation DESC LIMIT 10;
//
// PostStats only has users that exist, and UserBadges has them all, so the
// FULL JOIN is a LEFT JOIN from UserBadges.
fn q5008(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let Post { post_type_id, score, view_count, creation_date, owner_user, .. } = &db.post;
    let ps = owned(db)
        .group_by(owner_user)
        .select(post_type_id.and(score).and(view_count.opt()).and(creation_date).and(owner_user.select(&db.user.creation_date)))
        .fold(([0i64; 6], (0.0f64, 0.0f64)), |(a, k), ((((t, s), v), pd), ud)| {
            ([a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + v.is_some() as i64, a[4] + v.unwrap_or(0), a[5] + 1], kahan(k, secs(pd - ud) / 86400.0))
        });
    let v = drain(db.user.with((&db.user.reputation).gt(100)).select(Ident::<User>::new().and(&ub).and((&ps).opt())));
    let v = top_n(v, |&(_, ((u, _), p))| (Reverse(p.map_or(0, |(a, _)| a[2])), Reverse(db.user.reputation.get(u).unwrap())), 10);
    rows(v.iter().map(|&(_, ((u, b), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        f.extend(match p {
            Some((a, k)) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[4]), fmean(k, a[5])],
            None => [V::I(0), V::I(0), V::I(0), V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH RecentPosts AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//            SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(DISTINCT p.Id) AS PostCount
//              FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName ORDER BY TotalScore DESC LIMIT 5),
// PostHistoryInfo AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS MostRecentEdit FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes,
//        phi.EditCount, phi.MostRecentEdit, tu.DisplayName AS TopUserName, tu.TotalScore as TopUserScore
// FROM RecentPosts rp LEFT JOIN PostHistoryInfo phi ON rp.PostId = phi.PostId CROSS JOIN TopUsers tu
// ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 10;
fn q9419(db: &'static So) -> String {
    let rp = owned(db)
        .with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let phi = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let tu = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |s, x| s + x);
    let tu = top_n(drain(&tu), |&(_, s)| Reverse(s), 5);
    let posts = drain((&rp).and((&phi).opt()));
    let key = |p: Id<Post>| {
        let w = db.post.view_count.get(p);
        (Reverse(db.post.score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let v = cross_top(posts, |&(p, _)| key(p), tu, |_| 0, 10);
    rows(v.iter().map(|&((p, (a, h)), (u, s))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        f.extend([user_col(db, u, "name"), V::I(s)]);
        row(f)
    }))
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.Views, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//            COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.Views),
// PostStats AS (
//     SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
//            COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers, SUM(P.Score) AS TotalScore, COUNT(P.Id) FILTER (WHERE P.ClosedDate IS NOT NULL) AS ClosedPosts
//     FROM Posts P GROUP BY P.OwnerUserId),
// CombinedStats AS (
//     SELECT U.DisplayName, U.Reputation, US.UpVotes, US.DownVotes, PS.TotalPosts, PS.Questions, PS.Answers, PS.TotalScore, PS.ClosedPosts
//     FROM UserStats US FULL OUTER JOIN PostStats PS ON US.UserId = PS.OwnerUserId JOIN Users U ON COALESCE(US.UserId, PS.OwnerUserId) = U.Id)
// SELECT C.DisplayName, C.Reputation, C.UpVotes, C.DownVotes, C.TotalPosts, C.Questions, C.Answers, C.TotalScore, C.ClosedPosts,
//        CASE WHEN C.Questions > 0 THEN ROUND(CAST(C.ClosedPosts AS DECIMAL) / C.Questions, 2) ELSE NULL END AS ClosedToQuestionRatio,
//        CASE WHEN C.Answers > 0 THEN ROUND(CAST(C.ClosedPosts AS DECIMAL) / C.Answers, 2) ELSE NULL END AS ClosedToAnswerRatio
// FROM CombinedStats C WHERE C.Reputation > 100 ORDER BY C.Reputation DESC LIMIT 50;
//
// The PostStats-only side of the FULL JOIN is an owner that is no user, and
// the join to Users drops it: a LEFT JOIN from UserStats.
fn q4040(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let ps = owned(db)
        .group_by(&db.post.owner_user)
        .select((&db.post.post_type_id).and(&db.post.score).and((&db.post.closed_date).opt()))
        .fold([0i64; 5], |a, ((t, s), c)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + c.is_some() as i64]);
    let v = drain(db.user.with((&db.user.reputation).gt(100)).select(Ident::<User>::new().and(&us).and((&ps).opt())));
    let v = top_n(v, |&(_, ((u, _), _))| Reverse(db.user.reputation.get(u).unwrap()), 50);
    let ratio = |c: i64, n: i64| if n > 0 { V::F(round2(c as f64 / n as f64)) } else { V::Null };
    rows(v.iter().map(|&(_, ((u, (up, dn)), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(up), V::I(dn)]);
        f.extend(match p {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), ratio(a[4], a[1]), ratio(a[4], a[2])],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserMetrics AS (
//     SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS Questions,
//            COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS Answers, SUM(p.ViewCount) AS TotalViews,
//            SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// PostTypeMetrics AS (
//     SELECT pt.Id AS PostTypeId, pt.Name AS PostTypeName, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Id, pt.Name),
// VoteMetrics AS (
//     SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//            SUM(CASE WHEN v.VoteTypeId IN (1, 4) THEN 1 ELSE 0 END) AS AcceptedVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT um.UserId, um.Reputation, um.TotalPosts, um.Questions, um.Answers, um.TotalViews AS UserTotalViews, um.AcceptedAnswers, ptm.PostTypeName,
//        ptm.PostCount, ptm.TotalViews AS PostTypeTotalViews, ptm.TotalScore AS PostTypeTotalScore, vm.UpVotes, vm.DownVotes, vm.AcceptedVotes
// FROM UserMetrics um JOIN PostTypeMetrics ptm ON (um.Questions > 0 OR um.Answers > 0) JOIN VoteMetrics vm ON um.UserId = vm.PostId
// ORDER BY um.Reputation DESC, um.TotalPosts DESC;
//
// The last join matches a user Id against a post Id, as written.
fn q11583(db: &'static So) -> String {
    let ups = user_posts(db);
    let acc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.accepted_answer_id).opt())).fold(0i64, |n, x| n + x.is_some() as i64);
    let ptm = type_left_posts(db);
    let vm = db
        .post
        .group_by(&db.post.origid)
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + matches!(t, Some(1 | 4)) as i64]);
    let mut out = Vec::new();
    (&ups)
        .filt(|a| a[2] > 0 || a[3] > 0)
        .and((&acc).opt())
        .and((&db.user.origid).select(&vm))
        .cross(&ptm)
        .drive(|(u, t), (((a, x), w), b)| {
            let mut f = ucols(db, u, &["uid", "rep"]);
            f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), V::I(x.unwrap_or(0))]);
            f.extend([tname(db, t), V::I(b[0]), nullable(b[3], b[2]), nullable(b[1], b[0]), V::I(w[0]), V::I(w[1]), V::I(w[2])]);
            out.push(row(f))
        });
    rows(out)
}

fn substr(s: Str, start: i64, len: i64) -> Str {
    let cs: Vec<char> = s.chars().collect();
    let from = (start - 1).max(0) as usize;
    let to = ((start - 1 + len).max(0) as usize).min(cs.len());
    Box::leak(cs.get(from..to.max(from)).map_or(String::new(), |c| c.iter().collect()).into_boxed_str())
}

// WITH TagCount AS (SELECT Tags, COUNT(*) AS PostCount FROM Posts WHERE PostTypeId = 1 GROUP BY Tags),
// UserReputation AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//            SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PopularTags AS (SELECT SUBSTRING(Tags FROM 2 FOR LENGTH(Tags) - 2) AS TagList, COUNT(*) AS UsageCount FROM Posts WHERE PostTypeId = 1
//                 GROUP BY Tags ORDER BY UsageCount DESC LIMIT 10),
// PostDetails AS (
//     SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerName, U.Reputation AS OwnerReputation, TC.PostCount AS TagPostCount,
//            URep.QuestionCount, URep.Upvotes, URep.Downvotes, COALESCE(TC.PostCount, 0) AS TagUsage
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN TagCount TC ON P.Tags = TC.Tags LEFT JOIN UserReputation URep ON U.Id = URep.UserId
//     WHERE P.PostTypeId = 1)
// SELECT PD.Title, PD.CreationDate, PD.OwnerName, PD.OwnerReputation, PD.QuestionCount, PD.Upvotes, PD.Downvotes, PT.TagList, PD.TagUsage
// FROM PostDetails PD JOIN PopularTags PT ON PD.TagPostCount > 0 ORDER BY PD.CreationDate DESC LIMIT 100;
fn q26355(db: &'static So) -> String {
    let q = || db.post.with((&db.post.post_type_id).eq(1));
    let tc = q().group_by((&db.post.tags_str).opt()).fold(0i64, |n, _| n + 1);
    let ur = q()
        .with(&db.post.owner_user)
        .group_by(&db.post.owner_user)
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let uq = q().with(&db.post.owner_user).group_by(&db.post.owner_user).count_distinct();
    let pt = rel(top_n(drain(&tc), |&(_, n)| Reverse(n), 10));
    let pd = q()
        .with(&db.post.owner_user)
        .select(Ident::<Post>::new().and((&db.post.tags_str).select(Same::new().map(Some).select(&tc)).filt(|n| n > 0)).and((&db.post.owner_user).select((&uq).and(&ur).opt())));
    let posts = drain(pd);
    let v = cross_top(posts, |&(p, _)| Reverse(db.post.creation_date.get(p).unwrap()), drain(&pt), |_| 0, 100);
    rows(v.iter().map(|&((_, ((p, n), r)), (_, (tags, _)))| {
        let mut f = post_fields(db, p, &["title", "created", "owner", "rep"]);
        f.extend(match r {
            Some((q, (u, d))) => [V::I(q), V::I(u), V::I(d)],
            None => [V::I(0), V::I(0), V::I(0)],
        });
        f.extend([ostr(tags.map(|t| substr(t, 2, t.chars().count() as i64 - 2))), V::I(n)]);
        row(f)
    }))
}

// WITH UserReputation AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// PopularTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
//                 GROUP BY t.TagName ORDER BY PostCount DESC LIMIT 5),
// PostStats AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
//            (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount,
//            (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVoteCount
//     FROM Posts p WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days')
// SELECT u.DisplayName, u.Reputation, ut.TagName, ps.PostId, ps.Title, ps.CreationDate, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount
// FROM UserReputation u JOIN PostStats ps ON ps.UpVoteCount > 0 JOIN PopularTags ut ON ps.Title LIKE '%' || ut.TagName || '%'
// ORDER BY u.Reputation DESC, ps.UpVoteCount DESC, ps.CommentCount DESC;
//
// CURRENT_TIMESTAMP is when the query runs; the data ends in 2024, so
// PostStats is empty. The comparison is TIMESTAMPTZ, so CreationDate is
// read as New York local time.
fn q6721(db: &'static So) -> String {
    let now = now_utc();
    let ts_ = tag_stats(db);
    let pt = rel(top_n(drain((&ts_).filt(|a| a[0] > 0)), |&(_, a)| Reverse(a[0]), 5));
    let (cp, up, dn) = (comments_per_post(db), votes_of_type(db, 2), votes_of_type(db, 3));
    let ps = db
        .post
        .with((&db.post.creation_date).filt(move |d| ny_to_utc(d) >= now - 30 * DAY_US))
        .select(Ident::<Post>::new().and(&cp).and(&up).and(&dn))
        .filt(|(((_, _), u), _)| u > 0);
    let ps = rel(drain(ps));
    let tn: MatSet<Id<Tag>> = (&pt).map(|(t, _)| t).collect();
    let names: HashIdx<Str, Id<Tag>> = db.tag.with(&tn).select(&db.tag.tag_name).inv().collect();
    let matched = (&ps).and((&ps).map(|(p, _)| p).select(&db.post.title).select_where(&names, |title: Str, name: Str| title.contains(name)));
    let matched = rel(drain(matched));
    let ur = user_posts(db);
    let mut out = Vec::new();
    (&ur).cross(&matched).drive(|(u, _), (_, (_, ((p, (((_, c), uv), dv)), t)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::S(db.tag.tag_name.get(t).unwrap()));
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(c), V::I(uv), V::I(dv)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties,
//            COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// ServerStats AS (
//     SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
//            COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers, COUNT(CASE WHEN PH.PostId IS NOT NULL THEN 1 END) AS TotalHistoryEntries,
//            AVG(P.Score) AS AvgPostScore
//     FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.OwnerUserId)
// SELECT COALESCE(US.UserId, SS.OwnerUserId) AS UserId, COALESCE(US.DisplayName, '') AS DisplayName, COALESCE(US.Reputation, 0) AS Reputation,
//        COALESCE(US.TotalBounties, 0) AS TotalBounties, ..., COALESCE(SS.AvgPostScore, 0) AS AvgPostScore
// FROM UserStats US FULL OUTER JOIN ServerStats SS ON US.UserId = SS.OwnerUserId
// WHERE (COALESCE(US.TotalUpvotes, 0) + COALESCE(US.TotalDownvotes, 0)) > 10 OR (COALESCE(SS.TotalPosts, 0) > 5 AND COALESCE(SS.AvgPostScore, 0) > 10)
// ORDER BY COALESCE(US.Reputation, 0) DESC, COALESCE(SS.TotalPosts, 0) DESC LIMIT 20;
fn q3109(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt().and(&db.vote.vote_type_id)).opt())
        .fold([0i64; 3], |a, v| match v {
            Some((b, t)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
            None => a,
        });
    let ss = db
        .post
        .group_by((&db.post.owner_user_id).opt())
        .select((&db.post.post_type_id).and(&db.post.score).and(history_of(db).opt()))
        .fold([0i64; 5], |a, ((t, s), h)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + h.is_some() as i64, a[4] + s]);
    let users: MatSet<Option<i64>> = (&db.user.origid).map(Some).collect();
    let keep = |u: [i64; 3], s: Option<[i64; 5]>| u[1] + u[2] > 10 || s.is_some_and(|s| s[0] > 5 && s[4] as f64 / s[0] as f64 > 10.0);
    let mut v: Vec<(Option<Id<User>>, Option<i64>, [i64; 3], Option<[i64; 5]>)> = Vec::new();
    db.user.select(Ident::<User>::new().and(&us).and((&db.user.origid).map(Some).select(&ss).opt())).filt(move |((_, u), s)| keep(u, s)).drive(|_, ((u, a), s)| v.push((Some(u), None, a, s)));
    (&ss).minus(&users).filt(move |s| keep([0; 3], Some(s))).drive(|k, s| v.push((None, k, [0; 3], Some(s))));
    let v = top_n(v, |&(u, _, _, s)| (Reverse(u.map_or(0, |u| db.user.reputation.get(u).unwrap())), Reverse(s.map_or(0, |s| s[0]))), 20);
    rows(v.iter().map(|&(u, k, a, s)| {
        let mut f = match u {
            Some(u) => ucols(db, u, &["uid", "name", "rep"]),
            None => vec![oint(k), V::S(""), V::I(0)],
        };
        let s = s.unwrap_or([0; 5]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(s[0]), V::I(s[1]), V::I(s[2]), V::I(s[3])]);
        f.push(if s[0] == 0 { V::F(0.0) } else { avg(s[4], s[0]) });
        row(f)
    }))
}

// WITH TagStats AS (
//     SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(p.ViewCount) AS AvgViewCount, AVG(p.Score) AS AvgScore,
//            MAX(p.CreationDate) AS RecentPostDate
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// UserActivity AS (
//     SELECT u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//            SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
//     FROM Users u LEFT JOIN Votes v ON v.UserId = u.Id LEFT JOIN Comments c ON c.UserId = u.Id LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.DisplayName),
// PostTrends AS (
//     SELECT p.OwnerDisplayName, COUNT(*) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//            SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts
//     FROM Posts p GROUP BY p.OwnerDisplayName)
// SELECT ts.*, ua.DisplayName AS ActiveUser, ua.TotalUpVotes, ua.TotalDownVotes, ua.TotalComments, ua.TotalBadges, pt.TotalPosts, pt.PositivePosts, pt.PopularPosts
// FROM TagStats ts JOIN UserActivity ua ON ua.TotalUpVotes > 0 JOIN PostTrends pt ON pt.TotalPosts > 5
// ORDER BY ts.PostCount DESC, ua.TotalUpVotes DESC, pt.TotalPosts DESC;
//
// UserActivity is the product of each user's votes, comments and badges,
// driven in full: several hundred million rows.
fn q28306(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let Post { post_type_id, view_count, score, creation_date, .. } = &db.post;
    let ts_ = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select(post_type_id.and(view_count.opt()).and(score).and(creation_date)).opt())
        .fold([0, 0, 0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some((((t, v), s), d)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + v.is_some() as i64, a[4] + v.unwrap_or(0), a[5] + s, a[6].max(d)],
            None => a,
        });
    let ua = db
        .user
        .group_by(&db.user.display_name)
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(comments_by(db).opt()).and(badges_of(db).opt()))
        .fold([0i64; 4], |a, ((t, c), b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64, a[3] + b.is_some() as i64]);
    let pt = db
        .post
        .group_by((&db.post.owner_display_name).opt())
        .select(score.and(view_count.opt()))
        .fold([0i64; 3], |a, (s, v)| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (v.unwrap_or(0) > 100) as i64]);
    let users = rel(drain((&ua).filt(|a| a[0] > 0)));
    let trends = rel(drain((&pt).filt(|a| a[0] > 5)));
    let mut out = Vec::new();
    (&ts_).cross(&users).cross(&trends).drive(|((t, _), _), ((a, (name, b)), (o, c))| {
        out.push(row(vec![
            V::S(t),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            avg(a[4], a[3]),
            avg(a[5], a[0]),
            tmax(a[6]),
            V::S(name),
            V::I(b[0]),
            V::I(b[1]),
            V::I(b[2]),
            V::I(b[3]),
            V::I(c[0]),
            V::I(c[1]),
            V::I(c[2]),
        ]));
        let _ = o;
    });
    rows(out)
}

// WITH RECURSIVE UserVotes AS (
//     SELECT u.Id AS UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END) AS TotalScore, COUNT(DISTINCT v.PostId) AS TotalVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostAnalytics AS (
//     SELECT p.Id AS PostId, p.Title, p.ViewCount, COUNT(DISTINCT a.Id) AS AnswerCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS HasAcceptedAnswer,
//            MAX(v.CreationDate) AS LastVoteDate
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.ViewCount),
// PostHistoryChanges AS (
//     SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN ph.CreationDate END) AS LastClosedDate,
//            MAX(CASE WHEN ph.PostHistoryTypeId IN (1, 4, 24) THEN ph.CreationDate END) AS LastEditedDate, COUNT(DISTINCT ph.Id) AS TotalHistoryRecords
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT u.DisplayName AS UserName, u.Reputation, ua.TotalScore, p.Title, p.ViewCount, p.AnswerCount, ph.LastClosedDate, ph.LastEditedDate,
//        ph.TotalHistoryRecords, CASE WHEN p.HasAcceptedAnswer > 0 THEN 'YES' ELSE 'NO' END AS AcceptedAnswer,
//        CASE WHEN ph.LastClosedDate IS NOT NULL AND ph.LastClosedDate >= p.LastVoteDate THEN 'Closed Recently' ELSE 'Active' END AS PostStatus
// FROM Users u JOIN UserVotes ua ON u.Id = ua.UserId JOIN PostAnalytics p ON u.Id IN (SELECT OwnerUserId FROM Posts WHERE PostTypeId = 1)
// JOIN PostHistoryChanges ph ON p.PostId = ph.PostId WHERE ua.TotalVotes > 5 ORDER BY ua.TotalScore DESC, p.ViewCount DESC;
//
// RECURSIVE is written but nothing recurses.
fn q32127(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |s, t| s + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let dv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.post_id)).count_distinct();
    let askers: MatSet<i64> = db.post.with((&db.post.post_type_id).eq(1)).select(&db.post.owner_user_id).collect();
    let q = || db.post.with((&db.post.post_type_id).eq(1));
    let pa = q()
        .group_by(Ident::<Post>::new())
        .select((&db.post.accepted_answer_id).opt().and(children_of(db).opt()).and(votes_of(db).select(&db.vote.creation_date).opt()))
        .fold((0i64, i64::MIN), |(h, m), ((x, _), d)| (h + x.is_some() as i64, d.map_or(m, |d| m.max(d))));
    let pac = q().group_by(Ident::<Post>::new()).select(children_of(db).opt()).buf_fold(distinct_some);
    let PostHistory { post_history_type_id, creation_date, .. } = &db.post_history;
    let phc = db
        .post_history
        .group_by(&db.post_history.post)
        .select(post_history_type_id.and(creation_date))
        .fold((i64::MIN, i64::MIN, 0i64), |(c, e, n), (t, d)| (if matches!(t, 10 | 11) { c.max(d) } else { c }, if matches!(t, 1 | 4 | 24) { e.max(d) } else { e }, n + 1));
    let users = rel(drain(db.user.with((&db.user.origid).with(&askers)).select(Ident::<User>::new().and((&dv).filt(|n| n > 5)).and(&uv))));
    let posts = rel(drain((&pa).and(&pac).and(&phc)));
    let mut out = Vec::new();
    (&users).cross(&posts).drive(|_, ((_, ((u, _), s)), (p, (((h, lv), n), (c, e, k))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(s));
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(n), tmax(c), tmax(e), V::I(k), V::S(if h > 0 { "YES" } else { "NO" }), V::S(if c != i64::MIN && lv != i64::MIN && c >= lv { "Closed Recently" } else { "Active" })]);
        out.push(row(f))
    });
    rows(out)
}


// WITH UserStats AS (
//     SELECT u.Id AS UserId, u.Reputation, u.CreationDate, u.UpVotes, u.DownVotes, COUNT(DISTINCT p.Id) AS PostCount,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//            SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS WikiCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation, u.CreationDate, u.UpVotes, u.DownVotes),
// PostSummary AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViewCount, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY pt.Name),
// BadgeStats AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, ... GoldBadges, SilverBadges, BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT us.UserId, us.Reputation, us.PostCount, us.QuestionCount, us.AnswerCount, us.WikiCount, COALESCE(bs.BadgeCount, 0) AS BadgeCount, ...,
//        ps.PostType, ps.TotalPosts, ps.AverageScore, ps.AverageViewCount, ps.TotalComments
// FROM UserStats us LEFT JOIN BadgeStats bs ON us.UserId = bs.UserId LEFT JOIN PostSummary ps ON true ORDER BY us.Reputation DESC, us.PostCount DESC;
fn q14509(db: &'static So) -> String {
    let ups = user_posts(db);
    let wiki = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id)).fold(0i64, |n, t| n + matches!(t, 3 | 4 | 5) as i64);
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let ps = db
        .post
        .group_by(ptype_name(db))
        .select((&db.post.score).and((&db.post.view_count).opt()).and(comments_of(db).opt()))
        .fold([0i64; 4], |a, ((s, v), _)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]);
    let pc = db.post.group_by(ptype_name(db)).select(comments_of(db)).count_distinct();
    let ps = left_all(drain((&ps).and((&pc).opt())));
    let mut out = Vec::new();
    (&ups).and((&wiki).opt()).and((&bs).opt()).cross(&ps).drive(|(u, _), (((a, w), b), x)| {
        let b = b.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(w.unwrap_or(0)), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        f.extend(match x {
            Some((t, (c, n))) => [V::S(t), V::I(c[0]), avg(c[1], c[0]), avg(c[3], c[2]), V::I(n.unwrap_or(0))],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH UserVoteSummary AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (
//     SELECT p.Id AS PostId, p.Title, p.ParentId, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount,
//            COUNT(DISTINCT ph.Id) FILTER (WHERE ph.PostHistoryTypeId IN (10, 11)) AS CloseReopenCount, SUM(CASE WHEN ph.PostHistoryTypeId = 2 THEN 1 ELSE 0 END) AS EditBodyCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title, p.ParentId, p.ViewCount),
// AnsweredPosts AS (
//     SELECT p.Id AS PostId, p.AcceptedAnswerId, p.Title, COALESCE(a.OwnerDisplayName, 'N/A') AS AcceptedAnswerOwner
//     FROM Posts p LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id WHERE p.PostTypeId = 1)
// SELECT ps.PostId, ps.Title, ps.ViewCount, ps.CommentCount, aps.AcceptedAnswerOwner, uvs.DisplayName AS UserVoteDisplayName, uvs.TotalVotes,
//        uvs.UpVotes, uvs.DownVotes, ps.CloseReopenCount, ps.EditBodyCount
// FROM PostStatistics ps LEFT JOIN AnsweredPosts aps ON ps.PostId = aps.PostId
// LEFT JOIN UserVoteSummary uvs ON ps.PostId = (SELECT v.PostId FROM Votes v WHERE v.UserId = uvs.UserId ORDER BY v.CreationDate DESC LIMIT 1)
// WHERE ps.ViewCount > 100 ORDER BY ps.CommentCount DESC, ps.ViewCount DESC LIMIT 20;
//
// The correlated subquery is each user's latest vote: an arg-max fold per
// voter, joined back to the post it names.
fn q4710(db: &'static So) -> String {
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let latest = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.creation_date).and(&db.vote.post_id)))
        .fold((i64::MIN, 0i64, false), |(m, p, tie), (d, q)| if d > m { (d, q, false) } else if d == m { (m, p, tie || q != p) } else { (m, p, tie) });
    let pids: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let by_post: HashIdx<Id<Post>, Id<User>> = (&latest)
        .map(|(_, p, tie): (i64, i64, bool)| {
            if tie {
                eprintln!("tie in the correlated LIMIT 1");
            }
            p
        })
        .select(&pids)
        .inv()
        .collect();
    let PostHistory { post_history_type_id, .. } = &db.post_history;
    let ps = db
        .post
        .with((&db.post.view_count).gt(100))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(post_history_type_id).opt()))
        .fold(0i64, |n, (_, t)| n + (t == Some(2)) as i64);
    let dc = db.post.with((&db.post.view_count).gt(100)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(distinct_some);
    let cr = db.post.with((&db.post.view_count).gt(100)).group_by(Ident::<Post>::new()).select(history_of(db).with(post_history_type_id.is_in([10, 11])).opt()).buf_fold(distinct_some);
    let aps = db.post.with((&db.post.post_type_id).eq(1)).select((&db.post.accepted_answer).select(&db.post.owner_display_name).opt());
    let v = drain((&ps).and(&dc).and(&cr).and(aps.opt()).and((&by_post).select(Ident::<User>::new().and(&uvs)).opt()));
    let v = top_n(v, |&(p, ((((_, c), _), _), _))| (Reverse(c), Reverse(db.post.view_count.get(p))), 20);
    rows(v.iter().map(|&(p, ((((e, c), k), a), u))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(c), a.map_or(V::Null, |a| V::S(a.unwrap_or("N/A")))]);
        f.extend(match u {
            Some((u, b)) => [user_col(db, u, "name"), V::I(b[0]), V::I(b[1]), V::I(b[2])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend([V::I(k), V::I(e)]);
        row(f)
    }))
}

// WITH TagCounts AS (
//     SELECT t.TagName, COUNT(*) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// RecentActivity AS (
//     SELECT p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS EditCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON c.PostId = p.Id
//     LEFT JOIN PostHistory ph ON ph.PostId = p.Id AND ph.PostHistoryTypeId IN (4, 5)
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Title, p.CreationDate, u.DisplayName),
// TopUsers AS (
//     SELECT u.DisplayName, u.Reputation, COUNT(*) AS PostsCount FROM Users u JOIN Posts p ON p.OwnerUserId = u.Id
//     WHERE u.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.DisplayName, u.Reputation ORDER BY u.Reputation DESC LIMIT 10)
// SELECT tc.TagName, tc.PostCount, tc.QuestionCount, tc.AnswerCount, ra.Title AS RecentPost, ra.CreationDate AS RecentPostDate, ra.OwnerDisplayName AS RecentPostOwner,
//        ra.CommentCount AS RecentCommentCount, tu.DisplayName AS TopUser, tu.Reputation AS TopUserReputation, tu.PostsCount AS TopUserPostCount
// FROM TagCounts tc JOIN RecentActivity ra ON tc.TagName = ANY(string_to_array(ra.Title, ' ')) JOIN TopUsers tu ON ra.OwnerDisplayName = tu.DisplayName
// ORDER BY tc.PostCount DESC, ra.CreationDate DESC, tu.Reputation DESC LIMIT 20;
fn q27717(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let tc = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select(&db.post.post_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64]);
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let ra = owned(db)
        .with(creation_date.gt(add_days(now, -30)))
        .group_by(title.opt().and(creation_date).and(owner_user.select(&db.user.display_name)))
        .select(comments_of(db).opt().and(history_of(db).with((&db.post_history.post_history_type_id).is_in([4, 5])).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let tu = db
        .user
        .with((&db.user.creation_date).lt(add_years(now, -1)))
        .group_by((&db.user.display_name).and(&db.user.reputation))
        .select(posts_of(db))
        .fold(0i64, |n, _| n + 1);
    let tu = rel(top_n(drain(&tu), |&((_, r), _)| Reverse(r), 10));
    let tu_keys: MatSet<((Str, i64), i64)> = (&tu).collect();
    let tu_by: HashIdx<Str, ((Str, i64), i64)> = (&tu_keys).map(|((n, _), _)| n).inv().collect();
    let rows_ra = rel(drain(&ra));
    let words = (&rows_ra).flat_map(|(((t, _), _), _): (((Option<Str>, i64), Str), i64)| {
        let mut w: Vec<Str> = t.map_or(Vec::new(), |t| t.split(' ').collect());
        w.sort_unstable();
        w.dedup();
        w
    });
    let joined = (&rows_ra).and(words.select(Same::new().and(&tc))).and((&rows_ra).map(|((_, n), _)| n).select(&tu_by));
    let v = top_n(drain(joined), |&(_, (((((_, d), _), _), (_, a)), ((_, r), _)))| (Reverse(a[0]), Reverse(d), Reverse(r)), 20);
    rows(v.iter().map(|&(_, (((((t, d), o), c), (g, a)), ((n, r), k)))| {
        row(vec![V::S(g), V::I(a[0]), V::I(a[1]), V::I(a[2]), ostr(t), V::T(d), V::S(o), V::I(c), V::S(n), V::I(r), V::I(k)])
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, ... GoldBadges, SilverBadges, BronzeBadges
//                          FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(1) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//                           SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, AVG(p.Score) AS AverageScore FROM Posts p GROUP BY p.OwnerUserId),
// UserPostBadgeStats AS (
//     SELECT ubc.UserId, ubc.DisplayName, ps.TotalPosts, ps.Questions, ps.Answers, ps.AverageScore, ubc.BadgeCount, ubc.GoldBadges, ubc.SilverBadges, ubc.BronzeBadges
//     FROM UserBadgeCounts ubc JOIN PostStatistics ps ON ubc.UserId = ps.OwnerUserId)
// SELECT upbs.DisplayName, COALESCE(upbs.TotalPosts, 0) AS TotalPosts, ..., COALESCE(upbs.BronzeBadges, 0) AS BronzeBadges,
//        CASE WHEN upbs.BadgeCount > 10 AND upbs.AverageScore > 50 THEN 'High Engagement'
//             WHEN upbs.BadgeCount BETWEEN 5 AND 10 AND upbs.AverageScore BETWEEN 20 AND 50 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM UserBadgeCounts ubc FULL OUTER JOIN UserPostBadgeStats upbs ON ubc.UserId = upbs.UserId
// WHERE ubc.BadgeCount IS NOT NULL OR upbs.TotalPosts IS NOT NULL ORDER BY BadgeCount DESC, AverageScore DESC;
//
// Every UserPostBadgeStats row is some user's, so the FULL JOIN is a LEFT
// JOIN from UserBadgeCounts, and the WHERE keeps every row.
fn q4738(db: &'static So) -> String {
    let ubc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let mut out = Vec::new();
    (&ubc).and((&ps).opt()).drive(|u, (b, p)| {
        let name = ostr(p.map(|_| db.user.display_name.get(u).unwrap()));
        let f = match p {
            Some(p) => {
                let avg_s = p[3] as f64 / p[0] as f64;
                let level = if b[0] > 10 && avg_s > 50.0 {
                    "High Engagement"
                } else if (5..=10).contains(&b[0]) && (20.0..=50.0).contains(&avg_s) {
                    "Moderate Engagement"
                } else {
                    "Low Engagement"
                };
                vec![name, V::I(p[0]), V::I(p[1]), V::I(p[2]), V::F(avg_s), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3]), V::S(level)]
            }
            None => vec![name, V::I(0), V::I(0), V::I(0), V::F(0.0), V::I(0), V::I(0), V::I(0), V::I(0), V::S("Low Engagement")],
        };
        out.push(row(f))
    });
    rows(out)
}

// WITH TagStatistics AS (
//     SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(CASE WHEN PT.Name = 'Answer' THEN 1 ELSE 0 END, 0)) AS AnswerCount,
//            SUM(COALESCE(CASE WHEN PT.Name = 'Question' THEN 1 ELSE 0 END, 0)) AS QuestionCount, MAX(P.CreationDate) AS LatestPostDate
//     FROM Tags T LEFT JOIN Posts P ON POSITION(T.TagName IN P.Tags) > 0 LEFT JOIN PostTypes PT ON PT.Id = P.PostTypeId GROUP BY T.TagName),
// MostActiveUsers AS (
//     SELECT U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN PT.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount,
//            SUM(CASE WHEN PT.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount
//     FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id LEFT JOIN PostTypes PT ON PT.Id = P.PostTypeId
//     GROUP BY U.DisplayName, U.Reputation HAVING COUNT(DISTINCT P.Id) > 5),
// FrequentEditors AS (
//     SELECT U.DisplayName, COUNT(PH.Id) AS EditCount FROM PostHistory PH JOIN Users U ON PH.UserId = U.Id
//     WHERE PH.PostHistoryTypeId IN (4, 5, 6) GROUP BY U.DisplayName HAVING COUNT(PH.Id) > 10),
// FinalReport AS (
//     SELECT TS.TagName, TS.PostCount, TS.AnswerCount, TS.QuestionCount, TS.LatestPostDate, AU.DisplayName AS ActiveUser, EU.DisplayName AS Editor, EU.EditCount
//     FROM TagStatistics TS
//     JOIN (SELECT DisplayName, Reputation, PostCount, AnswerCount, QuestionCount FROM MostActiveUsers ORDER BY PostCount DESC LIMIT 1) AU ON TS.PostCount > 10
//     JOIN (SELECT DisplayName, EditCount FROM FrequentEditors ORDER BY EditCount DESC LIMIT 1) EU ON TS.PostCount > 5)
// SELECT TagName, PostCount, AnswerCount, QuestionCount, LatestPostDate, ActiveUser, Editor, EditCount FROM FinalReport ORDER BY PostCount DESC, AnswerCount DESC;
fn q27183(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select(ptype_name(db).and(&db.post.creation_date)).opt())
        .fold([0, 0, 0, i64::MIN], |a, p| match p {
            Some((n, d)) => [a[0] + 1, a[1] + (n == "Answer") as i64, a[2] + (n == "Question") as i64, a[3].max(d)],
            None => a,
        });
    let au = db.user.group_by((&db.user.display_name).and(&db.user.reputation)).select(posts_of(db).opt()).buf_fold(distinct_some);
    let au = rel(top_n(drain((&au).filt(|n| n > 5)), |&(_, n)| Reverse(n), 1));
    let eu = db
        .post_history
        .with((&db.post_history.post_history_type_id).is_in([4, 5, 6]))
        .group_by((&db.post_history.user).select(&db.user.display_name))
        .fold(0i64, |n, _| n + 1);
    let eu = rel(top_n(drain((&eu).filt(|n| n > 10)), |&(_, n)| Reverse(n), 1));
    let mut out = Vec::new();
    (&ts_).filt(|a| a[0] > 10).cross(&au).cross(&eu).drive(|((t, _), _), ((a, ((n, _), _)), (e, k))| {
        out.push(row(vec![V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), tmax(a[3]), V::S(n), V::S(e), V::I(k)]))
    });
    rows(out)
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount,
//            SUM(v.BountyAmount) AS TotalBounties, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     WHERE u.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName, u.Reputation),
// TagStatistics AS (
//     SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AverageScore
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY t.TagName),
// PostHistoryDetails AS (
//     SELECT ph.PostId, p.Title, p.Body, ph.UserDisplayName, ph.CreationDate, p.Tags, ph.Comment, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseVoteCount
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id GROUP BY ph.PostId, p.Title, p.Body, ph.UserDisplayName, ph.CreationDate, p.Tags, ph.Comment)
// SELECT ua.DisplayName AS User, ..., ts.TagName AS PopularTag, ..., phd.Title AS RecentPostTitle, ..., phd.CloseVoteCount AS TotalCloseVotes
// FROM UserActivity ua JOIN TagStatistics ts ON ua.PostCount > 0 JOIN PostHistoryDetails phd ON ua.UserId = phd.PostId
// WHERE ua.Reputation > 1000 ORDER BY ua.Reputation DESC, ts.PostCount DESC LIMIT 50;
//
// The last join matches a user Id against a post Id, as written.
fn q28839(db: &'static So) -> String {
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let recent = || db.user.with((&db.user.creation_date).ge(since)).with((&db.user.reputation).gt(1000));
    let ua = recent()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(comments_by(db).opt()).and(votes_by(db).select((&db.vote.bounty_amount).opt().and(&db.vote.vote_type_id)).opt()))
        .fold([0i64; 4], |a, (_, v)| match v {
            Some((b, t)) => [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + (t == 2) as i64, a[3] + (t == 3) as i64],
            None => a,
        });
    let dp = recent().group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let dc = recent().group_by(Ident::<User>::new()).select(comments_by(db).opt()).buf_fold(distinct_some);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let tsx = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).with((&db.post.creation_date).ge(since)).select((&db.post.view_count).opt().and(&db.post.score)))
        .fold([0i64; 4], |a, (v, s)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s]);
    let PostHistory { post, user_display_name, creation_date, comment, post_history_type_id, .. } = &db.post_history;
    let phd = db
        .post_history
        .group_by(post.and(user_display_name.opt()).and(creation_date).and(comment.opt()))
        .select(post_history_type_id)
        .fold(0i64, |n, t| n + (t == 10) as i64);
    let phd_keys: MatSet<(((Id<Post>, Option<Str>), i64), Option<Str>)> = whole(&phd).collect();
    let phd_by: HashIdx<i64, (((Id<Post>, Option<Str>), i64), Option<Str>)> = (&phd_keys).map(|(((p, _), _), _)| db.post.origid.get(p).unwrap()).inv().collect();
    let users = drain((&dp).filt(|n| n > 0).and(&dc).and(&ua).and((&db.user.origid).select((&phd_by).select(Same::new().and(&phd)))));
    let tags = drain(&tsx);
    let v = cross_top(users, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), tags, |&(_, a)| Reverse(a[0]), 50);
    rows(v.iter().map(|&((u, (((n, c), a), ((((p, e), d), m), k))), (t, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(c), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3])]);
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(b[0]), nullable(b[2], b[1]), avg(b[3], b[0])]);
        f.extend(post_fields(db, p, &["title", "body"]));
        f.extend([ostr(e), V::T(d), post_fields(db, p, &["tags"]).remove(0), ostr(m), V::I(k)]);
        row(f)
    }))
}

// WITH PostStatistics AS (
//     SELECT p.PostTypeId, COUNT(*) AS TotalPosts, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews,
//            AVG(COALESCE(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)), 0)) AS AvgPostAgeInSeconds
//     FROM Posts p GROUP BY p.PostTypeId),
// UserStatistics AS (SELECT u.Id AS UserId, AVG(u.Reputation) AS AvgReputation, COUNT(DISTINCT b.Id) AS TotalBadges
//                    FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT pt.Name AS PostType, ps.TotalPosts, ps.TotalScore, ps.TotalViews, ps.AvgPostAgeInSeconds, us.AvgReputation, us.TotalBadges
// FROM PostTypes pt JOIN PostStatistics ps ON pt.Id = ps.PostTypeId LEFT JOIN UserStatistics us ON us.UserId IS NOT NULL ORDER BY ps.TotalPosts DESC;
fn q11598(db: &'static So) -> String {
    let Post { score, view_count, last_activity_date, creation_date, .. } = &db.post;
    let ps = db
        .post
        .group_by(&db.post.post_type)
        .select(score.and(view_count.opt()).and(last_activity_date).and(creation_date))
        .fold(([0i64; 4], (0.0f64, 0.0f64)), |(a, k), (((s, v), la), cd)| ([a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)], kahan(k, secs(la - cd))));
    let us = db.user.group_by(Ident::<User>::new()).select((&db.user.reputation).and(badges_of(db).opt())).fold((0i64, 0i64, 0i64), |(n, r, b), (x, y)| (n + 1, r + x, b + y.is_some() as i64));
    let us = left_all(drain(&us));
    let mut out = Vec::new();
    (&ps).cross(&us).drive(|(t, _), ((a, k), x)| {
        let mut f = vec![tname(db, t), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), fmean(k, a[0])];
        f.extend(match x {
            Some((_, (n, r, b))) => [avg(r, n), V::I(b)],
            None => [V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH PostTagCounts AS (
//     SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagFrequencies AS (SELECT Tag, COUNT(*) AS Frequency FROM PostTagCounts GROUP BY Tag),
// ActiveUsers AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount FROM Users u JOIN Posts p ON p.OwnerUserId = u.Id WHERE u.Reputation > 1000 GROUP BY u.Id)
// SELECT tf.Tag, tf.Frequency, au.UserId, au.PostCount,
//        (SELECT COUNT(DISTINCT p.Id) FROM Posts p WHERE p.Tags LIKE '%' || tf.Tag || '%') AS TotalPostsWithTag
// FROM TagFrequencies tf CROSS JOIN ActiveUsers au WHERE tf.Frequency > 5 ORDER BY tf.Frequency DESC, au.PostCount DESC;
//
// The correlated count is of posts whose Tags string contains the element:
// the elements are the tag names, so the same element-contains join as
// `tag_mentions`, keyed by element instead of by Tag.
fn q25573(db: &'static So) -> String {
    let q = db.post.with((&db.post.post_type_id).eq(1));
    let tf = q.select((&db.post.tags_str).flat_map(tag_list)).group_by(Same::new()).fold(0i64, |n, _| n + 1);
    let elems: MatSet<Str> = (&db.post.tags_str).flat_map(tag_list).collect();
    let contains: HashIdx<Str, Str> = (&elems).select_where(&elems, |x: Str, e: Str| x.contains(e)).collect();
    let pairs: MatSet<(Id<Post>, Str)> = db.post.select(Ident::<Post>::new().and((&db.post.tags_str).flat_map(tag_list).select(&contains))).collect();
    let with_tag = (&pairs).group_by(Same::<(Id<Post>, Str)>::new().map(|(_, e)| e)).fold(0i64, |n, _| n + 1);
    let au = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&tf).filt(|n| n > 5).and(&with_tag).cross(&au).drive(|(e, u), ((n, w), c)| {
        out.push(row(vec![V::S(e), V::I(n), user_col(db, u, "uid"), V::I(c), V::I(w)]))
    });
    rows(out)
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//            SUM(CASE WHEN B.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// PostStats AS (SELECT PT.Name AS PostType, COUNT(P.Id) AS TotalPosts, AVG(P.ViewCount) AS AvgViewCount, AVG(P.Score) AS AvgScore
//               FROM Posts P INNER JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY PT.Name)
// SELECT US.UserId, US.Reputation, US.PostCount, US.TotalScore, US.BadgeCount, PS.PostType, PS.TotalPosts, PS.AvgViewCount, PS.AvgScore
// FROM UserStats US CROSS JOIN PostStats PS ORDER BY US.Reputation DESC, US.TotalScore DESC, PS.AvgViewCount DESC;
fn q13960(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.score).opt().and(badges_of(db).opt()))
        .fold((0i64, 0i64), |(s, b), (x, y)| (s + x.unwrap_or(0), b + y.is_some() as i64));
    let dp = user_distinct_posts(db);
    let ps = stats_by_type(db);
    let mut out = Vec::new();
    (&us).and(&dp).cross(&ps).drive(|(u, t), (((s, bc), n), b)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::I(s), V::I(bc)]);
        f.extend([V::S(t), V::I(b[0]), avg(b[3], b[2]), avg(b[1], b[0])]);
        out.push(row(f))
    });
    rows(out)
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        p.ViewCount, p.Score, T.TagName, COALESCE(TAG_COUNT.Count, 0) AS TagCount
// FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN (SELECT PostId, COUNT(*) AS Count FROM Votes GROUP BY PostId) TAG_COUNT ON p.Id = TAG_COUNT.PostId
// LEFT JOIN (SELECT pt.Id, STRING_AGG(t.TagName, ', ') AS TagName FROM Posts p JOIN Tags t ON t.ExcerptPostId = p.Id
//            JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Id) AS T ON p.PostTypeId = T.Id
// WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.ViewCount, p.Score, T.TagName, TAG_COUNT.Count
// ORDER BY p.CreationDate DESC LIMIT 100;
//
// The STRING_AGG has no ORDER BY, but it only exists for the excerpt posts'
// type, and none of the hundred newest posts is one, so it is NULL
// throughout.
fn q10806(db: &'static So) -> String {
    let tn = db.tag.group_by((&db.tag.excerpt_post).select(&db.post.post_type)).select(&db.tag.tag_name).buf_fold(|v| {
        let mut v: Vec<Str> = v.into_iter().collect();
        v.sort_unstable();
        Box::leak(v.join(", ").into_boxed_str()) as Str
    });
    let vc = db.vote.group_by(&db.vote.post).fold(0i64, |n, _| n + 1);
    let recent = db.post.with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let agg = recent
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(c, v), (x, y)| (c + x.is_some() as i64, v + y.is_some() as i64));
    let v = drain((&agg).and((&vc).opt()).and((&db.post.post_type).select(&tn).opt()));
    let v = top_n(v, |&(p, _)| Reverse(db.post.creation_date.get(p).unwrap()), 100);
    rows(v.iter().map(|&(p, (((c, n), k), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(post_fields(db, p, &["views", "score"]));
        f.extend([ostr(t), V::I(k.unwrap_or(0))]);
        row(f)
    }))
}


// WITH FrequentTags AS (SELECT TRIM(UNNEST(string_to_array(SUBSTRING(Tags FROM 2 FOR LENGTH(Tags) - 2), '><'))) AS Tag FROM Posts WHERE PostTypeId = 1),
// TagUsage AS (SELECT Tag, COUNT(*) AS UsageCount FROM FrequentTags GROUP BY Tag ORDER BY UsageCount DESC LIMIT 10),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//                    WHERE p.PostTypeId IN (1, 2) GROUP BY u.Id, u.Reputation),
// CombinedData AS (SELECT tr.Tag, ur.UserId, ur.Reputation, ur.PostCount FROM TagUsage tr CROSS JOIN UserReputation ur)
// SELECT Tag, COUNT(DISTINCT UserId) AS UserCount, AVG(Reputation) AS AverageReputation, SUM(PostCount) AS TotalPosts
// FROM CombinedData GROUP BY Tag ORDER BY UserCount DESC;
fn q27757(db: &'static So) -> String {
    let q = db.post.with((&db.post.post_type_id).eq(1));
    let tu = q.select((&db.post.tags_str).flat_map(|t| tag_list(t).map(|e| e.trim()))).group_by(Same::new()).fold(0i64, |n, _| n + 1);
    let tu = rel(top_n(drain(&tu), |&(_, n)| Reverse(n), 10));
    let ur = db.post.with((&db.post.post_type_id).is_in([1, 2])).with(&db.post.owner_user).group_by(&db.post.owner_user).count_distinct();
    let urr = || db.user.select(Ident::<User>::new().and(&db.user.reputation).and(&ur));
    type Row = ((Str, i64), ((Id<User>, i64), i64));
    let key = || Same::<Row>::new().map(|((t, _), _)| t);
    let agg = (&tu).cross(urr()).group_by(key()).fold((0i64, 0i64, 0i64), |(k, r, p), (_, ((_, rep), pc))| (k + 1, r + rep, p + pc));
    let du = (&tu).cross(urr()).group_by(key()).map(|(_, ((u, _), _)): Row| u).count_distinct();
    let mut out = Vec::new();
    (&du).and(&agg).drive(|t, (n, (k, r, p))| out.push(row(vec![V::S(t), V::I(n), avg(r, k), V::I(p)])));
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.ViewCount) AS AverageViewCount, AVG(p.Score) AS AverageScore,
//            SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.DisplayName, COUNT(DISTINCT b.Id) AS TotalBadges, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//            COALESCE(SUM(CASE WHEN v.CreationDate IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalVotes
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.DisplayName)
// SELECT ps.PostType, ps.TotalPosts, ps.AverageViewCount, ps.AverageScore, ps.TotalAcceptedAnswers, us.DisplayName, us.TotalBadges, us.TotalBounty, us.TotalVotes
// FROM PostStats ps JOIN UserStats us ON us.TotalVotes > 0 ORDER BY ps.AverageViewCount DESC, ps.TotalPosts DESC;
fn q13927(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let acc = db.post.group_by(ptype_name(db)).select((&db.post.accepted_answer_id).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let us = db
        .user
        .group_by(&db.user.display_name)
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64), |(b, v), (_, x)| (b + x.flatten().unwrap_or(0), v + x.is_some() as i64));
    let db_ = db.user.group_by(&db.user.display_name).select(badges_of(db).opt()).buf_fold(distinct_some);
    let users = rel(drain((&us).filt(|(_, v)| v > 0).and(&db_)));
    let mut out = Vec::new();
    (&ps).and(&acc).cross(&users).drive(|(t, _), ((a, x), (name, ((b, v), n)))| {
        out.push(row(vec![V::S(t), V::I(a[0]), avg(a[3], a[2]), avg(a[1], a[0]), V::I(x), V::S(name), V::I(n), V::I(b), V::I(v)]))
    });
    rows(out)
}

// WITH PopularTags AS (
//     SELECT T.TagName, T.Count, P.Title, COUNT(DISTINCT C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//            SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' AND P.PostTypeId = 1 LEFT JOIN Comments C ON C.PostId = P.Id
//     LEFT JOIN Votes V ON V.PostId = P.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY T.TagName, T.Count, P.Title),
// TopTags AS (
//     SELECT TagName, SUM(CommentCount) AS TotalComments, SUM(Upvotes) AS TotalUpvotes, SUM(Downvotes) AS TotalDownvotes, COUNT(DISTINCT Title) AS PostCount
//     FROM PopularTags GROUP BY TagName HAVING COUNT(DISTINCT Title) > 5 ORDER BY TotalUpvotes DESC LIMIT 10)
// SELECT TT.TagName, TT.TotalComments, TT.TotalUpvotes, TT.TotalDownvotes, TT.PostCount,
//        ROUND((TotalUpvotes::FLOAT / NULLIF(PostCount, 0)) * 100, 2) AS UpvotePercentage
// FROM TopTags TT ORDER BY UpvotePercentage DESC;
//
// FLOAT is DuckDB's 4-byte REAL, so the percentage is computed and rounded
// in f32.
fn q29656(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let Post { post_type_id, creation_date, title, .. } = &db.post;
    let recent: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).collect();
    type Pair = (Id<Post>, Id<Tag>);
    let pairs: MatSet<Pair> = (&lt).with(Same::<Pair>::new().map(|(p, _)| p).with(&recent)).collect();
    let key = || Same::<Pair>::new().map(|(_, t)| t).and(Same::<Pair>::new().map(|(p, _)| p).select(title.opt()));
    let post = || Same::<Pair>::new().map(|(p, _)| p);
    let pt = (&pairs)
        .group_by(key())
        .select(post().select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold((0i64, 0i64), |(u, d), (_, t)| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let pc = (&pairs).group_by(key()).select(post().select(comments_of(db).opt())).buf_fold(distinct_some);
    type Group = ((Id<Tag>, Option<Str>), ((i64, i64), i64));
    let tt = whole(&pt)
        .select(Same::new().and((&pt).and(&pc)))
        .group_by(Same::<Group>::new().map(|((t, _), _)| t))
        .fold([0i64; 4], |a, ((_, ti), ((u, d), c)): Group| [a[0] + c, a[1] + u, a[2] + d, a[3] + ti.is_some() as i64]);
    let tt = top_n(drain((&tt).filt(|a| a[3] > 5)), |&(_, a)| Reverse(a[1]), 10);
    rows(tt.iter().map(|&(t, a)| {
        let pct = if a[3] == 0 { V::Null } else { V::F((((a[1] as f32 / a[3] as f32) * 100.0 * 100.0).round() / 100.0) as f64) };
        row(vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), pct])
    }))
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.Reputation, U.Views, U.UpVotes, U.DownVotes, COUNT(DISTINCT P.Id) AS PostCount,
//            COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//            COALESCE(SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END), 0) AS AcceptedAnswerCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.Reputation, U.Views, U.UpVotes, U.DownVotes),
// PostTypesStats AS (SELECT PT.Name AS PostTypeName, COUNT(P.Id) AS TotalPosts, SUM(P.ViewCount) AS TotalViewCount, AVG(P.Score) AS AverageScore
//                    FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY PT.Name)
// SELECT U.UserId, U.Reputation, U.Views, U.UpVotes, U.DownVotes, U.PostCount, U.QuestionCount, U.AnswerCount, U.AcceptedAnswerCount,
//        PTS.PostTypeName, PTS.TotalPosts, PTS.TotalViewCount, PTS.AverageScore
// FROM UserStats U CROSS JOIN PostTypesStats PTS ORDER BY U.Reputation DESC, PTS.TotalPosts DESC;
fn q12864(db: &'static So) -> String {
    let ups = user_posts(db);
    let acc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and((&db.post.accepted_answer_id).opt()))).fold(0i64, |n, (t, x)| n + (t == 1 && x.is_some()) as i64);
    let ps = stats_by_type(db);
    let mut out = Vec::new();
    (&ups).and((&acc).opt()).cross(&ps).drive(|(u, t), ((a, x), b)| {
        let mut f = ucols(db, u, &["uid", "rep", "uviews", "uup", "udown"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(x.unwrap_or(0)), V::S(t), V::I(b[0]), nullable(b[3], b[2]), avg(b[1], b[0])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//            COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COUNT(DISTINCT p.Id) AS TotalPosts,
//            COUNT(DISTINCT c.Id) AS TotalComments, COUNT(DISTINCT ba.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN Badges ba ON u.Id = ba.UserId GROUP BY u.Id, u.DisplayName),
// TrendingTags AS (
//     SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY t.TagName ORDER BY PostCount DESC LIMIT 10)
// SELECT ua.UserId, ua.DisplayName, ua.TotalUpvotes, ua.TotalDownvotes, ua.TotalPosts, ua.TotalComments, tt.TagName AS TrendingTag
// FROM UserActivity ua JOIN TrendingTags tt ON ua.TotalPosts > 0 ORDER BY ua.TotalPosts DESC, ua.TotalUpvotes DESC LIMIT 50;
fn q5140(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold((0i64, 0i64), |(u, d), (p, _)| {
            let t = p.and_then(|(_, t)| t);
            (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64)
        });
    let dp = user_distinct_posts(db);
    let dc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).count_distinct();
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let tt = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))))
        .fold(0i64, |n, _| n + 1);
    let tt = top_n(drain(&tt), |&(_, n)| Reverse(n), 10);
    let users = drain((&dp).filt(|n| n > 0).and(&ua).and((&dc).opt()));
    let v = cross_top(users, |&(_, ((n, (u, _)), _))| (Reverse(n), Reverse(u)), tt, |_| 0, 50);
    rows(v.iter().map(|&((u, ((n, (up, dn)), c)), (t, _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(up), V::I(dn), V::I(n), V::I(c.unwrap_or(0)), V::S(db.tag.tag_name.get(t).unwrap())]);
        row(f)
    }))
}

// WITH PostTagCounts AS (SELECT post.Id AS PostId, unnest(string_to_array(substring(post.Tags, 2, length(post.Tags) - 2), '><')) AS Tag
//                        FROM Posts post WHERE post.PostTypeId = 1),
// TagAggregates AS (SELECT Tag, COUNT(*) AS PostCount FROM PostTagCounts GROUP BY Tag HAVING COUNT(*) > 5),
// UserPostCounts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS UserPostCount FROM Posts p WHERE p.PostTypeId IN (1, 2) GROUP BY p.OwnerUserId),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, up.UserPostCount FROM Users u JOIN UserPostCounts up ON u.Id = up.OwnerUserId
//                 WHERE u.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - interval '1 year'),
// TopPostTags AS (SELECT t.Tag, SUM(up.UserPostCount) AS TotalPosts FROM TagAggregates t JOIN PostTagCounts pt ON t.Tag = pt.Tag
//                 JOIN UserPostCounts up ON pt.PostId = up.OwnerUserId GROUP BY t.Tag ORDER BY TotalPosts DESC LIMIT 10)
// SELECT u.DisplayName, u.Reputation, t.Tag, t.TotalPosts FROM ActiveUsers u JOIN TopPostTags t ON u.UserPostCount > 10
// ORDER BY u.Reputation DESC, t.TotalPosts DESC;
//
// TopPostTags joins a post Id to an owner Id, as written.
fn q29115(db: &'static So) -> String {
    let q = || db.post.with((&db.post.post_type_id).eq(1));
    let ta = q().select((&db.post.tags_str).flat_map(tag_list)).group_by(Same::new()).fold(0i64, |n, _| n + 1);
    let up = db.post.with((&db.post.post_type_id).is_in([1, 2])).group_by((&db.post.owner_user_id).opt()).fold(0i64, |n, _| n + 1);
    let tpt = q()
        .select((&db.post.tags_str).flat_map(tag_list).with((&ta).filt(|n| n > 5)).and((&db.post.origid).map(Some).select(&up)))
        .group_by(Same::<(Str, i64)>::new().map(|(t, _)| t))
        .fold(0i64, |s, (_, c)| s + c);
    let tpt = rel(top_n(drain(&tpt), |&(_, s)| Reverse(s), 10));
    let active = db
        .user
        .with((&db.user.last_access_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .select(Ident::<User>::new().and((&db.user.origid).map(Some).select(&up).filt(|n| n > 10)));
    let mut out = Vec::new();
    active.cross(&tpt).drive(|_, ((u, _), (t, s))| out.push(row(vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::S(t), V::I(s)])));
    rows(out)
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(CASE WHEN p.Score IS NOT NULL THEN 1 ELSE 0 END) AS PositivePosts,
//            SUM(CASE WHEN p.Score IS NULL THEN 1 ELSE 0 END) AS NegativePosts, SUM(bb.Class) AS TotalBadges, AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges bb ON u.Id = bb.UserId
//     WHERE u.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, AVG(p.ViewCount) AS AvgViewCount, SUM(p.CommentCount) AS TotalComments,
//            COUNT(DISTINCT p.AcceptedAnswerId) AS AcceptedAnswers
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '7 days' GROUP BY pt.Name)
// SELECT ua.DisplayName, ua.PostsCreated, ua.PositivePosts, ua.NegativePosts, ua.TotalBadges, ua.AvgReputation, ps.PostType, ps.PostCount,
//        ps.AvgViewCount, ps.TotalComments, ps.AcceptedAnswers
// FROM UserActivity ua JOIN PostStatistics ps ON ua.PostsCreated > 0 ORDER BY ua.AvgReputation DESC, ps.PostCount DESC FETCH FIRST 100 ROWS ONLY;
fn q5448(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let old = || db.user.with((&db.user.creation_date).lt(add_years(now, -1)));
    let ua = old()
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select(&db.post.score).opt()).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, ((r, s), c)| [a[0] + s.is_some() as i64, a[1] + s.is_none() as i64, a[2] + c.is_some() as i64, a[3] + c.unwrap_or(0), a[4] + r, a[5] + 1]);
    let dp = old().group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let Post { view_count, comment_count, creation_date, accepted_answer_id, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_days(now, -7)))
        .group_by(ptype_name(db))
        .select(view_count.opt().and(comment_count))
        .fold([0i64; 4], |a, (v, c)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + c]);
    let pa = db.post.with(creation_date.ge(add_days(now, -7))).group_by(ptype_name(db)).select(accepted_answer_id).count_distinct();
    let users = drain((&dp).filt(|n| n > 0).and(&ua));
    let types = drain((&ps).and((&pa).opt()));
    let v = cross_top(users, |&(_, (_, a))| Reverse(fkey(a[4] as f64 / a[5] as f64)), types, |&(_, (b, _))| Reverse(b[0]), 100);
    rows(v.iter().map(|&((u, (n, a)), (t, (b, x)))| {
        row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[4], a[5]), V::S(t), V::I(b[0]), avg(b[2], b[1]), V::I(b[3]), V::I(x.unwrap_or(0))])
    }))
}

// WITH UserStatistics AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore,
//            COALESCE(SUM(b.Class), 0) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// RecentActivity AS (SELECT OwnerUserId AS UserId, COUNT(*) AS RecentPostActivity, MAX(CreationDate) AS LastActivityDate FROM Posts
//                    WHERE CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 MONTH' GROUP BY OwnerUserId),
// CombinedData AS (SELECT us.*, COALESCE(ra.RecentPostActivity, 0) AS RecentPostActivity, ra.LastActivityDate
//                  FROM UserStatistics us LEFT JOIN RecentActivity ra ON us.UserId = ra.UserId)
// SELECT UserId, DisplayName, TotalPosts, QuestionCount, AnswerCount, TotalViews, TotalScore, TotalBadges, RecentPostActivity, LastActivityDate
// FROM CombinedData WHERE TotalPosts > 0 ORDER BY TotalScore DESC, TotalViews DESC FETCH FIRST 10 ROWS ONLY;
//
// CURRENT_TIMESTAMP is when the query runs; the data ends in 2024, so
// RecentActivity is empty.
fn q5016(db: &'static So) -> String {
    let now = now_utc();
    let Post { post_type_id, view_count, score, creation_date, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score)).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, c)| {
            let mut a = a;
            if let Some(((t, v), s)) = p {
                a[0] += (t == 1) as i64;
                a[1] += (t == 2) as i64;
                a[2] += v.is_some() as i64;
                a[3] += v.unwrap_or(0);
                a[4] += 1;
                a[5] += s;
            }
            a[6] += c.unwrap_or(0);
            a
        });
    let dp = user_distinct_posts(db);
    let ra = owned(db)
        .with(creation_date.filt(move |d| ny_to_utc(d) >= ny_to_utc(add_months(utc_to_ny(now), -1))))
        .group_by(&db.post.owner_user)
        .select(creation_date)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&dp).filt(|n| n > 0).and(&us).and((&ra).opt()));
    let v = top_n(v, |&(_, ((_, a), _))| (Reverse(if a[4] == 0 { i64::MIN } else { a[5] }), a[2] == 0, Reverse(a[3])), 10);
    rows(v.iter().map(|&(u, ((n, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), nullable(a[5], a[4]), V::I(a[6])]);
        f.extend(match r {
            Some((k, m)) => [V::I(k), V::T(m)],
            None => [V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//            SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, AVG(p.Score) AS AverageScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PopularTags AS (
//     SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY t.TagName ORDER BY PostCount DESC LIMIT 10),
// UserBadges AS (SELECT b.UserId, COUNT(*) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE b.Class = 2) AS SilverBadges,
//                       COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT us.UserId, us.DisplayName, us.TotalPosts, us.PositivePosts, us.NegativePosts, us.AverageScore, COALESCE(ub.GoldBadges, 0) AS GoldBadges,
//        COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, pt.TagName, pt.PostCount, pt.TotalViews
// FROM UserStatistics us LEFT JOIN UserBadges ub ON us.UserId = ub.UserId
// LEFT JOIN PopularTags pt ON pt.TagName IN (SELECT DISTINCT p.Tags FROM Posts p WHERE p.Tags LIKE '%' || pt.TagName || '%')
// ORDER BY us.TotalPosts DESC, us.AverageScore DESC LIMIT 50;
//
// A tag name is in that set exactly when some post's whole Tags string is
// the name itself (such a string contains it). None is: Tags strings carry
// their angle brackets. The condition does not mention us, so the LEFT JOIN
// crosses every user with the surviving tags, or with one NULL row.
fn q4568(db: &'static So) -> String {
    let ups = user_posts(db);
    let neg = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.score)).fold(0i64, |n, s| n + (s < 0) as i64);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let pt = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select((&db.post.view_count).opt()))
        .fold((0i64, 0i64, 0i64), |(n, wn, w), v| (n + 1, wn + v.is_some() as i64, w + v.unwrap_or(0)));
    let pt = rel(top_n(drain(&pt), |&(_, (n, _, _))| Reverse(n), 10));
    let strs: MatSet<Str> = (&db.post.tags_str).collect();
    let pt = left_all(drain((&pt).with(Same::<(Id<Tag>, (i64, i64, i64))>::new().map(|(t, _)| t).select(&db.tag.tag_name).with(&strs))));
    let v = drain((&ups).and((&neg).opt()).and((&ub).opt()).cross(&pt));
    let v = top_n(v, |&(_, (((a, _), _), _))| (Reverse(a[1]), a[1] == 0, Reverse(if a[1] == 0 { 0 } else { fkey(a[4] as f64 / a[1] as f64) })), 50);
    rows(v.iter().map(|&((u, _), (((a, ng), b), x))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[8]), V::I(ng.unwrap_or(0)), avg(a[4], a[1]), V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        f.extend(match x {
            Some((_, (t, (n, wn, w)))) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(n), nullable(w, wn)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}


// WITH UserVoteSummary AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, AVG(EXTRACT(EPOCH FROM (V.CreationDate - U.CreationDate)) / 3600) AS AvgHoursSinceJoin
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostDetails AS (
//     SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(UP.AvgHoursSinceJoin, 0) AS AvgUserVoteTime
//     FROM Posts P LEFT JOIN UserVoteSummary UP ON P.OwnerUserId = UP.UserId WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// ClosedPosts AS (SELECT PH.PostId, MIN(PH.CreationDate) AS FirstClosedDate FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId)
// SELECT PD.PostId, PD.Title, PD.CreationDate, PD.Score, CP.FirstClosedDate, PD.AvgUserVoteTime,
//        CASE WHEN CP.FirstClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus, COALESCE(B.Name, 'No Badge') AS RecentBadge
// FROM PostDetails PD LEFT JOIN ClosedPosts CP ON PD.PostId = CP.PostId
// LEFT JOIN Badges B ON PD.PostId = B.UserId AND B.Date = (SELECT MAX(Date) FROM Badges WHERE UserId = B.UserId)
// WHERE PD.Score > (SELECT AVG(Score) FROM Posts) ORDER BY PD.Score DESC, PD.CreationDate ASC LIMIT 50 OFFSET 10;
//
// The badge join matches a post Id against a user Id, as written; the
// correlated MAX keeps each user's latest badges.
fn q3076(db: &'static So) -> String {
    let uvs = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.creation_date).and(votes_by(db).select(&db.vote.creation_date)))
        .fold(((0.0f64, 0.0f64), 0i64), |(s, n), (u, v)| (kahan(s, secs(v - u) / 3600.0), n + 1));
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(&db.post_history.creation_date).fold(i64::MAX, |m, d| m.min(d));
    let maxd = db.badge.group_by(&db.badge.user).select(&db.badge.date).fold(i64::MIN, |m, d| m.max(d));
    let latest: HashIdx<i64, Id<Badge>> = db.badge.with((&db.badge.user).select(&maxd).and(&db.badge.date).filt(|(m, d)| m == d)).select(&db.badge.user_id).inv().collect();
    let (n, s) = (&db.post.score).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let mean = s as f64 / n as f64;
    let pd = db
        .post
        .with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with((&db.post.score).filt(move |x| x as f64 > mean))
        .select(Ident::<Post>::new().and((&db.post.owner_user).select(&uvs).opt()).and((&cp).opt()).and((&db.post.origid).select(&latest).opt()));
    let v = top_n(drain(pd), |&(p, _)| (Reverse(db.post.score.get(p).unwrap()), db.post.creation_date.get(p).unwrap()), 60);
    rows(v.iter().skip(10).map(|&(_, (((p, a), c), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(if c.is_some() { V::T(c.unwrap()) } else { V::Null });
        f.push(match a {
            Some((s, n)) if n > 0 => V::F(s.0 / n as f64),
            _ => V::F(0.0),
        });
        f.push(V::S(if c.is_some() { "Closed" } else { "Open" }));
        f.push(V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        row(f)
    }))
}

// WITH UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//            SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PopularPosts AS (
//     SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount,
//            COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
//     GROUP BY p.Id, p.Title, p.ViewCount, p.Score ORDER BY p.ViewCount DESC LIMIT 10)
// SELECT u.DisplayName, u.Reputation, us.PostCount, us.QuestionCount, us.AnswerCount, pp.Title AS PopularPostTitle, pp.ViewCount AS PopularPostViews,
//        pp.Score AS PopularPostScore, pp.UpvoteCount AS PopularPostUpvotes, pp.DownvoteCount AS PopularPostDownvotes
// FROM UserStats us JOIN Users u ON us.UserId = u.Id LEFT JOIN PopularPosts pp ON pp.UpvoteCount > 10 ORDER BY u.Reputation DESC LIMIT 20;
fn q9775(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(100));
    let us = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((t, _)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64],
            None => a,
        });
    let dp = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let pp = db
        .post
        .with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let pp = rel(top_n(drain(&pp), |&(p, _)| {
        let w = db.post.view_count.get(p);
        (w.is_none(), Reverse(w))
    }, 10));
    let pp = left_all(drain((&pp).filt(|(_, (u, _))| u > 10)));
    let users = drain((&us).and(&dp));
    let v = cross_top(users, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), drain(&pp), |_| 0, 20);
    rows(v.iter().map(|&((u, (a, n)), (_, x))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[1]), V::I(a[0])]);
        match x {
            Some((_, (p, (up, dn)))) => {
                f.extend(post_fields(db, p, &["title", "views", "score"]));
                f.extend([V::I(up), V::I(dn)]);
            }
            None => f.extend((0..5).map(|_| V::Null)),
        }
        row(f)
    }))
}

// WITH UserMetrics AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount,
//            COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//            COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount,
//            COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, ... SilverBadges, BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPostMetrics AS (SELECT p.OwnerUserId, COUNT(*) AS RecentPostCount, MAX(p.CreationDate) AS LastPostDate FROM Posts p
//                       WHERE p.CreationDate > NOW() - INTERVAL '30 days' GROUP BY p.OwnerUserId)
// SELECT um.UserId, um.DisplayName, um.Reputation, um.PostCount, um.QuestionCount, um.AnswerCount, um.UpvoteCount, um.DownvoteCount, um.GoldBadges,
//        um.SilverBadges, um.BronzeBadges, rpm.RecentPostCount, rpm.LastPostDate
// FROM UserMetrics um LEFT JOIN RecentPostMetrics rpm ON um.UserId = rpm.OwnerUserId WHERE um.Reputation > 1000
// ORDER BY um.Reputation DESC, rpm.LastPostDate DESC LIMIT 50;
//
// NOW() is when the query runs; the data ends in 2024, so every user has
// no recent posts.
fn q6890(db: &'static So) -> String {
    let now = now_utc();
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let um = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, c)| {
            let (t, v) = p.map_or((None, None), |(t, v)| (Some(t), v));
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(1)) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + (c == Some(1)) as i64, a[5] + (c == Some(2)) as i64, a[6] + (c == Some(3)) as i64]
        });
    let dp = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let rpm = owned(db)
        .with((&db.post.creation_date).filt(move |d| ny_to_utc(d) > now - 30 * DAY_US))
        .group_by(&db.post.owner_user)
        .select(&db.post.creation_date)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&um).and(&dp).and((&rpm).opt()));
    let v = top_n(v, |&(u, (_, r))| (Reverse(db.user.reputation.get(u).unwrap()), r.is_none(), Reverse(r.map(|r| r.1))), 50);
    rows(v.iter().map(|&(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[1]), V::I(a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[5]), V::I(a[6])]);
        f.extend(match r {
            Some((k, m)) => [V::I(k), V::T(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//            SUM(CASE WHEN b.Name IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PopularPosts AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 AND p.Score > 5 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount ORDER BY p.ViewCount DESC LIMIT 10),
// ActiveUsers AS (
//     SELECT u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvotesReceived
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE u.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY u.DisplayName HAVING COUNT(p.Id) > 5)
// SELECT us.DisplayName AS UserName, us.Reputation, us.PostCount, us.AnswerCount, us.BadgeCount, pp.Title AS PopularPostTitle, pp.CreationDate AS PopularPostDate,
//        pp.Score AS PopularPostScore, pp.ViewCount AS PopularPostViews, au.TotalPosts AS ActiveUserPostCount, au.UpvotesReceived AS ActiveUserUpvotes
// FROM UserStats us JOIN PopularPosts pp ON us.UserId = (SELECT OwnerUserId FROM Posts ORDER BY ViewCount DESC LIMIT 1)
// JOIN ActiveUsers au ON us.DisplayName = au.DisplayName ORDER BY us.Reputation DESC, pp.ViewCount DESC;
fn q6599(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let us = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).opt()))
        .fold((0i64, 0i64), |(a, b), (t, x)| (a + (t == Some(2)) as i64, b + x.is_some() as i64));
    let dp = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let pp = db.post.with((&db.post.post_type_id).eq(1)).with((&db.post.score).gt(5)).select((&db.post.view_count).opt());
    let pp = rel(top_n(drain(pp), |&(_, w)| (w.is_none(), Reverse(w)), 10));
    let top = top_n(drain(db.post.select((&db.post.view_count).opt())), |&(_, w)| (w.is_none(), Reverse(w)), 1);
    let owner: MatSet<Option<i64>> = rel(top).map(|(p, _)| db.post.owner_user_id.get(p)).collect();
    let au = db
        .user
        .with((&db.user.last_access_date).gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(n, u), t| (n + 1, u + (t == Some(2)) as i64));
    let mut out = Vec::new();
    rich()
        .with((&db.user.origid).map(Some).with(&owner))
        .select(Ident::<User>::new().and(&us).and(&dp).and((&db.user.display_name).select((&au).filt(|(n, _)| n > 5))))
        .cross(&pp)
        .drive(|_, ((((u, (a, b)), n), (k, up)), (p, _))| {
            let mut f = ucols(db, u, &["name", "rep"]);
            f.extend([V::I(n), V::I(a), V::I(b)]);
            f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
            f.extend([V::I(k), V::I(up)]);
            out.push(row(f))
        });
    rows(out)
}

// WITH UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TagUsage AS (SELECT t.Id AS TagId, t.TagName, COUNT(DISTINCT p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.Id, t.TagName),
// ActiveUsers AS (SELECT UserId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseActions,
//                        COUNT(CASE WHEN ph.PostHistoryTypeId = 24 THEN 1 END) AS SuggestedEdits FROM PostHistory ph GROUP BY UserId),
// UserPostEngagement AS (
//     SELECT u.Id AS UserId, COALESCE(us.TotalPosts, 0) AS TotalPosts, ..., COALESCE(au.CloseActions, 0) AS CloseActions, COALESCE(au.SuggestedEdits, 0) AS SuggestedEdits,
//            u.DisplayName, u.Reputation
//     FROM Users u LEFT JOIN UserStats us ON u.Id = us.UserId LEFT JOIN ActiveUsers au ON u.Id = au.UserId)
// SELECT up.DisplayName, up.Reputation, up.TotalPosts, up.TotalQuestions, up.TotalAnswers, up.AcceptedAnswers, up.CloseActions, up.SuggestedEdits, tg.TagName, tg.PostCount
// FROM UserPostEngagement up JOIN TagUsage tg ON up.TotalPosts > 0 ORDER BY up.Reputation DESC, up.TotalPosts DESC, tg.PostCount DESC LIMIT 10;
fn q27806(db: &'static So) -> String {
    let ups = user_posts(db);
    let acc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and((&db.post.accepted_answer_id).opt()))).fold(0i64, |n, (t, x)| n + (t == 1 && x.is_some()) as i64);
    let au = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold((0i64, 0i64), |(c, s), t| (c + matches!(t, 10 | 11) as i64, s + (t == 24) as i64));
    let ts_ = tag_stats(db);
    let users = drain((&ups).filt(|a| a[1] > 0).and((&acc).opt()).and((&au).opt()));
    let tags = drain((&ts_).filt(|a| a[0] > 0));
    let v = cross_top(users, |&(u, ((a, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1])), tags, |&(_, b)| Reverse(b[0]), 10);
    rows(v.iter().map(|&((u, ((a, x), h)), (t, b))| {
        let h = h.unwrap_or((0, 0));
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(x.unwrap_or(0)), V::I(h.0), V::I(h.1), V::S(db.tag.tag_name.get(t).unwrap()), V::I(b[0])]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS Upvotes,
//            SUM(CASE WHEN V.VoteTypeId IN (3) THEN 1 ELSE 0 END) AS Downvotes, AVG(COALESCE(P.Score, 0)) AS AvgPostScore
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (
//     SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(PV.VoteCount, 0) AS TotalVotes, COALESCE(C.CommentCount, 0) AS TotalComments,
//            COALESCE(A.AcceptedAnswerCount, 0) AS AcceptedAnswers
//     FROM Posts P LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) PV ON P.Id = PV.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AcceptedAnswerCount FROM Posts WHERE PostTypeId = 2 AND AcceptedAnswerId IS NOT NULL GROUP BY ParentId) A ON P.Id = A.ParentId
//     WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year')
// SELECT U.DisplayName, S.AvgPostScore, PS.Title, PS.TotalVotes, PS.TotalComments, PS.AcceptedAnswers
// FROM UserVoteSummary U JOIN PostStatistics PS ON U.UserId = PS.PostId
// JOIN (SELECT UserId, AVG(Score) AS AvgPostScore FROM Posts WHERE CreationDate >= cast('2024-10-01' as date) - INTERVAL '2 years' GROUP BY UserId) S
//   ON U.UserId = S.UserId
// WHERE PS.TotalVotes > 5 ORDER BY S.AvgPostScore DESC, PS.TotalVotes DESC LIMIT 10;
//
// Posts has no UserId, so DuckDB binds S's UserId to U.UserId and runs S as
// a LATERAL subquery: one row per user, (U.UserId, the average score of
// every post since 2022-10-01). S's join condition is then always true. The
// PostStatistics join matches a user Id against a post Id, as written.
fn q3760(db: &'static So) -> String {
    let since1 = add_years(date(2024, 10, 1), -1);
    let since2 = add_years(date(2024, 10, 1), -2);
    let pv = db.vote.group_by(&db.vote.post_id).fold(0i64, |n, _| n + 1);
    let cc = db.comment.group_by(&db.comment.post).fold(0i64, |n, _| n + 1);
    let aa = db.post.with((&db.post.post_type_id).eq(2)).with(&db.post.accepted_answer_id).group_by(&db.post.parent).fold(0i64, |n, _| n + 1);
    type Ps = (((Id<Post>, i64), Option<i64>), Option<i64>);
    let ps: MatSet<Ps> = db
        .post
        .with((&db.post.creation_date).ge(since1))
        .select(Ident::<Post>::new().and((&db.post.origid).select(&pv).filt(|v| v > 5)).and((&cc).opt()).and((&aa).opt()))
        .collect();
    let by_id: HashIdx<i64, Ps> = (&ps).map(|(((p, _), _), _)| p).select(&db.post.origid).inv().collect();
    let (sn, ss) = db.post.with((&db.post.creation_date).ge(since2)).select(&db.post.score).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let joined = drain(db.user.select(Ident::<User>::new().and((&db.user.origid).select(&by_id))));
    let v = top_n(joined, |&(_, (_, (((_, t), _), _)))| Reverse(t), 10);
    rows(v.iter().map(|&(_, (u, (((p, t), c), a)))| row(vec![user_col(db, u, "name"), avg(ss, sn), title(db, p), V::I(t), V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0))])))
}

// WITH UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore,
//            AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)) / 60) AS AvgResponseTime
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// BadgeCounts AS (SELECT b.UserId, COUNT(*) AS BadgeCount, ... GoldBadges, SilverBadges, BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostActivity AS (SELECT p.OwnerUserId, COUNT(c.Id) AS TotalComments, COUNT(DISTINCT ph.Id) AS TotalHistoryEntries
//                  FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.OwnerUserId)
// SELECT us.*, COALESCE(bc.BadgeCount, 0) AS BadgeCount, ..., COALESCE(pa.TotalComments, 0) AS TotalComments, COALESCE(pa.TotalHistoryEntries, 0) AS TotalHistoryEntries
// FROM UserStats us LEFT JOIN BadgeCounts bc ON us.UserId = bc.UserId LEFT JOIN PostActivity pa ON us.UserId = pa.OwnerUserId
// ORDER BY us.Reputation DESC, us.TotalPosts DESC FETCH FIRST 100 ROWS ONLY;
fn q9812(db: &'static So) -> String {
    let ups = user_posts(db);
    let rt = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.last_activity_date).and(&db.post.creation_date)))
        .fold(((0.0f64, 0.0f64), 0i64), |(s, n), (la, cd)| ((s.0 + secs(la - cd) / 60.0, 0.0), n + 1));
    let bc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let pa = owned(db).group_by(&db.post.owner_user).select(comments_of(db).opt().and(history_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let ph = owned(db).group_by(&db.post.owner_user).select(history_of(db)).count_distinct();
    let v = drain((&ups).and((&rt).opt()).and((&bc).opt()).and((&pa).opt()).and((&ph).opt()));
    let v = top_n(v, |&(u, ((((a, _), _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1])), 100);
    rows(v.iter().map(|&(u, ((((a, r), b), c), h))| {
        let b = b.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1]), r.map_or(V::Null, |(s, n)| fmean(s, n))]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3]), V::I(c.unwrap_or(0)), V::I(h.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, ... SilverBadges, BronzeBadges, COUNT(*) AS TotalBadges
//                          FROM Badges GROUP BY UserId),
// PostMetrics AS (
//     SELECT p.Id AS PostId, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes,
//            COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes, COALESCE(ROUND(AVG(p.Score), 2), 0) AS AvgScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '30 days' GROUP BY p.Id, p.OwnerUserId),
// HighScorers AS (SELECT OwnerUserId, COUNT(PostId) AS HighScoreCount FROM PostMetrics WHERE AvgScore > 10 GROUP BY OwnerUserId),
// UserWithBadges AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ubc.GoldBadges, ubc.SilverBadges, ubc.BronzeBadges, COALESCE(hsc.HighScoreCount, 0) AS HighScoreCount
//     FROM Users u LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId LEFT JOIN HighScorers hsc ON u.Id = hsc.OwnerUserId
//     WHERE u.Reputation IS NOT NULL AND (u.Reputation > 100 OR ubc.GoldBadges > 0))
// SELECT u.UserId, u.DisplayName, COALESCE(u.Reputation, 0) AS Reputation, COALESCE(u.GoldBadges, 0) AS GoldBadges, ..., u.HighScoreCount,
//        CASE WHEN u.Reputation IS NULL THEN 'Reputation Missing' WHEN u.HighScoreCount = 0 THEN 'No High Scoring Posts' ELSE 'Active Contributor' END AS UserStatus
// FROM UserWithBadges u WHERE u.GoldBadges > 0 OR u.HighScoreCount > 0 ORDER BY u.Reputation DESC, u.DisplayName LIMIT 100;
//
// CURRENT_DATE is the day the query runs; the data ends in 2024, so there
// are no high scorers.
fn q24493(db: &'static So) -> String {
    let ubc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64, a[3] + 1]);
    let hs = owned(db)
        .with((&db.post.creation_date).ge(add_days(current_date(), -30)))
        .with((&db.post.score).gt(10))
        .group_by(&db.post.owner_user)
        .fold(0i64, |n, _| n + 1);
    let rows_ = db.user.select(Ident::<User>::new().and(&db.user.reputation).and((&ubc).opt()).and((&hs).opt())).filt(|(((_, r), b), h): (((Id<User>, i64), Option<[i64; 4]>), Option<i64>)| {
        let g = b.map_or(0, |b| b[0]);
        (r > 100 || g > 0) && (g > 0 || h.unwrap_or(0) > 0)
    });
    let v = top_n(drain(rows_), |&(_, (((u, r), _), _))| (Reverse(r), db.user.display_name.get(u).unwrap()), 100);
    rows(v.iter().map(|&(_, (((u, _), b), h))| {
        let b = b.unwrap_or([0; 4]);
        let h = h.unwrap_or(0);
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(h), V::S(if h == 0 { "No High Scoring Posts" } else { "Active Contributor" })]);
        row(f)
    }))
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(COALESCE(v.UpVotes, 0) - COALESCE(v.DownVotes, 0)) AS AvgScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//                FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName),
// RecentActiveUsers AS (SELECT p.OwnerUserId AS UserId, COUNT(*) AS RecentActivityCount FROM Posts p WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'
//                       GROUP BY p.OwnerUserId),
// PopularQuestions AS (SELECT p.Id, p.Title, p.CreationDate, COALESCE(v.UpVotes, 0) - COALESCE(v.DownVotes, 0) AS Score FROM Posts p
//                      LEFT JOIN (... the same vote sums ...) v ON p.Id = v.PostId WHERE p.PostTypeId = 1 ORDER BY Score DESC LIMIT 10)
// SELECT ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.AvgScore, rau.RecentActivityCount, pq.Title AS PopularQuestionTitle,
//        pq.Score AS PopularQuestionScore
// FROM UserPostStats ups LEFT JOIN RecentActiveUsers rau ON ups.UserId = rau.UserId LEFT JOIN PopularQuestions pq ON pq.Id IS NOT NULL
// ORDER BY ups.AvgScore DESC, ups.TotalPosts DESC LIMIT 5;
//
// The LIMIT 5 cuts inside one user's ten PopularQuestions rows, which tie
// on the sort key; the rewrite adds `ups.UserId, pq.Score DESC, pq.Id`
// (and `p.Id` to PopularQuestions). CURRENT_TIMESTAMP is when the query
// runs; the data ends in 2024, so RecentActiveUsers is empty.
fn q3339(db: &'static So) -> String {
    let now = now_utc();
    let net = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |s, t| s + (t == 2) as i64 - (t == 3) as i64);
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&net).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s.unwrap_or(0), a[4] + 1],
            None => [a[0], a[1], a[2], a[3], a[4] + 1],
        });
    let rau = owned(db).with((&db.post.creation_date).filt(move |d| ny_to_utc(d) >= now - 30 * DAY_US)).group_by(&db.post.owner_user).fold(0i64, |n, _| n + 1);
    let pq = db.post.with((&db.post.post_type_id).eq(1)).select((&net).opt().map(|s| s.unwrap_or(0)));
    let pq = top_n(drain(pq), |&(p, s)| (Reverse(s), db.post.origid.get(p).unwrap()), 10);
    let users = drain((&ups).and((&rau).opt()));
    let key = |&(u, (a, _)): &(Id<User>, ([i64; 5], Option<i64>))| (Reverse(fkey(a[3] as f64 / a[4] as f64)), Reverse(a[0]), db.user.origid.get(u).unwrap());
    let v = cross_top(users, key, pq, |&(p, s)| (Reverse(s), db.post.origid.get(p).unwrap()), 5);
    rows(v.iter().map(|&((u, (a, r)), (p, s))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[4]), oint(r), title(db, p), V::I(s)])
    }))
}

// WITH UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//            COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, CommentCount, (PostCount + UpVotes - DownVotes) AS EngagementScore
//              FROM UserStats WHERE Reputation > 1000 ORDER BY EngagementScore DESC LIMIT 10),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerName FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//                 WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' ORDER BY p.CreationDate DESC),
// RecentComments AS (SELECT c.Id AS CommentId, c.Text, c.CreationDate, p.Title AS PostTitle, u.DisplayName AS Commenter
//                    FROM Comments c JOIN Posts p ON c.PostId = p.Id JOIN Users u ON c.UserId = u.Id
//                    WHERE c.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
// SELECT tu.DisplayName AS TopUser, tu.Reputation AS UserReputation, COUNT(DISTINCT rp.PostId) AS RecentPostCount, COUNT(DISTINCT rc.CommentId) AS RecentCommentCount
// FROM TopUsers tu LEFT JOIN RecentPosts rp ON tu.DisplayName = rp.OwnerName LEFT JOIN RecentComments rc ON rc.Commenter = tu.DisplayName
// GROUP BY tu.DisplayName, tu.Reputation ORDER BY tu.Reputation DESC;
fn q6707(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let us = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold(0i64, |s, (_, t)| s + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let dp = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let tu = rel(top_n(drain((&dp).and((&us).opt())), |&(_, (n, s))| Reverse(n + s.unwrap_or(0)), 10));
    let since = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let rp: HashIdx<Str, Id<Post>> = owned(db).with((&db.post.creation_date).gt(since)).select((&db.post.owner_user).select(&db.user.display_name)).inv().collect();
    let rc: HashIdx<Str, Id<Comment>> = db.comment.with((&db.comment.creation_date).gt(since)).select((&db.comment.user).select(&db.user.display_name)).inv().collect();
    let key = || (&tu).map(|(u, _)| (db.user.display_name.get(u).unwrap(), db.user.reputation.get(u).unwrap()));
    type K = (Str, i64);
    let posts = key().group_by(Same::<K>::new()).select(Same::<K>::new().map(|(n, _)| n).select(&rp)).count_distinct();
    let comments = key().group_by(Same::<K>::new()).select(Same::<K>::new().map(|(n, _)| n).select(&rc)).count_distinct();
    let groups: MatSet<K> = key().collect();
    let mut out = Vec::new();
    (&groups).select(Same::new().and((&posts).opt()).and((&comments).opt())).drive(|_, (((n, r), p), c)| {
        out.push(row(vec![V::S(n), V::I(r), V::I(p.unwrap_or(0)), V::I(c.unwrap_or(0))]))
    });
    rows(out)
}


// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
//            COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//            SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostDetails AS (
//     SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, COALESCE(PH.Comment, 'No comments') AS LastActionComment,
//            PH.CreationDate AS LastActionDate, PH.PostHistoryTypeId, U.DisplayName AS LastEditor
//     FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId LEFT JOIN Users U ON PH.UserId = U.Id
//     WHERE P.LastActivityDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')),
// AggregateData AS (SELECT UserId, SUM(TotalPosts) AS TotalUserPosts, ... SUM(TotalDownvotes) AS TotalUserDownvotes FROM UserStats GROUP BY UserId)
// SELECT U.DisplayName, AD.TotalUserPosts, AD.TotalUserQuestions, AD.TotalUserAnswers, AD.TotalUserUpvotes, AD.TotalUserDownvotes, PD.PostId, PD.Title,
//        PD.CreationDate, PD.ViewCount, PD.Score, PD.LastActionComment, PD.LastActionDate, PD.LastEditor
// FROM AggregateData AD JOIN Users U ON AD.UserId = U.Id JOIN PostDetails PD ON U.DisplayName = PD.LastEditor
// WHERE AD.TotalUserPosts > 10 ORDER BY AD.TotalUserUpvotes DESC, PD.ViewCount DESC LIMIT 100;
//
// The same post's edits tie on the ORDER BY; the rewrite adds U.Id and the
// edit's columns.
fn q8762(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 4], |a, (t, v)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]);
    let dp = user_distinct_posts(db);
    let q = db.user.group_by(Ident::<User>::new()).select(posts_of(db).with((&db.post.post_type_id).eq(1))).count_distinct();
    let a = db.user.group_by(Ident::<User>::new()).select(posts_of(db).with((&db.post.post_type_id).eq(2))).count_distinct();
    let recent: MatSet<Id<Post>> = db.post.with((&db.post.last_activity_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).collect();
    let PostHistory { post, user, .. } = &db.post_history;
    let pd: HashIdx<Str, Id<PostHistory>> = db.post_history.with(post.with(&recent)).select(user.select(&db.user.display_name)).inv().collect();
    let v = drain((&dp).filt(|n| n > 10).and(&us).and((&q).opt()).and((&a).opt()).and((&db.user.display_name).select(&pd)));
    let hist = |h: Id<PostHistory>| {
        let p = post.get(h).unwrap();
        let w = db.post.view_count.get(p);
        (w.is_none(), Reverse(w), db.post.origid.get(p).unwrap(), db.post_history.creation_date.get(h).unwrap(), db.post_history.comment.get(h).unwrap_or("No comments"), db.post_history.post_history_type_id.get(h).unwrap())
    };
    let v = top_n(v, |&(u, ((((_, b), _), _), h))| {
        let (n, w, p, d, c, t) = hist(h);
        (Reverse(b[2]), n, w, db.user.origid.get(u).unwrap(), p, d, c, t)
    }, 100);
    rows(v.iter().map(|&(u, ((((n, b), q), a), h))| {
        let p = post.get(h).unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(q.unwrap_or(0)), V::I(a.unwrap_or(0)), V::I(b[2]), V::I(b[3])];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::S(db.post_history.comment.get(h).unwrap_or("No comments")), V::T(db.post_history.creation_date.get(h).unwrap()), user_col(db, u, "name")]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS TotalBadges, ... GoldBadges, SilverBadges, BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostActivity AS (
//     SELECT P.OwnerUserId, COUNT(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopens, COUNT(CASE WHEN PH.PostHistoryTypeId IN (12, 13) THEN 1 END) AS PostDeletes,
//            COUNT(DISTINCT C.Id) AS CommentCount
//     FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.OwnerUserId),
// RecentVotes AS (SELECT V.UserId, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//                        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V WHERE V.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY V.UserId),
// UserStatistics AS (
//     SELECT U.Id, U.DisplayName, COALESCE(UBC.TotalBadges, 0) AS BadgeCount, COALESCE(PA.CommentCount, 0) AS CommentCount, COALESCE(RV.TotalVotes, 0) AS TotalVotes,
//            COALESCE(RV.UpVotes, 0) AS UpVotes, COALESCE(RV.DownVotes, 0) AS DownVotes,
//            CASE WHEN COALESCE(PA.CloseReopens, 0) + COALESCE(PA.PostDeletes, 0) > 0 THEN 'Active' ELSE 'Inactive' END AS ActivityStatus
//     FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN PostActivity PA ON U.Id = PA.OwnerUserId LEFT JOIN RecentVotes RV ON U.Id = RV.UserId)
// SELECT U.DisplayName, U.BadgeCount, U.CommentCount, U.TotalVotes, U.UpVotes, U.DownVotes, U.ActivityStatus FROM UserStatistics U
// WHERE U.BadgeCount > 0 AND U.TotalVotes > 0 AND U.CommentCount > (SELECT AVG(CommentCount) FROM UserStatistics)
// ORDER BY U.BadgeCount DESC, U.TotalVotes DESC LIMIT 10;
fn q20028(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pa = owned(db)
        .group_by(&db.post.owner_user)
        .select(history_of(db).select(&db.post_history.post_history_type_id).opt().and(comments_of(db).opt()))
        .fold((0i64, 0i64), |(c, d), (t, _)| (c + matches!(t, Some(10 | 11)) as i64, d + matches!(t, Some(12 | 13)) as i64));
    let pc = owned(db).group_by(&db.post.owner_user).select(comments_of(db).opt()).buf_fold(distinct_some);
    let rv = db
        .vote
        .with((&db.vote.creation_date).ge(add_days(date(2024, 10, 1), -30)))
        .group_by(&db.vote.user)
        .select(&db.vote.vote_type_id)
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let stats = || db.user.select(Ident::<User>::new().and(&bc).and((&pa).opt()).and((&pc).opt()).and((&rv).opt()));
    let (sum, n) = db.user.select((&pc).opt()).fold_flat((0i64, 0i64), |(s, n), c| (s + c.unwrap_or(0), n + 1));
    let mean = sum as f64 / n as f64;
    let v = drain(stats().filt(move |((((_, b), _), c), r)| b > 0 && r.map_or(0, |r| r[0]) > 0 && c.unwrap_or(0) as f64 > mean));
    let v = top_n(v, |&(_, ((((_, b), _), _), r))| (Reverse(b), Reverse(r.map_or(0, |r| r[0]))), 10);
    rows(v.iter().map(|&(_, ((((u, b), p), c), r))| {
        let r = r.unwrap_or([0; 3]);
        let p = p.unwrap_or((0, 0));
        row(vec![user_col(db, u, "name"), V::I(b), V::I(c.unwrap_or(0)), V::I(r[0]), V::I(r[1]), V::I(r[2]), V::S(if p.0 + p.1 > 0 { "Active" } else { "Inactive" })])
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, MAX(B.Date) AS LastBadgeDate FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostActivity AS (
//     SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, COALESCE(SUM(P.ViewCount), 0) AS TotalViews, COALESCE(AVG(P.Score), 0) AS AverageScore,
//            MAX(P.LastActivityDate) AS LastPostActivity
//     FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     WHERE PH.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' OR PH.PostHistoryTypeId IN (10, 11) GROUP BY P.OwnerUserId),
// UserPerformance AS (
//     SELECT UB.UserId, UB.DisplayName, UB.BadgeCount, PA.PostCount, PA.TotalViews, PA.AverageScore,
//            CASE WHEN PA.LastPostActivity IS NULL THEN NULL ELSE DATE_PART('day', cast('2024-10-01 12:34:56' as timestamp) - PA.LastPostActivity) END AS DaysSinceLastPost
//     FROM UserBadges UB LEFT JOIN PostActivity PA ON UB.UserId = PA.OwnerUserId ORDER BY UB.BadgeCount DESC, PA.TotalViews DESC)
// SELECT U.DisplayName, U.BadgeCount, U.PostCount, U.TotalViews, U.AverageScore, U.DaysSinceLastPost,
//        CASE WHEN U.PostCount > 50 THEN 'Active Contributor' WHEN U.BadgeCount > 10 THEN 'Badge Collector' ELSE 'Newcomer' END AS UserType,
//        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.UserId = U.UserId AND V.VoteTypeId = 2), 0) AS UpvotesGiven,
//        COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.UserId = U.UserId), 0) AS CommentsMade
// FROM UserPerformance U WHERE U.PostCount > 0 AND U.BadgeCount IS NOT NULL AND U.DaysSinceLastPost < 30 ORDER BY U.AverageScore DESC LIMIT 10;
//
// The WHERE on PH drops the LEFT JOIN's NULL rows: an inner join, counted
// per qualifying history row.
fn q20141(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { creation_date, post_history_type_id, .. } = &db.post_history;
    let qh = history_of(db).with(creation_date.gt(add_months(now, -6)).or(post_history_type_id.is_in([10, 11])));
    let pa = owned(db)
        .group_by(&db.post.owner_user)
        .select((&db.post.view_count).opt().and(&db.post.score).and(&db.post.last_activity_date).and(qh))
        .fold([0, 0, 0, 0, i64::MIN], |a, (((v, s), la), _)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s, a[4].max(la)]);
    let up = db.vote.with((&db.vote.vote_type_id).eq(2)).group_by(&db.vote.user).fold(0i64, |n, _| n + 1);
    let cm = db.comment.group_by(&db.comment.user).fold(0i64, |n, _| n + 1);
    let days = |a: [i64; 5]| (now - a[4]) / DAY_US;
    let v = drain((&bc).and((&pa).filt(move |a| a[0] > 0 && (now - a[4]) / DAY_US < 30)).and((&up).opt()).and((&cm).opt()));
    let v = top_n(v, |&(_, (((_, a), _), _))| Reverse(fkey(a[3] as f64 / a[0] as f64)), 10);
    rows(v.iter().map(|&(u, (((b, a), up), c))| {
        let t = if a[0] > 50 { "Active Contributor" } else if b > 10 { "Badge Collector" } else { "Newcomer" };
        row(vec![user_col(db, u, "name"), V::I(b), V::I(a[0]), V::I(a[2]), avg(a[3], a[0]), V::I(days(a)), V::S(t), V::I(up.unwrap_or(0)), V::I(c.unwrap_or(0))])
    }))
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
//            SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikis, AVG(u.Reputation) AS AvgReputation, MAX(p.CreationDate) AS LastActiveDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (
//     SELECT p.Id AS PostId, p.Title, COALESCE(ph.Comment, 'No comment') AS LastEditComment, COUNT(c.Id) AS CommentCount,
//            SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, MAX(p.LastActivityDate) AS LastActivity
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON ph.PostId = p.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - interval '1 year' GROUP BY p.Id, p.Title, ph.Comment),
// TopUsers AS (
//     SELECT ua.UserId, ua.DisplayName, ua.PostCount, ua.Questions, ua.Answers, ua.Wikis, ua.TagWikis, ua.AvgReputation, ua.LastActiveDate,
//            ps.CommentCount, ps.Upvotes, ps.Downvotes
//     FROM UserActivity ua JOIN PostStatistics ps ON ua.UserId = ps.PostId ORDER BY ua.PostCount DESC LIMIT 10)
// SELECT tu.DisplayName, tu.PostCount, tu.Questions, tu.Answers, tu.Wikis, tu.TagWikis, tu.AvgReputation, tu.LastActiveDate, ps.CommentCount, ps.Upvotes, ps.Downvotes
// FROM TopUsers tu JOIN PostStatistics ps ON tu.PostCount = ps.PostId ORDER BY tu.AvgReputation DESC;
//
// Both joins match a user Id or a post count against a post Id, as written.
fn q9976(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(100));
    let ua = rich()
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select((&db.post.post_type_id).and(&db.post.creation_date)).opt()))
        .fold([0, 0, 0, 0, 0, 0, i64::MIN, 0], |a, (r, p)| match p {
            Some((t, d)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + matches!(t, 4 | 5) as i64, a[4] + r, a[5] + 1, a[6].max(d), a[7]],
            None => [a[0], a[1], a[2], a[3], a[4] + r, a[5] + 1, a[6], a[7]],
        });
    let dp = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let recent = || db.post.with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    type K = (Id<Post>, Option<Str>);
    let ps = recent()
        .select(Ident::<Post>::new().and(history_of(db).select((&db.post_history.comment).opt()).opt().map(|c| c.flatten())))
        .group_by(Same::<K>::new())
        .select(Same::<K>::new().map(|(p, _)| p).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let keys: MatSet<K> = whole(&ps).collect();
    let by_id: HashIdx<i64, K> = (&keys).map(|(p, _)| p).select(&db.post.origid).inv().collect();
    let tu = drain((&dp).and(&ua).and((&db.user.origid).select((&by_id).select(&ps))));
    let tu = rel(top_n(tu, |&(_, ((n, _), _))| Reverse(n), 10));
    let mut out = Vec::new();
    (&tu).and((&tu).map(|(_, ((n, _), _))| n).select((&by_id).select(&ps))).drive(|_, ((u, ((n, a), _)), b)| {
        out.push(row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[5]), tmax(a[6]), V::I(b[0]), V::I(b[1]), V::I(b[2])]))
    });
    rows(out)
}

// WITH TagStats AS (
//     SELECT tag.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//            SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount, SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN p.ViewCount ELSE 0 END) AS TotalViewCount
//     FROM Tags AS tag LEFT JOIN Posts AS p ON p.Tags LIKE '%' || tag.TagName || '%' GROUP BY tag.TagName),
// UserReputation AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(coalesce(b.Class, 0)) AS TotalBadges, AVG(coalesce(r.Reputation, 0)) AS AvgReputation
//     FROM Users AS u LEFT JOIN Posts AS p ON p.OwnerUserId = u.Id LEFT JOIN Badges AS b ON b.UserId = u.Id LEFT JOIN Users AS r ON r.Id = u.Id GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPostActivity AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, count(c.Id) AS CommentCount, MAX(v.CreationDate) AS LastVoteDate
//     FROM Posts AS p LEFT JOIN Comments AS c ON c.PostId = p.Id LEFT JOIN Votes AS v ON v.PostId = p.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount)
// SELECT ts.TagName, ts.PostCount, ts.QuestionCount, ts.AnswerCount, ts.WikiCount, ts.TotalViewCount, ur.DisplayName AS TopUser, ur.Reputation AS TopUserReputation,
//        ur.PostsCreated AS UserPostCount, ur.TotalBadges AS UserBadges, rpa.PostId, rpa.Title, rpa.CreationDate, rpa.ViewCount AS RecentPostViewCount,
//        rpa.CommentCount AS RecentCommentCount, rpa.LastVoteDate
// FROM TagStats AS ts JOIN UserReputation AS ur ON ur.Reputation = (SELECT MAX(Reputation) FROM UserReputation)
// JOIN RecentPostActivity AS rpa ON rpa.ViewCount = (SELECT MAX(ViewCount) FROM RecentPostActivity)
// ORDER BY ts.TotalViewCount DESC, ts.PostCount DESC LIMIT 10;
fn q29558(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select((&db.post.post_type_id).and((&db.post.view_count).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some((t, v)) => {
                let w = if matches!(t, 1 | 2) { v } else { Some(0) };
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
            }
            None => [a[0], a[1], a[2], a[3], a[4] + 1, a[5]],
        });
    let top = db.user.select(&db.user.reputation).fold_flat(i64::MIN, |m, r| m.max(r));
    let ur = db
        .user
        .with((&db.user.reputation).eq(top))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold(0i64, |s, (_, c)| s + c.unwrap_or(0));
    let urd = db.user.with((&db.user.reputation).eq(top)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let recent = || db.post.with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let vmax = recent().select(&db.post.view_count).fold_flat(i64::MIN, |m, v| m.max(v));
    let rpa = recent()
        .with((&db.post.view_count).eq(vmax))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.creation_date).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, d.map_or(m, |d| m.max(d))));
    let users = rel(drain((&ur).and(&urd)));
    let posts = rel(drain(&rpa));
    let v = drain((&ts_).cross(&users).cross(&posts));
    let v = top_n(v, |&(_, ((a, _), _))| (a[4] == 0, Reverse(a[5]), Reverse(a[0])), 10);
    rows(v.iter().map(|&(((t, _), _), ((a, (u, (b, n))), (p, (c, d))))| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4])];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(n), V::I(b)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.extend([V::I(c), tmax(d)]);
        row(f)
    }))
}

// WITH RecentQuestions AS (
//     SELECT p.Id AS QuestionId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// QuestionStats AS (
//     SELECT rq.*, COALESCE(COUNT(DISTINCT c.Id), 0) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//            COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, ...
//     FROM RecentQuestions rq LEFT JOIN Comments c ON rq.QuestionId = c.PostId LEFT JOIN Votes v ON rq.QuestionId = v.PostId
//     LEFT JOIN Badges b ON rq.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = b.UserId)
//     GROUP BY rq.QuestionId, rq.Title, rq.CreationDate, rq.ViewCount, rq.Score, rq.OwnerDisplayName),
// OverallStats AS (SELECT COUNT(*) AS TotalQuestions, SUM(ViewCount) AS TotalViews, SUM(Score) AS TotalScore, SUM(CommentCount) AS TotalComments, ... FROM QuestionStats)
// SELECT qs.*, os.* FROM QuestionStats qs CROSS JOIN OverallStats os ORDER BY qs.CreationDate DESC LIMIT 10;
//
// The badge join matches on the owner's display name, so it takes the
// badges of every user with that name.
fn q6966(db: &'static So) -> String {
    let by_name: HashIdx<Str, Id<Badge>> = (&db.badge.user).select(&db.user.display_name).inv().collect();
    let rq = || owned(db).with((&db.post.post_type_id).eq(1)).with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let qs = rq()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and((&db.post.owner_user).select(&db.user.display_name).select((&by_name).select(&db.badge.class).opt())))
        .fold([0i64; 5], |a, ((t, _), c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]);
    let dc = rq().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(distinct_some);
    let os = rq().select((&db.post.view_count).opt().and(&db.post.score).and(&qs).and(&dc)).fold_flat([0i64; 10], |a, (((v, s), q), c)| {
        [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s, a[4] + c, a[5] + q[0], a[6] + q[1], a[7] + q[2], a[8] + q[3], a[9] + q[4]]
    });
    let v = top_n(drain((&qs).and(&dc)), |&(p, _)| Reverse(db.post.creation_date.get(p).unwrap()), 10);
    rows(v.iter().map(|&(p, (q, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(c), V::I(q[0]), V::I(q[1]), V::I(q[2]), V::I(q[3]), V::I(q[4])]);
        f.extend([V::I(os[0]), nullable(os[2], os[1]), V::I(os[3]), V::I(os[4]), V::I(os[5]), V::I(os[6]), V::I(os[7]), V::I(os[8]), V::I(os[9])]);
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCounts AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount, ... Silver, Bronze
//                                    FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostMetrics AS (
//     SELECT p.OwnerUserId, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//            COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount, MAX(p.CreationDate) AS LastPostDate
//     FROM Posts p GROUP BY p.OwnerUserId),
// CombinedMetrics AS (
//     SELECT u.Id AS UserId, u.DisplayName, COALESCE(ubc.GoldBadgeCount, 0) AS GoldBadgeCount, ..., COALESCE(pm.AvgViewCount, 0) AS AvgViewCount, pm.LastPostDate
//     FROM Users u LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId LEFT JOIN PostMetrics pm ON u.Id = pm.OwnerUserId)
// SELECT c.UserId, c.DisplayName, c.GoldBadgeCount, c.SilverBadgeCount, c.BronzeBadgeCount, c.TotalPosts, c.TotalQuestions, c.TotalAnswers, c.TotalScore, c.AvgViewCount,
//        CASE WHEN c.LastPostDate IS NOT NULL THEN DATE '2024-10-01' - c.LastPostDate ELSE NULL END AS DaysSinceLastPost
// FROM CombinedMetrics c WHERE (c.TotalQuestions > 0 OR c.TotalAnswers > 0) AND (c.GoldBadgeCount > 0 OR c.SilverBadgeCount > 0 OR c.BronzeBadgeCount > 0)
// ORDER BY c.TotalScore DESC, c.TotalPosts DESC LIMIT 100;
//
// RECURSIVE is written but nothing recurses.
fn q31859(db: &'static So) -> String {
    let ubc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ups = user_posts(db);
    let v = drain((&ups).filt(|a| a[2] > 0 || a[3] > 0).and((&ubc).filt(|b| b[0] > 0 || b[1] > 0 || b[2] > 0)));
    let v = top_n(v, |&(_, (a, _))| (Reverse(a[4]), Reverse(a[1])), 100);
    let day0 = date(2024, 10, 1);
    rows(v.iter().map(|&(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        f.push(if a[5] == 0 { V::F(0.0) } else { avg(a[6], a[5]) });
        f.push(if a[7] == i64::MIN { V::Null } else { V::Iv(day0 - a[7]) });
        row(f)
    }))
}

// WITH UserPostStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(P.Score) AS AvgPostScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// QuestionStats AS (
//     SELECT Q.Id AS QuestionId, Q.Title, COALESCE(V.UpVotes, 0) AS UpVoteCount, COALESCE(C.CommentCount, 0) AS CommentCount,
//            CASE WHEN Q.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END AS HasAcceptedAnswer, Q.OwnerUserId
//     FROM Posts Q LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON C.PostId = Q.Id
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes FROM Votes GROUP BY PostId) V ON V.PostId = Q.Id WHERE Q.PostTypeId = 1),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, ... SilverBadges, BronzeBadges FROM Badges B GROUP BY B.UserId)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.AvgPostScore, COUNT(DISTINCT Q.QuestionId) AS TotalQuestionsWithStats,
//        SUM(Q.UpVoteCount) AS TotalUpVotes, SUM(Q.CommentCount) AS TotalComments, SUM(Q.HasAcceptedAnswer) AS TotalAcceptedAnswers,
//        COALESCE(B.GoldBadges, 0) AS GoldBadges, COALESCE(B.SilverBadges, 0) AS SilverBadges, COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM UserPostStats U LEFT JOIN QuestionStats Q ON U.UserId = Q.OwnerUserId LEFT JOIN UserBadges B ON U.UserId = B.UserId
// GROUP BY U.UserId, U.DisplayName, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.AvgPostScore, B.GoldBadges, B.SilverBadges, B.BronzeBadges
// ORDER BY U.TotalPosts DESC, U.AvgPostScore DESC FETCH FIRST 50 ROWS ONLY;
fn q3245(db: &'static So) -> String {
    let ups = user_posts(db);
    let up = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |n, t| n + (t == 2) as i64);
    let cc = db.comment.group_by(&db.comment.post).fold(0i64, |n, _| n + 1);
    let qs = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).with((&db.post.post_type_id).eq(1)).select((&up).opt().and((&cc).opt()).and((&db.post.accepted_answer_id).opt())))
        .fold([0i64; 4], |a, ((u, c), x)| [a[0] + 1, a[1] + u.unwrap_or(0), a[2] + c.unwrap_or(0), a[3] + x.is_some() as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&ups).and((&qs).opt()).and((&ub).opt()));
    let v = top_n(v, |&(_, ((a, _), _))| (Reverse(a[1]), a[1] == 0, Reverse(if a[1] == 0 { 0 } else { fkey(a[4] as f64 / a[1] as f64) })), 50);
    rows(v.iter().map(|&(u, ((a, q), b))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[1])]);
        f.extend(match q {
            Some(q) => [V::I(q[0]), V::I(q[1]), V::I(q[2]), V::I(q[3])],
            None => [V::I(0), V::Null, V::Null, V::Null],
        });
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        row(f)
    }))
}

// WITH UserReputation AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//            AVG(V.BountyAmount) AS AvgBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 9 GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostHistoryStats AS (
//     SELECT PH.PostId, COUNT(DISTINCT PH.Id) AS HistoryChanges, MIN(PH.CreationDate) AS FirstChangeDate, MAX(PH.CreationDate) AS LastChangeDate,
//            COUNT(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount
//     FROM PostHistory PH GROUP BY PH.PostId),
// UserPostSummary AS (
//     SELECT U.UserId, U.DisplayName, U.Reputation, COALESCE(SUM(P.ViewCount), 0) AS TotalViewCount, COALESCE(SUM(P.Score), 0) AS TotalScore,
//            COALESCE(SUM(P.AnswerCount), 0) AS TotalAnswerCount, COALESCE(SUM(P.CommentCount), 0) AS TotalCommentCount, PH.HistoryChanges, PH.FirstChangeDate,
//            PH.LastChangeDate, PH.CloseReopenCount
//     FROM UserReputation U LEFT JOIN Posts P ON U.UserId = P.OwnerUserId LEFT JOIN PostHistoryStats PH ON P.Id = PH.PostId
//     GROUP BY U.UserId, U.DisplayName, U.Reputation, PH.HistoryChanges, PH.FirstChangeDate, PH.LastChangeDate, PH.CloseReopenCount)
// SELECT U.DisplayName, U.Reputation, U.TotalViewCount, U.TotalScore, U.TotalAnswerCount, U.TotalCommentCount, U.HistoryChanges, U.FirstChangeDate, U.LastChangeDate,
//        U.CloseReopenCount, CASE WHEN U.TotalViewCount > 1000 THEN 'Highly viewed' ELSE 'Moderately viewed' END AS ViewStatus,
//        CASE WHEN U.TotalAnswerCount > 50 THEN 'Expert' WHEN U.TotalAnswerCount BETWEEN 10 AND 50 THEN 'Intermediate' ELSE 'Novice' END AS ExpertiseLevel
// FROM UserPostSummary U ORDER BY U.Reputation DESC, U.TotalScore DESC LIMIT 10;
//
// UserPostSummary groups each user's posts by their history summary, so a
// user has one row per distinct (changes, first, last, closes) tuple.
fn q1987(db: &'static So) -> String {
    let PostHistory { creation_date, post_history_type_id, .. } = &db.post_history;
    let phs = db
        .post_history
        .group_by(&db.post_history.post)
        .select(creation_date.and(post_history_type_id))
        .fold((0i64, i64::MAX, i64::MIN, 0i64), |(n, lo, hi, c), (d, t)| (n + 1, lo.min(d), hi.max(d), c + matches!(t, 10 | 11) as i64));
    let Post { view_count, score, answer_count, comment_count, owner_user, .. } = &db.post;
    let ups = owned(db)
        .group_by(owner_user.and((&phs).opt()))
        .select(view_count.opt().and(score).and(answer_count.opt()).and(comment_count))
        .fold([0i64; 4], |a, (((v, s), an), c)| [a[0] + v.unwrap_or(0), a[1] + s, a[2] + an.unwrap_or(0), a[3] + c]);
    let mut v: Vec<(Id<User>, Option<(i64, i64, i64, i64)>, [i64; 4])> = Vec::new();
    whole(&ups).select(Same::new().and(&ups)).drive(|_, ((u, h), a)| v.push((u, h, a)));
    db.user.minus(posts_of(db)).drive(|u, _| v.push((u, None, [0; 4])));
    let v = top_n(v, |&(u, _, a)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1])), 10);
    rows(v.iter().map(|&(u, h, a)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(match h {
            Some((n, lo, hi, c)) => [V::I(n), V::T(lo), V::T(hi), V::I(c)],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::S(if a[0] > 1000 { "Highly viewed" } else { "Moderately viewed" }));
        f.push(V::S(if a[2] > 50 { "Expert" } else if (10..=50).contains(&a[2]) { "Intermediate" } else { "Novice" }));
        row(f)
    }))
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore,
//            AVG(COALESCE(ROUND(EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - p.CreationDate)) / 3600.0), 0)) AS AverageAgeHours
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostVoteSummary AS (SELECT p.OwnerUserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//                            SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.OwnerUserId),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges, ... GoldBadges, SilverBadges, BronzeBadges FROM Badges b GROUP BY b.UserId),
// FinalMetrics AS (SELECT u.DisplayName, ua.TotalPosts, ua.TotalAnswers, ua.TotalQuestions, ua.TotalViews, ua.TotalScore, ua.AverageAgeHours, vs.TotalVotes, vs.UpVotes,
//                         vs.DownVotes, bs.TotalBadges, bs.GoldBadges, bs.SilverBadges, bs.BronzeBadges
//                  FROM UserActivity ua JOIN PostVoteSummary vs ON ua.UserId = vs.OwnerUserId LEFT JOIN UserBadges bs ON ua.UserId = bs.UserId JOIN Users u ON ua.UserId = u.Id)
// SELECT * FROM FinalMetrics ORDER BY TotalScore DESC, TotalViews DESC LIMIT 10;
fn q9388(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let ups = user_posts(db);
    let age = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.creation_date)).fold(0i64, |s, d| s + (secs(now - d) / 3600.0).round() as i64);
    let pvs = owned(db)
        .group_by(&db.post.owner_user)
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain((&ups).and((&age).opt()).and(&pvs).and((&ub).opt()));
    let v = top_n(v, |&(_, (((a, _), _), _))| (a[1] == 0, Reverse(a[4]), a[5] == 0, Reverse(a[6])), 10);
    rows(v.iter().map(|&(u, (((a, g), p), b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[3]), V::I(a[2]), nullable(a[6], a[5]), nullable(a[4], a[1]), avg(g.unwrap_or(0), a[0])];
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2])]);
        f.extend(match b {
            Some(b) => [V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}


// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, ... Silver, Bronze
//                    FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//                      AVG(p.Score) AS AvgScore, COUNT(DISTINCT p.Tags) AS UniqueTags
//               FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// CommentStats AS (SELECT c.UserId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.UserId),
// VotingStats AS (SELECT v.UserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//                 FROM Votes v GROUP BY v.UserId)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.Views, us.GoldBadges, us.SilverBadges, us.BronzeBadges, COALESCE(ps.PostCount, 0) AS TotalPosts, ...,
//        COALESCE(ps.AvgScore, 0) AS AverageScore, COALESCE(ps.UniqueTags, 0) AS UniqueTagsCount, COALESCE(cs.CommentCount, 0) AS TotalComments, COALESCE(vs.VoteCount, 0) AS TotalVotes, ...
// FROM UserStats us LEFT JOIN PostStats ps ON us.UserId = ps.OwnerUserId LEFT JOIN CommentStats cs ON us.UserId = cs.UserId LEFT JOIN VotingStats vs ON us.UserId = vs.UserId
// ORDER BY us.Reputation DESC, us.Views DESC;
fn q1415(db: &'static So) -> String {
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let recent = || owned(db).with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ps = recent().group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let ut = recent().group_by(&db.post.owner_user).select(&db.post.tags_str).count_distinct();
    let cs = db.comment.group_by(&db.comment.user).fold(0i64, |n, _| n + 1);
    let vs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut out = Vec::new();
    db.user.select(Ident::<User>::new().and((&ub).opt()).and((&ps).opt()).and((&ut).opt()).and((&cs).opt()).and((&vs).opt())).drive(|_, (((((u, b), p), t), c), v)| {
        let b = b.unwrap_or([0; 3]);
        let v = v.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        f.extend(match p {
            Some(p) => [V::I(p[0]), V::I(p[1]), V::I(p[2]), avg(p[3], p[0])],
            None => [V::I(0), V::I(0), V::I(0), V::F(0.0)],
        });
        f.extend([V::I(t.unwrap_or(0)), V::I(c.unwrap_or(0)), V::I(v[0]), V::I(v[1]), V::I(v[2])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, ... FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//                           SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AverageViews FROM Posts P GROUP BY P.OwnerUserId),
// RecentPostActivity AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.LastActivityDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 1 END) AS RecentActivityCount
//                        FROM Posts P GROUP BY P.OwnerUserId),
// UserActivitySummary AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UBC.BadgeCount, 0) AS BadgeCount, COALESCE(PS.PostCount, 0) AS PostCount, ...,
//                                COALESCE(PS.AverageViews, 0) AS AverageViews, COALESCE(RPA.RecentActivityCount, 0) AS RecentActivityCount
//                         FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId LEFT JOIN RecentPostActivity RPA ON U.Id = RPA.OwnerUserId)
// SELECT UAS.UserId, UAS.DisplayName, UAS.BadgeCount, UAS.PostCount, UAS.QuestionCount, UAS.AnswerCount, UAS.TotalScore, UAS.AverageViews, UAS.RecentActivityCount
// FROM UserActivitySummary UAS
// WHERE UAS.BadgeCount > 0 AND (UAS.TotalScore >= (SELECT AVG(TotalScore) FROM PostStatistics WHERE TotalScore IS NOT NULL) OR UAS.RecentActivityCount > 5)
// ORDER BY UAS.TotalScore DESC, UAS.QuestionCount DESC LIMIT 10;
//
// The AVG is over every PostStatistics group, the posts with no owner and
// the ones whose owner is gone included.
fn q22203(db: &'static So) -> String {
    let since = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let bc = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let Post { post_type_id, score, view_count, last_activity_date, .. } = &db.post;
    let ps = owned(db)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(score).and(view_count.opt()).and(last_activity_date))
        .fold([0i64; 7], |a, (((t, s), v), la)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + v.is_some() as i64, a[5] + v.unwrap_or(0), a[6] + (la >= since) as i64]);
    let all = db.post.group_by((&db.post.owner_user_id).opt()).select(score).fold(0i64, |s, x| s + x);
    let (n, s) = whole(&all).select(&all).fold_flat((0i64, 0i64), |(n, t), x| (n + 1, t + x));
    let mean = s as f64 / n as f64;
    let v = drain((&bc).and((&ps).opt()).filt(move |(_, p)| p.map_or(0, |p| p[3]) as f64 >= mean || p.map_or(0, |p| p[6]) > 5));
    let v = top_n(v, |&(_, (_, p))| (Reverse(p.map_or(0, |p| p[3])), Reverse(p.map_or(0, |p| p[1]))), 10);
    rows(v.iter().map(|&(u, (b, p))| {
        let p = p.unwrap_or([0; 7]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[3]), if p[4] == 0 { V::F(0.0) } else { avg(p[5], p[4]) }, V::I(p[6])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//                               SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS ChangeCount FROM PostHistory ph
//                        WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY ph.PostId, ph.PostHistoryTypeId),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//                 FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY v.PostId),
// MergedDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(up.PostCount, 0) AS UserPosts, COALESCE(up.TotalViews, 0) AS UserViews, COALESCE(up.TotalScore, 0) AS UserScore,
//                          COALESCE(phd.ChangeCount, 0) AS HistoryCount, COALESCE(rv.VoteCount, 0) AS RecentVoteCount, RV.UpVotes, RV.DownVotes
//                   FROM Posts p LEFT JOIN UserPostStats up ON p.OwnerUserId = up.UserId LEFT JOIN PostHistoryDetails phd ON p.Id = phd.PostId
//                   LEFT JOIN RecentVotes rv ON p.Id = rv.PostId WHERE p.CreationDate >= '2023-01-01')
// SELECT md.*, CASE WHEN md.RecentVoteCount > 50 THEN 'Very Active' WHEN md.RecentVoteCount > 20 THEN 'Active' ELSE 'Inactive' END AS ActivityLevel
// FROM MergedDetails md ORDER BY md.UserScore DESC, md.RecentVoteCount DESC LIMIT 100;
fn q24305(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let ups = user_posts(db);
    let phd = db
        .post_history
        .with((&db.post_history.creation_date).ge(add_years(now, -1)))
        .group_by((&db.post_history.post).and(&db.post_history.post_history_type_id))
        .fold(0i64, |n, _| n + 1);
    let keys: MatSet<(Id<Post>, i64)> = whole(&phd).collect();
    let phd_by: HashIdx<Id<Post>, (Id<Post>, i64)> = (&keys).map(|(p, _)| p).inv().collect();
    let rv = db
        .vote
        .with((&db.vote.creation_date).ge(add_months(now, -6)))
        .group_by(&db.vote.post)
        .select(vtype_name(db))
        .fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let md = db
        .post
        .with((&db.post.creation_date).ge(date(2023, 1, 1)))
        .select(Ident::<Post>::new().and((&db.post.owner_user).select(&ups).opt()).and((&phd_by).select(&phd).opt()).and((&rv).opt()));
    let v = top_n(drain(md), |&(_, (((_, a), _), r))| (Reverse(a.map_or(0, |a| a[4])), Reverse(r.map_or(0, |r| r[0]))), 100);
    rows(v.iter().map(|&(_, (((p, a), h), r))| {
        let a = a.unwrap_or([0; 10]);
        let rc = r.map_or(0, |r| r[0]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[1]), V::I(a[6]), V::I(a[4]), V::I(h.unwrap_or(0)), V::I(rc), oint(r.map(|r| r[1])), oint(r.map(|r| r[2]))]);
        f.push(V::S(if rc > 50 { "Very Active" } else if rc > 20 { "Active" } else { "Inactive" }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, CONCAT(u.DisplayName, ' (Reputation: ', u.Reputation, ')') AS UserInfo, COUNT(b.Id) AS BadgeCount, ... Gold, Silver, Bronze
//                          FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// ActivePosts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//                        SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts
//                 FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// PostHistoryAnalysis AS (SELECT ph.UserId, COUNT(ph.Id) AS Edits, COUNT(DISTINCT ph.PostId) AS EditedPosts FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6, 24) GROUP BY ph.UserId),
// CombinedData AS (SELECT ubc.*, ap.TotalPosts, ap.Questions, ap.Answers, ap.PopularPosts, pha.Edits, pha.EditedPosts
//                  FROM UserBadgeCounts ubc LEFT JOIN ActivePosts ap ON ubc.UserId = ap.OwnerUserId LEFT JOIN PostHistoryAnalysis pha ON ubc.UserId = pha.UserId)
// SELECT UserId, UserInfo, COALESCE(BadgeCount, 0) AS BadgeCount, ..., COALESCE(EditedPosts, 0) AS EditedPosts FROM CombinedData ORDER BY BadgeCount DESC, UserInfo DESC;
fn q29294(db: &'static So) -> String {
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let ap = owned(db)
        .with((&db.post.creation_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(&db.post.owner_user)
        .select((&db.post.post_type_id).and((&db.post.view_count).opt()))
        .fold([0i64; 4], |a, (t, v)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v.unwrap_or(0) > 100) as i64]);
    let edits = || db.post_history.with((&db.post_history.post_history_type_id).is_in([4, 5, 6, 24]));
    let pha = edits().group_by(&db.post_history.user).fold(0i64, |n, _| n + 1);
    let phd = edits().group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let mut out = Vec::new();
    db.user.select(Ident::<User>::new().and((&ub).opt()).and((&ap).opt()).and((&pha).opt()).and((&phd).opt())).drive(|_, ((((u, b), a), e), d)| {
        let b = b.unwrap_or([0; 4]);
        let a = a.unwrap_or([0; 4]);
        let info = format!("{} (Reputation: {})", db.user.display_name.get(u).unwrap(), db.user.reputation.get(u).unwrap());
        let mut f = vec![user_col(db, u, "uid"), V::Owned(info)];
        f.extend(b.iter().chain(a.iter()).map(|&x| V::I(x)));
        f.extend([V::I(e.unwrap_or(0)), V::I(d.unwrap_or(0))]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, ... FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews FROM Posts P GROUP BY P.OwnerUserId),
// ClosedPostCounts AS (SELECT Ph.UserId, COUNT(DISTINCT Ph.PostId) AS ClosedPosts FROM PostHistory Ph WHERE Ph.PostHistoryTypeId = 10 GROUP BY Ph.UserId),
// CombinedStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.PostCount, 0) AS PostCount,
//                               COALESCE(PS.TotalScore, 0) AS TotalScore, COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(CPC.ClosedPosts, 0) AS ClosedPosts
//                        FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId LEFT JOIN ClosedPostCounts CPC ON U.Id = CPC.UserId)
// SELECT UserId, DisplayName, Reputation, BadgeCount, PostCount, TotalScore, TotalViews, ClosedPosts,
//        CASE WHEN Reputation > 1000 THEN 'High Reputation' ELSE 'Newbie' END AS UserLevel,
//        CASE WHEN TotalScore < 0 THEN 'Negative Score' WHEN TotalScore BETWEEN 0 AND 100 THEN 'Low Score' WHEN TotalScore BETWEEN 101 AND 500 THEN 'Moderate Score' ELSE 'High Score' END AS ScoreCategory,
//        CONCAT('User: ', DisplayName, ' has total views: ', TotalViews) AS UserViewMessage, NULLIF((BadgeCount - (ClosedPosts + PostCount)), 0) AS BadgeDeficiency
// FROM CombinedStatistics WHERE (BadgeCount > 0 OR ClosedPosts > 0) AND Reputation IS NOT NULL ORDER BY Reputation DESC, TotalScore ASC OFFSET 0 ROWS FETCH NEXT 100 ROWS ONLY;
fn q23758(db: &'static So) -> String {
    let bc = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 3], |a, (s, v)| [a[0] + 1, a[1] + s, a[2] + v.unwrap_or(0)]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let cs = db.user.select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt()).and((&cp).opt())).filt(|(((_, b), _), c)| b.unwrap_or(0) > 0 || c.unwrap_or(0) > 0);
    let v = top_n(drain(cs), |&(_, (((u, _), p), _))| (Reverse(db.user.reputation.get(u).unwrap()), p.map_or(0, |p| p[1])), 100);
    rows(v.iter().map(|&(_, (((u, b), p), c))| {
        let (b, c, p) = (b.unwrap_or(0), c.unwrap_or(0), p.unwrap_or([0; 3]));
        let rep = db.user.reputation.get(u).unwrap();
        let name = db.user.display_name.get(u).unwrap();
        let cat = if p[1] < 0 { "Negative Score" } else if p[1] <= 100 { "Low Score" } else if p[1] <= 500 { "Moderate Score" } else { "High Score" };
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(c), V::S(if rep > 1000 { "High Reputation" } else { "Newbie" }), V::S(cat)]);
        f.push(V::Owned(format!("User: {name} has total views: {}", p[2])));
        let d = b - (c + p[0]);
        f.push(if d == 0 { V::Null } else { V::I(d) });
        row(f)
    }))
}

// WITH UsersWithBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, ... Gold, Silver, Bronze FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostsWithVoteCounts AS (SELECT p.Id AS PostId, p.Title, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, ... DownVotes, ... (VoteTypeId = 4) OffensiveVotes
//                         FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title),
// ClosedPostHistory AS (SELECT ph.PostId, MIN(ph.CreationDate) AS FirstCloseDate, MAX(ph.CreationDate) AS LastCloseDate, COUNT(ph.Id) AS CloseCount
//                       FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// UserScoreAndPostCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, COALESCE(SUM(CASE WHEN p.Score > 0 THEN p.Score ELSE 0 END), 0) AS PositiveScore,
//                                   COALESCE(SUM(CASE WHEN p.Score < 0 THEN p.Score ELSE 0 END), 0) AS NegativeScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT u.UserId, u.DisplayName, u.BadgeCount, u.GoldBadges, u.SilverBadges, u.BronzeBadges, p.PostId, p.Title, p.UpVotes, p.DownVotes, p.OffensiveVotes,
//        c.FirstCloseDate, c.LastCloseDate, c.CloseCount, pc.PostCount, pc.PositiveScore, pc.NegativeScore
// FROM UsersWithBadges u JOIN PostsWithVoteCounts p ON u.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = p.PostId AND OwnerUserId IS NOT NULL LIMIT 1)
// LEFT JOIN ClosedPostHistory c ON p.PostId = c.PostId LEFT JOIN UserScoreAndPostCounts pc ON u.UserId = pc.UserId
// WHERE u.BadgeCount > 5 AND (p.UpVotes + p.DownVotes) > 10 AND COALESCE(c.CloseCount, 0) = 0 ORDER BY u.DisplayName, p.UpVotes DESC, pc.PostCount DESC;
//
// The correlated subquery is the post's own owner (Posts.Id is unique).
fn q22414(db: &'static So) -> String {
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(4)) as i64]);
    let closed = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).fold(0i64, |n, _| n + 1);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.score).opt()).fold((0i64, 0i64), |(p, n), s| (p + s.unwrap_or(0).max(0), n + s.unwrap_or(0).min(0)));
    let dp = user_distinct_posts(db);
    let mut out = Vec::new();
    owned(db)
        .minus(&closed)
        .select(Ident::<Post>::new().and((&pv).filt(|a| a[0] + a[1] > 10)).and((&db.post.owner_user).select(Ident::<User>::new().and((&ub).filt(|b| b[0] > 5)).and(&pc).and(&dp))))
        .drive(|_, ((p, a), (((u, b), (ps, ns)), n))| {
            let mut f = ucols(db, u, &["uid", "name"]);
            f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])]);
            f.extend(post_fields(db, p, &["id", "title"]));
            f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::Null, V::Null, V::Null, V::I(n), V::I(ps), V::I(ns)]);
            out.push(row(f))
        });
    rows(out)
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//            SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, COUNT(DISTINCT ph.PostId) AS PostHistoryCount, AVG(COALESCE(LENGTH(c.Text), 0)) AS AvgCommentLength
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE u.Reputation > 100 GROUP BY u.Id),
// TagStats AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AvgScore, MAX(p.CreationDate) AS LastPostDate
//              FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY t.TagName),
// CloseReasons AS (SELECT ph.PostId, ph.Comment AS CloseReason, COUNT(ph.Id) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.Comment),
// FinalResults AS (SELECT ua.*, ts.TagName, ts.PostCount, ts.TotalViews, ts.AvgScore, ts.LastPostDate, cr.CloseReason, cr.CloseCount
//                  FROM UserActivity ua LEFT JOIN TagStats ts ON ua.UserId = ts.PostCount
//                  LEFT JOIN CloseReasons cr ON ua.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = cr.PostId LIMIT 1))
// SELECT UserId, QuestionCount, AnswerCount, TotalBounty, PostHistoryCount, AvgCommentLength, TagName, PostCount, TotalViews, AvgScore, LastPostDate, CloseReason, CloseCount
// FROM FinalResults WHERE COALESCE(PostCount, 0) > 5 ORDER BY TotalBounty DESC, AvgScore DESC NULLS LAST LIMIT 100;
//
// UserActivity drives each user's posts x votes x comments x history in
// full. The TagStats join matches a user Id against a tag's post count, as
// written; the correlated subquery is the closed post's owner.
fn q20967(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(100));
    let ua = rich()
        .group_by(Ident::<User>::new())
        .select(
            posts_of(db)
                .select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).and(comments_of(db).select((&db.comment.text).map(|t| t.chars().count() as i64)).opt()).and(history_of(db).opt()))
                .opt(),
        )
        .fold([0i64; 5], |a, p| match p {
            Some((((t, b), l), _)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.flatten().unwrap_or(0), a[3] + l.unwrap_or(0), a[4] + 1],
            None => [a[0], a[1], a[2], a[3], a[4] + 1],
        });
    let uh = rich().group_by(Ident::<User>::new()).select(posts_of(db).select(history_of(db)).select(&db.post_history.post)).count_distinct();
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select((&db.post.view_count).opt().and(&db.post.score).and(&db.post.creation_date)))
        .fold([0, 0, 0, 0, i64::MIN], |a, ((v, s), d)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s, a[4].max(d)]);
    let ts_by: HashIdx<i64, Id<Tag>> = (&ts_).map(|a| a[0]).inv().collect();
    let cr = db
        .post_history
        .with((&db.post_history.post_history_type_id).eq(10))
        .group_by((&db.post_history.post).and((&db.post_history.comment).opt()))
        .fold(0i64, |n, _| n + 1);
    let keys: MatSet<(Id<Post>, Option<Str>)> = whole(&cr).collect();
    let cr_by: HashIdx<Id<User>, (Id<Post>, Option<Str>)> = (&keys).map(|(p, _)| p).select(&db.post.owner_user).inv().collect();
    let rows_ = drain(
        (&ua)
            .and((&uh).opt())
            .and((&db.user.origid).select((&ts_by).select(Ident::<Tag>::new().and(&ts_).filt(|(_, a)| a[0] > 5))))
            .and((&cr_by).select(Same::new().and(&cr)).opt()),
    );
    let v = top_n(rows_, |&(_, (((a, _), (_, t)), _))| (Reverse(a[2]), Reverse(fkey(t[3] as f64 / t[0] as f64))), 100);
    rows(v.iter().map(|&(u, (((a, h), (g, t)), c))| {
        let mut f = vec![user_col(db, u, "uid"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(h.unwrap_or(0)), avg(a[3], a[4])];
        f.extend([V::S(db.tag.tag_name.get(g).unwrap()), V::I(t[0]), nullable(t[2], t[1]), avg(t[3], t[0]), tmax(t[4])]);
        f.extend(match c {
            Some(((_, r), n)) => [ostr(r), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, ... FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//                           AVG(P.Score) AS AverageScore, MAX(P.ViewCount) AS MaxViews, MIN(P.CreationDate) AS FirstPostDate FROM Posts P GROUP BY P.OwnerUserId),
// VoteSummary AS (SELECT V.UserId, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//                 FROM Votes V GROUP BY V.UserId)
// SELECT U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(PS.QuestionCount, 0) AS QuestionCount,
//        COALESCE(PS.AnswerCount, 0) AS AnswerCount, COALESCE(VS.TotalVotes, 0) AS TotalVotes, COALESCE(VS.UpVotes, 0) AS UpVotes, COALESCE(VS.DownVotes, 0) AS DownVotes,
//        CASE WHEN COALESCE(PS.FirstPostDate, cast('2024-10-01 12:34:56' as timestamp)) > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN 'Active' ELSE 'Inactive' END AS UserActivityStatus,
//        CONCAT('User: ', U.DisplayName, '; Badges: ', COALESCE(UB.BadgeCount, 0), '; Posts: ', COALESCE(PS.PostCount, 0), '; Questions: ', COALESCE(PS.QuestionCount, 0),
//               '; Answers: ', COALESCE(PS.AnswerCount, 0), '; Votes: ', COALESCE(VS.TotalVotes, 0)) AS Summary
// FROM Users U LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId LEFT JOIN VoteSummary VS ON U.Id = VS.UserId
// WHERE (UPPER(U.DisplayName) LIKE '%SQL%' OR U.Location IS NOT NULL) AND (UB.BadgeCount > 0 OR PS.PostCount > 0)
// ORDER BY BadgeCount DESC, PostCount DESC NULLS LAST, TotalVotes DESC;
fn q21342(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let bc = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.creation_date)).fold([0, 0, 0, i64::MAX], |a, (t, d)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].min(d)]);
    let vs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let named = (&db.user.display_name).filt(|n| n.to_uppercase().contains("SQL")).or(&db.user.location);
    let mut out = Vec::new();
    db.user
        .with(named)
        .select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt()).and((&vs).opt()))
        .filt(|(((_, b), p), _)| b.unwrap_or(0) > 0 || p.map_or(0, |p| p[0]) > 0)
        .drive(|_, (((u, b), p), v)| {
            let (b, v) = (b.unwrap_or(0), v.unwrap_or([0; 3]));
            let first = p.map_or(now, |p| p[3]);
            let p = p.unwrap_or([0; 4]);
            let name = db.user.display_name.get(u).unwrap();
            let mut f = vec![V::S(name), V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(v[0]), V::I(v[1]), V::I(v[2])];
            f.push(V::S(if first > add_years(now, -1) { "Active" } else { "Inactive" }));
            f.push(V::Owned(format!("User: {name}; Badges: {b}; Posts: {}; Questions: {}; Answers: {}; Votes: {}", p[0], p[1], p[2], v[0])));
            out.push(row(f))
        });
    rows(out)
}

// WITH RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS AuthorName FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id
//                      WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND P.PostTypeId = 1),
// AggregatedVotes AS (SELECT PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(*) AS TotalVotes
//                     FROM Votes V GROUP BY PostId),
// PostDetails AS (SELECT RP.*, COALESCE(AV.UpVotes, 0) AS UpVotes, COALESCE(AV.DownVotes, 0) AS DownVotes,
//                        CASE WHEN COALESCE(AV.TotalVotes, 0) > 0 THEN (COALESCE(AV.UpVotes, 0) * 1.0 / COALESCE(AV.TotalVotes, 1)) * 100 ELSE NULL END AS UpVotePercentage
//                 FROM RecentPosts RP LEFT JOIN AggregatedVotes AV ON RP.PostId = AV.PostId),
// ClosedPosts AS (SELECT PH.PostId, MAX(PH.CreationDate) AS LastClosedDate, COUNT(*) FILTER (WHERE PH.PostHistoryTypeId = 10) AS CloseCount
//                 FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId),
// FinalResults AS (SELECT PD.*, CP.LastClosedDate, CP.CloseCount FROM PostDetails PD LEFT JOIN ClosedPosts CP ON PD.PostId = CP.PostId)
// SELECT FR.*, CASE WHEN FR.CloseCount IS NULL THEN 'Not Closed' WHEN FR.LastClosedDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '90 days' THEN 'Recently Closed' ELSE 'Older Closed' END AS CloseStatus
// FROM FinalResults FR WHERE FR.UpVotePercentage IS NOT NULL ORDER BY FR.UpVotePercentage DESC, FR.ViewCount DESC LIMIT 100;
fn q20925(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let av = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let cp = db
        .post_history
        .with((&db.post_history.post_history_type_id).is_in([10, 11]))
        .group_by(&db.post_history.post)
        .select((&db.post_history.creation_date).and(&db.post_history.post_history_type_id))
        .fold((i64::MIN, 0i64), |(m, n), (d, t)| (m.max(d), n + (t == 10) as i64));
    let rp = db
        .post
        .with((&db.post.creation_date).ge(add_days(now, -30)))
        .with((&db.post.post_type_id).eq(1))
        .select(Ident::<Post>::new().and((&db.post.origid).select((&av).filt(|a| a[2] > 0))).and((&cp).opt()));
    let pct = |a: [i64; 3]| a[0] as f64 * 1.0 / a[2] as f64 * 100.0;
    let v = top_n(drain(rp), |&(_, ((p, a), _))| {
        let w = db.post.view_count.get(p);
        (Reverse(fkey(pct(a))), w.is_none(), Reverse(w))
    }, 100);
    rows(v.iter().map(|&(_, ((p, a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::F(pct(a))]);
        f.extend(match c {
            Some((m, n)) => [V::T(m), V::I(n), V::S(if m >= add_days(now, -90) { "Recently Closed" } else { "Older Closed" })],
            None => [V::Null, V::Null, V::S("Not Closed")],
        });
        row(f)
    }))
}

// WITH RECURSIVE UserPostCounts AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// UserScore AS (SELECT u.Id AS UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//                      (SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END)) AS Score
//               FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// TopUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, COALESCE(p.PostCount, 0) AS TotalPosts, COALESCE(s.Upvotes, 0) AS TotalUpvotes, COALESCE(s.Downvotes, 0) AS TotalDownvotes,
//                     COALESCE(s.Score, 0) AS TotalScore
//              FROM Users u LEFT JOIN UserPostCounts p ON u.Id = p.UserId LEFT JOIN UserScore s ON u.Id = s.UserId WHERE u.Reputation > 1000 ORDER BY TotalScore DESC LIMIT 10),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(COUNT(c.Id), 0) AS CommentCount, ... UpvoteCount, DownvoteCount, COALESCE(AVG(p.Score), 0) AS AverageScore
//                    FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//                    WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount),
// CombinedStatistics AS (SELECT u.DisplayName, p.Title, p.ViewCount, p.CommentCount, p.UpvoteCount, p.DownvoteCount, u.TotalPosts, p.AverageScore
//                        FROM TopUsers u JOIN PostStatistics p ON u.Id = p.PostId)
// SELECT cs.DisplayName, cs.Title, cs.ViewCount, cs.CommentCount, cs.UpvoteCount, cs.DownvoteCount, cs.TotalPosts,
//        CASE WHEN cs.AverageScore IS NULL THEN 'No Score' ELSE CAST(cs.AverageScore AS VARCHAR) END AS AverageScore
// FROM CombinedStatistics cs ORDER BY cs.UpvoteCount DESC, cs.ViewCount DESC;
//
// TopUsers ties at its LIMIT (every such user's vote score is 0), but the
// last join matches a user Id against a recent post Id, and every user Id is
// below every recent post's, so the answer is empty for any choice.
fn q30355(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |s, t| s + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let upc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = rel(top_n(drain((&us).and(db.user.with((&db.user.reputation).gt(1000))).and(&upc)), |&(_, ((s, _), _))| Reverse(s), 10));
    let ps = db
        .post
        .with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let pids: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let mut out = Vec::new();
    (&tu).and((&tu).map(|(u, _)| u).select((&db.user.origid).select(&pids).select(Ident::<Post>::new().and(&ps)))).drive(|_, ((u, (_, n)), (p, a))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n), V::Owned(format!("{:.1}", db.post.score.get(p).unwrap() as f64))]);
        out.push(row(f))
    });
    rows(out)
}


// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, ... GoldCount, SilverCount, BronzeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//                      SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AvgScore FROM Posts p GROUP BY p.OwnerUserId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(bs.BadgeCount, 0) AS BadgeCount, ..., COALESCE(ps.TotalViews, 0) AS TotalViews,
//                         COALESCE(ps.AvgScore, 0) AS AvgScore FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN UserBadges bs ON u.Id = bs.UserId),
// ClosedPosts AS (SELECT ph.UserId, COUNT(DISTINCT ph.PostId) AS ClosedPostCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.UserId),
// FinalStats AS (SELECT ua.*, COALESCE(cp.ClosedPostCount, 0) AS ClosedPostCount,
//                       CASE WHEN ua.TotalViews > 10000 THEN 'Popular Contributor' WHEN ua.BadgeCount > 5 THEN 'Active Contributor' ELSE 'New Contributor' END AS ContributorStatus
//                FROM UserActivity ua LEFT JOIN ClosedPosts cp ON ua.UserId = cp.UserId)
// SELECT DisplayName, PostCount, BadgeCount, GoldCount, SilverCount, BronzeCount, TotalViews, AvgScore, ClosedPostCount, ContributorStatus FROM FinalStats
// WHERE (PostCount > 5 OR ClosedPostCount > 0) AND (AvgScore IS NULL OR AvgScore >= 1) ORDER BY TotalViews DESC, BadgeCount DESC LIMIT 10;
fn q21552(db: &'static So) -> String {
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let ups = user_posts(db);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let avg_s = |a: [i64; 10]| if a[1] == 0 { 0.0 } else { a[4] as f64 / a[1] as f64 };
    let v = drain((&ups).and((&ub).opt()).and((&cp).opt()).filt(move |((a, _), c)| (a[1] > 5 || c.unwrap_or(0) > 0) && avg_s(a) >= 1.0));
    let v = top_n(v, |&(_, ((a, b), _))| (Reverse(a[6]), Reverse(b.map_or(0, |b| b[0]))), 10);
    rows(v.iter().map(|&(u, ((a, b), c))| {
        let b = b.unwrap_or([0; 4]);
        let status = if a[6] > 10000 { "Popular Contributor" } else if b[0] > 5 { "Active Contributor" } else { "New Contributor" };
        row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3]), V::I(a[6]), V::F(avg_s(a)), V::I(c.unwrap_or(0)), V::S(status)])
    }))
}

// WITH UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, ... SilverBadges, BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//                           SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers FROM Posts p GROUP BY p.OwnerUserId),
// ClosedPostHistory AS (SELECT ph.UserId, COUNT(*) AS CloseVotes, MIN(ph.CreationDate) AS FirstCloseDate, MAX(ph.CreationDate) AS LastCloseDate
//                       FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId),
// PostStatsWithBadges AS (
//     SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.GoldBadges, 0) AS GoldBadges, ..., COALESCE(cph.CloseVotes, 0) AS CloseVotes,
//            CASE WHEN d.LastCloseDate IS NOT NULL AND COALESCE(cph.CloseVotes, 0) > 0 THEN 'Active Closer' ELSE 'Novice' END AS CloserStatus
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId LEFT JOIN ClosedPostHistory cph ON u.Id = cph.UserId
//     LEFT JOIN (SELECT ph.UserId, MAX(ph.CreationDate) AS LastCloseDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId) d ON u.Id = d.UserId)
// SELECT ps.UserId, ps.DisplayName, ps.GoldBadges + ps.SilverBadges + ps.BronzeBadges AS TotalBadges, ps.TotalPosts, ps.TotalViews, ps.AcceptedAnswers, ps.CloseVotes, ps.CloserStatus,
//        CASE WHEN ps.TotalPosts > 0 AND ps.CloseVotes > ps.TotalPosts * 0.5 THEN 'High Risk of Closure' WHEN ps.TotalViews > 1000 AND ps.AcceptedAnswers = 0 THEN 'Needs Attention'
//             ELSE 'Stable User' END AS UserRiskLevel
// FROM PostStatsWithBadges ps WHERE ps.GoldBadges > 0 OR ps.SilverBadges > 0 ORDER BY TotalBadges DESC, TotalViews DESC;
fn q20789(db: &'static So) -> String {
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.view_count).opt().and((&db.post.accepted_answer_id).opt())).fold([0i64; 3], |a, (v, x)| [a[0] + 1, a[1] + v.unwrap_or(0), a[2] + x.is_some() as i64]);
    let closes = || db.post_history.with((&db.post_history.post_history_type_id).eq(10));
    let cph = closes().group_by(&db.post_history.user).fold(0i64, |n, _| n + 1);
    let d = closes().group_by(&db.post_history.user).select(&db.post_history.creation_date).fold(i64::MIN, |m, x| m.max(x));
    let mut out = Vec::new();
    db.user
        .select(Ident::<User>::new().and((&ub).filt(|b| b[0] > 0 || b[1] > 0)).and((&ps).opt()).and((&cph).opt()).and((&d).opt()))
        .drive(|_, ((((u, b), p), c), last)| {
            let p = p.unwrap_or([0; 3]);
            let c = c.unwrap_or(0);
            let status = if last.is_some() && c > 0 { "Active Closer" } else { "Novice" };
            let risk = if p[0] > 0 && c * 2 > p[0] { "High Risk of Closure" } else if p[1] > 1000 && p[2] == 0 { "Needs Attention" } else { "Stable User" };
            let mut f = ucols(db, u, &["uid", "name"]);
            f.extend([V::I(b[0] + b[1] + b[2]), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(c), V::S(status), V::S(risk)]);
            out.push(row(f))
        });
    rows(out)
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.ViewCount) AS TotalViews,
//            AVG(EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - p.CreationDate)) ) AS AvgPostAgeInSeconds
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// VoteStatistics AS (SELECT v.UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS TotalDownVotes
//                    FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId),
// BadgesSummary AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, ... FROM Badges b GROUP BY b.UserId),
// PostEngagement AS (SELECT p.OwnerUserId, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT pl.RelatedPostId) AS RelatedLinksCount
//                    FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostLinks pl ON p.Id = pl.PostId
//                    WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') GROUP BY p.OwnerUserId)
// SELECT ua.UserId, ua.DisplayName, COALESCE(ua.TotalPosts, 0) AS TotalPosts, ..., COALESCE(pe.RelatedLinksCount, 0) AS RelatedLinksCount,
//        CASE WHEN COALESCE(ua.TotalPosts, 0) = 0 THEN 'No posts yet!' WHEN COALESCE(va.TotalVotes, 0) > COALESCE(ua.TotalPosts, 0) THEN 'Votes exceed posts' ELSE 'Engaged User' END AS EngagementStatus,
//        CASE WHEN (COALESCE(ua.AvgPostAgeInSeconds, 0) >= 31536000 AND COALESCE(ua.TotalPosts, 0) < 10) THEN 'Inactive'
//             WHEN (COALESCE(ua.TotalViews, 0) = 0 AND COALESCE(ua.TotalPosts, 0) > 5) THEN 'Silent Contributor' ELSE 'Active User' END AS ActivityStatus
// FROM UserActivity ua LEFT JOIN VoteStatistics va ON ua.UserId = va.UserId LEFT JOIN BadgesSummary bs ON ua.UserId = bs.UserId LEFT JOIN PostEngagement pe ON ua.UserId = pe.OwnerUserId
// ORDER BY TotalVotes DESC, TotalPosts DESC, ActivityStatus;
fn q21586(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let ups = user_posts(db);
    let age = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.creation_date)).fold((0.0f64, 0i64), |(s, n), d| (s + secs(now - d), n + 1));
    let va = db.vote.group_by(&db.vote.user).select(vtype_name(db)).fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let recent = || owned(db).with((&db.post.creation_date).ge(add_years(now, -1)));
    let pc = recent().group_by(&db.post.owner_user).select(comments_of(db).opt()).buf_fold(distinct_some);
    let pl = recent().group_by(&db.post.owner_user).select(links_of(db).select(&db.post_link.related_post_id).opt()).buf_fold(distinct_some);
    let mut out = Vec::new();
    (&ups).and((&age).opt()).and((&va).opt()).and((&bs).opt()).and((&pc).opt()).and((&pl).opt()).drive(|u, (((((a, g), v), b), c), l)| {
        let v = v.unwrap_or([0; 3]);
        let b = b.unwrap_or([0; 3]);
        let avg_age = g.map_or(0.0, |(s, n)| s / n as f64);
        let eng = if a[1] == 0 { "No posts yet!" } else if v[0] > a[1] { "Votes exceed posts" } else { "Engaged User" };
        let act = if avg_age >= 31536000.0 && a[1] < 10 { "Inactive" } else if a[6] == 0 && a[1] > 5 { "Silent Contributor" } else { "Active User" };
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(v[0]), V::I(v[1]), V::I(v[2]), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(c.unwrap_or(0)), V::I(l.unwrap_or(0)), V::S(eng), V::S(act)]);
        out.push(row(f))
    });
    rows(out)
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        ARRAY_AGG(DISTINCT t.TagName) AS Tags, MAX(ph.CreationDate) AS LastEditDate
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN LATERAL unnest(string_to_array(p.Tags, ',')) AS tag(tagname) ON TRUE LEFT JOIN Tags t ON t.TagName = tag.tagname
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName ORDER BY p.CreationDate DESC LIMIT 100;
//
// Tags has no commas, so each post's split is its whole Tags string (or
// nothing, for NULL), which names no tag: every array comes out [NULL]. The
// ARRAY_AGG has no ORDER BY; with one element the order never shows.
fn q12273(db: &'static So) -> String {
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let q = owned(db).with((&db.post.post_type_id).is_in([1, 2]));
    let agg = q
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, 0i64, i64::MIN), |(c, v, m), ((x, y), d)| (c + x.is_some() as i64, v + y.is_some() as i64, d.map_or(m, |d| m.max(d))));
    let tags = owned(db)
        .with((&db.post.post_type_id).is_in([1, 2]))
        .group_by(Ident::<Post>::new())
        .select((&db.post.tags_str).flat_map(|t: Str| t.split(',').collect::<Vec<_>>()).select(&names).opt())
        .buf_fold(|v| {
            let mut t: Vec<Option<Id<Tag>>> = v.into_iter().collect();
            t.sort_unstable();
            t.dedup();
            &*Box::leak(t.into_boxed_slice())
        });
    let v = top_n(drain((&agg).and((&tags).opt())), |&(p, _)| Reverse(db.post.creation_date.get(p).unwrap()), 100);
    rows(v.iter().map(|&(p, ((c, n, m), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(n)]);
        let t: &[Option<Id<Tag>>] = t.unwrap_or(&[None]);
        f.push(V::L(t.iter().map(|x| x.map_or(V::Null, |g| V::S(db.tag.tag_name.get(g).unwrap()))).collect()));
        f.push(tmax(m));
        row(f)
    }))
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COALESCE(t.TagName, 'No Tags') AS TagName
// FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN (SELECT t.Id, t.TagName, pt.PostId FROM Tags t
//            JOIN (SELECT p.Id AS PostId, unnest(string_to_array(p.Tags, '>')) AS Tag FROM Posts p) AS pt ON t.TagName = pt.Tag) t ON p.Id = t.PostId
// WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, t.TagName ORDER BY p.CreationDate DESC LIMIT 100;
//
// Splitting on '>' leaves every piece with its leading '<', so no piece is a
// tag name and every post is 'No Tags'.
fn q14996(db: &'static So) -> String {
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let tg = (&db.post.tags_str).flat_map(|t: Str| t.split('>').collect::<Vec<_>>()).select(&names);
    let q = || db.post.with((&db.post.post_type_id).eq(1));
    type K = (Id<Post>, Option<Id<Tag>>);
    let groups = q().select(Ident::<Post>::new().and(tg.opt())).group_by(Same::<K>::new());
    let dc = groups.select(Same::<K>::new().map(|(p, _)| p).select(comments_of(db).opt())).buf_fold(distinct_some);
    let v = top_n(drain(whole(&dc).select(Same::new().and(&dc))), |&(_, ((p, _), _))| Reverse(db.post.creation_date.get(p).unwrap()), 100);
    rows(v.iter().map(|&(_, ((p, t), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::S(t.map_or("No Tags", |t| db.tag.tag_name.get(t).unwrap()))]);
        row(f)
    }))
}

// WITH PostStats AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, COUNT(v.Id) AS VoteCount, ARRAY_AGG(DISTINCT t.TagName) AS Tags,
//            u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Tags t ON t.ExcerptPostId = p.Id
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName, u.Reputation),
// PostTypeCounts AS (SELECT PostTypeId, COUNT(*) AS PostCount FROM Posts GROUP BY PostTypeId)
// SELECT ps.*, pt.PostCount AS TotalPostsOfType FROM PostStats ps JOIN PostTypeCounts pt ON pt.PostTypeId = (SELECT PostTypeId FROM Posts WHERE Id = ps.PostId LIMIT 1)
// ORDER BY ps.CreationDate DESC;
//
// A post is the excerpt of at most one tag, so each array has one element
// and its missing ORDER BY never shows.
fn q11327(db: &'static So) -> String {
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).opt().and((&excerpt).opt()))
        .fold(0i64, |n, (v, _)| n + v.is_some() as i64);
    let tags = db.post.group_by(Ident::<Post>::new()).select((&excerpt).opt()).buf_fold(|v| {
        let mut t: Vec<Option<Id<Tag>>> = v.into_iter().collect();
        t.sort_unstable();
        t.dedup();
        &*Box::leak(t.into_boxed_slice())
    });
    let pt = db.post.group_by(&db.post.post_type_id).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&ps).and(&tags).and((&db.post.post_type_id).select(&pt)).drive(|p, ((n, t), c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.push(V::I(n));
        f.push(V::L(t.iter().map(|x: &Option<Id<Tag>>| x.map_or(V::Null, |g| V::S(db.tag.tag_name.get(g).unwrap()))).collect()));
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.push(V::I(c));
        out.push(row(f))
    });
    rows(out)
}

// WITH PostTagCounts AS (SELECT p.Id AS PostId, p.Title, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagStatistics AS (SELECT Tag, COUNT(*) AS TagCount, COUNT(DISTINCT pt.PostId) AS PostCount FROM PostTagCounts pt GROUP BY Tag ORDER BY TagCount DESC LIMIT 10),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount, MAX(v.CreationDate) AS LastVoteDate FROM Votes v
//                 WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId),
// PopularPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.AnswerCount FROM Posts p JOIN RecentVotes rv ON p.Id = rv.PostId WHERE p.PostTypeId = 1
//                  ORDER BY rv.VoteCount DESC, p.ViewCount DESC LIMIT 5)
// SELECT pp.Title AS PopularPostTitle, pp.ViewCount, pp.AnswerCount, ts.Tag AS TopTag, ts.TagCount FROM PopularPosts pp JOIN TagStatistics ts ON ts.PostCount >= 1
// ORDER BY pp.ViewCount DESC, ts.TagCount DESC;
fn q27660(db: &'static So) -> String {
    let q = || db.post.with((&db.post.post_type_id).eq(1));
    let tc = q().select((&db.post.tags_str).flat_map(tag_list)).group_by(Same::new()).fold(0i64, |n, _| n + 1);
    let pc = q().select(Ident::<Post>::new().and((&db.post.tags_str).flat_map(tag_list))).group_by(Same::<(Id<Post>, Str)>::new().map(|(_, t)| t)).map(|(p, _): (Id<Post>, Str)| p).count_distinct();
    let ts_ = rel(top_n(drain((&tc).and(&pc)), |&(_, (n, _))| Reverse(n), 10));
    let rv = db.vote.with((&db.vote.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(&db.vote.post).fold(0i64, |n, _| n + 1);
    let pp = rel(top_n(drain(q().select(&rv)), |&(p, n)| {
        let w = db.post.view_count.get(p);
        (Reverse(n), w.is_none(), Reverse(w))
    }, 5));
    let mut out = Vec::new();
    (&pp).cross((&ts_).filt(|(_, (_, d))| d >= 1)).drive(|_, ((p, _), (t, (n, _)))| {
        let mut f = post_fields(db, p, &["title", "views", "answers"]);
        f.extend([V::S(t), V::I(n)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//                           SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount
//                    FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.Reputation),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, COALESCE(A.AcceptedAnswerId, 0) AS AcceptedAnswerId, COUNT(C.Id) AS CommentCount, MAX(P.CreationDate) AS LastActivityDate
//               FROM Posts P LEFT JOIN Posts A ON P.AcceptedAnswerId = A.Id LEFT JOIN Comments C ON P.Id = C.PostId
//               WHERE P.LastActivityDate >= CURRENT_TIMESTAMP - INTERVAL '1 YEAR' GROUP BY P.Id, P.Title, P.ViewCount, P.Score, A.AcceptedAnswerId)
// SELECT U.UserId, U.Reputation, U.PostCount, U.QuestionCount, U.AnswerCount, U.CommentCount, P.PostId, P.Title, P.ViewCount, P.Score, P.AcceptedAnswerId,
//        P.CommentCount AS PostCommentCount, P.LastActivityDate
// FROM UserStats U JOIN PostStats P ON U.UserId = P.AcceptedAnswerId ORDER BY U.Reputation DESC, P.ViewCount DESC FETCH FIRST 100 ROWS ONLY;
//
// CURRENT_TIMESTAMP is when the query runs; the data ends in 2024, so
// PostStats is empty.
fn q14828(db: &'static So) -> String {
    let now = now_utc();
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt())))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + c.is_some() as i64]);
    let dp = user_distinct_posts(db);
    let recent = || db.post.with((&db.post.last_activity_date).filt(move |d| ny_to_utc(d) >= ny_to_utc(add_years(utc_to_ny(now), -1))));
    let pc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let acc = || (&db.post.accepted_answer).select((&db.post.accepted_answer_id).opt()).opt().map(|x: Option<Option<i64>>| x.flatten().unwrap_or(0));
    let by_acc: HashIdx<i64, Id<Post>> = recent().select(acc()).inv().collect();
    let v = drain((&dp).and((&us).opt()).and((&db.user.origid).select((&by_acc).select(Ident::<Post>::new().and(&pc).and(acc())))));
    let v = top_n(v, |&(u, (_, ((p, _), _)))| {
        let w = db.post.view_count.get(p);
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w))
    }, 100);
    rows(v.iter().map(|&(u, ((n, a), ((p, c), acc)))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::I(a[1]), V::I(a[0]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
        f.extend([V::I(acc), V::I(c), V::T(db.post.creation_date.get(p).unwrap())]);
        row(f)
    }))
}

// WITH PostInfo AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//            COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//            COALESCE(SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserInfo AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT pi.PostId, pi.Title, pi.CreationDate, pi.Score, pi.ViewCount, pi.CommentCount, pi.VoteCount, pi.UpVotes, pi.DownVotes, ui.UserId, ui.DisplayName AS OwnerDisplayName,
//        ui.Reputation AS OwnerReputation, ui.BadgeCount AS OwnerBadgeCount
// FROM PostInfo pi JOIN Users u ON pi.OwnerUserId = u.Id JOIN UserInfo ui ON u.Id = ui.UserId ORDER BY pi.Score DESC, pi.ViewCount DESC LIMIT 100;
//
// Posts with no views tie on the ORDER BY; the rewrite adds pi.PostId.
fn q11708(db: &'static So) -> String {
    let pi = owned(db)
        .with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user).select(badges_of(db).opt())))
        .fold([0i64; 4], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + t.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&pi).and((&db.post.owner_user).select(&ub))), |&(p, _)| {
        let w = db.post.view_count.get(p);
        (Reverse(db.post.score.get(p).unwrap()), w.is_none(), Reverse(w), db.post.origid.get(p).unwrap())
    }, 100);
    rows(v.iter().map(|&(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["uid", "owner", "rep"]));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//                         WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PopularTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
//                 GROUP BY t.TagName HAVING COUNT(p.Id) > 10),
// TopPostHistories AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(ph.Id) AS ChangeCount, STRING_AGG(ph.UserDisplayName, ', ') AS Editors FROM PostHistory ph
//                      GROUP BY ph.PostId, ph.PostHistoryTypeId ORDER BY ChangeCount DESC LIMIT 10),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS Badges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT ur.DisplayName, ur.Reputation, ur.PostCount, pb.TagName, pb.PostCount AS TagPostCount, pb.TotalViews, tph.PostId, tph.ChangeCount, tph.Editors, ub.BadgeCount, ub.Badges
// FROM UserReputation ur JOIN PopularTags pb ON ur.PostCount > pb.PostCount JOIN TopPostHistories tph ON ur.UserId = tph.PostId JOIN UserBadges ub ON ur.UserId = ub.UserId
// ORDER BY ur.Reputation DESC, pb.TotalViews DESC;
//
// TopPostHistories cuts inside a tie at ChangeCount 46, and its STRING_AGG
// has no order, but the answer does not depend on either: the join matches
// a user Id against a post Id, and no post with 46 or more changes of one
// type has the Id of a user with reputation over 1000.
fn q28292(db: &'static So) -> String {
    let ur = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let ts_ = tag_stats(db);
    let pbk: MatSet<(Id<Tag>, [i64; 6])> = whole(&ts_).select(Same::new().and(&ts_)).filt(|(_, a)| a[0] > 10).collect();
    let by_count: HashIdx<i64, (Id<Tag>, [i64; 6])> = (&pbk).map(|(_, a)| a[0]).inv().collect();
    type K = (Id<Post>, i64);
    let tph = db.post_history.group_by((&db.post_history.post).and(&db.post_history.post_history_type_id)).fold(0i64, |n, _| n + 1);
    let tph: MatSet<K> = rel(top_n(drain(&tph), |&(_, n)| Reverse(n), 10)).map(|(k, _)| k).collect();
    let editors = db.post_history.group_by((&db.post_history.post).and(&db.post_history.post_history_type_id)).select(&db.post_history.user_display_name).buf_fold(|v| {
        let mut v: Vec<Str> = v.into_iter().collect();
        v.sort_unstable();
        Box::leak(v.join(", ").into_boxed_str()) as Str
    });
    let counts = db.post_history.group_by((&db.post_history.post).and(&db.post_history.post_history_type_id)).fold(0i64, |n, _| n + 1);
    let tph_by: HashIdx<i64, K> = (&tph).map(|(p, _)| p).select(&db.post.origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let bn = db.badge.group_by(&db.badge.user).select(&db.badge.name).buf_fold(|v| {
        let mut v: Vec<Str> = v.into_iter().collect();
        v.sort_unstable();
        Box::leak(v.join(", ").into_boxed_str()) as Str
    });
    let mut out = Vec::new();
    (&ur)
        .and((&db.user.origid).select((&tph_by).select(Same::<K>::new().and(&counts).and((&editors).opt()))))
        .and(&ub)
        .and((&bn).opt())
        .and((&ur).select_where(&by_count, |n: i64, c: i64| n > c))
        .drive(|u, ((((n, ((_, k), e)), b), names), (t, a))| {
            let mut f = ucols(db, u, &["name", "rep"]);
            f.extend([V::I(n), V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), nullable(a[2], a[1]), user_col(db, u, "uid"), V::I(k), ostr(e), V::I(b), ostr(names)]);
            out.push(row(f))
        });
    rows(out)
}

pub static ENTRIES: &[harness::Entry] = &[
    ("12820", q12820),
    ("13277", q13277),
    ("13698", q13698),
    ("13872", q13872),
    ("13041", q13041),
    ("13130", q13130),
    ("14408", q14408),
    ("11151", q11151),
    ("10607", q10607),
    ("12942", q12942),
    ("13305", q13305),
    ("11167", q11167),
    ("8459", q8459),
    ("12495", q12495),
    ("9251", q9251),
    ("26254", q26254),
    ("7346", q7346),
    ("8487", q8487),
    ("10859", q10859),
    ("9834", q9834),
    ("10964", q10964),
    ("26232", q26232),
    ("12662", q12662),
    ("3658", q3658),
    ("26548", q26548),
    ("12020", q12020),
    ("27106", q27106),
    ("25034", q25034),
    ("8746", q8746),
    ("6607", q6607),
    ("5008", q5008),
    ("9419", q9419),
    ("4040", q4040),
    ("11583", q11583),
    ("26355", q26355),
    ("6721", q6721),
    ("3109", q3109),
    ("28306", q28306),
    ("32127", q32127),
    ("14509", q14509),
    ("4710", q4710),
    ("27717", q27717),
    ("4738", q4738),
    ("27183", q27183),
    ("28839", q28839),
    ("11598", q11598),
    ("25573", q25573),
    ("13960", q13960),
    ("10806", q10806),
    ("27757", q27757),
    ("13927", q13927),
    ("29656", q29656),
    ("12864", q12864),
    ("5140", q5140),
    ("29115", q29115),
    ("5448", q5448),
    ("5016", q5016),
    ("4568", q4568),
    ("3076", q3076),
    ("9775", q9775),
    ("6890", q6890),
    ("6599", q6599),
    ("27806", q27806),
    ("3760", q3760),
    ("9812", q9812),
    ("24493", q24493),
    ("3339", q3339),
    ("6707", q6707),
    ("8762", q8762),
    ("20028", q20028),
    ("20141", q20141),
    ("9976", q9976),
    ("29558", q29558),
    ("6966", q6966),
    ("31859", q31859),
    ("3245", q3245),
    ("1987", q1987),
    ("9388", q9388),
    ("1415", q1415),
    ("22203", q22203),
    ("24305", q24305),
    ("29294", q29294),
    ("23758", q23758),
    ("22414", q22414),
    ("20967", q20967),
    ("21342", q21342),
    ("20925", q20925),
    ("30355", q30355),
    ("21552", q21552),
    ("20789", q20789),
    ("21586", q21586),
    ("12273", q12273),
    ("14996", q14996),
    ("11327", q11327),
    ("27660", q27660),
    ("14828", q14828),
    ("11708", q11708),
    ("28292", q28292),
];
