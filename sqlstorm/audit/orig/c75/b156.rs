use harness::prelude::*;
use std::cmp::Reverse;

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikis,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, MAX(b.Date) AS MostRecentBadgeDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// ActiveUsers AS (SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.Questions, ua.Answers, ua.TagWikis, ua.TotalUpVotes, ua.TotalDownVotes, ua.MostRecentBadgeDate,
//        RANK() OVER (ORDER BY ua.TotalPosts DESC, ua.TotalUpVotes DESC) AS PostRank FROM UserActivity ua WHERE ua.TotalPosts > 0)
// SELECT au.DisplayName, au.TotalPosts, au.Questions, au.Answers, au.TagWikis, au.TotalUpVotes, au.TotalDownVotes, au.MostRecentBadgeDate,
//        CASE WHEN au.PostRank <= 10 THEN 'Top Contributor' WHEN au.TotalPosts >= 50 THEN 'Active User' ELSE 'Newcomer' END AS UserStatus
// FROM ActiveUsers au WHERE au.PostRank <= 50 ORDER BY au.PostRank;
fn q7043(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.date).opt()))
        .fold([0, 0, 0, 0, 0, i64::MIN], |a, (p, d)| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 4 | 5) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5].max(d.unwrap_or(i64::MIN))]
        });
    let dp = user_distinct_posts(db);
    let v = ranked(drain((&dp).filt(|n| n > 0).and(&ua)), |&(_, (n, a))| (Reverse(n), Reverse(a[3])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 50).map(|((u, (n, a)), r)| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a[..5].iter().map(|&x| V::I(x)));
        f.push(tmax(a[5]));
        f.push(V::S(if r <= 10 { "Top Contributor" } else if n >= 50 { "Active User" } else { "Newcomer" }));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, UpVoteCount, DownVoteCount, RANK() OVER (ORDER BY PostCount DESC) AS PostRank,
//        RANK() OVER (ORDER BY UpVoteCount DESC) AS UpVoteRank FROM UserPostStats),
// CombinedRanked AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, UpVoteCount, DownVoteCount, PostRank, UpVoteRank, (PostRank + UpVoteRank) AS CombinedRank FROM RankedUsers)
// SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, UpVoteCount, DownVoteCount, CombinedRank FROM CombinedRanked WHERE CombinedRank <= 10 ORDER BY CombinedRank;
fn q9855(db: &'static So) -> String {
    let s = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[3]), false);
    let mut v: Vec<_> = v.into_iter().map(|(((u, a), p), w)| (u, a, p + w)).filter(|x| x.2 <= 10).collect();
    v.sort_by_key(|x| x.2);
    rows(v.into_iter().map(|(u, a, c)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankScore, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Users U ON p.OwnerUserId = U.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > '2023-01-01'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, U.DisplayName),
// TopScoringPosts AS (SELECT Id, Title, CreationDate, Score, OwnerUserId, OwnerDisplayName FROM RankedPosts WHERE RankScore = 1),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT ps.Title, ps.CreationDate, ps.Score, ps.OwnerDisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount,
//        CONCAT('Post ID: ', CAST(ps.Id AS VARCHAR), ' | Score: ', CAST(ps.Score AS VARCHAR)) AS PostSummary,
//        CASE WHEN ps.Score > 100 THEN 'High Score' WHEN ps.Score BETWEEN 50 AND 100 THEN 'Moderate Score' ELSE 'Low Score' END AS ScoreCategory
// FROM TopScoringPosts ps LEFT JOIN UserBadges ub ON ps.OwnerUserId = ub.UserId ORDER BY ps.Score DESC LIMIT 10;
//
// CommentCount is never read, and the comment join cannot remove a post from its group, so it is not computed.
fn q3932(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(ts(2023, 1, 1, 0, 0, 0))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&tp).select(owner_user.select(&ub).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, b)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score", "owner"]);
        f.push(V::I(b.unwrap_or(0)));
        f.push(V::Owned(format!("Post ID: {} | Score: {}", origid.get(p).unwrap(), s)));
        f.push(V::S(if s > 100 { "High Score" } else if s >= 50 { "Moderate Score" } else { "Low Score" }));
        row(f)
    }))
}

// Rewritten (rewrites/3862.sql): the ROW_NUMBER order tie-broken on P.Id, the final order on U.UserId.
// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC, P.Id) AS PostRank FROM Posts P WHERE P.PostTypeId = 1),
// UserStats AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViewCount,
//        SUM(COALESCE(P.Score, 0)) AS TotalScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 GROUP BY U.Id, U.Reputation, U.DisplayName),
// AnswerDetails AS (SELECT A.ParentId AS QuestionId, COUNT(A.Id) AS AnswerCount, SUM(COALESCE(A.Score, 0)) AS TotalAnswerScore FROM Posts A WHERE A.PostTypeId = 2 GROUP BY A.ParentId)
// SELECT U.DisplayName, U.Reputation, U.QuestionCount, U.TotalViewCount, U.TotalScore, COALESCE(AD.AnswerCount, 0) AS AnswerCount,
//        COALESCE(AD.TotalAnswerScore, 0) AS TotalAnswerScore, RP.Title AS TopPostTitle, RP.CreationDate AS TopPostDate, RP.Score AS TopPostScore
// FROM UserStats U LEFT JOIN AnswerDetails AD ON U.UserId = AD.QuestionId LEFT JOIN RankedPosts RP ON U.UserId = RP.PostId
// WHERE RP.PostRank = 1 ORDER BY U.Reputation DESC, U.QuestionCount DESC, U.UserId FETCH FIRST 50 ROWS ONLY;
//
// Both joins compare a user id with a post id, so they go through the raw ids.
fn q3862(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, parent_id, origid, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), origid.get(p).unwrap()), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_id: HashIdx<i64, Id<Post>> = (&tp).select(origid).inv().collect();
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(score.and(view_count.opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((s, w)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s],
            None => a,
        });
    let ad = db.post.with(post_type_id.eq(2)).group_by(parent_id).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let uo = &db.user.origid;
    let v = drain((&us).and(uo.select(&by_id)).and(uo.select(&ad).opt()));
    let v = top_n(v, |&(u, ((a, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), uo.get(u).unwrap()), 50);
    rows(v.into_iter().map(|(u, ((a, p), d))| {
        let d = d.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(d[0]), V::I(d[1])]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.VoteCount,
//        ROW_NUMBER() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS Rank FROM RankedPosts rp WHERE rp.rn = 1),
// TopBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, tb.DisplayName AS User, tb.BadgeCount
// FROM TopPosts tp JOIN Posts p ON tp.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id JOIN TopBadges tb ON u.Id = tb.UserId
// WHERE tp.Rank <= 10 ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// rn partitions by the post itself, so it is always 1. Rank reads only base columns, so the ten posts are picked first
// and the comment x vote product is driven for those alone; VoteCount is never read.
fn q6595(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(Ident::<Post>::new()));
    let v = top_n(v, |&(p, _)| key(p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let tb = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let mut v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&tb))));
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, (c, (u, b)))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(c), user_col(db, u, "name"), V::I(b)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(v.Upvotes, 0) AS UpVotes, COALESCE(v.Downvotes, 0) AS DownVotes,
//        COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(a.AnswerCount, 0) AS AnswerCount, COALESCE(ph.EditCount, 0) AS EditCount
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS Upvotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT ParentId AS PostId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS EditCount FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6) GROUP BY PostId) ph ON p.Id = ph.PostId WHERE p.PostTypeId = 1),
// SortedPostStats AS (SELECT PostId, Title, CreationDate, UpVotes, DownVotes, CommentCount, AnswerCount, EditCount, ROW_NUMBER() OVER (ORDER BY UpVotes DESC, CreationDate ASC) AS Rank FROM PostStats)
// SELECT PostId, Title, CreationDate, UpVotes, DownVotes, CommentCount, AnswerCount, EditCount, Rank FROM SortedPostStats WHERE Rank <= 100;
fn q11459(db: &'static So) -> String {
    let qs = || db.post.with((&db.post.post_type_id).eq(1));
    let vs = qs().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + (t == 3) as i64]);
    let cs = qs().group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let an = qs().group_by(Ident::<Post>::new()).select(answers_of(db)).fold(0i64, |n, _| n + 1);
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])));
    let ed = qs().group_by(Ident::<Post>::new()).select(edits).fold(0i64, |n, _| n + 1);
    let v = drain(qs().select((&vs).opt().and((&cs).opt()).and((&an).opt()).and((&ed).opt())));
    let v = top_n(v, |&(p, (((v, _), _), _))| (Reverse(v.map_or(0, |a| a[0])), db.post.creation_date.get(p).unwrap(), p), 100);
    rows(v.into_iter().enumerate().map(|(i, (p, (((v, c), a), e)))| {
        let v = v.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(v[0]), V::I(v[1]), V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0)), V::I(e.unwrap_or(0)), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// TopUsers AS (SELECT Id, DisplayName, Reputation FROM UserReputation WHERE ReputationRank <= 10),
// PostSummary AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(COALESCE(p.Score, 0)) AS TotalScore, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions,
//        COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers, SUM(p.ViewCount) AS TotalViews, MAX(p.CreationDate) AS LatestPostDate FROM Posts p GROUP BY p.OwnerUserId),
// UserPostDetails AS (SELECT u.Id AS UserId, u.DisplayName, ps.TotalPosts, ps.TotalScore, ps.Questions, ps.Answers, ps.TotalViews, ps.LatestPostDate, COALESCE(b.Count, 0) AS BadgeCount
//     FROM TopUsers u LEFT JOIN PostSummary ps ON u.Id = ps.OwnerUserId LEFT JOIN (SELECT UserId, COUNT(*) AS Count FROM Badges GROUP BY UserId) b ON u.Id = b.UserId)
// SELECT u.DisplayName, u.TotalPosts, u.TotalScore, u.Questions, u.Answers, u.TotalViews, u.LatestPostDate,
//        CASE WHEN u.BadgeCount >= 5 THEN 'Active Contributor' ELSE 'New User' END AS UserStatus
// FROM UserPostDetails u WHERE u.TotalPosts > 0 ORDER BY u.TotalScore DESC;
fn q306(db: &'static So) -> String {
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { owner_user, score, post_type_id, view_count, creation_date, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(post_type_id).and(view_count.opt()).and(creation_date)).fold([0, 0, 0, 0, 0, 0, i64::MIN], |a, (((s, t), w), d)| {
        [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6].max(d)]
    });
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&tu).select((&ps).filt(|a| a[0] > 0).and((&bc).opt())));
    v.sort_by_key(|&(_, (a, _))| Reverse(a[1]));
    rows(v.into_iter().map(|(u, (a, b))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4]), V::T(a[6]), V::S(if b.unwrap_or(0) >= 5 { "Active Contributor" } else { "New User" })])
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.PostTypeId, P.CreationDate, P.Score, COUNT(C.Id) AS CommentsCount, AVG(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
//        AVG(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.PostTypeId, P.CreationDate, P.Score),
// RankedTopPosts AS (SELECT TP.*, RANK() OVER (ORDER BY TP.Score DESC, TP.CommentsCount DESC) AS PostRank FROM TopPosts TP WHERE TP.Score > 0)
// SELECT U.DisplayName, U.Reputation, RTP.Title, RTP.Score, RTP.CommentsCount, RTP.PostRank, CASE WHEN RTP.PostRank <= 10 THEN 'Top 10 Posts' ELSE 'Other Posts' END AS PostCategory
// FROM UserReputation U JOIN Posts P ON U.UserId = P.OwnerUserId JOIN RankedTopPosts RTP ON P.Id = RTP.PostId
// WHERE U.Reputation >= (SELECT AVG(Reputation) FROM Users) AND RTP.PostRank <= 20 ORDER BY U.Reputation DESC, RTP.Score DESC LIMIT 50;
//
// The vote averages are never read, so only COUNT(C.Id) is folded over the comment x vote product.
fn q948(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let tp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let v = ranked(drain(&tp), |&(p, c)| (Reverse(score.get(p).unwrap()), Reverse(c)), false);
    let rt = rel(v.into_iter().filter(|x| x.1 <= 20).map(|((p, c), r)| (p, c, r)).collect());
    let rtp: HashIdx<Id<Post>, (Id<Post>, i64, i64)> = (&rt).map(|(p, _, _)| p).inv().select(&rt).collect();
    let (s, n) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let avg_rep = s as f64 / n as f64;
    let rich = Ident::<User>::new().with((&db.user.reputation).filt(move |r| r as f64 >= avg_rep));
    let v = drain((&rtp).select(Same::<(Id<Post>, i64, i64)>::new().and(Same::<(Id<Post>, i64, i64)>::new().map(|(p, _, _)| p).select(owner_user.select(rich)))));
    let v = top_n(v, |&(_, ((p, _, _), u))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(_, ((p, c, r), u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(c), V::I(r), V::S(if r <= 10 { "Top 10 Posts" } else { "Other Posts" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, COALESCE(SUM(b.Class), 0) AS BadgeCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId AND v.VoteTypeId = 8 GROUP BY u.Id),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// AggregatedResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, ur.BadgeCount, ur.TotalBounties, COALESCE(pc.CommentCount, 0) AS CommentCount
//     FROM RankedPosts rp LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId WHERE rp.Rank = 1)
// SELECT ar.PostId, ar.Title, ar.CreationDate, ar.Score, ar.BadgeCount, ar.TotalBounties, ar.CommentCount,
//        CASE WHEN ar.Score > 10 THEN 'Highly Active' WHEN ar.Score BETWEEN 5 AND 10 THEN 'Moderately Active' ELSE 'Less Active' END AS ActivityLevel
// FROM AggregatedResults ar WHERE ar.BadgeCount > 0 ORDER BY ar.Score DESC, ar.CreationDate DESC LIMIT 50;
fn q28(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(bounty.opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.unwrap_or(0), a[1] + b.flatten().unwrap_or(0)]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&pc).and(owner_user.select((&ur).filt(|a| a[0] > 0))));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (c, a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if s > 10 { "Highly Active" } else if s >= 5 { "Moderately Active" } else { "Less Active" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.LastActivityDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        COUNT(DISTINCT c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rank,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpvoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownvoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' AND p.ViewCount > 0
//     GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, p.LastActivityDate, p.OwnerUserId, u.DisplayName),
// TopUserPosts AS (SELECT r.PostId, r.Title, r.ViewCount, r.OwnerDisplayName, r.CommentCount, r.UpvoteCount, r.DownvoteCount, r.Rank FROM RankedPosts r WHERE r.Rank <= 5)
// SELECT t.OwnerDisplayName, COUNT(t.PostId) AS TotalPosts, SUM(t.ViewCount) AS TotalViews, SUM(t.UpvoteCount) AS TotalUpvotes, SUM(t.DownvoteCount) AS TotalDownvotes,
//        AVG(t.CommentCount) AS AvgCommentsPerPost
// FROM TopUserPosts t GROUP BY t.OwnerDisplayName ORDER BY TotalViews DESC LIMIT 10;
fn q1245(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).with(view_count.gt(0)).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(view_count.get(p)), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let g = (&tp)
        .group_by(owner_user.select(&db.user.display_name).opt())
        .select(view_count.and(&vc).and(&cc))
        .fold([0i64; 5], |a, ((w, v), c)| [a[0] + 1, a[1] + w, a[2] + v[0], a[3] + v[1], a[4] + c]);
    let v = top_n(drain(&g), |&(n, a)| (Reverse(a[1]), n), 10);
    rows(v.into_iter().map(|(n, a)| row(vec![harness::fmt::ostr(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0])])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation),
// ClosedQuestions AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PostDetails AS (SELECT rp.Id, rp.Title, ur.Reputation, COALESCE(uq.CloseCount, 0) AS CloseCount, CASE WHEN COALESCE(uq.CloseCount, 0) > 0 THEN 'Closed' ELSE 'Active' END AS Status
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN ClosedQuestions uq ON rp.Id = uq.PostId WHERE rp.PostRank = 1)
// SELECT pd.Title, pd.Reputation, pd.Status, COUNT(c.Id) AS CommentCount FROM PostDetails pd LEFT JOIN Comments c ON pd.Id = c.PostId WHERE pd.Reputation > 1000
// GROUP BY pd.Title, pd.Reputation, pd.Status ORDER BY pd.Reputation DESC, pd.Title ASC;
//
// TotalBounty is never read; UserReputation has one row per user, so its join only drops the ownerless posts.
fn q4714(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, title, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).collect();
    let rep = owner_user.select((&db.user.reputation).gt(1000));
    let status = Ident::<Post>::new().with(&closed).opt().map(|c: Option<Id<Post>>| if c.is_some() { "Closed" } else { "Active" });
    let g = (&tp).with(rep).group_by(title.opt().and(owner_user.select(&db.user.reputation)).and(status)).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain(&g);
    v.sort_by_key(|&(((t, r), _), _)| (Reverse(r), t.is_none(), t));
    rows(v.into_iter().map(|(((t, r), s), n)| row(vec![harness::fmt::ostr(t), V::I(r), V::S(s), V::I(n)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.OwnerUserId, p.CreationDate),
// TopPosts AS (SELECT PostId, Title, Score, CommentCount, UpvoteCount, DownvoteCount FROM RankedPosts WHERE rn = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS TotalBadges, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT up.DisplayName, up.TotalBadges, up.TotalScore, tp.Title, tp.Score AS PostScore, tp.CommentCount, tp.UpvoteCount, tp.DownvoteCount
// FROM UserReputation up JOIN TopPosts tp ON up.UserId = tp.PostId ORDER BY up.TotalScore DESC, tp.Score DESC LIMIT 50;
//
// The join compares a user id with a post id, so it goes through the raw ids. rn reads only base columns, so the newest
// posts are picked first and the comment x vote product is driven for those alone.
fn q6290(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let pv = rel(drain(&ps));
    let by_id: HashIdx<i64, (Id<Post>, [i64; 3])> = (&pv).map(|(p, _)| p).select(origid).inv().select(&pv).collect();
    let ur = db
        .user
        .with((&db.user.origid).select(&by_id))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(score).opt()))
        .fold([0i64; 3], |a, (b, s)| [a[0] + b.is_some() as i64, a[1] + s.is_some() as i64, a[2] + s.unwrap_or(0)]);
    let v = drain((&ur).and((&db.user.origid).select(&by_id)));
    let v = top_n(v, |&(_, (a, (p, _)))| (a[1] == 0, Reverse(a[2]), Reverse(score.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(u, (a, (p, c)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1])];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend(c.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerUserId, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.rn = 1
//     ORDER BY rp.UpVotes - rp.DownVotes DESC LIMIT 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes, pht.Name AS PostHistoryType
// FROM TopPosts tp JOIN PostHistory ph ON ph.PostId = tp.PostId JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes, pht.Name ORDER BY tp.UpVotes DESC;
//
// rn partitions by the post itself, so it is always 1.
fn q9614(db: &'static So) -> String {
    let s = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let top = top_n(drain(&s), |&(p, a)| (Reverse(a[1] - a[2]), p), 10);
    let tv = rel(top);
    let tp: HashIdx<Id<Post>, (Id<Post>, [i64; 3])> = (&tv).map(|(p, _)| p).inv().select(&tv).collect();
    let recent = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(htype_name(db));
    let g = (&tp).group_by(Same::<(Id<Post>, [i64; 3])>::new().and(Same::<(Id<Post>, [i64; 3])>::new().map(|(p, _)| p).select(recent))).select(Same::<(Id<Post>, [i64; 3])>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain(&g);
    v.sort_by_key(|&(((_, a), _), _)| Reverse(a[1]));
    rows(v.into_iter().map(|(((p, a), n), _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(n));
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation > 0),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, ReputationRank FROM UserReputation WHERE ReputationRank <= 10),
// PostDetails AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        COALESCE(SUM(p.ViewCount), 0) AS TotalViews, COALESCE(SUM(p.Score), 0) AS TotalScore FROM Posts p WHERE p.OwnerUserId IN (SELECT UserId FROM TopUsers) GROUP BY p.OwnerUserId)
// SELECT tu.DisplayName, tu.Reputation, tu.ReputationRank, pd.TotalPosts, pd.TotalQuestions, pd.TotalAnswers, pd.TotalViews, pd.TotalScore, COALESCE(BadgeCount.TotalBadges, 0) AS TotalBadges
// FROM TopUsers tu LEFT JOIN PostDetails pd ON tu.UserId = pd.OwnerUserId
// LEFT JOIN (SELECT b.UserId, COUNT(b.Id) AS TotalBadges FROM Badges b WHERE b.Class IN (1, 2, 3) GROUP BY b.UserId) BadgeCount ON tu.UserId = BadgeCount.UserId
// ORDER BY tu.Reputation DESC, pd.TotalScore DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q30399(db: &'static So) -> String {
    let tu = ranked(drain((&db.user.reputation).gt(0)), |&(_, r)| Reverse(r), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let pd = (&by_user)
        .map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score)))
        .fold([0i64; 6], |a, ((t, w), s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + s, 0]);
    let bc = (&by_user).map(|(u, _)| u).group_by(Ident::<User>::new()).select(badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).is_in([1, 2, 3])))).fold(0i64, |n, _| n + 1);
    let mut v = drain((&by_user).and((&pd).opt()).and((&bc).opt()));
    v.sort_by_key(|&(u, ((_, p), _))| (Reverse(db.user.reputation.get(u).unwrap()), p.is_none(), Reverse(p.map(|a| a[4]))));
    rows(v.into_iter().map(|(u, (((_, r), p), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(r));
        f.extend(match p {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::I(b.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, AVG(U.Reputation) AS AvgReputation
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalAnswers, TotalQuestions, TotalUpVotes, TotalDownVotes, AvgReputation, RANK() OVER (ORDER BY TotalUpVotes DESC) AS UpvoteRank FROM UserActivity)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.TotalAnswers, U.TotalQuestions, U.TotalUpVotes, U.TotalDownVotes, U.AvgReputation, B.Name AS BadgeName, PH.PostId,
//        PH.CreationDate AS HistoryTimestamp, PH.Comment AS EditComment
// FROM TopUsers U LEFT JOIN Badges B ON U.UserId = B.UserId LEFT JOIN PostHistory PH ON U.UserId = PH.UserId
// WHERE U.TotalPosts > 10 AND U.UpvoteRank <= 10 AND PH.PostHistoryTypeId IN (24, 10) ORDER BY U.UpvoteRank, U.AvgReputation DESC;
//
// AVG(U.Reputation) averages one user's reputation over that user's joined rows, so it is the reputation itself.
fn q8834(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]
        });
    let dp = user_distinct_posts(db);
    let v = ranked(drain((&ua).and(&dp)), |&(_, (a, _))| Reverse(a[2]), false);
    let rk = rel(v);
    let tu = rel(drain((&rk).filt(|((_, (_, n)), r): ((Id<User>, ([i64; 4], i64)), i64)| r <= 10 && n > 10)).into_iter().map(|(_, ((u, (a, n)), _))| (u, (a, n))).collect());
    let top: HashIdx<Id<User>, (Id<User>, ([i64; 4], i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let PostHistory { user, post_history_type_id, post_id, creation_date, comment, .. } = &db.post_history;
    let ph_by: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(post_history_type_id.is_in([24, 10])).select(user).inv().collect();
    let v = drain((&top).and(badges_of(db).opt().and(&ph_by)));
    rows(v.into_iter().map(|(u, ((_, (a, n)), (b, h)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::F(db.user.reputation.get(u).unwrap() as f64)]);
        f.push(b.map_or(V::Null, |b| V::S(db.badge.name.get(b).unwrap())));
        f.extend([V::I(post_id.get(h).unwrap()), V::T(creation_date.get(h).unwrap()), harness::fmt::ostr(comment.get(h))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostMetrics AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, COALESCE(c.CommentCount, 0) AS TotalComments, COALESCE(v.TotalVotes, 0) AS TotalVotes
//     FROM TopPosts tp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON tp.PostId = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId) v ON tp.PostId = v.PostId)
// SELECT pm.PostId, pm.Title, pm.OwnerDisplayName, pm.CreationDate, pm.Score, pm.TotalComments, pm.TotalVotes FROM PostMetrics pm ORDER BY pm.Score DESC, pm.TotalVotes DESC;
fn q6474(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, v))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.extend([V::I(c), V::I(v)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT B.Id) AS BadgeCount, SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN P.ViewCount ELSE 0 END) AS AnswerViewCount, SUM(CASE WHEN P.PostTypeId IN (1, 2) THEN P.Score ELSE 0 END) AS TotalScore
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, QuestionCount, AnswerViewCount, TotalScore, ROW_NUMBER() OVER (ORDER BY Reputation DESC, TotalScore DESC) AS Rank
//     FROM UserStats WHERE Reputation >= 1000)
// SELECT T.UserId, T.DisplayName, T.Reputation, T.BadgeCount, T.QuestionCount, T.AnswerViewCount, T.TotalScore, T.Rank, PH.CreationDate AS LastActivityDate, COUNT(Comment.Id) AS TotalComments
// FROM TopUsers T LEFT JOIN Posts P ON T.UserId = P.OwnerUserId LEFT JOIN PostHistory PH ON P.Id = PH.PostId LEFT JOIN Comments Comment ON P.Id = Comment.PostId
// WHERE T.Rank <= 10 GROUP BY T.UserId, T.DisplayName, T.Reputation, T.BadgeCount, T.QuestionCount, T.AnswerViewCount, T.TotalScore, T.Rank, PH.CreationDate ORDER BY T.Rank;
fn q6924(db: &'static So) -> String {
    let Post { post_type_id, answer_count, view_count, score, .. } = &db.post;
    let rich = || db.user.with((&db.user.reputation).ge(1000));
    let us = rich()
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(answer_count.opt()).and(view_count.opt()).and(score)).opt()))
        .fold([0i64; 5], |a, (_, p)| {
            let (q, w, s) = match p {
                Some((((t, an), vw), s)) => (if t == 1 { an } else { Some(0) }, if t == 2 { vw } else { Some(0) }, if t == 1 || t == 2 { s } else { 0 }),
                None => (Some(0), Some(0), 0),
            };
            [a[0] + q.is_some() as i64, a[1] + q.unwrap_or(0), a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s]
        });
    let bd = rich().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = top_n(drain((&us).and(&bd)), |&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[4]), u), 10);
    let tu = rel(v.into_iter().enumerate().map(|(i, (u, (a, b)))| (u, (a, b, i as i64 + 1))).collect());
    let top: HashIdx<Id<User>, (Id<User>, ([i64; 5], i64, i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    type T = (Id<User>, Option<(Option<i64>, Option<Id<Comment>>)>);
    let base = (&top).map(|(u, _)| u).select(Ident::<User>::new().and(posts_of(db).select(history_of(db).select(&db.post_history.creation_date).opt().and(comments_of(db).opt())).opt()));
    let g = base.group_by(Same::<T>::new().map(|(u, x): T| (u, x.and_then(|(h, _)| h)))).select(Same::<T>::new()).fold(0i64, |n, (_, x): T| n + x.map_or(false, |(_, c)| c.is_some()) as i64);
    let v = drain((&g).map(|x| x).and(Same::<(Id<User>, Option<i64>)>::new().map(|(u, _)| u).select(&top)));
    rows(v.into_iter().map(|((u, d), (n, (_, (a, b, r))))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), nullable(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4]), V::I(r), harness::fmt::ots(d), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE vt.Name = 'UpMod') AS UpVotes,
//        COUNT(v.Id) FILTER (WHERE vt.Name = 'DownMod') AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE Rank <= 10)
// SELECT u.DisplayName AS UserDisplayName, tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes, SUM(b.Class) AS TotalBadgeClass, COUNT(DISTINCT b.Id) AS BadgeCount
// FROM TopPosts tp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) LEFT JOIN Badges b ON b.UserId = u.Id
// GROUP BY u.DisplayName, tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes ORDER BY tp.Score DESC, tp.CreationDate DESC LIMIT 15;
//
// Rank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
fn q8821(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, title, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vtype_name(db)).opt()))
        .fold([0i64; 3], |a, (c, n)| [a[0] + c.is_some() as i64, a[1] + (n == Some("UpMod")) as i64, a[2] + (n == Some("DownMod")) as i64]);
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(score).and(&s);
    let owned = (&tp).with(owner_user);
    let sums = owned.group_by(&key).select(owner_user.select(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 2], |a, c| [a[0] + c.is_some() as i64, a[1] + c.unwrap_or(0)]);
    let ids = (&tp).with(owner_user).group_by(&key).select(owner_user.select(badges_of(db).opt())).buf_fold(distinct_some);
    let v = drain((&sums).and(&ids));
    let v = top_n(v, |&(((((_, _), d), sc), _), _)| (Reverse(sc), Reverse(d)), 15);
    rows(v.into_iter().map(|(((((n, t), d), sc), a), (b, k))| {
        row(vec![V::S(n), harness::fmt::ostr(t), V::T(d), V::I(sc), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(b[1], b[0]), V::I(k)])
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.Score, p.ViewCount, p.AnswerCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank FROM Posts p WHERE p.PostTypeId = 1),
// PostAnalytics AS (SELECT up.UserId, COUNT(DISTINCT tp.Id) AS PostCount, SUM(tp.Score) AS TotalScore, AVG(tp.ViewCount) AS AverageViews, MAX(tp.Score) AS MaxScore, SUM(tp.AnswerCount) AS TotalAnswers
//     FROM TopPosts tp JOIN UserBadges up ON tp.OwnerUserId = up.UserId WHERE tp.ScoreRank <= 5 GROUP BY up.UserId)
// SELECT u.DisplayName, ua.BadgeCount, ua.GoldBadges, ua.SilverBadges, ua.BronzeBadges, pa.PostCount, pa.TotalScore, pa.AverageViews, pa.MaxScore, pa.TotalAnswers
// FROM Users u JOIN UserBadges ua ON u.Id = ua.UserId JOIN PostAnalytics pa ON u.Id = pa.UserId WHERE ua.BadgeCount > 0 ORDER BY pa.TotalScore DESC, u.Reputation DESC;
fn q9206(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, owner_user, score, view_count, answer_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pa = (&tp).group_by(owner_user).select(score.and(view_count.opt()).and(answer_count.opt())).fold([0, 0, 0, 0, i64::MIN, 0, 0], |a, ((s, w), n)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4].max(s), a[5] + n.is_some() as i64, a[6] + n.unwrap_or(0)]
    });
    let mut v = drain((&ub).filt(|a| a[0] > 0).and(&pa));
    v.sort_by_key(|&(u, (_, a))| (Reverse(a[1]), Reverse(db.user.reputation.get(u).unwrap())));
    rows(v.into_iter().map(|(u, (b, a))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(a[4]), nullable(a[6], a[5])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COALESCE(v.UpVoteCount, 0) AS UpVotes, COALESCE(v.DownVoteCount, 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.OwnerDisplayName, rp.UpVotes, rp.DownVotes, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount,
//        (SELECT COUNT(*) FROM Posts a WHERE a.ParentId = rp.PostId) AS AnswerCount FROM RankedPosts rp WHERE rp.rn = 1)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.OwnerDisplayName, pd.UpVotes, pd.DownVotes, pd.CommentCount, pd.AnswerCount,
//        DENSE_RANK() OVER (ORDER BY pd.ViewCount DESC) AS ViewRank, DENSE_RANK() OVER (ORDER BY pd.UpVotes DESC) AS UpVoteRank
// FROM PostDetails pd ORDER BY pd.CreationDate DESC LIMIT 100;
//
// rn partitions by the post itself, so it is always 1.
fn q5423(db: &'static So) -> String {
    let Post { owner_user, view_count, creation_date, .. } = &db.post;
    let owned = || db.post.with(owner_user);
    let vs = owned().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = ranked(drain(&vs), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, true);
    let v = ranked(v, |&((_, a), _)| Reverse(a[0]), true);
    let v = top_n(v, |&(((p, _), _), _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    let sel = rel(v.into_iter().map(|(((p, a), w), u)| (p, (a, w, u))).collect());
    let idx: HashIdx<Id<Post>, (Id<Post>, ([i64; 2], i64, i64))> = (&sel).map(|(p, _)| p).inv().select(&sel).collect();
    let cc = (&idx).map(|(p, _)| p).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&idx).map(|(p, _)| p).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&idx).and(&cc).and(&ac)).into_iter().map(|(p, (((_, (a, w, u)), c), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(n), V::I(w), V::I(u)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS RankScore FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND P.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE RankScore <= 10),
// PostComments AS (SELECT PC.PostId, COUNT(PC.Id) AS CommentCount FROM Comments PC GROUP BY PC.PostId),
// PostHistoryAggregated AS (SELECT PH.PostId, COUNT(PH.Id) AS EditCount, MAX(PH.CreationDate) AS LastEdited FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (4, 5, 6) GROUP BY PH.PostId)
// SELECT TP.PostId, TP.Title, TP.CreationDate, TP.Score, TP.ViewCount, TP.OwnerDisplayName, COALESCE(PC.CommentCount, 0) AS TotalComments, COALESCE(PHA.EditCount, 0) AS TotalEdits, PHA.LastEdited
// FROM TopPosts TP LEFT JOIN PostComments PC ON TP.PostId = PC.PostId LEFT JOIN PostHistoryAggregated PHA ON TP.PostId = PHA.PostId ORDER BY TP.Score DESC, TP.ViewCount DESC;
fn q9472(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.is_in([1, 2]))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6]))).select(&db.post_history.creation_date);
    let ed = (&tp).group_by(Ident::<Post>::new()).select(edits.opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    rows(drain((&cc).and(&ed)).into_iter().map(|(p, (c, (n, m)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(n), tmax(m)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, Score, ViewCount FROM RankedPosts WHERE PostRank <= 5),
// CommentsSummary AS (SELECT PostId, COUNT(*) AS CommentCount, AVG(Score) AS AverageCommentScore FROM Comments GROUP BY PostId),
// FinalResult AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, cs.CommentCount, cs.AverageCommentScore FROM TopPosts tp LEFT JOIN CommentsSummary cs ON tp.PostId = cs.PostId)
// SELECT fr.PostId, fr.Title, fr.OwnerDisplayName, fr.CreationDate, fr.Score, fr.ViewCount, COALESCE(fr.CommentCount, 0) AS CommentCount, COALESCE(fr.AverageCommentScore, 0) AS AverageCommentScore
// FROM FinalResult fr ORDER BY fr.Score DESC, fr.CreationDate DESC;
fn q7353(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0)).and(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    rows(drain(&cs).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(a[0]), if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) }]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '6 months'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.Tags, COUNT(c.Id) AS CommentCount FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId
//     WHERE rp.Rank <= 10 GROUP BY rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.Tags),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, ue.DisplayName, ue.UpVotes, ue.DownVotes, ue.BadgeCount
// FROM TopPosts tp JOIN Posts p ON tp.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id JOIN UserEngagement ue ON u.Id = ue.UserId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q7287(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, view_count, .. } = &db.post;
    let since = ny_to_utc(add_months(utc_to_ny(now_utc()), -6));
    let v = drain(db.post.with(creation_date.filt(move |d| ny_to_utc(d) >= since)).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ue = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]);
    let mut v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&ue))));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(c), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// RecentActivity AS (SELECT ua.UserId, ua.DisplayName, ua.TotalViews, ua.TotalPosts, ua.TotalComments, RANK() OVER (ORDER BY ua.TotalViews DESC) AS ViewRank,
//        RANK() OVER (ORDER BY ua.TotalPosts DESC) AS PostRank FROM UserActivity ua),
// TopUsers AS (SELECT r.UserId, r.DisplayName, r.TotalViews, r.TotalPosts, r.TotalComments, r.ViewRank, r.PostRank FROM RecentActivity r WHERE r.ViewRank <= 10 OR r.PostRank <= 10)
// SELECT tu.DisplayName, tu.TotalViews, tu.TotalPosts, tu.TotalComments, COALESCE(CONCAT('Rank in Views: ', tu.ViewRank), 'N/A') AS ViewRank,
//        COALESCE(CONCAT('Rank in Posts: ', tu.PostRank), 'N/A') AS PostRank, COALESCE(b.Name, 'No Badge') AS HighestBadge, COUNT(v.Id) AS VoteCount
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId AND b.Class = 1 LEFT JOIN Votes v ON tu.UserId = v.UserId
// GROUP BY tu.UserId, tu.DisplayName, tu.TotalViews, tu.TotalPosts, tu.TotalComments, tu.ViewRank, tu.PostRank, b.Name ORDER BY tu.TotalViews DESC, tu.TotalPosts DESC;
//
// Each comment belongs to one post, so over Posts x Comments the comment rows are already distinct.
fn q347(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(comments_of(db).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((w, c)) => [a[0] + w.unwrap_or(0), a[1] + c.is_some() as i64],
            None => a,
        });
    let dp = user_distinct_posts(db);
    let v = ranked(drain((&ua).and(&dp)), |&(_, (a, _))| Reverse(a[0]), false);
    let v = ranked(v, |&((_, (_, n)), _)| Reverse(n), false);
    let tu = rel(v.into_iter().filter(|&(((_, _), w), p)| w <= 10 || p <= 10).map(|(((u, (a, n)), w), p)| (u, (a, n, w, p))).collect());
    type R = (Id<User>, ([i64; 2], i64, i64, i64));
    let top: HashIdx<Id<User>, R> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    type T = (R, Option<Str>);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))).select(&db.badge.name);
    let base = (&top).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _)| u).select(gold.opt().and(votes_by(db).opt()))));
    let g = base.group_by(Same::<(R, (Option<Str>, Option<Id<Vote>>))>::new().map(|(r, (b, _))| (r, b))).select(Same::<(R, (Option<Str>, Option<Id<Vote>>))>::new()).fold(0i64, |n, (_, (_, v))| n + v.is_some() as i64);
    let mut v: Vec<(T, i64)> = drain(&g);
    v.sort_by_key(|&(((_, (a, n, _, _)), _), _)| (Reverse(a[0]), Reverse(n)));
    rows(v.into_iter().map(|(((u, (a, n, w, p)), b), c)| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(n), V::I(a[1]), V::Owned(format!("Rank in Views: {w}")), V::Owned(format!("Rank in Posts: {p}")), V::S(b.unwrap_or("No Badge")), V::I(c)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// CommentsSummary AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments FROM Comments c GROUP BY c.PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS ClosedCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ur.Reputation, COALESCE(ur.BadgeCount, 0) AS BadgeCount, COALESCE(cs.TotalComments, 0) AS TotalComments,
//        COALESCE(cp.ClosedCount, 0) AS ClosedPostCount
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN CommentsSummary cs ON rp.PostId = cs.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE rp.rn = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 100;
fn q2604(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let cl = (&tp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(&cl).and(owner_user.select(Ident::<User>::new().and(&bc))));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 100);
    rows(v.into_iter().map(|(p, ((c, k), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([user_col(db, u, "rep"), V::I(b), V::I(c), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount,
//        CASE WHEN rp.Score > 100 THEN 'High Score' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory, pt.Name AS PostType, u.DisplayName AS OwnerDisplayName
//     FROM RankedPosts rp JOIN PostTypes pt ON pt.Id = (SELECT PostTypeId FROM Posts WHERE Id = rp.PostId) JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId)
//     WHERE rp.UserPostRank <= 5)
// SELECT p.Title, p.CreationDate, p.Score, p.ViewCount, p.CommentCount, p.ScoreCategory, p.PostType, p.OwnerDisplayName FROM PostStatistics p
// WHERE p.ScoreCategory = 'High Score' OR p.ScoreCategory = 'Medium Score' ORDER BY p.CreationDate DESC LIMIT 10;
fn q7223(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).with(score.ge(50)).with(owner_user).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain(&cc), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, c)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(c), V::S(if s > 100 { "High Score" } else { "Medium Score" })]);
        f.extend(post_fields(db, p, &["type", "owner"]));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.ViewCount > 1000 THEN 1 ELSE 0 END) AS PopularPosts
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, PopularPosts, RANK() OVER (ORDER BY Reputation DESC) AS RankByReputation FROM UserReputation),
// UserBadges AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId)
// SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.PopularPosts, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges
// FROM TopUsers TU LEFT JOIN UserBadges UB ON TU.UserId = UB.UserId WHERE TU.RankByReputation <= 10 ORDER BY TU.RankByReputation;
fn q7376(db: &'static So) -> String {
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, view_count, .. } = &db.post;
    let ur = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.map_or(false, |w| w > 1000) as i64],
        None => a,
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    rows(drain((&ur).and((&ub).opt())).into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        match b {
            Some(b) => f.extend(b.map(V::I)),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(p.Score) AS TotalScore,
//        SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// ClosedPostCounts AS (SELECT ph.UserId, COUNT(*) AS ClosedPostCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId),
// UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ps.QuestionCount, 0) AS QuestionCount,
//        COALESCE(ps.AnswerCount, 0) AS AnswerCount, COALESCE(ps.TotalScore, 0) AS TotalScore, COALESCE(ps.TotalViews, 0) AS TotalViews, COALESCE(cpc.ClosedPostCount, 0) AS ClosedPostCount
//     FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN ClosedPostCounts cpc ON u.Id = cpc.UserId)
// SELECT UserId, DisplayName, Reputation, BadgeCount, QuestionCount, AnswerCount, TotalScore, TotalViews, ClosedPostCount, RANK() OVER (ORDER BY TotalScore DESC, Reputation DESC) AS PerformanceRank
// FROM UserPerformance WHERE Reputation > 100 ORDER BY PerformanceRank, TotalScore DESC;
fn q8217(db: &'static So) -> String {
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 4], |a, ((t, s), w)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.unwrap_or(0)]);
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with((&db.user.reputation).gt(100)).select((&ub).opt().and((&ps).opt()).and((&cp).opt())));
    let v = ranked(v, |&(u, ((_, p), _))| (Reverse(p.map_or(0, |a| a[2])), Reverse(db.user.reputation.get(u).unwrap())), false);
    rows(v.into_iter().map(|((u, ((b, p), c)), r)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(b.unwrap_or(0)));
        f.extend(p.unwrap_or([0; 4]).map(V::I));
        f.extend([V::I(c.unwrap_or(0)), V::I(r)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.AnswerCount > 0 THEN 1 ELSE 0 END) AS TotalAcceptedAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalAcceptedAnswers, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserReputation WHERE Reputation > 1000),
// UserBadges AS (SELECT ub.UserId, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b INNER JOIN TopUsers ub ON b.UserId = ub.UserId GROUP BY ub.UserId)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalAcceptedAnswers, ub.TotalBadges, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM TopUsers tu JOIN UserBadges ub ON tu.UserId = ub.UserId WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC;
fn q5504(db: &'static So) -> String {
    let tu = ranked(drain((&db.user.reputation).gt(1000)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, answer_count, .. } = &db.post;
    let ur = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(answer_count.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, n)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + n.map_or(false, |n| n > 0) as i64],
        None => a,
    });
    let ub = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    rows(drain((&ur).and(&ub)).into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore, p.PostTypeId
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount, rp.VoteCount, rp.RankByScore, pt.Name AS PostTypeName
//     FROM RankedPosts rp JOIN PostTypes pt ON rp.PostTypeId = pt.Id WHERE rp.RankByScore <= 5)
// SELECT trp.PostId, trp.Title, trp.Score, trp.ViewCount, trp.OwnerDisplayName, trp.CommentCount, trp.VoteCount, trp.PostTypeName, pht.Name AS PostHistoryType, ph.CreationDate AS HistoryCreationDate
// FROM TopRankedPosts trp LEFT JOIN PostHistory ph ON trp.PostId = ph.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id ORDER BY trp.RankByScore, trp.PostId;
//
// RankByScore reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
fn q9185(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&cc).and(&vc).and(history_of(db).opt())).into_iter().map(|(p, ((c, n), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(post_fields(db, p, &["type"]));
        f.extend(match h {
            Some(h) => [V::S(htype_name(db).get(h).unwrap()), V::T(db.post_history.creation_date.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers,
//        SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.Questions, 0) AS Questions,
//        COALESCE(ps.Answers, 0) AS Answers, COALESCE(ps.TotalScore, 0) AS TotalScore, COALESCE(ps.TotalViews, 0) AS TotalViews
//     FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT UserId, DisplayName, BadgeCount, TotalPosts, Questions, Answers, TotalScore, TotalViews, RANK() OVER (ORDER BY TotalScore DESC, TotalPosts DESC) AS PerformanceRank
// FROM UserPerformance ORDER BY PerformanceRank LIMIT 10;
fn q9517(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 5], |a, ((t, s), w)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0)]);
    let v = ranked(drain((&ub).and((&ps).opt())), |&(_, (_, p))| {
        let a = p.unwrap_or([0; 5]);
        (Reverse(a[3]), Reverse(a[0]))
    }, false);
    let v = top_n(v, |&(_, r)| r, 10);
    rows(v.into_iter().map(|((u, (b, p)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(b));
        f.extend(p.unwrap_or([0; 5]).map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// PostStats AS (SELECT p.Id, COUNT(cm.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments cm ON p.Id = cm.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id),
// ClosedPosts AS (SELECT p.Id, p.Title, ph.CreationDate, r.OwnerDisplayName, ph.Comment AS CloseReason FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10
//     JOIN RankedPosts r ON p.Id = r.Id)
// SELECT rp.Id, rp.Title, rp.CreationDate, rp.OwnerDisplayName, COALESCE(ps.CommentCount, 0) AS CommentCount, COALESCE(ps.UpVotes, 0) AS UpVotes, COALESCE(ps.DownVotes, 0) AS DownVotes, cp.CloseReason
// FROM RankedPosts rp LEFT JOIN PostStats ps ON rp.Id = ps.Id LEFT JOIN ClosedPosts cp ON rp.Id = cp.Id WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC LIMIT 10;
//
// rn reads only base columns, so each owner's newest question is picked first and the comment x vote product is driven for those alone.
fn q4214(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&ps).and(closes.opt()));
    let v = top_n(v, |&(p, (_, h))| (Reverse(creation_date.get(p).unwrap()), p, h), 10);
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(h.map_or(V::Null, |h| harness::fmt::ostr(db.post_history.comment.get(h))));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS Owner, p.CreationDate,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName, p.CreationDate, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, Owner, CreationDate, CommentCount FROM RankedPosts WHERE RankScore <= 10)
// SELECT tp.Title, tp.Owner, tp.Score, tp.ViewCount, tp.CommentCount, pt.Name AS PostType, COALESCE(h.RevisionCount, 0) AS RevisionCount, COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM TopPosts tp JOIN PostTypes pt ON tp.PostId IN (SELECT Id FROM Posts WHERE PostTypeId = pt.Id)
// LEFT JOIN (SELECT PostId, COUNT(*) AS RevisionCount FROM PostHistory GROUP BY PostId) h ON tp.PostId = h.PostId
// LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) = b.UserId
// ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q6497(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&cc).and(&hc).and(owner_user.select(&bc).opt())).into_iter().map(|(p, ((c, h), b))| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["type"]));
        f.extend([V::I(h), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount, SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteCount,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, AcceptedAnswerCount, VoteCount, GoldBadgeCount, SilverBadgeCount, BronzeBadgeCount,
//        RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStatistics)
// SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, AcceptedAnswerCount, VoteCount, GoldBadgeCount, SilverBadgeCount, BronzeBadgeCount
// FROM TopUsers WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the posts x votes x badges product is driven for those alone.
fn q5994(db: &'static So) -> String {
    let tu = ranked(drain((&db.user.reputation).gt(0)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, c)| {
            let (n, t, acc, v) = p.map_or((0, 0, false, None), |((t, x), v)| (1, t, x.is_some(), v));
            [a[0] + n, a[1] + (t == 2) as i64, a[2] + (t == 1 && acc) as i64, a[3] + matches!(v, Some(2 | 3)) as i64, a[4] + (c == Some(1)) as i64, a[5] + (c == Some(2)) as i64, a[6] + (c == Some(3)) as i64]
        });
    rows(drain(&s).into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges GROUP BY UserId),
// TopUsers AS (SELECT Id, DisplayName, Reputation, CreationDate, LastAccessDate, Views, UpVotes, DownVotes, COALESCE(uc.BadgeCount, 0) AS TotalBadges, uc.GoldBadges, uc.SilverBadges, uc.BronzeBadges
//     FROM Users u LEFT JOIN UserBadgeCounts uc ON u.Id = uc.UserId WHERE Reputation > 1000 ORDER BY Reputation DESC LIMIT 10),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN TopUsers tu ON p.OwnerUserId = tu.Id)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalBadges, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount
// FROM TopUsers tu INNER JOIN RecentPosts rp ON tu.Id = rp.OwnerUserId WHERE rp.rn <= 3 ORDER BY tu.Reputation DESC, rp.CreationDate DESC;
fn q6832(db: &'static So) -> String {
    let tu = top_n(drain((&db.user.reputation).gt(1000)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain((&tu).select(posts_of(db)));
    let top = top_per(v, |&(u, _)| u, |&(_, p)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let rp = rel(top.into_iter().map(|x| x.1).collect());
    rows(drain((&rp).select(Ident::<Post>::new().and(owner_user.select((&ub).opt())))).into_iter().map(|(_, (p, b))| {
        let u = owner_user.get(p).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        match b {
            Some(b) => f.extend(b.map(V::I)),
            None => f.extend([V::I(0), V::Null, V::Null, V::Null]),
        }
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        row(f)
    }))
}

// WITH UserPosts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, AVG(p.ViewCount) AS AvgViews FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostScores AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Score, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank,
//        CASE WHEN p.Score >= 10 THEN 'High' WHEN p.Score >= 5 THEN 'Medium' ELSE 'Low' END AS ScoreCategory FROM Posts p),
// TopPosts AS (SELECT up.UserId, up.DisplayName, ps.PostId, ps.ScoreRank, ps.ScoreCategory, COALESCE(pht.Comment, 'No comments') AS LastEditComment
//     FROM UserPosts up JOIN PostScores ps ON up.UserId = ps.OwnerUserId LEFT JOIN PostHistory pht ON ps.PostId = pht.PostId AND pht.PostHistoryTypeId IN (24, 12) WHERE ps.ScoreRank <= 3)
// SELECT tp.UserId, tp.DisplayName, tp.PostId, tp.ScoreRank, tp.ScoreCategory, tp.LastEditComment, up.TotalPosts, up.Questions, up.Answers, up.AvgViews
// FROM TopPosts tp JOIN UserPosts up ON tp.UserId = up.UserId ORDER BY up.TotalPosts DESC, tp.ScoreRank ASC;
fn q74(db: &'static So) -> String {
    let ups = user_posts(db);
    let Post { owner_user, score, .. } = &db.post;
    let v = ranked(drain(db.post.select(owner_user.opt())), |&(p, u)| (u, Reverse(score.get(p).unwrap())), false);
    let v = per_group(v, |&(_, u)| u);
    let ps = rel(v.into_iter().filter(|x| x.1 <= 3).map(|((p, _), r)| (p, r)).collect());
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([24, 12])));
    let v = drain((&ps).select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(p, _)| p).select(owner_user.select(Ident::<User>::new().and(&ups)).and(edits.opt())))));
    rows(v.into_iter().map(|(_, ((p, r), ((u, a), h)))| {
        let s = score.get(p).unwrap();
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(r), V::S(if s >= 10 { "High" } else if s >= 5 { "Medium" } else { "Low" })]);
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No comments")));
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[6], a[5])]);
        row(f)
    }))
}

// Rewritten (rewrites/9698.sql): the ROW_NUMBER order tie-broken on p.Id.
// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// HighReputationUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, GoldBadges, SilverBadges, BronzeBadges FROM UserBadges WHERE Reputation > 1000),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, p.CommentCount, p.CreationDate, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC, p.Id) AS Rank FROM Posts p WHERE p.PostTypeId = 1)
// SELECT bh.DisplayName AS Author, bh.Reputation, p.Title AS PostTitle, p.ViewCount AS Popularity, bh.GoldBadges, bh.SilverBadges, bh.BronzeBadges
// FROM HighReputationUsers bh INNER JOIN PopularPosts p ON bh.UserId = p.OwnerUserId WHERE p.Rank <= 5 ORDER BY bh.Reputation DESC, p.ViewCount DESC;
fn q9698(db: &'static So) -> String {
    let ub = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let Post { post_type_id, owner_user, view_count, origid, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), origid.get(p).unwrap())
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    rows(drain((&tp).select(owner_user.select(Ident::<User>::new().and(&ub)))).into_iter().map(|(p, (u, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// EligibleUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(rp.Score) AS TotalScore, RANK() OVER (ORDER BY SUM(rp.Score) DESC) AS UserRank
//     FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT eu.UserId, eu.DisplayName, eu.Reputation, eu.BadgeCount, eu.TotalScore, eu.UserRank, COUNT(rp.PostId) AS PostCount
// FROM EligibleUsers eu LEFT JOIN RankedPosts rp ON eu.UserId = rp.OwnerUserId WHERE eu.UserRank <= 10
// GROUP BY eu.UserId, eu.DisplayName, eu.Reputation, eu.BadgeCount, eu.TotalScore, eu.UserRank ORDER BY eu.UserRank;
//
// RankedPosts has one row per recent post; none of its aggregates or its rank is read, so only the posts are used.
fn q5852(db: &'static So) -> String {
    let recent = || posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let eu = db
        .user
        .group_by(Ident::<User>::new())
        .select(recent().select(&db.post.score).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (s, b)| [a[0] + b.is_some() as i64, a[1] + s.is_some() as i64, a[2] + s.unwrap_or(0)]);
    let v = ranked(drain(&eu), |&(_, a)| (a[1] == 0, Reverse(a[2])), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| (u, (a, r))).collect());
    let top: HashIdx<Id<User>, (Id<User>, ([i64; 3], i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let pc = (&top).map(|(u, _)| u).group_by(Ident::<User>::new()).select(recent().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&top).and(&pc)).into_iter().map(|(u, ((_, (a, r)), n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::I(r), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerName, p.CreationDate, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerName, rp.CreationDate, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostStatistics AS (SELECT tp.PostId, tp.Title, tp.OwnerName, tp.CreationDate, tp.Score, tp.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.OwnerName, tp.CreationDate, tp.Score, tp.ViewCount)
// SELECT ps.PostId, ps.Title, ps.OwnerName, ps.CreationDate, ps.Score, ps.ViewCount, ps.CommentCount, ps.UpVotes, ps.DownVotes, (ps.UpVotes - ps.DownVotes) AS NetVotes
// FROM PostStatistics ps ORDER BY ps.Score DESC, ps.ViewCount DESC;
fn q6900(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(V::I(a[1] - a[2]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopContributors AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats WHERE PostCount > 0),
// ScoreBoard AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, BadgeCount, Rank, (Reputation + UpVotes * 10 - DownVotes * 5 + BadgeCount * 3) AS Score FROM TopContributors)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, BadgeCount, Rank, Score FROM ScoreBoard ORDER BY Score DESC FETCH FIRST 10 ROWS ONLY;
fn q8267(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let dp = user_distinct_posts(db);
    let v = ranked(drain((&dp).filt(|n| n > 0).and(&us)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let score = |u: Id<User>, a: [i64; 5]| db.user.reputation.get(u).unwrap() + a[2] * 10 - a[3] * 5 + a[4] * 3;
    let v = top_n(v, |&((u, (_, a)), _)| (Reverse(score(u, a)), u), 10);
    rows(v.into_iter().map(|((u, (n, a)), r)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(r), V::I(score(u, a))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, rp.VoteCount, au.DisplayName AS TopUser FROM RankedPosts rp JOIN ActiveUsers au ON rp.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.CommentCount, tp.VoteCount, tp.TopUser FROM TopPosts tp ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// The ON clause names only rp, so the top posts are crossed with the active users. Rank reads only base columns, so the top posts
// are picked first and the comment x vote product is driven for those alone; the vote sums of ActiveUsers are never read.
fn q8988(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = ranked(drain(db.post.with(creation_date.gt(add_years(t0, -1))).select(Ident::<Post>::new())), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let rp = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let au: MatSet<Id<User>> = db.user.with((&db.user.creation_date).gt(add_years(t0, -1))).select(Ident::<User>::new()).collect();
    let mut v = Vec::new();
    (&rp).cross(&au).drive(|(p, u), (a, _)| v.push((p, u, a)));
    rows(v.into_iter().map(|(p, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), user_col(db, u, "name")]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        AVG(p.Score) AS AverageScore FROM Posts p GROUP BY p.OwnerUserId),
// TopUsers AS (SELECT ur.DisplayName, ps.OwnerUserId, ps.PostCount, ps.QuestionCount, ps.AnswerCount, ps.AverageScore FROM UserReputation ur JOIN PostStatistics ps ON ur.UserId = ps.OwnerUserId
//     WHERE ur.Reputation > 1000 ORDER BY ur.Reputation DESC LIMIT 10)
// SELECT tu.DisplayName, COALESCE(ps.QuestionCount, 0) AS TotalQuestions, COALESCE(ps.AnswerCount, 0) AS TotalAnswers, COALESCE(ps.AverageScore, 0) AS AvgPostScore,
//        (SELECT COUNT(*) FROM Badges b WHERE b.UserId = tu.OwnerUserId AND b.Class = 1) AS GoldBadges, (SELECT COUNT(*) FROM Badges b WHERE b.UserId = tu.OwnerUserId AND b.Class = 2) AS SilverBadges,
//        (SELECT COUNT(*) FROM Badges b WHERE b.UserId = tu.OwnerUserId AND b.Class = 3) AS BronzeBadges
// FROM TopUsers tu LEFT JOIN PostStatistics ps ON tu.OwnerUserId = ps.OwnerUserId WHERE (ps.PostCount > 5 OR ps.QuestionCount > 0) ORDER BY TotalQuestions DESC, TotalAnswers DESC;
fn q961(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&ps)), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    let tu = rel(tu);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let kept = (&tu).filt(|(_, a): (Id<User>, [i64; 4])| a[0] > 5 || a[1] > 0);
    let mut v = drain(kept.select(Same::<(Id<User>, [i64; 4])>::new().and(Same::<(Id<User>, [i64; 4])>::new().map(|(u, _)| u).select(&bc))));
    v.sort_by_key(|&(_, ((_, a), _))| (Reverse(a[1]), Reverse(a[2])));
    rows(v.into_iter().map(|(_, ((u, a), b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])];
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopUsers AS (SELECT UserId, Reputation, CreationDate, LastAccessDate, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank
//     FROM UserBadges ub JOIN Users u ON ub.UserId = u.Id),
// TopPosts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' GROUP BY p.OwnerUserId),
// FinalBenchmark AS (SELECT tu.UserId, tu.Reputation, tu.CreationDate, tu.LastAccessDate, tu.BadgeCount, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, tp.PostCount, tp.TotalScore, tp.AvgViewCount
//     FROM TopUsers tu JOIN TopPosts tp ON tu.UserId = tp.OwnerUserId WHERE tu.Rank <= 10)
// SELECT *, (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = fb.UserId) AS TotalPostsCreated FROM FinalBenchmark fb ORDER BY Reputation DESC;
fn q5517(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ub = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let tp = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let all = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&ub).and(&tp).and(&all)).into_iter().map(|(u, ((b, a), n))| {
        let mut f = ucols(db, u, &["uid", "rep", "ucreated", "last_access"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RankByType
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, u.DisplayName, p.CreationDate, p.Score, p.PostTypeId),
// TopRankedPosts AS (SELECT PostId, Title, OwnerUserId, OwnerDisplayName, CreationDate, Score, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE RankByType <= 5)
// SELECT t.OwnerDisplayName, t.Title, t.Score, t.CommentCount, t.UpVotes, t.DownVotes, ph.CreationDate AS HistoryCreationDate, pht.Name AS PostHistoryType, ph.Comment AS EditComment
// FROM TopRankedPosts t LEFT JOIN PostHistory ph ON t.PostId = ph.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id ORDER BY t.Score DESC, t.PostId ASC;
//
// RankByType reads only base columns, so the newest posts are picked first and the comment x vote product is driven for those alone.
fn q5654(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&s).and(history_of(db).opt())).into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["owner", "title", "score"]);
        f.extend(a.map(V::I));
        f.extend(match h {
            Some(h) => [V::T(db.post_history.creation_date.get(h).unwrap()), V::S(htype_name(db).get(h).unwrap()), harness::fmt::ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AverageScore FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// PerformanceBenchmark AS (SELECT ub.UserId, ub.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, tp.PostCount, tp.TotalViews, tp.AverageScore,
//        RANK() OVER (ORDER BY tp.TotalViews DESC) AS ViewRank, RANK() OVER (ORDER BY tp.PostCount DESC) AS PostRank FROM UserBadges ub LEFT JOIN TopPosts tp ON ub.UserId = tp.OwnerUserId)
// SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, PostCount, TotalViews, AverageScore, ViewRank, PostRank
// FROM PerformanceBenchmark WHERE BadgeCount > 0 ORDER BY BadgeCount DESC, TotalViews DESC LIMIT 10;
fn q9986(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let tp = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]
    });
    let tv = |t: Option<[i64; 4]>| t.filter(|a| a[1] > 0).map(|a| a[2]);
    let v = ranked(drain((&ub).and((&tp).opt())), |&(_, (_, t))| (tv(t).is_none(), Reverse(tv(t))), false);
    let v = ranked(v, |&((_, (_, t)), _)| (t.is_none(), Reverse(t.map(|a| a[0]))), false);
    let pb = rel(v);
    let kept = (&pb).filt(|(((_, (b, _)), _), _): (((Id<User>, ([i64; 4], Option<[i64; 4]>)), i64), i64)| b[0] > 0);
    let v = top_n(drain(kept), |&(_, (((u, (b, t)), _), _))| (Reverse(b[0]), tv(t).is_none(), Reverse(tv(t)), u), 10);
    rows(v.into_iter().map(|(_, (((u, (b, t)), w), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(match t {
            Some(a) => [V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0])],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::I(w), V::I(p)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserWithBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS MaxBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostsWithVotes AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT up.DisplayName, rp.Title, rp.Score, pwv.UpVotes, pwv.DownVotes, uwb.BadgeCount,
//        CASE WHEN uwb.MaxBadgeClass = 1 THEN 'Gold' WHEN uwb.MaxBadgeClass = 2 THEN 'Silver' WHEN uwb.MaxBadgeClass = 3 THEN 'Bronze' ELSE 'No Badge' END AS MaxBadge
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id JOIN UserWithBadges uwb ON up.Id = uwb.UserId JOIN PostsWithVotes pwv ON rp.PostId = pwv.PostId
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, uwb.BadgeCount DESC LIMIT 10;
fn q530(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&pv).and(owner_user.select(Ident::<User>::new().and(&ub))));
    let v = top_n(v, |&(p, (_, (_, (n, _))))| (Reverse(score.get(p).unwrap()), Reverse(n), p), 10);
    rows(v.into_iter().map(|(p, (a, (u, (n, m))))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::S(match m { 1 => "Gold", 2 => "Silver", 3 => "Bronze", _ => "No Badge" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostVoteCounts AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT up.DisplayName, up.Reputation, up.BadgeCount, up.GoldBadges, up.SilverBadges, up.BronzeBadges, rp.Title, rp.CreationDate, rp.Score, COALESCE(pvc.UpVotes, 0) AS UpVoteCount,
//        COALESCE(pvc.DownVotes, 0) AS DownVoteCount
// FROM RankedPosts rp JOIN UserReputation up ON rp.OwnerUserId = up.UserId LEFT JOIN PostVoteCounts pvc ON rp.PostId = pvc.PostId WHERE rp.rn = 1 ORDER BY up.Reputation DESC, rp.Score DESC LIMIT 10;
fn q4506(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&pv).and(owner_user.select(Ident::<User>::new().and(&ub))));
    let v = top_n(v, |&(p, (_, (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers, SUM(P.ViewCount) AS TotalViews,
//        AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId),
// AggregatedData AS (SELECT U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.Questions, 0) AS Questions, COALESCE(PS.Answers, 0) AS Answers, COALESCE(PS.TotalViews, 0) AS TotalViews,
//        COALESCE(PS.AverageScore, 0) AS AverageScore FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId),
// RankedUsers AS (SELECT *, RANK() OVER (ORDER BY BadgeCount DESC, TotalViews DESC, AverageScore DESC) AS UserRank FROM AggregatedData)
// SELECT DisplayName, BadgeCount, Questions, Answers, TotalViews, AverageScore, UserRank FROM RankedUsers WHERE UserRank <= 10 AND (BadgeCount > 0 OR TotalViews > 1000) ORDER BY UserRank, DisplayName;
fn q4962(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 5], |a, ((t, w), s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + s]);
    let row_of = |b: i64, p: Option<[i64; 5]>| {
        let a = p.unwrap_or([0; 5]);
        (b, a[1], a[2], a[3], if a[0] == 0 { 0.0 } else { a[4] as f64 / a[0] as f64 })
    };
    let v = ranked(drain((&ub).and((&ps).opt())), |&(_, (b, p))| {
        let r = row_of(b, p);
        (Reverse(r.0), Reverse(r.3), Reverse(fkey(r.4)))
    }, false);
    let rk = rel(v);
    let kept = (&rk).filt(|((_, (b, p)), r): ((Id<User>, (i64, Option<[i64; 5]>)), i64)| r <= 10 && (b > 0 || p.map_or(0, |a| a[3]) > 1000));
    rows(drain(kept).into_iter().map(|(_, ((u, (b, p)), r))| {
        let x = row_of(b, p);
        row(vec![user_col(db, u, "name"), V::I(x.0), V::I(x.1), V::I(x.2), V::I(x.3), V::F(x.4), V::I(r)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 0 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount FROM RankedPosts rp WHERE rp.rn <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpvoteCount, tp.DownvoteCount, (tp.UpvoteCount - tp.DownvoteCount) AS NetVotes,
//        (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = tp.PostId AND ph.PostHistoryTypeId = 10) AS CloseCount,
//        (SELECT COUNT(*) FROM Badges b WHERE b.UserId IN (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId)) AS OwnerBadgeCount
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// rn reads only base columns, so the newest posts are picked first and the comment x vote product is driven for those alone.
fn q8918(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let cl = (&tp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&s).and(&cl).and(owner_user.select(&bc).opt())).into_iter().map(|(p, ((a, c), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend([V::I(a[1] - a[2]), V::I(c), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RecursiveUserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// UserVotes AS (SELECT UserId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(p.Score) AS AvgScore FROM Posts p GROUP BY p.OwnerUserId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(r.BadgeCount, 0) AS BadgeCount, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes,
//        COALESCE(s.TotalPosts, 0) AS TotalPosts, COALESCE(s.Questions, 0) AS Questions, COALESCE(s.Answers, 0) AS Answers, COALESCE(s.AvgScore, 0) AS AvgScore
//     FROM Users u LEFT JOIN RecursiveUserBadges r ON u.Id = r.UserId LEFT JOIN UserVotes v ON u.Id = v.UserId LEFT JOIN PostStats s ON u.Id = s.OwnerUserId),
// FilteredUsers AS (SELECT * FROM UserActivity WHERE BadgeCount > 5 AND UpVotes > DownVotes)
// SELECT u.UserId, u.DisplayName, u.BadgeCount, u.UpVotes, u.DownVotes, u.TotalPosts, u.Questions, u.Answers, u.AvgScore, RANK() OVER (ORDER BY u.BadgeCount DESC, u.UpVotes DESC) AS Rank
// FROM FilteredUsers u ORDER BY Rank LIMIT 10;
fn q33081(db: &'static So) -> String {
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let ua = db.user.select((&bc).filt(|n| n > 5).and((&uv).filt(|a| a[0] > a[1])).and((&ps).opt()));
    let v = ranked(drain(ua), |&(_, ((b, a), _))| (Reverse(b), Reverse(a[0])), false);
    let v = top_n(v, |&(_, r)| r, 10);
    rows(v.into_iter().map(|((u, ((b, a), p)), r)| {
        let s = p.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(s[0]), V::I(s[1]), V::I(s[2]), if s[0] == 0 { V::F(0.0) } else { avg(s[3], s[0]) }, V::I(r)]);
        row(f)
    }))
}

// WITH RankedQuestions AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON a.ParentId = p.Id AND a.PostTypeId = 2 WHERE p.PostTypeId = 1 GROUP BY p.Id, u.DisplayName, p.Title, p.CreationDate, p.Score),
// RecentVotes AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId),
// PostStats AS (SELECT rq.Id, rq.Title, rq.CreationDate, rq.Score, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes, rq.AnswerCount,
//        CASE WHEN rq.Score >= 0 THEN 'Positive' ELSE 'Negative' END AS ScoreType FROM RankedQuestions rq LEFT JOIN RecentVotes rv ON rq.Id = rv.PostId)
// SELECT ps.*, CASE WHEN ps.ScoreType = 'Positive' THEN 'This question is well-received.' WHEN ps.ScoreType = 'Negative' THEN 'This question could use improvement.' ELSE 'No votes yet.' END AS Feedback
// FROM PostStats ps WHERE ps.AnswerCount > 5 ORDER BY ps.CreationDate DESC LIMIT 10;
fn q2691(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let rq = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let many: MatSet<Id<Post>> = db.post.with((&rq).filt(|n| n > 5)).collect();
    let pv = (&many).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain((&pv).and(&rq)), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, n))| {
        let pos = score.get(p).unwrap() >= 0;
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::S(if pos { "Positive" } else { "Negative" })]);
        f.push(V::S(if pos { "This question is well-received." } else { "This question could use improvement." }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, U.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users U ON p.OwnerUserId = U.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, U.DisplayName),
// AggregatedVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostHistoryDetail AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastEdited FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT p.Title, p.CreationDate, COALESCE(v.UpVotes, 0) AS TotalUpVotes, COALESCE(v.DownVotes, 0) AS TotalDownVotes, ph.LastEdited, p.OwnerName AS OwnerDisplayName, COALESCE(c.CommentCount, 0) AS TotalComments
// FROM RankedPosts p LEFT JOIN AggregatedVotes v ON p.PostId = v.PostId LEFT JOIN PostHistoryDetail ph ON p.PostId = ph.PostId
// LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.PostId = c.PostId WHERE p.rn = 1 ORDER BY p.CreationDate DESC FETCH FIRST 10 ROWS ONLY;
//
// rn partitions by the post itself, so it is always 1; the ten newest questions are picked first.
fn q1407(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new())), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6]))).select(&db.post_history.creation_date);
    let le = (&tp).group_by(Ident::<Post>::new()).select(edits.opt()).fold(i64::MIN, |m, d| m.max(d.unwrap_or(i64::MIN)));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&pv).and(&le).and(&cc)).into_iter().map(|(p, ((a, m), c))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), tmax(m)]);
        f.extend(post_fields(db, p, &["owner"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT pv.UserId) AS UniqueVoters, SUM(v.BountyAmount) AS TotalBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Votes pv ON p.Id = pv.PostId AND pv.VoteTypeId = 2 GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.CommentCount, ps.UniqueVoters, ps.TotalBounty, ROW_NUMBER() OVER (ORDER BY ps.Score DESC) AS Rank FROM PostStats ps)
// SELECT ups.UserId, ups.DisplayName, ups.TotalVotes, ups.Upvotes, ups.Downvotes, tp.Title, tp.CreationDate AS PostCreationDate, tp.Score AS PostScore, tp.CommentCount, tp.UniqueVoters, tp.TotalBounty
// FROM UserVoteStats ups JOIN TopPosts tp ON ups.TotalVotes > 10 WHERE tp.Rank <= 10 ORDER BY ups.TotalVotes DESC, tp.Score DESC;
//
// The ON clause names only ups, so users and top posts are crossed. Rank reads only Score, so the ten posts are picked first and the
// comments x votes x upvotes product is driven for those alone.
fn q8437(db: &'static So) -> String {
    let Vote { user, vote_type_id, bounty_amount, .. } = &db.vote;
    let uv = db.vote.group_by(user).select(vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let score = &db.post.score;
    let v = top_n(drain(db.post.select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let up = || votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2)));
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(bounty_amount.opt()).opt()).and(up().opt()))
        .fold([0i64; 3], |a, ((c, b), _)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let uq = (&tp).group_by(Ident::<Post>::new()).select(up().select(user)).count_distinct();
    let tv = rel(drain((&ps).and((&uq).opt())));
    let users: MatSet<Id<User>> = db.user.with((&uv).filt(|a| a[0] > 10)).collect();
    let mut v = Vec::new();
    (&users).cross(&tv).drive(|(u, _), (_, (p, (a, q)))| v.push((u, p, a, q)));
    rows(v.into_iter().map(|(u, p, a, q)| {
        let n = uv.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(n.map(V::I));
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(a[0]), V::I(q.unwrap_or(0)), nullable(a[2], a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.OwnerDisplayName, COUNT(*) AS TotalPosts, SUM(rp.Score) AS TotalScore, AVG(rp.ViewCount) AS AvgViewCount FROM RankedPosts rp WHERE rp.PostRank <= 5 GROUP BY rp.OwnerDisplayName),
// UsersWithBadges AS (SELECT u.Id, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// FinalResults AS (SELECT up.OwnerDisplayName AS DisplayName, up.TotalPosts, up.TotalScore, up.AvgViewCount, ub.BadgeCount FROM TopPosts up JOIN UsersWithBadges ub ON up.OwnerDisplayName = ub.DisplayName)
// SELECT fr.DisplayName, fr.TotalPosts, fr.TotalScore, fr.AvgViewCount, fr.BadgeCount, CASE WHEN fr.BadgeCount > 0 THEN 'Has Badges' ELSE 'No Badges' END AS BadgeStatus
// FROM FinalResults fr ORDER BY fr.TotalScore DESC, fr.TotalPosts DESC;
fn q9538(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let g = (&tp).group_by(owner_user.select(&db.user.display_name)).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    rows(drain((&g).and((&by_name).select(&ub))).into_iter().map(|(n, (a, b))| {
        row(vec![V::S(n), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(b), V::S(if b > 0 { "Has Badges" } else { "No Badges" })])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(vb.BountyAmount), 0) AS TotalBounty FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes vb ON p.Id = vb.PostId AND vb.VoteTypeId = 8
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.Rank, rp.CommentCount, rp.TotalBounty FROM RankedPosts rp WHERE rp.Rank <= 5 AND rp.CommentCount > 0),
// PostWithCloseReasons AS (SELECT p.Id AS PostId, ph.Comment AS CloseReason, COUNT(*) AS CloseReasonCount FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY p.Id, ph.Comment)
// SELECT fp.Title AS PostTitle, fp.Score, fp.CommentCount, COALESCE(pcr.CloseReason, 'No Close Reason') AS CloseReason, fp.TotalBounty
// FROM FilteredPosts fp LEFT JOIN PostWithCloseReasons pcr ON fp.PostId = pcr.PostId ORDER BY fp.Score DESC, fp.PostId OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// Rank reads only base columns, so the top posts are picked first and the comment x bounty-vote product is driven for those alone.
fn q24632(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let rp = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post.and(comment.opt())).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cv = rel(drain(&cr));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, Option<Str>), i64)> = (&cv).map(|((p, _), _)| p).inv().select(&cv).collect();
    let v = drain((&rp).filt(|a| a[0] > 0).and((&by_post).map(|((_, c), _)| c).opt()));
    let v = top_n(v, |&(p, (_, c))| (Reverse(score.get(p).unwrap()), db.post.origid.get(p).unwrap(), c), 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(a[0]), V::S(c.flatten().unwrap_or("No Close Reason")), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UniqueVoters,
//        RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3)
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UniqueVoters FROM RankedPosts rp WHERE rp.RankScore <= 10),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvotesGiven,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvotesGiven FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.UniqueVoters, ua.DisplayName AS CreatedBy, ua.PostsCreated, ua.UpvotesGiven, ua.DownvotesGiven
// FROM TopPosts tp JOIN UserActivity ua ON tp.PostId = ua.UserId ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// RankScore reads only base columns, so the ten questions are picked first. The join compares a post id with a user id, so it goes through the raw ids.
fn q8685(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, origid, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap())), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let ud = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(ud().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let uq = (&tp).group_by(Ident::<Post>::new()).select(ud().select(&db.vote.user)).count_distinct();
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let joined: MatSet<Id<User>> = (&tp).select(origid.select(&uid)).collect();
    let ua = (&joined)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, p| {
            let t = p.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let dp = (&joined).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&cc).and((&uq).opt()).and(origid.select(&uid).select(Ident::<User>::new().and(&dp).and(&ua)))).into_iter().map(|(p, ((c, q), ((u, n), a)))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([V::I(c), V::I(q.unwrap_or(0)), user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostHistories AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT up.UserId, u.DisplayName, u.Reputation, up.GoldBadges, up.SilverBadges, up.BronzeBadges, pp.PostId, pp.Title, pp.CreationDate, pp.Score, ph.EditCount, ph.LastEditDate
// FROM UserReputation up JOIN RankedPosts pp ON up.UserId = pp.OwnerUserId LEFT JOIN PostHistories ph ON pp.PostId = ph.PostId JOIN Users u ON up.UserId = u.Id
// WHERE pp.Rank = 1 AND (pp.Score > 10 OR up.Reputation > 1000) ORDER BY up.Reputation DESC, pp.Score DESC LIMIT 50;
fn q1867(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp = rel(top);
    let kept = (&tp).filt(|(p, u): (Id<Post>, Id<User>)| score.get(p).unwrap() > 10 || db.user.reputation.get(u).unwrap() > 1000);
    let pp: MatSet<Id<Post>> = kept.map(|(p, _)| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6]))).select(&db.post_history.creation_date);
    let ph = (&pp).group_by(Ident::<Post>::new()).select(edits).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&pp).select(owner_user.select(Ident::<User>::new().and(&ub)).and((&ph).opt())));
    let v = top_n(v, |&(p, ((u, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((u, b), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis, AVG(P.Score) AS AvgScore,
//        AVG(P.ViewCount) AS AvgViewCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalTagWikis, AvgScore, AvgViewCount, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.TotalTagWikis, U.AvgScore, U.AvgViewCount, COALESCE(BadgeCount.BadgeCount, 0) AS TotalBadges,
//        COALESCE(VoteCount.VoteCount, 0) AS TotalVotes
// FROM TopUsers U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) AS BadgeCount ON U.UserId = BadgeCount.UserId
// LEFT JOIN (SELECT UserId, COUNT(*) AS VoteCount FROM Votes GROUP BY UserId) AS VoteCount ON U.UserId = VoteCount.UserId WHERE U.Rank <= 10 ORDER BY U.TotalPosts DESC;
fn q7469(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 7], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64, a[4] + s, a[5] + w.is_some() as i64, a[6] + w.unwrap_or(0)],
        None => a,
    });
    let v = top_n(drain(&s), |&(u, a)| (Reverse(a[0]), u), 10);
    let tu = rel(v);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let vc = db.vote.group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tu).select(Same::<(Id<User>, [i64; 7])>::new().and(Same::<(Id<User>, [i64; 7])>::new().map(|(u, _)| u).select((&bc).opt().and((&vc).opt())))));
    rows(v.into_iter().map(|(_, ((u, a), (b, n)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), avg(a[6], a[5]), V::I(b.unwrap_or(0)), V::I(n.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(DISTINCT p.Id) AS TotalPosts
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName)
// SELECT tu.DisplayName, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, COUNT(rp.PostId) AS QuestionCount, SUM(rp.CommentCount) AS TotalComments, SUM(rp.UpVoteCount) AS TotalUpVotes,
//        SUM(rp.DownVoteCount) AS TotalDownVotes
// FROM TopUsers tu LEFT JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId WHERE rp.RN <= 5 GROUP BY tu.DisplayName, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges ORDER BY tu.DisplayName;
//
// RN reads only base columns, so each owner's five newest questions are picked first and the comment x vote product is driven for those alone.
// TotalPosts is never read.
fn q8486(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).with(owner_user.select(rich)).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let dc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let gsb = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt()))
        .fold([0i64; 3], |a, (c, _)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let g = (&tp)
        .group_by(owner_user.select(&db.user.display_name).and(owner_user.select(&gsb)))
        .select((&dc).and(&rp))
        .fold([0i64; 4], |a, (c, v)| [a[0] + 1, a[1] + c, a[2] + v[0], a[3] + v[1]]);
    rows(drain(&g).into_iter().map(|((n, b), a)| {
        let mut f = vec![V::S(n)];
        f.extend(b.map(V::I));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation >= 100 GROUP BY u.Id, u.Reputation),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PopularPosts AS (SELECT rp.Id, rp.Title, rp.Score, rp.CreationDate, ur.Reputation, pc.CommentCount FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId
//     LEFT JOIN PostComments pc ON rp.Id = pc.PostId WHERE rp.rn = 1 AND (ur.TotalBounty > 0 OR ur.Reputation > 200))
// SELECT pp.Title, pp.Score, pp.CreationDate, pp.Reputation, COALESCE(pp.CommentCount, 0) AS CommentCount,
//        CASE WHEN pp.Score >= 100 THEN 'Highly Active' WHEN pp.Score BETWEEN 50 AND 99 THEN 'Moderately Active' ELSE 'Less Active' END AS ActivityLevel
// FROM PopularPosts pp ORDER BY pp.Score DESC, pp.CreationDate DESC LIMIT 10;
fn q700(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ur = db
        .user
        .with((&db.user.reputation).ge(100))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())
        .fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let ok = Ident::<User>::new().and(&ur).filt(|(u, b): (Id<User>, i64)| b > 0 || db.user.reputation.get(u).unwrap() > 200);
    let pc = (&tp).with(owner_user.select(ok)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain(&pc), |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, c)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "score", "created", "rep"]);
        f.extend([V::I(c), V::S(if s >= 100 { "Highly Active" } else if s >= 50 { "Moderately Active" } else { "Less Active" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName),
// FilteredPosts AS (SELECT PostId, Title, Body, Tags, CreationDate, OwnerDisplayName, AnswerCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE Rank = 1)
// SELECT fp.PostId, fp.Title, fp.Body, fp.Tags, fp.CreationDate, fp.OwnerDisplayName, fp.AnswerCount, fp.UpVoteCount, fp.DownVoteCount, COALESCE(ph.EditCount, 0) AS EditCount
// FROM FilteredPosts fp LEFT JOIN (SELECT ph.PostId, COUNT(ph.Id) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId) ph ON fp.PostId = ph.PostId
// ORDER BY fp.CreationDate DESC LIMIT 100;
//
// Rank partitions by the post itself, so it is always 1, and the ORDER BY reads only CreationDate: the hundred newest owned questions
// are picked first and the answer x vote product is driven for those alone.
fn q29085(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(Ident::<Post>::new())), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (x, t)| [a[0] + x.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])));
    let ed = (&tp).group_by(Ident::<Post>::new()).select(edits.opt()).fold(0i64, |n, e| n + e.is_some() as i64);
    rows(drain((&s).and(&ed)).into_iter().map(|(p, (a, e))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(e));
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName AS OwnerDisplayName, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpvoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownvoteCount,
//        COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.OwnerDisplayName, pd.CreationDate, pd.CommentCount, pd.UpvoteCount, pd.DownvoteCount,
//        ROW_NUMBER() OVER (ORDER BY pd.UpvoteCount DESC, pd.CommentCount DESC) AS Rnk FROM PostDetails pd WHERE pd.CommentCount > 0 AND pd.UpvoteCount > 0),
// RecentBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Date > CURRENT_DATE - INTERVAL '6 MONTH' GROUP BY b.UserId)
// SELECT tp.Rnk, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.CommentCount, tp.UpvoteCount, tp.DownvoteCount, rb.BadgeCount
// FROM TopPosts tp LEFT JOIN RecentBadges rb ON rb.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) WHERE tp.Rnk <= 10 ORDER BY tp.Rnk;
fn q29432(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let vs = qs().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&cc).and((&vs).filt(|a| a[0] > 0)));
    let v = top_n(v, |&(p, (c, a))| (Reverse(a[0]), Reverse(c), p), 10);
    let tp = rel(v);
    let since = add_months(current_date(), -6);
    let rb = db.badge.with((&db.badge.date).gt(since)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    type R = (Id<Post>, (i64, [i64; 2]));
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _)| p).select(owner_user.select(&rb).opt()))));
    rows(v.into_iter().map(|(i, ((p, (c, a)), b))| {
        let mut f = vec![V::I(i as i64 + 1)];
        f.extend(post_fields(db, p, &["title", "owner", "created"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), harness::fmt::oint(b)]);
        row(f)
    }))
}

// WITH UserBadgeSummary AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostSummary AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Posts P GROUP BY P.OwnerUserId),
// CombinedMetrics AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(PS.TotalScore, 0) AS TotalScore,
//        COALESCE(PS.QuestionCount, 0) AS QuestionCount, COALESCE(PS.AnswerCount, 0) AS AnswerCount, ROW_NUMBER() OVER (ORDER BY COALESCE(PS.TotalScore, 0) DESC) AS Rank
//     FROM Users U LEFT JOIN UserBadgeSummary UB ON U.Id = UB.UserId LEFT JOIN PostSummary PS ON U.Id = PS.OwnerUserId)
// SELECT UserId, DisplayName, BadgeCount, PostCount, TotalScore, QuestionCount, AnswerCount, Rank FROM CombinedMetrics WHERE BadgeCount > 0 OR PostCount > 0 ORDER BY Rank LIMIT 50;
fn q1946(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { owner_user, score, post_type_id, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(post_type_id)).fold([0i64; 4], |a, (s, t)| [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64]);
    let v = top_n(drain((&ub).and((&ps).opt())), |&(u, (_, p))| (Reverse(p.map_or(0, |a| a[1])), u), 0);
    let cm = rel(v);
    type R = (Id<User>, (i64, Option<[i64; 4]>));
    let kept = (&cm).filt(|(_, (b, p)): R| b > 0 || p.map_or(0, |a| a[0]) > 0);
    let v = top_n(drain(kept), |&(i, _)| i, 50);
    rows(v.into_iter().map(|(i, (u, (b, p)))| {
        let a = p.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, COALESCE(SUM(b.Class), 0) AS TotalBadgePoints, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS Upvotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS Downvotes
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// TopContributors AS (SELECT ur.UserId, ur.TotalBadgePoints, ur.Upvotes - ur.Downvotes AS NetVotes FROM UserReputation ur WHERE ur.TotalBadgePoints > 5)
// SELECT p.Title, p.CreationDate, p.Score, u.DisplayName, COALESCE(tc.TotalBadgePoints, 0) AS BadgePoints, COALESCE(tc.NetVotes, 0) AS NetVotes, COUNT(c.Id) AS CommentCount
// FROM RankedPosts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN TopContributors tc ON u.Id = tc.UserId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.Rank = 1
// GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, tc.TotalBadgePoints, tc.NetVotes ORDER BY p.Score DESC, p.CreationDate DESC LIMIT 10;
fn q4041(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ur = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.select((&ur).filt(|a| a[0] > 5)).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (c, t))| {
        let a = t.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["title", "created", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1] - a[2]), V::I(c)]);
        row(f)
    }))
}

// Rewritten (rewrites/4565.sql): the ROW_NUMBER order tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.Id) AS UserPostRank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score >= 0),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS TotalBadges, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostCommentCounts AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, COALESCE(pcc.CommentCount, 0) AS CommentCount, ub.TotalBadges, ub.HighestBadgeClass,
//        CASE WHEN ub.HighestBadgeClass = 1 THEN 'Gold' WHEN ub.HighestBadgeClass = 2 THEN 'Silver' WHEN ub.HighestBadgeClass = 3 THEN 'Bronze' ELSE 'None' END AS BadgeLevel
//     FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostCommentCounts pcc ON rp.PostId = pcc.PostId WHERE rp.UserPostRank <= 10)
// SELECT PostId, Title, CreationDate, CommentCount, TotalBadges, BadgeLevel FROM FinalResults ORDER BY CreationDate DESC LIMIT 50;
fn q4565(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1).and(score.ge(0))).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp = rel(top);
    let v = top_n(drain(&tp), |&(_, (p, _))| (Reverse(creation_date.get(p).unwrap()), p), 50);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.1 .0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&cc).and(owner_user.select(&ub).opt())).into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::I(c));
        f.extend(match b {
            Some((n, m)) => [V::I(n), V::S(match m { 1 => "Gold", 2 => "Silver", 3 => "Bronze", _ => "None" })],
            None => [V::Null, V::S("None")],
        });
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments, SUM(V.BountyAmount) AS TotalBounty,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalBounty, TotalUpvotes, TotalDownvotes, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC, TotalUpvotes DESC) AS rn FROM UserEngagement)
// SELECT T.UserId, T.DisplayName, T.TotalPosts, T.TotalComments, T.TotalBounty, T.TotalUpvotes, T.TotalDownvotes, P.TotalPosts AS SimilarPostCount, T2.TopUserUpvotes
// FROM TopUsers T JOIN (SELECT U2.Id AS UserId, COUNT(P2.Id) AS TotalPosts FROM Users U2 JOIN Posts P2 ON U2.Id = P2.OwnerUserId GROUP BY U2.Id) P ON T.TotalPosts > P.TotalPosts
// JOIN (SELECT UserId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TopUserUpvotes FROM Votes GROUP BY UserId) T2 ON T.UserId = T2.UserId WHERE T.rn <= 10;
//
// The join to P compares counts, not ids, so it is a cross join filtered on T.TotalPosts > P.TotalPosts.
// Each comment belongs to one post, so over Posts x Comments the comment count is already distinct; the product with the user's votes is
// folded for the vote sums, and the distinct post and comment counts come from a second fold without it.
fn q8481(db: &'static So) -> String {
    let Vote { bounty_amount, vote_type_id, .. } = &db.vote;
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt()).opt().and(votes_by(db).select(bounty_amount.opt().and(vote_type_id)).opt()))
        .fold([0i64; 4], |a, (_, v)| match v {
            Some((b, t)) => [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + (t == 2) as i64, a[3] + (t == 3) as i64],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db).opt()).opt()).fold([0i64; 2], |a, p| match p {
        Some(c) => [a[0], a[1] + c.is_some() as i64],
        None => a,
    });
    let dp = user_distinct_posts(db);
    let v = top_n(drain((&dp).and(&pc).and(&ue)), |&(u, ((n, _), a))| (Reverse(n), Reverse(a[2]), u), 10);
    let tu = rel(v);
    let t2 = db.vote.group_by(&db.vote.user).select(vote_type_id).fold(0i64, |n, t| n + (t == 2) as i64);
    type R = (Id<User>, ((i64, [i64; 2]), [i64; 4]));
    let t = (&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _)| u).select(&t2)));
    let p = db.post.group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let pairs = t.cross(&p).filt(|(((_, ((n, _), _)), _), m): ((R, i64), i64)| n > m);
    rows(drain(pairs).into_iter().map(|(_, (((u, ((n, c), a)), up), m))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c[1]), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(m), V::I(up)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT b.UserId, COUNT(*) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// PostDetails AS (SELECT p.Id AS PostId, p.PostTypeId, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COALESCE(pb.BadgeCount, 0) AS OwnerBadgeCount,
//        COALESCE(pb.BadgeNames, 'None') AS OwnerBadges, EXTRACT(EPOCH FROM TIMESTAMP '2024-10-01 12:34:56' - p.CreationDate) AS AgeInSeconds,
//        ARRAY_LENGTH(string_to_array(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><'), 1) AS TagCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserBadgeCounts pb ON u.Id = pb.UserId WHERE p.PostTypeId = 1),
// AggregatedData AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.OwnerDisplayName, pd.OwnerBadgeCount, pd.OwnerBadges, pd.AgeInSeconds, pd.TagCount,
//        CASE WHEN pd.AgeInSeconds < 3600 THEN 'New' WHEN pd.AgeInSeconds < 86400 THEN 'Moderate' ELSE 'Old' END AS PostAgeCategory FROM PostDetails pd)
// SELECT PostAgeCategory, COUNT(*) AS PostCount, AVG(ViewCount) AS AverageViews, AVG(OwnerBadgeCount) AS AverageBadges FROM AggregatedData GROUP BY PostAgeCategory ORDER BY PostAgeCategory DESC;
//
// The badge names and the tag count are never read.
fn q26962(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let cat = creation_date.map(move |d| {
        let age = secs(t0 - d);
        if age < 3600.0 { "New" } else if age < 86400.0 { "Moderate" } else { "Old" }
    });
    let g = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(cat)
        .select(view_count.opt().and(owner_user.select(&bc).opt()))
        .fold([0i64; 4], |a, (w, b)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + b.unwrap_or(0)]);
    rows(drain(&g).into_iter().map(|(c, a)| row(vec![V::S(c), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS RankByTag, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1),
// TopRankedPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.RankByTag <= 5),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionsAsked, COUNT(DISTINCT a.Id) AS AnswersProvided, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id AND p.PostTypeId = 1 LEFT JOIN Posts a ON a.OwnerUserId = u.Id AND a.PostTypeId = 2 LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.DisplayName),
// CombinedData AS (SELECT trp.Title, trp.Body, trp.CreationDate, trp.Score, ua.DisplayName AS UserDisplayName, ua.QuestionsAsked, ua.AnswersProvided, ua.UpVotesReceived
//     FROM TopRankedPosts trp JOIN Users u ON trp.OwnerUserId = u.Id JOIN UserActivity ua ON u.Id = ua.UserId)
// SELECT Title, Body, CreationDate, Score, UserDisplayName, QuestionsAsked, AnswersProvided, UpVotesReceived FROM CombinedData ORDER BY Score DESC, CreationDate DESC LIMIT 10;
//
// UserActivity has a row for every user, so the joins only drop the ownerless posts; the ten rows are picked first and the
// questions x answers x votes product is driven for their owners alone.
fn q27501(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, creation_date, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(tags_str.opt())), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = top_n(drain((&tp).select(owner_user)), |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    let sel = rel(v);
    let owners: MatSet<Id<User>> = (&sel).map(|(_, u)| u).collect();
    let of_type = |t: i64| posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(t)));
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(of_type(1).opt().and(of_type(2).opt()).and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold(0i64, |n, (_, t)| n + (t == Some(2)) as i64);
    let qa = (&owners).group_by(Ident::<User>::new()).select(of_type(1).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let aa = (&owners).group_by(Ident::<User>::new()).select(of_type(2).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&sel).select(Same::<(Id<Post>, Id<User>)>::new().and(Same::<(Id<Post>, Id<User>)>::new().map(|(_, u)| u).select((&qa).and(&aa).and(&ua))))).into_iter().map(|(_, ((p, u), ((q, a), n)))| {
        let mut f = post_fields(db, p, &["title", "body", "created", "score"]);
        f.extend([user_col(db, u, "name"), V::I(q), V::I(a), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount FROM Badges b WHERE b.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY b.UserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, ub.BadgeCount
//     ORDER BY TotalScore DESC, TotalViews DESC LIMIT 10)
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, ru.DisplayName AS TopUser, ru.BadgeCount, ru.TotalScore, ru.TotalViews
// FROM RankedPosts rp JOIN TopUsers ru ON rp.OwnerDisplayName = ru.DisplayName WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, ru.TotalScore DESC;
fn q8976(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, score, owner_user, post_type_id, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1)).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.badge.with((&db.badge.date).ge(add_years(t0, -1))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let tu = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)],
            None => a,
        });
    let v = top_n(drain(&tu), |&(u, a)| (a[0] == 0, Reverse(a[1]), a[2] == 0, Reverse(a[3]), u), 10);
    let tv = rel(v);
    let by_name: HashIdx<Str, (Id<User>, [i64; 4])> = (&tv).map(|(u, _)| u).select(&db.user.display_name).inv().select(&tv).collect();
    type R = (Id<User>, [i64; 4]);
    let mut v = drain((&tp).select(owner_user.select(&db.user.display_name).select(&by_name).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _)| u).select((&ub).opt())))));
    v.sort_by_key(|&(p, ((_, a), _))| (Reverse(score.get(p).unwrap()), Reverse(a[1])));
    rows(v.into_iter().map(|(p, ((u, a), b))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([user_col(db, u, "name"), V::I(b.unwrap_or(0)), nullable(a[1], a[0]), nullable(a[3], a[2])]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(p.Score) AS TotalScore, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalScore, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStatistics)
// SELECT tu.UserId, tu.DisplayName, tu.TotalPosts, tu.Questions, tu.Answers, tu.TotalScore, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, ph.PostHistoryTypeId, COUNT(ph.Id) AS HistoryCount
// FROM TopUsers tu LEFT JOIN PostHistory ph ON tu.UserId = ph.UserId WHERE tu.ScoreRank <= 10
// GROUP BY tu.UserId, tu.DisplayName, tu.TotalPosts, tu.Questions, tu.Answers, tu.TotalScore, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, ph.PostHistoryTypeId ORDER BY tu.TotalScore DESC, tu.UserId;
fn q7638(db: &'static So) -> String {
    let ub = user_posts_badges(db);
    let Post { post_type_id, .. } = &db.post;
    let qa = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64]);
    let dp = user_distinct_posts(db);
    let v = ranked(drain((&ub).and(&qa).and(&dp)), |&(_, ((a, _), _))| (a[0] == 0, Reverse(a[1])), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, x), _)| (u, x)).collect());
    type R = (Id<User>, (([i64; 8], [i64; 2]), i64));
    let ph_by: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let base = (&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _)| u).select((&ph_by).select(&db.post_history.post_history_type_id).opt())));
    let g = base.group_by(Same::<(R, Option<i64>)>::new()).select(Same::<(R, Option<i64>)>::new()).fold(0i64, |n, (_, t)| n + t.is_some() as i64);
    rows(drain(&g).into_iter().map(|(((u, ((a, q), n)), t), c)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(q[0]), V::I(q[1]), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4]), harness::fmt::oint(t), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Posts a ON a.ParentId = p.Id AND a.PostTypeId = 2 LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Tags, p.CreationDate, p.Score),
// AggregateData AS (SELECT Tags, COUNT(PostId) AS PostCount, SUM(CommentCount) AS TotalComments, SUM(AnswerCount) AS TotalAnswers, SUM(UpVotes) AS TotalUpVotes, SUM(DownVotes) AS TotalDownVotes
//     FROM RankedPosts GROUP BY Tags),
// FinalResults AS (SELECT Tags, PostCount, TotalComments, TotalAnswers, TotalUpVotes, TotalDownVotes, (TotalUpVotes - TotalDownVotes) AS NetVotes, RANK() OVER (ORDER BY PostCount DESC) AS TagsRank FROM AggregateData)
// SELECT Tags, PostCount, TotalComments, TotalAnswers, TotalUpVotes, TotalDownVotes, NetVotes, TagsRank FROM FinalResults WHERE TagsRank <= 5 ORDER BY TagsRank;
//
// Rank is never read.
fn q25276(db: &'static So) -> String {
    let qs = || db.post.with((&db.post.post_type_id).eq(1));
    let vs = qs()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = qs().group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let g = qs().group_by((&db.post.tags_str).opt()).select((&cc).and(&ac).and(&vs)).fold([0i64; 5], |a, ((c, n), v)| [a[0] + 1, a[1] + c, a[2] + n, a[3] + v[0], a[4] + v[1]]);
    let v = ranked(drain(&g), |&(_, a)| Reverse(a[0]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 5).map(|((t, a), r)| {
        let mut f = vec![harness::fmt::ostr(t)];
        f.extend(a.map(V::I));
        f.extend([V::I(a[3] - a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore, AVG(u.Reputation) AS AverageUserReputation
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, TotalViews, TotalScore, AverageUserReputation, RANK() OVER (ORDER BY PostCount DESC, TotalViews DESC, TotalScore DESC) AS TagRank FROM TagStatistics),
// MostActiveUsers AS (SELECT u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionsAnswered, SUM(p.ViewCount) AS TotalViewsOnAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvotesReceived
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2) GROUP BY u.DisplayName ORDER BY QuestionsAnswered DESC LIMIT 10)
// SELECT t.TagName, t.PostCount, t.TotalViews, t.TotalScore, t.AverageUserReputation, au.DisplayName AS MostActiveUser, au.QuestionsAnswered, au.TotalViewsOnAnswers, au.UpvotesReceived
// FROM TopTags t JOIN MostActiveUsers au ON t.TagRank = 1 WHERE t.PostCount > 0 ORDER BY t.TagRank;
//
// The ON clause names only t, so the top tag is crossed with the active users.
fn q28766(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let Post { post_type_id, view_count, score, owner_user, .. } = &db.post;
    type P = (Id<Post>, Id<Tag>);
    let ts = (&lt)
        .with(Same::<P>::new().map(|(p, _)| p).select(post_type_id.eq(1)))
        .group_by(Same::<P>::new().map(|(_, t)| t))
        .select(Same::<P>::new().map(|(p, _)| p).select(view_count.opt().and(score).and(owner_user.select(&db.user.reputation).opt())))
        .fold([0i64; 6], |a, ((w, s), r)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)]);
    let v = ranked(drain(&ts), |&(_, a)| (Reverse(a[0]), a[1] == 0, Reverse(a[2]), Reverse(a[3])), false);
    let tt = rel(v.into_iter().take_while(|x| x.1 <= 1).map(|x| x.0).collect());
    let ans = || db.post.with(post_type_id.eq(2)).with(owner_user);
    let name = owner_user.select(&db.user.display_name);
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let mu = ans().group_by(&name).select(view_count.opt().and(up.opt())).fold([0i64; 3], |a, (w, v)| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + v.is_some() as i64]);
    let qa = ans().group_by(&name).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = top_n(drain((&qa).and(&mu)), |&(n, (q, _))| (Reverse(q), n), 10);
    let au = rel(top);
    let mut v = Vec::new();
    (&tt).cross(&au).drive(|_, ((t, a), (n, (q, m)))| v.push((t, a, n, q, m)));
    rows(v.into_iter().map(|(t, a, n, q, m)| {
        row(vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), avg(a[5], a[4]), V::S(n), V::I(q), nullable(m[1], m[0]), V::I(m[2])])
    }))
}

// WITH RECURSIVE TopPosts AS (SELECT P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate, P.OwnerUserId, COALESCE(U.DisplayName, 'Community User') AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS rn FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1),
// PostVotes AS (SELECT V.PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount,
//        COUNT(CASE WHEN V.VoteTypeId = 6 THEN 1 END) AS CloseVoteCount FROM Votes V GROUP BY V.PostId),
// PostHistoryAggregate AS (SELECT PH.PostId, COUNT(*) FILTER (WHERE PH.PostHistoryTypeId IN (10, 11)) AS CloseActions, COUNT(*) FILTER (WHERE PH.PostHistoryTypeId = 24) AS SuggestedEdits
//     FROM PostHistory PH GROUP BY PH.PostId)
// SELECT TP.Id AS PostId, TP.Title, TP.Score, TP.ViewCount, TP.CreationDate, TP.OwnerDisplayName, COALESCE(PV.UpVoteCount, 0) AS UpVoteCount, COALESCE(PV.DownVoteCount, 0) AS DownVoteCount,
//        COALESCE(PH.CloseActions, 0) AS CloseActions, COALESCE(PH.SuggestedEdits, 0) AS SuggestedEdits
// FROM TopPosts TP LEFT JOIN PostVotes PV ON TP.Id = PV.PostId LEFT JOIN PostHistoryAggregate PH ON TP.Id = PH.PostId WHERE TP.rn <= 10 ORDER BY TP.Score DESC, TP.ViewCount DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q34763(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ph = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0] + matches!(t, Some(10 | 11)) as i64, a[1] + (t == Some(24)) as i64]
    });
    rows(drain((&pv).and(&ph)).into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.push(V::S(owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(h[0]), V::I(h[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.OwnerUserId IS NOT NULL AND p.PostTypeId IN (1, 2)),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//        SUM(p.Score) AS TotalScore, DENSE_RANK() OVER (ORDER BY SUM(p.Score) DESC) AS UserRank FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// RecentlyEditedPosts AS (SELECT p.Id AS PostId, p.Title, MAX(ph.CreationDate) AS LastEditDate FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title)
// SELECT tu.UserId, tu.DisplayName, tu.QuestionsCount, tu.AnswersCount, tu.TotalScore, rp.PostId, rp.Title AS PostTitle, rp.CreationDate AS PostCreationDate, rp.Score AS PostScore, rp.ViewCount,
//        rp.AnswerCount, rp.CommentCount, re.LastEditDate AS PostLastEditDate
// FROM TopUsers tu JOIN RankedPosts rp ON tu.UserId = rp.PostId LEFT JOIN RecentlyEditedPosts re ON rp.PostId = re.PostId WHERE tu.UserRank <= 10 AND rp.PostRank <= 5
// ORDER BY tu.TotalScore DESC, rp.Score DESC;
//
// The join compares a user id with a post id, so it goes through the raw ids.
fn q6973(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, origid, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.is_in([1, 2])).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_id: HashIdx<i64, Id<Post>> = (&rp).select(origid).inv().collect();
    let tu = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 3], |a, (t, s)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s]);
    let v = ranked(drain(&tu), |&(_, a)| Reverse(a[2]), true);
    let tv = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let top: HashIdx<Id<User>, (Id<User>, [i64; 3])> = (&tv).map(|(u, _)| u).inv().select(&tv).collect();
    let le = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let mut v = drain((&top).and((&db.user.origid).select(&by_id).select(Ident::<Post>::new().and((&le).opt()))));
    v.sort_by_key(|&(_, ((_, a), (p, _)))| (Reverse(a[2]), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(u, ((_, a), (p, e)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]));
        f.push(harness::fmt::ots(e));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS AuthorDisplayName, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= '2023-01-01' AND p.Score > 0),
// HistoricalVotes AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS VoteCount, MIN(ph.CreationDate) AS FirstVoteDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12, 13)
//     GROUP BY ph.PostId, ph.PostHistoryTypeId),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AuthorDisplayName, rp.CommentCount, hv.VoteCount AS HistoricalVoteCount, hv.FirstVoteDate
//     FROM RankedPosts rp LEFT JOIN HistoricalVotes hv ON rp.Id = hv.PostId WHERE rp.PostRank <= 10)
// SELECT tp.Id, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AuthorDisplayName, COALESCE(tp.CommentCount, 0) AS TotalComments, COALESCE(tp.HistoricalVoteCount, 0) AS HistoricalVoteCount,
//        COALESCE(EXTRACT(EPOCH FROM tp.FirstVoteDate), 0) AS FirstVoteTimeInSeconds FROM TopPosts tp ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q1049(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0)).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let hv = db.post_history.with(post_history_type_id.is_in([10, 11, 12, 13])).group_by(post.and(post_history_type_id)).select(hd).fold((0i64, i64::MAX), |(n, m), d| (n + 1, m.min(d)));
    let hvv = rel(drain(&hv));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, i64), (i64, i64))> = (&hvv).map(|((p, _), _)| p).inv().select(&hvv).collect();
    rows(drain((&cc).and((&by_post).map(|(_, x)| x).opt())).into_iter().map(|(p, (c, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.push(V::I(c));
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::F(secs(m))],
            None => [V::I(0), V::F(0.0)],
        });
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT v.UserId) AS VoterCount, p.CreationDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.CommentCount, ps.UpVotes, ps.DownVotes, ps.VoterCount, ROW_NUMBER() OVER (ORDER BY ps.UpVotes DESC, ps.CommentCount DESC) AS Rank FROM PostStats ps)
// SELECT t.UserId, t.DisplayName, t.UpVotes AS UserUpVotes, t.DownVotes AS UserDownVotes, p.Title AS PostTitle, p.CommentCount AS PostCommentCount, p.UpVotes AS PostUpVotes, p.DownVotes AS PostDownVotes
// FROM UserVoteStats t JOIN TopPosts p ON t.UpVotes > 0 OR t.DownVotes > 0 WHERE t.TotalVotes > 10 AND p.Rank <= 10 ORDER BY t.UpVotes DESC, p.UpVotes DESC;
//
// The ON clause names only t, so users and top posts are crossed. VoterCount is never read.
fn q5183(db: &'static So) -> String {
    let Vote { user, vote_type_id, .. } = &db.vote;
    let uv = db.vote.group_by(user).select(vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tp = rel(top_n(drain(&ps), |&(p, a)| (Reverse(a[1]), Reverse(a[0]), p), 10));
    let users: MatSet<Id<User>> = db.user.with((&uv).filt(|a| a[2] > 10 && (a[0] > 0 || a[1] > 0))).collect();
    let mut v = Vec::new();
    (&users).cross(&tp).drive(|(u, _), (_, (p, a))| v.push((u, p, a)));
    rows(v.into_iter().map(|(u, p, a)| {
        let n = uv.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n[0]), V::I(n[1])]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Users U ON p.OwnerUserId = U.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, U.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CreationDate, OwnerDisplayName, ScoreRank, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE ScoreRank <= 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CreationDate, tp.OwnerDisplayName, tp.ScoreRank, tp.CommentCount, tp.UpVotes, tp.DownVotes,
//        CASE WHEN tp.UpVotes > tp.DownVotes THEN 'Positive' WHEN tp.UpVotes < tp.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.CommentCount DESC;
//
// ScoreRank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
fn q5471(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap())), false);
    let v = per_group(v, |&(_, t)| t);
    let tv = rel(v.into_iter().filter(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    let tp: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tv).map(|(p, _)| p).inv().select(&tv).collect();
    let s = (&tp)
        .map(|(p, _)| p)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&tp).and(&s)).into_iter().map(|(p, ((_, r), a))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner"]);
        f.push(V::I(r));
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS RankScore
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') AND P.ViewCount > 100),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName FROM RankedPosts WHERE RankScore <= 5),
// PostStats AS (SELECT PP.PostId, PP.Title, PP.CreationDate, PP.ViewCount, PP.Score, PP.OwnerDisplayName, COUNT(C.Id) AS CommentsCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM TopPosts PP LEFT JOIN Comments C ON PP.PostId = C.PostId LEFT JOIN Votes V ON PP.PostId = V.PostId
//     GROUP BY PP.PostId, PP.Title, PP.CreationDate, PP.ViewCount, PP.Score, PP.OwnerDisplayName)
// SELECT PS.PostId, PS.Title, PS.CreationDate, PS.ViewCount, PS.Score, PS.OwnerDisplayName, PS.CommentsCount, PS.UpVotes, PS.DownVotes, (PS.UpVotes - PS.DownVotes) AS VoteBalance
// FROM PostStats PS ORDER BY PS.Score DESC, PS.ViewCount DESC;
fn q5940(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(view_count.gt(100)).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(a[1] - a[2]));
        row(f)
    }))
}

// WITH UserVotes AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS Downvotes,
//        SUM(CASE WHEN V.VoteTypeId IN (6, 7) THEN 1 ELSE 0 END) AS CloseReopenVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.Id AS PostId, P.Title, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(CASE WHEN H.Id IS NOT NULL THEN 1 END) AS EditCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, MAX(P.CreationDate) AS LastActivityDate
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory H ON P.Id = H.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.CommentCount, PS.EditCount, PS.TotalUpvotes - PS.TotalDownvotes AS NetScore, RANK() OVER (ORDER BY PS.TotalUpvotes DESC) AS Rank FROM PostStats PS)
// SELECT UP.UserId, UP.DisplayName, TP.Title, TP.CommentCount, TP.EditCount, TP.NetScore FROM UserVotes UP JOIN TopPosts TP ON UP.Upvotes > 0 WHERE TP.Rank <= 10 ORDER BY UP.Upvotes DESC, TP.NetScore DESC;
//
// The ON clause names only UP, so users and top posts are crossed.
fn q5724(db: &'static So) -> String {
    let Vote { user, vote_type_id, .. } = &db.vote;
    let uv = db.vote.group_by(user).select(vote_type_id).fold(0i64, |n, t| n + (t == 2) as i64);
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).opt()).and(votes_of(db).select(vote_type_id).opt()))
        .fold([0i64; 4], |a, ((c, h), t)| [a[0] + c.is_some() as i64, a[1] + h.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let v = ranked(drain(&ps), |&(_, a)| Reverse(a[2]), false);
    let tp = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let users: MatSet<Id<User>> = db.user.with((&uv).filt(|n| n > 0)).collect();
    let mut v = Vec::new();
    (&users).cross(&tp).drive(|(u, _), (_, (p, a))| v.push((u, p, a)));
    rows(v.into_iter().map(|(u, p, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2] - a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TopRankedPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.OwnerRank <= 5),
// PostVoteStats AS (SELECT p.Id, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(CASE WHEN v.VoteTypeId = 1 THEN 1 END) AS AcceptedVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// PostComments AS (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId)
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, COALESCE(pvs.UpVotes, 0) AS TotalUpVotes, COALESCE(pvs.DownVotes, 0) AS TotalDownVotes,
//        COALESCE(pvs.AcceptedVotes, 0) AS TotalAcceptedVotes, COALESCE(pc.CommentCount, 0) AS TotalComments, trp.OwnerDisplayName
// FROM TopRankedPosts trp LEFT JOIN PostVoteStats pvs ON trp.PostId = pvs.Id LEFT JOIN PostComments pc ON trp.PostId = pc.PostId ORDER BY trp.CreationDate DESC;
fn q9768(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(1)) as i64]
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&pv).and(&cc)).into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c)]);
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.PostTypeId = 1 AND p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS TotalGoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS TotalSilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS TotalBronzeBadges FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT u.DisplayName, u.TotalPosts, u.TotalGoldBadges, u.TotalSilverBadges, u.TotalBronzeBadges, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.AnswerCount
// FROM UserStats u JOIN RankedPosts rp ON u.UserId = rp.PostId WHERE rp.RN <= 5 ORDER BY u.TotalPosts DESC, rp.ViewCount DESC;
//
// The join compares a user id with a post id, so it goes through the raw ids. RN reads only base columns, so the posts are picked first.
fn q5491(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let hit: MatSet<Id<Post>> = (&tp).with(origid.select(&uid)).collect();
    let rp = (&hit).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(answers_of(db).opt())).fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    let users: MatSet<Id<User>> = (&hit).select(origid.select(&uid)).collect();
    let us = user_posts_badges(db);
    let mut v = drain((&rp).and(origid.select(&uid).select(Ident::<User>::new().with(&users).and(&us))));
    v.sort_by_key(|&(p, (_, (_, a)))| {
        let w = db.post.view_count.get(p);
        (Reverse(a[0]), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend(c.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     GROUP BY U.Id, U.DisplayName),
// PostHistoryStats AS (SELECT PH.UserId, COUNT(PH.Id) AS TotalEdits, SUM(CASE WHEN PH.PostHistoryTypeId IN (24, 10, 11) THEN 1 ELSE 0 END) AS TotalSignificantEdits FROM PostHistory PH GROUP BY PH.UserId),
// RankedUsers AS (SELECT UA.UserId, UA.DisplayName, UA.TotalPosts, UA.TotalQuestions, UA.TotalAnswers, UA.PopularPosts, COALESCE(PHS.TotalEdits, 0) AS TotalEdits,
//        COALESCE(PHS.TotalSignificantEdits, 0) AS TotalSignificantEdits, ROW_NUMBER() OVER (ORDER BY UA.TotalPosts DESC, UA.TotalQuestions DESC) AS Rank
//     FROM UserActivity UA LEFT JOIN PostHistoryStats PHS ON UA.UserId = PHS.UserId)
// SELECT RU.DisplayName, RU.TotalPosts, RU.TotalQuestions, RU.TotalAnswers, RU.PopularPosts, RU.TotalEdits, RU.TotalSignificantEdits,
//        (CASE WHEN RU.TotalAnswers > 5 THEN 'Frequent Answerer' WHEN RU.TotalQuestions > 5 THEN 'Active Questioner' ELSE 'Novice User' END) AS UserType
// FROM RankedUsers RU WHERE RU.Rank <= 10 ORDER BY RU.Rank;
fn q4283(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.map_or(false, |w| w > 100) as i64],
        None => a,
    });
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let phs = db.post_history.group_by(user).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + matches!(t, 24 | 10 | 11) as i64]);
    let v = top_n(drain((&ua).and((&phs).opt())), |&(u, (a, _))| (Reverse(a[0]), Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, (a, h))| {
        let h = h.unwrap_or([0, 0]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([V::I(h[0]), V::I(h[1]), V::S(if a[2] > 5 { "Frequent Answerer" } else if a[1] > 5 { "Active Questioner" } else { "Novice User" })]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation > 1000),
// UserPostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        AVG(p.Score) AS AvgPostScore, SUM(p.ViewCount) AS TotalViews FROM Posts p WHERE p.OwnerUserId IS NOT NULL GROUP BY p.OwnerUserId),
// UserWithBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// FinalStats AS (SELECT R.UserId, R.DisplayName, R.Reputation, us.TotalPosts, us.TotalQuestions, us.TotalAnswers, us.AvgPostScore, us.TotalViews, ub.TotalBadges, R.ReputationRank
//     FROM RankedUsers R JOIN UserPostStats us ON R.UserId = us.OwnerUserId JOIN UserWithBadges ub ON R.UserId = ub.UserId)
// SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, AvgPostScore, TotalViews, TotalBadges, ReputationRank FROM FinalStats
// ORDER BY ReputationRank, TotalPosts DESC FETCH FIRST 100 ROWS ONLY;
fn q7312(db: &'static So) -> String {
    let rr = rel(ranked(drain((&db.user.reputation).gt(1000)), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type R = (Id<User>, i64);
    let v = drain((&rr).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _)| u).select((&ps).and(&ub)))));
    let v = top_n(v, |&(_, ((u, r), (a, _)))| (r, Reverse(a[0]), u), 100);
    rows(v.into_iter().map(|(_, ((u, r), (a, b)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), nullable(a[5], a[4]), V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostClosedCount AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PostWithCloseCount AS (SELECT p.Id AS PostId, COALESCE(pc.CloseCount, 0) AS CloseCount FROM Posts p LEFT JOIN PostClosedCount pc ON p.Id = pc.PostId)
// SELECT up.UserId, up.DisplayName, rp.Id AS PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, up.BadgeCount, up.TotalBounties, pcc.CloseCount
// FROM RankedPosts rp JOIN UserStats up ON rp.Id IN (SELECT AcceptedAnswerId FROM Posts WHERE OwnerUserId = up.UserId) LEFT JOIN PostWithCloseCount pcc ON rp.Id = pcc.PostId
// WHERE rp.rn = 1 ORDER BY up.BadgeCount DESC, rp.Score DESC FETCH FIRST 100 ROWS ONLY;
//
// The IN is a semi-join: a user matches a post once however many of their questions accept it.
fn q2531(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, accepted_answer, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let accepted_by: HashIdx<Id<Post>, Id<Post>> = accepted_answer.inv().collect();
    let pairs: MatSet<(Id<Post>, Id<User>)> = (&rp).select(Ident::<Post>::new().and((&accepted_by).select(owner_user))).collect();
    let users: MatSet<Id<User>> = (&pairs).map(|(_, u)| u).collect();
    let us = (&users)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 2], |a, (b, v)| [a[0] + b.is_some() as i64, a[1] + v.flatten().unwrap_or(0)]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let cc = (&rp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    type R = (Id<Post>, Id<User>);
    let v = drain((&pairs).select(Same::<R>::new().and(Same::<R>::new().map(|(_, u)| u).select(&us)).and(Same::<R>::new().map(|(p, _)| p).select(&cc))));
    let v = top_n(v, |&(_, (((p, u), a), _))| (Reverse(a[0]), Reverse(score.get(p).unwrap()), u, p), 100);
    rows(v.into_iter().map(|(_, (((p, u), a), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserRank, COUNT(v.Id) AS VoteCount, p.OwnerUserId
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
//     GROUP BY p.Id, p.OwnerUserId, p.Title, p.Score, p.CreationDate),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(DISTINCT p.Id) AS PostCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 10),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, tu.DisplayName AS OwnerName, tu.TotalScore AS OwnerTotalScore, tu.PostCount AS OwnerPostCount, rp.VoteCount
//     FROM RankedPosts rp JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId WHERE rp.UserRank <= 5)
// SELECT pd.PostId, pd.Title, pd.Score, pd.CreationDate, pd.OwnerName, pd.OwnerTotalScore, pd.OwnerPostCount, pd.VoteCount FROM PostDetails pd ORDER BY pd.Score DESC, pd.CreationDate DESC;
fn q7191(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let top = top_per(drain(recent().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tu = recent().group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + s, a[1] + 1]);
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let vc = (&tp).with(owner_user.select((&tu).filt(|a| a[1] > 10))).group_by(Ident::<Post>::new()).select(up.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&vc).and(owner_user.select(Ident::<User>::new().and(&tu)))).into_iter().map(|(p, (n, (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(v.BountyAmount) OVER (PARTITION BY p.Id) AS TotalBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerUserId, Rank, CommentCount, TotalBounty FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.PostId, tp.Title, U.DisplayName AS OwnerDisplayName, tp.Score, tp.ViewCount, tp.CommentCount, tp.TotalBounty, COALESCE(ba.Name, 'No Badge') AS RecentBadge, COUNT(ph.Id) AS EditHistoryCount
// FROM TopPosts tp LEFT JOIN Users U ON tp.OwnerUserId = U.Id LEFT JOIN Badges ba ON U.Id = ba.UserId AND ba.Date = (SELECT MAX(Date) FROM Badges WHERE UserId = U.Id)
// LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId WHERE U.Reputation > 1000
// GROUP BY tp.PostId, tp.Title, U.DisplayName, tp.Score, tp.ViewCount, tp.CommentCount, tp.TotalBounty, ba.Name ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// There is no GROUP BY in RankedPosts, so its rows are the post x comment x vote rows and Rank numbers those. The rows of one post
// tie on the window order, so which of them get Rank <= 10 is arbitrary; the port takes them by comment and vote id.
fn q33547(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let since = add_years(current_date(), -1);
    let recent = || db.post.with(creation_date.ge(since).and(score.gt(0)));
    type J = ((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>);
    let joined: MatSet<J> = recent().select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt())).collect();
    let pw = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 3], |a, (c, b)| {
        let b = b.flatten();
        [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
    });
    let v = drain((&joined).select(Same::<J>::new().map(|((p, _), _)| p).select(post_type_id)));
    let top = top_per(v, |&(_, t)| t, |&(((p, c), x), _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, c, x), 10, false);
    let tp = rel(top.into_iter().map(|x| x.0 .0 .0).collect());
    let Badge { user, date, name, .. } = &db.badge;
    let md = db.badge.group_by(user).select(date).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<User>, i64), Id<Badge>> = db.badge.select(user.and(date)).inv().collect();
    let latest = Ident::<User>::new().and(&md).select(&at).select(name);
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let base = (&tp).select(Ident::<Post>::new().and(owner_user.select(rich.select(latest.opt()))).and(history_of(db).opt()));
    type R = ((Id<Post>, Option<Str>), Option<Id<PostHistory>>);
    let g = base.group_by(Same::<R>::new().map(|(k, _)| k)).select(Same::<R>::new()).fold(0i64, |n, (_, h)| n + h.is_some() as i64);
    rows(drain((&g).and(Same::<(Id<Post>, Option<Str>)>::new().map(|(p, _)| p).select(&pw))).into_iter().map(|((p, b), (n, a))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::S(b.unwrap_or("No Badge")), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRanking FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes FROM Votes v GROUP BY v.PostId),
// PostHistoryDetails AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastClosedDate, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS LastReopenedDate,
//        COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, COALESCE(rv.Upvotes, 0) AS Upvotes, COALESCE(rv.Downvotes, 0) AS Downvotes, COALESCE(rp.UserPostRanking, 0) AS UserPostRanking,
//        COALESCE(phd.CloseCount, 0) AS CloseCount, phd.LastClosedDate, phd.LastReopenedDate
// FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId WHERE rp.UserPostRanking <= 5 ORDER BY rp.CreationDate DESC;
fn q34192(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let v = ranked(top, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false);
    let v = per_group(v, |&(_, u)| u);
    let rp = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let idx: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let tp = || (&idx).map(|(p, _)| p);
    let pv = tp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = tp().group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd)).opt()).fold((false, 0i64, i64::MIN, i64::MIN), |(any, n, c, r), h| match h {
        Some((t, d)) => (true, n + (t == 10) as i64, if t == 10 { c.max(d) } else { c }, if t == 11 { r.max(d) } else { r }),
        None => (any, n, c, r),
    });
    rows(drain((&idx).and(&pv).and(&ph)).into_iter().map(|(p, (((_, r), a), (_, n, c, o)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(r), V::I(n), tmax(c), tmax(o)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS Author, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Author FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostMetrics AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.Author, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(b.BadgeCount, 0) AS BadgeCount
//     FROM TopPosts tp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON tp.PostId = c.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON tp.Author = (SELECT DisplayName FROM Users WHERE Id = b.UserId))
// SELECT pm.Title, pm.CreationDate, pm.Score, pm.ViewCount, pm.CommentCount, pm.BadgeCount FROM PostMetrics pm ORDER BY pm.Score DESC, pm.ViewCount DESC;
//
// The badge join is on the display name, so an author shares the badge rows of every user with the same name.
fn q5362(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let bv = rel(drain(&bc));
    let by_name: HashIdx<Str, (Id<User>, i64)> = (&bv).map(|(u, _)| u).select(&db.user.display_name).inv().select(&bv).collect();
    rows(drain((&cc).and(owner_user.select(&db.user.display_name).select(&by_name).opt())).into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(b.map_or(0, |(_, n)| n))]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// PostActivity AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore, AVG(P.AnswerCount) AS AvgAnswerCount FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// UserEngagement AS (SELECT U.UserId, U.DisplayName, U.Reputation, U.CreationDate, COALESCE(PA.PostCount, 0) AS PostCount, COALESCE(PA.TotalViews, 0) AS TotalViews, COALESCE(PA.TotalScore, 0) AS TotalScore,
//        COALESCE(PA.AvgAnswerCount, 0) AS AvgAnswerCount, U.BadgeCount FROM UserReputation U LEFT JOIN PostActivity PA ON U.UserId = PA.OwnerUserId),
// FinalReport AS (SELECT U.DisplayName, U.Reputation, U.BadgeCount, U.PostCount, U.TotalViews, U.TotalScore, U.AvgAnswerCount, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM UserEngagement U)
// SELECT DisplayName, Reputation, BadgeCount, PostCount, TotalViews, TotalScore, AvgAnswerCount, ReputationRank FROM FinalReport WHERE ReputationRank <= 100 ORDER BY Reputation DESC;
fn q6463(db: &'static So) -> String {
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 100).map(|((u, _), r)| (u, r)).collect());
    let top: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let bc = (&top).map(|(u, _)| u).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { owner_user, creation_date, view_count, score, answer_count, .. } = &db.post;
    let pa = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(view_count.opt().and(score).and(answer_count.opt())).fold([0i64; 5], |a, ((w, s), n)| {
        [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + n.is_some() as i64, a[4] + n.unwrap_or(0)]
    });
    rows(drain((&top).and(&bc).and((&pa).opt())).into_iter().map(|(u, (((_, r), b), p))| {
        let a = p.unwrap_or([0; 5]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), if a[3] == 0 { V::F(0.0) } else { avg(a[4], a[3]) }, V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ut.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, p.PostTypeId
//     FROM Posts p JOIN Users ut ON p.OwnerUserId = ut.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopQuestions AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerReputation, rp.PostTypeId FROM RankedPosts rp WHERE rp.Rank <= 10 AND rp.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT rp.PostId) AS QuestionsCreated, SUM(rp.Score) AS TotalScore, SUM(rp.ViewCount) AS TotalViews, AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN RankedPosts rp ON p.Id = rp.PostId GROUP BY u.Id, u.DisplayName)
// SELECT us.DisplayName, us.QuestionsCreated, us.TotalScore, us.TotalViews, us.AvgReputation, tq.Title AS TopQuestionTitle, tq.Score AS TopQuestionScore, tq.ViewCount AS TopQuestionViews
// FROM UserStats us LEFT JOIN TopQuestions tq ON us.UserId = tq.OwnerReputation ORDER BY us.QuestionsCreated DESC, us.TotalScore DESC;
//
// The last join compares a user id with a reputation, so it goes through the raw id. AVG(u.Reputation) is over one user's rows, so it is the reputation.
fn q5917(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let top = top_per(drain(recent().select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tq: MatSet<Id<Post>> = rel(top).filt(|(_, t): (Id<Post>, i64)| t == 1).map(|(p, _)| p).collect();
    let by_rep: HashIdx<i64, Id<Post>> = (&tq).select(owner_user.select(&db.user.reputation)).inv().collect();
    let rp = || posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let us = db.user.group_by(Ident::<User>::new()).select(rp().select(score.and(view_count.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)],
        None => a,
    });
    rows(drain((&us).and((&db.user.origid).select(&by_rep).opt())).into_iter().map(|(u, (a, q))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), nullable(a[1], a[0]), nullable(a[3], a[2]), V::F(db.user.reputation.get(u).unwrap() as f64)];
        f.extend(match q {
            Some(p) => post_fields(db, p, &["title", "score", "views"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges,
//        COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostScoreCTE AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(P.Score) AS TotalScore, AVG(P.Score) AS AverageScore FROM Posts P
//     WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(P.TotalPosts, 0) AS TotalPosts, COALESCE(P.TotalScore, 0) AS TotalScore, COALESCE(P.AverageScore, 0) AS AverageScore,
//        COALESCE(B.GoldBadges, 0) AS GoldBadges, COALESCE(B.SilverBadges, 0) AS SilverBadges, COALESCE(B.BronzeBadges, 0) AS BronzeBadges
//     FROM Users U LEFT JOIN PostScoreCTE P ON U.Id = P.OwnerUserId LEFT JOIN UserBadges B ON U.Id = B.UserId)
// SELECT T.UserId, T.DisplayName, T.TotalPosts, T.TotalScore, T.AverageScore, T.GoldBadges, T.SilverBadges, T.BronzeBadges, RANK() OVER (ORDER BY T.TotalScore DESC) AS ScoreRank
// FROM TopUsers T WHERE T.TotalPosts > 0 ORDER BY T.TotalScore DESC, T.DisplayName ASC LIMIT 10;
fn q2083(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let ps = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = ranked(drain((&ps).and(&ub)), |&(_, (a, _))| Reverse(a[1]), false);
    let v = top_n(v, |&((u, (a, _)), _)| (Reverse(a[1]), db.user.display_name.get(u).unwrap(), u), 10);
    rows(v.into_iter().map(|((u, (a, b)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[1], a[0])]);
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserScoreStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score >= 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts, AVG(p.Score) AS AverageScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopContributors AS (SELECT UserId, DisplayName, Reputation, PostCount, PositivePosts, NegativePosts, PopularPosts, AverageScore, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserScoreStats),
// BadgeStats AS (SELECT userId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges GROUP BY userId)
// SELECT t.UserId, t.DisplayName, t.Reputation, t.PostCount, t.PositivePosts, t.NegativePosts, t.PopularPosts, t.AverageScore, b.BadgeCount, b.GoldBadges, b.SilverBadges, b.BronzeBadges
// FROM TopContributors t LEFT JOIN BadgeStats b ON t.UserId = b.userId WHERE t.ReputationRank <= 10 ORDER BY t.Reputation DESC;
fn q9996(db: &'static So) -> String {
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { score, view_count, .. } = &db.post;
    let us = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + (s >= 0) as i64, a[2] + (s < 0) as i64, a[3] + w.map_or(false, |w| w > 100) as i64, a[4] + s],
        None => a,
    });
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    rows(drain((&us).and((&bs).opt())).into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0])]);
        match b {
            Some(b) => f.extend(b.map(V::I)),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(u.DisplayName, 'Community') AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE PostRank <= 10)
// SELECT tr.PostId, tr.Title, tr.CreationDate, tr.ViewCount, tr.Score, tr.OwnerDisplayName, tr.CommentCount, tr.UpVotes, tr.DownVotes, pt.Name AS PostType, COALESCE(b.Name, 'No badge') AS MostRecentBadge
// FROM TopRankedPosts tr LEFT JOIN PostTypes pt ON pt.Id = (SELECT PostTypeId FROM Posts WHERE Id = tr.PostId)
// LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tr.PostId) AND b.Date = (SELECT MAX(Date) FROM Badges WHERE UserId = b.UserId)
// ORDER BY tr.Score DESC, tr.ViewCount DESC;
//
// PostRank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
fn q8575(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.is_in([1, 2])).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let Badge { user, date, name, .. } = &db.badge;
    let md = db.badge.group_by(user).select(date).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<User>, i64), Id<Badge>> = db.badge.select(user.and(date)).inv().collect();
    let latest = Ident::<User>::new().and(&md).select(&at).select(name);
    rows(drain((&s).and(owner_user.select(latest).opt())).into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(V::S(owner_user.get(p).map_or("Community", |u| db.user.display_name.get(u).unwrap())));
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["type"]));
        f.push(V::S(b.unwrap_or("No badge")));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(u.Reputation) AS AvgReputation FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// RankedUserStats AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AvgReputation, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats)
// SELECT u.UserId, u.DisplayName, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.AvgReputation,
//        CASE WHEN u.TotalPosts > 50 THEN 'High Activity' WHEN u.TotalPosts BETWEEN 20 AND 50 THEN 'Moderate Activity' ELSE 'Low Activity' END AS ActivityLevel,
//        (SELECT COUNT(*) FROM Votes v WHERE v.UserId = u.UserId AND v.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month') AS RecentVotes
// FROM RankedUserStats u WHERE u.TotalQuestions > 5 AND u.TotalAnswers > 5 AND NOT EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = u.UserId AND b.Class = 1)
// ORDER BY u.PostRank, u.AvgReputation DESC LIMIT 10;
fn q4217(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ups = db.user.with((&db.user.creation_date).gt(add_years(t0, -1))).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let v = ranked(drain(&ups), |&(_, a)| Reverse(a[0]), false);
    let rk = rel(v);
    let gold: MatSet<Id<User>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).collect();
    let rv = db.vote.with((&db.vote.creation_date).gt(add_months(t0, -1))).group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    type R = ((Id<User>, [i64; 3]), i64);
    let kept = (&rk).filt(|((_, a), _): R| a[1] > 5 && a[2] > 5).select(Same::<R>::new().and(Same::<R>::new().map(|((u, _), _)| u).select(Ident::<User>::new().minus(&gold).and((&rv).opt()))));
    let v = top_n(drain(kept), |&(_, (((u, _), r), _))| (r, Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(_, (((u, a), _), (_, n)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(db.user.reputation.get(u).unwrap() as f64)]);
        f.push(V::S(if a[0] > 50 { "High Activity" } else if a[0] >= 20 { "Moderate Activity" } else { "Low Activity" }));
        f.push(V::I(n.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserVoteStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, COALESCE(ph.UserId, 0) AS LastEditedBy, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.CreationDate = (SELECT MAX(ph2.CreationDate) FROM PostHistory ph2 WHERE ph2.PostId = p.Id)),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.AnswerCount, ps.CommentCount, uv.TotalVotes, uv.UpVotes, uv.DownVotes, ps.LastEditedBy
//     FROM PostStatistics ps JOIN UserVoteStatistics uv ON ps.LastEditedBy = uv.UserId WHERE ps.PostRank <= 10)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.TotalVotes, tp.UpVotes, tp.DownVotes, u.DisplayName AS LastEditorDisplayName
// FROM TopPosts tp JOIN Users u ON tp.LastEditedBy = u.Id ORDER BY tp.Score DESC;
//
// PostRank numbers post x latest-history rows; the rows of one post tie on CreationDate and are taken by history id.
fn q9340(db: &'static So) -> String {
    let PostHistory { post, creation_date: hd, user_id, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    let v = drain(db.post.select(Ident::<Post>::new().and(&md).select(&at).opt()));
    let v = top_n(v, |&(p, h)| (Reverse(db.post.creation_date.get(p).unwrap()), p, h), 10);
    type R = (Id<Post>, Option<Id<PostHistory>>);
    let ps = rel(v);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let editor = Same::<R>::new().map(move |(_, h): R| h.and_then(|h| user_id.get(h)).unwrap_or(0));
    let v = drain((&ps).select(Same::<R>::new().and(editor.select(&uid).select(Ident::<User>::new().and(&uv)))));
    rows(v.into_iter().map(|(_, ((p, _), (u, a)))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "answers", "comments"]);
        f.extend(a.map(V::I));
        f.push(user_col(db, u, "name"));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
//        MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, LastPostDate,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT Rank, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, LastPostDate FROM RankedUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only Reputation, so the top users are picked first and the posts x votes x badges product is driven for those alone.
fn q5930(db: &'static So) -> String {
    let tu = top_n(drain((&db.user.reputation).gt(1000)), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(tu.into_iter().map(|x| x.0).collect());
    let Post { post_type_id, creation_date, .. } = &db.post;
    let s = (&tu)
        .group_by(Same::<Id<User>>::new())
        .select(posts_of(db).select(post_type_id.and(creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0, 0, 0, 0, 0, 0, 0, i64::MIN], |a, (p, c)| {
            let (t, d, v) = p.map_or((0, i64::MIN, None), |((t, d), v)| (t, d, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + (c == Some(1)) as i64, a[5] + (c == Some(2)) as i64, a[6] + (c == Some(3)) as i64, a[7].max(d)]
        });
    let dp = user_distinct_posts(db);
    rows(drain((&tu).select(Same::<Id<User>>::new().and((&s).and(&dp)))).into_iter().map(|(i, (u, (a, n)))| {
        let mut f = vec![V::I(i as i64 + 1)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a[..7].iter().map(|&x| V::I(x)));
        f.push(tmax(a[7]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, pt.Name),
// TopRankedPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 5),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(v.BountyAmount) AS TotalBounty, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT t.PostId, t.Title, t.OwnerDisplayName, t.CreationDate, t.Score, t.ViewCount, u.DisplayName AS UserName, u.TotalBounty, u.BadgeCount, u.TotalViews
// FROM TopRankedPosts t JOIN UserStats u ON t.OwnerDisplayName = u.DisplayName ORDER BY t.Score DESC, t.ViewCount DESC;
//
// CommentCount is never read. UserStats is driven only for the users whose name an owner of a top post shares.
fn q5857(db: &'static So) -> String {
    let Post { creation_date, score, post_type, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type.select(&db.post_type.name)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let names: MatSet<Str> = (&tp).select(owner_user.select(&db.user.display_name)).collect();
    let by_name: HashIdx<Str, Id<User>> = db.user.select(&db.user.display_name).with(&names).map(|n| n).inv().select(Ident::<User>::new()).collect();
    let users: MatSet<Id<User>> = (&names).select(&by_name).collect();
    let us = (&users)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()).and(posts_of(db).select(view_count.opt()).opt()))
        .fold([0i64; 4], |a, ((b, _), w)| {
            let (b, w) = (b.flatten(), w.flatten());
            [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
        });
    let bc = (&users).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&tp).select(owner_user.select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and(&us).and(&bc))));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, ((u, a), b))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([user_col(db, u, "name"), nullable(a[1], a[0]), V::I(b), nullable(a[3], a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score >= 0 AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserScores AS (SELECT u.Id AS UserId, SUM(p.Score) AS TotalScore, COUNT(p.Id) AS PostCount, AVG(p.ViewCount) AS AvgViewCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.Reputation > 1000 GROUP BY u.Id),
// PostsWithBadge AS (SELECT p.Id AS PostId, p.Title, b.Name AS BadgeName, b.Date AS BadgeDate FROM Posts p LEFT JOIN Badges b ON p.OwnerUserId = b.UserId WHERE b.Class = 1 OR b.Class = 2),
// FinalResults AS (SELECT p.Title, p.Score, u.UserId, u.TotalScore, u.PostCount, u.AvgViewCount, COALESCE(pb.BadgeName, 'No Badge') AS BadgeName
//     FROM RankedPosts p JOIN UserScores u ON p.PostId = u.UserId LEFT JOIN PostsWithBadge pb ON p.PostId = pb.PostId WHERE p.UserPostRank <= 5)
// SELECT Title, Score, UserId, TotalScore, PostCount, AvgViewCount, BadgeName FROM FinalResults ORDER BY TotalScore DESC, Score DESC LIMIT 50;
//
// The join compares a post id with a user id, so it goes through the raw ids.
fn q139(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, origid, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.ge(0)).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uid: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.origid).inv().collect();
    let users: MatSet<Id<User>> = (&tp).select(origid.select(&uid)).collect();
    let us = (&users).group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)],
        None => a,
    });
    let gs = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).is_in([1, 2]))).select(&db.badge.name);
    let v = drain((&tp).select(origid.select(&uid).select(Ident::<User>::new().and(&us)).and(owner_user.select(gs).opt())));
    let v = top_n(v, |&(p, ((u, a), b))| (a[0] == 0, Reverse(a[1]), Reverse(score.get(p).unwrap()), p, u, b), 50);
    rows(v.into_iter().map(|(p, ((u, a), b))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([user_col(db, u, "uid"), nullable(a[1], a[0]), V::I(a[0]), avg(a[3], a[2]), V::S(b.unwrap_or("No Badge"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostsWithBadges AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, COUNT(b.Id) AS BadgeCount
//     FROM TopPosts tp LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount)
// SELECT p.Title, p.OwnerDisplayName, p.CreationDate, p.Score, p.ViewCount, p.BadgeCount,
//        CASE WHEN p.BadgeCount >= 5 THEN 'Gold Contributor' WHEN p.BadgeCount >= 3 THEN 'Silver Contributor' WHEN p.BadgeCount >= 1 THEN 'Bronze Contributor' ELSE 'No Badges' END AS ContributionLevel
// FROM PostsWithBadges p ORDER BY p.Score DESC, p.ViewCount DESC;
fn q9543(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain(&bc).into_iter().map(|(p, b)| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.extend([V::I(b), V::S(if b >= 5 { "Gold Contributor" } else if b >= 3 { "Silver Contributor" } else if b >= 1 { "Bronze Contributor" } else { "No Badges" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, u.Views, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.Views),
// TopUsersPosts AS (SELECT us.DisplayName, us.TotalPosts, us.TotalQuestions, us.TotalAnswers, us.TotalUpVotes, us.TotalDownVotes, RANK() OVER (ORDER BY us.Reputation DESC) AS ReputationRank
//     FROM UserStats us WHERE us.TotalPosts > 0)
// SELECT uup.DisplayName, uup.TotalPosts, uup.TotalQuestions, uup.TotalAnswers, uup.TotalUpVotes, uup.TotalDownVotes, (uup.TotalUpVotes - uup.TotalDownVotes) AS NetVotes,
//        CASE WHEN uup.ReputationRank <= 10 THEN 'Top Contributor' WHEN uup.ReputationRank <= 50 THEN 'Active Contributor' ELSE 'Emerging Contributor' END AS ContributorLevel
// FROM TopUsersPosts uup WHERE uup.ReputationRank <= 100 ORDER BY uup.ReputationRank;
//
// ReputationRank reads only Reputation over the users with a post, so the top users are picked first and the posts x votes product is driven for those alone.
fn q6264(db: &'static So) -> String {
    let dp = user_distinct_posts(db);
    let v = ranked(drain(db.user.with((&db.user.reputation).gt(100)).with((&dp).filt(|n| n > 0)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 100).map(|((u, _), r)| (u, r)).collect());
    let top: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let us = (&top)
        .map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let qa = (&top).map(|(u, _)| u).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64]);
    rows(drain((&top).and(&dp).and(&qa).and(&us)).into_iter().map(|(u, ((((_, r), n), q), a))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(q[0]), V::I(q[1]), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])];
        f.push(V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Active Contributor" } else { "Emerging Contributor" }));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotes, TotalDownvotes, RANK() OVER (ORDER BY TotalPosts DESC, TotalUpvotes DESC) AS UserRank FROM UserPostStats)
// SELECT r.UserId, r.DisplayName, r.TotalPosts, r.TotalQuestions, r.TotalAnswers, r.TotalUpvotes, r.TotalDownvotes,
//        CASE WHEN r.TotalPosts = 0 THEN 0 ELSE ROUND((CAST(r.TotalUpvotes AS decimal) / NULLIF(r.TotalPosts, 0)) * 100, 2) END AS UpvotePercentage,
//        CASE WHEN r.TotalPosts = 0 THEN 0 ELSE ROUND((CAST(r.TotalDownvotes AS decimal) / NULLIF(r.TotalPosts, 0)) * 100, 2) END AS DownvotePercentage, r.UserRank
// FROM RankedUsers r WHERE r.UserRank <= 10 ORDER BY r.UserRank;
fn q7310(db: &'static So) -> String {
    let s = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| (Reverse(a[0]), Reverse(a[3])), false);
    let pct = |x: i64, n: i64| V::F(if n == 0 { 0.0 } else { (x as f64 / n as f64 * 100.0 * 100.0).round() / 100.0 });
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([pct(a[3], a[0]), pct(a[4], a[0]), V::I(r)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("7043", q7043),
    ("9855", q9855),
    ("3932", q3932),
    ("3862", q3862),
    ("6595", q6595),
    ("11459", q11459),
    ("306", q306),
    ("948", q948),
    ("28", q28),
    ("1245", q1245),
    ("4714", q4714),
    ("6290", q6290),
    ("9614", q9614),
    ("30399", q30399),
    ("8834", q8834),
    ("6474", q6474),
    ("6924", q6924),
    ("8821", q8821),
    ("9206", q9206),
    ("5423", q5423),
    ("9472", q9472),
    ("7353", q7353),
    ("7287", q7287),
    ("347", q347),
    ("2604", q2604),
    ("7223", q7223),
    ("7376", q7376),
    ("8217", q8217),
    ("5504", q5504),
    ("9185", q9185),
    ("9517", q9517),
    ("4214", q4214),
    ("6497", q6497),
    ("5994", q5994),
    ("6832", q6832),
    ("74", q74),
    ("9698", q9698),
    ("5852", q5852),
    ("6900", q6900),
    ("8267", q8267),
    ("8988", q8988),
    ("961", q961),
    ("5517", q5517),
    ("5654", q5654),
    ("9986", q9986),
    ("530", q530),
    ("4506", q4506),
    ("4962", q4962),
    ("8918", q8918),
    ("33081", q33081),
    ("2691", q2691),
    ("1407", q1407),
    ("8437", q8437),
    ("9538", q9538),
    ("24632", q24632),
    ("8685", q8685),
    ("1867", q1867),
    ("7469", q7469),
    ("8486", q8486),
    ("700", q700),
    ("29085", q29085),
    ("29432", q29432),
    ("1946", q1946),
    ("4041", q4041),
    ("4565", q4565),
    ("8481", q8481),
    ("26962", q26962),
    ("27501", q27501),
    ("8976", q8976),
    ("7638", q7638),
    ("25276", q25276),
    ("28766", q28766),
    ("34763", q34763),
    ("6973", q6973),
    ("1049", q1049),
    ("5183", q5183),
    ("5471", q5471),
    ("5940", q5940),
    ("5724", q5724),
    ("9768", q9768),
    ("5491", q5491),
    ("4283", q4283),
    ("7312", q7312),
    ("2531", q2531),
    ("7191", q7191),
    ("33547", q33547),
    ("34192", q34192),
    ("5362", q5362),
    ("6463", q6463),
    ("5917", q5917),
    ("2083", q2083),
    ("9996", q9996),
    ("8575", q8575),
    ("4217", q4217),
    ("9340", q9340),
    ("5930", q5930),
    ("5857", q5857),
    ("139", q139),
    ("9543", q9543),
    ("6264", q6264),
    ("7310", q7310),
];
