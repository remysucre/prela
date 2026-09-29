use harness::prelude::*;
use std::cmp::Reverse;

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViewCount,
//        SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViewCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS RankByScore FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViewCount, TotalScore, RankByScore FROM TopUsers WHERE RankByScore <= 10 ORDER BY RankByScore;
fn q10216(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| Reverse(a[4]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId)
// SELECT u.DisplayName, rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.CommentCount, COALESCE(b.Name, 'No Badge') AS BadgeName
// FROM RankedPosts rp JOIN Users u ON rp.PostId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId AND b.Class = 1
// WHERE rp.PostRank = 1 AND EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 2)
// ORDER BY rp.ViewCount DESC FETCH FIRST 10 ROWS ONLY;
//
// rp.PostId = u.Id compares a post id with a user id, so it goes through origid.
fn q1288(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let v = drain((&tp).with(up).select((&cc).and(origid.select(&uidx).select(Ident::<User>::new().and(gold.opt())))));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, 10);
    rows(v.into_iter().map(|(p, (n, (u, b)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.push(V::I(n));
        f.push(V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, UpVotes, DownVotes, Rank FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q12071(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let d = user_distinct_posts(db);
    let v = ranked(drain((&s).and(&d)), |&(_, (_, n))| Reverse(n), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id
//     GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalUpVotes, TotalDownVotes, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC, TotalUpVotes DESC) AS Ranking FROM UserActivity)
// SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalUpVotes, TotalDownVotes, Ranking FROM TopUsers WHERE Ranking <= 10;
//
// v.UserId = u.Id with p.OwnerUserId = u.Id is a vote cast by the post's owner: `own_votes`.
fn q11222(db: &'static So) -> String {
    let own = own_votes(db);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and((&own).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, x| match x {
            Some((_, v)) => [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64],
            None => a,
        });
    let dc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = drain((&s).and(user_distinct_posts(db)).and(&dc));
    let v = top_n(v, |&(u, ((a, n), _))| (Reverse(n), Reverse(a[0]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, ((a, n), c)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        SUM(COALESCE(c.CommentCount, 0)) AS TotalComments, AVG(COALESCE(p.Score, 0)) AS AvgScorePerPost
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     GROUP BY u.Id, u.DisplayName)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, TotalComments, AvgScorePerPost,
//        RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank
// FROM UserPostStats ORDER BY TotalPosts DESC LIMIT 100;
fn q12013(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and((&cc).opt())).opt())
        .fold([0i64; 7], |a, x| match x {
            Some((((t, s), w), c)) => [a[0] + 1, a[1] + 1, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + s, a[5] + w.unwrap_or(0), a[6] + c.unwrap_or(0)],
            None => [a[0] + 1, a[1], a[2], a[3], a[4], a[5], a[6]],
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[4]), false);
    let v = top_n(v, |&((_, a), _)| Reverse(a[1]), 100);
    rows(v.into_iter().map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[5]), V::I(a[6]), avg(a[4], a[0]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments, SUM(Vote.Value) AS TotalVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId
//     LEFT JOIN (SELECT V.PostId, CASE WHEN V.VoteTypeId = 2 THEN 1 WHEN V.VoteTypeId = 3 THEN -1 ELSE 0 END AS Value FROM Votes V) AS Vote ON P.Id = Vote.PostId
//     GROUP BY U.Id, U.DisplayName, U.Reputation)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.TotalPosts, U.TotalComments, U.TotalVotes, RANK() OVER (ORDER BY U.TotalVotes DESC) AS VoteRank,
//        RANK() OVER (ORDER BY U.TotalPosts DESC) AS PostRank
// FROM UserPostStats U ORDER BY U.TotalPosts DESC, U.TotalVotes DESC;
fn q13444(db: &'static So) -> String {
    let val = (&db.vote.vote_type_id).map(|t: i64| if t == 2 { 1 } else if t == 3 { -1 } else { 0 });
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(val).opt())).opt())
        .fold([0i64; 2], |a, x| match x.and_then(|(_, v)| v) {
            Some(v) => [a[0] + 1, a[1] + v],
            None => a,
        });
    let dc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = drain((&s).and(user_distinct_posts(db)).and(&dc));
    let v = ranked(v, |&(_, ((a, _), _))| (a[0] == 0, Reverse(a[1])), false);
    let v = ranked(v, |&((_, ((_, n), _)), _)| Reverse(n), false);
    rows(v.into_iter().map(|(((u, ((a, n), c)), vr), pr)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(c), nullable(a[1], a[0]), V::I(vr), V::I(pr)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Owner, p.CreationDate, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Owner, rp.CreationDate, rp.Score FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostComments AS (SELECT pc.PostId, COUNT(pc.Id) AS CommentCount FROM Comments pc GROUP BY pc.PostId)
// SELECT tp.PostId, tp.Title, tp.Owner, tp.CreationDate, tp.Score, COALESCE(pcm.CommentCount, 0) AS TotalComments
// FROM TopPosts tp LEFT JOIN PostComments pcm ON tp.PostId = pcm.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q8133(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain(&cc).into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.push(V::I(n));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(COALESCE(C.Score, 0)) AS TotalCommentScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.Reputation),
// MostActiveUsers AS (SELECT UserId, PostCount, TotalScore, TotalCommentScore, DENSE_RANK() OVER (ORDER BY PostCount DESC) AS RankByPostCount,
//        DENSE_RANK() OVER (ORDER BY TotalScore DESC) AS RankByTotalScore FROM UserStats)
// SELECT U.Id AS UserId, U.DisplayName, A.PostCount, A.TotalScore, A.TotalCommentScore, A.RankByPostCount, A.RankByTotalScore
// FROM Users U JOIN MostActiveUsers A ON U.Id = A.UserId WHERE A.RankByPostCount <= 10 OR A.RankByTotalScore <= 10 ORDER BY A.RankByPostCount, A.RankByTotalScore;
fn q10155(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(comments_of(db).select(&db.comment.score).opt())).opt())
        .fold([0i64; 2], |a, x| match x {
            Some((s, c)) => [a[0] + s, a[1] + c.unwrap_or(0)],
            None => a,
        });
    let v = drain((&s).and(user_distinct_posts(db)));
    let v = ranked(v, |&(_, (_, n))| Reverse(n), true);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[0]), true);
    rows(v.into_iter().filter(|&(((_, _), pr), sr)| pr <= 10 || sr <= 10).map(|(((u, (a, n)), pr), sr)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(pr), V::I(sr)]);
        row(f)
    }))
}

// WITH UserRep AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, CommentCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserRep)
// SELECT U.DisplayName, U.Reputation, T.PostCount, T.CommentCount, T.UpVotes, T.DownVotes, T.ReputationRank
// FROM TopUsers T JOIN Users U ON T.UserId = U.Id WHERE T.ReputationRank <= 10 ORDER BY T.ReputationRank;
//
// ReputationRank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q10238(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect();
    let tr = rel(v);
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let top: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, x| {
            let t = x.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let nc = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = drain((&tu).and((&s).and(&np).and(&nc)));
    rows(v.into_iter().map(|(u, ((_, r), ((a, p), c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p), V::I(c), V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS WikiCount,
//        SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews, AVG(p.Score) AS AverageScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// TopUsers AS (SELECT UserId, PostCount, QuestionCount, AnswerCount, WikiCount, TotalViews, AverageScore, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews FROM UserPostStats)
// SELECT u.DisplayName, t.PostCount, t.QuestionCount, t.AnswerCount, t.WikiCount, t.TotalViews, t.AverageScore
// FROM TopUsers t JOIN Users u ON t.UserId = u.Id WHERE t.RankByViews <= 10 ORDER BY t.TotalViews DESC;
fn q10619(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 6], |a, x| match x {
            Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 3 | 4 | 5) as i64, a[4] + w.unwrap_or(0), a[5] + s],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[4]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[5], a[0])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
//        SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalViews, TotalScore, CommentCount, BadgeCount, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, TotalViews, TotalScore, CommentCount, BadgeCount, Rank FROM TopUsers WHERE Rank <= 10;
fn q10911(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (n, w, s, c) = p.map_or((0, 0, 0, 0), |((s, w), c)| (1, w.unwrap_or(0), s, c.is_some() as i64));
            [a[0] + n, a[1] + w, a[2] + s, a[3] + c, a[4] + b.is_some() as i64]
        });
    let v = top_n(drain(&s), |&(u, a)| (Reverse(a[2]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// TopPosts AS (SELECT Id, Title, CreationDate, Score, OwnerName, CommentCount, VoteCount, RANK() OVER (ORDER BY Score DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY CommentCount DESC) AS CommentRank FROM RecentPosts),
// CombinedRanks AS (SELECT Id, Title, CreationDate, OwnerName, Score, CommentCount, VoteCount, (ScoreRank + CommentRank) AS CombinedRank FROM TopPosts)
// SELECT Id, Title, OwnerName, Score, CommentCount, VoteCount, CombinedRank FROM CombinedRanks WHERE CombinedRank <= 10 ORDER BY CombinedRank;
fn q9208(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let base = || db.post.with(creation_date.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let cc = base().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vu = base().group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.user_id).opt()).opt()).buf_fold(|xs| distinct_some(xs.into_iter().map(|x| x.flatten())));
    let v = ranked(drain((&cc).and(&vu)), |&(p, _)| Reverse(score.get(p).unwrap()), false);
    let v = ranked(v, |&((_, (c, _)), _)| Reverse(c), false);
    rows(v.into_iter().filter(|&((_, s), c)| s + c <= 10).map(|(((p, (c, u)), s), r)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score"]);
        f.extend([V::I(c), V::I(u), V::I(s + r)]);
        row(f)
    }))
}

// WITH UserPosts AS (SELECT u.Id AS UserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.ViewCount) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// TopUsers AS (SELECT UserId, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS RankScore,
//        RANK() OVER (ORDER BY TotalViews DESC) AS RankViews FROM UserPosts)
// SELECT u.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalViews, tu.TotalScore, tu.RankScore, tu.RankViews
// FROM TopUsers tu JOIN Users u ON tu.UserId = u.Id WHERE tu.TotalPosts > 0 ORDER BY tu.RankScore, tu.RankViews;
fn q13195(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| Reverse(a[4]), false);
    let v = ranked(v, |&((_, a), _)| (a[5] == 0, Reverse(a[6])), false);
    rows(drain(rel(v).filt(|(((_, a), _), _)| a[1] > 0)).into_iter().map(|x| x.1).map(|(((u, a), s), w)| {
        row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), V::I(a[4]), V::I(s), V::I(w)])
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, ScoreRank FROM TopUsers WHERE ScoreRank <= 10 ORDER BY ScoreRank;
fn q12574(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| Reverse(a[4]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, UpVoteCount, DownVoteCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation)
// SELECT TU.UserId, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.UpVoteCount, TU.DownVoteCount, TU.ReputationRank FROM TopUsers TU WHERE TU.ReputationRank <= 10;
//
// ReputationRank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q10586(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let top: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&tu).and((&s).and(&np))).into_iter().map(|(u, ((_, r), (a, n)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, SUM(P.Score) AS TotalScores
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id
//     WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalUpVotes, TotalDownVotes, TotalScores, RANK() OVER (ORDER BY TotalScores DESC) AS ScoreRank FROM UserEngagement)
// SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalUpVotes, TotalDownVotes, TotalScores, ScoreRank FROM TopUsers WHERE ScoreRank <= 10 ORDER BY ScoreRank;
//
// v.UserId = u.Id with p.OwnerUserId = u.Id is a vote cast by the post's owner: `own_votes`.
fn q13828(db: &'static So) -> String {
    let own = own_votes(db);
    let us = || db.user.with((&db.user.reputation).gt(0));
    let s = us()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(comments_of(db).opt().and((&own).select(&db.vote.vote_type_id).opt()))).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((s, (_, v))) => [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + 1, a[3] + s],
            None => a,
        });
    let np = us().group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let nc = us().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(&np).and(&nc)), |&(_, ((a, _), _))| (a[2] == 0, Reverse(a[3])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, p), c)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p), V::I(c), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AvgScore,
//        AVG(p.ViewCount) AS AvgViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, AvgScore, AvgViews,
//        ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, AvgScore, AvgViews FROM TopUsers WHERE Rank <= 10;
fn q10923(db: &'static So) -> String {
    let v = top_n(drain(&user_posts(db)), |&(u, a)| (Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), nullable(a[6], a[5]), avg(a[4], a[1]), avg(a[6], a[5])]);
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, COUNT(c.Id) AS TotalComments, COUNT(DISTINCT b.Id) AS TotalBadges, SUM(v.BountyAmount) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName),
// FilteredEngagement AS (SELECT *, RANK() OVER (ORDER BY TotalPosts DESC) AS EngagementRank FROM UserEngagement)
// SELECT fe.DisplayName, fe.TotalPosts, fe.Questions, fe.Answers, fe.TotalComments, fe.TotalBadges, fe.TotalBounty FROM FilteredEngagement fe
// WHERE fe.EngagementRank <= 10 ORDER BY fe.EngagementRank;
fn q6553(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()))).opt().and(badges_of(db).opt()))
        .fold([0i64; 6], |a, (p, _)| match p {
            Some((t, (c, v))) => {
                let b = v.flatten();
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0)]
            }
            None => a,
        });
    let nb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(&nb)), |&(_, (a, _))| Reverse(a[0]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, b)), _)| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(b), nullable(a[5], a[4])])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotesCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotesCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation)
// SELECT up.UserId, up.Reputation, up.UpVotesCount, up.DownVotesCount, rp.PostId, rp.Title, rp.CreationDate, rp.Score
// FROM UserReputation up LEFT JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId AND rp.PostRank = 1 WHERE up.Reputation > 1000
// ORDER BY up.Reputation DESC, rp.Score DESC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
fn q691(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let pp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let first: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&pp).map(|(u, _)| u).inv().select(&pp).collect();
    let us = db.user.with((&db.user.reputation).gt(1000));
    let vc = us.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&vc).and((&first).map(|(_, p)| p).opt()));
    let v = top_n(v, |&(u, (_, p))| {
        let s = p.map(|p| score.get(p).unwrap());
        (Reverse(db.user.reputation.get(u).unwrap()), s.is_none(), Reverse(s))
    }, 20);
    rows(v.into_iter().skip(10).map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "created", "score"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3)
//     WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// TopPosts AS (SELECT PostId, Title, OwnerUserId, CreationDate, CommentCount, UpvoteCount FROM RankedPosts WHERE rn = 1 ORDER BY UpvoteCount DESC LIMIT 10)
// SELECT up.DisplayName AS OwnerName, tp.Title, tp.CommentCount, tp.UpvoteCount, tp.CreationDate FROM TopPosts tp JOIN Users up ON tp.OwnerUserId = up.Id
// ORDER BY tp.UpvoteCount DESC, tp.CreationDate ASC;
//
// PARTITION BY p.Id after GROUP BY p.Id gives every row rn = 1.
fn q9971(db: &'static So) -> String {
    let since = add_years(now_utc(), -1);
    let Post { creation_date, owner_user, .. } = &db.post;
    let vs = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select(&db.vote.vote_type_id);
    let s = db
        .post
        .with(creation_date.filt(move |d| ny_to_utc(d) >= since))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(vs.opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let v = top_n(drain(&s), |&(p, a)| (Reverse(a[1]), p), 10);
    let tp = rel(v);
    let v = drain((&tp).select(Same::<(Id<Post>, [i64; 2])>::new().and(Same::<(Id<Post>, [i64; 2])>::new().map(|(p, _)| p).select(owner_user))));
    rows(v.into_iter().map(|(_, ((p, a), u))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, UpVoteCount, DownVoteCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserStatistics)
// SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, UpVoteCount, DownVoteCount, Rank FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q10057(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let v = top_n(drain((&s).and(user_distinct_posts(db))), |&(u, (_, n))| (Reverse(n), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, (a, n)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(c.Score) AS TotalCommentScore, SUM(v.BountyAmount) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalCommentScore, TotalBounty,
//        RANK() OVER (ORDER BY PostCount DESC, Reputation DESC) AS rn FROM UserActivity)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalCommentScore, TotalBounty FROM TopUsers WHERE rn <= 10;
//
// v.UserId = u.Id with p.OwnerUserId = u.Id is a vote cast by the post's owner: `own_votes`.
fn q6691(db: &'static So) -> String {
    let own = own_votes(db);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).select(&db.comment.score).opt().and((&own).select((&db.vote.bounty_amount).opt()).opt()))).opt())
        .fold([0i64; 6], |a, x| match x {
            Some((t, (c, v))) => {
                let b = v.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + c.unwrap_or(0), a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0)]
            }
            None => a,
        });
    let v = ranked(drain((&s).and(user_distinct_posts(db))), |&(u, (_, n))| (Reverse(n), Reverse(db.user.reputation.get(u).unwrap())), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), _)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), nullable(a[5], a[4])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, tp.VoteCount, u.DisplayName AS UserDisplayName, u.Reputation
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// tp.PostId = u.Id compares a post id with a user id, so it goes through origid. Rank reads only Score, so the top posts are picked first.
fn q6755(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let nc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(distinct_some);
    let nv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).buf_fold(distinct_some);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&nc).and(&nv).and(origid.select(&uidx)));
    rows(v.into_iter().map(|(p, ((c, n), u))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, COUNT(C.Id) AS TotalComments, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY COUNT(C.Id) DESC) AS UserRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY P.Id, P.Title, P.OwnerUserId),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.TotalComments, RP.UpVotes, RP.DownVotes FROM RankedPosts RP WHERE RP.UserRank <= 5)
// SELECT UP.Id AS UserId, UP.DisplayName, TP.Title, TP.TotalComments, TP.UpVotes, TP.DownVotes FROM Users UP JOIN TopPosts TP ON UP.Id = TP.PostId
// ORDER BY TP.UpVotes DESC, TP.TotalComments DESC;
//
// UP.Id = TP.PostId compares a user id with a post id, so it goes through origid.
fn q7786(db: &'static So) -> String {
    let Post { creation_date, owner_user, origid, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&s).and(owner_user.opt()));
    let top = top_per(v, |&(_, (_, u))| u, |&(p, (a, _))| (Reverse(a[0]), p), 5, false);
    let tp = rel(top.into_iter().map(|(p, (a, _))| (p, a)).collect());
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type R = (Id<Post>, [i64; 3]);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(origid).select(&uidx))));
    rows(v.into_iter().map(|(_, ((p, a), u))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT v.Id) AS TotalVotes, COUNT(DISTINCT c.Id) AS TotalComments,
//        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalVotes, TotalComments, TotalViews, TotalScore,
//        RANK() OVER (ORDER BY TotalPosts DESC, TotalVotes DESC, TotalComments DESC) AS ActivityRank FROM UserActivity)
// SELECT UserId, DisplayName, TotalPosts, TotalVotes, TotalComments, TotalViews, TotalScore, ActivityRank FROM RankedUsers WHERE ActivityRank <= 10 ORDER BY ActivityRank;
fn q10457(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(votes_of(db).opt().and(comments_of(db).opt()))).opt())
        .fold([0i64; 2], |a, x| match x {
            Some(((s, w), _)) => [a[0] + w.unwrap_or(0), a[1] + s],
            None => a,
        });
    let nv = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db)).opt()).buf_fold(distinct_some);
    let nc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = drain((&s).and(user_distinct_posts(db)).and(&nv).and(&nc));
    let v = ranked(v, |&(_, (((_, p), v), c))| (Reverse(p), Reverse(v), Reverse(c)), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (((a, p), v), c)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p), V::I(v), V::I(c), V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, CommentCount, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, CommentCount FROM TopUsers WHERE Rank <= 10;
fn q10837(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((t, (v, c))) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + c.is_some() as i64],
            None => a,
        });
    let v = ranked(drain((&s).and(user_distinct_posts(db))), |&(_, (_, n))| Reverse(n), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, RANK() OVER (ORDER BY TotalScore DESC) AS RankByScore,
//        RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts FROM UserStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, RankByScore, RankByPosts FROM TopUsers
// WHERE RankByScore <= 10 OR RankByPosts <= 10 ORDER BY RankByScore, RankByPosts;
fn q14119(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| (a[1] == 0, Reverse(a[4])), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[1]), false);
    rows(v.into_iter().filter(|&((_, s), p)| s <= 10 || p <= 10).map(|(((u, a), s), p)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), nullable(a[6], a[5]), V::I(s), V::I(p)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) DESC) AS PostRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.OwnerUserId),
// HighScoreUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation FROM Users u WHERE u.Reputation > 1000)
// SELECT u.DisplayName, COUNT(p.Id) AS TotalPosts, AVG(rp.Score) AS AverageScore
// FROM HighScoreUsers u LEFT JOIN RankedPosts rp ON u.UserId = rp.OwnerUserId LEFT JOIN Posts p ON p.OwnerUserId = u.UserId
// GROUP BY u.DisplayName HAVING AVG(rp.Score) > 10 ORDER BY AverageScore DESC;
//
// PostRank is never read.
fn q3131(db: &'static So) -> String {
    let vs = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let g = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(&vs).opt().and(posts_of(db).opt()))
        .fold([0i64; 3], |a, (s, p)| [a[0] + p.is_some() as i64, a[1] + s.is_some() as i64, a[2] + s.unwrap_or(0)]);
    rows(drain((&g).filt(|a| a[1] > 0 && a[2] > 10 * a[1])).into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), avg(a[2], a[1])])))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionsCount, AnswersCount, UpVotesCount, DownVotesCount, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionsCount, AnswersCount, UpVotesCount, DownVotesCount, Rank FROM TopUsers WHERE Rank <= 10;
//
// Rank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q11309(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let top: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&tu).and((&s).and(&np))).into_iter().map(|(u, ((_, r), (a, n)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// BadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// RankedUsers AS (SELECT upc.UserId, upc.DisplayName, upc.PostCount, upc.QuestionCount, upc.AnswerCount, COALESCE(bc.BadgeCount, 0) AS BadgeCount,
//        RANK() OVER (ORDER BY upc.PostCount DESC) AS UserRank FROM UserPostCounts upc LEFT JOIN BadgeCounts bc ON upc.UserId = bc.UserId)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, BadgeCount, UserRank FROM RankedUsers ORDER BY UserRank LIMIT 100;
fn q11441(db: &'static So) -> String {
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = ranked(drain((&user_posts(db)).and((&bc).opt())), |&(_, (a, _))| Reverse(a[1]), false);
    let v = top_n(v, |x| x.1, 100);
    rows(v.into_iter().map(|((u, (a, b)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(b.unwrap_or(0)), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionsCount, AnswersCount, UpVotesCount, DownVotesCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT UserId, Reputation, PostCount, QuestionsCount, AnswersCount, UpVotesCount, DownVotesCount, Rank FROM TopUsers WHERE Rank <= 10;
//
// Rank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q10381(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tr = rel(v.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let top: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&tu).and((&s).and(&np))).into_iter().map(|(u, ((_, r), (a, n)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, ScoreRank, ViewRank FROM TopUsers
// WHERE ScoreRank <= 10 OR ViewRank <= 10 ORDER BY ScoreRank, ViewRank;
fn q13982(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| Reverse(a[4]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[6]), false);
    rows(v.into_iter().filter(|&((_, s), w)| s <= 10 || w <= 10).map(|(((u, a), s), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6]), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        (SELECT COUNT(*) FROM Posts sub WHERE sub.ParentId = p.Id) AS AnswerCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, CommentCount, VoteCount, AnswerCount, RANK() OVER (ORDER BY Score DESC) AS ScoreRank FROM PostStats)
// SELECT PostId, Title, CreationDate, ViewCount, Score, CommentCount, VoteCount, AnswerCount, ScoreRank FROM TopPosts WHERE ScoreRank <= 10 ORDER BY Score DESC;
//
// ScoreRank reads only Score, so the top posts are picked first and the product is driven for those alone.
fn q10777(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score)), |&(_, s)| Reverse(s), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    let tpr: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let tp: MatSet<Id<Post>> = (&tr).map(|(p, _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let nv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).buf_fold(distinct_some);
    let na = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&tpr).and((&cc).and(&nv).and(&na))).into_iter().map(|(p, ((_, r), ((c, v), a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(v), V::I(a), V::I(r)]);
        row(f)
    }))
}

// Rewritten (rewrites/8514.sql): the ROW_NUMBER order is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '3 years' AND p.Score > 0),
// TopRankedPosts AS (SELECT PostId, Title, Score, CreationDate, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// CommentCounts AS (SELECT PostId, COUNT(*) AS TotalComments FROM Comments GROUP BY PostId)
// SELECT trp.Title, trp.Score, trp.CreationDate, trp.ViewCount, trp.OwnerDisplayName, COALESCE(cc.TotalComments, 0) AS CommentCount
// FROM TopRankedPosts trp LEFT JOIN CommentCounts cc ON trp.PostId = cc.PostId ORDER BY trp.Score DESC, trp.ViewCount DESC;
fn q8514(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -3)).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain(&cc).into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["title", "score", "created", "views", "owner"]);
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(u.UpVotes) AS TotalUpVotes,
//        SUM(u.DownVotes) AS TotalDownVotes, RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT * FROM RankedUsers WHERE UserRank <= 10),
// PostStatistics AS (SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AverageScore
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name)
// SELECT t.DisplayName, t.PostCount AS UserPostCount, t.TotalScore AS UserTotalScore, t.TotalUpVotes, t.TotalDownVotes, ps.PostType, ps.PostCount AS TypePostCount,
//        ps.TotalViews, ps.AverageScore
// FROM TopUsers t CROSS JOIN PostStatistics ps ORDER BY t.TotalScore DESC, ps.TotalViews DESC;
fn q8370(db: &'static So) -> String {
    let User { up_votes, down_votes, .. } = &db.user;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db).select(&db.post.score).opt()))
        .fold([0i64; 3], |a, ((up, dn), s)| [a[0] + s.unwrap_or(0), a[1] + up, a[2] + dn]);
    let v = ranked(drain((&s).and(user_distinct_posts(db))), |&(_, (_, n))| Reverse(n), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), _)| (u, a, n)).collect());
    let Post { score, view_count, .. } = &db.post;
    let ps = db
        .post
        .group_by(ptype_name(db))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let mut v = Vec::new();
    (&tu).cross(&ps).drive(|(_, t), ((u, a, n), b)| v.push((u, a, n, t, b)));
    rows(v.into_iter().map(|(u, a, n, t, b)| {
        row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(t), V::I(b[0]), nullable(b[2], b[1]), avg(b[3], b[0])])
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation > 1000),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// ActivePosts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId)
// SELECT ru.UserId, ru.DisplayName, ru.Reputation, ru.ReputationRank, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ap.PostCount, 0) AS ActivePostCount,
//        COALESCE(ap.TotalScore, 0) AS ActivePostScore
// FROM RankedUsers ru LEFT JOIN UserBadges ub ON ru.UserId = ub.UserId LEFT JOIN ActivePosts ap ON ru.UserId = ap.OwnerUserId ORDER BY ru.ReputationRank, ru.Reputation DESC;
fn q7434(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ap = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let v = ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select((&bc).opt().and((&ap).opt()))), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    rows(v.into_iter().map(|((u, (b, a)), r)| {
        let a = a.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(r), V::I(b.unwrap_or(0)), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, AVG(p.ViewCount) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalUpvotes, TotalDownvotes, AvgViewCount, RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts,
//        RANK() OVER (ORDER BY TotalUpvotes DESC) AS RankByUpvotes FROM UserActivity)
// SELECT tu.DisplayName, tu.PostCount, tu.TotalUpvotes, tu.TotalDownvotes, tu.AvgViewCount, (tu.RankByPosts + tu.RankByUpvotes) / 2.0 AS OverallRank
// FROM TopUsers tu WHERE tu.PostCount > 5 ORDER BY OverallRank ASC FETCH FIRST 10 ROWS ONLY;
fn q9950(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((w, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)],
            None => a,
        });
    let v = ranked(drain((&s).and(user_distinct_posts(db))), |&(_, (_, n))| Reverse(n), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[0]), false);
    let v: Vec<_> = drain(rel(v).filt(|(((_, (_, n)), _), _)| n > 5)).into_iter().map(|x| x.1).collect();
    let v = top_n(v, |&((_, p), q)| p + q, 10);
    rows(v.into_iter().map(|(((u, (a, n)), p), q)| row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::F((p + q) as f64 / 2.0)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.Score, p.PostTypeId, p.CreationDate),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT f.Title, f.Score, f.CommentCount, (f.UpvoteCount - f.DownvoteCount) AS NetVotes FROM FilteredPosts f ORDER BY f.Score DESC, f.CommentCount DESC;
//
// Rank reads only base columns, so the five newest posts per type are picked first and the product is driven for those alone.
fn q9154(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1] - a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1),
// CombinedData AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        COALESCE(b.Count, 0) AS BadgeCount, CASE WHEN rp.Rank = 1 THEN 'Most Recent' ELSE 'Previous' END AS PostStatus
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN (SELECT UserId, COUNT(*) AS Count FROM Badges GROUP BY UserId) b ON u.Id = b.UserId)
// SELECT Title, CreationDate, OwnerDisplayName, OwnerReputation, ViewCount, Score, BadgeCount, PostStatus FROM CombinedData ORDER BY ViewCount DESC LIMIT 10;
fn q26476(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.select((&bc).opt()).and(Ident::<Post>::new().with(&first).opt())));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (b, r))| {
        let mut f = post_fields(db, p, &["title", "created", "owner", "rep", "views", "score"]);
        f.push(V::I(b.unwrap_or(0)));
        f.push(V::S(if r.is_some() { "Most Recent" } else { "Previous" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TopScoringPosts AS (SELECT PostId, Title, OwnerDisplayName, Score, ViewCount FROM RankedPosts WHERE Rank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT p.PostId, p.Title, p.OwnerDisplayName, p.Score, p.ViewCount, COALESCE(pc.CommentCount, 0) AS CommentCount
// FROM TopScoringPosts p LEFT JOIN PostComments pc ON p.PostId = pc.PostId ORDER BY p.Score DESC, p.ViewCount DESC;
fn q9138(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain(&cc).into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views"]);
        f.push(V::I(n));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, AVG(p.Score) AS AverageScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        SUM(COALESCE(p.FavoriteCount, 0)) AS TotalFavorites
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, AverageScore, TotalViews, TotalFavorites, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank,
//        RANK() OVER (ORDER BY AverageScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, AverageScore, TotalViews, TotalFavorites, PostRank, ScoreRank FROM TopUsers
// WHERE PostRank <= 10 OR ScoreRank <= 10 ORDER BY PostRank, ScoreRank;
fn q14950(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, favorite_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(favorite_count.opt())).opt())
        .fold([0i64; 6], |a, x| match x {
            Some((((t, s), w), f)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0), a[5] + f.unwrap_or(0)],
            None => a,
        });
    let m = |a: &[i64; 6]| if a[0] == 0 { None } else { Some(fkey(a[3] as f64 / a[0] as f64)) };
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    let v = ranked(v, |&((_, a), _)| (m(&a).is_none(), Reverse(m(&a))), false);
    rows(v.into_iter().filter(|&((_, p), s)| p <= 10 || s <= 10).map(|(((u, a), p), s)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::I(a[4]), V::I(a[5]), V::I(p), V::I(s)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotes, TotalDownvotes, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserStatistics)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotes, TotalDownvotes, PostRank FROM TopUsers WHERE PostRank <= 10 ORDER BY PostRank;
fn q14775(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let v = ranked(drain((&s).and(user_distinct_posts(db))), |&(_, (_, n))| Reverse(n), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER(PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, u.DisplayName, p.Title, p.PostTypeId, p.CreationDate),
// TopPosts AS (SELECT PostId, Title, OwnerName, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE Rank <= 10)
// SELECT t.Title, t.OwnerName, t.CommentCount, t.UpVotes, t.DownVotes, (t.UpVotes - t.DownVotes) AS Score FROM TopPosts t ORDER BY Score DESC, t.CommentCount DESC;
//
// Rank reads only base columns, so the ten newest posts per type are picked first and the product is driven for those alone.
fn q6326(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS Author, P.Score,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId IN (1, 2) AND P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Author, Score FROM RankedPosts WHERE Rank <= 10),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, SUM(c.Score) AS TotalScore FROM Comments c GROUP BY c.PostId)
// SELECT TP.PostId, TP.Title, TP.CreationDate, TP.Author, TP.Score, COALESCE(PC.CommentCount, 0) AS CommentCount, COALESCE(PC.TotalScore, 0) AS CommentTotalScore
// FROM TopPosts TP LEFT JOIN PostComments PC ON TP.PostId = PC.PostId ORDER BY TP.Score DESC, TP.CreationDate ASC;
fn q7472(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(date(2024, 10, 1), -1)))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    rows(drain(&pc).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore, COUNT(B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, TotalViews, TotalScore, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT T.UserId, T.DisplayName, T.Reputation, T.PostCount, T.AnswerCount, T.TotalViews, T.TotalScore, T.BadgeCount FROM TopUsers T WHERE T.ReputationRank <= 10
// ORDER BY T.TotalViews DESC, T.Reputation DESC;
//
// ReputationRank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q5150(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), _)| u).collect()).map(|u| u).collect();
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 6], |a, (p, b)| match p {
            Some(((t, s), w)) => [a[0] + (t == 2) as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + 1, a[4] + s, a[5] + b.is_some() as i64],
            None => [a[0], a[1], a[2], a[3], a[4], a[5] + b.is_some() as i64],
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let mut v = drain((&s).and(&np));
    v.sort_by_key(|&(u, (a, _))| (a[1] == 0, Reverse(a[2]), Reverse(db.user.reputation.get(u).unwrap())));
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), nullable(a[2], a[1]), nullable(a[4], a[3]), V::I(a[5])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, SUM(P.Score) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalUpVotes, TotalDownVotes, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserActivity)
// SELECT T.DisplayName, T.TotalPosts, T.TotalComments, T.TotalUpVotes, T.TotalDownVotes, T.TotalScore, T.Rank FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.Rank;
fn q6245(db: &'static So) -> String {
    let us = || db.user.with((&db.user.reputation).gt(1000));
    let s = us()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((s, (_, t))) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + 1, a[3] + s],
            None => a,
        });
    let np = us().group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let nc = us().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(&np).and(&nc)), |&(_, ((a, _), _))| (a[2] == 0, Reverse(a[3])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, p), c)), r)| {
        row(vec![user_col(db, u, "name"), V::I(p), V::I(c), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(r)])
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
//        SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews, SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, TotalScore, TotalViews, VoteCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewsRank FROM UserStats)
// SELECT UserId, Reputation, PostCount, TotalScore, TotalViews, VoteCount, ReputationRank, ScoreRank, ViewsRank FROM TopUsers
// WHERE ReputationRank <= 10 OR ScoreRank <= 10 OR ViewsRank <= 10 ORDER BY Reputation DESC;
fn q11464(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(votes_of(db).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some(((s, w), v)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + v.is_some() as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[1]), false);
    let v = ranked(v, |&(((_, a), _), _)| Reverse(a[2]), false);
    rows(v.into_iter().filter(|&(((_, r), s), w)| r <= 10 || s <= 10 || w <= 10).map(|((((u, a), r), s), w)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(r), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, u.DisplayName AS Owner, p.CreationDate, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        DENSE_RANK() OVER (PARTITION BY p.Tags ORDER BY COALESCE(SUM(v.VoteTypeId), 0) DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Tags, u.DisplayName, p.CreationDate),
// TopPosts AS (SELECT * FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.Tags, tp.Owner, tp.CreationDate, tp.CommentCount, tp.Upvotes, tp.Downvotes, pt.Name AS PostType
// FROM TopPosts tp JOIN PostTypes pt ON tp.PostId = pt.Id ORDER BY tp.Rank, tp.Upvotes DESC;
//
// tp.PostId = pt.Id compares a post id with a post type id, so it goes through origid.
fn q27972(db: &'static So) -> String {
    let Post { post_type_id, tags_str, origid, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + t.unwrap_or(0)]);
    let v = drain((&s).and(tags_str.opt()));
    let v = ranked(v, |&(_, (a, t))| (t, Reverse(a[3])), true);
    let v = per_group(v, |&(_, (_, t))| t);
    let tp = rel(v.into_iter().filter(|x| x.1 <= 5).map(|((p, (a, _)), r)| (p, a, r)).collect());
    let ptype: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    type R = (Id<Post>, [i64; 4], i64);
    let mut v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _, _): R| p).select(origid).select(&ptype))));
    v.sort_by_key(|&(_, ((_, a, r), _))| (r, Reverse(a[1])));
    rows(v.into_iter().map(|(_, ((p, a, _), t))| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "owner", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(db.post_type.name.get(t).unwrap())]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(p.Score) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// ActiveUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserPostStats
//     WHERE TotalPosts > 0 ORDER BY TotalScore DESC LIMIT 10),
// UserBadges AS (SELECT ub.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b JOIN ActiveUsers ub ON b.UserId = ub.UserId GROUP BY ub.UserId)
// SELECT au.DisplayName, au.TotalPosts, au.Questions, au.Answers, au.TotalScore, COALESCE(ub.BadgeCount, 0) AS BadgeCount FROM ActiveUsers au
// LEFT JOIN UserBadges ub ON au.UserId = ub.UserId ORDER BY au.Rank;
fn q6649(db: &'static So) -> String {
    let v = top_n(drain((&user_posts(db)).filt(|a| a[1] > 0)), |&(_, a)| Reverse(a[4]), 10);
    let au: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bc = (&au).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&bc).and(user_posts(db))).into_iter().map(|(u, (b, a))| row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(b)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Owner, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL),
// TopScores AS (SELECT Owner, MAX(Score) AS MaxScore, COUNT(PostId) AS PostCount FROM RankedPosts WHERE Rank <= 3 GROUP BY Owner),
// RecentActivity AS (SELECT p.OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.LastActivityDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY p.OwnerDisplayName)
// SELECT t.Owner, t.MaxScore, t.PostCount, ra.CommentCount, ra.VoteCount FROM TopScores t JOIN RecentActivity ra ON t.Owner = ra.OwnerDisplayName
// ORDER BY t.MaxScore DESC, t.PostCount DESC LIMIT 10;
fn q7815(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, last_activity_date, owner_display_name, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ts_ = (&tp).group_by(owner_user.select(&db.user.display_name)).select(score).fold((i64::MIN, 0i64), |(m, n), s| (m.max(s), n + 1));
    let ra = db
        .post
        .with(last_activity_date.ge(add_days(date(2024, 10, 1), -30)))
        .group_by(owner_display_name)
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let v = top_n(drain((&ts_).and(&ra)), |&(_, ((m, n), _))| (Reverse(m), Reverse(n)), 10);
    rows(v.into_iter().map(|(o, ((m, n), a))| row(vec![V::S(o), V::I(m), V::I(n), V::I(a[0]), V::I(a[1])])))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT U.DisplayName, U.CreationDate, T.Reputation, T.PostCount, T.QuestionCount, T.AnswerCount, T.UpVotes, T.DownVotes FROM TopUsers T JOIN Users U ON T.UserId = U.Id
// WHERE T.Rank <= 10 ORDER BY T.Rank;
//
// Rank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q13363(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&s).and(&np)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "ucreated", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank,
//        COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        AVG(v.VoteTypeId) AS AverageVoteType
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate)
// SELECT ru.UserId, ru.DisplayName, ru.Reputation, ru.ReputationRank, ru.PostCount, ru.AnswerCount, ru.QuestionCount, ru.AverageVoteType, bh.BadgeCount
// FROM RankedUsers ru LEFT JOIN (SELECT UserId, COUNT(Id) AS BadgeCount FROM Badges GROUP BY UserId) bh ON ru.UserId = bh.UserId
// WHERE ru.ReputationRank <= 10 ORDER BY ru.Reputation DESC, ru.DisplayName;
//
// ReputationRank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q26838(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let top: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)],
            None => a,
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&tu).and((&s).and(&np).and((&bc).opt()))).into_iter().map(|(u, ((_, r), ((a, n), b)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(r), V::I(n), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), oint(b)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, AVG(P.Score) AS AvgScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, UpVotes, DownVotes, AvgScore, ROW_NUMBER() OVER (ORDER BY PostCount DESC, AvgScore DESC) AS Rank FROM UserActivity)
// SELECT T.DisplayName, T.PostCount, T.UpVotes, T.DownVotes, T.AvgScore, PH.Comment AS RecentActivity
// FROM TopUsers T LEFT JOIN PostHistory PH ON T.UserId = PH.UserId AND PH.CreationDate = (SELECT MAX(PH2.CreationDate) FROM PostHistory PH2 WHERE PH2.UserId = T.UserId)
// WHERE T.Rank <= 10 ORDER BY T.Rank;
fn q472(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((s, v)) => [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + 1, a[3] + s],
            None => a,
        });
    let m = |a: &[i64; 4]| if a[2] == 0 { None } else { Some(fkey(a[3] as f64 / a[2] as f64)) };
    let v = top_n(drain((&s).and(user_distinct_posts(db))), |&(u, (a, n))| (Reverse(n), m(&a).is_none(), Reverse(m(&a)), u), 10);
    let tu = rel(v.into_iter().enumerate().map(|(i, (u, (a, n)))| (u, a, n, i)).collect());
    let PostHistory { user, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(user).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<User>, i64), Id<PostHistory>> = db.post_history.select(user.and(hd)).inv().collect();
    type R = (Id<User>, [i64; 4], i64, usize);
    let mut v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _, _, _): R| u).select(Ident::<User>::new().and(&md).select(&at)).opt())));
    v.sort_by_key(|&(_, ((_, _, _, i), _))| i);
    rows(v.into_iter().map(|(_, ((u, a, n, _), h))| {
        row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), h.map_or(V::Null, |h| ostr(db.post_history.comment.get(h)))])
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.AnswerCount, p.CommentCount, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        COUNT(DISTINCT c.Id) AS TotalComments, AVG(v.BountyAmount) AS AvgBountyAmount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.AnswerCount, p.CommentCount, p.ViewCount, p.Score, u.DisplayName),
// TopPosts AS (SELECT *, RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostStats)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.AnswerCount, tp.CommentCount, tp.ViewCount, tp.Score, tp.TotalComments, tp.AvgBountyAmount
// FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Rank;
//
// Rank reads only base columns, so the top posts are picked first and the product is driven for those alone.
fn q8565(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(score)), |&(p, _)| key(p), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let b = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 2], |a, (_, b)| match b.flatten() {
        Some(b) => [a[0] + 1, a[1] + b],
        None => a,
    });
    let nc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&b).and(&nc)).into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "answers", "comments", "views", "score"]);
        f.extend([V::I(n), avg(a[1], a[0])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 1 THEN p.ViewCount ELSE 0 END) AS TotalQuestionViews,
//        AVG(p.Score) AS AvgScore, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalQuestionViews, AvgScore, TotalComments,
//        ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalQuestionViews, AvgScore, TotalComments FROM TopUsers WHERE Rank <= 10;
fn q11263(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 7], |a, x| match x {
            Some((((t, s), w), _)) => {
                let q = if t == 1 { w } else { Some(0) };
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + q.is_some() as i64, a[4] + q.unwrap_or(0), a[5] + s, 0]
            }
            None => [a[0], a[1], a[2], a[3] + 1, a[4], a[5], a[6]],
        });
    let nc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&s).and(&nc)), |&(u, (a, _))| (Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, (a, c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), avg(a[5], a[0]), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, P.ViewCount, P.Score, COALESCE(C.Count, 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN (SELECT PostId, COUNT(*) AS Count FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.OwnerDisplayName, RP.ViewCount, RP.Score, RP.CommentCount, PHT.Name AS PostHistoryType, U2.DisplayName AS LastEditedBy, RP.Rank
// FROM RankedPosts RP LEFT JOIN PostHistory PH ON RP.PostId = PH.PostId LEFT JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id LEFT JOIN Users U2 ON PH.UserId = U2.Id
// WHERE RP.Rank <= 5 ORDER BY RP.Score DESC, RP.CreationDate DESC;
fn q5788(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let v = ranked(top, |&(p, t)| (t, Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), false);
    let v = per_group(v, |&(_, t)| t);
    let tr = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let tp: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let ps: MatSet<Id<Post>> = (&tr).map(|(p, _)| p).collect();
    let cc = (&ps).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&tp).and((&cc).and(history_of(db).opt())));
    rows(v.into_iter().map(|(p, ((_, r), (n, h)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "views", "score"]);
        f.push(V::I(n));
        f.extend(match h {
            Some(h) => [V::S(htype_name(db).get(h).unwrap()), ostr(db.post_history.user.get(h).map(|u| db.user.display_name.get(u).unwrap()))],
            None => [V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount > 1000),
// TopUsers AS (SELECT OwnerDisplayName, COUNT(Id) AS PostCount, SUM(ViewCount) AS TotalViews, AVG(Score) AS AverageScore FROM RankedPosts WHERE PostRank <= 5 GROUP BY OwnerDisplayName)
// SELECT u.DisplayName, u.Reputation, u.CreationDate, COALESCE(tu.PostCount, 0) AS TopPosts, COALESCE(tu.TotalViews, 0) AS TotalPostViews, COALESCE(tu.AverageScore, 0) AS AvgPostScore
// FROM Users u LEFT JOIN TopUsers tu ON u.DisplayName = tu.OwnerDisplayName WHERE u.Reputation > 5000 ORDER BY u.Reputation DESC, TopPosts DESC;
fn q6773(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(view_count.gt(1000)).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tu = (&tp).group_by(owner_user.select(&db.user.display_name)).select(view_count.and(score)).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w, a[2] + s]);
    let v = drain(db.user.with((&db.user.reputation).gt(5000)).select((&db.user.display_name).select(&tu).opt()));
    rows(v.into_iter().map(|(u, t)| {
        let a = t.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["name", "rep", "ucreated"]);
        f.extend([V::I(a[0]), V::I(a[1]), if a[0] == 0 { V::F(0.0) } else { avg(a[2], a[0]) }]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, UpVotes, DownVotes, QuestionCount, AnswerCount, BadgeCount, RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, UpVotes, DownVotes, QuestionCount, AnswerCount, BadgeCount FROM TopUsers WHERE PostRank <= 10;
fn q10944(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let b = b.is_some() as i64;
            match p {
                Some((t, v)) => [a[0] + 1, a[1] + (v == Some(2)) as i64, a[2] + (v == Some(3)) as i64, a[3] + (t == 1) as i64, a[4] + (t == 2) as i64, a[5] + b],
                None => [a[0], a[1], a[2], a[3], a[4], a[5] + b],
            }
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Owner, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, u.DisplayName, p.Title, p.CreationDate, p.PostTypeId),
// TopPosts AS (SELECT * FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Owner, tp.CommentCount, tp.UpVotes, tp.DownVotes, (tp.UpVotes - tp.DownVotes) AS NetVotes FROM TopPosts tp ORDER BY tp.CreationDate DESC;
//
// Rank reads only base columns, so the five newest posts per type are picked first and the product is driven for those alone.
fn q5388(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.is_in([1, 2]))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.Tags, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// CommentStats AS (SELECT c.PostId, COUNT(*) AS CommentCount, AVG(LENGTH(c.Text)) AS AvgCommentLength FROM Comments c GROUP BY c.PostId)
// SELECT up.DisplayName AS UserName, rp.Title, rp.CreationDate, rp.Score, COALESCE(cs.CommentCount, 0) AS TotalComments, COALESCE(cs.AvgCommentLength, 0) AS AverageCommentLength,
//        CASE WHEN rp.Score >= 100 THEN 'High Score' WHEN rp.Score BETWEEN 50 AND 99 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM RankedPosts rp LEFT JOIN Users up ON rp.OwnerUserId = up.Id LEFT JOIN CommentStats cs ON rp.PostId = cs.PostId
// WHERE rp.PostRank = 1 ORDER BY rp.Score DESC, rp.CreationDate FETCH FIRST 10 ROWS ONLY;
fn q79(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let v = top_n(top, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.text).opt()).fold([0i64; 2], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + t.chars().count() as i64],
        None => a,
    });
    rows(drain(&cs).into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["owner", "title", "created", "score"]);
        f.extend([V::I(a[0]), if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) }]);
        f.push(V::S(if s >= 100 { "High Score" } else if (50..=99).contains(&s) { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, COALESCE(u.DisplayName, 'Community User') AS Author,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, ViewCount, AnswerCount, Author FROM RankedPosts WHERE Rank = 1)
// SELECT tp.Title, tp.ViewCount, tp.AnswerCount, tp.Author, pt.Name AS PostTypeName, COALESCE(bt.BadgeCount, 0) AS AuthorBadgeCount
// FROM TopPosts tp JOIN PostTypes pt ON 1 = pt.Id LEFT JOIN (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId) bt ON bt.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId)
// ORDER BY tp.ViewCount DESC LIMIT 10;
//
// ON 1 = pt.Id names only pt, so the top posts are crossed with the post types whose Id is 1.
fn q25284(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, last_activity_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(date(2024, 10, 1), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(last_activity_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let pt = db.post_type.with((&db.post_type.origid).eq(1)).select(&db.post_type.name);
    let mut v = Vec::new();
    (&tp).select(owner_user.select(&bc).opt()).cross(pt).drive(|(p, _), (b, n)| v.push((p, b, n)));
    let v = top_n(v, |&(p, _, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, 10);
    rows(v.into_iter().map(|(p, b, n)| {
        let mut f = post_fields(db, p, &["title", "views", "answers"]);
        f.push(V::S(owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::S(n), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, DENSE_RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStats)
// SELECT TU.DisplayName, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.TotalScore, TU.TotalViews,
//        CASE WHEN TU.ScoreRank <= 10 THEN 'Top Contributor' WHEN TU.ScoreRank BETWEEN 11 AND 50 THEN 'Contributing Member' ELSE 'New Contributor' END AS ContributionLevel
// FROM TopUsers TU WHERE TU.TotalPosts > 0 ORDER BY TU.TotalScore DESC LIMIT 20;
fn q9369(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| (a[1] == 0, Reverse(a[4])), true);
    let v: Vec<_> = drain(rel(v).filt(|x| x.0 .1[1] > 0)).into_iter().map(|x| x.1).collect();
    let v = top_n(v, |&((_, a), _)| Reverse(a[4]), 20);
    rows(v.into_iter().map(|((u, a), r)| {
        row(vec![
            user_col(db, u, "name"),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            V::I(a[4]),
            nullable(a[6], a[5]),
            V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Contributing Member" } else { "New Contributor" }),
        ])
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.Reputation, u.Views, u.UpVotes, u.DownVotes, COUNT(DISTINCT p.Id) AS TotalPosts,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
//        AVG(v.BountyAmount) AS AverageBountyAmount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.Reputation, u.Views, u.UpVotes, u.DownVotes),
// TopUsers AS (SELECT UserId, Reputation, TotalPosts, TotalQuestions, TotalAnswers, AverageBountyAmount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStatistics)
// SELECT UserId, Reputation, TotalPosts, TotalQuestions, TotalAnswers, AverageBountyAmount, ReputationRank FROM TopUsers WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
//
// ReputationRank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q10170(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let top: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let s = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold([0i64; 2], |a, x| match x.flatten().flatten() {
        Some(b) => [a[0] + 1, a[1] + b],
        None => a,
    });
    let of = |t: i64| (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(t))).opt()).buf_fold(distinct_some);
    let (nq, na) = (of(1), of(2));
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&tu).and((&s).and(&np).and(&nq).and(&na))).into_iter().map(|(u, ((_, r), (((a, p), q), n)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(p), V::I(q), V::I(n), avg(a[1], a[0]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN C.PostId IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(A.Id) AS AnswerCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON C.PostId = P.Id LEFT JOIN Posts A ON A.ParentId = P.Id AND A.PostTypeId = 2
//     WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, AnswerCount, RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM RankedPosts)
// SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, AnswerCount, Rank FROM TopPosts WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only base columns, so the top questions are picked first and the product is driven for those alone.
fn q10753(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, _)| key(p), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    let tpr: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let tp: MatSet<Id<Post>> = (&tr).map(|(p, _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()))
        .fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    rows(drain((&tpr).and(&s)).into_iter().map(|(p, ((_, r), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, QuestionCount, AnswerCount, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, PostCount, TotalScore, QuestionCount, AnswerCount, UpVotes, DownVotes, Rank FROM TopUsers WHERE Rank <= 10 ORDER BY TotalScore DESC;
fn q12510(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, x| match x {
            Some(((s, t), v)) => [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[1]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserScores AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes - DownVotes AS NetVotes, TotalPosts, TotalComments, RANK() OVER (ORDER BY UpVotes - DownVotes DESC, TotalPosts DESC) AS Rank FROM UserScores)
// SELECT tu.DisplayName, tu.NetVotes, tu.TotalPosts, tu.TotalComments,
//        CASE WHEN tu.Rank <= 10 THEN 'Top Contributor' WHEN tu.Rank <= 50 THEN 'Prolific Contributor' ELSE 'New Contributor' END AS ContributorRating
// FROM TopUsers tu WHERE tu.Rank <= 100 ORDER BY tu.Rank;
//
// v.UserId = u.Id with p.OwnerUserId = u.Id is a vote cast by the post's owner: `own_votes`.
fn q5467(db: &'static So) -> String {
    let own = own_votes(db);
    let us = || db.user.with((&db.user.reputation).gt(1000));
    let s = us()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and((&own).select(&db.vote.vote_type_id).opt())).opt())
        .fold(0i64, |n, x| {
            let t = x.and_then(|(_, t)| t);
            n + (t == Some(2)) as i64 - (t == Some(3)) as i64
        });
    let np = us().group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let nc = us().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(&np).and(&nc)), |&(_, ((n, p), _))| (Reverse(n), Reverse(p)), false);
    rows(v.into_iter().take_while(|x| x.1 <= 100).map(|((u, ((n, p), c)), r)| {
        row(vec![user_col(db, u, "name"), V::I(n), V::I(p), V::I(c), V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Prolific Contributor" } else { "New Contributor" })])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.LastActivityDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.LastActivityDate, p.PostTypeId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.LastActivityDate, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
// FROM RankedPosts rp JOIN Users u ON rp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = u.Id) WHERE rp.Rank <= 5 ORDER BY rp.LastActivityDate DESC;
//
// Rank reads only base columns, so the five most recently active posts per type are picked first and the product is driven for those alone.
fn q5228(db: &'static So) -> String {
    let Post { post_type_id, creation_date, last_activity_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(last_activity_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&s).and(owner_user)).into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.Score > 0),
// PostHistorySummary AS (SELECT p.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph JOIN RankedPosts p ON ph.PostId = p.PostId GROUP BY p.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerReputation, COALESCE(pHS.EditCount, 0) AS EditCount, pHS.LastEditDate
// FROM RankedPosts rp LEFT JOIN PostHistorySummary pHS ON rp.PostId = pHS.PostId WHERE rp.rn <= 5 ORDER BY rp.PostId DESC;
fn q9505(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let hs = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    rows(drain(&hs).into_iter().map(|(p, (n, m))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "rep"]);
        f.extend([V::I(n), tmax(m)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount FROM RankedPosts WHERE Rank <= 10)
// SELECT u.DisplayName AS Author, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
// GROUP BY u.DisplayName, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// tp.PostId = u.Id compares a post id with a user id, so it goes through origid.
fn q8111(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, origid, title, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let key = origid.select(&uidx).select(&db.user.display_name).and(title.opt()).and(creation_date).and(score).and(view_count.opt());
    let g = (&tp)
        .group_by(key)
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&g).into_iter().map(|(((((n, t), d), s), w), a)| row(vec![V::S(n), ostr(t), V::T(d), V::I(s), oint(w), V::I(a[0]), V::I(a[1]), V::I(a[2])])))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS DownvotedPosts, COUNT(DISTINCT c.Id) AS CommentCount, SUM(v.BountyAmount) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, UpvotedPosts, DownvotedPosts, CommentCount, TotalBounties, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserActivity)
// SELECT t.UserId, t.DisplayName, t.PostCount, t.UpvotedPosts, t.DownvotedPosts, t.CommentCount, t.TotalBounties, RANK() OVER (ORDER BY t.TotalBounties DESC) AS BountyRank
// FROM TopUsers t WHERE t.Rank <= 10 ORDER BY t.PostCount DESC;
//
// v.UserId = u.Id with p.OwnerUserId = u.Id is a vote cast by the post's owner: `own_votes`.
fn q5313(db: &'static So) -> String {
    let own = own_votes(db);
    let us = || db.user.with((&db.user.reputation).gt(1000));
    let s = us()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(comments_of(db).opt().and((&own).select((&db.vote.bounty_amount).opt()).opt()))).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((s, (_, b))) => {
                let b = b.flatten();
                [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
            }
            None => a,
        });
    let np = us().group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let nc = us().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&s).and(&np).and(&nc)), |&(u, ((_, p), _))| (Reverse(p), u), 10);
    let v = ranked(v, |&(_, ((a, _), _))| (a[2] == 0, Reverse(a[3])), false);
    rows(v.into_iter().map(|((u, ((a, p), c)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1]), V::I(c), nullable(a[3], a[2]), V::I(r)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT Users.Id AS UserId, Users.DisplayName, COUNT(Posts.Id) AS PostCount, SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN Comments.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
//        SUM(CASE WHEN Votes.Id IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
//     FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId LEFT JOIN Comments ON Posts.Id = Comments.PostId LEFT JOIN Votes ON Posts.Id = Votes.PostId GROUP BY Users.Id, Users.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, CommentCount, VoteCount, RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, CommentCount, VoteCount FROM TopUsers WHERE PostRank <= 10;
fn q13395(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt().and(votes_of(db).opt()))).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((t, (c, v))) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + v.is_some() as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT u.Id AS UserId, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(a.VoteCount, 0)) AS TotalUpVotes, COUNT(DISTINCT p.Id) AS TotalPosts,
//        COUNT(DISTINCT c.Id) AS TotalComments, COUNT(DISTINCT b.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) a ON p.Id = a.PostId
//     LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopUsers AS (SELECT ue.UserId, ue.TotalViews, ue.TotalUpVotes, ue.TotalPosts, ue.TotalComments, ue.TotalBadges, ROW_NUMBER() OVER (ORDER BY ue.TotalViews DESC, ue.TotalUpVotes DESC) AS Rank
//     FROM UserEngagement ue)
// SELECT u.DisplayName, tu.TotalViews, tu.TotalUpVotes, tu.TotalPosts, tu.TotalComments, tu.TotalBadges FROM TopUsers tu JOIN Users u ON tu.UserId = u.Id
// WHERE tu.Rank <= 10 ORDER BY tu.TotalViews DESC, tu.TotalUpVotes DESC;
fn q6913(db: &'static So) -> String {
    let vc = db.vote.with((&db.vote.vote_type_id).eq(2)).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and((&vc).opt()).and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| match p {
            Some(((w, n), _)) => [a[0] + w.unwrap_or(0), a[1] + n.unwrap_or(0)],
            None => a,
        });
    let nc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let nb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&s).and(user_distinct_posts(db)).and(&nc).and(&nb)), |&(u, (((a, _), _), _))| (Reverse(a[0]), Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, (((a, p), c), b))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(p), V::I(c), V::I(b)])))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UR.UserId, UR.Reputation, UR.PostCount, RANK() OVER (ORDER BY UR.Reputation DESC) AS UserRank FROM UserReputation UR)
// SELECT TU.UserId, U.DisplayName, TU.Reputation, TU.PostCount, RP.Title, RP.CreationDate, RP.Score
// FROM TopUsers TU JOIN Users U ON TU.UserId = U.Id LEFT JOIN RankedPosts RP ON TU.UserId = RP.OwnerUserId AND RP.PostRank = 1 WHERE TU.UserRank <= 10 ORDER BY TU.Reputation DESC;
fn q2028(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), _)| u).collect()).map(|u| u).collect();
    let first = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let fr = rel(first.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&fr).map(|(u, _)| u).inv().select(&fr).collect();
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&np).and((&by_user).map(|(_, p)| p).opt())).into_iter().map(|(u, (n, p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created", "score"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.rn <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerName, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, (tp.UpVoteCount - tp.DownVoteCount) AS NetVoteScore
// FROM TopPosts tp ORDER BY NetVoteScore DESC;
//
// rn reads only base columns, so the ten newest posts per type are picked first and the product is driven for those alone.
fn q6220(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS Questions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS Answers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.Questions, tu.Answers, tu.UpVotes, tu.DownVotes FROM TopUsers tu WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC;
//
// ReputationRank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q7069(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), _)| u).collect()).map(|u| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, x| {
            let t = x.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let of = |t: i64| (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(t))).opt()).buf_fold(distinct_some);
    let (nq, na) = (of(1), of(2));
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&s).and(&np).and(&nq).and(&na)).into_iter().map(|(u, (((a, p), q), n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p), V::I(q), V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CreationDate, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount,
//        ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM RankedPosts)
// SELECT PostId, Title, Score, ViewCount, CreationDate, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount FROM TopPosts WHERE Rank <= 10;
//
// Rank reads only base columns, so the top posts are picked first and the product is driven for those alone.
fn q13803(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.is_in([1, 2])).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH TagDetails AS (SELECT t.Id AS TagId, t.TagName, p.Title AS PostTitle, p.CreationDate AS PostCreationDate, COALESCE(a.Score, 0) AS AnswerScore,
//        (SELECT COUNT(*) FROM Posts AS sub WHERE sub.Tags LIKE '%' || t.TagName || '%' AND sub.PostTypeId = 2) AS AnswerCount,
//        (SELECT COUNT(*) FROM Comments AS c WHERE c.PostId = p.Id) AS CommentCount
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Posts a ON a.ParentId = p.Id AND a.PostTypeId = 2 WHERE t.Count > 100),
// RankedTags AS (SELECT TagId, TagName, COUNT(DISTINCT PostTitle) AS PostCount, AVG(AnswerScore) AS AvgAnswerScore, SUM(CommentCount) AS TotalComments,
//        DENSE_RANK() OVER (ORDER BY COUNT(DISTINCT PostTitle) DESC) AS PopularityRank FROM TagDetails GROUP BY TagId, TagName)
// SELECT rt.TagId, rt.TagName, rt.PostCount, rt.AvgAnswerScore, rt.TotalComments FROM RankedTags rt WHERE rt.PopularityRank <= 10 ORDER BY rt.PopularityRank;
//
// AnswerCount is never read by RankedTags.
fn q26483(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|(_, t)| t).inv().select((&lt).map(|(p, _)| p)).collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let tags = || db.tag.with((&db.tag.count).gt(100));
    let g = tags()
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).select((&cc).opt().and(answers_of(db).select(&db.post.score).opt())))
        .fold([0i64; 3], |a, (c, s)| [a[0] + 1, a[1] + s.unwrap_or(0), a[2] + c.unwrap_or(0)]);
    let nt = tags().group_by(Ident::<Tag>::new()).select((&by_tag).select(&db.post.title)).buf_fold(|xs| distinct_some(xs.into_iter().map(Some)));
    let v = ranked(drain((&g).and(&nt)), |&(_, (_, n))| Reverse(n), true);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((t, (a, n)), _)| {
        row(vec![V::I(db.tag.origid.get(t).unwrap()), V::S(db.tag.tag_name.get(t).unwrap()), V::I(n), avg(a[1], a[0]), V::I(a[2])])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentTotal
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.ViewCount) AS TotalViews, RANK() OVER (ORDER BY SUM(p.ViewCount) DESC) AS UserRank
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName HAVING COUNT(p.Id) > 5)
// SELECT tu.DisplayName, tu.TotalViews, rp.Title, rp.ViewCount, rp.CommentTotal, CASE WHEN rp.Rank <= 3 THEN 'Top Post' ELSE 'Other Post' END AS PostCategory
// FROM TopUsers tu JOIN RankedPosts rp ON tu.UserId = rp.PostId WHERE tu.UserRank <= 10 ORDER BY tu.TotalViews DESC, rp.ViewCount DESC LIMIT 100;
//
// tu.UserId = rp.PostId compares a user id with a post id, so it goes through origid. RankedPosts numbers the post x comment rows, so those are materialised.
fn q4299(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, origid, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)));
    let joined: MatSet<(Id<Post>, Option<Id<Comment>>)> = recent().select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    let ct = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let wkey = |p: Id<Post>| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    };
    let v = drain((&joined).map(|(p, _)| p).select(owner_user.opt()));
    let v = ranked(v, |&((p, c), u)| (u, wkey(p), p, c), false);
    let v = per_group(v, |&(_, u)| u);
    let rp = rel(v.into_iter().map(|(((p, _), _), r)| (p, r)).collect());
    let rp_by: HashIdx<i64, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).select(origid).inv().select(&rp).collect();
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt())).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let tr = ranked(drain((&tu).filt(|a| a[0] > 5)), |&(_, a)| (a[1] == 0, Reverse(a[2])), false);
    let tr = rel(tr.into_iter().filter(|x| x.1 <= 10).map(|((u, a), _)| (u, a)).collect());
    type R = (Id<User>, [i64; 3]);
    let v = drain((&tr).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(&db.user.origid).select(&rp_by))));
    let v = top_n(v, |&(_, ((_, a), (p, _)))| (a[1] == 0, Reverse(a[2]), wkey(p)), 100);
    rows(v.into_iter().map(|(_, ((u, a), (p, r)))| {
        let mut f = vec![user_col(db, u, "name"), nullable(a[2], a[1])];
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(ct.get(p).unwrap()), V::S(if r <= 3 { "Top Post" } else { "Other Post" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.PostTypeId = 1),
// AnswerStatistics AS (SELECT p.Id AS QuestionId, COUNT(a.Id) AS TotalAnswers, COALESCE(SUM(a.Score), 0) AS TotalAnswerScore FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id),
// VoteCounts AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id)
// SELECT rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, asw.TotalAnswers, asw.TotalAnswerScore, vc.UpvoteCount, vc.DownvoteCount
// FROM RankedPosts rp JOIN AnswerStatistics asw ON rp.Id = asw.QuestionId JOIN VoteCounts vc ON rp.Id = vc.PostId WHERE rp.rn = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC;
fn q5410(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(tags_str.opt())), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let asw = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).select(&db.post.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&asw).and(&vc)).into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b[0]), V::I(b[1])]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(P.ViewCount), 0) AS TotalViewCount, COALESCE(SUM(P.Score), 0) AS TotalScore,
//        COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, TotalViewCount, TotalScore, PostCount, CommentCount, BadgeCount,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC, TotalScore DESC) AS UserRank FROM UserStatistics)
// SELECT UserId, DisplayName, Reputation, TotalViewCount, TotalScore, PostCount, CommentCount, BadgeCount, UserRank FROM RankedUsers WHERE UserRank <= 10 ORDER BY UserRank;
//
// The order leads with Reputation, so only users whose reputation ranks in the top ten can make the cut; the product is driven for those alone.
fn q8620(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), _)| u).collect()).map(|u| u).collect();
    let Post { score, view_count, .. } = &db.post;
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| match p {
            Some(((s, w), _)) => [a[0] + w.unwrap_or(0), a[1] + s],
            None => a,
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let nc = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let nb = (&top).group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&s).and(&np).and(&nc).and(&nb)), |&(u, (((a, _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, (((a, p), c), b)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(p), V::I(c), V::I(b), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, OwnerName FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.OwnerName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.OwnerName
// ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q5450(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id),
// TopUsers AS (SELECT UserId, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, CommentCount, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserStats)
// SELECT u.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.UpVotes, tu.DownVotes, tu.CommentCount FROM TopUsers tu JOIN Users u ON tu.UserId = u.Id
// WHERE tu.Rank <= 10 ORDER BY tu.Rank;
fn q11786(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((t, (v, c))) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + c.is_some() as i64],
            None => a,
        });
    let v = ranked(drain((&s).and(user_distinct_posts(db))), |&(_, (_, n))| Reverse(n), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), _)| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostsCreated, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostsCreated, Questions, Answers, Upvotes, Downvotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank
//     FROM UserStats WHERE Reputation > 0)
// SELECT TU.Rank, TU.DisplayName, TU.Reputation, TU.PostsCreated, TU.Questions, TU.Answers, TU.Upvotes, TU.Downvotes FROM TopUsers TU WHERE TU.Rank <= 10 ORDER BY TU.Rank;
//
// Rank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q9848(db: &'static So) -> String {
    let v = top_n(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tr = rel(v.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let top: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&tu).and((&s).and(&np))).into_iter().map(|(u, ((_, r), (a, n)))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2) AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CreationDate, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// PostCommentCounts AS (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CreationDate, tp.OwnerDisplayName, COALESCE(pcc.CommentCount, 0) AS CommentCount
// FROM TopPosts tp LEFT JOIN PostCommentCounts pcc ON tp.PostId = pcc.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q5704(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain(&cc).into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner"]);
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2) AND p.Score > 0),
// AggregatedVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, av.UpVotes, av.DownVotes
// FROM RankedPosts rp LEFT JOIN AggregatedVotes av ON rp.PostId = av.PostId WHERE rp.Rank <= 5 ORDER BY rp.PostId, rp.Rank;
fn q8306(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let av = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    rows(drain((&tp).select((&av).opt())).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS WikiCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, WikiCount, UpVoteCount, DownVoteCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStatistics)
// SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, WikiCount, UpVoteCount, DownVoteCount, ReputationRank FROM TopUsers WHERE ReputationRank <= 10 ORDER BY ReputationRank;
//
// ReputationRank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q10274(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let top: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 3 | 4 | 5) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&tu).and((&s).and(&np))).into_iter().map(|(u, ((_, r), (a, n)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalBounty, TotalViews, ROW_NUMBER() OVER (ORDER BY PostCount DESC, TotalViews DESC) AS RN FROM UserPostStats)
// SELECT TU.DisplayName, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.TotalBounty, TU.TotalViews, COALESCE(B.Name, 'No Badge') AS BadgeName
// FROM TopUsers TU LEFT JOIN Badges B ON TU.UserId = B.UserId WHERE TU.RN <= 10 ORDER BY TU.PostCount DESC, TU.TotalViews DESC;
fn q6608(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&db.post.view_count).opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((t, w), b)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + b.flatten().unwrap_or(0), a[4] + w.unwrap_or(0)],
            None => a,
        });
    let v = top_n(drain(&s), |&(u, a)| (Reverse(a[0]), Reverse(a[4]), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    rows(drain((&tu).select((&s).and(badges_of(db).opt()))).into_iter().map(|(u, (a, b))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.push(V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        row(f)
    }))
}

// WITH ProcessedTags AS (SELECT p.Id AS PostId, p.Title, UNNEST(string_to_array(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT pt.Tag, COUNT(DISTINCT pt.PostId) AS QuestionCount, AVG(ur.Reputation) AS AverageReputation, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM ProcessedTags pt JOIN Posts p ON pt.PostId = p.Id JOIN UserReputation ur ON p.OwnerUserId = ur.UserId LEFT JOIN Badges b ON ur.UserId = b.UserId
// WHERE p.CreationDate >= '2023-01-01' AND p.CreationDate < '2024-01-01' GROUP BY pt.Tag ORDER BY QuestionCount DESC, AverageReputation DESC;
fn q25541(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, owner_user, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(creation_date.ge(date(2023, 1, 1))).and(creation_date.lt(date(2024, 1, 1)))).with(owner_user);
    let g = qs()
        .group_by(tags_str.flat_map(tag_list))
        .select(owner_user.select((&db.user.reputation).and(badges_of(db).select(&db.badge.class).opt())))
        .fold([0i64; 5], |a, (r, c)| [a[0] + 1, a[1] + r, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]);
    let nq = qs().group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new()).buf_fold(|xs| distinct_some(xs.into_iter().map(Some)));
    rows(drain((&g).and(&nq)).into_iter().map(|(t, (a, n))| row(vec![V::S(t), V::I(n), avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(b.Id) AS BadgeCount, COUNT(DISTINCT p.Id) AS QuestionCount
//     FROM Users u LEFT JOIN Votes v ON v.UserId = u.Id LEFT JOIN Badges b ON b.UserId = u.Id LEFT JOIN Posts p ON p.OwnerUserId = u.Id AND p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName)
// SELECT us.UserId, us.DisplayName, us.TotalBounty, us.BadgeCount, us.QuestionCount, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount
// FROM UserStats us LEFT JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId AND rp.rn = 1 WHERE us.QuestionCount > 5 AND us.TotalBounty > 0 ORDER BY us.TotalBounty DESC, us.BadgeCount DESC;
fn q1513(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let asked = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()).and(asked().opt()))
        .fold([0i64; 2], |a, ((v, b), _)| [a[0] + v.flatten().unwrap_or(0), a[1] + b.is_some() as i64]);
    let nq = db.user.group_by(Ident::<User>::new()).select(asked().opt()).buf_fold(distinct_some);
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 1, false);
    let fr = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&fr).map(|(u, _)| u).inv().select(&fr).collect();
    let mut v = drain((&s).filt(|a| a[0] > 0).and((&nq).filt(|n| n > 5)).and((&by_user).map(|(_, p)| p).opt()));
    v.sort_by_key(|&(_, ((a, _), _))| (Reverse(a[0]), Reverse(a[1])));
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created", "score", "views"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Title ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, VoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT u.DisplayName, u.Reputation, u.CreationDate AS UserCreationDate, tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, tp.VoteCount
// FROM TopPosts tp JOIN Users u ON tp.PostId IN (SELECT AcceptedAnswerId FROM Posts WHERE AcceptedAnswerId IS NOT NULL) ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// The ON clause names only tp, so it filters tp and crosses the rest with every user.
fn q6288(db: &'static So) -> String {
    let Post { post_type_id, title, score, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(title.opt())), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let accepted: MatSet<Id<Post>> = db.post.select(&db.post.accepted_answer).collect();
    let hit: MatSet<Id<Post>> = (&tp).with(&accepted).collect();
    let nc = (&hit).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(distinct_some);
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let nv = (&hit).group_by(Ident::<Post>::new()).select(up.opt()).buf_fold(distinct_some);
    let tpc = (&nc).and(&nv);
    let mut v = Vec::new();
    (&tpc).cross(db.user.select(Ident::<User>::new())).drive(|(p, u), ((c, n), _)| v.push((p, u, c, n)));
    rows(v.into_iter().map(|(p, u, c, n)| {
        let mut f = ucols(db, u, &["name", "rep", "ucreated"]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(c), V::I(n)]);
        row(f)
    }))
}

// Rewritten (rewrites/12550.sql): the float AVG of epoch seconds became the exact-integer mean.
// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
//        COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews,
//        SUM(epoch_us(P.CreationDate))::DOUBLE / COUNT(P.CreationDate) / 1e6 AS AveragePostAge
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, AveragePostAge, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY TotalViews DESC) AS ViewsRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, AveragePostAge, ScoreRank, ViewsRank FROM TopUsers
// WHERE ScoreRank <= 10 OR ViewsRank <= 10 ORDER BY ScoreRank, ViewsRank;
fn q12550(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(creation_date)).opt())
        .fold(([0i64; 6], 0i128), |(a, e), x| match x {
            Some((((t, s), w), c)) => ([a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)], e + c as i128),
            None => (a, e),
        });
    let v = ranked(drain(&s), |&(_, (a, _))| (a[0] == 0, Reverse(a[3])), false);
    let v = ranked(v, |&((_, (a, _)), _)| (a[4] == 0, Reverse(a[5])), false);
    rows(v.into_iter().filter(|&((_, s), w)| s <= 10 || w <= 10).map(|(((u, (a, e)), s), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[0]), nullable(a[5], a[4])]);
        f.push(if a[0] == 0 { V::Null } else { V::F(e as f64 / a[0] as f64 / 1e6) });
        f.extend([V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH TagStats AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore
//     FROM Tags T LEFT JOIN Posts P ON P.Tags ILIKE '%' || T.TagName || '%' GROUP BY T.TagName),
// TopTags AS (SELECT TagName, PostCount, QuestionCount, AnswerCount, TotalViews, TotalScore, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM TagStats WHERE PostCount > 0)
// SELECT T.TagName, T.PostCount, T.QuestionCount, T.AnswerCount, T.TotalViews, T.TotalScore,
//        CONCAT('Tag: ', T.TagName, ' | Posts: ', T.PostCount, ' | Questions: ', T.QuestionCount, ' | Answers: ', T.AnswerCount, ' | Views: ', T.TotalViews, ' | Score: ', T.TotalScore) AS TagSummary
// FROM TopTags T WHERE Rank <= 10 ORDER BY T.Rank;
//
// ILIKE: the substring test is made on lowercased strings, through the distinct tag strings as in `tag_mentions`.
fn q29517(db: &'static So) -> String {
    let elems: MatSet<Str> = (&db.post.tags_str).flat_map(tag_list).collect();
    let contains: HashIdx<Str, Id<Tag>> = (&elems).select_where((&db.tag.tag_name).inv(), |e: Str, n: Str| e.to_lowercase().contains(&n.to_lowercase())).collect();
    let lt: MatSet<(Id<Post>, Id<Tag>)> = db.post.select(Ident::<Post>::new().and((&db.post.tags_str).flat_map(tag_list).select(&contains))).collect();
    let by_name: HashIdx<Str, Id<Post>> = (&lt).map(|(_, t)| t).select(&db.tag.tag_name).inv().select((&lt).map(|(p, _)| p)).collect();
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let g = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&db.tag.tag_name).select(&by_name).select(post_type_id.and(view_count.opt()).and(score)).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((t, w), s)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s],
            None => a,
        });
    let np = db.tag.group_by(&db.tag.tag_name).select((&db.tag.tag_name).select(&by_name).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&g).and((&np).filt(|n| n > 0))), |&(t, (_, n))| (Reverse(n), t), 10);
    rows(v.into_iter().map(|(t, (a, n))| {
        let views = if a[2] == 0 { String::new() } else { a[3].to_string() };
        let sum = format!("Tag: {t} | Posts: {n} | Questions: {} | Answers: {} | Views: {views} | Score: {}", a[0], a[1], a[4]);
        row(vec![V::S(t), V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[4]), V::Owned(sum)])
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScores,
//        SUM(CASE WHEN P.ViewCount > 0 THEN 1 ELSE 0 END) AS PostsWithViews, AVG(P.ViewCount) AS AvgViewCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, PositiveScores, PostsWithViews, AvgViewCount, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserActivity)
// SELECT TU.DisplayName, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.PositiveScores, TU.PostsWithViews, TU.AvgViewCount,
//        (SELECT COUNT(*) FROM Badges B WHERE B.UserId = TU.UserId) AS BadgeCount
// FROM TopUsers TU WHERE TU.PostRank <= 10 ORDER BY TU.TotalPosts DESC;
fn q5908(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 7], |a, x| match x {
            Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + w.map_or(false, |w| w > 0) as i64, a[5] + w.is_some() as i64, a[6] + w.unwrap_or(0)],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&s).and(&bc)).into_iter().map(|(u, (a, b))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[6], a[5]), V::I(b)])))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT b.Id) AS BadgeCount,
//        SUM(COALESCE(v.BountyAmount, 0)) AS TotalBountyAmount, SUM(COALESCE(v.UserId, 0)) AS TotalVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, BadgeCount, TotalBountyAmount, TotalVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY PostCount DESC) AS PostCountRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, BadgeCount, TotalBountyAmount, TotalVotes, ReputationRank, PostCountRank FROM TopUsers
// WHERE ReputationRank <= 10 OR PostCountRank <= 10 ORDER BY ReputationRank, PostCountRank;
fn q14433(db: &'static So) -> String {
    let Vote { bounty_amount, user_id, .. } = &db.vote;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(bounty_amount.opt().and(user_id.opt())).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| match p.flatten() {
            Some((b, u)) => [a[0] + b.unwrap_or(0), a[1] + u.unwrap_or(0)],
            None => a,
        });
    let nb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(user_distinct_posts(db)).and(&nb)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = ranked(v, |&((_, ((_, n), _)), _)| Reverse(n), false);
    rows(v.into_iter().filter(|&((_, r), p)| r <= 10 || p <= 10).map(|(((u, ((a, n), b)), r), p)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(b), V::I(a[0]), V::I(a[1]), V::I(r), V::I(p)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT T.DisplayName, T.Reputation, T.PostCount, T.QuestionCount, T.AnswerCount, T.UpVotes, T.DownVotes, (T.UpVotes - T.DownVotes) AS NetVotes FROM TopUsers T
// WHERE T.Rank <= 10 ORDER BY NetVotes DESC;
//
// Rank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q6530(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&s).and(&np)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(a[2] - a[3]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 1 AND PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS ClosedQuestionCount,
//        COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostHistory PH ON P.Id = PH.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, ClosedQuestionCount, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, ClosedQuestionCount, BadgeCount, ReputationRank FROM TopUsers WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
//
// ReputationRank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q5763(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let top: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(history_of(db).select(&db.post_history.post_history_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, _)| match p {
            Some((t, h)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 1 && h == Some(10)) as i64],
            None => a,
        });
    let nb = (&top).group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&tu).and((&s).and(&nb))).into_iter().map(|(u, ((_, r), (a, b)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionCount,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStatistics)
// SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, BadgeCount, ReputationRank FROM TopUsers WHERE ReputationRank <= 10 ORDER BY Reputation DESC, Upvotes DESC;
//
// ReputationRank reads only Users, so the top users are picked first and the product is driven for those alone.
fn q9411(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let top: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| {
            let t = p.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let of = |t: i64| (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(t))).opt()).buf_fold(distinct_some);
    let (nq, na) = (of(1), of(2));
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(distinct_some);
    let nb = (&top).group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    rows(drain((&tu).and((&s).and(&np).and(&nq).and(&na).and(&nb))).into_iter().map(|(u, ((_, r), ((((a, p), q), n), b)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(p), V::I(q), V::I(n), V::I(a[0]), V::I(a[1]), V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews, AVG(P.AnswerCount) AS AvgAnswerCount,
//        AVG(P.CommentCount) AS AvgCommentCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, AvgAnswerCount, AvgCommentCount, RANK() OVER (ORDER BY TotalScore DESC) AS RankByScore
//     FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, AvgAnswerCount, AvgCommentCount, RankByScore FROM TopUsers WHERE RankByScore <= 10 ORDER BY RankByScore;
fn q14981(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, answer_count, comment_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(answer_count.opt()).and(comment_count)).opt())
        .fold([0i64; 9], |a, x| match x {
            Some(((((t, s), w), an), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + an.is_some() as i64, a[7] + an.unwrap_or(0), a[8] + c],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| (a[0] == 0, Reverse(a[3])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[0]), nullable(a[5], a[4]), avg(a[7], a[6]), avg(a[8], a[0]), V::I(r)]);
        row(f)
    }))
}

// WITH RECURSIVE UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(b.Class, 0)) AS TotalBadges,
//        ROW_NUMBER() OVER (ORDER BY COUNT(p.Id) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// FilteredStats AS (SELECT UserId, DisplayName, PostCount, TotalScore, TotalBadges, Rank FROM UserPostStats WHERE PostCount > 5),
// UserAverageScore AS (SELECT UserId, AVG(TotalScore) AS AvgScore FROM FilteredStats GROUP BY UserId)
// SELECT fs.DisplayName, fs.PostCount, fs.TotalScore, fs.TotalBadges, uas.AvgScore,
//        CASE WHEN fs.TotalScore > uas.AvgScore THEN 'Above Average' WHEN fs.TotalScore < uas.AvgScore THEN 'Below Average' ELSE 'Average' END AS ScoreComparison
// FROM FilteredStats fs JOIN UserAverageScore uas ON fs.UserId = uas.UserId ORDER BY fs.PostCount DESC, fs.TotalScore DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. Rank is never read.
fn q31678(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.score).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (s, c)| [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0), a[2] + c.unwrap_or(0)]);
    let fs = (&s).filt(|a| a[0] > 5);
    let uas = db.user.group_by(Ident::<User>::new()).select(&fs).fold([0i64; 2], |m, a| [m[0] + a[1], m[1] + 1]);
    rows(drain((&fs).and(&uas)).into_iter().map(|(u, (a, m))| {
        let (t, n) = (a[1] as i128 * m[1] as i128, m[0] as i128);
        let cmp = if t > n { "Above Average" } else if t < n { "Below Average" } else { "Average" };
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(m[0], m[1]), V::S(cmp)])
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, AVG(U.Reputation) AS AverageReputation
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotes, TotalDownvotes, AverageReputation, RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts
//     FROM UserStatistics)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotes, TotalDownvotes, AverageReputation FROM TopUsers WHERE RankByPosts <= 10;
fn q11050(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 6], |a, (r, x)| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + 1, a[5] + r],
            None => [a[0], a[1], a[2], a[3], a[4] + 1, a[5] + r],
        });
    let v = ranked(drain((&s).and(user_distinct_posts(db))), |&(_, (_, n))| Reverse(n), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, MAX(p.LastActivityDate) AS LastActive FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p)
// SELECT ua.DisplayName, ua.TotalPosts, ua.QuestionCount, ua.AnswerCount, ua.LastActive, rp.Title, rp.ViewCount, rp.CreationDate
// FROM UserActivity ua LEFT JOIN RecentPosts rp ON ua.UserId = rp.OwnerUserId AND rp.rn = 1
// WHERE ua.TotalPosts > 5 AND ua.LastActive >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND (SELECT COUNT(*) FROM Votes v WHERE v.UserId = ua.UserId AND v.VoteTypeId = 2) > 0
// ORDER BY ua.LastActive DESC FETCH FIRST 20 ROWS ONLY;
fn q2658(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, last_activity_date, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(last_activity_date)).opt()).fold([0, 0, 0, i64::MIN], |a, x| match x {
        Some((t, d)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(d)],
        None => a,
    });
    let voter: MatSet<Id<User>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.user).collect();
    let first = top_per(drain(db.post.select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let fr = rel(first.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&fr).map(|(u, _)| u).inv().select(&fr).collect();
    let since = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let v = drain(db.user.with(&voter).select((&ua).filt(move |a| a[0] > 5 && a[3] >= since).and((&by_user).map(|(_, p)| p).opt())));
    let v = top_n(v, |&(_, (a, _))| Reverse(a[3]), 20);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::T(a[3])];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "views", "created"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, AVG(COALESCE(p.Score, 0)) AS AverageScore,
//        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, AverageScore, TotalViews, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, AverageScore, TotalViews, ScoreRank, PostRank FROM TopUsers
// WHERE ScoreRank <= 10 OR PostRank <= 10 ORDER BY TotalScore DESC, PostCount DESC;
fn q14726(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| Reverse(a[4]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[1]), false);
    rows(v.into_iter().filter(|&((_, s), p)| s <= 10 || p <= 10).map(|(((u, a), s), p)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[4], a[0]), V::I(a[6]), V::I(s), V::I(p)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.CommentCount, 0)) AS TotalComments, SUM(COALESCE(b.Class, 0)) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionCount, AnswerCount, TotalScore, TotalViews, TotalComments, TotalBadges,
//        RANK() OVER (ORDER BY TotalScore DESC, TotalViews DESC, TotalComments DESC) AS UserRank FROM UserPostStats)
// SELECT tu.DisplayName, tu.QuestionCount, tu.AnswerCount, tu.TotalScore, tu.TotalViews, tu.TotalComments, tu.TotalBadges FROM TopUsers tu WHERE tu.UserRank <= 10 ORDER BY tu.UserRank;
fn q5056(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, comment_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comment_count)).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, c)| {
            let c = c.unwrap_or(0);
            match p {
                Some((((t, s), w), cc)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.unwrap_or(0), a[4] + cc, a[5] + c],
                None => [a[0], a[1], a[2], a[3], a[4], a[5] + c],
            }
        });
    let v = ranked(drain(&s), |&(_, a)| (Reverse(a[2]), Reverse(a[3]), Reverse(a[4])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT B.Id) AS BadgeCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, UpVotes, DownVotes, QuestionCount, AnswerCount, RANK() OVER (ORDER BY Reputation DESC, BadgeCount DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, BadgeCount, UpVotes, DownVotes, QuestionCount, AnswerCount, Rank FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// The order leads with Reputation, so only users whose reputation ranks in the top ten can make the cut; the product is driven for those alone.
fn q6840(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), _)| u).collect()).map(|u| u).collect();
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()).and(posts_of(db).select(&db.post.post_type_id).opt()))
        .fold([0i64; 4], |a, ((_, v), t)| [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (t == Some(1)) as i64, a[3] + (t == Some(2)) as i64]);
    let nb = (&top).group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(&nb)), |&(u, (_, b))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(b)), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, b)), r)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(b));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10216", q10216),
    ("1288", q1288),
    ("12071", q12071),
    ("11222", q11222),
    ("12013", q12013),
    ("13444", q13444),
    ("8133", q8133),
    ("10155", q10155),
    ("10238", q10238),
    ("10619", q10619),
    ("10911", q10911),
    ("9208", q9208),
    ("13195", q13195),
    ("12574", q12574),
    ("10586", q10586),
    ("13828", q13828),
    ("10923", q10923),
    ("6553", q6553),
    ("691", q691),
    ("9971", q9971),
    ("10057", q10057),
    ("6691", q6691),
    ("6755", q6755),
    ("7786", q7786),
    ("10457", q10457),
    ("10837", q10837),
    ("14119", q14119),
    ("3131", q3131),
    ("11309", q11309),
    ("11441", q11441),
    ("10381", q10381),
    ("13982", q13982),
    ("10777", q10777),
    ("8514", q8514),
    ("8370", q8370),
    ("7434", q7434),
    ("9950", q9950),
    ("9154", q9154),
    ("26476", q26476),
    ("9138", q9138),
    ("14950", q14950),
    ("14775", q14775),
    ("6326", q6326),
    ("7472", q7472),
    ("5150", q5150),
    ("6245", q6245),
    ("11464", q11464),
    ("27972", q27972),
    ("6649", q6649),
    ("7815", q7815),
    ("13363", q13363),
    ("26838", q26838),
    ("472", q472),
    ("8565", q8565),
    ("11263", q11263),
    ("5788", q5788),
    ("6773", q6773),
    ("10944", q10944),
    ("5388", q5388),
    ("79", q79),
    ("25284", q25284),
    ("9369", q9369),
    ("10170", q10170),
    ("10753", q10753),
    ("12510", q12510),
    ("5467", q5467),
    ("5228", q5228),
    ("9505", q9505),
    ("8111", q8111),
    ("5313", q5313),
    ("13395", q13395),
    ("6913", q6913),
    ("2028", q2028),
    ("6220", q6220),
    ("7069", q7069),
    ("13803", q13803),
    ("26483", q26483),
    ("4299", q4299),
    ("5410", q5410),
    ("8620", q8620),
    ("5450", q5450),
    ("11786", q11786),
    ("9848", q9848),
    ("5704", q5704),
    ("8306", q8306),
    ("10274", q10274),
    ("6608", q6608),
    ("25541", q25541),
    ("1513", q1513),
    ("6288", q6288),
    ("12550", q12550),
    ("29517", q29517),
    ("5908", q5908),
    ("14433", q14433),
    ("6530", q6530),
    ("5763", q5763),
    ("9411", q9411),
    ("14981", q14981),
    ("31678", q31678),
    ("11050", q11050),
    ("2658", q2658),
    ("14726", q14726),
    ("5056", q5056),
    ("6840", q6840),
];
