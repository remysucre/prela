use harness::prelude::*;
use std::cmp::Reverse;

fn by_first<A: Copy + Eq + std::hash::Hash, B: Copy + Eq + std::hash::Hash>(m: &MatSet<(A, B)>) -> HashIdx<A, B> {
    m.map(|(a, _)| a).inv().map(|(_, b): (A, B)| b).collect()
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, p.Score, p.ViewCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score
// DESC) AS RankScore, COUNT(c.Id) AS CommentCount, AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteRatio FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT
// JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, p.Score,
// p.ViewCount), CTE_BadgedUsers AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id), PopularPosts AS (SELECT
// rp.*, u.DisplayName, ub.BadgeCount FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN CTE_BadgedUsers ub ON u.Id = ub.UserId WHERE rp.RankScore <= 10) SELECT
// pp.PostId, pp.Title, pp.CreationDate, pp.DisplayName, pp.Score, pp.ViewCount, COALESCE(pp.BadgeCount, 0) AS BadgeCount, CASE WHEN pp.UpVoteRatio IS NULL THEN 'No Votes' WHEN
// pp.UpVoteRatio > 0.5 THEN 'Predominantly Upvoted' ELSE 'Mixed Votes' END AS VoteStatus FROM PopularPosts pp WHERE pp.PostTypeId = 1 ORDER BY pp.Score DESC, pp.CreationDate
// DESC;
//
// RankScore reads only Score, so the top ten of each post type are picked first and the comment x vote product is driven for those alone.
fn q30222(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(current_date(), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + 1, a[1] + (t == Some(2)) as i64]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&s).and(owner_user.select(&bc)));
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        f.extend([V::I(b), V::S(if 2 * a[1] > a[0] { "Predominantly Upvoted" } else { "Mixed Votes" })]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(pt.Name, 'Unknown') AS PostType, COALESCE(u.DisplayName, 'Deleted User') AS
// OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 3) AS
// DownVotes FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON
// p.Id = v.PostId WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, pt.Name,
// u.DisplayName), TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, PostType, OwnerDisplayName, CommentCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY
// Score DESC, ViewCount DESC) AS Rank FROM PostStats) SELECT tp.Title, tp.CreationDate, tp.PostType, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes, CASE WHEN
// tp.Rank <= 10 THEN 'Top Ranked' WHEN tp.Rank BETWEEN 11 AND 20 THEN 'Next Best' ELSE 'Below 20' END AS RankCategory FROM TopPosts tp WHERE tp.CommentCount > 0 ORDER BY
// tp.Rank;
fn q6936(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let cc = base().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let voters = |t: i64| votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(t))).select(&db.vote.user_id);
    let up = base().group_by(Ident::<Post>::new()).select(voters(2)).count_distinct();
    let dn = base().group_by(Ident::<Post>::new()).select(voters(3)).count_distinct();
    let w = whole(base())
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w): ((Id<Post>, i64), Option<i64>)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).map(|(((p, _), _), r)| (p, r)).collect();
    let v = drain(by_first(&rk).and((&cc).filt(|c| c > 0)).and((&up).opt()).and((&dn).opt()));
    rows(v.into_iter().map(|(p, (((r, c), u), d))| {
        let mut f = post_fields(db, p, &["title", "created", "type"]);
        f.push(V::S(owner_user.get(p).map_or("Deleted User", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(c), V::I(u.unwrap_or(0)), V::I(d.unwrap_or(0))]);
        f.push(V::S(if r <= 10 { "Top Ranked" } else if r <= 20 { "Next Best" } else { "Below 20" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, COUNT(DISTINCT
// c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN
// Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.Id, p.Title,
// p.CreationDate, p.Score, p.OwnerUserId), PostSummary AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes, CASE WHEN rp.Score >
// 100 THEN 'Highly Rated' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Moderately Rated' ELSE 'Low Rated' END AS RatingGroup, CASE WHEN rp.CommentCount > 10 THEN 'Active Discussion'
// ELSE 'Minimal Discussion' END AS DiscussionStatus FROM RankedPosts rp WHERE rp.rn = 1) SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.CommentCount, ps.UpVotes,
// ps.DownVotes, ps.RatingGroup, ps.DiscussionStatus FROM PostSummary ps WHERE ps.Score IS NOT NULL AND ps.CommentCount IS NOT NULL ORDER BY ps.Score DESC, ps.CommentCount DESC
// LIMIT 10;
//
// rn reads only base columns, so each owner's newest post is picked first (the ownerless posts are one partition) and the product is driven for those alone.
fn q2011(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&s).and(&cc)), |&(p, (_, c))| (Reverse(score.get(p).unwrap()), Reverse(c), p), 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if s > 100 { "Highly Rated" } else if s >= 50 { "Moderately Rated" } else { "Low Rated" }));
        f.push(V::S(if c > 10 { "Active Discussion" } else { "Minimal Discussion" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u), PostStats AS
// (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN
// 1 ELSE 0 END), 0) AS TotalQuestions FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.OwnerUserId), CombinedStats AS (SELECT
// u.DisplayName, ur.Reputation, ps.TotalPosts, ps.TotalAnswers, ps.TotalQuestions, COALESCE((SELECT SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) FROM Votes v WHERE v.PostId
// IN (SELECT Id FROM Posts WHERE OwnerUserId = u.Id)), 0) AS TotalUpvotes FROM Users u JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
// WHERE ur.Reputation > 1000) SELECT cs.DisplayName, cs.Reputation, cs.TotalPosts, cs.TotalAnswers, cs.TotalQuestions, cs.TotalUpvotes, CASE WHEN cs.TotalPosts IS NULL THEN 'No
// Posts Yet' WHEN cs.TotalQuestions > 0 THEN 'Active Questioner' WHEN cs.TotalAnswers > 0 THEN 'Active Responder' ELSE 'Inactive' END AS ActivityStatus FROM CombinedStats cs
// WHERE cs.TotalPosts > (SELECT AVG(TotalPosts) FROM PostStats) ORDER BY cs.Reputation DESC LIMIT 10;
//
// `TotalPosts > AVG(TotalPosts)` is compared exactly, as `TotalPosts * groups > total`; the ownerless posts are one PostStats group.
fn q1435(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, post_type_id, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(owner_user_id.opt())
        .select(post_type_id)
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64]);
    let (sum, n) = (&ps).fold_flat((0i64, 0i64), |(s, n), a: [i64; 3]| (s + a[0], n + 1));
    let upv = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id)).opt())
        .fold(0i64, |k, t| k + (t == Some(2)) as i64);
    let v = drain((&upv).and((&db.user.origid).map(|u| Some(u)).select((&ps).filt(move |a: [i64; 3]| a[0] * n > sum))));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, (k, a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(k)]);
        f.push(V::S(if a[2] > 0 { "Active Questioner" } else if a[1] > 0 { "Active Responder" } else { "Inactive" }));
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS NumberOfPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE
// WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(p.ViewCount) AS TotalViews, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId =
// 3 THEN 1 ELSE 0 END) AS DownVotes, AVG(COALESCE(p.Score, 0)) AS AverageScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE
// u.Reputation > 1000 AND u.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName), RankedEngagement AS (SELECT UserId, DisplayName,
// NumberOfPosts, Questions, Answers, TotalViews, UpVotes, DownVotes, AverageScore, RANK() OVER (ORDER BY TotalViews DESC, UpVotes - DownVotes DESC) AS EngagementRank FROM
// UserEngagement) SELECT re.UserId, re.DisplayName, re.NumberOfPosts, re.Questions, re.Answers, re.TotalViews, re.UpVotes, re.DownVotes, re.AverageScore, CASE WHEN
// re.EngagementRank <= 10 THEN 'Top Contributor' WHEN re.EngagementRank <= 50 THEN 'Active Contributor' ELSE 'Regular User' END AS EngagementLevel FROM RankedEngagement re WHERE
// re.EngagementRank <= 100 ORDER BY re.EngagementRank;
fn q6535(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000)).with((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 8], |a, x| match x {
            Some((((t, w), s), v)) => [
                a[0] + 1,
                a[1] + (t == 1) as i64,
                a[2] + (t == 2) as i64,
                a[3] + w.is_some() as i64,
                a[4] + w.unwrap_or(0),
                a[5] + (v == Some(2)) as i64,
                a[6] + (v == Some(3)) as i64,
                a[7] + s,
            ],
            None => [a[0] + 1, a[1], a[2], a[3], a[4], a[5], a[6], a[7]],
        });
    let np = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&s).select(Ident::<User>::new().and((&s).and(&np))).window(rank, |(_, (a, _)): (Id<User>, ([i64; 8], i64))| (a[3] == 0, Reverse(a[4]), Reverse(a[5] - a[6])), asc);
    let v = drain((&w).filt(|(_, r)| r <= 100));
    rows(v.into_iter().map(|(_, ((u, (a, n)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), V::I(a[5]), V::I(a[6]), avg(a[7], a[0])]);
        f.push(V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Active Contributor" } else { "Regular User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS
// Rank FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1 AND p.Score > 0), RecentActivities AS (SELECT Ph.PostId, Ph.UserId, Ph.CreationDate, P.Title AS
// PostTitle, P.Score, ROW_NUMBER() OVER (PARTITION BY Ph.PostId ORDER BY Ph.CreationDate DESC) AS ActivityRank FROM PostHistory Ph JOIN Posts P ON Ph.PostId = P.Id WHERE
// Ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 DAYS'), CombinedResults AS (SELECT rp.Title AS PostTitle, rp.OwnerDisplayName, ra.UserId,
// ra.CreationDate as LastActivityDate, COALESCE(ra.Score, 0) AS ActivityScore FROM RankedPosts rp FULL OUTER JOIN RecentActivities ra ON rp.Id = ra.PostId WHERE (rp.Rank = 1 OR
// ra.ActivityRank = 1)) SELECT PostTitle, OwnerDisplayName, LastActivityDate, ActivityScore, CASE WHEN ActivityScore > 0 THEN 'Active' WHEN LastActivityDate IS NULL THEN 'No
// Activity' ELSE 'Inactive' END AS PostStatus FROM CombinedResults WHERE OwnerDisplayName IS NOT NULL AND (ActivityScore > 0 OR LastActivityDate IS NOT NULL) ORDER BY
// ActivityScore DESC, LastActivityDate DESC LIMIT 50;
//
// The FULL OUTER JOIN keeps only rows with an OwnerDisplayName, which only a RankedPosts row has, so it is RankedPosts LEFT JOIN RecentActivities under the same WHERE.
fn q496(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .with(owner_user)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s): (Id<Post>, i64)| (Reverse(s), p), asc);
    let rp: MatSet<(Id<Post>, (i64, i64))> = (&w).map(|((p, s), r)| (p, (s, r))).collect();
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let ra = db
        .post_history
        .with(hd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(post)
        .select(Ident::<PostHistory>::new().and(hd))
        .window(row_number, |(h, d): (Id<PostHistory>, i64)| (Reverse(d), h), asc);
    type A = Option<((Id<PostHistory>, i64), i64)>;
    let act = |s: i64, a: A| a.map_or(0, |_| s);
    let j = by_first(&rp)
        .and((&ra).opt())
        .filt(|((_, r), a): ((i64, i64), A)| r == 1 || a.map_or(false, |x| x.1 == 1))
        .filt(move |((s, _), a): ((i64, i64), A)| act(s, a) > 0 || a.is_some());
    let v = drain(j);
    let v = top_n(v, |&(p, ((s, _), a))| {
        let d = a.map(|x| (x.0).1);
        (Reverse(act(s, a)), d.is_none(), Reverse(d), p, a.map(|x| (x.0).0))
    }, 50);
    rows(v.into_iter().map(|(p, ((s, _), a))| {
        let s = act(s, a);
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend([ots(a.map(|x| (x.0).1)), V::I(s)]);
        f.push(V::S(if s > 0 { "Active" } else if a.is_none() { "No Activity" } else { "Inactive" }));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id LEFT JOIN
// Comments C ON P.Id = C.PostId GROUP BY U.Id), PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
// COALESCE(PS.UpVotes, 0) AS TotalUpVotes, COALESCE(PS.DownVotes, 0) AS TotalDownVotes, COALESCE(PS.PostCount, 0) AS TotalPosts, COALESCE(PS.CommentCount, 0) AS TotalComments
// FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN UserVoteStats PS ON U.Id = PS.UserId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1
// year' AND P.Score > 0 ORDER BY P.Score DESC, P.ViewCount DESC), TopPosts AS (SELECT PostId, Title, OwnerDisplayName, Score, ViewCount, ROW_NUMBER() OVER (ORDER BY Score DESC,
// ViewCount DESC) AS Rank FROM PostStats WHERE TotalPosts > 5) SELECT T.PostId, T.Title, T.OwnerDisplayName, T.Score, T.ViewCount FROM TopPosts T WHERE Rank <= 10 ORDER BY
// T.Score DESC;
//
// UserVoteStats is read only for `TotalPosts > 5`, its distinct voted-on post count; the vote sums and the comment count are never read, so they are not computed.
fn q9816(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let voted = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.post)).count_distinct();
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user.select((&voted).filt(|n| n > 5))));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, _)| row(post_fields(db, p, &["id", "title", "owner", "score", "views"]))))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
// p.OwnerUserId FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 10), UserStatistics AS (SELECT u.Id AS UserId,
// u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN
// b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN
// Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName) SELECT us.UserId, us.DisplayName, us.TotalPosts,
// us.TotalBounty, us.GoldBadges, us.SilverBadges, us.BronzeBadges, COUNT(rp.Id) AS RecentPostsCount, AVG(rp.Score) AS AverageRecentPostScore FROM UserStatistics us LEFT JOIN
// RankedPosts rp ON us.UserId = rp.OwnerUserId GROUP BY us.UserId, us.DisplayName, us.TotalPosts, us.TotalBounty, us.GoldBadges, us.SilverBadges, us.BronzeBadges HAVING
// COUNT(rp.Id) > 5 AND AVG(COALESCE(rp.Score, 0)) >= 20 ORDER BY us.TotalBounty DESC, AverageRecentPostScore DESC;
//
// The HAVING reads only the RankedPosts side, so the users that pass it are found first and the posts x badges x votes product is driven for them alone.
fn q3097(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(10)))
        .group_by(owner_user)
        .select(score)
        .fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let cand: MatSet<Id<User>> = db.user.with((&rp).filt(|a: [i64; 2]| a[0] > 5 && a[1] >= 20 * a[0])).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(bounty.opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (p, c)| {
            let b = p.flatten().flatten().unwrap_or(0);
            [a[0] + b, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]
        });
    let np = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&us).and(&np).and(&rp));
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(r[0]), avg(r[1], r[0])]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId
// = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount, DENSE_RANK() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) DESC)
// AS Rank FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL
// '1 year' GROUP BY p.Id, p.Title, p.ViewCount), TopPosts AS (SELECT ps.PostId, ps.Title, ps.ViewCount, ps.UpVotes, ps.DownVotes, ps.CommentCount, ps.Rank, CASE WHEN ps.Rank <=
// 10 THEN 'Top Ranked' ELSE 'Other' END AS PostCategory FROM PostStats ps) SELECT tp.PostId, tp.Title, tp.ViewCount, tp.UpVotes, tp.CommentCount, tp.PostCategory, u.DisplayName
// AS OwnerDisplayName, COALESCE(b.Name, 'No Badge') AS BadgeName, CASE WHEN EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = tp.PostId AND v.VoteTypeId = 6) THEN 'Closed' ELSE
// 'Open' END AS PostStatus FROM TopPosts tp LEFT JOIN Users u ON tp.PostId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId AND b.Class = 1 WHERE (tp.Rank <= 10 OR tp.CommentCount >
// 5) ORDER BY tp.Rank, tp.ViewCount DESC LIMIT 50;
//
// `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q3353(db: &'static So) -> String {
    let Post { creation_date, view_count, origid, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let s = base()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold(0i64, |n, (t, _)| n + (t == Some(2)) as i64);
    let cc = base().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(&s).select(Ident::<Post>::new().and((&s).and(&cc))).window(dense_rank, |(_, (u, _)): (Id<Post>, (i64, i64))| Reverse(u), asc);
    let tp: MatSet<(Id<Post>, ((i64, i64), i64))> = (&w).filt(|((_, (_, c)), r)| r <= 10 || c > 5).map(|((p, x), r)| (p, (x, r))).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let closed: MatSet<Id<Post>> = db.vote.with((&db.vote.vote_type_id).eq(6)).select(&db.vote.post).collect();
    let v = drain(by_first(&tp).and(origid.select(&uidx).select(Ident::<User>::new().and(gold.opt())).opt()).and(Ident::<Post>::new().with(&closed).opt()));
    let v = top_n(v, |&(p, (((_, r), u), _))| {
        let w = view_count.get(p);
        (r, w.is_none(), Reverse(w), p, u.map(|x| x.1))
    }, 50);
    rows(v.into_iter().map(|(p, ((((up, c), r), u), cl))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(up), V::I(c), V::S(if r <= 10 { "Top Ranked" } else { "Other" })]);
        f.push(match u {
            Some((u, _)) => user_col(db, u, "name"),
            None => V::Null,
        });
        f.push(V::S(u.and_then(|x| x.1).map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        f.push(V::S(if cl.is_some() { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0
// END) AS DownVotesCount, COUNT(DISTINCT P.Id) AS PostsCount, SUM(P.Score) AS TotalScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id =
// V.PostId GROUP BY U.Id, U.DisplayName), TopUsers AS (SELECT UserId, DisplayName, UpVotesCount, DownVotesCount, PostsCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC)
// AS ScoreRank FROM UserReputation), PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COUNT(C) AS CommentCount, ROW_NUMBER() OVER (ORDER BY
// P.Score DESC, P.ViewCount DESC) AS PostRank FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY P.Id,
// P.Title, P.CreationDate, P.Score, P.ViewCount) SELECT U.UserId, U.DisplayName, U.UpVotesCount, U.DownVotesCount, U.PostsCount, U.TotalScore, P.PostId, P.Title AS
// PopularPostTitle, P.Score AS PopularPostScore, P.ViewCount AS PopularPostViewCount, P.CommentCount AS PopularPostCommentCount FROM TopUsers U FULL OUTER JOIN PopularPosts P ON
// U.ScoreRank = 1 AND P.PostRank <= 5 WHERE U.PostsCount > 10 OR P.CommentCount IS NOT NULL ORDER BY U.TotalScore DESC, P.Score DESC;
//
// `FULL OUTER JOIN ... ON U.ScoreRank = 1 AND P.PostRank <= 5` crosses the rank-1 users with the top five posts; every other user and post is an unmatched row.
// COUNT(C) counts the joined rows, the NULL one included.
fn q3636(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((s, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + 1, a[3] + s],
            None => a,
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tw = whole(&ur).select(Ident::<User>::new().and(&ur)).window(rank, |(_, a): (Id<User>, [i64; 4])| (a[2] == 0, Reverse(a[3])), asc);
    let tr: MatSet<(Id<User>, i64)> = (&tw).map(|((u, _), r)| (u, r)).collect();
    let rk = by_first(&tr);
    let since = ny_to_utc(add_days(utc_to_ny(now_utc()), -30));
    let recent = || db.post.with(creation_date.filt(move |d: i64| ny_to_utc(d) >= since));
    let pc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, _| n + 1);
    let pw = whole(recent())
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w): ((Id<Post>, i64), Option<i64>)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let p5: HashIdx<(), (Id<Post>, i64)> = (&pw).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).select(Ident::<Post>::new().and(&pc)).collect();
    let u1: HashIdx<(), Id<User>> = (&tw).filt(|(_, r)| r == 1).map(|((u, _), _)| u).collect();
    let p5set: MatSet<Id<Post>> = (&p5).map(|(p, _)| p).collect();
    type Row = (Option<(Id<User>, [i64; 4], i64)>, Option<(Id<Post>, i64)>);
    let users = db
        .user
        .select(Ident::<User>::new().and((&ur).and(&np)).and((&rk).filt(|r| r == 1).map(|_| ()).select(&p5).opt()))
        .filt(|((_, (_, n)), p): ((Id<User>, ([i64; 4], i64)), Option<(Id<Post>, i64)>)| n > 10 || p.is_some())
        .map(|((u, (a, n)), p)| -> Row { (Some((u, a, n)), p) });
    let posts = recent().select(Ident::<Post>::new().and(&pc)).minus(Ident::<Post>::new().with(&p5set).map(|_| ()).select(&u1)).map(|(p, c)| -> Row { (None, Some((p, c))) });
    let v = drain(users.inv().map(|_| ()).union(posts.inv().map(|_| ())));
    rows(v.into_iter().map(|((u, p), _)| {
        let mut f = match u {
            Some((u, a, n)) => {
                let mut f = ucols(db, u, &["uid", "name"]);
                f.extend([V::I(a[0]), V::I(a[1]), V::I(n), nullable(a[3], a[2])]);
                f
            }
            None => (0..6).map(|_| V::Null).collect(),
        };
        f.extend(match p {
            Some((p, c)) => {
                let mut g = post_fields(db, p, &["id", "title", "score", "views"]);
                g.push(V::I(c));
                g
            }
            None => (0..5).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

// WITH UserVotes AS (SELECT v.UserId, v.VoteTypeId, COUNT(v.Id) AS VoteCount FROM Votes v GROUP BY v.UserId, v.VoteTypeId), TopUsers AS (SELECT u.Id AS UserId, u.DisplayName,
// u.Reputation, COALESCE(uv.VoteCount, 0) AS TotalVotes FROM Users u LEFT JOIN UserVotes uv ON u.Id = uv.UserId WHERE u.Reputation >= 1000), PostMetrics AS (SELECT p.Id AS
// PostId, p.Title, p.OwnerUserId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 END), 0) AS DownVotes,
// COALESCE(COUNT(c.Id), 0) AS CommentCount, COUNT(DISTINCT pl.RelatedPostId) AS RelatedLinksCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id
// = c.PostId LEFT JOIN PostLinks pl ON p.Id = pl.PostId WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title,
// p.OwnerUserId), RankedPosts AS (SELECT pm.PostId, pm.Title, pm.OwnerUserId, pm.UpVotes, pm.DownVotes, pm.CommentCount, pm.RelatedLinksCount, RANK() OVER (ORDER BY pm.UpVotes
// DESC, pm.CommentCount DESC) AS PostRank FROM PostMetrics pm) SELECT rp.Title, u.DisplayName AS OwnerDisplayName, rp.UpVotes, rp.DownVotes, rp.CommentCount,
// rp.RelatedLinksCount, rp.PostRank FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.PostRank <= 10 ORDER BY rp.PostRank;
//
// TopUsers is never referenced by the final SELECT, so it is not computed.
fn q6195(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pm = base()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(links_of(db).opt()))
        .fold([0i64; 3], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let rl = base().group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let w = whole(&pm).select(Ident::<Post>::new().and(&pm)).window(rank, |(_, a): (Id<Post>, [i64; 3])| (Reverse(a[0]), Reverse(a[2])), asc);
    let rp: MatSet<(Id<Post>, ([i64; 3], i64))> = (&w).filt(|(_, r)| r <= 10).map(|((p, a), r)| (p, (a, r))).collect();
    let v = drain(by_first(&rp).and(owner_user).and((&rl).opt()));
    rows(v.into_iter().map(|(p, (((a, r), u), l))| {
        let mut f = post_fields(db, p, &["title"]);
        f.push(user_col(db, u, "name"));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(l.unwrap_or(0)), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.OwnerUserId, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank
// FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.OwnerUserId), PostHistorySummary AS (SELECT ph.PostId,
// COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS ClosureCount, MAX(ph.CreationDate) AS LastModifiedDate FROM PostHistory ph GROUP BY ph.PostId), TopUsers AS
// (SELECT u.Id, u.DisplayName, u.Reputation, SUM(v.BountyAmount) AS TotalBounty FROM Users u JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation HAVING
// SUM(v.BountyAmount) > 0) SELECT rp.Title, rp.Score, rp.CommentCount, u.DisplayName AS PostOwner, u.Reputation AS OwnerReputation, COALESCE(phs.ClosureCount, 0) AS
// PostClosureCount, phs.LastModifiedDate, tu.TotalBounty FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN PostHistorySummary phs ON rp.Id = phs.PostId LEFT
// JOIN TopUsers tu ON u.Id = tu.Id WHERE rp.UserPostRank = 1 AND rp.Score > 10 AND (phs.LastModifiedDate IS NULL OR phs.LastModifiedDate >= cast('2024-10-01 12:34:56' as
// timestamp) - INTERVAL '30 days') ORDER BY rp.Score DESC, u.Reputation DESC;
fn q4816(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold((0i64, i64::MIN), |(n, m), (t, d)| (n + matches!(t, 10 | 11) as i64, m.max(d)));
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let tb = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.bounty_amount)).fold(0i64, |s, b| s + b);
    let cc = (&tp).with(score.gt(10)).with(owner_user).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and((&phs).opt().filt(move |h: Option<(i64, i64)>| h.map_or(true, |(_, m)| m >= cut))).and(owner_user.select(Ident::<User>::new().and((&tb).filt(|s| s > 0).opt()))));
    rows(v.into_iter().map(|(p, ((c, h), (u, b)))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.push(V::I(c));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(h.map_or(0, |x| x.0)), ots(h.map(|x| x.1)), oint(b)]);
        row(f)
    }))
}

// WITH UserTags AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT t.TagName) AS UniqueTagsContributed, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS
// AcceptedAnswers, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT DISTINCT
// UNNEST(string_to_array(substring(Tags, 2, length(Tags) - 2), '><')) AS TagName, p.Id AS PostId FROM Posts p WHERE p.PostTypeId = 1) t ON p.Id = t.PostId GROUP BY u.Id,
// u.DisplayName), UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN
// b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId), CombinedStats AS (SELECT ut.UserId, ut.DisplayName, ut.UniqueTagsContributed, ub.GoldBadges,
// ub.SilverBadges, ub.BronzeBadges, ut.AcceptedAnswers, ut.TotalViews, ut.TotalScore FROM UserTags ut LEFT JOIN UserBadges ub ON ut.UserId = ub.UserId) SELECT *,
// COALESCE(TotalViews / NULLIF(AcceptedAnswers, 0), 0) AS ViewsPerAcceptedAnswer, COALESCE(TotalScore / NULLIF(UniqueTagsContributed, 0), 0) AS ScorePerTag FROM CombinedStats
// ORDER BY TotalScore DESC, UniqueTagsContributed DESC LIMIT 10;
fn q28128(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, view_count, accepted_answer_id, .. } = &db.post;
    let pt: MatSet<(Id<Post>, Str)> = db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list))).collect();
    let tags: HashIdx<Id<Post>, (Id<Post>, Str)> = (&pt).map(|(p, _)| p).inv().select(&pt).collect();
    let tag_of = (&tags).map(|(_, t)| t);
    let ut = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(accepted_answer_id.opt()).and(tag_of.opt())))
        .fold([0i64; 4], |a, (((s, w), x), _)| [a[0] + x.is_some() as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let nt = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&tags).map(|(_, t)| t))).count_distinct();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&ut).and((&nt).opt()).and((&ub).opt()));
    let v = top_n(v, |&(u, ((a, n), _))| (Reverse(a[3]), Reverse(n.unwrap_or(0)), u), 10);
    rows(v.into_iter().map(|(u, ((a, n), b))| {
        let n = n.unwrap_or(0);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::I(a[3])]);
        f.push(V::F(if a[0] == 0 || a[1] == 0 { 0.0 } else { a[2] as f64 / a[0] as f64 }));
        f.push(V::F(if n == 0 { 0.0 } else { a[3] as f64 / n as f64 }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank,
// COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY
// p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score), UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(rp.PostRank) AS TotalPostRank FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id,
// u.DisplayName, u.Reputation HAVING COUNT(DISTINCT b.Id) > 5) SELECT us.UserId, us.DisplayName, us.Reputation, us.BadgeCount, COALESCE(MAX(rp.Score), 0) AS MaxPostScore,
// COALESCE(SUM(rp.CommentCount), 0) AS TotalComments, COALESCE(AVG(rp.UpvoteCount), 0) AS AvgUpvotes, COALESCE(AVG(rp.DownvoteCount), 0) AS AvgDownvotes FROM UserStatistics us
// LEFT JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId GROUP BY us.UserId, us.DisplayName, us.Reputation, us.BadgeCount ORDER BY us.Reputation DESC, MaxPostScore DESC LIMIT
// 10;
//
// PostRank feeds only TotalPostRank, which is never read, so it is not computed.
fn q1290(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let bc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .with((&bc).filt(|n| n > 5))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(&rp)).opt())
        .fold([i64::MIN, 0, 0, 0, 0], |a, p| match p {
            Some((s, r)) => [a[0].max(s), a[1] + r[0], a[2] + r[1], a[3] + r[2], a[4] + 1],
            None => a,
        });
    let v = top_n(drain((&s).and(&bc)), |&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(if a[4] == 0 { 0 } else { a[0] }), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(if a[4] == 0 { 0 } else { a[0] }), V::I(a[1])]);
        f.extend(if a[4] == 0 { [V::F(0.0), V::F(0.0)] } else { [avg(a[2], a[4]), avg(a[3], a[4])] });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC)
// AS ScoreRank, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UpvoterCount FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id =
// c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' AND p.Score IS NOT NULL AND
// p.ViewCount > 0 GROUP BY p.Id, pt.Name, p.Title, p.CreationDate, p.LastActivityDate, p.Score, p.ViewCount), FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Score,
// rp.ViewCount, rp.CommentCount, rp.UpvoterCount, CASE WHEN rp.ScoreRank <= 5 THEN 'Top5' WHEN rp.ScoreRank BETWEEN 6 AND 10 THEN 'Top10' ELSE 'Others' END AS ScoreCategory FROM
// RankedPosts rp WHERE rp.CommentCount > 5) SELECT fp.ScoreCategory, COUNT(fp.PostId) AS TotalPosts, AVG(fp.Score) AS AvgScore, SUM(fp.ViewCount) AS TotalViews,
// SUM(fp.UpvoterCount) AS UniqueUpvoters, MAX(fp.Title) AS MostPopularTitle FROM FilteredPosts fp GROUP BY fp.ScoreCategory ORDER BY TotalPosts DESC; SELECT 1 AS Dummy, SUM(1)
// AS DummyCount FROM Posts WHERE Score IS NULL;
//
// Two statements; DuckDB returns the result of the last one, `SELECT 1, SUM(1) FROM Posts WHERE Score IS NULL`, and that is what is ported.
fn q20936(db: &'static So) -> String {
    let n = count(db.post.minus(&db.post.score));
    rows([row(vec![V::I(1), nullable(n, n)])])
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u), TopUsers AS
// (SELECT UserId, DisplayName, Reputation FROM RankedUsers WHERE Rank <= 10), PostStats AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER
// (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes, COALESCE(NULLIF(SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END),
// 0), 0) AS AcceptedAnswers FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS
// TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title), UserPosts AS (SELECT u.UserId, u.DisplayName, COUNT(p.Id) AS PostCount, COALESCE(SUM(ps.CommentCount), 0) AS
// TotalComments, COALESCE(SUM(ps.UpVotes), 0) AS TotalUpVotes, COALESCE(SUM(ps.DownVotes), 0) AS TotalDownVotes, COALESCE(SUM(ps.AcceptedAnswers), 0) AS TotalAcceptedAnswers
// FROM TopUsers u LEFT JOIN Posts p ON u.UserId = p.OwnerUserId LEFT JOIN PostStats ps ON p.Id = ps.PostId GROUP BY u.UserId, u.DisplayName) SELECT u.DisplayName, u.PostCount,
// u.TotalComments, u.TotalUpVotes, u.TotalDownVotes, u.TotalAcceptedAnswers FROM UserPosts u ORDER BY u.PostCount DESC, u.TotalUpVotes DESC;
fn q5174(db: &'static So) -> String {
    let Post { creation_date, accepted_answer_id, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(accepted_answer_id.opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, ((x, c), t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + x.is_some() as i64]);
    let up = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select((&ps).opt()).opt()).fold([0i64; 5], |a, p| match p {
        Some(Some(s)) => [a[0] + 1, a[1] + s[0], a[2] + s[1], a[3] + s[2], a[4] + s[3]],
        Some(None) => [a[0] + 1, a[1], a[2], a[3], a[4]],
        None => a,
    });
    let v = drain(&up);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId)
// v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation), TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, AcceptedAnswers,
// TotalVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank, RANK() OVER (ORDER BY PostCount DESC) AS PostCountRank FROM UserStats) SELECT UserId, DisplayName,
// Reputation, PostCount, QuestionCount, AnswerCount, AcceptedAnswers, TotalVotes, ReputationRank, PostCountRank, CASE WHEN ReputationRank <= 10 AND PostCountRank <= 10 THEN 'Top
// Contributor' WHEN ReputationRank <= 20 THEN 'High Reputation' ELSE 'Regular User' END AS UserType FROM TopUsers WHERE Reputation > 0 ORDER BY Reputation DESC, PostCount DESC;
fn q6825(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and((&vc).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, x), n)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && x.is_some()) as i64, a[4] + n.unwrap_or(0)],
            None => a,
        });
    let w = whole(&us).select(Ident::<User>::new().and(&db.user.reputation).and(&us)).window(rank, |((_, r), _): ((Id<User>, i64), [i64; 5])| Reverse(r), asc);
    let w = (&w).window(rank, |((_, a), _): (((Id<User>, i64), [i64; 5]), i64)| Reverse(a[0]), asc);
    let v = drain((&w).filt(|((((_, r), _), _), _)| r > 0));
    rows(v.into_iter().map(|(_, ((((u, _), a), r), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(r), V::I(p)]);
        f.push(V::S(if r <= 10 && p <= 10 { "Top Contributor" } else if r <= 20 { "High Reputation" } else { "Regular User" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE
// WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount FROM Users U LEFT JOIN Posts P ON U.Id =
// P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation), RecentActiveUsers AS (SELECT UserId, DisplayName, Reputation, RANK() OVER
// (ORDER BY MAX(P.LastActivityDate) DESC) AS ActivityRank FROM UserStats JOIN Posts P ON UserStats.UserId = P.OwnerUserId GROUP BY UserId, DisplayName, Reputation HAVING
// MAX(P.LastActivityDate) > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 DAY')), TopUsers AS (SELECT UserId, DisplayName, Reputation, ActivityRank FROM
// RecentActiveUsers WHERE ActivityRank <= 10) SELECT U.DisplayName, U.Reputation, U.TotalPosts, U.QuestionCount, U.AnswerCount, U.VoteCount, COALESCE(B.BadgeCount, 0) AS
// BadgeCount FROM UserStats U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON U.UserId = B.UserId WHERE U.UserId IN (SELECT UserId FROM
// TopUsers) ORDER BY U.Reputation DESC, U.TotalPosts DESC;
//
// TopUsers reads only the posts' LastActivityDate, so the ten most recently active users are found first and the posts x votes product is driven for them alone.
fn q8081(db: &'static So) -> String {
    let Post { owner_user, last_activity_date, post_type_id, .. } = &db.post;
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let la = db.post.group_by(owner_user).select(last_activity_date).fold(i64::MIN, |m, d| m.max(d));
    let act = (&la).filt(move |m| m > cut);
    let w = whole(&act).select(Ident::<User>::new().and(&act)).window(rank, |(_, m): (Id<User>, i64)| Reverse(m), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + v.is_some() as i64],
            None => a,
        });
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&s).and((&bc).opt()));
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::I(b.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(COUNT(c.Id), 0) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY
// p.CreationDate DESC) AS PostRank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, p.Title,
// p.CreationDate, p.OwnerUserId), UserRankings AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostStats AS (SELECT p.Id, p.Title, COUNT(v.Id) AS VoteCount, AVG(COALESCE(v.BountyAmount, 0)) AS AverageBounty, MAX(p.CreationDate) AS LatestCreationDate FROM Posts p LEFT
// JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title) SELECT ur.DisplayName, ur.Reputation, ur.ReputationRank, rp.Title, rp.CommentCount,
// ps.VoteCount, ps.AverageBounty, CASE WHEN rp.PostRank = 1 THEN 'Most Recent Post' ELSE 'Earlier Post' END AS PostStatus, CASE WHEN ps.LatestCreationDate IS NULL THEN 'No
// Votes' ELSE 'Votes Received' END AS VoteStatus FROM RankedPosts rp JOIN UserRankings ur ON ur.UserId = rp.PostId JOIN PostStats ps ON ps.Id = rp.PostId WHERE ur.Reputation >
// 1000 ORDER BY ur.Reputation DESC, rp.CommentCount DESC;
//
// `ur.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q3367(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, post_type_id, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(current_date(), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let uw = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let ur: MatSet<(Id<User>, i64)> = (&uw).map(|((u, _), r)| (u, r)).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ps = db
        .post
        .with(post_type_id.is_in([1, 2]))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())
        .fold([0i64; 3], |a, b| [a[0] + b.is_some() as i64, a[1] + b.flatten().unwrap_or(0), a[2] + 1]);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let high = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let v = drain(by_first(&rp).and(origid.select(&uidx).select(high).select(Ident::<User>::new().and(by_first(&ur)))).and(&ps).and(&cc));
    rows(v.into_iter().map(|(p, (((r, (u, ur)), s), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(ur));
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(c), V::I(s[0]), avg(s[1], s[2])]);
        f.extend([V::S(if r == 1 { "Most Recent Post" } else { "Earlier Post" }), V::S("Votes Received")]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostID, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
// FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'), UserMetrics AS (SELECT u.Id AS UserID, u.DisplayName, COALESCE(SUM(CASE WHEN
// v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, COUNT(DISTINCT b.Id) AS BadgeCount,
// AVG(p.Score) AS AvgPostScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id,
// u.DisplayName), ClosedPosts AS (SELECT ph.PostId, ph.UserDisplayName, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId,
// ph.UserDisplayName) SELECT rp.PostID, rp.Title, rp.CreationDate, um.DisplayName AS OwnerDisplayName, um.UpVoteCount, um.DownVoteCount, um.BadgeCount, um.AvgPostScore,
// cp.CloseCount, COALESCE(cp.CloseCount, 0) AS CloseCountNullable FROM RecentPosts rp JOIN UserMetrics um ON rp.OwnerUserId = um.UserID LEFT JOIN ClosedPosts cp ON rp.PostID =
// cp.PostId WHERE um.AvgPostScore > 10 ORDER BY rp.CreationDate DESC LIMIT 100;
//
// rn is never read. UserMetrics is needed only for the owners of the recent posts, so the posts x votes x badges product is driven for them alone.
fn q2803(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let owners: MatSet<Id<User>> = recent().select(owner_user).collect();
    let um = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, _)| match p {
            Some((s, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + 1, a[3] + s],
            None => a,
        });
    let bc = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { post, post_history_type_id, user_display_name, .. } = &db.post_history;
    type K = (Id<Post>, Option<Str>);
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post.and(user_display_name.opt())).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cv: MatSet<(K, i64)> = whole(&cp).select(Same::<K>::new().and(&cp)).collect();
    let by_post: HashIdx<Id<Post>, (K, i64)> = (&cv).map(|((p, _), _)| p).inv().collect();
    let v = drain(recent().select(owner_user.select(Ident::<User>::new().and((&um).filt(|a: [i64; 4]| a[2] > 0 && a[3] > 10 * a[2])).and(&bc)).and((&by_post).opt())));
    let v = top_n(v, |&(p, (_, c))| (Reverse(creation_date.get(p).unwrap()), p, c.map(|x| (x.0).1)), 100);
    rows(v.into_iter().map(|(p, (((u, a), b), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(b), avg(a[3], a[2])]);
        f.extend([oint(c.map(|x| x.1)), V::I(c.map_or(0, |x| x.1))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN
// v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(COUNT(DISTINCT p.Id), 0) AS PostCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON
// u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation), PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER
// (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId FROM Posts p WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1
// year')), ClosedPosts AS (SELECT h.PostId, h.CreationDate, (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = h.PostId AND ph.PostHistoryTypeId = 10) AS CloseCount FROM
// PostHistory h WHERE h.PostHistoryTypeId = 10) SELECT us.UserId, us.DisplayName, us.Reputation, us.UpVotes, us.DownVotes, pd.Title, pd.Score, pd.ViewCount, pp.CloseCount FROM
// UserStats us LEFT JOIN PostDetails pd ON us.UserId = pd.OwnerUserId LEFT JOIN ClosedPosts pp ON pd.PostId = pp.PostId WHERE us.Reputation > 1000 AND (us.UpVotes -
// us.DownVotes) > 100 AND pd.rn <= 5 ORDER BY us.Reputation DESC, pd.Score DESC;
//
// The ownerless posts form their own rn partition, which no user joins, so they are left out before ranking.
fn q4170(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let us = users()
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let w = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let pd = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let closes = || db.post_history.with(post_history_type_id.eq(10));
    let cn = closes().group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pp: HashIdx<Id<Post>, Id<PostHistory>> = closes().select(post).inv().collect();
    let v = drain((&us).filt(|a: [i64; 2]| a[0] - a[1] > 100).and(pd.select(Ident::<Post>::new().and(pp.select(post.select(&cn)).opt()))));
    rows(v.into_iter().map(|(u, (a, (p, c)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(oint(c));
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1), TagStats AS (SELECT
// Tag, COUNT(DISTINCT pt.PostId) AS QuestionCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount, COUNT(DISTINCT COALESCE(c.UserId,
// p.OwnerUserId)) AS UniqueUsers, AVG(u.Reputation) AS AvgUserReputation FROM PostTags pt JOIN Posts p ON pt.PostId = p.Id LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN
// Users u ON u.Id = p.OwnerUserId GROUP BY Tag), BadgeStats AS (SELECT b.Name AS BadgeName, COUNT(DISTINCT b.UserId) AS UserCount, COUNT(DISTINCT p.Id) AS PostCount FROM Badges
// b JOIN Users u ON b.UserId = u.Id JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY b.Name), CombinedStats AS (SELECT ts.Tag, ts.QuestionCount, ts.AcceptedAnswerCount,
// ts.UniqueUsers, ts.AvgUserReputation, bs.BadgeName, bs.UserCount AS BadgeUserCount, bs.PostCount AS BadgePostCount FROM TagStats ts LEFT JOIN BadgeStats bs ON ts.UniqueUsers >
// 0) SELECT Tag, QuestionCount, AcceptedAnswerCount, UniqueUsers, AvgUserReputation, BadgeName, BadgeUserCount, BadgePostCount FROM CombinedStats ORDER BY QuestionCount DESC,
// UniqueUsers DESC;
//
// `LEFT JOIN BadgeStats bs ON ts.UniqueUsers > 0` names only ts, so the tags that pass it are crossed with every badge.
fn q25392(db: &'static So) -> String {
    let Post { post_type_id, tags_str, accepted_answer_id, owner_user_id, owner_user, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(tag_list));
    let ts = qs()
        .select(accepted_answer_id.opt().and(comments_of(db).opt()).and(owner_user.select(&db.user.reputation).opt()))
        .fold([0i64; 3], |a, ((x, _), r)| [a[0] + x.is_some() as i64, a[1] + r.unwrap_or(0), a[2] + r.is_some() as i64]);
    let qc = qs().select(Ident::<Post>::new()).count_distinct();
    let uu = qs()
        .select(comments_of(db).select((&db.comment.user_id).opt()).opt().and(owner_user_id.opt()).flat_map(|(c, o): (Option<Option<i64>>, Option<i64>)| c.flatten().or(o)))
        .count_distinct();
    let Badge { name, user, .. } = &db.badge;
    let bu = db.badge.group_by(name).select(user.with(posts_of(db))).count_distinct();
    let bp = db.badge.group_by(name).select(user.select(posts_of(db))).count_distinct();
    let bs: HashIdx<(), (Str, (i64, i64))> = whole(&bu).select(Same::<Str>::new().and((&bu).and(&bp))).collect();
    let tags = (&ts).and(&qc).and((&uu).opt());
    let on = (&tags).filt(|(_, u)| u.unwrap_or(0) > 0).map(|_| ()).select(&bs);
    let v = drain((&tags).and(on.opt()));
    rows(v.into_iter().map(|(t, (((a, q), u), b))| {
        let mut f = vec![V::S(t), V::I(q), V::I(a[0]), V::I(u.unwrap_or(0)), avg(a[1], a[2])];
        f.extend(match b {
            Some((n, (k, m))) => [V::S(n), V::I(k), V::I(m)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// Rewritten (rewrites/468.sql): see rewrites/README.md.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
// p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 10), UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(b.Class), 0) AS TotalBadges,
// COUNT(DISTINCT p.Id) AS TotalQuestions FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 WHERE u.Reputation IS
// NOT NULL GROUP BY u.Id, u.Reputation), ClosedPosts AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, ph.Comment FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10) SELECT
// up.UserId, up.Reputation, up.TotalBadges, up.TotalQuestions, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(cp.Comment, 'No close comment') AS
// CloseComment, CASE WHEN up.TotalQuestions > 0 THEN 'Active User' ELSE 'Inactive User' END AS UserStatus FROM UserReputation up LEFT JOIN RankedPosts rp ON up.UserId =
// rp.OwnerUserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE up.Reputation > (SELECT AVG(Reputation) FROM Users WHERE Reputation IS NOT NULL) ORDER BY up.Reputation
// DESC, rp.Score DESC, rp.PostId, cp.CreationDate, cp.Comment FETCH FIRST 10 ROWS ONLY;
fn q468(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let (rs, rn) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let users = || db.user.with((&db.user.reputation).filt(move |r: i64| r * rn > rs));
    let q = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let ur = users()
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(q().opt()))
        .fold(0i64, |s, (c, _)| s + c.unwrap_or(0));
    let tq = users().group_by(Ident::<User>::new()).select(q().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let rp = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1).and(score.gt(10))));
    let v = drain((&ur).and(&tq).and(rp.select(Ident::<Post>::new().and(closes.opt())).opt()));
    let PostHistory { creation_date: hd, comment, .. } = &db.post_history;
    let v = top_n(v, |&(u, (_, p))| {
        let h = p.and_then(|x| x.1);
        let d = h.map(|h| hd.get(h).unwrap());
        let c = h.and_then(|h| comment.get(h));
        (
            Reverse(db.user.reputation.get(u).unwrap()),
            p.is_none(),
            Reverse(p.map(|x| score.get(x.0).unwrap())),
            p.map(|x| db.post.origid.get(x.0).unwrap()),
            d.is_none(),
            d,
            c.is_none(),
            c,
        )
    }, 10);
    rows(v.into_iter().map(|(u, ((b, n), p))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(b), V::I(n)]);
        match p {
            Some((p, h)) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
                f.push(V::S(h.and_then(|h| comment.get(h)).unwrap_or("No close comment")));
            }
            None => {
                f.extend((0..5).map(|_| V::Null));
                f.push(V::S("No close comment"));
            }
        }
        f.push(V::S(if n > 0 { "Active User" } else { "Inactive User" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN
// p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS
// DownVotes, COUNT(CASE WHEN b.Id IS NOT NULL THEN 1 END) AS BadgeCount, ROW_NUMBER() OVER (ORDER BY COUNT(p.Id) DESC) AS PostRank FROM Users u LEFT JOIN Posts p ON u.Id =
// p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName), TopUsers AS (SELECT UserId, DisplayName, TotalPosts,
// QuestionCount, AnswerCount, UpVotes, DownVotes, BadgeCount FROM UserStats WHERE PostRank <= 10) SELECT t.DisplayName, t.TotalPosts, t.QuestionCount, t.AnswerCount, t.UpVotes,
// t.DownVotes, t.BadgeCount, COALESCE((SELECT SUM(CASE WHEN ph.CreationDate IS NOT NULL THEN 1 ELSE 0 END) FROM PostHistory ph WHERE ph.UserId = t.UserId), 0) AS PostEdits,
// COALESCE((SELECT COUNT(DISTINCT c.Id) FROM Comments c JOIN Posts po ON c.PostId = po.Id WHERE po.OwnerUserId = t.UserId), 0) AS TotalComments FROM TopUsers t ORDER BY
// (t.UpVotes - t.DownVotes) DESC, t.BadgeCount DESC;
fn q7100(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + p.is_some() as i64, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5] + b.is_some() as i64]
        });
    let tu = top_n(drain(&us), |&(u, a)| (Reverse(a[0]), u), 10);
    let tu = rel(tu);
    type R = (Id<User>, [i64; 6]);
    let uid = || Same::<R>::new().map(|(u, _): R| u);
    let edits = db.post_history.group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cmts = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let mut v = drain((&tu).select(Same::<R>::new().and(uid().select(&edits).opt()).and(uid().select(&cmts).opt())));
    v.sort_by_key(|&(_, (((_, a), _), _))| (Reverse(a[3] - a[4]), Reverse(a[5])));
    rows(v.into_iter().map(|(_, (((u, a), e), c))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([V::I(e.unwrap_or(0)), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS TotalComments, SUM(CASE WHEN
// v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank FROM
// Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01
// 12:34:56' as timestamp) - INTERVAL '30 days' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName), TopPosts AS (SELECT PostId, Title,
// Score, CreationDate, OwnerDisplayName, TotalComments, TotalUpVotes, TotalDownVotes FROM RankedPosts WHERE ScoreRank <= 10) SELECT tp.Title, tp.OwnerDisplayName, tp.Score,
// tp.TotalComments, tp.TotalUpVotes, tp.TotalDownVotes, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS CloseCount, COALESCE(SUM(CASE WHEN
// ph.PostHistoryTypeId = 12 THEN 1 ELSE 0 END), 0) AS DeleteCount FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId GROUP BY tp.PostId, tp.Title,
// tp.OwnerDisplayName, tp.Score, tp.TotalComments, tp.TotalUpVotes, tp.TotalDownVotes ORDER BY tp.Score DESC;
//
// ScoreRank reads only Score, so the top posts are picked first and the comment x vote product is driven for those alone.
fn q9719(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let w = whole(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.is_in([1, 2]))).with(owner_user))
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ph = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(12)) as i64]);
    let v = drain((&s).and(&cc).and(&ph));
    rows(v.into_iter().map(|(p, ((a, c), h))| {
        let mut f = post_fields(db, p, &["title", "owner", "score"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(h[0]), V::I(h[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER
// BY p.Score DESC) AS PostRank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id,
// p.Title, p.CreationDate, p.Score, p.OwnerUserId), UserReputation AS (SELECT u.Id AS UserId, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users
// u WHERE u.Reputation IS NOT NULL), BadgesSummary AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS
// SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId) SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, ur.Reputation,
// ur.ReputationRank, COALESCE(bs.GoldBadges, 0) AS GoldBadges, COALESCE(bs.SilverBadges, 0) AS SilverBadges, COALESCE(bs.BronzeBadges, 0) AS BronzeBadges FROM RankedPosts rp
// LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN BadgesSummary bs ON rp.OwnerUserId = bs.UserId WHERE (ur.Reputation > 500 AND rp.Score > 10) OR
// (ur.Reputation IS NULL AND rp.CommentCount > 5) ORDER BY rp.Score DESC, ur.Reputation DESC FETCH FIRST 50 ROWS ONLY;
//
// PostRank is never read. `ur.Reputation IS NULL` holds exactly for the posts with no owner in Users.
fn q3526(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let ur: MatSet<(Id<User>, (i64, i64))> = (&w).map(|((u, rep), r)| (u, (rep, r))).collect();
    let by_user = by_first(&ur);
    let bs = db.badge.group_by(&db.badge.user_id).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    type R = (((i64, i64), Option<(i64, i64)>), Option<[i64; 3]>);
    let v = drain(
        recent()
            .select(score.and(&cc).and(owner_user.select(&by_user).opt()).and(owner_user_id.select(&bs).opt()))
            .filt(|(((s, c), u), _): R| match u {
                Some((rep, _)) => rep > 500 && s > 10,
                None => c > 5,
            }),
    );
    let v = top_n(v, |&(p, (((s, _), u), _))| (Reverse(s), u.is_none(), Reverse(u.map(|x| x.0)), p), 50);
    rows(v.into_iter().map(|(p, ((_, u), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(match u {
            Some((rep, r)) => [V::I(rep), V::I(r)],
            None => [V::Null, V::Null],
        });
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH PostScores AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0
// END), 0) AS NetScore, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id =
// c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId GROUP BY p.Id, p.Title, p.PostTypeId), TopPosts AS (SELECT ps.PostId, ps.Title, ps.PostTypeId, ps.NetScore,
// ps.CommentCount, RANK() OVER (PARTITION BY ps.PostTypeId ORDER BY ps.NetScore DESC) AS PostRank FROM PostScores ps), FilteredTopPosts AS (SELECT t.PostId, t.Title, t.NetScore,
// t.CommentCount, pt.Name AS PostTypeName FROM TopPosts t JOIN PostTypes pt ON t.PostTypeId = pt.Id WHERE t.PostRank <= 10) SELECT f.*, CASE WHEN f.CommentCount > 5 THEN 'Highly
// Discussed' ELSE 'Less Discussion' END AS DiscussionType, (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = f.PostId AND ph.PostHistoryTypeId = 10) AS CloseCount,
// NULLIF((SELECT AVG(CASE WHEN v.VoteTypeId IN (2, 3) THEN v.VoteTypeId END) FROM Votes v WHERE v.PostId = f.PostId), 0) AS AverageVoteType FROM FilteredTopPosts f LEFT JOIN
// Users u ON f.PostId = u.Id WHERE u.Reputation IS NOT NULL ORDER BY f.NetScore DESC;
//
// `f.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. The AVG over vote types 2 and 3 is an exact integer mean.
fn q1259(db: &'static So) -> String {
    let Post { owner_user_id, post_type_id, origid, .. } = &db.post;
    let by_uid: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(owner_user_id.select(&by_uid).opt()))
        .fold(0i64, |n, ((t, _), _)| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(&ps)).window(rank, |(_, n): (Id<Post>, i64)| Reverse(n), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let closes = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).fold(0i64, |n, t| n + (t == Some(10)) as i64);
    let av = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| match t {
        Some(t @ (2 | 3)) => [a[0] + t, a[1] + 1],
        _ => a,
    });
    let v = drain((&tp).with(origid.select(&uidx)).select((&ps).and(&cc).and(&closes).and(&av)));
    rows(v.into_iter().map(|(p, (((n, c), k), a))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(n), V::I(c)]);
        f.extend(post_fields(db, p, &["type"]));
        f.extend([V::S(if c > 5 { "Highly Discussed" } else { "Less Discussion" }), V::I(k), avg(a[0], a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p LEFT JOIN
// Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY
// p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId), RecentPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes, p.OwnerUserId FROM
// RankedPosts rp JOIN Posts p ON rp.PostId = p.Id WHERE rp.rn = 1), UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(rp.Score), 0) AS TotalScore,
// COUNT(rp.PostId) AS TotalPosts, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN RecentPosts rp ON u.Id = rp.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE
// u.Reputation >= 1000 GROUP BY u.Id, u.DisplayName) SELECT us.UserId, us.DisplayName, us.TotalScore, us.TotalPosts, us.BadgeCount FROM UserStats us WHERE us.BadgeCount > 2
// ORDER BY us.TotalScore DESC LIMIT 10;
//
// Only rn = 1 is read, and rn reads only base columns, so each owner's newest question is picked first; its comment and vote sums are never read.
fn q498(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let by_owner = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let us = db
        .user
        .with((&db.user.reputation).ge(1000))
        .group_by(Ident::<User>::new())
        .select(by_owner.select(score).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (s, b)| [a[0] + s.unwrap_or(0), a[1] + s.is_some() as i64, a[2] + b.is_some() as i64]);
    let v = top_n(drain((&us).filt(|a: [i64; 3]| a[2] > 2)), |&(u, a)| (Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS
// UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotes, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS rn FROM
// Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'), TopQuestions AS (SELECT Id,
// Title, CreationDate, Score, ViewCount, UpVotes, DownVotes FROM RankedPosts WHERE rn <= 10), HelpfulComments AS (SELECT c.PostId, COUNT(*) AS HelpfulCount FROM Comments c WHERE
// LOWER(c.Text) LIKE '%helpful%' GROUP BY c.PostId) SELECT tq.Title, tq.CreationDate, tq.Score, tq.ViewCount, tq.UpVotes, tq.DownVotes, COALESCE(hc.HelpfulCount, 0) AS
// HelpfulCommentsCount, CASE WHEN tq.Score >= 10 THEN 'High Score' WHEN tq.Score BETWEEN 5 AND 9 THEN 'Moderate Score' ELSE 'Low Score' END AS ScoreCategory, CASE WHEN
// p.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus FROM TopQuestions tq LEFT JOIN Posts p ON tq.Id = p.Id LEFT JOIN HelpfulComments hc ON tq.Id = hc.PostId
// ORDER BY tq.Score DESC, tq.CreationDate DESC;
//
// rn numbers the post x vote rows; the rows of one post tie on CreationDate and agree in every projected column, so which of them make the cut cannot be observed.
fn q4308(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, closed_date, .. } = &db.post;
    let base = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let v = drain(base().select(votes_of(db).opt()));
    let v = top_n(v, |&(p, x)| (Reverse(creation_date.get(p).unwrap()), p, x), 10);
    let tq = rel(v);
    type R = (Id<Post>, Option<Id<Vote>>);
    let pid = || Same::<R>::new().map(|(p, _): R| p);
    let ud = base().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let helpful = db.comment.with((&db.comment.text).filt(|t: Str| t.to_lowercase().contains("helpful"))).group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tq).select(pid().and(pid().select(&ud)).and(pid().select(&helpful).opt())));
    rows(v.into_iter().map(|(_, ((p, a), h))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(h.unwrap_or(0))]);
        f.push(V::S(if s >= 10 { "High Score" } else if s >= 5 { "Moderate Score" } else { "Low Score" }));
        f.push(V::S(if closed_date.get(p).is_some() { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.AcceptedAnswerId, U.DisplayName AS OwnerDisplayName, RANK()
// OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS Rank FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as
// timestamp) - INTERVAL '1 year' AND P.Score IS NOT NULL), TopPosts AS (SELECT PostId, Title, Body, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank
// <= 10), VoteAggregates AS (SELECT PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM
// Votes V GROUP BY PostId), CommentsSummary AS (SELECT C.PostId, COUNT(*) AS CommentCount, MAX(C.CreationDate) AS LastCommentDate FROM Comments C GROUP BY C.PostId) SELECT
// TP.Title, TP.OwnerDisplayName, TP.CreationDate, TP.Score, COALESCE(VA.UpVotes, 0) AS UpVotes, COALESCE(VA.DownVotes, 0) AS DownVotes, COALESCE(CS.CommentCount, 0) AS
// CommentCount, CS.LastCommentDate FROM TopPosts TP LEFT JOIN VoteAggregates VA ON TP.PostId = VA.PostId LEFT JOIN CommentsSummary CS ON TP.PostId = CS.PostId ORDER BY TP.Score
// DESC, TP.CreationDate ASC;
fn q30077(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let va = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cs = db.comment.group_by(&db.comment.post).select(&db.comment.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&tp).select((&va).opt().and((&cs).opt())));
    rows(v.into_iter().map(|(p, (a, c))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["title", "owner", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.map_or(0, |c| c.0)), ots(c.map(|c| c.1))]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE
// WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COUNT(CM.Id) AS TotalComments, SUM(V.BountyAmount) AS TotalBounty, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(U.UpVotes) AS TotalUpVotes, SUM(U.DownVotes) AS TotalDownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments CM ON P.Id = CM.PostId LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (2, 3) GROUP BY U.Id, U.DisplayName), TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers,
// TotalComments, TotalBounty, TotalViews, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY TotalPosts DESC) AS UserRank FROM UserActivity) SELECT TU.DisplayName,
// TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.TotalComments, TU.TotalBounty, TU.TotalViews, TU.TotalUpVotes, TU.TotalDownVotes, HP.TotalQuestionsAnswered FROM TopUsers
// TU LEFT JOIN (SELECT OwnerUserId, COUNT(DISTINCT P.Id) AS TotalQuestionsAnswered FROM Posts P WHERE P.PostTypeId = 2 AND P.ParentId IS NOT NULL GROUP BY OwnerUserId) HP ON
// TU.UserId = HP.OwnerUserId WHERE TU.UserRank <= 10;
//
// UserRank reads only the distinct post count, so the top users are picked first and the posts x comments x votes product is driven for them alone.
fn q6725(db: &'static So) -> String {
    let Post { post_type_id, view_count, parent_id, .. } = &db.post;
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&np).select(Ident::<User>::new().and(&np)).window(rank, |(_, n): (Id<User>, i64)| Reverse(n), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select((&db.vote.bounty_amount).opt());
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and(posts_of(db).select(post_type_id.and(view_count.opt()).and(comments_of(db).opt()).and(bounty.opt())).opt()))
        .fold([0i64; 8], |a, ((up, dn), p)| match p {
            Some((((t, w), c), b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0), a[5] + w.unwrap_or(0), a[6] + up, a[7] + dn]
            }
            None => [a[0], a[1], a[2], a[3], a[4], a[5], a[6] + up, a[7] + dn],
        });
    let hp = db.post.with(post_type_id.eq(2)).with(parent_id).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&ua).and(&np).and((&hp).opt()));
    rows(v.into_iter().map(|(u, ((a, n), h))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), V::I(a[5]), V::I(a[6]), V::I(a[7])];
        f.push(oint(h));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END), 0) AS VoteCount, COALESCE(SUM(CASE WHEN
// P.AnswerCount > 0 THEN 1 ELSE 0 END), 0) AS AnsweredQuestions, COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount, COALESCE(SUM(P.ViewCount), 0) AS
// TotalViews, DATE_TRUNC('month', U.CreationDate) AS MonthJoined FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Votes V ON U.Id = V.UserId
// LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, MonthJoined), RankedUsers AS (SELECT UserId, DisplayName, VoteCount, AnsweredQuestions, CommentCount,
// TotalViews, MonthJoined, ROW_NUMBER() OVER (PARTITION BY MonthJoined ORDER BY TotalViews DESC) AS ViewRank, RANK() OVER (ORDER BY VoteCount DESC) AS VoteRank FROM
// UserActivity) SELECT U.DisplayName, U.VoteCount, U.AnsweredQuestions, U.CommentCount, U.TotalViews, U.MonthJoined, COALESCE(AVG(U2.TotalViews), 0) AS AverageViewsInMonth,
// COALESCE(MAX(U2.VoteCount), 0) AS HighestVotesInMonth FROM RankedUsers U LEFT JOIN RankedUsers U2 ON U.MonthJoined = U2.MonthJoined AND U.UserId <> U2.UserId WHERE U.ViewRank
// <= 5 GROUP BY U.DisplayName, U.VoteCount, U.AnsweredQuestions, U.CommentCount, U.TotalViews, U.MonthJoined ORDER BY U.MonthJoined, U.TotalViews DESC;
//
// The final GROUP BY is by the projected columns, not the user id. The self-join `U.MonthJoined = U2.MonthJoined AND U.UserId <> U2.UserId` is driven for the top five users of each month against every user of that month.
fn q1399(db: &'static So) -> String {
    let Post { post_type_id, answer_count, view_count, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(answer_count.opt().and(view_count.opt()).and(comments_of(db).opt())).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (p, t)| {
            let (n, w, c) = p.map_or((None, None, None), |((n, w), c)| (n, w, c));
            [a[0] + matches!(t, Some(2 | 3)) as i64, a[1] + (n.unwrap_or(0) > 0) as i64, a[2] + c.is_some() as i64, a[3] + w.unwrap_or(0)]
        });
    let month = (&db.user.creation_date).map(trunc_month);
    let w = db.user.group_by(&month).select(Ident::<User>::new().and(&ua)).window(row_number, |(u, a): (Id<User>, [i64; 4])| (Reverse(a[3]), u), asc);
    type R = (Id<User>, (([i64; 4], i64), Str));
    type O = (Id<User>, [i64; 4]);
    let top: MatSet<R> = (&w).filt(|(_, r)| r <= 5).map(|((u, _), _)| u).select(Ident::<User>::new().and((&ua).and(&month).and(&db.user.display_name))).collect();
    let by_month: HashIdx<i64, Id<User>> = db.user.select(&month).inv().collect();
    let others = Same::<R>::new()
        .and(Same::<R>::new().map(|(_, ((_, m), _)): R| m).select(&by_month).select(Ident::<User>::new().and(&ua)))
        .filt(|((u, _), (w, _)): (R, O)| w != u)
        .map(|(_, o): (R, O)| o);
    type J = (R, Option<O>);
    let rows_: MatSet<J> = (&top).select(Same::<R>::new().and(others.opt())).collect();
    let s = (&rows_)
        .group_by(Same::<J>::new().map(|((_, ((a, m), n)), _): J| (n, a, m)))
        .select(Same::<J>::new().map(|(_, o): J| o))
        .fold([0i64, 0, i64::MIN], |a, o| match o {
            Some((_, b)) => [a[0] + b[3], a[1] + 1, a[2].max(b[0])],
            None => a,
        });
    let v = drain(&s);
    rows(v.into_iter().map(|((n, a, m), s)| {
        let mut f = vec![V::S(n)];
        f.extend(a.map(V::I));
        f.push(V::T(m));
        f.push(if s[1] == 0 { V::F(0.0) } else { avg(s[0], s[1]) });
        f.push(V::I(if s[1] == 0 { 0 } else { s[2] }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS
// PostRank, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, p.OwnerUserId, u.DisplayName AS OwnerDisplayName FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT
// JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'), TopPosts AS (SELECT rp.PostId, rp.Title,
// rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, COALESCE(b.Name, 'No Badge') AS OwnerBadge, rp.CommentCount, (SELECT AVG(v.BountyAmount) FROM Votes v WHERE
// v.PostId = rp.PostId AND v.VoteTypeId = 8) AS AverageBounty FROM RankedPosts rp LEFT JOIN Badges b ON rp.OwnerUserId = b.UserId AND b.Class = 1 WHERE rp.PostRank <= 5) SELECT
// tp.PostId, tp.Title, tp.CreationDate, tp.Score AS TotalScore, tp.ViewCount, tp.OwnerDisplayName, tp.OwnerBadge, tp.CommentCount, CASE WHEN tp.CommentCount > 0 THEN 'Comments
// present' ELSE 'No comments' END AS CommentsStatus, CASE WHEN tp.AverageBounty IS NULL THEN 'No bounty offered' ELSE CONCAT('Avg Bounty: ', tp.AverageBounty) END AS BountyInfo
// FROM TopPosts tp WHERE tp.Score > 10 ORDER BY tp.Score DESC, tp.ViewCount DESC LIMIT 50;
//
// PostRank numbers the post x comment rows, so the rows of one post tie on CreationDate; ties are broken by the comment id, which no projected column shows.
fn q32290(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user_id, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let w = base()
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date).and(comments_of(db).opt()))
        .window(row_number, |((p, d), c): ((Id<Post>, i64), Option<Id<Comment>>)| (Reverse(d), p, c), asc);
    let tp = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p);
    let cc = base().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let gold: HashIdx<i64, Id<Badge>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user_id).inv().collect();
    let ab = db.vote.with((&db.vote.vote_type_id).eq(8)).group_by(&db.vote.post).select((&db.vote.bounty_amount).opt()).fold([0i64; 2], |a, b| [a[0] + b.unwrap_or(0), a[1] + b.is_some() as i64]);
    let v = drain(tp.with(score.gt(10)).select(Ident::<Post>::new().and(owner_user_id.select(&gold).select(&db.badge.name).opt()).and(&cc).and((&ab).opt())));
    let v = top_n(v, |&(_, (((p, b), _), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, b)
    }, 50);
    rows(v.into_iter().map(|(_, (((p, b), c), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::S(b.unwrap_or("No Badge")), V::I(c), V::S(if c > 0 { "Comments present" } else { "No comments" })]);
        f.push(match a {
            Some([s, n]) if n > 0 => V::Owned(format!("Avg Bounty: {:?}", s as f64 / n as f64)),
            _ => V::S("No bounty offered"),
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBountyEarned, DENSE_RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 GROUP BY U.Id, U.DisplayName, U.Reputation), PopularTags AS (SELECT T.TagName, COUNT(P.Id) AS PostCount FROM Tags T
// JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName HAVING COUNT(P.Id) > 10), RecentPostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate,
// P.ViewCount, U.DisplayName AS OwnerName, (SELECT COUNT(C.Id) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days') SELECT U.UserId, U.DisplayName AS UserName, U.Reputation, U.TotalPosts, U.TotalAnswers,
// U.TotalBountyEarned, U.ReputationRank, PT.TagName, RPD.PostId, RPD.Title, RPD.CreationDate, RPD.ViewCount, RPD.CommentCount FROM UserStats U LEFT JOIN PopularTags PT ON
// U.TotalPosts > 0 LEFT JOIN RecentPostDetails RPD ON RPD.OwnerName = U.DisplayName WHERE U.ReputationRank <= 10 ORDER BY U.Reputation DESC, PT.PostCount DESC OFFSET 0 ROWS
// FETCH NEXT 5 ROWS ONLY;
//
// ReputationRank reads only Reputation, so the top users are picked first and their posts x votes product is driven for them alone. `LEFT JOIN PopularTags
// PT ON U.TotalPosts > 0` names only U, so a user with posts is crossed with every popular tag.
fn q4354(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(dense_rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let tu: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let tus: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let us = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(bounty.opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let np = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let lt = tag_mentions(db);
    let pt = (&lt).group_by((&lt).map(|(_, t)| t).select(&db.tag.tag_name)).select(Same::<(Id<Post>, Id<Tag>)>::new()).fold(0i64, |n, _| n + 1);
    let ptw: HashIdx<(), (Str, i64)> = whole((&pt).filt(|n| n > 10)).select(Same::<Str>::new().and(&pt)).collect();
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rpd: HashIdx<Str, Id<Post>> = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(owner_user.select(&db.user.display_name)).inv().collect();
    let tags = Ident::<User>::new().with((&np).filt(|n| n > 0)).map(|_| ()).select(&ptw);
    let posts = (&db.user.display_name).select((&rpd).select(Ident::<Post>::new().and(&cc)));
    let v = drain(by_first(&tu).and(&us).and(&np).and(tags.opt()).and(posts.opt()));
    let v = top_n(v, |&(u, ((((_, _), _), t), p))| {
        let n = t.map(|(_, n)| n);
        (Reverse(db.user.reputation.get(u).unwrap()), n.is_none(), Reverse(n), u, t, p)
    }, 5);
    rows(v.into_iter().map(|(u, ((((r, a), n), t), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(r)]);
        f.push(t.map_or(V::Null, |(t, _)| V::S(t)));
        match p {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
                f.push(V::I(c));
            }
            None => f.extend((0..5).map(|_| V::Null)),
        }
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE
// WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount, COUNT(Vote.UserId) AS VoteCount, COALESCE(SUM(CASE
// WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1
// ELSE 0 END), 0) AS BronzeBadges, SUM(P.ViewCount) AS TotalViewCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN
// Votes Vote ON P.Id = Vote.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName), RankedUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount,
// AnswerCount, CommentCount, VoteCount, GoldBadges, SilverBadges, BronzeBadges, TotalViewCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC, TotalViewCount DESC) AS Rank FROM
// UserActivity) SELECT R.Rank, R.DisplayName, R.PostCount, R.QuestionCount, R.AnswerCount, R.CommentCount, R.VoteCount, R.GoldBadges, R.SilverBadges, R.BronzeBadges,
// R.TotalViewCount FROM RankedUsers R WHERE R.Rank <= 10 ORDER BY R.Rank;
//
// Rank leads with the distinct post count, so only users with at least the tenth-highest count can be in the top ten; the product is driven for those alone,
// and every other user ranks below all of them.
fn q5125(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nw = whole(&np).select(Ident::<User>::new().and(&np)).window(rank, |(_, n): (Id<User>, i64)| Reverse(n), asc);
    let cand: MatSet<Id<User>> = (&nw).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let ua = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).select((&db.vote.user_id).opt()).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 9], |a, (p, b)| {
            let (t, w, c, v) = p.map_or((0, None, None, None), |(((t, w), c), v)| (t, w, c, v));
            [
                a[0] + (t == 1) as i64,
                a[1] + (t == 2) as i64,
                a[2] + c.is_some() as i64,
                a[3] + v.flatten().is_some() as i64,
                a[4] + (b == Some(1)) as i64,
                a[5] + (b == Some(2)) as i64,
                a[6] + (b == Some(3)) as i64,
                a[7] + w.is_some() as i64,
                a[8] + w.unwrap_or(0),
            ]
        });
    let w = whole(&ua).select(Ident::<User>::new().and((&ua).and(&np))).window(row_number, |(u, (a, n)): (Id<User>, ([i64; 9], i64))| (Reverse(n), a[7] == 0, Reverse(a[8]), u), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10));
    rows(v.into_iter().map(|(_, ((u, (a, n)), r))| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), V::I(n)];
        f.extend(a[..7].iter().map(|&x| V::I(x)));
        f.push(nullable(a[8], a[7]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY
// p.CreationDate DESC) AS UserRank FROM Posts p WHERE p.PostTypeId = 1), TagSplit AS (SELECT rp.PostId, UNNEST(string_to_array(rp.Tags, '><')) AS Tag, rp.Title, rp.CreationDate,
// rp.ViewCount, rp.AnswerCount, rp.Score FROM RankedPosts rp), UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(p.ViewCount) AS
// TotalViews, AVG(p.Score) AS AvgScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName), PopularTags AS (SELECT ts.Tag,
// COUNT(ts.PostId) AS TagCount FROM TagSplit ts GROUP BY ts.Tag HAVING COUNT(ts.PostId) > 10), TopUsers AS (SELECT us.UserId, us.DisplayName, us.QuestionCount, us.TotalViews,
// us.AvgScore, ROW_NUMBER() OVER (ORDER BY us.TotalViews DESC) AS ViewRank FROM UserStats us) SELECT tu.DisplayName AS TopUser, tu.QuestionCount, tu.TotalViews, tu.AvgScore,
// pt.Tag AS PopularTag, pt.TagCount FROM TopUsers tu JOIN PopularTags pt ON tu.QuestionCount > 5 ORDER BY tu.QuestionCount DESC, pt.TagCount DESC;
//
// `JOIN PopularTags pt ON tu.QuestionCount > 5` names only tu, so those users are crossed with every popular tag. The split is on the raw Tags text, so the
// first and last elements keep their `<` and `>`.
fn q26840(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, score, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let pt = qs().group_by(tags_str.flat_map(|t: Str| t.split("><"))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let us = qs()
        .with(&db.post.owner_user)
        .group_by(&db.post.owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let v = drain((&us).filt(|a: [i64; 4]| a[0] > 5).cross((&pt).filt(|n| n > 10)));
    rows(v.into_iter().map(|((u, t), (a, n))| row(vec![user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0]), V::S(t), V::I(n)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.OwnerUserId, p.CreationDate, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
// COUNT(DISTINCT c.Id) AS CommentCount, p.Score FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id,
// p.Title, p.PostTypeId, p.OwnerUserId, p.CreationDate, p.Score), UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class
// = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId), TopPosts AS (SELECT rp.PostId, rp.Title,
// rp.OwnerUserId, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, rp.CommentCount, ROW_NUMBER() OVER (ORDER BY rp.Score DESC) AS TopRank FROM RankedPosts rp JOIN UserBadges ub
// ON rp.OwnerUserId = ub.UserId WHERE rp.Rank = 1) SELECT p.Title, u.DisplayName, u.Reputation, COALESCE(tp.GoldBadges, 0) AS GoldBadges, COALESCE(tp.SilverBadges, 0) AS
// SilverBadges, COALESCE(tp.BronzeBadges, 0) AS BronzeBadges, tp.CommentCount FROM TopPosts tp JOIN Users u ON tp.OwnerUserId = u.Id JOIN Posts p ON tp.PostId = p.Id WHERE
// u.Reputation > 500 ORDER BY tp.CommentCount DESC, u.Reputation DESC;
fn q31711(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(current_date(), -1))).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let high = Ident::<User>::new().with((&db.user.reputation).gt(500));
    let v = drain((&cc).and(owner_user.select(high.and(&ub))));
    rows(v.into_iter().map(|(p, (c, (u, b)))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(b.map(V::I));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN
// 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS rn FROM
// Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'), AcceptedAnswers AS (SELECT P.AcceptedAnswerId AS AnswerId, COUNT(P.AcceptedAnswerId) AS
// AcceptedCount FROM Posts P WHERE P.PostTypeId = 2 GROUP BY P.AcceptedAnswerId) SELECT UB.UserId, UB.DisplayName, COALESCE(P.Title, 'No Recent Posts') AS RecentPostTitle,
// COALESCE(P.CreationDate, NULL) AS LastPostDate, COALESCE(P.Score, 0) AS LastPostScore, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges,
// COALESCE(A.AcceptedCount, 0) AS AcceptedCount FROM UserBadges UB LEFT JOIN RecentPosts P ON UB.UserId = P.OwnerUserId AND P.rn = 1 LEFT JOIN AcceptedAnswers A ON A.AnswerId =
// P.PostId WHERE (UB.BadgeCount > 0 OR P.PostId IS NOT NULL) ORDER BY UB.DisplayName ASC, P.CreationDate DESC NULLS LAST LIMIT 50;
//
// `A.AnswerId = P.PostId` joins the answers' AcceptedAnswerId values to the post's raw id.
fn q41(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, accepted_answer_id, origid, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let by_owner = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let aa = db.post.with(post_type_id.eq(2)).group_by(accepted_answer_id).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(
        (&ub)
            .and(by_owner.select(Ident::<Post>::new().and(origid.select(&aa).opt())).opt())
            .filt(|(a, p): ([i64; 4], Option<(Id<Post>, Option<i64>)>)| a[0] > 0 || p.is_some()),
    );
    let v = top_n(v, |&(u, (_, p))| {
        let d = p.map(|(p, _)| creation_date.get(p).unwrap());
        (db.user.display_name.get(u).unwrap(), d.is_none(), Reverse(d), u)
    }, 50);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        match p {
            Some((p, n)) => {
                f.push(V::S(db.post.title.get(p).unwrap_or("No Recent Posts")));
                f.extend(post_fields(db, p, &["created", "score"]));
                f.extend(a.map(V::I));
                f.push(V::I(n.unwrap_or(0)));
            }
            None => {
                f.extend([V::S("No Recent Posts"), V::Null, V::I(0)]);
                f.extend(a.map(V::I));
                f.push(V::I(0));
            }
        }
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT B.Id) AS BadgeCount FROM Users U LEFT JOIN Votes V
// ON U.Id = V.UserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation), PostRanked AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Score, P.CreationDate,
// ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank FROM Posts P WHERE P.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'), PostStats
// AS (SELECT UR.UserId, P.OwnerUserId, COUNT(P.PostId) AS PostCount, SUM(P.Score) AS TotalScore FROM UserReputation UR JOIN PostRanked P ON UR.UserId = P.OwnerUserId GROUP BY
// UR.UserId, P.OwnerUserId), ClosedPosts AS (SELECT PH.PostId, COUNT(*) AS CloseCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId) SELECT
// U.DisplayName, UR.Reputation, COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(PS.TotalScore, 0) AS TotalScore, COALESCE(CP.CloseCount, 0) AS ClosedPostCount, (CASE WHEN
// UR.Reputation > 1000 THEN 'Expert' WHEN UR.Reputation BETWEEN 500 AND 1000 THEN 'Veteran' ELSE 'Novice' END) AS UserLevel FROM Users U JOIN UserReputation UR ON U.Id =
// UR.UserId LEFT JOIN PostStats PS ON U.Id = PS.UserId LEFT JOIN ClosedPosts CP ON U.Id = CP.PostId WHERE UR.TotalBounty > 0 OR UR.BadgeCount > 0 ORDER BY UR.Reputation DESC,
// PS.PostCount DESC;
//
// `U.Id = CP.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q3809(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()))
        .fold(0i64, |s, (b, _)| s + b.flatten().unwrap_or(0));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post_id).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(
        (&ur)
            .and(&bc)
            .filt(|(b, n): (i64, i64)| b > 0 || n > 0)
            .and((&ps).opt())
            .and((&db.user.origid).select(&cp).opt()),
    );
    let mut v = v;
    v.sort_by_key(|&(u, ((_, p), _))| (Reverse(db.user.reputation.get(u).unwrap()), p.is_none(), Reverse(p.map(|p| p[0]))));
    rows(v.into_iter().map(|(u, ((_, p), c))| {
        let r = db.user.reputation.get(u).unwrap();
        let p = p.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p[0]), V::I(p[1]), V::I(c.unwrap_or(0))]);
        f.push(V::S(if r > 1000 { "Expert" } else if r >= 500 { "Veteran" } else { "Novice" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RN FROM Posts
// p), UserVoteAnalytics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN
// v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN v.VoteTypeId IN (1, 4, 7) THEN 1 ELSE 0 END) AS Acceptances FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY u.Id, u.DisplayName) SELECT up.DisplayName AS UserName, rp.PostId, rp.Title, rp.ViewCount, up.TotalVotes, up.UpVotes, up.DownVotes, up.Acceptances, CASE WHEN
// up.TotalVotes IS NULL THEN 'No Votes' WHEN up.TotalVotes > 50 THEN 'High Engagement' ELSE 'Moderate Engagement' END AS EngagementLevel, COUNT(DISTINCT c.Id) AS CommentCount,
// COALESCE(MAX(cl.Name), 'No Close Reason') AS CloseReason FROM RankedPosts rp LEFT JOIN UserVoteAnalytics up ON up.UserId = rp.PostId % 1000 LEFT JOIN Comments c ON c.PostId =
// rp.PostId LEFT JOIN PostHistory ph ON ph.PostId = rp.PostId AND ph.PostHistoryTypeId = 10 LEFT JOIN CloseReasonTypes cl ON cl.Id = CAST(ph.Comment AS INT) WHERE rp.RN <= 10
// GROUP BY up.DisplayName, rp.PostId, rp.Title, rp.ViewCount, up.TotalVotes, up.UpVotes, up.DownVotes, up.Acceptances HAVING AVG(rp.ViewCount) IS NULL OR SUM(up.UpVotes) >
// SUM(up.DownVotes) ORDER BY rp.ViewCount DESC, EngagementLevel;
//
// `up.UserId = rp.PostId % 1000` is the SQL's own join key, computed from the post's raw id and looked up in the users' raw ids. The HAVING sums are
// folded over the joined comment x history rows.
fn q21346(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, origid, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let uva = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 4], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + matches!(t, 1 | 4 | 7) as i64],
        None => a,
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let up = || origid.map(|i: i64| i % 1000).select(&uidx).select(Ident::<User>::new().and(&uva));
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let ph = history_of(db)
        .select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)))
        .select((&db.post_history.comment).flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason).opt());
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let s = (&rp)
        .group_by(Ident::<Post>::new())
        .select(up().opt().and(comments_of(db).opt()).and(ph.opt()))
        .fold((None::<Str>, 0i64, 0i64), |(m, su, sd), ((u, _), r)| {
            let m = match (m, r.flatten()) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            };
            match u {
                Some((_, x)) => (m, su + x[1], sd + x[2]),
                None => (m, su, sd),
            }
        });
    type U = Option<(Id<User>, [i64; 4])>;
    type S = (Option<Str>, i64, i64);
    let v = drain(
        (&rp)
            .select(Ident::<Post>::new().and(view_count.opt()).and(up().opt()).and(&cc).and(&s))
            .filt(|((((_, w), u), _), (_, su, sd)): ((((Id<Post>, Option<i64>), U), i64), S)| w.is_none() || (u.is_some() && su > sd)),
    );
    let lvl = |u: U| match u {
        None => "No Votes",
        Some((_, a)) if a[0] > 50 => "High Engagement",
        _ => "Moderate Engagement",
    };
    rows(v.into_iter().map(|(_, ((((p, _), u), cc), (mx, _, _)))| {
        let mut f = vec![u.map_or(V::Null, |(u, _)| user_col(db, u, "name"))];
        f.extend(post_fields(db, p, &["id", "title", "views"]));
        match u {
            Some((_, a)) => f.extend(a.map(V::I)),
            None => f.extend((0..4).map(|_| V::Null)),
        }
        f.extend([V::S(lvl(u)), V::I(cc), V::S(mx.unwrap_or("No Close Reason"))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS
// AnswerCount, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP
// BY u.Id, u.DisplayName, u.Reputation, u.CreationDate), TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, BadgeCount, RANK() OVER (ORDER BY
// Reputation DESC) AS ReputationRank FROM UserStats), RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Owner, p.Score, p.ViewCount, COUNT(c.Id)
// AS CommentCount FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) -
// INTERVAL '1 month' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount) SELECT tu.DisplayName AS UserDisplayName, tu.Reputation, tu.PostCount,
// tu.AnswerCount, tu.BadgeCount, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate, rp.Score AS RecentPostScore, rp.ViewCount AS RecentPostViews, rp.CommentCount AS
// RecentPostComments FROM TopUsers tu JOIN RecentPosts rp ON tu.UserId = rp.PostId WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC, rp.CreationDate DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first. `tu.UserId = rp.PostId` joins a user id to a post id, through the raw ids.
fn q7637(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, b)| [a[0] + (t == Some(2)) as i64, a[1] + b.is_some() as i64]);
    let np = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let recent = Ident::<Post>::new().with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let v = drain((&us).and(&np).and((&db.user.origid).select(&pidx).select(recent.and(&cc))));
    let mut v = v;
    v.sort_by_key(|&(u, (_, (p, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(u, ((a, n), (p, c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS
// TotalQuestions, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes, SUM(COALESCE(b.Class, 0)) AS TotalBadges FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP
// BY u.Id, u.Reputation, u.DisplayName), TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalUpVotes, TotalDownVotes,
// TotalBadges, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats), RecentActivePosts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS RecentPosts,
// MAX(p.LastActivityDate) AS MostRecentActivity FROM Posts p WHERE p.LastActivityDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.OwnerUserId)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalScore, tu.TotalUpVotes, tu.TotalDownVotes, tu.TotalBadges, rap.RecentPosts,
// rap.MostRecentActivity FROM TopUsers tu LEFT JOIN RecentActivePosts rap ON tu.UserId = rap.OwnerUserId WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the posts x badges product is driven for them alone.
fn q8276(db: &'static So) -> String {
    let Post { post_type_id, score, last_activity_date, owner_user, .. } = &db.post;
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and(posts_of(db).select(score).opt()).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (((up, dn), s), c)| [a[0] + s.unwrap_or(0), a[1] + up, a[2] + dn, a[3] + c.unwrap_or(0)]);
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let rap = db
        .post
        .with(last_activity_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(owner_user)
        .select(last_activity_date)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = drain((&us).and(&pc).and((&rap).opt()));
    v.sort_by_key(|&(u, _)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(u, ((a, p), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(p.map(V::I));
        f.extend(a.map(V::I));
        f.extend([oint(r.map(|r| r.0)), ots(r.map(|r| r.1))]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN
// 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, P.Score, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank FROM
// Posts P WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'), CombinedData AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges, RP.PostId, RP.Title AS RecentPostTitle,
// RP.CreationDate AS RecentPostDate, RP.Score AS RecentPostScore FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN RecentPosts RP ON U.Id = RP.OwnerUserId AND
// RP.PostRank = 1) SELECT C.UserId, C.DisplayName, C.BadgeCount, C.GoldBadges, C.SilverBadges, C.BronzeBadges, C.RecentPostTitle, C.RecentPostDate, C.RecentPostScore FROM
// CombinedData C WHERE (C.BadgeCount > 0 OR C.RecentPostTitle IS NOT NULL) ORDER BY C.BadgeCount DESC, C.RecentPostDate DESC FETCH FIRST 10 ROWS ONLY;
//
// `P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'` compares instants in the session zone (America/New_York).
fn q3309(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let since = ny_to_utc(add_days(utc_to_ny(now_utc()), -30));
    let w = db
        .post
        .with(creation_date.filt(move |d: i64| ny_to_utc(d) >= since))
        .with(owner_user)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(rank, |(_, d): (Id<Post>, i64)| Reverse(d), asc);
    let by_owner = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    type R = ([i64; 4], Option<(Id<Post>, Option<Str>)>);
    let v = drain((&ub).and(by_owner.select(Ident::<Post>::new().and(title.opt())).opt()).filt(|(a, p): R| a[0] > 0 || p.is_some_and(|(_, t)| t.is_some())));
    let v = top_n(v, |&(u, (a, p))| {
        let d = p.map(|(p, _)| creation_date.get(p).unwrap());
        (Reverse(a[0]), d.is_none(), Reverse(d), u, p)
    }, 10);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        match p {
            Some((p, _)) => f.extend(post_fields(db, p, &["title", "created", "score"])),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate
// ASC) AS PostRank FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'), UserReputation AS (SELECT u.Id AS UserId, u.Reputation,
// u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation, u.DisplayName), TopUsers AS (SELECT
// UserId, DisplayName, Reputation, PostsCount, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserReputation WHERE PostsCount > 5), CommentStats AS (SELECT PostId,
// COUNT(*) AS TotalComments, AVG(Score) AS AvgCommentScore FROM Comments GROUP BY PostId) SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, tu.DisplayName AS
// TopUser, tu.Reputation AS UserReputation, cs.TotalComments, cs.AvgCommentScore, CASE WHEN cs.TotalComments IS NULL THEN 'No comments' ELSE 'Has comments' END AS CommentStatus
// FROM RankedPosts rp LEFT JOIN CommentStats cs ON rp.PostId = cs.PostId LEFT JOIN Posts p ON rp.PostId = p.Id LEFT JOIN TopUsers tu ON p.OwnerUserId = tu.UserId WHERE
// rp.PostRank <= 10 ORDER BY rp.CreationDate DESC;
//
// PostRank reads only base columns; ties at the tenth place are broken by the post id.
fn q1428(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d): ((Id<Post>, i64), i64)| (Reverse(s), d, p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let cs = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = Ident::<User>::new().with((&pc).filt(|n| n > 5));
    let mut v = drain((&rp).select((&cs).opt().and(owner_user.select(tu).opt())));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (c, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        match u {
            Some(u) => f.extend(ucols(db, u, &["name", "rep"])),
            None => f.extend([V::Null, V::Null]),
        }
        match c {
            Some(c) => f.extend([V::I(c[0]), avg(c[1], c[0]), V::S("Has comments")]),
            None => f.extend([V::Null, V::Null, V::S("No comments")]),
        }
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U), PostDetail AS (SELECT P.Id
// AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id =
// V.PostId AND V.VoteTypeId IN (8, 9) WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName),
// TopPosts AS (SELECT PD.PostId, PD.Title, PD.CreationDate, PD.ViewCount, PD.Score, PD.OwnerDisplayName, PD.CommentCount, PD.TotalBounty, R.ReputationRank FROM PostDetail PD
// JOIN RankedUsers R ON PD.OwnerDisplayName = R.DisplayName WHERE PD.Score > 10 ORDER BY PD.Score DESC, PD.ViewCount DESC LIMIT 10) SELECT TP.PostId, TP.Title, TP.CreationDate,
// TP.ViewCount, TP.Score, TP.OwnerDisplayName, TP.CommentCount, TP.TotalBounty, CASE WHEN TP.ReputationRank <= 10 THEN 'Top User' ELSE 'Regular User' END AS UserCategory FROM
// TopPosts TP LEFT JOIN PostHistory PH ON TP.PostId = PH.PostId AND PH.PostHistoryTypeId IN (10, 11) WHERE PH.Id IS NULL ORDER BY TP.Score DESC, TP.ViewCount DESC;
//
// `PD.OwnerDisplayName = R.DisplayName` joins by name, so a post appears once per user of that name. ReputationRank is a RANK over all users.
fn q3917(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(current_date(), -1)));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let pd = recent()
        .with(score.gt(10))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(bounty.opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let rw = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let rr: MatSet<(Id<User>, i64)> = (&rw).map(|((u, _), r)| (u, r)).collect();
    let by_name: HashIdx<Str, (Id<User>, i64)> = (&rr).map(|(u, _)| u).select(&db.user.display_name).inv().collect();
    let v = drain((&pd).and(owner_user.select(&db.user.display_name).select(&by_name)));
    let v = top_n(v, |&(p, (_, (u, _)))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, u)
    }, 10);
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).in_v(vec![10, 11])).select(&db.post_history.post).collect();
    type R = (Id<Post>, ([i64; 2], (Id<User>, i64)));
    let tp = rel(v);
    let v = drain((&tp).select(Same::<R>::new().minus(Same::<R>::new().map(|(p, _): R| p).with(&closed))));
    rows(v.into_iter().map(|(_, (p, (a, (_, r))))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if r <= 10 { "Top User" } else { "Regular User" })]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN
// p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, AVG(p.Score) AS AvgScore, SUM(b.Class) AS BadgeCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b
// ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName), RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.CreationDate, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY
// p.CreationDate DESC) AS RecentRank FROM Posts p WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '30 days'), PostVotes AS (SELECT p.Id AS PostId, SUM(CASE
// WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY
// p.Id) SELECT us.UserId, us.DisplayName, us.PostCount, us.PositivePosts, us.NegativePosts, us.AvgScore, us.BadgeCount, rp.PostId AS RecentPostId, rp.CreationDate AS
// RecentPostDate, pv.Upvotes, pv.Downvotes FROM UserStatistics us LEFT JOIN RecentPosts rp ON us.UserId = rp.OwnerUserId AND rp.RecentRank = 1 LEFT JOIN PostVotes pv ON
// rp.PostId = pv.PostId WHERE (us.PostCount > 5 OR us.BadgeCount > 2) AND us.AvgScore IS NOT NULL ORDER BY us.DisplayName ASC;
//
// RecentPosts, PostVotes and the rest of the join are driven for every user; the final ORDER BY has no LIMIT.
fn q2263(db: &'static So) -> String {
    let Post { score, creation_date, owner_user, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (s, c)| match s {
            Some(s) => [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + s, a[3] + 1, a[4] + c.unwrap_or(0), a[5] + c.is_some() as i64],
            None => [a[0], a[1], a[2], a[3], a[4] + c.unwrap_or(0), a[5] + c.is_some() as i64],
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d): (Id<Post>, i64)| Reverse(d), asc);
    let by_owner = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    type A = ([i64; 6], i64);
    let v = drain(
        (&us)
            .and(&np)
            .filt(|(a, n): A| (n > 5 || (a[5] > 0 && a[4] > 2)) && a[3] > 0)
            .and(by_owner.select(Ident::<Post>::new().and(&pv)).opt()),
    );
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), avg(a[2], a[3]), nullable(a[4], a[5])]);
        match r {
            Some((p, v)) => f.extend([V::I(db.post.origid.get(p).unwrap()), V::T(creation_date.get(p).unwrap()), V::I(v[0]), V::I(v[1])]),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score
// DESC) AS RankScore, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate ASC) AS RowAsc FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId), TopPosts AS (SELECT
// rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.RankScore FROM RankedPosts rp WHERE rp.RankScore <= 5), PostVoteSummary AS (SELECT p.Id AS
// PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes FROM Posts p
// LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id) SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, pvs.UpVotes, pvs.DownVotes, CASE WHEN
// tp.RankScore IS NULL THEN 'Unranked' ELSE 'Ranked' END AS RankStatus, CONCAT('Score: ', tp.Score, ', Views: ', tp.ViewCount) AS ScoreViewInfo FROM TopPosts tp LEFT JOIN
// PostVoteSummary pvs ON tp.PostId = pvs.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q2277(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let w = base().group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&cc).and(&pv));
    rows(v.into_iter().map(|(p, (c, a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S("Ranked")]);
        f.push(V::Owned(format!("Score: {s}, Views: {}", view_count.get(p).map_or(String::new(), |w| w.to_string()))));
        row(f)
    }))
}

// Rewritten (rewrites/582.sql): see rewrites/README.md.
// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.CreationDate, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Id, a.Id) AS
// RankByViews, COALESCE(a.OwnerUserId, -1) AS AnswerOwnerId FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 WHERE p.CreationDate >= '2024-10-01
// 12:34:56'::timestamp - INTERVAL '1 year'), TopPosts AS (SELECT rp.Id, rp.Title, rp.ViewCount, rp.RankByViews, rp.AnswerOwnerId FROM RankedPosts rp WHERE rp.RankByViews <= 10),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(v.BountyAmount) AS TotalBountyAwarded, AVG(u.Reputation) AS AvgReputation FROM
// Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName) SELECT tp.Title, tp.ViewCount,
// us.DisplayName, us.TotalPosts, us.TotalBountyAwarded, CONCAT('@', us.DisplayName) AS TwitterHandle, CASE WHEN us.TotalPosts > 20 THEN 'Active' WHEN us.TotalPosts BETWEEN 10
// AND 20 THEN 'Moderately Active' ELSE 'Less Active' END AS ActivityStatus, COALESCE(us.AvgReputation, 0) AS ReputationScore FROM TopPosts tp JOIN UserStatistics us ON us.UserId
// = tp.AnswerOwnerId ORDER BY tp.ViewCount DESC, us.TotalBountyAwarded DESC, tp.Id, us.UserId LIMIT 10 OFFSET 5;
//
// RankByViews numbers the post x answer rows; the rewrite breaks its ties by post and answer id, and the port does the same. AnswerOwnerId is a raw id, and
// its -1 default matches the Community user (Id -1).
fn q582(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, owner_user_id, .. } = &db.post;
    type A = Option<(Id<Post>, Option<i64>)>;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(view_count.opt()).and(answers_of(db).select(Ident::<Post>::new().and(owner_user_id.opt())).opt()))
        .window(row_number, |((p, w), a): ((Id<Post>, Option<i64>), A)| (w.is_none(), Reverse(w), p, a.map(|x| x.0)), asc);
    type R = (Id<Post>, i64);
    let tp = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), a), _)| -> R { (p, a.and_then(|(_, o)| o).unwrap_or(-1)) });
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select(bounty.opt()).opt()))
        .fold([0i64; 4], |a, (r, p)| {
            let b = p.flatten().flatten();
            [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + r, a[3] + 1]
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let owner = Same::<R>::new().map(|(_, o): R| o).select(&uidx).select(Ident::<User>::new().and(&us).and(&np));
    let v = drain(tp.select(Same::<R>::new().and(owner)));
    let v = top_n(v, |&(_, ((p, _), ((u, a), _)))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), a[0] == 0, Reverse(a[1]), db.post.origid.get(p).unwrap(), db.user.origid.get(u).unwrap())
    }, 15);
    rows(v.into_iter().skip(5).map(|(_, ((p, _), ((u, a), n)))| {
        let name = db.user.display_name.get(u).unwrap();
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([V::S(name), V::I(n), nullable(a[1], a[0]), V::Owned(format!("@{name}"))]);
        f.push(V::S(if n > 20 { "Active" } else if n >= 10 { "Moderately Active" } else { "Less Active" }));
        f.push(avg(a[2], a[3]));
        row(f)
    }))
}

// WITH RecentActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE
// WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS WikiPostsCount, SUM(v.BountyAmount) AS TotalBounty,
// RANK() OVER (PARTITION BY u.Id ORDER BY SUM(v.BountyAmount) DESC) AS BountyRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND
// v.VoteTypeId IN (8, 9) WHERE u.Reputation > 1000 AND u.Location IS NOT NULL GROUP BY u.Id, u.DisplayName), TopUsers AS (SELECT UserId, DisplayName, TotalPosts, QuestionsCount,
// AnswersCount, WikiPostsCount, TotalBounty, BountyRank FROM RecentActivity WHERE BountyRank <= 5), UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS
// GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId) SELECT
// tu.DisplayName, tu.TotalPosts, tu.QuestionsCount, tu.AnswersCount, tu.WikiPostsCount, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, tu.TotalBounty FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId ORDER BY tu.TotalBounty DESC, tu.DisplayName
// ASC;
//
// BountyRank partitions by the user, so it is 1 for every user and TopUsers is all of RecentActivity.
fn q22971(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ra = db
        .user
        .with((&db.user.reputation).gt(1000))
        .with(&db.user.location)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(bounty.opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 3..=5) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
            }
            None => a,
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let mut v = drain((&ra).and(&np).and((&ub).opt()));
    v.sort_by_key(|&(u, ((a, _), _))| (a[3] == 0, Reverse(a[4]), db.user.display_name.get(u).unwrap()));
    rows(v.into_iter().map(|(u, ((a, n), b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.push(nullable(a[4], a[3]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT CASE
// WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount, ROW_NUMBER() OVER (ORDER BY COALESCE(SUM(v.BountyAmount), 0) DESC) AS Rank FROM Users u LEFT JOIN Posts p ON u.Id =
// p.OwnerUserId LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId = p.Id GROUP BY u.Id, u.DisplayName, u.Reputation), PostRanked AS (SELECT p.Id AS PostId, p.Title,
// p.CreationDate, p.Score, RANK() OVER (ORDER BY p.Score DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1), TopPosts AS (SELECT pr.PostId, pr.Title, pr.CreationDate,
// pr.Score, us.DisplayName, us.Reputation, us.TotalBounty FROM PostRanked pr JOIN UserStats us ON us.PostCount > 0 WHERE pr.PostRank <= 10) SELECT tp.PostId, tp.Title,
// tp.CreationDate, tp.Score, tp.DisplayName, tp.Reputation, tp.TotalBounty, CASE WHEN tp.TotalBounty > 100 THEN 'High Bounty' WHEN tp.TotalBounty BETWEEN 50 AND 100 THEN 'Medium
// Bounty' ELSE 'Low Bounty' END AS BountyCategory FROM TopPosts tp LEFT JOIN Comments c ON c.PostId = tp.PostId WHERE c.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' -
// INTERVAL '30 days' GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.DisplayName, tp.Reputation, tp.TotalBounty HAVING COUNT(c.Id) > 5 ORDER BY tp.Score DESC,
// tp.CreationDate DESC;
//
// `JOIN UserStats us ON us.PostCount > 0` names only us, so the top questions' recent comments are crossed with every user who has a post, and the product
// is grouped by the SQL's own key (post, name, reputation, bounty), which merges users that agree on all three.
fn q864(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let ov = own_votes(db);
    let bounty = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(ov.select((&db.vote.bounty_amount).opt()).opt()))
        .fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let w = whole(db.post.with(post_type_id.eq(1))).select(Ident::<Post>::new().and(score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let recent = comments_of(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let pc = (&tp).select(Ident::<Post>::new().and(recent));
    let us = db.user.select(Ident::<User>::new().and(&db.user.display_name).and(&db.user.reputation).and(&bounty));
    type X = ((Id<Post>, Id<Comment>), (((Id<User>, Str), i64), i64));
    let g = pc.cross(us).group_by(Same::<X>::new().map(|((p, _), (((_, n), r), b)): X| (p, n, r, b))).select(Same::<X>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&g).filt(|n| n > 5));
    v.sort_by_key(|&((p, ..), _)| (Reverse(score.get(p).unwrap()), Reverse(db.post.creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|((p, name, rep, b), _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::S(name), V::I(rep), V::I(b)]);
        f.push(V::S(if b > 100 { "High Bounty" } else if b >= 50 { "Medium Bounty" } else { "Low Bounty" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN
// v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments FROM Users u LEFT JOIN Posts p ON u.Id =
// p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation), PopularPosts AS (SELECT p.Id AS PostId,
// p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank, p.OwnerUserId FROM Posts p WHERE
// p.AcceptedAnswerId IS NOT NULL), CombinedData AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalUpvotes, us.TotalDownvotes, us.TotalPosts, us.TotalComments,
// pp.PostId, pp.Title, pp.Score, pp.ViewCount FROM UserStats us LEFT JOIN PopularPosts pp ON us.UserId = pp.OwnerUserId AND pp.PostRank <= 3) SELECT cd.DisplayName,
// cd.Reputation, cd.TotalPosts, cd.TotalComments, COALESCE(cd.Title, 'No Posts') AS PostTitle, COALESCE(cd.Score, 0) AS PostScore, COALESCE(cd.ViewCount, 0) AS PostViewCount
// FROM CombinedData cd ORDER BY cd.Reputation DESC, cd.TotalPosts DESC, cd.UserId LIMIT 10;
//
// The vote sums are never read, so only the distinct post and comment counts are computed. The ownerless posts form their own PostRank partition, which
// joins no user, so they are left out before ranking.
fn q3659(db: &'static So) -> String {
    let Post { accepted_answer_id, owner_user, score, view_count, .. } = &db.post;
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = db
        .post
        .with(accepted_answer_id)
        .with(owner_user)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w): ((Id<Post>, i64), Option<i64>)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let pp = (&w).filt(|(_, r)| r <= 3).map(|(((p, _), _), _)| p);
    let v = drain((&np).and(&nc).and(pp.opt()));
    let v = top_n(v, |&(u, ((n, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), db.user.origid.get(u).unwrap()), 10);
    rows(v.into_iter().map(|(u, ((n, c), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(c)]);
        match p {
            Some(p) => f.extend([V::S(db.post.title.get(p).unwrap_or("No Posts")), V::I(score.get(p).unwrap()), V::I(view_count.get(p).unwrap_or(0))]),
            None => f.extend([V::S("No Posts"), V::I(0), V::I(0)]),
        }
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U LEFT JOIN Posts P ON U.Id =
// P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation), ClosedPosts AS (SELECT P.Id AS PostId, P.Title,
// P.CreationDate, H.CreationDate AS CloseDate, C.Name AS CloseReason FROM Posts P JOIN PostHistory H ON P.Id = H.PostId AND H.PostHistoryTypeId = 10 LEFT JOIN CloseReasonTypes C
// ON CAST(H.Comment AS INT) = C.Id), TopUsers AS (SELECT UA.UserId, UA.DisplayName, UA.Reputation, UA.QuestionCount, (UA.UpVotes - UA.DownVotes) AS NetVotes, RANK() OVER (ORDER
// BY (UA.UpVotes - UA.DownVotes) DESC, UA.Reputation DESC) AS VoteRank FROM UserActivity UA WHERE UA.QuestionCount > 5) SELECT TU.DisplayName, TU.Reputation, TU.QuestionCount,
// TU.NetVotes, CP.Title AS ClosedPostTitle, CP.CloseDate, COALESCE(CP.CloseReason, 'No Reason Provided') AS CloseReason FROM TopUsers TU LEFT JOIN ClosedPosts CP ON TU.UserId =
// CP.PostId WHERE TU.VoteRank <= 10 OR CP.CloseReason IS NOT NULL ORDER BY TU.Reputation DESC, TU.NetVotes DESC;
//
// `TU.UserId = CP.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q2366(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let qc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)))).fold(0i64, |n, _| n + 1);
    let w = whole((&qc).filt(|n| n > 5))
        .select(Ident::<User>::new().and(&db.user.reputation).and(&ua).and(&qc))
        .window(rank, |(((_, r), a), _): (((Id<User>, i64), [i64; 2]), i64)| (Reverse(a[0] - a[1]), Reverse(r)), asc);
    let tu: MatSet<(Id<User>, (i64, i64, i64))> = (&w).map(|((((u, _), a), n), r)| (u, (a[0] - a[1], n, r))).collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(post).inv().collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let why = comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason);
    let closed = (&db.user.origid).select(&pidx).select(Ident::<Post>::new().and((&cp).select(Ident::<PostHistory>::new().and(why.opt()))));
    type C = Option<(Id<Post>, (Id<PostHistory>, Option<Str>))>;
    let mut v = drain(by_first(&tu).and(closed.opt()).filt(|((_, _, r), c): ((i64, i64, i64), C)| r <= 10 || c.is_some_and(|c| (c.1).1.is_some())));
    v.sort_by_key(|&(u, ((n, _, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n)));
    rows(v.into_iter().map(|(u, ((net, q, _), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(q), V::I(net)]);
        match c {
            Some((p, (h, r))) => f.extend([ostr(db.post.title.get(p)), V::T(hd.get(h).unwrap()), V::S(r.unwrap_or("No Reason Provided"))]),
            None => f.extend([V::Null, V::Null, V::S("No Reason Provided")]),
        }
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE
// WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId = 2 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 9 GROUP BY U.Id,
// U.DisplayName, U.Reputation), TagStatistics AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Tags T
// LEFT JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%') WHERE P.PostTypeId = 1 GROUP BY T.TagName), TopUsers AS (SELECT UA.UserId, UA.DisplayName, UA.Reputation,
// DENSE_RANK() OVER (ORDER BY UA.TotalPosts DESC) AS PostRank FROM UserActivity UA WHERE UA.TotalPosts > 0), TopTags AS (SELECT TS.TagName, TS.PostCount, DENSE_RANK() OVER
// (ORDER BY TS.PostCount DESC) AS TagRank FROM TagStatistics TS) SELECT TU.DisplayName, TU.Reputation, TT.TagName, TT.PostCount FROM TopUsers TU JOIN TopTags TT ON TU.PostRank
// <= 10 AND TT.TagRank <= 10 ORDER BY TU.Reputation DESC, TT.PostCount DESC FETCH FIRST 50 ROWS ONLY;
//
// `JOIN TopTags TT ON TU.PostRank <= 10 AND TT.TagRank <= 10` names each side separately, so it is a cross join of the two filtered sides.
fn q3863(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9)));
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let uw = whole((&ua).filt(|n| n > 0)).select(Ident::<User>::new().and(&ua)).window(dense_rank, |(_, n): (Id<User>, i64)| Reverse(n), asc);
    let tu = (&uw).filt(|(_, r)| r <= 10).map(|((u, _), _)| u);
    let lt = tag_mentions(db);
    let q = Ident::<Post>::new().with(post_type_id.eq(1));
    let ts = (&lt).group_by((&lt).map(|(_, t)| t).select(&db.tag.tag_name)).select((&lt).map(|(p, _)| p).select(q)).count_distinct();
    let tw = whole(&ts).select(Same::<Str>::new().and(&ts)).window(dense_rank, |(_, n): (Str, i64)| Reverse(n), asc);
    let tt = (&tw).filt(|(_, r)| r <= 10).map(|(x, _)| x);
    let v = drain(tu.cross(tt));
    let v = top_n(v, |&(_, (u, (t, n)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u, t), 50);
    rows(v.into_iter().map(|(_, (u, (t, n)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE
// WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN
// Votes v ON u.Id = v.UserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName), TopUsers AS (SELECT *, RANK() OVER (ORDER BY TotalPosts DESC, TotalQuestions DESC) AS
// ActivityRank FROM UserActivity), RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(c.Count, 0) AS CommentCount, COALESCE(ba.TotalBounty, 0) AS
// BountyCount, p.OwnerUserId FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS Count FROM Comments GROUP BY PostId) c ON p.Id = c.PostId LEFT JOIN (SELECT PostId,
// SUM(BountyAmount) AS TotalBounty FROM Votes WHERE VoteTypeId = 8 GROUP BY PostId) ba ON p.Id = ba.PostId WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL
// '30 days')) SELECT u.DisplayName, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalBounty, rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.BountyCount FROM
// TopUsers u LEFT JOIN RecentPosts rp ON u.UserId = rp.OwnerUserId WHERE u.ActivityRank <= 10 ORDER BY u.TotalPosts DESC, rp.CreationDate DESC;
//
// ActivityRank leads with the distinct post count, so only users with at least the tenth-highest count can rank in the top ten; the posts x votes product is
// driven for those alone, and every other user ranks below all of them.
fn q1474(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(0));
    let np = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nw = whole(&np).select(Ident::<User>::new().and(&np)).window(rank, |(_, n): (Id<User>, i64)| Reverse(n), asc);
    let cand: MatSet<Id<User>> = (&nw).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let ua = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (t, b)| {
            let b = b.flatten();
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.unwrap_or(0)]
        });
    let w = whole(&ua).select(Ident::<User>::new().and((&ua).and(&np))).window(rank, |(_, (a, n)): (Id<User>, ([i64; 3], i64))| (Reverse(n), Reverse(a[0])), asc);
    let tu: MatSet<(Id<User>, ([i64; 3], i64))> = (&w).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let ba = db.vote.with((&db.vote.vote_type_id).eq(8)).group_by(&db.vote.post).select((&db.vote.bounty_amount).opt()).fold([0i64; 2], |a, b| [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0)]);
    let rp = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(Ident::<Post>::new().and((&cc).opt()).and((&ba).opt()));
    let mut v = drain(by_first(&tu).and(rp.opt()));
    v.sort_by_key(|&(_, ((_, n), p))| (Reverse(n), p.is_none(), Reverse(p.map(|((p, _), _)| creation_date.get(p).unwrap()))));
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        match p {
            Some(((p, c), b)) => {
                f.extend(post_fields(db, p, &["id", "title", "created"]));
                f.extend([V::I(c.unwrap_or(0)), V::I(b.map_or(0, |b| if b[0] == 0 { 0 } else { b[1] }))]);
            }
            None => f.extend((0..5).map(|_| V::Null)),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY
// p.CreationDate DESC) AS Rank, COALESCE(u.DisplayName, 'Deleted User') AS OwnerDisplayName FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >=
// cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1), PostVotes AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS
// UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId), ClosedPosts AS (SELECT
// ph.PostId, MIN(ph.CreationDate) AS FirstClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId), PostComments AS (SELECT c.PostId, COUNT(c.Id) AS
// CommentCount FROM Comments c GROUP BY c.PostId) SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.OwnerDisplayName, pv.UpVotes,
// pv.DownVotes, pc.CommentCount, cp.FirstClosedDate, CASE WHEN cp.FirstClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus FROM RankedPosts rp LEFT JOIN PostVotes
// pv ON rp.PostId = pv.PostId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.Rank = 1 ORDER BY rp.CreationDate
// DESC LIMIT 100;
fn q2036(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, owner_user_id, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let pv = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(&db.post_history.creation_date).fold(i64::MAX, |m, d| m.min(d));
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(rp.select(Ident::<Post>::new().and((&pv).opt()).and((&pc).opt()).and((&cp).opt())));
    let v = top_n(v, |&(_, (((p, _), _), _))| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(_, (((p, v), c), d))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::S(owner_user.get(p).map_or("Deleted User", |u| db.user.display_name.get(u).unwrap())));
        match v {
            Some(v) => f.extend(v.map(V::I)),
            None => f.extend([V::Null, V::Null]),
        }
        f.extend([oint(c), ots(d), V::S(if d.is_some() { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(v.BountyAmount) AS TotalBounty, ROW_NUMBER() OVER
// (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND
// v.VoteTypeId = 8 WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId), UserReputation AS
// (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(b.Class, 0)) AS TotalBadges FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT
// JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation), ActiveUsers AS (SELECT ur.UserId, ur.Reputation, ur.PostCount, ur.TotalBadges, ROW_NUMBER() OVER (ORDER BY
// ur.Reputation DESC) AS UserRank FROM UserReputation ur WHERE ur.PostCount > 5) SELECT rp.PostId, rp.Title, rp.CreationDate, ua.UserId, ua.Reputation, rp.CommentCount,
// rp.TotalBounty, ua.PostCount, ua.TotalBadges FROM RankedPosts rp JOIN ActiveUsers ua ON rp.OwnerUserId = ua.UserId WHERE rp.TotalBounty > 0 OR EXISTS (SELECT 1 FROM Comments c
// WHERE c.PostId = rp.PostId AND c.UserId IS NOT NULL) ORDER BY ua.Reputation DESC, rp.CreationDate DESC LIMIT 100;
//
// PostRank and UserRank are never read.
fn q1116(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let rp = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(bounty.opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let active = || db.user.with((&np).filt(|n| n > 5));
    let tb = active().group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold(0i64, |s, (_, c)| s + c.unwrap_or(0));
    let commented: MatSet<Id<Post>> = db.comment.with(&db.comment.user_id).select(&db.comment.post).collect();
    type R = ([i64; 3], Option<Id<Post>>);
    let v = drain(
        (&rp)
            .and(Ident::<Post>::new().with(&commented).opt())
            .filt(|(a, c): R| (a[1] > 0 && a[2] > 0) || c.is_some())
            .and(owner_user.select(Ident::<User>::new().and(&np).and(&tb))),
    );
    let v = top_n(v, |&(p, (_, ((u, _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((a, _), ((u, n), b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::I(n), V::I(b)]);
        row(f)
    }))
}

// WITH RecursivePosts AS (SELECT p.Id, p.Title, p.PostTypeId, p.OwnerUserId, p.CreationDate, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS
// PostRank FROM Posts p), UserPostActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS
// TotalAnswers, MAX(COALESCE(p.CreationDate, '1900-01-01')) AS LatestPostDate FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(ph.Id) AS RevisionCount, SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseReopenCount FROM PostHistory ph
// GROUP BY ph.PostId), TopPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER(ORDER BY p.Score DESC) AS RankByScore, p.OwnerUserId FROM Posts p WHERE
// p.PostTypeId = 1) SELECT u.DisplayName, up.TotalPosts, up.TotalAnswers, up.LatestPostDate, pp.Title AS TopPostTitle, pp.Score AS TopPostScore, phs.RevisionCount,
// phs.CloseReopenCount, CASE WHEN up.LatestPostDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') THEN 'Active' ELSE 'Inactive' END AS ActivityStatus FROM
// UserPostActivity up LEFT JOIN TopPosts pp ON up.UserId = pp.OwnerUserId AND pp.RankByScore = 1 LEFT JOIN PostHistoryStats phs ON pp.Id = phs.PostId JOIN Users u ON up.UserId =
// u.Id WHERE up.TotalPosts > 0 ORDER BY up.TotalAnswers DESC, up.TotalPosts DESC;
//
// RecursivePosts is never referenced. RankByScore numbers all questions together, so `pp.RankByScore = 1` is the one top question overall.
fn q30463(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let upa = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(creation_date))).fold([0, 0, i64::MIN], |a, (t, d)| [a[0] + 1, a[1] + (t == 2) as i64, a[2].max(d)]);
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), p), 1);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pp: HashIdx<Id<User>, Id<Post>> = (&tp).select(owner_user).inv().collect();
    let phs = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + matches!(t, 10 | 11) as i64]);
    let mut v = drain((&upa).and((&pp).select(Ident::<Post>::new().and((&phs).opt())).opt()));
    v.sort_by_key(|&(_, (a, _))| (Reverse(a[1]), Reverse(a[0])));
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::T(a[2])];
        match p {
            Some((p, h)) => {
                f.extend(post_fields(db, p, &["title", "score"]));
                f.extend([oint(h.map(|h| h[0])), oint(h.map(|h| h[1]))]);
            }
            None => f.extend((0..4).map(|_| V::Null)),
        }
        f.push(V::S(if a[2] >= cut { "Active" } else { "Inactive" }));
        row(f)
    }))
}

// WITH TagPostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS
// UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT b.Id) AS BadgeCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER
// BY p.CreationDate DESC) AS UserPostRank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId =
// b.UserId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId), AggregatedData AS (SELECT
// t.TagName, COUNT(DISTINCT t.Id) AS TagCount, AVG(pd.CommentCount) AS AvgComments, SUM(pd.UpVotes) AS TotalUpVotes, SUM(pd.DownVotes) AS TotalDownVotes FROM Tags t JOIN Posts p
// ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN TagPostDetails pd ON pd.PostId = p.Id GROUP BY t.TagName) SELECT a.TagName, a.TagCount, a.AvgComments, a.TotalUpVotes,
// a.TotalDownVotes, CASE WHEN a.TotalUpVotes > a.TotalDownVotes THEN 'Positive' WHEN a.TotalUpVotes < a.TotalDownVotes THEN 'Negative' ELSE 'Neutral' END AS Sentiment,
// ROW_NUMBER() OVER (ORDER BY a.TotalUpVotes DESC) AS Rank FROM AggregatedData a WHERE a.TagCount > 5 ORDER BY a.TotalUpVotes DESC LIMIT 10;
//
// TagCount (COUNT(DISTINCT t.Id) per TagName) is computed first; TagPostDetails is driven only for the posts of the tags that pass `TagCount > 5`.
fn q3853(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, .. } = &db.post;
    let by_uid: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let lt = tag_mentions(db);
    let name = || (&lt).map(|(_, t)| t).select(&db.tag.tag_name);
    let tc = (&lt).group_by(name()).select((&lt).map(|(_, t)| t)).count_distinct();
    let keep: MatSet<(Id<Post>, Id<Tag>)> = (&lt).with(name().select((&tc).filt(|n| n > 5))).collect();
    let posts: MatSet<Id<Post>> = (&keep).map(|(p, _)| p).collect();
    let recent = || (&posts).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pd = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user_id.select(&by_uid).opt()))
        .fold([0i64; 2], |a, ((_, t), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    type K = (Id<Post>, Id<Tag>);
    let kname = Same::<K>::new().map(|(_, t): K| t).select(&db.tag.tag_name);
    let agg = (&keep)
        .group_by(kname)
        .select(Same::<K>::new().map(|(p, _): K| p).select((&pd).and(&cc)).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((v, c)) => [a[0] + 1, a[1] + c, a[2] + v[0], a[3] + v[1]],
            None => a,
        });
    let w = whole((&agg).and(&tc)).select(Same::<Str>::new().and((&agg).and(&tc))).window(row_number, |(t, (a, _)): (Str, ([i64; 4], i64))| (a[0] == 0, Reverse(a[2]), t), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10));
    rows(v.into_iter().map(|(_, ((t, (a, n)), i))| {
        let s = if a[0] == 0 { "Neutral" } else if a[2] > a[3] { "Positive" } else if a[2] < a[3] { "Negative" } else { "Neutral" };
        row(vec![V::S(t), V::I(n), avg(a[1], a[0]), nullable(a[2], a[0]), nullable(a[3], a[0]), V::S(s), V::I(i)])
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1
// ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, SUM(COALESCE(b.Class, 0)) AS TotalBadgeClass, SUM(CASE WHEN
// p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.UserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName), PostStats AS (SELECT p.Id AS PostId, p.Title,
// p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score,
// p.ViewCount), TopUsers AS (SELECT u.UserId, u.DisplayName, u.TotalPosts, u.TotalComments, u.TotalUpVotes, u.TotalDownVotes, RANK() OVER (ORDER BY u.TotalUpVotes DESC) AS
// UserRank FROM UserActivity u WHERE u.TotalPosts > 0) SELECT tu.DisplayName, tu.TotalPosts, tu.TotalUpVotes, ps.Title AS MostCommentedPost, ps.CommentCount FROM TopUsers tu
// JOIN PostStats ps ON tu.UserId = ps.PostId WHERE tu.UserRank <= 10 ORDER BY tu.UserRank, ps.CommentCount DESC;
//
// `tu.UserId = ps.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q9282(db: &'static So) -> String {
    let ov = own_votes(db);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(ov.select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| match p {
            Some((_, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
            None => a,
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole((&np).filt(|n| n > 0)).select(Ident::<User>::new().and(&ua).and(&np)).window(rank, |((_, a), _): ((Id<User>, [i64; 2]), i64)| Reverse(a[0]), asc);
    let tu: MatSet<(Id<User>, (i64, i64, i64))> = (&w).filt(|(_, r)| r <= 10).map(|(((u, a), n), r)| (u, (a[0], n, r))).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let ps = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain(by_first(&tu).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&ps))));
    v.sort_by_key(|&(_, ((_, _, r), (_, c)))| (r, Reverse(c)));
    rows(v.into_iter().map(|(u, ((up, n, _), (p, c)))| row(vec![user_col(db, u, "name"), V::I(n), V::I(up), ostr(db.post.title.get(p)), V::I(c)])))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS
// AnswerCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0
// END) AS BronzeBadges FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName,
// U.Reputation), TopUsers AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation WHERE QuestionCount > 5), PopularPosts AS (SELECT P.Id AS
// PostId, P.Title, P.ViewCount, P.AnswerCount, P.Score, COALESCE(PL.RelatedPostId, 0) AS RelatedPostId, LT.Name AS LinkTypeName FROM Posts P LEFT JOIN PostLinks PL ON P.Id =
// PL.PostId LEFT JOIN LinkTypes LT ON PL.LinkTypeId = LT.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND P.ViewCount > 1000) SELECT
// TU.DisplayName, TU.Reputation, TU.QuestionCount, TU.AnswerCount, TU.GoldBadges, TU.SilverBadges, TU.BronzeBadges, PP.PostId, PP.Title, PP.ViewCount, PP.AnswerCount, PP.Score,
// PP.LinkTypeName FROM TopUsers TU LEFT JOIN PopularPosts PP ON TU.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = PP.PostId) WHERE TU.ReputationRank <= 10 ORDER BY
// TU.Reputation DESC, PP.ViewCount DESC;
//
// ReputationRank reads only Reputation among users with more than five questions, so those users are ranked first and the questions x badges product is
// driven for the top ten alone. The join only admits questions, so AnswerCount is 0.
fn q636(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, .. } = &db.post;
    let q = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let qc = db.user.group_by(Ident::<User>::new()).select(q()).fold(0i64, |n, _| n + 1);
    let w = whole((&qc).filt(|n| n > 5)).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let ur = (&tu)
        .group_by(Ident::<User>::new())
        .select(q().select(post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]);
    let popular = Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(view_count.gt(1000)));
    let pp = posts_of(db).select(popular).select(Ident::<Post>::new().and(links_of(db).select(&db.post_link.link_type).select(&db.link_type.name).opt()));
    let mut v = drain((&ur).and(&qc).and(pp.opt()));
    v.sort_by_key(|&(u, (_, p))| {
        let w = p.and_then(|(p, _)| view_count.get(p));
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        match p {
            Some((p, l)) => {
                f.extend(post_fields(db, p, &["id", "title", "views", "answers", "score"]));
                f.push(ostr(l));
            }
            None => f.extend((0..6).map(|_| V::Null)),
        }
        row(f)
    }))
}

// WITH UserScores AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN
// V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, (COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1
// ELSE 0 END), 0)) AS Score, COUNT(DISTINCT P.Id) AS PostCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id,
// U.DisplayName, U.Reputation), HighScoreUsers AS (SELECT UserId, DisplayName, Reputation, Score, RANK() OVER (ORDER BY Score DESC) AS Rank FROM UserScores WHERE Score > 0),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.Score AS PostScore, P.OwnerUserId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, RANK() OVER (PARTITION BY
// P.OwnerUserId ORDER BY P.Score DESC) AS Rank FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.Score > 0 GROUP BY P.Id, P.Title, P.Score, P.OwnerUserId) SELECT
// U.DisplayName AS User, U.Reputation, HS.Score AS UserScore, TP.Title AS TopPost, TP.PostScore, TP.CommentCount FROM HighScoreUsers HS JOIN Users U ON HS.UserId = U.Id LEFT
// JOIN TopPosts TP ON U.Id = TP.OwnerUserId AND TP.Rank = 1 WHERE HS.Rank <= 10 ORDER BY HS.Score DESC, U.Reputation DESC;
fn q4552(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold(0i64, |n, t| {
        n + (t.flatten() == Some(2)) as i64 - (t.flatten() == Some(3)) as i64
    });
    let hw = whole((&us).filt(|s| s > 0)).select(Ident::<User>::new().and(&us)).window(rank, |(_, s): (Id<User>, i64)| Reverse(s), asc);
    let hs: MatSet<(Id<User>, i64)> = (&hw).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let tw = db.post.with(score.gt(0)).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let tp = (&tw).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain(by_first(&hs).and(tp.select(Ident::<Post>::new().and(&cc)).opt()));
    v.sort_by_key(|&(u, (s, _))| (Reverse(s), Reverse(db.user.reputation.get(u).unwrap())));
    rows(v.into_iter().map(|(u, (s, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(s));
        match p {
            Some((p, c)) => f.extend([ostr(db.post.title.get(p)), V::I(score.get(p).unwrap()), V::I(c)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE((SELECT COUNT(DISTINCT c.Id) FROM Comments c WHERE c.PostId = p.Id), 0) AS
// CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL
// '1 year'), TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(DISTINCT p.Id) AS PostCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 5), UserBadges AS (SELECT b.UserId, COUNT(*) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE
// b.Class = 2) AS SilverBadges, COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Badges b GROUP BY b.UserId) SELECT u.UserId, u.DisplayName, u.TotalScore, ub.GoldBadges,
// ub.SilverBadges, ub.BronzeBadges, p.Title, p.CommentCount, p.CreationDate, CASE WHEN p.CommentCount > 10 THEN 'Highly Engaged' WHEN p.CommentCount BETWEEN 5 AND 10 THEN
// 'Moderately Engaged' ELSE 'Low Engagement' END AS EngagementLevel FROM TopUsers u JOIN RankedPosts p ON u.UserId = p.PostId LEFT JOIN UserBadges ub ON u.UserId = ub.UserId
// WHERE p.RN = 1 ORDER BY u.TotalScore DESC, p.CreationDate DESC LIMIT 100;
//
// `u.UserId = p.PostId` joins a user id to a post id, so it goes through the raw ids. RN = 1 is each owner's newest recent post (the ownerless posts are one partition).
fn q4918(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold([0i64; 2], |a, s| [a[0] + s, a[1] + 1]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let v = drain((&tu).filt(|a: [i64; 2]| a[1] > 5).and((&ub).opt()).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().with(&rp).and(&cc))));
    let v = top_n(v, |&(u, ((a, _), (p, _)))| (Reverse(a[0]), Reverse(creation_date.get(p).unwrap()), u), 100);
    rows(v.into_iter().map(|(u, ((a, b), (p, c)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(a[0]));
        match b {
            Some(b) => f.extend(b.map(V::I)),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(c), V::T(creation_date.get(p).unwrap())]);
        f.push(V::S(if c > 10 { "Highly Engaged" } else if c >= 5 { "Moderately Engaged" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT UserId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(CASE WHEN
// VoteTypeId IN (10, 12) THEN 1 END) AS DeletedVotes FROM Votes GROUP BY UserId), RecentPostActivity AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, P.Score,
// COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount, COALESCE(COUNT(PH.Id), 0) AS EditHistoryCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN PostHistory PH ON P.Id = PH.PostId AND PH.PostHistoryTypeId IN (4, 5, 6) WHERE P.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP
// BY P.Id, P.OwnerUserId, P.Title, P.CreationDate, P.Score), RankedPosts AS (SELECT RPA.*, RANK() OVER (PARTITION BY OwnerUserId ORDER BY Score DESC) AS PostRank FROM
// RecentPostActivity RPA) SELECT UPS.UserId, U.DisplayName, U.Reputation, RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.CommentCount, RP.EditHistoryCount, UPS.UpVotes,
// UPS.DownVotes, UPS.DeletedVotes FROM UserVoteSummary UPS JOIN Users U ON UPS.UserId = U.Id LEFT JOIN RankedPosts RP ON U.Id = RP.OwnerUserId WHERE UPS.UpVotes > UPS.DownVotes
// AND RP.PostRank <= 5 AND U.CreationDate < (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '6 months') ORDER BY U.Reputation DESC, RP.Score DESC;
fn q2726(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])));
    let rpa = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(edits.opt())).fold([0i64; 2], |a, (c, h)| [a[0] + c.is_some() as i64, a[1] + h.is_some() as i64]);
    let w = recent().with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let by_owner = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p);
    let ups = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + matches!(t, 10 | 12) as i64]);
    let v = drain(
        db.user
            .with((&db.user.creation_date).lt(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))
            .select((&ups).filt(|a: [i64; 3]| a[0] > a[1]).and(by_owner.select(Ident::<Post>::new().and(&rpa)))),
    );
    let mut v = v;
    v.sort_by_key(|&(u, (_, (p, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(u, (a, (p, r)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend([V::I(r[0]), V::I(r[1])]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS
// DownVotes, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(P.Score) AS TotalScore FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON P.Id = V.PostId GROUP BY U.Id,
// U.DisplayName), PostDetails AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Score, P.Title, P.CreationDate, COALESCE(PH.EditBodyCount, 0) AS EditCount, COUNT(DISTINCT C.Id) AS
// CommentCount FROM Posts P LEFT JOIN (SELECT PH.PostId, COUNT(*) AS EditBodyCount FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (5, 24) GROUP BY PH.PostId) PH ON P.Id =
// PH.PostId LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY P.Id, P.OwnerUserId, P.Score,
// P.Title, P.CreationDate, PH.EditBodyCount), RankedPosts AS (SELECT PD.*, RANK() OVER (ORDER BY PD.Score DESC) AS Rank FROM PostDetails PD) SELECT U.DisplayName, U.UpVotes,
// U.DownVotes, RP.PostId, RP.Title, RP.Score, RP.EditCount, RP.CommentCount, RP.Rank FROM UserVoteStats U JOIN RankedPosts RP ON U.UserId = RP.OwnerUserId WHERE RP.Rank <= 10
// ORDER BY U.UpVotes DESC, RP.Score DESC;
//
// Rank reads only Score, so the top recent posts are picked first. Only UpVotes and DownVotes of UserVoteStats are read.
fn q7626(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let w = whole(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(Ident::<Post>::new().and(score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let rp: MatSet<(Id<Post>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), r)| (p, r)).collect();
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let eb = db.post_history.with((&db.post_history.post_history_type_id).is_in([5, 24])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain(by_first(&rp).and(owner_user.select(Ident::<User>::new().and((&uvs).opt()))).and((&eb).opt()).and(&cc));
    v.sort_by_key(|&(p, (((_, (_, a)), _), _))| (Reverse(a.map_or(0, |a| a[0])), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (((r, (u, a)), e), c))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1])];
        f.extend(post_fields(db, p, &["id", "title", "score"]));
        f.extend([V::I(e.unwrap_or(0)), V::I(c), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate
// DESC) as rn FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'), UserVotes AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE -1
// END) AS VoteScore FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId), ClosedPosts AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastClosedDate FROM
// PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId) SELECT CONCAT(u.DisplayName, ' (ID: ', u.Id, ')') AS UserDisplayName, rp.PostId, rp.Title, rp.CreationDate,
// COALESCE(uv.VoteScore, 0) AS NetVoteScore, CASE WHEN cp.LastClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus, CASE WHEN rp.Score > 0 AND rp.ViewCount > 100
// THEN 'Popular' ELSE 'Regular' END AS Category, CASE WHEN rp.Score IS NULL THEN 'No Score Available' ELSE CAST(rp.Score AS varchar) END AS ScoreStatus FROM RankedPosts rp LEFT
// JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserVotes uv ON rp.PostId = uv.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.rn = 1 AND (SELECT COUNT(*)
// FROM Comments c WHERE c.PostId = rp.PostId) > 5 ORDER BY NetVoteScore DESC, rp.CreationDate DESC LIMIT 10;
//
// rn = 1 is each owner's newest recent post (the ownerless posts are one partition). CONCAT treats a missing user's NULLs as empty strings.
fn q21199(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, score, view_count, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let uv = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold(0i64, |s, n| s + if n == "UpMod" { 1 } else { -1 });
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(&db.post_history.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&rp).with((&cc).filt(|n| n > 5)).select((&uv).opt().and((&cp).opt())));
    let v = top_n(v, |&(p, (u, _))| (Reverse(u.unwrap_or(0)), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (u, c))| {
        let s = score.get(p).unwrap();
        let w = view_count.get(p);
        let name = match owner_user.get(p) {
            Some(u) => format!("{} (ID: {})", db.user.display_name.get(u).unwrap(), db.user.origid.get(u).unwrap()),
            None => " (ID: )".to_string(),
        };
        let mut f = vec![V::Owned(name), V::I(origid.get(p).unwrap())];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(u.unwrap_or(0)), V::S(if c.is_some() { "Closed" } else { "Open" })]);
        f.push(V::S(if s > 0 && w.map_or(false, |w| w > 100) { "Popular" } else { "Regular" }));
        f.push(V::Owned(s.to_string()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank FROM
// Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE p.CreationDate >= TIMESTAMP '2024-10-01
// 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId, u.DisplayName), StringProcess AS (SELECT rp.PostId, rp.Title,
// rp.OwnerDisplayName, rp.CommentCount, rp.BadgeCount, rp.ViewCount, rp.Score, CASE WHEN rp.PostTypeId = 1 THEN 'Question' WHEN rp.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END
// AS PostType, REPLACE(REPLACE(rp.Title, 'Stack Overflow', 'SO'), 'Help', 'Assistance') AS ProcessedTitle FROM RankedPosts rp WHERE rp.PostRank <= 10) SELECT sp.PostId,
// sp.ProcessedTitle, sp.OwnerDisplayName, sp.CommentCount, sp.BadgeCount, sp.ViewCount, sp.Score, CASE WHEN LENGTH(sp.ProcessedTitle) > 50 THEN 'Long Title' ELSE 'Short Title'
// END AS TitleLengthCategory FROM StringProcess sp ORDER BY sp.Score DESC, sp.ViewCount DESC;
//
// PostRank reads only base columns, so the top ten posts of each type are picked first and the comments x badges product is driven for those alone.
fn q26465(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, title, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d): ((Id<Post>, i64), i64)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(owner_user.select(badges_of(db)).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let bc = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&s).and(&bc));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (c, b))| {
        let t = title.get(p).map(|t| t.replace("Stack Overflow", "SO").replace("Help", "Assistance"));
        let long = t.as_ref().map_or(false, |t| t.chars().count() > 50);
        let mut f = vec![V::I(db.post.origid.get(p).unwrap()), t.map_or(V::Null, V::Owned)];
        f.push(V::S(owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(c), V::I(b)]);
        f.extend(post_fields(db, p, &["views", "score"]));
        f.push(V::S(if long { "Long Title" } else { "Short Title" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, MAX(v.VoteTypeId)
// OVER (PARTITION BY p.Id) AS MaxVoteType FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// FilteredPosts AS (SELECT rp.Id, rp.Title, rp.Score, rp.ViewCount, CASE WHEN rp.MaxVoteType IS NULL THEN 'No Votes' ELSE 'Has Votes' END AS VoteStatus FROM RankedPosts rp WHERE
// rp.Score > 10 AND rp.UserPostRank <= 5), UserStatistics AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN b.Class = 1
// THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT
// JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.Reputation, u.DisplayName) SELECT fs.Title, fs.Score,
// fs.ViewCount, us.DisplayName, us.Reputation, us.TotalPosts, us.GoldBadges, us.SilverBadges, us.BronzeBadges, fs.VoteStatus FROM FilteredPosts fs JOIN UserStatistics us ON
// fs.Id = us.UserId ORDER BY fs.Score DESC, us.Reputation DESC LIMIT 100;
//
// UserPostRank numbers the post x vote rows; the rows of one post tie on CreationDate and agree in every projected column, so only how many of them make the
// cut is observable, and ties are broken by the vote id. `fs.Id = us.UserId` joins a post id to a user id, through the raw ids.
fn q4174(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, .. } = &db.post;
    type R = (Id<Post>, Option<Id<Vote>>);
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date).and(votes_of(db).opt()))
        .window(row_number, |((p, d), x): ((Id<Post>, i64), Option<Id<Vote>>)| (Reverse(d), p, x), asc);
    let fs = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), x), _)| -> R { (p, x) });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let high = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let us = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 3], |a, (_, c)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let hv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let pid = || Same::<R>::new().map(|(p, _): R| p);
    let v = drain(fs.with(pid().with(score.gt(10))).select(Same::<R>::new().and(pid().select(&db.post.origid).select(&uidx).select(high.and(&us).and(&np))).and(pid().select(&hv))));
    let v = top_n(v, |&(_, (((p, x), ((u, _), _)), _))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p, x), 100);
    rows(v.into_iter().map(|(_, (((p, _), ((u, b), n)), h))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(b.map(V::I));
        f.push(V::S(if h > 0 { "Has Votes" } else { "No Votes" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, p.ViewCount, p.AnswerCount, U.Reputation, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER
// BY p.CreationDate DESC) AS rn FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND p.ViewCount IS
// NOT NULL), TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, AnswerCount, Reputation FROM RankedPosts WHERE rn <= 5), PostStats AS (SELECT p.Id AS PostId, SUM(CASE
// WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Votes v
// ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id) SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.AnswerCount,
// ps.UpVotes, ps.DownVotes, (ps.UpVotes - ps.DownVotes) AS NetVotes, COALESCE(ps.CommentCount, 0) AS CommentCount, CASE WHEN tp.Reputation > 1000 THEN 'High Reputation' WHEN
// tp.Reputation BETWEEN 500 AND 1000 THEN 'Moderate Reputation' ELSE 'Low Reputation' END AS ReputationCategory FROM TopPosts tp LEFT JOIN PostStats ps ON tp.PostId = ps.PostId
// ORDER BY tp.ViewCount DESC, tp.CreationDate DESC LIMIT 10;
//
// rn reads only base columns, so the five newest posts of each type are picked first and PostStats is driven for those alone.
fn q23580(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(view_count)
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let ps = (&tp)
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let v = drain((&tp).select((&ps).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(view_count.get(p)), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, s)| {
        let r = db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "answers"]);
        match s {
            Some(a) => f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::I(a[2])]),
            None => f.extend([V::Null, V::Null, V::Null, V::I(0)]),
        }
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Moderate Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate
// DESC) AS RecentRank FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1), TopUsers AS (SELECT u.Id AS UserId,
// u.DisplayName, u.Reputation, u.Views, COUNT(p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END)
// AS DownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views HAVING COUNT(p.Id)
// > 5 ORDER BY UpVotes DESC LIMIT 10) SELECT u.DisplayName, u.Reputation, rp.Title, rp.CreationDate, rp.Score, CASE WHEN rp.ViewCount IS NULL THEN 'No Views Yet' ELSE
// CAST(rp.ViewCount AS VARCHAR) END AS ViewCount, COALESCE(bp.BadgeCount, 0) AS BadgeCount, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount FROM
// RecentPosts rp JOIN TopUsers u ON rp.OwnerUserId = u.UserId LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) bp ON u.UserId = bp.UserId WHERE
// rp.RecentRank = 1 ORDER BY u.Reputation DESC, rp.Score DESC LIMIT 100;
fn q4385(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, p| match p {
            Some(t) => [a[0] + 1, a[1] + (t == Some(2)) as i64],
            None => a,
        });
    let tu = top_n(drain((&tu).filt(|a: [i64; 2]| a[0] > 5)), |&(u, a)| (Reverse(a[1]), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let by_owner = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let bp = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&tu).select(by_owner.select(Ident::<Post>::new().and(&cc)).and((&bp).opt())));
    let v = top_n(v, |&(u, ((p, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), u), 100);
    rows(v.into_iter().map(|(u, ((p, c), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.push(view_count.get(p).map_or(V::S("No Views Yet"), |w| V::Owned(w.to_string())));
        f.extend([V::I(b.unwrap_or(0)), V::I(c)]);
        row(f)
    }))
}

// WITH FilteredPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, p.CreationDate, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN
// v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, COALESCE(COUNT(DISTINCT b.Id), 0) AS
// BadgeCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Badges b ON u.Id =
// b.UserId WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Body, p.Tags, u.DisplayName,
// p.CreationDate), RankedPosts AS (SELECT PostId, Title, Body, Tags, OwnerDisplayName, CreationDate, CommentCount, UpVoteCount, DownVoteCount, BadgeCount, RANK() OVER (ORDER BY
// (UpVoteCount - DownVoteCount) DESC, CommentCount DESC) AS Rank FROM FilteredPosts) SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.OwnerDisplayName, rp.CreationDate,
// rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, rp.BadgeCount, (UPPER(SUBSTRING(rp.Body FROM 1 FOR 30)) || '...') AS ShortenedBody, CASE WHEN rp.BadgeCount > 0 THEN 'Yes'
// ELSE 'No' END AS HasBadges, rp.Rank FROM RankedPosts rp WHERE rp.Rank <= 10 ORDER BY rp.Rank;
fn q26996(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, body, .. } = &db.post;
    let base = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let fp = base()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let bc = base().group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let w = whole(&fp).select(Ident::<Post>::new().and((&fp).and(&bc))).window(rank, |(_, (a, _)): (Id<Post>, ([i64; 3], i64))| (Reverse(a[1] - a[2]), Reverse(a[0])), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10));
    rows(v.into_iter().map(|(_, ((p, (a, b)), r))| {
        let short: String = body.get(p).unwrap().chars().take(30).collect::<String>().to_uppercase();
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "owner", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b), V::Owned(format!("{short}...")), V::S(if b > 0 { "Yes" } else { "No" }), V::I(r)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3
// THEN 1 ELSE 0 END), 0) AS Downvotes, COALESCE(COUNT(DISTINCT P.Id), 0) AS PostCount, COALESCE(SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END), 0) AS TotalScore,
// RANK() OVER (ORDER BY COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) DESC) AS UpvoteRank FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes
// V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName), TopUsers AS (SELECT UA.UserId, UA.DisplayName, UA.Upvotes, UA.Downvotes, UA.PostCount, UA.TotalScore, UA.UpvoteRank FROM
// UserActivity UA WHERE UA.PostCount > 5 AND UA.Upvotes > 10), RelevantTags AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%'
// || T.TagName || '%' GROUP BY T.TagName HAVING COUNT(DISTINCT P.Id) > 10) SELECT U.DisplayName AS TopUser, U.Upvotes, U.Downvotes, T.TagName, T.PostCount, CASE WHEN U.Upvotes >
// U.Downvotes THEN 'Positive Engagement' WHEN U.Upvotes < U.Downvotes THEN 'Negative Engagement' ELSE 'Neutral Engagement' END AS EngagementType FROM TopUsers U JOIN
// RelevantTags T ON U.PostCount > T.PostCount WHERE U.UpvoteRank <= 10 ORDER BY U.Upvotes DESC, T.PostCount DESC;
//
// `JOIN RelevantTags T ON U.PostCount > T.PostCount` is a theta join, driven as a cross join filtered on the counts. UpvoteRank is taken over all users,
// before TopUsers filters them.
fn q442(db: &'static So) -> String {
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, p| match p {
        Some(t) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
        None => a,
    });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    type U = (Id<User>, ([i64; 2], i64));
    let w = whole(&ua).select(Ident::<User>::new().and((&ua).and(&np))).window(rank, |(_, (a, _)): U| Reverse(a[0]), asc);
    let tu = (&w).filt(|((_, (a, n)), r): (U, i64)| n > 5 && a[0] > 10 && r <= 10).map(|(x, _)| x);
    let lt = tag_mentions(db);
    let rt = (&lt).group_by((&lt).map(|(_, t)| t).select(&db.tag.tag_name)).select((&lt).map(|(p, _)| p)).count_distinct();
    let v = drain(tu.cross((&rt).filt(|n| n > 10)).filt(|((_, (_, n)), k): (U, i64)| n > k));
    rows(v.into_iter().map(|((_, t), ((u, (a, _)), k))| {
        let e = if a[0] > a[1] { "Positive Engagement" } else if a[0] < a[1] { "Negative Engagement" } else { "Neutral Engagement" };
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::S(t), V::I(k), V::S(e)])
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE
// WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS
// SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, u.Reputation FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id =
// b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation), TopUsers AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation), PostStatistics AS (SELECT
// p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v
// ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title) SELECT u.DisplayName,
// u.Reputation, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.GoldBadges, u.SilverBadges, u.BronzeBadges, ps.PostId, ps.Title, ps.CommentCount, ps.TotalBounties FROM
// TopUsers u JOIN PostStatistics ps ON u.UserId = ps.PostId WHERE u.Rank <= 10 ORDER BY u.Reputation DESC, ps.CommentCount DESC;
//
// Rank reads only Reputation, so the top users are picked first. `u.UserId = ps.PostId` joins a user id to a post id, through the raw ids.
fn q3472(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let ur = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (t, c)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]);
    let np = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(bounty.opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let mut v = drain((&ur).and(&np).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&ps))));
    v.sort_by_key(|&(u, (_, (_, a)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0])));
    rows(v.into_iter().map(|(u, ((a, n), (p, s)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(s[0]), V::I(s[1])]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.Score, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS
// UserPostsRank FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'), UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE
// WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT B.Id) AS BadgeCount FROM
// Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName), PostDetails AS (SELECT RP.PostId, RP.Title, U.DisplayName AS
// Owner, US.UpVotes, US.DownVotes, U.Reputation, RP.CreationDate FROM RecentPosts RP JOIN Users U ON RP.OwnerUserId = U.Id JOIN UserStats US ON U.Id = US.UserId WHERE
// RP.UserPostsRank <= 5) SELECT PD.PostId, PD.Title, PD.Owner, PD.UpVotes, PD.DownVotes, PD.CreationDate, CASE WHEN PD.UpVotes > PD.DownVotes THEN 'Positive Engagement' WHEN
// PD.UpVotes < PD.DownVotes THEN 'Negative Engagement' ELSE 'Neutral Engagement' END AS EngagementLevel, EXISTS (SELECT 1 FROM Posts P WHERE P.AcceptedAnswerId = PD.PostId) AS
// HasAcceptedAnswer FROM PostDetails PD ORDER BY PD.CreationDate DESC;
//
// UserStats is needed only for the owners of the picked posts, so its votes x badges product is driven for them alone.
fn q2044(db: &'static So) -> String {
    let Post { creation_date, owner_user, origid, accepted_answer_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let owners: MatSet<Id<User>> = (&rp).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let accepted: MatSet<i64> = db.post.select(accepted_answer_id).collect();
    let mut v = drain((&rp).select(owner_user.select(&us).and(origid.with(&accepted).opt())));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (a, x))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::T(creation_date.get(p).unwrap())]);
        f.push(V::S(if a[0] > a[1] { "Positive Engagement" } else if a[0] < a[1] { "Negative Engagement" } else { "Neutral Engagement" }));
        f.push(V::B(x.is_some()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
// FROM Posts p WHERE p.PostTypeId = 1), UserVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0
// END) AS DownVotes FROM Votes v GROUP BY v.PostId), PostHistoryStats AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE
// ph.PostHistoryTypeId IN (4, 5, 24) GROUP BY ph.PostId) SELECT u.DisplayName, up.PostId, up.Title, up.CreationDate, up.Score, COALESCE(uv.UpVotes, 0) AS TotalUpVotes,
// COALESCE(uv.DownVotes, 0) AS TotalDownVotes, COALESCE(ph.EditCount, 0) AS TotalEdits, ph.LastEditDate, COUNT(DISTINCT c.Id) AS CommentCount FROM Users u LEFT JOIN RankedPosts
// up ON u.Id = up.OwnerUserId AND up.rn = 1 LEFT JOIN UserVotes uv ON up.PostId = uv.PostId LEFT JOIN PostHistoryStats ph ON up.PostId = ph.PostId LEFT JOIN Comments c ON
// up.PostId = c.PostId WHERE u.Reputation > 1000 AND (up.Score IS NOT NULL OR up.CreationDate IS NULL) GROUP BY u.DisplayName, up.PostId, up.Title, up.CreationDate, up.Score,
// uv.UpVotes, uv.DownVotes, ph.EditCount, ph.LastEditDate ORDER BY TotalUpVotes DESC, TotalDownVotes ASC LIMIT 50;
//
// The WHERE's `up.Score IS NOT NULL OR up.CreationDate IS NULL` holds for every row. The GROUP BY is by DisplayName and the post, not the user id, so users of
// one name without a newest question share a group.
fn q397(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let by_owner = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    type R = (Str, Option<Id<Post>>);
    let g = db.user.with((&db.user.reputation).gt(1000)).select((&db.user.display_name).and(by_owner.opt())).group_by(Same::<R>::new()).select(Same::<R>::new()).fold(0i64, |n, _| n + 1);
    let uv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.with(post_history_type_id.is_in([4, 5, 24])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pid = || Same::<R>::new().flat_map(|(_, p): R| p);
    let v = drain((&g).map(|_| ()).inv().select(Same::<R>::new().and(pid().select(&uv).opt()).and(pid().select(&ph).opt()).and(pid().select(&cc).opt())));
    let v = top_n(v, |&(_, ((((n, p), u), _), _))| {
        let u = u.unwrap_or([0; 2]);
        (Reverse(u[0]), u[1], n, p)
    }, 50);
    rows(v.into_iter().map(|(_, ((((n, p), u), h), c))| {
        let u = u.unwrap_or([0; 2]);
        let mut f = vec![V::S(n)];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "created", "score"])),
            None => f.extend((0..4).map(|_| V::Null)),
        }
        f.extend([V::I(u[0]), V::I(u[1]), V::I(h.map_or(0, |h| h.0)), ots(h.map(|h| h.1)), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u), RecentPosts AS (SELECT p.Id AS PostId,
// p.OwnerUserId, p.CreationDate, p.Score, p.Title, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.CreationDate >=
// cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'), HighScoringPosts AS (SELECT rp.PostId, rp.OwnerUserId, rp.CreationDate, rp.Score, rp.Title, ru.DisplayName FROM
// RecentPosts rp JOIN RankedUsers ru ON rp.OwnerUserId = ru.Id WHERE rp.Score > 10) SELECT hsp.PostId, hsp.Title AS PostTitle, hsp.Score AS PostScore, hsp.CreationDate,
// hsp.DisplayName AS AuthorName, CASE WHEN EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = hsp.PostId AND v.VoteTypeId = 2) THEN 'Highly Voted' ELSE 'Less Popular' END AS
// PopularityStatus, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = hsp.PostId), 0) AS CommentCount FROM HighScoringPosts hsp LEFT JOIN PostHistory ph ON hsp.PostId =
// ph.PostId AND ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = hsp.PostId AND PostHistoryTypeId IN (10, 11)) WHERE ph.PostHistoryTypeId IS NULL
// ORDER BY hsp.Score DESC, hsp.CreationDate DESC LIMIT 10;
//
// The LEFT JOIN matches the post's history rows at its latest close/reopen date; `ph.PostHistoryTypeId IS NULL` keeps the posts where none matched.
fn q4222(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let hsp = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(score.gt(10)).with(owner_user);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    let up: MatSet<Id<Post>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.post).collect();
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    type R = (Option<Id<PostHistory>>, (Option<Id<Post>>, i64));
    let v = drain(hsp().select(Ident::<Post>::new().and(&md).select(&at).opt().and(Ident::<Post>::new().with(&up).opt().and(&cc))).filt(|(h, _): R| h.is_none()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (_, (u, c)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner"]);
        f.extend([V::S(if u.is_some() { "Highly Voted" } else { "Less Popular" }), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
// FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'), UserReputation AS (SELECT u.Id AS UserId, u.Reputation,
// u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes FROM
// Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation, u.DisplayName), BadgedUsers AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE
// b.Class = 1 GROUP BY b.UserId), PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId) SELECT up.UserId, up.DisplayName,
// up.Reputation, up.Upvotes, up.Downvotes, bu.BadgeCount, rp.PostId, rp.Title, rp.CreationDate, COALESCE(pc.CommentCount, 0) AS CommentCount FROM UserReputation up LEFT JOIN
// BadgedUsers bu ON up.UserId = bu.UserId LEFT JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId AND rp.UserPostRank = 1 LEFT JOIN PostComments pc ON rp.PostId = pc.PostId WHERE
// up.Reputation > 1000 AND (bu.BadgeCount IS NULL OR bu.BadgeCount > 2) ORDER BY up.Reputation DESC, rp.CreationDate ASC;
fn q20726(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let ur = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let bu = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let by_owner = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&ur).and((&bu).opt().filt(|b: Option<i64>| b.map_or(true, |b| b > 2))).and(by_owner.select(Ident::<Post>::new().and((&pc).opt())).opt()));
    let mut v = v;
    v.sort_by_key(|&(u, (_, p))| {
        let d = p.map(|(p, _)| creation_date.get(p).unwrap());
        (Reverse(db.user.reputation.get(u).unwrap()), d.is_none(), d)
    });
    rows(v.into_iter().map(|(u, ((a, b), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), oint(b)]);
        match p {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["id", "title", "created"]));
                f.push(V::I(c.unwrap_or(0)));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::I(0)]),
        }
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, LastAccessDate, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM Users WHERE Reputation IS NOT NULL), RecentPosts AS
// (SELECT p.OwnerUserId, p.Id AS PostId, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes FROM Posts p LEFT JOIN
// Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) WHERE p.CreationDate > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month')
// GROUP BY p.OwnerUserId, p.Id, p.CreationDate), TopPostAuthors AS (SELECT ur.DisplayName, ur.Reputation, COUNT(rp.PostId) AS TotalPosts, SUM(rp.CommentCount) AS TotalComments,
// SUM(rp.UpVotes) AS TotalUpVotes FROM RecentPosts rp JOIN Users ur ON rp.OwnerUserId = ur.Id GROUP BY ur.DisplayName, ur.Reputation), CombinedData AS (SELECT u.Id AS UserId,
// u.DisplayName, u.Reputation, COALESCE(tpa.TotalPosts, 0) AS TotalPosts, COALESCE(tpa.TotalComments, 0) AS TotalComments, COALESCE(tpa.TotalUpVotes, 0) AS TotalUpVotes FROM
// Users u LEFT JOIN TopPostAuthors tpa ON u.DisplayName = tpa.DisplayName) SELECT cd.UserId, cd.DisplayName, cd.Reputation, cd.TotalPosts, cd.TotalComments, cd.TotalUpVotes,
// CASE WHEN cd.Reputation > 1000 THEN 'High Reputation' WHEN cd.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory FROM
// CombinedData cd WHERE cd.Reputation IS NOT NULL ORDER BY cd.Reputation DESC LIMIT 10;
//
// UserReputation is never referenced. `u.DisplayName = tpa.DisplayName` joins by name, so a user meets every (name, reputation) group of that name.
fn q2670(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let votes = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select(&db.vote.vote_type_id);
    let rp = db
        .post
        .with(creation_date.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes.opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let tpa = db
        .post
        .with(&rp)
        .group_by(owner_user.select((&db.user.display_name).and(&db.user.reputation)))
        .select(&rp)
        .fold([0i64; 3], |a, r| [a[0] + 1, a[1] + r[0], a[2] + r[1]]);
    let tv: MatSet<((Str, i64), [i64; 3])> = whole(&tpa).select(Same::<(Str, i64)>::new().and(&tpa)).collect();
    let by_name: HashIdx<Str, ((Str, i64), [i64; 3])> = (&tv).map(|((n, _), _)| n).inv().select(&tv).collect();
    let v = drain(db.user.select((&db.user.display_name).select((&by_name).map(|(_, a)| a)).opt()));
    let v = top_n(v, |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), u, a.map(|a| a[0])), 10);
    rows(v.into_iter().map(|(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let a = a.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RECURSIVE UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COALESCE(SUM(CASE WHEN
// v.VoteTypeId IN (2, 4) THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes FROM Users u LEFT JOIN Votes v ON u.Id =
// v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate), PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId
// = 1 THEN p.Id END) AS Questions, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS Answers, AVG(p.ViewCount) AS AvgViews, COUNT(DISTINCT CASE WHEN p.AcceptedAnswerId
// IS NOT NULL THEN p.AcceptedAnswerId END) AS AcceptedAnswers FROM Posts p GROUP BY p.OwnerUserId), UserRanking AS (SELECT up.UserId, up.DisplayName, up.Reputation,
// ps.TotalPosts, ps.Questions, ps.Answers, up.TotalBounty, up.Upvotes, up.Downvotes, ps.AcceptedAnswers, RANK() OVER (ORDER BY up.Reputation DESC, ps.TotalPosts DESC) AS Rank
// FROM UserPerformance up LEFT JOIN PostStatistics ps ON up.UserId = ps.OwnerUserId) SELECT ur.Rank, ur.DisplayName, ur.Reputation, ur.TotalPosts, ur.Questions, ur.Answers,
// ur.TotalBounty, ur.Upvotes, ur.Downvotes, ur.AcceptedAnswers FROM UserRanking ur WHERE ur.Rank <= 10 ORDER BY ur.Rank;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q31537(db: &'static So) -> String {
    let Post { post_type_id, owner_user, accepted_answer_id, .. } = &db.post;
    let up = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).fold([0i64; 3], |a, v| match v {
        Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + matches!(t, 2 | 4) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let ps = db.post.group_by(owner_user).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let acc = db.post.group_by(owner_user).select(accepted_answer_id).count_distinct();
    type X = ([i64; 3], Option<[i64; 3]>);
    let w = whole(&up)
        .select(Ident::<User>::new().and(&db.user.reputation).and((&up).and((&ps).opt())))
        .window(rank, |((_, r), (_, p)): ((Id<User>, i64), X)| (Reverse(r), p.is_none(), Reverse(p.map(|p| p[0]))), asc);
    let tu: MatSet<(Id<User>, (X, i64))> = (&w).filt(|(_, r)| r <= 10).map(|(((u, _), x), r)| (u, (x, r))).collect();
    let v = drain(by_first(&tu).and((&acc).opt()));
    rows(v.into_iter().map(|(u, (((a, p), r), n))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        match p {
            Some(p) => f.extend(p.map(V::I)),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.extend(a.map(V::I));
        f.push(match p {
            Some(_) => V::I(n.unwrap_or(0)),
            None => V::Null,
        });
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id), PostStats AS (SELECT
// p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(p.Score) AS AverageScore FROM Posts p GROUP BY p.OwnerUserId), ClosedPostHistory AS (SELECT p.Id AS PostId, p.Title, ph.UserId AS EditorId, ph.CreationDate AS ClosedDate,
// ph.Comment AS CloseReason, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS rn FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE
// ph.PostHistoryTypeId = 10) SELECT u.DisplayName, u.Reputation, COALESCE(ub.BadgeCount, 0) AS TotalBadges, COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.QuestionCount,
// 0) AS QuestionCount, COALESCE(ps.AnswerCount, 0) AS AnswerCount, COALESCE(ps.AverageScore, 0) AS AverageScore, cph.Title AS ClosedPostTitle, cph.ClosedDate, cph.CloseReason
// FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN ClosedPostHistory cph ON u.Id = cph.EditorId AND cph.rn
// = 1 WHERE u.Reputation > 500 AND (cph.ClosedDate IS NULL OR cph.ClosedDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') ORDER BY u.Reputation DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. rn = 1 is each post's latest close row; a tie on its CreationDate is broken by the history id.
fn q31641(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let PostHistory { post, post_history_type_id, creation_date: hd, user, .. } = &db.post_history;
    let w = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d): (Id<PostHistory>, i64)| (Reverse(d), h), asc);
    let cph: MatSet<(Id<PostHistory>, i64)> = (&w).filt(|(_, r)| r == 1).map(|(x, _)| x).collect();
    let by_editor: HashIdx<Id<User>, (Id<PostHistory>, i64)> = (&cph).map(|(h, _)| h).select(user).inv().collect();
    let cut = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    type R = ((Option<i64>, Option<[i64; 4]>), Option<(Id<PostHistory>, i64)>);
    let v = drain(
        db.user
            .with((&db.user.reputation).gt(500))
            .select((&ub).opt().and((&ps).opt()).and((&by_editor).opt()))
            .filt(move |(_, h): R| h.map_or(true, |(_, d)| d > cut)),
    );
    rows(v.into_iter().map(|(u, ((b, p), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b.unwrap_or(0)));
        match p {
            Some(p) => f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), avg(p[3], p[0])]),
            None => f.extend([V::I(0), V::I(0), V::I(0), V::F(0.0)]),
        }
        match h {
            Some((h, d)) => f.extend([ostr(db.post.title.get(post.get(h).unwrap())), V::T(d), ostr(db.post_history.comment.get(h))]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// Rewritten (rewrites/96.sql): see rewrites/README.md.
// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY
// P.CreationDate DESC, P.Id) AS rn FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND
// P.Score > 0), TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(V.BountyAmount) AS TotalBounty FROM Users U LEFT JOIN Posts P ON U.Id =
// P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 GROUP BY U.Id, U.DisplayName HAVING COUNT(P.Id) > 10), ActiveUserPosts AS (SELECT RP.PostId, RP.Title,
// RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, TU.PostCount, TU.TotalBounty FROM RankedPosts RP JOIN TopUsers TU ON RP.OwnerDisplayName = TU.DisplayName WHERE
// RP.rn = 1) SELECT AUP.PostId, AUP.Title, AUP.CreationDate, AUP.Score, COALESCE(AUP.ViewCount, 0) AS ViewCount, AUP.OwnerDisplayName, AUP.PostCount, AUP.TotalBounty, CASE WHEN
// AUP.TotalBounty > 0 THEN 'High Value' ELSE 'Normal' END AS ValueCategory FROM ActiveUserPosts AUP LEFT JOIN Tags T ON AUP.Title ILIKE '%' || T.TagName || '%' WHERE T.TagName
// IS NOT NULL ORDER BY AUP.Score DESC, AUP.ViewCount DESC, AUP.PostId, T.Id LIMIT 100;
//
// `RP.OwnerDisplayName = TU.DisplayName` joins by name. The ILIKE runs over the distinct titles of the picked posts against every tag name.
fn q96(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, title, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold([0i64; 3], |a, p| match p {
        Some(b) => {
            let b = b.flatten();
            [a[0] + 1, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        }
        None => a,
    });
    let tv: MatSet<(Id<User>, [i64; 3])> = db.user.select(Ident::<User>::new().and((&tu).filt(|a: [i64; 3]| a[0] > 10))).collect();
    let by_name: HashIdx<Str, (Id<User>, [i64; 3])> = (&tv).map(|(u, _)| u).select(&db.user.display_name).inv().collect();
    let titles: MatSet<Str> = (&rp).select(title).collect();
    let hit: HashIdx<Str, Id<Tag>> = (&titles).select_where((&db.tag.tag_name).inv(), |t: Str, n: Str| t.to_lowercase().contains(&n.to_lowercase())).collect();
    let v = drain((&rp).select(owner_user.select(&db.user.display_name).select(&by_name).and(title.select(&hit))));
    let v = top_n(v, |&(p, (_, t))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), db.post.origid.get(p).unwrap(), db.tag.origid.get(t).unwrap())
    }, 100);
    rows(v.into_iter().map(|(p, ((_, a), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(view_count.get(p).unwrap_or(0))]);
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::S(if a[1] > 0 && a[2] > 0 { "High Value" } else { "Normal" })]);
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT U.Id AS UserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2
// THEN 1 ELSE 0 END) AS AnswerCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id), RecentVotes AS (SELECT V.UserId, COUNT(V.Id) AS VoteCount, SUM(CASE
// WHEN VT.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VT.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN VT.Name IS NULL THEN 1 ELSE 0 END) AS
// NullVotes FROM Votes V JOIN VoteTypes VT ON V.VoteTypeId = VT.Id WHERE V.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY V.UserId),
// CombinedData AS (SELECT U.DisplayName, COALESCE(UPC.PostCount, 0) AS TotalPosts, COALESCE(UPC.QuestionCount, 0) AS TotalQuestions, COALESCE(UPC.AnswerCount, 0) AS
// TotalAnswers, COALESCE(RV.VoteCount, 0) AS TotalVotes, COALESCE(RV.UpVotes, 0) AS UpVoteCount, COALESCE(RV.DownVotes, 0) AS DownVoteCount, COALESCE(RV.NullVotes, 0) AS
// NullVoteCount FROM Users U LEFT JOIN UserPostCounts UPC ON U.Id = UPC.UserId LEFT JOIN RecentVotes RV ON U.Id = RV.UserId) SELECT CD.DisplayName, CD.TotalPosts,
// CD.TotalQuestions, CD.TotalAnswers, CD.TotalVotes, CD.UpVoteCount, CD.DownVoteCount, CD.NullVoteCount, ROW_NUMBER() OVER (ORDER BY CD.TotalPosts DESC) AS Rank FROM
// CombinedData CD WHERE CD.TotalPosts > 0 ORDER BY CD.TotalPosts DESC, CD.TotalVotes DESC LIMIT 10;
fn q1439(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let upc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let rv = db
        .vote
        .with((&db.vote.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(&db.vote.user)
        .select(vtype_name(db))
        .fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    type X = ([i64; 3], Option<[i64; 3]>);
    let w = whole(&upc).select(Ident::<User>::new().and((&upc).and((&rv).opt()))).window(row_number, |(u, (a, r)): (Id<User>, X)| (Reverse(a[0]), Reverse(r.map_or(0, |r| r[0])), u), asc);
    let v = top_n(drain(&w), |&(_, ((u, (a, r)), _))| (Reverse(a[0]), Reverse(r.map_or(0, |r| r[0])), u), 10);
    rows(v.into_iter().map(|(_, ((u, (a, r)), i))| {
        let r = r.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(r.map(V::I));
        f.extend([V::I(0), V::I(i)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT CASE WHEN c.Id IS NOT NULL THEN c.Id END)
// AS CommentCount, COUNT(DISTINCT CASE WHEN v.Id IS NOT NULL THEN v.Id END) AS VoteCount, AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AverageUpVotes, AVG(CASE WHEN
// v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AverageDownVotes FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON
// p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName),
// Ranking AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, CommentCount, VoteCount, AverageUpVotes, AverageDownVotes, RANK() OVER (ORDER BY VoteCount DESC, CommentCount
// DESC) AS PostRank FROM PostStats) SELECT r.PostId, r.Title, r.CreationDate, r.OwnerDisplayName, r.CommentCount, r.VoteCount, r.AverageUpVotes, r.AverageDownVotes, r.PostRank,
// pt.Name AS PostType, COUNT(DISTINCT b.Id) AS BadgeCount FROM Ranking r JOIN PostTypes pt ON r.PostId = r.PostId LEFT JOIN Badges b ON r.PostId = b.UserId GROUP BY r.PostId,
// r.Title, r.CreationDate, r.OwnerDisplayName, r.CommentCount, r.VoteCount, r.AverageUpVotes, r.AverageDownVotes, r.PostRank, pt.Name ORDER BY r.PostRank;
//
// `JOIN PostTypes pt ON r.PostId = r.PostId` names only r, so every post is crossed with every post type. `r.PostId = b.UserId` joins a post id to a user
// id, through the raw ids.
fn q7483(db: &'static So) -> String {
    let Post { creation_date, origid, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ps = base()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (_, t)| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let cc = base().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = base().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let w = whole(&ps).select(Ident::<Post>::new().and((&ps).and(&cc).and(&vc))).window(rank, |(_, ((_, c), n)): (Id<Post>, (([i64; 3], i64), i64))| (Reverse(n), Reverse(c)), asc);
    let bc = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    type R = ((Id<Post>, (([i64; 3], i64), i64)), i64);
    let rb = (&w).select(Same::<R>::new().and(Same::<R>::new().map(|((p, _), _): R| p).select(origid).select(&bc).opt()));
    let v = drain(rb.cross(&db.post_type.name));
    rows(v.into_iter().map(|(_, ((((p, ((a, c), n)), r), b), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(n), avg(a[1], a[0]), avg(a[2], a[0]), V::I(r), V::S(t), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.UpVotes, 0)) AS TotalUpVotes, SUM(COALESCE(v.DownVotes, 0)) AS
// TotalDownVotes, AVG(COALESCE(p.Score, 0)) AS AvgPostScore, DENSE_RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS UserRank FROM Users u LEFT JOIN Posts p ON u.Id =
// p.OwnerUserId LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes
// GROUP BY PostId) v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName), TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalUpVotes, TotalDownVotes, AvgPostScore FROM
// UserEngagement WHERE UserRank <= 10), PostDetails AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, pct.PostedBy AS UserId FROM
// Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId JOIN (SELECT OwnerUserId AS PostedBy, Id FROM Posts WHERE
// PostTypeId = 1 AND CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')) pct ON p.OwnerUserId = pct.PostedBy) SELECT ue.DisplayName, pd.Title, pd.ViewCount,
// pd.CommentCount, ue.TotalUpVotes - ue.TotalDownVotes AS NetVotes, ue.AvgPostScore FROM TopUsers ue JOIN PostDetails pd ON ue.UserId = pd.UserId ORDER BY NetVotes DESC,
// pd.ViewCount DESC;
//
// UserRank reads only the distinct post count, so the top users are picked first. PostDetails joins every post of a user to each of their recent questions.
fn q1965(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&np).select(Ident::<User>::new().and(&np)).window(dense_rank, |(_, n): (Id<User>, i64)| Reverse(n), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ue = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(score.and((&pv).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((s, v)) => {
            let v = v.unwrap_or([0; 2]);
            [a[0] + v[0], a[1] + v[1], a[2] + s, a[3] + 1]
        }
        None => [a[0], a[1], a[2], a[3] + 1],
    });
    let pct = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))));
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&ue).and(posts_of(db).select(Ident::<Post>::new().and((&cc).opt()))).and(pct));
    v.sort_by_key(|&(_, ((a, (p, _)), _))| (Reverse(a[0] - a[1]), Reverse(db.post.view_count.get(p))));
    rows(v.into_iter().map(|(u, ((a, (p, c)), _))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(a[0] - a[1]), avg(a[2], a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
// COALESCE(u.DisplayName, 'Anonymous') AS UserDisplayName FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' -
// INTERVAL '1 year'), PostScoreSummary AS (SELECT PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS
// DownVotes FROM Votes v GROUP BY PostId), PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ps.UpVotes, ps.DownVotes, rp.UserDisplayName FROM
// RankedPosts rp LEFT JOIN PostScoreSummary ps ON rp.PostId = ps.PostId WHERE rp.rn = 1) SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.UpVotes,
// pd.DownVotes, pd.UserDisplayName, CASE WHEN pd.UpVotes IS NULL OR pd.DownVotes IS NULL THEN 'Vote data unavailable' ELSE CASE WHEN pd.UpVotes > pd.DownVotes THEN 'Positive'
// WHEN pd.UpVotes < pd.DownVotes THEN 'Negative' ELSE 'Neutral' END END AS VoteSentiment FROM PostDetails pd WHERE pd.ViewCount > (SELECT AVG(ViewCount) FROM Posts) AND
// (pd.Score > 0 OR pd.UpVotes IS NOT NULL) ORDER BY pd.Score DESC, pd.ViewCount DESC LIMIT 50;
//
// `pd.ViewCount > AVG(ViewCount)` is compared exactly, as `ViewCount * n > sum`.
fn q3394(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, score, view_count, .. } = &db.post;
    let (sum, n) = db.post.select(view_count).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let pss = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    type R = ((Id<Post>, i64), Option<[i64; 2]>);
    let v = drain(
        (&rp)
            .with(view_count.filt(move |w: i64| w * n > sum))
            .select(Ident::<Post>::new().and(score).and((&pss).opt()))
            .filt(|((_, s), v): R| s > 0 || v.is_some()),
    );
    let v = top_n(v, |&(_, ((p, s), _))| (Reverse(s), Reverse(view_count.get(p)), p), 50);
    rows(v.into_iter().map(|(_, ((p, _), s))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        match s {
            Some(a) => f.extend([V::I(a[0]), V::I(a[1])]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(V::S(owner_user.get(p).map_or("Anonymous", |u| db.user.display_name.get(u).unwrap())));
        f.push(V::S(match s {
            None => "Vote data unavailable",
            Some(a) if a[0] > a[1] => "Positive",
            Some(a) if a[0] < a[1] => "Negative",
            _ => "Neutral",
        }));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId =
// 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT P.Id) AS PostCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id GROUP BY U.Id,
// U.DisplayName), PostSummary AS (SELECT P.Id AS PostId, P.Title, CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 'Answered' ELSE 'Unanswered' END AS Status, COALESCE(SUM(CASE
// WHEN C.PostId IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id, P.Title, P.AcceptedAnswerId),
// ClosedPostHistory AS (SELECT PH.PostId, PH.CreationDate, PH.Comment AS CloseReason, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS rn FROM
// PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11)) SELECT U.UserId, U.DisplayName, U.UpVotes, U.DownVotes, U.PostCount, P.PostId, P.Title, P.Status, P.CommentCount,
// COALESCE(CP.CloseReason, 'Not Closed') AS LastCloseReason FROM UserVoteSummary U LEFT JOIN PostSummary P ON U.UserId = P.PostId LEFT JOIN ClosedPostHistory CP ON P.PostId =
// CP.PostId AND CP.rn = 1 WHERE U.PostCount > 0 AND (U.UpVotes - U.DownVotes) > 5 ORDER BY U.PostCount DESC, P.CommentCount DESC LIMIT 50;
//
// `U.UserId = P.PostId` joins a user id to a post id, so it goes through the raw ids. rn = 1 is each post's latest close/reopen row, ties broken by the history id.
fn q21555(db: &'static So) -> String {
    let Post { accepted_answer_id, .. } = &db.post;
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.post).opt())).opt()).fold([0i64; 2], |a, v| match v {
        Some((t, _)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64],
        None => a,
    });
    let pc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.post)).count_distinct();
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let w = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d): (Id<PostHistory>, i64)| (Reverse(d), h), asc);
    let last = (&w).filt(|(_, r)| r == 1).map(|((h, _), _)| h);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    type A = ([i64; 2], i64);
    let v = drain(
        (&uvs)
            .and(&pc)
            .filt(|(a, n): A| n > 0 && a[0] - a[1] > 5)
            .and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&cc).and(last.opt())).opt()),
    );
    let v = top_n(v, |&(u, ((_, n), p))| (Reverse(n), p.is_none(), Reverse(p.map(|((_, c), _)| c)), u), 50);
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        match p {
            Some(((p, c), h)) => {
                f.extend(post_fields(db, p, &["id", "title"]));
                f.extend([V::S(if accepted_answer_id.get(p).is_some() { "Answered" } else { "Unanswered" }), V::I(c)]);
                f.push(V::S(h.and_then(|h| comment.get(h)).unwrap_or("Not Closed")));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::S("Not Closed")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS
// UserPostRank FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR'), UserStats AS (SELECT u.Id AS UserId, u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(b.Class, 0)) AS TotalBadges, AVG(COALESCE(v.VoteTypeId, 0)) AS AverageVoteType FROM
// Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName), PostHistoryInfo AS
// (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph
// GROUP BY ph.PostId) SELECT us.UserId, us.DisplayName, us.TotalPosts, us.TotalScore, us.TotalBadges, us.AverageVoteType, pp.Title, pp.UserPostRank, COALESCE(phi.EditCount, 0)
// AS EditCount, COALESCE(phi.CloseCount, 0) AS CloseCount, phi.LastEditDate FROM UserStats us JOIN RankedPosts pp ON us.UserId = pp.OwnerUserId LEFT JOIN PostHistoryInfo phi ON
// pp.PostId = phi.PostId WHERE us.TotalPosts > 5 AND us.TotalScore > 50 ORDER BY us.TotalScore DESC, pp.UserPostRank LIMIT 10;
//
// The ownerless posts form their own UserPostRank partition, which joins no user, so they are left out before ranking.
fn q905(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (p, c)| {
            let (s, t) = p.map_or((0, None), |(s, t)| (s, t));
            [a[0] + s, a[1] + c.unwrap_or(0), a[2] + t.unwrap_or(0), a[3] + 1]
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let by_owner = (&w).map(|((p, _), r)| (p, r));
    let phi = db.post_history.group_by(&db.post_history.post).select((&db.post_history.post_history_type_id).and(&db.post_history.creation_date)).fold([0, 0, i64::MIN], |a, (t, d)| {
        [a[0] + 1, a[1].max((t == 10) as i64), a[2].max(d)]
    });
    type A = ([i64; 4], i64);
    let v = drain((&us).and(&np).filt(|(a, n): A| n > 5 && a[0] > 50).and(by_owner.select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(p, _): (Id<Post>, i64)| p).select(&phi).opt()))));
    let v = top_n(v, |&(u, ((a, _), ((_, r), _)))| (Reverse(a[0]), r, u), 10);
    rows(v.into_iter().map(|(u, ((a, n), ((p, r), h)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), avg(a[2], a[3])]);
        f.extend(post_fields(db, p, &["title"]));
        f.push(V::I(r));
        match h {
            Some(h) => f.extend([V::I(h[0]), V::I(h[1]), V::T(h[2])]),
            None => f.extend([V::I(0), V::I(0), V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER
// BY P.CreationDate DESC) AS rn FROM Posts P WHERE P.PostTypeId = 1 AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'), UserStats AS (SELECT
// U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(V.BountyAmount) AS TotalBounty, COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
// COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges FROM Users U LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId) SELECT U.DisplayName AS User, U.TotalPosts, U.TotalBounty, U.GoldBadges,
// U.SilverBadges, U.BronzeBadges, R.Title, R.CreationDate, R.Score, R.ViewCount, COALESCE(PC.CommentCount, 0) AS TotalComments FROM UserStats U JOIN RankedPosts R ON U.UserId =
// R.PostId LEFT JOIN PostComments PC ON R.PostId = PC.PostId WHERE R.rn = 1 ORDER BY U.TotalBounty DESC, R.Score DESC LIMIT 10;
//
// `U.UserId = R.PostId` joins a user id to a post id, through the raw ids; UserStats is driven only for the users that join.
fn q3737(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user_id, score, view_count, origid, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let by_user: HashIdx<Id<User>, Id<Post>> = (&rp).select(origid.select(&uidx)).inv().collect();
    let cand: MatSet<Id<User>> = (&rp).select(origid.select(&uidx)).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(bounty.opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (p, c)| {
            let b = p.flatten().flatten();
            [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
        });
    let np = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&us).and(&np).and((&by_user).select(Ident::<Post>::new().and((&cc).opt()))));
    let v = top_n(v, |&(u, ((a, _), (p, _)))| (a[0] == 0, Reverse(a[1]), Reverse(score.get(p).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, ((a, n), (p, c)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([oint(view_count.get(p)), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// Rewritten (rewrites/4253.sql): see rewrites/README.md.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id) AS Rank
// FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01' AS DATE) - INTERVAL '1 year'), RecentPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount,
// COALESCE(ph.CreationDate, CAST('1900-01-01' AS TIMESTAMP)) AS LastHistoryDate, CASE WHEN ph.Comment IS NULL THEN 'No comments' ELSE ph.Comment END AS LastComment FROM
// RankedPosts rp LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId AND ph.CreationDate = (SELECT MAX(ph2.CreationDate) FROM PostHistory ph2 WHERE ph2.PostId = rp.PostId) WHERE
// rp.Rank <= 5), UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1
// ELSE 0 END) AS DownVotes, COUNT(c.Id) AS CommentCount FROM Users u LEFT JOIN Votes v ON v.UserId = u.Id LEFT JOIN Comments c ON c.UserId = u.Id GROUP BY u.Id, u.DisplayName)
// SELECT rp.Title, rp.Score, rp.ViewCount, ue.DisplayName, ue.UpVotes, ue.DownVotes, rp.LastHistoryDate, rp.LastComment FROM RecentPosts rp JOIN UserEngagement ue ON ue.UserId =
// (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) ORDER BY rp.Score DESC, rp.ViewCount DESC, rp.PostId, rp.LastComment LIMIT 10;
//
// The UserEngagement votes x comments product is driven only for the owners of the picked posts.
fn q4253(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s): (Id<Post>, i64)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let PostHistory { post, creation_date: hd, comment, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ue = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(comments_by(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&tp).select(Ident::<Post>::new().and(&md).select(&at).opt().and(owner_user.select(Ident::<User>::new().and(&ue)))));
    let v = top_n(v, |&(p, (h, _))| {
        let w = view_count.get(p);
        let c = h.and_then(|h| comment.get(h)).unwrap_or("No comments");
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), db.post.origid.get(p).unwrap(), c)
    }, 10);
    rows(v.into_iter().map(|(p, (h, (u, a)))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1])]);
        f.push(V::T(h.map_or(ts(1900, 1, 1, 0, 0, 0), |h| hd.get(h).unwrap())));
        f.push(V::S(h.and_then(|h| comment.get(h)).unwrap_or("No comments")));
        row(f)
    }))
}

// WITH UserPostStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN
// p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.ViewCount) AS TotalViews, SUM(CASE WHEN b.Name IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges, MAX(p.CreationDate) AS
// LastPostDate FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName), UserActivityAnalytics AS (SELECT
// ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalViews, ups.TotalBadges, ups.LastPostDate, COALESCE(MAX(ph.CreationDate),
// '1970-01-01') AS LastEditDate, COUNT(DISTINCT ph.Id) AS TotalEdits FROM UserPostStatistics ups LEFT JOIN PostHistory ph ON ups.UserId = ph.UserId GROUP BY ups.UserId,
// ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalViews, ups.TotalBadges, ups.LastPostDate), RankedUserStatistics AS (SELECT ua.*, ROW_NUMBER()
// OVER (ORDER BY ua.TotalPosts DESC, ua.TotalViews DESC) AS UserRanking FROM UserActivityAnalytics ua) SELECT rus.UserId, rus.DisplayName, rus.TotalPosts, rus.TotalQuestions,
// rus.TotalAnswers, rus.TotalViews, rus.TotalBadges, rus.LastPostDate, rus.LastEditDate, rus.TotalEdits, rus.UserRanking FROM RankedUserStatistics rus WHERE rus.TotalPosts > 5
// AND rus.TotalBadges > 0 ORDER BY rus.UserRanking LIMIT 10;
fn q25163(db: &'static So) -> String {
    let Post { post_type_id, view_count, creation_date, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(creation_date)).opt().and(badges_of(db).opt()))
        .fold([0, 0, 0, 0, 0, 0, i64::MIN], |a, (p, b)| match p {
            Some(((t, w), d)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + b.is_some() as i64, a[6].max(d)],
            None => [a[0], a[1], a[2], a[3], a[4], a[5] + b.is_some() as i64, a[6]],
        });
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    type R = (Id<User>, ([i64; 7], Option<(i64, i64)>));
    let w = whole(&ups).select(Ident::<User>::new().and((&ups).and((&ph).opt()))).window(row_number, |(u, (a, _)): R| (Reverse(a[0]), a[3] == 0, Reverse(a[4]), u), asc);
    let v = top_n(drain((&w).filt(|((_, (a, _)), _): (R, i64)| a[0] > 5 && a[5] > 0)), |&(_, (_, r))| r, 10);
    rows(v.into_iter().map(|(_, ((u, (a, h)), i))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), V::I(a[5]), tmax(a[6])]);
        f.extend([V::T(h.map_or(0, |h| h.1)), V::I(h.map_or(0, |h| h.0)), V::I(i)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, COUNT(DISTINCT a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId =
// 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1
// LEFT JOIN Posts a ON u.Id = a.OwnerUserId AND a.PostTypeId = 2 LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.DisplayName, u.Reputation), TopUsers AS (SELECT UserId,
// DisplayName, Reputation, QuestionCount, AnswerCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats), RecentPosts AS (SELECT p.Id
// AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount FROM Posts p JOIN
// Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days') SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.QuestionCount,
// tu.AnswerCount, rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, COALESCE(tu.UpVotes, 0) AS UpVotesTotal, COALESCE(tu.DownVotes, 0) AS DownVotesTotal FROM TopUsers tu
// LEFT JOIN RecentPosts rp ON tu.DisplayName = rp.OwnerDisplayName WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC, rp.CreationDate DESC LIMIT 100;
//
// ReputationRank reads only Reputation, so the top users are picked first and the questions x answers x votes product is driven for them alone.
// `tu.DisplayName = rp.OwnerDisplayName` joins by name; CURRENT_TIMESTAMP is compared in the session zone.
fn q1575(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let q = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let a = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2)));
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(q().opt().and(a().opt()).and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |x, (_, t)| [x[0] + (t == Some(2)) as i64, x[1] + (t == Some(3)) as i64]);
    let qc = (&tu).group_by(Ident::<User>::new()).select(q().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ac = (&tu).group_by(Ident::<User>::new()).select(a().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let since = ny_to_utc(add_days(utc_to_ny(now_utc()), -30));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rp: HashIdx<Str, Id<Post>> = db.post.with(creation_date.filt(move |d: i64| ny_to_utc(d) >= since)).with(owner_user).select(owner_user.select(&db.user.display_name)).inv().collect();
    let v = drain((&us).and(&qc).and(&ac).and((&db.user.display_name).select((&rp).select(Ident::<Post>::new().and(&cc))).opt()));
    let v = top_n(v, |&(u, (_, p))| {
        let d = p.map(|(p, _)| creation_date.get(p).unwrap());
        (Reverse(db.user.reputation.get(u).unwrap()), d.is_none(), Reverse(d), u, p)
    }, 100);
    rows(v.into_iter().map(|(u, (((x, q), a), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(q), V::I(a)]);
        match p {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["id", "title", "created"]));
                f.push(V::I(c));
            }
            None => f.extend((0..4).map(|_| V::Null)),
        }
        f.extend([V::I(x[0]), V::I(x[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate
// DESC) AS Rank FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'), UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName,
// ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation > 1000), CommentStats AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments,
// AVG(c.Score) AS AvgCommentScore FROM Comments c GROUP BY c.PostId), PostAnalysis AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, ur.DisplayName AS TopUser,
// ur.Reputation AS UserReputation, cs.TotalComments, cs.AvgCommentScore, COALESCE(pl.Id, 0) AS RelatedPostLink FROM RankedPosts rp LEFT JOIN UserReputation ur ON ur.UserRank = 1
// LEFT JOIN Posts pl ON pl.Id = rp.PostId LEFT JOIN CommentStats cs ON cs.PostId = rp.PostId) SELECT pa.Title, pa.Score, pa.ViewCount, pa.TopUser, pa.UserReputation,
// pa.TotalComments, pa.AvgCommentScore, CASE WHEN pa.RelatedPostLink = 0 THEN 'No related links' ELSE 'Has related post' END AS RelationStatus FROM PostAnalysis pa WHERE
// pa.Score IS NOT NULL AND pa.ViewCount > 100 ORDER BY pa.Score DESC, pa.UserReputation DESC LIMIT 10;
//
// `LEFT JOIN UserReputation ur ON ur.UserRank = 1` names only ur, so every post meets the single top user. `pl.Id = rp.PostId` is the post itself, so
// RelatedPostLink is its id and never 0. Rank is never read.
fn q3817(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let uw = whole(db.user.with((&db.user.reputation).gt(1000))).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r): (Id<User>, i64)| (Reverse(r), u), asc);
    let top: HashIdx<(), Id<User>> = (&uw).filt(|(_, r)| r == 1).map(|((u, _), _)| u).collect();
    let cs = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let v = drain(
        db.post
            .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
            .with(view_count.gt(100))
            .select(Ident::<Post>::new().and((&cs).opt()).and(Ident::<Post>::new().map(|_| ()).select(&top).opt())),
    );
    let v = top_n(v, |&(_, ((p, _), u))| (Reverse(score.get(p).unwrap()), u.map(|u| Reverse(db.user.reputation.get(u).unwrap())), p), 10);
    rows(v.into_iter().map(|(_, ((p, c), u))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        match u {
            Some(u) => f.extend(ucols(db, u, &["name", "rep"])),
            None => f.extend([V::Null, V::Null]),
        }
        match c {
            Some(c) => f.extend([V::I(c[0]), avg(c[1], c[0])]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(V::S("Has related post"));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY
// p.CreationDate DESC) AS PostRank, RANK() OVER (ORDER BY p.ViewCount DESC) AS PopularityRank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >=
// CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '5 years' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.PostTypeId), PostHistoryDetails AS (SELECT ph.PostId,
// MIN(ph.CreationDate) AS FirstEditDate, MAX(ph.CreationDate) AS LastEditDate, COUNT(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 END) AS EditCount, COUNT(CASE WHEN
// ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount FROM PostHistory ph GROUP BY ph.PostId) SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount,
// rp.CommentCount, COALESCE(phd.EditCount, 0) AS EditCount, COALESCE(phd.CloseReopenCount, 0) AS CloseReopenCount, CASE WHEN rp.PopularityRank <= 10 THEN 'Hot' WHEN
// rp.PopularityRank <= 50 THEN 'Trending' ELSE 'Average' END AS PopularityStatus, CASE WHEN phd.FirstEditDate IS NOT NULL THEN EXTRACT(EPOCH FROM (phd.LastEditDate -
// phd.FirstEditDate)) / 3600 ELSE NULL END AS HoursBetweenFirstAndLastEdit FROM RankedPosts rp LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId WHERE rp.PostRank = 1
// ORDER BY rp.ViewCount DESC, rp.CreationDate ASC;
fn q22618(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -5)));
    let pw = whole(base()).select(Ident::<Post>::new().and(view_count.opt())).window(rank, |(_, w): (Id<Post>, Option<i64>)| (w.is_none(), Reverse(w)), asc);
    let pr: MatSet<(Id<Post>, i64)> = (&pw).map(|((p, _), r)| (p, r)).collect();
    let w = base().group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let phd = db.post_history.group_by(&db.post_history.post).select((&db.post_history.creation_date).and(&db.post_history.post_history_type_id)).fold([i64::MAX, i64::MIN, 0, 0], |a, (d, t)| {
        [a[0].min(d), a[1].max(d), a[2] + matches!(t, 4..=6) as i64, a[3] + matches!(t, 10 | 11) as i64]
    });
    let mut v = drain((&cc).and(by_first(&pr)).and((&phd).opt()));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), creation_date.get(p).unwrap())
    });
    rows(v.into_iter().map(|(p, ((c, r), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.push(V::I(c));
        match h {
            Some(h) => f.extend([V::I(h[2]), V::I(h[3])]),
            None => f.extend([V::I(0), V::I(0)]),
        }
        f.push(V::S(if r <= 10 { "Hot" } else if r <= 50 { "Trending" } else { "Average" }));
        f.push(h.map_or(V::Null, |h| V::F(secs(h[1] - h[0]) / 3600.0)));
        row(f)
    }))
}

// WITH UserScore AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, CASE WHEN u.Reputation > 1000 THEN 'High' WHEN u.Reputation BETWEEN 500 AND 1000 THEN 'Medium' ELSE
// 'Low' END AS ReputationLevel, COUNT(DISTINCT p.Id) AS QuestionsCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1
// ELSE 0 END) AS DownVotesCount, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON u.Id =
// v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation), TopUsers AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
// FROM UserScore WHERE ReputationLevel = 'High' AND QuestionsCount > 5), QuestionStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalQuestions, SUM(CASE WHEN p.AcceptedAnswerId
// IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedQuestions, AVG(COALESCE(p.Score, 0)) AS AverageScore FROM Posts p WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId) SELECT
// tu.DisplayName, tu.Reputation, tu.QuestionsCount, qs.TotalQuestions, qs.AcceptedQuestions, qs.AverageScore, (tu.UpVotesCount - tu.DownVotesCount) AS NetVotes, (SELECT COUNT(*)
// FROM Comments c WHERE c.UserId = tu.UserId) AS CommentCount FROM TopUsers tu LEFT JOIN QuestionStats qs ON tu.UserId = qs.OwnerUserId WHERE (tu.UpVotesCount > 10 OR
// tu.DownVotesCount < 5) ORDER BY tu.Reputation DESC, qs.AverageScore DESC LIMIT 50;
//
// TopUsers keeps users with Reputation > 1000 and more than five questions, both computable without the product, so the questions x votes x badges product
// is driven for those users alone. ReputationRank is never read.
fn q721(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, score, .. } = &db.post;
    let q = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let qc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(q()).fold(0i64, |n, _| n + 1);
    let cand: MatSet<Id<User>> = db.user.with((&qc).filt(|n| n > 5)).collect();
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(q().opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()).and(badges_of(db).opt()))
        .fold([0i64; 2], |a, ((_, t), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let qs = db.post.with(post_type_id.eq(1)).group_by(&db.post.owner_user).select(accepted_answer_id.opt().and(score)).fold([0i64; 3], |a, (x, s)| [a[0] + 1, a[1] + x.is_some() as i64, a[2] + s]);
    let cm = db.comment.group_by(&db.comment.user).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    type A = [i64; 2];
    let v = drain((&us).filt(|a: A| a[0] > 10 || a[1] < 5).and(&qc).and((&qs).opt()).and((&cm).opt()));
    let v = top_n(v, |&(u, (((_, _), s), _))| {
        let a = s.map(|s| fkey(s[2] as f64 / s[0] as f64));
        (Reverse(db.user.reputation.get(u).unwrap()), a.is_none(), Reverse(a), u)
    }, 50);
    rows(v.into_iter().map(|(u, (((a, n), s), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        match s {
            Some(s) => f.extend([V::I(s[0]), V::I(s[1]), avg(s[2], s[0])]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.extend([V::I(a[0] - a[1]), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN
// v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY COUNT(DISTINCT c.Id) DESC, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS PostRank FROM
// Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP
// BY p.Id, p.Title, p.CreationDate), TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes, COALESCE(b.Name, 'No Badge') AS
// BadgeName FROM RankedPosts rp LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) WHERE rp.PostRank <= 10), PostDetails AS (SELECT tp.*, CASE
// WHEN tp.UpVotes > tp.DownVotes THEN 'Positive' WHEN tp.UpVotes < tp.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment, CASE WHEN tp.CommentCount > 5 THEN 'Highly
// Discussed' WHEN tp.CommentCount BETWEEN 1 AND 5 THEN 'Moderately Discussed' ELSE 'Not Discussed' END AS DiscussionLevel FROM TopPosts tp) SELECT pd.PostId, pd.Title,
// pd.CreationDate, pd.CommentCount, pd.UpVotes, pd.DownVotes, pd.BadgeName, pd.VoteSentiment, pd.DiscussionLevel FROM PostDetails pd ORDER BY pd.CreationDate DESC LIMIT 15;
//
// PostRank is over the recent posts' comment x vote product. The final ORDER BY ... LIMIT 15 cuts at a post boundary; badge ties within a post are broken by the badge id.
fn q3111(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rp = base()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let dc = base().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(&rp).select(Ident::<Post>::new().and(&dc).and(&rp)).window(rank, |((_, d), a): ((Id<Post>, i64), [i64; 3])| (Reverse(d), Reverse(a[1])), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let by_uid: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let v = drain((&tp).select((&rp).and(owner_user_id.select(&by_uid).opt())));
    let v = top_n(v, |&(p, (_, b))| (Reverse(creation_date.get(p).unwrap()), p, b), 15);
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        f.push(V::S(if a[0] > 5 { "Highly Discussed" } else if a[0] >= 1 { "Moderately Discussed" } else { "Not Discussed" }));
        row(f)
    }))
}

// WITH UserRanks AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U), PostStatistics AS (SELECT
// P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS
// TotalAnswers, COUNT(CASE WHEN P.ClosedDate IS NOT NULL THEN 1 END) AS TotalClosedPosts, COUNT(DISTINCT C.Id) AS TotalComments FROM Posts P LEFT JOIN Comments C ON P.Id =
// C.PostId GROUP BY P.OwnerUserId), UserBadges AS (SELECT B.UserId, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class
// = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId) SELECT UR.UserId, UR.DisplayName,
// UR.Reputation, UR.ReputationRank, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.TotalQuestions, 0) AS TotalQuestions, COALESCE(PS.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(PS.TotalClosedPosts, 0) AS TotalClosedPosts, COALESCE(PS.TotalComments, 0) AS TotalComments, COALESCE(UB.TotalBadges, 0) AS TotalBadges, COALESCE(UB.GoldBadges, 0) AS
// GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges FROM UserRanks UR LEFT JOIN PostStatistics PS ON UR.UserId =
// PS.OwnerUserId LEFT JOIN UserBadges UB ON UR.UserId = UB.UserId ORDER BY UR.Reputation DESC, UR.DisplayName ASC LIMIT 50;
//
// The order reads only Reputation and DisplayName, so the first fifty users are picked first and PostStatistics is driven for them alone.
fn q9887(db: &'static So) -> String {
    let Post { post_type_id, closed_date, .. } = &db.post;
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let v = top_n(drain(&w), |&(_, ((u, r), _))| (Reverse(r), db.user.display_name.get(u).unwrap(), u), 50);
    let ur: MatSet<(Id<User>, i64)> = rel(v.into_iter().map(|(_, ((u, _), r))| (u, r)).collect()).map(|x| x).collect();
    let tu: MatSet<Id<User>> = (&ur).map(|(u, _)| u).collect();
    let ps = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(closed_date.opt()).and(comments_of(db).opt())))
        .fold([0i64; 5], |a, ((t, c), m)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + m.is_some() as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain(by_first(&ur).and((&ps).opt()).and((&ub).opt()));
    rows(v.into_iter().map(|(u, ((r, p), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(r));
        f.extend(p.unwrap_or([0; 5]).map(V::I));
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS
// UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p LEFT JOIN
// Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title,
// p.CreationDate, p.Score, p.OwnerUserId), UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(bp.Score), 0) AS TotalScore, COUNT(b.Id) AS BadgeCount
// FROM Users u LEFT JOIN Posts bp ON u.Id = bp.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation), TopUsers AS (SELECT us.UserId,
// us.DisplayName, us.Reputation, us.TotalScore, us.BadgeCount, DENSE_RANK() OVER (ORDER BY us.Reputation DESC, us.TotalScore DESC) AS UserRank FROM UserStats us WHERE
// us.Reputation IS NOT NULL) SELECT tp.UserId, tp.DisplayName, tp.Reputation, tp.TotalScore, tp.BadgeCount, rp.PostId, rp.Title AS PostTitle, rp.CreationDate AS
// PostCreationDate, rp.Score AS PostScore, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM TopUsers tp LEFT JOIN RankedPosts rp ON tp.UserId = rp.OwnerUserId WHERE tp.UserRank <=
// 10 ORDER BY tp.Reputation DESC, rp.Score DESC NULLS LAST;
//
// UserRank is a DENSE_RANK led by Reputation, so only users with one of the ten highest reputations can reach rank 10; the posts x badges product is
// driven for those alone. Rank is never read.
fn q65(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let rw = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(dense_rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let cand: MatSet<Id<User>> = (&rw).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (s, b)| [a[0] + s.unwrap_or(0), a[1] + b.is_some() as i64]);
    let w = whole(&us).select(Ident::<User>::new().and(&db.user.reputation).and(&us)).window(dense_rank, |((_, r), a): ((Id<User>, i64), [i64; 2])| (Reverse(r), Reverse(a[0])), asc);
    let tu: MatSet<(Id<User>, [i64; 2])> = (&w).filt(|(_, r)| r <= 10).map(|(((u, _), a), _)| (u, a)).collect();
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain(by_first(&tu).and(posts_of(db).select(Ident::<Post>::new().and(&rp)).opt()));
    v.sort_by_key(|&(u, (_, p))| (Reverse(db.user.reputation.get(u).unwrap()), p.is_none(), Reverse(p.map(|(p, _)| score.get(p).unwrap()))));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        match p {
            Some((p, r)) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
                f.extend(r.map(V::I));
            }
            None => f.extend((0..7).map(|_| V::Null)),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY
// p.CreationDate DESC) AS rn FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year'), UserStats AS (SELECT u.Id AS UserId,
// u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionsAsked, SUM(COALESCE(v.VoteValue, 0)) AS TotalScore, MAX(u.Reputation) AS Reputation FROM Users u LEFT JOIN Posts p ON u.Id =
// p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END) AS VoteValue FROM Votes v GROUP
// BY v.PostId) v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName), TopUsers AS (SELECT us.UserId, us.DisplayName, us.QuestionsAsked, us.TotalScore, us.Reputation, ROW_NUMBER()
// OVER (ORDER BY us.TotalScore DESC, us.Reputation DESC) AS Rank FROM UserStats us WHERE us.Reputation > 1000) SELECT tu.DisplayName, tu.QuestionsAsked, tu.TotalScore,
// RP.PostId, RP.Title, RP.CreationDate FROM TopUsers tu LEFT JOIN RankedPosts RP ON tu.UserId = RP.OwnerUserId AND RP.rn = 1 WHERE tu.Rank <= 10 ORDER BY tu.TotalScore DESC,
// tu.Reputation DESC;
fn q31894(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let vv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |s, t| s + (t == 2) as i64 - (t == 3) as i64);
    let q = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let us = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(q().select((&vv).opt()).opt()).fold([0i64; 2], |a, p| match p {
        Some(v) => [a[0] + 1, a[1] + v.unwrap_or(0)],
        None => a,
    });
    let tu = top_n(drain(&us), |&(u, a)| (Reverse(a[1]), Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    let tu = rel(tu);
    let w = db.post.with(post_type_id.eq(1).and(creation_date.gt(add_years(date(2024, 10, 1), -1)))).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let by_owner = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    type R = (Id<User>, [i64; 2]);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(by_owner).opt())));
    rows(v.into_iter().map(|(_, ((u, a), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1])];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "created"])),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN
// 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id, p.Title, p.ViewCount, ROW_NUMBER() OVER (ORDER BY p.ViewCount DESC) AS RN FROM Posts p WHERE p.ViewCount IS NOT NULL AND p.CreationDate >
// cast('2024-10-01' as date) - INTERVAL '1 year'), ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, COUNT(*) AS CloseActionCount FROM PostHistory ph WHERE ph.PostHistoryTypeId
// IN (10, 11) GROUP BY ph.PostId, ph.CreationDate) SELECT ub.UserId, ub.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, pp.Title, pp.ViewCount,
// cp.CloseActionCount, COALESCE(cp.CloseActionCount, 0) AS CloseCount, CASE WHEN ub.BadgeCount > 5 THEN 'Highly Decorated' WHEN ub.BadgeCount BETWEEN 3 AND 5 THEN 'Moderately
// Decorated' ELSE 'New User' END AS UserStatus FROM UserBadges ub LEFT JOIN PostLinks pl ON ub.UserId = pl.RelatedPostId LEFT JOIN PopularPosts pp ON pl.PostId = pp.Id LEFT JOIN
// ClosedPosts cp ON pp.Id = cp.PostId WHERE pp.RN <= 10 OR cp.CloseActionCount IS NOT NULL ORDER BY ub.BadgeCount DESC, pp.ViewCount DESC NULLS LAST;
//
// `ub.UserId = pl.RelatedPostId` joins a user id to a post id, through the raw ids. RN ties on ViewCount go to the smaller post id (the SQL leaves them open; flipping it in DuckDB leaves this answer unchanged).
fn q2204(db: &'static So) -> String {
    let Post { view_count, creation_date, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let pw = whole(db.post.with(view_count).with(creation_date.gt(add_years(date(2024, 10, 1), -1)))).select(Ident::<Post>::new().and(view_count)).window(row_number, |(p, w): (Id<Post>, i64)| (Reverse(w), p), asc);
    let pp: MatSet<(Id<Post>, i64)> = (&pw).map(|((p, _), r)| (p, r)).collect();
    let rn: HashIdx<Id<Post>, (Id<Post>, i64)> = (&pp).map(|(p, _)| p).inv().collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post.and(hd)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cv: MatSet<((Id<Post>, i64), i64)> = whole(&cp).select(Same::<(Id<Post>, i64)>::new().and(&cp)).collect();
    let by_post: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&cv).map(|((p, _), _)| p).inv().collect();
    let links: HashIdx<i64, Id<PostLink>> = (&db.post_link.related_post_id).inv().collect();
    type L = ((Id<Post>, i64), Option<((Id<Post>, i64), i64)>);
    let pl = (&links).select(&db.post_link.post).select((&rn).map(|x| x).and((&by_post).opt()));
    let v = drain((&ub).and((&db.user.origid).select(pl.filt(|((_, r), c): L| r <= 10 || c.is_some()))));
    let mut v = v;
    v.sort_by_key(|&(_, (a, ((p, _), _)))| (Reverse(a[0]), Reverse(view_count.get(p))));
    rows(v.into_iter().map(|(u, (a, ((p, _), c)))| {
        let n = a[0];
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([oint(c.map(|c| c.1)), V::I(c.map_or(0, |c| c.1))]);
        f.push(V::S(if n > 5 { "Highly Decorated" } else if n >= 3 { "Moderately Decorated" } else { "New User" }));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("30222", q30222),
    ("6936", q6936),
    ("2011", q2011),
    ("1435", q1435),
    ("6535", q6535),
    ("496", q496),
    ("9816", q9816),
    ("3097", q3097),
    ("3353", q3353),
    ("3636", q3636),
    ("6195", q6195),
    ("4816", q4816),
    ("28128", q28128),
    ("1290", q1290),
    ("20936", q20936),
    ("5174", q5174),
    ("6825", q6825),
    ("8081", q8081),
    ("3367", q3367),
    ("2803", q2803),
    ("4170", q4170),
    ("25392", q25392),
    ("468", q468),
    ("7100", q7100),
    ("9719", q9719),
    ("3526", q3526),
    ("1259", q1259),
    ("498", q498),
    ("4308", q4308),
    ("30077", q30077),
    ("6725", q6725),
    ("1399", q1399),
    ("32290", q32290),
    ("4354", q4354),
    ("5125", q5125),
    ("26840", q26840),
    ("31711", q31711),
    ("41", q41),
    ("3809", q3809),
    ("21346", q21346),
    ("7637", q7637),
    ("8276", q8276),
    ("3309", q3309),
    ("1428", q1428),
    ("3917", q3917),
    ("2263", q2263),
    ("2277", q2277),
    ("582", q582),
    ("22971", q22971),
    ("864", q864),
    ("3659", q3659),
    ("2366", q2366),
    ("3863", q3863),
    ("1474", q1474),
    ("2036", q2036),
    ("1116", q1116),
    ("30463", q30463),
    ("3853", q3853),
    ("9282", q9282),
    ("636", q636),
    ("4552", q4552),
    ("4918", q4918),
    ("2726", q2726),
    ("7626", q7626),
    ("21199", q21199),
    ("26465", q26465),
    ("4174", q4174),
    ("23580", q23580),
    ("4385", q4385),
    ("26996", q26996),
    ("442", q442),
    ("3472", q3472),
    ("2044", q2044),
    ("397", q397),
    ("4222", q4222),
    ("20726", q20726),
    ("2670", q2670),
    ("31537", q31537),
    ("31641", q31641),
    ("96", q96),
    ("1439", q1439),
    ("7483", q7483),
    ("1965", q1965),
    ("3394", q3394),
    ("21555", q21555),
    ("905", q905),
    ("3737", q3737),
    ("4253", q4253),
    ("25163", q25163),
    ("1575", q1575),
    ("3817", q3817),
    ("22618", q22618),
    ("721", q721),
    ("3111", q3111),
    ("9887", q9887),
    ("65", q65),
    ("31894", q31894),
    ("2204", q2204),
];
