use harness::prelude::*;
use std::cmp::Reverse;

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers,
//        AVG(COALESCE(p.Score, 0)) AS AverageScore, AVG(COALESCE(p.ViewCount, 0)) AS AverageViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalAcceptedAnswers, AverageScore, AverageViews,
//        RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalAcceptedAnswers, tu.AverageScore, tu.AverageViews, u.Reputation, b.Name AS BadgeName
// FROM TopUsers tu JOIN Users u ON tu.UserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId WHERE tu.PostRank <= 10
// ORDER BY tu.PostRank, u.Reputation DESC, tu.AverageScore DESC;
fn q8271(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, accepted_answer, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(accepted_answer.opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some((((t, s), w), acc)) => [a[0] + 1, a[1] + 1, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + (t == 1 && acc.is_some()) as i64, a[5] + s, a[6] + w.unwrap_or(0)],
            None => [a[0] + 1, a[1], a[2], a[3], a[4], a[5], a[6]],
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[1]), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let v = drain((&top).select((&s).and(badges_of(db).select(&db.badge.name).opt())));
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[5], a[0]), avg(a[6], a[0]), user_col(db, u, "rep")];
        f.push(harness::fmt::ostr(b));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS Rank
//     FROM Posts P WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty, COUNT(DISTINCT V.PostId) AS TotalVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// ClosedPosts AS (SELECT H.PostId, COUNT(DISTINCT H.Id) AS CloseCount FROM PostHistory H WHERE H.PostHistoryTypeId = 10 GROUP BY H.PostId)
// SELECT R.PostId, R.Title, R.CreationDate, R.Score, R.ViewCount, U.DisplayName, U.TotalBounty, U.TotalVotes, COALESCE(C.CloseCount, 0) AS NumberOfClosures
// FROM RankedPosts R JOIN UserStats U ON R.PostId = (SELECT AcceptedAnswerId FROM Posts WHERE Id = R.PostId) LEFT JOIN ClosedPosts C ON R.PostId = C.PostId
// WHERE R.Rank <= 5 ORDER BY R.Score DESC, R.CreationDate DESC;
//
// The ON clause names only R (a post that is its own accepted answer), so the surviving posts are crossed with UserStats.
// Only the rank-5 cut reads the window; the ROW_NUMBER tie order is irrelevant because no post passes the ON test here.
fn q2499(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, accepted_answer, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let own: MatSet<Id<Post>> = (&tp).with(Ident::<Post>::new().and(accepted_answer).filt(|(p, a)| p == a)).collect();
    let Vote { bounty_amount, post_id, .. } = &db.vote;
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(bounty_amount.opt().and(post_id)).opt())
        .fold(0i64, |n, v| n + v.and_then(|(b, _)| b).unwrap_or(0));
    let uv = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(votes_by(db).select(post_id)).count_distinct();
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let cc = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(Ident::<PostHistory>::new()).count_distinct();
    let mut v = drain((&own).select((&cc).opt()).cross((&us).and((&uv).opt())));
    v.sort_by_key(|&((p, _), _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|((p, u), (c, (b, n)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([user_col(db, u, "name"), V::I(b), V::I(n.unwrap_or(0)), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, AVG(P.Score) AS AverageScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, UpVotes, DownVotes, AverageScore, ROW_NUMBER() OVER (ORDER BY PostCount DESC, AverageScore DESC) AS Rank FROM UserActivity)
// SELECT T.UserId, T.DisplayName, T.Reputation, T.PostCount, T.UpVotes, T.DownVotes, T.AverageScore, COALESCE(B.Count, 0) AS BadgeCount,
//        CASE WHEN T.Reputation > 1000 THEN 'Experienced' WHEN T.Reputation > 500 THEN 'Moderate' ELSE 'Novice' END AS UserLevel
// FROM TopUsers T LEFT JOIN (SELECT UserId, COUNT(*) AS Count FROM Badges GROUP BY UserId) B ON T.UserId = B.UserId WHERE T.Rank <= 10
// ORDER BY T.AverageScore DESC, T.PostCount DESC;
fn q3(db: &'static So) -> String {
    let score = &db.post.score;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, t)) => [a[0] + 1, a[1] + s, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    let pc = user_distinct_posts(db);
    let mean = |a: [i64; 4]| if a[0] == 0 { None } else { Some(fkey(a[1] as f64 / a[0] as f64)) };
    let v = top_n(drain((&ua).and(&pc)), |&(u, (a, n))| (Reverse(n), mean(a).is_none(), Reverse(mean(a)), u), 10);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bc = (&top).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&top).select((&ua).and(&pc).and((&bc).opt())));
    rows(v.into_iter().map(|(u, ((a, n), b))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[2]), V::I(a[3]), avg(a[1], a[0]), V::I(b.unwrap_or(0))]);
        f.push(V::S(if rep > 1000 { "Experienced" } else if rep > 500 { "Moderate" } else { "Novice" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, COALESCE(SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END), 0) AS PositivePostCount,
//        COALESCE(SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END), 0) AS NegativePostCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, P.Score, P.ViewCount, ROW_NUMBER() OVER (ORDER BY P.Score DESC) AS Rank, P.OwnerUserId
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id)
// SELECT US.UserId, US.Reputation, US.PostCount, US.PositivePostCount, US.NegativePostCount, US.TotalUpVotes, US.TotalDownVotes, TP.PostId, TP.Title, TP.CreationDate,
//        TP.OwnerDisplayName, TP.Score, TP.ViewCount
// FROM UserStats US LEFT JOIN TopPosts TP ON US.UserId = TP.OwnerUserId WHERE TP.Rank <= 10 ORDER BY US.Reputation DESC, TP.Score DESC;
//
// Rank reads only Score, so the ten top posts are picked first and the posts x votes product is driven only for their owners.
fn q13256(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let tp = top_n(drain(db.post.with(owner_user).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(tp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, t)) => [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    let pc = user_distinct_posts(db);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&us).and(&pc))));
    rows(v.into_iter().map(|(p, ((u, a), n))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]));
        row(f)
    }))
}

// WITH PostWithTags AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount,
//        unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT pwt.PostId, pwt.Title, pwt.Body, pwt.CreationDate, pwt.ViewCount, pwt.AnswerCount, pwt.CommentCount, COUNT(*) OVER(PARTITION BY pwt.Tag) AS TagCount
//     FROM PostWithTags pwt WHERE pwt.ViewCount > 100),
// RankedPosts AS (SELECT fp.PostId, fp.Title, fp.Body, fp.CreationDate, fp.ViewCount, fp.AnswerCount, fp.CommentCount, fp.TagCount,
//        ROW_NUMBER() OVER(ORDER BY fp.TagCount DESC, fp.ViewCount DESC) AS Rank FROM FilteredPosts fp)
// SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.TagCount, rp.Rank, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
// FROM RankedPosts rp JOIN Users u ON rp.PostId = u.Id WHERE rp.Rank <= 10 ORDER BY rp.Rank;
//
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q28669(db: &'static So) -> String {
    let Post { post_type_id, view_count, tags_str, origid, .. } = &db.post;
    let pt: MatSet<(Id<Post>, Str)> = db.post.with(post_type_id.eq(1).and(view_count.gt(100))).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list))).collect();
    type R = (Id<Post>, Str);
    let tc = (&pt).group_by(Same::<R>::new().map(|(_, t): R| t)).select(Same::<R>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&pt).select(Same::<R>::new().and(Same::<R>::new().map(|(_, t): R| t).select(&tc))));
    let v = top_n(v, |&(_, ((p, _), n))| (Reverse(n), Reverse(view_count.get(p).unwrap())), 10);
    let r = rel(v.into_iter().enumerate().map(|(i, (_, ((p, _), n)))| (p, n, i as i64 + 1)).collect());
    type T = (Id<Post>, i64, i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&r).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _, _): T| p).select(origid).select(&uidx))));
    rows(v.into_iter().map(|(_, ((p, n, k), u))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "views", "answers", "comments"]);
        f.extend([V::I(n), V::I(k)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RecursiveVoteCounts AS (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY PostId),
// UserReputation AS (SELECT Id AS UserId, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, COALESCE(rv.VoteCount, 0) AS RecentVoteCount, u.DisplayName AS OwnerDisplayName
//     FROM Posts p LEFT JOIN RecursiveVoteCounts rv ON p.Id = rv.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '90 days')
// SELECT rp.Title, rp.RecentVoteCount, ur.Reputation, ur.ReputationRank,
//        CASE WHEN rp.RecentVoteCount = 0 THEN 'No Votes Yet' WHEN rp.RecentVoteCount > 10 THEN 'Popular Post' ELSE 'Moderately Active' END AS ActivityStatus
// FROM RecentPosts rp INNER JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.RecentVoteCount > 0 ORDER BY ur.Reputation DESC, rp.RecentVoteCount DESC LIMIT 20;
fn q31527(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Vote { post, creation_date: vd, .. } = &db.vote;
    let rv = db.vote.with(vd.ge(add_days(t0, -30))).group_by(post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(t0, -90))).select((&rv).and(owner_user.select(&by_user))));
    let v = top_n(v, |&(p, (n, (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), p), 20);
    rows(v.into_iter().map(|(p, (n, (u, r)))| {
        row(vec![title(db, p), V::I(n), user_col(db, u, "rep"), V::I(r), V::S(if n > 10 { "Popular Post" } else { "Moderately Active" })])
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS UpVotes,
//        SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation)
// SELECT T.DisplayName, T.Reputation, T.PostCount, T.UpVotes, T.DownVotes, T.Rank, COALESCE(AVG(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(AVG(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(AVG(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
// FROM TopUsers T LEFT JOIN Badges B ON T.UserId = B.UserId WHERE T.Rank <= 10 GROUP BY T.UserId, T.DisplayName, T.Reputation, T.PostCount, T.UpVotes, T.DownVotes, T.Rank ORDER BY T.Rank;
//
// Rank reads only Reputation, so the top users are picked first and the posts x votes product is driven for them alone.
fn q4581(db: &'static So) -> String {
    let v = ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let top: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let ur = (&top).map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let pc = user_distinct_posts(db);
    let bc = (&top).map(|(u, _)| u).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| {
        [a[0] + 1, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]
    });
    let v = drain((&top).and(&ur).and(&pc).and(&bc));
    rows(v.into_iter().map(|(u, ((((_, r), a), n), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(r), avg(b[1], b[0]), avg(b[2], b[0]), avg(b[3], b[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.Id END) AS UpVoteCount, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 3 THEN v.Id END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId, p.CreationDate),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.*, CASE WHEN tp.UpVoteCount > tp.DownVoteCount THEN 'More Upvotes' WHEN tp.UpVoteCount < tp.DownVoteCount THEN 'More Downvotes' ELSE 'Equal' END AS VoteTrend
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the top posts are picked first; each COUNT(DISTINCT) is its own fold over that post's children.
fn q5149(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&cc).and(&vc));
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(if a[0] > a[1] { "More Upvotes" } else if a[0] < a[1] { "More Downvotes" } else { "Equal" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY COUNT(p.Id) DESC) AS PostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.PostCount, ua.TotalViews, ua.TotalUpVotes - ua.TotalDownVotes AS VoteBalance, RANK() OVER (ORDER BY ua.TotalViews DESC) AS ViewRank
//     FROM UserActivity ua WHERE ua.PostCount > 5)
// SELECT tu.DisplayName, tu.PostCount, tu.TotalViews, tu.VoteBalance,
//        CASE WHEN tu.ViewRank <= 10 THEN 'Top Contributor' WHEN tu.ViewRank <= 50 THEN 'Moderate Contributor' ELSE 'New Contributor' END AS ContributorLevel,
//        COALESCE(tu.TotalViews / NULLIF(tu.PostCount, 0), 0) AS AvgViewsPerPost
// FROM TopUsers tu WHERE tu.VoteBalance > 0 ORDER BY tu.TotalViews DESC LIMIT 20;
fn q3346(db: &'static So) -> String {
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((w, t)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    let v = ranked(drain((&ua).filt(|a| a[0] > 5)), |&(_, a)| Reverse(a[1]), false);
    let v = rel(v.into_iter().map(|((u, a), r)| (u, a, r)).collect());
    type T = (Id<User>, [i64; 4], i64);
    let v = drain((&v).with(Same::<T>::new().filt(|(_, a, _): T| a[2] - a[3] > 0)));
    let v = top_n(v, |&(_, (u, a, _))| (Reverse(a[1]), u), 20);
    rows(v.into_iter().map(|(_, (u, a, r))| {
        let lvl = if r <= 10 { "Top Contributor" } else if r <= 50 { "Moderate Contributor" } else { "New Contributor" };
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2] - a[3]), V::S(lvl), V::F(a[1] as f64 / a[0] as f64)])
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers,
//        COALESCE(SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END), 0) AS AcceptedAnswers, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(C.Id) AS TotalComments, MAX(C.CreationDate) AS LastCommentDate FROM Users U LEFT JOIN Comments C ON U.Id = C.UserId
//     GROUP BY U.Id, U.DisplayName)
// SELECT UPS.UserId, UPS.DisplayName, UPS.Reputation, UPS.TotalPosts, UPS.TotalAnswers, UPS.AcceptedAnswers, UPS.ReputationRank, RA.TotalComments, RA.LastCommentDate
// FROM UserPostStats UPS FULL OUTER JOIN RecentActivity RA ON UPS.UserId = RA.UserId
// WHERE (UPS.Reputation > 100 OR RA.TotalComments > 5) AND (UPS.TotalPosts IS NOT NULL OR RA.TotalComments IS NOT NULL)
// ORDER BY UPS.ReputationRank ASC NULLS LAST, UPS.TotalPosts DESC NULLS LAST;
//
// Both sides of the FULL OUTER JOIN have exactly one row per user (each is Users LEFT JOIN .. GROUP BY U.Id), so it pairs every user with itself.
fn q1189(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(accepted_answer.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, acc)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + acc.is_some() as i64],
        None => a,
    });
    let ra = db.user.group_by(Ident::<User>::new()).select(comments_by(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(100).or((&ra).filt(|(n, _)| n > 5))).select((&ups).and(&ra).and(&rank)));
    rows(v.into_iter().map(|(u, ((a, (n, m)), (_, r)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r), V::I(n), tmax(m)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, P.Score, U.DisplayName AS OwnerDisplayName, COALESCE(COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END), 0) AS UpVoteCount,
//        COALESCE(COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END), 0) AS DownVoteCount, COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount,
//        ROW_NUMBER() OVER (ORDER BY P.Score DESC) AS Rank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId
//     GROUP BY P.Id, P.Title, P.Body, P.CreationDate, P.Score, U.DisplayName),
// TopPosts AS (SELECT PostId, Title, Body, CreationDate, Score, OwnerDisplayName, UpVoteCount, DownVoteCount, CommentCount FROM RankedPosts WHERE Rank <= 10),
// PostHistoryDetails AS (SELECT PH.PostId, MAX(PH.CreationDate) AS LastEditDate, MAX(PHT.Name) AS LastEditType FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
//     GROUP BY PH.PostId)
// SELECT TP.*, PHD.LastEditDate, PHD.LastEditType FROM TopPosts TP LEFT JOIN PostHistoryDetails PHD ON TP.PostId = PHD.PostId ORDER BY TP.Score DESC, TP.CreationDate DESC;
//
// Rank reads only Score, so the ten top posts are picked first and the votes x comments product is driven for those alone.
fn q6374(db: &'static So) -> String {
    let score = &db.post.score;
    let tp = top_n(drain(score), |&(p, s)| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(tp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let phd = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select((&db.post_history.creation_date).and(htype_name(db)))).fold((i64::MIN, ""), |(d, n), (x, y)| (d.max(x), n.max(y)));
    let v = drain((&s).and((&phd).opt()));
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "score", "owner"]);
        f.extend(a.map(V::I));
        f.extend(match h {
            Some((d, n)) => [V::T(d), V::S(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, COUNT(DISTINCT c.Id) AS TotalComments, AVG(COALESCE(v.VoteCount, 0)) AS AvgVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, p.CreationDate, RANK() OVER (ORDER BY COUNT(DISTINCT c.Id) DESC) AS CommentRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN (SELECT PostId, VoteTypeId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId, VoteTypeId) v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.TotalComments, rp.AvgVotes, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.CommentRank <= 10)
// SELECT tp.Title, tp.TotalComments, tp.AvgVotes, tp.UpVotes, tp.DownVotes, u.DisplayName, u.Reputation
// FROM TopPosts tp JOIN Posts p ON tp.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id WHERE u.Reputation > 1000 ORDER BY tp.TotalComments DESC;
//
// CommentRank reads only the distinct comment count, so those questions are picked first and the comments x vote-groups product is driven for them alone.
fn q7597(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let cc = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain(&cc), |&(_, n)| Reverse(n), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let Vote { post, vote_type_id, .. } = &db.vote;
    let vg = db.vote.with(post).group_by(post.and(vote_type_id)).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let vg = rel(drain(&vg));
    let vg_of: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&vg).map(|((p, _), _)| p).inv().select(&vg).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&vg_of).opt()))
        .fold([0i64; 4], |a, (_, g)| match g {
            Some(((_, t), n)) => [a[0] + 1, a[1] + n, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64],
            None => [a[0] + 1, a[1], a[2], a[3]],
        });
    let rich = owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)));
    let v = drain((&s).and(&cc).and(rich));
    rows(v.into_iter().map(|(p, ((a, n), u))| {
        let mut f = vec![title(db, p), V::I(n), avg(a[1], a[0]), V::I(a[2]), V::I(a[3])];
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY u.Id ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.PostRank <= 5),
// PostVotes AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.ViewCount, tp.AnswerCount, pv.UpVotes, pv.DownVotes, tp.Score
// FROM TopPosts tp JOIN PostVotes pv ON tp.PostId = pv.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q7847(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain(&pv);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "views", "answers"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(score.get(p).unwrap())]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.UpVotes, u.DownVotes, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(b.Class) AS TotalBadgeClass,
//        AVG(COALESCE(c.Score, 0)) AS AvgCommentScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation, u.UpVotes, u.DownVotes),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalScore, AnswerCount, QuestionCount, TotalBadgeClass, AvgCommentScore,
//        ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Ranking FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, TotalScore, AnswerCount, QuestionCount, TotalBadgeClass, AvgCommentScore FROM TopUsers WHERE Ranking <= 10 ORDER BY TotalScore DESC;
fn q9165(db: &'static So) -> String {
    let Post { score, post_type_id, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(post_type_id).and(comments_of(db).select(&db.comment.score).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let (s, t, c) = p.map_or((0, 0, 0), |((s, t), c)| (s, t, c.unwrap_or(0)));
            [a[0] + 1, a[1] + s, a[2] + (t == 2) as i64, a[3] + (t == 1) as i64, a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0), a[6] + c]
        });
    let pc = user_distinct_posts(db);
    let v = top_n(drain((&us).and(&pc)), |&(u, (a, _))| (Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4]), avg(a[6], a[0])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, Questions, Answers, CommentCount, RANK() OVER (ORDER BY PostCount DESC) AS UserRank FROM UserActivity)
// SELECT t.UserId, t.DisplayName, t.PostCount, t.Questions, t.Answers, t.CommentCount, bh.Name AS BadgeName, COUNT(DISTINCT ph.Id) AS PostHistoryCount
// FROM TopUsers t LEFT JOIN Badges b ON t.UserId = b.UserId AND b.Class = 1 LEFT JOIN PostHistory ph ON t.UserId = ph.UserId LEFT JOIN PostHistoryTypes bh ON ph.PostHistoryTypeId = bh.Id
// WHERE t.UserRank <= 10 GROUP BY t.UserId, t.DisplayName, t.PostCount, t.Questions, t.Answers, t.CommentCount, bh.Name ORDER BY t.PostCount DESC;
//
// The gold-badge join only repeats rows, which COUNT(DISTINCT ph.Id) undoes and no other output reads, so the groups are the top users' history rows keyed by type name.
fn q7913(db: &'static So) -> String {
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64],
            None => a,
        });
    let v = ranked(drain(&ua), |&(_, a)| Reverse(a[0]), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let hist_by: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let j: MatSet<(Id<User>, Option<Id<PostHistory>>)> = (&top).select(Ident::<User>::new().and((&hist_by).opt())).collect();
    type J = (Id<User>, Option<Id<PostHistory>>);
    let user_of = Same::<J>::new().map(|(u, _): J| u);
    let hist_of = Same::<J>::new().flat_map(|(_, h): J| h);
    let g = (&j).group_by(user_of.and(hist_of.select(htype_name(db)).opt())).select(Same::<J>::new()).fold(0i64, |n, (_, h)| n + h.is_some() as i64);
    type K = (Id<User>, Option<Str>);
    let v = drain((&g).and(Same::<K>::new().map(|(u, _): K| u).select(&ua)));
    rows(v.into_iter().map(|((u, name), (n, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([harness::fmt::ostr(name), V::I(n)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalBadges, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserStatistics)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalBadges, tu.UpVotes, tu.DownVotes, (tu.UpVotes - tu.DownVotes) AS NetVotes
// FROM TopUsers tu WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the posts x badges x votes product is driven for them alone.
fn q8139(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64]
        });
    let pc = user_distinct_posts(db);
    let v = drain((&us).and(&pc));
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(a[3] - a[4]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount, MAX(U.CreationDate) AS AccountCreationDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpvoteCount, DownvoteCount, AccountCreationDate,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.UpvoteCount, TU.DownvoteCount, TU.AccountCreationDate
// FROM TopUsers TU WHERE TU.Rank <= 10 ORDER BY TU.Reputation DESC;
//
// Rank reads only Reputation, so the top users are picked first and the posts x votes product is driven for them alone.
fn q7621(db: &'static So) -> String {
    let v = top_n(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = user_distinct_posts(db);
    let v = drain((&us).and(&pc));
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(user_col(db, u, "ucreated"));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseCount, AVG(COALESCE(P.Score, 0)) AS AvgScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostHistory PH ON P.Id = PH.PostId WHERE U.Reputation >= 1000 GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// CloseReasons AS (SELECT PH.UserId, PH.Comment AS CloseReason, COUNT(*) AS CloseReasonCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId, PH.Comment),
// RankedUsers AS (SELECT US.*, ROW_NUMBER() OVER (PARTITION BY US.Reputation ORDER BY US.AvgScore DESC) AS UserRank FROM UserStatistics US)
// SELECT R.UserId, R.DisplayName, R.Reputation, R.PostCount, R.AnswerCount, R.CloseCount, R.AvgScore, CR.CloseReason, COALESCE(CR.CloseReasonCount, 0) AS TotalCloseReasons
// FROM RankedUsers R LEFT JOIN CloseReasons CR ON R.UserId = CR.UserId WHERE R.UserRank = 1 ORDER BY R.Reputation DESC, R.AvgScore DESC LIMIT 10;
fn q22445(db: &'static So) -> String {
    let Post { score, post_type_id, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).ge(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(post_type_id).and(history_of(db).select(&db.post_history.post_history_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((s, t), h)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + matches!(h, Some(10 | 11)) as i64, a[3] + s],
            None => [a[0] + 1, a[1], a[2], a[3]],
        });
    let pc = user_distinct_posts(db);
    let mean = |a: [i64; 4]| fkey(a[3] as f64 / a[0] as f64);
    let rep = &db.user.reputation;
    let v = drain(&us);
    let first = top_per(v, |&(u, _)| rep.get(u).unwrap(), |&(u, a)| (Reverse(mean(a)), u), 1, false);
    let first: MatSet<Id<User>> = rel(first.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let PostHistory { user, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db.post_history.with(post_history_type_id.eq(10)).with(user).group_by(user.and(comment.opt())).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cr = rel(drain(&cr));
    let cr_of: HashIdx<Id<User>, ((Id<User>, Option<Str>), i64)> = (&cr).map(|((u, _), _)| u).inv().select(&cr).collect();
    let v = drain((&first).select((&us).and(&pc).and((&cr_of).opt())));
    let v = top_n(v, |&(u, ((a, _), c))| (Reverse(rep.get(u).unwrap()), Reverse(mean(a)), u, c.map(|((_, r), _)| r)), 10);
    rows(v.into_iter().map(|(u, ((a, n), c))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])]);
        f.extend(match c {
            Some(((_, r), k)) => [harness::fmt::ostr(r), V::I(k)],
            None => [V::Null, V::I(0)],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS PostRank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01' as date) - interval '1 year'),
// TotalVotes AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId),
// CommentsCount AS (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId)
// SELECT RP.PostId, RP.Title, RP.CreationDate, COALESCE(TV.UpVotes, 0) AS UpVotes, COALESCE(TV.DownVotes, 0) AS DownVotes, COALESCE(CC.CommentCount, 0) AS TotalComments, RP.ViewCount,
//        CASE WHEN RP.PostRank = 1 THEN 'Top Post' ELSE NULL END AS RankDescription
// FROM RankedPosts RP LEFT JOIN TotalVotes TV ON RP.PostId = TV.PostId LEFT JOIN CommentsCount CC ON RP.PostId = CC.PostId WHERE RP.PostRank <= 5 ORDER BY RP.Score DESC, RP.CreationDate DESC;
fn q1498(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1))).select(post_type_id));
    let v = ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap())), false);
    let v = per_group(v, |&(_, t)| t);
    let tp = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let tp: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tp).filt(|(_, r)| r <= 5).map(|(p, _)| p).inv().select((&tp).filt(|(_, r)| r <= 5)).collect();
    let vc = (&tp).map(|(p, _)| p).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).map(|(p, _)| p).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&tp).and(&vc).and(&cc));
    rows(v.into_iter().map(|(p, (((_, r), a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.extend(post_fields(db, p, &["views"]));
        f.push(if r == 1 { V::S("Top Post") } else { V::Null });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers, SUM(case when V.VoteTypeId = 2 then 1 else 0 end) AS TotalUpvotes,
//        SUM(case when V.VoteTypeId = 3 then 1 else 0 end) AS TotalDownvotes, SUM(case when B.Id IS NOT NULL then 1 else 0 end) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotes, TotalDownvotes, TotalBadges, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank
//     FROM UserStats)
// SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotes, TotalDownvotes, TotalBadges FROM TopUsers WHERE Rank <= 10 ORDER BY TotalPosts DESC, Reputation DESC;
//
// Rank reads only Reputation, so the top users are picked first and the posts x votes x badges product is driven for them alone; each COUNT(DISTINCT) is a fold over the user's posts.
fn q5473(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pt = &db.post.post_type_id;
    let us = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (v, b)| {
            let v = v.flatten();
            [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + b.is_some() as i64]
        });
    let pc = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(pt).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let v = drain((&pc).and(&us));
    rows(v.into_iter().map(|(u, (p, a))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(p.map(V::I));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserScoreCTE AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.UpVotes, us.DownVotes, us.TotalBounty, RANK() OVER (ORDER BY us.Reputation + us.UpVotes * 2 - us.DownVotes) AS RankScore
//     FROM UserScoreCTE us)
// SELECT tu.DisplayName, tu.Reputation, tu.UpVotes, tu.DownVotes, tu.TotalBounty, tu.RankScore, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, MAX(p.CreationDate) AS LastPostDate
// FROM TopUsers tu LEFT JOIN Posts p ON tu.UserId = p.OwnerUserId WHERE tu.RankScore <= 10
// GROUP BY tu.UserId, tu.DisplayName, tu.Reputation, tu.UpVotes, tu.DownVotes, tu.TotalBounty, tu.RankScore ORDER BY tu.RankScore;
fn q7315(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id.and(bounty_amount.opt())).opt()).fold([0i64; 3], |a, v| match v {
        Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + b.unwrap_or(0)],
        None => a,
    });
    let rep = &db.user.reputation;
    let v = ranked(drain(&us), |&(u, a)| rep.get(u).unwrap() + a[0] * 2 - a[1], false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| (u, (a, r))).collect());
    let top: HashIdx<Id<User>, (Id<User>, ([i64; 3], i64))> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let ps = (&top)
        .map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.creation_date)).opt())
        .fold([0, 0, 0, i64::MIN], |a, p| match p {
            Some((t, d)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(d)],
            None => a,
        });
    let v = drain((&top).and(&ps));
    rows(v.into_iter().map(|(u, ((_, (a, r)), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r), V::I(p[0]), V::I(p[1]), V::I(p[2]), tmax(p[3])]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, MAX(u.LastAccessDate) AS LastActiveDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, CommentCount, Upvotes, Downvotes, LastActiveDate, ROW_NUMBER() OVER (ORDER BY PostCount DESC, Reputation DESC) AS Rank
//     FROM UserStatistics)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.CommentCount, tu.Upvotes, tu.Downvotes, tu.LastActiveDate, pt.Name AS PostType
// FROM TopUsers tu JOIN Posts p ON tu.UserId = p.OwnerUserId JOIN PostTypes pt ON p.PostTypeId = pt.Id
// WHERE tu.Rank <= 10 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' ORDER BY tu.Rank;
//
// Rank reads only the distinct post count and Reputation, so the top users are picked first and the posts x comments x votes product is driven for them alone.
fn q8001(db: &'static So) -> String {
    let pc = user_distinct_posts(db);
    let v = top_n(drain(&pc), |&(u, n)| (Reverse(n), Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt()).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let recent = Ident::<Post>::new().with((&db.post.creation_date).ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)));
    let v = drain((&top).select((&pc).and((&cc).opt()).and(&us).and(posts_of(db).select(recent).select(ptype_name(db)))));
    rows(v.into_iter().map(|(u, (((n, c), a), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1]), user_col(db, u, "last_access"), V::S(t)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, BadgeCount, RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts,
//        RANK() OVER (ORDER BY Upvotes DESC) AS RankByUpvotes FROM UserStatistics)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, BadgeCount, RankByPosts, RankByUpvotes FROM TopUsers
// WHERE RankByPosts <= 10 OR RankByUpvotes <= 10 ORDER BY RankByPosts, RankByUpvotes;
fn q8096(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let pc = user_distinct_posts(db);
    let v = ranked(drain((&us).and(&pc)), |&(_, (_, n))| Reverse(n), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[2]), false);
    let r = rel(v.into_iter().map(|(((u, (a, n)), rp), ru)| (u, a, n, rp, ru)).collect());
    type T = (Id<User>, [i64; 5], i64, i64, i64);
    let v = drain((&r).with(Same::<T>::new().filt(|(_, _, _, rp, ru): T| rp <= 10 || ru <= 10)));
    rows(v.into_iter().map(|(_, (u, a, n, rp, ru))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(rp), V::I(ru)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopActiveUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalAnswers, TotalQuestions, TotalBounty, TotalUpvotes, TotalDownvotes,
//        ROW_NUMBER() OVER (ORDER BY TotalPosts DESC, Reputation DESC) AS Rank FROM UserStatistics)
// SELECT t.UserId, t.DisplayName, t.Reputation, t.TotalPosts, t.TotalAnswers, t.TotalQuestions, t.TotalBounty, t.TotalUpvotes, t.TotalDownvotes FROM TopActiveUsers t WHERE t.Rank <= 10 ORDER BY t.TotalPosts DESC;
//
// Rank reads only the distinct post count and Reputation, so the top users are picked first and the posts x votes product is driven for them alone.
fn q7246(db: &'static So) -> String {
    let pc = user_distinct_posts(db);
    let v = top_n(drain(&pc), |&(u, n)| (Reverse(n), Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let us = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(vote_type_id.and(bounty_amount.opt())).opt()).opt())
        .fold([0i64; 3], |a, v| match v.flatten() {
            Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
            None => a,
        });
    let tc = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64]);
    let v = drain((&pc).and(&tc).and(&us));
    rows(v.into_iter().map(|(u, ((n, t), a))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(t[0]), V::I(t[1])]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS post_id, p.Title AS post_title, p.CreationDate AS post_creation_date, COUNT(c.Id) AS comment_count,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS upvote_count, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS downvote_count, COUNT(DISTINCT b.Id) AS badge_count,
//        MAX(ph.CreationDate) AS last_edit_date
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate),
// RankedPosts AS (SELECT ps.*, RANK() OVER (ORDER BY ps.upvote_count DESC, ps.comment_count DESC) AS rank FROM PostStats ps)
// SELECT rp.post_id, rp.post_title, rp.post_creation_date, rp.comment_count, rp.upvote_count, rp.downvote_count, rp.badge_count, rp.last_edit_date,
//        CASE WHEN rp.rank <= 10 THEN 'Top 10' WHEN rp.rank <= 50 THEN 'Top 50' ELSE 'Others' END AS rank_category
// FROM RankedPosts rp ORDER BY rp.rank;
fn q8851(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ps = recent()
        .group_by(Ident::<Post>::new())
        .select(
            comments_of(db)
                .opt()
                .and(votes_of(db).select(&db.vote.vote_type_id).opt())
                .and(owner_user.select(badges_of(db)).opt())
                .and(history_of(db).select(&db.post_history.creation_date).opt()),
        )
        .fold([0, 0, 0, i64::MIN], |a, (((c, t), _), d)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3].max(d.unwrap_or(i64::MIN))]);
    let bc = recent().group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain((&ps).and(&bc)), |&(_, (a, _))| (Reverse(a[1]), Reverse(a[0])), false);
    rows(v.into_iter().map(|((p, (a, b)), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b), tmax(a[3])]);
        f.push(V::S(if r <= 10 { "Top 10" } else if r <= 50 { "Top 50" } else { "Others" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, U.DisplayName AS OwnerDisplayName FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month')
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.AnswerCount, TU.QuestionCount, RP.PostId, RP.Title, RP.CreationDate, RP.ViewCount
// FROM TopUsers TU JOIN RecentPosts RP ON TU.DisplayName = RP.OwnerDisplayName WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC, RP.ViewCount DESC;
fn q5552(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ur = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64],
        None => a,
    });
    let Post { creation_date, owner_user, .. } = &db.post;
    let by_name: HashIdx<Str, Id<Post>> = db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.select(&db.user.display_name)).inv().collect();
    let v = drain((&ur).and((&db.user.display_name).select(&by_name)));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY COUNT(c.Id) DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 0 GROUP BY p.Id, p.Title, p.PostTypeId, p.CreationDate),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.Rank, CASE WHEN rp.PostTypeId = 1 THEN 'Question' ELSE 'Other' END AS PostCategory FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT trp.PostId, trp.Title, trp.PostCategory, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = trp.PostId AND v.VoteTypeId IN (2, 3)) AS TotalVotes,
//        (SELECT COUNT(*) FROM Badges b WHERE b.UserId IN (SELECT DISTINCT p.OwnerUserId FROM Posts p WHERE p.Id = trp.PostId)) AS BadgeCount
// FROM TopRankedPosts trp ORDER BY trp.Rank, trp.Title;
fn q8307(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let v = drain((&rp).and(post_type_id));
    let top = top_per(v, |&(_, (_, t))| t, |&(p, (n, _))| (Reverse(n), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let bc = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&tv).and(&bc));
    rows(v.into_iter().map(|(p, (t, b))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::S(if post_type_id.get(p).unwrap() == 1 { "Question" } else { "Other" }), V::I(t), V::I(b)]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.UserId IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
//        RANK() OVER (ORDER BY COUNT(DISTINCT P.Id) DESC) AS PostRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalVotes FROM RankedUsers WHERE PostRank <= 10),
// PostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        COUNT(CASE WHEN P.PostTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount FROM Posts P GROUP BY P.OwnerUserId)
// SELECT T.DisplayName, T.Reputation, PS.QuestionCount, PS.AnswerCount, PS.CloseReopenCount, T.TotalVotes, (SELECT COUNT(*) FROM Comments C WHERE C.UserId = T.UserId) AS CommentCount
// FROM TopUsers T JOIN PostStats PS ON T.UserId = PS.OwnerUserId ORDER BY T.Reputation DESC, T.TotalVotes DESC;
//
// PostRank reads only the distinct post count, so the top users are picked first and the posts x votes product is driven for them alone.
fn q9261(db: &'static So) -> String {
    let v = ranked(drain(&user_distinct_posts(db)), |&(_, n)| Reverse(n), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let tv = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.user)).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let ps = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id)).fold([0i64; 3], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 10 | 11) as i64]);
    let cc = (&top).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&tv).and(&ps).and(&cc));
    rows(v.into_iter().map(|(u, ((t, a), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(t), V::I(c)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, CreationDate, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PopularPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS OwnerPostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// PostLinksCount AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS LinkCount FROM PostLinks pl GROUP BY pl.PostId),
// TopUsers AS (SELECT u.Id as UserId, u.DisplayName, ur.Reputation, ub.BadgeCount FROM UserReputation ur JOIN Users u ON ur.Id = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId
//     WHERE ur.ReputationRank <= 10)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.BadgeCount, COUNT(DISTINCT pp.Id) AS TotalPosts, SUM(COALESCE(plc.LinkCount, 0)) AS TotalLinks,
//        SUM(CASE WHEN pp.OwnerPostRank = 1 THEN 1 ELSE 0 END) AS MostViewedPostCount
// FROM TopUsers tu JOIN PopularPosts pp ON tu.UserId = pp.OwnerUserId LEFT JOIN PostLinksCount plc ON pp.Id = plc.PostId
// GROUP BY tu.UserId, tu.DisplayName, tu.Reputation, tu.BadgeCount ORDER BY tu.Reputation DESC, TotalPosts DESC;
fn q5449(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let qs = Ident::<Post>::new().with(post_type_id.eq(1));
    let mine: MatSet<Id<Post>> = (&top).select(posts_of(db).select(&qs)).collect();
    let first = top_per(drain((&mine).select(owner_user)), |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let lc = (&mine).group_by(Ident::<Post>::new()).select(links_of(db).opt()).fold(0i64, |n, l| n + l.is_some() as i64);
    let s = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(qs.and(&lc).and(Ident::<Post>::new().with(&first).opt())))
        .fold([0i64; 3], |a, ((_, l), f)| [a[0] + 1, a[1] + l, a[2] + f.is_some() as i64]);
    let bc = (&top).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&s).and((&bc).opt()));
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(harness::fmt::oint(b));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AcceptedAnswers, TotalUpVotes, TotalDownVotes, LastPostDate,
//        ROW_NUMBER() OVER (ORDER BY TotalUpVotes DESC) AS Rank FROM UserStats)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.AcceptedAnswers, tu.TotalUpVotes, tu.TotalDownVotes, tu.LastPostDate FROM TopUsers tu WHERE tu.Rank <= 10;
fn q6505(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, creation_date, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer.opt()).and(creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0, 0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some((((t, acc), d), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 2 && acc.is_some()) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5].max(d)],
            None => a,
        });
    let pc = user_distinct_posts(db);
    let v = top_n(drain((&us).and(&pc)), |&(u, (a, _))| (Reverse(a[3]), u), 10);
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a[..5].iter().map(|&x| V::I(x)));
        f.push(tmax(a[5]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// BadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// RankedUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, us.AnswerCount, us.QuestionCount, us.UpVotes, us.DownVotes, COALESCE(bc.BadgeCount, 0) AS BadgeCount,
//        RANK() OVER (ORDER BY us.Reputation DESC) AS Rank FROM UserStats us LEFT JOIN BadgeCounts bc ON us.UserId = bc.UserId)
// SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, UpVotes, DownVotes, BadgeCount, Rank FROM RankedUsers WHERE Rank <= 100 ORDER BY Reputation DESC, PostCount DESC;
//
// Rank reads only Reputation, so the top users are picked first and the posts x votes product is driven for them alone.
fn q8649(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 100).map(|((u, _), r)| (u, r)).collect());
    let top: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let us = (&top)
        .map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let bc = (&top).map(|(u, _)| u).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pc = user_distinct_posts(db);
    let v = drain((&top).and(&us).and(&pc).and(&bc));
    rows(v.into_iter().map(|(u, ((((_, r), a), n), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//        SUM(P.ViewCount) AS TotalViewCount FROM Posts P GROUP BY P.OwnerUserId)
// SELECT UR.UserId, UR.DisplayName, COALESCE(PS.TotalPosts, 0) AS PostsPublished, COALESCE(PS.QuestionsCount, 0) AS QuestionsAsked, COALESCE(PS.AnswersCount, 0) AS AnswersGiven, UR.Reputation,
//        CASE WHEN UR.Reputation > 10000 THEN 'Expert' WHEN UR.Reputation BETWEEN 5000 AND 10000 THEN 'Pro' ELSE 'Newbie' END AS UserLevel,
//        CASE WHEN PS.TotalViewCount IS NULL THEN 'No Views' WHEN PS.TotalViewCount > 0 AND PS.TotalViewCount <= 100 THEN 'Low Engagement'
//             WHEN PS.TotalViewCount > 100 AND PS.TotalViewCount <= 1000 THEN 'Moderate Engagement' ELSE 'High Engagement' END AS EngagementLevel
// FROM UserReputation UR LEFT JOIN PostStats PS ON UR.UserId = PS.OwnerUserId WHERE UR.Reputation > 1000 ORDER BY UR.Reputation DESC LIMIT 10;
fn q2289(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let v = top_n(drain(db.user.with(rep.gt(1000)).select(rep)), |&(u, r)| (Reverse(r), u), 10);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ps = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and((&db.post.view_count).opt()))).fold([0i64; 5], |a, (t, w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
    });
    let v = drain((&top).select((&ps).opt()));
    rows(v.into_iter().map(|(u, a)| {
        let r = rep.get(u).unwrap();
        let a = a.unwrap_or([0; 5]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r)]);
        f.push(V::S(if r > 10000 { "Expert" } else if r >= 5000 { "Pro" } else { "Newbie" }));
        let tv = a[4];
        f.push(V::S(if a[3] == 0 { "No Views" } else if tv > 0 && tv <= 100 { "Low Engagement" } else if tv > 100 && tv <= 1000 { "Moderate Engagement" } else { "High Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(u.Reputation, 0) AS UserReputation,
//        ROW_NUMBER() OVER (PARTITION BY p.Title ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVoteCounts AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN VoteTypeId = 1 THEN 1 END) AS AcceptedVotes FROM Votes GROUP BY PostId),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UserReputation, pvc.UpVotes, pvc.DownVotes, pvc.AcceptedVotes
//     FROM RankedPosts rp LEFT JOIN PostVoteCounts pvc ON rp.PostId = pvc.PostId WHERE rp.RN = 1)
// SELECT Title, CreationDate, Score, ViewCount, UserReputation, UpVotes, DownVotes, AcceptedVotes FROM FinalResults ORDER BY ViewCount DESC, Score DESC LIMIT 10;
fn q9838(db: &'static So) -> String {
    let Post { creation_date, title, view_count, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(title.opt()));
    let first = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = top_n(drain(&first), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(score.get(p).unwrap()), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 1) as i64]);
    let v = drain((&tp).select((&pv).opt().and(owner_user.select(&db.user.reputation).opt())));
    rows(v.into_iter().map(|(p, (a, r))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.push(V::I(r.unwrap_or(0)));
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id, p.Title, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.ViewCount),
// TopPosts AS (SELECT Id, Title, ViewCount, UpVotes, DownVotes, CommentCount, RANK() OVER (ORDER BY (UpVotes - DownVotes) DESC) AS VoteRank FROM PostStatistics),
// ClosedPosts AS (SELECT p.Id, p.Title, ph.CreationDate AS CloseDate, ph.UserDisplayName AS ClosedBy, ph.Comment AS CloseReason FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE ph.PostHistoryTypeId = 10)
// SELECT tp.Title, tp.ViewCount, tp.UpVotes, tp.DownVotes, tp.CommentCount, cp.CloseDate, cp.ClosedBy, cp.CloseReason
// FROM TopPosts tp LEFT JOIN ClosedPosts cp ON tp.Id = cp.Id WHERE tp.VoteRank <= 10 ORDER BY tp.VoteRank;
fn q4463(db: &'static So) -> String {
    let posts = || db.post.with((&db.post.creation_date).ge(ts(2023, 1, 1, 0, 0, 0)));
    let ps = posts()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = ranked(drain(&ps), |&(_, a)| Reverse(a[0] - a[1]), false);
    let top: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&top).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&top).select((&ps).and(&cc).and(closes.opt())));
    let PostHistory { creation_date: hd, user_display_name, comment, .. } = &db.post_history;
    rows(v.into_iter().map(|(p, ((a, c), h))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), harness::fmt::ostr(user_display_name.get(h)), harness::fmt::ostr(comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// MostCommented AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpvoteCount, rp.DownvoteCount, rp.CommentCount, RANK() OVER (ORDER BY rp.CommentCount DESC) AS CommentRank
//     FROM RankedPosts rp)
// SELECT mc.PostId, mc.Title, mc.CreationDate, mc.Score, mc.ViewCount, mc.UpvoteCount, mc.DownvoteCount, mc.CommentCount FROM MostCommented mc WHERE mc.CommentRank <= 10
// ORDER BY mc.CommentCount DESC, mc.CreationDate DESC;
fn q6651(db: &'static So) -> String {
    let rp = db
        .post
        .with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let v = ranked(drain(&rp), |&(_, a)| Reverse(a[2]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, a), _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, TotalBounty, RANK() OVER (ORDER BY TotalBounty DESC) AS BountyRank FROM UserPostStats)
// SELECT u.UserId, u.DisplayName, COALESCE(u.PostCount, 0) AS PostCount, COALESCE(u.AnswerCount, 0) AS AnswerCount, COALESCE(u.QuestionCount, 0) AS QuestionCount,
//        COALESCE(u.TotalBounty, 0) AS TotalBounty,
//        CASE WHEN u.BountyRank <= 10 THEN 'Top Bounty Users' WHEN u.BountyRank IS NOT NULL THEN 'Other Bounty Users' ELSE 'No Bounty' END AS UserCategory
// FROM TopUsers u FULL OUTER JOIN Users us ON u.UserId = us.Id WHERE us.Location IS NOT NULL OR us.WebsiteUrl IS NOT NULL ORDER BY UserCategory, TotalBounty DESC, u.DisplayName;
//
// Every TopUsers row is a user, so nothing is unmatched on that side of the FULL OUTER JOIN: it is Users LEFT JOIN TopUsers.
fn q206(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8))).select(bounty_amount.opt());
    let ups = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, b)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let v = ranked(drain(&ups), |&(_, a)| Reverse(a[3]), false);
    let tr = rel(v.into_iter().map(|((u, a), r)| (u, (a, r))).collect());
    let tu: HashIdx<Id<User>, (Id<User>, ([i64; 4], i64))> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let v = drain(db.user.with((&db.user.location).or(&db.user.website_url)).select((&tu).opt()));
    rows(v.into_iter().map(|(_, t)| match t {
        Some((u, (a, r))) => {
            let mut f = ucols(db, u, &["uid", "name"]);
            f.extend(a.map(V::I));
            f.push(V::S(if r <= 10 { "Top Bounty Users" } else { "Other Bounty Users" }));
            row(f)
        }
        None => row(vec![V::Null, V::Null, V::I(0), V::I(0), V::I(0), V::I(0), V::S("No Bounty")]),
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.QuestionCount, ua.TotalViews, ua.TotalScore, ROW_NUMBER() OVER (ORDER BY ua.TotalScore DESC) AS UserRank FROM UserActivity ua WHERE ua.QuestionCount > 10)
// SELECT tu.DisplayName, tu.QuestionCount, tu.TotalViews, tu.TotalScore, rp.Title, rp.CreationDate
// FROM TopUsers tu LEFT JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId AND rp.PostRank = 1 WHERE tu.UserRank <= 5 ORDER BY tu.TotalScore DESC, tu.DisplayName ASC;
fn q1285(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, creation_date, owner_user, .. } = &db.post;
    let qs = Ident::<Post>::new().with(post_type_id.eq(1));
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(qs.select(view_count.opt().and(score)))).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let tu = top_n(drain((&ua).filt(|a| a[0] > 10)), |&(u, a)| (Reverse(a[2]), u), 5);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let recent = drain((&tu).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))))));
    let best = top_per(recent, |&(u, _)| u, |&(_, p)| Reverse(score.get(p).unwrap()), 1, true);
    let best = rel(best.into_iter().map(|(_, p)| p).collect());
    let by_user: HashIdx<Id<User>, Id<Post>> = (&best).select(owner_user).inv().select(&best).collect();
    let v = drain((&tu).select((&ua).and((&by_user).opt())));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
//        COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers, SUM(p.Score) AS TotalScore, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserActivity WHERE TotalPosts > 0),
// PopularTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName HAVING COUNT(p.Id) > 5)
// SELECT u.DisplayName, u.Reputation, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalScore, p.TagName, p.PostCount,
//        CASE WHEN u.LastPostDate IS NULL THEN 'No posts made' ELSE 'Active user' END AS UserStatus
// FROM TopUsers u LEFT JOIN PopularTags p ON u.TotalPosts > 5 WHERE u.ReputationRank <= 10 ORDER BY u.TotalScore DESC, u.Reputation DESC;
//
// The ON clause names only u: users with more than five posts are crossed with PopularTags, the rest keep one NULL row.
fn q1487(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score).and(creation_date))).fold([0, 0, 0, 0, i64::MIN], |a, ((t, s), d)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4].max(d)]
    });
    let v = ranked(drain(&ua), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let pt = db.tag.group_by(&db.tag.tag_name).select(&by_tag).fold(0i64, |n, _| n + 1);
    let pt = rel(drain((&pt).filt(|n| n > 5)));
    type U = (Id<User>, [i64; 5]);
    let mut v: Vec<(U, Option<(Str, i64)>)> = Vec::new();
    (&tu).filt(|(_, a): U| a[0] > 5).cross(&pt).drive(|_, (u, t)| v.push((u, Some(t))));
    (&tu).filt(|(_, a): U| a[0] <= 5).drive(|_, u| v.push((u, None)));
    rows(v.into_iter().map(|((u, a), t)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(match t {
            Some((n, c)) => [V::S(n), V::I(c)],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if a[4] == i64::MIN { "No posts made" } else { "Active user" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation),
// UserActivity AS (SELECT pu.OwnerUserId AS UserId, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS HistoryCount
//     FROM Posts pu LEFT JOIN Comments c ON pu.Id = c.PostId LEFT JOIN PostHistory ph ON pu.Id = ph.PostId GROUP BY pu.OwnerUserId)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.BadgeCount, ua.CommentCount, ua.HistoryCount
// FROM TopUsers tu JOIN UserActivity ua ON tu.UserId = ua.UserId WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// Rank reads only Reputation, so the top users are picked first and the posts x badges product is driven for them alone; each COUNT(DISTINCT) is its own fold.
fn q5318(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ur = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.is_some() as i64]);
    let pc = user_distinct_posts(db);
    let cc = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db).opt())).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(history_of(db).opt())).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&ur).and(&pc).and(&cc).and(&hc));
    rows(v.into_iter().map(|(u, (((a, n), c), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(c), V::I(h)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(V.BountyAmount) AS TotalBounties, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalBounties, TotalComments, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserActivity)
// SELECT TU.DisplayName, TU.TotalQuestions, TU.TotalAnswers, TU.TotalBounties, TU.TotalComments, PT.Name AS PostTypeName,
//        (SELECT COUNT(*) FROM Posts P1 WHERE P1.OwnerUserId = TU.UserId AND P1.PostTypeId = PT.Id) AS PostCountByType
// FROM TopUsers TU CROSS JOIN PostTypes PT WHERE TU.PostRank <= 10 ORDER BY TU.PostRank, PT.Id;
//
// PostRank reads only the distinct post count, so the top users are picked first and the posts x bounty votes x comments product is driven for them alone.
fn q8008(db: &'static So) -> String {
    let v = ranked(drain(&user_distinct_posts(db)), |&(_, n)| Reverse(n), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8))).select(bounty_amount.opt());
    let ua = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, b), c)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0), a[4] + c.is_some() as i64]
            }
            None => a,
        });
    let bt = (&top).group_by(Ident::<User>::new().and(posts_of(db).select(&db.post.post_type))).select(Ident::<User>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&ua).cross(db.post_type.select(Ident::<PostType>::new())).and((&bt).opt()));
    rows(v.into_iter().map(|((u, t), ((a, _), n))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[4]), tname(db, t)];
        f.push(V::I(n.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostsCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(P.ViewCount) AS TotalViews, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE U.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostsCount, QuestionsCount, AnswersCount, TotalViews, UpVotesCount, DownVotesCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserActivity)
// SELECT UserId, DisplayName, Reputation, PostsCount, QuestionsCount, AnswersCount, TotalViews, UpVotesCount, DownVotesCount FROM TopUsers WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the posts x votes product is driven for them alone.
fn q7835(db: &'static So) -> String {
    let User { reputation, creation_date, .. } = &db.user;
    let v = ranked(drain(db.user.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(reputation)), |&(_, r)| Reverse(r), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ua = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&db.post.view_count).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, w), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64],
            None => a,
        });
    let v = drain((&ua).and(&user_distinct_posts(db)));
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[4]), V::I(a[5])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT v.UserId) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.Score DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, CommentCount, VoteCount FROM RankedPosts WHERE rn = 1 ORDER BY Score DESC, ViewCount DESC LIMIT 10)
// SELECT tp.*, CASE WHEN tp.VoteCount >= 100 THEN 'Highly Voted' WHEN tp.CommentCount >= 50 THEN 'Popular Discussion' ELSE 'Normal' END AS PostCategory
// FROM TopPosts tp JOIN PostHistory ph ON tp.PostId = ph.PostId
// WHERE ph.CreationDate BETWEEN TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND TIMESTAMP '2024-10-01 12:34:56' ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// rn partitions by p.Id, so it is 1 for every post; the LIMIT reads only base columns, so the ten posts are picked first.
fn q9017(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let up = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(up().select(&db.vote.user)).count_distinct();
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let recent = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).between(add_days(t0, -30), t0)));
    let v = drain((&cc).and((&vc).opt()).and(recent));
    rows(v.into_iter().map(|(p, ((c, n), _))| {
        let n = n.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(n), V::S(if n >= 100 { "Highly Voted" } else if c >= 50 { "Popular Discussion" } else { "Normal" })]);
        row(f)
    }))
}

// WITH UserScoreBreakdown AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserScoreBreakdown WHERE PostCount > 0),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT u.DisplayName, u.TotalScore, COALESCE(c.CloseCount, 0) AS ClosedPosts,
//        CASE WHEN u.TotalScore > 1000 THEN 'High Contributor' WHEN u.TotalScore BETWEEN 500 AND 1000 THEN 'Moderate Contributor' ELSE 'Low Contributor' END AS ContributorType
// FROM TopUsers u LEFT JOIN ClosedPosts c ON u.UserId = c.PostId WHERE u.Rank <= 10 ORDER BY u.TotalScore DESC, u.DisplayName ASC;
//
// `u.UserId = c.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q360(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).opt())))
        .fold(0i64, |n, (s, _)| n + s);
    let v = ranked(drain(&us), |&(_, s)| Reverse(s), false);
    let top = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let PostHistory { post_id, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post_id).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    type T = (Id<User>, i64);
    let v = drain((&top).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(&db.user.origid).select((&cp).opt()))));
    rows(v.into_iter().map(|(_, ((u, s), c))| {
        row(vec![user_col(db, u, "name"), V::I(s), V::I(c.unwrap_or(0)), V::S(if s > 1000 { "High Contributor" } else if s >= 500 { "Moderate Contributor" } else { "Low Contributor" })])
    }))
}

// WITH UserActivity AS (SELECT Users.Id AS UserId, Users.DisplayName, COUNT(DISTINCT Posts.Id) AS PostsCount, SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY COUNT(DISTINCT Posts.Id) DESC) AS ActivityRank
//     FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId LEFT JOIN Votes ON Posts.Id = Votes.PostId WHERE Users.Reputation > 1000 GROUP BY Users.Id, Users.DisplayName),
// ClosedPosts AS (SELECT Posts.Id, Posts.Title, COUNT(PostHistory.Id) AS CloseActions, MAX(PostHistory.CreationDate) AS LastClosed
//     FROM Posts INNER JOIN PostHistory ON Posts.Id = PostHistory.PostId WHERE PostHistory.PostHistoryTypeId = 10 GROUP BY Posts.Id, Posts.Title)
// SELECT UA.UserId, UA.DisplayName, UA.PostsCount, UA.UpVotes, UA.DownVotes, UA.ActivityRank, COALESCE(CP.CloseActions, 0) AS TotalCloseActions, CP.LastClosed
// FROM UserActivity UA LEFT JOIN ClosedPosts CP ON UA.UserId = (SELECT Posts.OwnerUserId FROM Posts WHERE Posts.Id = CP.Id) WHERE UA.ActivityRank <= 10 ORDER BY UA.ActivityRank;
//
// ActivityRank reads only the distinct post count, so the top users are picked first and the posts x votes product is driven for them alone.
fn q3100(db: &'static So) -> String {
    let pc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, n), r)| (u, (n, r))).collect());
    let top: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let ua = (&top).map(|(u, _)| u).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id)).opt()).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let PostHistory { post, post_history_type_id, creation_date, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&top).and(&ua).and(posts_of(db).select(&cp).opt()));
    rows(v.into_iter().map(|(u, (((_, (n, r)), a), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(r)]);
        f.extend(match c {
            Some((k, d)) => [V::I(k), V::T(d)],
            None => [V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT a.Id) AS AnswerCount, RANK() OVER (ORDER BY p.Score DESC) AS PostRank
//     FROM Posts AS p LEFT JOIN Users AS u ON p.OwnerUserId = u.Id LEFT JOIN Comments AS c ON p.Id = c.PostId LEFT JOIN Posts AS a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, AnswerCount FROM RankedPosts WHERE PostRank <= 10)
// SELECT t.Title, t.OwnerDisplayName, t.CreationDate, t.ViewCount, t.Score, t.CommentCount, t.AnswerCount,
//        CASE WHEN t.Score > 100 THEN 'Hot' WHEN t.Score BETWEEN 50 AND 100 THEN 'Trending' ELSE 'New' END AS PostStatus
// FROM TopPosts AS t ORDER BY t.ViewCount DESC, t.CreationDate DESC;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so CreationDate is read as New York local time; the data ends in 2024, so nothing is recent.
fn q6995(db: &'static So) -> String {
    let now = now_utc();
    let since = ny_to_utc(add_years(utc_to_ny(now), -1));
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1).and(creation_date.filt(move |d| ny_to_utc(d) >= since))).select(score)), |&(_, s)| Reverse(s), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(&ac));
    rows(v.into_iter().map(|(p, (c, a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "owner", "created", "views", "score"]);
        f.extend([V::I(c), V::I(a), V::S(if s > 100 { "Hot" } else if s >= 50 { "Trending" } else { "New" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId IN (10, 11) THEN 1 ELSE 0 END) AS ClosedPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 AND ph.PostHistoryTypeId = 5 THEN 1 ELSE 0 END) AS EditedQuestions, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE u.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY u.Id, u.DisplayName),
// MostActiveUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, ClosedPosts, EditedQuestions, Upvotes, Downvotes, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS RN
//     FROM UserActivity)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, ClosedPosts, EditedQuestions, Upvotes, Downvotes FROM MostActiveUsers WHERE RN <= 10;
//
// RN reads only the distinct post count, so the top users are picked first and the posts x history x votes product is driven for them alone.
fn q8745(db: &'static So) -> String {
    let users = || db.user.with((&db.user.creation_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 10);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ua = (&top)
        .group_by(Ident::<User>::new())
        .select(
            posts_of(db)
                .select((&db.post.post_type_id).and(history_of(db).select(&db.post_history.post_history_type_id).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
                .opt(),
        )
        .fold([0i64; 6], |a, p| match p {
            Some(((t, h), v)) => [
                a[0] + (t == 1) as i64,
                a[1] + (t == 2) as i64,
                a[2] + matches!(t, 10 | 11) as i64,
                a[3] + (t == 1 && h == Some(5)) as i64,
                a[4] + (v == Some(2)) as i64,
                a[5] + (v == Some(3)) as i64,
            ],
            None => a,
        });
    let v = drain((&ua).and(&pc));
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id, P.Title, P.ViewCount, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.ViewCount DESC) AS Rank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, SUM(P.Score) AS TotalScore FROM Users U INNER JOIN Posts P ON U.Id = P.OwnerUserId WHERE P.ViewCount > 100
//     GROUP BY U.Id, U.DisplayName HAVING SUM(P.Score) > 0),
// ClosestPosts AS (SELECT PH.PostId, COUNT(*) AS CloseCount, MAX(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS Closed,
//        MAX(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS Reopened FROM PostHistory PH GROUP BY PH.PostId)
// SELECT U.DisplayName, RP.Title, RP.ViewCount, COALESCE(CP.CloseCount, 0) AS CloseCount, COALESCE(CP.Closed, 0) AS Closed, COALESCE(CP.Reopened, 0) AS Reopened,
//        RANK() OVER (ORDER BY U.TotalScore DESC) AS UserRank
// FROM TopUsers U JOIN RankedPosts RP ON U.UserId = RP.Id LEFT JOIN ClosestPosts CP ON RP.Id = CP.PostId WHERE RP.Rank <= 5 ORDER BY UserRank, RP.ViewCount DESC;
//
// `U.UserId = RP.Id` joins a user id to a post id, so it goes through the raw ids.
fn q1254(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tu = db.post.with(view_count.gt(100)).group_by(owner_user).select(score).fold(0i64, |n, s| n + s);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.group_by(post).select(post_history_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1].max((t == 10) as i64), a[2].max((t == 11) as i64)]);
    let v = drain((&rp).select(origid.select(&uidx).select(Ident::<User>::new().and((&tu).filt(|s| s > 0))).and((&cp).opt())));
    let v = ranked(v, |&(_, ((_, s), _))| Reverse(s), false);
    rows(v.into_iter().map(|((p, ((u, _), c)), r)| {
        let c = c.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(c[0]), V::I(c[1]), V::I(c[2]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalAnswers, TotalQuestions, TotalUpvotes, TotalDownvotes, TotalBadges, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserStats)
// SELECT TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.TotalUpvotes, TU.TotalDownvotes, TU.TotalBadges FROM TopUsers TU
// WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the posts x votes x badges product is driven for them alone.
fn q9257(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let v = drain((&us).and(&user_distinct_posts(db)));
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, UpVotes, DownVotes, TotalBadges, RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts,
//        RANK() OVER (ORDER BY UpVotes - DownVotes DESC) AS RankByReputation FROM UserStats)
// SELECT t.UserId, t.DisplayName, t.TotalPosts, t.Questions, t.Answers, t.UpVotes, t.DownVotes, t.TotalBadges, t.RankByPosts, t.RankByReputation FROM TopUsers t
// WHERE t.RankByPosts <= 10 ORDER BY t.RankByReputation DESC, t.RankByPosts ASC;
fn q5596(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let v = ranked(drain((&us).and(&user_distinct_posts(db))), |&(_, (_, n))| Reverse(n), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[2] - a[3]), false);
    let r = rel(v.into_iter().map(|(((u, (a, n)), rp), rr)| (u, a, n, rp, rr)).collect());
    type T = (Id<User>, [i64; 5], i64, i64, i64);
    let v = drain((&r).with(Same::<T>::new().filt(|(_, _, _, rp, _): T| rp <= 10)));
    rows(v.into_iter().map(|(_, (u, a, n, rp, rr))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(rp), V::I(rr)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY p.CreationDate DESC) AS RecentRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE RecentRank <= 10)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes, ROUND(CAST(tp.UpVotes AS FLOAT) / NULLIF(tp.UpVotes + tp.DownVotes, 0), 2) AS UpVoteRatio,
//        SUM(b.Class) AS TotalBadges
// FROM TopPosts tp LEFT JOIN Badges b ON tp.PostId = b.UserId GROUP BY tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes
// ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// RecentRank reads only CreationDate, so the ten newest questions are picked first. `tp.PostId = b.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q8867(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(_, d)| Reverse(d), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let j: MatSet<(Id<Post>, Option<Id<Badge>>)> = (&tp).select(Ident::<Post>::new().and(origid.select(&uidx).select(badges_of(db)).opt())).collect();
    type J = (Id<Post>, Option<Id<Badge>>);
    let post_of = Same::<J>::new().map(|(p, _): J| p);
    let key = post_of.select((&db.post.title).opt().and(creation_date).and((&db.post.view_count).opt()).and(&db.post.score).and(&cc).and(&vc));
    let g = (&j).group_by(key).select(Same::<J>::new().flat_map(|(_, b): J| b).select(&db.badge.class).opt()).fold((0i64, 0i64), |(n, s), c| (n + c.is_some() as i64, s + c.unwrap_or(0)));
    let v = drain(&g);
    rows(v.into_iter().map(|(k, (n, s))| {
        let (((((t, d), w), sc), c), a) = k;
        let ratio = if a[0] + a[1] == 0 { V::Null } else { V::F((a[0] as f64 / (a[0] + a[1]) as f64 * 100.0).round() / 100.0) };
        row(vec![harness::fmt::ostr(t), V::T(d), harness::fmt::oint(w), V::I(sc), V::I(c), V::I(a[0]), V::I(a[1]), ratio, nullable(s, n)])
    }))
}

// WITH PostMetrics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT A.Id) AS AnswerCount,
//        U.Reputation AS OwnerReputation, U.Location AS OwnerLocation, U.CreationDate AS OwnerCreationDate
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Posts A ON P.Id = A.ParentId LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1
//     GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.Reputation, U.Location, U.CreationDate),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, CommentCount, AnswerCount, OwnerReputation, OwnerLocation, OwnerCreationDate,
//        DENSE_RANK() OVER (ORDER BY Score DESC) AS RankByScore, DENSE_RANK() OVER (ORDER BY ViewCount DESC) AS RankByViews FROM PostMetrics)
// SELECT *, CASE WHEN RankByScore <= 10 THEN 'Top Score' WHEN RankByViews <= 10 THEN 'Top Views' ELSE 'Other' END AS PerformanceCategory FROM TopPosts ORDER BY Score DESC, ViewCount DESC;
fn q10881(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(children_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let ac = qs().group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let v = ranked(drain((&cc).and(&ac)), |&(p, _)| Reverse(score.get(p).unwrap()), true);
    let v = ranked(v, |&((p, _), _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, true);
    rows(v.into_iter().map(|(((p, (c, a)), rs), rv)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(a)]);
        let u = owner_user.get(p);
        f.push(harness::fmt::oint(u.map(|u| db.user.reputation.get(u).unwrap())));
        f.push(harness::fmt::ostr(u.and_then(|u| db.user.location.get(u))));
        f.push(harness::fmt::ots(u.map(|u| db.user.creation_date.get(u).unwrap())));
        f.extend([V::I(rs), V::I(rv), V::S(if rs <= 10 { "Top Score" } else if rv <= 10 { "Top Views" } else { "Other" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(b.Class), 0) AS BadgeScore FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT ur.UserId, ur.Reputation, ur.BadgeScore, RANK() OVER (ORDER BY ur.Reputation + ur.BadgeScore DESC) AS UserRank FROM UserReputation ur WHERE ur.Reputation > 1000)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.Score, tu.UserId, tu.Reputation, tu.BadgeScore
// FROM RankedPosts rp JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId
// WHERE rp.PostRank = 1 AND (rp.Score IS NULL OR rp.Score > 10) AND rp.ViewCount IS NOT NULL AND (DATE_PART('dow', rp.CreationDate) IN (0, 6) OR (EXTRACT(HOUR FROM rp.CreationDate) BETWEEN 8 AND 18))
// ORDER BY tu.UserRank, rp.ViewCount DESC LIMIT 100;
fn q22635(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 0, 0, 0), -1))).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ur = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + c.unwrap_or(0));
    let v = ranked(drain(&ur), |&(u, b)| Reverse(db.user.reputation.get(u).unwrap() + b), false);
    let tr = rel(v.into_iter().map(|((u, b), r)| (u, (b, r))).collect());
    let tu: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let when = creation_date.filt(|d| matches!(dow(d), 0 | 6) || (8..=18).contains(&hour(d)));
    let v = drain((&first).with(score.gt(10)).with(view_count).with(when).select(owner_user.select(&tu)));
    let v = top_n(v, |&(p, (_, (_, r)))| (r, Reverse(view_count.get(p)), p), 100);
    rows(v.into_iter().map(|(p, (u, (b, _)))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "score"]);
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, COALESCE(po.RevisionCount, 0) AS RevisionCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) as rn
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS RevisionCount FROM PostHistory GROUP BY PostId) po ON p.Id = po.PostId
//     WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days')
// SELECT rp.Title, u.DisplayName, u.Reputation, u.Views, rp.CreationDate, CASE WHEN rp.PostTypeId = 1 THEN 'Question' WHEN rp.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END AS PostType,
//        rp.RevisionCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN Comments c ON rp.Id = c.PostId LEFT JOIN Votes v ON rp.Id = v.PostId WHERE rp.rn = 1
// GROUP BY rp.Title, u.DisplayName, u.Reputation, u.Views, rp.CreationDate, rp.PostTypeId, rp.RevisionCount ORDER BY rp.CreationDate DESC LIMIT 100;
//
// The GROUP BY names post and owner columns but not rp.Id, so the joined rows are grouped by that tuple.
fn q2936(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, title, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 0, 0, 0), -30))).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rc = (&first).group_by(Ident::<Post>::new()).select(history_of(db)).fold(0i64, |n, _| n + 1);
    let j: MatSet<(Id<Post>, Option<Id<Comment>>, Option<Id<Vote>>)> =
        (&first).select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt())).map(|((p, c), v)| (p, c, v)).collect();
    type J = (Id<Post>, Option<Id<Comment>>, Option<Id<Vote>>);
    let post_of = || Same::<J>::new().map(|(p, _, _): J| p);
    let u = &db.user;
    let key = post_of().select(title.opt().and(owner_user.select((&u.display_name).and(&u.reputation).and(&u.views))).and(creation_date).and(post_type_id).and((&rc).opt()));
    let row_of = Same::<J>::new().map(|(_, c, _): J| c.is_some()).and(Same::<J>::new().flat_map(|(_, _, v): J| v).select(&db.vote.vote_type_id).opt());
    let g = (&j).group_by(key).select(row_of).fold([0i64; 3], |a, (c, t)| [a[0] + c as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&g), |&(k, _)| {
        let ((((_, _), d), _), _) = k;
        (Reverse(d), k)
    }, 100);
    rows(v.into_iter().map(|(((((t, ((n, r), w)), d), ty), rc), a)| {
        row(vec![
            harness::fmt::ostr(t),
            V::S(n),
            V::I(r),
            V::I(w),
            V::T(d),
            V::S(if ty == 1 { "Question" } else if ty == 2 { "Answer" } else { "Other" }),
            V::I(rc.unwrap_or(0)),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
        ])
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.PostTypeId, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, U.Reputation AS OwnerReputation,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.ViewCount DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, OwnerReputation FROM RankedPosts WHERE PostRank <= 10),
// VotesSummary AS (SELECT V.PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes V GROUP BY V.PostId),
// FinalResults AS (SELECT TP.Title, TP.OwnerDisplayName, TP.OwnerReputation, COALESCE(VS.UpVotes, 0) AS UpVotes, COALESCE(VS.DownVotes, 0) AS DownVotes
//     FROM TopPosts TP LEFT JOIN VotesSummary VS ON TP.PostId = VS.PostId)
// SELECT Title, OwnerDisplayName, OwnerReputation, UpVotes, DownVotes FROM FinalResults ORDER BY UpVotes DESC, DownVotes ASC;
fn q7113(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain(&vs);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.UpVotes, u.DownVotes, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.UpVotes, u.DownVotes),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalAnswers, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStatistics)
// SELECT t.DisplayName, t.Reputation, t.TotalPosts, t.TotalAnswers, COALESCE(t.TotalUpVotes, 0) AS UpVotes, COALESCE(t.TotalDownVotes, 0) AS DownVotes,
//        CASE WHEN t.ReputationRank <= 10 THEN 'Top User' WHEN t.ReputationRank <= 50 THEN 'Contributing User' ELSE 'New User' END AS UserCategory
// FROM TopUsers t WHERE t.TotalPosts > 5 ORDER BY t.Reputation DESC, t.TotalPosts DESC;
fn q6401(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let pa = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id)).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + (t == 2) as i64]);
    let v = ranked(drain((&us).and((&pa).opt())), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let r = rel(v.into_iter().map(|((u, (a, p)), r)| (u, a, p.unwrap_or([0; 2]), r)).collect());
    type T = (Id<User>, [i64; 2], [i64; 2], i64);
    let v = drain((&r).with(Same::<T>::new().filt(|(_, _, p, _): T| p[0] > 5)));
    rows(v.into_iter().map(|(_, (u, a, p, r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p[0]), V::I(p[1]), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if r <= 10 { "Top User" } else if r <= 50 { "Contributing User" } else { "New User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionsAsked,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswersProvided, COALESCE(SUM(p.ViewCount), 0) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionsAsked, AnswersProvided, TotalViews, ROW_NUMBER() OVER (ORDER BY TotalViews DESC) AS UserRank FROM UserPostStats)
// SELECT t.DisplayName, t.QuestionsAsked, t.AnswersProvided, t.TotalViews, rp.Title, rp.ViewCount, rp.Score, rp.CreationDate
// FROM TopUsers t LEFT JOIN RankedPosts rp ON t.UserId = rp.PostId WHERE t.UserRank <= 10 ORDER BY t.TotalViews DESC, rp.Score DESC;
//
// `t.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids. PostRank is never read.
fn q568(db: &'static So) -> String {
    let Post { post_type_id, view_count, creation_date, origid, .. } = &db.post;
    let ups = db.user.with((&db.user.reputation).gt(100)).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, w)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.unwrap_or(0)],
        None => a,
    });
    let v = top_n(drain(&ups), |&(u, a)| (Reverse(a[2]), u), 10);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pidx: HashIdx<i64, Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(origid).inv().collect();
    let v = drain((&top).select((&ups).and((&db.user.origid).select(&pidx).opt())));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "views", "score", "created"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM Users),
// TopUsers AS (SELECT UserId, Reputation FROM UserReputation WHERE Rank <= 10),
// PostSummary AS (SELECT P.Id AS PostId, P.OwnerUserId, P.PostTypeId, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, MAX(P.CreationDate) AS RecentActivity
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.OwnerUserId, P.PostTypeId),
// TopPosts AS (SELECT PS.PostId, PS.OwnerUserId, PS.AnswerCount, PS.CommentCount, PS.Upvotes, PS.Downvotes, PS.RecentActivity, U.DisplayName
//     FROM PostSummary PS JOIN Users U ON PS.OwnerUserId = U.Id WHERE U.Id IN (SELECT UserId FROM TopUsers) ORDER BY PS.Upvotes DESC LIMIT 5)
// SELECT TP.PostId, TP.DisplayName AS OwnerName, TP.AnswerCount, TP.CommentCount, TP.Upvotes, TP.Downvotes, EXTRACT(EPOCH FROM TP.RecentActivity) AS RecentActivityEpoch
// FROM TopPosts TP ORDER BY TP.Upvotes DESC;
fn q8709(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let mine: MatSet<Id<Post>> = (&top).select(posts_of(db)).collect();
    let pt = &db.post.post_type_id;
    let ps = (&mine)
        .group_by(Ident::<Post>::new())
        .select(pt.and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, ((t, c), v)| [a[0] + (t == 2) as i64, a[1] + c.is_some() as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]);
    let v = top_n(drain(&ps), |&(p, a)| (Reverse(a[2]), p), 5);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::F(db.post.creation_date.get(p).unwrap() as f64 / 1e6));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalVotes, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalVotes DESC) AS UserRank FROM UserVoteStats WHERE TotalVotes > 0),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, DENSE_RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'))
// SELECT TU.DisplayName, TU.TotalVotes, TU.UpVotes, TU.DownVotes, RP.Title, RP.CreationDate, COALESCE((SELECT U.DisplayName FROM Users U WHERE U.Id = RP.OwnerUserId), 'Anonymous') AS PostOwner,
//        (SELECT COUNT(C.Id) FROM Comments C WHERE C.PostId = RP.PostId) AS CommentCount
// FROM TopUsers TU LEFT JOIN RecentPosts RP ON TU.UserId = RP.OwnerUserId WHERE TU.UserRank <= 10 ORDER BY TU.TotalVotes DESC, RP.CreationDate DESC;
fn q1810(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let v = ranked(drain(&uv), |&(_, a)| Reverse(a[0]), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let cc = db.post.with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&top).select((&uv).and(posts_of(db).select(Ident::<Post>::new().and(&cc)).opt())));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(match p {
            Some((p, c)) => [title(db, p), V::T(db.post.creation_date.get(p).unwrap()), user_col(db, u, "name"), V::I(c)],
            None => [V::Null, V::Null, V::S("Anonymous"), V::I(0)],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        p.OwnerUserId FROM Posts p WHERE p.PostTypeId IN (1, 2)),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) >= 5),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT u.UserId, u.DisplayName, tp.PostId, tp.Title, tp.ViewCount, tp.Score, tp.CreationDate, COALESCE(ph.EditCount, 0) AS EditCount, ph.LastEditDate
// FROM TopUsers u JOIN RankedPosts tp ON u.UserId = tp.OwnerUserId AND tp.Rank <= 3 LEFT JOIN PostHistorySummary ph ON tp.PostId = ph.PostId
// ORDER BY u.TotalScore DESC, tp.Score DESC FETCH FIRST 50 ROWS ONLY;
fn q7719(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let tu = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let v = drain(db.post.with(post_type_id.is_in([1, 2])).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and((&tu).filt(|a| a[0] >= 5)))).and((&phs).opt()));
    let v = top_n(v, |&(p, ((u, a), _))| (Reverse(a[1]), Reverse(score.get(p).unwrap()), u, p), 50);
    rows(v.into_iter().map(|(p, ((u, _), h))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title", "views", "score", "created"]));
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, Views, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT ru.DisplayName, ru.Reputation, ru.Views, ru.PostCount, ru.QuestionCount, ru.AnswerCount, ru.UpVotes, ru.DownVotes,
//        CASE WHEN ru.QuestionCount = 0 THEN 0 ELSE CAST(ru.AnswerCount AS FLOAT) / ru.QuestionCount END AS AnswerToQuestionRatio, COALESCE(b.Class, 0) AS BadgeClass
// FROM RankedUsers ru LEFT JOIN Badges b ON ru.UserId = b.UserId AND b.Class = 1 WHERE ru.Rank <= 100 ORDER BY ru.Rank;
//
// Rank reads only Reputation, so the top users are picked first and the posts x votes product is driven for them alone.
fn q4925(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 100);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))).select(&db.badge.class);
    let v = drain((&us).and(&user_distinct_posts(db)).and(gold.opt()));
    rows(v.into_iter().map(|(u, ((a, n), c))| {
        let mut f = ucols(db, u, &["name", "rep", "uviews"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::F(if a[0] == 0 { 0.0 } else { (a[1] as f32 / a[0] as f32) as f64 }));
        f.push(V::I(c.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Ranking
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 10),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ub.TotalBadges, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, COALESCE(pc.CommentCount, 0) AS TotalComments
// FROM RankedPosts rp LEFT JOIN Users u ON rp.PostId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId
// WHERE rp.Ranking <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q420(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(10))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&pc).and(origid.select(&uidx).select(&ub).opt()));
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::I(c));
        row(f)
    }))
}

// Rewritten (rewrites/7563.sql): CombinedData also carries PP.Id AS PostId, and the ORDER BY gains `, PostId`.
// WITH UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// TopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, UB.BadgeCount FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId WHERE U.Reputation > 1000),
// PopularPosts AS (SELECT P.Id, P.Title, P.Score, P.ViewCount, P.OwnerUserId, ROW_NUMBER() OVER(ORDER BY P.Score DESC) AS Rank FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' AND P.PostTypeId = 1),
// PostWithVoteCounts AS (SELECT P.Id, P.Title, COALESCE(V.UpVotes, 0) AS UpVotes, COALESCE(V.DownVotes, 0) AS DownVotes FROM Posts P
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) V
//     ON P.Id = V.PostId),
// CombinedData AS (SELECT TU.DisplayName, TU.Reputation, TU.BadgeCount, PP.Title, PP.Score, PP.ViewCount, PV.UpVotes, PV.DownVotes, PP.Id AS PostId
//     FROM TopUsers TU JOIN PopularPosts PP ON TU.Id = PP.OwnerUserId JOIN PostWithVoteCounts PV ON PP.Id = PV.Id)
// SELECT DisplayName, Reputation, BadgeCount, Title, Score, ViewCount, UpVotes, DownVotes FROM CombinedData WHERE BadgeCount >= 5 ORDER BY Reputation DESC, Score DESC, PostId LIMIT 10;
fn q7563(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, origid, .. } = &db.post;
    let bc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let pp = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)));
    let v = drain(pp.select(owner_user.select(Ident::<User>::new().and((&bc).filt(|n| n >= 5)))));
    let v = top_n(v, |&(p, (u, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), origid.get(p).unwrap()), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&bc))).and(&pv));
    rows(v.into_iter().map(|(p, ((u, b), a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS Comments, SUM(v.BountyAmount) AS TotalBounties,
//        RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS ActivityRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, Questions, Answers, Comments, TotalBounties FROM UserActivity WHERE ActivityRank <= 10)
// SELECT u.UserId, u.DisplayName, u.PostCount, u.Questions, u.Answers, u.Comments, CASE WHEN u.TotalBounties IS NULL THEN 0 ELSE u.TotalBounties END AS BountySum,
//        (SELECT AVG(PostCount) FROM UserActivity) AS AvgPosts
// FROM TopUsers u JOIN Badges b ON u.UserId = b.UserId WHERE b.Class = 1 ORDER BY u.PostCount DESC, u.UserId;
//
// ActivityRank reads only the distinct post count, so the top users are picked first and the posts x comments x votes product is driven for them alone.
// AvgPosts is an uncorrelated scalar: one fold over every UserActivity row, crossed with the output.
fn q8827(db: &'static So) -> String {
    let pc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let avgp = whole(&pc).select(&pc).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let v = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Vote { bounty_amount, .. } = &db.vote;
    let ua = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt())).opt().and(votes_by(db).select(bounty_amount.opt()).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (t, c) = p.map_or((0, false), |(t, c)| (t, c.is_some()));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c as i64, a[3] + b.flatten().unwrap_or(0)]
        });
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let v = drain((&ua).and(&pc).and(gold).cross(&avgp));
    rows(v.into_iter().map(|((u, _), (((a, n), _), (k, s)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(s, k)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, DENSE_RANK() OVER (ORDER BY COUNT(v.Id) DESC) AS VoteRank
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// ClosedPostReasons AS (SELECT ph.PostId, ph.Comment AS CloseReason, ph.CreationDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10)
// SELECT u.DisplayName, pus.PostId, pus.Title, pus.CreationDate AS PostCreationDate, COALESCE(cpr.CloseReason, 'Open') AS PostCloseReason, uv.TotalVotes, uv.UpVotes, uv.DownVotes, uv.VoteRank
// FROM Users u JOIN RecentPosts pus ON u.Id = pus.OwnerUserId LEFT JOIN ClosedPostReasons cpr ON pus.PostId = cpr.PostId JOIN UserVoteStats uv ON u.Id = uv.UserId
// WHERE uv.TotalVotes > 0 ORDER BY uv.VoteRank, pus.CreationDate DESC;
fn q23(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let v = ranked(drain(&uv), |&(_, a)| Reverse(a[0]), true);
    let r = rel(v.into_iter().map(|((u, a), r)| (u, (a, r))).collect());
    let ur: HashIdx<Id<User>, (Id<User>, ([i64; 3], i64))> = (&r).map(|(u, _)| u).inv().select((&r).filt(|(_, (a, _))| a[0] > 0)).collect();
    let recent = Ident::<Post>::new().with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10))).select((&db.post_history.comment).opt());
    let v = drain(db.post.with(recent).select((&db.post.owner_user).select(&ur).and(closes.opt())));
    rows(v.into_iter().map(|(p, ((u, (a, r)), c))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.push(match c {
            Some(c) => c.map_or(V::Null, V::S),
            None => V::S("Open"),
        });
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r)]);
        row(f)
    }))
}

// WITH LatestPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Tags, U.DisplayName AS OwnerName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// PostVoteSummary AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId),
// PostHistoryCounts AS (SELECT Ph.PostId, COUNT(*) AS EditCount, MAX(CASE WHEN Ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS IsClosed FROM PostHistory Ph GROUP BY Ph.PostId)
// SELECT lp.PostId, lp.Title, lp.CreationDate, lp.Tags, lp.OwnerName, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes, phc.EditCount,
//        CASE WHEN phc.IsClosed = 1 THEN 'Yes' ELSE 'No' END AS IsClosedPost
// FROM LatestPosts lp LEFT JOIN PostVoteSummary pvs ON lp.PostId = pvs.PostId LEFT JOIN PostHistoryCounts phc ON lp.PostId = phc.PostId WHERE lp.rn = 1 ORDER BY lp.CreationDate DESC LIMIT 100;
//
// The LIMIT reads only CreationDate, so the hundred newest latest-posts are picked first.
fn q3954(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1))).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first = top_n(first, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    let tp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id)).fold((0i64, false), |(n, c), t| (n + 1, c || matches!(t, 10 | 11)));
    let v = drain((&pv).and((&hc).opt()));
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "tags", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), h.map_or(V::Null, |h| V::I(h.0)), V::S(if h.map_or(false, |h| h.1) { "Yes" } else { "No" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// AggregatedStats AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, CommentCount, UpVotes, DownVotes, (UpVotes - DownVotes) AS NetVotes FROM RankedPosts WHERE Rank = 1)
// SELECT COUNT(*) AS TotalPosts, AVG(CommentCount) AS AvgComments, AVG(UpVotes) AS AvgUpVotes, AVG(DownVotes) AS AvgDownVotes, AVG(NetVotes) AS AvgNetVotes
// FROM AggregatedStats WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month';
//
// Rank partitions by p.Id, so it is 1 for every post.
fn q6176(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let qs = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1))).and(creation_date.ge(add_months(t0, -1))));
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let s = whole(&cc).select((&cc).and(&vc)).fold([0i64; 4], |a, (c, v)| [a[0] + 1, a[1] + c, a[2] + v[0], a[3] + v[1]]);
    let v = drain(&s);
    rows(v.into_iter().map(|(_, a)| row(vec![V::I(a[0]), avg(a[1], a[0]), avg(a[2], a[0]), avg(a[3], a[0]), avg(a[2] - a[3], a[0])])))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalBounty, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY TotalPosts DESC) AS PostsRank FROM UserStats),
// CombinedRanks AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalBounty, (ReputationRank + PostsRank) AS CombinedRank FROM TopUsers)
// SELECT DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalBounty, CombinedRank FROM CombinedRanks WHERE CombinedRank <= 10 ORDER BY CombinedRank;
//
// Both ranks read only Reputation and the distinct post count, so the users are picked first and the posts x bounty votes product is driven for them alone.
fn q8719(db: &'static So) -> String {
    let pc = user_distinct_posts(db);
    let v = ranked(drain(&pc), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = ranked(v, |&((_, n), _)| Reverse(n), false);
    let r = rel(v.into_iter().map(|(((u, n), a), b)| (u, n, a + b)).collect());
    type T = (Id<User>, i64, i64);
    let top: HashIdx<Id<User>, T> = (&r).filt(|(_, _, c): T| c <= 10).map(|(u, _, _): T| u).inv().select(&r).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([8, 9]))).select(bounty_amount.opt());
    let us = (&top).map(|(u, _, _)| u).group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, b)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let v = drain((&top).and(&us));
    rows(v.into_iter().map(|(u, ((_, n, c), a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViewCount,
//        SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, TotalViewCount, AcceptedAnswers, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserStatistics)
// SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.PostCount, TU.AnswerCount, TU.QuestionCount, TU.TotalViewCount, TU.AcceptedAnswers,
//        CASE WHEN TU.UserRank <= 10 THEN 'Top User' WHEN TU.UserRank <= 50 THEN 'Top Contributor' ELSE 'Regular User' END AS UserCategory
// FROM TopUsers TU WHERE TU.QuestionCount > 0 AND TU.AcceptedAnswers > 0 ORDER BY TU.UserRank LIMIT 20;
fn q8580(db: &'static So) -> String {
    let Post { post_type_id, view_count, accepted_answer, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(accepted_answer.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, w), acc)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + w.unwrap_or(0), a[4] + acc.is_some() as i64],
        None => a,
    });
    let v = ranked(drain(&us), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let r = rel(v.into_iter().map(|((u, a), r)| (u, a, r)).collect());
    type T = (Id<User>, [i64; 5], i64);
    let v = drain((&r).with(Same::<T>::new().filt(|(_, a, _): T| a[2] > 0 && a[4] > 0)));
    let v = top_n(v, |&(_, (u, _, r))| (r, u), 20);
    rows(v.into_iter().map(|(_, (u, a, r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top User" } else if r <= 50 { "Top Contributor" } else { "Regular User" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, TotalPosts, TotalQuestions, TotalAnswers, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserActivity)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.UpVotes, tu.DownVotes, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers,
//        CASE WHEN tu.TotalPosts > 0 THEN ROUND((CAST(tu.UpVotes AS FLOAT) / NULLIF((tu.UpVotes + tu.DownVotes), 0)) * 100, 2) ELSE NULL END AS UpvotePercentage
// FROM TopUsers tu WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the posts x votes product is driven for them alone. FLOAT is 32-bit.
fn q131(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let pc = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let v = drain((&us).and(&pc));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(p[0]), V::I(p[1]), V::I(p[2])]);
        f.push(if p[0] > 0 && a[0] + a[1] > 0 {
            let r = a[0] as f32 / (a[0] + a[1]) as f32 * 100.0f32;
            V::F(((r as f64 * 100.0).round() / 100.0) as f32 as f64)
        } else {
            V::Null
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionsAsked, AnswersGiven, TotalUpVotes, TotalDownVotes, TotalComments, RANK() OVER (ORDER BY TotalUpVotes - TotalDownVotes DESC) AS UserRank FROM UserActivity)
// SELECT U.UserId, U.DisplayName, U.QuestionsAsked, U.AnswersGiven, U.TotalUpVotes, U.TotalDownVotes, U.TotalComments,
//        CASE WHEN U.UserRank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorStatus
// FROM TopUsers U WHERE U.QuestionsAsked > 0 AND U.AnswersGiven > 0 ORDER BY U.UserRank, U.DisplayName;
fn q2195(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((t, v), _)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let v = ranked(drain((&ua).and((&cc).opt())), |&(_, (a, _))| Reverse(a[2] - a[3]), false);
    let r = rel(v.into_iter().map(|((u, (a, c)), r)| (u, a, c.unwrap_or(0), r)).collect());
    type T = (Id<User>, [i64; 4], i64, i64);
    let v = drain((&r).with(Same::<T>::new().filt(|(_, a, _, _): T| a[0] > 0 && a[1] > 0)));
    rows(v.into_iter().map(|(_, (u, a, c, r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(c), V::S(if r <= 10 { "Top Contributor" } else { "Regular Contributor" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats WHERE PostCount > 0)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.PostCount, U.QuestionCount, U.AnswerCount, U.TotalVotes, U.Rank, COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM TopUsers U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON U.UserId = b.UserId WHERE U.Rank <= 10 ORDER BY U.Rank;
fn q5846(db: &'static So) -> String {
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and((&vc).opt()))).fold([0i64; 4], |a, (t, n)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + n.unwrap_or(0)]
    });
    let v = top_n(drain(&us), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    let r = rel(v.into_iter().enumerate().map(|(i, (u, a))| (u, a, i as i64 + 1)).collect());
    type T = (Id<User>, [i64; 4], i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&r).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _, _): T| u).select((&bc).opt()))));
    rows(v.into_iter().map(|(_, ((u, a, k), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(k), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0 AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalQuestions, COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.PostId END) AS ClosedQuestions,
//        COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 12 THEN ph.PostId END) AS DeletedQuestions
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY u.Id, u.DisplayName)
// SELECT us.DisplayName, us.TotalQuestions, us.ClosedQuestions, us.DeletedQuestions, rp.Title AS LatestPostTitle, rp.CreationDate AS LatestPostDate, rp.Score AS LatestPostScore,
//        rp.ViewCount AS LatestPostViews
// FROM UserStats us LEFT JOIN RankedPosts rp ON us.UserId = rp.PostId WHERE us.TotalQuestions > 0 ORDER BY us.TotalQuestions DESC, us.ClosedQuestions ASC LIMIT 10;
//
// Each COUNT(DISTINCT ph.PostId) counts the user's questions that have such a history row, so it is a per-question flag summed per user.
// `us.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids. PostRank is never read.
fn q2865(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, origid, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let flags = qs().group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0].max((t == Some(10)) as i64), a[1].max((t == Some(12)) as i64)]
    });
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&flags)).fold([0i64; 3], |a, f| [a[0] + 1, a[1] + f[0], a[2] + f[1]]);
    let v = top_n(drain(&us), |&(u, a)| (Reverse(a[0]), a[1], u), 10);
    let top = rel(v);
    let rp: HashIdx<i64, Id<Post>> = qs().with(score.gt(0).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(origid).inv().collect();
    type T = (Id<User>, [i64; 3]);
    let v = drain((&top).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(&db.user.origid).select((&rp).opt()))));
    rows(v.into_iter().map(|(_, ((u, a), p))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created", "score", "views"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStats AS (SELECT P.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
//     FROM Posts P GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT UR.UserId, UR.DisplayName, UR.Reputation, PS.TotalPosts, PS.TotalQuestions, PS.TotalAnswers FROM UserReputation UR JOIN PostStats PS ON UR.UserId = PS.OwnerUserId
//     WHERE UR.Reputation > 1000)
// SELECT COALESCE(TU.DisplayName, 'Unknown User') AS UserName, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, COALESCE(PHT.Name, 'No History') AS RecentActionType,
//        COUNT(PH.Id) AS ActionCount
// FROM TopUsers TU LEFT JOIN PostHistory PH ON PH.UserId = TU.UserId AND PH.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// LEFT JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// GROUP BY TU.UserId, TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, PHT.Name ORDER BY TU.Reputation DESC, ActionCount DESC LIMIT 10;
fn q4053(db: &'static So) -> String {
    let ps = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id)).fold([0i64; 3], |a, t| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]
    });
    let PostHistory { user, creation_date, .. } = &db.post_history;
    let recent: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(user).inv().collect();
    let j: MatSet<(Id<User>, Option<Id<PostHistory>>)> = db.user.with(&ps).select(Ident::<User>::new().and((&recent).opt())).collect();
    type J = (Id<User>, Option<Id<PostHistory>>);
    let g = (&j)
        .group_by(Same::<J>::new().map(|(u, _): J| u).and(Same::<J>::new().flat_map(|(_, h): J| h).select(htype_name(db)).opt()))
        .select(Same::<J>::new())
        .fold(0i64, |n, (_, h)| n + h.is_some() as i64);
    type K = (Id<User>, Option<Str>);
    let v = drain((&g).and(Same::<K>::new().map(|(u, _): K| u).select(&ps)));
    let v = top_n(v, |&((u, t), (n, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u, t), 10);
    rows(v.into_iter().map(|((u, t), (n, a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::S(t.unwrap_or("No History")), V::I(n)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(CASE WHEN B.Id IS NOT NULL THEN 1 END) AS BadgeCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON P.OwnerUserId = B.UserId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount),
// RankedPosts AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.ViewCount, PS.UpVotes, PS.DownVotes, PS.CommentCount, PS.BadgeCount,
//        RANK() OVER (ORDER BY PS.Score DESC, PS.ViewCount DESC) AS Rank FROM PostStats PS)
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.UpVotes, RP.DownVotes, RP.CommentCount, RP.BadgeCount, RP.Rank FROM RankedPosts RP WHERE RP.Rank <= 10 ORDER BY RP.Rank, RP.Score DESC;
//
// Rank reads only base columns, so the top posts are picked first and the votes x comments x badges product is driven for them alone.
fn q7905(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    let tp: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let s = (&tp)
        .map(|(p, _)| p)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 4], |a, ((t, c), b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64, a[3] + b.is_some() as i64]);
    let v = drain((&tp).and(&s));
    rows(v.into_iter().map(|(p, ((_, r), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id, p.Title, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, MAX(p.CreationDate) AS LastActivity, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     GROUP BY p.Id, p.Title),
// PostRanked AS (SELECT ps.Id, ps.Title, ps.CommentCount, ps.AnswerCount, ps.UpVoteCount, ps.DownVoteCount, ps.LastActivity, ps.BadgeCount,
//        DENSE_RANK() OVER (ORDER BY (ps.UpVoteCount - ps.DownVoteCount) DESC, ps.CommentCount DESC) AS Rank FROM PostStats ps)
// SELECT pr.Id, pr.Title, pr.CommentCount, pr.AnswerCount, pr.UpVoteCount, pr.DownVoteCount, pr.Rank, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName, u.Reputation
// FROM PostRanked pr LEFT JOIN Users u ON pr.Id = u.AccountId WHERE pr.Rank <= 100 ORDER BY pr.Rank;
//
// `pr.Id = u.AccountId` joins a post id to an account id, so it goes through the raw ids.
fn q8390(db: &'static So) -> String {
    let owner_user = &db.post.owner_user;
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 2], |a, (((_, _), t), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&ps).and(&cc)), |&(_, (a, c))| (Reverse(a[0] - a[1]), Reverse(c)), true);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 100).map(|((p, (a, c)), r)| (p, (a, c, r))).collect());
    let tp: HashIdx<Id<Post>, (Id<Post>, ([i64; 2], i64, i64))> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let ac = (&tp).map(|(p, _)| p).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let acct: HashIdx<i64, Id<User>> = (&db.user.account_id).inv().collect();
    let v = drain((&tp).and(&ac).and((&db.post.origid).select(&acct).opt()));
    rows(v.into_iter().map(|(p, (((_, (a, c, r)), n), u))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1]), V::I(r)]);
        f.extend(match u {
            Some(u) => ucols(db, u, &["name", "rep"]),
            None => vec![V::S("Anonymous"), V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' AND p.ViewCount > 100 AND p.PostTypeId IN (1, 2)),
// PostStats AS (SELECT p.PostId, p.Title, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(c.Id) AS CommentCount FROM RankedPosts p LEFT JOIN Votes v ON p.PostId = v.PostId LEFT JOIN Comments c ON p.PostId = c.PostId GROUP BY p.PostId, p.Title, p.ViewCount)
// SELECT s.PostId, s.Title, s.ViewCount, s.UpVotes, s.DownVotes, s.CommentCount, CASE WHEN rp.Rank <= 5 THEN 'Top Post' ELSE 'Regular Post' END AS PostRank
// FROM PostStats s JOIN RankedPosts rp ON s.PostId = rp.PostId ORDER BY s.UpVotes DESC, s.DownVotes ASC, s.ViewCount DESC;
fn q5034(db: &'static So) -> String {
    let Post { creation_date, view_count, post_type_id, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30)).and(view_count.gt(100)).and(post_type_id.is_in([1, 2]))).with(owner_user).select(post_type_id));
    let v = ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), false);
    let v = per_group(v, |&(_, t)| t);
    let tr = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rp: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let s = (&rp).map(|(p, _)| p).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 3], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]
    });
    let v = drain((&rp).and(&s));
    rows(v.into_iter().map(|(p, ((_, r), a))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 5 { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.CommentCount, fp.UpVoteCount - fp.DownVoteCount AS NetVote,
//        CASE WHEN fp.Score > 10 THEN 'Highly Engaging' WHEN fp.Score BETWEEN 5 AND 10 THEN 'Moderately Engaging' ELSE 'Needs Improvement' END AS EngagementLevel
// FROM FilteredPosts fp ORDER BY fp.Score DESC;
//
// Rank reads only base columns, so the newest posts are picked first and the comments x votes product is driven for them alone.
fn q6906(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain(&s);
    rows(v.into_iter().map(|(p, a)| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1] - a[2]), V::S(if sc > 10 { "Highly Engaging" } else if sc >= 5 { "Moderately Engaging" } else { "Needs Improvement" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'))
// SELECT us.DisplayName, us.Reputation, us.PostCount, us.QuestionCount, us.AnswerCount, us.TotalUpvotes, us.TotalDownvotes, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate
// FROM UserStats us LEFT JOIN RecentPosts rp ON us.UserId = rp.OwnerUserId AND rp.rn = 1 ORDER BY us.Reputation DESC LIMIT 10;
//
// The LIMIT reads only Reputation, so the ten users are picked first and the posts x votes product is driven for them alone.
fn q5708(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let v = top_n(drain(db.user.with(rep.gt(1000)).select(rep)), |&(u, r)| (Reverse(r), u), 10);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = drain((&top).select(posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))));
    let first = top_per(recent, |&(u, _)| u, |&(_, p)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first = rel(first.into_iter().map(|(_, p)| p).collect());
    let by_user: HashIdx<Id<User>, Id<Post>> = (&first).select(owner_user).inv().select(&first).collect();
    let v = drain((&us).and(&user_distinct_posts(db)).and((&by_user).opt()));
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, u.DisplayName AS OwnerDisplayName, u.Reputation, u.CreationDate AS UserCreationDate
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id ORDER BY tp.Score DESC;
//
// Rank reads only base columns, so the top posts are picked first. `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q5464(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&s).and(origid.select(&uidx)));
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep", "ucreated"]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, COUNT(DISTINCT b.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, TotalUpvotes, TotalDownvotes, TotalPosts, TotalQuestions, TotalAnswers, TotalBadges, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserStats)
// SELECT UserId, DisplayName, Reputation, TotalUpvotes, TotalDownvotes, TotalPosts, TotalQuestions, TotalAnswers, TotalBadges, ReputationRank FROM RankedUsers
// WHERE TotalPosts > 10 ORDER BY ReputationRank FETCH FIRST 50 ROWS ONLY;
//
// The rank, the filter and the cut read only Reputation and the distinct post count, so the fifty users are picked first and the posts x votes x badges product is driven for them alone.
fn q5505(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let v = ranked(drain(&pc), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let r = rel(v.into_iter().map(|((u, a), r)| (u, a, r)).collect());
    type T = (Id<User>, [i64; 3], i64);
    let v = drain((&r).with(Same::<T>::new().filt(|(_, a, _): T| a[0] > 10)));
    let v = top_n(v, |&(_, (u, _, r))| (r, u), 50);
    let tr = rel(v.into_iter().map(|x| x.1).collect());
    let top: HashIdx<Id<User>, T> = (&tr).map(|(u, _, _): T| u).inv().select(&tr).collect();
    let us = (&top).map(|(u, _, _)| u).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (t, _)| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let bc = (&top).map(|(u, _, _)| u).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&top).and(&us).and(&bc));
    rows(v.into_iter().map(|(u, (((_, p, r), a), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.ViewCount, u.DisplayName, p.OwnerUserId, p.CreationDate),
// FilteredPosts AS (SELECT PostId, Title, ViewCount, OwnerDisplayName, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE rn = 1 AND ViewCount > 100),
// TopPosts AS (SELECT *, RANK() OVER (ORDER BY ViewCount DESC) AS ViewRank FROM FilteredPosts)
// SELECT tp.Title, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes, COALESCE(tp.UpVotes - tp.DownVotes, 0) AS NetScore,
//        CASE WHEN tp.UpVotes IS NULL THEN 'No Votes' ELSE 'Voted' END AS VoteStatus
// FROM TopPosts tp WHERE tp.ViewRank <= 10 ORDER BY tp.ViewCount DESC;
//
// Both ranks read only base columns, so the posts are picked first and the comments x votes product is driven for them alone.
// UpVotes is a SUM of a CASE over at least one joined row, so it is never NULL and VoteStatus is always 'Voted'.
fn q4313(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, .. } = &db.post;
    let first = top_per(drain(db.post.select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = ranked(drain((&first).with(view_count.gt(100)).select(view_count)), |&(_, w)| Reverse(w), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain(&s);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2]), V::S("Voted")]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven,
//        SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentsMade, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvotesReceived,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvotesReceived, COUNT(DISTINCT B.Id) AS BadgesCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionsAsked, AnswersGiven, CommentsMade, UpvotesReceived, DownvotesReceived, BadgesCount,
//        ROW_NUMBER() OVER (ORDER BY UpvotesReceived - DownvotesReceived DESC) AS Rank FROM UserActivity)
// SELECT TU.Rank, TU.DisplayName, TU.QuestionsAsked, TU.AnswersGiven, TU.CommentsMade, TU.UpvotesReceived, TU.DownvotesReceived, TU.BadgesCount FROM TopUsers TU WHERE TU.Rank <= 10 ORDER BY TU.Rank;
fn q5108(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, _)| match p {
            Some(((t, c), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&ua).and((&bc).opt())), |&(u, (a, _))| (Reverse(a[3] - a[4]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, (a, b)))| {
        let mut f = vec![V::I(i as i64 + 1), user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.push(V::I(b.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, U.DisplayName AS OwnerName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users U ON p.OwnerUserId = U.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, U.DisplayName, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.OwnerName, rp.UpVotes, rp.DownVotes, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT fp.OwnerName, COUNT(fp.PostId) AS PostCount, SUM(fp.ViewCount) AS TotalViews, SUM(fp.UpVotes) AS TotalUpVotes, SUM(fp.DownVotes) AS TotalDownVotes, SUM(fp.CommentCount) AS TotalComments
// FROM FilteredPosts fp GROUP BY fp.OwnerName ORDER BY TotalUpVotes DESC, PostCount DESC;
//
// Rank reads only base columns, so each owner's five newest questions are picked first and the votes x comments product is driven for them alone.
fn q27224(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let fp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 2], |a, (t, _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let cc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let g = (&fp).group_by(owner_user.select(&db.user.display_name).opt()).select(view_count.opt().and(&vc).and(&cc)).fold([0i64; 6], |a, ((w, v), c)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + v[0], a[4] + v[1], a[5] + c]
    });
    let v = drain(&g);
    rows(v.into_iter().map(|(n, a)| row(vec![harness::fmt::ostr(n), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::I(a[4]), V::I(a[5])])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// RecentActivePosts AS (SELECT p.Id, p.Title, p.ViewCount, p.CreationDate, RANK() OVER (ORDER BY p.LastActivityDate DESC) AS ActivityRank FROM Posts p WHERE p.LastActivityDate IS NOT NULL)
// SELECT rp.Title, rp.CreationDate, u.DisplayName AS OwnerDisplayName, COALESCE(rap.ViewCount, 0) AS ViewCount, COALESCE(rp.CommentCount, 0) AS TotalComments, rp.UpVotes,
//        CASE WHEN rp.UserPostRank = 1 THEN 'Latest Post' ELSE 'Previous Post' END AS PostStatus
// FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN RecentActivePosts rap ON rp.Id = rap.Id
// WHERE u.Reputation > 1000 AND rap.ActivityRank <= 10 ORDER BY rp.UpVotes DESC, rp.CreationDate DESC LIMIT 50;
//
// ActivityRank reads only LastActivityDate, so those posts are picked first and the comments x votes product is driven for them alone.
fn q3090(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, last_activity_date, view_count, .. } = &db.post;
    let v = ranked(drain(last_activity_date), |&(_, d)| Reverse(d), false);
    let rap: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false);
    let v = per_group(v, |&(_, u)| u);
    let r = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let upr: HashIdx<Id<Post>, (Id<Post>, i64)> = (&r).map(|(p, _)| p).inv().select(&r).collect();
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let cand = (&rap).with(post_type_id.eq(1)).with(owner_user.select(rich));
    let s = cand.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]
    });
    let v = drain((&s).and(&upr));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[1]), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (a, (_, r)))| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend([V::I(view_count.get(p).unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::S(if r == 1 { "Latest Post" } else { "Previous Post" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostCommentSummary AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, ur.Reputation, ur.BadgeCount, COALESCE(pcs.TotalComments, 0) AS TotalComments, pcs.LastCommentDate,
//        CASE WHEN rp.Rank = 1 THEN 'Top Question' WHEN rp.Rank <= 5 THEN 'High Interest' ELSE 'Low Interest' END AS InterestLevel
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostCommentSummary pcs ON rp.PostId = pcs.PostId WHERE ur.Reputation >= 1000
// ORDER BY rp.ViewCount DESC, ur.Reputation DESC;
fn q3873(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let v = ranked(v, |&(p, u)| {
        let w = view_count.get(p);
        (u, w.is_none(), Reverse(w), p)
    }, false);
    let v = per_group(v, |&(_, u)| u);
    let r = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rp: HashIdx<Id<Post>, (Id<Post>, i64)> = (&r).map(|(p, _)| p).inv().select(&r).collect();
    let rich = Ident::<User>::new().with((&db.user.reputation).ge(1000));
    let bc = db.user.with((&db.user.reputation).ge(1000)).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pcs = (&rp).map(|(p, _)| p).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&rp).and(owner_user.select(rich.and(&bc))).and((&pcs).opt()));
    rows(v.into_iter().map(|(p, (((_, r), (u, b)), c))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([user_col(db, u, "rep"), V::I(b)]);
        f.extend(match c {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        f.push(V::S(if r == 1 { "Top Question" } else if r <= 5 { "High Interest" } else { "Low Interest" }));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("8271", q8271),
    ("2499", q2499),
    ("3", q3),
    ("13256", q13256),
    ("28669", q28669),
    ("31527", q31527),
    ("4581", q4581),
    ("5149", q5149),
    ("3346", q3346),
    ("1189", q1189),
    ("6374", q6374),
    ("7597", q7597),
    ("7847", q7847),
    ("9165", q9165),
    ("7913", q7913),
    ("8139", q8139),
    ("7621", q7621),
    ("22445", q22445),
    ("1498", q1498),
    ("5473", q5473),
    ("7315", q7315),
    ("8001", q8001),
    ("8096", q8096),
    ("7246", q7246),
    ("8851", q8851),
    ("5552", q5552),
    ("8307", q8307),
    ("9261", q9261),
    ("5449", q5449),
    ("6505", q6505),
    ("8649", q8649),
    ("2289", q2289),
    ("9838", q9838),
    ("4463", q4463),
    ("6651", q6651),
    ("206", q206),
    ("1285", q1285),
    ("1487", q1487),
    ("5318", q5318),
    ("8008", q8008),
    ("7835", q7835),
    ("9017", q9017),
    ("360", q360),
    ("3100", q3100),
    ("6995", q6995),
    ("8745", q8745),
    ("1254", q1254),
    ("9257", q9257),
    ("5596", q5596),
    ("8867", q8867),
    ("10881", q10881),
    ("22635", q22635),
    ("2936", q2936),
    ("7113", q7113),
    ("6401", q6401),
    ("568", q568),
    ("8709", q8709),
    ("1810", q1810),
    ("7719", q7719),
    ("4925", q4925),
    ("420", q420),
    ("7563", q7563),
    ("8827", q8827),
    ("23", q23),
    ("3954", q3954),
    ("6176", q6176),
    ("8719", q8719),
    ("8580", q8580),
    ("131", q131),
    ("2195", q2195),
    ("5846", q5846),
    ("2865", q2865),
    ("4053", q4053),
    ("7905", q7905),
    ("8390", q8390),
    ("5034", q5034),
    ("6906", q6906),
    ("5708", q5708),
    ("5464", q5464),
    ("5505", q5505),
    ("4313", q4313),
    ("5108", q5108),
    ("27224", q27224),
    ("3090", q3090),
    ("3873", q3873),
];
