use harness::prelude::*;
use std::cmp::Reverse;

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalUpvotes, TotalDownvotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalUpvotes, TotalDownvotes, ReputationRank
// FROM TopUsers WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the post x vote product is driven for those alone.
fn q13439(db: &'static So) -> String {
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let top: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let tops: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = (&tops).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&s).and(&pc).and((&top).map(|(_, r)| r)));
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, p.PostTypeId
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month')
// SELECT rp.OwnerDisplayName, rp.OwnerReputation, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, COALESCE(PH.EditCount, 0) AS EditCount, PT.Name AS PostTypeName
// FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS EditCount FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6) GROUP BY PostId) PH ON rp.PostId = PH.PostId
// JOIN PostTypes PT ON rp.PostTypeId = PT.Id WHERE rp.Rank <= 5 ORDER BY PT.Name, rp.Rank;
fn q8069(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])));
    let ec = (&tp).group_by(Ident::<Post>::new()).select(edits.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    rows(drain(&ec).into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["owner", "rep", "title", "score", "views", "answers", "comments"]);
        f.extend([V::I(n), V::S(ptype_name(db).get(p).unwrap())]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
// GROUP BY tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q6429(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, PositiveVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation)
// SELECT TU.DisplayName, TU.Reputation, TU.QuestionCount, TU.AnswerCount, TU.PositiveVotes,
//        (SELECT COUNT(*) FROM Votes V WHERE V.UserId = TU.UserId AND V.VoteTypeId = 2) AS UpVotesGiven,
//        (SELECT COUNT(*) FROM Votes V WHERE V.UserId = TU.UserId AND V.VoteTypeId = 3) AS DownVotesGiven,
//        (SELECT COUNT(*) FROM Badges B WHERE B.UserId = TU.UserId) AS BadgeCount
// FROM TopUsers TU WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC;
fn q28468(db: &'static So) -> String {
    let tu: MatSet<Id<User>> = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, score, .. } = &db.post;
    let ps = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (s > 0) as i64],
        None => a,
    });
    let vs = (&tu).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&ps).and(&vs).and(&bc)).into_iter().map(|(u, ((a, w), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(w[0]), V::I(w[1]), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS rn, pt.Name AS PostTypeName, COALESCE(ph.Rolls, 0) AS RollbackCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN (SELECT PostId, COUNT(*) AS Rolls FROM PostHistory WHERE PostHistoryTypeId IN (7, 9) GROUP BY PostId) ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, rp.PostTypeName, rp.RollbackCount
// FROM RankedPosts rp WHERE rp.rn = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 50;
fn q8382(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rolls = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([7, 9])));
    let rc = (&tp).group_by(Ident::<Post>::new()).select(rolls.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = top_n(drain(&rc), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "type"]);
        f.push(V::I(n));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        AVG(COALESCE(p.Score, 0)) AS AvgScorePerPost, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewsPerPost, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, TotalViews, AvgScorePerPost, AvgViewsPerPost, CommentCount,
//        RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, TotalScore, TotalViews, AvgScorePerPost, AvgViewsPerPost, CommentCount, ScoreRank, ViewRank
// FROM TopUsers WHERE ScoreRank <= 10 OR ViewRank <= 10 ORDER BY ScoreRank, ViewRank;
fn q14925(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some(((s, w), _)) => [a[0] + 1, a[1] + 1, a[2] + s, a[3] + w.unwrap_or(0)],
            None => [a[0] + 1, a[1], a[2], a[3]],
        });
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&s).and(&cc)), |&(_, (a, _))| Reverse(a[2]), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[3]), false);
    let mut v: Vec<_> = v.into_iter().filter(|&((_, s), w)| s <= 10 || w <= 10).collect();
    v.sort_by_key(|&((_, s), w)| (s, w));
    rows(v.into_iter().map(|(((u, (a, c)), s), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[2], a[0]), avg(a[3], a[0]), V::I(c), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, VoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, u.DisplayName AS Author, COALESCE(b.Name, 'No Badges') AS BadgeName
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.AccountId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 ORDER BY tp.Score DESC;
//
// Rank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
fn q9588(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let acct: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.account_id).inv().collect();
    let v = drain((&cc).and(origid.select(&acct).select(Ident::<User>::new().and(badges_of(db).select(&db.badge.name).opt()))));
    rows(v.into_iter().map(|(p, (n, (u, b)))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(n), user_col(db, u, "name"), V::S(b.unwrap_or("No Badges"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Owner, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON a.ParentId = p.Id LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// PostRank AS (SELECT *, RANK() OVER (ORDER BY CreationDate DESC) AS PostRank FROM RankedPosts)
// SELECT pr.PostId, pr.Title, pr.CreationDate, pr.Owner, pr.CommentCount, pr.AnswerCount, pr.UpVotes - pr.DownVotes AS Score, pr.PostRank
// FROM PostRank pr WHERE pr.rn = 1 AND pr.PostRank <= 10 ORDER BY pr.PostRank;
//
// rn partitions by p.Id, so it is always 1. PostRank reads only CreationDate, so the newest questions are picked first
// and the comment x answer x vote product is driven for those alone.
fn q6922(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(_, d)| Reverse(d), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    let rk: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let tp: MatSet<Id<Post>> = (&tr).map(|(p, _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold(0i64, |n, (_, t)| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&s).and(&cc).and(&ac).and((&rk).map(|(_, r)| r))).into_iter().map(|(p, (((s, c), a), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(a), V::I(s), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT a.Id) AS AnswerCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        PERCENT_RANK() OVER (ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT rp.*, ROW_NUMBER() OVER (ORDER BY rp.RankScore DESC) AS RowNum FROM RankedPosts rp WHERE rp.RankScore > 0.5)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.RankScore, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
// FROM TopPosts tp JOIN Users u ON tp.PostId IN (SELECT PostId FROM Votes WHERE VoteTypeId = 2) WHERE tp.RowNum <= 10 ORDER BY tp.RankScore DESC;
//
// The ON clause names only tp, so the upvoted top posts are crossed with every user.
fn q9326(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(score)), |&(_, s)| Reverse(s), false);
    let n = v.len() as f64;
    let v: Vec<_> = v.into_iter().map(|((p, _), r)| (p, if n > 1.0 { (r - 1) as f64 / (n - 1.0) } else { 0.0 })).filter(|&(_, r)| r > 0.5).collect();
    let v = top_n(v, |&(p, r)| (Reverse(fkey(r)), p), 10);
    let tr = rel(v);
    let rs: HashIdx<Id<Post>, (Id<Post>, f64)> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let tp: MatSet<Id<Post>> = (&tr).map(|(p, _)| p).collect();
    let up: MatSet<Id<Post>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.post).collect();
    let ac = (&tp).with(&up).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let cc = (&tp).with(&up).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pr: HashIdx<Id<Post>, ((i64, i64), (Id<Post>, f64))> = (&ac).and(&cc).and(&rs).collect();
    let mut v = drain((&pr).cross(db.user.select(Ident::<User>::new())));
    v.sort_by_key(|&(_, ((_, (_, r)), _))| Reverse(fkey(r)));
    rows(v.into_iter().map(|((p, _), (((a, c), (_, r)), u))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(a), V::I(c), V::F(r)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, us.QuestionCount, us.AnswerCount, us.UpVoteCount, us.DownVoteCount,
//        RANK() OVER (ORDER BY us.Reputation DESC) AS Rank FROM UserStatistics us)
// SELECT tu.Rank, tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.UpVoteCount, tu.DownVoteCount
// FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// Rank reads only Reputation, so the top users are picked first and the post x vote product is driven for those alone.
fn q8552(db: &'static So) -> String {
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let top: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let tops: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = (&tops).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&s).and(&pc).and((&top).map(|(_, r)| r)));
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation)
// SELECT TU.Rank, TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.Upvotes, TU.Downvotes, (TU.Upvotes - TU.Downvotes) AS NetVotes
// FROM TopUsers TU WHERE TU.Rank <= 10 ORDER BY NetVotes DESC;
//
// Rank reads only Reputation, so the top users are picked first and the post x vote product is driven for those alone.
fn q6213(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel((0..tu.len()).map(|i| (tu[i].0, i as i64 + 1)).collect());
    let top: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let tops: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = (&tops).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&s).and(&pc).and((&top).map(|(_, r)| r)));
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(a[2] - a[3]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS HistoryCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, p.LastActivityDate),
// TopPosts AS (SELECT PostId, Title, CreationDate, LastActivityDate, Upvotes, Downvotes, CommentCount, HistoryCount,
//        ROW_NUMBER() OVER (ORDER BY (Upvotes - Downvotes) DESC, HistoryCount DESC, LastActivityDate DESC) AS Rank FROM RankedPosts)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.LastActivityDate, tp.Upvotes, tp.Downvotes, tp.CommentCount, tp.HistoryCount
// FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Rank;
fn q6074(db: &'static So) -> String {
    let s = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = db.post.group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&s).and(&cc).and(&hc)), |&(p, ((a, _), h))| (Reverse(a[0] - a[1]), Reverse(h), Reverse(db.post.last_activity_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, ((a, c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(h)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, CommentCount, UpvoteCount, DownvoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT t.Title, t.CreationDate, t.Score, t.CommentCount, t.UpvoteCount, t.DownvoteCount, u.DisplayName AS OwnerDisplayName, b.Name AS BadgeName
// FROM TopPosts t JOIN Users u ON t.PostId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId WHERE b.Class = 1 OR b.Class = 2 ORDER BY t.Score DESC, t.CommentCount DESC;
//
// Rank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
// t.PostId = u.Id compares a post id with a user id, so it joins through origid.
fn q8868(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let gs = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).is_in([1, 2]))).select(&db.badge.name);
    let v = drain((&s).and(origid.select(&uid).select(Ident::<User>::new().and(gs))));
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend(a.map(V::I));
        f.extend([user_col(db, u, "name"), V::S(b)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, SUM(cb.Class) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges cb ON u.Id = cb.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, UpVoteCount, DownVoteCount, BadgeCount,
//        RANK() OVER (ORDER BY PostCount DESC, UpVoteCount DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, UpVoteCount, DownVoteCount, BadgeCount FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q13009(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0)]
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&s).and(&pc)), |&(_, (a, n))| (Reverse(n), Reverse(a[2])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4])]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(DISTINCT b.Id) AS BadgeCount, MAX(u.Reputation) AS Reputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, BadgeCount, Reputation,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStatistics)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, BadgeCount, Reputation FROM TopUsers WHERE Rank <= 10 ORDER BY Reputation DESC;
//
// Rank reads only Reputation, so the top users are picked first and the post x vote x badge product is driven for those alone.
fn q5750(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tops: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, _)| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let bc = (&tops).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&s).and(&bc)).into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(b), user_col(db, u, "rep")]);
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName),
// PopularPosts AS (SELECT PostId, Title, Score, ViewCount, AnswerCount, CommentCount, UpVoteCount, DownVoteCount, RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostMetrics)
// SELECT PostId, Title, Score, ViewCount, AnswerCount, CommentCount, UpVoteCount, DownVoteCount, Rank FROM PopularPosts WHERE Rank <= 10;
//
// Rank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
fn q10069(db: &'static So) -> String {
    let view_count = &db.post.view_count;
    let v = ranked(drain(&db.post.score), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    let rk: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let tp: MatSet<Id<Post>> = (&tr).map(|(p, _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&s).and((&rk).map(|(_, r)| r))).into_iter().map(|(p, (a, r))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(V.BountyAmount) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalAnswers, TotalQuestions, TotalBounty, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank,
//        RANK() OVER (ORDER BY TotalAnswers DESC) AS AnswerRank, RANK() OVER (ORDER BY TotalBounty DESC) AS BountyRank FROM UserStats),
// FilteredUsers AS (SELECT * FROM TopUsers WHERE PostRank <= 10 OR AnswerRank <= 10 OR BountyRank <= 10)
// SELECT DisplayName, TotalPosts, TotalAnswers, TotalQuestions, TotalBounty, PostRank, AnswerRank, BountyRank FROM FilteredUsers ORDER BY PostRank, AnswerRank, BountyRank;
fn q6503(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, b)) => {
                let b = b.flatten();
                [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
            }
            None => a,
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&s).and(&pc)), |&(_, (_, n))| Reverse(n), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[0]), false);
    let v = ranked(v, |&(((_, (a, _)), _), _)| (a[2] == 0, Reverse(a[3])), false);
    let mut v: Vec<_> = v.into_iter().filter(|&(((_, p), a), b)| p <= 10 || a <= 10 || b <= 10).collect();
    v.sort_by_key(|&(((_, p), a), b)| (p, a, b));
    rows(v.into_iter().map(|((((u, (a, n)), p), r), b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(p), V::I(r), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, Score, ViewCount FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM TopPosts tp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON tp.PostId = c.PostId
// LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) = b.UserId
// ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q5332(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&cc).and(owner_user.select(&bc).opt())).into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.extend([V::I(c), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, BadgeCount, ReputationRank
// FROM TopUsers WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the post x vote x badge product is driven for those alone.
fn q6102(db: &'static So) -> String {
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let top: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let tops: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let pc = (&tops).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&s).and(&pc).and((&top).map(|(_, r)| r))).into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AverageViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, AverageViews, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank
//     FROM UserPostStats WHERE PostCount > 0)
// SELECT t.DisplayName, t.PostCount, t.QuestionCount, t.AnswerCount, t.TotalScore, t.AverageViews, COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM TopUsers t LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON t.UserId = b.UserId WHERE t.ScoreRank <= 10
// ORDER BY t.TotalScore DESC, t.PostCount DESC;
fn q8056(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = ranked(drain((&ups).filt(|a| a[1] > 0)), |&(_, a)| Reverse(a[4]), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&bc).and(&ups));
    v.sort_by_key(|&(_, (_, a))| (Reverse(a[4]), Reverse(a[1])));
    rows(v.into_iter().map(|(u, (b, a))| row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[6], a[5]), V::I(b)])))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopPostStats AS (SELECT ps.UserId, SUM(ps.TotalPosts) AS TotalPosts, SUM(ps.TotalQuestions) AS TotalQuestions, SUM(ps.TotalAnswers) AS TotalAnswers,
//        SUM(ps.TotalViews) AS TotalViews, SUM(ps.TotalScore) AS TotalScore FROM UserPostStats ps GROUP BY ps.UserId)
// SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalViews, ups.TotalScore, RANK() OVER (ORDER BY ups.TotalScore DESC) AS ScoreRank
// FROM UserPostStats ups ORDER BY ups.TotalScore DESC LIMIT 10;
//
// TopPostStats is never referenced by the final SELECT.
fn q11127(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = ranked(drain(&ups), |&(_, a)| Reverse(a[4]), false);
    rows(v.into_iter().take(10).map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT Posts.Id AS PostId, Posts.Title, Posts.CreationDate, Posts.ViewCount, COUNT(Comments.Id) AS CommentCount,
//        SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        RANK() OVER (ORDER BY Posts.CreationDate DESC) AS Rank
//     FROM Posts LEFT JOIN Comments ON Posts.Id = Comments.PostId LEFT JOIN Votes ON Posts.Id = Votes.PostId
//     WHERE Posts.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY Posts.Id, Posts.Title, Posts.CreationDate, Posts.ViewCount),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, CommentCount, Upvotes, Downvotes FROM RankedPosts WHERE Rank <= 10)
// SELECT U.DisplayName, U.Reputation, T.Title, T.CreationDate, T.ViewCount, T.CommentCount, T.Upvotes, T.Downvotes
// FROM TopPosts T JOIN Users U ON T.PostId = U.Id ORDER BY T.Upvotes DESC, T.ViewCount DESC;
//
// Rank reads only CreationDate, so the newest posts are picked first and the comment x vote product is driven for those alone.
// T.PostId = U.Id compares a post id with a user id, so it joins through origid.
fn q8299(db: &'static So) -> String {
    let Post { creation_date, origid, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(creation_date)), |&(_, d)| Reverse(d), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    rows(drain((&s).and(origid.select(&uid))).into_iter().map(|(p, (a, u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, p.ViewCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(b.Class), 0) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.AnswerCount, rp.ViewCount, rp.CommentCount, ur.Reputation, ur.TotalBadges
// FROM RankedPosts rp JOIN UserReputation ur ON rp.PostId IN (SELECT b.UserId FROM Badges b WHERE b.Date >= cast('2024-10-01' as date) - INTERVAL '6 months')
// WHERE rp.PostRank = 1 AND (ur.Reputation > 1000 OR ur.TotalBadges > 5) ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 50;
//
// RankedPosts has one row per (post, comment); every row of a post shares its score, so PostRank = 1 keeps all rows of each owner's
// top-scoring posts. The ON clause names only rp (a post id tested against badge user ids), so those rows are crossed with UserReputation.
fn q1443(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, origid, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let recent: MatSet<i64> = db.badge.with((&db.badge.date).ge(add_months(date(2024, 10, 1), -6))).select(&db.badge.user_id).collect();
    let cc = (&tp).with(origid.with(&recent)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rp: MatSet<(Id<Post>, Option<Id<Comment>>)> = (&tp).with(origid.with(&recent)).select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt())
        .fold(0i64, |s, c| s + c.unwrap_or(0));
    let urf = rel(drain((&ur).and(&db.user.reputation).filt(|(b, r)| r > 1000 || b > 5)));
    let mut v = Vec::new();
    (&rp).cross(&urf).drive(|_, ((p, _), (_, (b, r)))| v.push((p, b, r)));
    let v = top_n(v, |&(p, _, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 50);
    rows(v.into_iter().map(|(p, b, r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "answers", "views"]);
        f.extend([V::I(cc.get(p).unwrap()), V::I(r), V::I(b)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS NumberOfPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(V.BountyAmount) AS TotalBountyEarned, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) WHERE U.Reputation > 100 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, NumberOfPosts, AnswerCount, QuestionCount, TotalBountyEarned, LastPostDate, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, NumberOfPosts, AnswerCount, QuestionCount, TotalBountyEarned, LastPostDate FROM TopUsers WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the post x vote product is driven for those alone.
fn q9037(db: &'static So) -> String {
    let tu = ranked(drain(db.user.with((&db.user.reputation).gt(100)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tops: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.creation_date).and(bounty.opt())).opt())
        .fold([0, 0, 0, 0, i64::MIN], |a, x| match x {
            Some(((t, d), b)) => {
                let b = b.flatten();
                [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0), a[4].max(d)]
            }
            None => a,
        });
    let pc = (&tops).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&s).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), tmax(a[4])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// CommentsDetails AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MIN(c.CreationDate) AS FirstCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COALESCE(cd.CommentCount, 0) AS CommentCount, cd.FirstCommentDate
// FROM TopPosts tp LEFT JOIN CommentsDetails cd ON tp.PostId = cd.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q9764(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cd = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MAX), |(n, m), d| match d {
        Some(d) => (n + 1, m.min(d)),
        None => (n, m),
    });
    rows(drain(&cd).into_iter().map(|(p, (n, m))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(n), tmin(m)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, COALESCE(SUM(p.Score), 0) AS TotalScore, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(p.CommentCount), 0) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ub.UserId, ub.BadgeCount, ups.DisplayName, ups.PostCount, ups.TotalScore, ups.QuestionCount, ups.AnswerCount, ups.TotalComments,
//        RANK() OVER (ORDER BY ub.BadgeCount DESC, ups.TotalScore DESC) AS Rank FROM UserBadgeCounts ub JOIN UserPostStats ups ON ub.UserId = ups.UserId)
// SELECT Rank, DisplayName, PostCount, TotalScore, QuestionCount, AnswerCount, TotalComments, BadgeCount FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q7111(db: &'static So) -> String {
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let Post { score, post_type_id, comment_count, .. } = &db.post;
    let ups = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(post_type_id).and(comment_count)).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((s, t), c)) => [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + c],
            None => a,
        });
    let v = ranked(drain((&bc).and(&ups)), |&(_, (b, a))| (Reverse(b), Reverse(a[1])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (b, a)), r)| {
        let mut f = vec![V::I(r), user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        RANK() OVER (ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.Rank, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
// FROM RankedPosts rp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) WHERE rp.Rank <= 10 ORDER BY rp.Rank;
fn q5244(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[1] - a[2]), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).collect());
    let v = drain((&tr).select(Same::<((Id<Post>, [i64; 3]), i64)>::new().and(Same::<((Id<Post>, [i64; 3]), i64)>::new().map(|((p, _), _): ((Id<Post>, [i64; 3]), i64)| p).select(owner_user))));
    rows(v.into_iter().map(|(_, (((p, a), r), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE 0 END) AS TotalQuestionScore,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN p.Score ELSE 0 END) AS TotalAnswerScore, u.Reputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalQuestions, ua.TotalAnswers, ua.TotalQuestionScore, ua.TotalAnswerScore, ua.Reputation,
//        RANK() OVER (ORDER BY ua.Reputation DESC) AS UserRank FROM UserActivity ua)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalQuestionScore, TotalAnswerScore, Reputation, UserRank FROM RankedUsers WHERE UserRank <= 10 ORDER BY UserRank;
fn q14983(db: &'static So) -> String {
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let top: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let Post { post_type_id, score, .. } = &db.post;
    let s = (&top)
        .map(|(u, _)| u)
        .group_by(Same::<Id<User>>::new())
        .select(posts_of(db).select(post_type_id.and(score)).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { s } else { 0 }, a[4] + if t == 2 { s } else { 0 }],
            None => a,
        });
    rows(drain((&s).and((&top).map(|(_, r)| r))).into_iter().map(|(u, (a, r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([user_col(db, u, "rep"), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT *, RANK() OVER (ORDER BY ViewCount DESC, UpvoteCount DESC) AS OverallRank FROM RankedPosts)
// SELECT PostId, Title, ViewCount, CreationDate, OwnerDisplayName, CommentCount, UpvoteCount, DownvoteCount, OverallRank FROM TopPosts WHERE OverallRank <= 10 ORDER BY OverallRank;
fn q7183(db: &'static So) -> String {
    let Post { creation_date, view_count, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = ranked(drain(&s), |&(p, a)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(a[1]))
    }, false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, a), r)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.UpVotes, tp.DownVotes, (tp.UpVotes - tp.DownVotes) AS Score, bg.Name AS BadgeName, u.DisplayName AS UserName, u.Reputation
// FROM TopPosts tp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) LEFT JOIN Badges bg ON bg.UserId = u.Id WHERE bg.Class = 1
// ORDER BY Score DESC, tp.CreationDate DESC;
fn q9815(db: &'static So) -> String {
    let s = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, a)| (Reverse(a[0]), p), 10);
    let tp = rel(v);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))).select(&db.badge.name);
    type R = (Id<Post>, [i64; 2]);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&db.post.owner_user).select(Ident::<User>::new().and(gold))))));
    rows(v.into_iter().map(|(_, ((p, a), (u, b)))| {
        let mut f = post_fields(db, p, &["title", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::S(b)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, SUM(COALESCE(v.UpVotes, 0)) AS TotalUpVotes, SUM(COALESCE(v.DownVotes, 0)) AS TotalDownVotes,
//        SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     GROUP BY u.Id),
// TopUsers AS (SELECT UserId, PostCount, TotalUpVotes, TotalDownVotes, TotalComments, RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserPostCounts)
// SELECT u.DisplayName, t.PostCount, t.TotalUpVotes, t.TotalDownVotes, t.TotalComments FROM TopUsers t JOIN Users u ON t.UserId = u.Id WHERE t.PostRank <= 10 ORDER BY t.PostRank;
fn q11992(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let Vote { post, vote_type_id, .. } = &db.vote;
    let vc = db.vote.group_by(post).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&vc).opt().and((&cc).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((v, c)) => {
                let v = v.unwrap_or([0, 0]);
                [a[0] + 1, a[1] + v[0], a[2] + v[1], a[3] + c.unwrap_or(0)]
            }
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND p.ViewCount > 100),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount FROM Votes v WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '7 days' GROUP BY v.PostId),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.RankScore, COALESCE(rv.VoteCount, 0) AS RecentVoteCount
//     FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId WHERE rp.RankScore <= 5)
// SELECT pd.Title, pd.OwnerDisplayName, pd.RankScore, pd.RecentVoteCount FROM PostDetails pd ORDER BY pd.RankScore, pd.RecentVoteCount DESC;
fn q6729(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, post_type_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_days(t0, -30))).with(view_count.gt(100)).with(owner_user).select(post_type_id));
    let v = ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap()), Reverse(view_count.get(p).unwrap())), false);
    let v = per_group(v, |&(_, t)| t);
    let tr = rel(v.into_iter().filter(|x| x.1 <= 5).map(|((p, _), r)| (p, r)).collect());
    let rk: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_days(t0, -7))));
    let rv = (&rk).map(|(p, _)| p).group_by(Same::<Id<Post>>::new()).select(recent.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&rv).and((&rk).map(|(_, r)| r))).into_iter().map(|(p, (n, r))| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend([V::I(r), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rn = 1
//     ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.VoteCount, u.DisplayName AS OwnerDisplayName, u.Reputation
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id WHERE u.Reputation > 100 ORDER BY tp.Score DESC;
//
// rn partitions by p.Id, so it is always 1. The LIMIT reads only base columns, so the top posts are picked first and the
// comment x vote product is driven for those alone. tp.PostId = u.Id compares a post id with a user id, so it joins through origid.
fn q8856(db: &'static So) -> String {
    let Post { creation_date, score, view_count, origid, .. } = &db.post;
    let v = top_n(drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let uid: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(100)).select(&db.user.origid).inv().collect();
    rows(drain((&s).and(&vc).and(origid.select(&uid))).into_iter().map(|(p, ((c, n), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY COUNT(v.Id) DESC, p.CreationDate DESC) AS VoteRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rp.VoteRank <= 5)
// SELECT tp.*, CASE WHEN tp.CommentCount > 15 THEN 'Highly Engaged' WHEN tp.CommentCount BETWEEN 5 AND 15 THEN 'Moderately Engaged' ELSE 'Less Engaged' END AS EngagementLevel
// FROM TopPosts tp ORDER BY tp.VoteCount DESC, tp.CreationDate DESC;
fn q5740(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let v = ranked(drain(&s), |&(p, a)| (post_type_id.get(p).unwrap(), Reverse(a[1]), Reverse(creation_date.get(p).unwrap())), false);
    let v = per_group(v, |&(p, _)| post_type_id.get(p).unwrap());
    rows(v.into_iter().filter(|x| x.1 <= 5).map(|((p, a), _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if a[0] > 15 { "Highly Engaged" } else if a[0] >= 5 { "Moderately Engaged" } else { "Less Engaged" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN p.Score ELSE 0 END) AS TotalScore, AVG(p.ViewCount) AS AvgViews,
//        COUNT(DISTINCT c.Id) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalScore, AvgViews, TotalComments, RANK() OVER (ORDER BY TotalScore DESC, TotalPosts DESC) AS Rank FROM UserActivity)
// SELECT tu.DisplayName, tu.TotalPosts, tu.Questions, tu.Answers, tu.TotalScore, tu.AvgViews, tu.TotalComments, CASE WHEN tu.Rank <= 10 THEN 'Top User' ELSE 'Regular User' END AS UserStatus
// FROM TopUsers tu WHERE tu.TotalPosts > 5 ORDER BY tu.Rank;
fn q5934(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((((t, s), w), _)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + if t == 1 || t == 2 { s } else { 0 }, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)],
            None => a,
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&s).and(&pc).and(&cc)), |&(_, ((a, n), _))| (Reverse(a[2]), Reverse(n)), false);
    rows(drain(rel(v).filt(|x| (x.0 .1).0 .1 > 5)).into_iter().map(|x| x.1).map(|((u, ((a, n), c)), r)| {
        row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), V::I(c), V::S(if r <= 10 { "Top User" } else { "Regular User" })])
    }))
}

// Rewritten (rewrites/5214.sql): the ROW_NUMBER order is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY COUNT(c.Id) DESC, SUM(v.VoteTypeId) DESC, p.Id) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.CommentCount, tp.UpvoteCount, tp.DownvoteCount,
//        CASE WHEN tp.UpvoteCount - tp.DownvoteCount > 0 THEN 'Positive' WHEN tp.UpvoteCount - tp.DownvoteCount < 0 THEN 'Negative' ELSE 'Neutral' END AS Sentiment
// FROM TopPosts tp ORDER BY tp.UpvoteCount DESC;
fn q5214(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 5], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + t.is_some() as i64, a[4] + t.unwrap_or(0)]);
    let top = top_per(drain(&s), |&(p, _)| post_type_id.get(p).unwrap(), |&(p, a)| (Reverse(a[0]), a[3] == 0, Reverse(a[4]), p), 5, false);
    rows(top.into_iter().map(|(p, a)| {
        let d = a[1] - a[2];
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(if d > 0 { "Positive" } else if d < 0 { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions, COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers,
//        MAX(U.Reputation) AS MaxReputation
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalUpVotes, TotalDownVotes, TotalPosts, TotalQuestions, TotalAnswers, MaxReputation, ROW_NUMBER() OVER (ORDER BY MaxReputation DESC) AS Rank FROM UserVoteSummary)
// SELECT T.UserId, T.DisplayName, T.TotalUpVotes, T.TotalDownVotes, T.TotalPosts, T.TotalQuestions, T.TotalAnswers, T.MaxReputation FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.MaxReputation DESC;
//
// Rank reads only Reputation, so the top users are picked first.
fn q5527(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tops: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Vote { post, vote_type_id, .. } = &db.vote;
    let s = (&tops).group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let of_type = |t: i64| votes_by(db).select(post.select(Ident::<Post>::new().with((&db.post.post_type_id).eq(t)))).opt();
    let dp = (&tops).group_by(Ident::<User>::new()).select(votes_by(db).select(post).opt()).buf_fold(distinct_some);
    let dq = (&tops).group_by(Ident::<User>::new()).select(of_type(1)).buf_fold(distinct_some);
    let da = (&tops).group_by(Ident::<User>::new()).select(of_type(2)).buf_fold(distinct_some);
    rows(drain((&s).and(&dp).and(&dq).and(&da)).into_iter().map(|(u, (((a, p), q), n))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(p), V::I(q), V::I(n), user_col(db, u, "rep")]);
        row(f)
    }))
}

// Rewritten (rewrites/8164.sql): the STRING_AGG is ordered by rp.PostId.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 10),
// TopUsers AS (SELECT OwnerUserId, COUNT(*) AS PostCount, SUM(Score) AS TotalScore FROM RankedPosts WHERE UserPostRank <= 5 GROUP BY OwnerUserId HAVING COUNT(*) >= 5),
// MostActiveUser AS (SELECT u.Id, u.DisplayName, tu.PostCount, tu.TotalScore FROM Users u JOIN TopUsers tu ON u.Id = tu.OwnerUserId ORDER BY tu.TotalScore DESC LIMIT 1)
// SELECT mu.DisplayName AS MostActiveUser, mu.PostCount, mu.TotalScore, STRING_AGG(rp.Title, '; ' ORDER BY rp.PostId) AS TopPostTitles
// FROM MostActiveUser mu JOIN RankedPosts rp ON mu.Id = rp.OwnerUserId GROUP BY mu.DisplayName, mu.PostCount, mu.TotalScore;
fn q8164(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, title, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1).and(score.gt(10))).with(owner_user);
    let top = top_per(drain(rp().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tu = (&tp).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let mu = top_n(drain((&tu).filt(|a| a[0] >= 5)), |&(u, a)| (Reverse(a[1]), u), 1);
    let mu = rel(mu);
    let agg = (&mu)
        .group_by(Same::<(Id<User>, [i64; 2])>::new())
        .select(Same::<(Id<User>, [i64; 2])>::new().map(|(u, _): (Id<User>, [i64; 2])| u).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1).and(score.gt(10)))).select(Ident::<Post>::new().and(title.opt()))))
        .buf_fold(|ts| {
            let mut ts: Vec<(Id<Post>, Option<Str>)> = ts.iter().copied().collect();
            ts.sort_by_key(|x| x.0);
            let parts: Vec<&str> = ts.iter().flat_map(|x| x.1).collect();
            if parts.is_empty() { None } else { Some(&*Box::leak(parts.join("; ").into_boxed_str())) }
        });
    rows(drain(&agg).into_iter().map(|((u, a), t)| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), ostr(t)])))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        COUNT(DISTINCT P.Id) AS TotalPosts, AVG(P.Score) AS AvgScore FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// PostClosureSummary AS (SELECT PH.PostId, COUNT(PH.Id) AS ClosureCount, STRING_AGG(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 'Closed' ELSE 'Reopened' END, ', ') AS ClosureHistory
//     FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId)
// SELECT U.DisplayName, UV.Upvotes, UV.Downvotes, UV.TotalPosts, UV.AvgScore, COALESCE(PCS.ClosureCount, 0) AS ClosureCount, COALESCE(PCS.ClosureHistory, 'No Closure History') AS ClosureHistory
// FROM UserVoteSummary UV JOIN Users U ON UV.UserId = U.Id LEFT JOIN PostClosureSummary PCS ON PCS.PostId = U.Id WHERE UV.TotalPosts > 5
// ORDER BY UV.AvgScore DESC, UV.Upvotes - UV.Downvotes DESC LIMIT 10;
//
// PCS.PostId = U.Id compares a post id with a user id, so it joins on the raw ids.
fn q1545(db: &'static So) -> String {
    let Vote { post, vote_type_id, .. } = &db.vote;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id.and(post.select(&db.post.score).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, s)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + s.is_some() as i64, a[3] + s.unwrap_or(0)],
            None => a,
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(post).opt()).buf_fold(distinct_some);
    let PostHistory { post_id, post_history_type_id, .. } = &db.post_history;
    let pcs = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post_id).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&s).and((&dp).filt(|n| n > 5)).and((&db.user.origid).select(&pcs).opt()));
    let v = top_n(v, |&(u, ((a, _), _))| (a[2] == 0, Reverse(fkey(a[3] as f64 / a[2] as f64)), Reverse(a[0] - a[1]), u), 10);
    rows(v.into_iter().map(|(u, ((a, n), c))| {
        let h = match c {
            Some(c) => V::Owned(vec!["Closed"; c as usize].join(", ")),
            None => V::S("No Closure History"),
        };
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n), avg(a[3], a[2]), V::I(c.unwrap_or(0)), h])
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, QuestionCount, AnswerCount, WikiCount, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats)
// SELECT t.DisplayName, t.TotalPosts, t.QuestionCount, t.AnswerCount, t.WikiCount, t.UpVotes, t.DownVotes, (t.UpVotes - t.DownVotes) AS NetVotes
// FROM TopUsers t WHERE t.PostRank <= 10 ORDER BY NetVotes DESC;
//
// PostRank reads only COUNT(DISTINCT p.Id), which is folded over one row per post, so the top users are picked first and the post x vote product is driven for those alone.
fn q6026(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let tops: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    rows(drain((&s).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        f.push(V::I(a[3] - a[4]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserStats)
// SELECT U.DisplayName, U.Reputation, U.PostCount, U.AnswerCount, U.QuestionCount, U.UpVotes, U.DownVotes, RANK() OVER (ORDER BY U.Reputation DESC) AS OverallRank
// FROM TopUsers U WHERE U.Rank <= 10 ORDER BY U.Reputation DESC, U.PostCount DESC;
//
// Rank reads only Reputation and COUNT(DISTINCT P.Id), so the top users are picked first and the post x vote product is driven for those alone.
fn q7864(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = top_n(drain(&pc), |&(u, n)| (Reverse(rep.get(u).unwrap()), Reverse(n), u), 10);
    let tu = ranked(tu, |&(u, _)| Reverse(rep.get(u).unwrap()), false);
    let tr = rel(tu.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rk: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let tops: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    rows(drain((&s).and(&pc).and((&rk).map(|(_, r)| r))).into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH TagWordCounts AS (SELECT t.TagName, COUNT(*) AS TagCount, SUM(LENGTH(p.Body) - LENGTH(REPLACE(p.Body, t.TagName, ''))) / LENGTH(t.TagName) AS WordOccurrence
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostsCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS DownvotedPosts FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// PopularTags AS (SELECT TagName, TagCount, WordOccurrence, ROW_NUMBER() OVER (ORDER BY TagCount DESC) AS TagRank FROM TagWordCounts)
// SELECT u.UserId, u.Reputation, u.PostsCount, u.UpvotedPosts, u.DownvotedPosts, pt.TagName, pt.TagCount, pt.WordOccurrence
// FROM UserReputation u JOIN PopularTags pt ON u.PostsCount > 10 AND u.Reputation > 1000 WHERE pt.TagRank <= 10 ORDER BY u.Reputation DESC, pt.TagCount DESC;
//
// The ON clause names only u, so the qualifying users are crossed with the top tags.
fn q26340(db: &'static So) -> String {
    let lt = tag_mentions(db);
    type PT = (Id<Post>, Id<Tag>);
    let tw = (&lt)
        .group_by(Same::<PT>::new().map(|(_, t): PT| t))
        .select(Same::<PT>::new().map(|(p, _): PT| p).select(&db.post.body).and(Same::<PT>::new().map(|(_, t): PT| t).select(&db.tag.tag_name)))
        .fold([0i64; 2], |a, (b, t): (Str, Str)| [a[0] + 1, a[1] + (b.matches(t).count() * t.chars().count()) as i64]);
    let top = rel(top_n(drain(&tw), |&(t, a)| (Reverse(a[0]), t), 10));
    let ur = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.score).opt())
        .fold([0i64; 3], |a, s| match s {
            Some(s) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64],
            None => a,
        });
    let mut v = drain((&ur).filt(|a| a[0] > 10).cross(&top));
    v.sort_by_key(|&((u, _), (_, (_, a)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0])));
    rows(v.into_iter().map(|((u, _), (a, (t, n)))| {
        let name = db.tag.tag_name.get(t).unwrap();
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(name), V::I(n[0]), V::F(n[1] as f64 / name.chars().count() as f64)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserStats)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.Upvotes, tu.Downvotes, (tu.Upvotes - tu.Downvotes) AS NetVotes
// FROM TopUsers tu WHERE tu.UserRank <= 10 ORDER BY tu.Reputation DESC;
//
// UserRank reads only Reputation, so the top users are picked first and the post x vote product is driven for those alone.
fn q6734(db: &'static So) -> String {
    let tu = ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tops: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = (&tops).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&s).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(a[2] - a[3]));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
//        AVG(P.Score) AS AvgScore, AVG(P.ViewCount) AS AvgViews FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, TotalViews, AvgScore, AvgViews,
//        RANK() OVER (ORDER BY TotalScore DESC) AS RankByScore, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, TotalViews, AvgScore, AvgViews, RankByScore, RankByViews
// FROM TopUsers WHERE RankByScore <= 10 OR RankByViews <= 10 ORDER BY RankByScore, RankByViews;
fn q10555(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| Reverse(a[4]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[6]), false);
    let mut v: Vec<_> = v.into_iter().filter(|&((_, s), w)| s <= 10 || w <= 10).collect();
    v.sort_by_key(|&((_, s), w)| (s, w));
    rows(v.into_iter().map(|(((u, a), s), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6]), avg(a[4], a[1]), avg(a[6], a[5]), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, p.CreationDate, u.Reputation AS OwnerReputation,
//        COUNT(DISTINCT c.Id) AS CommentTotal, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, p.CreationDate, u.Reputation),
// TopPosts AS (SELECT PostId, ViewCount, Score, AnswerCount, CommentCount, FavoriteCount, OwnerReputation, RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS RankScore FROM PostStats)
// SELECT tp.PostId, tp.ViewCount, tp.Score, tp.AnswerCount, tp.CommentCount, tp.FavoriteCount, tp.OwnerReputation, tp.RankScore FROM TopPosts tp WHERE tp.RankScore <= 10;
//
// None of the aggregates is projected; RankScore reads only base columns, over the posts that have an owner.
fn q13239(db: &'static So) -> String {
    let Post { owner_user, view_count, .. } = &db.post;
    let v = ranked(drain(db.post.with(owner_user).select(&db.post.score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, _), r)| {
        let mut f = post_fields(db, p, &["id", "views", "score", "answers", "comments", "favorites", "rep"]);
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, MAX(p.CreationDate) AS LastPostDate FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, p.CommentCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p)
// SELECT ua.DisplayName, ua.Reputation, ua.PostCount, ua.PositivePosts, ua.NegativePosts, ps.PostId, ps.Title, ps.ViewCount, ps.AnswerCount, ps.CommentCount, ps.CreationDate
// FROM UserActivity ua LEFT JOIN PostStats ps ON ua.UserId = ps.PostId AND ps.PostRank = 1
// WHERE ua.Reputation > 1000 AND (ua.PostCount > 5 OR ps.AnswerCount > 2 OR ps.CommentCount > 10) AND ps.CreationDate IS NOT NULL
// ORDER BY ua.Reputation DESC, ps.ViewCount DESC LIMIT 10;
//
// ua.UserId = ps.PostId compares a user id with a post id, so it joins on the raw ids.
fn q608(db: &'static So) -> String {
    let Post { owner_user, creation_date, origid, answer_count, comment_count, .. } = &db.post;
    let newest = top_per(drain(db.post.select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let nv: MatSet<Id<Post>> = rel(newest.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_id: HashIdx<i64, Id<Post>> = (&nv).select(origid).inv().collect();
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.score).opt())
        .fold([0i64; 3], |a, s| match s {
            Some(s) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64],
            None => a,
        });
    let ps = (&db.user.origid).select(&by_id).select(Ident::<Post>::new().and(answer_count.opt()).and(comment_count));
    let v = drain((&ua).and(ps).filt(|(a, ((_, n), c))| a[0] > 5 || n.map_or(false, |n| n > 2) || c > 10));
    let v = top_n(v, |&(u, (_, ((p, _), _)))| {
        let w = db.post.view_count.get(p);
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(u, (a, ((p, _), _)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "views", "answers", "comments", "created"]));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, RANK() OVER (ORDER BY PostCount DESC, UpVotes DESC) AS UserRank FROM UserPostStats)
// SELECT tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.UpVotes, tu.DownVotes, CASE WHEN tu.UserRank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributionLevel
// FROM TopUsers tu WHERE tu.PostCount > 0 ORDER BY tu.UserRank;
fn q9663(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| (Reverse(a[0]), Reverse(a[3])), false);
    rows(drain(rel(v).filt(|x| (x.0 .1)[0] > 0)).into_iter().map(|x| x.1).map(|((u, a), r)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top Contributor" } else { "Regular Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.Score, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.Score, rp.CreationDate FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostCommentCounts AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostVoteCounts AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount FROM Votes v GROUP BY v.PostId)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.Score, tp.CreationDate, COALESCE(pcc.CommentCount, 0) AS CommentCount, COALESCE(pvc.VoteCount, 0) AS VoteCount
// FROM TopPosts tp LEFT JOIN PostCommentCounts pcc ON tp.PostId = pcc.PostId LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q5512(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(date(2024, 10, 1), -30))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, n))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "created"]);
        f.extend([V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.UpVotes, tu.DownVotes, (tu.UpVotes - tu.DownVotes) AS NetVotes
// FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY NetVotes DESC, Reputation DESC;
//
// Rank reads only Reputation, so the top users are picked first and the post x vote product is driven for those alone.
fn q9312(db: &'static So) -> String {
    let tu = ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tops: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = (&tops).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&s).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(a[2] - a[3]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        u.DisplayName AS OwnerDisplayName, p.PostTypeId, DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.RankScore <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.AnswerCount, tp.OwnerDisplayName, pt.Name AS PostType
// FROM TopPosts tp JOIN PostTypes pt ON tp.PostTypeId = pt.Id ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q6023(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let v = ranked(v, |&(p, t)| {
        let w = view_count.get(p);
        (t, Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, true);
    let mut first = 0;
    let v: Vec<_> = (0..v.len())
        .map(|i| {
            if i == 0 || (v[i].0).1 != (v[i - 1].0).1 {
                first = v[i].1 - 1;
            }
            (v[i].0, v[i].1 - first)
        })
        .collect();
    let tp: MatSet<Id<Post>> = rel(v.into_iter().filter(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&cc).and(&ac)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a)]);
        f.extend(post_fields(db, p, &["owner", "type"]));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount, SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// UserRanking AS (SELECT ua.UserId, ua.PostCount, ua.QuestionCount, ua.AnswerCount, ua.UpVotesCount - ua.DownVotesCount AS VotingBalance, ua.BadgeCount,
//        RANK() OVER (ORDER BY ua.UpVotesCount DESC, ua.PostCount DESC) AS UserRank FROM UserActivity ua)
// SELECT ur.UserId, u.DisplayName, ur.PostCount, ur.QuestionCount, ur.AnswerCount, ur.VotingBalance, ur.BadgeCount, ur.UserRank
// FROM UserRanking ur JOIN Users u ON ur.UserId = u.Id WHERE ur.UserRank <= 10 ORDER BY ur.UserRank;
fn q6962(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let (n, t, v) = p.map_or((0, 0, None), |(t, v)| (1, t, v));
            [a[0] + n, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5] + b.is_some() as i64]
        });
    let v = ranked(drain(&s), |&(_, a)| (Reverse(a[3]), Reverse(a[0])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3] - a[4]), V::I(a[5]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// TopPosts AS (SELECT r.PostId, r.Title, r.CreationDate, r.ViewCount, r.Score, r.CommentCount, r.TotalBounty FROM RankedPosts r WHERE r.PostRank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.CommentCount, tp.TotalBounty, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// PostRank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
// tp.PostId = u.Id compares a post id with a user id, so it joins through origid.
fn q9199(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    rows(drain((&s).and(origid.select(&uid))).into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, COUNT(c.Id) AS CommentCount, AVG(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        AVG(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes vt ON p.Id = vt.PostId GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN rp.CommentCount > 0 THEN 1 ELSE 0 END) AS PostsWithComments, SUM(rp.UpvoteCount) AS TotalUpvotes,
//        SUM(rp.DownvoteCount) AS TotalDownvotes FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.Id GROUP BY u.Id, u.DisplayName)
// SELECT us.DisplayName, us.PostsWithComments, us.TotalUpvotes, us.TotalDownvotes,
//        COALESCE(ROUND((us.TotalUpvotes::decimal / NULLIF((us.TotalUpvotes + us.TotalDownvotes), 0)) * 100, 2), 0) AS UpvotePercentage
// FROM UserStats us WHERE us.PostsWithComments > 0 ORDER BY us.TotalUpvotes DESC, us.DisplayName LIMIT 10;
//
// u.Id = rp.Id compares a user id with a post id, so it joins on the raw ids. RN is never read.
fn q5292(db: &'static So) -> String {
    let rp = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + 1, a[1] + c.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let pid: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.origid).select(&pid).select(&rp).opt())
        .fold((0i64, 0i64, 0.0f64, 0.0f64), |(n, w, u, d), a| match a {
            Some(a) => (n + 1, w + (a[1] > 0) as i64, u + a[2] as f64 / a[0] as f64, d + a[3] as f64 / a[0] as f64),
            None => (n, w, u, d),
        });
    let v = top_n(drain((&us).filt(|(_, w, _, _)| w > 0)), |&(u, (_, _, up, _))| (Reverse(fkey(up)), db.user.display_name.get(u).unwrap()), 10);
    rows(v.into_iter().map(|(u, (_, w, up, d))| {
        let pct = if up + d == 0.0 { 0.0 } else { (((up * 1000.0).round() / 1000.0) / (up + d) * 100.0 * 100.0).round() / 100.0 };
        row(vec![user_col(db, u, "name"), V::I(w), V::F(up), V::F(d), V::F(pct)])
    }))
}

// WITH UserPostStatistics AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount, SUM(V.BountyAmount) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 GROUP BY U.Id, U.DisplayName),
// TopPerformingUsers AS (SELECT UserId, DisplayName, PostCount, PositivePosts, NegativePosts, AcceptedAnswers, CommentCount, TotalBounty,
//        RANK() OVER (ORDER BY PostCount DESC, TotalBounty DESC) AS UserRank FROM UserPostStatistics)
// SELECT T.UserId, T.DisplayName, T.PostCount, T.PositivePosts, T.NegativePosts, T.AcceptedAnswers, T.CommentCount, T.TotalBounty FROM TopPerformingUsers T WHERE T.UserRank <= 10 ORDER BY T.UserRank;
fn q29228(db: &'static So) -> String {
    let Post { score, accepted_answer_id, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(accepted_answer_id.opt()).and(comments_of(db).opt()).and(bounty.opt())).opt())
        .fold([0i64; 7], |a, x| match x {
            Some((((s, acc), c), b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + acc.is_some() as i64, a[4] + c.is_some() as i64, a[5] + b.is_some() as i64, a[6] + b.unwrap_or(0)]
            }
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| (Reverse(a[0]), a[5] == 0, Reverse(a[6])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a[..5].iter().map(|&x| V::I(x)));
        f.push(nullable(a[6], a[5]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostVoteCounts AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT up.DisplayName, rp.Title, rp.CreationDate, rp.Score, COALESCE(pv.UpVotes, 0) AS UpVotes, COALESCE(pv.DownVotes, 0) AS DownVotes, ur.Reputation, ur.BadgeCount
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id LEFT JOIN PostVoteCounts pv ON rp.PostId = pv.PostId JOIN UserReputation ur ON up.Id = ur.UserId
// WHERE ur.Reputation > 1000 AND rp.PostRank <= 10 ORDER BY rp.Score DESC, ur.Reputation DESC OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY;
fn q4171(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let bc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&bc)).and((&pv).opt())));
    let v = top_n(v, |&(p, ((u, _), _))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p), 15);
    rows(v.into_iter().skip(5).map(|(p, ((u, b), pv))| {
        let pv = pv.unwrap_or([0, 0]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(pv[0]), V::I(pv[1]), user_col(db, u, "rep"), V::I(b)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserActivity)
// SELECT T.DisplayName, T.Reputation, T.PostCount, T.AnswerCount, T.QuestionCount, T.UpVotes, T.DownVotes, CASE WHEN T.ReputationRank <= 10 THEN 'Top User' ELSE 'Regular User' END AS UserCategory
// FROM TopUsers T WHERE T.PostCount > 10 ORDER BY T.Reputation DESC, T.PostCount DESC;
fn q7969(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(0));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&s).and(&pc)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    rows(drain(rel(v).filt(|x| (x.0 .1).1 > 10)).into_iter().map(|x| x.1).map(|((u, (a, n)), r)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top User" } else { "Regular User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, p.PostTypeId),
// PostInsights AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes, pt.Name AS PostTypeName
//     FROM RankedPosts rp JOIN PostTypes pt ON rp.PostId = pt.Id WHERE rp.Rank <= 10)
// SELECT pi.Title, pi.ViewCount, pi.CommentCount, pi.UpVotes, pi.DownVotes, pi.PostTypeName, (pi.UpVotes - pi.DownVotes) AS NetVotes FROM PostInsights pi ORDER BY pi.ViewCount DESC;
//
// Rank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
// rp.PostId = pt.Id compares a post id with a post type id, so it joins on the raw ids.
fn q7193(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ptid: HashIdx<i64, Str> = (&db.post_type.origid).inv().select(&db.post_type.name).collect();
    rows(drain((&s).and(origid.select(&ptid))).into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(n), V::I(a[1] - a[2])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(V.BountyAmount) AS TotalBounty, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, TotalBounty, Upvotes, Downvotes, RANK() OVER (ORDER BY PostCount DESC) AS PostRank,
//        RANK() OVER (ORDER BY Upvotes DESC) AS UpvoteRank FROM UserStats)
// SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, TotalBounty, Upvotes, Downvotes, PostRank, UpvoteRank FROM TopUsers WHERE PostRank <= 10 OR UpvoteRank <= 10 ORDER BY PostRank, UpvoteRank;
fn q12802(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(vote_type_id.and(bounty_amount.opt())).opt())).opt())
        .fold([0i64; 6], |a, x| match x {
            Some((t, v)) => {
                let (vt, b) = v.map_or((0, None), |x| x);
                [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0), a[4] + (vt == 2) as i64, a[5] + (vt == 3) as i64]
            }
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&s).and(&pc)), |&(_, (_, n))| Reverse(n), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[4]), false);
    let mut v: Vec<_> = v.into_iter().filter(|&((_, p), w)| p <= 10 || w <= 10).collect();
    v.sort_by_key(|&((_, p), w)| (p, w));
    rows(v.into_iter().map(|(((u, (a, n)), p), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[4]), V::I(a[5]), V::I(p), V::I(w)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(p.Score) AS AverageScore FROM Posts p GROUP BY p.OwnerUserId),
// TagStatistics AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName)
// SELECT ur.DisplayName, ur.Reputation, ps.TotalPosts, ps.TotalQuestions, ps.TotalAnswers, ps.AverageScore, ts.TagName, ts.PostCount, ts.TotalViews
// FROM UserReputation ur LEFT JOIN PostStatistics ps ON ur.UserId = ps.OwnerUserId LEFT JOIN TagStatistics ts ON ts.PostCount > 10
// WHERE ur.ReputationRank <= 10 ORDER BY ur.Reputation DESC, ts.TotalViews DESC;
//
// The second ON clause names only ts, so the top users are crossed with the tags that pass it.
fn q1968(db: &'static So) -> String {
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tops: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let ts = left_all(drain((&tag_stats(db)).filt(|a| a[0] > 10)));
    let mut v = drain((&tops).select((&ps).opt()).cross(&ts));
    v.sort_by_key(|&((u, _), (_, t))| (Reverse(db.user.reputation.get(u).unwrap()), t.map(|(_, a)| Reverse(a[2]))));
    rows(v.into_iter().map(|((u, _), (a, t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match t {
            Some((t, a)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[2])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, AVG(p.Score) AS AvgScore, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalAnswers, TotalQuestions, AvgScore, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY TotalUpVotes DESC) AS Ranking FROM UserStats)
// SELECT tu.DisplayName, CASE WHEN tu.TotalPosts > 100 THEN 'Veteran' WHEN tu.TotalPosts BETWEEN 51 AND 100 THEN 'Experienced' ELSE 'Novice' END AS UserType,
//        tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.AvgScore, tu.Ranking
// FROM TopUsers tu WHERE tu.Ranking <= 10 OR (SELECT COUNT(*) FROM Badges b WHERE b.UserId = tu.UserId AND b.Class = 1) > 0 ORDER BY tu.Ranking, tu.TotalPosts DESC;
fn q4970(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(posts_of(db).select(post_type_id.and(score)).opt()))
        .fold([0i64; 5], |a, (up, p)| match p {
            Some((t, s)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + s, a[4] + up],
            None => [a[0], a[1], a[2], a[3], a[4] + up],
        });
    let gold: MatSet<Id<User>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).collect();
    let v = ranked(drain((&s).and(Ident::<User>::new().with(&gold).opt())), |&(_, (a, _))| Reverse(a[4]), false);
    rows(drain(rel(v).filt(|((_, (_, g)), r)| r <= 10 || g.is_some())).into_iter().map(|x| x.1).map(|((u, (a, _)), r)| {
        let n = a[0];
        row(vec![
            user_col(db, u, "name"),
            V::S(if n > 100 { "Veteran" } else if n >= 51 { "Experienced" } else { "Novice" }),
            V::I(n),
            V::I(a[2]),
            V::I(a[1]),
            avg(a[3], n),
            V::I(r),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, OwnerName, CommentCount, VoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT t.Title, t.OwnerName, t.CreationDate, t.Score, t.CommentCount, t.VoteCount, pt.Name AS PostType
// FROM TopPosts t JOIN PostTypes pt ON t.PostId IN (SELECT Id FROM Posts WHERE PostTypeId = pt.Id) ORDER BY t.Score DESC, t.CreationDate DESC;
//
// Rank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
// The IN matches each post to its own type.
fn q7979(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let up = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(up().opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&cc).and(&vc).and(ptype_name(db))).into_iter().map(|(p, ((c, n), t))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score"]);
        f.extend([V::I(c), V::I(n), V::S(t)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.Questions, us.Answers, us.Wikis, us.Upvotes, us.Downvotes,
//        ROW_NUMBER() OVER (ORDER BY us.Reputation DESC) AS Rank FROM UserStats us)
// SELECT tu.Rank, tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.Questions, tu.Answers, tu.Wikis, tu.Upvotes, tu.Downvotes FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// Rank reads only Reputation, so the top users are picked first and the post x vote product is driven for those alone.
fn q9414(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tr = rel((0..tu.len()).map(|i| (tu[i].0, i as i64 + 1)).collect());
    let rk: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let tops: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = (&tops).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&s).and(&pc).and((&rk).map(|(_, r)| r))).into_iter().map(|(u, ((a, n), r))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount FROM RankedPosts WHERE rn <= 5),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(v.BountyAmount) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName)
// SELECT ua.DisplayName, ua.PostCount, ua.TotalBounty, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount
// FROM UserActivity ua LEFT JOIN TopPosts tp ON ua.UserId = tp.PostId WHERE ua.TotalBounty IS NOT NULL OR ua.PostCount > 0
// ORDER BY ua.TotalBounty DESC, ua.PostCount DESC LIMIT 10;
//
// ua.UserId = tp.PostId compares a user id with a post id, so it joins on the raw ids.
fn q825(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, origid, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_id: HashIdx<i64, Id<Post>> = (&tp).select(origid).inv().collect();
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).opt())
        .fold([0i64; 3], |a, x| match x {
            Some(v) => {
                let b = v.flatten();
                [a[0] + 1, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
            }
            None => a,
        });
    let v = drain((&ua).filt(|a| a[1] > 0 || a[0] > 0).and((&db.user.origid).select(&by_id).opt()));
    let v = top_n(v, |&(u, (a, _))| (a[1] == 0, Reverse(a[2]), Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1])];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score", "views", "answers"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostScore AS (SELECT P.OwnerUserId, SUM(P.Score) AS TotalScore, COUNT(P.Id) AS PostCount FROM Posts P GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT UB.UserId, UB.DisplayName, COALESCE(PS.TotalScore, 0) AS TotalScore, UB.BadgeCount,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(PS.TotalScore, 0) DESC, UB.BadgeCount DESC) AS Rank FROM UserBadges UB LEFT JOIN PostScore PS ON UB.UserId = PS.OwnerUserId)
// SELECT T.UserId, T.DisplayName, T.TotalScore, T.BadgeCount,
//        CASE WHEN T.TotalScore >= 100 THEN 'High Contributor' WHEN T.TotalScore BETWEEN 50 AND 99 THEN 'Moderate Contributor' ELSE 'New Contributor' END AS ContributionLevel
// FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.Rank;
fn q809(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |s, x| s + x);
    let v = top_n(drain((&bc).and((&ps).opt())), |&(u, (b, s))| (Reverse(s.unwrap_or(0)), Reverse(b), u), 10);
    rows(v.into_iter().map(|(u, (b, s))| {
        let s = s.unwrap_or(0);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(s), V::I(b), V::S(if s >= 100 { "High Contributor" } else if (50..=99).contains(&s) { "Moderate Contributor" } else { "New Contributor" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY COUNT(c.Id) DESC, p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score),
// RecentActivity AS (SELECT PostId, COUNT(*) AS EditCount FROM PostHistory WHERE CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes, ra.EditCount, rp.PostRank
// FROM RankedPosts rp LEFT JOIN RecentActivity ra ON rp.PostId = ra.PostId WHERE rp.PostRank <= 10 ORDER BY rp.PostRank;
fn q5411(db: &'static So) -> String {
    let s = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post, creation_date, .. } = &db.post_history;
    let ra = db.post_history.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = ranked(drain((&s).and((&ra).opt())), |&(p, (a, _))| (Reverse(a[0]), Reverse(db.post.score.get(p).unwrap())), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, (a, e)), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.extend([oint(e), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE 0 END) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalScore, AvgViewCount, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats),
// RecentActivity AS (SELECT p.OwnerUserId, MAX(p.LastActivityDate) AS LastActivity FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.OwnerUserId)
// SELECT tu.DisplayName, tu.TotalPosts, tu.Questions, tu.Answers, tu.TotalScore, tu.AvgViewCount, ra.LastActivity
// FROM TopUsers tu LEFT JOIN RecentActivity ra ON tu.UserId = ra.OwnerUserId WHERE tu.ScoreRank <= 10 ORDER BY tu.TotalScore DESC;
fn q9233(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, creation_date, last_activity_date, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 6], |a, x| match x {
            Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { s } else { 0 }, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
            None => a,
        });
    let ra = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(last_activity_date).fold(i64::MIN, |m, d| m.max(d));
    let v = ranked(drain((&s).and((&ra).opt())), |&(_, (a, _))| Reverse(a[3]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, l)), _)| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4]), ots(l)])
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, CommentCount, UpVoteCount, DownVoteCount, RANK() OVER (ORDER BY PostCount DESC) AS UserRank FROM UserActivity)
// SELECT tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.CommentCount, tu.UpVoteCount, tu.DownVoteCount, COALESCE(b.Name, 'No Badges') AS BadgeName
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId WHERE tu.UserRank <= 10 ORDER BY tu.UserRank, b.Date DESC;
fn q9536(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, x| match x {
            Some(((t, c), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    let top = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| (u, a)).collect());
    type R = (Id<User>, [i64; 6]);
    let v = drain((&top).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(badges_of(db).opt()))));
    rows(v.into_iter().map(|(_, ((u, a), b))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.push(V::S(b.map_or("No Badges", |b| db.badge.name.get(b).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, P.Score, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS ScoreRank
//     FROM Posts P WHERE P.PostTypeId = 1 AND P.Score > 0),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT UP.DisplayName, UP.Reputation, RB.PostId, RB.Title, RB.CreationDate, RB.Score, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges
// FROM Users UP LEFT JOIN RankedPosts RB ON UP.Id = RB.OwnerUserId AND RB.ScoreRank = 1 LEFT JOIN UserBadges UB ON UP.Id = UB.UserId
// WHERE UP.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND (UB.GoldBadges > 0 OR UB.SilverBadges > 1 OR UB.BronzeBadges > 2)
// ORDER BY UB.GoldBadges DESC, UB.SilverBadges DESC, RB.Score DESC LIMIT 10;
fn q3525(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let rb = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rb).map(|(u, _)| u).inv().select(&rb).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = drain(
        db.user
            .with((&db.user.last_access_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
            .select((&by_user).map(|(_, p)| p).opt().and((&ub).filt(|a| a[0] > 0 || a[1] > 1 || a[2] > 2))),
    );
    let v = top_n(v, |&(u, (p, a))| (Reverse(a[0]), Reverse(a[1]), p.is_none(), p.map(|p| Reverse(score.get(p).unwrap())), u, p), 10);
    rows(v.into_iter().map(|(u, (p, a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "created", "score"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount, SUM(V.BountyAmount) AS TotalBounties,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, CommentCount, TotalBounties, Upvotes - Downvotes AS NetVotes, RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserActivity)
// SELECT U.UserId, U.DisplayName, U.PostCount, U.CommentCount, U.TotalBounties, U.NetVotes, P.Title AS MostActivePostTitle, P.CreationDate AS MostActivePostDate
// FROM TopUsers U LEFT JOIN Posts P ON U.UserId = P.OwnerUserId WHERE U.PostRank <= 10 ORDER BY U.PostCount DESC, U.NetVotes DESC;
//
// PostRank reads only COUNT(DISTINCT P.Id), folded over one row per post, so the top users are picked first and the post x comment x vote product is driven for those alone.
fn q26968(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(0));
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let tops: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(vote_type_id.and(bounty_amount.opt())).opt())).opt())
        .fold([0i64; 3], |a, x| match x {
            Some((_, Some((t, b)))) => [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + (t == 2) as i64 - (t == 3) as i64],
            _ => a,
        });
    let cc = (&tops).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and(&cc).and(&pc).and(posts_of(db).opt()));
    rows(v.into_iter().map(|(u, (((a, c), n), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c), nullable(a[1], a[0]), V::I(a[2])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS Owner, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// RecentVotes AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE v.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 month' GROUP BY p.Id),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.Owner, COALESCE(rv.VoteCount, 0) AS VoteCount, COALESCE(pc.CommentCount, 0) AS CommentCount
// FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId WHERE rp.rn <= 5 ORDER BY rp.PostId;
fn q7570(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_months(date(2024, 10, 1), -1))));
    let rv = (&tp).group_by(Ident::<Post>::new()).select(recent.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&rv).and(&cc)).into_iter().map(|(p, (n, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(n), V::I(c)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore, SUM(CASE WHEN p.AnswerCount IS NOT NULL THEN p.AnswerCount ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.CommentCount IS NOT NULL THEN p.CommentCount ELSE 0 END) AS TotalComments FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalViews, TotalScore, TotalAnswers, TotalComments, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalViews, TotalScore, TotalAnswers, TotalComments FROM TopUsers WHERE Rank <= 10;
fn q10880(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, answer_count, comment_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(answer_count.opt()).and(comment_count)).opt())
        .fold([0i64; 7], |a, x| match x {
            Some(((((t, s), w), n), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + s, a[5] + n.unwrap_or(0), a[6] + c],
            None => a,
        });
    let v = top_n(drain(&s), |&(u, a)| (Reverse(a[4]), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Location, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.AnswerCount > 0 THEN 1 ELSE 0 END) AS QuestionsWithAnswers, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation, u.Location),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Location, PostCount, AnswerCount, QuestionsWithAnswers FROM RankedUsers WHERE ReputationRank <= 10)
// SELECT tu.DisplayName, tu.Reputation, tu.Location, tu.PostCount, tu.AnswerCount, tu.QuestionsWithAnswers, b.Name AS BadgeName, bh.Date AS BadgeDate
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId LEFT JOIN (SELECT UserId, MAX(Date) AS Date FROM Badges GROUP BY UserId) bh ON tu.UserId = bh.UserId
// ORDER BY tu.Reputation DESC, tu.DisplayName;
fn q8937(db: &'static So) -> String {
    let tu = ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), true);
    let tops: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, answer_count, .. } = &db.post;
    let s = (&tops).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(answer_count.opt())).opt()).fold([0i64; 3], |a, x| match x {
        Some((t, n)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + n.map_or(false, |n| n > 0) as i64],
        None => a,
    });
    let bh = db.badge.group_by(&db.badge.user).select(&db.badge.date).fold(i64::MIN, |m, d| m.max(d));
    let v = drain((&s).and(badges_of(db).select(&db.badge.name).opt()).and((&bh).opt()));
    rows(v.into_iter().map(|(u, ((a, b), d))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(ostr(db.user.location.get(u)));
        f.extend(a.map(V::I));
        f.extend([ostr(b), ots(d)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, RANK() OVER (ORDER BY COUNT(p.Id) DESC) AS PostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore FROM UserPostStats WHERE PostRank <= 10),
// CommentStatistics AS (SELECT p.OwnerUserId, COUNT(c.Id) AS CommentCount, AVG(LENGTH(c.Text)) AS AverageCommentLength FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.OwnerUserId)
// SELECT tu.DisplayName, tu.PostCount, tu.TotalScore, COALESCE(cs.CommentCount, 0) AS TotalComments, COALESCE(cs.AverageCommentLength, 0) AS AvgCommentLength,
//        CASE WHEN tu.TotalScore > 1000 THEN 'High' WHEN tu.TotalScore BETWEEN 500 AND 1000 THEN 'Medium' ELSE 'Low' END AS ScoreCategory
// FROM TopUsers tu LEFT JOIN CommentStatistics cs ON tu.UserId = cs.OwnerUserId ORDER BY tu.PostCount DESC, tu.TotalScore DESC;
fn q320(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = ranked(drain(&ups), |&(_, a)| Reverse(a[1]), false);
    let tops: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let cs = (&tops).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db).select(&db.comment.text).opt())).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + 1, a[2] + t.chars().count() as i64],
        None => [a[0] + 1, a[1], a[2]],
    });
    rows(drain((&tops).select((&ups).and((&cs).opt()))).into_iter().map(|(u, (a, c))| {
        let c = c.unwrap_or([0; 3]);
        let s = a[4];
        row(vec![
            user_col(db, u, "name"),
            V::I(a[1]),
            V::I(s),
            V::I(c[1]),
            if c[1] == 0 { V::F(0.0) } else { avg(c[2], c[1]) },
            V::S(if s > 1000 { "High" } else if s >= 500 { "Medium" } else { "Low" }),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT up.DisplayName, COUNT(DISTINCT rp.Id) AS TotalPosts, SUM(rp.Score) AS TotalScore, MAX(rp.ViewCount) AS MostViewedPost, ub.BadgeCount, ub.HighestBadgeClass
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id JOIN UserBadges ub ON up.Id = ub.UserId WHERE ub.BadgeCount > 0 AND rp.Rank <= 3
// GROUP BY up.DisplayName, ub.BadgeCount, ub.HighestBadgeClass ORDER BY TotalScore DESC, TotalPosts DESC LIMIT 10;
//
// CommentCount is never read.
fn q177(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64, i64::MIN], |a, c| [a[0] + 1, a[1].max(c)]);
    let g = (&tp)
        .group_by(owner_user.select((&db.user.display_name).and(&ub)))
        .select(score.and(view_count.opt()))
        .fold([0, 0, 0, i64::MIN], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3].max(w.unwrap_or(i64::MIN))]);
    let v = top_n(drain(&g), |&((n, b), a)| (Reverse(a[1]), Reverse(a[0]), n, b), 10);
    rows(v.into_iter().map(|((n, b), a)| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), omax(a[3], a[2]), V::I(b[0]), V::I(b[1])])))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(COALESCE(P.Score, 0)) AS TotalScore, ROW_NUMBER() OVER (ORDER BY SUM(COALESCE(P.Score, 0)) DESC) AS Rank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// QuestionHistory AS (SELECT PH.PostId, COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS ClosedCount, COUNT(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 END) AS ReopenedCount
//     FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE P.PostTypeId = 1 GROUP BY PH.PostId)
// SELECT UPS.UserId, UPS.DisplayName, UPS.TotalPosts, UPS.TotalQuestions, UPS.TotalAnswers, UPS.TotalScore, QH.ClosedCount, QH.ReopenedCount, UPS.TotalPosts - COALESCE(QH.ClosedCount, 0) AS ActivePosts
// FROM UserPostStats UPS LEFT JOIN QuestionHistory QH ON UPS.TotalQuestions = QH.ClosedCount WHERE UPS.TotalQuestions > 10
// ORDER BY UPS.TotalScore DESC, ActivePosts DESC FETCH FIRST 10 ROWS ONLY;
//
// The LEFT JOIN matches a user's question count against each question's close count.
fn q1669(db: &'static So) -> String {
    let ups = user_posts(db);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let qh = db
        .post_history
        .with(post.select(Ident::<Post>::new().with((&db.post.post_type_id).eq(1))))
        .group_by(post)
        .select(post_history_type_id)
        .fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let qv = rel(drain(&qh));
    let by_closed: HashIdx<i64, (Id<Post>, [i64; 2])> = (&qv).map(|(_, a)| a[0]).inv().select(&qv).collect();
    let v = drain((&ups).filt(|a| a[2] > 10).select(Same::<[i64; 10]>::new().and(Same::<[i64; 10]>::new().map(|a: [i64; 10]| a[2]).select(&by_closed).opt())));
    let v = top_n(v, |&(u, (a, q))| (Reverse(a[4]), Reverse(a[1] - q.map_or(0, |(_, c)| c[0])), u, q.map(|x| x.0)), 10);
    rows(v.into_iter().map(|(u, (a, q))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        f.extend(match q {
            Some((_, c)) => [V::I(c[0]), V::I(c[1]), V::I(a[1] - c[0])],
            None => [V::Null, V::Null, V::I(a[1])],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT rp.OwnerDisplayName, COUNT(rp.Id) AS PostCount, SUM(rp.Score) AS TotalScore, SUM(rp.ViewCount) AS TotalViews FROM RankedPosts rp WHERE rp.PostRank <= 5 GROUP BY rp.OwnerDisplayName),
// BadgedUsers AS (SELECT u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName)
// SELECT t.OwnerDisplayName, t.PostCount, t.TotalScore, t.TotalViews, COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM TopPosts t LEFT JOIN BadgedUsers b ON t.OwnerDisplayName = b.DisplayName ORDER BY t.TotalScore DESC, t.TotalViews DESC LIMIT 10;
fn q6964(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let name = &db.user.display_name;
    let t = (&tp).group_by(owner_user.select(name)).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let bu = db.user.group_by(name).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = top_n(drain((&t).and((&bu).opt())), |&(n, (a, _))| (Reverse(a[1]), a[2] == 0, Reverse(a[3]), n), 10);
    rows(v.into_iter().map(|(n, (a, b))| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(b.unwrap_or(0))])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerName, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId, p.Score),
// FilteredPosts AS (SELECT PostId, Title, CreationDate, OwnerName, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT fp.Title, fp.CreationDate, fp.OwnerName, fp.CommentCount, fp.UpVoteCount, fp.DownVoteCount, (fp.UpVoteCount - fp.DownVoteCount) AS NetVotes,
//        CASE WHEN fp.CommentCount > 0 THEN 'Active' ELSE 'Inactive' END AS ActivityState
// FROM FilteredPosts fp ORDER BY fp.UpVoteCount DESC, fp.CommentCount DESC;
//
// Rank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
fn q7869(db: &'static So) -> String {
    let score = &db.post.score;
    let top = top_per(drain(&db.post.post_type_id), |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&s).and(&cc)).into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::S(if c > 0 { "Active" } else { "Inactive" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.ViewCount) AS TotalViews, AVG(CASE WHEN v.BountyAmount IS NOT NULL THEN v.BountyAmount END) AS AvgBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, AvgBounty, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews,
//        RANK() OVER (ORDER BY Reputation DESC) AS RankByReputation FROM UserActivity)
// SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, AvgBounty, RankByViews, RankByReputation
// FROM TopUsers WHERE RankByViews <= 10 OR RankByReputation <= 10 ORDER BY RankByViews, RankByReputation;
fn q28405(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 7], |a, x| match x {
            Some(((t, w), b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + b.is_some() as i64, a[6] + b.unwrap_or(0)]
            }
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| (a[3] == 0, Reverse(a[4])), false);
    let v = ranked(v, |&((u, _), _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let mut v: Vec<_> = v.into_iter().filter(|&((_, w), r)| w <= 10 || r <= 10).collect();
    v.sort_by_key(|&((_, w), r)| (w, r));
    rows(v.into_iter().map(|(((u, a), w), r)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), avg(a[6], a[5]), V::I(w), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT US.UserId, US.DisplayName, US.Reputation, US.TotalPosts, US.Questions, US.Answers, US.Wikis, US.Upvotes, US.Downvotes,
//        DENSE_RANK() OVER (ORDER BY US.Reputation DESC) AS Rank FROM UserStats US)
// SELECT T.UserId, T.DisplayName, T.Reputation, T.TotalPosts, T.Questions, T.Answers, T.Wikis, T.Upvotes, T.Downvotes FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.Reputation DESC;
//
// Rank reads only Reputation, so the top users are picked first and the post x vote product is driven for those alone.
fn q6918(db: &'static So) -> String {
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let tops: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let s = (&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = (&tops).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&s).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(u.UpVotes) AS TotalUpVotes,
//        SUM(u.DownVotes) AS TotalDownVotes, MAX(p.CreationDate) AS LastPostDate FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, p.CommentCount, MAX(p.Score) AS MaxScore, MIN(p.CreationDate) AS FirstActivityDate,
//        MAX(p.LastActivityDate) AS LastActivityDate FROM Posts p GROUP BY p.Id, p.Title, p.ViewCount, p.AnswerCount, p.CommentCount),
// TopPosts AS (SELECT ps.PostId, ps.Title, us.DisplayName, us.PostCount, us.TotalScore, ROW_NUMBER() OVER (ORDER BY ps.MaxScore DESC) AS Rank
//     FROM PostStats ps JOIN UserStats us ON ps.PostId = us.UserId)
// SELECT tp.Rank, tp.Title, tp.DisplayName, tp.PostCount, tp.TotalScore FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Rank;
//
// ps.PostId = us.UserId compares a post id with a user id, so it joins on the raw ids.
fn q11262(db: &'static So) -> String {
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain(db.post.select((&db.post.origid).select(&uid)));
    let v = top_n(v, |&(p, _)| (Reverse(db.post.score.get(p).unwrap()), p), 10);
    let tp = rel(v);
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    type R = (Id<Post>, Id<User>);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(_, u): R| u).select(&us))));
    rows(v.into_iter().map(|(i, ((p, u), a))| row(vec![V::I(i as i64 + 1), title(db, p), user_col(db, u, "name"), V::I(a[0]), V::I(a[1])])))
}

// WITH PostStats AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount, STDDEV(P.Score) OVER() AS ScoreStdDev
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AveragePostScore
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName)
// SELECT PS.PostId, PS.Title, PS.Score, PS.ViewCount, PS.CommentCount, PS.VoteCount, US.UserId, US.DisplayName, US.BadgeCount, US.TotalViews, US.AveragePostScore
// FROM PostStats PS JOIN Users U ON PS.PostId = U.Id JOIN UserStats US ON U.Id = US.UserId ORDER BY PS.Score DESC, PS.ViewCount DESC;
//
// PS.PostId = U.Id compares a post id with a user id, so it joins on the raw ids. ScoreStdDev is never read.
fn q11931(db: &'static So) -> String {
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ps = db
        .post
        .with((&db.post.origid).select(&uid))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let Post { view_count, score, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(score.and(view_count.opt())).opt()))
        .fold([0i64; 5], |a, (b, p)| match p {
            Some((s, w)) => [a[0] + b.is_some() as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + 1, a[4] + s],
            None => [a[0] + b.is_some() as i64, a[1], a[2], a[3], a[4]],
        });
    rows(drain((&ps).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))).into_iter().map(|(p, (c, (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c[0]), V::I(c[1])]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(a[0]), nullable(a[2], a[1]), avg(a[4], a[3])]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, (SELECT COUNT(*) FROM Posts AS a WHERE a.AcceptedAnswerId = p.Id) AS AcceptedAnswerCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// RankedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.AcceptedAnswerCount,
//        RANK() OVER (ORDER BY rp.Score DESC, rp.CommentCount DESC) AS PostRank FROM RecentPosts rp)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.AcceptedAnswerCount, rp.PostRank FROM RankedPosts rp WHERE rp.PostRank <= 10 ORDER BY rp.PostRank;
fn q7744(db: &'static So) -> String {
    let Post { creation_date, accepted_answer, score, .. } = &db.post;
    let acc: HashIdx<Id<Post>, Id<Post>> = accepted_answer.inv().collect();
    let s = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ac = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(Ident::<Post>::new()).select((&acc).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let v = ranked(drain((&s).and(&ac)), |&(p, (a, _))| (Reverse(score.get(p).unwrap()), Reverse(a[0])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, (a, n)), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(a.map(V::I));
        f.extend([V::I(n), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.PostRank <= 10)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.UpVoteCount, tp.DownVoteCount, COALESCE(b.Name, 'No Badge') AS UserBadge, u.DisplayName AS AuthorDisplayName
// FROM TopPosts tp JOIN Posts p ON tp.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId AND b.Class = 1 ORDER BY tp.Score DESC;
//
// PostRank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
fn q5548(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let v = top_n(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))).select(&db.badge.name);
    rows(drain((&s).and(owner_user.select(Ident::<User>::new().and(gold.opt())))).into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(b.unwrap_or("No Badge")), user_col(db, u, "name")]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND p.Score >= 0 GROUP BY p.Id, p.Title, p.Score, p.PostTypeId, p.CreationDate),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes, u.DisplayName AS OwnerDisplayName, u.Reputation, u.Location
// FROM TopPosts tp JOIN Users u ON tp.PostId IN (SELECT Id FROM Posts WHERE OwnerUserId = u.Id) ORDER BY tp.Score DESC, tp.CommentCount DESC;
//
// Rank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
// The IN matches each post to its owner.
fn q6940(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(score.ge(0))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&s).and(owner_user)).into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(ostr(db.user.location.get(u)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount > 100),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score FROM RankedPosts rp WHERE rp.Rank <= 5),
// UserPostCounts AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN p.Score <= 0 THEN 1 ELSE 0 END) AS NegativePosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id)
// SELECT u.DisplayName, u.Reputation, up.PostCount, up.PositivePosts, up.NegativePosts, tp.Title AS TopPostTitle, tp.ViewCount, tp.Score
// FROM Users u JOIN UserPostCounts up ON u.Id = up.UserId LEFT JOIN TopPosts tp ON up.PostCount > 0 ORDER BY u.Reputation DESC, up.PostCount DESC;
//
// The LEFT JOIN's ON names only up: a user with posts meets every top post, one without gets the NULL row.
fn q9830(db: &'static So) -> String {
    let Post { creation_date, view_count, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(view_count.gt(100)).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p).unwrap()), p), 5, false);
    let tv = rel(top.into_iter().map(|(p, _)| (true, p)).collect());
    let tp: HashIdx<bool, (bool, Id<Post>)> = (&tv).map(|(b, _)| b).inv().select(&tv).collect();
    let up = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt()).fold([0i64; 3], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s <= 0) as i64],
        None => a,
    });
    let v = drain((&up).and((&up).map(|a| a[0] > 0).select(&tp).opt()));
    rows(v.into_iter().map(|(u, (a, t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match t {
            Some((_, p)) => post_fields(db, p, &["title", "views", "score"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalViews, TotalScore, ROW_NUMBER() OVER (ORDER BY TotalViews DESC) AS ViewRank, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT tu.DisplayName, tu.PostCount, tu.TotalViews, tu.TotalScore, CASE WHEN tu.ViewRank < 11 THEN 'Top Viewers' ELSE 'Average Viewers' END AS ViewStatus,
//        CASE WHEN tu.ScoreRank < 11 THEN 'Top Scorers' ELSE 'Average Scorers' END AS ScoreStatus, COALESCE(b.BadgeCount, 0) AS NumberOfBadges
// FROM TopUsers tu LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON tu.UserId = b.UserId WHERE tu.PostCount > 0
// ORDER BY tu.TotalScore DESC, tu.TotalViews DESC LIMIT 20;
fn q4368(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = top_n(drain(&ups), |&(u, a)| (Reverse(a[6]), u), 0);
    let v: Vec<_> = v.into_iter().enumerate().map(|(i, x)| (x, i as i64 + 1)).collect();
    let v = top_n(v, |&((u, a), _)| (Reverse(a[4]), u), 0);
    let v: Vec<_> = v.into_iter().enumerate().map(|(i, x)| (x, i as i64 + 1)).collect();
    let rk = rel(v.into_iter().map(|(((u, a), w), s)| (u, (a, w, s))).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, ([i64; 10], i64, i64))> = (&rk).map(|(u, _)| u).inv().select(&rk).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&by_user).map(|(_, x)| x).filt(|(a, _, _)| a[1] > 0).and((&bc).opt()));
    let v = top_n(v, |&(u, ((a, _, _), _))| (Reverse(a[4]), Reverse(a[6]), u), 20);
    rows(v.into_iter().map(|(u, ((a, w, s), b))| {
        row(vec![
            user_col(db, u, "name"),
            V::I(a[1]),
            V::I(a[6]),
            V::I(a[4]),
            V::S(if w < 11 { "Top Viewers" } else { "Average Viewers" }),
            V::S(if s < 11 { "Top Scorers" } else { "Average Scorers" }),
            V::I(b.unwrap_or(0)),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.LastActivityDate, p.PostTypeId, p.Score),
// PopularPosts AS (SELECT PostId, Title, CreationDate, LastActivityDate, CommentCount, UpVotes, DownVotes, Rank FROM RankedPosts WHERE Rank <= 10)
// SELECT pp.PostId, pp.Title, pp.CreationDate, pp.LastActivityDate, pp.CommentCount, pp.UpVotes, pp.DownVotes, pt.Name AS PostTypeName
// FROM PopularPosts pp JOIN PostTypes pt ON pp.Rank = pt.Id ORDER BY pp.UpVotes DESC, pp.CreationDate DESC;
//
// Rank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
// pp.Rank = pt.Id joins a row number to a post type id, so it goes through the raw id.
fn q5324(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let v = ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), false);
    let v = per_group(v, |&(_, t)| t);
    let tr = rel(v.into_iter().filter(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    let rk: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let tp: MatSet<Id<Post>> = (&tr).map(|(p, _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ptid: HashIdx<i64, Str> = (&db.post_type.origid).inv().select(&db.post_type.name).collect();
    rows(drain((&s).and((&rk).map(|(_, r)| r).select(&ptid))).into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity"]);
        f.extend(a.map(V::I));
        f.push(V::S(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS Author, p.CreationDate, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.Body, p.Tags, u.DisplayName, p.CreationDate),
// PopularPosts AS (SELECT rp.*, (SELECT COUNT(*) FROM Votes WHERE PostId = rp.PostId AND VoteTypeId = 2) AS UpVotes FROM RankedPosts rp WHERE rp.Rank = 1 AND rp.CommentCount > 10)
// SELECT pp.PostId, pp.Title, pp.Body, pp.Tags, pp.Author, pp.CreationDate, pp.CommentCount, pp.VoteCount, pp.UpVotes,
//        CASE WHEN pp.UpVotes >= 50 THEN 'Hot' WHEN pp.UpVotes >= 20 THEN 'Trending' ELSE 'Regular' END AS PopularityStatus
// FROM PopularPosts pp ORDER BY pp.UpVotes DESC, pp.CommentCount DESC;
//
// Rank partitions by p.Id, so it is always 1.
fn q27361(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let base = || db.post.with(post_type_id.is_in([1, 2])).with(owner_user);
    let s = base().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = base().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    rows(drain((&s).filt(|n| n > 10).and(&vc)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "owner", "created"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(if a[1] >= 50 { "Hot" } else if a[1] >= 20 { "Trending" } else { "Regular" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, U.DisplayName AS OwnerDisplayName, P.CreationDate, P.ViewCount, P.AnswerCount, P.Score,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, ViewCount, AnswerCount, Score FROM RankedPosts WHERE PostRank <= 10)
// SELECT TP.Title, TP.OwnerDisplayName, TP.CreationDate, TP.ViewCount, TP.AnswerCount, TP.Score, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM TopPosts TP LEFT JOIN Comments C ON TP.PostId = C.PostId LEFT JOIN Votes V ON TP.PostId = V.PostId
// GROUP BY TP.PostId, TP.Title, TP.OwnerDisplayName, TP.CreationDate, TP.ViewCount, TP.AnswerCount, TP.Score ORDER BY TP.Score DESC, TP.ViewCount DESC;
fn q7985(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "views", "answers", "score"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostSummary AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(P.Score) AS TotalScore FROM Posts P GROUP BY P.OwnerUserId),
// CloseReasonCounts AS (SELECT PH.UserId, COUNT(PH.Id) AS CloseReasonVotes FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.UserId)
// SELECT UR.DisplayName, UR.Reputation, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.Questions, 0) AS Questions, COALESCE(PS.Answers, 0) AS Answers,
//        COALESCE(PS.TotalScore, 0) AS TotalScore, COALESCE(CR.CloseReasonVotes, 0) AS CloseReasonVotes
// FROM UserReputation UR LEFT JOIN PostSummary PS ON UR.UserId = PS.OwnerUserId LEFT JOIN CloseReasonCounts CR ON UR.UserId = CR.UserId
// WHERE UR.ReputationRank <= 50 ORDER BY UR.Reputation DESC, UR.DisplayName;
fn q3299(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 50);
    let tops: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ups = user_posts(db);
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let cr = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&tops).select((&ups).and((&cr).opt()))).into_iter().map(|(u, (a, c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH PostSummary AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount,
//        COUNT(DISTINCT CASE WHEN b.Id IS NOT NULL THEN b.Id END) AS BadgeCount, ARRAY_AGG(DISTINCT t.TagName) AS Tags
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId LEFT JOIN Tags t ON t.ExcerptPostId = p.Id
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// PostHistorySummary AS (SELECT PostId, COUNT(*) AS HistoryActionCount, MAX(CreationDate) AS LastActivityDate FROM PostHistory GROUP BY PostId)
// SELECT ps.PostId, ps.Title, ps.CreationDate AS PostCreationDate, ps.Score, ps.ViewCount, ps.CommentCount, ps.VoteCount, ps.BadgeCount, ps.Tags, phs.HistoryActionCount, phs.LastActivityDate
// FROM PostSummary ps LEFT JOIN PostHistorySummary phs ON ps.PostId = phs.PostId ORDER BY ps.CreationDate DESC FETCH FIRST 100 ROWS ONLY;
//
// The ORDER BY reads only CreationDate, so the newest posts are picked first and the comment x vote x badge x tag product is driven for those alone.
fn q12950(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = top_n(drain(creation_date), |&(p, d)| (Reverse(d), p), 100);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let cc = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(owner_user.select(badges_of(db)).opt()).and((&excerpt).opt()))
        .fold(0i64, |n, (((c, _), _), _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.user_id).opt()).opt()).buf_fold(|v| distinct_some(v.iter().map(|x| x.flatten())));
    let bc = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let tg = (&tp).group_by(Ident::<Post>::new()).select((&excerpt).select(&db.tag.tag_name).opt()).buf_fold(|v| {
        let mut x: Vec<Option<Str>> = v.iter().copied().collect();
        x.sort_unstable();
        x.dedup();
        &*Box::leak(x.into_boxed_slice())
    });
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    rows(drain((&cc).and(&vc).and(&bc).and(&tg).and((&phs).opt())).into_iter().map(|(p, ((((c, n), b), t), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n), V::I(b), V::L(t.iter().map(|&x| ostr(x)).collect())]);
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank <= 10),
// UserVotes AS (SELECT v.UserId, COUNT(v.Id) AS VoteCount FROM Votes v WHERE v.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY v.UserId)
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, COALESCE(uv.VoteCount, 0) AS VoteCount
// FROM Users u JOIN TopPosts tp ON u.Id = tp.PostId LEFT JOIN UserVotes uv ON u.Id = uv.UserId ORDER BY tp.Score DESC, tp.ViewCount DESC, uv.VoteCount DESC;
//
// u.Id = tp.PostId compares a user id with a post id, so it joins on the raw ids.
fn q8235(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(t0, -1)).and(post_type_id.is_in([1, 2]))).select(&db.post.score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let Vote { user, creation_date: vd, .. } = &db.vote;
    let uv = db.vote.with(vd.ge(add_years(t0, -1))).group_by(user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    rows(drain((&cc).and(origid.select(&uid).select(Ident::<User>::new().and((&uv).opt())))).into_iter().map(|(p, (c, (u, n)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(c), V::I(n.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount FROM Votes v WHERE v.VoteTypeId IN (2, 3) GROUP BY v.PostId),
// CombinedData AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, COALESCE(rv.VoteCount, 0) AS VoteCount, rp.CommentCount FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId)
// SELECT cd.PostId, cd.Title, cd.Body, cd.Tags, cd.VoteCount, cd.CommentCount,
//        CASE WHEN cd.VoteCount > 10 THEN 'Hot Topic' WHEN cd.CommentCount > 5 THEN 'Engaging Question' ELSE 'Needs Improvement' END AS PostType
// FROM CombinedData cd WHERE cd.CommentCount > 0 ORDER BY cd.VoteCount DESC, cd.CommentCount DESC LIMIT 50;
fn q26186(db: &'static So) -> String {
    let qs = || db.post.with((&db.post.post_type_id).eq(1));
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rv = db.vote.with((&db.vote.vote_type_id).is_in([2, 3])).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&cc).filt(|n| n > 0).and((&rv).opt())), |&(p, (c, n))| (Reverse(n.unwrap_or(0)), Reverse(c), p), 50);
    rows(v.into_iter().map(|(p, (c, n))| {
        let n = n.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "body", "tags"]);
        f.extend([V::I(n), V::I(c), V::S(if n > 10 { "Hot Topic" } else if c > 5 { "Engaging Question" } else { "Needs Improvement" })]);
        row(f)
    }))
}

// WITH UserPosts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS TotalWikis, SUM(p.ViewCount) AS TotalViews,
//        AVG(p.Score) AS AvgScore, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalWikis, TotalViews, AvgScore, TotalComments,
//        RANK() OVER (ORDER BY TotalPosts DESC, TotalViews DESC) AS UserRank FROM UserPosts)
// SELECT u.DisplayName, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalWikis, u.TotalViews, u.AvgScore, u.TotalComments, ROW_NUMBER() OVER (ORDER BY u.UserRank) AS RankedPosition
// FROM TopUsers u WHERE u.UserRank <= 10 ORDER BY u.UserRank;
fn q6071(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score).and(comments_of(db).opt())).opt())
        .fold([0i64; 7], |a, x| match x {
            Some((((t, w), s), _)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 3 | 4 | 5) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + s],
            None => a,
        });
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&s).and(&cc)), |&(_, (a, _))| (Reverse(a[0]), a[4] == 0, Reverse(a[5])), false);
    let v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).collect();
    rows(v.into_iter().enumerate().map(|(i, ((u, (a, c)), _))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4]), avg(a[6], a[0]), V::I(c), V::I(i as i64 + 1)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, u.DisplayName, p.OwnerUserId, p.CreationDate),
// FilteredPosts AS (SELECT rp.Id, rp.Title, rp.OwnerDisplayName, rp.CommentCount, rp.AnswerCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.PostRank <= 10)
// SELECT fp.Title, fp.OwnerDisplayName, fp.CommentCount, fp.AnswerCount, (fp.UpVotes - fp.DownVotes) AS NetVotes FROM FilteredPosts fp ORDER BY NetVotes DESC, fp.CommentCount DESC;
//
// PostRank reads only base columns, so each owner's ten newest questions are picked first and the comment x answer x vote product is driven for those alone.
fn q8914(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold(0i64, |n, (_, t)| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&s).and(&cc).and(&ac)).into_iter().map(|(p, ((s, c), a))| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend([V::I(c), V::I(a), V::I(s)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("13439", q13439),
    ("8069", q8069),
    ("6429", q6429),
    ("28468", q28468),
    ("8382", q8382),
    ("14925", q14925),
    ("9588", q9588),
    ("6922", q6922),
    ("9326", q9326),
    ("8552", q8552),
    ("6213", q6213),
    ("6074", q6074),
    ("8868", q8868),
    ("13009", q13009),
    ("5750", q5750),
    ("10069", q10069),
    ("6503", q6503),
    ("5332", q5332),
    ("6102", q6102),
    ("8056", q8056),
    ("11127", q11127),
    ("8299", q8299),
    ("1443", q1443),
    ("9037", q9037),
    ("9764", q9764),
    ("7111", q7111),
    ("5244", q5244),
    ("14983", q14983),
    ("7183", q7183),
    ("9815", q9815),
    ("11992", q11992),
    ("6729", q6729),
    ("8856", q8856),
    ("5740", q5740),
    ("5934", q5934),
    ("5214", q5214),
    ("5527", q5527),
    ("8164", q8164),
    ("1545", q1545),
    ("6026", q6026),
    ("7864", q7864),
    ("26340", q26340),
    ("6734", q6734),
    ("10555", q10555),
    ("13239", q13239),
    ("608", q608),
    ("9663", q9663),
    ("5512", q5512),
    ("9312", q9312),
    ("6023", q6023),
    ("6962", q6962),
    ("9199", q9199),
    ("5292", q5292),
    ("29228", q29228),
    ("4171", q4171),
    ("7969", q7969),
    ("7193", q7193),
    ("12802", q12802),
    ("1968", q1968),
    ("4970", q4970),
    ("7979", q7979),
    ("9414", q9414),
    ("825", q825),
    ("809", q809),
    ("5411", q5411),
    ("9233", q9233),
    ("9536", q9536),
    ("3525", q3525),
    ("26968", q26968),
    ("7570", q7570),
    ("10880", q10880),
    ("8937", q8937),
    ("320", q320),
    ("177", q177),
    ("1669", q1669),
    ("6964", q6964),
    ("7869", q7869),
    ("28405", q28405),
    ("6918", q6918),
    ("11262", q11262),
    ("11931", q11931),
    ("7744", q7744),
    ("5548", q5548),
    ("6940", q6940),
    ("9830", q9830),
    ("4368", q4368),
    ("5324", q5324),
    ("27361", q27361),
    ("7985", q7985),
    ("3299", q3299),
    ("12950", q12950),
    ("8235", q8235),
    ("26186", q26186),
    ("6071", q6071),
    ("8914", q8914),
];
