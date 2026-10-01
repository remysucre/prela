use harness::prelude::*;
use std::cmp::Reverse;

/// A materialised set of pairs indexed by the first component: a -> b.
fn by_first<A: Copy + Eq + std::hash::Hash, B: Copy + Eq + std::hash::Hash>(m: &MatSet<(A, B)>) -> HashIdx<A, B> {
    m.map(|(a, _)| a).inv().map(|(_, b): (A, B)| b).collect()
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(AVG(CASE WHEN V.VoteTypeId = 2 THEN 1 END), 0) AS AverageUpVotes,
//        COALESCE(AVG(CASE WHEN V.VoteTypeId = 3 THEN 1 END), 0) AS AverageDownVotes, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostHistoryAggregates AS (SELECT PH.PostId, MIN(PH.CreationDate) AS FirstEditDate, COUNT(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount
//     FROM PostHistory PH GROUP BY PH.PostId)
// SELECT US.DisplayName, US.Reputation, US.PostCount, US.QuestionCount, US.AnswerCount, PHA.FirstEditDate, PHA.CloseReopenCount
// FROM UserStatistics US LEFT JOIN PostHistoryAggregates PHA ON US.UserId = PHA.PostId
// WHERE US.Reputation > (SELECT AVG(Reputation) FROM Users) AND US.PostCount > 3 ORDER BY US.UserRank, US.DisplayName DESC;
//
// US.UserId = PHA.PostId compares a user id with a post id, so it goes through the raw ids.
fn q2999(db: &'static So) -> String {
    let (rs, rn) = (&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let Post { post_type_id, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).filt(move |r| (r as i128) * (rn as i128) > rs as i128))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, _)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
            None => a,
        });
    let PostHistory { post_id, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.group_by(post_id).select(hd.and(post_history_type_id)).fold((i64::MAX, 0i64), |(m, n), (d, t)| (m.min(d), n + matches!(t, 10 | 11) as i64));
    let v = drain((&us).filt(|a| a[0] > 3).and((&db.user.origid).select(&pha).opt()));
    rows(v.into_iter().map(|(u, (a, h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match h {
            Some((d, n)) => [V::T(d), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT CASE WHEN vt.VoteTypeId = 2 THEN vt.Id END) AS UpvoteCount, COUNT(DISTINCT CASE WHEN vt.VoteTypeId = 3 THEN vt.Id END) AS DownvoteCount,
//        COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostsCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes vt ON p.Id = vt.PostId LEFT JOIN PostLinks pl ON p.Id = pl.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount),
// TopPosts AS (SELECT *, RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS RankByScore, RANK() OVER (ORDER BY CommentCount DESC, CreationDate DESC) AS RankByComments FROM PostStats)
// SELECT PostId, Title, CreationDate, OwnerDisplayName, Score, ViewCount, CommentCount, UpvoteCount, DownvoteCount, RelatedPostsCount, RankByScore, RankByComments
// FROM TopPosts WHERE RankByScore <= 10 OR RankByComments <= 10 ORDER BY RankByScore, RankByComments;
//
// Every aggregate is a COUNT(DISTINCT) of one child, so each is folded over that child alone.
fn q7642(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    type R0 = (((Id<Post>, i64), i64), Option<i64>);
    let w = whole(&cc).select(Ident::<Post>::new().and(&cc).and(score).and(view_count.opt())).window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let w = (&w).window(rank, |((((p, n), _), _), _): (R0, i64)| (Reverse(n), Reverse(creation_date.get(p).unwrap())), asc);
    type R = (((Id<Post>, i64), i64), i64);
    let r: MatSet<R> = (&w).filt(|((_, s), c)| s <= 10 || c <= 10).map(|(((((p, n), _), _), s), c)| (((p, n), s), c)).collect();
    let tp: MatSet<Id<Post>> = (&r).map(|(((p, _), _), _): R| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let lc = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let v = drain((&r).select(Same::<R>::new().and(Same::<R>::new().map(|(((p, _), _), _): R| p).select((&vc).and((&lc).opt())))));
    rows(v.into_iter().map(|(_, ((((p, c), s), r), (a, l)))| {
        let l = l.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(l), V::I(s), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UniqueVoters,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COUNT(p.Id) AS TotalPosts,
//        RANK() OVER (ORDER BY COUNT(p.Id) DESC) AS UserRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName ORDER BY UserRank)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UniqueVoters, au.DisplayName AS MostActiveUser, au.TotalPosts AS ActiveUserPostCount
// FROM RankedPosts rp JOIN MostActiveUsers au ON rp.UniqueVoters > 5 AND rp.Rank = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC FETCH FIRST 10 ROWS ONLY;
//
// The ON clause names only rp, so the surviving posts are crossed with every user. Rank reads only base columns, so the newest question per owner is picked first.
fn q8706(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let up = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let uv = (&tp).group_by(Ident::<Post>::new()).select(up().select(&db.vote.user_id)).count_distinct();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let rp: HashIdx<Id<Post>, (i64, i64)> = (&cc).and((&uv).filt(|n| n > 5)).collect();
    let mau = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = top_n(drain((&rp).cross(&mau)), |&((p, u), _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, u)
    }, 10);
    rows(v.into_iter().map(|((p, u), ((c, n), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n), user_col(db, u, "name"), V::I(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes,
//        CASE WHEN tp.UpVotes > tp.DownVotes THEN 'Positive' WHEN tp.UpVotes < tp.DownVotes THEN 'Negative' ELSE 'Neutral' END AS Sentiment
// FROM TopPosts tp ORDER BY tp.UpVotes DESC, tp.CommentCount DESC;
//
// Rank reads only base columns, so the five newest posts per type are picked first and the comment x vote product is driven for those alone.
fn q9542(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(current_date(), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        SUM(CASE WHEN b.Date IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalWikis, TotalUpvotes, TotalDownvotes, TotalBadges, RANK() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserActivity)
// SELECT t.UserId, t.DisplayName, t.TotalPosts, t.TotalQuestions, t.TotalAnswers, t.TotalWikis, t.TotalUpvotes, t.TotalDownvotes, t.TotalBadges, t.Rank
// FROM TopUsers t WHERE t.Rank <= 10 ORDER BY t.Rank;
fn q6566(db: &'static So) -> String {
    let own = own_votes(db);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&own).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let t = p.map(|x| x.0);
            let vt = p.and_then(|x| x.1);
            [a[0] + p.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64, a[4] + (vt == Some(2)) as i64, a[5] + (vt == Some(3)) as i64, a[6] + b.is_some() as i64]
        });
    let w = whole(&s).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[0]), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY COUNT(c.Id) DESC) AS RankByComments
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes, (rp.UpVotes - rp.DownVotes) AS NetVotes, rp.RankByComments
//     FROM RankedPosts rp WHERE rp.RankByComments <= 10)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.OwnerDisplayName, pd.CommentCount, pd.UpVotes, pd.DownVotes, pd.NetVotes FROM PostDetails pd ORDER BY pd.NetVotes DESC, pd.CommentCount DESC;
fn q6624(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = whole(&s).select(Ident::<Post>::new().and(&s)).window(rank, |(_, a)| Reverse(a[0]), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, ((p, a), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(a[1] - a[2]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, COALESCE(v.upvote_count, 0) AS UpVotes, COALESCE(v.downvote_count, 0) AS DownVotes, COALESCE(c.comment_count, 0) AS CommentCount,
//        p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY COALESCE(v.upvote_count, 0) DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS upvote_count, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS downvote_count FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS comment_count FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate BETWEEN '2023-01-01' AND '2023-12-31')
// SELECT rp.PostId, rp.Title, rp.Body, rp.UpVotes, rp.DownVotes, rp.CommentCount, CASE WHEN rp.Rank <= 5 THEN 'Top 5 Questions This Year' ELSE 'Other Questions' END as RankCategory
// FROM RankedPosts rp WHERE rp.Rank <= 10 ORDER BY RankCategory DESC, rp.UpVotes DESC;
fn q28651(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(creation_date.between(ts(2023, 1, 1, 0, 0, 0), ts(2023, 12, 31, 0, 0, 0))));
    let vc = qs().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = qs().group_by(post_type_id).select(Ident::<Post>::new().and(&vc).and(&cc).and(creation_date)).window(row_number, |(((p, a), _), d)| (Reverse(a[0]), Reverse(d), p), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((((p, a), c), _), r))| {
        let mut f = post_fields(db, p, &["id", "title", "body"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if r <= 5 { "Top 5 Questions This Year" } else { "Other Questions" })]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId AS UserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore FROM Posts p GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT ub.UserId, ub.DisplayName, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(ps.QuestionCount, 0) AS QuestionCount,
//        COALESCE(ps.AnswerCount, 0) AS AnswerCount, COALESCE(ps.TotalScore, 0) AS TotalScore, ub.BadgeCount FROM UserBadges ub LEFT JOIN PostStats ps ON ub.UserId = ps.UserId),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, BadgeCount, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserPerformance WHERE BadgeCount > 0)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, BadgeCount, Rank FROM TopUsers WHERE Rank <= 10 ORDER BY TotalScore DESC;
fn q5251(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let w = whole((&bc).filt(|n| n > 0)).select(Ident::<User>::new().and(&bc).and(&ups)).window(row_number, |((u, _), a)| (Reverse(a[4]), u), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, b), a), i))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(b), V::I(i)]);
        row(f)
    }))
}

// WITH TagCounts AS (SELECT T.TagName, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore, AVG(P.AnswerCount) AS AverageAnswers
//     FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName),
// TopTags AS (SELECT TagName, PostCount, TotalViews, TotalScore, AverageAnswers, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM TagCounts),
// UserEngagement AS (SELECT U.DisplayName, SUM(P.ViewCount) AS TotalViews, COUNT(DISTINCT P.Id) AS NumberOfPosts, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.DisplayName)
// SELECT TT.TagName, TT.PostCount, TT.TotalViews, TT.TotalScore, TT.AverageAnswers, UE.DisplayName, UE.TotalViews AS UserTotalViews, UE.NumberOfPosts AS UserNumberOfPosts,
//        UE.PositivePosts AS UserPositivePosts, UE.NegativePosts AS UserNegativePosts
// FROM TopTags TT JOIN UserEngagement UE ON UE.NumberOfPosts > 10 WHERE TT.Rank <= 10 ORDER BY TT.TotalScore DESC, UE.TotalViews DESC;
//
// The ON clause names only UE, so the top tags are crossed with the busy names. Each post is one row of its name's group, so COUNT(DISTINCT P.Id) is the row count.
fn q25029(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let Post { view_count, score, answer_count, owner_user, .. } = &db.post;
    let tc = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select(view_count.opt().and(score).and(answer_count.opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((w, s), n)) => [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + n.is_some() as i64, a[5] + n.unwrap_or(0)],
            None => a,
        });
    let tt = top_n(drain(&tc), |&(t, a)| (a[0] == 0, Reverse(a[3]), t), 10);
    let tt = rel(tt);
    let ue = db
        .post
        .with(owner_user)
        .group_by(owner_user.select(&db.user.display_name))
        .select(view_count.opt().and(score))
        .fold([0i64; 5], |a, (w, s)| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + 1, a[3] + (s > 0) as i64, a[4] + (s < 0) as i64]);
    let v = drain((&tt).cross((&ue).filt(|a| a[2] > 10)));
    rows(v.into_iter().map(|((_, n), ((t, a), b))| {
        row(vec![V::S(t), V::I(a[0]), nullable(a[2], a[1]), nullable(a[3], a[0]), avg(a[5], a[4]), V::S(n), nullable(b[1], b[0]), V::I(b[2]), V::I(b[3]), V::I(b[4])])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, CommentCount, VoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.VoteCount,
//        CASE WHEN tp.Score > 100 THEN 'Highly Popular' WHEN tp.Score BETWEEN 50 AND 100 THEN 'Moderately Popular' ELSE 'Less Popular' END AS Popularity
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the top ten posts per type are picked first and the comment x upvote product is driven for those alone.
fn q7750(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up.opt())).fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if sc > 100 { "Highly Popular" } else if sc >= 50 { "Moderately Popular" } else { "Less Popular" })]);
        row(f)
    }))
}

// Rewritten (rewrites/6087.sql): the OwnerRank window and the final ORDER BY are tie-broken on the post id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC, p.Id) AS OwnerRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// BestPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.OwnerUserId, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.OwnerRank <= 5),
// PostVoteStats AS (SELECT p.Id AS PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.Id IN (SELECT PostId FROM BestPosts) GROUP BY p.Id)
// SELECT bp.Title, bp.ViewCount, bp.CreationDate, bp.OwnerDisplayName, pvs.UpVotes, pvs.DownVotes, pvs.TotalVotes
// FROM BestPosts bp JOIN PostVoteStats pvs ON bp.PostId = pvs.PostId ORDER BY pvs.UpVotes DESC, pvs.TotalVotes DESC, bp.PostId LIMIT 10;
fn q6087(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(view_count.opt()))
        .window(row_number, |(p, w)| (w.is_none(), Reverse(w), p), asc);
    let bp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&bp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]);
    let v = top_n(drain(&s), |&(p, a)| (Reverse(a[0]), Reverse(a[2]), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "views", "created", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT Id, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE PostRank <= 5),
// PostVotes AS (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId IN (2, 3) GROUP BY PostId),
// PostComments AS (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId)
// SELECT tp.Title, tp.CreationDate, tp.OwnerDisplayName, COALESCE(pv.VoteCount, 0) AS VoteCount, COALESCE(pc.CommentCount, 0) AS CommentCount, tp.Score, tp.ViewCount
// FROM TopPosts tp LEFT JOIN PostVotes pv ON tp.Id = pv.PostId LEFT JOIN PostComments pc ON tp.Id = pc.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q6389(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let ud = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let vc = (&tp).group_by(Ident::<Post>::new()).select(ud.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&vc).and(&cc)).into_iter().map(|(p, (n, c))| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend([V::I(n), V::I(c)]);
        f.extend(post_fields(db, p, &["score", "views"]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, SUM(CASE WHEN P.PostTypeId = 2 THEN P.Score ELSE 0 END) AS TotalAnswerScore,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions, COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS TotalUpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS TotalDownVotes, COUNT(B.Id) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id LEFT JOIN Badges B ON U.Id = B.UserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalAnswerScore, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes, TotalBadges,
//        RANK() OVER (ORDER BY TotalAnswerScore DESC, (TotalUpVotes - TotalDownVotes) DESC) AS Rank FROM UserStats)
// SELECT T.UserId, T.DisplayName, T.Reputation, T.TotalAnswerScore, T.TotalQuestions, T.TotalAnswers, T.TotalUpVotes, T.TotalDownVotes, T.TotalBadges, T.Rank
// FROM TopUsers T WHERE T.Rank <= 10;
//
// The COUNT(DISTINCT)s come from a second fold over one row per post.
fn q8543(db: &'static So) -> String {
    let own = own_votes(db);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score).and((&own).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (sc, vt) = match p {
                Some(((t, s), vt)) => (if t == 2 { s } else { 0 }, vt),
                None => (0, None),
            };
            [a[0] + sc, a[1] + (vt == Some(2)) as i64, a[2] + (vt == Some(3)) as i64, a[3] + b.is_some() as i64]
        });
    let ups = user_posts(db);
    let w = whole(&s).select(Ident::<User>::new().and(&s).and(&ups)).window(rank, |((_, a), _)| (Reverse(a[0]), Reverse(a[1] - a[2])), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, a), q), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(q[2]), V::I(q[3]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(p.ViewCount) AS TotalViews, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
//        COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE u.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, UpVotes, DownVotes, TotalViews, QuestionCount, AnswerCount,
//        RANK() OVER (ORDER BY Reputation DESC, TotalViews DESC) AS UserRank FROM UserStats)
// SELECT ru.UserId, ru.DisplayName, ru.Reputation, ru.BadgeCount, ru.UpVotes, ru.DownVotes, ru.TotalViews, ru.QuestionCount, ru.AnswerCount, ru.UserRank
// FROM RankedUsers ru WHERE ru.UserRank <= 10 ORDER BY ru.UserRank, ru.Reputation DESC;
fn q6204(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let s = db
        .user
        .with((&db.user.creation_date).ge(add_years(date(2024, 10, 1), -1)))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 7], |a, (b, p)| {
            let (t, w, vt) = match p {
                Some(((t, w), vt)) => (Some(t), w, vt),
                None => (None, None, None),
            };
            [a[0] + b.is_some() as i64, a[1] + (vt == Some(2)) as i64, a[2] + (vt == Some(3)) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + (t == Some(1)) as i64, a[6] + (t == Some(2)) as i64]
        });
    let w = whole(&s).select(Ident::<User>::new().and(&s).and(&db.user.reputation)).window(rank, |((_, a), r)| (Reverse(r), a[3] == 0, Reverse(a[4])), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, a), _), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), V::I(a[5]), V::I(a[6]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY u.Id, u.DisplayName ORDER BY PostCount DESC LIMIT 5)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, au.DisplayName AS ActiveUser, au.PostCount, au.Upvotes, au.Downvotes
// FROM RankedPosts rp JOIN MostActiveUsers au ON rp.OwnerDisplayName = au.DisplayName WHERE rp.Rank <= 3 ORDER BY rp.Score DESC LIMIT 10;
fn q6407(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let w = recent().group_by(post_type_id).select(Ident::<Post>::new().and(score).and(view_count.opt())).window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|(((p, _), _), _)| p).collect();
    let mau = recent().group_by(owner_user).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let au = top_n(drain(&mau), |&(u, a)| (Reverse(a[0]), u), 5);
    let au = rel(au);
    let by_name: HashIdx<Str, (Id<User>, [i64; 3])> = (&au).map(|(u, _)| u).select(&db.user.display_name).inv().select(&au).collect();
    let v = drain((&rp).select(owner_user.select(&db.user.display_name).select(&by_name)));
    let v = top_n(v, |&(p, (u, _))| (Reverse(score.get(p).unwrap()), p, u), 10);
    rows(v.into_iter().map(|(p, (u, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotes - TotalDownvotes AS NetVotes, TotalBadges,
//        ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserActivity WHERE TotalPosts > 0)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.NetVotes, U.TotalBadges, RANK() OVER (ORDER BY U.NetVotes DESC) AS VoteRank,
//        RANK() OVER (ORDER BY U.TotalBadges DESC) AS BadgeRank
// FROM TopUsers U WHERE U.Rank <= 10 ORDER BY U.NetVotes DESC, U.TotalBadges DESC;
fn q6852(db: &'static So) -> String {
    let own = own_votes(db);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&own).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let t = p.map(|x| x.0);
            let vt = p.and_then(|x| x.1);
            [a[0] + p.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + (vt == Some(2)) as i64, a[4] + (vt == Some(3)) as i64, a[5] + b.is_some() as i64]
        });
    let top = top_n(drain((&s).filt(|a| a[0] > 0)), |&(u, a)| (Reverse(a[0]), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let w = whole(&tu).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[3] - a[4]), asc);
    let w = (&w).window(rank, |((_, a), _): ((Id<User>, [i64; 6]), i64)| Reverse(a[5]), asc);
    rows(drain(&w).into_iter().map(|(_, (((u, a), vr), br))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3] - a[4]), V::I(a[5]), V::I(vr), V::I(br)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerName, p.CreationDate, p.Score, p.ViewCount, COALESCE(a.AcceptedAnswerId, -1) AS AcceptedAnswerId,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.AcceptedAnswerId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TagStatistics AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, TotalViews, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews FROM TagStatistics)
// SELECT rp.PostId, rp.Title, rp.OwnerName, rp.CreationDate, rp.Score, rp.ViewCount, ts.TagName, ts.PostCount, ts.TotalViews
// FROM RankedPosts rp JOIN TopTags ts ON rp.RankScore <= 10 AND ts.RankByViews <= 5 ORDER BY rp.Score DESC, ts.TotalViews DESC;
//
// The ON clause names each side separately, so the top posts are crossed with the top tags.
fn q7097(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let accepted_by: HashIdx<Id<Post>, Id<Post>> = (&db.post.accepted_answer).inv().collect();
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and((&accepted_by).opt()))
        .window(rank, |((_, s), _)| Reverse(s), asc);
    let rp = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts = db.tag.group_by(&db.tag.tag_name).select((&by_tag).map(|(p, _)| p).select(view_count.opt())).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let tw = whole(&ts).select(Same::<Str>::new().and(&ts)).window(rank, |(_, a)| (a[1] == 0, Reverse(a[2])), asc);
    let v = drain(rp.cross((&tw).filt(|(_, r)| r <= 5)));
    rows(v.into_iter().map(|(_, (p, ((t, a), _)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::S(t), V::I(a[0]), nullable(a[2], a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT ur.UserId, ur.Reputation, ur.TotalBounties, RANK() OVER (ORDER BY ur.Reputation + ur.TotalBounties DESC) AS UserRank FROM UserReputation ur)
// SELECT p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ur.Reputation AS OwnerReputation, ur.TotalBounties, tp.UserRank
// FROM RankedPosts p JOIN Users u ON p.OwnerUserId = u.Id JOIN UserReputation ur ON u.Id = ur.UserId JOIN TopUsers tp ON ur.UserId = tp.UserId
// WHERE p.rn = 1 AND p.Score > (SELECT AVG(Score) FROM Posts WHERE PostTypeId = 1) OR ur.Reputation > 5000
// ORDER BY tp.UserRank, p.Score DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
fn q1602(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let (ss, sn) = db.post.with(post_type_id.eq(1)).select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let base = || db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)).and(post_type_id.eq(1))).with(owner_user);
    let w = base().group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let ur = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let tw = whole(&ur).select(Ident::<User>::new().and(&ur).and(&db.user.reputation)).window(rank, |((_, b), r)| Reverse(r + b), asc);
    let tr: MatSet<(Id<User>, (Id<User>, (i64, i64)))> = (&tw).map(|(((u, b), _), r)| (u, (u, (b, r)))).collect();
    let tu = by_first(&tr);
    let good = Ident::<Post>::new().with(&first).with(score.filt(move |s| (s as i128) * (sn as i128) > ss as i128));
    let rich = owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(5000)));
    let v = drain(base().with(good.or(rich)).select(owner_user.select(&tu)));
    let v = top_n(v, |&(p, (_, (_, r)))| (r, Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (u, (b, r)))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.Score, p.CreationDate, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR' GROUP BY p.Id, u.DisplayName, p.Score, p.Title, p.CreationDate, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, Score, CreationDate, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.*, CASE WHEN tp.Score > 100 THEN 'High Scoring' WHEN tp.Score BETWEEN 50 AND 100 THEN 'Moderate Scoring' ELSE 'Low Scoring' END AS ScoreCategory,
//        CONCAT('https://stackoverflow.com/posts/', tp.PostId) AS PostLink
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// Rank reads only base columns, so the top ten posts per type are picked first and the comment x vote product is driven for those alone.
fn q8602(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "created"]);
        f.extend(a.map(V::I));
        f.push(V::S(if sc > 100 { "High Scoring" } else if sc >= 50 { "Moderate Scoring" } else { "Low Scoring" }));
        f.push(V::Owned(format!("https://stackoverflow.com/posts/{}", origid.get(p).unwrap())));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, CASE WHEN Reputation >= 1000 THEN 'High' WHEN Reputation >= 500 THEN 'Medium' ELSE 'Low' END AS ReputationCategory FROM Users),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(ph.UserId, -1) AS UserId
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, ph.UserId),
// RankedPosts AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.UpVotes, pd.DownVotes, ud.ReputationCategory,
//        ROW_NUMBER() OVER (PARTITION BY ud.ReputationCategory ORDER BY pd.UpVotes DESC) AS PostRank FROM PostDetails pd JOIN UserReputation ud ON pd.UserId = ud.Id)
// SELECT rp.ReputationCategory, COUNT(*) AS PostCount, AVG(rp.UpVotes) AS AvgUpVotes, AVG(rp.DownVotes) AS AvgDownVotes
// FROM RankedPosts rp WHERE rp.PostRank <= 5 GROUP BY rp.ReputationCategory ORDER BY rp.ReputationCategory;
//
// GROUP BY ph.UserId groups the joined post x close-history rows; the votes hang off each joined row's post.
fn q463(db: &'static So) -> String {
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = db.post.select(Ident::<Post>::new().and(closes.opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let pd = (&j)
        .group_by((&post_of).and((&hist_of).select(&db.post_history.user_id).opt()))
        .select((&post_of).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type K = (Id<Post>, Option<i64>);
    let cat = Same::<K>::new()
        .map(|(_, u): K| u.unwrap_or(-1))
        .select(&uidx)
        .select(&db.user.reputation)
        .map(|r: i64| if r >= 1000 { "High" } else if r >= 500 { "Medium" } else { "Low" });
    let w = whole(&pd).group_by(cat).select(Same::<K>::new().and(&pd)).window(row_number, |((p, _), a)| (Reverse(a[0]), p), asc);
    let g = (&w).filt(|(_, r)| r <= 5).fold([0i64; 3], |s, ((_, a), _)| [s[0] + 1, s[1] + a[0], s[2] + a[1]]);
    rows(drain(&g).into_iter().map(|(c, s)| row(vec![V::S(c), V::I(s[0]), avg(s[1], s[0]), avg(s[2], s[0])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, ARRAY_LENGTH(STRING_TO_ARRAY(p.Tags, '>'), 1) AS TagCount,
//        COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName, COALESCE(b.BadgeCount, 0) AS BadgeCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON u.Id = b.UserId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostScores AS (SELECT PostId, Title, Body, CreationDate, ViewCount, Score, TagCount, OwnerDisplayName, BadgeCount, (ViewCount + Score + TagCount + BadgeCount) AS TotalScore FROM RankedPosts),
// TopPosts AS (SELECT *, RANK() OVER (ORDER BY TotalScore DESC) AS Rank FROM PostScores WHERE TotalScore > 0)
// SELECT p.PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerDisplayName, p.BadgeCount, p.Rank FROM TopPosts p WHERE p.Rank <= 10 ORDER BY p.Rank;
fn q26277(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, view_count, score, tags_str, .. } = &db.post;
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let total = view_count.and(score).and(tags_str.map(|t: Str| t.split('>').count() as i64)).and(owner_user.select(&bc).opt()).map(|(((w, s), t), b)| w + s + t + b.unwrap_or(0));
    let w = whole(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))))
        .select(Ident::<Post>::new().and(total.filt(|x| x > 0)).and(owner_user.select(&bc).opt()))
        .window(rank, |((_, x), _)| Reverse(x), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, (((p, _), b), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(owner_user.get(p).map_or(V::S("Community User"), |u| user_col(db, u, "name")));
        f.extend([V::I(b.unwrap_or(0)), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, AVG(p.Score) AS AverageScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, PositivePosts, AverageScore, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats),
// RecentPostHistory AS (SELECT ph.PostId, p.Title, ph.CreationDate, ph.Comment, ph.PostHistoryTypeId, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS RevRank
//     FROM PostHistory ph INNER JOIN Posts p ON ph.PostId = p.Id WHERE ph.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
// SELECT tu.DisplayName, tu.TotalPosts, tu.PositivePosts, tu.AverageScore, rp.Title, rp.Comment AS LatestComment, ph.Comment AS ReasonForClosure
// FROM TopUsers tu LEFT JOIN RecentPostHistory rp ON tu.UserId = rp.PostId LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId AND ph.PostHistoryTypeId = 10
// WHERE tu.Rank <= 10 ORDER BY tu.TotalPosts DESC;
//
// tu.UserId = rp.PostId compares a user id with a post id, so it goes through the raw ids.
fn q3603(db: &'static So) -> String {
    let ups = user_posts(db);
    let tu = top_n(drain(&ups), |&(u, a)| (Reverse(a[1]), u), 10);
    let tu = rel(tu);
    let PostHistory { post, creation_date: hd, post_history_type_id, comment, .. } = &db.post_history;
    let recent: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(hd.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post).inv().collect();
    let closes: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(post).inv().collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    type T = (Id<User>, [i64; 10]);
    let rp = Ident::<PostHistory>::new().and(post.select(&closes).opt());
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(&db.user.origid).select(&pidx).select((&recent).select(rp)).opt())));
    rows(v.into_iter().map(|(_, ((u, a), h))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[8]), avg(a[4], a[1])];
        f.extend(match h {
            Some((h, c)) => [harness::fmt::ostr(db.post.title.get(post.get(h).unwrap())), ostr(comment.get(h)), c.map_or(V::Null, |c| ostr(comment.get(c)))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC) AS UserRank, p.OwnerUserId, p.LastActivityDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') AND p.Score > 0 GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.OwnerUserId, p.LastActivityDate),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT r.Title, r.Score, r.ViewCount, r.CommentCount, r.VoteCount, u.DisplayName AS Owner, u.Reputation AS OwnerReputation, u.PostCount AS OwnerPostCount, u.TotalScore AS OwnerTotalScore
// FROM RankedPosts r JOIN UserReputation u ON r.OwnerUserId = u.UserId WHERE r.UserRank = 1 ORDER BY r.Score DESC, r.ViewCount DESC LIMIT 10;
//
// UserRank reads only base columns, so the latest-active post per owner is picked first and the comment x vote product is driven for those alone.
fn q5457(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, last_activity_date, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(last_activity_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let ups = user_posts(db);
    let v = drain((&cc).and(&vc).and(owner_user.and(owner_user.select(&ups))));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, ((c, n), (u, a)))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[1]), V::I(a[4])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.CreationDate >= '2023-01-01' GROUP BY u.Id, u.DisplayName),
// BadgeCount AS (SELECT UserId, COUNT(*) AS TotalBadges FROM Badges GROUP BY UserId),
// RankActivity AS (SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.Questions, ua.Answers, ua.UpVotes, ua.DownVotes, COALESCE(bc.TotalBadges, 0) AS TotalBadges,
//        RANK() OVER (ORDER BY ua.UpVotes DESC, ua.TotalPosts DESC) AS ActivityRank FROM UserActivity ua LEFT JOIN BadgeCount bc ON ua.UserId = bc.UserId)
// SELECT ra.UserId, ra.DisplayName, ra.TotalPosts, ra.Questions, ra.Answers, ra.UpVotes, ra.DownVotes, ra.TotalBadges, ra.ActivityRank FROM RankActivity ra WHERE ra.ActivityRank <= 10 ORDER BY ra.ActivityRank;
fn q9342(db: &'static So) -> String {
    let s = db
        .user
        .with((&db.user.creation_date).ge(ts(2023, 1, 1, 0, 0, 0)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, vt)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (vt == Some(2)) as i64, a[4] + (vt == Some(3)) as i64],
            None => a,
        });
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let w = whole(&s).select(Ident::<User>::new().and(&s).and((&bc).opt())).window(rank, |((_, a), _)| (Reverse(a[3]), Reverse(a[0])), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, a), b), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(b.unwrap_or(0)), V::I(r)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews, AVG(p.AnswerCount) AS AvgAnswers, AVG(p.CommentCount) AS AvgComments
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// RankedUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, COALESCE(ubc.BadgeCount, 0) AS BadgeCount, ps.PostCount, ps.TotalScore, ps.TotalViews, ps.AvgAnswers, ps.AvgComments,
//        RANK() OVER (ORDER BY u.Reputation DESC, ps.TotalScore DESC) AS ReputationRank FROM Users u LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT ru.Id, ru.DisplayName, ru.Reputation, ru.BadgeCount, ru.PostCount, ru.TotalScore, ru.TotalViews, ru.AvgAnswers, ru.AvgComments, ru.ReputationRank
// FROM RankedUsers ru WHERE ru.ReputationRank <= 10 ORDER BY ru.ReputationRank;
fn q9641(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { owner_user, creation_date, score, view_count, answer_count, comment_count, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count))
        .fold([0i64; 7], |a, (((s, w), n), c)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + n.is_some() as i64, a[5] + n.unwrap_or(0), a[6] + c]);
    let w = whole(&ub).select(Ident::<User>::new().and(&ub).and((&ps).opt()).and(&db.user.reputation)).window(rank, |(((_, _), a), r)| (Reverse(r), a.is_none(), Reverse(a.map(|a| a[1]))), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, ((((u, b), a), _), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(b));
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[5], a[4]), avg(a[6], a[0])],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserPostStats AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, AVG(COALESCE(v.BountyAmount, 0)) AS AvgBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c WHERE c.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY c.PostId)
// SELECT pp.PostId, pp.Title, pp.CreationDate, pp.Score, COALESCE(rc.CommentCount, 0) AS CommentCount, ups.UserId, ups.PostCount, ups.TotalBounty, ups.AvgBounty
// FROM RankedPosts pp JOIN UserPostStats ups ON pp.PostId = ups.UserId LEFT JOIN RecentComments rc ON pp.PostId = rc.PostId
// WHERE pp.Rank <= 5 ORDER BY pp.Score DESC, pp.PostId ASC FETCH FIRST 10 ROWS ONLY;
//
// pp.PostId = ups.UserId compares a post id with a user id, so it goes through the raw ids.
fn q2460(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(bounty.opt()).opt())
        .fold([0i64; 4], |a, p| {
            let b = p.flatten().flatten();
            [a[0] + p.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0), a[3] + 1]
        });
    let rc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).ge(add_months(t0, -6)))).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&rc).and(origid.select(&uidx).select(Ident::<User>::new().and(&ups))));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), user_col(db, u, "uid"), V::I(a[0]), V::I(a[2]), avg(a[2], a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, pt.Name AS PostTypeName
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, OwnerDisplayName, PostTypeName FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.OwnerDisplayName, tp.PostTypeName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Badges b ON tp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = b.UserId)
// GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.OwnerDisplayName, tp.PostTypeName ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q6643(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user)
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let by_name: HashIdx<Str, Id<Badge>> = (&db.badge.user).select(&db.user.display_name).inv().collect();
    let named = || owner_user.select(&db.user.display_name).select(&by_name);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(named().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let bc = (&tp).group_by(Ident::<Post>::new()).select(named().opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&cc).and(&bc)).into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner", "type"]);
        f.extend([V::I(c), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= DATE '2023-01-01' AND p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// CloseReasonCounts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostID, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes, COALESCE(crc.CloseCount, 0) AS CloseCount,
//        CASE WHEN rp.Rank <= 5 THEN 'Top Post' ELSE 'Other Post' END AS PostRankCategory
// FROM RankedPosts rp LEFT JOIN CloseReasonCounts crc ON rp.PostID = crc.PostId WHERE rp.ViewCount > 100 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 10 OFFSET 5;
fn q115(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(date(2023, 1, 1)).and(post_type_id.is_in([1, 2])));
    let w = base().group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let rank = by_first(&rk);
    let s = base()
        .with(view_count.gt(100))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let cl = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&s).and((&cl).opt()).and(&rank));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p), 15);
    rows(v.into_iter().skip(5).map(|(p, ((a, c), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(a.map(V::I));
        f.extend([V::I(c.unwrap_or(0)), V::S(if r <= 5 { "Top Post" } else { "Other Post" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        COALESCE(CAST(NULLIF(SUBSTRING(p.Body, 1, 100), '') AS VARCHAR), 'No content') AS Snippet FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ups.UserId, ups.DisplayName, ups.PostCount, ups.TotalScore, ups.TotalViews, RANK() OVER (ORDER BY ups.TotalScore DESC) AS UserRank FROM UserPostStats ups WHERE ups.PostCount > 0)
// SELECT tu.DisplayName, tu.PostCount, tu.TotalScore, tu.TotalViews, rp.Title, rp.CreationDate, rp.Snippet
// FROM TopUsers tu LEFT JOIN RankedPosts rp ON tu.UserId = rp.PostId WHERE tu.UserRank <= 10 AND rp.Rank = 1 ORDER BY tu.TotalScore DESC, rp.CreationDate DESC;
//
// tu.UserId = rp.PostId compares a user id with a post id, so it goes through the raw ids.
fn q1376(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, body, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let ups = user_posts(db);
    let uw = whole((&ups).filt(|a| a[1] > 0)).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| Reverse(a[4]), asc);
    type T = (Id<User>, [i64; 10]);
    let tu: MatSet<T> = (&uw).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(&db.user.origid).select(&pidx).select(Ident::<Post>::new().with(&first)))));
    rows(v.into_iter().map(|(_, ((u, a), p))| {
        let b: String = body.get(p).unwrap().chars().take(100).collect();
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[4]), V::I(a[6])];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.push(if b.is_empty() { V::S("No content") } else { V::Owned(b) });
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(Id) AS BadgeCount FROM Badges GROUP BY UserId),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(a.AnswerCount, 0) AS AnswerCount,
//        COALESCE(b.BadgeCount, 0) AS UserBadgeCount
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(Id) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
//     LEFT JOIN UserBadgeCounts b ON p.OwnerUserId = b.UserId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.CommentCount, ps.AnswerCount, ps.UserBadgeCount,
//        RANK() OVER (ORDER BY ps.Score DESC, ps.ViewCount DESC) AS PostRank FROM PostStatistics ps)
// SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.CommentCount, r.AnswerCount, r.UserBadgeCount FROM RankedPosts r WHERE r.PostRank <= 100 ORDER BY r.PostRank;
//
// PostRank reads only base columns, so the top posts are picked first and the counts computed for those alone.
fn q7185(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user_id, .. } = &db.post;
    let w = whole(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 100).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&cc).and(&ac).and(owner_user_id.select(&bc).opt())).into_iter().map(|(p, ((c, a), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostComments AS (SELECT pc.PostId, COUNT(pc.Id) AS CommentCount FROM Comments pc GROUP BY pc.PostId),
// PostBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(pb.BadgeCount, 0) AS OwnerBadgeCount
// FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId LEFT JOIN Users u ON tp.OwnerDisplayName = u.DisplayName LEFT JOIN PostBadges pb ON u.Id = pb.UserId
// ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q7758(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let v = drain((&cc).and(owner_user.select(&db.user.display_name).select((&by_name).select((&bc).opt()))));
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, AnswerCount, CommentCount, OwnerDisplayName FROM RankedPosts WHERE Rank = 1),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments FROM Comments c GROUP BY c.PostId),
// PostScores AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.AnswerCount, tp.CommentCount, pco.TotalComments FROM TopPosts tp LEFT JOIN PostComments pco ON tp.PostId = pco.PostId)
// SELECT ps.Title, ps.CreationDate, ps.Score, ps.AnswerCount, ps.CommentCount, COALESCE(ps.TotalComments, 0) AS TotalComments FROM PostScores ps ORDER BY ps.Score DESC, ps.CreationDate ASC LIMIT 10;
fn q7808(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain(&cc), |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 10);
    rows(v.into_iter().map(|(p, c)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "answers", "comments"]);
        f.push(V::I(c));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Score DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= '2023-01-01' AND p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.ViewCount, rp.Score, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.rn <= 10),
// PostDetails AS (SELECT tp.Id, tp.Title, tp.ViewCount, tp.Score, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.Id = c.PostId LEFT JOIN Votes v ON tp.Id = v.PostId GROUP BY tp.Id, tp.Title, tp.ViewCount, tp.Score, tp.OwnerDisplayName)
// SELECT pd.Title, pd.ViewCount, pd.Score, pd.CommentCount, pd.VoteCount,
//        CASE WHEN pd.Score >= 50 THEN 'High Score' WHEN pd.Score BETWEEN 20 AND 49 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC;
fn q9912(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0)).and(post_type_id.is_in([1, 2])))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(view_count.opt()).and(score))
        .window(row_number, |((p, w), s)| (w.is_none(), Reverse(w), Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, n))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "views", "score"]);
        f.extend([V::I(c), V::I(n), V::S(if s >= 50 { "High Score" } else if s >= 20 { "Medium Score" } else { "Low Score" })]);
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// PopularTags AS (SELECT Tag, COUNT(*) AS TagUsage FROM PostTags GROUP BY Tag HAVING COUNT(*) > 5),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        COUNT(DISTINCT p.Id) AS TotalPosts FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName ORDER BY TotalUpvotes DESC LIMIT 10),
// TagPostCounts AS (SELECT pt.Tag, COUNT(DISTINCT p.Id) AS PostCount FROM PostTags pt JOIN Posts p ON pt.PostId = p.Id GROUP BY pt.Tag ORDER BY PostCount DESC)
// SELECT u.DisplayName AS TopUser, u.TotalUpvotes, u.TotalDownvotes, tg.Tag, tg.PostCount FROM TopUsers u JOIN TagPostCounts tg ON u.TotalPosts > 1
// WHERE tg.Tag IN (SELECT Tag FROM PopularTags) ORDER BY u.TotalUpvotes DESC, tg.PostCount DESC;
//
// The ON clause names only u, so the top users are crossed with the tags. COUNT(DISTINCT p.Id) per user comes from a second fold over one row per post.
fn q25458(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let pt = || db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list)));
    type PT = (Id<Post>, Str);
    let usage = pt().group_by(Same::<PT>::new().map(|(_, t): PT| t)).select(Same::<PT>::new()).fold(0i64, |n, _| n + 1);
    let tpc = pt().group_by(Same::<PT>::new().map(|(_, t): PT| t)).select(Same::<PT>::new().map(|(p, _): PT| p)).count_distinct();
    let tags = (&tpc).and((&usage).filt(|n| n > 5)).map(|(c, _)| c);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = rel(top_n(drain((&s).and(&dp)), |&(u, (a, _))| (Reverse(a[0]), u), 10));
    let v = drain((&tu).filt(|(_, (_, n))| n > 1).cross(tags));
    rows(v.into_iter().map(|((_, t), ((u, (a, _)), c))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::S(t), V::I(c)])))
}

// WITH UserReputation AS (SELECT Id AS UserId, DisplayName, Reputation, CASE WHEN Reputation < 500 THEN 'Newbie' WHEN Reputation BETWEEN 500 AND 1000 THEN 'Intermediate'
//        WHEN Reputation > 1000 THEN 'Expert' ELSE 'Unknown' END AS ReputationLevel FROM Users),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.Score > 0),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostBadges AS (SELECT p.Id AS PostId, COUNT(b.Id) AS BadgeCount FROM Posts p LEFT JOIN Badges b ON p.OwnerUserId = b.UserId GROUP BY p.Id)
// SELECT up.UserId, up.DisplayName, up.Reputation, up.ReputationLevel, pp.PostId, pp.Title, pp.Score, pp.ViewCount, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(pb.BadgeCount, 0) AS BadgeCount
// FROM UserReputation up JOIN PopularPosts pp ON up.UserId = pp.OwnerUserId LEFT JOIN PostComments pc ON pp.PostId = pc.PostId LEFT JOIN PostBadges pb ON pp.PostId = pb.PostId
// WHERE pp.Rank <= 5 ORDER BY up.Reputation DESC, pp.Score DESC;
fn q969(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, score, .. } = &db.post;
    let w = db.post.with(score.gt(0)).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bu: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let bc = (&tp).group_by(Ident::<Post>::new()).select(owner_user_id.select(&bu).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&cc).and(&bc).and(owner_user));
    rows(v.into_iter().map(|(p, ((c, b), u))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::S(if r < 500 { "Newbie" } else if r <= 1000 { "Intermediate" } else { "Expert" }));
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend([V::I(c), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2) AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10),
// PostStatistics AS (SELECT p.PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM TopPosts p LEFT JOIN Comments c ON p.PostId = c.PostId LEFT JOIN Votes v ON p.PostId = v.PostId GROUP BY p.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, ps.CommentCount, ps.UpVotes, ps.DownVotes, tp.OwnerDisplayName
// FROM TopPosts tp JOIN PostStatistics ps ON tp.PostId = ps.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q8449(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, TRIM(UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><'))) AS TagName FROM Posts p WHERE p.PostTypeId = 1 AND p.Tags IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostViews AS (SELECT p.Id AS PostId, SUM(p.ViewCount) AS TotalViews FROM Posts p WHERE p.PostTypeId = 1 GROUP BY p.Id),
// AggregatedData AS (SELECT pt.TagName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(pv.TotalViews) AS TotalViewCount, AVG(ur.Reputation) AS AvgUserReputation, SUM(ur.BadgeCount) AS TotalBadges
//     FROM PostTags pt JOIN Posts p ON pt.PostId = p.Id JOIN PostViews pv ON pv.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id JOIN UserReputation ur ON ur.UserId = u.Id GROUP BY pt.TagName)
// SELECT TagName, QuestionCount, TotalViewCount, AvgUserReputation, TotalBadges FROM AggregatedData ORDER BY QuestionCount DESC, TotalViewCount DESC;
fn q27685(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user, view_count, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pt = || db.post.with(post_type_id.eq(1)).with(owner_user).select(Ident::<Post>::new().and(tags_str.flat_map(|t: Str| tag_list(t).map(|x| x.trim()))));
    type PT = (Id<Post>, Str);
    let post = || Same::<PT>::new().map(|(p, _): PT| p);
    let tag = || Same::<PT>::new().map(|(_, t): PT| t);
    let agg = pt()
        .group_by(tag())
        .select(post().select(view_count.opt().and(owner_user.select((&db.user.reputation).and(&ub)))))
        .fold([0i64; 5], |a, (w, (r, b))| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + 1, a[3] + r, a[4] + b]);
    let qc = pt().group_by(tag()).select(post()).count_distinct();
    rows(drain((&qc).and(&agg)).into_iter().map(|(t, (q, a))| row(vec![V::S(t), V::I(q), nullable(a[1], a[0]), avg(a[3], a[2]), V::I(a[4])])))
}

// WITH UserBadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// TopQuestions AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(ub.BadgeCount, 0) AS TotalBadges, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId)
// SELECT ur.UserId, ur.Reputation, ur.TotalBadges, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges, tq.PostId, tq.Title, tq.Score, tq.CreationDate
// FROM UserReputation ur LEFT JOIN TopQuestions tq ON ur.UserId = tq.OwnerUserId WHERE tq.Rank <= 5 ORDER BY ur.Reputation DESC, tq.Score DESC LIMIT 100;
fn q7592(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(score).and(creation_date)).window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tq: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let v = drain((&tq).select(owner_user.select(Ident::<User>::new().and(&ub))));
    let v = top_n(v, |&(p, (u, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (u, b))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "score", "created"]));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// PostStats AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
//        COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.OwnerUserId),
// RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ub.BadgeCount, ps.PostId, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount, ps.QuestionCount, ps.AnswerCount,
//        RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT UserId, DisplayName, Reputation, BadgeCount, PostId, CommentCount, UpVoteCount, DownVoteCount, QuestionCount, AnswerCount, ReputationRank
// FROM RankedUsers WHERE ReputationRank <= 50 ORDER BY ReputationRank, Reputation DESC;
//
// The rank is over the users x posts rows and reads only Reputation, so those rows are ranked first and the post stats computed for the survivors alone.
fn q6262(db: &'static So) -> String {
    type R = ((Id<User>, Option<Id<Post>>), i64);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(posts_of(db).opt()).and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let r: MatSet<R> = (&w).filt(|(_, k)| k <= 50).map(|((x, _), k)| (x, k)).collect();
    let tp: MatSet<Id<Post>> = (&r).flat_map(|((_, p), _): R| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).opt()))
        .fold([0i64; 3], |a, ((t, c), _)| [a[0] + c.is_some() as i64, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let dv = |t: i64| (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(t))).select(&db.vote.user_id)).count_distinct();
    let (up, down) = (dv(2), dv(3));
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let stats = (&ps).and((&up).opt()).and((&down).opt());
    let v = drain((&r).select(Same::<R>::new().and(Same::<R>::new().map(|((u, _), _): R| u).select(&bc).opt()).and(Same::<R>::new().flat_map(|((_, p), _): R| p).select(&stats).opt())));
    rows(v.into_iter().map(|(_, ((((u, p), rk), b), s))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(oint(b));
        f.push(p.map_or(V::Null, |p| V::I(db.post.origid.get(p).unwrap())));
        f.extend(match s {
            Some(((a, up), dn)) => [V::I(a[0]), V::I(up.unwrap_or(0)), V::I(dn.unwrap_or(0)), V::I(a[1]), V::I(a[2])],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::I(rk));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rp.rn = 1 ORDER BY rp.Score DESC LIMIT 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.OwnerDisplayName, tp.CommentCount, tp.VoteCount, pht.Name AS PostHistoryType, ph.CreationDate AS HistoryCreationDate,
//        ph.UserDisplayName AS HistoryEditor
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// WHERE ph.CreationDate IS NOT NULL ORDER BY tp.Score DESC, ph.CreationDate DESC;
//
// Each group is one post, so rn = 1 always; the top ten questions by score are picked first.
fn q6728(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = drain((&cc).and(&vc).and(history_of(db)));
    rows(v.into_iter().map(|(p, ((c, n), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c), V::I(n), V::S(htype_name(db).get(h).unwrap()), V::T(db.post_history.creation_date.get(h).unwrap()), ostr(db.post_history.user_display_name.get(h))]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        AVG(P.Score) AS AverageScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalViews, PositivePosts, AverageScore, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserActivity),
// BadgeSummary AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId)
// SELECT U.DisplayName, U.PostCount, U.TotalViews, U.PositivePosts, U.AverageScore, COALESCE(B.BadgeCount, 0) AS BadgeCount, COALESCE(B.GoldBadges, 0) AS GoldBadges,
//        COALESCE(B.SilverBadges, 0) AS SilverBadges, COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM TopUsers U LEFT JOIN BadgeSummary B ON U.UserId = B.UserId WHERE U.Rank <= 10 ORDER BY U.Rank;
fn q3624(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ups = user_posts(db);
    let w = whole(&ups).select(Ident::<User>::new().and(&ups).and(&ub)).window(rank, |((_, a), _)| Reverse(a[1]), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, a), b), _))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[6]), V::I(a[8]), avg(a[4], a[1])];
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePostsCount,
//        SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePostsCount, SUM(COALESCE(UPVOTES.UpVoteCount, 0)) AS TotalUpVotes, SUM(COALESCE(DOWNVOTES.DownVoteCount, 0)) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS UpVoteCount FROM Votes V WHERE V.VoteTypeId = 2 GROUP BY PostId) UPVOTES ON P.Id = UPVOTES.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS DownVoteCount FROM Votes V WHERE V.VoteTypeId = 3 GROUP BY PostId) DOWNVOTES ON P.Id = DOWNVOTES.PostId GROUP BY U.Id, U.DisplayName)
// SELECT UA.DisplayName, UA.TotalPosts, UA.QuestionsCount, UA.AnswersCount, UA.PositivePostsCount, UA.NegativePostsCount, UA.TotalUpVotes, UA.TotalDownVotes,
//        RANK() OVER (ORDER BY UA.TotalPosts DESC) AS UserRank
// FROM UserActivity UA WHERE UA.TotalPosts > 0 ORDER BY UA.TotalPosts DESC, UA.DisplayName ASC LIMIT 10;
fn q7769(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let up = db.vote.with((&db.vote.vote_type_id).eq(2)).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let down = db.vote.with((&db.vote.vote_type_id).eq(3)).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and((&up).opt()).and((&down).opt())))
        .fold([0i64; 7], |a, (((t, s), u), d)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + (s < 0) as i64, a[5] + u.unwrap_or(0), a[6] + d.unwrap_or(0)]);
    let w = whole(&s).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let v = top_n(drain(&w), |&(_, ((u, a), _))| (Reverse(a[0]), db.user.display_name.get(u).unwrap()), 10);
    rows(v.into_iter().map(|(_, ((u, a), r))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(b.Class), 0) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostComments AS (SELECT cm.PostId, COUNT(*) AS CommentCount FROM Comments cm GROUP BY cm.PostId),
// PostLinks AS (SELECT pl.PostId, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostsCount FROM PostLinks pl GROUP BY pl.PostId)
// SELECT rp.PostId, rp.Title, u.DisplayName, u.Reputation, ur.TotalBadges, rp.CreationDate, rp.Score, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(pl.RelatedPostsCount, 0) AS RelatedPostsCount
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserReputation ur ON ur.UserId = rp.OwnerUserId LEFT JOIN PostComments pc ON pc.PostId = rp.PostId
// LEFT JOIN PostLinks pl ON pl.PostId = rp.PostId WHERE rp.rn = 1 AND ur.Reputation > 100 AND (rp.Score > 5 OR COALESCE(pl.RelatedPostsCount, 0) > 0) ORDER BY rp.Score DESC, rp.CreationDate DESC;
fn q2452(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let tb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + c.unwrap_or(0));
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let lc = db.post_link.group_by(&db.post_link.post).select(&db.post_link.related_post_id).count_distinct();
    let rich = owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100)));
    let keep = score.and((&lc).opt()).filt(|(s, l)| s > 5 || l.unwrap_or(0) > 0);
    let v = drain((&tp).with(rich).select(keep.and(owner_user.select(&tb)).and((&cc).opt())));
    rows(v.into_iter().map(|(p, (((_, l), b), c))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["created", "score"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(l.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.CommentCount, pt.Name AS PostTypeName
//     FROM RankedPosts rp JOIN PostTypes pt ON rp.PostRank <= 5 AND rp.PostId IN (SELECT DISTINCT PostId FROM Votes WHERE VoteTypeId IN (2, 3)))
// SELECT tp.PostId, tp.Title, tp.Score, tp.CreationDate, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.PostTypeName, COUNT(DISTINCT c.Id) AS TotalComments, AVG(u.Reputation) AS AverageAuthorReputation
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId JOIN Users u ON tp.PostId IN (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId)
// GROUP BY tp.PostId, tp.Title, tp.Score, tp.CreationDate, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.PostTypeName ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Neither ON names pt or u, so TopPosts crosses every post type and the final join crosses every user, keeping only posts whose raw id equals their owner's raw id.
fn q6760(db: &'static So) -> String {
    let Post { creation_date, score, origid, owner_user_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(ptype_name(db)).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let voted: MatSet<Id<Post>> = db.vote.with((&db.vote.vote_type_id).is_in([2, 3])).select(&db.vote.post).collect();
    let own = origid.and(owner_user_id).filt(|(a, b)| a == b);
    let keep: MatSet<Id<Post>> = (&tp).with(&voted).with(own).collect();
    type D = ((Id<Post>, Id<PostType>), Id<User>);
    let j = || (&keep).cross(&db.post_type).cross(&db.user);
    let key = || Same::<D>::new().map(|(k, _): D| k);
    let post = || Same::<D>::new().map(|((p, _), _): D| p);
    let g = j().group_by(key()).select(post().select(comments_of(db).opt()).and(Same::<D>::new().map(|(_, u): D| u).select(&db.user.reputation))).fold([0i64; 2], |a, (_, r)| [a[0] + r, a[1] + 1]);
    let dc = j().group_by(key()).select(post().select(comments_of(db))).count_distinct();
    rows(drain((&g).and((&dc).opt())).into_iter().map(|((p, t), ([s, n], c))| {
        let c = c.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "views", "answers", "comments"]);
        f.extend([V::S(db.post_type.name.get(t).unwrap()), V::I(c), avg(s, n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS QuestionsCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount, MIN(ph.CreationDate) AS FirstClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT u.DisplayName, r.PostId, r.Title, r.CreationDate, COALESCE(cp.CloseCount, 0) AS TotalClosed, cp.FirstClosedDate, us.QuestionsCount, us.TotalBounty,
//        RANK() OVER (ORDER BY us.QuestionsCount DESC, us.TotalBounty DESC) AS UserRank
// FROM RankedPosts r JOIN Users u ON r.OwnerUserId = u.Id LEFT JOIN UserStats us ON u.Id = us.UserId LEFT JOIN ClosedPosts cp ON r.PostId = cp.PostId
// WHERE r.rn = 1 ORDER BY UserRank, r.CreationDate DESC;
//
// COUNT(DISTINCT p.Id) comes from a fold over one row per question.
fn q1692(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let tb = db.user.group_by(Ident::<User>::new()).select(qs().select(bounty.opt()).opt()).fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let qc = db.user.group_by(Ident::<User>::new()).select(qs().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold((0i64, i64::MAX), |(n, m), d| (n + 1, m.min(d)));
    let w = whole(&tp).select(Ident::<Post>::new().and(owner_user.and(owner_user.select((&qc).and(&tb)))).and((&cp).opt())).window(rank, |((_, (_, (q, b))), _)| (Reverse(q), Reverse(b)), asc);
    rows(drain(&w).into_iter().map(|(_, (((p, (u, (q, b))), c), r))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend(match c {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        f.extend([V::I(q), V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// RecentEdits AS (SELECT p.Id AS PostId, p.Title, ph.UserDisplayName AS Editor, ph.CreationDate AS EditDate, ph.Comment,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS EditRank FROM Posts p INNER JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (4, 5, 6)),
// TopUsers AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId HAVING COUNT(*) > 3)
// SELECT u.DisplayName, u.Reputation, us.TotalVotes, us.UpVotes, us.DownVotes, re.Title, re.Editor, re.EditDate, CASE WHEN re.EditRank = 1 THEN 'Latest Edit' ELSE 'Earlier Edit' END AS EditStatus,
//        tb.BadgeCount
// FROM Users u LEFT JOIN UserVoteStats us ON u.Id = us.UserId LEFT JOIN RecentEdits re ON u.DisplayName = re.Editor LEFT JOIN TopUsers tb ON u.Id = tb.UserId
// WHERE u.Reputation >= 1000 AND (us.TotalVotes IS NULL OR us.TotalVotes > 10) ORDER BY u.Reputation DESC, us.TotalVotes DESC NULLS LAST LIMIT 50;
fn q3704(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, user_display_name, .. } = &db.post_history;
    let w = db.post_history.with(post_history_type_id.in_v(vec![4, 5, 6])).group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d)| (Reverse(d), h), asc);
    let re: MatSet<(Id<PostHistory>, i64)> = (&w).map(|((h, _), r)| (h, r)).collect();
    let by_editor: HashIdx<Str, (Id<PostHistory>, i64)> = (&re).map(|(h, _)| h).select(user_display_name).inv().collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with((&db.user.reputation).ge(1000)).select((&us).filt(|a| a[0] > 10).and((&db.user.display_name).select(&by_editor).opt()).and((&bc).filt(|n| n > 3).opt())));
    let v = top_n(v, |&(u, ((a, h), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), u, h.map(|x| x.0)), 50);
    rows(v.into_iter().map(|(u, ((a, h), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match h {
            Some((h, r)) => [ostr(db.post.title.get(post.get(h).unwrap())), ostr(user_display_name.get(h)), V::T(hd.get(h).unwrap()), V::S(if r == 1 { "Latest Edit" } else { "Earlier Edit" })],
            None => [V::Null, V::Null, V::Null, V::S("Earlier Edit")],
        });
        f.push(oint(b));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.Body, p.CreationDate, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount,
//        GREATEST(COALESCE(up.VoteCount, 0), COALESCE(down.VoteCount, 0), 0) AS Score
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS VoteCount FROM Votes GROUP BY PostId) up ON p.Id = up.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS VoteCount FROM Votes GROUP BY PostId) down ON p.Id = down.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Tags, p.Body, p.CreationDate, u.DisplayName, up.VoteCount, down.VoteCount),
// TopPosts AS (SELECT *, ROW_NUMBER() OVER (ORDER BY Score DESC, CreationDate DESC) AS Rank FROM RankedPosts)
// SELECT tp.PostId, tp.Title, tp.Tags, tp.Body, tp.CreationDate, tp.Author, tp.CommentCount, tp.Score FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Score DESC, tp.CommentCount DESC;
//
// The rank reads only the vote counts, so the top ten are picked first and their comments counted after.
fn q25117(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let vc = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let top = top_n(drain(&vc), |&(p, a)| (Reverse(a[0].max(a[1])), Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp = rel(top);
    type T = (Id<Post>, [i64; 2]);
    let cc = (&tp).group_by(Same::<T>::new()).select(Same::<T>::new().map(|(p, _): T| p).select(comments_of(db).opt())).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain(&cc).into_iter().map(|((p, a), c)| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "body", "created", "owner"]);
        f.extend([V::I(c), V::I(a[0].max(a[1]))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, COUNT(C.Id) AS CommentCount,
//        RANK() OVER (ORDER BY P.Score DESC, P.ViewCount DESC) AS RankScore
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.Score, P.ViewCount, U.DisplayName),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, Score, ViewCount, CommentCount FROM RankedPosts WHERE RankScore <= 10),
// PostMetrics AS (SELECT T.Title, T.OwnerDisplayName, T.Score, T.ViewCount, T.CommentCount, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = T.PostId AND V.VoteTypeId = 2) AS UpVotes,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = T.PostId AND V.VoteTypeId = 3) AS DownVotes FROM TopPosts T)
// SELECT PM.Title, PM.OwnerDisplayName, PM.Score, PM.ViewCount, PM.CommentCount, PM.UpVotes, PM.DownVotes,
//        ROUND((PM.UpVotes::decimal / NULLIF((PM.UpVotes + PM.DownVotes), 0)) * 100, 2) AS UpVotePercentage
// FROM PostMetrics PM ORDER BY PM.Score DESC, PM.ViewCount DESC;
fn q5454(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let w = whole(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.push(if a[0] + a[1] == 0 { V::Null } else { V::F((a[0] as f64 / (a[0] + a[1]) as f64 * 100.0 * 100.0).round() / 100.0) });
        row(f)
    }))
}

// Rewritten (rewrites/4860.sql): the final ORDER BY is tie-broken on PostId.
// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) as PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(p.Id) AS PostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// BadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// FinalResults AS (SELECT up.Id AS PostId, up.Title, up.CreationDate, ur.Reputation, COALESCE(bc.BadgeCount, 0) AS BadgeCount, up.Score, up.Tags
//     FROM RankedPosts up LEFT JOIN UserReputation ur ON up.OwnerUserId = ur.UserId LEFT JOIN BadgeCounts bc ON up.OwnerUserId = bc.UserId WHERE up.PostRank = 1)
// SELECT fr.PostId, fr.Title, fr.CreationDate, fr.Reputation, fr.BadgeCount, fr.Score, fr.Tags FROM FinalResults fr WHERE fr.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY fr.Score DESC, fr.PostId LIMIT 10;
fn q4860(db: &'static So) -> String {
    let (rs, rn) = (&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let Post { owner_user, owner_user_id, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let rich = Ident::<User>::new().with((&db.user.reputation).filt(move |r| (r as i128) * (rn as i128) > rs as i128));
    let v = drain((&tp).select(owner_user.select(rich.and((&bc).opt()))));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (u, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([user_col(db, u, "rep"), V::I(b.unwrap_or(0))]);
        f.extend(post_fields(db, p, &["score", "tags"]));
        row(f)
    }))
}

// WITH RecentPostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(a.AnswerCount, 0) AS AnswerCount,
//        COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND p.PostTypeId = 1),
// TopTags AS (SELECT Tags, COUNT(*) AS PostCount FROM RecentPostStats GROUP BY Tags ORDER BY PostCount DESC LIMIT 5),
// FilteredPosts AS (SELECT r.* FROM RecentPostStats r JOIN TopTags t ON r.Tags = t.Tags)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.CommentCount, fp.AnswerCount, fp.OwnerDisplayName, fp.Tags FROM FilteredPosts fp ORDER BY fp.Score DESC, fp.CreationDate DESC;
fn q6238(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.eq(1)));
    let tc = recent().group_by(tags_str.opt()).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 5);
    let tt: MatSet<Str> = rel(top).flat_map(|(t, _): (Option<Str>, i64)| t).collect();
    let fp = || recent().with(tags_str.with(&tt));
    let cc = fp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = fp().group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&cc).and(&ac)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), V::I(a), owner_user.get(p).map_or(V::S("Community User"), |u| user_col(db, u, "name"))]);
        f.extend(post_fields(db, p, &["tags"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(*) AS TotalPosts, SUM(v.BountyAmount) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName),
// RecentComments AS (SELECT c.PostId, COUNT(*) AS CommentCount FROM Comments c WHERE c.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '15 days' GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, us.DisplayName AS UserDisplayName, us.TotalPosts, us.TotalBounties, COALESCE(rc.CommentCount, 0) AS RecentCommentCount
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserStats us ON u.Id = us.UserId LEFT JOIN RecentComments rc ON rp.PostId = rc.PostId
// WHERE rp.PostRank <= 5 ORDER BY rp.PostId, rp.Score DESC;
fn q1581(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_days(t0, -30))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold([0i64; 3], |a, p| {
        let b = p.flatten().flatten();
        [a[0] + 1, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
    });
    let rc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).ge(add_days(t0, -15)))).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&rc).and(owner_user.and(owner_user.select(&us))));
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount FROM Posts p WHERE p.Score > 10),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, CASE WHEN U.Reputation >= 1000 THEN 'High' WHEN U.Reputation BETWEEN 500 AND 999 THEN 'Medium' ELSE 'Low' END AS ReputationCategory FROM Users U),
// PostsWithTags AS (SELECT p.Id, p.Title, p.Tags, COALESCE(NULLIF(t.TagName, ''), 'No Tags') AS TagName FROM Posts p LEFT JOIN Tags t ON t.WikiPostId = p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, up.ReputationCategory, pt.TagName, CASE WHEN rp.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus
// FROM RankedPosts rp JOIN UserReputation up ON up.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) JOIN PostsWithTags pt ON pt.Id = rp.PostId
// WHERE rp.rn = 1 ORDER BY rp.Score DESC, rp.CreationDate DESC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
fn q3519(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(score.gt(10)).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let wiki: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let v = drain((&cc).and(owner_user).and((&wiki).select(&db.tag.tag_name).opt()));
    let v = top_n(v, |&(p, ((_, _), t))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, t), 20);
    rows(v.into_iter().skip(10).map(|(p, ((c, u), t))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::S(if r >= 1000 { "High" } else if r >= 500 { "Medium" } else { "Low" }));
        f.push(V::S(t.filter(|t| !t.is_empty()).unwrap_or("No Tags")));
        f.push(V::S(if c > 0 { "Has Comments" } else { "No Comments" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) as rn,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, COUNT(v.Id) OVER (PARTITION BY p.Id, v.VoteTypeId) AS UpVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment, ph.UserDisplayName FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10),
// FinalReport AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, COALESCE(cp.Comment, 'No comments on closed posts') AS CloseComment
//     FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.rn <= 5)
// SELECT fr.PostId, fr.Title, fr.CreationDate, fr.Score, fr.ViewCount, fr.CommentCount, fr.UpVoteCount, fr.CloseComment FROM FinalReport fr ORDER BY fr.CreationDate DESC LIMIT 50;
//
// No GROUP BY: the windows number and count the post x comment x upvote rows, so those rows are materialised. Rows of one post differ only in the ids, which are not projected.
fn q3004(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, .. } = &db.post;
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let j: MatSet<((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>)> =
        db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(Ident::<Post>::new().and(comments_of(db).opt()).and(up.opt())).collect();
    type J = ((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>);
    let post_of = || Same::<J>::new().map(|((p, _), _): J| p);
    let cnt = (&j).group_by(post_of()).select(Same::<J>::new()).fold([0i64; 2], |a, ((_, c), v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let w = (&j).group_by(post_of().select(owner_user_id.opt())).select(Same::<J>::new().and(post_of().select(creation_date))).window(row_number, |(((p, c), v), d)| (Reverse(d), p, c, v), asc);
    let rr: MatSet<J> = (&w).filt(|(_, r)| r <= 5).map(|(x, _)| x.0).collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&rr).select(Same::<J>::new().and(post_of().select(&cnt)).and(post_of().select(closes.opt()))));
    let v = top_n(v, |&(_, ((((p, c), w), _), h))| (Reverse(creation_date.get(p).unwrap()), p, c, w, h), 50);
    rows(v.into_iter().map(|(_, ((((p, _), _), a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(h.and_then(|h| db.post_history.comment.get(h)).map_or(V::S("No comments on closed posts"), V::S));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS OwnerRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount, rp.OwnerRank, ROW_NUMBER() OVER (ORDER BY rp.Score DESC) AS GlobalRank
//     FROM RankedPosts rp WHERE rp.OwnerRank = 1)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.GlobalRank, COALESCE(b.Count, 0) AS UserBadgeCount
// FROM TopPosts tp LEFT JOIN (SELECT UserId, COUNT(*) AS Count FROM Badges GROUP BY UserId) b ON tp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = b.UserId)
// WHERE tp.GlobalRank <= 10 ORDER BY tp.GlobalRank;
//
// Both ranks read only Score, so the ten posts are picked first and the comment x vote product is driven for those alone.
fn q5532(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let first: MatSet<(Id<Post>, i64)> = (&w).filt(|(_, r)| r == 1).map(|(x, _)| x).collect();
    let gw = whole(&first).select(Same::<(Id<Post>, i64)>::new()).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<(Id<Post>, i64)> = (&gw).filt(|(_, g)| g <= 10).map(|((p, _), g)| (p, g)).collect();
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let cc = (&tps).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, (Id<User>, i64)> = db.user.with(&bc).select(&db.user.display_name).inv().select(Ident::<User>::new().and(&bc)).collect();
    type T = (Id<Post>, i64);
    let v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select((&cc).and(owner_user.select(&db.user.display_name).select(&by_name).opt())))));
    rows(v.into_iter().map(|(_, ((p, g), (c, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(g), V::I(b.map_or(0, |b| b.1))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(v.BountyAmount) AS TotalBounties
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopBadgedUsers AS (SELECT us.DisplayName, us.Reputation, us.BadgeCount, RANK() OVER (ORDER BY us.BadgeCount DESC, us.Reputation DESC) AS BadgeRank, us.UserId FROM UserStats us WHERE us.BadgeCount > 0)
// SELECT tb.DisplayName, tb.Reputation, COALESCE(rp.Title, 'No Posts Found') AS RecentPostTitle, COALESCE(rp.Score, 0) AS RecentPostScore, ts.TotalBounties, tb.BadgeCount AS UserBadgeCount, tb.BadgeRank
// FROM TopBadgedUsers tb LEFT JOIN RecentPosts rp ON tb.UserId = rp.OwnerUserId AND rp.RecentPostRank = 1 LEFT JOIN UserStats ts ON tb.UserId = ts.UserId
// WHERE tb.BadgeCount = (SELECT MAX(BadgeCount) FROM TopBadgedUsers) ORDER BY tb.Reputation DESC LIMIT 10;
fn q2207(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (b, v)| {
            let x = v.flatten();
            [a[0] + b.is_some() as i64, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0)]
        });
    type T = (Id<User>, ([i64; 3], i64));
    let tw = whole((&us).filt(|a| a[0] > 0)).select(Ident::<User>::new().and(&us).and(&db.user.reputation)).window(rank, |((_, a), r)| (Reverse(a[0]), Reverse(r)), asc);
    let tb: MatSet<T> = (&tw).map(|(((u, a), _), r)| (u, (a, r))).collect();
    let most = (&tb).fold_flat(0i64, |m, (_, (a, _))| m.max(a[0]));
    let Post { owner_user, creation_date, .. } = &db.post;
    let rw = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let recent: HashIdx<Id<User>, Id<Post>> = (&rw).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let v = drain((&tb).filt(move |(_, (a, _))| a[0] == most).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select((&recent).opt().and(&us)))));
    let v = top_n(v, |&(_, ((u, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(_, ((u, (a, r)), (p, s)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(p.and_then(|p| db.post.title.get(p)).map_or(V::S("No Posts Found"), V::S));
        f.push(V::I(p.map_or(0, |p| db.post.score.get(p).unwrap())));
        f.extend([nullable(s[2], s[1]), V::I(a[0]), V::I(r)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostReputation AS (SELECT P.OwnerUserId, SUM(V.BountyAmount) AS TotalBounty, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
//        COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 GROUP BY P.OwnerUserId),
// UserDetails AS (SELECT U.DisplayName, COALESCE(UR.Reputation, 0) AS Reputation, COALESCE(PR.TotalBounty, 0) AS TotalBounty, COALESCE(PR.TotalPosts, 0) AS TotalPosts,
//        COALESCE(PR.TotalQuestions, 0) AS TotalQuestions, COALESCE(PR.TotalAnswers, 0) AS TotalAnswers
//     FROM Users U LEFT JOIN UserReputation UR ON U.Id = UR.UserId LEFT JOIN PostReputation PR ON U.Id = PR.OwnerUserId)
// SELECT UD.DisplayName, UD.Reputation, UD.TotalBounty, UD.TotalPosts, UD.TotalQuestions, UD.TotalAnswers,
//        CASE WHEN UD.Reputation > 1000 THEN 'High Reputation' WHEN UD.Reputation BETWEEN 500 AND 1000 THEN 'Moderate Reputation' ELSE 'Low Reputation' END AS ReputationCategory
// FROM UserDetails UD WHERE UD.TotalPosts > 0 ORDER BY UD.Reputation DESC LIMIT 10;
fn q1584(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let pr = db.post.group_by(owner_user).select(post_type_id.and(bounty.opt())).fold([0i64; 4], |a, (t, b)| {
        let b = b.flatten();
        [a[0] + b.unwrap_or(0), a[1] + 1, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64]
    });
    let v = top_n(drain((&pr).filt(|a| a[1] > 0)), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Moderate Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(P.ViewCount) AS TotalViews, AVG(COALESCE(P.Score, 0)) AS AvgScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopBadgers AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount FROM Badges B GROUP BY B.UserId),
// RankedUsers AS (SELECT UA.UserId, UA.DisplayName, UA.TotalPosts, UA.TotalQuestions, UA.TotalAnswers, UA.TotalViews, UA.AvgScore, TB.BadgeCount,
//        RANK() OVER (ORDER BY UA.TotalViews DESC) AS RankByViews, RANK() OVER (ORDER BY UA.AvgScore DESC) AS RankByScore FROM UserActivity UA LEFT JOIN TopBadgers TB ON UA.UserId = TB.UserId)
// SELECT R.DisplayName, R.TotalPosts, R.TotalQuestions, R.TotalAnswers, R.TotalViews, R.AvgScore, COALESCE(R.BadgeCount, 0) AS BadgeCount, R.RankByViews, R.RankByScore
// FROM RankedUsers R WHERE R.TotalPosts > 10 AND (R.RankByViews <= 10 OR R.RankByScore <= 10) ORDER BY R.RankByViews, R.RankByScore;
fn q1766(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    type R0 = ((Id<User>, [i64; 10]), Option<i64>);
    let w = whole(&ups).select(Ident::<User>::new().and(&ups).and((&bc).opt())).window(rank, |((_, a), _)| (a[5] == 0, Reverse(a[6])), asc);
    let w = (&w).window(rank, |(((_, a), _), _): (R0, i64)| Reverse(fkey(a[4] as f64 / a[0] as f64)), asc);
    let v = drain((&w).filt(|((((_, a), _), w), s): ((R0, i64), i64)| a[1] > 10 && (w <= 10 || s <= 10)));
    rows(v.into_iter().map(|(_, ((((u, a), b), w), s))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), avg(a[4], a[0])];
        f.extend([V::I(b.unwrap_or(0)), V::I(w), V::I(s)]);
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, p.ViewCount, p.Score, COALESCE(COUNT(ans.Id), 0) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS CloseVotes, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END), 0) AS ReopenVotes,
//        u.DisplayName AS OwnerDisplayName
//     FROM Posts p LEFT JOIN Posts ans ON p.Id = ans.ParentId LEFT JOIN Votes vt ON p.Id = vt.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.LastActivityDate, p.ViewCount, p.Score, u.DisplayName),
// RankedPosts AS (SELECT pm.*, RANK() OVER (ORDER BY pm.Score DESC, pm.AnswerCount DESC, pm.ViewCount DESC) AS PostRank FROM PostMetrics pm)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.LastActivityDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.UpVotes, rp.DownVotes, rp.CloseVotes, rp.ReopenVotes, rp.OwnerDisplayName
// FROM RankedPosts rp WHERE rp.PostRank <= 100 ORDER BY rp.PostRank;
fn q5956(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 5], |a, ((c, v), h)| [a[0] + c.is_some() as i64, a[1] + (v == Some(2)) as i64, a[2] + (v == Some(3)) as i64, a[3] + (h == Some(10)) as i64, a[4] + (h == Some(11)) as i64]);
    let w = whole(&s)
        .select(Ident::<Post>::new().and(&s).and(score).and(view_count.opt()))
        .window(rank, |(((_, a), s), w)| (Reverse(s), Reverse(a[0]), w.is_none(), Reverse(w)), asc);
    rows(drain((&w).filt(|(_, r)| r <= 100)).into_iter().map(|(_, ((((p, a), _), _), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity", "views", "score"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId),
// UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COALESCE(UBC.BadgeCount, 0) AS BadgeCount, PS.TotalPosts, PS.TotalQuestions,
//        PS.TotalAnswers, PS.TotalScore, PS.TotalViews, PS.AverageScore FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId)
// SELECT UserId, DisplayName, Reputation, BadgeCount, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, AverageScore, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
// FROM UserEngagement WHERE Reputation > 1000 ORDER BY TotalScore DESC, Reputation DESC LIMIT 50;
fn q7723(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let w = whole(db.user.with((&db.user.reputation).gt(1000))).select(Ident::<User>::new().and(&ub).and(&ups).and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let v = top_n(drain(&w), |&(_, ((((u, _), a), r), _))| (a[1] == 0, Reverse(a[4]), Reverse(r), u), 50);
    rows(v.into_iter().map(|(_, ((((u, b), a), _), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(b));
        f.extend(if a[1] == 0 {
            [V::Null, V::Null, V::Null, V::Null, V::Null, V::Null]
        } else {
            [V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), nullable(a[6], a[5]), avg(a[4], a[1])]
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount, COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, Views, AnswerCount, QuestionCount, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT RU.DisplayName, RU.Reputation, RU.Views, RU.AnswerCount, RU.QuestionCount, RU.GoldBadges, RU.SilverBadges, RU.BronzeBadges, CONCAT('Rank: ', CAST(RU.ReputationRank AS VARCHAR)) AS Ranking
// FROM RankedUsers RU WHERE RU.AnswerCount > 5 AND RU.ReputationRank <= 100 AND (RU.GoldBadges > 0 OR RU.SilverBadges > 0) ORDER BY RU.Reputation DESC;
fn q4697(db: &'static So) -> String {
    let s = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(1)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]);
    let w = whole(&s).select(Ident::<User>::new().and(&s).and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let v = drain((&w).filt(|(((_, a), _), r)| a[0] > 5 && r <= 100 && (a[2] > 0 || a[3] > 0)));
    rows(v.into_iter().map(|(_, (((u, a), _), r))| {
        let mut f = ucols(db, u, &["name", "rep", "uviews"]);
        f.extend(a.map(V::I));
        f.push(V::Owned(format!("Rank: {r}")));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = u.Id) AS TotalPosts FROM Users u WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users)),
// TopUsers AS (SELECT ur.UserId, ur.DisplayName, ur.Reputation, ur.TotalPosts, ROW_NUMBER() OVER (ORDER BY ur.Reputation DESC) AS UserRank FROM UserReputation ur)
// SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, tu.UserId, tu.DisplayName AS UserName, tu.Reputation AS UserReputation
// FROM RecentPosts rp JOIN Posts p ON rp.Id = p.Id LEFT JOIN TopUsers tu ON p.OwnerUserId = tu.UserId WHERE rp.rn = 1 AND (rp.CommentCount > 5 OR rp.Score > 10) ORDER BY rp.ViewCount DESC LIMIT 50;
fn q3489(db: &'static So) -> String {
    let (rs, rn) = (&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let Post { owner_user, owner_user_id, creation_date, score, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rich = Ident::<User>::new().with((&db.user.reputation).filt(move |r| (r as i128) * (rn as i128) > rs as i128));
    let v = drain((&cc).and(score).filt(|(c, s)| c > 5 || s > 10).and(owner_user.select(rich).opt()));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, ((c, _), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(c));
        f.extend(match u {
            Some(u) => ucols(db, u, &["uid", "name", "rep"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// PostHistoryStats AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS HistoryCount FROM PostHistory ph GROUP BY ph.PostId, ph.PostHistoryTypeId),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Class = 1 /* Gold */ GROUP BY b.UserId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, COALESCE(phs.HistoryCount, 0) AS TotalEditCount,
//        COALESCE(ub.BadgeCount, 0) AS GoldBadgeCount
// FROM RankedPosts rp LEFT JOIN PostHistoryStats phs ON rp.PostId = phs.PostId LEFT JOIN UserBadges ub ON rp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = ub.UserId)
// WHERE rp.Rank <= 5 /* Top 5 posts by type */ ORDER BY rp.CreationDate DESC;
fn q6494(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let phs = db.post_history.group_by(post.and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pv = rel(drain(&phs));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&pv).map(|((p, _), _)| p).inv().select(&pv).collect();
    let gold = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, (Id<User>, i64)> = db.user.with(&gold).select(&db.user.display_name).inv().select(Ident::<User>::new().and(&gold)).collect();
    let v = drain((&tp).select((&by_post).opt().and(owner_user.select(&db.user.display_name).select(&by_name).opt())));
    rows(v.into_iter().map(|(p, (h, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([V::I(h.map_or(0, |x| x.1)), V::I(b.map_or(0, |x| x.1))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Score,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY COUNT(c.Id) DESC, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.OwnerUserId, u.DisplayName, p.PostTypeId),
// FilteredPosts AS (SELECT rp.*, (SELECT COUNT(*) FROM Votes WHERE PostId = rp.PostId AND VoteTypeId = 1) AS AcceptedVotes,
//        (SELECT COUNT(*) FROM Badges WHERE UserId = rp.OwnerUserId AND Class = 1) AS GoldBadgeCount FROM RankedPosts rp WHERE Rank <= 10)
// SELECT fp.PostId, fp.Title, fp.OwnerDisplayName, fp.CommentCount, fp.Score, fp.AcceptedVotes, fp.GoldBadgeCount FROM FilteredPosts fp ORDER BY fp.Score DESC, fp.CommentCount DESC;
fn q5696(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64 - (t == Some(3)) as i64]);
    type T = (Id<Post>, [i64; 2]);
    let w = db.post.with(&s).group_by(post_type_id).select(Ident::<Post>::new().and(&s)).window(row_number, |(p, a)| (Reverse(a[0]), Reverse(a[1]), p), asc);
    let tp: MatSet<T> = (&w).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let acc = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(1)));
    let gold: HashIdx<i64, Id<Badge>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user_id).inv().collect();
    let ac = (&tp).group_by(Same::<T>::new()).select(Same::<T>::new().map(|(p, _): T| p).select(acc.opt())).fold(0i64, |n, v| n + v.is_some() as i64);
    let gc = (&tp).group_by(Same::<T>::new()).select(Same::<T>::new().map(|(p, _): T| p).select(owner_user_id.select(&gold)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&ac).and(&gc)).into_iter().map(|((p, a), (n, g))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::I(g)]);
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId
//     WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// UserRanked AS (SELECT UserId, DisplayName, PostCount, CommentCount, Upvotes, Downvotes, GoldBadges, SilverBadges, BronzeBadges,
//        DENSE_RANK() OVER (ORDER BY PostCount DESC, Upvotes DESC) AS EngagementRank FROM UserEngagement)
// SELECT UserId, DisplayName, PostCount, CommentCount, Upvotes, Downvotes, GoldBadges, SilverBadges, BronzeBadges, EngagementRank FROM UserRanked WHERE EngagementRank <= 10 ORDER BY EngagementRank;
//
// The COUNT(DISTINCT)s come from folds over one row per post and one row per comment.
fn q5162(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (p, c)| {
            let t = p.and_then(|x| x.1);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(&s).select(Ident::<User>::new().and(&s).and(&pc).and(&cc)).window(dense_rank, |(((_, a), p), _)| (Reverse(p), Reverse(a[0])), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, ((((u, a), p), c), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p), V::I(c)]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score >= 10),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostStatistics AS (SELECT tp.PostId, tp.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title)
// SELECT ps.PostId, ps.Title, ps.CommentCount, ps.UpvoteCount, ps.DownvoteCount, tp.CreationDate, tp.ViewCount, tp.Score, tp.AnswerCount
// FROM PostStatistics ps JOIN TopPosts tp ON ps.PostId = tp.PostId ORDER BY ps.UpvoteCount DESC, tp.Score DESC;
fn q7207(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.ge(10)))
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), d, p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["created", "views", "score", "answers"]));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(*) AS TotalBadges, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges GROUP BY UserId),
// PostStats AS (SELECT OwnerUserId, COUNT(CASE WHEN PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN PostTypeId = 2 THEN 1 END) AS Answers, SUM(ViewCount) AS TotalViews, SUM(Score) AS TotalScore
//     FROM Posts GROUP BY OwnerUserId),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UBC.TotalBadges, 0) AS TotalBadges, COALESCE(PS.Questions, 0) AS TotalQuestions, COALESCE(PS.Answers, 0) AS TotalAnswers,
//        COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(PS.TotalScore, 0) AS TotalScore, DENSE_RANK() OVER (ORDER BY COALESCE(PS.TotalViews, 0) DESC, COALESCE(PS.TotalScore, 0) DESC) AS Rank
//     FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId WHERE U.Reputation > 0)
// SELECT UserId, DisplayName, TotalBadges, TotalQuestions, TotalAnswers, TotalViews, TotalScore, Rank FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q22746(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let w = whole(db.user.with((&db.user.reputation).gt(0))).select(Ident::<User>::new().and(&ub).and(&ups)).window(dense_rank, |(_, a)| (Reverse(a[6]), Reverse(a[4])), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, b), a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, Score, ViewCount, CreationDate FROM RankedPosts WHERE PostRank <= 5),
// PostVoteCounts AS (SELECT PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpvoteCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownvoteCount FROM Votes v GROUP BY PostId),
// PostCommentCounts AS (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.Score, tp.ViewCount, tp.CreationDate, pv.UpvoteCount, pv.DownvoteCount, pc.CommentCount
// FROM TopPosts tp LEFT JOIN PostVoteCounts pv ON tp.PostId = pv.PostId LEFT JOIN PostCommentCounts pc ON tp.PostId = pc.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q6104(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&tp).select((&pv).opt().and((&pc).opt()))).into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views", "created"]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        f.push(oint(c));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerName, p.CreationDate, p.Score, p.ViewCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerName, rp.CreationDate, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.RankScore <= 10),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT trp.PostId, trp.Title, trp.OwnerName, trp.CreationDate, trp.Score, trp.ViewCount, COALESCE(pc.CommentCount, 0) AS TotalComments, COALESCE(pb.BadgeCount, 0) AS OwnerBadgeCount
// FROM TopRankedPosts trp LEFT JOIN PostComments pc ON trp.PostId = pc.PostId LEFT JOIN Users u ON trp.OwnerName = u.DisplayName LEFT JOIN PostBadges pb ON u.Id = pb.UserId
// ORDER BY trp.Score DESC, trp.ViewCount DESC;
fn q9844(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let v = drain((&cc).and(owner_user.select(&db.user.display_name).select((&by_name).select((&bc).opt()))));
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(c), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, bt.Name AS BadgeName, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM RankedPosts rp LEFT JOIN Badges bt ON rp.PostId = bt.UserId LEFT JOIN Comments c ON rp.PostId = c.PostId WHERE rp.RankScore <= 10
//     GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, bt.Name)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.ViewCount, fp.Score, fp.AnswerCount, fp.CommentCount, fp.OwnerDisplayName, fp.BadgeName, fp.TotalComments
// FROM FilteredPosts fp WHERE fp.TotalComments > 5 ORDER BY fp.Score DESC, fp.ViewCount DESC;
//
// rp.PostId = bt.UserId compares a post id with a user id, so it goes through the raw ids. Every (post, badge name) group sees all the post's comments, and the COUNT(DISTINCT) counts each once.
fn q7614(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let names: HashIdx<i64, Str> = (&db.badge.user_id).inv().select(&db.badge.name).collect();
    let g = (&tp).group_by(Ident::<Post>::new().and(origid.select(&names).opt())).select(comments_of(db)).count_distinct();
    rows(drain((&g).filt(|n| n > 5)).into_iter().map(|((p, b), n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner"]);
        f.extend([ostr(b), V::I(n)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PopularUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, PopularPosts, Upvotes, Downvotes, (Upvotes - Downvotes) AS VoteBalance FROM UserStats WHERE TotalPosts > 5),
// UserRankings AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, PopularPosts, Upvotes, Downvotes, VoteBalance, RANK() OVER (ORDER BY VoteBalance DESC) AS Rank FROM PopularUsers)
// SELECT ur.Rank, ur.DisplayName, ur.TotalPosts, ur.TotalQuestions, ur.TotalAnswers, ur.PopularPosts, ur.Upvotes, ur.Downvotes FROM UserRankings ur WHERE ur.Rank <= 10 ORDER BY ur.Rank;
//
// COUNT(DISTINCT p.Id) comes from a fold over one row per post.
fn q8329(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, w), vt)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.map_or(false, |w| w > 100) as i64, a[3] + (vt == Some(2)) as i64, a[4] + (vt == Some(3)) as i64],
            None => a,
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole((&dp).filt(|n| n > 5)).select(Ident::<User>::new().and(&dp).and(&s)).window(rank, |(_, a)| Reverse(a[3] - a[4]), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, n), a), r))| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, GoldBadges, SilverBadges, BronzeBadges FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// COUNT(DISTINCT p.Id) comes from a fold over one row per post.
fn q5579(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, c)| {
            let t = p.map(|x| x.0);
            let vt = p.and_then(|x| x.1);
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (vt == Some(2)) as i64, a[3] + (vt == Some(3)) as i64, a[4] + (c == Some(1)) as i64, a[5] + (c == Some(2)) as i64, a[6] + (c == Some(3)) as i64]
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&pc).select(Ident::<User>::new().and(&pc).and(&s)).window(rank, |((_, n), _)| Reverse(n), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, n), a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation >= 1000 GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, rp.VoteCount, au.DisplayName AS ActiveUserName, au.GoldBadges, au.SilverBadges, au.BronzeBadges
// FROM RankedPosts rp JOIN ActiveUsers au ON rp.OwnerDisplayName = au.DisplayName WHERE rp.PostRank <= 5 ORDER BY rp.CreationDate DESC, rp.VoteCount DESC;
fn q6989(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let au = db.user.with((&db.user.reputation).ge(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let by_name: HashIdx<Str, (Id<User>, [i64; 3])> = db.user.with(&au).select(&db.user.display_name).inv().select(Ident::<User>::new().and(&au)).collect();
    let v = drain((&cc).and(&vc).and(owner_user.select(&db.user.display_name).select(&by_name)));
    rows(v.into_iter().map(|(p, ((c, n), (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(n), user_col(db, u, "name")]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS WikiCount, SUM(p.ViewCount) AS TotalViews,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, QuestionCount, AnswerCount, WikiCount, TotalViews, UpVotes, DownVotes, BadgeCount, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, QuestionCount, AnswerCount, WikiCount, TotalViews, UpVotes, DownVotes, BadgeCount FROM TopUsers WHERE PostRank <= 10 ORDER BY TotalPosts DESC;
fn q29347(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let s = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 9], |a, (p, b)| match p {
            Some(((t, w), vt)) => [
                a[0] + 1,
                a[1] + (t == 1) as i64,
                a[2] + (t == 2) as i64,
                a[3] + matches!(t, 3 | 4 | 5) as i64,
                a[4] + w.is_some() as i64,
                a[5] + w.unwrap_or(0),
                a[6] + (vt == Some(2)) as i64,
                a[7] + (vt == Some(3)) as i64,
                a[8] + b.is_some() as i64,
            ],
            None => {
                let mut a = a;
                a[8] += b.is_some() as i64;
                a
            }
        });
    let w = whole(&s).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[0]), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4]), V::I(a[6]), V::I(a[7]), V::I(a[8])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(COALESCE(b.Class, 0)) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// LatestEdits AS (SELECT ph.PostId, ph.UserId, ph.CreationDate AS EditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6))
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.PositivePosts, us.NegativePosts, us.TotalBadges, rp.Title, rp.CreationDate AS PostCreationDate, le.EditDate
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId AND rp.PostRank = 1 LEFT JOIN LatestEdits le ON rp.Id = le.PostId WHERE us.Reputation > 100
// ORDER BY us.TotalPosts DESC, us.Reputation DESC;
fn q7756(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(100));
    let us = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (s, c)| [a[0] + s.is_some() as i64, a[1] + s.map_or(false, |s| s > 0) as i64, a[2] + s.map_or(false, |s| s < 0) as i64, a[3] + c.unwrap_or(0)]);
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).in_v(vec![4, 5, 6])));
    let v = drain((&tp).select(owner_user.and(owner_user.select(&us))).and(edits.opt()));
    rows(v.into_iter().map(|(p, ((u, a), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "created"]));
        f.push(h.map_or(V::Null, |h| V::T(db.post_history.creation_date.get(h).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation > 0)
// SELECT up.UserId, up.DisplayName, up.Reputation, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.AnswerCount, rp.UpvoteCount, rp.DownvoteCount
// FROM RankedPosts rp JOIN UserReputation up ON rp.PostRank = 1 AND up.UserId = rp.OwnerUserId ORDER BY up.Reputation DESC, rp.Score DESC LIMIT 10;
//
// PostRank reads only base columns, so the newest question per owner is picked first; the COUNT(DISTINCT)s are folded over one child at a time and the vote sums over the whole product.
fn q8233(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let owners = owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(0)));
    let w = db.post.with(post_type_id.eq(1)).group_by(owners).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let top = top_n(drain((&w).filt(|(_, r)| r == 1)), |&(u, ((p, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.1 .0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and(&cc).and(&ac).and(owner_user));
    rows(v.into_iter().map(|(p, (((a, c), n), u))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounties
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) WHERE U.CreationDate >= '2020-01-01' GROUP BY U.Id, U.DisplayName, U.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalBounties, RANK() OVER (ORDER BY PostCount DESC, TotalBounties DESC) AS UserRank FROM UserActivity),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalBounties FROM RankedUsers WHERE UserRank <= 10)
// SELECT TU.DisplayName, TU.PostCount, TU.TotalBounties, (SELECT COUNT(*) FROM Votes V WHERE V.UserId = TU.UserId AND V.VoteTypeId = 2) AS UpVotesReceived,
//        (SELECT COUNT(*) FROM Votes V WHERE V.UserId = TU.UserId AND V.VoteTypeId = 3) AS DownVotesReceived
// FROM TopUsers TU LEFT JOIN Badges B ON TU.UserId = B.UserId WHERE B.Class = 1 OR B.Class = 2 GROUP BY TU.UserId, TU.DisplayName, TU.PostCount, TU.TotalBounties
// ORDER BY TU.PostCount DESC, TU.TotalBounties DESC;
//
// COUNT(DISTINCT P.Id) comes from a fold over one row per post.
fn q2574(db: &'static So) -> String {
    let users = || db.user.with((&db.user.creation_date).ge(ts(2020, 1, 1, 0, 0, 0)));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let tb = users().group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&pc).select(Ident::<User>::new().and(&pc).and(&tb)).window(rank, |((_, n), b)| (Reverse(n), Reverse(b)), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, k)| k <= 10).map(|(((u, _), _), _)| u).collect();
    let gs = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).is_in([1, 2])));
    let keep = (&tu).group_by(Ident::<User>::new()).select(gs).fold(0i64, |n, _| n + 1);
    let uv = (&tu).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&keep).and(&pc).and(&tb).and(&uv)).into_iter().map(|(u, (((_, n), b), a))| row(vec![user_col(db, u, "name"), V::I(n), V::I(b), V::I(a[0]), V::I(a[1])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore,
//        COALESCE(pm.BadgeCount, 0) AS BadgeCount, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges WHERE Date >= CURRENT_DATE - INTERVAL '1 year' GROUP BY UserId) pm ON p.OwnerUserId = pm.UserId
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '30 days' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, pm.BadgeCount, p.OwnerUserId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, RankScore, BadgeCount, OwnerUserId FROM RankedPosts WHERE RankScore <= 10)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.BadgeCount, ut.DisplayName AS OwnerDisplayName, ut.Reputation AS OwnerReputation
// FROM TopPosts tp JOIN Users ut ON tp.OwnerUserId = ut.Id ORDER BY tp.RankScore;
fn q6265(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, creation_date, score, view_count, .. } = &db.post;
    let today = current_date();
    let w = whole(db.post.with(creation_date.ge(add_days(today, -30)).and(post_type_id.is_in([1, 2]))))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.with((&db.badge.date).ge(add_years(today, -1))).group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&cc).and(owner_user_id.select(&bc).opt()).and(owner_user));
    rows(v.into_iter().map(|(p, ((c, b), u))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(b.unwrap_or(0))]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id as PostId, p.Title, p.CreationDate, p.Score, COALESCE(NULLIF(p.OwnerDisplayName, ''), 'Community User') as OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) as RowNum FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.RowNum <= 5),
// PostStatistics AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpvoteCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownvoteCount FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.OwnerDisplayName)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.OwnerDisplayName, ps.CommentCount, ps.UpvoteCount, ps.DownvoteCount FROM PostStatistics ps ORDER BY ps.Score DESC, ps.CreationDate ASC;
fn q8419(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_display_name, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::S(owner_display_name.get(p).filter(|s| !s.is_empty()).unwrap_or("Community User")));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotes,
//        RANK() OVER (PARTITION BY p.Tags ORDER BY p.ViewCount DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT PostId, Title, Tags, ViewCount, UpVotes, DownVotes FROM RankedPosts WHERE TagRank <= 5),
// AggregateStats AS (SELECT Tags, COUNT(PostId) AS PostCount, SUM(ViewCount) AS TotalViews, SUM(UpVotes) AS TotalUpVotes, SUM(DownVotes) AS TotalDownVotes FROM FilteredPosts GROUP BY Tags)
// SELECT Tags, PostCount, TotalViews, TotalUpVotes, TotalDownVotes, ROUND(TotalUpVotes::DECIMAL / NULLIF(PostCount, 0), 2) AS AvgUpVotesPerPost,
//        ROUND(TotalDownVotes::DECIMAL / NULLIF(PostCount, 0), 2) AS AvgDownVotesPerPost
// FROM AggregateStats ORDER BY TotalViews DESC;
//
// No GROUP BY in RankedPosts: the windows run over the question x vote rows, so those rows are materialised and ranked.
fn q28680(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, view_count, .. } = &db.post;
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> = db.post.with(post_type_id.eq(1)).with(owner_user).select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    type J = (Id<Post>, Option<Id<Vote>>);
    let post_of = || Same::<J>::new().map(|(p, _): J| p);
    let ud = (&j).group_by(post_of()).select(Same::<J>::new().map(|(_, v): J| v).flat_map(|v: Option<Id<Vote>>| v).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let w = (&j).group_by(post_of().select(tags_str.opt())).select(Same::<J>::new().and(post_of().select(view_count.opt()))).window(rank, |(_, w)| (w.is_none(), Reverse(w)), asc);
    type R = ((J, Option<i64>), i64);
    let g = (&w)
        .filt(|(_, r)| r <= 5)
        .select(Same::<R>::new().map(|((x, _), _): R| x.0).select(view_count.opt().and(&ud)))
        .fold([0i64; 5], |a, (w, u)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + u[0], a[4] + u[1]]);
    let r2 = |s: i64, n: i64| V::F(((200 * s + n) / (2 * n)) as f64 / 100.0);
    rows(drain(&g).into_iter().map(|(t, a)| row(vec![ostr(t), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::I(a[4]), r2(a[3], a[0]), r2(a[4], a[0])])))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COUNT(P.AcceptedAnswerId) AS AcceptedAnswers, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViews FROM Posts P GROUP BY P.OwnerUserId),
// TopContributors AS (SELECT UR.UserId, UR.DisplayName, PS.TotalPosts, PS.AcceptedAnswers, PS.TotalScore, PS.AvgViews, RANK() OVER (ORDER BY PS.TotalScore DESC) AS ScoreRank
//     FROM UserReputation UR JOIN PostStatistics PS ON UR.UserId = PS.OwnerUserId WHERE UR.Reputation > 1000)
// SELECT TC.DisplayName, TC.TotalPosts, TC.AcceptedAnswers, TC.TotalScore, TC.AvgViews, CASE WHEN TC.ScoreRank <= 10 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributorType,
//        COALESCE(B.BadgeCount, 0) AS BadgeCount
// FROM TopContributors TC LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON TC.UserId = B.UserId WHERE TC.AvgViews > 50 ORDER BY TC.TotalScore DESC, BadgeCount DESC;
fn q1654(db: &'static So) -> String {
    let Post { owner_user, accepted_answer_id, score, view_count, .. } = &db.post;
    let ps = db
        .post
        .group_by(owner_user)
        .select(accepted_answer_id.opt().and(score).and(view_count.opt()))
        .fold([0i64; 5], |a, ((x, s), w)| [a[0] + 1, a[1] + x.is_some() as i64, a[2] + s, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let w = whole(db.user.with((&db.user.reputation).gt(1000)).with(&ps)).select(Ident::<User>::new().and(&ps).and((&bc).opt())).window(rank, |((_, a), _)| Reverse(a[2]), asc);
    let v = drain((&w).filt(|(((_, a), _), _)| a[3] > 0 && a[4] > 50 * a[3]));
    rows(v.into_iter().map(|(_, (((u, a), b), r))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), V::S(if r <= 10 { "Top Contributor" } else { "Contributor" }), V::I(b.unwrap_or(0))])
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.CreationDate, P.ViewCount, P.AnswerCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND P.Score > 0),
// MostVotedPosts AS (SELECT RP.PostId, RP.Title, RP.OwnerDisplayName, RP.Score, COUNT(V.Id) AS VoteCount FROM RankedPosts RP LEFT JOIN Votes V ON RP.PostId = V.PostId WHERE RP.Rank <= 10
//     GROUP BY RP.PostId, RP.Title, RP.OwnerDisplayName, RP.Score),
// PostWithBadges AS (SELECT MVP.PostId, MVP.Title, MVP.OwnerDisplayName, MVP.Score, COALESCE(B.Name, 'No Badge') AS BadgeName
//     FROM MostVotedPosts MVP LEFT JOIN Badges B ON MVP.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = B.UserId))
// SELECT PWB.PostId, PWB.Title, PWB.OwnerDisplayName, PWB.Score, PWB.BadgeName, (SELECT COUNT(*) FROM Comments C WHERE C.PostId = PWB.PostId) AS CommentCount
// FROM PostWithBadges PWB ORDER BY PWB.Score DESC, PWB.PostId ASC;
fn q8538(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let by_name: HashIdx<Str, Id<Badge>> = (&db.badge.user).select(&db.user.display_name).inv().collect();
    let v = drain((&cc).and(owner_user.select(&db.user.display_name).select((&by_name).select(&db.badge.name)).opt()));
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score"]);
        f.extend([V::S(b.unwrap_or("No Badge")), V::I(c)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges GROUP BY UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// RankedUsers AS (SELECT u.Id, u.DisplayName, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, ps.TotalPosts,
//        ps.Questions, ps.Answers, ps.TotalViews, RANK() OVER (ORDER BY COALESCE(ub.GoldBadges, 0) DESC, ps.TotalViews DESC) AS UserRank
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT ru.DisplayName, ru.UserRank, ru.GoldBadges, ru.SilverBadges, ru.BronzeBadges, ru.TotalPosts, ru.Questions, ru.Answers, ru.TotalViews FROM RankedUsers ru WHERE ru.UserRank <= 10 ORDER BY ru.UserRank;
fn q3179(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let ups = user_posts(db);
    let w = whole(&ub).select(Ident::<User>::new().and(&ub).and(&ups)).window(rank, |((_, b), a)| (Reverse(b[0]), a[5] == 0, Reverse(a[6])), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, b), a), r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(r)];
        f.extend(b.map(V::I));
        f.extend(if a[1] == 0 { [V::Null, V::Null, V::Null, V::Null] } else { [V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5])] });
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COALESCE(b.Class, 0) AS BadgeClass
//     FROM Users u LEFT JOIN (SELECT UserId, MAX(Class) AS Class FROM Badges GROUP BY UserId) b ON u.Id = b.UserId)
// SELECT up.RecentPostRank, up.Title, up.CommentCount, ur.DisplayName, ur.Reputation, ur.BadgeClass FROM RecentPosts up JOIN UserReputation ur ON up.OwnerUserId = ur.UserId
// WHERE ur.Reputation > 1000 AND (ur.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '2 years' OR ur.BadgeClass = 1)
//   AND EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = up.PostId AND v.VoteTypeId IN (2, 3) GROUP BY v.PostId HAVING COUNT(v.VoteTypeId) > 5)
// ORDER BY up.CommentCount DESC, ur.Reputation ASC LIMIT 10;
fn q1413(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { owner_user, creation_date, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(t0, -1)));
    let w = recent().group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let rank = by_first(&rk);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold(0i64, |m, c| m.max(c));
    let ur = Ident::<User>::new()
        .with((&db.user.reputation).gt(1000))
        .and((&db.user.creation_date).and((&mc).opt()))
        .filt(move |(_, (d, c))| d < add_years(t0, -2) || c.unwrap_or(0) == 1)
        .map(|(u, (_, c))| (u, c.unwrap_or(0)));
    let voted = db.vote.with((&db.vote.vote_type_id).is_in([2, 3])).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(recent().with((&voted).filt(|n| n > 5)).select((&cc).and(&rank).and(owner_user.select(ur))));
    let v = top_n(v, |&(p, ((c, _), (u, _)))| (Reverse(c), db.user.reputation.get(u).unwrap(), p), 10);
    rows(v.into_iter().map(|(p, ((c, r), (u, b)))| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["title"]));
        f.push(V::I(c));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH PostAnalytics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN bh.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory bh ON p.Id = bh.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, UpVotes, DownVotes, CloseCount, RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostAnalytics)
// SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, UpVotes, DownVotes, CloseCount FROM TopPosts WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only base columns, so the top questions are picked first and the comment x vote x history product is driven for those alone.
fn q5582(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let w = whole(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 4], |a, ((c, t), h)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (h == Some(10)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH TagStats AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount,
//        SUM(p.ViewCount) AS TotalViews, SUM(v.BountyAmount) AS TotalBounties
//     FROM Tags AS t LEFT JOIN Posts AS p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Comments AS c ON c.PostId = p.Id LEFT JOIN Votes AS v ON v.PostId = p.Id AND v.VoteTypeId = 8 GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, CommentCount, AnswerCount, TotalViews, TotalBounties, RANK() OVER (ORDER BY PostCount DESC) AS PostRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM TagStats),
// OverallTopTags AS (SELECT TagName, PostCount, CommentCount, AnswerCount, TotalViews, TotalBounties, (PostRank + ViewRank) AS OverallRank FROM TopTags)
// SELECT TagName, PostCount, CommentCount, AnswerCount, TotalViews, TotalBounties, OverallRank, CASE WHEN OverallRank <= 10 THEN 'Top Tag' WHEN OverallRank <= 20 THEN 'Mid Tag' ELSE 'Low Tag' END AS TagCategory
// FROM OverallTopTags WHERE PostCount > 0 ORDER BY OverallRank;
//
// The COUNT(DISTINCT)s come from folds over one row per matching post and one row per comment.
fn q25869(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let posts = || (&by_tag).map(|(p, _)| p);
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let s = db
        .tag
        .group_by(&db.tag.tag_name)
        .select(posts().select((&db.post.view_count).opt().and(comments_of(db).opt()).and(bounty.opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((w, _), b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
            }
            None => a,
        });
    let dc = db.tag.group_by(&db.tag.tag_name).select(posts().select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let da = db.tag.group_by(&db.tag.tag_name).select(posts().select(Ident::<Post>::new().with((&db.post.post_type_id).eq(2))).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    type R = (((Str, [i64; 5]), i64), i64);
    let w = whole(&s).select(Same::<Str>::new().and(&s).and(&dc).and(&da)).window(rank, |(((_, a), _), _)| Reverse(a[0]), asc);
    let w = (&w).window(rank, |((((_, a), _), _), _): (R, i64)| (a[1] == 0, Reverse(a[2])), asc);
    let v = drain((&w).filt(|(((((_, a), _), _), _), _): ((R, i64), i64)| a[0] > 0));
    rows(v.into_iter().map(|(_, (((((t, a), c), n), pr), vr))| {
        let o = pr + vr;
        row(vec![V::S(t), V::I(a[0]), V::I(c), V::I(n), nullable(a[2], a[1]), nullable(a[4], a[3]), V::I(o), V::S(if o <= 10 { "Top Tag" } else if o <= 20 { "Mid Tag" } else { "Low Tag" })])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.CreationDate, rp.CommentCount, rp.AnswerCount, rp.UpVoteCount, rp.DownVoteCount, rp.Rank FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT fp.PostId, fp.Title, fp.Body, fp.Tags, fp.CreationDate, fp.CommentCount, fp.AnswerCount, fp.UpVoteCount, fp.DownVoteCount, (fp.UpVoteCount - fp.DownVoteCount) AS Score
// FROM FilteredPosts fp WHERE fp.CommentCount > 5 ORDER BY Score DESC, fp.CreationDate DESC LIMIT 10;
//
// Rank reads only base columns, so each owner's five newest questions are picked first; the COUNT(DISTINCT)s are folded one child at a time and the vote sums over the product.
fn q28283(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).filt(|n| n > 5).and(&ac).and(&s));
    let v = top_n(v, |&(p, (_, a))| (Reverse(a[0] - a[1]), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, ((c, n), a))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount, MAX(Date) AS LastBadgeDate FROM Badges GROUP BY UserId),
// PostStats AS (SELECT OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
//     FROM Posts GROUP BY OwnerUserId),
// CommentStats AS (SELECT UserId, COUNT(*) AS TotalComments FROM Comments GROUP BY UserId),
// TopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.TotalQuestions, 0) AS TotalQuestions,
//        COALESCE(PS.TotalAnswers, 0) AS TotalAnswers, COALESCE(CS.TotalComments, 0) AS TotalComments
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId LEFT JOIN CommentStats CS ON U.Id = CS.UserId)
// SELECT DisplayName, Reputation, BadgeCount, TotalPosts, TotalQuestions, TotalAnswers, TotalComments, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM TopUsers WHERE Reputation > 1000 ORDER BY Rank LIMIT 10;
fn q1913(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let uc = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ups = user_posts(db);
    let w = whole(db.user.with((&db.user.reputation).gt(1000))).select(Ident::<User>::new().and(&ub).and(&ups).and(&uc).and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let v = top_n(drain(&w), |&(_, (((((u, _), _), _), _), r))| (r, u), 10);
    rows(v.into_iter().map(|(_, (((((u, b), a), c), _), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(c), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.Id END) AS UpVoteCount, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 3 THEN v.Id END) AS DownVoteCount,
//        DENSE_RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.OwnerDisplayName, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount,
//        CASE WHEN tp.Score > 100 THEN 'Highly Rated' WHEN tp.Score BETWEEN 50 AND 100 THEN 'Moderately Rated' ELSE 'Low Rated' END AS RatingCategory
// FROM TopPosts tp ORDER BY tp.Score DESC;
//
// Rank reads only base columns, so the top questions are picked first; each COUNT(DISTINCT) is folded over its own child.
fn q6636(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let w = whole(db.post.with(post_type_id.eq(1))).select(Ident::<Post>::new().and(score).and(creation_date)).window(dense_rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "views", "score", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(if s > 100 { "Highly Rated" } else if s >= 50 { "Moderately Rated" } else { "Low Rated" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, Upvotes, Downvotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.AnswerCount, TU.QuestionCount, TU.Upvotes, TU.Downvotes,
//        CASE WHEN TU.Reputation >= 10000 THEN 'Gold' WHEN TU.Reputation >= 1000 THEN 'Silver' ELSE 'Bronze' END AS Badge, PH.CreationDate AS LastEdited
// FROM TopUsers TU LEFT JOIN PostHistory PH ON PH.UserId = TU.UserId WHERE TU.Rank <= 10 ORDER BY TU.Rank, TU.QuestionCount DESC, TU.Upvotes DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the post x vote product is driven for them alone.
fn q9592(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, vt)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (vt == Some(2)) as i64, a[3] + (vt == Some(3)) as i64],
            None => a,
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let by_user: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let v = drain((&pc).and(&s).and((&by_user).opt()));
    rows(v.into_iter().map(|(u, ((n, a), h))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::S(if r >= 10000 { "Gold" } else if r >= 1000 { "Silver" } else { "Bronze" }));
        f.push(h.map_or(V::Null, |h| V::T(db.post_history.creation_date.get(h).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.Tags, COUNT(DISTINCT ans.Id) AS AnswerCount, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS TagRank
//     FROM Posts p LEFT JOIN Posts ans ON p.Id = ans.ParentId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Tags),
// TagMetrics AS (SELECT Tags, AVG(AnswerCount) AS AvgAnswers, AVG(CommentCount) AS AvgComments, SUM(UpVotes) AS TotalUpVotes, SUM(DownVotes) AS TotalDownVotes FROM RankedPosts GROUP BY Tags)
// SELECT t.TagName AS Tag, tm.AvgAnswers, tm.AvgComments, tm.TotalUpVotes, tm.TotalDownVotes,
//        CASE WHEN tm.TotalUpVotes > tm.TotalDownVotes THEN 'Positive' WHEN tm.TotalUpVotes < tm.TotalDownVotes THEN 'Negative' ELSE 'Neutral' END AS Sentiment
// FROM Tags t JOIN TagMetrics tm ON t.TagName = ANY(string_to_array(tm.Tags, ',')) ORDER BY tm.TotalUpVotes DESC, tm.AvgAnswers DESC;
fn q29281(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let ud = qs()
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ac = qs().group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tm = qs()
        .group_by(tags_str.opt())
        .select((&ud).and(&ac).and(&cc))
        .fold([0i64; 5], |s, ((u, a), c)| [s[0] + 1, s[1] + a, s[2] + c, s[3] + u[0], s[4] + u[1]]);
    let tv = rel(drain(&tm));
    type T = (Option<Str>, [i64; 5]);
    let by_elem: HashIdx<Str, T> = (&tv).flat_map(|(t, _): T| t.into_iter().flat_map(|t: Str| t.split(','))).inv().select(&tv).collect();
    let v = drain(db.tag.select((&db.tag.tag_name).and((&db.tag.tag_name).select(&by_elem))));
    rows(v.into_iter().map(|(_, (n, (_, s)))| {
        row(vec![V::S(n), avg(s[1], s[0]), avg(s[2], s[0]), V::I(s[3]), V::I(s[4]), V::S(if s[3] > s[4] { "Positive" } else if s[3] < s[4] { "Negative" } else { "Neutral" })])
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(P.Score) AS AvgScore FROM Posts P GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT U.Id, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.Questions, 0) AS Questions, COALESCE(PS.Answers, 0) AS Answers,
//        COALESCE(PS.AvgScore, 0) AS AvgScore, ROW_NUMBER() OVER (ORDER BY COALESCE(PS.TotalPosts, 0) DESC, COALESCE(UB.BadgeCount, 0) DESC) AS Rank
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId)
// SELECT T.DisplayName, T.TotalPosts, T.Questions, T.Answers, T.AvgScore, T.BadgeCount, T.Rank FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.Rank;
fn q1713(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ub).and(&ups)).window(row_number, |((u, b), a)| (Reverse(a[1]), Reverse(b), u), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, (((u, b), a), i))| {
        row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), if a[1] == 0 { V::F(0.0) } else { avg(a[4], a[1]) }, V::I(b), V::I(i)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.LastActivityDate, COUNT(c.Id) AS CommentCount, COALESCE(AVG(v.vote_score), 0) AS AverageVoteScore
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 WHEN vt.Name = 'DownMod' THEN -1 ELSE 0 END) AS vote_score FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY PostId) v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, u.DisplayName, p.CreationDate, p.LastActivityDate),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, LastActivityDate, CommentCount, AverageVoteScore, ROW_NUMBER() OVER (ORDER BY AverageVoteScore DESC, CommentCount DESC) AS Rank FROM RankedPosts)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.LastActivityDate, tp.CommentCount, tp.AverageVoteScore FROM TopPosts tp WHERE tp.Rank <= 10
// ORDER BY tp.AverageVoteScore DESC, tp.CommentCount DESC;
fn q7522(db: &'static So) -> String {
    let vs = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold(0i64, |s, n| s + if n == "UpMod" { 1 } else if n == "DownMod" { -1 } else { 0 });
    let s = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&vs).opt()))
        .fold([0i64; 3], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0)]);
    let key = |a: [i64; 3]| if a[1] == 0 { 0.0 } else { a[2] as f64 / a[1] as f64 };
    let v = top_n(drain(&s), |&(p, a)| (Reverse(fkey(key(a))), Reverse(a[0]), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "activity"]);
        f.extend([V::I(a[0]), V::F(key(a))]);
        row(f)
    }))
}

// WITH UserScore AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, (U.UpVotes - U.DownVotes) AS NetVotes, RANK() OVER (ORDER BY (U.UpVotes - U.DownVotes) DESC) AS RankPosition FROM Users U),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Posts P GROUP BY P.OwnerUserId),
// UserPerformance AS (SELECT U.DisplayName, U.Reputation, U.NetVotes, PS.PostCount, PS.QuestionCount, PS.AnswerCount, COALESCE(PS.PostCount, 0) * 1.0 / NULLIF((SELECT COUNT(*) FROM Posts), 0) AS PostContribution
//     FROM UserScore U LEFT JOIN PostStats PS ON U.UserId = PS.OwnerUserId)
// SELECT UP.DisplayName, UP.Reputation, UP.NetVotes, UP.PostCount, UP.QuestionCount, UP.AnswerCount, UP.PostContribution,
//        CASE WHEN UP.Reputation > 1000 THEN 'High Contributor' WHEN UP.Reputation BETWEEN 500 AND 1000 THEN 'Moderate Contributor' ELSE 'New Contributor' END AS ContributorStatus
// FROM UserPerformance UP WHERE UP.PostCount > 5 ORDER BY UP.NetVotes DESC, UP.Reputation DESC LIMIT 10;
fn q4666(db: &'static So) -> String {
    let total = count(db.post.select(Ident::<Post>::new()));
    let User { up_votes, down_votes, reputation, .. } = &db.user;
    let ps = db.post.group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let v = drain((&ps).filt(|a| a[0] > 5));
    let v = top_n(v, |&(u, _)| (Reverse(up_votes.get(u).unwrap() - down_votes.get(u).unwrap()), Reverse(reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let r = reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(up_votes.get(u).unwrap() - down_votes.get(u).unwrap()));
        f.extend(a.map(V::I));
        f.push(V::F(a[0] as f64 / total as f64));
        f.push(V::S(if r > 1000 { "High Contributor" } else if r >= 500 { "Moderate Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount, p.CreationDate, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, u.DisplayName, p.Score, p.ViewCount, p.CreationDate, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, Score, ViewCount, CreationDate, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT t.PostId, t.Title, t.OwnerDisplayName, t.Score, t.ViewCount, t.CreationDate, t.CommentCount, t.UpVoteCount, t.DownVoteCount,
//        CASE WHEN t.UpVoteCount > t.DownVoteCount THEN 'Positive' WHEN t.UpVoteCount < t.DownVoteCount THEN 'Negative' ELSE 'Neutral' END AS Sentiment
// FROM TopPosts t ORDER BY t.Score DESC, t.CreationDate DESC;
//
// Rank reads only base columns, so the top ten posts per type are picked first and the comment x vote product is driven for those alone.
fn q6893(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views", "created"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(V.BountyAmount) AS TotalBounty, AVG(COALESCE(P.Score, 0.0)) AS AvgScore,
//        SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS UpVotes, SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalBounty, AvgScore, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserActivity)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.TotalBounty, TU.AvgScore, TU.UpVotes, TU.DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY TU.ReputationRank ORDER BY TU.AvgScore DESC) AS ScoreRank
// FROM TopUsers TU WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC, TU.UpVotes DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the post x vote product is driven for them alone.
fn q7458(db: &'static So) -> String {
    type T = (Id<User>, i64);
    let rw = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<T> = (&rw).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, k)).collect();
    let ids: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let s = (&ids)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score).and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt())).opt())
        .fold([0i64; 9], |a, p| match p {
            Some(((t, s), v)) => {
                let (vt, b) = v.map_or((None, None), |(t, b)| (Some(t), b));
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0), a[5] + s, a[6] + (vt == Some(2)) as i64, a[7] + (vt == Some(3)) as i64, a[8] + 1]
            }
            None => {
                let mut a = a;
                a[8] += 1;
                a
            }
        });
    let w = (&tu)
        .group_by(Same::<T>::new().map(|(_, r): T| r))
        .select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(&s)))
        .window(row_number, |((u, _), a)| (Reverse(fkey(a[5] as f64 / a[8] as f64)), u), asc);
    rows(drain(&w).into_iter().map(|(_, (((u, _), a), sr))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), avg(a[5], a[8]), V::I(a[6]), V::I(a[7]), V::I(sr)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// PopularTags AS (SELECT T.TagName, COUNT(P.Id) AS PostCount FROM Tags T INNER JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName HAVING COUNT(P.Id) > 5),
// RankedUsers AS (SELECT UA.UserId, UA.DisplayName, UA.Reputation, UA.PostCount, UA.UpVotes, UA.DownVotes, RANK() OVER (ORDER BY UA.Reputation DESC) AS UserRank FROM UserActivity UA)
// SELECT Ru.UserId, Ru.DisplayName, Ru.Reputation, Ru.PostCount, Ru.UpVotes - Ru.DownVotes AS NetVotes, Pt.TagName, CASE WHEN Ru.PostCount > 10 THEN 'Active Contributor' ELSE 'Newcomer' END AS ContributorStatus
// FROM RankedUsers Ru LEFT JOIN PopularTags Pt ON Ru.PostCount = Pt.PostCount WHERE Ru.UserRank <= 100 ORDER BY Ru.Reputation DESC, Pt.PostCount DESC;
//
// UserRank reads only Reputation, so the top users are picked first. COUNT(DISTINCT P.Id) comes from a fold over one row per post.
fn q3355(db: &'static So) -> String {
    let w = whole(db.user.with((&db.user.reputation).gt(1000))).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let ids: MatSet<Id<User>> = (&w).filt(|(_, k)| k <= 100).map(|((u, _), _)| u).collect();
    let s = (&ids).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold(0i64, |n, t| {
        let t = t.flatten();
        n + (t == Some(2)) as i64 - (t == Some(3)) as i64
    });
    let pc = (&ids).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let lt = tag_mentions(db);
    let tc = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t).select(&db.tag.tag_name)).select(Same::<(Id<Post>, Id<Tag>)>::new()).fold(0i64, |n, _| n + 1);
    let by_count: HashIdx<i64, Str> = (&tc).filt(|n| n > 5).inv().collect();
    let v = drain((&pc).and(&s).and((&pc).select(&by_count).opt()));
    rows(v.into_iter().map(|(u, ((n, net), t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(net), ostr(t), V::S(if n > 10 { "Active Contributor" } else { "Newcomer" })]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("2999", q2999),
    ("7642", q7642),
    ("8706", q8706),
    ("9542", q9542),
    ("6566", q6566),
    ("6624", q6624),
    ("28651", q28651),
    ("5251", q5251),
    ("25029", q25029),
    ("7750", q7750),
    ("6087", q6087),
    ("6389", q6389),
    ("8543", q8543),
    ("6204", q6204),
    ("6407", q6407),
    ("6852", q6852),
    ("7097", q7097),
    ("1602", q1602),
    ("8602", q8602),
    ("463", q463),
    ("26277", q26277),
    ("3603", q3603),
    ("5457", q5457),
    ("9342", q9342),
    ("9641", q9641),
    ("2460", q2460),
    ("6643", q6643),
    ("115", q115),
    ("1376", q1376),
    ("7185", q7185),
    ("7758", q7758),
    ("7808", q7808),
    ("9912", q9912),
    ("25458", q25458),
    ("969", q969),
    ("8449", q8449),
    ("27685", q27685),
    ("7592", q7592),
    ("6262", q6262),
    ("6728", q6728),
    ("3624", q3624),
    ("7769", q7769),
    ("2452", q2452),
    ("6760", q6760),
    ("1692", q1692),
    ("3704", q3704),
    ("25117", q25117),
    ("5454", q5454),
    ("4860", q4860),
    ("6238", q6238),
    ("1581", q1581),
    ("3519", q3519),
    ("3004", q3004),
    ("5532", q5532),
    ("2207", q2207),
    ("1584", q1584),
    ("1766", q1766),
    ("5956", q5956),
    ("7723", q7723),
    ("4697", q4697),
    ("3489", q3489),
    ("6494", q6494),
    ("5696", q5696),
    ("5162", q5162),
    ("7207", q7207),
    ("22746", q22746),
    ("6104", q6104),
    ("9844", q9844),
    ("7614", q7614),
    ("8329", q8329),
    ("5579", q5579),
    ("6989", q6989),
    ("29347", q29347),
    ("7756", q7756),
    ("8233", q8233),
    ("2574", q2574),
    ("6265", q6265),
    ("8419", q8419),
    ("28680", q28680),
    ("1654", q1654),
    ("8538", q8538),
    ("3179", q3179),
    ("1413", q1413),
    ("5582", q5582),
    ("25869", q25869),
    ("28283", q28283),
    ("1913", q1913),
    ("6636", q6636),
    ("9592", q9592),
    ("29281", q29281),
    ("1713", q1713),
    ("7522", q7522),
    ("4666", q4666),
    ("6893", q6893),
    ("7458", q7458),
    ("3355", q3355),
];
