use harness::prelude::*;
use std::cmp::Reverse;

type UK = (Id<User>, i64);
type PK = (Id<Post>, i64);

/// RANK() OVER (ORDER BY Reputation DESC) over every user, at most n, keyed by the user.
fn rep_rank(db: &'static So, n: i64) -> HashIdx<Id<User>, i64> {
    let w: MatSet<UK> = whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation))
        .window(rank, |(_, r)| r, desc)
        .filt(move |(_, k)| k <= n)
        .map(|((u, _), k)| (u, k))
        .collect();
    (&w).map(|x: UK| x.0).inv().map(|x: UK| x.1).collect()
}

/// ROW_NUMBER() OVER (ORDER BY Reputation DESC) over every user, at most n, keyed by the user.
fn rep_row(db: &'static So, n: i64) -> HashIdx<Id<User>, i64> {
    let w: MatSet<UK> = whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation))
        .window(row_number, |(u, r)| (r, Reverse(u)), desc)
        .filt(move |(_, k)| k <= n)
        .map(|((u, _), k)| (u, k))
        .collect();
    (&w).map(|x: UK| x.0).inv().map(|x: UK| x.1).collect()
}

/// A set of (post, rank) rows keyed by the post.
fn by_post(w: &MatSet<PK>) -> HashIdx<Id<Post>, i64> {
    w.map(|x: PK| x.0).inv().map(|x: PK| x.1).collect()
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, u.DisplayName, p.PostTypeId, p.CreationDate),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.rn <= 5)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes,
//        CASE WHEN tp.UpVotes - tp.DownVotes > 0 THEN 'Positive' WHEN tp.UpVotes - tp.DownVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM TopPosts tp ORDER BY tp.CommentCount DESC, tp.UpVotes DESC;
//
// rn reads only base columns, so the five newest posts of each type are picked first and the comment x vote product is driven for those alone.
fn q5377(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend(a.map(V::I));
        let d = a[1] - a[2];
        f.push(V::S(if d > 0 { "Positive" } else if d < 0 { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND p.PostTypeId = 1),
// PostStats AS (SELECT rp.Id, rp.Title, rp.OwnerDisplayName, rp.ViewCount, rp.Score, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount, COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount
//     FROM RecentPosts rp LEFT JOIN Comments c ON rp.Id = c.PostId LEFT JOIN Votes v ON rp.Id = v.PostId GROUP BY rp.Id, rp.Title, rp.OwnerDisplayName, rp.ViewCount, rp.Score),
// RankedPosts AS (SELECT ps.*, RANK() OVER (ORDER BY ps.Score DESC, ps.ViewCount DESC) AS RankScore, RANK() OVER (ORDER BY ps.CommentCount DESC) AS RankComments FROM PostStats ps)
// SELECT rp.Id, rp.Title, rp.OwnerDisplayName, rp.ViewCount, rp.Score, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, rp.RankScore, rp.RankComments
// FROM RankedPosts rp WHERE rp.RankScore <= 10 OR rp.RankComments <= 10 ORDER BY rp.RankScore, rp.RankComments;
fn q7599(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let Vote { vote_type_id, user_id, .. } = &db.vote;
    let rp = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.eq(1))).with(owner_user);
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let voters = |t: i64| votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(t))).select(user_id);
    let up = rp().group_by(Ident::<Post>::new()).select(voters(2)).count_distinct();
    let down = rp().group_by(Ident::<Post>::new()).select(voters(3)).count_distinct();
    type R = (((((Id<Post>, i64), Option<i64>), Option<i64>), i64), Option<i64>);
    let w = whole(&cc)
        .select(Ident::<Post>::new().and(&cc).and((&up).opt()).and((&down).opt()).and(score).and(view_count.opt()))
        .window(rank, |((_, s), v): R| (s, v), desc)
        .window(rank, |((((((_, c), _), _), _), _), _): (R, i64)| c, desc);
    rows(drain((&w).filt(|((_, s), c)| s <= 10 || c <= 10)).into_iter().map(|(_, (((((((p, c), u), d), _), _), s), r))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "views", "score"]);
        f.extend([V::I(c), V::I(u.unwrap_or(0)), V::I(d.unwrap_or(0)), V::I(s), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, WikiCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.WikiCount, tu.UpVotes, tu.DownVotes
// FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Reputation DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the post x vote product is driven for them alone.
fn q6637(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, x| match x {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64],
            None => a,
        });
    rows(drain(&s).into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.UpVotes, tp.DownVotes, COUNT(ph.Id) AS HistoryChangeCount
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.UpVotes, tp.DownVotes
// ORDER BY tp.Score DESC, tp.CreationDate ASC;
fn q9471(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    rows(drain((&vs).and(&hc)).into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(h)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Owner, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopScoringPosts AS (SELECT rp.PostId, rp.Title, rp.Owner, rp.CreationDate, rp.Score FROM RankedPosts rp WHERE rp.Rank <= 5),
// CommentsCount AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments FROM Comments c GROUP BY c.PostId),
// PostDetails AS (SELECT tsp.PostId, tsp.Title, tsp.Owner, tsp.CreationDate, tsp.Score, COALESCE(cc.TotalComments, 0) AS CommentCount
//     FROM TopScoringPosts tsp LEFT JOIN CommentsCount cc ON tsp.PostId = cc.PostId)
// SELECT pd.Title, pd.Owner, pd.CreationDate, pd.Score, pd.CommentCount, EXTRACT(YEAR FROM pd.CreationDate) AS PostYear
// FROM PostDetails pd WHERE pd.CommentCount > 0 ORDER BY pd.Score DESC, pd.CreationDate DESC LIMIT 10;
fn q6268(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (s, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = top_n(drain(&cc), |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score"]);
        f.extend([V::I(n), V::I(year(creation_date.get(p).unwrap()))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.Title, tp.OwnerDisplayName, tp.Score, tp.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, (SELECT COUNT(*) FROM Badges b WHERE b.UserId = u.Id) AS BadgeCount
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId LEFT JOIN Users u ON tp.OwnerDisplayName = u.DisplayName
// GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.Score, tp.ViewCount, u.Id ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q9831(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (s, w, Reverse(p)), desc)
        .filt(|(_, k)| k <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let pu: MatSet<(Id<Post>, Id<User>)> = (&tp).select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name).select(&by_name))).collect();
    type PU = (Id<Post>, Id<User>);
    let s = (&pu)
        .group_by(Same::<PU>::new())
        .select(Same::<PU>::new().map(|(p, _): PU| p).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&s).and(Same::<PU>::new().map(|(_, u): PU| u).select(&bc)));
    rows(v.into_iter().map(|((p, _), (a, b))| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.CreationDate, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.ViewCount DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Posts A ON P.Id = A.ParentId JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1
//     GROUP BY P.Id, P.Title, P.ViewCount, P.CreationDate, U.DisplayName, P.OwnerUserId),
// FilteredPosts AS (SELECT RP.PostId, RP.Title, RP.ViewCount, RP.CreationDate, RP.OwnerDisplayName, RP.CommentCount, RP.AnswerCount, RANK() OVER (ORDER BY RP.ViewCount DESC) AS GlobalRank
//     FROM RankedPosts RP WHERE RP.Rank <= 3)
// SELECT FP.PostId, FP.Title, FP.ViewCount, FP.CreationDate, FP.OwnerDisplayName, FP.CommentCount, FP.AnswerCount, FP.GlobalRank FROM FilteredPosts FP ORDER BY FP.GlobalRank;
//
// Rank reads only base columns, so each owner's three questions are picked first and the comment x answer product is driven for those alone.
fn q26163(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, view_count, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(view_count.opt()).and(creation_date))
        .window(row_number, |((p, w), d)| (w, d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 3)
        .map(|(((p, _), _), _)| p)
        .collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).opt()))
        .fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    let w = whole(&s).select(Ident::<Post>::new().and(&s).and(view_count.opt())).window(rank, |(_, w)| w, desc);
    rows(drain(&w).into_iter().map(|(_, (((p, a), _), r))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(P.ViewCount) AS TotalViews, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, Upvotes, Downvotes, RANK() OVER (ORDER BY TotalAnswers DESC, Upvotes DESC) AS Rank FROM UserStatistics)
// SELECT TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.TotalViews, TU.Upvotes, TU.Downvotes, TU.Rank FROM TopUsers TU WHERE TU.Rank <= 10 ORDER BY TU.Rank;
fn q5296(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let us = || db.user.with((&db.user.reputation).gt(1000));
    let s = us()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, x| match x {
            Some(((t, w), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64],
            None => a,
        });
    let dp = us().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&s).select(Ident::<User>::new().and(&s).and(&dp)).window(rank, |((_, a), _)| (a[1], a[4]), desc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, a), d), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(d), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[4]), V::I(a[5]), V::I(r)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// ActiveUsers AS (SELECT P.OwnerUserId AS UserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount FROM Posts P
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR' GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT U.UserId, U.DisplayName, U.BadgeCount, A.PostCount, A.TotalScore, A.AvgViewCount, ROW_NUMBER() OVER (ORDER BY A.TotalScore DESC, A.PostCount DESC) AS Rank
//     FROM UserBadges U JOIN ActiveUsers A ON U.UserId = A.UserId)
// SELECT UserId, DisplayName, BadgeCount, PostCount, TotalScore, AvgViewCount, Rank FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q5778(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let au = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let w = whole(&au).select(Ident::<User>::new().and(&au).and(&bc)).window(row_number, |((u, a), _)| (a[1], a[0], Reverse(u)), desc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, a), b), k))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
//        COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        u.DisplayName AS OwnerDisplayName, p.PostTypeId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT t.PostId, t.Title, t.CreationDate, t.Score, t.ViewCount, t.CommentCount, t.UpVotes, t.DownVotes, t.OwnerDisplayName, pt.Name AS PostTypeName
// FROM TopPosts t JOIN PostTypes pt ON t.PostTypeId = pt.Id ORDER BY t.Score DESC, t.CreationDate DESC;
//
// Rank reads only base columns, so the five posts of each type are picked first and the comment x vote product is driven for those alone.
fn q6358(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (s, d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&s).and(ptype_name(db))).into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend([post_fields(db, p, &["owner"]).remove(0), V::S(t)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(P.Score) AS AvgScore, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, AvgScore, TotalBounty, RANK() OVER (ORDER BY PostCount DESC) AS PostRank,
//        RANK() OVER (ORDER BY QuestionCount DESC) AS QuestionRank, RANK() OVER (ORDER BY AvgScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, AvgScore, TotalBounty, PostRank, QuestionRank, ScoreRank, (PostRank + QuestionRank + ScoreRank) AS OverallRank
// FROM TopUsers WHERE (PostRank + QuestionRank + ScoreRank) <= 10 ORDER BY OverallRank;
fn q8636(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(bounty.opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((t, s), b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + b.unwrap_or(0)]
            }
            None => a,
        });
    let mean = |a: [i64; 5]| if a[0] == 0 { None } else { Some(a[3] as f64 / a[0] as f64) };
    type R = (Id<User>, [i64; 5]);
    let w = whole(&s)
        .select(Ident::<User>::new().and(&s))
        .window(rank, |(_, a): R| a[0], desc)
        .window(rank, |((_, a), _): (R, i64)| a[1], desc)
        .window(rank, move |(((_, a), _), _): ((R, i64), i64)| mean(a).map(fkey), desc);
    rows(drain((&w).filt(|(((_, p), q), s)| p + q + s <= 10)).into_iter().map(|(_, ((((u, a), p), q), s))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), ofloat(mean(a)), V::I(a[4]), V::I(p), V::I(q), V::I(s), V::I(p + q + s)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount, COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, p.Score, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, ViewCount, CreationDate, Score, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.ViewCount, tp.CreationDate, tp.Score, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, u.DisplayName AS TopUser
// FROM TopPosts tp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the five posts of each type are picked first and the comment x vote product is driven for those alone.
fn q8819(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let Vote { vote_type_id, user_id, .. } = &db.vote;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (s, d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let voters = |t: i64| votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(t))).select(user_id);
    let up = (&tp).group_by(Ident::<Post>::new()).select(voters(2)).count_distinct();
    let down = (&tp).group_by(Ident::<Post>::new()).select(voters(3)).count_distinct();
    rows(drain((&cc).and((&up).opt()).and((&down).opt()).and(owner_user)).into_iter().map(|(p, (((c, u), d), o))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "score"]);
        f.extend([V::I(c), V::I(u.unwrap_or(0)), V::I(d.unwrap_or(0))]);
        f.push(user_col(db, o, "name"));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalCloseVotes,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId, p.Score)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.TotalCloseVotes, rp.Rank
// FROM RankedPosts rp JOIN PostTypes pt ON pt.Id = (SELECT MIN(PostTypeId) FROM Posts WHERE Id = rp.PostId) WHERE rp.Rank <= 10 ORDER BY rp.Rank, rp.CreationDate DESC;
//
// Rank reads only Score, so the ranked posts are picked first and the comment x vote x history product is driven for those alone.
fn q5677(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let tr: MatSet<PK> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| s, desc)
        .filt(|(_, k)| k <= 10)
        .map(|((p, _), k)| (p, k))
        .collect();
    let rank = by_post(&tr);
    let s = db
        .post
        .with(&rank)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 4], |a, ((c, t), h)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (h == Some(10)) as i64]);
    let v = drain((&s).and(&rank).and(ptype_name(db)));
    rows(v.into_iter().map(|(p, ((a, r), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, DENSE_RANK() OVER (ORDER BY p.ViewCount DESC, p.Score DESC) AS PopularityRank
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1)
// SELECT ru.DisplayName, ru.Reputation, ru.BadgeCount, pp.Title AS PopularPostTitle, pp.Score AS PostScore, pp.ViewCount AS PostViewCount, ru.UserRank, pp.PopularityRank
// FROM RankedUsers ru JOIN PopularPosts pp ON ru.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pp.PostId) WHERE ru.UserRank <= 10 ORDER BY ru.UserRank, pp.PopularityRank;
//
// UserRank reads only Reputation, so the top users are picked first and the badge x vote product is driven for them alone.
fn q6499(db: &'static So) -> String {
    let top = rep_rank(db, 10);
    let bc = db.user.with(&top).group_by(Ident::<User>::new()).select(badges_of(db).opt().and(votes_by(db).opt())).fold(0i64, |n, (b, _)| n + b.is_some() as i64);
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    type W = (((Id<Post>, Option<i64>), i64), i64);
    let pr = whole(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))))
        .select(Ident::<Post>::new().and(view_count.opt()).and(score))
        .window(dense_rank, |((_, w), s)| (w, s), desc);
    let v = drain(pr.select(Same::<W>::new().and(Same::<W>::new().map(|x: W| x.0 .0 .0).select(owner_user).select(Ident::<User>::new().and((&bc).and(&top))))));
    rows(v.into_iter().map(|(_, ((((p, _), _), pr), (u, (b, r))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(r), V::I(pr)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.CreationDate, COUNT(c.Id) AS TotalComments, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId AND v.VoteTypeId IN (8, 9)
// GROUP BY tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.CreationDate ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q5941(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (s, d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(bounty.opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "score", "views", "answers", "comments", "created"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(c.CommentCount) AS TotalComments, SUM(v.VoteCount) AS TotalVotes, MAX(p.CreationDate) AS LastActiveDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalComments, TotalVotes, LastActiveDate, RANK() OVER (ORDER BY PostCount DESC, TotalVotes DESC) AS Rank FROM UserActivity)
// SELECT t.UserId, t.DisplayName, t.PostCount, t.QuestionCount, t.AnswerCount, t.TotalComments, t.TotalVotes, t.LastActiveDate FROM TopUsers t WHERE t.Rank <= 10 ORDER BY t.Rank;
fn q6295(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let vc = db.vote.group_by(&db.vote.post_id).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and((&cc).opt()).and((&db.post.origid).select(&vc).opt()).and(creation_date)).opt())
        .fold([0, 0, 0, 0, 0, 0, 0, i64::MIN], |a, x| match x {
            Some((((t, c), v), d)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + c.unwrap_or(0), a[5] + v.is_some() as i64, a[6] + v.unwrap_or(0), a[7].max(d)],
            None => a,
        });
    let w = whole(&s).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| (a[0], if a[5] > 0 { Some(a[6]) } else { None }), desc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), nullable(a[6], a[5]), tmax(a[7])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        p.OwnerUserId FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation)
// SELECT up.UserId, up.Reputation, up.BadgeCount, up.GoldBadges, up.SilverBadges, up.BronzeBadges, rp.Id AS PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount
// FROM UserReputation up INNER JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId
// WHERE rp.Rank <= 5 AND EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = rp.Id AND v.UserId = up.UserId AND v.VoteTypeId = 2) ORDER BY up.Reputation DESC, rp.Score DESC;
fn q4751(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (s, d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let Vote { user, post, vote_type_id, .. } = &db.vote;
    let upself: MatSet<Id<Post>> = db.vote.with(vote_type_id.eq(2)).with(user.and(post.select(owner_user)).filt(|(a, b)| a == b)).select(post).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    rows(drain((&tp).with(&upself).select(owner_user.and(owner_user.select(&ub)))).into_iter().map(|(p, (u, b))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "answers"]));
        row(f)
    }))
}

// WITH VotesSummary AS (SELECT P.Id AS PostId, P.Title, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 END) AS TotalVotes FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, COALESCE(HS.TotalVotes, 0) AS TotalVotes, COALESCE(HS.UpVotes, 0) AS UpVotes, COALESCE(HS.DownVotes, 0) AS DownVotes,
//        U.DisplayName AS Author, P.CreationDate AS PostDate FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN VotesSummary HS ON P.Id = HS.PostId WHERE P.PostTypeId = 1),
// RankedPosts AS (SELECT PD.*, RANK() OVER (ORDER BY PD.Score DESC, PD.TotalVotes DESC, PD.ViewCount DESC) AS Rank FROM PostDetails PD)
// SELECT RP.Rank, RP.Title, RP.Author, RP.Score, RP.ViewCount, RP.UpVotes, RP.DownVotes, RP.PostDate FROM RankedPosts RP WHERE RP.Rank <= 10 ORDER BY RP.Rank;
fn q7926(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + matches!(t, Some(2 | 3)) as i64]);
    let w = whole(&s).select(Ident::<Post>::new().and(&s).and(score).and(view_count.opt())).window(rank, |(((_, a), s), w)| (s, a[2], w), desc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, ((((p, a), _), _), r))| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["title", "owner", "score", "views"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        AVG(P.Score) AS AverageScore, AVG(P.ViewCount) AS AverageViews FROM Posts P GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT UR.UserId, UR.DisplayName, UR.Reputation, PS.TotalPosts, PS.TotalQuestions, PS.TotalAnswers, PS.AverageScore, PS.AverageViews
//     FROM UserReputation UR JOIN PostStatistics PS ON UR.UserId = PS.OwnerUserId WHERE UR.ReputationRank <= 10)
// SELECT TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.AverageScore, TU.AverageViews, COALESCE(B.BadgeCount, 0) AS BadgeCount
// FROM TopUsers TU LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON TU.UserId = B.UserId ORDER BY TU.Reputation DESC;
fn q6701(db: &'static So) -> String {
    let tu = rep_rank(db, 10);
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    rows(drain(db.user.with(&tu).select((&ps).and((&bc).opt()))).into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), avg(a[5], a[4]), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(P.Score) AS TotalScore, AVG(V.BountyAmount) AS AverageBounty, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, AverageBounty, BadgeCount, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, AverageBounty, BadgeCount, ScoreRank, PostRank FROM TopUsers
// WHERE ScoreRank <= 10 OR PostRank <= 10 ORDER BY GREATEST(ScoreRank, PostRank);
fn q9410(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(bounty.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 6], |a, (p, _)| match p {
            Some(((t, s), b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0)]
            }
            None => a,
        });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type R = ((Id<User>, [i64; 6]), i64);
    let w = whole(&s)
        .select(Ident::<User>::new().and(&s).and(&bc))
        .window(rank, |((_, a), _): R| if a[0] > 0 { Some(a[3]) } else { None }, desc)
        .window(rank, |(((_, a), _), _): (R, i64)| a[0], desc);
    rows(drain((&w).filt(|((_, s), p)| s <= 10 || p <= 10)).into_iter().map(|(_, ((((u, a), b), s), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[0]), avg(a[5], a[4]), V::I(b), V::I(s), V::I(p)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(cnt.CommentsCount, 0) AS CommentsCount,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore, p.OwnerUserId
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentsCount FROM Comments GROUP BY PostId) AS cnt ON p.Id = cnt.PostId
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopContributors AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, RANK() OVER (ORDER BY SUM(p.Score) DESC) AS ScoreRank
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName)
// SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.CommentsCount, tc.DisplayName AS TopContributor, tc.TotalScore AS ContributorScore
// FROM RankedPosts r LEFT JOIN TopContributors tc ON r.OwnerUserId = tc.UserId WHERE r.RankByScore <= 5 ORDER BY r.CreationDate DESC FETCH FIRST 50 ROWS ONLY;
fn q4263(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| s, desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let tc = db.post.with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).group_by(owner_user).select(score).fold(0i64, |n, s| n + s);
    let tcu = Ident::<User>::new().and(&tc);
    let v = drain((&tp).select((&cc).opt().and(owner_user.select(tcu).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (c, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(c.unwrap_or(0)));
        f.extend(match t {
            Some((u, s)) => [user_col(db, u, "name"), V::I(s)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS RN,
//        p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(p.ViewCount) AS TotalViews, RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS PostRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.PositivePosts, us.TotalViews, rp.Title AS TopPostTitle, rp.CreationDate AS TopPostDate, rp.Score AS TopPostScore, rp.ViewCount AS TopPostViews
// FROM UserStatistics us LEFT JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId AND rp.RN = 1 WHERE us.Reputation > 1000 ORDER BY us.PostRank, us.Reputation DESC;
fn q9371(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let first: HashIdx<Id<User>, Id<Post>> = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (s, d, Reverse(p)), desc)
        .filt(|(_, k)| k == 1)
        .map(|(((p, _), _), _)| p)
        .collect();
    let ups = user_posts(db);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ups).and((&first).opt())));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[8]), nullable(a[6], a[5])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created", "score", "views"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(p.ViewCount) AS TotalViews,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.TotalPosts, ua.TotalViews, ua.TotalUpvotes, ua.TotalDownvotes, RANK() OVER (ORDER BY ua.Reputation DESC, ua.TotalViews DESC) AS UserRank
//     FROM UserActivity ua WHERE ua.TotalPosts > 0)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.TotalViews, tu.TotalUpvotes, tu.TotalDownvotes,
//        CASE WHEN tu.TotalUpvotes > tu.TotalDownvotes THEN 'Positive' WHEN tu.TotalUpvotes < tu.TotalDownvotes THEN 'Negative' ELSE 'Neutral' END AS UserSentiment
// FROM TopUsers tu WHERE tu.UserRank <= 10 ORDER BY tu.Reputation DESC;
fn q4910(db: &'static So) -> String {
    let Post { view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 4], |a, (w, t)| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let w = whole(&s)
        .select(Ident::<User>::new().and(&s).and(&dp).and(&db.user.reputation))
        .window(rank, |(((_, a), _), r)| (r, if a[0] > 0 { Some(a[1]) } else { None }), desc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, ((((u, a), d), _), _))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(d), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3])]);
        f.push(V::S(if a[2] > a[3] { "Positive" } else if a[2] < a[3] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, p.CommentCount, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS rn FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE u.Reputation > 1000),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, AnswerCount, CommentCount, ViewCount FROM RankedPosts WHERE rn <= 5),
// PostVoteSummary AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN Posts p ON v.PostId = p.Id GROUP BY p.Id)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.AnswerCount, tp.CommentCount, tp.ViewCount, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes
// FROM TopPosts tp LEFT JOIN PostVoteSummary pvs ON tp.PostId = pvs.PostId ORDER BY tp.CreationDate DESC;
fn q6869(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (s, d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    rows(drain((&tp).select((&pvs).opt())).into_iter().map(|(p, a)| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "answers", "comments", "views"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COUNT(DISTINCT bh.UserId) AS BadgeCount
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id LEFT JOIN Badges bh ON bh.UserId = p.OwnerUserId GROUP BY p.Id),
// TopPosts AS (SELECT ps.PostId, ps.CommentCount, ps.VoteCount, ps.UpVoteCount, ps.DownVoteCount, ps.AnswerCount, ps.BadgeCount,
//        ROW_NUMBER() OVER (ORDER BY ps.VoteCount DESC, ps.CommentCount DESC, ps.UpVoteCount DESC) AS Rank FROM PostStats ps)
// SELECT p.Id AS PostId, p.Title, p.OwnerDisplayName, p.CreationDate, tp.CommentCount, tp.VoteCount, tp.UpVoteCount, tp.DownVoteCount, tp.AnswerCount, tp.BadgeCount
// FROM TopPosts tp JOIN Posts p ON tp.PostId = p.Id WHERE tp.Rank <= 10 ORDER BY tp.Rank;
fn q8245(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, .. } = &db.post;
    let badges_by_id: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let ob = || owner_user_id.select(&badges_by_id);
    let s = db
        .post
        .group_by(Ident::<Post>::new())
        .select(post_type_id.and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(ob().opt()))
        .fold([0i64; 3], |a, (((t, _), v), _)| [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (t == 2) as i64]);
    let dc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let dv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let du = db.post.group_by(Ident::<Post>::new()).select(ob().select(&db.badge.user_id)).count_distinct();
    let v = top_n(drain((&s).and(&dc).and(&dv).and((&du).opt())), |&(p, (((a, c), v), _))| (Reverse(v), Reverse(c), Reverse(a[0]), p), 10);
    rows(v.into_iter().map(|(p, (((a, c), v), b))| {
        let mut f = post_fields(db, p, &["id", "title", "owner_name", "created"]);
        f.extend([V::I(c), V::I(v), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS Rank, u.DisplayName AS Author,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, pt.Name)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.Author, rp.CommentCount, rp.UpVotes, rp.DownVotes, pt.Name AS PostType
// FROM RankedPosts rp JOIN PostTypes pt ON pt.Name = (SELECT pt1.Name FROM PostTypes pt1 WHERE pt1.Id = (SELECT p1.PostTypeId FROM Posts p1 WHERE p1.Id = rp.PostId))
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC;
//
// Rank reads only base columns, so the five newest posts of each type are picked first and the comment x vote product is driven for those alone.
fn q9668(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user)
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let by_name: HashIdx<Str, Id<PostType>> = (&db.post_type.name).inv().collect();
    rows(drain((&s).and(ptype_name(db).select(&by_name))).into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(db.post_type.name.get(t).unwrap()));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId),
// BadgesSummary AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges B GROUP BY B.UserId)
// SELECT UR.DisplayName, UR.Reputation, PS.TotalPosts, PS.TotalQuestions, PS.TotalAnswers, PS.AverageScore, COALESCE(BS.GoldBadges, 0) AS GoldBadges, COALESCE(BS.SilverBadges, 0) AS SilverBadges,
//        COALESCE(BS.BronzeBadges, 0) AS BronzeBadges
// FROM UserReputation UR LEFT JOIN PostStatistics PS ON UR.UserId = PS.OwnerUserId LEFT JOIN BadgesSummary BS ON UR.UserId = BS.UserId WHERE UR.Reputation > 1000 ORDER BY UR.Rank, UR.Reputation DESC LIMIT 10;
fn q1508(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ps).opt().and((&bs).opt())));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotes, TotalDownvotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT T.DisplayName, T.TotalPosts, T.TotalQuestions, T.TotalAnswers, T.TotalUpvotes, T.TotalDownvotes, CASE WHEN T.ReputationRank <= 10 THEN 'Top User' ELSE 'Regular User' END AS UserType
// FROM TopUsers T WHERE T.TotalPosts > 0 ORDER BY T.TotalUpvotes DESC, T.TotalPosts DESC;
fn q7415(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&s).select(Ident::<User>::new().and(&s).and(&dp).and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    rows(drain((&w).filt(|(((_, d), _), _)| d > 0)).into_iter().map(|(_, ((((u, a), d), _), r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(d)];
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top User" } else { "Regular User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10),
// VotesSummary AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotesCount, COUNT(*) AS TotalVotesCount FROM Votes v GROUP BY v.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, vs.UpVotesCount, vs.DownVotesCount, vs.TotalVotesCount
// FROM TopPosts tp LEFT JOIN VotesSummary vs ON tp.PostId = vs.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q8353(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    rows(drain((&tp).select((&vs).opt())).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AvgPostScore, SUM(p.ViewCount) AS TotalViews, SUM(p.AnswerCount) AS TotalAnswers FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(ps.AvgPostScore, 0) AS AvgPostScore,
//        COALESCE(ps.TotalViews, 0) AS TotalViews, COALESCE(ps.TotalAnswers, 0) AS TotalAnswers, ROW_NUMBER() OVER (ORDER BY COALESCE(ps.TotalViews, 0) DESC) AS EngagementRank
//     FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT UserId, DisplayName, BadgeCount, PostCount, AvgPostScore, TotalViews, TotalAnswers, EngagementRank FROM UserEngagement WHERE EngagementRank <= 10 ORDER BY EngagementRank;
fn q8877(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, answer_count, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()).and(answer_count.opt()))
        .fold([0i64; 4], |a, ((s, w), n)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + n.unwrap_or(0)]);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let w = whole(&db.user.id)
        .select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt()))
        .window(row_number, |((u, _), a)| (a.map_or(0, |a| a[2]), Reverse(u)), desc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, b), a), k))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(b.unwrap_or(0)));
        f.extend(match a {
            Some(a) => [V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3])],
            None => [V::I(0), V::F(0.0), V::I(0), V::I(0)],
        });
        f.push(V::I(k));
        row(f)
    }))
}

// WITH TagStats AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        AVG(u.Reputation) AS AvgUserReputation FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' JOIN Users u ON p.OwnerUserId = u.Id GROUP BY t.TagName),
// RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, t.TagName, ROW_NUMBER() OVER (PARTITION BY t.TagName ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%' JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR')
// SELECT ts.TagName, ts.PostCount, ts.QuestionCount, ts.AnswerCount, ts.AvgUserReputation, rp.Title, rp.CreationDate, rp.Body, rp.OwnerDisplayName
// FROM TagStats ts LEFT JOIN RecentPosts rp ON ts.TagName = rp.TagName AND rp.rn = 1 ORDER BY ts.PostCount DESC, ts.TagName;
fn q28482(db: &'static So) -> String {
    let Post { owner_user, post_type_id, creation_date, .. } = &db.post;
    let lt = tag_mentions(db);
    type PT = (Id<Post>, Id<Tag>);
    let owned: MatSet<PT> = (&lt).with(Same::<PT>::new().map(|(p, _)| p).select(owner_user)).collect();
    let name = || Same::<PT>::new().map(|(_, t): PT| t).select(&db.tag.tag_name);
    let post = || Same::<PT>::new().map(|(p, _): PT| p);
    let ts_ = (&owned)
        .group_by(name())
        .select(post().select(post_type_id.and(owner_user.select(&db.user.reputation))))
        .fold([0i64; 4], |a, (t, r)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + r]);
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let latest: HashIdx<Str, Id<Post>> = (&owned)
        .with(post().select(creation_date.gt(since)))
        .group_by(name())
        .select(Same::<PT>::new().and(post().select(creation_date)))
        .window(row_number, |(x, d)| (d, Reverse(x)), desc)
        .filt(|(_, k)| k == 1)
        .map(|(((p, _), _), _)| p)
        .collect();
    rows(drain((&ts_).and((&latest).opt())).into_iter().map(|(n, (a, p))| {
        let mut f = vec![V::S(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created", "body", "owner"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.Reputation AS OwnerReputation, COUNT(c.Id) AS TotalComments,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2023-01-01'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.Reputation),
// Ranking AS (SELECT *, RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS ScoreRank, RANK() OVER (ORDER BY AnswerCount DESC) AS AnswerRank FROM PostStatistics)
// SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerReputation, TotalComments, TotalUpVotes, TotalDownVotes, ScoreRank, AnswerRank FROM Ranking ORDER BY ScoreRank, AnswerRank;
fn q14473(db: &'static So) -> String {
    let Post { creation_date, score, view_count, answer_count, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    type R = ((((Id<Post>, [i64; 3]), i64), Option<i64>), Option<i64>);
    let w = whole(&s)
        .select(Ident::<Post>::new().and(&s).and(score).and(view_count.opt()).and(answer_count.opt()))
        .window(rank, |(((_, s), w), _): R| (s, w), desc)
        .window(rank, |((_, n), _): (R, i64)| n, desc);
    rows(drain(&w).into_iter().map(|(_, ((((((p, a), _), _), _), s), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(s), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS WikiPosts,
//        SUM(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPosts, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, WikiPosts, ClosedPosts, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalPosts DESC, UpVotes - DownVotes DESC) AS UserRank FROM UserPostStats)
// SELECT tu.DisplayName, tu.TotalPosts, tu.Questions, tu.Answers, tu.WikiPosts, tu.ClosedPosts, tu.UpVotes, tu.DownVotes FROM TopUsers tu WHERE tu.UserRank <= 10 ORDER BY tu.UserRank;
fn q8225(db: &'static So) -> String {
    let Post { post_type_id, closed_date, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(closed_date.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 7], |a, x| match x {
            Some(((t, c), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64, a[4] + c.is_some() as i64, a[5] + (v == Some(2)) as i64, a[6] + (v == Some(3)) as i64],
            None => a,
        });
    let w = whole(&s).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| (a[0], a[5] - a[6]), desc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.Score DESC, p.ViewCount DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2022-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE rn = 1 ORDER BY Score DESC, ViewCount DESC LIMIT 10)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
// FROM TopPosts tp LEFT JOIN Users u ON tp.PostId = u.Id ORDER BY tp.Score DESC;
//
// rn is always 1 (one row per post), and the LIMIT reads only base columns, so the ten posts are picked first and the comment x vote product is driven for them alone.
// u.Id = tp.PostId compares a user id with a post id, so it goes through the raw ids.
fn q9678(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let v = top_n(drain(db.post.with(creation_date.ge(ts(2022, 1, 1, 0, 0, 0))).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    rows(drain((&s).and((&db.post.origid).select(&uidx).opt())).into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend(a.map(V::I));
        f.extend(match u {
            Some(u) => ucols(db, u, &["name", "rep"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, p.Score, p.Body, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Tags, p.CreationDate, p.Score, p.Body, u.DisplayName),
// TopTaggedPosts AS (SELECT rp.*, ROW_NUMBER() OVER (ORDER BY rp.Score DESC, rp.CreationDate ASC) AS OverallRank FROM RankedPosts rp WHERE rp.TagRank <= 3)
// SELECT ttp.PostId, ttp.Title, ttp.Tags, ttp.CreationDate, ttp.Score, ttp.Body, ttp.OwnerName, ttp.CommentCount, ttp.UpVotes, ttp.DownVotes
// FROM TopTaggedPosts ttp WHERE ttp.OverallRank <= 10 ORDER BY ttp.Score DESC, ttp.CreationDate ASC;
//
// Both ranks read only base columns, so the ten posts are picked first and the comment x vote product is driven for them alone.
fn q26652(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, score, creation_date, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), _)| (s, Reverse(p)), desc)
        .filt(|(_, k)| k <= 3)
        .map(|(x, _)| x);
    let v = top_n(drain(&w).into_iter().map(|x| x.1).collect(), |&((p, s), d)| (Reverse(s), d, p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "created", "score", "body", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(c.Id) DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.ViewCount) AS TotalViews, RANK() OVER (ORDER BY SUM(p.ViewCount) DESC) AS UserRank FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName)
// SELECT u.DisplayName AS TopUser, u.TotalViews, rp.Title AS TopPostTitle, rp.CommentCount, rp.UpVotes, rp.DownVotes
// FROM TopUsers u JOIN RankedPosts rp ON u.UserId = rp.PostId WHERE u.UserRank <= 5 AND rp.Rank = 1 ORDER BY u.TotalViews DESC, rp.UpVotes DESC FETCH FIRST 10 ROWS ONLY;
//
// u.UserId = rp.PostId compares a user id with a post id, so it goes through the raw ids.
fn q1473(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, view_count, origid, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let s = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    type PA = (Id<Post>, [i64; 3]);
    let rp: MatSet<PA> = recent()
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(&s))
        .window(row_number, |(p, a)| (a[0], Reverse(p)), desc)
        .filt(|(_, k)| k == 1)
        .map(|(x, _)| x)
        .collect();
    let by_raw: HashIdx<i64, PA> = (&rp).map(|x: PA| x.0).select(origid).inv().collect();
    let tu = recent().group_by(owner_user).select(view_count.opt()).fold([0i64; 2], |a, w| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0)]);
    type W = ((Id<User>, [i64; 2]), i64);
    let w = whole(&tu).select(Ident::<User>::new().and(&tu)).window(rank, |(_, a)| if a[0] > 0 { Some(a[1]) } else { None }, desc).filt(|(_, k)| k <= 5);
    let v = drain(w.select(Same::<W>::new().and(Same::<W>::new().map(|x: W| x.0 .0).select(&db.user.origid).select(&by_raw))));
    let v = top_n(v, |&(_, (((_, a), _), (_, b)))| (a[0] == 0, Reverse(a[1]), Reverse(b[1])), 10);
    rows(v.into_iter().map(|(_, (((u, a), _), (p, b)))| {
        let mut f = vec![user_col(db, u, "name"), nullable(a[1], a[0])];
        f.extend(post_fields(db, p, &["title"]));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY COUNT(c.Id) DESC) AS CommentRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.CommentRank <= 10)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes, (tp.UpVotes - tp.DownVotes) AS Score,
//        (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = tp.PostId AND ph.PostHistoryTypeId IN (10, 11)) AS ClosureHistory
// FROM TopPosts tp ORDER BY Score DESC;
fn q6172(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)).and(post_type_id.is_in([1, 2])))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let ch = db.post.group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let w = whole(&s).select(Ident::<Post>::new().and(&s).and(&ch)).window(rank, |((_, a), _)| a[0], desc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((p, a), h), _))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend(a.map(V::I));
        f.extend([V::I(a[1] - a[2]), V::I(h)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.RankByScore <= 5),
// PostVoteDetails AS (SELECT p.Id AS PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT trp.Title, trp.OwnerDisplayName, trp.CreationDate, pd.UpVotes, pd.DownVotes, trp.Score, trp.ViewCount, (pd.UpVotes - pd.DownVotes) AS NetScore
// FROM TopRankedPosts trp JOIN PostVoteDetails pd ON trp.PostId = pd.PostId ORDER BY NetScore DESC, trp.Score DESC;
fn q6365(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| s, desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let pd = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain(&pd).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["score", "views"]));
        f.push(V::I(a[0] - a[1]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName),
// RankedQuestions AS (SELECT PostId, Title, Score, CreationDate, OwnerDisplayName, CommentCount, UpvoteCount, RANK() OVER (ORDER BY Score DESC, CreationDate ASC) AS Rank FROM RankedPosts)
// SELECT rq.Title, rq.Score, rq.CreationDate, rq.OwnerDisplayName, rq.CommentCount, rq.UpvoteCount,
//        CASE WHEN rq.Rank <= 10 THEN 'Top 10 Questions' WHEN rq.Rank <= 50 THEN 'Top 50 Questions' ELSE 'Other Questions' END AS RankCategory
// FROM RankedQuestions rq WHERE rq.Rank <= 100 ORDER BY rq.Rank;
//
// Rank reads only base columns, so the ranked questions are picked first and the comment x vote product is driven for those alone.
fn q8750(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let tr: MatSet<PK> = whole(db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1)).and(post_type_id.eq(1))))
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (Reverse(s), d), asc)
        .filt(|(_, k)| k <= 100)
        .map(|(((p, _), _), k)| (p, k))
        .collect();
    let rank = by_post(&tr);
    let s = db
        .post
        .with(&rank)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    rows(drain((&s).and(&rank)).into_iter().map(|(p, (a, r))| {
        let mut f = post_fields(db, p, &["title", "score", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top 10 Questions" } else if r <= 50 { "Top 50 Questions" } else { "Other Questions" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, WikiCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Ranking FROM UserStats)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.WikiCount, TU.UpVotes, TU.DownVotes FROM TopUsers TU WHERE TU.Ranking <= 10 ORDER BY TU.Reputation DESC, TU.PostCount DESC;
//
// Ranking reads Reputation and the distinct post count, so the ten users are picked first and the post x vote product is driven for them alone.
fn q5507(db: &'static So) -> String {
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = top_n(drain(&dp), |&(u, n)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u), 10);
    let tus: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    rows(drain((&s).and(&dp)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, DisplayName, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// TopTags AS (SELECT TagName, COUNT(*) AS TagCount FROM Tags GROUP BY TagName HAVING COUNT(*) > 5),
// AcceptedAnswers AS (SELECT p.OwnerUserId, COUNT(*) AS AcceptedAnswersCount FROM Posts p WHERE p.AcceptedAnswerId IS NOT NULL GROUP BY p.OwnerUserId),
// PostsInfo AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COALESCE(aa.AcceptedAnswersCount, 0) AS AcceptedAnswersCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN AcceptedAnswers aa ON u.Id = aa.OwnerUserId WHERE p.PostTypeId = 1)
// SELECT p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerDisplayName, rr.Reputation, rr.ReputationRank, tt.TagName, tt.TagCount, p.AcceptedAnswersCount
// FROM PostsInfo p JOIN UserReputation rr ON p.OwnerDisplayName = rr.DisplayName JOIN TopTags tt ON p.Title LIKE CONCAT('%', tt.TagName, '%')
// ORDER BY rr.Reputation DESC, p.Score DESC LIMIT 50;
fn q6283(db: &'static So) -> String {
    let Post { post_type_id, owner_user, title, accepted_answer_id, score, .. } = &db.post;
    let tt = db.tag.group_by(&db.tag.tag_name).select(Ident::<Tag>::new()).fold(0i64, |n, _| n + 1);
    let tt = rel(drain((&tt).filt(|n| n > 5)));
    let ttk: HashIdx<Str, (Str, i64)> = (&tt).map(|(n, _)| n).inv().select(&tt).collect();
    let titles: MatSet<Str> = db.post.with(post_type_id.eq(1)).select(title).collect();
    let like: HashIdx<Str, (Str, i64)> = (&titles).select_where(&ttk, |t: Str, n: Str| t.contains(n)).collect();
    let aa = db.post.with(accepted_answer_id).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let rr: MatSet<UK> = whole(&db.user.id).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc).map(|((u, _), k)| (u, k)).collect();
    let rr_name: HashIdx<Str, UK> = (&rr).map(|x: UK| x.0).select(&db.user.display_name).inv().collect();
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.and(owner_user.select(&db.user.display_name).select(&rr_name)).and(title.select(&like)).and(owner_user.select((&aa).opt()))));
    let v = top_n(v, |&(p, (((_, (u, _)), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(p, (((_, (u, r)), (n, c)), a))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "created", "owner"]);
        f.extend([user_col(db, u, "rep"), V::I(r), V::S(n), V::I(c), V::I(a.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(V.BountyAmount) AS TotalBounties, AVG(U.Reputation) OVER () AS AverageReputation, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalBounties, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStatistics)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalBounties, BadgeCount, ReputationRank FROM TopUsers
// WHERE Reputation > (SELECT AVG(Reputation) FROM UserStatistics) ORDER BY ReputationRank LIMIT 10;
//
// ReputationRank and the WHERE read only Reputation, so the ten users are picked first and the post x vote x badge product is driven for them alone.
fn q7410(db: &'static So) -> String {
    let (n, sum) = (&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let avg_rep = sum as f64 / n as f64;
    let w = whole(&db.user.id).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc).filt(move |((_, r), _)| r as f64 > avg_rep);
    let v = top_n(drain(&w).into_iter().map(|x| x.1).collect(), |&((u, _), k)| (k, u), 10);
    let tu = rel(v.into_iter().map(|((u, _), k)| (u, k)).collect());
    let tuk: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let Post { post_type_id, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let tus: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let s = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(bounty.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, _)| match p {
            Some((t, b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
            }
            None => a,
        });
    let bc = (&tus).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&s).and(&bc).and((&tuk).map(|(_, r)| r))).into_iter().map(|(u, ((a, b), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS RankScore FROM Posts P WHERE P.PostTypeId IN (1, 2)),
// UserVoteCounts AS (SELECT V.UserId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotesCount FROM Votes V GROUP BY V.UserId),
// FrequentCommenters AS (SELECT C.UserId, COUNT(*) AS CommentCount FROM Comments C GROUP BY C.UserId HAVING COUNT(*) > 5)
// SELECT U.Id AS UserId, U.DisplayName, COALESCE(UVC.UpVotesCount, 0) AS UpVotesCount, COALESCE(UVC.DownVotesCount, 0) AS DownVotesCount, COALESCE(FC.CommentCount, 0) AS FrequentCommentCount,
//        RP.PostId, RP.Title, RP.Score, RP.ViewCount, RP.CreationDate
// FROM Users U LEFT JOIN UserVoteCounts UVC ON U.Id = UVC.UserId LEFT JOIN FrequentCommenters FC ON U.Id = FC.UserId LEFT JOIN RankedPosts RP ON RP.RankScore <= 10
// WHERE U.Reputation > 50 ORDER BY U.Reputation DESC, RP.Score DESC;
//
// The last ON names only RP, so the users are crossed with the ranked posts.
fn q34677(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let rp: HashIdx<(), Id<Post>> = db
        .post
        .with(post_type_id.is_in([1, 2]))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| s, desc)
        .filt(|(_, k)| k <= 10)
        .map(|((p, _), _)| p)
        .collect::<MatSet<Id<Post>>>()
        .map(|_| ())
        .inv()
        .collect();
    let uvc = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let fc = db.comment.group_by(&db.comment.user).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with((&db.user.reputation).gt(50)).select((&uvc).opt().and((&fc).filt(|n| n > 5).opt()).and(Ident::<User>::new().map(|_| ()).select(&rp).opt())));
    rows(v.into_iter().map(|(u, ((a, c), p))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0))]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "score", "views", "created"]),
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, u.Reputation AS UserReputation
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score >= 10),
// PostWithComments AS (SELECT r.PostId, r.Title, r.Score, r.ViewCount, r.UserReputation, COALESCE(c.CommentCount, 0) as CommentCount
//     FROM RankedPosts r LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON r.PostId = c.PostId),
// FinalResults AS (SELECT p.*, CASE WHEN UserReputation > 1000 THEN 'High Reputation' WHEN UserReputation BETWEEN 501 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
//     FROM PostWithComments p WHERE p.CommentCount > 5)
// SELECT f.PostId, f.Title, f.Score, f.ViewCount, f.ReputationCategory FROM FinalResults f ORDER BY f.Score DESC, f.ViewCount DESC LIMIT 10;
fn q1572(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.post.with(post_type_id.eq(1).and(score.ge(10))).select(owner_user.select(&db.user.reputation).and((&cc).filt(|n| n > 5))));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (r, _))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 501 { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UpvoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.CreationDate),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.CommentCount, rp.UpvoteCount FROM RankedPosts rp WHERE rp.rn = 1 ORDER BY rp.Score DESC, rp.CommentCount DESC LIMIT 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.CommentCount, u.DisplayName AS AuthorDisplayName, u.Reputation AS AuthorReputation, COUNT(b.Id) AS BadgeCount
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY tp.PostId, u.Id, tp.Title, tp.Score, tp.CommentCount, u.DisplayName, u.Reputation ORDER BY tp.Score DESC;
//
// u.Id = tp.PostId compares a user id with a post id, so it goes through the raw ids.
fn q6103(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let ups = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let s = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(ups.opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let tp = top_n(drain(&s), |&(p, c)| (Reverse(score.get(p).unwrap()), Reverse(c), p), 10);
    let tp = rel(tp);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type PC = (Id<Post>, i64);
    let v = drain((&tp).select(Same::<PC>::new().and(Same::<PC>::new().map(|(p, _): PC| p).select(&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&bc)))));
    rows(v.into_iter().map(|(_, ((p, c), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.push(V::I(c));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH FilteredPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Body, p.Tags, COALESCE(REPLACE(SUBSTRING(p.Body FROM '(<p>)(.*?)(</p>)'), '<p>', ''), '') AS CleanBody,
//        ARRAY_LENGTH(string_to_array(p.Tags, '><'), 1) AS TagCount, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate BETWEEN '2023-01-01' AND '2023-12-31' AND p.ViewCount > 100 AND p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Body, p.Tags),
// RankedPosts AS (SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Body, fp.TagCount, fp.CommentCount, RANK() OVER (ORDER BY fp.CommentCount DESC) AS CommentRank FROM FilteredPosts fp)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.TagCount,
//        CASE WHEN rp.CommentRank <= 10 THEN 'Top Discussion' WHEN rp.CommentRank <= 30 THEN 'Moderate Interest' ELSE 'Low Engagement' END AS EngagementLevel,
//        SUBSTRING(rp.Body FROM 1 FOR 200) AS Preview
// FROM RankedPosts rp WHERE rp.TagCount >= 3 ORDER BY rp.CommentRank;
//
// CleanBody is never read.
fn q27985(db: &'static So) -> String {
    let Post { creation_date, view_count, post_type_id, tags_str, body, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.between(ts(2023, 1, 1, 0, 0, 0), ts(2023, 12, 31, 0, 0, 0)).and(view_count.gt(100)).and(post_type_id.eq(1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt())
        .fold(0i64, |n, c| n + c.is_some() as i64);
    let tc = tags_str.opt().map(|t: Option<Str>| t.map(|t| t.split("><").count() as i64));
    let w = whole(&s).select(Ident::<Post>::new().and(&s).and(tc)).window(rank, |((_, c), _)| c, desc);
    rows(drain((&w).filt(|((_, n), _)| n.map_or(false, |n| n >= 3))).into_iter().map(|(_, (((p, c), n), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), oint(n)]);
        f.push(V::S(if r <= 10 { "Top Discussion" } else if r <= 30 { "Moderate Interest" } else { "Low Engagement" }));
        f.push(V::Owned(body.get(p).unwrap().chars().take(200).collect()));
        row(f)
    }))
}

// WITH RankedVotes AS (SELECT p.Id AS PostId, v.VoteTypeId, COUNT(v.Id) AS VoteCount, ROW_NUMBER() OVER(PARTITION BY p.Id ORDER BY COUNT(v.Id) DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, v.VoteTypeId),
// TopPosts AS (SELECT p.Id, p.Title, SUM(CASE WHEN rv.VoteTypeId = 2 THEN rv.VoteCount ELSE 0 END) AS Upvotes, SUM(CASE WHEN rv.VoteTypeId = 3 THEN rv.VoteCount ELSE 0 END) AS Downvotes,
//        COUNT(c.Id) AS CommentCount, COUNT(b.Id) AS BadgeCount, p.CreationDate, p.ViewCount
//     FROM Posts p LEFT JOIN RankedVotes rv ON p.Id = rv.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount HAVING COUNT(c.Id) > 5 ORDER BY Upvotes DESC LIMIT 10)
// SELECT tp.Title, tp.Upvotes, tp.Downvotes, tp.CommentCount, tp.BadgeCount, tp.ViewCount, EXTRACT(YEAR FROM tp.CreationDate) AS CreationYear FROM TopPosts tp ORDER BY tp.Upvotes DESC, tp.CreationDate DESC;
//
// Rank is never read.
fn q7542(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user_id, .. } = &db.post;
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    type PV = (Id<Post>, Option<Id<Vote>>);
    let vote_of = || Same::<PV>::new().flat_map(|(_, v): PV| v);
    let rvg = (&j)
        .group_by(Same::<PV>::new().map(|(p, _): PV| p).and(vote_of().select(&db.vote.vote_type_id).opt()))
        .select(vote_of().opt())
        .fold(0i64, |n, v| n + v.is_some() as i64);
    let rv = rel(drain(&rvg));
    let rvk: HashIdx<Id<Post>, ((Id<Post>, Option<i64>), i64)> = (&rv).map(|((p, _), _)| p).inv().select(&rv).collect();
    let badges_by_id: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let s = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select((&rvk).opt().and(comments_of(db).opt()).and(owner_user_id.select(&badges_by_id).opt()))
        .fold([0i64; 4], |a, ((r, c), b)| {
            let (t, n) = r.map_or((None, 0), |((_, t), n)| (t, n));
            [a[0] + if t == Some(2) { n } else { 0 }, a[1] + if t == Some(3) { n } else { 0 }, a[2] + c.is_some() as i64, a[3] + b.is_some() as i64]
        });
    let v = top_n(drain((&s).filt(|a| a[2] > 5)), |&(p, a)| (Reverse(a[0]), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend(a.map(V::I));
        f.extend([post_fields(db, p, &["views"]).remove(0), V::I(year(creation_date.get(p).unwrap()))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerReputation, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(a.AnswerCount, 0) AS AnswerCount, rp.Rank
//     FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON rp.PostId = c.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON rp.PostId = a.ParentId)
// SELECT pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.OwnerReputation, pd.CommentCount, pd.AnswerCount FROM PostDetails pd WHERE pd.Rank <= 5 ORDER BY pd.CreationDate DESC;
fn q5302(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, parent, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (s, w, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let ac = db.post.with(post_type_id.eq(2)).group_by(parent).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&tp).select((&cc).opt().and((&ac).opt()))).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "rep"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Ranking FROM UserStats)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges FROM TopUsers tu WHERE tu.Ranking <= 10 ORDER BY tu.Ranking;
//
// Ranking reads Reputation and the distinct post count, so the ranked users are picked first and the post x badge product is driven for them alone.
fn q7282(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let dp = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu: MatSet<Id<User>> = whole(&dp)
        .select(Ident::<User>::new().and(&dp).and(&db.user.reputation))
        .window(rank, |((_, n), r)| (r, n), desc)
        .filt(|(_, k)| k <= 10)
        .map(|(((u, _), _), _)| u)
        .collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (t, c)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]);
    rows(drain((&s).and(&dp)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS Author, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Author FROM RankedPosts rp WHERE rp.PostRank <= 5),
// PostVoteCounts AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.Author, pvc.VoteCount, pvc.UpVotes, pvc.DownVotes FROM TopPosts tp JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q8384(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerName, p.Score, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN pt.Id = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year' GROUP BY p.Id, p.Title, u.DisplayName, p.Score, p.CreationDate, p.ViewCount),
// PostStatistics AS (SELECT PostId, Title, OwnerName, Score, CreationDate, ViewCount, CommentCount, AnswerCount, RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM RankedPosts)
// SELECT ps.PostId, ps.Title, ps.OwnerName, ps.Score, ps.CreationDate, ps.ViewCount, ps.CommentCount, ps.AnswerCount,
//        CASE WHEN ps.Rank <= 10 THEN 'Top 10' WHEN ps.Rank BETWEEN 11 AND 50 THEN 'Top 50' ELSE 'Other' END AS RankCategory
// FROM PostStatistics ps ORDER BY ps.Rank;
fn q8884(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&db.post.post_type).select(&db.post_type.origid).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let w = whole(&s).select(Ident::<Post>::new().and(&s).and(score).and(view_count.opt())).window(rank, |((_, s), w)| (s, w), desc);
    rows(drain(&w).into_iter().map(|(_, ((((p, a), _), _), r))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "created", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top 10" } else if r <= 50 { "Top 50" } else { "Other" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(b.Class) AS TotalBadges, MAX(u.CreationDate) AS AccountCreationDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT Us.UserId, Us.DisplayName, Us.Reputation, Us.PostCount, Us.QuestionCount, Us.AnswerCount, Us.TotalBadges, Us.AccountCreationDate, DENSE_RANK() OVER (ORDER BY Us.Reputation DESC) AS Rank FROM UserStats Us)
// SELECT Tu.Rank, Tu.DisplayName, Tu.Reputation, Tu.PostCount, Tu.QuestionCount, Tu.AnswerCount, Tu.TotalBadges, Tu.AccountCreationDate, COUNT(DISTINCT c.Id) AS CommentCount
// FROM TopUsers Tu LEFT JOIN Comments c ON Tu.UserId = c.UserId WHERE Tu.Rank <= 10
// GROUP BY Tu.Rank, Tu.DisplayName, Tu.Reputation, Tu.PostCount, Tu.QuestionCount, Tu.AnswerCount, Tu.TotalBadges, Tu.AccountCreationDate ORDER BY Tu.Rank;
//
// Rank reads only Reputation, so the ranked users are picked first and the post x badge product is driven for them alone.
fn q5681(db: &'static So) -> String {
    let rk: HashIdx<Id<User>, i64> = whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation))
        .window(dense_rank, |(_, r)| r, desc)
        .filt(|(_, k)| k <= 10)
        .map(|((u, _), k)| (u, k))
        .collect::<MatSet<UK>>()
        .map(|x: UK| x.0)
        .inv()
        .map(|x: UK| x.1)
        .collect();
    let tus = || db.user.with(&rk);
    let s = tus()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (t, c)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + c.is_some() as i64, a[3] + c.unwrap_or(0)]);
    let dp = tus().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = tus().group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&s).and(&dp).and(&cc).and(&rk)).into_iter().map(|(u, (((a, d), c), r))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(d), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), user_col(db, u, "ucreated"), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 10),
// RecentUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserPosts AS (SELECT u.DisplayName, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.DisplayName)
// SELECT u.DisplayName, u.Reputation, rp.Title AS TopPostTitle, rp.ViewCount, rp.Score, up.PostCount, up.TotalViews, up.TotalScore
// FROM RecentUsers u LEFT JOIN RankedPosts rp ON u.UserId = rp.PostId LEFT JOIN UserPosts up ON u.DisplayName = up.DisplayName WHERE u.UserRank <= 10
// ORDER BY u.Reputation DESC, rp.Score DESC FETCH FIRST 10 ROWS ONLY;
//
// u.UserId = rp.PostId compares a user id with a post id, so it goes through the raw ids. Rank is never read.
fn q4324(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, origid, .. } = &db.post;
    let rp: HashIdx<i64, Id<Post>> = db.post.with(post_type_id.eq(1).and(score.gt(10))).select(origid).inv().collect();
    let up = db
        .user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(score.and(view_count.opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, w)) => [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s],
            None => a,
        });
    let tu: MatSet<Id<User>> = whole(db.user.with((&db.user.last_access_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))))
        .select(Ident::<User>::new().and(&db.user.reputation))
        .window(rank, |(_, r)| r, desc)
        .filt(|(_, k)| k <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let v = drain((&tu).select((&db.user.origid).select(&rp).opt().and((&db.user.display_name).select(&up))));
    let v = top_n(v, |&(u, (p, _))| (Reverse(db.user.reputation.get(u).unwrap()), p.is_none(), Reverse(p.map(|p| score.get(p).unwrap()))), 10);
    rows(v.into_iter().map(|(u, (p, a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "views", "score"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::I(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        (SELECT COUNT(DISTINCT ph.PostHistoryTypeId) FROM PostHistory ph WHERE ph.PostId = p.Id AND ph.CreationDate >= CURRENT_DATE - INTERVAL '1 year') AS HistoryCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '6 months'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.HistoryCount FROM RankedPosts rp WHERE rp.Rank <= 5),
// UserBadges AS (SELECT u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, ub.BadgeCount FROM TopPosts tp JOIN UserBadges ub ON tp.PostId = ub.BadgeCount
// ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// tp.PostId = ub.BadgeCount joins the raw post id to a count. HistoryCount is never read.
fn q6302(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_months(current_date(), -6)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (s, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let ub = db.user.group_by(&db.user.display_name).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let by_count: HashIdx<i64, Str> = (&ub).inv().collect();
    rows(drain((&tp).select(origid.select(&by_count))).into_iter().map(|(p, _)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers", "comments"]);
        f.push(V::I(origid.get(p).unwrap()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, p.AnswerCount, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.Score, rp.AnswerCount, rp.ViewCount FROM RankedPosts rp WHERE rp.Rank <= 3)
// SELECT t.PostId, t.Title, t.CreationDate, t.OwnerDisplayName, t.Score, t.AnswerCount, t.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM TopPosts t LEFT JOIN Comments c ON t.PostId = c.PostId LEFT JOIN Votes v ON t.PostId = v.PostId
// GROUP BY t.PostId, t.Title, t.CreationDate, t.OwnerDisplayName, t.Score, t.AnswerCount, t.ViewCount ORDER BY t.Score DESC, t.ViewCount DESC;
fn q7450(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(date(2024, 10, 1), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 3)
        .map(|((p, _), _)| p)
        .collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "answers", "views"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RECURSIVE TopUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, u.Views, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS RN FROM Users u WHERE u.Reputation > 1000),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(*) AS TotalPosts, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount FROM Posts p GROUP BY p.OwnerUserId),
// ClosedPosts AS (SELECT p.OwnerUserId, COUNT(*) AS ClosedPostCount FROM Posts p WHERE p.PostTypeId = 1 AND p.ClosedDate IS NOT NULL GROUP BY p.OwnerUserId)
// SELECT tu.DisplayName, tu.Reputation, tu.Views, COALESCE(ub.BadgeCount, 0) AS GoldBadgeCount, COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.TotalScore, 0) AS TotalScore,
//        COALESCE(ps.AvgViewCount, 0) AS AvgViewCount, COALESCE(cp.ClosedPostCount, 0) AS ClosedPostCount
// FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.Id = ub.UserId LEFT JOIN PostStatistics ps ON tu.Id = ps.OwnerUserId LEFT JOIN ClosedPosts cp ON tu.Id = cp.OwnerUserId
// WHERE tu.RN <= 50 ORDER BY tu.Reputation DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q30639(db: &'static So) -> String {
    let Post { owner_user, score, view_count, post_type_id, closed_date, .. } = &db.post;
    let tu = top_n(drain((&db.user.reputation).gt(1000)), |&(u, r)| (Reverse(r), u), 50);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ub = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ps = db.post.group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let cp = db.post.with(post_type_id.eq(1)).with(closed_date).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&tu).select((&ub).opt().and((&ps).opt()).and((&cp).opt()))).into_iter().map(|(u, ((b, a), c))| {
        let mut f = ucols(db, u, &["name", "rep", "uviews"]);
        f.push(V::I(b.unwrap_or(0)));
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), if a[2] == 0 { V::F(0.0) } else { avg(a[3], a[2]) }],
            None => [V::I(0), V::I(0), V::F(0.0)],
        });
        f.push(V::I(c.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.PostTypeId = 1),
// RecentPosts AS (SELECT rp.Id, rp.Title, rp.OwnerUserId, rp.CreationDate, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.rn <= 5),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostVoteStats AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT rp.Title, u.DisplayName AS OwnerDisplayName, rp.CreationDate, rp.Score, pvs.UpVotes, pvs.DownVotes, ub.BadgeCount
// FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostVoteStats pvs ON rp.Id = pvs.PostId
// WHERE ub.BadgeCount IS NOT NULL AND pvs.UpVotes > pvs.DownVotes ORDER BY rp.CreationDate DESC;
fn q3322(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    rows(drain((&tp).select(owner_user.select(Ident::<User>::new().and(&ub)).and((&pvs).filt(|a| a[0] > a[1])))).into_iter().map(|(p, ((u, b), a))| {
        let mut f = post_fields(db, p, &["title"]);
        f.push(user_col(db, u, "name"));
        f.extend(post_fields(db, p, &["created", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.PostTypeId, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate ASC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PopularPosts AS (SELECT RP.PostId, RP.Title, RP.OwnerDisplayName, RP.Score, RP.ViewCount, COUNT(C.Id) AS CommentCount FROM RankedPosts RP LEFT JOIN Comments C ON RP.PostId = C.PostId
//     WHERE RP.PostRank <= 5 GROUP BY RP.PostId, RP.Title, RP.OwnerDisplayName, RP.Score, RP.ViewCount)
// SELECT PP.Title, PP.OwnerDisplayName, PP.Score, PP.ViewCount, PP.CommentCount, COALESCE(B.BadgeCount, 0) AS BadgeCount
// FROM PopularPosts PP LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON PP.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = B.UserId)
// ORDER BY PP.Score DESC, PP.ViewCount DESC;
fn q6228(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (Reverse(s), d), asc)
        .filt(|(_, k)| k <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = rel(drain(db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1)));
    let by_name: HashIdx<Str, (Id<User>, i64)> = (&bc).map(|(u, _)| u).select(&db.user.display_name).inv().select(&bc).collect();
    rows(drain((&cc).and(owner_user.select(&db.user.display_name).select(&by_name).opt())).into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.extend([V::I(c), V::I(b.map_or(0, |(_, n)| n))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate AS PostCreationDate, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.PostCreationDate, rp.Score, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount FROM RankedPosts rp WHERE rp.rn <= 5)
// SELECT t.Title, t.PostCreationDate, t.Score, t.CommentCount, t.UpvoteCount, t.DownvoteCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
// FROM TopPosts t JOIN Users u ON t.PostId = u.Id WHERE u.Reputation > 1000 ORDER BY t.Score DESC, t.CommentCount DESC;
//
// t.PostId = u.Id compares a post id with a user id, so it goes through the raw ids.
fn q6291(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.origid).inv().collect();
    rows(drain((&s).and(origid.select(&uidx))).into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, AVG(p.Score) AS AvgPostScore, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT ph.UserId, HOT.Name AS HistoryTypeName, COUNT(ph.Id) AS HistoryCount FROM PostHistory ph JOIN PostHistoryTypes HOT ON ph.PostHistoryTypeId = HOT.Id GROUP BY ph.UserId, HOT.Name),
// FinalStats AS (SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.Questions, ups.Answers, ups.AvgPostScore, ups.AvgViewCount, COALESCE(phs.HistoryCount, 0) AS HistoryCount
//     FROM UserPostStats ups LEFT JOIN PostHistoryStats phs ON ups.UserId = phs.UserId)
// SELECT *, RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts, RANK() OVER (ORDER BY HistoryCount DESC) AS RankByHistory FROM FinalStats ORDER BY RankByPosts;
fn q11683(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + 1, a[5] + w.unwrap_or(0)],
            None => [a[0], a[1], a[2], a[3], a[4] + 1, a[5]],
        });
    let PostHistory { user, .. } = &db.post_history;
    let phs = rel(drain(db.post_history.group_by(user.and(htype_name(db))).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1)));
    let by_user: HashIdx<Id<User>, ((Id<User>, Str), i64)> = (&phs).map(|((u, _), _)| u).inv().select(&phs).collect();
    type R = ((Id<User>, [i64; 6]), Option<i64>);
    let w = whole(&ups)
        .select(Ident::<User>::new().and(&ups).and((&by_user).map(|(_, n)| n).opt()))
        .window(rank, |((_, a), _): R| a[0], desc)
        .window(rank, |((_, h), _): (R, i64)| h.unwrap_or(0), desc);
    rows(drain(&w).into_iter().map(|(_, ((((u, a), h), rp), rh))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), avg(a[5], a[4]), V::I(h.unwrap_or(0)), V::I(rp), V::I(rh)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostVotes AS (SELECT p.Id AS PostId, MAX(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoted, MAX(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoted
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.Id IN (SELECT PostId FROM TopPosts) GROUP BY p.Id)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, pv.UpVoted, pv.DownVoted FROM TopPosts tp LEFT JOIN PostVotes pv ON tp.PostId = pv.PostId
// ORDER BY tp.ViewCount DESC, tp.CreationDate DESC;
fn q5701(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0].max((t == Some(2)) as i64), a[1].max((t == Some(3)) as i64)]);
    rows(drain(&pv).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' AND p.Score > 0),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostLinksCount AS (SELECT pl.PostId, COUNT(*) AS LinkCount FROM PostLinks pl GROUP BY pl.PostId),
// CombinedResults AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, COALESCE(plc.LinkCount, 0) AS LinkCount FROM TopPosts tp LEFT JOIN PostLinksCount plc ON tp.PostId = plc.PostId)
// SELECT c.* FROM CombinedResults c JOIN Users u ON u.Id = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = c.PostId) WHERE u.Reputation >= 100 ORDER BY c.Score DESC, c.ViewCount DESC;
fn q29552(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)).and(score.gt(0)))
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (s, w, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let lc = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).opt()).fold(0i64, |n, l| n + l.is_some() as i64);
    let good = owner_user.select(Ident::<User>::new().with((&db.user.reputation).ge(100)));
    rows(drain((&tp).with(good).select((&cc).and(&lc))).into_iter().map(|(p, (c, l))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::I(l)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostInteraction AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT tp.PostId, tp.Title, tp.Score, tp.OwnerDisplayName, pi.CommentCount, pi.UpvoteCount, pi.DownvoteCount FROM TopPosts tp JOIN PostInteraction pi ON tp.PostId = pi.PostId
// ORDER BY tp.Score DESC, tp.Title ASC;
//
// PostInteraction is one group per post and only the top posts are read, so its comment x vote product is driven for those alone.
fn q9964(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (s, d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH MostActiveUsers AS (SELECT u.Id, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Badges b GROUP BY b.UserId),
// UserPostStats AS (SELECT ua.Id, ua.DisplayName, ua.PostCount, ua.TotalViews, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ub.HighestBadgeClass, 0) AS HighestBadgeClass
//     FROM MostActiveUsers ua LEFT JOIN UserBadges ub ON ua.Id = ub.UserId),
// RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT ups.DisplayName, ups.PostCount, ups.TotalViews, ups.BadgeCount, ups.HighestBadgeClass, rp.Title AS LatestPostTitle, rp.CreationDate AS LatestPostDate
// FROM UserPostStats ups LEFT JOIN RecentPosts rp ON ups.Id = rp.OwnerUserId AND rp.rn = 1 WHERE ups.TotalViews > 100 ORDER BY ups.PostCount DESC, ups.TotalViews DESC, ups.BadgeCount DESC;
fn q4812(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let mau = db
        .post
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))
        .group_by(owner_user)
        .select(view_count.opt())
        .fold([0i64; 2], |a, w| [a[0] + 1, a[1] + w.unwrap_or(0)]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 2], |a, c| [a[0] + 1, a[1].max(c)]);
    let latest: HashIdx<Id<User>, Id<Post>> = db
        .post
        .with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, Reverse(p)), desc)
        .filt(|(_, k)| k == 1)
        .map(|((p, _), _)| p)
        .collect();
    rows(drain((&mau).filt(|a| a[1] > 100).and((&ub).opt()).and((&latest).opt())).into_iter().map(|(u, ((a, b), p))| {
        let b = b.unwrap_or([0, 0]);
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(b[0]), V::I(b[1])];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rank, p.OwnerUserId FROM Posts p
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStats AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers, SUM(p.Score) AS TotalScore FROM Posts p GROUP BY p.OwnerUserId)
// SELECT ur.DisplayName, ur.Reputation, ur.BadgeCount, rp.Title, rp.ViewCount, ps.TotalQuestions, ps.TotalAnswers, ps.TotalScore
// FROM UserReputation ur LEFT JOIN PostStats ps ON ur.UserId = ps.OwnerUserId JOIN RankedPosts rp ON ur.UserId = rp.OwnerUserId WHERE rp.Rank <= 5
// ORDER BY ur.Reputation DESC, rp.ViewCount DESC LIMIT 10;
fn q730(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, post_type_id, score, .. } = &db.post;
    let rp: HashIdx<Id<User>, Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(view_count.opt()))
        .window(row_number, |(p, w)| (w, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 3], |a, (t, s)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s]);
    let v = drain((&rp).and(&bc).and((&ps).opt()));
    let v = top_n(v, |&(u, ((p, _), _))| {
        let w = view_count.get(p);
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(u, ((p, b), a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, u.Reputation, u.CreationDate, u.LastAccessDate, ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate),
// TopUsers AS (SELECT *, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserActivity)
// SELECT tu.UserId, tu.DisplayName, tu.TotalPosts, tu.Questions, tu.Answers, tu.Wikis, tu.Upvotes, tu.Downvotes, tu.Reputation, tu.Rank, tu.ReputationRank FROM TopUsers tu
// WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC, tu.TotalPosts DESC;
//
// Both ranks read only Reputation and the distinct post count, so the ranked users are picked first and the post x vote product is driven for them alone.
fn q9144(db: &'static So) -> String {
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    type R = ((Id<User>, i64), i64);
    let w: MatSet<((R, i64), i64)> = whole(&dp)
        .select(Ident::<User>::new().and(&dp).and(&db.user.reputation))
        .window(row_number, |((u, n), _): R| (n, Reverse(u)), desc)
        .window(dense_rank, |((_, r), _): (R, i64)| r, desc)
        .filt(|(_, q)| q <= 10)
        .collect();
    type W = ((R, i64), i64);
    let rk: HashIdx<Id<User>, W> = (&w).map(|x: W| x.0 .0 .0 .0).inv().collect();
    let s = db
        .user
        .with(&rk)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    rows(drain((&s).and(&rk)).into_iter().map(|(u, (a, ((((_, n), _), r), q)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([user_col(db, u, "rep"), V::I(r), V::I(q)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.Tags, u.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// MostVotedPosts AS (SELECT rp.PostId, COUNT(v.Id) AS VoteCount FROM RankedPosts rp LEFT JOIN Votes v ON rp.PostId = v.PostId WHERE v.VoteTypeId = 2 GROUP BY rp.PostId),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.OwnerDisplayName, COALESCE(mvp.VoteCount, 0) AS VoteCount
//     FROM RankedPosts rp LEFT JOIN MostVotedPosts mvp ON rp.PostId = mvp.PostId WHERE rp.ScoreRank <= 10)
// SELECT fr.PostId, fr.Title, fr.Score, fr.ViewCount, fr.CreationDate, fr.OwnerDisplayName, fr.VoteCount FROM FinalResults fr ORDER BY fr.Score DESC, fr.VoteCount DESC LIMIT 50;
fn q5420(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| s, desc)
        .filt(|(_, k)| k <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let ups = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let mvp = (&tp).group_by(Ident::<Post>::new()).select(ups).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&tp).select((&mvp).opt())), |&(p, n)| (Reverse(score.get(p).unwrap()), Reverse(n.unwrap_or(0)), p), 50);
    rows(v.into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner"]);
        f.push(V::I(n.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RowNum,
//        COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswer FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Date) AS LastBadgeDate FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT up.DisplayName, rp.Title, rp.Score, ub.BadgeCount, ub.LastBadgeDate, pc.CommentCount, CASE WHEN rp.AcceptedAnswer > 0 THEN 'Accepted' ELSE 'Not Accepted' END AS AcceptanceStatus
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id JOIN UserBadges ub ON up.Id = ub.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId
// WHERE ub.BadgeCount > 0 AND rp.RowNum <= 3 ORDER BY rp.Score DESC, up.Reputation DESC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
fn q2557(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, accepted_answer_id, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (s, Reverse(p)), desc)
        .filt(|(_, k)| k <= 3)
        .map(|((p, _), _)| p)
        .collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.date).opt()).fold([0i64, i64::MIN], |a, d| match d {
        Some(d) => [a[0] + 1, a[1].max(d)],
        None => a,
    });
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(owner_user.and(owner_user.select((&ub).filt(|a| a[0] > 0))).and((&cc).opt())));
    let v = top_n(v, |&(p, ((u, _), _))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p), 20);
    rows(v.into_iter().skip(10).map(|(p, ((u, b), c))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(b[0]), tmax(b[1]), oint(c)]);
        f.push(V::S(if accepted_answer_id.get(p).unwrap_or(-1) > 0 { "Accepted" } else { "Not Accepted" }));
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, MAX(p.LastActivityDate) AS LastActivityDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// UserEngagement AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(pm.ViewCount, 0)) AS TotalViews, SUM(COALESCE(pm.Score, 0)) AS TotalScore,
//        SUM(COALESCE(pm.CommentCount, 0)) AS TotalComments, SUM(COALESCE(pm.VoteCount, 0)) AS TotalVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostMetrics pm ON p.Id = pm.PostId GROUP BY u.Id, u.Reputation)
// SELECT ue.UserId, ue.Reputation, ue.PostCount, ue.TotalViews, ue.TotalScore, ue.TotalComments, ue.TotalVotes, RANK() OVER (ORDER BY ue.TotalViews DESC) AS ViewRank,
//        RANK() OVER (ORDER BY ue.TotalScore DESC) AS ScoreRank
// FROM UserEngagement ue ORDER BY ue.TotalViews DESC;
fn q13682(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let pm = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(score).and(&pm)).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((w, s), m)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + m[0], a[4] + m[1]],
            None => a,
        });
    type R = (Id<User>, [i64; 5]);
    let w = whole(&ue)
        .select(Ident::<User>::new().and(&ue))
        .window(rank, |(_, a): R| a[1], desc)
        .window(rank, |((_, a), _): (R, i64)| a[2], desc);
    rows(drain(&w).into_iter().map(|(_, (((u, a), vr), sr))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(vr), V::I(sr)]);
        row(f)
    }))
}

// Rewritten (rewrites/28282.sql): the ROW_NUMBER order is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.Rank FROM RankedPosts rp WHERE rp.Rank <= 5),
// TagStatistics AS (SELECT SPLIT_PART(tags, '><', 1) AS TagName, COUNT(*) AS PostCount, AVG(ViewCount) AS AverageViews FROM Posts WHERE PostTypeId = 1 GROUP BY TagName)
// SELECT tp.PostId, tp.Title, tp.Body, tp.Tags, ts.TagName, ts.PostCount, ts.AverageViews FROM TopPosts tp JOIN TagStatistics ts ON tp.Tags LIKE '%' || ts.TagName || '%'
// ORDER BY ts.PostCount DESC, ts.AverageViews DESC;
//
// The vote sums are never read, so the vote join is not driven.
fn q28282(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, view_count, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1))
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (s, w, Reverse(p)), desc)
        .filt(|(_, k)| k <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let first = |t: Str| -> Str { t.split("><").next().unwrap() };
    let ts_ = db.post.with(post_type_id.eq(1)).group_by(tags_str.map(first)).select(view_count.opt()).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let tv = rel(drain(&ts_));
    let tsk: HashIdx<Str, (Str, [i64; 3])> = (&tv).map(|(t, _)| t).inv().select(&tv).collect();
    let full: MatSet<Str> = (&tp).select(tags_str).collect();
    let like: HashIdx<Str, (Str, [i64; 3])> = (&full).select_where(&tsk, |f: Str, t: Str| f.contains(t)).collect();
    rows(drain((&tp).select(tags_str.select(&like))).into_iter().map(|(p, (t, a))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags"]);
        f.extend([V::S(t), V::I(a[0]), avg(a[2], a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankByUser
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.Score > 0 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserReputations AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName,
//        CASE WHEN SUM(b.Class) >= 5 THEN 'Experienced' WHEN SUM(b.Class) BETWEEN 1 AND 4 THEN 'Moderate' ELSE 'Newbie' END AS UserType
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.DisplayName)
// SELECT up.DisplayName, up.Reputation, up.UserType, rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount FROM RankedPosts rp INNER JOIN UserReputations up ON rp.OwnerUserId = up.UserId
// WHERE rp.RankByUser <= 5 AND (rp.CommentCount > 10 OR up.Reputation > 5000) ORDER BY up.Reputation DESC, rp.CreationDate DESC LIMIT 100;
fn q3293(db: &'static So) -> String {
    let Post { score, creation_date, owner_user, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(score.gt(0).and(creation_date.ge(add_years(date(2024, 10, 1), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(rank, |(_, d)| d, desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 2], |a, c| [a[0] + c.is_some() as i64, a[1] + c.unwrap_or(0)]);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&db.user.reputation).and(&ur))).filt(|(c, ((_, r), _))| c > 10 || r > 5000));
    let v = top_n(v, |&(p, (_, ((_, r), _)))| (Reverse(r), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (c, ((u, _), b)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::S(if b[0] > 0 && b[1] >= 5 { "Experienced" } else if b[0] > 0 && b[1] >= 1 && b[1] <= 4 { "Moderate" } else { "Newbie" }));
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.CommentCount, tp.VoteCount, u.DisplayName AS OwnerDisplayName, u.Reputation
// FROM TopPosts tp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the ten posts of each type are picked first and the comment x vote product is driven for those alone.
fn q9767(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (s, d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let ups = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(ups().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let du = (&tp).group_by(Ident::<Post>::new()).select(ups().select(&db.vote.user_id)).count_distinct();
    rows(drain((&cc).and((&du).opt()).and(owner_user)).into_iter().map(|(p, ((c, d), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(d.unwrap_or(0))]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedVotes AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (ORDER BY COUNT(v.Id) DESC) AS VoteRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id),
// MostVotedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, uv.DisplayName AS UserOwner, r.VoteCount, r.UpVotes, r.DownVotes
//     FROM RankedVotes r JOIN Posts p ON r.PostId = p.Id JOIN Users uv ON p.OwnerUserId = uv.Id WHERE r.VoteRank <= 10)
// SELECT mv.PostId, mv.Title, mv.CreationDate, mv.UserOwner, mv.VoteCount, mv.UpVotes, mv.DownVotes, COUNT(c.Id) AS CommentCount
// FROM MostVotedPosts mv LEFT JOIN Comments c ON mv.PostId = c.PostId GROUP BY mv.PostId, mv.Title, mv.CreationDate, mv.UserOwner, mv.VoteCount, mv.UpVotes, mv.DownVotes ORDER BY mv.VoteCount DESC;
fn q9192(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rv = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(vtype_name(db)).opt())
        .fold([0i64; 3], |a, n| [a[0] + n.is_some() as i64, a[1] + (n == Some("UpMod")) as i64, a[2] + (n == Some("DownMod")) as i64]);
    let top = top_n(drain(&rv), |&(p, a)| (Reverse(a[0]), p), 10);
    let tp = rel(top);
    let tpk: HashIdx<Id<Post>, (Id<Post>, [i64; 3])> = (&tp).map(|(p, _)| p).inv().select(&tp).collect();
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let cc = (&tps).with(owner_user).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&cc).and((&tpk).map(|(_, a)| a))).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostSummary AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(V.BountyAmount) AS TotalBounty, AVG(CASE WHEN P.ViewCount > 0 THEN P.Score / NULLIF(P.ViewCount, 0) ELSE 0 END) AS AverageScorePerView
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) GROUP BY P.OwnerUserId),
// ClosedQuestions AS (SELECT PH.UserId, COUNT(PH.Id) AS ClosedCount FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId)
// SELECT U.UserId, U.DisplayName, COALESCE(P.PostCount, 0) AS NumberOfPosts, COALESCE(P.TotalBounty, 0) AS TotalBountyEarned, COALESCE(P.AverageScorePerView, 0) AS AverageScorePerView,
//        COALESCE(C.ClosedCount, 0) AS NumberOfClosedQuestions, U.Reputation AS UserReputation, U.ReputationRank
// FROM UserReputation U LEFT JOIN PostSummary P ON U.UserId = P.OwnerUserId LEFT JOIN ClosedQuestions C ON U.UserId = C.UserId WHERE U.Reputation > 1000
// ORDER BY U.Reputation DESC, C.ClosedCount DESC LIMIT 10;
//
// The LIMIT reads Reputation and ClosedCount, so the ten users are picked first and the post x bounty-vote product is driven for them alone.
fn q4998(db: &'static So) -> String {
    let rn: MatSet<UK> = whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation))
        .window(row_number, |(u, r)| (r, Reverse(u)), desc)
        .map(|((u, _), k)| (u, k))
        .collect();
    let rnk: HashIdx<Id<User>, i64> = (&rn).map(|x: UK| x.0).inv().map(|x: UK| x.1).collect();
    let cq = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&cq).opt()));
    let v = top_n(v, |&(u, c)| (Reverse(db.user.reputation.get(u).unwrap()), c.is_none(), Reverse(c), u), 10);
    let tu = rel(v);
    let tus: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let cqk: HashIdx<Id<User>, (Id<User>, Option<i64>)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let Post { score, view_count, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ps = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(bounty.opt())))
        .fold((0i64, 0i64, 0i64, 0.0f64), |(n, bn, bs, f), ((s, w), b)| {
            let b = b.flatten();
            let x = match w {
                Some(w) if w > 0 => s as f64 / w as f64,
                _ => 0.0,
            };
            (n + 1, bn + b.is_some() as i64, bs + b.unwrap_or(0), f + x)
        });
    rows(drain((&tus).select((&ps).opt().and((&cqk).map(|(_, c)| c)).and(&rnk))).into_iter().map(|(u, ((a, c), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(match a {
            Some((n, _, bs, s)) => [V::I(n), V::I(bs), V::F(s / n as f64)],
            None => [V::I(0), V::I(0), V::F(0.0)],
        });
        f.extend([V::I(c.unwrap_or(0)), user_col(db, u, "rep"), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p
//     WHERE p.PostTypeId = 1 AND p.Score > 10),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(*) AS PostCount FROM Users u INNER JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation IS NOT NULL GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, PERCENT_RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation)
// SELECT u.DisplayName, r.PostId, r.Title, r.CreationDate, r.Score, COALESCE(c.CommentCount, 0) AS TotalComments, tu.ReputationRank
// FROM RankedPosts r JOIN Users u ON r.OwnerUserId = u.Id LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON r.PostId = c.PostId
// JOIN TopUsers tu ON r.OwnerUserId = tu.UserId WHERE tu.ReputationRank < 0.1 ORDER BY r.CreationDate DESC LIMIT 50;
//
// Rank is never read.
fn q803(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let posters: MatSet<Id<User>> = db.post.select(owner_user).collect();
    let n = count(&posters) as f64;
    let rk: MatSet<UK> = whole(&posters).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc).map(|((u, _), k)| (u, k)).collect();
    let rku: HashIdx<Id<User>, i64> = (&rk).map(|x: UK| x.0).inv().map(|x: UK| x.1).collect();
    let pr = (&rku).map(move |r: i64| (r - 1) as f64 / (n - 1.0));
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let top = pr.filt(|x| x < 0.1);
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(10))).select(owner_user.and(owner_user.select(top)).and((&cc).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((u, x), c))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend([V::I(c.unwrap_or(0)), V::F(x)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.CreationDate FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, MAX(ph.CreationDate) AS LastEditDate
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId
// GROUP BY tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.CreationDate ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q5179(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (s, d, Reverse(p)), desc)
        .filt(|(_, k)| k <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold([0, 0, 0, i64::MIN], |a, ((c, t), h)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3].max(h.unwrap_or(i64::MIN))]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), tmax(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankByScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, p.PostTypeId),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v
//     WHERE v.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.Author, rp.CommentCount, rv.UpVotes, rv.DownVotes, (rv.UpVotes - rv.DownVotes) AS ScoreAdjustment, rp.RankByScore
// FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId WHERE rp.RankByScore <= 10 ORDER BY rp.RankByScore, ScoreAdjustment DESC;
fn q5566(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let t0 = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let tr: MatSet<PK> = db
        .post
        .with(creation_date.ge(t0))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (s, d), desc)
        .filt(|(_, k)| k <= 10)
        .map(|(((p, _), _), k)| (p, k))
        .collect();
    let rank = by_post(&tr);
    let cc = db.post.with(&rank).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rv = db.vote.with((&db.vote.creation_date).ge(t0)).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    rows(drain((&cc).and((&rv).opt()).and(&rank)).into_iter().map(|(p, ((c, a), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.push(V::I(c));
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT U.DisplayName, U.Reputation, U.TotalPosts, U.Questions, U.Answers, U.UpVotes, U.DownVotes, U.Rank, (SELECT AVG(Reputation) FROM Users) AS AverageReputation,
//        CASE WHEN U.Reputation > (SELECT AVG(Reputation) FROM Users) THEN 'Above Average' ELSE 'Below Average' END AS ReputationComparison
// FROM TopUsers U WHERE U.Rank <= 10 ORDER BY U.Rank;
//
// Rank reads only Reputation, so the ten users are picked first and the post x vote product is driven for them alone.
fn q9670(db: &'static So) -> String {
    let (n, sum) = (&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let mean = sum as f64 / n as f64;
    let rk = rep_row(db, 10);
    let tus = || db.user.with(&rk);
    let s = tus()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let dp = tus().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&s).and(&dp).and(&rk)).into_iter().map(|(u, ((a, d), r))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(d));
        f.extend(a.map(V::I));
        f.extend([V::I(r), V::F(mean), V::S(if rep as f64 > mean { "Above Average" } else { "Below Average" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(c.CommentCount) AS TotalComments, SUM(v.VoteCount) AS TotalVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalComments, TotalVotes, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rnk FROM UserActivity)
// SELECT tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.TotalComments, tu.TotalVotes, u.Reputation, u.CreationDate AS AccountCreationDate, u.LastAccessDate
// FROM TopUsers tu JOIN Users u ON tu.UserId = u.Id WHERE tu.Rnk <= 10 ORDER BY tu.PostCount DESC, tu.QuestionCount DESC;
fn q7605(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let vc = db.vote.group_by(&db.vote.post_id).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and((&cc).opt()).and((&db.post.origid).select(&vc).opt())).opt())
        .fold([0i64; 7], |a, x| match x {
            Some(((t, c), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + c.unwrap_or(0), a[5] + v.is_some() as i64, a[6] + v.unwrap_or(0)],
            None => a,
        });
    let v = top_n(drain(&s), |&(u, a)| (Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), nullable(a[6], a[5])];
        f.extend(ucols(db, u, &["rep", "ucreated", "last_access"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, p.PostTypeId),
// FilteredPosts AS (SELECT rp.*, pt.Name AS PostTypeName FROM RankedPosts rp JOIN PostTypes pt ON rp.PostId = pt.Id WHERE rp.PostRank <= 10)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.OwnerDisplayName, fp.CommentCount, fp.UpVoteCount, fp.DownVoteCount, fp.PostTypeName FROM FilteredPosts fp
// WHERE fp.UpVoteCount > fp.DownVoteCount ORDER BY fp.Score DESC, fp.CreationDate DESC;
//
// rp.PostId = pt.Id compares a post id with a post type id, so it goes through the raw ids.
fn q9307(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, origid, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (s, Reverse(p)), desc)
        .filt(|(_, k)| k <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let pt: HashIdx<i64, Str> = (&db.post_type.origid).inv().select(&db.post_type.name).collect();
    rows(drain((&s).filt(|a| a[1] > a[2]).and(origid.select(&pt))).into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerName FROM RankedPosts rp WHERE rp.PostRank <= 10)
// SELECT tp.*, COALESCE(badge_count.BadgeCount, 0) AS BadgeCount FROM TopPosts tp
// LEFT JOIN (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b JOIN Users u ON u.Id = b.UserId WHERE u.Reputation > 500 GROUP BY b.UserId) badge_count
//     ON badge_count.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId)
// ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q9824(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (s, w), desc)
        .filt(|(_, k)| k <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let bc = db.badge.with((&db.badge.user).select(Ident::<User>::new().with((&db.user.reputation).gt(500)))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&tp).select(owner_user.select(&bc).opt())).into_iter().map(|(p, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.push(V::I(b.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.Score, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(b.Class), 0) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// FinalResults AS (SELECT up.PostId, up.Title, up.Score, up.CommentCount, ur.Reputation, ur.TotalBadges, COALESCE(up.ScoreRank, 0) AS UserRank
//     FROM RankedPosts up JOIN UserReputation ur ON up.OwnerUserId = ur.UserId)
// SELECT PostId, Title, Score, CommentCount, Reputation, TotalBadges, CASE WHEN Reputation > 1000 THEN 'High Reputation' ELSE 'Low Reputation' END AS ReputationCategory
// FROM FinalResults WHERE Score > 5 OR UserRank <= 5 ORDER BY Reputation DESC, Score DESC;
fn q3005(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let keep: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| s, desc)
        .filt(|((_, s), r)| r <= 5 || s > 5)
        .map(|((p, _), _)| p)
        .collect();
    let cc = (&keep).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |n, c| n + c.unwrap_or(0));
    rows(drain((&cc).and(owner_user.and(owner_user.select(&tb)))).into_iter().map(|(p, (c, (u, b)))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(c), V::I(rep), V::I(b), V::S(if rep > 1000 { "High Reputation" } else { "Low Reputation" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= '2022-01-01' AND p.Score > 0),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 4 THEN 1 ELSE 0 END), 0) AS OffensiveVoteCount
// FROM TopPosts tp LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName
// ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q7921(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(ts(2022, 1, 1, 0, 0, 0)).and(score.gt(0)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (s, w, Reverse(p)), desc)
        .filt(|(_, k)| k <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(4)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "score", "views", "answers", "comments", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, COUNT(DISTINCT C.Id) AS CommentCount, SUM(B.Class) AS BadgeScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.Reputation > 50
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, CommentCount, BadgeScore, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT U.DisplayName, U.Reputation, U.PostCount, U.QuestionCount, U.AnswerCount, U.CommentCount, U.BadgeScore, PH.CreationDate, P.Title, PH.Comment
// FROM TopUsers U INNER JOIN PostHistory PH ON U.UserId = PH.UserId INNER JOIN Posts P ON PH.PostId = P.Id WHERE U.Rank <= 10 ORDER BY U.Rank, PH.CreationDate DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the post x comment x badge product is driven for them alone.
fn q5967(db: &'static So) -> String {
    let tu = top_n(drain((&db.user.reputation).gt(50)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (p, c)| {
            let t = p.map(|(t, _)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(1)) as i64, a[2] + c.is_some() as i64, a[3] + c.unwrap_or(0)]
        });
    let dp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let dc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hist: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    rows(drain((&s).and(&dp).and(&dc).and(&hist)).into_iter().map(|(u, (((a, p), c), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p), V::I(a[1]), V::I(a[0]), V::I(c), nullable(a[3], a[2]), V::T(db.post_history.creation_date.get(h).unwrap())]);
        f.extend(post_fields(db, db.post_history.post.get(h).unwrap(), &["title"]));
        f.push(ostr(db.post_history.comment.get(h)));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, PostCount, Upvotes, Downvotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT T.DisplayName, T.Reputation, T.QuestionCount, T.AnswerCount, T.PostCount, T.Upvotes, T.Downvotes, CASE WHEN T.Rank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorType
// FROM TopUsers T WHERE T.PostCount > 0 ORDER BY T.Rank;
fn q5209(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&s)
        .select(Ident::<User>::new().and(&s).and(&dp).and(&db.user.reputation))
        .window(row_number, |(((u, _), _), r)| (r, Reverse(u)), desc);
    rows(drain((&w).filt(|(((_, d), _), _)| d > 0)).into_iter().map(|(_, ((((u, a), d), _), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(d), V::I(a[2]), V::I(a[3])]);
        f.push(V::S(if r <= 10 { "Top Contributor" } else { "Regular Contributor" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        COUNT(DISTINCT BA.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges BA ON U.Id = BA.UserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserActivity)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, BadgeCount FROM TopUsers WHERE ReputationRank <= 10 ORDER BY ReputationRank;
//
// ReputationRank reads only Reputation, so the ranked users are picked first and the post x vote x badge product is driven for them alone.
fn q8576(db: &'static So) -> String {
    let tu: MatSet<Id<User>> = whole(db.user.with((&db.user.reputation).gt(1000)))
        .select(Ident::<User>::new().and(&db.user.reputation))
        .window(rank, |(_, r)| r, desc)
        .filt(|(_, k)| k <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (x, _)| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let dp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&s).and(&dp).and(&bc)).into_iter().map(|(u, ((a, d), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(d));
        f.extend(a.map(V::I));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate AS CloseDate, ph.UserDisplayName AS ClosedBy, ph.Comment AS CloseReason FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate AS QuestionDate, rp.Score AS QuestionScore, rp.ViewCount AS QuestionViews, rp.AnswerCount AS TotalAnswers, cp.CloseDate, cp.ClosedBy, cp.CloseReason,
//        CASE WHEN rp.OwnerReputation > 1000 THEN 'Experienced' ELSE 'Newbie' END AS UserExperienceLevel
// FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.Rank = 1 ORDER BY rp.Score DESC NULLS LAST, rp.ViewCount DESC;
fn q33013(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, Reverse(p)), desc)
        .filt(|(_, k)| k == 1)
        .map(|((p, _), _)| p)
        .collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    rows(drain((&tp).select(owner_user.select(&db.user.reputation).and(closes.opt()))).into_iter().map(|(p, (r, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend(match h {
            Some(h) => [V::T(db.post_history.creation_date.get(h).unwrap()), ostr(db.post_history.user_display_name.get(h)), ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if r > 1000 { "Experienced" } else { "Newbie" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 10),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT us.DisplayName, us.TotalPosts, us.TotalAnswers, us.GoldBadges, ROW_NUMBER() OVER (ORDER BY us.TotalPosts DESC, us.TotalAnswers DESC) AS UserRank FROM UserStats us WHERE us.TotalPosts > 5)
// SELECT rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.Score, rp.ViewCount, rp.AnswerCount, tu.DisplayName AS TopUser, tu.TotalPosts, tu.TotalAnswers, tu.GoldBadges
// FROM RankedPosts rp JOIN TopUsers tu ON rp.OwnerDisplayName = tu.DisplayName WHERE tu.UserRank <= 10 ORDER BY rp.CreationDate DESC, rp.Score DESC;
//
// PostRank is never read.
fn q6594(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let us = db
        .user
        .with((&dp).filt(|n| n > 5))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 2], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (c == Some(1)) as i64]);
    let tu = top_n(drain((&us).and(&dp)), |&(u, (a, d))| (Reverse(d), Reverse(a[0]), u), 10);
    let tu = rel(tu);
    let by_name: HashIdx<Str, (Id<User>, ([i64; 2], i64))> = (&tu).map(|(u, _)| u).select(&db.user.display_name).inv().select(&tu).collect();
    rows(drain(db.post.with(post_type_id.eq(1).and(score.gt(10))).select(owner_user.select(&db.user.display_name).select(&by_name))).into_iter().map(|(p, (u, (a, d)))| {
        let mut f = post_fields(db, p, &["title", "created", "owner", "score", "views", "answers"]);
        f.extend([user_col(db, u, "name"), V::I(d), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, P.OwnerUserId, R.ReputationRank FROM Posts P JOIN RankedUsers R ON P.OwnerUserId = R.UserId
//     WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// PostStats AS (SELECT RP.*, COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = RP.PostId AND V.VoteTypeId = 2), 0) AS UpVoteCount,
//        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = RP.PostId AND V.VoteTypeId = 3), 0) AS DownVoteCount,
//        COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = RP.PostId), 0) AS CommentCount FROM RecentPosts RP)
// SELECT PS.PostId, PS.Title, PS.ViewCount, PS.Score, PS.UpVoteCount, PS.DownVoteCount, PS.CommentCount, R.DisplayName AS OwnerName, PS.ReputationRank
// FROM PostStats PS LEFT JOIN Users R ON PS.OwnerUserId = R.Id WHERE PS.CommentCount > 0 ORDER BY PS.Score DESC, PS.ViewCount DESC;
fn q174(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rr: MatSet<UK> = whole(&db.user.id).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc).map(|((u, _), k)| (u, k)).collect();
    let rrk: HashIdx<Id<User>, UK> = (&rr).map(|x: UK| x.0).inv().collect();
    let rp = || db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).with(owner_user);
    let vc = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&vc).and((&cc).filt(|n| n > 0)).and(owner_user.select(&rrk))).into_iter().map(|(p, ((a, c), (u, r)))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), user_col(db, u, "name"), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, RANK() OVER (ORDER BY Score DESC) AS RankScore FROM RankedPosts)
// SELECT t.Title, t.OwnerDisplayName, t.ViewCount, t.Score, t.CommentCount, COALESCE(ph.CommentsEdited, 0) AS CommentsEdited, COALESCE(ph.PostsClosed, 0) AS PostsClosed
// FROM TopPosts t LEFT JOIN (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 24 THEN 1 END) AS CommentsEdited, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS PostsClosed
//     FROM PostHistory ph GROUP BY ph.PostId) ph ON t.PostId = ph.PostId WHERE t.RankScore <= 10 ORDER BY t.Score DESC, t.CommentCount DESC;
//
// RankScore reads only Score, so the ranked questions are picked first and their comments counted for those alone.
fn q5742(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = whole(db.post.with(post_type_id.eq(1)))
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| s, desc)
        .filt(|(_, k)| k <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ph = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 24) as i64, a[1] + (t == 10) as i64]);
    rows(drain((&cc).and((&ph).opt())).into_iter().map(|(p, (c, h))| {
        let h = h.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["title", "owner", "views", "score"]);
        f.extend([V::I(c), V::I(h[0]), V::I(h[1])]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("5377", q5377),
    ("7599", q7599),
    ("6637", q6637),
    ("9471", q9471),
    ("6268", q6268),
    ("9831", q9831),
    ("26163", q26163),
    ("5296", q5296),
    ("5778", q5778),
    ("6358", q6358),
    ("8636", q8636),
    ("8819", q8819),
    ("5677", q5677),
    ("6499", q6499),
    ("5941", q5941),
    ("6295", q6295),
    ("4751", q4751),
    ("7926", q7926),
    ("6701", q6701),
    ("9410", q9410),
    ("4263", q4263),
    ("9371", q9371),
    ("4910", q4910),
    ("6869", q6869),
    ("8245", q8245),
    ("9668", q9668),
    ("1508", q1508),
    ("7415", q7415),
    ("8353", q8353),
    ("8877", q8877),
    ("28482", q28482),
    ("14473", q14473),
    ("8225", q8225),
    ("9678", q9678),
    ("26652", q26652),
    ("1473", q1473),
    ("6172", q6172),
    ("6365", q6365),
    ("8750", q8750),
    ("5507", q5507),
    ("6283", q6283),
    ("7410", q7410),
    ("34677", q34677),
    ("1572", q1572),
    ("6103", q6103),
    ("27985", q27985),
    ("7542", q7542),
    ("5302", q5302),
    ("7282", q7282),
    ("8384", q8384),
    ("8884", q8884),
    ("5681", q5681),
    ("4324", q4324),
    ("6302", q6302),
    ("7450", q7450),
    ("30639", q30639),
    ("3322", q3322),
    ("6228", q6228),
    ("6291", q6291),
    ("11683", q11683),
    ("5701", q5701),
    ("29552", q29552),
    ("9964", q9964),
    ("4812", q4812),
    ("730", q730),
    ("9144", q9144),
    ("5420", q5420),
    ("2557", q2557),
    ("13682", q13682),
    ("28282", q28282),
    ("3293", q3293),
    ("9767", q9767),
    ("9192", q9192),
    ("4998", q4998),
    ("803", q803),
    ("5179", q5179),
    ("5566", q5566),
    ("9670", q9670),
    ("7605", q7605),
    ("9307", q9307),
    ("9824", q9824),
    ("3005", q3005),
    ("7921", q7921),
    ("5967", q5967),
    ("5209", q5209),
    ("8576", q8576),
    ("33013", q33013),
    ("6594", q6594),
    ("174", q174),
    ("5742", q5742),
];
