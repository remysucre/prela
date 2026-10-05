use harness::prelude::*;
use std::cmp::Reverse;

fn by_first<A: Copy + Eq + std::hash::Hash, B: Copy + Eq + std::hash::Hash>(m: &MatSet<(A, B)>) -> HashIdx<A, B> {
    m.map(|(a, _)| a).inv().map(|(_, b): (A, B)| b).collect()
}

fn runs(s: Str) -> Vec<Str> {
    s.split(['<', '>']).collect()
}

/// The (post, tag) pairs with `p.Tags LIKE '%' || t.TagName || '%'`. A name free of '<', '>', '%' and '_' can only occur inside one
/// run of Tags between angle brackets, so it is matched against those runs; any other name goes through a real LIKE over every distinct Tags string.
fn like_tags(db: &'static So) -> MatSet<(Id<Post>, Id<Tag>)> {
    let name = &db.tag.tag_name;
    let plain = |n: Str| !n.contains(['<', '>', '%', '_']);
    let named: HashIdx<Str, Id<Tag>> = db.tag.with(name.filt(plain)).select(name).inv().collect();
    let odd: HashIdx<Str, Id<Tag>> = db.tag.with(name.filt(move |n| !plain(n))).select(name).inv().collect();
    let tags = &db.post.tags_str;
    let rs: MatSet<Str> = tags.flat_map(runs).collect();
    let inside: HashIdx<Str, Id<Tag>> = (&rs).select_where(&named, |r: Str, n: Str| r.contains(n)).collect();
    let strs: MatSet<Str> = tags.collect();
    let hit: HashIdx<Str, Id<Tag>> = (&strs).select_where(&odd, |s: Str, n: Str| like(s, &format!("%{n}%"))).collect();
    db.post.select(Ident::<Post>::new().and(tags.flat_map(runs).select(&inside))).union(db.post.select(Ident::<Post>::new().and(tags.select(&hit)))).collect()
}

// WITH RecursiveBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// RecentPostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(c.CountComments, 0) AS CommentCount, COALESCE(v.UpvoteCount, 0) AS UpvoteCount,
//        COALESCE(v.DownvoteCount, 0) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CountComments FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 month')),
// PostDerivedData AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, COALESCE(bc.BadgeCount, 0) AS BadgeCount, COALESCE(rp.CommentCount, 0) AS CommentCount,
//        COALESCE(rp.UpvoteCount, 0) AS UpvoteCount, COALESCE(rp.DownvoteCount, 0) AS DownvoteCount
//     FROM Posts p LEFT JOIN RecursiveBadgeCounts bc ON p.OwnerUserId = bc.UserId LEFT JOIN RecentPostActivity rp ON p.Id = rp.PostId WHERE p.OwnerUserId IS NOT NULL)
// SELECT p.PostId, p.Title, p.CreationDate, p.CommentCount, p.UpvoteCount, p.DownvoteCount, (p.UpvoteCount - p.DownvoteCount) AS Score,
//        CASE WHEN p.BadgeCount > 0 THEN 'Active Contributor' WHEN p.CommentCount > 10 THEN 'Frequent Commenter' ELSE 'New User' END AS UserType
// FROM PostDerivedData p WHERE (p.UpvoteCount - p.DownvoteCount) > 0 ORDER BY Score DESC, p.CreationDate DESC LIMIT 50;
//
// A post outside RecentPostActivity has 0 up and 0 down votes and fails the WHERE, so only the recent posts are grouped. rn is never read.
fn q30203(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, .. } = &db.post;
    let recent = || db.post.with(owner_user_id).with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)));
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&vc).filt(|a| a[0] - a[1] > 0).and(&cc).and(owner_user_id.select(&bc).opt()));
    let v = top_n(v, |&(p, ((a, _), _))| (Reverse(a[0] - a[1]), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((a, c), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        f.push(V::S(if b.unwrap_or(0) > 0 { "Active Contributor" } else if c > 10 { "Frequent Commenter" } else { "New User" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT B.Id) AS BadgeCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) DESC) AS ActivityRank
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, COALESCE(COUNT(CM.Id), 0) AS CommentCount, P.ViewCount
//     FROM Posts P LEFT JOIN Comments CM ON P.Id = CM.PostId WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
//     GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId, P.Score, P.ViewCount),
// ActiveUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, UpVotes, DownVotes FROM UserStats WHERE ActivityRank <= 10),
// UserPostStats AS (SELECT AU.UserId, AU.DisplayName, RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.CommentCount, RP.ViewCount
//     FROM ActiveUsers AU JOIN RecentPosts RP ON AU.UserId = RP.OwnerUserId)
// SELECT U.DisplayName AS UserName, U.Reputation, U.BadgeCount, U.UpVotes, U.DownVotes, P.Title AS RecentPostTitle, P.CreationDate AS PostCreationDate, P.Score,
//        P.CommentCount, P.ViewCount
// FROM UserPostStats P JOIN ActiveUsers U ON P.UserId = U.UserId ORDER BY U.Reputation DESC, P.CreationDate DESC;
//
// ActivityRank partitions by the user, one row each, so it is always 1 and ActiveUsers is every user. Only owners of recent posts reach the output,
// so the badge x vote product is driven for those users alone.
fn q3491(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let rp = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let owners: MatSet<Id<User>> = recent().select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bd = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&rp).and(owner_user.select(Ident::<User>::new().and(&us).and(&bd))));
    rows(v.into_iter().map(|(p, (c, ((u, a), b)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["views"]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Views, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats),
// ClosedPosts AS (SELECT PH.PostId, COUNT(PH.Id) AS CloseCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId),
// DetailedPostStats AS (SELECT P.Id AS PostId, P.Title, P.Score, COALESCE(CP.CloseCount, 0) AS CloseCount, P.CreationDate, RANK() OVER (ORDER BY P.Score DESC) AS ScoreRank
//     FROM Posts P LEFT JOIN ClosedPosts CP ON P.Id = CP.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT TU.DisplayName, TU.Reputation, DP.Title, DP.Score, DP.CloseCount, DP.CreationDate, CASE WHEN DP.CloseCount > 0 THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM TopUsers TU JOIN DetailedPostStats DP ON TU.UserId = DP.PostId WHERE TU.Rank <= 10 ORDER BY TU.Reputation DESC, DP.Score DESC;
//
// Rank reads only Reputation, and none of UserStats' aggregates is projected, so the ten users are picked from Users directly.
// `TU.UserId = DP.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q371(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let dp = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).and((&cp).opt());
    let v = drain((&tu).select((&db.user.origid).select(&pidx).select(dp)));
    rows(v.into_iter().map(|(u, (p, c))| {
        let c = c.unwrap_or(0);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(score.get(p).unwrap()), V::I(c)]);
        f.extend(post_fields(db, p, &["created"]));
        f.push(V::S(if c > 0 { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// BadgeCounts AS (SELECT B.UserId, COUNT(*) AS TotalBadges FROM Badges B GROUP BY B.UserId),
// PostHistoryCounts AS (SELECT PH.UserId, COUNT(*) AS TotalPostHistory FROM PostHistory PH WHERE PH.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY PH.UserId),
// UserRankings AS (SELECT U.UserId, U.DisplayName, U.Reputation, U.TotalPosts, U.QuestionsCount, U.AnswersCount, U.TotalScore, U.AvgViewCount, COALESCE(BC.TotalBadges, 0) AS TotalBadges,
//        COALESCE(PH.TotalPostHistory, 0) AS TotalPostHistory, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank
//     FROM UserStatistics U LEFT JOIN BadgeCounts BC ON U.UserId = BC.UserId LEFT JOIN PostHistoryCounts PH ON U.UserId = PH.UserId)
// SELECT RR.UserId, RR.DisplayName, RR.Reputation, RR.TotalPosts, RR.QuestionsCount, RR.AnswersCount, RR.TotalScore, ROUND(RR.AvgViewCount, 2) AS AvgViewCount, RR.TotalBadges,
//        RR.TotalPostHistory, RR.Rank, CASE WHEN RR.TotalPosts = 0 THEN 'No Posts' WHEN RR.AnswersCount > RR.QuestionsCount THEN 'More Answers' ELSE 'More Questions' END AS PostTypeFeedback
// FROM UserRankings RR ORDER BY RR.Rank LIMIT 10;
fn q1317(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { user, creation_date, .. } = &db.post_history;
    let phc = db.post_history.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, k)).collect();
    let rank = by_first(&rk);
    let v = drain((&rank).and(&ups).and((&bc).opt()).and((&phc).opt()));
    rows(v.into_iter().map(|(u, (((k, a), b), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1])]);
        f.push(if a[5] == 0 { V::Null } else { V::F((a[6] as f64 / a[5] as f64 * 100.0).round() / 100.0) });
        f.extend([V::I(b.unwrap_or(0)), V::I(h.unwrap_or(0)), V::I(k)]);
        f.push(V::S(if a[1] == 0 { "No Posts" } else if a[3] > a[2] { "More Answers" } else { "More Questions" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, COALESCE(ph.Comment, 'No close reason applicable') AS CloseReason,
//        ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS ViewRank, RANK() OVER (ORDER BY p.CreationDate DESC) AS RecentRank,
//        DENSE_RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10
//     WHERE p.CreationDate > '2020-01-01' AND (p.ViewCount > 100 OR ph.Comment IS NOT NULL)),
// AggregatedData AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, COUNT(DISTINCT p.Id) AS PostCount, COUNT(b.Id) AS BadgeCount,
//        AVG(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS AvgScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName)
// SELECT a.UserId, a.DisplayName, a.TotalViews, a.PostCount, a.BadgeCount, a.AvgScore, r.PostId, r.Title, r.ViewCount, r.CloseReason,
//        CASE WHEN r.ViewRank IS NULL THEN 'No posts ranked' ELSE 'Ranked as ' || r.ViewRank::TEXT || ' by views' END AS ViewRankComment,
//        CASE WHEN r.RecentRank < 10 THEN 'Recent high activity' ELSE 'Less recent activity' END AS RecentActivityComment,
//        CASE WHEN r.ScoreRank = 1 THEN 'Top scorer!' ELSE 'Score below top rank' END AS ScoreComment
// FROM AggregatedData a LEFT JOIN RankedPosts r ON a.UserId = r.PostId WHERE a.TotalViews > 500
// ORDER BY a.BadgeCount DESC, a.TotalViews DESC, r.ViewCount DESC NULLS LAST OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY;
//
// `a.UserId = r.PostId` joins a user id to a post id, so it goes through the raw ids. ViewRank numbers joined rows whose ViewCount ties;
// the port breaks that tie by post and history id (the SQL leaves it open).
fn q21365(db: &'static So) -> String {
    let Post { creation_date, view_count, score, owner_user_id, .. } = &db.post;
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let ph10 = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)));
    type J = (((Id<Post>, i64), i64), (Option<i64>, Option<(Id<PostHistory>, Option<Str>)>));
    let rp: MatSet<J> = db
        .post
        .with(creation_date.gt(ts(2020, 1, 1, 0, 0, 0)))
        .select(Ident::<Post>::new().and(creation_date).and(score).and(view_count.opt().and(ph10.select(Ident::<PostHistory>::new().and(comment.opt())).opt())))
        .filt(|(_, (w, h)): J| w.map_or(false, |w| w > 100) || h.map_or(false, |(_, c)| c.is_some()))
        .collect();
    type X = (J, i64);
    type Y = (X, i64);
    let w = whole(&rp).window(rank, |(((_, d), _), _): J| Reverse(d), asc);
    let w = (&w).window(dense_rank, |(((_, s), _), _): X| Reverse(s), asc);
    let w = (&w)
        .group_by(Same::<Y>::new().map(|y: Y| y.0 .0 .0 .0 .0).select(owner_user_id.opt()))
        .select(Same::<Y>::new())
        .window(row_number, |(((((p, _), _), (w, h)), _), _): Y| (w.is_none(), Reverse(w), p, h.map(|x| x.0)), asc);
    type R = (Id<Post>, Option<i64>, Option<Option<Str>>, i64, i64, i64);
    let rk: MatSet<R> = (&w).map(|((((((p, _), _), (w, h)), rr), sr), vr)| (p, w, h.map(|x| x.1), rr, sr, vr)).collect();
    let by_id: HashIdx<i64, R> = (&rk).select(Same::<R>::new().map(|x: R| x.0).select(&db.post.origid)).inv().collect();
    let agg = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (s, w) = p.map_or((0, None), |(s, w)| (s, w));
            [a[0] + w.unwrap_or(0), a[1] + b.is_some() as i64, a[2] + s, a[3] + 1]
        });
    let pc = db.user.with((&db.user.reputation).gt(100)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&agg).filt(|a| a[0] > 500).and(&pc).and((&db.user.origid).select(&by_id).opt()));
    let v = top_n(v, |&(u, ((a, _), r))| {
        let w = r.and_then(|r| r.1);
        (Reverse(a[1]), Reverse(a[0]), w.is_none(), Reverse(w), u, r.map(|r| (r.0, r.5)))
    }, 15);
    rows(v.into_iter().skip(5).map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(n), V::I(a[1]), avg(a[2], a[3])]);
        match r {
            Some((p, _, c, rr, sr, vr)) => {
                f.extend(post_fields(db, p, &["id", "title", "views"]));
                f.push(V::S(c.flatten().unwrap_or("No close reason applicable")));
                f.push(V::Owned(format!("Ranked as {vr} by views")));
                f.push(V::S(if rr < 10 { "Recent high activity" } else { "Less recent activity" }));
                f.push(V::S(if sr == 1 { "Top scorer!" } else { "Score below top rank" }));
            }
            None => {
                f.extend([V::Null, V::Null, V::Null, V::Null]);
                f.extend([V::S("No posts ranked"), V::S("Less recent activity"), V::S("Score below top rank")]);
            }
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months'
//     GROUP BY u.Id, u.Reputation),
// PostHistoryInfo AS (SELECT ph.PostId, ph.UserId, MAX(ph.CreationDate) AS LastEditDate, COUNT(*) AS EditHistoryCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6)
//     GROUP BY ph.PostId, ph.UserId)
// SELECT up.UserId, up.Reputation, ra.PostId, ra.Title, ra.Score, ra.ViewCount, ra.CreationDate, up.CommentCount, up.UpvoteCount, up.DownvoteCount, ph.LastEditDate, ph.EditHistoryCount,
//        CASE WHEN up.Reputation > 1000 THEN 'Expert' WHEN up.Reputation BETWEEN 500 AND 1000 THEN 'Enthusiast' ELSE 'Novice' END AS UserLevel
// FROM UserActivity up INNER JOIN RankedPosts ra ON up.UserId = ra.PostId LEFT JOIN PostHistoryInfo ph ON ra.PostId = ph.PostId AND up.UserId = ph.UserId
// WHERE ra.Score > 10 AND (up.CommentCount IS NOT NULL OR up.UpvoteCount > 0) AND (ph.EditHistoryCount IS NULL OR ph.EditHistoryCount > 2)
// ORDER BY ra.Score DESC, up.Reputation DESC LIMIT 100 OFFSET 0;
//
// `up.UserId = ra.PostId` joins a user id to a post id, so it goes through the raw ids. ScoreRank is never read, and `up.CommentCount IS NOT NULL`
// always holds (a COUNT), so only the users paired with a post are aggregated.
fn q20866(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let old = Ident::<User>::new().with((&db.user.creation_date).lt(add_months(t0, -6)));
    let pairs = drain(db.post.with(creation_date.gt(add_years(t0, -1)).and(score.gt(10))).select((&db.post.origid).select(&uidx).select(old)));
    let cand: MatSet<Id<User>> = rel(pairs.clone()).map(|(_, u)| u).collect();
    let ua = (&cand)
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&cand).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, user, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phi = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post.and(user)).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    type P = (Id<Post>, Id<User>);
    let pv = rel(pairs);
    let v = drain(
        (&pv)
            .select(Same::<P>::new().and(Same::<P>::new().select(&phi).opt()))
            .filt(|(_, h): (P, Option<(i64, i64)>)| h.map_or(true, |(n, _)| n > 2))
            .and((&pv).map(|(_, u): P| u).select((&ua).and(&cc))),
    );
    let v = top_n(v, |&(_, (((p, u), _), _))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p, u), 100);
    rows(v.into_iter().map(|(_, (((p, u), h), (a, c)))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "created"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some((n, d)) => [V::T(d), V::I(n)],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if r > 1000 { "Expert" } else if r >= 500 { "Enthusiast" } else { "Novice" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank,
//        CASE WHEN U.Reputation IS NULL THEN 'No Reputation' WHEN U.Reputation > 1000 THEN 'High Reputation' WHEN U.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation'
//        ELSE 'Low Reputation' END AS ReputationCategory FROM Users U),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers
//     FROM Posts P GROUP BY P.OwnerUserId),
// VotedPosts AS (SELECT V.PostId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes V GROUP BY V.PostId),
// RankedPosts AS (SELECT P.Id AS PostId, P.Title, PS.TotalPosts, PS.Questions, PS.Answers, COALESCE(VP.VoteCount, 0) AS VoteCount, COALESCE(VP.UpVotes, 0) AS UpVotes,
//        COALESCE(VP.DownVotes, 0) AS DownVotes, RANK() OVER (ORDER BY COALESCE(VP.VoteCount, 0) DESC) AS PostRank, P.OwnerUserId
//     FROM Posts P LEFT JOIN PostStats PS ON P.OwnerUserId = PS.OwnerUserId LEFT JOIN VotedPosts VP ON P.Id = VP.PostId
//     WHERE P.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') AND (P.Score IS NOT NULL AND P.Score > 0))
// SELECT UR.UserId, UR.Reputation, UR.ReputationCategory, RP.PostId, RP.Title, RP.TotalPosts, RP.VoteCount, RP.UpVotes, RP.DownVotes, RP.PostRank
// FROM UserReputation UR JOIN RankedPosts RP ON UR.UserId = RP.OwnerUserId WHERE UR.Rank <= 10 AND RP.PostRank <= 5 ORDER BY RP.PostRank, UR.Reputation DESC;
fn q24867(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let recent = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)));
    let vp = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let w = whole(&vp).select(Ident::<Post>::new().and(&vp)).window(rank, |(_, a)| Reverse(a[0]), asc);
    type R = (Id<Post>, [i64; 3], i64);
    let rk: MatSet<R> = (&w).filt(|(_, k)| k <= 5).map(|((p, a), k)| (p, a, k)).collect();
    let top: HashIdx<Id<User>, R> = (&rk).select(Same::<R>::new().map(|x: R| x.0).select(owner_user)).inv().collect();
    let tp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&tu).select(Ident::<User>::new().and(&top).and(&tp)));
    rows(v.into_iter().map(|(_, ((u, (p, a, k)), n))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::S(if rep > 1000 { "High Reputation" } else if rep >= 500 { "Medium Reputation" } else { "Low Reputation" }));
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(k)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// PostAnalytics AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, COUNT(C.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRow,
//        P.CreationDate FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id, P.Title, P.ViewCount, P.Score, P.CreationDate, P.OwnerUserId),
// ClosedPosts AS (SELECT PH.PostId, MIN(PH.CreationDate) AS FirstClosureDate, COUNT(PH.Id) AS CloseCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.TotalUpvotes, U.TotalDownvotes, U.TotalPosts, U.TotalComments, P.PostId, P.Title, P.ViewCount, P.Score, P.CommentCount, P.RecentPostRow,
//        C.FirstClosureDate, C.CloseCount
// FROM UserStats U LEFT JOIN PostAnalytics P ON U.UserId = P.PostId LEFT JOIN ClosedPosts C ON P.PostId = C.PostId
// WHERE (U.Reputation > 100 OR U.CreationDate < (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')) AND P.RecentPostRow <= 5
// ORDER BY U.Reputation DESC, P.ViewCount DESC LIMIT 50;
//
// `U.UserId = P.PostId` joins a user id to a post id, so it goes through the raw ids. RecentPostRow breaks CreationDate ties by post id (the SQL leaves them open).
fn q1240(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, view_count, origid, .. } = &db.post;
    let w = db.post.group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date).and(origid)).window(row_number, |((p, d), _)| (Reverse(d), p), asc);
    type R = (Id<Post>, i64, i64);
    let rk: MatSet<R> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), o), k)| (p, o, k)).collect();
    let by_id: HashIdx<i64, R> = (&rk).select(Same::<R>::new().map(|x: R| x.1)).inv().collect();
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let users: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(100).or((&db.user.creation_date).lt(add_years(t0, -1)))).with((&db.user.origid).select(&by_id)).collect();
    let us = (&users)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, x| {
            let t = x.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let pc = (&users).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ccu = (&users).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold((0i64, i64::MAX), |(n, m), d| (n + 1, m.min(d)));
    let pa = Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(&cc)).and(Same::<R>::new().map(|x: R| x.0).select(&cp).opt());
    let v = drain((&us).and(&pc).and(&ccu).and((&db.user.origid).select(&by_id).select(pa)));
    let v = top_n(v, |&(u, (_, (((p, _, _), _), _)))| {
        let w = view_count.get(p);
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w), u, p)
    }, 50);
    rows(v.into_iter().map(|(u, (((a, n), c), (((p, _, k), pcc), cl)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::I(c)]);
        f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
        f.extend([V::I(pcc), V::I(k)]);
        f.extend(match cl {
            Some((n, d)) => [V::T(d), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 0),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(DISTINCT b.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(*) AS HistoryCount, COUNT(DISTINCT ph.UserId) AS UniqueEditors FROM PostHistory ph
//     WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' GROUP BY ph.PostId)
// SELECT rp.Title, rp.Score, rp.ViewCount, us.DisplayName AS OwnerName, us.TotalPosts, us.TotalUpVotes, us.TotalDownVotes, COALESCE(phs.HistoryCount, 0) AS PostHistoryCount,
//        COALESCE(phs.UniqueEditors, 0) AS UniqueEditorsCount,
//        CASE WHEN us.TotalUpVotes - us.TotalDownVotes > 10 THEN 'Highly Voted' WHEN us.TotalUpVotes - us.TotalDownVotes BETWEEN 5 AND 10 THEN 'Moderately Voted' ELSE 'Low Engagement' END AS EngagementLevel
// FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId LEFT JOIN PostHistoryStats phs ON rp.PostId = phs.PostId WHERE rp.rn <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// rn reads only base columns, so the posts are picked first and the posts x votes x badges product is driven for their owners alone.
fn q32303(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db
        .post
        .with(creation_date.ge(add_years(t0, -1)).and(score.gt(0)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { post, user_id, creation_date: hd, .. } = &db.post_history;
    let recent_h = || db.post_history.with(hd.ge(add_months(t0, -6)));
    let hc = recent_h().group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let ue = recent_h().group_by(post).select(user_id).count_distinct();
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&us).and(&pc))).and((&hc).opt()).and((&ue).opt())));
    rows(v.into_iter().map(|(_, (((p, ((u, a), n)), hc), ue))| {
        let d = a[0] - a[1];
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(hc.unwrap_or(0)), V::I(ue.unwrap_or(0))]);
        f.push(V::S(if d > 10 { "Highly Voted" } else if (5..=10).contains(&d) { "Moderately Voted" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COALESCE(SUM(rp.Score), 0) AS UserPostScore, COUNT(rp.PostId) AS PostCount,
//        AVG(CASE WHEN rp.UserPostRank = 1 THEN rp.Score END) AS AvgTopPostScore
//     FROM Users u LEFT JOIN RecentPosts rp ON u.Id = rp.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// TopEngagedUsers AS (SELECT ue.UserId, ue.DisplayName, ue.Reputation, ue.Views, ue.UserPostScore, ue.PostCount, ue.AvgTopPostScore,
//        RANK() OVER (ORDER BY ue.UserPostScore DESC) AS EngagementRank FROM UserEngagement ue)
// SELECT tu.DisplayName, tu.Reputation, tu.Views, tu.UserPostScore, tu.PostCount, tu.AvgTopPostScore,
//        CASE WHEN tu.EngagementRank <= 10 THEN 'Top Engager' WHEN tu.EngagementRank <= 20 THEN 'Moderate Engager' ELSE 'New Engager' END AS EngagementLevel
// FROM TopEngagedUsers tu WHERE tu.UserPostScore > 0 ORDER BY tu.UserPostScore DESC;
//
// RecentPosts' vote and comment aggregates are never read, so only its rows are built. UserPostRank breaks CreationDate ties by post id (the SQL leaves them open).
fn q34803(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, .. } = &db.post;
    let since = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let w = db.post.with(creation_date.ge(since)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(since))).select(score.and(Ident::<Post>::new().with(&first).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((s, f)) => [a[0] + s, a[1] + 1, a[2] + f.is_some() as i64, a[3] + if f.is_some() { s } else { 0 }],
            None => a,
        });
    let w = whole(&ue).select(Ident::<User>::new().and(&ue)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let r = drain((&w).filt(|((_, a), _)| a[0] > 0));
    rows(r.into_iter().map(|(_, ((u, a), k))| {
        let mut f = ucols(db, u, &["name", "rep", "uviews"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[3], a[2])]);
        f.push(V::S(if k <= 10 { "Top Engager" } else if k <= 20 { "Moderate Engager" } else { "New Engager" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.AcceptedAnswerId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '10 days' AND p.Score IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation > 1000
//     GROUP BY u.Id, u.Reputation),
// PostEditHistory AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, COUNT(*) OVER (PARTITION BY ph.PostId) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6)),
// ClosedPosts AS (SELECT p.Id AS PostId, p.Title, ph.Comment AS CloseReason, ph.CreationDate AS CloseDate FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, ur.Reputation, ur.TotalBounty, pe.EditCount, cp.CloseReason, cp.CloseDate,
//        CASE WHEN rp.AcceptedAnswerId IS NOT NULL THEN 'Accepted' WHEN rp.Score > 0 AND pe.EditCount > 0 THEN 'Edited and Popular'
//        WHEN cp.CloseReason IS NOT NULL THEN 'Closed: ' || cp.CloseReason ELSE 'Open' END AS PostState
// FROM RankedPosts rp LEFT JOIN UserReputation ur ON ur.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId LIMIT 1)
// LEFT JOIN PostEditHistory pe ON pe.PostId = rp.PostId LEFT JOIN ClosedPosts cp ON cp.PostId = rp.PostId
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.CreationDate ASC;
//
// The `LIMIT 1` subquery looks a post up by its primary key, so it is the post's owner. Each PostEditHistory row of a post joins, and carries the post's edit count.
fn q21879(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, accepted_answer_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.gt(add_days(ts(2024, 10, 1, 0, 0, 0), -10)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), d, p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let ur = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())
        .fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let edits = || db.post_history.with(post_history_type_id.is_in([4, 5, 6]));
    let ec = edits().group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pe = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6])));
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)));
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&ur)).opt().and(pe.opt().and(cp.opt())).and((&ec).opt())));
    rows(v.into_iter().map(|(p, ((u, (e, c)), n))| {
        let n = e.and(n);
        let reason = c.and_then(|c| comment.get(c));
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(match u {
            Some((u, b)) => [user_col(db, u, "rep"), V::I(b)],
            None => [V::Null, V::Null],
        });
        f.extend([oint(n), ostr(reason), ots(c.map(|c| db.post_history.creation_date.get(c).unwrap()))]);
        f.push(if accepted_answer_id.get(p).is_some() {
            V::S("Accepted")
        } else if score.get(p).unwrap() > 0 && n.map_or(false, |n| n > 0) {
            V::S("Edited and Popular")
        } else if let Some(r) = reason {
            V::Owned(format!("Closed: {r}"))
        } else {
            V::S("Open")
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, COALESCE(NULLIF(u.DisplayName, ''), 'Anonymous') AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.Score, u.DisplayName),
// RecentVotes AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes, COUNT(*) AS TotalVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId),
// PostRecommendations AS (SELECT pl.PostId AS SourcePostId, pl.RelatedPostId AS RecommendedPostId, lt.Name AS LinkType FROM PostLinks pl JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id
//     WHERE lt.Name IN ('Linked', 'Duplicate'))
// SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.Score, rp.OwnerDisplayName, rp.CommentCount, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes,
//        COALESCE(pr.RecommendedPostId, -1) AS SuggestedRelatedPostId, CASE WHEN rp.Score IS NULL THEN 'No Score' WHEN rp.Score > 100 THEN 'High Score' ELSE 'Moderate Score' END AS ScoreRank
// FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN PostRecommendations pr ON rp.PostId = pr.SourcePostId
// WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC LIMIT 50;
//
// rn partitions by the post itself, so it is always 1. Every post yields at least one row and the order is by the post's CreationDate, so the 50 newest posts
// are picked first and the link join is driven for those alone.
fn q21447(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.is_in([1, 2])).select(creation_date)), |&(p, d)| (Reverse(d), p), 50);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rvotes = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(vtype_name(db));
    let rv = (&tp).group_by(Ident::<Post>::new()).select(rvotes).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let lname = (&db.post_link.link_type).select(&db.link_type.name);
    let pr = links_of(db).select(Ident::<PostLink>::new().with(lname.filt(|n: Str| n == "Linked" || n == "Duplicate")));
    let v = drain((&cc).and((&rv).opt()).and(pr.opt()));
    let v = top_n(v, |&(p, (_, l))| (Reverse(creation_date.get(p).unwrap()), p, l), 50);
    rows(v.into_iter().map(|(p, ((c, r), l))| {
        let r = r.unwrap_or([0, 0]);
        let owner = db.post.owner_user.get(p).map(|u| db.user.display_name.get(u).unwrap()).filter(|n| !n.is_empty()).unwrap_or("Anonymous");
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "score"]);
        f.extend([V::S(owner), V::I(c), V::I(r[0]), V::I(r[1])]);
        f.push(V::I(l.map_or(-1, |l| db.post_link.related_post_id.get(l).unwrap())));
        f.push(V::S(if s > 100 { "High Score" } else { "Moderate Score" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.Reputation, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// PostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        AVG(EXTRACT(EPOCH FROM (P.LastActivityDate - P.CreationDate))) AS AvgAnswerTime, COALESCE(SUM(P.Score), 0) AS TotalScore FROM Posts P GROUP BY P.OwnerUserId),
// FilteredUsers AS (SELECT U.UserId, U.Reputation, U.TotalBadges, U.GoldBadges, U.SilverBadges, U.BronzeBadges, P.TotalQuestions, P.TotalAnswers, P.AvgAnswerTime, P.TotalScore
//     FROM UserBadges U LEFT JOIN PostStats P ON U.UserId = P.OwnerUserId WHERE U.Reputation > 100 AND U.TotalBadges > 5),
// RankedUsers AS (SELECT *, ROW_NUMBER() OVER (PARTITION BY GoldBadges ORDER BY TotalScore DESC, Reputation DESC) AS RankByGold,
//        DENSE_RANK() OVER (ORDER BY AvgAnswerTime ASC) AS RankByResponseTime FROM FilteredUsers)
// SELECT Us.DisplayName, U.Reputation, U.TotalBadges, U.GoldBadges, U.SilverBadges, U.BronzeBadges, U.TotalQuestions, U.TotalAnswers,
//        CAST(U.AvgAnswerTime AS INTEGER) AS AvgResponseTimeInSeconds, U.RankByGold, U.RankByResponseTime
// FROM RankedUsers U INNER JOIN Users Us ON U.UserId = Us.Id
// WHERE U.RankByGold = 1 AND U.RankByResponseTime <= 5 AND (U.TotalAnswers IS NOT NULL OR U.TotalQuestions IS NOT NULL)
// ORDER BY U.TotalScore DESC, U.Reputation DESC LIMIT 10;
//
// RankByGold breaks (TotalScore, Reputation) ties by user id (the SQL leaves them open).
fn q21992(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, last_activity_date, creation_date, score, .. } = &db.post;
    let ps = db
        .post
        .group_by(owner_user)
        .select(post_type_id.and(last_activity_date).and(creation_date).and(score))
        .fold(([0i64; 4], 0.0f64), |(a, s), (((t, la), cd), sc)| ([a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + 1, a[3] + sc], s + secs(la - cd)));
    let mean = |p: Option<([i64; 4], f64)>| p.map(|(a, s)| s / a[2] as f64);
    let fu = db.user.with((&db.user.reputation).gt(100)).with((&ub).filt(|a| a[0] > 5));
    let wg = fu
        .group_by((&ub).map(|a| a[1]))
        .select(Ident::<User>::new().and(&ub).and((&ps).opt()).and(&db.user.reputation))
        .window(row_number, |(((u, _), p), r)| (p.is_none(), Reverse(p.map(|x| x.0[3])), Reverse(r), u), asc);
    type G = (Id<User>, [i64; 4], i64);
    let g: MatSet<G> = (&wg).map(|((((u, b), _), _), g)| (u, b, g)).collect();
    let wt = whole(&g)
        .select(Same::<G>::new().and(Same::<G>::new().map(|x: G| x.0).select((&ps).opt())))
        .window(dense_rank, move |(_, p)| (p.is_none(), mean(p).map(fkey)), asc);
    let r = drain((&wt).filt(|((g, p), t)| g.2 == 1 && t <= 5 && p.is_some()));
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let v = top_n(r, |&(_, (((u, _, _), p), _))| (p.is_none(), Reverse(p.map(|x| x.0[3])), Reverse(rep(u)), u), 10);
    rows(v.into_iter().map(|(_, (((u, b, g), p), t))| {
        let (a, m) = (p.unwrap().0, mean(p).unwrap());
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(m.round() as i64), V::I(g), V::I(t)]);
        row(f)
    }))
}

// WITH ProcessedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Tags, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) DESC) AS PopularityRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.Tags),
// FilteredPosts AS (SELECT PostId, Title, Body, CreationDate, Tags, Upvotes, Downvotes, CommentCount, PopularityRank FROM ProcessedPosts
//     WHERE (Tags LIKE '%SQL%' OR Tags LIKE '%Database%') AND CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND Upvotes > 5),
// RankedPosts AS (SELECT PostId, Title, Body, CreationDate, Tags, Upvotes, Downvotes, CommentCount, PopularityRank, ROW_NUMBER() OVER (ORDER BY PopularityRank) AS Rank FROM FilteredPosts)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Tags, rp.Upvotes, rp.Downvotes, rp.CommentCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        pht.Name AS PostHistoryTypeName
// FROM RankedPosts rp JOIN Users u ON rp.PostId = u.Id JOIN PostHistory ph ON rp.PostId = ph.PostId JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// WHERE rp.PopularityRank <= 10 ORDER BY rp.PopularityRank;
//
// PopularityRank breaks Upvotes ties by post id (the SQL leaves them open). `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q25711(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, tags_str, creation_date, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let pp = qs()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(owner_user_id.select(&bidx).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(&pp).select(Ident::<Post>::new().and(&pp).and(&cc)).window(row_number, |((p, a), _)| (Reverse(a[0]), p), asc);
    let since = add_years(today_ny(), -1);
    let keep = Ident::<Post>::new().with(tags_str.filt(|t: Str| t.contains("SQL") || t.contains("Database"))).with(creation_date.ge(since));
    type X = (Id<Post>, ([i64; 2], i64));
    let fp: MatSet<X> = (&w).filt(|(_, k)| k <= 10).map(|(((p, a), c), _)| (p, (a, c))).filt(|(_, (a, _)): X| a[0] > 5).with(Same::<X>::new().map(|x: X| x.0).select(keep)).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pid = || Same::<X>::new().map(|x: X| x.0);
    let v = drain((&fp).select(Same::<X>::new().and(pid().select(&db.post.origid).select(&uidx)).and(pid().select(history_of(db)))));
    rows(v.into_iter().map(|(_, (((p, (a, c)), u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "tags"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::S(htype_name(db).get(h).unwrap()));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, p.PostTypeId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Badges b GROUP BY b.UserId),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment AS CloseReason FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate AS PostCreationDate, rp.Score, rp.ViewCount, ub.BadgeCount, ub.HighestBadgeClass, COALESCE(pv.UpVotes, 0) AS TotalUpVotes,
//        COALESCE(pv.DownVotes, 0) AS TotalDownVotes, cp.CloseReason, CASE WHEN rp.PostTypeId = 1 THEN 'Question' WHEN rp.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END AS PostType,
//        CASE WHEN rp.rn = 1 THEN 'Latest Post' ELSE 'Older Post' END AS PostStatus, CASE WHEN cp.CloseReason IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostState
// FROM RecentPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE rp.rn = 1 OR ub.BadgeCount > 0 ORDER BY rp.CreationDate DESC LIMIT 100;
//
// rn breaks CreationDate ties within an owner by post id (the SQL leaves them open).
fn q31705(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, post_type_id, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let w = recent().group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let rp: MatSet<Id<Post>> = recent().collect();
    let ub = db.badge.group_by(&db.badge.user_id).select(&db.badge.class).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let pv = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let cp = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    type R = (Id<Post>, (((Option<(i64, i64)>, Option<[i64; 2]>), Option<Id<PostHistory>>), Option<Id<Post>>));
    let v = drain(
        (&rp)
            .select(Ident::<Post>::new().and(owner_user_id.select(&ub).opt().and((&pv).opt()).and(cp.opt()).and(Ident::<Post>::new().with(&first).opt())))
            .filt(|(_, (((b, _), _), f)): R| f.is_some() || b.map_or(false, |b| b.0 > 0)),
    );
    let v = top_n(v, |&(_, (p, (_, c)))| (Reverse(creation_date.get(p).unwrap()), p, c), 100);
    rows(v.into_iter().map(|(_, (p, (((b, u), c), f)))| {
        let u = u.unwrap_or([0, 0]);
        let reason = c.and_then(|c| db.post_history.comment.get(c));
        let t = post_type_id.get(p).unwrap();
        let mut f_ = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f_.extend([oint(b.map(|b| b.0)), oint(b.map(|b| b.1)), V::I(u[0]), V::I(u[1]), ostr(reason)]);
        f_.push(V::S(if t == 1 { "Question" } else if t == 2 { "Answer" } else { "Other" }));
        f_.push(V::S(if f.is_some() { "Latest Post" } else { "Older Post" }));
        f_.push(V::S(if reason.is_some() { "Closed" } else { "Open" }));
        row(f_)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COALESCE(c.UserId, u.Id) AS CommentUserId, COALESCE(c.CreationDate, p.CreationDate) AS RelevantDate,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY COALESCE(c.CreationDate, p.CreationDate) DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteCount, SUM(p.ViewCount) AS TotalViews, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// UserPostInteraction AS (SELECT a.UserId, a.DisplayName, COUNT(DISTINCT rp.PostId) AS PostsInteracted, SUM(CASE WHEN rp.rn = 1 THEN 1 ELSE 0 END) AS LatestCommentsCount,
//        AVG(a.TotalViews) AS AverageViews, RANK() OVER (ORDER BY COUNT(DISTINCT rp.PostId) DESC) AS UserRank
//     FROM ActiveUsers a JOIN RankedPosts rp ON a.UserId = rp.CommentUserId GROUP BY a.UserId, a.DisplayName)
// SELECT u.DisplayName, u.VoteCount, u.TotalViews, u.BadgeCount, upi.PostsInteracted, upi.LatestCommentsCount, upi.AverageViews,
//        CASE WHEN upi.UserRank <= 10 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributionLevel
// FROM ActiveUsers u JOIN UserPostInteraction upi ON u.UserId = upi.UserId WHERE u.VoteCount > 20 ORDER BY u.VoteCount DESC, upi.PostsInteracted DESC;
//
// rn breaks RelevantDate ties within a post by comment id (the SQL leaves them open).
fn q695(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let active = || db.user.with((&db.user.reputation).gt(1000));
    let au = active()
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(view_count.opt()).opt()).and(badges_of(db).opt()))
        .fold([0i64; 3], |a, ((t, w), _)| {
            let w = w.flatten();
            [a[0] + matches!(t, Some(2 | 3)) as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]
        });
    let bc = active().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Comment { user_id, creation_date: cd, .. } = &db.comment;
    type J = ((Id<Post>, i64), Option<((Id<Comment>, i64), Option<i64>)>);
    let j: MatSet<J> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .select(Ident::<Post>::new().and(creation_date).and(comments_of(db).select(Ident::<Comment>::new().and(cd).and(user_id.opt())).opt()))
        .collect();
    let w = (&j)
        .group_by(Same::<J>::new().map(|x: J| x.0 .0))
        .select(Same::<J>::new())
        .window(row_number, |((_, pd), c): J| (Reverse(c.map_or(pd, |((_, d), _)| d)), c.map(|x| x.0 .0)), asc);
    let first: MatSet<J> = (&w).filt(|(_, k)| k == 1).map(|(x, _)| x).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let cuid = || {
        Same::<J>::new()
            .and(Same::<J>::new().map(|x: J| x.0 .0).select(owner_user.select(&db.user.origid)).opt())
            .flat_map(|((_, c), o): (J, Option<i64>)| c.and_then(|(_, u)| u).or(o))
            .select(&uidx)
            .select(Ident::<User>::new().with(&au))
    };
    let pi = (&j).group_by(cuid()).select(Same::<J>::new().map(|x: J| x.0 .0)).count_distinct();
    let lc = (&j).group_by(cuid()).select(Same::<J>::new().with(&first).opt()).fold(0i64, |n, f| n + f.is_some() as i64);
    let w = whole(&pi).select(Ident::<User>::new().and(&pi).and(&lc)).window(rank, |((_, n), _)| Reverse(n), asc);
    type R = (((Id<User>, i64), i64), i64);
    let v = drain((&w).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0 .0).select((&au).filt(|a| a[0] > 20).and(&bc)))));
    rows(v.into_iter().map(|(_, ((((u, n), l), k), (a, b)))| {
        let tv = if a[1] == 0 { V::Null } else { V::I(a[2]) };
        let av = if a[1] == 0 { V::Null } else { V::F(a[2] as f64) };
        row(vec![user_col(db, u, "name"), V::I(a[0]), tv, V::I(b), V::I(n), V::I(l), av, V::S(if k <= 10 { "Top Contributor" } else { "Contributor" })])
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// CTE_ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, PH.UserDisplayName, CT.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes CT ON PH.Comment::integer = CT.Id
//     WHERE PH.PostHistoryTypeId IN (10, 11) AND PH.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months'),
// UserVotes AS (SELECT V.PostId, COUNT(CASE WHEN VT.Name = 'UpMod' THEN 1 END) AS UpVotes, COUNT(CASE WHEN VT.Name = 'DownMod' THEN 1 END) AS DownVotes
//     FROM Votes V JOIN VoteTypes VT ON V.VoteTypeId = VT.Id GROUP BY V.PostId),
// TagStatistics AS (SELECT T.TagName, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName
//     HAVING COUNT(P.Id) > 10)
// SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, RP.OwnerDisplayName, COALESCE(UV.UpVotes, 0) AS UpVotes, COALESCE(UV.DownVotes, 0) AS DownVotes, CT.CloseReason,
//        TS.TagName, TS.PostCount AS TagPostCount, TS.TotalScore
// FROM RankedPosts RP LEFT JOIN UserVotes UV ON RP.PostId = UV.PostId LEFT JOIN CTE_ClosedPosts CT ON RP.PostId = CT.PostId
// LEFT JOIN TagStatistics TS ON RP.Title ILIKE '%' || TS.TagName || '%'
// WHERE RP.PostRank = 1 AND (RP.Score > 0 OR CT.CloseReason IS NOT NULL) ORDER BY RP.Score DESC, RP.ViewCount DESC;
//
// The ILIKE is matched on the lowercased title and tag name with a LIKE matcher, so a `_` or `%` in a tag name stays a wildcard.
fn q24190(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, title, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cl = history_of(db)
        .select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]).and(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))))
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason));
    let lt = like_tags(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db.tag.group_by(&db.tag.tag_name).select((&by_tag).map(|(p, _): (Id<Post>, Id<Tag>)| p).select(score)).fold((0i64, 0i64), |(n, s), sc| (n + 1, s + sc));
    type T = (Str, (i64, i64));
    let tsm: MatSet<T> = whole(&ts_).select(Same::<Str>::new().and(&ts_)).filt(|(_, (n, _)): T| n > 10).collect();
    let tsn: HashIdx<Str, T> = (&tsm).map(|(n, _): T| n).inv().collect();
    let titles: MatSet<Str> = (&tp).select(title).collect();
    let hit: HashIdx<Str, T> = (&titles).select_where(&tsn, |t: Str, n: Str| like(&t.to_lowercase(), &format!("%{}%", n.to_lowercase()))).collect();
    type J = (((Id<Post>, i64), Option<[i64; 2]>), Option<Str>);
    let v = drain(
        (&tp)
            .select(Ident::<Post>::new().and(score).and((&uv).opt()).and(cl.opt()).and(title.select(&hit).opt()))
            .filt(|((((_, s), _), c), _): (J, Option<T>)| s > 0 || c.is_some()),
    );
    rows(v.into_iter().map(|(_, ((((p, _), u), c), t))| {
        let u = u.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::I(u[0]), V::I(u[1]), ostr(c)]);
        f.extend(match t {
            Some((n, (k, s))) => [V::S(n), V::I(k), V::I(s)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RecentRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score >= 0 AND p.PostTypeId IN (1, 2)),
// TagStats AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.ViewCount IS NULL THEN 0 ELSE p.ViewCount END) AS TotalViewCount, AVG(p.Score) AS AverageScore
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName HAVING COUNT(p.Id) > 5),
// UserReputation AS (SELECT u.Id AS UserId, MAX(u.Reputation) AS MaxReputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.Id
//     HAVING MAX(u.Reputation) IS NOT NULL),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, ts.TagName, ts.PostCount, ts.TotalViewCount, ts.AverageScore, ur.MaxReputation, ur.BadgeCount, phc.EditCount, phc.LastEditDate
// FROM RankedPosts rp LEFT JOIN TagStats ts ON ts.PostCount > 10 JOIN UserReputation ur ON ur.UserId = rp.OwnerUserId LEFT JOIN PostHistoryCounts phc ON phc.PostId = rp.PostId
// WHERE (phc.EditCount IS NULL OR phc.EditCount <= 3) AND (rp.Score - COALESCE(phc.EditCount, 0) > 0) AND rp.RecentRank <= 10 ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// `ON ts.PostCount > 10` names only TagStats, so it is a cross join onto the tags with more than ten posts. RecentRank breaks CreationDate ties by post id.
fn q23804(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.ge(0)).and(post_type_id.is_in([1, 2])))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phc = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let lt = like_tags(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _): (Id<Post>, Id<Tag>)| p).select(view_count.opt().and(score)).opt())
        .fold([0i64; 3], |a, x| match x {
            Some((w, s)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s],
            None => a,
        });
    let tags: HashIdx<(), (Str, [i64; 3])> = whole(&ts_).select(Same::<Str>::new().and(&ts_)).filt(|(_, a): (Str, [i64; 3])| a[0] > 5 && a[0] > 10).collect();
    type B = (((Id<Post>, i64), (Id<User>, i64)), Option<(i64, i64)>);
    let base = (&tp)
        .select(Ident::<Post>::new().and(score).and(owner_user.select(Ident::<User>::new().and(&bc))).and((&phc).opt()))
        .filt(|(((_, s), _), h): B| h.map_or(true, |(n, _)| n <= 3) && s - h.map_or(0, |(n, _)| n) > 0);
    let v = drain(base.and(Ident::<Post>::new().map(|_| ()).select(&tags).opt()));
    rows(v.into_iter().map(|(_, ((((p, _), (u, b)), h), t))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        f.extend(match t {
            Some((n, a)) => [V::S(n), V::I(a[0]), V::I(a[1]), avg(a[2], a[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend([user_col(db, u, "rep"), V::I(b)]);
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

/// SQL `s LIKE pat`: `%` matches any run of characters, `_` exactly one; no escape character.
fn like(s: &str, pat: &str) -> bool {
    let (s, p): (Vec<char>, Vec<char>) = (s.chars().collect(), pat.chars().collect());
    let (mut i, mut j, mut star, mut mark) = (0, 0, usize::MAX, 0);
    while i < s.len() {
        if j < p.len() && (p[j] == '_' || (p[j] != '%' && p[j] == s[i])) {
            i += 1;
            j += 1;
        } else if j < p.len() && p[j] == '%' {
            star = j;
            mark = i;
            j += 1;
        } else if star != usize::MAX {
            j = star + 1;
            mark += 1;
            i = mark;
        } else {
            return false;
        }
    }
    while j < p.len() && p[j] == '%' {
        j += 1;
    }
    j == p.len()
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes, COUNT(DISTINCT P.Id) AS PostCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, AVG(COALESCE(V.BountyAmount, 0)) AS AvgBounty,
//        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 WHERE U.Reputation > 0
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Views, UpVotes, DownVotes, PostCount, Questions, Answers, AvgBounty, UserRank FROM UserStats WHERE UserRank <= 10),
// MostActivePosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COUNT(C) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.LastActivityDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
//     GROUP BY P.Id, P.Title, P.CreationDate HAVING COUNT(C) > 0 ORDER BY UpVotes DESC LIMIT 5)
// SELECT U.DisplayName AS TopUser, U.Reputation, U.PostCount, U.Questions, U.Answers, P.Title AS MostActivePost, P.CommentCount, P.UpVotes, P.DownVotes
// FROM TopUsers U JOIN MostActivePosts P ON U.UserId = P.PostId ORDER BY U.Reputation DESC, P.UpVotes DESC;
//
// UserRank reads only Reputation, so the ten users are picked first. `COUNT(C)` counts every joined row (an unmatched C is a struct of NULLs), so the HAVING
// always holds. The LIMIT 5 breaks UpVotes ties by post id. `U.UserId = P.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q90(db: &'static So) -> String {
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Vote { vote_type_id, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8)));
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((t, _)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64],
            None => a,
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let map_ = db
        .post
        .with((&db.post.last_activity_date).ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id).opt()))
        .fold([0i64; 3], |a, (_, t)| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let top = top_n(drain((&map_).filt(|a| a[0] > 0)), |&(p, a)| (Reverse(a[1]), p), 5);
    let tv = rel(top);
    let by_id: HashIdx<i64, (Id<Post>, [i64; 3])> = (&tv).map(|(p, _): (Id<Post>, [i64; 3])| p).select(&db.post.origid).inv().select(&tv).collect();
    let v = drain((&tu).select(Ident::<User>::new().and(&us).and(&pc).and((&db.user.origid).select(&by_id))));
    rows(v.into_iter().map(|(_, (((u, a), n), (p, m)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend(m.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionCount,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS AnswerCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostActivity AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score AS PostScore, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT H.Id) AS EditCount,
//        MAX(H.CreationDate) AS LastEditDate FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory H ON P.Id = H.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.Score),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, QuestionCount, AnswerCount, TotalScore, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY Reputation DESC) AS UserRank
//     FROM UserStats WHERE Reputation > 1000)
// SELECT TU.UserRank, TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.QuestionCount, TU.AnswerCount, TU.TotalScore, PA.Title AS RecentPostTitle, PA.PostScore AS RecentPostScore,
//        PA.CommentCount AS RecentPostCommentCount, PA.LastEditDate AS RecentPostLastEdit
// FROM TopUsers TU LEFT JOIN PostActivity PA ON TU.UserId = PA.PostId WHERE TU.UserRank <= 10 ORDER BY TU.UserRank;
//
// UserRank reads only Reputation, so the top users are picked first. `TU.UserId = PA.PostId` joins a user id to a post id, so it goes through the raw ids;
// PostActivity is grouped per post, so it is built for those posts alone.
fn q8479(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let w = whole(rich()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, k)).collect();
    let tu = by_first(&rk);
    let tus: MatSet<Id<User>> = (&rk).map(|(u, _)| u).collect();
    let Post { post_type_id, score, .. } = &db.post;
    let pq = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let us = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(own_votes(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold(0i64, |s, x| s + x.map_or(0, |(sc, _)| sc));
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let pp: MatSet<Id<Post>> = (&tus).select((&db.user.origid).select(&pidx)).collect();
    let pa = (&pp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, d.map_or(m, |d| m.max(d))));
    let v = drain((&tu).and(&pq).and(&us).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&pa)).opt()));
    rows(v.into_iter().map(|(u, (((k, a), s), p))| {
        let mut f = vec![V::I(k)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(a.map(V::I));
        f.push(V::I(s));
        f.extend(match p {
            Some((p, (c, d))) => {
                let mut g = post_fields(db, p, &["title", "score"]);
                g.extend([V::I(c), tmax(d)]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U
//     WHERE U.Reputation IS NOT NULL AND U.Reputation > 0),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.CreationDate, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.Score, P.CreationDate),
// UpvotedPosts AS (SELECT P.Id AS PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.Score, COALESCE(UP.Upvotes, 0) AS UpvoteCount, COALESCE(UP.Downvotes, 0) AS DownvoteCount,
//        (COALESCE(UP.Upvotes, 0) - COALESCE(UP.Downvotes, 0)) AS NetVotes, U.DisplayName AS OwnerName, U.Reputation AS OwnerReputation, U.Location AS OwnerLocation, P.OwnerUserId
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN UpvotedPosts UP ON P.Id = UP.PostId)
// SELECT PD.PostId, PD.Title, PD.Score, PD.UpvoteCount, PD.DownvoteCount, PD.NetVotes, PD.OwnerName, PD.OwnerReputation, PD.OwnerLocation,
//        CASE WHEN R.ReputationRank < 11 THEN 'Top User' ELSE 'Regular User' END AS UserCategory
// FROM PostDetails PD LEFT JOIN UserReputation R ON PD.OwnerUserId = R.UserId WHERE PD.Score > 10 ORDER BY PD.NetVotes DESC, PD.Score DESC LIMIT 50;
//
// TopPosts is never read. ReputationRank breaks Reputation ties by user id.
fn q4983(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let rr = top_n(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let top10: MatSet<Id<User>> = rel(rr.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pd = db
        .post
        .with(score.gt(10))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&pd).and(owner_user.select(Ident::<User>::new().and(Ident::<User>::new().with(&top10).opt()))));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[0] - a[1]), Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (a, (u, t)))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(ostr(db.user.location.get(u)));
        f.push(V::S(if t.is_some() { "Top User" } else { "Regular User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ua.DisplayName, ua.Reputation, ua.UpVotes, ua.DownVotes, rp.CommentCount,
//        ROW_NUMBER() OVER (ORDER BY rp.Score DESC) AS Rank FROM RankedPosts rp JOIN UserActivity ua ON rp.OwnerUserId = ua.UserId WHERE rp.PostRank = 1)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.DisplayName AS OwnerDisplayName, tp.Reputation AS OwnerReputation, tp.UpVotes, tp.DownVotes, tp.CommentCount,
//        CASE WHEN tp.Score > 10 THEN 'High Score' WHEN tp.Score BETWEEN 5 AND 10 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Score DESC;
//
// PostRank and Rank read only base columns, and every user with Reputation > 1000 has a UserActivity row, so the ten posts are picked first and the
// votes x badges product is driven for their owners alone. Rank breaks Score ties by post id (the SQL leaves them open).
fn q30377(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| Reverse(s), asc);
    let cand = drain(db.user.with((&db.user.reputation).gt(1000)).select((&w).filt(|(_, k)| k == 1)));
    let top = top_n(cand, |&(_, ((p, s), _))| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.1 .0 .0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&ua))));
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.push(V::S(if s > 10 { "High Score" } else if s >= 5 { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, COALESCE(COUNT(CASE WHEN C.UserId IS NOT NULL THEN 1 END), 0) AS CommentCount,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.ViewCount DESC) AS ViewRank, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS ScoreRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= (CAST('2024-10-01' AS DATE) - INTERVAL '1 year') GROUP BY P.Id, P.Title, P.ViewCount, P.Score, P.PostTypeId),
// PostVotes AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteNet FROM Votes V GROUP BY V.PostId),
// PostHistoryStats AS (SELECT PH.PostId, COUNT(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 END) AS ClosureCount, COUNT(CASE WHEN PH.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeletionCount
//     FROM PostHistory PH GROUP BY PH.PostId),
// FinalReport AS (SELECT RP.PostId, RP.Title, RP.ViewCount, RP.CommentCount, COALESCE(PV.VoteNet, 0) AS VoteNet, COALESCE(PHS.ClosureCount, 0) AS ClosureCount,
//        COALESCE(PHS.DeletionCount, 0) AS DeletionCount, RP.ViewRank, RP.ScoreRank,
//        CASE WHEN RP.ViewRank <= 10 THEN 'Top View' WHEN RP.ScoreRank <= 10 THEN 'Top Score' ELSE 'Regular' END AS RankCategory
//     FROM RankedPosts RP LEFT JOIN PostVotes PV ON RP.PostId = PV.PostId LEFT JOIN PostHistoryStats PHS ON RP.PostId = PHS.PostId)
// SELECT Title, ViewCount, CommentCount, VoteNet, ClosureCount, DeletionCount, RankCategory FROM FinalReport WHERE VoteNet > 0 OR ClosureCount > 0 ORDER BY ViewCount DESC, ScoreRank ASC;
fn q2359(db: &'static So) -> String {
    let Post { creation_date, view_count, score, post_type_id, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(ts(2023, 10, 1, 0, 0, 0)));
    let w = rp().group_by(post_type_id).select(Ident::<Post>::new().and(view_count.opt()).and(score)).window(rank, |((_, w), _)| (w.is_none(), Reverse(w)), asc);
    let w = (&w).window(rank, |(((_, _), s), _): (((Id<Post>, Option<i64>), i64), i64)| Reverse(s), asc);
    type K = (Id<Post>, i64, i64);
    let rk: MatSet<K> = (&w).map(|((((p, _), _), vr), sr)| (p, vr, sr)).collect();
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).select((&db.comment.user_id).opt()).opt()).fold(0i64, |n, u| n + u.flatten().is_some() as i64);
    let pv = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold(0i64, |n, t| n + matches!(t, 2 | 3) as i64);
    let phs = rp().group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id)).fold([0i64; 2], |a, t| {
        [a[0] + matches!(t, 10 | 11) as i64, a[1] + matches!(t, 12 | 13) as i64]
    });
    let pid = || Same::<K>::new().map(|x: K| x.0);
    type R = (K, ((i64, Option<i64>), Option<[i64; 2]>));
    let v = drain(
        (&rk)
            .select(Same::<K>::new().and(pid().select(&cc).and(pid().select(&pv).opt()).and(pid().select(&phs).opt())))
            .filt(|(_, ((_, n), h)): R| n.unwrap_or(0) > 0 || h.map_or(0, |h| h[0]) > 0),
    );
    rows(v.into_iter().map(|(_, ((p, w, s), ((c, n), h)))| {
        let h = h.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([V::I(c), V::I(n.unwrap_or(0)), V::I(h[0]), V::I(h[1])]);
        f.push(V::S(if w <= 10 { "Top View" } else if s <= 10 { "Top Score" } else { "Regular" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.ViewCount DESC) AS PopularityRank, COALESCE(ut.DisplayName, 'Anonymous') AS OwnerDisplayName, p.OwnerUserId
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Users ut ON p.OwnerUserId = ut.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.ViewCount IS NOT NULL),
// ClosedPostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.Comment, ph.CreationDate - LAG(ph.CreationDate) OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate) AS TimeSinceLastEdit
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// BadgedUsers AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, cb.CreationDate AS ClosedDate, cb.Comment AS CloseComment,
//        bu.GoldBadges, bu.SilverBadges, bu.BronzeBadges, CASE WHEN rp.PopularityRank <= 5 THEN 'Top 5' ELSE 'Other' END AS PopularityCategory
// FROM RankedPosts rp LEFT JOIN ClosedPostHistory cb ON rp.PostId = cb.PostId LEFT JOIN BadgedUsers bu ON rp.OwnerUserId = bu.UserId
// WHERE (cb.CreationDate IS NULL OR cb.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 MONTH') ORDER BY rp.ViewCount DESC, rp.Score DESC LIMIT 100;
//
// TimeSinceLastEdit is never read. PopularityRank breaks ViewCount ties by post id (the SQL leaves them open).
fn q4180(db: &'static So) -> String {
    let Post { creation_date, view_count, score, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rp = || db.post.with(creation_date.ge(add_years(t0, -1))).with(view_count).with(ptype_name(db));
    let w = rp().group_by(ptype_name(db)).select(Ident::<Post>::new().and(view_count)).window(row_number, |(p, w)| (Reverse(w), p), asc);
    let top5: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let rps: MatSet<Id<Post>> = rp().collect();
    let bu = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cb = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11])).and(hd));
    let late = add_months(t0, -1);
    type R = (Id<Post>, ((Option<(Id<PostHistory>, i64)>, Option<[i64; 3]>), Option<Id<Post>>));
    let v = drain(
        (&rps)
            .select(Ident::<Post>::new().and(cb.opt().and(owner_user.select(&bu).opt()).and(Ident::<Post>::new().with(&top5).opt())))
            .filt(move |(_, ((h, _), _)): R| h.map_or(true, |(_, d)| d > late)),
    );
    let v = top_n(v, |&(_, (p, ((h, _), _)))| (Reverse(view_count.get(p)), Reverse(score.get(p).unwrap()), p, h.map(|x| x.0)), 100);
    rows(v.into_iter().map(|(_, (p, ((h, b), t)))| {
        let owner = owner_user.get(p).map_or("Anonymous", |u| db.user.display_name.get(u).unwrap());
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(owner));
        f.extend(post_fields(db, p, &["created", "views", "score", "answers", "comments"]));
        f.extend(match h {
            Some((h, d)) => [V::T(d), ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null],
        });
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if t.is_some() { "Top 5" } else { "Other" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId,
//        COALESCE((SELECT SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) FROM Votes WHERE PostId = p.Id), 0) AS UpVotes,
//        COALESCE((SELECT SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) FROM Votes WHERE PostId = p.Id), 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostHistories AS (SELECT ph.PostId, MIN(ph.CreationDate) AS FirstEditDate, COUNT(ph.Id) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId),
// TopPostStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.UpVotes, rp.DownVotes, ph.FirstEditDate, ph.EditCount,
//        CASE WHEN ph.EditCount > 3 THEN 'Frequent Edits' WHEN ph.EditCount BETWEEN 1 AND 3 THEN 'Infrequent Edits' ELSE 'No Edits' END AS EditFrequency,
//        (rp.UpVotes - rp.DownVotes) AS NetVotes FROM RankedPosts rp LEFT JOIN PostHistories ph ON rp.PostId = ph.PostId WHERE rp.rn <= 10)
// SELECT ps.Title, ps.CreationDate, ps.Score, ps.UpVotes, ps.DownVotes, ps.EditFrequency, ps.FirstEditDate, ROUND(CAST(ps.Score AS DECIMAL) / NULLIF(ps.EditCount, 0), 2) AS ScorePerEdit,
//        CASE WHEN ps.FirstEditDate IS NULL THEN 'Never Edited' ELSE 'Edited' END AS EditStatus
// FROM TopPostStats ps WHERE ps.NetVotes >= 5 AND (ps.EditFrequency = 'Frequent Edits' OR ps.EditFrequency = 'No Edits') ORDER BY ps.Score DESC, ps.CreationDate DESC;
//
// rn breaks (Score, CreationDate) ties by post id (the SQL leaves them open).
fn q20904(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), d, p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MAX), |(n, m), d| (n + 1, m.min(d)));
    let v = drain((&vc).and((&ph).opt()).filt(|(a, h): ([i64; 2], Option<(i64, i64)>)| a[0] - a[1] >= 5 && h.map_or(true, |(n, _)| n > 3)));
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        match h {
            Some((n, d)) => {
                let s = score.get(p).unwrap() as f64;
                f.extend([V::S("Frequent Edits"), V::T(d), V::F((s / n as f64 * 100.0).round() / 100.0), V::S("Edited")]);
            }
            None => f.extend([V::S("No Edits"), V::Null, V::Null, V::S("Never Edited")]),
        }
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, RANK() OVER (ORDER BY COUNT(p.Id) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, UserRank FROM UserActivity WHERE UserRank <= 10),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Not Accepted' END AS AnswerStatus,
//        COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS CloseCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.AcceptedAnswerId)
// SELECT tu.DisplayName, tu.PostCount, tu.UpVotes, tu.DownVotes, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, ps.PostId, ps.Title, ps.CreationDate, ps.AnswerStatus,
//        ps.CommentCount, ps.CloseCount
// FROM TopUsers tu JOIN PostStats ps ON tu.UserId = ps.PostId ORDER BY tu.UserRank, ps.CreationDate DESC;
//
// `tu.UserId = ps.PostId` joins a user id to a post id, so it goes through the raw ids. PostStats is grouped per post, so it is built for those posts alone.
fn q1535(db: &'static So) -> String {
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, c)| {
            let t = p.flatten();
            [a[0] + p.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (c == Some(1)) as i64, a[4] + (c == Some(2)) as i64, a[5] + (c == Some(3)) as i64]
        });
    let w = whole(&ua).select(Ident::<User>::new().and(&ua)).window(rank, |(_, a)| Reverse(a[0]), asc);
    type T = (Id<User>, [i64; 6], i64);
    let tu: MatSet<T> = (&w).filt(|(_, k)| k <= 10).map(|((u, a), k)| (u, a, k)).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let recent = Ident::<Post>::new().with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pp: MatSet<Id<Post>> = (&tu).map(|x: T| x.0).select(&db.user.origid).select(&pidx).select(recent).collect();
    let ps = (&pp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(10)) as i64]);
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select(&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&ps)))));
    let v = top_n(v, |&(_, ((_, _, k), (p, _)))| (k, Reverse(db.post.creation_date.get(p).unwrap()), p), 0);
    rows(v.into_iter().map(|(_, ((u, a, _), (p, s)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.push(V::S(if db.post.accepted_answer_id.get(p).is_some() { "Accepted" } else { "Not Accepted" }));
        f.extend(s.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        COUNT(DISTINCT p.Id) AS PostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE u.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Upvotes, Downvotes, PostCount, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats WHERE PostCount > 5),
// PostInfo AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, COALESCE(ph.UserDisplayName, 'System') AS LastModifiedBy,
//        DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC) AS LastActivityRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = p.Id)
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// FinalOutput AS (SELECT tu.UserId, tu.DisplayName, COUNT(DISTINCT pi.PostId) AS RecentPosts, SUM(pi.ViewCount) AS TotalViews, AVG(tu.Reputation) AS AvgReputation
//     FROM TopUsers tu LEFT JOIN PostInfo pi ON tu.UserId = pi.OwnerUserId GROUP BY tu.UserId, tu.DisplayName)
// SELECT fo.DisplayName, fo.RecentPosts, fo.TotalViews, fo.AvgReputation FROM FinalOutput fo WHERE fo.RecentPosts > 2 ORDER BY fo.AvgReputation DESC LIMIT 10;
//
// UserStats' vote sums and both DENSE_RANKs are never read. The PostHistory join keeps every history row at the post's latest date, so TotalViews sums over those rows.
// AvgReputation is the user's own reputation; the LIMIT breaks its ties by user id.
fn q2942(db: &'static So) -> String {
    let Post { creation_date, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let pc = db.user.with((&db.user.creation_date).lt(add_years(t0, -1))).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    let recent = || Ident::<Post>::new().with(creation_date.gt(add_days(t0, -30)));
    let fu = || db.user.with((&pc).filt(|n| n > 5));
    let rpn = fu().group_by(Ident::<User>::new()).select(posts_of(db).select(recent()).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tv = fu()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(recent()).select(view_count.opt().and(Ident::<Post>::new().and(&md).select(&at).opt())).opt())
        .fold((0i64, 0i64), |(k, s), x| match x.and_then(|(w, _)| w) {
            Some(w) => (k + 1, s + w),
            None => (k, s),
        });
    let v = drain((&rpn).filt(|n| n > 2).and(&tv));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, (n, (k, s)))| row(vec![user_col(db, u, "name"), V::I(n), nullable(s, k), V::F(db.user.reputation.get(u).unwrap() as f64)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionsAsked, SUM(v.BountyAmount) AS TotalBountySpent
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON u.Id = v.UserId
//     WHERE u.Reputation > 1000 AND u.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 years' GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.QuestionsAsked, ua.TotalBountySpent, RANK() OVER (ORDER BY ua.QuestionsAsked DESC) AS UserRank FROM UserActivity ua
//     WHERE ua.TotalBountySpent IS NOT NULL),
// PostHistorySummary AS (SELECT ph.PostId, MAX(CASE WHEN pht.Name = 'Post Closed' THEN ph.CreationDate END) AS LastCloseDate, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS ClosureCount
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, tu.DisplayName AS TopUserDisplayName, tu.QuestionsAsked, tu.TotalBountySpent, phs.LastCloseDate, phs.ClosureCount
// FROM RankedPosts rp LEFT JOIN TopUsers tu ON rp.ViewCount > 100 AND tu.UserRank <= 5 LEFT JOIN PostHistorySummary phs ON rp.PostId = phs.PostId
// WHERE rp.PostRank <= 10 ORDER BY rp.Score DESC, rp.CreationDate ASC;
//
// The first ON names no key of tu: a post with ViewCount > 100 is crossed with the top users, any other post keeps one NULL row. PostRank breaks Score ties by post id.
fn q32025(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1)).and(post_type_id.eq(1))).select(score));
    let top = top_n(v, |&(p, s)| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let users = || db.user.with((&db.user.reputation).gt(1000).and((&db.user.creation_date).ge(add_years(t0, -2))));
    let asked = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let bounty = users()
        .group_by(Ident::<User>::new())
        .select(asked().opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64), |(n, s), (_, b)| match b.flatten() {
            Some(b) => (n + 1, s + b),
            None => (n, s),
        });
    let qa = users().group_by(Ident::<User>::new()).select(asked().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ua = || (&bounty).filt(|(n, _)| n > 0).and(&qa);
    let w = whole(ua()).select(Ident::<User>::new().and(ua())).window(rank, |(_, (_, q))| Reverse(q), asc);
    let tus: HashIdx<(), (Id<User>, i64, i64)> = (&w).filt(|(_, k)| k <= 5).map(|((u, ((_, b), q)), _)| (u, b, q)).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db
        .post_history
        .group_by(post)
        .select(htype_name(db).and(post_history_type_id).and(hd))
        .fold((i64::MIN, 0i64), |(m, n), ((nm, t), d)| (if nm == "Post Closed" { m.max(d) } else { m }, n + (t == 10) as i64));
    let v = drain((&tp).select(Ident::<Post>::new().and(Ident::<Post>::new().with(view_count.gt(100)).map(|_| ()).select(&tus).opt()).and((&phs).opt())));
    rows(v.into_iter().map(|(_, ((p, t), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(match t {
            Some((u, b, q)) => [user_col(db, u, "name"), V::I(q), V::I(b)],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match h {
            Some((m, n)) => [tmax(m), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, p.Title, ph.CreationDate, ph.UserDisplayName, ph.Comment, DENSE_RANK() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS CloseRank
//     FROM PostHistory ph INNER JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId = 10)
// SELECT um.UserId, um.DisplayName, rp.PostId, rp.Title, rp.ViewCount, rp.Score, um.GoldBadges, um.SilverBadges, um.BronzeBadges, um.TotalViews, cp.Title AS ClosedPostTitle,
//        cp.CreationDate AS ClosedPostDate, cp.UserDisplayName AS ClosureUser, cp.Comment AS ClosureComment
// FROM UserMetrics um LEFT JOIN RankedPosts rp ON um.UserId = rp.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId AND cp.CloseRank = 1
// WHERE (um.GoldBadges > 0 OR um.SilverBadges > 0 OR um.BronzeBadges > 0) AND (rp.ViewCount IS NOT NULL OR cp.PostId IS NOT NULL)
// ORDER BY um.TotalViews DESC, rp.Score DESC NULLS LAST;
//
// `um.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids. The WHERE needs a matched rp (cp hangs off it), so UserMetrics is
// built only for the users some RankedPosts id reaches. UserRank is never read.
fn q57(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, view_count, .. } = &db.post;
    let rp = || Ident::<Post>::new().with(post_type_id.eq(1).and(score.gt(0)).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let users: MatSet<Id<User>> = db.user.with((&db.user.origid).select(&pidx).select(rp())).collect();
    let asked = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let um = (&users)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(asked.select(view_count.opt()).opt()))
        .fold([0i64; 4], |a, (c, w)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + w.flatten().unwrap_or(0)]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let closes = || db.post_history.with(post_history_type_id.eq(10));
    let md = closes().group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = closes().select(post.and(hd)).inv().collect();
    type R = (Id<User>, ([i64; 4], ((Id<Post>, Option<i64>), Option<Id<PostHistory>>)));
    let v = drain(
        (&users)
            .select(Ident::<User>::new().and((&um).filt(|a| a[0] + a[1] + a[2] > 0).and((&db.user.origid).select(&pidx).select(rp().select(Ident::<Post>::new().and(view_count.opt()).and(Ident::<Post>::new().and(&md).select(&at).opt()))))))
            .filt(|(_, (_, ((_, w), h))): R| w.is_some() || h.is_some()),
    );
    rows(v.into_iter().map(|(_, (u, (a, ((p, _), h))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
        f.extend(a.map(V::I));
        f.extend(match h {
            Some(h) => vec![title(db, p), V::T(hd.get(h).unwrap()), ostr(db.post_history.user_display_name.get(h)), ostr(db.post_history.comment.get(h))],
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerName, DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank,
//        COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.Id END) AS UpvoteCount,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId = 3 THEN v.Id END) AS DownvoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, Score, CreationDate, OwnerName, ScoreRank, TotalBounty, CommentCount, UpvoteCount, DownvoteCount FROM RankedPosts WHERE ScoreRank <= 10),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(*) AS EditCount, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 12) THEN 1 END) AS CloseCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT tp.PostId, tp.Title, tp.Score, tp.CreationDate, tp.OwnerName, tp.TotalBounty, tp.CommentCount, tp.UpvoteCount, tp.DownvoteCount, COALESCE(phto.EditCount, 0) AS EditCount,
//        COALESCE(phto.CloseCount, 0) AS CloseCount, CASE WHEN tp.Score > 100 THEN 'Highly Active' WHEN tp.Score > 50 THEN 'Moderately Active' ELSE 'Low Activity' END AS ActivityLevel
// FROM TopPosts tp LEFT JOIN PostHistoryCounts phto ON tp.PostId = phto.PostId ORDER BY tp.Score DESC, tp.CreationDate ASC LIMIT 100 OFFSET 0;
//
// ScoreRank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
fn q33291(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(dense_rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let rpa = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(bounty_amount.opt()).opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + b.flatten().unwrap_or(0), a[1] + c.is_some() as i64]);
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let phc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id)).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + matches!(t, 10 | 12) as i64]);
    let v = drain((&rpa).and(&ud).and((&phc).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 100);
    rows(v.into_iter().map(|(p, ((a, d), h))| {
        let s = score.get(p).unwrap();
        let h = h.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner"]);
        f.extend(a.map(V::I));
        f.extend(d.map(V::I));
        f.extend(h.map(V::I));
        f.push(V::S(if s > 100 { "Highly Active" } else if s > 50 { "Moderately Active" } else { "Low Activity" }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount, MAX(Date) AS LastBadgeDate FROM Badges GROUP BY UserId),
// PostVoteCounts AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId IN (2, 8) THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotesCount FROM Votes GROUP BY PostId),
// RecentPostHistory AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastHistoryDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12, 13) GROUP BY ph.PostId),
// FilteredPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(v.UpVotesCount, 0) - COALESCE(v.DownVotesCount, 0) AS ScoreDifference, COALESCE(bc.BadgeCount, 0) AS UserBadgeCount,
//        ph.LastHistoryDate FROM Posts p LEFT JOIN PostVoteCounts v ON p.Id = v.PostId LEFT JOIN UserBadgeCounts bc ON p.OwnerUserId = bc.UserId LEFT JOIN RecentPostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND p.Score > 10
//     AND (ph.LastHistoryDate IS NULL OR ph.LastHistoryDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '15 days')),
// RankedPosts AS (SELECT fp.PostId, fp.Title, fp.CreationDate, fp.ScoreDifference, fp.UserBadgeCount, RANK() OVER (ORDER BY fp.ScoreDifference DESC, fp.UserBadgeCount DESC) AS PostRank
//     FROM FilteredPosts fp)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ScoreDifference, rp.UserBadgeCount,
//        CASE WHEN rp.ScoreDifference > 5 THEN 'High Engagement' WHEN rp.ScoreDifference BETWEEN 0 AND 5 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM RankedPosts rp WHERE rp.PostRank <= 10 ORDER BY rp.PostRank, rp.UserBadgeCount DESC;
fn q23435(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let fp = || db.post.with(creation_date.ge(add_days(t0, -30)).and(score.gt(10)));
    let pv = fp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + matches!(t, Some(2 | 8)) as i64 - (t == Some(3)) as i64);
    let bc = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let rph = db.post_history.with(post_history_type_id.is_in([10, 11, 12, 13])).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let since = add_days(t0, -15);
    let (pv, bc, rph) = (&pv, &bc, &rph);
    let fps = move || pv.and(owner_user_id.select(bc).opt()).and(rph.opt()).filt(move |(_, h): ((i64, Option<i64>), Option<i64>)| h.map_or(true, |d| d >= since));
    let w = whole(fps()).select(Ident::<Post>::new().and(fps())).window(rank, |(_, ((d, b), _))| (Reverse(d), Reverse(b.unwrap_or(0))), asc);
    let v = drain((&w).filt(|(_, k)| k <= 10));
    rows(v.into_iter().map(|(_, ((p, ((d, b), _)), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(d), V::I(b.unwrap_or(0))]);
        f.push(V::S(if d > 5 { "High Engagement" } else if d >= 0 { "Moderate Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COALESCE((SELECT SUM(v.BountyAmount) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 8), 0) AS TotalBounty
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// PostWithMaxViews AS (SELECT PostId, MAX(ViewCount) AS MaxViewCount FROM RankedPosts GROUP BY PostId),
// EligiblePosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.Rank, pwv.MaxViewCount FROM RankedPosts rp JOIN PostWithMaxViews pwv ON rp.PostId = pwv.PostId
//     WHERE rp.Rank <= 5 AND rp.TotalBounty >= 50),
// VotesStats AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotesCount, AVG(BountyAmount) AS AvgBounty
//     FROM Votes v GROUP BY PostId),
// BountyQualifiedPosts AS (SELECT ep.*, vs.UpVotesCount, vs.DownVotesCount, vs.AvgBounty FROM EligiblePosts ep LEFT JOIN VotesStats vs ON ep.PostId = vs.PostId WHERE ep.MaxViewCount > 1000)
// SELECT bqp.Title, bqp.Score, bqp.MaxViewCount, bqp.UpVotesCount, bqp.DownVotesCount, COALESCE(bqp.AvgBounty, 0) AS AvgBounty,
//        CASE WHEN bqp.UpVotesCount > bqp.DownVotesCount THEN 'Positive' WHEN bqp.UpVotesCount < bqp.DownVotesCount THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM BountyQualifiedPosts bqp WHERE bqp.MaxViewCount IS NOT NULL ORDER BY bqp.Score DESC, bqp.Title ASC;
//
// PostWithMaxViews groups RankedPosts by its own key, so MaxViewCount is the post's ViewCount. Rank breaks Score ties by post id (the SQL leaves them open).
fn q21696(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let tb = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8))).select(bounty_amount.opt()).opt())
        .fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vote_type_id.and(bounty_amount.opt()))).fold([0i64; 4], |a, (t, b)| {
        [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
    });
    let v = drain((&tb).filt(|b| b >= 50).and(view_count.filt(|w| w > 1000)).and((&vs).opt()));
    rows(v.into_iter().map(|(p, ((_, w), a))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.push(V::I(w));
        f.extend(match a {
            Some(a) => [oint(Some(a[0])), oint(Some(a[1])), if a[2] == 0 { V::F(0.0) } else { avg(a[3], a[2]) }],
            None => [V::Null, V::Null, V::F(0.0)],
        });
        let (u, d) = a.map_or((None, None), |a| (Some(a[0]), Some(a[1])));
        f.push(V::S(match (u, d) {
            (Some(u), Some(d)) if u > d => "Positive",
            (Some(u), Some(d)) if u < d => "Negative",
            _ => "Neutral",
        }));
        row(f)
    }))
}

// Rewritten (rewrites/34713.sql): the final ORDER BY refined with `, m.PostId, m.ClosedDate` and the RN window with `, p.Id`.
// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS RN
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'),
// PostVoteSummary AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId IN (2, 6) THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate AS CloseDate, ph.UserDisplayName AS ClosedBy, ph.Comment FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10),
// Metrics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, COALESCE(pvs.TotalUpvotes, 0) AS Upvotes, COALESCE(pvs.TotalDownvotes, 0) AS Downvotes,
//        COALESCE(cp.CloseDate, NULL) AS ClosedDate, COALESCE(cp.ClosedBy, 'Open') AS ClosedBy, COALESCE(cp.Comment, 'N/A') AS CloseReason,
//        (rp.ViewCount * 1.0 / NULLIF(rp.AnswerCount, 0)) AS ViewsPerAnswer, rp.Score
//     FROM RecentPosts rp LEFT JOIN PostVoteSummary pvs ON rp.PostId = pvs.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.RN = 1)
// SELECT m.PostId, m.Title, m.CreationDate, m.ViewCount, m.Upvotes, m.Downvotes, m.ClosedDate, m.ClosedBy, m.CloseReason, m.ViewsPerAnswer,
//        CASE WHEN m.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM Metrics m ORDER BY m.Score DESC, m.ViewCount DESC, m.PostId, m.ClosedDate LIMIT 100;
fn q34713(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, view_count, answer_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let pvs = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0] + matches!(t, Some(2 | 6)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let PostHistory { post_history_type_id, creation_date: hd, user_display_name, comment, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)));
    let v = drain((&pvs).and(cp.opt()));
    let v = top_n(v, |&(p, (_, h))| {
        let w = view_count.get(p);
        let d = h.map(|h| hd.get(h).unwrap());
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, d.is_none(), d)
    }, 100);
    rows(v.into_iter().map(|(p, (a, h))| {
        let w = view_count.get(p);
        let vpa = match (w, answer_count.get(p)) {
            (Some(w), Some(n)) if n != 0 => V::F(w as f64 / n as f64),
            _ => V::Null,
        };
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), V::S(user_display_name.get(h).unwrap_or("Open")), V::S(comment.get(h).unwrap_or("N/A"))],
            None => [V::Null, V::S("Open"), V::S("N/A")],
        });
        f.push(vpa);
        f.push(V::S(if h.is_some() { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerUserId, rp.Score, rp.CommentCount, rp.AnswerCount, rp.PostRank, u.Reputation, u.DisplayName,
//        CASE WHEN u.Location IS NOT NULL THEN CONCAT('Located in: ', u.Location) ELSE 'Location not specified' END AS UserLocation
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.PostRank <= 5),
// PostHistoryDetails AS (SELECT ph.PostId, ph.CreationDate AS HistoryDate, pht.Name AS PostHistoryType, ph.UserDisplayName, ph.Text FROM PostHistory ph
//     JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month')
// SELECT tp.PostId, tp.Title, tp.Score, tp.CommentCount, tp.AnswerCount, tp.Reputation AS UserReputation, tp.DisplayName AS OwnerDisplayName, tp.UserLocation, pht.HistoryDate,
//        pht.PostHistoryType, pht.UserDisplayName AS EditorName, pht.Text AS EditDetails
// FROM TopPosts tp LEFT JOIN PostHistoryDetails pht ON tp.PostId = pht.PostId ORDER BY tp.Score DESC, tp.PostId;
//
// PostRank reads only base columns, so the top posts are picked first and the comment and answer counts are taken for those alone.
fn q32256(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db
        .post
        .with(creation_date.ge(add_years(t0, -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).with(owner_user).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let PostHistory { creation_date: hd, user_display_name, text, .. } = &db.post_history;
    let phd = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_months(t0, -1))));
    let v = drain((&cc).and(&ac).and(owner_user).and(phd.opt()));
    rows(v.into_iter().map(|(p, (((c, a), u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(c), V::I(a)]);
        f.extend(ucols(db, u, &["rep", "name"]));
        f.push(match db.user.location.get(u) {
            Some(l) => V::Owned(format!("Located in: {l}")),
            None => V::S("Location not specified"),
        });
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), V::S(htype_name(db).get(h).unwrap()), ostr(user_display_name.get(h)), ostr(text.get(h))],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COALESCE(vote.Upvotes, 0) AS Upvotes, COALESCE(vote.Downvotes, 0) AS Downvotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount, SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenCount
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS Downvotes FROM Votes GROUP BY PostId) AS vote
//     ON vote.PostId = p.Id LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN PostHistory ph ON ph.PostId = p.Id
//     WHERE p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, vote.Upvotes, vote.Downvotes),
// PostStatistics AS (SELECT rp.OwnerUserId, COUNT(rp.PostId) AS TotalPosts, SUM(rp.Upvotes) AS TotalUpvotes, SUM(rp.Downvotes) AS TotalDownvotes, MAX(rp.CommentCount) AS MaxComments,
//        AVG(rp.CloseCount) AS AvgCloseCount, AVG(rp.ReopenCount) AS AvgReopenCount FROM RankedPosts rp GROUP BY rp.OwnerUserId)
// SELECT u.Id AS UserId, u.DisplayName, ps.TotalPosts, ps.TotalUpvotes, ps.TotalDownvotes, ps.MaxComments, ps.AvgCloseCount, ps.AvgReopenCount,
//        CASE WHEN ps.TotalPosts IS NULL THEN 'No Posts' ELSE 'Active Contributor' END AS ContributionStatus
// FROM Users u LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId WHERE u.Reputation > 1000 ORDER BY ps.TotalUpvotes DESC NULLS LAST, ps.TotalPosts DESC NULLS LAST;
//
// RN is never read.
fn q24531(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ch = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(10)) as i64, a[2] + (t == Some(11)) as i64]);
    let rp = rel(drain((&vc).and(&ch)));
    type R = (Id<Post>, ([i64; 2], [i64; 3]));
    let ps = (&rp)
        .group_by(Same::<R>::new().map(|x: R| x.0).select(owner_user))
        .select(Same::<R>::new().map(|x: R| x.1))
        .fold([0i64; 6], |a, (v, c)| [a[0] + 1, a[1] + v[0], a[2] + v[1], a[3].max(c[0]), a[4] + c[1], a[5] + c[2]]);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ps).opt()));
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(match a {
            Some(a) => vec![V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), avg(a[5], a[0]), V::S("Active Contributor")],
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::S("No Posts")],
        });
        row(f)
    }))
}

// WITH UserVotes AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(SUM(c.Score), 0) AS CommentScore, COALESCE(SUM(b.Class), 0) AS BadgeScore,
//        COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.PostId END) AS CloseCount, COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.PostId END) AS ReopenCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR') GROUP BY p.Id, p.Title, p.CreationDate),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.CommentScore, ps.BadgeScore, RANK() OVER (ORDER BY (ps.CommentScore + ps.BadgeScore) DESC) AS PostRank FROM PostStats ps)
// SELECT u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(rp.CommentScore, 0)) AS TotalCommentScore, SUM(COALESCE(rp.BadgeScore, 0)) AS TotalBadgeScore,
//        AVG(COALESCE(rp.CommentScore, 0)) AS AverageCommentScore, AVG(COALESCE(rp.BadgeScore, 0)) AS AverageBadgeScore,
//        (SELECT COUNT(*) FROM RankedPosts rp WHERE rp.PostRank <= 10) AS TopTenPostsCount
// FROM UserVotes u LEFT JOIN Posts p ON u.UserId = p.OwnerUserId LEFT JOIN RankedPosts rp ON p.Id = rp.PostId GROUP BY u.UserId, u.DisplayName
// HAVING SUM(COALESCE(rp.CommentScore, 0)) > 0 ORDER BY TotalPosts DESC, AverageCommentScore DESC LIMIT 20;
//
// UserVotes has one row per user and none of its aggregates is read. The HAVING needs a recent post, so the users are those owning one.
fn q23537(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let bu: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let ps = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select(&db.comment.score).opt().and(history_of(db).opt()).and(owner_user_id.select(&bu).select(&db.badge.class).opt()))
        .fold([0i64; 2], |a, ((c, _), b)| [a[0] + c.unwrap_or(0), a[1] + b.unwrap_or(0)]);
    let w = whole(&ps).select(Ident::<Post>::new().and(&ps)).window(rank, |(_, a)| Reverse(a[0] + a[1]), asc);
    let top10 = count((&w).filt(|(_, k)| k <= 10));
    let owners: MatSet<Id<User>> = recent().select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&ps).opt()).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(x) => {
                let x = x.unwrap_or([0, 0]);
                [a[0] + 1, a[1] + 1, a[2] + x[0], a[3] + x[1]]
            }
            None => [a[0] + 1, a[1], a[2], a[3]],
        });
    let v = drain((&us).filt(|a| a[2] > 0));
    let v = top_n(v, |&(u, a)| (Reverse(a[1]), Reverse(fkey(a[2] as f64 / a[0] as f64)), u), 20);
    rows(v.into_iter().map(|(u, a)| row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[2], a[0]), avg(a[3], a[0]), V::I(top10)])))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        MAX(u.CreationDate) AS AccountCreation FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// MostActiveUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.Questions, us.Answers, us.AcceptedAnswers, us.AccountCreation,
//        RANK() OVER (ORDER BY us.TotalPosts DESC) AS PostRank FROM UserStats us WHERE us.TotalPosts > 0),
// TopCloseReason AS (SELECT ph.UserId, COUNT(ph.Id) AS CloseReasonsCount, cr.Name AS CloseReasonName FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS int) = cr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId, cr.Name),
// UserCloseReasonRanked AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(tcrc.CloseReasonsCount), 0) AS TotalCloseReasons FROM Users u LEFT JOIN TopCloseReason tcrc ON u.Id = tcrc.UserId
//     GROUP BY u.Id, u.DisplayName)
// SELECT mau.DisplayName AS MostActiveUser, mau.TotalPosts, mau.Questions, mau.Answers, mau.AcceptedAnswers, ucr.DisplayName AS UserWithMostCloseReasons, ucr.TotalCloseReasons
// FROM MostActiveUsers mau JOIN UserCloseReasonRanked ucr ON ucr.TotalCloseReasons = (SELECT MAX(TotalCloseReasons) FROM UserCloseReasonRanked) WHERE mau.PostRank <= 10
// ORDER BY mau.TotalPosts DESC, ucr.TotalCloseReasons DESC;
//
// The ON compares ucr with an uncorrelated scalar, so it is a filter on UserCloseReasonRanked and then a cross join. TopCloseReason's per-reason groups are
// summed straight back per user, so it is counted per user.
fn q7991(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()))).fold([0i64; 4], |a, (t, x)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 2 && x.is_some()) as i64]
    });
    let w = whole(&us).select(Ident::<User>::new().and(&us)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { user, post_history_type_id, comment, .. } = &db.post_history;
    let tcr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .with(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .group_by(user)
        .select(Ident::<PostHistory>::new())
        .fold(0i64, |n, _| n + 1);
    let ucr = db.user.group_by(Ident::<User>::new()).select((&tcr).opt()).fold(0i64, |n, c| n + c.unwrap_or(0));
    let most = (&ucr).fold_flat(0i64, |m, n| m.max(n));
    let v = drain((&w).filt(|(_, k)| k <= 10).cross((&ucr).filt(move |n| n == most)));
    rows(v.into_iter().map(|((_, w), (((u, a), _), n))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([user_col(db, w, "name"), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        DENSE_RANK() OVER (ORDER BY p.CreationDate DESC) AS RankDate FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score FROM RankedPosts rp WHERE rp.RankScore <= 10),
// PostVoteDetails AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostCommentCount AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT tp.PostId, tp.Title, tp.ViewCount, COALESCE(pvd.UpVotes, 0) AS UpVotes, COALESCE(pvd.DownVotes, 0) AS DownVotes, COALESCE(pcc.CommentCount, 0) AS CommentCount,
//        CASE WHEN COALESCE(pvd.UpVotes, 0) > COALESCE(pvd.DownVotes, 0) THEN 'Positive Feedback' WHEN COALESCE(pvd.UpVotes, 0) < COALESCE(pvd.DownVotes, 0) THEN 'Negative Feedback'
//        ELSE 'No Feedback' END AS Feedback,
//        CASE WHEN tp.ViewCount IS NULL THEN 'No views recorded' WHEN tp.ViewCount > 1000 THEN 'Highly Viewed' ELSE 'Moderately Viewed' END AS ViewClassification
// FROM TopPosts tp LEFT JOIN PostVoteDetails pvd ON tp.PostId = pvd.PostId LEFT JOIN PostCommentCount pcc ON tp.PostId = pcc.PostId
// WHERE EXISTS (SELECT 1 FROM PostHistory ph WHERE ph.PostId = tp.PostId AND ph.PostHistoryTypeId IN (10, 11) AND ph.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 month')
// ORDER BY tp.ViewCount DESC, tp.Score DESC;
//
// RankScore breaks Score ties by post id (the SQL leaves them open). RankDate is never read.
fn q21916(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(ts(2023, 10, 1, 0, 0, 0))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let closed: MatSet<Id<Post>> = db.post_history.with(post_history_type_id.is_in([10, 11]).and(hd.gt(add_months(ts(2024, 10, 1, 0, 0, 0), -1)))).select(&db.post_history.post).collect();
    let tp = (&tp).with(&closed);
    let vd = votes_of(db).select(&db.vote.vote_type_id);
    let v = drain(
        tp.group_by(Ident::<Post>::new()).select(vd.opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]).and(
            (&closed).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64),
        ),
    );
    rows(v.into_iter().map(|(p, (a, c))| {
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.push(V::S(if a[0] > a[1] { "Positive Feedback" } else if a[0] < a[1] { "Negative Feedback" } else { "No Feedback" }));
        f.push(V::S(match w {
            None => "No views recorded",
            Some(w) if w > 1000 => "Highly Viewed",
            _ => "Moderately Viewed",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.Score, p.CreationDate, p.LastActivityDate, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// FilteredUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, CASE WHEN u.Views IS NULL THEN 'Unknown' WHEN u.Views < 100 THEN 'Low Engagement'
//        WHEN u.Views BETWEEN 100 AND 1000 THEN 'Moderate Engagement' ELSE 'High Engagement' END AS EngagementLevel FROM Users u WHERE u.Reputation > 100),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// PostHistoryData AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.LastActivityDate, fu.DisplayName AS OwnerName, fu.EngagementLevel, COALESCE(rc.CommentCount, 0) AS RecentCommentCount,
//        COALESCE(rc.LastCommentDate, NULL) AS LastCommentDate, COALESCE(phd.EditCount, 0) AS EditHistoryCount, COALESCE(phd.LastEditDate, NULL) AS LastEditDate
// FROM RankedPosts rp LEFT JOIN FilteredUsers fu ON rp.OwnerUserId = fu.UserId LEFT JOIN RecentComments rc ON rp.PostId = rc.PostId LEFT JOIN PostHistoryData phd ON rp.PostId = phd.PostId
// WHERE rp.rn = 1 AND (fu.Reputation >= 500 OR rp.Score > 10) ORDER BY rp.Score DESC NULLS LAST, rp.CreationDate DESC;
//
// rn breaks CreationDate ties by post id (the SQL leaves them open).
fn q21189(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let fu = owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100)).and(&db.user.reputation));
    let rc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phd = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    type R = (((Id<Post>, i64), Option<(Id<User>, i64)>), (Option<(i64, i64)>, Option<(i64, i64)>));
    let v = drain(
        (&tp)
            .select(Ident::<Post>::new().and(score).and(fu.opt()).and((&rc).opt().and((&phd).opt())))
            .filt(|(((_, s), u), _): R| u.map_or(false, |(_, r)| r >= 500) || s > 10),
    );
    rows(v.into_iter().map(|(_, (((p, _), u), (c, h)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "activity"]);
        f.extend(match u {
            Some((u, _)) => {
                let vw = db.user.views.get(u).unwrap();
                [user_col(db, u, "name"), V::S(if vw < 100 { "Low Engagement" } else if vw <= 1000 { "Moderate Engagement" } else { "High Engagement" })]
            }
            None => [V::Null, V::Null],
        });
        f.extend(match c {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH RECURSIVE PostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount, P.AnswerCount, P.CommentCount,
//        P.FavoriteCount, COUNT(c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 4 OR ph.PostHistoryTypeId = 5
//     GROUP BY ph.PostId)
// SELECT pa.Title, pa.CreationDate, pa.OwnerDisplayName, pa.Score, pa.ViewCount, pa.PostRank, COALESCE(phs.EditCount, 0) AS EditCount, phs.LastEditDate,
//        (pa.TotalUpvotes - pa.TotalDownvotes) AS NetVotes,
//        CASE WHEN pa.TotalUpvotes IS NOT NULL THEN CASE WHEN pa.TotalUpvotes >= 100 THEN 'Highly Voted' WHEN pa.TotalUpvotes >= 50 THEN 'Moderately Voted' ELSE 'Low Votes' END
//        ELSE 'No Votes' END AS VoteCategory
// FROM PostActivity pa LEFT JOIN PostHistoryStats phs ON pa.PostId = phs.PostId WHERE pa.Score > 10 AND pa.ViewCount > 100 ORDER BY pa.ViewCount DESC, pa.CreationDate DESC LIMIT 50;
//
// WITH RECURSIVE, but no CTE refers to itself. PostRank is taken over every recent post (the WHERE comes after it) and breaks CreationDate ties by post id.
// The comment x vote product is driven only for the posts the WHERE keeps.
fn q34030(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(ts(2023, 10, 1, 0, 0, 0)));
    let w = recent().group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), k)| (p, k)).collect();
    let rank = by_first(&rk);
    let pa = recent()
        .with(score.gt(10).and(view_count.gt(100)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(post_history_type_id.is_in([4, 5])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&pa).and(&rank).and((&phs).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(view_count.get(p)), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((a, k), h))| {
        let mut f = post_fields(db, p, &["title", "created", "owner", "score", "views"]);
        f.push(V::I(k));
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        f.push(V::I(a[0] - a[1]));
        f.push(V::S(if a[0] >= 100 { "Highly Voted" } else if a[0] >= 50 { "Moderately Voted" } else { "Low Votes" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.LastActivityDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '2 years' AND p.Score > 0 AND p.ViewCount IS NOT NULL),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, COUNT(DISTINCT p.Id) AS ActivePostCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostLinksFilter AS (SELECT pl.PostId, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostCount FROM PostLinks pl GROUP BY pl.PostId),
// ClosedPosts AS (SELECT p.Id, COUNT(ph.Id) AS CloseCount FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10
//     WHERE ph.CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.LastActivityDate, au.DisplayName, au.Reputation, au.BadgeCount, pl.RelatedPostCount, cp.CloseCount,
//        CASE WHEN au.ActivePostCount > 5 THEN 'Highly Active' WHEN au.ActivePostCount BETWEEN 1 AND 5 THEN 'Moderately Active' ELSE 'Inactive' END AS UserActivityLevel
// FROM RankedPosts rp JOIN ActiveUsers au ON rp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = au.UserId)
// LEFT JOIN PostLinksFilter pl ON rp.PostId = pl.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.Id WHERE rp.Rank <= 10 ORDER BY rp.Score DESC, rp.LastActivityDate DESC;
//
// CURRENT_DATE is the New York date (`today_ny`). The IN subquery matches au to the post's owner. Rank breaks (Score, ViewCount) ties by post id (the SQL leaves them open). The WHERE on ph turns ClosedPosts into an inner join.
fn q21857(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, view_count, owner_user, .. } = &db.post;
    let today = today_ny();
    let w = db
        .post
        .with(creation_date.ge(add_years(today, -2)).and(score.gt(0)))
        .with(view_count)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count))
        .window(row_number, |((p, s), w)| (Reverse(s), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let active = || posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(today, -1))));
    let bcnt = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt().and(active().opt())).fold(0i64, |n, (b, _)| n + b.is_some() as i64);
    let apc = (&owners).group_by(Ident::<User>::new()).select(active().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pl = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10).and(hd.ge(add_months(today, -6))))))
        .fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&bcnt).and(&apc)).and((&pl).opt()).and((&cp).opt())));
    rows(v.into_iter().map(|(p, ((((u, b), n), l), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "activity"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), oint(l), oint(c)]);
        f.push(V::S(if n > 5 { "Highly Active" } else if n >= 1 { "Moderately Active" } else { "Inactive" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, TotalPosts, QuestionsAsked, AnswersGiven, UpVotesReceived, DownVotesReceived, RANK() OVER (ORDER BY UpVotesReceived DESC) AS UpvoteRank FROM UserActivity),
// UsersWithBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgesCount, MAX(b.Class) AS MaxBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// FinalReport AS (SELECT r.UserId, r.DisplayName, r.TotalPosts, r.QuestionsAsked, r.AnswersGiven, r.UpVotesReceived, r.DownVotesReceived, COALESCE(b.BadgesCount, 0) AS BadgesCount,
//        CASE WHEN b.MaxBadgeClass IS NULL THEN 'None' WHEN b.MaxBadgeClass = 1 THEN 'Gold' WHEN b.MaxBadgeClass = 2 THEN 'Silver' ELSE 'Bronze' END AS MaxBadgeClass
//     FROM RankedUsers r LEFT JOIN UsersWithBadges b ON r.UserId = b.UserId)
// SELECT *, CASE WHEN TotalPosts = 0 THEN 'No Activity' WHEN UpVotesReceived > DownVotesReceived THEN 'Positive Contributor' ELSE 'Needs Improvement' END AS ContributorStatus
// FROM FinalReport WHERE TotalPosts > 5 ORDER BY UpVotesReceived DESC, AnswersGiven DESC;
//
// UpvoteRank is never read.
fn q3132(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let v = drain((&tp).filt(|n| n > 5).and(&ua).and((&ub).opt()));
    rows(v.into_iter().map(|(u, ((n, a), b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(b.map_or(0, |b| b.0)));
        f.push(V::S(match b.map(|b| b.1) {
            None => "None",
            Some(1) => "Gold",
            Some(2) => "Silver",
            _ => "Bronze",
        }));
        f.push(V::S(if a[2] > a[3] { "Positive Contributor" } else { "Needs Improvement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS QuestionsAsked, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        AVG(EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - u.CreationDate)) / 86400) AS DaysActive FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// RecentEdits AS (SELECT p.Id AS PostId, ph.UserDisplayName, ph.CreationDate AS EditDate, ph.Comment FROM PostHistory ph INNER JOIN Posts p ON ph.PostId = p.Id
//     WHERE ph.PostHistoryTypeId IN (4, 5, 24) ORDER BY ph.CreationDate DESC LIMIT 10),
// VoteSummary AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId IN (2, 9) THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT u.DisplayName, us.QuestionsAsked, us.AcceptedAnswers, us.DaysActive, rp.Title, rp.CreationDate, rp.ViewCount, vs.UpVotes, vs.DownVotes, re.UserDisplayName AS LastEditor,
//        re.EditDate, re.Comment
// FROM Users u LEFT JOIN UserStats us ON u.Id = us.UserId LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId AND rp.rn = 1 LEFT JOIN VoteSummary vs ON rp.PostId = vs.PostId
// LEFT JOIN RecentEdits re ON rp.PostId = re.PostId WHERE u.Reputation > 1000 ORDER BY us.QuestionsAsked DESC, us.AcceptedAnswers DESC, rp.CreationDate DESC OFFSET 0 ROWS FETCH NEXT 50 ROWS ONLY;
//
// DaysActive averages the user's constant age over the joined post rows, summed as floats the way DuckDB does. rn breaks CreationDate ties by post id.
fn q21716(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, accepted_answer_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let us = rich()
        .group_by(Ident::<User>::new())
        .select((&db.user.creation_date).and(posts_of(db).select(accepted_answer_id.opt()).opt()))
        .fold((0i64, 0i64, 0i64, 0.0f64), |(n, q, a, s), (cd, p)| (n + 1, q + p.is_some() as i64, a + p.flatten().is_some() as i64, s + secs(t0 - cd) / 86400.0));
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let latest = || (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p);
    let lp: MatSet<Id<Post>> = latest().collect();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let re = top_n(drain(db.post_history.with(post_history_type_id.is_in([4, 5, 24])).with(&db.post_history.post).select(hd)), |&(h, d)| (Reverse(d), h), 10);
    let rev = rel(re.into_iter().map(|x| x.0).collect());
    let edits: HashIdx<Id<Post>, Id<PostHistory>> = (&rev).map(|h: Id<PostHistory>| h).select(&db.post_history.post).inv().select(&rev).collect();
    let vs = (&lp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + matches!(t, Some(2 | 9)) as i64, a[1] + (t == Some(3)) as i64]);
    let rp = latest().select(Ident::<Post>::new().and(&vs).and((&edits).opt()));
    let v = drain(rich().select((&us).and(rp.opt())));
    let v = top_n(v, |&(u, ((_, q, a, _), r))| {
        let d = r.map(|((p, _), _)| creation_date.get(p).unwrap());
        (Reverse(q), Reverse(a), d.is_none(), Reverse(d), u, r.and_then(|x| x.1))
    }, 50);
    rows(v.into_iter().map(|(u, ((n, q, a, s), r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(q), V::I(a), V::F(s / n as f64)];
        f.extend(match r {
            Some(((p, vt), h)) => {
                let mut g = post_fields(db, p, &["title", "created", "views"]);
                g.extend([V::I(vt[0]), V::I(vt[1])]);
                g.extend(match h {
                    Some(h) => [ostr(db.post_history.user_display_name.get(h)), V::T(hd.get(h).unwrap()), ostr(db.post_history.comment.get(h))],
                    None => [V::Null, V::Null, V::Null],
                });
                g
            }
            None => (0..8).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

/// DuckDB's `CURRENT_DATE`: today in the session zone (America/New_York), not in UTC.
fn today_ny() -> i64 {
    trunc_day(utc_to_ny(now_utc()))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COUNT(DISTINCT b.Id) AS TotalBadges, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Comments c ON u.Id = c.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalUpvotes, us.TotalDownvotes, us.TotalBadges, us.TotalComments, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount,
//        COALESCE(phs.CloseCount, 0) AS CloseCount, COALESCE(phs.ReopenCount, 0) AS ReopenCount,
//        CASE WHEN us.Reputation < 1000 THEN 'Newbie' WHEN us.Reputation BETWEEN 1000 AND 10000 THEN 'Intermediate' ELSE 'Expert' END AS UserLevel
// FROM UserStats us LEFT JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId AND rp.PostRank = 1 LEFT JOIN PostHistoryStats phs ON rp.PostId = phs.PostId
// WHERE us.TotalUpvotes > us.TotalDownvotes ORDER BY us.Reputation DESC, rp.Score DESC;
//
// The votes x badges x comments product is driven for every user (about 4.6e8 rows). PostRank breaks CreationDate ties by post id (the SQL leaves them open).
fn q3818(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()).and(comments_by(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let good: MatSet<Id<User>> = db.user.with((&us).filt(|a| a[0] > a[1])).collect();
    let bc = (&good).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cc = (&good).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let phs = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let rp = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).select(Ident::<Post>::new().and((&phs).opt()));
    let v = drain((&good).select(Ident::<User>::new().and(&us).and(&bc).and(&cc).and(rp.opt())));
    rows(v.into_iter().map(|(_, ((((u, a), b), c), r))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b), V::I(c)]);
        f.extend(match r {
            Some((p, h)) => {
                let h = h.unwrap_or([0, 0]);
                let mut g = post_fields(db, p, &["id", "title", "created", "score", "views"]);
                g.extend([V::I(h[0]), V::I(h[1])]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null, V::I(0), V::I(0)],
        });
        f.push(V::S(if rep < 1000 { "Newbie" } else if rep <= 10000 { "Intermediate" } else { "Expert" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(P.ViewCount), 0) AS TotalViews, COUNT(DISTINCT P.Id) AS PostCount
//     FROM Users U LEFT JOIN Votes V ON V.UserId = U.Id LEFT JOIN Posts P ON P.OwnerUserId = U.Id LEFT JOIN Badges B ON B.UserId = U.Id GROUP BY U.Id, U.DisplayName),
// FilteredStats AS (SELECT UserId, DisplayName, Upvotes, Downvotes, GoldBadges, SilverBadges, BronzeBadges, AnswerCount, TotalViews, PostCount, (Upvotes - Downvotes) AS Score,
//        RANK() OVER (ORDER BY (Upvotes - Downvotes + (GoldBadges * 3) + (SilverBadges * 2) + BronzeBadges) DESC) AS Rank,
//        CASE WHEN (GoldBadges + SilverBadges + BronzeBadges) = 0 THEN 'No Badges' ELSE 'Has Badges' END AS BadgeStatus FROM UserStats),
// TopUsers AS (SELECT F.*, ROW_NUMBER() OVER (PARTITION BY BadgeStatus ORDER BY Score DESC) AS BadgeRank FROM FilteredStats F)
// SELECT UserId, DisplayName, Upvotes, Downvotes, Score, BadgeStatus, Rank, BadgeRank,
//        CASE WHEN TotalViews IS NULL OR TotalViews = 0 THEN 'No Views Recorded' WHEN TotalViews > 1000 THEN 'Popular User' ELSE 'Less Popular User' END AS PopularityStatus
// FROM TopUsers WHERE Rank <= 10 AND (Score > 0 OR BadgeStatus = 'No Badges') ORDER BY Score DESC, BadgeStatus ASC;
//
// The votes x posts x badges product is driven for every user (about 4.9e8 rows). BadgeRank breaks Score ties by user id (the SQL leaves them open).
// AnswerCount and PostCount are never read.
fn q23600(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select((&db.post.view_count).opt()).opt()).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, ((t, w), c)| {
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64, a[5] + w.flatten().unwrap_or(0)]
        });
    let nob = |a: [i64; 6]| a[2] + a[3] + a[4] == 0;
    let w = whole(&us).select(Ident::<User>::new().and(&us)).window(rank, |(_, a)| Reverse(a[0] - a[1] + 3 * a[2] + 2 * a[3] + a[4]), asc);
    type X = ((Id<User>, [i64; 6]), i64);
    let w = (&w).group_by(Same::<X>::new().map(move |((_, a), _): X| nob(a))).select(Same::<X>::new()).window(row_number, |((u, a), _): X| (Reverse(a[0] - a[1]), u), asc);
    let v = drain((&w).filt(move |(((_, a), k), _)| k <= 10 && (a[0] - a[1] > 0 || nob(a))));
    rows(v.into_iter().map(|(_, (((u, a), k), b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::S(if nob(a) { "No Badges" } else { "Has Badges" }), V::I(k), V::I(b)]);
        f.push(V::S(if a[5] == 0 { "No Views Recorded" } else if a[5] > 1000 { "Popular User" } else { "Less Popular User" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (ORDER BY SUM(COALESCE(P.Score, 0)) DESC) AS Rank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE U.Reputation > 1000 AND U.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, PH.UserId, P.Title, COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id
//     WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId, PH.CreationDate, PH.UserId, P.Title),
// AverageVotes AS (SELECT PostId, AVG(VoteCount) AS AvgVoteCount FROM (SELECT PostId, COUNT(V.Id) AS VoteCount FROM Votes V GROUP BY PostId) AS VoteSummary GROUP BY PostId)
// SELECT UA.UserId, UA.DisplayName, UA.PostCount, UA.TotalScore, COALESCE(CP.CloseCount, 0) AS ClosedPostCount, AVG(AV.AvgVoteCount) AS AverageVotes,
//        CASE WHEN UA.TotalScore > 100 THEN 'High performer' WHEN UA.TotalScore BETWEEN 50 AND 100 THEN 'Medium performer' ELSE 'Low performer' END AS PerformanceCategory
// FROM UserActivity UA LEFT JOIN ClosedPosts CP ON UA.UserId = CP.UserId LEFT JOIN AverageVotes AV ON UA.UserId = AV.PostId WHERE UA.Rank <= 10
// GROUP BY UA.UserId, UA.DisplayName, UA.PostCount, UA.TotalScore, CP.CloseCount ORDER BY UA.TotalScore DESC;
//
// Rank breaks TotalScore ties by user id (the SQL leaves them open). `UA.UserId = AV.PostId` joins a user id to a post id (Votes.PostId, raw), so it goes through
// the raw ids; AverageVotes has one row per PostId, so the AVG over a group is that post's vote count. The final GROUP BY keeps one row per distinct CloseCount of the user.
fn q2749(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000).and((&db.user.creation_date).lt(add_years(t0, -1))))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).opt())).opt())
        .fold(0i64, |s, p| s + p.map_or(0, |(sc, _)| sc));
    let top = top_n(drain(&ua), |&(u, s)| (Reverse(s), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { post, creation_date: hd, user, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post.and(hd).and(user.opt())).select(post_history_type_id).fold(0i64, |n, t| n + (t == 10) as i64);
    type K = ((Id<Post>, i64), Option<Id<User>>);
    let pairs: MatSet<(Id<User>, i64)> = whole(&cp).select(Same::<K>::new().and(&cp)).flat_map(|(k, n): (K, i64)| k.1.map(|u| (u, n))).collect();
    let counts = by_first(&pairs);
    let vc = db.vote.group_by(&db.vote.post_id).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tu).select((&ua).and(&pc).and((&counts).opt()).and((&db.user.origid).select(&vc).opt())));
    rows(v.into_iter().map(|(u, (((s, n), c), a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(s), V::I(c.unwrap_or(0)), a.map_or(V::Null, |a| V::F(a as f64))]);
        f.push(V::S(if s > 100 { "High performer" } else if s >= 50 { "Medium performer" } else { "Low performer" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title AS PostTitle, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate ASC) AS RankByScore, COALESCE(NULLIF(p.OwnerDisplayName, ''), 'Anonymous') AS OwnerDisplayName,
//        p.OwnerUserId FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// PostHistoryOverview AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN TRUE ELSE FALSE END) AS IsClosed, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN TRUE ELSE FALSE END) AS IsReopened,
//        MAX(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN TRUE ELSE FALSE END) AS IsDeleted FROM PostHistory ph GROUP BY ph.PostId)
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, up.PostId, up.PostTitle, up.Score, up.ViewCount, up.AnswerCount, COALESCE(ub.GoldBadges, 0) AS GoldBadges,
//        COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        CASE WHEN pho.IsClosed THEN 'Closed' WHEN pho.IsReopened THEN 'Reopened' WHEN pho.IsDeleted THEN 'Deleted' ELSE 'Active' END AS PostStatus
// FROM Users u JOIN RankedPosts up ON u.Id = up.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostHistoryOverview pho ON up.PostId = pho.PostId
// WHERE up.RankByScore <= 5 AND u.Reputation > 100 AND up.Score IS NOT NULL ORDER BY u.Reputation DESC, up.Score DESC LIMIT 50;
//
// RankByScore breaks (Score, CreationDate) ties by post id, and the LIMIT breaks (Reputation, Score) ties by post id (the SQL leaves both open).
fn q21310(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), d, p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let pho = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([false; 3], |a, t| [a[0] || t == 10, a[1] || t == 11, a[2] || matches!(t, 12 | 13)]);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100)).and((&ub).opt())).and((&pho).opt())));
    let v = top_n(v, |&(p, ((u, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((u, b), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "answers"]));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.push(V::S(match h {
            Some([true, _, _]) => "Closed",
            Some([_, true, _]) => "Reopened",
            Some([_, _, true]) => "Deleted",
            _ => "Active",
        }));
        row(f)
    }))
}

// WITH RECURSIVE UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.Score, p.CreationDate, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn FROM Posts p
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// BadgeRankings AS (SELECT ub.UserId, ub.DisplayName, ub.BadgeCount, RANK() OVER (ORDER BY ub.BadgeCount DESC) AS BadgeRank FROM UserBadges ub WHERE ub.BadgeCount > 0),
// PostHistoryAnalysis AS (SELECT ph.UserId, COUNT(ph.Id) AS EditCount, SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TitleAndBodyEdits,
//        COUNT(DISTINCT ph.PostId) AS UniquePostsEdited FROM PostHistory ph GROUP BY ph.UserId)
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(b.BadgeCount, 0) AS BadgeCount, COALESCE(badge_rank.BadgeRank, NULL) AS BadgeRank, cp.TopPostCount AS TopPostsCount,
//        ph.EditCount AS TotalEditCount, ph.TitleAndBodyEdits AS TitleAndBodyEdits, ph.UniquePostsEdited AS UniquePostsEdited
// FROM Users u LEFT JOIN UserBadges b ON u.Id = b.UserId LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS TopPostCount FROM TopPosts WHERE rn = 1 GROUP BY OwnerUserId) cp ON u.Id = cp.OwnerUserId
// LEFT JOIN BadgeRankings badge_rank ON u.Id = badge_rank.UserId LEFT JOIN PostHistoryAnalysis ph ON u.Id = ph.UserId
// WHERE u.Reputation > 1000 ORDER BY u.Reputation DESC, COALESCE(badge_rank.BadgeRank, 999) ASC;
//
// WITH RECURSIVE, but no CTE refers to itself. rn = 1 keeps one post per owner, so TopPostCount counts the owners' rank-1 rows.
fn q31631(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let w = whole(&bc).select(Ident::<User>::new().and(&bc)).window(rank, |(_, n)| Reverse(n), asc);
    let bk: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), k)| (u, k)).collect();
    let bank = by_first(&bk);
    let wt = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let cp = db.user.group_by(Ident::<User>::new()).select((&wt).filt(|(_, k)| k == 1)).fold(0i64, |n, _| n + 1);
    let PostHistory { user, post_history_type_id, post_id, .. } = &db.post_history;
    let pha = db.post_history.group_by(user).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + matches!(t, 4 | 5) as i64]);
    let upe = db.post_history.group_by(user).select(post_id).count_distinct();
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&bc).opt().and((&bank).opt()).and((&cp).opt()).and((&pha).opt()).and((&upe).opt())));
    rows(v.into_iter().map(|(u, ((((b, r), c), h), d))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b.unwrap_or(0)), oint(r), oint(c)]);
        f.extend(match h {
            Some(h) => [V::I(h[0]), V::I(h[1]), V::I(d.unwrap_or(0))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// Rewritten (rewrites/32675.sql): the PostRank window refined with `, P.Id`.
// WITH RECURSIVE UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END), 0) AS TotalVotes,
//        COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalComments, TIMESTAMP '2024-10-01 12:34:56' - U.CreationDate AS AccountAge
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.CreationDate),
// UserPosts AS (SELECT U.Id AS UserId, U.DisplayName, P.Id AS PostId, P.Title, P.CreationDate, P.Score, ROW_NUMBER() OVER(PARTITION BY U.Id ORDER BY P.CreationDate DESC, P.Id) AS PostRank
//     FROM Users U INNER JOIN Posts P ON U.Id = P.OwnerUserId),
// TopEngagedUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalVotes, TotalComments, AccountAge, DENSE_RANK() OVER(ORDER BY TotalPosts DESC, TotalVotes DESC, TotalComments DESC) AS EngagementRank
//     FROM UserEngagement WHERE TotalPosts > 0)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.TotalVotes, U.TotalComments, U.AccountAge, P.Title AS LatestPostTitle, P.CreationDate AS LatestPostDate, P.Score AS LatestPostScore,
//        CASE WHEN P.Score IS NULL THEN 'No posts yet' ELSE CASE WHEN P.Score > 10 THEN 'High engagement' WHEN P.Score BETWEEN 1 AND 10 THEN 'Moderate engagement' ELSE 'Low engagement' END END AS EngagementLevel,
//        (SELECT COUNT(*) FROM Badges B WHERE B.UserId = U.UserId) AS BadgeCount
// FROM TopEngagedUsers U LEFT JOIN UserPosts P ON U.UserId = P.UserId AND P.PostRank = 1 WHERE U.EngagementRank <= 10 ORDER BY U.EngagementRank;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q32675(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())))
        .fold([0i64; 2], |a, (t, c)| [a[0] + matches!(t, Some(2 | 3)) as i64, a[1] + c.is_some() as i64]);
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let w = whole(&ue).select(Ident::<User>::new().and(&ue).and(&tp)).window(dense_rank, |((_, a), n)| (Reverse(n), Reverse(a[0]), Reverse(a[1])), asc);
    let wl = db.post.group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let lp = (&wl).filt(|(_, k)| k == 1).map(|((p, _), _)| p);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type X = (((Id<User>, [i64; 2]), i64), i64);
    let v = drain((&w).filt(|(_, k)| k <= 10).select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.0 .0 .0).select(lp.opt().and(&bc)))));
    rows(v.into_iter().map(|(_, ((((u, a), n), _), (p, b)))| {
        let t0 = ts(2024, 10, 1, 12, 34, 56);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::Iv(t0 - db.user.creation_date.get(u).unwrap())]);
        f.extend(match p {
            Some(p) => {
                let s = score.get(p).unwrap();
                let mut g = post_fields(db, p, &["title", "created", "score"]);
                g.push(V::S(if s > 10 { "High engagement" } else if s >= 1 { "Moderate engagement" } else { "Low engagement" }));
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::S("No posts yet")],
        });
        f.push(V::I(b));
        row(f)
    }))
}

// WITH UserScores AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 1 THEN 1 WHEN V.VoteTypeId = 8 THEN 1 ELSE 0 END), 0) AS SpecialVotes,
//        COUNT(DISTINCT P.Id) AS AnswerCount, COUNT(DISTINCT C.Id) AS CommentCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 2 LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes),
// RankedUsers AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserScores),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, U.DisplayName AS OwnerDisplayName, R.UserRank FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     JOIN RankedUsers R ON U.Id = R.UserId WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' AND P.PostTypeId IN (1, 2))
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.ViewCount, RP.OwnerDisplayName, RU.Reputation, RU.UpvoteCount, RU.DownvoteCount, RU.SpecialVotes, RU.AnswerCount, RU.CommentCount,
//        CASE WHEN RU.UserRank <= 10 THEN 'Top Contributor' WHEN RU.UserRank <= 50 THEN 'Valued Contributor' ELSE 'New Contributor' END AS ContributorCategory
// FROM RecentPosts RP JOIN RankedUsers RU ON RP.OwnerDisplayName = RU.DisplayName ORDER BY RP.CreationDate DESC, RU.Reputation DESC LIMIT 100;
//
// UserRank reads only Reputation. The answers x comments x votes product is driven only for the RankedUsers whose name is some recent post's owner's name.
// The LIMIT breaks ties by post and user id.
fn q2288(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let w = whole(rich()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), k)| (u, k)).collect();
    let rank = by_first(&rk);
    let rp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30)).and(post_type_id.is_in([1, 2])))
        .with(owner_user.select(Ident::<User>::new().with(&rank)))
        .collect();
    let names: MatSet<Str> = (&rp).select(owner_user.select(&db.user.display_name)).collect();
    let by_name: HashIdx<Str, Id<User>> = rich().with((&db.user.display_name).select(&names)).select(&db.user.display_name).inv().collect();
    let ru: MatSet<Id<User>> = rich().with((&db.user.display_name).select(&names)).collect();
    let answers_by: HashIdx<Id<User>, Id<Post>> = db.post.with(post_type_id.eq(2)).select(owner_user).inv().collect();
    let us = (&ru)
        .group_by(Ident::<User>::new())
        .select((&answers_by).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, x| {
            let t = x.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + matches!(t, Some(1 | 8)) as i64]
        });
    let ac = (&ru).group_by(Ident::<User>::new()).select((&answers_by).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let cc = (&ru).group_by(Ident::<User>::new()).select((&answers_by).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&rp).select(owner_user.select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and(&us).and(&ac).and(&cc).and(&rank))));
    let v = top_n(v, |&(p, ((((u, _), _), _), _))| (Reverse(creation_date.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p, u), 100);
    rows(v.into_iter().map(|(p, ((((u, a), n), c), k))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "owner"]);
        f.push(user_col(db, u, "rep"));
        f.extend(a.map(V::I));
        f.extend([V::I(n), V::I(c)]);
        f.push(V::S(if k <= 10 { "Top Contributor" } else if k <= 50 { "Valued Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes vt ON p.Id = vt.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(ut.UpVotes, 0) AS UserUpVotes, COALESCE(ut.DownVotes, 0) AS UserDownVotes,
//        ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS rn FROM Posts p
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) ut
//     ON p.Id = ut.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months'),
// RecentPosts AS (SELECT pm.PostId, pm.Title, pm.Score, pm.ViewCount, pm.UserUpVotes, pm.UserDownVotes, CASE WHEN pm.UserUpVotes IS NULL THEN 'No votes yet' ELSE 'Votes recorded' END AS VoteStatus
//     FROM PostMetrics pm WHERE pm.rn <= 10)
// SELECT u.DisplayName AS UserName, r.Title, r.Score AS PostScore, r.ViewCount, r.UserUpVotes, r.UserDownVotes, r.VoteStatus,
//        CASE WHEN r.UserUpVotes > r.UserDownVotes THEN 'Positive Engagement' WHEN r.UserUpVotes < r.UserDownVotes THEN 'Negative Engagement' ELSE 'Neutral Engagement' END AS EngagementStatus
// FROM UserActivity u JOIN RecentPosts r ON u.PostCount > 0 ORDER BY u.PostCount DESC, r.ViewCount DESC;
//
// The ON names only u, so the users with PostCount > 0 are crossed with the ten newest posts. rn breaks CreationDate ties by post id.
fn q2337(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).opt()).opt())
        .fold(0i64, |n, p| n + p.is_some() as i64);
    let top = top_n(drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).select(creation_date)), |&(p, d)| (Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pm = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let v = drain((&ua).filt(|n| n > 0).cross(&pm));
    rows(v.into_iter().map(|((u, p), (_, a))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::S("Votes recorded")]);
        f.push(V::S(if a[0] > a[1] { "Positive Engagement" } else if a[0] < a[1] { "Negative Engagement" } else { "Neutral Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, u.DisplayName AS Author,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// PostVoteDetails AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId IN (2, 8) THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN VoteTypeId = 6 THEN 1 END) AS CloseVotes, COUNT(CASE WHEN VoteTypeId = 7 THEN 1 END) AS ReopenVotes FROM Votes GROUP BY PostId),
// ClosedPosts AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LatestCloseDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.Author, COALESCE(pvd.UpVotes, 0) AS UpVotes, COALESCE(pvd.DownVotes, 0) AS DownVotes,
//        COALESCE(pvd.CloseVotes, 0) AS CloseVotes, COALESCE(pvd.ReopenVotes, 0) AS ReopenVotes, cp.LatestCloseDate
//     FROM RankedPosts rp LEFT JOIN PostVoteDetails pvd ON rp.PostId = pvd.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.UserPostRank = 1)
// SELECT PostId, Title, CreationDate, ViewCount, Score, AnswerCount, Author, UpVotes, DownVotes, CloseVotes, ReopenVotes, CASE WHEN LatestCloseDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM FinalResults ORDER BY CreationDate DESC LIMIT 100;
//
// UserPostRank breaks CreationDate ties by post id (the SQL leaves them open).
fn q1084(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let top = top_n(drain((&w).filt(|(_, k)| k == 1)), |&(_, ((p, d), _))| (Reverse(d), p), 100);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.1 .0 .0).collect()).map(|p| p).collect();
    let pvd = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 4], |a, t| {
        [a[0] + matches!(t, Some(2 | 8)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(6)) as i64, a[3] + (t == Some(7)) as i64]
    });
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).select(&db.post_history.post).collect();
    let v = drain((&pvd).and(Ident::<Post>::new().with(&closed).opt()));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(if c.is_some() { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS Rank FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - interval '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// ClosedPosts AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// PostMetrics AS (SELECT p.Id AS PostId, COALESCE(pc.CommentCount, 0) AS TotalComments, COALESCE(bp.BadgeCount, 0) AS UserBadgeCount, COALESCE(cl.ClosedDate, '1970-01-01') AS ClosedOn,
//        RANK() OVER (ORDER BY COALESCE(pc.CommentCount, 0) DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p LEFT JOIN PostComments pc ON p.Id = pc.PostId LEFT JOIN UserBadges bp ON p.OwnerUserId = bp.UserId LEFT JOIN ClosedPosts cl ON p.Id = cl.PostId WHERE p.AcceptedAnswerId IS NOT NULL)
// SELECT pm.PostId, pm.TotalComments, pm.UserBadgeCount, pm.ClosedOn, CASE WHEN pm.ClosedOn != '1970-01-01' THEN 'Closed' ELSE 'Open' END AS PostStatus,
//        CASE WHEN pm.UserBadgeCount > 5 THEN 'Highly Recognized' WHEN pm.UserBadgeCount BETWEEN 1 AND 5 THEN 'Moderately Recognized' ELSE 'New Contributor' END AS ContributorStatus
// FROM PostMetrics pm WHERE pm.PostRank <= 10 ORDER BY pm.PostRank ASC;
//
// RankedPosts is never read.
fn q22397(db: &'static So) -> String {
    let Post { accepted_answer_id, view_count, owner_user, .. } = &db.post;
    let pc = db.post.with(accepted_answer_id).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(&pc).select(Ident::<Post>::new().and(&pc).and(view_count.opt())).window(rank, |((_, n), w)| (Reverse(n), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<(Id<Post>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|(((p, n), _), _)| (p, n)).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cl = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(post_history_type_id.and(hd)).fold(i64::MIN, |m, (t, d)| if t == 10 { m.max(d) } else { m });
    type T = (Id<Post>, i64);
    let pid = || Same::<T>::new().map(|x: T| x.0);
    let v = drain((&tp).select(Same::<T>::new().and(pid().select(owner_user).select(&bc).opt()).and(pid().select(&cl).opt())));
    rows(v.into_iter().map(|(_, (((p, n), b), c))| {
        let b = b.unwrap_or(0);
        let closed = c.filter(|&d| d != i64::MIN).unwrap_or(0);
        let mut f = post_fields(db, p, &["id"]);
        f.extend([V::I(n), V::I(b), V::T(closed), V::S(if closed != 0 { "Closed" } else { "Open" })]);
        f.push(V::S(if b > 5 { "Highly Recognized" } else if b >= 1 { "Moderately Recognized" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionsAnswered, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges,
//        CASE WHEN COUNT(DISTINCT p.Id) > 10 THEN 'Active' ELSE 'Novice' END AS UserExperience
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 2 LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, pt.Name AS PostHistoryType, COUNT(*) AS CloseCount, MAX(ph.CreationDate) AS LastClosed FROM PostHistory ph INNER JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id
//     WHERE pt.Name LIKE '%Closed%' GROUP BY ph.PostId, pt.Name)
// SELECT up.DisplayName AS UserName, rp.Title AS QuestionTitle, rp.CreationDate AS QuestionDate, rp.Score AS QuestionScore, rp.ViewCount AS QuestionViews, us.QuestionsAnswered,
//        us.GoldBadges, us.SilverBadges, us.BronzeBadges, us.UserExperience, cp.CloseCount AS TotalClosures, cp.LastClosed
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id JOIN UserStats us ON us.UserId = up.Id LEFT JOIN ClosedPosts cp ON cp.PostId = rp.Id
// WHERE rp.PostRank = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 50;
//
// PostRank breaks Score ties by post id, and the LIMIT breaks (Score, ViewCount) ties by post id (the SQL leaves both open). The answers x badges product is
// driven only for the owners of the rank-1 questions.
fn q4517(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let owners: MatSet<Id<User>> = (&fp).select(owner_user).collect();
    let answers: HashIdx<Id<User>, Id<Post>> = db.post.with(post_type_id.eq(2)).select(owner_user).inv().collect();
    let qa = (&owners).group_by(Ident::<User>::new()).select((&answers).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let ub = (&owners)
        .group_by(Ident::<User>::new())
        .select((&answers).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (_, c)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let htn = htype_name(db);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(htn.filt(|n: Str| n.contains("Closed"))).group_by(post.and(htype_name(db))).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    type K = (Id<Post>, Str);
    type C = (K, (i64, i64));
    let cpm: MatSet<C> = whole(&cp).select(Same::<K>::new().and(&cp)).collect();
    let by_post: HashIdx<Id<Post>, C> = (&cpm).select(Same::<C>::new().map(|x: C| x.0 .0)).inv().collect();
    let v = drain((&fp).select(owner_user.select(Ident::<User>::new().and(&qa).and(&ub)).and((&by_post).opt())));
    let v = top_n(v, |&(p, (_, c))| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p, c.map(|c| c.0 .1)), 50);
    rows(v.into_iter().map(|(p, (((u, n), a), c))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::S(if n > 10 { "Active" } else { "Novice" }));
        f.extend(match c {
            Some((_, (n, d))) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, SUM(b.Class) AS TotalBadgeClass, COUNT(DISTINCT b.Id) AS TotalBadges,
//        COUNT(DISTINCT p.Id) FILTER (WHERE p.PostTypeId = 1) AS QuestionCount, COUNT(DISTINCT p.Id) FILTER (WHERE p.PostTypeId = 2) AS AnswerCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostInteraction AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, MAX(p.CreationDate) AS LastActivity,
//        COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostsCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) LEFT JOIN PostLinks pl ON p.Id = pl.PostId GROUP BY p.Id),
// RankedPosts AS (SELECT pi.PostId, pi.CommentCount, pi.TotalBounty, pi.LastActivity, pi.RelatedPostsCount,
//        ROW_NUMBER() OVER (PARTITION BY CASE WHEN pi.TotalBounty > 0 THEN 'WithBounty' ELSE 'WithoutBounty' END ORDER BY pi.CommentCount DESC) AS Rank FROM PostInteraction pi)
// SELECT ur.DisplayName, ur.TotalBadgeClass, ur.TotalBadges, ur.QuestionCount, ur.AnswerCount, rp.PostId, rp.CommentCount, rp.TotalBounty, rp.LastActivity, rp.RelatedPostsCount,
//        CASE WHEN rp.TotalBounty > 0 THEN 'Bounty' ELSE 'No Bounty' END AS BountyStatus
// FROM UserReputation ur JOIN RankedPosts rp ON ur.UserId = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = rp.PostId)
// WHERE rp.Rank <= 3 AND (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = ur.UserId AND p.Score IS NOT NULL) > 5 ORDER BY ur.TotalBadgeClass DESC, rp.CommentCount DESC;
//
// The comment x vote x link product is driven for every post, since Rank reads its COUNT(c.Id). Rank breaks CommentCount ties by post id (the SQL leaves them open).
// The subquery looks the post up by its primary key, so ur is the post's owner; UserReputation is built for those owners alone.
fn q24781(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let ud = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([2, 3]))).select(bounty_amount.opt());
    let pi = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(ud.opt()).and(links_of(db).opt()))
        .fold([0i64; 2], |a, ((c, b), _)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    type R = (Id<Post>, [i64; 2]);
    let w = db.post.group_by((&pi).map(|a| a[1] > 0)).select(Ident::<Post>::new().and(&pi)).window(row_number, |(p, a): R| (Reverse(a[0]), p), asc);
    let rp: MatSet<R> = (&w).filt(|(_, k)| k <= 3).map(|(x, _)| x).collect();
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let owners: MatSet<Id<User>> = (&rp).map(|x: R| x.0).select((&db.post.owner_user).select(Ident::<User>::new().with((&pc).filt(|n| n > 5)))).collect();
    let ur = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt()))
        .fold(0i64, |s, (c, _)| s + c.unwrap_or(0));
    let ub = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let uq = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64]);
    let lc = links_of(db).select(&db.post_link.related_post_id);
    let rpc = (&rp).map(|x: R| x.0).group_by(Ident::<Post>::new()).select(lc).count_distinct();
    let v = drain((&rp).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&db.post.owner_user).select(Ident::<User>::new().with(&owners).and(&ur).and(&ub).and((&uq).opt()))).and(Same::<R>::new().map(|x: R| x.0).select(&rpc).opt()))));
    rows(v.into_iter().map(|(_, ((p, a), ((((u, s), b), q), l)))| {
        let q = q.unwrap_or([0, 0]);
        let mut f = vec![user_col(db, u, "name"), if b == 0 { V::Null } else { V::I(s) }, V::I(b), V::I(q[0]), V::I(q[1])];
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["created"]));
        f.push(V::I(l.unwrap_or(0)));
        f.push(V::S(if a[1] > 0 { "Bounty" } else { "No Bounty" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, COALESCE(vote_counts.UpVotes, 0) AS UpVotes, COALESCE(vote_counts.DownVotes, 0) AS DownVotes,
//        ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId)
//     vote_counts ON p.Id = vote_counts.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopPostedUsers AS (SELECT OwnerUserId, COUNT(Id) AS PostCount FROM Posts GROUP BY OwnerUserId ORDER BY PostCount DESC LIMIT 10),
// PostHistoryDetails AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenedDate,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 35 OR ph.PostHistoryTypeId = 36 THEN ph.CreationDate END) AS MigrationDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.UpVotes, rp.DownVotes,
//        CASE WHEN phd.ClosedDate IS NOT NULL AND (phd.ReopenedDate IS NULL OR phd.ReopenedDate < phd.ClosedDate) THEN 'Closed' WHEN phd.ReopenedDate IS NOT NULL THEN 'Reopened' ELSE 'Active' END AS PostStatus,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount, tu.DisplayName, tu.Reputation
// FROM RankedPosts rp JOIN Posts p ON rp.PostId = p.Id LEFT JOIN PostHistoryDetails phd ON p.Id = phd.PostId JOIN Users tu ON p.OwnerUserId = tu.Id
// WHERE rp.Rank <= 5 AND tu.Location IS NOT NULL AND tu.Reputation > 100 ORDER BY rp.ViewCount DESC;
//
// TopPostedUsers is never referenced. Rank breaks Score ties by post id (the SQL leaves them open).
fn q31209(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let top = top_n(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30))).select(score)), |&(p, s)| (Reverse(s), p), 5);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tu = Ident::<User>::new().with(&db.user.location).with((&db.user.reputation).gt(100));
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phd = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold((i64::MIN, i64::MIN), |(c, r), (t, d)| {
        (if t == 10 { c.max(d) } else { c }, if t == 11 { r.max(d) } else { r })
    });
    let v = drain((&vc).and(&cc).and(owner_user.select(tu)).and((&phd).opt()));
    rows(v.into_iter().map(|(p, (((a, c), u), h))| {
        let (cl, re) = h.unwrap_or((i64::MIN, i64::MIN));
        let st = if cl != i64::MIN && (re == i64::MIN || re < cl) { "Closed" } else if re != i64::MIN { "Reopened" } else { "Active" };
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(st), V::I(c)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AcceptedAnswerId, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalQuestions, COUNT(a.Id) AS TotalAnswers, COALESCE(SUM(p.Score), 0) AS TotalScore FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId
//     WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(ps.TotalQuestions, 0) AS TotalQuestions, COALESCE(ps.TotalAnswers, 0) AS TotalAnswers, COALESCE(ps.TotalScore, 0) AS TotalScore,
//        CASE WHEN u.Reputation > 1000 THEN 'High' WHEN u.Reputation BETWEEN 500 AND 1000 THEN 'Medium' ELSE 'Low' END AS ReputationLevel FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(DISTINCT ph.UserId) AS CloseVoteCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// FinalOutput AS (SELECT ur.UserId, ur.TotalQuestions, ur.TotalAnswers, ur.Reputation, ur.ReputationLevel, cp.CloseVoteCount, COALESCE(rp.Title, 'No Questions') AS LatestQuestionTitle
//     FROM UserReputation ur LEFT JOIN ClosedPosts cp ON ur.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = cp.PostId)
//     LEFT JOIN RankedPosts rp ON ur.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.Id))
// SELECT * FROM FinalOutput WHERE TotalQuestions > 0 OR CloseVoteCount IS NOT NULL ORDER BY TotalQuestions DESC, CloseVoteCount DESC;
//
// Each subquery looks a post up by its primary key, so both joins are to the posts the user owns: every closed post of the user is paired with every question
// (UserPostRank is not in the ON, so not only the latest).
fn q4001(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let asked = || db.post.with(post_type_id.eq(1));
    let ps = asked().group_by(owner_user).select(children_of(db).opt()).fold([0i64; 2], |a, c| [a[0] + 1, a[1] + c.is_some() as i64]);
    let PostHistory { post, post_history_type_id, user_id, .. } = &db.post_history;
    let closes = || db.post_history.with(post_history_type_id.eq(10));
    let cvc = closes().group_by(post).select(user_id).count_distinct();
    let cp: MatSet<Id<Post>> = closes().select(post).collect();
    type C = (Id<Post>, Option<i64>);
    let closed_by: HashIdx<Id<User>, C> = (&cp).select(owner_user).inv().select(Ident::<Post>::new().and((&cvc).opt())).collect();
    let asked_by: HashIdx<Id<User>, Id<Post>> = asked().select(owner_user).inv().collect();
    type R = ((Option<[i64; 2]>, Option<C>), Option<Id<Post>>);
    let v = drain(db.user.select((&ps).opt().and((&closed_by).opt()).and((&asked_by).opt())).filt(|((a, c), _): R| a.map_or(0, |a| a[0]) > 0 || c.is_some()));
    rows(v.into_iter().map(|(u, ((a, c), q))| {
        let a = a.unwrap_or([0, 0]);
        let r = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "uid"), V::I(a[0]), V::I(a[1]), V::I(r)];
        f.push(V::S(if r > 1000 { "High" } else if r >= 500 { "Medium" } else { "Low" }));
        f.push(oint(c.map(|c| c.1.unwrap_or(0))));
        f.push(V::S(q.and_then(|q| db.post.title.get(q)).unwrap_or("No Questions")));
        row(f)
    }))
}

// WITH RecentActivity AS (SELECT p.Id AS PostId, p.PostTypeId, p.Title, p.CreationDate, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS ActivityRank, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 AND v.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 MONTH' THEN 1 ELSE 0 END), 0) AS RecentUpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 AND v.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 MONTH' THEN 1 ELSE 0 END), 0) AS RecentDownVotes
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostLinksData AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS RelatedPostsCount, MAX(pl.CreationDate) AS LastLinkDate FROM PostLinks pl GROUP BY pl.PostId),
// CombinedData AS (SELECT ra.PostId, ra.Title, us.DisplayName AS OwnerDisplayName, us.Reputation AS OwnerReputation, pl.RelatedPostsCount, pl.LastLinkDate
//     FROM RecentActivity ra JOIN UserStats us ON us.UserId = ra.OwnerUserId LEFT JOIN PostLinksData pl ON pl.PostId = ra.PostId WHERE ra.ActivityRank = 1)
// SELECT cd.PostId, cd.Title, cd.OwnerDisplayName, cd.OwnerReputation, COALESCE(cd.RelatedPostsCount, 0) AS RelatedCount,
//        CASE WHEN cd.LastLinkDate IS NULL THEN 'No links' ELSE 'Links available' END AS LinkStatus
// FROM CombinedData cd ORDER BY cd.OwnerReputation DESC, cd.LastLinkDate DESC NULLS LAST;
//
// UserStats has one row per user and none of its aggregates is read, so the join is to the owner.
fn q23201(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(dense_rank, |(_, d)| Reverse(d), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let pl = (&fp).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.creation_date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&fp).select(owner_user.and((&pl).opt())));
    rows(v.into_iter().map(|(p, (u, l))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(l.map_or(0, |l| l.0)), V::S(if l.is_some() { "Links available" } else { "No links" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankByScore
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount > 100),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostLinksRank AS (SELECT pl.PostId, pl.RelatedPostId, COUNT(pl.Id) AS LinkCount FROM PostLinks pl GROUP BY pl.PostId, pl.RelatedPostId HAVING COUNT(pl.Id) > 5),
// FilteredComments AS (SELECT c.PostId, c.UserId, LAG(c.UserId) OVER (PARTITION BY c.PostId ORDER BY c.CreationDate) AS PrevUserId, COUNT(*) OVER (PARTITION BY c.PostId) AS TotalComments
//     FROM Comments c WHERE c.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' AND c.UserId IS NOT NULL)
// SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, COALESCE(pl.LinkCount, 0) AS LinkCount, fc.TotalComments,
//        CASE WHEN fc.TotalComments > 0 AND fc.PrevUserId IS NOT NULL THEN 'Active Discussion' ELSE 'No Recent Activity' END AS DiscussionStatus
// FROM RankedPosts r LEFT JOIN UserBadges ub ON r.OwnerUserId = ub.UserId LEFT JOIN PostLinksRank pl ON r.PostId = pl.PostId LEFT JOIN FilteredComments fc ON r.PostId = fc.PostId
// WHERE r.RankByScore <= 5 ORDER BY r.Score DESC, r.CreationDate DESC;
//
// FilteredComments has one row per comment; its LAG is a window over each post's comments, which breaks CreationDate ties by comment id (the SQL leaves them open).
fn q22885(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, view_count, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db
        .post
        .with(creation_date.gt(add_years(t0, -1)).and(view_count.gt(100)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let PostLink { post, related_post_id, .. } = &db.post_link;
    let plr = db.post_link.group_by(post.and(related_post_id)).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    type K = (Id<Post>, i64);
    type L = (K, i64);
    let plm: MatSet<L> = whole(&plr).select(Same::<K>::new().and(&plr)).filt(|(_, n): L| n > 5).collect();
    let links: HashIdx<Id<Post>, L> = (&plm).select(Same::<L>::new().map(|x: L| x.0 .0)).inv().collect();
    let Comment { creation_date: cd, user_id, post: cpost, .. } = &db.comment;
    let fcs = || db.comment.with(cd.gt(add_months(t0, -6))).with(user_id).with(cpost.select(Ident::<Post>::new().with(&tp)));
    let tc = fcs().group_by(cpost).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let fc = fcs().group_by(cpost).select(Ident::<Comment>::new().and(cd).and(user_id)).window(lag, |((c, d), u)| (d, c, u), asc);
    let v = drain((&tp).select(owner_user.select(&ub).opt().and((&links).opt()).and((&fc).opt()).and((&tc).opt())));
    rows(v.into_iter().map(|(p, (((b, l), c), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::I(l.map_or(0, |l| l.1)));
        match (c, n) {
            (Some((_, prev)), Some(n)) => {
                f.push(V::I(n));
                f.push(V::S(if n > 0 && prev.is_some() { "Active Discussion" } else { "No Recent Activity" }));
            }
            _ => f.extend([V::Null, V::S("No Recent Activity")]),
        }
        row(f)
    }))
}

// WITH RecursivePostStats AS (SELECT p.Id AS PostId, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.CreationDate, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.PostTypeId = 1),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// VoteCounts AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostHistoryAnalysis AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastActivityDate, COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.UserId END) AS CloseVotes,
//        COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.UserId END) AS ReopenVotes FROM PostHistory ph GROUP BY ph.PostId)
// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ps.ViewCount, ps.AnswerCount, ps.CommentCount, ps.OwnerUserId, u.DisplayName AS OwnerDisplayName, COALESCE(ub.GoldBadges, 0) AS GoldBadges,
//        COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, COALESCE(vc.UpVotes, 0) - COALESCE(vc.DownVotes, 0) AS NetVotes, pha.LastActivityDate,
//        pha.CloseVotes, pha.ReopenVotes
// FROM Posts p JOIN RecursivePostStats ps ON p.Id = ps.PostId JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN VoteCounts vc ON p.Id = vc.PostId
// LEFT JOIN PostHistoryAnalysis pha ON p.Id = pha.PostId WHERE ps.rn = 1 AND p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') ORDER BY NetVotes DESC, p.CreationDate DESC LIMIT 100;
//
// rn breaks CreationDate ties by post id, and the LIMIT breaks (NetVotes, CreationDate) ties by post id (the SQL leaves both open).
fn q30424(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let nv = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let PostHistory { post_history_type_id, creation_date: hd, user_id, .. } = &db.post_history;
    let lad = (&fp).group_by(Ident::<Post>::new()).select(history_of(db).select(hd)).fold(i64::MIN, |m, d| m.max(d));
    let voters = |t: i64| (&fp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(t))).select(user_id)).count_distinct();
    let (cv, rv) = (voters(10), voters(11));
    let v = drain((&nv).and(owner_user.and(owner_user.select(&ub).opt())).and((&lad).and((&cv).opt()).and((&rv).opt()).opt()));
    let v = top_n(v, |&(p, ((n, _), _))| (Reverse(n), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((n, (u, b)), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner_id"]);
        f.push(user_col(db, u, "name"));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(n));
        f.extend(match h {
            Some(((d, c), r)) => [V::T(d), V::I(c.unwrap_or(0)), V::I(r.unwrap_or(0))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS PostCount, p.OwnerUserId FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score IS NOT NULL),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, AVG(v.BountyAmount) AS AvgBounty, COUNT(DISTINCT v.Id) AS VoteCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// RecentClosedPosts AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, ph.Comment, vt.Name AS VoteTypeName FROM PostHistory ph JOIN VoteTypes vt ON ph.PostHistoryTypeId = 10
//     WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months')
// SELECT us.UserId, us.DisplayName, us.GoldBadges, us.SilverBadges, us.BronzeBadges, us.AvgBounty, us.VoteCount, rp.PostId, rp.Title, rp.Rank, rp.PostCount, rcp.CreationDate AS RecentCloseDate,
//        rcp.Comment, rcp.VoteTypeName, CASE WHEN rp.Rank = 1 THEN 'Top Post' WHEN rp.PostCount > 5 THEN 'Frequent Contributor' ELSE 'New Contributor' END AS ContributorStatus
// FROM UserStats us LEFT JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId LEFT JOIN RecentClosedPosts rcp ON rp.PostId = rcp.PostId
// WHERE us.VoteCount > 0 ORDER BY us.VoteCount DESC, us.DisplayName ASC, rp.Rank ASC LIMIT 50;
//
// VoteCount > 0 needs a vote cast by the user, so the badges x votes product is driven for those users alone. The VoteTypes ON names only ph, so every recent
// close is crossed with every vote type. Rank breaks (Score, CreationDate) ties by post id, and the LIMIT breaks ties by user, history row and vote type.
fn q24446(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let voters: MatSet<Id<User>> = db.vote.select(&db.vote.user).collect();
    let us = (&voters)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 5], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
        });
    let vc = (&voters).group_by(Ident::<User>::new()).select(votes_by(db)).fold(0i64, |n, _| n + 1);
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let w = recent().group_by(owner_user).select(Ident::<Post>::new().and(score).and(creation_date)).window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    type R = (Id<Post>, i64);
    let rp: HashIdx<Id<User>, R> = (&w).map(|(((p, _), _), k)| (p, k)).collect();
    let pcount = recent().group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let closes: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10).and(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))).select(&db.post_history.post).inv().collect();
    let vts: HashIdx<(), Id<VoteType>> = db.vote_type.map(|_| ()).inv().collect();
    let rcp = (&closes).select(Ident::<PostHistory>::new().and(Ident::<PostHistory>::new().map(|_| ()).select(&vts)));
    type P = (R, i64);
    let rpj = (&rp).and(&pcount).select(Same::<P>::new().and(Same::<P>::new().map(|x: P| x.0 .0).select(rcp).opt()));
    let v = drain((&voters).select(Ident::<User>::new().and(&us).and(&vc).and(rpj.opt())));
    let v = top_n(v, |&(_, (((u, _), n), r))| {
        (Reverse(n), db.user.display_name.get(u).unwrap(), r.is_none(), r.map(|(((_, k), _), _)| k), r.and_then(|(_, h)| h))
    }, 50);
    rows(v.into_iter().map(|(_, (((u, a), n), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), V::I(n)]);
        match r {
            Some((((p, k), c), h)) => {
                f.extend(post_fields(db, p, &["id", "title"]));
                f.extend([V::I(k), V::I(c)]);
                f.extend(match h {
                    Some((h, t)) => [V::T(hd.get(h).unwrap()), ostr(db.post_history.comment.get(h)), V::S(db.vote_type.name.get(t).unwrap())],
                    None => [V::Null, V::Null, V::Null],
                });
                f.push(V::S(if k == 1 { "Top Post" } else if c > 5 { "Frequent Contributor" } else { "New Contributor" }));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::S("New Contributor")]),
        }
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, DisplayName, CreationDate, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        DENSE_RANK() OVER (PARTITION BY EXTRACT(YEAR FROM CreationDate), EXTRACT(MONTH FROM CreationDate) ORDER BY Reputation DESC) AS MonthlyReputationRank FROM Users),
// TopPosters AS (SELECT OwnerUserId, COUNT(Id) AS PostCount FROM Posts WHERE CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY OwnerUserId HAVING COUNT(Id) > 10),
// PopularTags AS (SELECT Tags.TagName, COUNT(Posts.Id) AS PostCount FROM Tags JOIN Posts ON Posts.Tags LIKE '%' || Tags.TagName || '%' GROUP BY Tags.TagName ORDER BY PostCount DESC
//     FETCH FIRST 5 ROWS ONLY),
// PostActivity AS (SELECT P.Id AS PostId, P.Title, SUM(CASE WHEN H.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseOpenCount, AVG(VoteCount.VoteCount) AS AverageVotes,
//        COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount
//     FROM Posts P LEFT JOIN PostHistory H ON H.PostId = P.Id LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) AS VoteCount ON VoteCount.PostId = P.Id
//     LEFT JOIN Comments C ON C.PostId = P.Id GROUP BY P.Id, P.Title),
// FinalReport AS (SELECT U.DisplayName, U.Reputation, U.ReputationRank, U.MonthlyReputationRank, T.PostCount, P.Title, P.CloseOpenCount, P.AverageVotes, P.CommentCount
//     FROM UserReputation U JOIN TopPosters T ON T.OwnerUserId = U.Id JOIN PostActivity P ON P.PostId = (SELECT Id FROM Posts WHERE OwnerUserId = U.Id ORDER BY Score DESC LIMIT 1)
//     WHERE U.ReputationRank <= 100)
// SELECT FR.DisplayName, FR.Reputation, FR.ReputationRank, FR.MonthlyReputationRank, FR.PostCount, FR.Title, FR.CloseOpenCount, FR.AverageVotes, FR.CommentCount, PT.TagName AS PopularTag
// FROM FinalReport FR LEFT JOIN PopularTags PT ON PT.PostCount > 10 ORDER BY FR.Reputation DESC, FR.PostCount DESC;
//
// CURRENT_DATE is the New York date (`today_ny`). The correlated `ORDER BY Score DESC LIMIT 1` is an arg-max per user; it breaks Score ties by post id,
// as does the top-5 tag cut by tag id (the SQL leaves both open). PostActivity is built for the chosen posts alone. The PopularTags ON names only PT,
// so every report row is crossed with the popular tags.
fn q6159(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let User { reputation, creation_date: ucd, .. } = &db.user;
    let wr = whole(reputation).select(Ident::<User>::new().and(reputation)).window(rank, |(_, r)| Reverse(r), asc);
    type X = ((Id<User>, i64), i64);
    let wm = (&wr)
        .group_by(Same::<X>::new().map(|x: X| x.0 .0).select(ucd.map(|d| (year(d), month(d)))))
        .select(Same::<X>::new())
        .window(dense_rank, |((_, r), _): X| Reverse(r), asc);
    type U = (Id<User>, i64, i64);
    let rv: MatSet<U> = (&wm).filt(|((_, k), _)| k <= 100).map(|(((u, _), k), m)| (u, k, m)).collect();
    let tp = db.post.with(creation_date.ge(add_months(today_ny(), -6))).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let wb = db.post.group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let best_of: HashIdx<Id<User>, Id<Post>> = (&wb).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    type F = (U, i64);
    let fr: MatSet<F> = (&rv).select(Same::<U>::new().and(Same::<U>::new().map(|x: U| x.0).select((&tp).filt(|n| n > 10)))).collect();
    let chosen: MatSet<Id<Post>> = (&fr).map(|x: F| x.0 .0).select(&best_of).collect();
    let vcount = (&chosen).group_by(Ident::<Post>::new()).select(votes_of(db)).fold(0i64, |n, _| n + 1);
    let pa = (&chosen)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(&db.post_history.post_history_type_id).opt().and(Ident::<Post>::new().select(&vcount).opt()).and(comments_of(db).opt()))
        .fold([0i64; 4], |a, ((t, vc), c)| [a[0] + matches!(t, Some(10 | 11)) as i64, a[1] + vc.is_some() as i64, a[2] + vc.unwrap_or(0), a[3] + c.is_some() as i64]);
    let lt = like_tags(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let tcount = db.tag.group_by(&db.tag.tag_name).select(&by_tag).fold(0i64, |n, _| n + 1);
    let pt = top_n(drain(&tcount), |&(n, c)| (Reverse(c), n), 5);
    let ptn: MatSet<Str> = rel(pt).filt(|(_, c): (Str, i64)| c > 10).map(|(n, _): (Str, i64)| n).collect();
    let pts: HashIdx<(), Str> = whole(&ptn).collect();
    let v = drain((&fr).select(
        Same::<F>::new()
            .and(Same::<F>::new().map(|x: F| x.0 .0).select(&best_of).select(Ident::<Post>::new().and(&pa)))
            .and(Same::<F>::new().map(|_| ()).select(&pts).opt()),
    ));
    rows(v.into_iter().map(|(_, ((((u, k, m), n), (p, a)), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(k), V::I(m), V::I(n)]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), avg(a[2], a[1]), V::I(a[3])]);
        f.push(t.map_or(V::Null, V::S));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank,
//        COALESCE(u.Reputation, 0) AS UserReputation FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' AND p.PostTypeId = 1),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.ViewCount, rp.Score, rp.UserReputation, CASE WHEN rp.UserReputation > 1000 THEN 'Veteran' ELSE 'Newbie' END AS UserTier FROM RankedPosts rp WHERE rp.PostRank <= 5),
// CommentsSummary AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// FinalResults AS (SELECT tp.Title, tp.ViewCount, tp.Score, tp.UserReputation, tp.UserTier, COALESCE(cs.CommentCount, 0) AS TotalComments, COALESCE(cs.LastCommentDate, '1970-01-01') AS LastCommentDate
//     FROM TopPosts tp LEFT JOIN CommentsSummary cs ON tp.Id = cs.PostId)
// SELECT fr.Title, fr.ViewCount, fr.Score, fr.UserTier, fr.TotalComments, fr.LastCommentDate,
//        'Reputation is ' || CASE WHEN fr.UserReputation IS NULL THEN 'Not Available' WHEN fr.UserReputation < 500 THEN 'Low' WHEN fr.UserReputation BETWEEN 500 AND 1000 THEN 'Moderate' ELSE 'High' END AS ReputationValue,
//        CASE WHEN fr.TotalComments > 10 THEN 'Highly Discussed' WHEN fr.TotalComments BETWEEN 1 AND 10 THEN 'Moderately Discussed' ELSE 'Not Discussed' END AS DiscussionLevel
// FROM FinalResults fr WHERE fr.ViewCount > 100 ORDER BY fr.Score DESC, fr.ViewCount DESC LIMIT 25;
//
// PostRank breaks (Score, ViewCount) ties by post id, and so does the LIMIT (the SQL leaves both open). The ownerless questions rank as one partition.
fn q20737(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, owner_user_id, .. } = &db.post;
    let key = |(p, s, w): (Id<Post>, i64, Option<i64>)| (Reverse(s), w.is_none(), Reverse(w), p);
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, move |((p, s), w)| key((p, s, w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&tp).with(view_count.gt(100)).select(Ident::<Post>::new().and(score).and(view_count.opt()).and(owner_user.select(&db.user.reputation).opt()).and((&cs).opt())));
    let v = top_n(v, |&(_, ((((p, s), w), _), _))| key((p, s, w)), 25);
    rows(v.into_iter().map(|(p, ((_, r), c))| {
        let r = r.unwrap_or(0);
        let (n, d) = c.unwrap_or((0, 0));
        let mut f = post_fields(db, p, &["title", "views", "score"]);
        f.extend([V::S(if r > 1000 { "Veteran" } else { "Newbie" }), V::I(n), V::T(d)]);
        f.push(V::S(if r < 500 { "Reputation is Low" } else if r <= 1000 { "Reputation is Moderate" } else { "Reputation is High" }));
        f.push(V::S(if n > 10 { "Highly Discussed" } else if n >= 1 { "Moderately Discussed" } else { "Not Discussed" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS RN FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AcceptedAnswers AS (SELECT P.Id AS AnswerId, P.AcceptedAnswerId, COUNT(COALESCE(V.Id, 0)) AS UpVoteCount FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 2
//     WHERE P.PostTypeId = 2 GROUP BY P.Id, P.AcceptedAnswerId),
// TagStats AS (SELECT T.TagName, COUNT(*) AS PostCount, SUM(COALESCE(CASE WHEN P.ViewCount IS NULL THEN 0 ELSE P.ViewCount END, 0)) AS TotalViews FROM Tags T
//     JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName),
// RecentPostHistory AS (SELECT PH.PostId, PH.PostHistoryTypeId, PH.CreationDate, PH.UserDisplayName, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS HistoryRank
//     FROM PostHistory PH WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, AA.UpVoteCount, TS.PostCount AS TagPostCount, TS.TotalViews AS TagTotalViews,
//        RPH.UserDisplayName AS RecentEditor, RPH.CreationDate AS RecentEditDate
// FROM RankedPosts RP LEFT JOIN AcceptedAnswers AA ON RP.PostId = AA.AcceptedAnswerId LEFT JOIN TagStats TS ON RP.Title LIKE '%' || TS.TagName || '%'
// LEFT JOIN RecentPostHistory RPH ON RP.PostId = RPH.PostId AND RPH.HistoryRank = 1 WHERE RP.RN <= 5 ORDER BY RP.CreationDate DESC;
//
// RN and HistoryRank break CreationDate ties by id (the SQL leaves them open). The Title LIKE is matched with a LIKE matcher, so a `_` or `%` in a tag name
// stays a wildcard. `COUNT(COALESCE(V.Id, 0))` counts every joined row, the unmatched one included.
fn q32708(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, title, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db
        .post
        .with(creation_date.ge(add_years(t0, -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let answers = || db.post.with(post_type_id.eq(2));
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let aa = answers().group_by(Ident::<Post>::new()).select(up.opt()).fold(0i64, |n, _| n + 1);
    let by_acc: HashIdx<Id<Post>, Id<Post>> = answers().select(&db.post.accepted_answer).inv().collect();
    let lt = like_tags(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db.tag.group_by(&db.tag.tag_name).select((&by_tag).map(|(p, _): (Id<Post>, Id<Tag>)| p).select(view_count.opt())).fold((0i64, 0i64), |(n, s), w| (n + 1, s + w.unwrap_or(0)));
    type T = (Str, (i64, i64));
    let tsm: MatSet<T> = whole(&ts_).select(Same::<Str>::new().and(&ts_)).collect();
    let tsn: HashIdx<Str, T> = (&tsm).map(|(n, _): T| n).inv().collect();
    let titles: MatSet<Str> = (&tp).select(title).collect();
    let hit: HashIdx<Str, T> = (&titles).select_where(&tsn, |t: Str, n: Str| like(t, &format!("%{n}%"))).collect();
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let wh = db.post_history.with(hd.ge(add_days(t0, -30))).group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d)| (Reverse(d), h), asc);
    let last: HashIdx<Id<Post>, (Id<PostHistory>, i64)> = (&wh).filt(|(_, k)| k == 1).map(|(x, _)| x).collect();
    let v = drain((&tp).select((&by_acc).select(&aa).opt().and(title.select(&hit).opt()).and((&last).opt())));
    rows(v.into_iter().map(|(p, ((a, t), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.push(oint(a));
        f.extend(match t {
            Some((_, (n, s))) => [V::I(n), V::I(s)],
            None => [V::Null, V::Null],
        });
        f.extend(match h {
            Some((h, d)) => [ostr(db.post_history.user_display_name.get(h)), V::T(d)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        COUNT(DISTINCT b.Id) AS TotalBadges FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// RecentPostHistory AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, ph.Comment, pst.Name AS PostHistoryType FROM PostHistory ph JOIN PostHistoryTypes pst ON ph.PostHistoryTypeId = pst.Id
//     WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'),
// AggregatedData AS (SELECT up.UserId, up.DisplayName, up.Reputation, up.Views, COALESCE(SUM(rp.Score), 0) AS TotalPostScore, COALESCE(SUM(rp.Score) FILTER (WHERE rp.rn = 1), 0) AS MostRecentPostScore,
//        COALESCE(MAX(rp.CreationDate), TIMESTAMP '1970-01-01') AS LastPostDate, COUNT(rph.PostId) AS RecentEditCount
//     FROM UserStats up LEFT JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId LEFT JOIN RecentPostHistory rph ON rph.UserId = up.UserId GROUP BY up.UserId, up.DisplayName, up.Reputation, up.Views)
// SELECT ad.UserId, ad.DisplayName, ad.Reputation, ad.Views, ad.TotalPostScore, ad.MostRecentPostScore, ad.LastPostDate, ad.RecentEditCount, (ad.Reputation * 0.1 + ad.TotalPostScore * 0.9) AS PerformanceScore
// FROM AggregatedData ad WHERE ad.TotalPostScore > 100 ORDER BY PerformanceScore DESC LIMIT 10;
//
// UserStats has one row per user and none of its aggregates is read; TotalPostScore > 100 needs a recent post, so only their owners are grouped.
// PerformanceScore is DECIMAL arithmetic, so it is computed in exact tenths.
// rn breaks CreationDate ties by post id (the SQL leaves them open).
fn q1657(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let recent = || db.post.with(creation_date.ge(add_years(t0, -1)));
    let w = recent().group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let rp: HashIdx<Id<User>, Id<Post>> = recent().select(owner_user).inv().collect();
    let rph: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with((&db.post_history.creation_date).ge(add_months(t0, -1))).select(&db.post_history.user).inv().collect();
    let owners: MatSet<Id<User>> = recent().select(owner_user).collect();
    let ad = (&owners)
        .group_by(Ident::<User>::new())
        .select((&rp).select(score.and(creation_date).and(Ident::<Post>::new().with(&first).opt())).and((&rph).opt()))
        .fold([0i64, 0, i64::MIN, 0], |a, (((s, d), f), h)| [a[0] + s, a[1] + if f.is_some() { s } else { 0 }, a[2].max(d), a[3] + h.is_some() as i64]);
    let v = drain((&ad).filt(|a| a[0] > 100));
    let v = top_n(v, |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap() + 9 * a[0]), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::T(a[2]), V::I(a[3]), V::F((r + 9 * a[0]) as f64 / 10.0)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank,
//        DENSE_RANK() OVER (ORDER BY p.CreationDate DESC) AS RecentRank FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.AnswerCount > 0),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, COALESCE(SUM(vb.BountyAmount), 0) AS TotalBounty, MAX(pa.CreationDate) AS LastActiveDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes vb ON p.Id = vb.PostId AND vb.VoteTypeId = 8 LEFT JOIN Posts pa ON u.Id = pa.OwnerUserId GROUP BY u.Id, u.DisplayName),
// ActiveUsers AS (SELECT ua.UserId, ua.DisplayName, ua.PostCount, ua.TotalBounty, ua.LastActiveDate,
//        CASE WHEN ua.PostCount > 10 THEN 'High Activity' WHEN ua.PostCount BETWEEN 5 AND 10 THEN 'Medium Activity' ELSE 'Low Activity' END AS ActivityLevel
//     FROM UserActivity ua WHERE ua.LastActiveDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months'),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT up.UserId, up.DisplayName, a.PostId, a.Title, a.CreationDate, a.Score, a.ViewCount, ph.EditCount, ph.LastEditDate, up.TotalBounty, up.ActivityLevel
// FROM ActiveUsers up JOIN RankedPosts a ON up.UserId = a.PostId JOIN PostHistorySummary ph ON a.PostId = ph.PostId WHERE a.ScoreRank = 1 ORDER BY up.ActivityLevel DESC, a.Score DESC LIMIT 50;
//
// `up.UserId = a.PostId` joins a user id to a post id, so it goes through the raw ids; the posts x bounties x posts product is driven only for the users that
// join reaches. ScoreRank breaks Score ties by post id, and the LIMIT breaks ties by post id (the SQL leaves both open).
fn q30807(db: &'static So) -> String {
    let Post { creation_date, score, answer_count, owner_user_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db
        .post
        .with(creation_date.ge(add_years(t0, -1)).and(answer_count.gt(0)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).with(history_of(db)).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let cand: MatSet<Id<User>> = (&fp).select((&db.post.origid).select(&uidx)).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ua = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(bounty.opt()).opt().and(posts_of(db).select(creation_date).opt()))
        .fold((0i64, i64::MIN), |(s, m), (b, d)| (s + b.flatten().flatten().unwrap_or(0), d.map_or(m, |d| m.max(d))));
    let pc = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let since = add_months(t0, -6);
    let phs = (&fp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.creation_date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let v = drain((&cand).select((&ua).filt(move |(_, m)| m >= since).and(&pc).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().with(&fp).and(&phs)))));
    let lvl = |n: i64| if n > 10 { "High Activity" } else if n >= 5 { "Medium Activity" } else { "Low Activity" };
    let v = top_n(v, |&(_, ((_, n), (p, _)))| (Reverse(lvl(n)), Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(u, (((b, _), n), (p, (e, d))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(e), V::T(d), V::I(b), V::S(lvl(n))]);
        row(f)
    }))
}

// WITH RecursiveUserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(SUM(V.BountyAmount), 0) DESC) AS UserRank
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// UserStatistics AS (SELECT UserId, DisplayName, TotalBounty, TotalUpVotes, TotalDownVotes, TotalPosts, TotalComments, UserRank, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM RecursiveUserActivity),
// ClosedQuestions AS (SELECT P.Id AS PostId, P.Title, PH.CreationDate AS ClosedDate, U.DisplayName AS ClosedBy, COUNT(V.Id) AS TotalVotes FROM Posts P
//     JOIN PostHistory PH ON P.Id = PH.PostId AND PH.PostHistoryTypeId = 10 LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Users U ON PH.UserId = U.Id WHERE P.PostTypeId = 1
//     GROUP BY P.Id, P.Title, PH.CreationDate, U.DisplayName),
// TopClosedQuestions AS (SELECT PostId, Title, ClosedDate, ClosedBy, TotalVotes, RANK() OVER (ORDER BY TotalVotes DESC) AS RankByVotes FROM ClosedQuestions)
// SELECT U.DisplayName AS UserName, U.TotalBounty, U.TotalUpVotes, U.TotalDownVotes, U.TotalPosts, U.TotalComments, T.Title AS TopClosedQuestionTitle, T.ClosedDate, T.ClosedBy
// FROM UserStatistics U LEFT JOIN TopClosedQuestions T ON U.UserId = T.PostId WHERE U.UserRank <= 10 AND (T.ClosedDate IS NULL OR T.ClosedBy IS NOT NULL) ORDER BY U.TotalBounty DESC, U.UserRank;
//
// WITH RECURSIVE, but no CTE refers to itself. The votes x posts x comments product is driven for every user. UserRank breaks TotalBounty ties by user id
// (the SQL leaves them open). `U.UserId = T.PostId` joins a user id to a post id, so it goes through the raw ids; PostRank, TotalVotes and RankByVotes are never read.
fn q30857(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let rua = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id.and(bounty_amount.opt())).opt().and(posts_of(db).select(comments_of(db).opt()).opt()))
        .fold([0i64; 3], |a, (v, _)| match v {
            Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
            None => a,
        });
    let top = top_n(drain(&rua), |&(u, a)| (Reverse(a[0]), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, user, .. } = &db.post_history;
    let asked = Ident::<Post>::new().with((&db.post.post_type_id).eq(1));
    let cq = db
        .post_history
        .with(post_history_type_id.eq(10))
        .with(post.select(asked))
        .group_by(post.and(hd).and(user.select(&db.user.display_name).opt()))
        .select(Ident::<PostHistory>::new())
        .fold(0i64, |n, _| n + 1);
    let cqv = rel(drain(&cq));
    type Q = (((Id<Post>, i64), Option<Str>), i64);
    let by_post: HashIdx<i64, Q> = (&cqv).map(|x: Q| x.0 .0 .0).select(&db.post.origid).inv().select(&cqv).collect();
    type R = (Id<User>, ((([i64; 3], i64), i64), Option<Q>));
    let v = drain((&tu).select(Ident::<User>::new().and((&rua).and(&pc).and(&cc).and((&db.user.origid).select(&by_post).opt()))).filt(|(_, (_, t)): R| t.map_or(true, |t| t.0 .1.is_some())));
    rows(v.into_iter().map(|(_, (u, (((a, n), c), t)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n), V::I(c)]);
        f.extend(match t {
            Some((((p, d), by), _)) => [title(db, p), V::T(d), ostr(by)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostsCount, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
//        ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY SUM(COALESCE(P.Score, 0)) DESC) AS Rank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 GROUP BY U.Id, U.DisplayName, U.Reputation),
// ActiveUsers AS (SELECT UserId, DisplayName, Reputation, PostsCount, TotalBounty, TotalViews FROM UserActivity WHERE Rank <= 10),
// ClosedPosts AS (SELECT PH.UserId, COUNT(P.Id) AS ClosedPostsCount, SUM(P.ViewCount) AS ClosedPostsViews, SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseVotes
//     FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.PostHistoryTypeId IN (10, 11) AND PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY PH.UserId),
// FinalStats AS (SELECT AU.UserId, AU.DisplayName, AU.Reputation, AU.PostsCount, AU.TotalBounty, AU.TotalViews, COALESCE(CP.ClosedPostsCount, 0) AS ClosedPostsCount,
//        COALESCE(CP.ClosedPostsViews, 0) AS ClosedPostsViews, COALESCE(CP.CloseVotes, 0) AS CloseVotes FROM ActiveUsers AU LEFT JOIN ClosedPosts CP ON AU.UserId = CP.UserId)
// SELECT UserId, DisplayName, Reputation, PostsCount, TotalBounty, TotalViews, ClosedPostsCount, ClosedPostsViews, CloseVotes, (TotalViews / NULLIF(PostsCount, 0)) AS AvgViewsPerPost,
//        (TotalBounty / NULLIF(ClosedPostsCount, 0)) AS AvgBountyPerClosedPost FROM FinalStats ORDER BY Reputation DESC, TotalViews DESC;
//
// Rank partitions by the user, one row each, so it is always 1 and ActiveUsers is every user.
fn q1125(db: &'static So) -> String {
    let Post { view_count, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(bounty.opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((w, b)) => [a[0] + b.flatten().unwrap_or(0), a[1] + w.unwrap_or(0)],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { user, post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]).and(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(user)
        .select(post.select(view_count.opt()).and(post_history_type_id))
        .fold([0i64; 4], |a, (w, t)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + (t == 10) as i64]);
    let v = drain((&ua).and(&pc).and((&cp).opt()));
    rows(v.into_iter().map(|(u, ((a, n), c))| {
        let c = c.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(c[0]), V::I(if c[1] == 0 { 0 } else { c[2] }), V::I(c[3])]);
        f.push(if n == 0 { V::Null } else { V::F(a[1] as f64 / n as f64) });
        f.push(if c[0] == 0 { V::Null } else { V::F(a[0] as f64 / c[0] as f64) });
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.Id, ph.PostId, ph.UserId, ph.CreationDate, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS rn FROM PostHistory ph),
// UserEngagement AS (SELECT u.Id AS UserId, SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteCount, SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount,
//        SUM(CASE WHEN c.UserId IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Comments c ON u.Id = c.UserId
//     GROUP BY u.Id),
// TopEngagedUsers AS (SELECT ue.UserId, ue.VoteCount, ue.BadgeCount, ue.CommentCount, DENSE_RANK() OVER (ORDER BY (ue.VoteCount + ue.BadgeCount + ue.CommentCount) DESC) AS EngagementRank FROM UserEngagement ue),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, COUNT(c.Id) AS TotalComments FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days') GROUP BY p.Id, p.Title
//     HAVING SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) > 10)
// SELECT pu.UserId, pu.VoteCount, pu.BadgeCount, pu.CommentCount, pp.PostId, pp.Title AS PopularPostTitle, pp.TotalUpVotes, pp.TotalComments, COALESCE(rph.Comment, 'No Comment') AS LastEditComment
// FROM TopEngagedUsers pu JOIN PostLinks pl ON pu.UserId = pl.PostId JOIN PopularPosts pp ON pp.PostId = pl.RelatedPostId LEFT JOIN RecursivePostHistory rph ON pp.PostId = rph.PostId AND rph.rn = 1
// ORDER BY pu.EngagementRank, pp.TotalUpVotes DESC;
//
// `pu.UserId = pl.PostId` joins a user id to a post id, so it goes through the raw ids, and UserEngagement is built only for the users that reach a popular post.
// EngagementRank only orders the output. rn breaks CreationDate ties by history id (the SQL leaves them open).
fn q32415(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let pp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 2], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + c.is_some() as i64]);
    let popular: MatSet<Id<Post>> = db.post.with((&pp).filt(|a| a[0] > 10)).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostLink { post_id, related_post, .. } = &db.post_link;
    let links = || db.post_link.with(related_post.select(Ident::<Post>::new().with(&popular)));
    let cand: MatSet<Id<User>> = links().select(post_id.select(&uidx)).collect();
    let ue = (&cand)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()).and(comments_by(db).opt()))
        .fold([0i64; 3], |a, ((t, b), c)| [a[0] + matches!(t, Some(2 | 3)) as i64, a[1] + b.is_some() as i64, a[2] + c.is_some() as i64]);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let wh = db.post_history.group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d)| (Reverse(d), h), asc);
    let lh: HashIdx<Id<Post>, Id<PostHistory>> = (&wh).filt(|(_, k)| k == 1).map(|((h, _), _)| h).collect();
    let v = drain(links().select(post_id.select(&uidx).select(Ident::<User>::new().and(&ue)).and(related_post.select(Ident::<Post>::new().and(&pp).and((&lh).opt())))));
    rows(v.into_iter().map(|(_, ((u, a), ((p, s), h)))| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(s[0]), V::I(s[1]), V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No Comment"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS Rank,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY P.Id) AS UpVoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY P.Id) AS DownVoteCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.Score > 0 AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostHistoryDetails AS (SELECT PH.PostId, PH.PostHistoryTypeId, PH.UserDisplayName, PH.CreationDate, COALESCE(PH.Comment, 'No Comment') AS Comment FROM PostHistory PH
//     LEFT JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id WHERE PHT.Name IN ('Post Closed', 'Post Reopened', 'Edit Body')
//     AND PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months'),
// RecursiveVotes AS (SELECT P.Id AS PostId, V.VoteTypeId, V.UserId, COUNT(*) AS VoteCount FROM Posts P INNER JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, V.VoteTypeId, V.UserId HAVING COUNT(*) > 2)
// SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, RP.OwnerDisplayName, RP.CreationDate, RP.UpVoteCount, RP.DownVoteCount, COALESCE(PHD.Comment, 'No Activity') AS RecentActivity,
//        CASE WHEN RV.VoteCount IS NOT NULL THEN 'High Voting Activity' ELSE 'Normal Activity' END AS ActivityStatus
// FROM RankedPosts RP LEFT JOIN PostHistoryDetails PHD ON RP.PostId = PHD.PostId LEFT JOIN RecursiveVotes RV ON RP.PostId = RV.PostId WHERE RP.Rank <= 10 ORDER BY RP.Score DESC, RP.ViewCount DESC;
//
// RankedPosts has no GROUP BY, so Rank numbers the post x vote rows; it breaks Score ties by post and vote id (the SQL leaves them open). A post's rows agree in
// every projected column, so only a tie between different posts at the cut could be observed.
fn q33225(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    type J = ((Id<Post>, i64), Option<Id<Vote>>);
    let j: MatSet<J> = db.post.with(score.gt(0).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(Ident::<Post>::new().and(score).and(votes_of(db).opt())).collect();
    let w = (&j).group_by(Same::<J>::new().map(|x: J| x.0 .0).select(post_type_id)).select(Same::<J>::new()).window(row_number, |((p, s), v): J| (Reverse(s), p, v), asc);
    let rp: MatSet<J> = (&w).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let pid = || Same::<J>::new().map(|x: J| x.0 .0);
    let posts: MatSet<Id<Post>> = (&rp).map(|x: J| x.0 .0).collect();
    let vc = (&posts).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { creation_date: hd, comment, .. } = &db.post_history;
    let names = htype_name(db).filt(|n: Str| n == "Post Closed" || n == "Post Reopened" || n == "Edit Body");
    let phd = history_of(db).select(Ident::<PostHistory>::new().with(names).with(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))));
    let Vote { vote_type_id, user_id, .. } = &db.vote;
    let rv = (&posts).group_by(Ident::<Post>::new().and(votes_of(db).select(vote_type_id.and(user_id.opt())))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let rvv = rel(drain((&rv).filt(|n| n > 2)));
    type G = ((Id<Post>, (i64, Option<i64>)), i64);
    let rvp: HashIdx<Id<Post>, G> = (&rvv).map(|x: G| x.0 .0).inv().select(&rvv).collect();
    let v = drain((&rp).select(Same::<J>::new().and(pid().select(&vc)).and(pid().select(phd.opt())).and(pid().select((&rvp).opt()))));
    rows(v.into_iter().map(|(_, (((((p, _), _), a), h), r))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner", "created"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(V::S(h.map_or("No Activity", |h| comment.get(h).unwrap_or("No Comment"))));
        f.push(V::S(if r.is_some() { "High Voting Activity" } else { "Normal Activity" }));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes, RANK() OVER (ORDER BY COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) DESC) AS VoteRank
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostMetrics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, COALESCE(SUM(CASE WHEN C.PostId IS NOT NULL THEN 1 END), 0) AS CommentCount,
//        COALESCE(MAX(PH.CreationDate), P.CreationDate) AS LastUpdateDate FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount),
// CombinedMetrics AS (SELECT PM.PostId, PM.Title, PM.CreationDate, PM.Score, PM.ViewCount, PM.AnswerCount, PM.CommentCount, U.UserId, U.DisplayName AS UserDisplayName, U.UpVotes, U.DownVotes,
//        U.TotalVotes, PM.LastUpdateDate FROM PostMetrics PM JOIN UserVoteStats U ON PM.PostId = (SELECT P.Id FROM Posts P WHERE P.OwnerUserId = U.UserId ORDER BY P.CreationDate DESC LIMIT 1)
//     WHERE U.TotalVotes > 0)
// SELECT CM.*, CASE WHEN CM.AnswerCount = 0 THEN 'No Answers' WHEN CM.AnswerCount > 0 AND CM.Score < 0 THEN 'Unpopular' ELSE 'Popular' END AS PostPopularity,
//        (EXTRACT(EPOCH FROM CM.LastUpdateDate) - EXTRACT(EPOCH FROM CM.CreationDate)) / 60 AS MinutesSinceLastUpdate
// FROM CombinedMetrics CM ORDER BY CM.UpVotes DESC, CM.ViewCount DESC;
//
// CURRENT_DATE is the New York date (`today_ny`). The correlated `ORDER BY CreationDate DESC LIMIT 1` is an arg-max per user, breaking ties by post id
// (the SQL leaves them open). VoteRank is never read.
fn q305(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, answer_count, .. } = &db.post;
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| {
        [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + matches!(t, 2 | 3) as i64]
    });
    let wl = db.post.group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let latest_of: HashIdx<Id<Post>, Id<User>> = (&wl).filt(|(_, k)| k == 1).map(|((p, _), _)| p).inv().collect();
    let pm = db
        .post
        .with(creation_date.ge(add_months(today_ny(), -6)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, d.map_or(m, |d| m.max(d))));
    let v = drain((&pm).and((&latest_of).select(Ident::<User>::new().and((&uvs).filt(|a| a[2] > 0)))));
    rows(v.into_iter().map(|(p, ((n, m), (u, a)))| {
        let cd = creation_date.get(p).unwrap();
        let last = if m == i64::MIN { cd } else { m };
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(n));
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::T(last)]);
        f.push(V::S(match answer_count.get(p) {
            Some(0) => "No Answers",
            Some(x) if x > 0 && s < 0 => "Unpopular",
            _ => "Popular",
        }));
        f.push(V::F((secs(last) - secs(cd)) / 60.0));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation > 1000),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, P.ViewCount, P.AnswerCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND P.PostTypeId = 1),
// VoteSummary AS (SELECT PV.PostId, COUNT(CASE WHEN PV.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN PV.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes PV GROUP BY PV.PostId),
// PostHistoryInfo AS (SELECT PH.PostId, PH.PostHistoryTypeId, PH.CreationDate, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS HistoryRank,
//        CASE WHEN PH.PostHistoryTypeId = 10 THEN 'Closed' WHEN PH.PostHistoryTypeId = 11 THEN 'Reopened' ELSE 'Other' END AS ChangeType FROM PostHistory PH)
// SELECT RU.DisplayName AS TopContributor, RU.Reputation AS ContributorReputation, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate, COALESCE(VS.UpVotes, 0) AS TotalUpVotes,
//        COALESCE(VS.DownVotes, 0) AS TotalDownVotes, PH.ChangeType AS PostChangeType, PH.CreationDate AS ChangeDate
// FROM RankedUsers RU LEFT JOIN RecentPosts RP ON RU.UserId = RP.OwnerUserId LEFT JOIN VoteSummary VS ON RP.PostId = VS.PostId LEFT JOIN PostHistoryInfo PH ON RP.PostId = PH.PostId AND PH.HistoryRank = 1
// WHERE RU.ReputationRank <= 10 AND (RP.RecentRank = 1 OR RP.RecentRank IS NULL) ORDER BY RU.Reputation DESC, RP.CreationDate DESC;
//
// ReputationRank, RecentRank and HistoryRank break their ties by id (the SQL leaves them open).
fn q32785(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.eq(1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let wh = db.post_history.group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d)| (Reverse(d), h), asc);
    let lh: HashIdx<Id<Post>, (Id<PostHistory>, i64)> = (&wh).filt(|(_, k)| k == 1).map(|(x, _)| x).collect();
    let v = drain((&tu).select((&rp).select(Ident::<Post>::new().and((&vs).opt()).and((&lh).opt())).opt()));
    rows(v.into_iter().map(|(u, r)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        match r {
            Some(((p, a), h)) => {
                let a = a.unwrap_or([0, 0]);
                f.extend(post_fields(db, p, &["title", "created"]));
                f.extend([V::I(a[0]), V::I(a[1])]);
                f.extend(match h {
                    Some((h, d)) => {
                        let t = post_history_type_id.get(h).unwrap();
                        [V::S(if t == 10 { "Closed" } else if t == 11 { "Reopened" } else { "Other" }), V::T(d)]
                    }
                    None => [V::Null, V::Null],
                });
            }
            None => f.extend([V::Null, V::Null, V::I(0), V::I(0), V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostMetrics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        AVG(P.Score) FILTER (WHERE P.Score IS NOT NULL) AS AverageScore, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers
//     FROM Posts P GROUP BY P.OwnerUserId),
// UserStatistics AS (SELECT U.Id, U.DisplayName, U.Reputation, U.LastAccessDate, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PM.PostCount, 0) AS PostCount, COALESCE(PM.TotalViews, 0) AS TotalViews,
//        COALESCE(PM.TotalScore, 0) AS TotalScore, COALESCE(PM.AverageScore, 0) AS AverageScore, COALESCE(PM.Questions, 0) AS Questions, COALESCE(PM.Answers, 0) AS Answers
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostMetrics PM ON U.Id = PM.OwnerUserId),
// RankedUsers AS (SELECT US.*, RANK() OVER (ORDER BY US.Reputation DESC, US.BadgeCount DESC, US.TotalScore DESC) AS UserRank FROM UserStatistics US)
// SELECT R.DisplayName, R.Reputation, R.BadgeCount, R.PostCount, R.TotalViews, R.TotalScore, R.AverageScore, R.Questions, R.Answers,
//        CASE WHEN R.UserRank <= 10 THEN 'Top User' WHEN R.UserRank <= 50 THEN 'Active User' ELSE 'New User' END AS UserCategory
// FROM RankedUsers R WHERE R.LastAccessDate <= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' ORDER BY R.UserRank;
fn q4516(db: &'static So) -> String {
    let Post { view_count, score, post_type_id, owner_user, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pm = db.post.group_by(owner_user).select(view_count.opt().and(score).and(post_type_id)).fold([0i64; 5], |a, ((w, s), t)| {
        [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + (t == 1) as i64, a[4] + (t == 2) as i64]
    });
    let w = whole(&bc)
        .select(Ident::<User>::new().and(&db.user.reputation).and(&bc).and((&pm).opt()).and(&db.user.last_access_date))
        .window(rank, |((((_, r), b), p), _)| (Reverse(r), Reverse(b), Reverse(p.map_or(0, |p| p[2]))), asc);
    let cut = add_months(ts(2024, 10, 1, 12, 34, 56), -1);
    let v = drain((&w).filt(move |((_, la), _)| la <= cut));
    rows(v.into_iter().map(|(_, (((((u, _), b), p), _), k))| {
        let p = p.unwrap_or([0; 5]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), if p[0] == 0 { V::F(0.0) } else { avg(p[2], p[0]) }, V::I(p[3]), V::I(p[4])]);
        f.push(V::S(if k <= 10 { "Top User" } else if k <= 50 { "Active User" } else { "New User" }));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT u.Id AS UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
//        COUNT(DISTINCT p.Id) AS PostsVotedOn, COUNT(DISTINCT b.Id) AS BadgesReceived
//     FROM Users u LEFT OUTER JOIN Votes v ON u.Id = v.UserId LEFT OUTER JOIN Posts p ON v.PostId = p.Id LEFT OUTER JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostSummary AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, COALESCE(CC.CommentCount, 0) AS CommentCount, COALESCE(PH.EditCount, 0) AS EditCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) CC ON p.Id = CC.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS EditCount FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6, 10, 13) GROUP BY PostId) PH ON p.Id = PH.PostId)
// SELECT u.DisplayName, u.Reputation, u.CreationDate, s.UpVotesCount, s.DownVotesCount, s.PostsVotedOn, s.BadgesReceived, ps.PostId, ps.Title, ps.CommentCount, ps.EditCount,
//        GREATEST(0, s.UpVotesCount - s.DownVotesCount) AS NetVotes, CASE WHEN ps.RecentPostRank <= 5 THEN 'Recent' ELSE 'Older' END AS PostRecency, COALESCE(t.TagName, 'No Tags') AS Tags
// FROM UserVoteSummary s JOIN Users u ON u.Id = s.UserId LEFT JOIN PostSummary ps ON ps.RecentPostRank <= 5 LEFT JOIN Tags t ON ps.PostId = t.ExcerptPostId
// WHERE (s.PostsVotedOn > 0 OR s.BadgesReceived > 0) AND u.Reputation IS NOT NULL AND (LOWER(u.Location) LIKE '%remote%' OR u.Location IS NULL)
// ORDER BY u.Reputation DESC, ps.CommentCount DESC;
//
// The PostSummary ON names only ps, so the users are crossed with each type's five newest posts. RecentPostRank breaks CreationDate ties by post id
// (the SQL leaves them open). The votes x badges product is driven for the users the Location filter keeps.
fn q21305(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let users = || db.user.with((&db.user.location).opt().filt(|l: Option<Str>| l.map_or(true, |l| l.to_lowercase().contains("remote"))));
    let ud = users()
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pv = users().group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.post)).count_distinct();
    let br = users().group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ec = (&tp)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6, 10, 13]))).opt())
        .fold(0i64, |n, h| n + h.is_some() as i64);
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    type S = (Id<Post>, ((i64, i64), Option<Id<Tag>>));
    let ps: HashIdx<(), S> = whole(&tp).select(Ident::<Post>::new().and((&cc).and(&ec).and((&excerpt).opt()))).collect();
    type U = (([i64; 2], Option<i64>), Option<i64>);
    let v = drain(
        users()
            .select((&ud).and((&pv).opt()).and((&br).opt()))
            .filt(|((_, n), b): U| n.unwrap_or(0) > 0 || b.unwrap_or(0) > 0)
            .and(Ident::<User>::new().map(|_| ()).select(&ps).opt()),
    );
    rows(v.into_iter().map(|(u, (((a, n), b), p))| {
        let mut f = ucols(db, u, &["name", "rep", "ucreated"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n.unwrap_or(0)), V::I(b.unwrap_or(0))]);
        match p {
            Some((p, ((c, e), t))) => {
                f.extend(post_fields(db, p, &["id", "title"]));
                f.extend([V::I(c), V::I(e), V::I((a[0] - a[1]).max(0)), V::S("Recent")]);
                f.push(V::S(t.map_or("No Tags", |t| db.tag.tag_name.get(t).unwrap())));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::I((a[0] - a[1]).max(0)), V::S("Older"), V::S("No Tags")]),
        }
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (PARTITION BY CASE WHEN u.Reputation >= 1000 THEN 'Gold' WHEN u.Reputation BETWEEN 500 AND 999 THEN 'Silver'
//        ELSE 'Bronze' END ORDER BY u.Reputation DESC) AS Rank FROM Users u),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, DATE_PART('day', cast('2024-10-01 12:34:56' as timestamp) - p.CreationDate) AS DaysOld FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// PostsWithComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT u.DisplayName, u.Reputation, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, rp.Title AS RecentPostTitle,
//        rp.DaysOld, pwc.CommentCount
// FROM RankedUsers u LEFT JOIN UserBadges ub ON u.UserId = ub.UserId LEFT JOIN RecentPosts rp ON u.UserId = rp.OwnerUserId LEFT JOIN PostsWithComments pwc ON rp.PostId = pwc.PostId
// WHERE (u.Reputation > 500 OR ub.GoldBadges IS NOT NULL) AND (rp.DaysOld < 15 OR pwc.CommentCount > 5) AND (ub.GoldBadges IS NULL OR ub.SilverBadges IS NULL OR ub.BronzeBadges IS NULL)
// ORDER BY u.Reputation DESC, rp.DaysOld ASC LIMIT 100;
//
// Rank is never read. The LIMIT breaks ties by user and post id.
fn q21187(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_days(t0, -30))));
    let pwc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let days = move |d: i64| (t0 - d) / DAY_US;
    type P = ((Id<Post>, i64), i64);
    type R = ((i64, Option<[i64; 3]>), Option<P>);
    let v = drain(
        db.user
            .select((&db.user.reputation).and((&ub).opt()).and(recent.select(Ident::<Post>::new().and(creation_date).and(&pwc)).opt()))
            .filt(move |((r, b), p): R| (r > 500 || b.is_some()) && p.map_or(false, |((_, d), c)| days(d) < 15 || c > 5) && b.is_none()),
    );
    let v = top_n(v, |&(u, ((r, _), p))| (Reverse(r), p.map(|((_, d), _)| days(d)), u, p.map(|x| x.0 .0)), 100);
    rows(v.into_iter().map(|(u, ((_, b), p))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(match p {
            Some(((p, d), c)) => [title(db, p), V::I(days(d)), V::I(c)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// Rewritten (rewrites/24601.sql): VoteId and CloseId carried through, the SELECT * spelled out, and the LEAD window and the final ORDER BY refined with
// `, PostId, VoteId, CloseId`.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, v.Id AS VoteId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) OVER (PARTITION BY p.Id) AS UpvoteCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) OVER (PARTITION BY p.Id) AS DownvoteCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= (cast('2024-10-01' as date) - INTERVAL '1 year') AND p.Score IS NOT NULL),
// ClosedPosts AS (SELECT p.Id AS ClosedPostId, p.Title, ph.CreationDate AS CloseDate, ph.Comment AS CloseReason, ph.Id AS CloseId FROM Posts p
//     JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10 WHERE ph.CreationDate >= (cast('2024-10-01' as date) - INTERVAL '1 year')),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpvoteCount, rp.DownvoteCount, rp.VoteId,
//        CASE WHEN rp.Score IS NULL THEN 'No Score' WHEN rp.Score > 0 THEN 'Positive' WHEN rp.Score < 0 THEN 'Negative' ELSE 'Zero Score' END AS ScoreCategory FROM RankedPosts rp),
// Combined AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.UpvoteCount, ps.DownvoteCount, ps.ScoreCategory, COALESCE(cp.CloseReason, 'Not Closed') AS ClosureStatus,
//        ps.VoteId, cp.CloseId FROM PostStatistics ps LEFT JOIN ClosedPosts cp ON ps.PostId = cp.ClosedPostId)
// SELECT PostId, Title, CreationDate, Score, ViewCount, UpvoteCount, DownvoteCount, ScoreCategory, ClosureStatus, (DownvoteCount * 1.0 / NULLIF(UpvoteCount, 0)) AS DownvoteToUpvoteRatio,
//        LEAD(CreationDate) OVER (ORDER BY CreationDate, PostId, VoteId, CloseId) AS NextPostCreationDate
// FROM Combined WHERE ScoreCategory != 'Zero Score' ORDER BY Score DESC, ClosureStatus ASC, PostId, VoteId, CloseId LIMIT 50;
//
// PostRank is never read. The LEAD is a window over every Combined row the WHERE keeps.
fn q24601(db: &'static So) -> String {
    let Post { creation_date, score, origid, .. } = &db.post;
    let since = ts(2023, 10, 1, 0, 0, 0);
    let PostHistory { post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10).and(hd.ge(since))));
    let base = || db.post.with(creation_date.ge(since)).with(score.ne(0));
    let vc = base().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    type J = (((Id<Post>, i64), i64), (Option<(Id<Vote>, i64)>, Option<(Id<PostHistory>, i64)>));
    let j: MatSet<J> = base()
        .select(
            Ident::<Post>::new()
                .and(creation_date)
                .and(origid)
                .and(votes_of(db).select(Ident::<Vote>::new().and(&db.vote.origid)).opt().and(cp.select(Ident::<PostHistory>::new().and(&db.post_history.origid)).opt())),
        )
        .collect();
    let led = whole(&j).window(lead, |(((_, d), pid), (v, h)): J| (d, pid, v.is_none(), v.map(|x| x.1), h.is_none(), h.map(|x| x.1)), asc);
    type L = (J, Option<(i64, i64, bool, Option<i64>, bool, Option<i64>)>);
    let v = drain((&led).select(Same::<L>::new().and(Same::<L>::new().map(|x: L| x.0 .0 .0 .0).select(&vc))));
    let status = |h: Option<(Id<PostHistory>, i64)>| h.map_or("Not Closed", |(h, _)| comment.get(h).unwrap_or("Not Closed"));
    let v = top_n(v, |&(_, (((((p, _), pid), (vv, h)), _), _))| {
        let (vi, hi) = (vv.map(|x| x.1), h.map(|x| x.1));
        (Reverse(score.get(p).unwrap()), status(h), pid, vi.is_none(), vi, hi.is_none(), hi)
    }, 50);
    rows(v.into_iter().map(|(_, (((((p, _), _), (_, h)), next), a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if s > 0 { "Positive" } else { "Negative" }), V::S(status(h))]);
        f.push(if a[0] == 0 { V::Null } else { V::F(a[1] as f64 / a[0] as f64) });
        f.push(ots(next.map(|x| x.0)));
        row(f)
    }))
}

// WITH UserInteractions AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, COUNT(DISTINCT c.Id) AS TotalComments, MAX(c.CreationDate) AS LastCommentDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// PostHistoryAggregates AS (SELECT ph.UserId, COUNT(ph.Id) AS HistoryCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN ph.PostId END) AS ClosureChanges FROM PostHistory ph JOIN Posts p ON p.Id = ph.PostId GROUP BY ph.UserId),
// EngagementMetrics AS (SELECT ui.UserId, ui.DisplayName, ui.TotalPosts, ui.PositivePosts, ui.NegativePosts, ui.TotalComments, ph.HistoryCount, ph.AcceptedAnswers, ph.ClosureChanges,
//        (COALESCE(ui.TotalPosts, 0) + COALESCE(ui.TotalComments, 0) + COALESCE(ph.HistoryCount, 0)) AS EngagementScore FROM UserInteractions ui LEFT JOIN PostHistoryAggregates ph ON ui.UserId = ph.UserId),
// RankedUsers AS (SELECT *, RANK() OVER (ORDER BY EngagementScore DESC) AS UserRank FROM EngagementMetrics)
// SELECT ru.DisplayName, ru.TotalPosts, ru.PositivePosts, ru.NegativePosts, ru.TotalComments, ru.HistoryCount, ru.AcceptedAnswers, ru.ClosureChanges,
//        CASE WHEN ru.UserRank <= 10 THEN 'Top Contributor' WHEN ru.UserRank BETWEEN 11 AND 50 THEN 'Contributor' ELSE 'Occasional User' END AS UserCategory
// FROM RankedUsers ru WHERE ru.AcceptedAnswers > 0 OR ru.ClosureChanges > 0 ORDER BY ru.UserRank;
fn q22642(db: &'static So) -> String {
    let Post { score, accepted_answer_id, .. } = &db.post;
    let ui = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(comments_of(db).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((s, _)) => [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { user, post, post_history_type_id, post_id, .. } = &db.post_history;
    let pha = db.post_history.group_by(user).select(post.select(accepted_answer_id.opt())).fold((0i64, 0i64), |(n, a), x| (n + 1, a + x.is_some() as i64));
    let cch = db.post_history.with(post_history_type_id.is_in([10, 11])).with(post).group_by(user).select(post_id).count_distinct();
    let w = whole(&ui)
        .select(Ident::<User>::new().and(&ui).and(&pc).and(&cc).and((&pha).opt()).and((&cch).opt()))
        .window(rank, |(((((_, _), n), c), h), _)| Reverse(n + c + h.map_or(0, |h| h.0)), asc);
    let v = drain((&w).filt(|(((_, h), x), _)| h.map_or(false, |h| h.1 > 0) || x.unwrap_or(0) > 0));
    rows(v.into_iter().map(|(_, ((((((u, a), n), c), h), x), k))| {
        let h = h.unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(c), V::I(h.0), V::I(h.1), V::I(x.unwrap_or(0))];
        f.push(V::S(if k <= 10 { "Top Contributor" } else if k <= 50 { "Contributor" } else { "Occasional User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank FROM Posts p
//     WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// ActiveUsers AS (SELECT ur.UserId, ur.Reputation, ur.PostCount, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS ClosedPosts
//     FROM UserReputation ur LEFT JOIN Posts p ON ur.UserId = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY ur.UserId, ur.Reputation, ur.PostCount, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges)
// SELECT au.UserId, au.Reputation, au.PostCount, au.GoldBadges, au.SilverBadges, au.BronzeBadges, COALESCE(SUM(rp.Score), 0) AS TotalScore, COUNT(DISTINCT rp.Id) AS TopPosts,
//        AVG(rp.Score) AS AvgPostScore, CASE WHEN au.ClosedPosts > 0 THEN 'Has Closed Posts' ELSE 'No Closed Posts' END AS PostClosureStatus
// FROM ActiveUsers au LEFT JOIN RankedPosts rp ON au.UserId = rp.OwnerUserId AND rp.Rank <= 3
// GROUP BY au.UserId, au.Reputation, au.PostCount, au.GoldBadges, au.SilverBadges, au.BronzeBadges, au.ClosedPosts ORDER BY au.Reputation DESC, TotalScore DESC LIMIT 10;
//
// The WHERE on p makes ActiveUsers the users with a post from the last year. The order reads only Reputation and the top-3 score sum, so the ten users are
// picked first and the posts x badges product is driven for those alone. Rank breaks Score ties by post id, and the LIMIT by user id.
fn q515(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(score.gt(0))).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let rp = (&w).filt(|(_, k)| k <= 3).fold((0i64, 0i64), |(n, t), ((_, s), _)| (n + 1, t + s));
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let active: MatSet<Id<User>> = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user).collect();
    let v = top_n(drain((&active).select((&db.user.reputation).and((&rp).opt()))), |&(u, (r, t))| (Reverse(r), Reverse(t.map_or(0, |t| t.1)), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ur = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 3], |a, (_, c)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cl = (&tu).group_by(Ident::<User>::new()).select(recent.select(history_of(db).select(&db.post_history.post_history_type_id).opt())).fold(0i64, |n, t| n + (t == Some(10)) as i64);
    let v = drain((&tu).select((&pc).and(&ur).and(&cl).and((&rp).opt())));
    rows(v.into_iter().map(|(u, (((n, b), c), r))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(n));
        f.extend(b.map(V::I));
        f.extend(match r {
            Some((k, s)) => [V::I(s), V::I(k), avg(s, k)],
            None => [V::I(0), V::I(0), V::Null],
        });
        f.push(V::S(if c > 0 { "Has Closed Posts" } else { "No Closed Posts" }));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(c.Id) DESC) AS RankByComments,
//        AVG(CASE WHEN v.VoteTypeId = 8 THEN v.BountyAmount END) AS AvgBounty FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.PostTypeId, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.Reputation > 1000 AND u.LastAccessDate <= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY u.Id, u.Reputation),
// FinalStats AS (SELECT ps.PostId, ps.Title, ps.CommentCount, ps.UpVotes, ps.DownVotes, ur.UserId, ur.Reputation, ur.BadgeCount, CASE WHEN ps.RankByComments = 1 THEN 'Top Commenter' ELSE NULL END AS CommenterStatus
//     FROM PostStats ps LEFT JOIN UserReputation ur ON ps.PostId = ur.UserId)
// SELECT fs.PostId, fs.Title, fs.CommentCount, fs.UpVotes, fs.DownVotes, fs.Reputation, COALESCE(fs.BadgeCount, 0) AS BadgeCount,
//        CONCAT('User has ', COALESCE(fs.BadgeCount, 0), ' badges (', CASE WHEN fs.BadgeCount >= 10 THEN 'A badge collector!' WHEN fs.BadgeCount >= 5 THEN 'A proficient user!' ELSE 'Just starting out!' END, ')') AS BadgeMessage
// FROM FinalStats fs WHERE fs.Reputation IS NOT NULL ORDER BY fs.UpVotes - fs.DownVotes DESC, fs.CommentCount DESC LIMIT 50;
//
// `ps.PostId = ur.UserId` joins a post id to a user id, so it goes through the raw ids; the WHERE keeps only matched posts, so the comment x vote product is
// driven for those alone. RankByComments and AvgBounty are never read. The LIMIT breaks ties by post id.
fn q20609(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ur = Ident::<User>::new().with((&db.user.reputation).gt(1000)).with((&db.user.last_access_date).le(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let ps: MatSet<Id<Post>> = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).with((&db.post.origid).select(&uidx).select(ur)).collect();
    let st = (&ps).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&st).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&bc))));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[1] - a[2]), Reverse(a[0]), p), 50);
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), user_col(db, u, "rep"), V::I(b)]);
        f.push(V::Owned(format!("User has {b} badges ({})", if b >= 10 { "A badge collector!" } else if b >= 5 { "A proficient user!" } else { "Just starting out!" })));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.AcceptedAnswerId, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank,
//        COALESCE(NULLIF(p.LastEditDate, p.CreationDate), p.CreationDate) AS MostRecentEdit FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// CommentsSummary AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, AVG(LENGTH(c.Text)) AS AverageCommentLength FROM Comments c GROUP BY c.PostId),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.MostRecentEdit, COALESCE(ub.BadgeCount, 0) AS UserBadgeCount, COALESCE(cs.CommentCount, 0) AS TotalComments,
//        COALESCE(cs.AverageCommentLength, 0) AS AvgCommentLength FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON ub.UserId = u.Id LEFT JOIN CommentsSummary cs ON cs.PostId = rp.PostId)
// SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.CreationDate, ps.UserBadgeCount, ps.TotalComments, ps.AvgCommentLength,
//        CASE WHEN ps.Score >= 10 THEN 'Highly Rated' WHEN ps.Score BETWEEN 5 AND 9 THEN 'Moderately Rated' ELSE 'Low Rated' END AS RatingCategory,
//        CASE WHEN ps.MostRecentEdit = ps.CreationDate THEN 'No Edits' WHEN ps.MostRecentEdit IS NULL THEN 'Never Edited' ELSE 'Edited' END AS EditStatus
// FROM PostStats ps WHERE ps.TotalComments > 5 ORDER BY ps.Score DESC, ps.ViewCount DESC;
//
// PostRank is never read. MostRecentEdit is LastEditDate when it is present and differs from CreationDate, else CreationDate.
fn q1854(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, last_edit_date, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cs = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.text)).fold((0i64, 0i64), |(n, l), t| (n + 1, l + t.chars().count() as i64));
    let v = drain(
        db.post
            .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
            .select(owner_user.select(&bc).and((&cs).filt(|(n, _)| n > 5)).and(last_edit_date.opt())),
    );
    rows(v.into_iter().map(|(p, ((b, (n, l)), e))| {
        let s = score.get(p).unwrap();
        let cd = creation_date.get(p).unwrap();
        let edited = e.map_or(false, |e| e != cd);
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.extend([V::I(b), V::I(n), avg(l, n)]);
        f.push(V::S(if s >= 10 { "Highly Rated" } else if s >= 5 { "Moderately Rated" } else { "Low Rated" }));
        f.push(V::S(if edited { "Edited" } else { "No Edits" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        DENSE_RANK() OVER (ORDER BY p.Score DESC) AS score_rank, COALESCE((SELECT COUNT(c.Id) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 10 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostInteractions AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, COUNT(DISTINCT c.Id) AS CommentCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.LastActivityDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 DAYS' GROUP BY p.Id)
// SELECT us.DisplayName, us.Reputation, us.GoldBadges, us.SilverBadges, us.BronzeBadges, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, pi.VoteCount, pi.CommentCount
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId JOIN PostInteractions pi ON rp.PostId = pi.PostId WHERE rp.rn = 1 AND pi.VoteCount > 5 AND pi.CommentCount > 2
// ORDER BY us.Reputation DESC, rp.Score DESC LIMIT 50;
//
// rn breaks CreationDate ties by post id (the SQL leaves them open). The badges x posts product is driven only for the owners of the posts that pass.
fn q24500(db: &'static So) -> String {
    let Post { creation_date, last_activity_date, owner_user, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).with(last_activity_date.ge(add_days(t0, -30))).collect();
    let vc = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).opt().and(comments_of(db).opt())).fold(0i64, |n, (v, _)| n + v.is_some() as i64);
    let cc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pi = (&vc).filt(|n| n > 5).and((&cc).filt(|n| n > 2));
    let owners: MatSet<Id<User>> = (&fp).with(&pi).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(10)))).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt())).fold([0i64; 3], |a, (c, _)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let v = drain((&fp).select(owner_user.select(Ident::<User>::new().and(&us))).and(pi));
    let v = top_n(v, |&(p, ((u, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((u, b), (n, c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(n), V::I(c)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY BadgeCount DESC) AS BadgeRank FROM UserBadges),
// ActivePosts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews FROM Posts p
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ap.PostCount, 0) AS PostCount, COALESCE(ap.TotalScore, 0) AS TotalScore, COALESCE(ap.TotalViews, 0) AS TotalViews, ub.BadgeCount,
//        ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges FROM Users u LEFT JOIN ActivePosts ap ON u.Id = ap.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId),
// FinalResults AS (SELECT up.UserId, up.DisplayName, up.PostCount, up.TotalScore, up.TotalViews, up.BadgeCount, up.GoldBadges, up.SilverBadges, up.BronzeBadges, t.BadgeRank
//     FROM UserPerformance up LEFT JOIN TopUsers t ON up.UserId = t.UserId)
// SELECT UserId, DisplayName, PostCount, TotalScore, TotalViews, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, BadgeRank FROM FinalResults WHERE BadgeRank IS NOT NULL ORDER BY BadgeRank;
fn q8674(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ap = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let w = whole(&ub).select(Ident::<User>::new().and(&ub).and((&ap).opt())).window(rank, |((_, b), _)| Reverse(b[0]), asc);
    rows(drain(&w).into_iter().map(|(_, (((u, b), a), k))| {
        let a = a.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(if a[2] == 0 { 0 } else { a[3] })]);
        f.extend(b.map(V::I));
        f.push(V::I(k));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        AVG(COALESCE(v.VoteTypeId, 0)) AS AverageVoteType, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.Reputation),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId, COALESCE(pc.CommentCount, 0) AS Comments, COALESCE(ph.RevisionCount, 0) AS RevisionCount,
//        CASE WHEN p.Score IS NULL THEN 'No Score' WHEN p.Score > 0 THEN 'Positive' WHEN p.Score < 0 THEN 'Negative' ELSE 'Unknown' END AS ScoreDescription, p.Tags AS PostTags
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) pc ON p.Id = pc.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS RevisionCount FROM PostHistory GROUP BY PostId) ph ON p.Id = ph.PostId),
// RankedPosts AS (SELECT pd.PostId, pd.Title, pd.Comments, pd.RevisionCount, pd.ScoreDescription, pd.PostTags, DENSE_RANK() OVER (ORDER BY pd.Comments DESC) AS CommentRank FROM PostDetails pd
//     WHERE pd.RevisionCount > 0)
// SELECT ur.UserId, ur.Reputation, ur.PostCount, ur.PositivePosts, ur.AverageVoteType, rp.PostId, rp.Title, rp.Comments, rp.RevisionCount, rp.ScoreDescription, rp.PostTags
// FROM UserReputation ur JOIN RankedPosts rp ON ur.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) WHERE ur.Reputation IS NOT NULL AND rp.CommentRank <= 5
// ORDER BY ur.Reputation DESC, rp.Comments DESC;
//
// The subquery looks the post up by its primary key, so ur is the post's owner; UserReputation is built for those owners alone. Rank is never read.
fn q23012(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let pd = db.post.with(history_of(db)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = db.post.group_by(Ident::<Post>::new()).select(history_of(db)).fold(0i64, |n, _| n + 1);
    let w = whole(&pd).select(Ident::<Post>::new().and(&pd)).window(dense_rank, |(_, c)| Reverse(c), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let owners: MatSet<Id<User>> = (&rp).select(owner_user).collect();
    let ur = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, t)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + t.unwrap_or(0)],
        None => [a[0] + 1, a[1], a[2]],
    });
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&rp).select(owner_user.select(Ident::<User>::new().and(&ur).and(&pc)).and(&pd).and(&hc)));
    rows(v.into_iter().map(|(p, ((((u, a), n), c), h))| {
        let s = score.get(p).unwrap();
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::I(a[1]), avg(a[2], a[0])]);
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(c), V::I(h), V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Unknown" })]);
        f.extend(post_fields(db, p, &["tags"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank,
//        COALESCE(v.TotalVotes, 0) AS TotalVotes FROM Posts p
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 WHEN VoteTypeId = 3 THEN -1 ELSE 0 END) AS TotalVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// ClosedPosts AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate ELSE NULL END) AS ClosedDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, pc.CommentCount, cp.ClosedDate, CASE WHEN cp.ClosedDate IS NOT NULL THEN 'Yes' ELSE 'No' END AS IsClosed,
//        CASE WHEN rp.Rank = 1 AND rp.TotalVotes >= 5 THEN 'Hot Post' ELSE 'Regular Post' END AS PostCategory
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id FULL OUTER JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId
// LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.Rank <= 3 ORDER BY rp.TotalVotes DESC, rp.CreationDate DESC;
//
// The WHERE on rp drops the rows only UserBadges contributes to the FULL OUTER JOIN, and every user has a UserBadges row, so it is the owner's badges.
fn q21268(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(dense_rank, |(_, d)| Reverse(d), asc);
    type K = (Id<Post>, i64);
    let tp: MatSet<K> = (&w).filt(|(_, k)| k <= 3).map(|((p, _), k)| (p, k)).collect();
    let tv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold(0i64, |n, t| n + (t == 2) as i64 - (t == 3) as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let pc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let cp = db.post_history.group_by(&db.post_history.post).select((&db.post_history.post_history_type_id).and(&db.post_history.creation_date)).fold(i64::MIN, |m, (t, d)| if t == 10 { m.max(d) } else { m });
    let pid = || Same::<K>::new().map(|x: K| x.0);
    let v = drain((&tp).select(Same::<K>::new().and(pid().select(owner_user.select(&ub))).and(pid().select((&tv).opt())).and(pid().select((&pc).opt())).and(pid().select((&cp).opt()))));
    rows(v.into_iter().map(|(_, (((((p, k), b), t), c), cl))| {
        let cl = cl.filter(|&d| d != i64::MIN);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(b.map(V::I));
        f.extend([oint(c), ots(cl), V::S(if cl.is_some() { "Yes" } else { "No" })]);
        f.push(V::S(if k == 1 && t.unwrap_or(0) >= 5 { "Hot Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH UserVotes AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes,
//        SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN (CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE -1 END) ELSE 0 END) AS VoteBalance,
//        ROW_NUMBER() OVER (PARTITION BY u.Reputation ORDER BY COUNT(v.VoteTypeId) DESC) AS VoteRank
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation IS NOT NULL GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentsCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(DISTINCT b.Id) AS TotalBadges
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
//     GROUP BY p.Id, p.Title, p.CreationDate),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.CommentsCount, ps.GoldBadges, ps.SilverBadges, ps.BronzeBadges, ps.TotalBadges,
//        DENSE_RANK() OVER (ORDER BY ps.CommentsCount DESC, ps.TotalBadges DESC) AS PostRank FROM PostStats ps)
// SELECT uv.DisplayName, rp.Title, rp.CommentsCount, rp.GoldBadges, rp.SilverBadges, rp.BronzeBadges, uv.VoteBalance, CASE WHEN rp.PostRank <= 10 THEN 'Top Posts' ELSE 'Other Posts' END AS PostCategory
// FROM RankedPosts rp JOIN UserVotes uv ON uv.UserId = (SELECT DISTINCT OwnerUserId FROM Posts WHERE Id = rp.PostId)
// WHERE uv.VoteBalance > 0 AND rp.CommentsCount < (SELECT AVG(CommentsCount) FROM PostStats) AND rp.GoldBadges IS NOT NULL ORDER BY uv.VoteBalance DESC, rp.CommentsCount ASC LIMIT 20;
//
// The subquery looks the post up by its primary key, so uv is the post's owner. The AVG is compared exactly, as CommentsCount x rows < sum.
// VoteRank is never read; the LIMIT breaks ties by post id.
fn q23368(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, .. } = &db.post;
    let recent = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let ps = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(owner_user_id.select(&bidx).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + (b == Some(1)) as i64, a[2] + (b == Some(2)) as i64, a[3] + (b == Some(3)) as i64]);
    let tb = recent().group_by(Ident::<Post>::new()).select(owner_user_id.select(&bidx).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let (n, sum) = (&ps).fold_flat((0i64, 0i64), |(n, s), a| (n + 1, s + a[0]));
    let w = whole(&ps).select(Ident::<Post>::new().and(&ps).and(&tb)).window(dense_rank, |((_, a), t)| (Reverse(a[0]), Reverse(t)), asc);
    type R = (Id<Post>, [i64; 4], i64);
    let rv: MatSet<R> = (&w).map(|(((p, a), _), k)| (p, a, k)).collect();
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id)).fold(0i64, |b, t| b + (t == 2) as i64 - (t == 3) as i64);
    let v = drain((&rv).filt(move |(_, a, _): R| a[0] * n < sum).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(owner_user.select(Ident::<User>::new().and((&uv).filt(|b| b > 0)))))));
    let v = top_n(v, |&(_, ((p, a, _), (_, b)))| (Reverse(b), a[0], p), 20);
    rows(v.into_iter().map(|(_, ((p, a, k), (u, b)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title"]));
        f.extend(a.map(V::I));
        f.push(V::I(b));
        f.push(V::S(if k <= 10 { "Top Posts" } else { "Other Posts" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(DISTINCT P.Id) AS QuestionCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.AnswerCount, P.ViewCount, COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id), 0) AS CommentCount,
//        RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentRank, P.OwnerUserId FROM Posts P WHERE P.ClosedDate IS NULL),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges B GROUP BY B.UserId)
// SELECT UR.UserId, UR.DisplayName, UR.Reputation, UR.Upvotes, UR.Downvotes, COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges,
//        COALESCE(UB.BronzeBadges, 0) AS BronzeBadges, COUNT(DISTINCT PS.PostId) AS TotalQuestions, SUM(PS.Score) AS TotalScore, AVG(PS.ViewCount) AS AverageViewCount,
//        SUM(CASE WHEN PS.RecentRank <= 5 THEN 1 ELSE 0 END) AS RecentHighActivityCount
// FROM UserReputation UR LEFT JOIN UserBadges UB ON UR.UserId = UB.UserId LEFT JOIN PostStats PS ON UR.UserId = PS.OwnerUserId
// GROUP BY UR.UserId, UR.DisplayName, UR.Reputation, UR.Upvotes, UR.Downvotes, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges HAVING SUM(PS.Score) > 100
// ORDER BY UR.Reputation DESC, TotalScore DESC LIMIT 100;
//
// The order reads only Reputation and the PostStats score sum, so the hundred users are picked first and the questions x votes product is driven for those alone.
// The LIMIT breaks ties by user id.
fn q11(db: &'static So) -> String {
    let Post { closed_date, owner_user, creation_date, score, view_count, post_type_id, .. } = &db.post;
    let w = db
        .post
        .minus(closed_date)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date).and(score).and(view_count.opt()))
        .window(rank, |(((_, d), _), _)| Reverse(d), asc);
    let ps = (&w).fold([0i64; 5], |a, ((((_, _), s), w), k)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + (k <= 5) as i64]);
    let top = top_n(drain((&ps).filt(|a| a[1] > 100).and(&db.user.reputation)), |&(u, (a, r))| (Reverse(r), Reverse(a[1]), u), 100);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ur = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let ub = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let v = drain((&tu).select(Ident::<User>::new().and(&ps).and(&ur).and(&ub)));
    rows(v.into_iter().map(|(_, (((u, a), vt), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(vt[0]), V::I(vt[1])]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(a[4])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COUNT(DISTINCT C.Id) AS TotalComments, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// QuestionActivity AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, COALESCE(PH.Comment, 'No Close Reason') AS CloseReason, COUNT(V.Id) AS TotalVotes,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN PostHistory PH ON P.Id = PH.PostId AND PH.PostHistoryTypeId = 10 LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, U.DisplayName, PH.Comment),
// TopQuestions AS (SELECT Q.PostId, Q.Title, Q.CreationDate, Q.OwnerDisplayName, Q.CloseReason, Q.TotalVotes, Q.UpVotes, Q.DownVotes, UA.Reputation AS OwnerReputation, UA.ReputationRank
//     FROM QuestionActivity Q JOIN UserActivity UA ON Q.OwnerDisplayName = UA.DisplayName WHERE UA.TotalPosts > 5)
// SELECT TQ.Title, TQ.CreationDate, TQ.OwnerDisplayName, TQ.CloseReason, TQ.TotalVotes, TQ.UpVotes, TQ.DownVotes, TQ.OwnerReputation, TQ.ReputationRank FROM TopQuestions TQ
// WHERE TQ.TotalVotes > 10 AND TQ.CloseReason IS NOT NULL ORDER BY TQ.UpVotes DESC, TQ.OwnerReputation DESC LIMIT 10;
//
// QuestionActivity groups the question x close x vote rows by the close comment, so the joined rows are materialised first. The LIMIT breaks ties by post,
// comment and user id.
fn q175(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, (i64, i64))> = (&w).map(|((u, r), k)| (u, (r, k))).collect();
    let rank = by_first(&rk);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = db.user.with((&pc).filt(|n| n > 5)).select(&db.user.display_name).inv().collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).and(comment.opt()));
    type J = (Id<Post>, Option<(Id<PostHistory>, Option<Str>)>);
    let j: MatSet<J> = db.post.with(post_type_id.eq(1)).with(owner_user).select(Ident::<Post>::new().and(closes.opt())).collect();
    let qa = (&j)
        .group_by(Same::<J>::new().map(|x: J| (x.0, x.1.and_then(|h| h.1))))
        .select(Same::<J>::new().map(|x: J| x.0).select(votes_of(db).select(&db.vote.vote_type_id)))
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    type Q = ((Id<Post>, Option<Str>), [i64; 3]);
    let qv = rel(drain((&qa).filt(|a| a[0] > 10)));
    let v = drain((&qv).select(Same::<Q>::new().and(Same::<Q>::new().map(|x: Q| x.0 .0).select(owner_user.select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and(&rank))))));
    let v = top_n(v, |&(_, (((p, c), a), (u, (r, _))))| (Reverse(a[1]), Reverse(r), p, c, u), 10);
    rows(v.into_iter().map(|(_, (((p, c), a), (u, (_, k))))| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend([V::S(c.unwrap_or("No Close Reason")), V::I(a[0]), V::I(a[1]), V::I(a[2]), user_col(db, u, "rep"), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Score, COALESCE(votes.UpVotes, 0) AS UpVotes, COALESCE(votes.DownVotes, 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) votes
//     ON p.Id = votes.PostId WHERE p.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostHistoryRecent AS (SELECT ph.PostId, ph.CreationDate AS HistoryDate, ph.UserDisplayName, ph.Comment, ph.Text FROM PostHistory ph WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '7 days'),
// PostsWithComments AS (SELECT p.PostId, COUNT(c.Id) AS CommentCount FROM RankedPosts p LEFT JOIN Comments c ON p.PostId = c.PostId GROUP BY p.PostId),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Score, rp.UpVotes, rp.DownVotes, pr.CommentCount, RANK() OVER (ORDER BY rp.Score DESC, rp.UpVotes DESC, Pr.CommentCount DESC) AS ScoreRank
//     FROM RankedPosts rp JOIN PostsWithComments pr ON rp.PostId = pr.PostId WHERE rp.PostRank <= 5)
// SELECT fr.PostId, fr.Title, fr.Body, fr.Score AS TotalScore, fr.UpVotes, fr.DownVotes, fr.CommentCount, ph.HistoryDate, ph.UserDisplayName AS Editor, ph.Comment AS EditComment
// FROM FinalResults fr LEFT JOIN PostHistoryRecent ph ON fr.PostId = ph.PostId WHERE ph.HistoryDate IS NOT NULL OR (fr.UpVotes > 10 AND fr.CommentCount > 5) ORDER BY fr.ScoreRank, fr.UpVotes DESC, fr.CommentCount DESC;
//
// PostRank breaks CreationDate ties by post id (the SQL leaves them open). ScoreRank only orders the output.
fn q22963(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.lt(add_days(t0, -30))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { creation_date: hd, user_display_name, comment, .. } = &db.post_history;
    let phr = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_days(t0, -7))));
    type R = (([i64; 2], i64), Option<Id<PostHistory>>);
    let v = drain((&vc).and(&cc).and(phr.opt()).filt(|((a, c), h): R| h.is_some() || (a[0] > 10 && c > 5)));
    rows(v.into_iter().map(|(p, ((a, c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), ostr(user_display_name.get(h)), ostr(comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankByUser,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS DownVotesCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.RankByUser, rp.UpVotesCount, rp.DownVotesCount,
//        CASE WHEN rp.Score > 0 THEN 'Positive' WHEN rp.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreCategory FROM RankedPosts rp WHERE rp.RankByUser <= 5),
// PostComments AS (SELECT c.PostId, COUNT(*) AS CommentCount FROM Comments c GROUP BY c.PostId),
// FinalPostData AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, pc.CommentCount, tp.UpVotesCount, tp.DownVotesCount, tp.ScoreCategory, COALESCE(pc.CommentCount, 0) AS Comments
//     FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId)
// SELECT fpd.PostId, fpd.Title, fpd.CreationDate, fpd.Score, fpd.ViewCount, fpd.CommentCount, fpd.UpVotesCount, fpd.DownVotesCount, fpd.ScoreCategory,
//        CASE WHEN fpd.Comments IS NULL THEN 'No Comments' WHEN fpd.Comments > 0 THEN 'Has Comments' ELSE 'Zero Comments' END AS CommentStatus
// FROM FinalPostData fpd WHERE fpd.ScoreCategory <> 'Neutral' ORDER BY fpd.Score DESC, fpd.CreationDate DESC OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY;
//
// RankedPosts has no GROUP BY, so RankByUser numbers each owner's post x vote rows; it breaks CreationDate ties by post and vote id, and the LIMIT by post id
// (the SQL leaves both open). A post's rows agree in every projected column.
fn q21910(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, .. } = &db.post;
    type J = ((Id<Post>, i64), Option<Id<Vote>>);
    let j: MatSet<J> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(Ident::<Post>::new().and(creation_date).and(votes_of(db).opt())).collect();
    let w = (&j).group_by(Same::<J>::new().map(|x: J| x.0 .0).select(owner_user_id.opt())).select(Same::<J>::new()).window(row_number, |((p, d), v): J| (Reverse(d), p, v), asc);
    let tp: MatSet<J> = (&w).filt(|(_, k)| k <= 5).map(|(x, _)| x).collect();
    let posts: MatSet<Id<Post>> = (&tp).map(|x: J| x.0 .0).collect();
    let vc = (&posts).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = (&posts).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let pid = || Same::<J>::new().map(|x: J| x.0 .0);
    let v = drain((&tp).select(Same::<J>::new().and(pid().select(score.filt(|s| s != 0))).and(pid().select(&vc)).and(pid().select((&pc).opt()))));
    let v = top_n(v, |&(_, (((((p, d), vv), s), _), _))| (Reverse(s), Reverse(d), p, vv), 15);
    rows(v.into_iter().skip(5).map(|(_, (((((p, _), _), s), a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([oint(c), V::I(a[0]), V::I(a[1]), V::S(if s > 0 { "Positive" } else { "Negative" })]);
        f.push(V::S(if c.unwrap_or(0) > 0 { "Has Comments" } else { "Zero Comments" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN vt.Id = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN vt.Id = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// CombinedData AS (SELECT up.UserId, up.DisplayName, up.Reputation, up.GoldBadges, up.SilverBadges, up.BronzeBadges, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount,
//        rp.UpVoteCount, rp.DownVoteCount, rp.Rank FROM UserReputation up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId)
// SELECT UserId, DisplayName, Reputation, GoldBadges, SilverBadges, BronzeBadges, PostId, Title, CreationDate, Score, ViewCount, CommentCount, UpVoteCount, DownVoteCount FROM CombinedData
// WHERE Rank <= 5 ORDER BY Reputation DESC, Score DESC;
//
// Rank reads only base columns, so each owner's five posts are picked first and the comment x vote product is driven for those alone. It breaks
// (Score, CreationDate) ties by post id (the SQL leaves them open).
fn q5616(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let vt = (&db.vote.vote_type).select(&db.vote_type.origid);
    let rp = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(vt).opt())).fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let v = drain((&rp).and(&cc).and(owner_user.select(Ident::<User>::new().and(&ur))));
    rows(v.into_iter().map(|(p, ((a, c), (u, b)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL AND p.ViewCount > 0),
// TagCounts AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName HAVING COUNT(DISTINCT p.Id) >= 5),
// UserScores AS (SELECT u.Id AS UserId, SUM(p.Score) AS TotalScore, COUNT(DISTINCT p.Id) AS QuestionCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 year' GROUP BY u.Id HAVING COUNT(DISTINCT p.Id) >= 10),
// PostHistorySummary AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS CloseDate, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenDate,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS ChangeCount FROM PostHistory ph WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, tc.TagName, us.TotalScore, us.QuestionCount, ph.CloseDate, ph.ReopenDate, ph.ChangeCount
// FROM RankedPosts rp LEFT JOIN TagCounts tc ON tc.PostCount > 3 JOIN UserScores us ON us.QuestionCount >= 5 LEFT JOIN PostHistorySummary ph ON ph.PostId = rp.PostId
// WHERE rp.Rank <= 10 AND (ph.CloseDate IS NULL OR ph.ReopenDate IS NOT NULL) AND (ph.ChangeCount IS NOT NULL OR ph.ChangeCount > 0) ORDER BY rp.Score DESC, rp.CreationDate ASC;
//
// Both ON clauses name one side only, so the ranked posts are crossed with TagCounts and with UserScores. The WHERE reads only rp and ph, so it is applied
// before the crosses. Rank breaks (Score, CreationDate) ties by post id (the SQL leaves them open).
fn q23394(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db
        .post
        .with(creation_date.ge(add_years(t0, -1)).and(view_count.gt(0)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(hd.ge(add_months(t0, -1))).group_by(post).select(post_history_type_id.and(hd)).fold((i64::MIN, i64::MIN, 0i64), |(c, r, n), (t, d)| {
        (if t == 10 { c.max(d) } else { c }, if t == 11 { r.max(d) } else { r }, n + matches!(t, 10 | 11) as i64)
    });
    type R = (Id<Post>, (i64, i64, i64));
    let rp: MatSet<R> = (&tp).select(Ident::<Post>::new().and((&phs).filt(|(c, r, _)| c == i64::MIN || r != i64::MIN))).collect();
    let lt = like_tags(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let tcount = db.tag.group_by(&db.tag.tag_name).select((&by_tag).map(|(p, _): (Id<Post>, Id<Tag>)| p)).count_distinct();
    let tags: HashIdx<(), Str> = whole(&tcount).select(Same::<Str>::new().and(&tcount)).filt(|(_, n): (Str, i64)| n >= 5 && n > 3).map(|(s, _): (Str, i64)| s).collect();
    let us = db
        .user
        .with((&db.user.creation_date).lt(add_years(t0, -2)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score))
        .fold((0i64, 0i64), |(n, s), sc| (n + 1, s + sc));
    let users: HashIdx<(), (Id<User>, (i64, i64))> = whole(&us).select(Ident::<User>::new().and(&us)).filt(|(_, (n, _)): (Id<User>, (i64, i64))| n >= 10 && n >= 5).collect();
    let v = drain((&rp).select(Same::<R>::new().and(Same::<R>::new().map(|_| ()).select(&tags).opt()).and(Same::<R>::new().map(|_| ()).select(&users))));
    rows(v.into_iter().map(|(_, (((p, (c, r, n)), t), (_, (q, s))))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(t.map_or(V::Null, V::S));
        f.extend([V::I(s), V::I(q), tmax(c), tmax(r), V::I(n)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges,
//        COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostActivity AS (SELECT P.OwnerUserId, COUNT(*) AS PostCount, SUM(P.ViewCount) AS TotalViews, MAX(P.CreationDate) AS LastPostDate FROM Posts P
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(BA.PostCount, 0) AS PostCount, COALESCE(BG.GoldBadges, 0) AS GoldBadges, COALESCE(BS.SilverBadges, 0) AS SilverBadges,
//        COALESCE(BR.BronzeBadges, 0) AS BronzeBadges, CASE WHEN COALESCE(BA.PostCount, 0) = 0 THEN 0
//        ELSE (COALESCE(BG.GoldBadges, 0) + COALESCE(BS.SilverBadges, 0) + COALESCE(BR.BronzeBadges, 0)) / NULLIF(COALESCE(BA.PostCount, 1), 0) END AS BadgeRatio
//     FROM Users U LEFT JOIN PostActivity BA ON U.Id = BA.OwnerUserId LEFT JOIN UserBadgeCounts BG ON U.Id = BG.UserId LEFT JOIN UserBadgeCounts BS ON U.Id = BS.UserId
//     LEFT JOIN UserBadgeCounts BR ON U.Id = BR.UserId WHERE U.Reputation >= 1000),
// FilteredTopUsers AS (SELECT UserId, DisplayName, PostCount, GoldBadges, SilverBadges, BronzeBadges, BadgeRatio, RANK() OVER (ORDER BY BadgeRatio DESC) AS Rank FROM TopUsers)
// SELECT FTU.DisplayName, FTU.PostCount, FTU.GoldBadges, FTU.SilverBadges, FTU.BronzeBadges, FTU.BadgeRatio FROM FilteredTopUsers FTU
// WHERE FTU.Rank <= 10 AND (FTU.BadgeRatio IS NOT NULL AND FTU.BadgeRatio > 0) ORDER BY FTU.BadgeRatio DESC;
//
// UserBadgeCounts has one row per user, so the three self-joins each add the same counts. `/` on integers is a float division.
fn q24362(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let pa = db.post.with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let ratio = |b: [i64; 3], n: Option<i64>| match n.unwrap_or(0) {
        0 => 0.0,
        n => (b[0] + b[1] + b[2]) as f64 / n as f64,
    };
    let rich = || db.user.with((&db.user.reputation).ge(1000));
    let w = whole(rich()).select(Ident::<User>::new().and(&ub).and((&pa).opt())).window(rank, move |((_, b), n)| Reverse(fkey(ratio(b, n))), asc);
    let v = drain((&w).filt(move |(((_, b), n), k)| k <= 10 && ratio(b, n) > 0.0));
    rows(v.into_iter().map(|(_, (((u, b), n), _))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n.unwrap_or(0))];
        f.extend(b.map(V::I));
        f.push(V::F(ratio(b, n)));
        row(f)
    }))
}

// WITH RecursiveUserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY P.CreationDate DESC) AS ActivityRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE P.CreationDate IS NOT NULL),
// UserVotes AS (SELECT V.UserId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount, COUNT(DISTINCT V.PostId) AS TotalVotes
//     FROM Votes V GROUP BY V.UserId),
// UserBadgeSummary AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges B GROUP BY B.UserId),
// PostsStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId)
// SELECT U.DisplayName, U.Reputation, COALESCE(UA.UpVotesCount, 0) AS UpVotesCount, COALESCE(UA.DownVotesCount, 0) AS DownVotesCount, COALESCE(UB.GoldBadges, 0) AS GoldBadges,
//        COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.TotalQuestions, 0) AS TotalQuestions,
//        COALESCE(PS.TotalAnswers, 0) AS TotalAnswers, COALESCE(PS.AverageScore, 0.0) AS AverageScore, COALESCE(RUA.ActivityRank, 0) AS LatestActivityRank
// FROM Users U LEFT JOIN UserVotes UA ON U.Id = UA.UserId LEFT JOIN UserBadgeSummary UB ON U.Id = UB.UserId LEFT JOIN PostsStatistics PS ON U.Id = PS.OwnerUserId
// LEFT JOIN RecursiveUserActivity RUA ON U.Id = RUA.UserId WHERE U.Reputation >= 1000 ORDER BY U.Reputation DESC, LatestActivityRank LIMIT 100;
//
// WITH RECURSIVE, but no CTE refers to itself. RecursiveUserActivity has one row per post of the user, numbered by date; the rows of one user differ only in
// that number, so its CreationDate ties (broken by post id here) cannot be observed. The LIMIT breaks ties by user id.
fn q31480(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, score, .. } = &db.post;
    let w = db.post.group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rua: HashIdx<Id<User>, i64> = (&w).map(|(_, k)| k).collect();
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let v = drain(db.user.with((&db.user.reputation).ge(1000)).select((&db.user.reputation).and((&uv).opt()).and((&ub).opt()).and((&ps).opt()).and((&rua).opt())));
    let v = top_n(v, |&(u, ((((r, _), _), _), k))| (Reverse(r), k.unwrap_or(0), u), 100);
    rows(v.into_iter().map(|(u, ((((_, vt), b), p), k))| {
        let vt = vt.unwrap_or([0, 0]);
        let b = b.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(vt[0]), V::I(vt[1])]);
        f.extend(b.map(V::I));
        f.extend(match p {
            Some(p) => [V::I(p[0]), V::I(p[1]), V::I(p[2]), avg(p[3], p[0])],
            None => [V::I(0), V::I(0), V::I(0), V::F(0.0)],
        });
        f.push(V::I(k.unwrap_or(0)));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("30203", q30203),
    ("3491", q3491),
    ("371", q371),
    ("1317", q1317),
    ("21365", q21365),
    ("20866", q20866),
    ("24867", q24867),
    ("1240", q1240),
    ("32303", q32303),
    ("34803", q34803),
    ("21879", q21879),
    ("21447", q21447),
    ("21992", q21992),
    ("25711", q25711),
    ("31705", q31705),
    ("695", q695),
    ("24190", q24190),
    ("23804", q23804),
    ("90", q90),
    ("8479", q8479),
    ("4983", q4983),
    ("30377", q30377),
    ("2359", q2359),
    ("4180", q4180),
    ("20904", q20904),
    ("1535", q1535),
    ("2942", q2942),
    ("32025", q32025),
    ("57", q57),
    ("33291", q33291),
    ("23435", q23435),
    ("21696", q21696),
    ("34713", q34713),
    ("32256", q32256),
    ("24531", q24531),
    ("23537", q23537),
    ("7991", q7991),
    ("21916", q21916),
    ("21189", q21189),
    ("34030", q34030),
    ("21857", q21857),
    ("3132", q3132),
    ("21716", q21716),
    ("3818", q3818),
    ("23600", q23600),
    ("2749", q2749),
    ("21310", q21310),
    ("31631", q31631),
    ("32675", q32675),
    ("2288", q2288),
    ("2337", q2337),
    ("1084", q1084),
    ("22397", q22397),
    ("4517", q4517),
    ("24781", q24781),
    ("31209", q31209),
    ("4001", q4001),
    ("23201", q23201),
    ("22885", q22885),
    ("30424", q30424),
    ("24446", q24446),
    ("6159", q6159),
    ("20737", q20737),
    ("32708", q32708),
    ("1657", q1657),
    ("30807", q30807),
    ("30857", q30857),
    ("1125", q1125),
    ("32415", q32415),
    ("33225", q33225),
    ("305", q305),
    ("32785", q32785),
    ("4516", q4516),
    ("21305", q21305),
    ("21187", q21187),
    ("24601", q24601),
    ("22642", q22642),
    ("515", q515),
    ("20609", q20609),
    ("1854", q1854),
    ("24500", q24500),
    ("8674", q8674),
    ("23012", q23012),
    ("21268", q21268),
    ("23368", q23368),
    ("11", q11),
    ("175", q175),
    ("22963", q22963),
    ("21910", q21910),
    ("5616", q5616),
    ("23394", q23394),
    ("24362", q24362),
    ("31480", q31480),
];
