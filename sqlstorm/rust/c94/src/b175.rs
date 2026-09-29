use harness::prelude::*;
use std::cmp::Reverse;

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetVotes,
//        DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// PostStats AS (SELECT p.OwnerUserId, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS Questions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS Answers, SUM(p.Score) AS TotalScore, AVG(COALESCE(p.ViewCount, 0)) AS AvgViews,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY SUM(p.Score) DESC) AS PostRank FROM Posts p GROUP BY p.OwnerUserId),
// ClosedPosts AS (SELECT ph.UserId, COUNT(DISTINCT ph.PostId) AS ClosedCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 12) GROUP BY ph.UserId),
// FinalStats AS (SELECT ur.UserId, ur.DisplayName, ur.Reputation, us.TotalPosts, us.Questions, us.Answers, us.TotalScore, us.AvgViews,
//        COALESCE(cp.ClosedCount, 0) AS ClosedPosts, ur.ReputationRank,
//        CASE WHEN ur.Reputation >= 1000 THEN 'High' WHEN ur.Reputation BETWEEN 500 AND 999 THEN 'Medium' ELSE 'Low' END AS ReputationCategory
//     FROM UserReputation ur LEFT JOIN PostStats us ON ur.UserId = us.OwnerUserId LEFT JOIN ClosedPosts cp ON ur.UserId = cp.UserId)
// SELECT fs.DisplayName, CASE WHEN fs.ClosedPosts < 5 THEN 'Newbie' WHEN fs.ClosedPosts BETWEEN 5 AND 10 THEN 'Regular' ELSE 'Veteran' END AS ClosureExperience,
//        fs.TotalPosts, fs.Questions, fs.Answers, fs.TotalScore, fs.AvgViews, fs.ClosedPosts, fs.ReputationCategory
// FROM FinalStats fs WHERE fs.Reputation > 100 ORDER BY fs.ReputationRank, fs.TotalScore DESC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
//
// The votes join in UserReputation is grouped back to one row per user and none of its aggregates is read, so it is not computed.
fn q24710(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 5], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0)]
    });
    let PostHistory { user, post, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 12])).group_by(user).select(post).count_distinct();
    let rep = &db.user.reputation;
    let v = drain(db.user.with(rep.gt(100)).select((&ps).opt().and((&cp).opt())));
    let v = top_n(v, |&(u, (p, _))| (Reverse(rep.get(u).unwrap()), p.is_none(), Reverse(p.map(|a| a[3]))), 20);
    rows(v.into_iter().skip(10).map(|(u, (p, c))| {
        let r = rep.get(u).unwrap();
        let c = c.unwrap_or(0);
        let mut f = vec![user_col(db, u, "name"), V::S(if c < 5 { "Newbie" } else if c <= 10 { "Regular" } else { "Veteran" })];
        f.extend(match p {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0])],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        f.extend([V::I(c), V::S(if r >= 1000 { "High" } else if r >= 500 { "Medium" } else { "Low" })]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(p.ViewCount), 0) AS TotalViews, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, TotalViews, PostCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserStatistics),
// FilteredUsers AS (SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, TotalViews, PostCount, ReputationRank, ViewRank FROM RankedUsers WHERE PostCount > 5),
// CloseReasonSummary AS (SELECT postHistory.UserId, COUNT(CASE WHEN postHistory.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
//        COUNT(CASE WHEN postHistory.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount FROM PostHistory postHistory GROUP BY postHistory.UserId),
// FinalResult AS (SELECT f.UserId, f.DisplayName, f.Reputation, f.UpVotes, f.DownVotes, f.TotalViews, f.PostCount, COALESCE(c.CloseCount, 0) AS TotalCloseActions,
//        COALESCE(c.ReopenCount, 0) AS TotalReopenActions FROM FilteredUsers f LEFT JOIN CloseReasonSummary c ON f.UserId = c.UserId)
// SELECT *, CASE WHEN Reputation > 1000 THEN 'High Reputation' WHEN Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory,
//        CASE WHEN PostCount > 10 THEN 'Frequent Contributor' ELSE 'Occasional Contributor' END AS ContributionType
// FROM FinalResult WHERE (TotalCloseActions > 3 OR TotalReopenActions > 3) ORDER BY Reputation DESC, TotalViews DESC;
fn q22552(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((w, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + w.unwrap_or(0)],
            None => a,
        });
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let cr = db.post_history.group_by(user).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let v = drain((&us).and(user_distinct_posts(db).filt(|n| n > 5)).and((&cr).filt(|c| c[0] > 3 || c[1] > 3)));
    rows(v.into_iter().map(|(u, ((a, n), c))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n), V::I(c[0]), V::I(c[1])]);
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" }));
        f.push(V::S(if n > 10 { "Frequent Contributor" } else { "Occasional Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COALESCE(v.UpVotes, 0) AS TotalUpVotes, COALESCE(v.DownVotes, 0) AS TotalDownVotes
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//         FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     WHERE p.CreationDate >= DATE('2024-10-01') - INTERVAL '1 year'),
// UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(COALESCE(v.TotalUpVotes, 0) - COALESCE(v.TotalDownVotes, 0)) AS VoteScore,
//        AVG(EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - u.CreationDate)) / 3600) AS AvgHoursSinceCreation
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     LEFT JOIN (SELECT p.OwnerUserId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//         FROM Posts p JOIN Votes v ON p.Id = v.PostId GROUP BY p.OwnerUserId) v ON u.Id = v.OwnerUserId
//     GROUP BY u.Id, u.DisplayName),
// PostHistoryAggregates AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN ph.CreationDate END) AS LastClosedOrReopened,
//        COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, um.DisplayName AS OwnerName, um.BadgeCount, um.VoteScore, pha.LastClosedOrReopened, pha.DeleteCount,
//        CASE WHEN rp.PostRank = 1 THEN 'Latest Post by User' WHEN rp.PostRank <= 5 THEN 'Top 5 Recent Posts' ELSE 'Older Posts' END AS PostCategory
// FROM RankedPosts rp JOIN UserMetrics um ON rp.OwnerUserId = um.UserId LEFT JOIN PostHistoryAggregates pha ON rp.PostId = pha.PostId
// WHERE um.VoteScore > 0 ORDER BY um.VoteScore DESC, rp.CreationDate DESC;
//
// The per-post vote sums in RankedPosts are never read, so they are not computed. VoteScore is summed over each user's badge rows, as the join makes it.
fn q24628(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let recent = drain(db.post.with(creation_date.ge(ts(2023, 10, 1, 0, 0, 0))).select(owner_user));
    let r = per_group(ranked(recent, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap())), false), |&(_, u)| u);
    let rr = rel(r);
    let vt = &db.vote.vote_type_id;
    let ov = db.post.group_by(owner_user).select(votes_of(db).select(vt)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let um = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt().and((&ov).opt())).fold([0i64; 3], |a, (b, o)| {
        [a[0] + b.is_some() as i64, a[1] + o.map_or(0, |x| x[0] - x[1]), a[2] + o.is_some() as i64]
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold((i64::MIN, 0i64), |(m, n), (t, d)| {
        (if t == 10 || t == 11 { m.max(d) } else { m }, n + (t == 12) as i64)
    });
    type T = ((Id<Post>, Id<User>), i64);
    let v = drain((&rr).select(
        Same::<T>::new()
            .and(Same::<T>::new().map(|((_, u), _): T| u).select((&um).filt(|a| a[2] > 0 && a[1] > 0)))
            .and(Same::<T>::new().map(|((p, _), _): T| p).select((&pha).opt())),
    ));
    rows(v.into_iter().map(|(_, ((((p, u), k), a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some((m, n)) => [tmax(m), V::I(n)],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if k == 1 { "Latest Post by User" } else if k <= 5 { "Top 5 Recent Posts" } else { "Older Posts" }));
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT Id AS UserId, Reputation, LastAccessDate, ROW_NUMBER() OVER (PARTITION BY Reputation ORDER BY LastAccessDate DESC) AS rn FROM Users),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, p.Score, COALESCE(c.Text, 'No comments') AS CommentText, COUNT(c.Id) AS CommentCount,
//        LAG(p.Score) OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate) AS PreviousScore,
//        DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, p.Score, c.Text),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.UserId, ph.CreationDate, EXTRACT(EPOCH FROM ph.CreationDate) AS EditTimestamp,
//        CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 'Closed/Open' WHEN ph.PostHistoryTypeId = 12 THEN 'Deleted' ELSE 'Other' END AS ActionType
//     FROM PostHistory ph WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount, ur.Reputation, ROW_NUMBER() OVER (ORDER BY ur.Reputation DESC) AS Rank
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId JOIN UserReputation ur ON u.Id = ur.UserId WHERE ur.rn = 1)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.BadgeCount, rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, phd.EditTimestamp, phd.ActionType
// FROM TopUsers tu JOIN RecentPosts rp ON tu.UserId = rp.OwnerUserId LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId
// WHERE tu.Rank <= 10 AND (rp.Score - COALESCE(rp.PreviousScore, 0)) > 0 ORDER BY tu.Reputation DESC, rp.CreationDate DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. RecentPosts has one row per (post, comment text), and the LAG orders them by CreationDate only, so the
// rows of one post tie; the port breaks the tie by post and then text (NULL first), and ties in rn = 1 by user id.
fn q21706(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let last_access_date = &db.user.last_access_date;
    let ur = top_per(drain(&db.user.reputation), |&(_, r)| r, |&(u, _)| (Reverse(last_access_date.get(u).unwrap()), u), 1, false);
    let tu = top_n(ur, |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let recent = || db.post.with(creation_date.ge(add_days(t0, -30)));
    let Comment { post, text, .. } = &db.comment;
    let cg = db.comment.with(post.select(Ident::<Post>::new().with(creation_date.ge(add_days(t0, -30))))).group_by(post.and(text)).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    type G = (Id<Post>, Option<Str>, i64);
    let mut groups: Vec<G> = drain(&cg).into_iter().map(|((p, t), n)| (p, Some(t), n)).collect();
    groups.extend(drain(recent().minus(comments_of(db))).into_iter().map(|(p, _)| (p, None, 0)));
    let gr = rel(groups);
    let w = (&gr)
        .group_by(Same::<G>::new().map(|(p, _, _): G| p).select(owner_user))
        .select(Same::<G>::new())
        .window(lag, |(p, t, _): G| (creation_date.get(p).unwrap(), p, t, score.get(p).unwrap()), |a, b| a.cmp(b));
    let rising = (&w).filt(|((p, _, _), prev): (G, Option<(i64, Id<Post>, Option<Str>, i64)>)| score.get(p).unwrap() - prev.map_or(0, |k| k.3) > 0);
    type H = (Id<User>, (G, Option<(i64, Id<Post>, Option<Str>, i64)>));
    let hits = rel(drain((&tu).select(Ident::<User>::new().and(rising))).into_iter().map(|x| x.1).collect::<Vec<H>>());
    let hd = &db.post_history.creation_date;
    let phd = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_years(t0, -1))));
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&hits).select(Same::<H>::new().and(Same::<H>::new().map(|(u, _): H| u).select(&bc)).and(Same::<H>::new().map(|(_, ((p, _, _), _)): H| p).select(phd.opt()))));
    rows(v.into_iter().map(|(_, (((u, ((p, _, n), _)), b), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.push(V::I(n));
        f.extend(match h {
            Some(h) => {
                let t = db.post_history.post_history_type_id.get(h).unwrap();
                [V::F(secs(hd.get(h).unwrap())), V::S(if t == 10 || t == 11 { "Closed/Open" } else if t == 12 { "Deleted" } else { "Other" })]
            }
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecursivePostCTE AS (SELECT Id, Title, PostTypeId, ParentId, OwnerUserId, CreationDate, Score,
//        ROW_NUMBER() OVER (PARTITION BY OwnerUserId ORDER BY CreationDate DESC) AS UserPostRank FROM Posts WHERE PostTypeId = 1),
// UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// RecentPostComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY p.Id),
// PostScores AS (SELECT p.Id, SUM(v.BountyAmount) AS TotalBounty, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT rp.Id AS PostId, rp.Title, ub.DisplayName AS Owner, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, COALESCE(pc.CommentCount, 0) AS CommentCount,
//        COALESCE(ps.TotalBounty, 0) AS TotalBounty, COALESCE(ps.Upvotes, 0) AS Upvotes, COALESCE(ps.Downvotes, 0) AS Downvotes,
//        CASE WHEN rp.Score + COALESCE(ps.Upvotes, 0) - COALESCE(ps.Downvotes, 0) < 0 THEN 0 ELSE rp.Score + COALESCE(ps.Upvotes, 0) - COALESCE(ps.Downvotes, 0) END AS NetScore,
//        CASE WHEN EXISTS (SELECT 1 FROM PostHistory ph WHERE ph.PostId = rp.Id AND ph.PostHistoryTypeId = 10) THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM RecursivePostCTE rp JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN RecentPostComments pc ON rp.Id = pc.PostId LEFT JOIN PostScores ps ON rp.Id = ps.Id
// WHERE rp.UserPostRank <= 10 ORDER BY NetScore DESC;
//
// UserPostRank ties on CreationDate are broken by post id.
fn q30678(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let pc = (&tp).with(creation_date.ge(add_months(current_date(), -6))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let ps = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vote_type_id.and(bounty_amount.opt())).opt()).fold([0i64; 3], |a, v| match v {
        Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let closed = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&ub)).and((&pc).opt()).and(&ps).and(Ident::<Post>::new().with(closed).opt())));
    rows(v.into_iter().map(|(p, ((((u, b), c), a), cl))| {
        let net = score.get(p).unwrap() + a[1] - a[2];
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(user_col(db, u, "name"));
        f.extend(b.map(V::I));
        f.extend([V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(net.max(0)), V::S(if cl.is_some() { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount,
//        MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, TotalViews, TotalBounty, QuestionCount, AnswerCount, LastPostDate,
//        RANK() OVER (ORDER BY TotalViews DESC, TotalBounty DESC) AS ViewRank, RANK() OVER (ORDER BY QuestionCount DESC, AnswerCount DESC) AS ActivityRank FROM UserActivity),
// TopUsers AS (SELECT UserId, DisplayName, TotalViews, TotalBounty, QuestionCount, AnswerCount, LastPostDate, ViewRank, ActivityRank FROM RankedUsers
//     WHERE ViewRank <= 10 OR ActivityRank <= 10),
// PostHistoryAggregates AS (SELECT ph.UserId, COUNT(*) AS EditCount, SUM(CASE WHEN ph.PostHistoryTypeId IN (2, 5) THEN 1 ELSE 0 END) AS BodyEdits,
//        SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS ClosureEdits FROM PostHistory ph GROUP BY ph.UserId)
// SELECT tu.DisplayName, tu.TotalViews, tu.TotalBounty, tu.QuestionCount, tu.AnswerCount, tu.LastPostDate, COALESCE(pha.EditCount, 0) AS TotalEdits,
//        COALESCE(pha.BodyEdits, 0) AS BodyEditCount, COALESCE(pha.ClosureEdits, 0) AS ClosureEditCount,
//        CASE WHEN COALESCE(pha.EditCount, 0) > 0 THEN ROUND(COALESCE(tu.TotalViews, 0) / NULLIF(pha.EditCount, 0), 2) ELSE NULL END AS ViewsPerEdit,
//        CASE WHEN tu.AnswerCount > 0 THEN ROUND(COALESCE(tu.TotalBounty, 0) / NULLIF(tu.AnswerCount, 0), 2) ELSE NULL END AS AverageBountyPerAnswer
// FROM TopUsers tu LEFT JOIN PostHistoryAggregates pha ON tu.UserId = pha.UserId ORDER BY tu.TotalViews DESC, tu.TotalBounty DESC;
fn q24351(db: &'static So) -> String {
    let Post { post_type_id, view_count, creation_date, .. } = &db.post;
    let own = own_votes(db).select((&db.vote.bounty_amount).opt());
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(creation_date).and(own.opt())).opt())
        .fold([0, 0, i64::MIN], |a, p| match p {
            Some((((_, w), d), b)) => [a[0] + w.unwrap_or(0), a[1] + b.flatten().unwrap_or(0), a[2].max(d)],
            None => a,
        });
    let qa = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64]);
    let v = ranked(drain((&ua).and(&qa)), |&(_, (a, _))| (Reverse(a[0]), Reverse(a[1])), false);
    let v = ranked(v, |&((_, (_, q)), _)| (Reverse(q[0]), Reverse(q[1])), false);
    let tu = rel(drain(rel(v).filt(|((_, w), r)| w <= 10 || r <= 10)).into_iter().map(|x| (x.1 .0 .0).0).collect::<Vec<Id<User>>>());
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let pha = db.post_history.group_by(user).select(post_history_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + matches!(t, 2 | 5) as i64, a[2] + matches!(t, 10 | 11) as i64]);
    let v = drain((&tu).select(Ident::<User>::new().and(&ua).and(&qa).and((&pha).opt())));
    let r2 = |x: f64| V::F((x * 100.0).round() / 100.0);
    rows(v.into_iter().map(|(_, (((u, a), q), h))| {
        let h = h.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(q[0]), V::I(q[1]), tmax(a[2]), V::I(h[0]), V::I(h[1]), V::I(h[2])];
        f.push(if h[0] > 0 { r2(a[0] as f64 / h[0] as f64) } else { V::Null });
        f.push(if q[1] > 0 { r2(a[1] as f64 / q[1] as f64) } else { V::Null });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.LastActivityDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank,
//        p.OwnerUserId FROM Posts p WHERE p.Score > 0),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId, ph.PostHistoryTypeId),
// MergedResults AS (SELECT ue.UserId, ue.DisplayName, ue.Reputation, SUM(pe.EditCount) AS TotalEdits, SUM(pe.EditCount) FILTER (WHERE pe.PostHistoryTypeId = 4) AS TitleEdits,
//        SUM(pe.EditCount) FILTER (WHERE pe.PostHistoryTypeId = 5) AS BodyEdits, SUM(pe.EditCount) FILTER (WHERE pe.PostHistoryTypeId = 6) AS TagEdits,
//        COUNT(DISTINCT rp.PostId) AS RankedPostsCount, SUM(ue.UpVotes) AS TotalUpVotes, SUM(ue.DownVotes) AS TotalDownVotes
//     FROM UserEngagement ue JOIN PostHistoryDetails pe ON ue.UserId = (SELECT OwnerUserId FROM Posts p WHERE p.Id = pe.PostId)
//     JOIN RankedPosts rp ON ue.UserId = rp.OwnerUserId GROUP BY ue.UserId, ue.DisplayName, ue.Reputation)
// SELECT UserId, DisplayName, Reputation, TotalEdits, TitleEdits, BodyEdits, TagEdits, RankedPostsCount, TotalUpVotes, TotalDownVotes,
//        CASE WHEN TotalEdits IS NULL THEN 'No Edits' WHEN TotalEdits > 10 THEN 'Super Editor' ELSE 'Editor' END AS EditorStatus
// FROM MergedResults WHERE Reputation > 1000 ORDER BY Reputation DESC, TotalEdits DESC LIMIT 10 OFFSET 0;
//
// The sort leads with Reputation, so only the users whose reputation reaches the tenth-highest among those in MergedResults can make the cut, and the
// pe x rp product is driven for those alone; everyone else ranks below all of them.
fn q24593(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pe = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post.and(post_history_type_id)).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pv = rel(drain(&pe));
    type E = ((Id<Post>, i64), (i64, i64));
    let pe_by: HashIdx<Id<User>, E> = (&pv).map(|((p, _), _): E| p).select(owner_user).inv().select(&pv).collect();
    let rp = || posts_of(db).select(Ident::<Post>::new().with(score.gt(0)));
    let rep = &db.user.reputation;
    let el: MatSet<Id<User>> = db.user.with(rep.gt(1000)).with(&pe_by).with(rp()).collect();
    let tenth = top_n(drain((&el).select(rep)), |&(u, r)| (Reverse(r), u), 10).last().map_or(i64::MAX, |x| x.1);
    let cand: MatSet<Id<User>> = (&el).with(rep.ge(tenth)).collect();
    let ue = (&cand).group_by(Ident::<User>::new()).select(comments_by(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let mr = (&cand).group_by(Ident::<User>::new()).select((&ue).and(&pe_by).and(rp())).fold([0i64; 9], |a, ((u, ((_, t), (n, _))), _)| {
        [a[0] + n, a[1] + (t == 4) as i64, a[2] + if t == 4 { n } else { 0 }, a[3] + (t == 5) as i64, a[4] + if t == 5 { n } else { 0 }, a[5] + (t == 6) as i64, a[6] + if t == 6 { n } else { 0 }, a[7] + u[0], a[8] + u[1]]
    });
    let rc = (&cand).group_by(Ident::<User>::new()).select(rp()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&mr).and(&rc)), |&(u, (a, _))| (Reverse(rep.get(u).unwrap()), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(u, (a, c))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1]), nullable(a[4], a[3]), nullable(a[6], a[5]), V::I(c), V::I(a[7]), V::I(a[8])]);
        f.push(V::S(if a[0] > 10 { "Super Editor" } else { "Editor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY p.Id) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY p.Id) AS DownVotes,
//        COALESCE(pt.Name, 'Unknown Type') AS PostType
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.UpVotes, rp.DownVotes, rp.PostType FROM RankedPosts rp WHERE Rank <= 5),
// PostWithComments AS (SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.UpVotes, fp.DownVotes, fp.PostType, COUNT(c.Id) AS CommentCount
//     FROM FilteredPosts fp LEFT JOIN Comments c ON fp.PostId = c.PostId GROUP BY fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.UpVotes, fp.DownVotes, fp.PostType),
// AggregateData AS (SELECT pwc.PostId, pwc.Title, pwc.CreationDate, pwc.Score, pwc.UpVotes, pwc.DownVotes, pwc.CommentCount,
//        CASE WHEN pwc.Score > 0 THEN 'Positive' WHEN pwc.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreCategory,
//        RANK() OVER (ORDER BY pwc.UpVotes DESC, pwc.CommentCount DESC) AS PopularityRank FROM PostWithComments pwc)
// SELECT ad.PostId, ad.Title, ad.CreationDate, ad.Score, ad.UpVotes, ad.DownVotes, ad.CommentCount, ad.ScoreCategory, ad.PopularityRank, mh.Date AS LastModified
// FROM AggregateData ad LEFT JOIN PostHistory ph ON ad.PostId = ph.PostId
// LEFT JOIN (SELECT PostId, MAX(CreationDate) AS Date FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6) GROUP BY PostId) mh ON mh.PostId = ad.PostId
// WHERE ad.PopularityRank <= 10 ORDER BY ad.PopularityRank, ad.CreationDate DESC;
//
// Rank numbers the post x vote rows, so a post can take several of the five places and the GROUP BY then counts its comments once per place.
// Rows tied on CreationDate are ordered by post id, then vote id.
fn q21972(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(ts(2024, 9, 1, 0, 0, 0)));
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> = recent().select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    let v = top_per(drain(&j), |&((p, _), _)| post_type_id.get(p).unwrap(), |&((p, v), _)| (Reverse(creation_date.get(p).unwrap()), p, v), 5, false);
    type J = (Id<Post>, Option<Id<Vote>>);
    let fp = rel(v.into_iter().map(|x| x.0).collect::<Vec<J>>());
    let ud = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pwc = (&fp).group_by(Same::<J>::new().map(|(p, _): J| p)).select(Same::<J>::new().map(|(p, _): J| p).select(comments_of(db).opt())).fold(0i64, |n, c| n + c.is_some() as i64);
    let ad = ranked(drain((&pwc).and(&ud)), |&(_, (c, u))| (Reverse(u[0]), Reverse(c)), false);
    let ad = rel(ad.into_iter().take_while(|x| x.1 <= 10).collect::<Vec<_>>());
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let mh = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    type A = ((Id<Post>, (i64, [i64; 2])), i64);
    let v = drain((&ad).select(Same::<A>::new().and(Same::<A>::new().map(|((p, _), _): A| p).select(history_of(db).opt().and((&mh).opt())))));
    rows(v.into_iter().map(|(_, (((p, (c, u)), r), (_, m)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(u[0]), V::I(u[1]), V::I(c), V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }), V::I(r), m.map_or(V::Null, V::T)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.ViewCount, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank FROM Posts p WHERE p.PostTypeId IN (1, 2)),
// UserVoteStats AS (SELECT u.Id AS UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 13 THEN 1 END) AS UndeleteCount
//     FROM PostHistory ph GROUP BY ph.PostId),
// CombinedStats AS (SELECT rp.PostId, rp.OwnerUserId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ups.TotalVotes AS UserTotalVotes, ups.Upvotes AS UserUpvotes,
//        ups.Downvotes AS UserDownvotes, phs.CloseReopenCount, phs.DeleteCount, phs.UndeleteCount
//     FROM RankedPosts rp JOIN UserVoteStats ups ON rp.OwnerUserId = ups.UserId LEFT JOIN PostHistoryStats phs ON rp.PostId = phs.PostId)
// SELECT cs.PostId, cs.OwnerUserId, cs.Title, cs.CreationDate, cs.ViewCount, COALESCE(cs.UserTotalVotes, 0) AS UserTotalVotes, COALESCE(cs.UserUpvotes, 0) AS UserUpvotes,
//        COALESCE(cs.UserDownvotes, 0) AS UserDownvotes, COALESCE(cs.CloseReopenCount, 0) AS CloseReopenCount, COALESCE(cs.DeleteCount, 0) AS DeleteCount,
//        COALESCE(cs.UndeleteCount, 0) AS UndeleteCount, CASE WHEN cs.Score > 0 THEN 'Positive' WHEN cs.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreType,
//        CASE WHEN cs.CloseReopenCount > 0 THEN 'Has been closed/reopened' ELSE 'Not closed/reopened' END AS ClosureStatus
// FROM CombinedStats cs WHERE cs.UserTotalVotes IS NOT NULL AND cs.ViewCount > 0 ORDER BY cs.ViewCount DESC, cs.Score DESC;
fn q24811(db: &'static So) -> String {
    let Post { post_type_id, view_count, owner_user, score, .. } = &db.post;
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let pht = &db.post_history.post_history_type_id;
    let phs = db.post_history.group_by(&db.post_history.post).select(pht).fold([0i64; 3], |a, t| [a[0] + matches!(t, 10 | 11) as i64, a[1] + (t == 12) as i64, a[2] + (t == 13) as i64]);
    let v = drain(db.post.with(post_type_id.is_in([1, 2])).with(view_count.gt(0)).select(owner_user.select(&uv).and((&phs).opt())));
    rows(v.into_iter().map(|(p, (u, h))| {
        let s = score.get(p).unwrap();
        let h = h.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "owner_id", "title", "created", "views"]);
        f.extend(u.map(V::I));
        f.extend(h.map(V::I));
        f.push(V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }));
        f.push(V::S(if h[0] > 0 { "Has been closed/reopened" } else { "Not closed/reopened" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.PostTypeId, rp.PostRank, rp.CommentCount,
//        CASE WHEN rp.PostTypeId = 1 AND rp.PostRank = 1 THEN 'Top Question' WHEN rp.PostTypeId = 2 AND rp.PostRank <= 5 THEN 'Popular Answer' ELSE 'Other' END AS Category
//     FROM RankedPosts rp WHERE rp.ViewCount >= 100),
// PostHistoryCTE AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.UserId, p.Title AS PostTitle, p.PostTypeId, ph.Comment,
//        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.CreationDate >= cast('2024-10-01' as date) - INTERVAL '6 months'),
// UserEngagement AS (SELECT u.Id AS UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT fp.PostId, fp.Title, fp.Score, fp.ViewCount, fp.CommentCount, fp.Category, ph.PostHistoryTypeId, ph.CreationDate AS HistoryDate, ph.Comment AS HistoryComment,
//        ue.UserId, ue.UpVotes, ue.DownVotes, ue.BadgeCount
// FROM FilteredPosts fp LEFT JOIN PostHistoryCTE ph ON fp.PostId = ph.PostId AND ph.HistoryRank = 1 LEFT JOIN UserEngagement ue ON ue.UserId = fp.PostId
// WHERE fp.Category IN ('Top Question', 'Popular Answer') ORDER BY fp.Score DESC, fp.ViewCount DESC, ph.CreationDate DESC;
//
// PostRank numbers the post x comment rows; rows tied on Score are ordered by post id, then comment id, and HistoryRank ties by history id (latest first).
// `ue.UserId = fp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q21017(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(ts(2023, 10, 1, 0, 0, 0)));
    let j: MatSet<(Id<Post>, Option<Id<Comment>>)> = recent().select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    let r = per_group(ranked(drain(&j), |&((p, c), _)| (post_type_id.get(p).unwrap(), Reverse(score.get(p).unwrap()), p, c), false), |&((p, _), _)| post_type_id.get(p).unwrap());
    type R = (((Id<Post>, Option<Id<Comment>>), (Id<Post>, Option<Id<Comment>>)), i64);
    let cat = |(((p, _), _), k): R| {
        let t = post_type_id.get(p).unwrap();
        if t == 1 && k == 1 { "Top Question" } else if t == 2 && k <= 5 { "Popular Answer" } else { "Other" }
    };
    let fp = rel(drain(rel(r).filt(move |x: R| view_count.get((x.0 .0).0).map_or(false, |w| w >= 100) && cat(x) != "Other")).into_iter().map(|x| x.1).collect::<Vec<R>>());
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let ph = top_per(drain(db.post_history.with(hd.ge(ts(2024, 4, 1, 0, 0, 0))).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let phr = rel(ph.into_iter().map(|(h, p)| (p, h)).collect::<Vec<_>>());
    let last: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&phr).map(|(p, _)| p).inv().select(&phr).collect();
    let by_raw: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let hit: MatSet<Id<User>> = (&fp).map(|x: R| (x.0 .0).0).select(origid).select(&by_raw).collect();
    let ue = (&hit).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (t, _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let ub = (&hit).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pv = || Same::<R>::new().map(|x: R| (x.0 .0).0);
    let v = drain((&fp).select(Same::<R>::new().and(pv().select(&cc)).and(pv().select((&last).map(|(_, h)| h).opt())).and(pv().select(origid).select(&by_raw).select(Ident::<User>::new().and(&ue).and(&ub)).opt())));
    rows(v.into_iter().map(|(_, (((x, c), h), u))| {
        let p = (x.0 .0).0;
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::S(cat(x))]);
        f.extend(match h {
            Some(h) => [V::I(db.post_history.post_history_type_id.get(h).unwrap()), V::T(hd.get(h).unwrap()), ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match u {
            Some(((u, a), b)) => [user_col(db, u, "uid"), V::I(a[0]), V::I(a[1]), V::I(b)],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank,
//        p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.Reputation),
// PostVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostHistoryWithCloseReasons AS (SELECT ph.PostId, ph.CreationDate, ph.UserDisplayName, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Comment END) AS CloseReason,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.Comment END) AS ReopenReason FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)
//     GROUP BY ph.PostId, ph.CreationDate, ph.UserDisplayName),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, ur.Reputation, ur.BadgeCount, ur.HighestBadgeClass, pv.UpVotes, pv.DownVotes,
//        COALESCE(ph.CloseReason, 'Not Closed') AS CloseReason
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId
//     LEFT JOIN PostHistoryWithCloseReasons ph ON rp.PostId = ph.PostId WHERE rp.PostRank = 1)
// SELECT fr.PostId, fr.Title, fr.Score, fr.CreationDate, fr.Reputation, fr.BadgeCount, fr.HighestBadgeClass, fr.UpVotes, fr.DownVotes, fr.CloseReason,
//        CASE WHEN fr.CloseReason IS NOT NULL THEN 'Closed' ELSE 'Open' END AS Status,
//        CASE WHEN fr.Reputation < 100 THEN 'Newbie' WHEN fr.Reputation BETWEEN 100 AND 1000 THEN 'Intermediate' ELSE 'Expert' END AS UserLevel
// FROM FinalResults fr WHERE fr.Reputation IS NOT NULL ORDER BY fr.Score DESC, fr.CreationDate DESC LIMIT 100 OFFSET 0;
//
// PostRank ties are broken by post id. CloseReason is COALESCEd, so Status is always 'Closed'.
fn q23765(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, i64::MIN), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, user_display_name, comment, .. } = &db.post_history;
    let ph = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post.and(hd).and(user_display_name.opt()))
        .select(post_history_type_id.and(comment.opt()))
        .fold(None::<Str>, |m, (t, c)| if t == 10 { match (m, c) { (Some(a), Some(b)) => Some(if b > a { b } else { a }), (a, b) => a.or(b) } } else { m });
    let pr = rel(drain(&ph));
    type P = (((Id<Post>, i64), Option<Str>), Option<Str>);
    let by_post: HashIdx<Id<Post>, P> = (&pr).map(|(((p, _), _), _): P| p).inv().select(&pr).collect();
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&ur)).and((&pv).opt()).and((&by_post).opt())));
    let v = top_n(v, |&(p, (_, h))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, h.map(|x| (x.0 .0 .1, x.0 .1))), 100);
    rows(v.into_iter().map(|(p, (((u, (n, m)), pv), h))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        f.extend([V::I(r), V::I(n), if n == 0 { V::Null } else { V::I(m) }]);
        f.extend(match pv {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        f.extend([V::S(h.and_then(|x| x.1).unwrap_or("Not Closed")), V::S("Closed"), V::S(if r < 100 { "Newbie" } else if r <= 1000 { "Intermediate" } else { "Expert" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.Score > 0 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN RankedPosts rp ON v.PostId = rp.PostId GROUP BY v.PostId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistoryDetails AS (SELECT p.Id AS PostId, MAX(ph.CreationDate) AS LastEdited, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Comment END) AS CloseReason,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 END) AS EditCount FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.AcceptedAnswerId IS NULL GROUP BY p.Id),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, pv.UpVotes, pv.DownVotes, ub.BadgeCount, ph.LastEdited, ph.CloseReason, ph.EditCount,
//        ARRAY_LENGTH(string_to_array(rp.Tags, ','), 1) AS TagCount,
//        CASE WHEN EXISTS (SELECT 1 FROM PostLinks pl WHERE pl.PostId = rp.PostId) THEN TRUE ELSE FALSE END AS HasLinks
//     FROM RankedPosts rp LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId LEFT JOIN UserBadges ub ON rp.PostId = ub.UserId LEFT JOIN PostHistoryDetails ph ON rp.PostId = ph.PostId)
// SELECT PostId, Title, CreationDate, Score, UpVotes, DownVotes, BadgeCount, LastEdited, CloseReason, EditCount, TagCount, HasLinks FROM FinalResults
// WHERE (UpVotes - DownVotes) > 0 AND TagCount > 1 AND (LastEdited IS NULL OR LastEdited >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// ORDER BY Score DESC, CreationDate ASC LIMIT 100;
//
// `ub.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids. PostRank is never read.
fn q21213(db: &'static So) -> String {
    let Post { score, creation_date, tags_str, accepted_answer_id, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rp: MatSet<Id<Post>> = db.post.with(score.gt(0).and(creation_date.ge(add_years(t0, -1)))).collect();
    let pv = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let by_raw: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let phd = db.post.minus(accepted_answer_id).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd).and(comment.opt())).opt()).fold(
        (i64::MIN, None::<Str>, 0i64),
        |(m, r, n), h| match h {
            Some(((t, d), c)) => (m.max(d), if t == 10 { match (r, c) { (Some(a), Some(b)) => Some(if b > a { b } else { a }), (a, b) => a.or(b) } } else { r }, n + matches!(t, 4 | 5 | 6) as i64),
            None => (m, r, n),
        },
    );
    let tagc = tags_str.map(|t: Str| t.split(',').count() as i64);
    let fresh = add_days(t0, -30);
    let links = links_of(db);
    let v = drain((&rp).select(
        (&pv).filt(|a| a[0] - a[1] > 0).and(origid.select(&by_raw).select(&ub).opt()).and((&phd).opt().filt(move |h: Option<(i64, Option<Str>, i64)>| h.map_or(true, |h| h.0 == i64::MIN || h.0 >= fresh))).and(tagc.filt(|n| n > 1)).and(Ident::<Post>::new().with(links).opt()),
    ));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 100);
    rows(v.into_iter().map(|(p, ((((a, b), h), n), l))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), oint(b)]);
        f.extend(match h {
            Some((m, r, e)) => [tmax(m), ostr(r), V::I(e)],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::I(n), V::B(l.is_some())]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS RankByViews,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankByDate, p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1 AND p.ViewCount IS NOT NULL),
// QuestionBadges AS (SELECT b.UserId, MAX(b.Class) AS HighestBadgeClass, COUNT(b.Id) AS BadgeCount FROM Badges b JOIN Posts p ON b.UserId = p.OwnerUserId WHERE p.PostTypeId = 1
//     GROUP BY b.UserId),
// CommentsWithVoteCounts AS (SELECT c.Id AS CommentId, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount,
//        c.PostId, c.Text, c.CreationDate FROM Comments c LEFT JOIN Votes v ON c.PostId = v.PostId GROUP BY c.Id, c.PostId, c.Text, c.CreationDate),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.ViewCount, qb.BadgeCount, COALESCE(cw.UpVoteCount, 0) AS TotalUpVotes, COALESCE(cw.DownVoteCount, 0) AS TotalDownVotes,
//        CASE WHEN rp.RankByViews = 1 THEN 'Top Viewed Post' WHEN rp.RankByViews <= 5 THEN 'Top 5 Viewed Posts' ELSE 'Other Posts' END AS ViewRanking,
//        CASE WHEN qb.HighestBadgeClass = 1 THEN 'Gold' WHEN qb.HighestBadgeClass = 2 THEN 'Silver' ELSE 'Bronze or No Badge' END AS BadgeLevel
//     FROM RankedPosts rp LEFT JOIN QuestionBadges qb ON rp.OwnerUserId = qb.UserId LEFT JOIN CommentsWithVoteCounts cw ON rp.PostId = cw.PostId
//     WHERE rp.RankByViews <= 5 OR rp.RankByDate <= 5)
// SELECT fr.PostId, fr.Title, fr.ViewCount, fr.BadgeCount, fr.TotalUpVotes, fr.TotalDownVotes, fr.ViewRanking, fr.BadgeLevel,
//        CASE WHEN fr.TotalUpVotes = 0 AND fr.TotalDownVotes = 0 THEN 'No Votes' ELSE 'Votes Registered' END AS VoteStatus
// FROM FinalResults fr WHERE fr.BadgeCount > 0 OR fr.TotalUpVotes > 0 OR fr.TotalDownVotes > 0 ORDER BY fr.ViewCount DESC, fr.TotalUpVotes DESC LIMIT 50;
//
// Both ROW_NUMBERs break ties by post id; the ownerless questions are one partition. BadgeCount is counted over the badges x questions join.
fn q24730(db: &'static So) -> String {
    let Post { post_type_id, view_count, owner_user, creation_date, .. } = &db.post;
    let base = || db.post.with(post_type_id.eq(1)).with(view_count);
    let v = drain(base().select(owner_user.opt()));
    let byv = per_group(ranked(v.clone(), |&(p, u)| (u, Reverse(view_count.get(p).unwrap()), p), false), |&(_, u)| u);
    let byd = per_group(ranked(byv, |&((p, u), _)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&((_, u), _)| u);
    type R = (((Id<Post>, Option<Id<User>>), i64), i64);
    let rp = rel(drain(rel(byd).filt(|(((_, _), rv), rd): R| rv <= 5 || rd <= 5)).into_iter().map(|x| x.1).collect::<Vec<R>>());
    let asks = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let qb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).and(asks)).fold((0i64, i64::MIN), |(n, m), (c, _)| (n + 1, m.max(c)));
    let vt = &db.vote.vote_type_id;
    let cw = (&rp)
        .select(Same::<R>::new().map(|x: R| x.0 .0 .0).select(comments_of(db)))
        .group_by(Ident::<Comment>::new())
        .select((&db.comment.post).select(votes_of(db).select(vt).opt()))
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pk = Same::<R>::new().map(|x: R| x.0 .0 .0);
    let v = drain((&rp).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0 .1).flat_map(|u: Option<Id<User>>| u).select(&qb).opt()).and(pk.select(comments_of(db).select(&cw)).opt())));
    let v = drain(rel(v).filt(|(_, ((_, q), c)): (usize, ((R, Option<(i64, i64)>), Option<[i64; 2]>))| q.is_some() || c.map_or(false, |c| c[0] > 0 || c[1] > 0)));
    let v = top_n(v.into_iter().map(|x| x.1).collect(), |&(_, ((x, _), c)): &(usize, ((R, Option<(i64, i64)>), Option<[i64; 2]>))| (Reverse(view_count.get(x.0 .0 .0).unwrap()), Reverse(c.map_or(0, |c| c[0]))), 50);
    rows(v.into_iter().map(|(_, ((x, q), c))| {
        let (((p, _), rv), _) = x;
        let c = c.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([q.map_or(V::Null, |q| V::I(q.0)), V::I(c[0]), V::I(c[1])]);
        f.push(V::S(if rv == 1 { "Top Viewed Post" } else if rv <= 5 { "Top 5 Viewed Posts" } else { "Other Posts" }));
        f.push(V::S(match q.map(|q| q.1) { Some(1) => "Gold", Some(2) => "Silver", _ => "Bronze or No Badge" }));
        f.push(V::S(if c[0] == 0 && c[1] == 0 { "No Votes" } else { "Votes Registered" }));
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.Id AS PostHistoryId, ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.UserId, ph.Comment,
//        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS rn FROM PostHistory ph),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PopularTags AS (SELECT t.TagName, SUM(p.ViewCount) AS TotalViews, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON t.Id = p.Id GROUP BY t.TagName HAVING COUNT(p.Id) > 5),
// PostScores AS (SELECT p.Id AS PostId, p.Title, p.Score, COALESCE(ps.TotalViews, 0) AS TotalViews, COUNT(c.Id) AS CommentCount, SUM(v.BountyAmount) AS TotalBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PopularTags ps ON p.Tags LIKE '%' || ps.TagName || '%'
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.Score, ps.TotalViews),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.Score, ps.TotalViews, ps.CommentCount, ps.TotalBounty, RANK() OVER (ORDER BY ps.Score DESC, ps.TotalViews DESC) AS PostRank
//     FROM PostScores ps),
// FinalOutput AS (SELECT rp.*, u.DisplayName AS PostOwner, COALESCE(uRep.Reputation, 0) AS OwnerReputation, ph.Comment AS LastEditComment
//     FROM RankedPosts rp LEFT JOIN Posts p ON rp.PostId = p.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserReputation uRep ON u.Id = uRep.UserId
//     LEFT JOIN RecursivePostHistory ph ON p.Id = ph.PostId AND ph.rn = 1)
// SELECT * FROM FinalOutput WHERE OwnerReputation > 1000 ORDER BY PostRank LIMIT 10;
//
// `t.Id = p.Id` joins a tag id to a post id, so it goes through the raw ids (each tag meets at most one post, so PopularTags is empty here, but it is computed).
// PostScores groups the post x comment x vote x tag rows by (post, TotalViews). rn = 1 ties on CreationDate go to the larger history id.
fn q33388(db: &'static So) -> String {
    let Post { creation_date, score, view_count, tags_str, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let by_raw: HashIdx<i64, Id<Post>> = (origid).inv().collect();
    let pt = db.tag.group_by(&db.tag.tag_name).select((&db.tag.origid).select(&by_raw).select(view_count.opt())).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let ptv = rel(drain((&pt).filt(|a| a[0] > 5)));
    let pti: HashIdx<Str, (Str, [i64; 3])> = (&ptv).map(|(t, _)| t).inv().select(&ptv).collect();
    let recent: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_days(t0, -30))).collect();
    let full: MatSet<Str> = (&recent).select(tags_str).collect();
    let like: HashIdx<Str, (Str, [i64; 3])> = (&full).select_where(&pti, |f: Str, t: Str| f.contains(t)).collect();
    let tv = tags_str.select(&like).map(|(t, a): (Str, [i64; 3])| (t, if a[1] == 0 { None } else { Some(a[2]) }));
    type J = (((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>), Option<(Str, Option<i64>)>);
    let j: MatSet<J> = (&recent).select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt()).and(tv.opt())).collect();
    let ps = (&j)
        .group_by(Same::<J>::new().map(|(((p, _), _), t): J| (p, t.and_then(|t| t.1))))
        .select(Same::<J>::new().map(|(((_, c), v), _): J| (c, v)).and(Same::<J>::new().map(|(((_, _), v), _): J| v).flat_map(|v: Option<Id<Vote>>| v).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, ((c, _), b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let r = ranked(drain(&ps), |&((p, t), _)| (Reverse(score.get(p).unwrap()), Reverse(t.unwrap_or(0))), false);
    type R = (((Id<Post>, Option<i64>), [i64; 3]), i64);
    let rr = rel(r);
    let ur = db.user.with((&db.user.reputation).gt(1000));
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let last = top_per(drain(db.post_history.select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let lr = rel(last.into_iter().map(|(h, p)| (p, h)).collect::<Vec<_>>());
    let li: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&lr).map(|(p, _)| p).inv().select(&lr).collect();
    let rp = Same::<R>::new().map(|(((p, _), _), _): R| p);
    let v = drain((&rr).select(Same::<R>::new().and(rp.select(&db.post.owner_user).select(ur)).and(Same::<R>::new().map(|(((p, _), _), _): R| p).select((&li).map(|(_, h)| h).opt()))));
    let v = top_n(v, |&(_, ((((_, _), k), _), _))| k, 10);
    rows(v.into_iter().map(|(_, (((((p, t), a), k), u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(t.unwrap_or(0)), V::I(a[0]), nullable(a[2], a[1]), V::I(k)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(h.map_or(V::Null, |h| ostr(db.post_history.comment.get(h))));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostVoteStats AS (SELECT p.OwnerUserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// AcceptedAnswers AS (SELECT p.OwnerUserId, COUNT(a.Id) AS AcceptedAnswersCount FROM Posts p LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id WHERE p.PostTypeId = 1
//     GROUP BY p.OwnerUserId),
// UserActivity AS (SELECT u.Id AS UserId, COALESCE(b.TotalBadges, 0) AS TotalBadges, COALESCE(ps.TotalVotes, 0) AS TotalVotes, COALESCE(ps.UpVotes, 0) AS UpVotes,
//        COALESCE(ps.DownVotes, 0) AS DownVotes, COALESCE(aa.AcceptedAnswersCount, 0) AS AcceptedAnswersCount, u.Reputation
//     FROM Users u LEFT JOIN UserBadges b ON u.Id = b.UserId LEFT JOIN PostVoteStats ps ON u.Id = ps.OwnerUserId LEFT JOIN AcceptedAnswers aa ON u.Id = aa.OwnerUserId),
// FinalStats AS (SELECT UserId, TotalBadges, TotalVotes, UpVotes, DownVotes, AcceptedAnswersCount, Reputation, RANK() OVER (ORDER BY Reputation DESC, TotalVotes DESC) AS Rank
//     FROM UserActivity WHERE Reputation > 1000)
// SELECT u.DisplayName, fs.TotalBadges, fs.TotalVotes, fs.UpVotes, fs.DownVotes, fs.AcceptedAnswersCount, fs.Reputation, fs.Rank,
//        CASE WHEN fs.UpVotes > fs.DownVotes THEN 'Positive Contributor' WHEN fs.UpVotes < fs.DownVotes THEN 'Negative Contributor' ELSE 'Neutral Contributor' END AS ContributorType
// FROM FinalStats fs JOIN Users u ON fs.UserId = u.Id WHERE fs.Rank <= 10 ORDER BY fs.Rank;
fn q24968(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, accepted_answer, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pvs = db.post.with(creation_date.ge(ts(2023, 10, 1, 0, 0, 0))).group_by(owner_user).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let aa = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(accepted_answer.opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let rep = &db.user.reputation;
    let v = drain(db.user.with(rep.gt(1000)).select((&ub).and((&pvs).opt()).and((&aa).opt())));
    let v = ranked(v, |&(u, ((_, p), _))| (Reverse(rep.get(u).unwrap()), Reverse(p.map_or(0, |a| a[0]))), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((b, p), a)), k)| {
        let p = p.unwrap_or([0; 3]);
        row(vec![user_col(db, u, "name"), V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(a.unwrap_or(0)), user_col(db, u, "rep"), V::I(k),
            V::S(if p[1] > p[2] { "Positive Contributor" } else if p[1] < p[2] { "Negative Contributor" } else { "Neutral Contributor" })])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.CreationDate, p.Title, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank FROM Posts p WHERE p.PostTypeId = 1),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostHistoryAnalysis AS (SELECT ph.PostId, MIN(ph.CreationDate) AS FirstEditDate, COUNT(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 END) AS EditCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount FROM PostHistory ph
//     WHERE ph.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY ph.PostId),
// VisiblePosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerUserId, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, pha.FirstEditDate,
//        pha.EditCount, pha.CloseCount, ROW_NUMBER() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS PostRank
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN PostHistoryAnalysis pha ON rp.PostId = pha.PostId WHERE rp.OwnerPostRank = 1
//     ORDER BY u.Reputation DESC, rp.Score DESC),
// FinalResults AS (SELECT vp.*, CASE WHEN vp.CloseCount > 0 THEN 'Closed' WHEN vp.EditCount > 0 THEN 'Edited' ELSE 'Original' END AS PostStatus,
//        CASE WHEN vp.OwnerReputation >= 1000 THEN 'Experienced' WHEN vp.OwnerReputation < 0 THEN 'Novice' ELSE 'Intermediate' END AS OwnerExperienceLevel FROM VisiblePosts vp)
// SELECT fr.PostId, fr.Title, fr.CreationDate, fr.OwnerDisplayName, fr.PostStatus, fr.OwnerExperienceLevel FROM FinalResults fr WHERE fr.PostRank <= 50
// ORDER BY fr.OwnerExperienceLevel, fr.PostRank;
//
// UserActivity is never referenced by the final SELECT, so it is not computed. Both ROW_NUMBERs break ties by post id.
fn q21224(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let v = top_n(top, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 50);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.with(hd.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post).select(post_history_type_id).fold([0i64; 2], |a, t| {
        [a[0] + matches!(t, 4 | 5 | 6) as i64, a[1] + (t == 10) as i64]
    });
    let v = drain((&tp).select(owner_user.and((&pha).opt())));
    rows(v.into_iter().map(|(p, (u, h))| {
        let r = db.user.reputation.get(u).unwrap();
        let h = h.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.push(V::S(if h[1] > 0 { "Closed" } else if h[0] > 0 { "Edited" } else { "Original" }));
        f.push(V::S(if r >= 1000 { "Experienced" } else if r < 0 { "Novice" } else { "Intermediate" }));
        row(f)
    }))
}

// WITH RankedQuestions AS (SELECT p.Id AS QuestionId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, u.Reputation,
//        ROW_NUMBER() OVER (PARTITION BY u.Reputation ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.PostTypeId = 1 AND u.Reputation >= 1000),
// CommentStatistics AS (SELECT PostId, AVG(Score) AS AvgCommentScore, MAX(CreationDate) AS LatestCommentDate FROM Comments GROUP BY PostId),
// CloseReasonCounts AS (SELECT PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount
//     FROM PostHistory ph GROUP BY PostId),
// RecentActivities AS (SELECT p.Id AS PostId, MAX(ph.CreationDate) AS LastActivityDate, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId = 24) AS EditCount
//     FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '365 days' GROUP BY p.Id),
// FinalSelection AS (SELECT q.QuestionId, q.Title, q.Score, q.CreationDate, q.RankScore, cs.AvgCommentScore, cs.LatestCommentDate, rc.CloseCount, rc.ReopenCount,
//        ra.LastActivityDate, ra.EditCount
//     FROM RankedQuestions q LEFT JOIN CommentStatistics cs ON cs.PostId = q.QuestionId LEFT JOIN CloseReasonCounts rc ON rc.PostId = q.QuestionId
//     LEFT JOIN RecentActivities ra ON ra.PostId = q.QuestionId WHERE q.RankScore <= 5)
// SELECT fs.QuestionId, fs.Title, fs.Score, fs.CreationDate, fs.AvgCommentScore, fs.LatestCommentDate, fs.CloseCount, fs.ReopenCount, fs.LastActivityDate, fs.EditCount,
//        COALESCE(d.DOWN, 0) AS DownVoteCount
// FROM FinalSelection fs LEFT JOIN (SELECT PostId, COUNT(*) AS DOWN FROM Votes WHERE VoteTypeId = 3 GROUP BY PostId) d ON d.PostId = fs.QuestionId
// ORDER BY fs.Score DESC, fs.CreationDate ASC LIMIT 10;
//
// RankScore numbers the question x comment rows, so a question with several comments takes several places; ties go to post id, then comment id.
fn q22696(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let rep = &db.user.reputation;
    let qs = db.post.with(post_type_id.eq(1)).with(owner_user.select(Ident::<User>::new().with(rep.ge(1000))));
    let j: MatSet<(Id<Post>, Option<Id<Comment>>)> = qs.select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    let top = top_per(drain(&j), |&((p, _), _)| rep.get(owner_user.get(p).unwrap()).unwrap(), |&((p, c), _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, c), 5, false);
    type J = (Id<Post>, Option<Id<Comment>>);
    let fs = rel(top.into_iter().map(|x| x.0).collect::<Vec<J>>());
    let Comment { post: cpost, score: cscore, creation_date: cd, .. } = &db.comment;
    let cs = db.comment.group_by(cpost).select(cscore.and(cd)).fold([0, 0, i64::MIN], |a, (s, d)| [a[0] + 1, a[1] + s, a[2].max(d)]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let rc = db.post_history.group_by(post).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let ra = db.post_history.with(hd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -365))).group_by(post).select(post_history_type_id.and(hd)).fold((i64::MIN, 0i64), |(m, n), (t, d)| (m.max(d), n + (t == 24) as i64));
    let dn = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(3)));
    let dc = db.post.group_by(Ident::<Post>::new()).select(dn).fold(0i64, |n, _| n + 1);
    let pk = || Same::<J>::new().map(|(p, _): J| p);
    let v = drain((&fs).select(pk().and(pk().select((&cs).opt())).and(pk().select((&rc).opt())).and(pk().select((&ra).opt())).and(pk().select((&dc).opt()))));
    let v = top_n(v, |&(i, ((((p, _), _), _), _))| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), i), 10);
    rows(v.into_iter().map(|(_, ((((p, c), r), a), d))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        f.extend(match c {
            Some(c) => [avg(c[1], c[0]), V::T(c[2])],
            None => [V::Null, V::Null],
        });
        f.extend(match r {
            Some(r) => [V::I(r[0]), V::I(r[1])],
            None => [V::Null, V::Null],
        });
        f.extend(match a {
            Some((m, n)) => [V::T(m), V::I(n)],
            None => [V::Null, V::Null],
        });
        f.push(V::I(d.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COALESCE(MAX(b.Date), DATE '1900-01-01') AS LastBadgeDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, p.PostTypeId),
// FeaturedPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.CreationDate, rp.ScoreRank, rp.CommentCount,
//        CASE WHEN rp.ScoreRank = 1 THEN 'Featured' WHEN rp.Score > 10 THEN 'Popular' ELSE 'Regular' END AS PopularityStatus,
//        CASE WHEN rp.LastBadgeDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 DAYS' THEN 'Active Contributor' ELSE 'Inactive Contributor' END AS ContributorStatus
//     FROM RankedPosts rp),
// ExternalLinks AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS RelatedLinksCount FROM PostLinks pl GROUP BY pl.PostId),
// FinalReport AS (SELECT fp.PostId, fp.Title, fp.ViewCount, fp.Score, fp.PopularityStatus, fp.CommentCount, ep.RelatedLinksCount, fp.ContributorStatus,
//        CASE WHEN fp.PopularityStatus = 'Featured' AND ep.RelatedLinksCount > 0 THEN 'Highly Recommended' ELSE 'Standard Recommendation' END AS Recommendation
//     FROM FeaturedPosts fp LEFT JOIN ExternalLinks ep ON fp.PostId = ep.PostId)
// SELECT fr.PostId, fr.Title, fr.ViewCount, fr.Score, fr.PopularityStatus, fr.CommentCount, fr.RelatedLinksCount, fr.ContributorStatus, fr.Recommendation
// FROM FinalReport fr WHERE fr.CommentCount > 0 AND (fr.PopularityStatus = 'Featured' OR fr.Score > 15) ORDER BY fr.Score DESC, fr.ViewCount DESC;
fn q22452(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let recent = || db.post.with(creation_date.ge(add_years(t0, -1)));
    let bd = owner_user.select(badges_of(db)).select(&db.badge.date);
    let rp = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt()).and(bd.opt())).fold((0i64, i64::MIN), |(n, m), ((c, _), d)| {
        (n + c.is_some() as i64, d.map_or(m, |d| m.max(d)))
    });
    let r = per_group(ranked(drain(&rp), |&(p, _)| (post_type_id.get(p).unwrap(), Reverse(score.get(p).unwrap())), false), |&(p, _)| post_type_id.get(p).unwrap());
    type R = ((Id<Post>, (i64, i64)), i64);
    let status = |((p, _), k): R| if k == 1 { "Featured" } else if score.get(p).unwrap() > 10 { "Popular" } else { "Regular" };
    let fr = rel(drain(rel(r).filt(move |x: R| (x.0 .1).0 > 0 && (status(x) == "Featured" || score.get(x.0 .0).unwrap() > 15))).into_iter().map(|x| x.1).collect::<Vec<R>>());
    let el = db.post.group_by(Ident::<Post>::new()).select(links_of(db)).fold(0i64, |n, _| n + 1);
    let cut = add_days(t0, -30);
    let v = drain((&fr).select(Same::<R>::new().and(Same::<R>::new().map(|((p, _), _): R| p).select((&el).opt()))));
    rows(v.into_iter().map(|(_, (x, l))| {
        let ((p, (n, m)), _) = x;
        let st = status(x);
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::S(st), V::I(n), oint(l), V::S(if m != i64::MIN && m > cut { "Active Contributor" } else { "Inactive Contributor" })]);
        f.push(V::S(if st == "Featured" && l.map_or(false, |l| l > 0) { "Highly Recommended" } else { "Standard Recommendation" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Date) AS MostRecentBadgeDate, STRING_AGG(b.Name, ', ') AS BadgeNames
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStatistics AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpvoteCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownvoteCount, MAX(COALESCE(p.ClosedDate, p.LastActivityDate)) AS LastActivity
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
//     GROUP BY p.Id, p.OwnerUserId),
// ClosedPosts AS (SELECT ph.PostId, STRING_AGG(CASE WHEN ph.PostHistoryTypeId = 10 THEN cr.Name END, ', ') AS CloseReasons, MIN(ph.CreationDate) AS ClosedOn,
//        COUNT(DISTINCT ph.UserId) FILTER (WHERE ph.PostHistoryTypeId = 10) AS UniqueCloseVoters
//     FROM PostHistory ph JOIN CloseReasonTypes cr ON ph.Comment::int = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// MergedPosts AS (SELECT p.Id AS PostId, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostCount, MAX(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS IsQuestion
//     FROM Posts p LEFT JOIN PostLinks pl ON p.Id = pl.PostId GROUP BY p.Id),
// GranularMetrics AS (SELECT ps.PostId, ps.CommentCount, ub.BadgeCount, mp.RelatedPostCount, mp.IsQuestion, ps.LastActivity, cp.CloseReasons, cp.ClosedOn, cp.UniqueCloseVoters
//     FROM PostStatistics ps JOIN UserBadges ub ON ps.OwnerUserId = ub.UserId LEFT JOIN MergedPosts mp ON ps.PostId = mp.PostId LEFT JOIN ClosedPosts cp ON ps.PostId = cp.PostId)
// SELECT gm.PostId, gm.CommentCount, gm.BadgeCount, gm.RelatedPostCount, gm.IsQuestion, gm.LastActivity, gm.CloseReasons, gm.ClosedOn, gm.UniqueCloseVoters,
//        CASE WHEN gm.UniqueCloseVoters > 0 THEN 'Closed by Users' ELSE 'Not Closed' END AS ClosureStatus,
//        CASE WHEN gm.LastActivity < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 'Stale' ELSE 'Active' END AS ActivityStatus
// FROM GranularMetrics gm WHERE gm.BadgeCount > 5 ORDER BY gm.PostId DESC;
//
// BadgeNames and MostRecentBadgeDate are never read, so they are not computed. The CloseReasons STRING_AGG has no ORDER BY; the port joins the names in
// PostHistory id order.
fn q23186(db: &'static So) -> String {
    let Post { creation_date, owner_user, closed_date, last_activity_date, post_type_id, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(ts(2023, 10, 1, 0, 0, 0)));
    let ps = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, user_id, .. } = &db.post_history;
    let closes = || db.post_history.with(post_history_type_id.eq(10)).with(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason));
    let names = closes().group_by(post).select(Ident::<PostHistory>::new().and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))).buf_fold(|xs| -> &'static str {
        let mut v: Vec<(Id<PostHistory>, Str)> = xs.iter().copied().collect();
        v.sort();
        Box::leak(v.iter().map(|x| x.1).collect::<Vec<_>>().join(", ").into_boxed_str())
    });
    let first = closes().group_by(post).select(hd).fold(i64::MAX, |m, d| m.min(d));
    let voters = closes().group_by(post).select(user_id.opt()).buf_fold(|xs| distinct_some(xs.iter().copied()));
    let mp = db.post.group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id).opt()).buf_fold(|xs| distinct_some(xs.iter().copied()));
    let v = drain(recent().select((&ps).and(owner_user.select((&ub).filt(|n| n > 5))).and(&mp).and((&names).and(&first).and(&voters).opt())));
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    rows(v.into_iter().map(|(p, (((a, b), m), c))| {
        let la = closed_date.get(p).unwrap_or(last_activity_date.get(p).unwrap());
        let mut f = post_fields(db, p, &["id"]);
        f.extend([V::I(a[0]), V::I(b), V::I(m), V::I((post_type_id.get(p).unwrap() == 1) as i64), V::T(la)]);
        f.extend(match c {
            Some(((n, d), u)) => [V::S(n), V::T(d), V::I(u)],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if c.map_or(false, |x| x.1 > 0) { "Closed by Users" } else { "Not Closed" }));
        f.push(V::S(if la < cut { "Stale" } else { "Active" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalQuestions, SUM(p.Score) AS TotalScore, SUM(COALESCE(b.Class, 0)) AS TotalBadgeClass,
//        SUM(COALESCE(v.BountyAmount, 0)) AS TotalBountyAmount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.TotalQuestions, us.TotalScore, us.TotalBadgeClass, us.TotalBountyAmount, RANK() OVER (ORDER BY us.TotalScore DESC) AS UserRank
//     FROM UserStatistics us WHERE us.TotalQuestions > 0),
// MostVotedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, SUM(v.BountyAmount) AS TotalBounty, COUNT(v.Id) AS VoteCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.OwnerUserId),
// PostActivity AS (SELECT ph.PostId, ph.UserId, ph.PostHistoryTypeId, ph.CreationDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// RecentActivity AS (SELECT p.OwnerUserId, COUNT(pa.PostId) AS RecentActivityCount FROM Posts p JOIN PostActivity pa ON p.Id = pa.PostId
//     WHERE pa.CreationDate > CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY p.OwnerUserId)
// SELECT tu.UserId, tu.DisplayName, tu.TotalQuestions, tu.TotalScore, tu.TotalBadgeClass, tu.TotalBountyAmount, COALESCE(ra.RecentActivityCount, 0) AS RecentActivity,
//        mp.Title AS MostVotedPostTitle, mp.TotalBounty, mp.VoteCount
// FROM TopUsers tu LEFT JOIN RecentActivity ra ON tu.UserId = ra.OwnerUserId LEFT JOIN MostVotedPosts mp ON tu.UserId = mp.OwnerUserId WHERE tu.UserRank <= 10
// ORDER BY tu.TotalScore DESC;
fn q33065(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let asks = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(asks.select(score).opt().and(badges_of(db).select(&db.badge.class).opt()).and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 4], |a, ((s, c), b)| [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0), a[2] + c.unwrap_or(0), a[3] + b.flatten().unwrap_or(0)]);
    let tq = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)))).fold(0i64, |n, _| n + 1);
    let v = ranked(drain((&us).and(&tq)), |&(_, (a, _))| (a[0] == 0, Reverse(a[1])), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect::<Vec<Id<User>>>());
    let since = now_utc() - 30 * DAY_US;
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ra = db.post_history.with(post_history_type_id.is_in([10, 11, 12])).with(hd.filt(move |d: i64| ny_to_utc(d) > since)).group_by(post.select(owner_user)).select(post).fold(0i64, |n, _| n + 1);
    let Vote { bounty_amount, .. } = &db.vote;
    let mv = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(votes_of(db).select(bounty_amount.opt()).opt()).fold([0i64; 3], |a, b| match b {
        Some(b) => [a[0] + 1, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)],
        None => a,
    });
    let owned = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)).and(&mv));
    let v = drain((&tu).select(Ident::<User>::new().and(&us).and(&tq).and((&ra).opt()).and(owned.opt())));
    rows(v.into_iter().map(|(_, ((((u, a), q), r), m))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(q), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(r.unwrap_or(0))]);
        f.extend(match m {
            Some((p, b)) => [title(db, p), nullable(b[2], b[1]), V::I(b[0])],
            None => [V::Null, V::Null, V::I(0)],
        });
        row(f)
    }))
}

// WITH RECURSIVE UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, B.Name AS BadgeName, B.Class, B.Date, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY B.Date DESC) AS BadgeRank
//     FROM Users U JOIN Badges B ON U.Id = B.UserId),
// TopUsers AS (SELECT Id, DisplayName, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users
//     WHERE CreationDate > (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate > (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
//     GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId, P.Score),
// UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostsCount, SUM(P.Score) AS TotalScore, AVG(P.Score) AS AvgScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// PostHistorySummary AS (SELECT PH.PostId, MIN(PH.CreationDate) AS FirstEditDate, MAX(PH.CreationDate) AS LastEditDate, COUNT(PH.Id) AS EditCount,
//        SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount FROM PostHistory PH GROUP BY PH.PostId)
// SELECT U.DisplayName AS Author, U.Reputation, UBS.BadgeName, R.Score AS RecentPostScore, R.CommentCount AS RecentPostComments, PHS.FirstEditDate, PHS.LastEditDate,
//        PHS.EditCount, PHS.CloseCount, UPosts.PostsCount, UPosts.TotalScore, UPosts.AvgScore,
//        CASE WHEN U.Reputation >= 1000 THEN 'High Reputation' WHEN U.Reputation >= 500 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
// FROM TopUsers U LEFT JOIN UserBadges UBS ON U.Id = UBS.UserId AND UBS.BadgeRank = 1 LEFT JOIN RecentPosts R ON U.Id = R.OwnerUserId
// LEFT JOIN UserPostStats UPosts ON U.Id = UPosts.UserId LEFT JOIN PostHistorySummary PHS ON R.PostId = PHS.PostId
// WHERE U.Reputation > 200 ORDER BY U.Reputation DESC, R.Score DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. BadgeRank ties on Date go to the smaller badge id.
fn q30741(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let User { creation_date: ucd, reputation, .. } = &db.user;
    let tu = db.user.with(ucd.gt(add_years(t0, -1))).with(reputation.gt(200));
    let Badge { user, date, .. } = &db.badge;
    let b1 = top_per(drain(db.badge.select(user)), |&(_, u)| u, |&(b, _)| (Reverse(date.get(b).unwrap()), b), 1, false);
    let br = rel(b1.into_iter().map(|(b, u)| (u, b)).collect::<Vec<_>>());
    let bi: HashIdx<Id<User>, (Id<User>, Id<Badge>)> = (&br).map(|(u, _)| u).inv().select(&br).collect();
    let Post { creation_date, score, .. } = &db.post;
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.gt(add_days(t0, -30))));
    let rc = db.post.with(creation_date.gt(add_days(t0, -30))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold([i64::MAX, i64::MIN, 0, 0], |a, (t, d)| [a[0].min(d), a[1].max(d), a[2] + 1, a[3] + (t == 10) as i64]);
    let v = drain(tu.select((&bi).map(|(_, b)| b).opt().and(recent.select(Ident::<Post>::new().and(&rc).and((&phs).opt())).opt()).and(&ups)));
    rows(v.into_iter().map(|(u, ((b, r), a))| {
        let rep = reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(b.map_or(V::Null, |b| V::S(db.badge.name.get(b).unwrap())));
        f.extend(match r {
            Some(((p, c), h)) => {
                let mut g = vec![V::I(score.get(p).unwrap()), V::I(c)];
                g.extend(match h {
                    Some(h) => [V::T(h[0]), V::T(h[1]), V::I(h[2]), V::I(h[3])],
                    None => [V::Null, V::Null, V::Null, V::Null],
                });
                g
            }
            None => (0..6).map(|_| V::Null).collect(),
        });
        f.extend([V::I(a[0]), nullable(a[1], a[0]), avg(a[1], a[0])]);
        f.push(V::S(if rep >= 1000 { "High Reputation" } else if rep >= 500 { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.LastActivityDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.LastActivityDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.LastActivityDate, rp.Score, rp.ViewCount, COALESCE(c.Count, 0) AS CommentCount, COALESCE(v.upVotes, 0) AS UpVoteCount,
//        COALESCE(v.downVotes, 0) AS DownVoteCount, (COALESCE(v.upVotes, 0) - COALESCE(v.downVotes, 0)) AS NetVoteCount
//     FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS Count FROM Comments GROUP BY PostId) c ON rp.PostId = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS upVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS downVotes FROM Votes GROUP BY PostId) v
//     ON rp.PostId = v.PostId WHERE rp.rn = 1),
// PostHistoryStats AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS HistoryCount FROM PostHistory ph GROUP BY ph.PostId, ph.PostHistoryTypeId),
// FinalMetrics AS (SELECT pm.PostId, pm.Title, pm.LastActivityDate, pm.Score, pm.ViewCount, pm.CommentCount, pm.UpVoteCount, pm.DownVoteCount, pm.NetVoteCount,
//        COALESCE(SUM(CASE WHEN phs.PostHistoryTypeId = 10 THEN phs.HistoryCount END), 0) AS CloseHistoryCount,
//        COALESCE(SUM(CASE WHEN phs.PostHistoryTypeId = 11 THEN phs.HistoryCount END), 0) AS ReopenHistoryCount
//     FROM PostMetrics pm LEFT JOIN PostHistoryStats phs ON pm.PostId = phs.PostId
//     GROUP BY pm.PostId, pm.Title, pm.LastActivityDate, pm.Score, pm.ViewCount, pm.CommentCount, pm.UpVoteCount, pm.DownVoteCount, pm.NetVoteCount)
// SELECT p.PostId, p.Title, p.LastActivityDate, p.Score, p.ViewCount, p.CommentCount, p.UpVoteCount, p.DownVoteCount, p.NetVoteCount, p.CloseHistoryCount, p.ReopenHistoryCount,
//        CASE WHEN p.ReopenHistoryCount > 0 THEN 'Post has been reopened' WHEN p.CloseHistoryCount > 0 THEN 'Post has been closed' ELSE 'Post status normal' END AS PostStatus
// FROM FinalMetrics p ORDER BY p.LastActivityDate DESC LIMIT 100;
//
// rn ties on LastActivityDate go to the smaller post id. PostHistoryStats is summed per type, which is the per-post count of that type.
fn q34350(db: &'static So) -> String {
    let Post { post_type_id, creation_date, last_activity_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(last_activity_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let phs = db.post_history.group_by(post.and(post_history_type_id)).select(post).fold(0i64, |n, _| n + 1);
    let pr = rel(drain(&phs));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&pr).map(|((p, _), _)| p).inv().select(&pr).collect();
    let fm = (&tp).group_by(Ident::<Post>::new()).select((&by_post).opt()).fold([0i64; 2], |a, h| match h {
        Some(((_, t), n)) => [a[0] + if t == 10 { n } else { 0 }, a[1] + if t == 11 { n } else { 0 }],
        None => a,
    });
    let v = top_n(drain((&cc).and(&vc).and(&fm)), |&(p, _)| (Reverse(last_activity_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((c, u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "activity", "score", "views"]);
        f.extend([V::I(c), V::I(u[0]), V::I(u[1]), V::I(u[0] - u[1]), V::I(h[0]), V::I(h[1])]);
        f.push(V::S(if h[1] > 0 { "Post has been reopened" } else if h[0] > 0 { "Post has been closed" } else { "Post status normal" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT US.Id AS UserId, US.DisplayName, COUNT(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 END) AS VoteCount, SUM(COALESCE(P.Score, 0)) AS TotalPostScore,
//        SUM(COALESCE(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END, 0)) AS QuestionCount, SUM(COALESCE(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END, 0)) AS AnswerCount
//     FROM Users US LEFT JOIN Posts P ON US.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY US.Id, US.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, US.DisplayName AS OwnerName, P.Score, P.ViewCount,
//        DENSE_RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS ScoreRank, DENSE_RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.ViewCount DESC) AS ViewRank
//     FROM Posts P JOIN Users US ON P.OwnerUserId = US.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, CRT.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes CRT ON CAST(PH.Comment AS INTEGER) = CRT.Id
//     WHERE PH.PostHistoryTypeId = 10),
// CombinedStats AS (SELECT UA.UserId, UA.DisplayName, UA.VoteCount, UA.TotalPostScore, UA.QuestionCount, UA.AnswerCount, PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.ViewCount,
//        CASE WHEN PS.ScoreRank <= 10 THEN 'Top 10 by Score' ELSE 'Others' END AS ScoreCategory, CASE WHEN PS.ViewRank <= 10 THEN 'Top 10 by Views' ELSE 'Others' END AS ViewCategory,
//        COALESCE(CP.CloseReason, 'Not Closed') AS PostCloseReason
//     FROM UserActivity UA LEFT JOIN PostStatistics PS ON UA.DisplayName = PS.OwnerName LEFT JOIN ClosedPosts CP ON PS.PostId = CP.PostId)
// SELECT UserId, DisplayName, VoteCount, TotalPostScore, QuestionCount, AnswerCount, PostId, Title, CreationDate, Score, ViewCount, ScoreCategory, ViewCategory, PostCloseReason
// FROM CombinedStats WHERE TotalPostScore > 50 AND (QuestionCount > 5 OR AnswerCount > 10) ORDER BY TotalPostScore DESC, CreationDate DESC;
fn q2886(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, view_count, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((t, s), v)) => [a[0] + matches!(v, Some(2 | 3)) as i64, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64],
            None => a,
        });
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let sr = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap())), true), |&(_, t)| t);
    let vr = per_group(ranked(sr, |&((p, t), _)| {
        let w = view_count.get(p);
        (t, w.is_none(), Reverse(w))
    }, true), |&((_, t), _)| t);
    type P = (((Id<Post>, i64), i64), i64);
    let pr = rel(vr);
    let by_name: HashIdx<Str, P> = (&pr).map(|(((p, _), _), _): P| p).select(owner_user).select(&db.user.display_name).inv().select(&pr).collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason));
    let psj = (&by_name).select(Same::<P>::new().and(Same::<P>::new().map(|(((p, _), _), _): P| p).select(cp.opt())));
    let v = drain(db.user.select((&ua).filt(|a| a[1] > 50 && (a[2] > 5 || a[3] > 10)).and((&db.user.display_name).select(psj).opt())));
    rows(v.into_iter().map(|(u, (a, x))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        match x {
            Some(((((p, _), s), w), r)) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
                f.extend([V::S(if s <= 10 { "Top 10 by Score" } else { "Others" }), V::S(if w <= 10 { "Top 10 by Views" } else { "Others" }), V::S(r.unwrap_or("Not Closed"))]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::S("Others"), V::S("Others"), V::S("Not Closed")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate > CURRENT_DATE - INTERVAL '1 year'),
// UserVoteStatistics AS (SELECT u.Id AS UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN v.VoteTypeId = 5 THEN 1 ELSE 0 END) AS Favorites FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.UserId, ph.Comment, DENSE_RANK() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS RevisionRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// RecentActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate, CURRENT_DATE - u.LastAccessDate AS DaysInactive FROM Users u
//     WHERE u.LastAccessDate > CURRENT_DATE - INTERVAL '6 months')
// SELECT rp.PostId, rp.Title, rp.Score AS PostScore, rp.ViewCount, u.DisplayName AS AuthorDisplayName, u.Reputation AS AuthorReputation, ubc.BadgeCount,
//        COALESCE(UVS.UpVotes, 0) AS UserUpVotes, COALESCE(UVS.DownVotes, 0) AS UserDownVotes, COALESCE(UVS.Favorites, 0) AS UserFavorites, phd.Comment AS LastPostHistoryComment,
//        phd.CreationDate AS LastPostHistoryDate, phd.PostHistoryTypeId, CASE WHEN DaysInactive <= INTERVAL '30 days' THEN 'Active' ELSE 'Inactive' END AS UserActivityStatus
// FROM RankedPosts rp JOIN Users u ON rp.PostId = u.Id LEFT JOIN UserVoteStatistics UVS ON u.Id = UVS.UserId LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId
// LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId AND phd.RevisionRank = 1 JOIN RecentActiveUsers rau ON u.Id = rau.UserId
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, u.Reputation DESC;
//
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. Rank ties on Score go to the smaller post id.
fn q34440(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let today = current_date();
    let v = drain(db.post.with(creation_date.gt(add_years(today, -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_raw: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let User { last_access_date, .. } = &db.user;
    let rau = Ident::<User>::new().with(last_access_date.gt(add_months(today, -6)));
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(5)) as i64]
    });
    let ubc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = || db.post_history.with(post_history_type_id.is_in([10, 11, 12]));
    let md = ph().group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = ph().select(post.and(hd)).inv().collect();
    let v = drain((&tp).select(origid.select(&by_raw).select(rau.and(&uvs).and(&ubc)).and(Ident::<Post>::new().and(&md).select(&at).opt())));
    rows(v.into_iter().map(|(p, (((u, a), b), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match h {
            Some(h) => [ostr(db.post_history.comment.get(h)), V::T(hd.get(h).unwrap()), V::I(post_history_type_id.get(h).unwrap())],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if today - last_access_date.get(u).unwrap() <= 30 * DAY_US { "Active" } else { "Inactive" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, MAX(v.CreationDate) OVER (PARTITION BY p.Id) AS LastVoteDate
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate AS HistoryCreatedDate, p.Title AS PostTitle,
//        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph JOIN Posts p ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11, 12, 13)),
// InterestingVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v GROUP BY v.PostId),
// FinalPostAggregation AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, COALESCE(iv.VoteCount, 0) AS TotalVotes,
//        COALESCE(iv.UpVotes, 0) AS TotalUpVotes, COALESCE(iv.DownVotes, 0) AS TotalDownVotes, COALESCE(phd.PostHistoryTypeId, -1) AS RecentHistoryId,
//        CASE WHEN phd.HistoryRank = 1 THEN phd.HistoryCreatedDate ELSE NULL END AS MostRecentHistoryDate
//     FROM RankedPosts rp LEFT JOIN InterestingVotes iv ON iv.PostId = rp.PostId LEFT JOIN PostHistoryDetails phd ON phd.PostId = rp.PostId AND phd.HistoryRank = 1)
// SELECT f.PostId, f.Title, f.CreationDate, f.Score, f.ViewCount, f.CommentCount, f.TotalVotes, f.TotalUpVotes, f.TotalDownVotes, f.RecentHistoryId, f.MostRecentHistoryDate,
//        CASE WHEN f.TotalVotes = 0 THEN 'No votes' ELSE CASE WHEN f.TotalUpVotes > f.TotalDownVotes THEN 'Net positive' WHEN f.TotalUpVotes < f.TotalDownVotes THEN 'Net negative'
//        ELSE 'Neutral votes' END END AS VoteStatus
// FROM FinalPostAggregation f WHERE f.CommentCount > 0 AND (f.Score * 1.0 / NULLIF(f.ViewCount, 0)) > 0.1 ORDER BY f.ViewCount DESC LIMIT 100;
//
// RankedPosts has a row per post x comment x vote, and CommentCount counts those rows. HistoryRank ties go to the larger history id.
fn q20864(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(ts(2024, 9, 1, 0, 0, 0)));
    type J = ((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>);
    let j: MatSet<J> = recent().select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt())).collect();
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let iv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = top_per(drain(db.post_history.with(post_history_type_id.is_in([10, 11, 12, 13])).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let phr = rel(ph.into_iter().map(|(h, p)| (p, h)).collect::<Vec<_>>());
    let last: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&phr).map(|(p, _)| p).inv().select(&phr).collect();
    let ratio = Ident::<Post>::new().with(score.and(view_count).filt(|(s, w): (i64, i64)| w != 0 && s as f64 * 1.0 / w as f64 > 0.1));
    let pk = || Same::<J>::new().map(|((p, _), _): J| p);
    let v = drain((&j).select(pk().select(ratio).and(pk().select((&cc).filt(|n| n > 0))).and(pk().select((&iv).opt())).and(pk().select((&last).map(|(_, h)| h).opt()))));
    let v = top_n(v, |&(_, (((p, _), _), _))| (Reverse(view_count.get(p)), p), 100);
    rows(v.into_iter().map(|(_, (((p, c), i), h))| {
        let i = i.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(i[0]), V::I(i[1]), V::I(i[2])]);
        f.extend(match h {
            Some(h) => [V::I(post_history_type_id.get(h).unwrap()), V::T(hd.get(h).unwrap())],
            None => [V::I(-1), V::Null],
        });
        f.push(V::S(if i[0] == 0 { "No votes" } else if i[1] > i[2] { "Net positive" } else if i[1] < i[2] { "Net negative" } else { "Neutral votes" }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, AVG(P.Score) AS AvgScore, SUM(P.ViewCount) AS TotalViews,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END) AS TotalAnswersForQuestions,
//        SUM(CASE WHEN P.PostTypeId = 1 AND P.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedQuestions FROM Posts P GROUP BY P.OwnerUserId),
// UserActivity AS (SELECT U.Id AS UserId, U.Reputation, COALESCE(UBC.TotalBadges, 0) AS TotalBadges, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.AvgScore, 0) AS AvgScore,
//        COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(PS.TotalAnswersForQuestions, 0) AS TotalAnswersForQuestions, COALESCE(PS.ClosedQuestions, 0) AS ClosedQuestions
//     FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId),
// UserEngagement AS (SELECT UA.UserId, UA.Reputation, UA.TotalBadges, UA.TotalPosts, UA.AvgScore, UA.TotalViews, UA.TotalAnswersForQuestions, UA.ClosedQuestions,
//        RANK() OVER (ORDER BY UA.Reputation DESC, UA.TotalPosts DESC) AS ReputationRank FROM UserActivity UA)
// SELECT U.UserId, U.Reputation, U.TotalBadges, U.TotalPosts, U.AvgScore, U.TotalViews, U.TotalAnswersForQuestions, U.ClosedQuestions,
//        CASE WHEN U.Reputation < 1000 THEN 'Newcomer' WHEN U.Reputation BETWEEN 1000 AND 5000 THEN 'Contributor' WHEN U.Reputation > 5000 THEN 'Expert' ELSE 'Undefined' END AS UserTier,
//        CASE WHEN U.ClosedQuestions > 0 THEN 'Has Closed Questions' ELSE 'No Closed Questions' END AS ClosureStatus
// FROM UserEngagement U WHERE U.TotalPosts > 10 ORDER BY U.ReputationRank;
fn q22958(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, view_count, answer_count, closed_date, .. } = &db.post;
    let ubc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt()).and(answer_count.opt()).and(closed_date.opt())).fold([0i64; 5], |a, ((((t, s), w), n), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + if t == 1 { n.unwrap_or(0) } else { 0 }, a[4] + (t == 1 && c.is_some()) as i64]
    });
    let v = drain((&ubc).and((&ps).filt(|a| a[0] > 10)));
    rows(v.into_iter().map(|(u, (b, a))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(b), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        f.push(V::S(if r < 1000 { "Newcomer" } else if r <= 5000 { "Contributor" } else { "Expert" }));
        f.push(V::S(if a[4] > 0 { "Has Closed Questions" } else { "No Closed Questions" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        COALESCE(NULLIF(p.Body, ''), 'No content available') AS SafeBody FROM Posts p WHERE p.CreationDate > CURRENT_DATE - INTERVAL '2 years'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName,
//        CASE WHEN u.Reputation > 1000 THEN 'High Rep' WHEN u.Reputation BETWEEN 501 AND 1000 THEN 'Medium Rep' ELSE 'Low Rep' END AS ReputationCategory,
//        u.Location, COALESCE(u.AboutMe, 'No description') AS AboutUser FROM Users u WHERE u.Reputation >= 0),
// PostVoteCounts AS (SELECT v.PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpvoteCount, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownvoteCount
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// CombinedStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ur.UserId, ur.DisplayName, ur.Reputation, ur.ReputationCategory, ur.Location, ur.AboutUser,
//        COALESCE(pvc.UpvoteCount, 0) AS UpvoteCount, COALESCE(pvc.DownvoteCount, 0) AS DownvoteCount, rp.SafeBody,
//        CASE WHEN rp.Score = 0 THEN 'No Score' WHEN rp.Score > 0 THEN 'Positive' ELSE 'Negative' END AS ScoreCategory,
//        CASE WHEN rp.ViewCount > 1000 THEN 'High Traffic' ELSE 'Regular Traffic' END AS TrafficCategory
//     FROM RankedPosts rp LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostVoteCounts pvc ON rp.PostId = pvc.PostId)
// SELECT cs.PostId, cs.Title, cs.CreationDate, cs.Score, cs.ViewCount, cs.DisplayName, cs.Reputation, cs.ReputationCategory, cs.Location, cs.AboutUser, cs.UpvoteCount, cs.DownvoteCount,
//        cs.SafeBody, cs.ScoreCategory, cs.TrafficCategory
// FROM CombinedStats cs WHERE cs.ReputationCategory = 'High Rep' OR (cs.ReputationCategory = 'Medium Rep' AND cs.Score > 0) ORDER BY cs.CreationDate DESC, cs.UpvoteCount DESC
// LIMIT 20 OFFSET 10;
fn q24181(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, body, .. } = &db.post;
    let rep = &db.user.reputation;
    let ok = db.post.with(creation_date.gt(add_years(current_date(), -2))).with(owner_user.select(rep.ge(0)).and(score).filt(|(r, s): (i64, i64)| r > 1000 || (r >= 501 && s > 0)));
    let pvc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let v = drain(ok.select(owner_user.and((&pvc).opt())));
    let v = top_n(v, |&(p, (_, c))| (Reverse(creation_date.get(p).unwrap()), Reverse(c.map_or(0, |c| c[0])), p), 30);
    rows(v.into_iter().skip(10).map(|(p, (u, c))| {
        let r = rep.get(u).unwrap();
        let s = score.get(p).unwrap();
        let c = c.unwrap_or([0; 2]);
        let b = body.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::S(if r > 1000 { "High Rep" } else { "Medium Rep" }), ostr(db.user.location.get(u)), V::S(db.user.about_me.get(u).unwrap_or("No description"))]);
        f.extend([V::I(c[0]), V::I(c[1]), V::S(if b.is_empty() { "No content available" } else { b })]);
        f.push(V::S(if s == 0 { "No Score" } else if s > 0 { "Positive" } else { "Negative" }));
        f.push(V::S(if view_count.get(p).map_or(false, |w| w > 1000) { "High Traffic" } else { "Regular Traffic" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= '2022-01-01' AND p.CreationDate < '2024-10-01 12:34:56'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.CreationDate <= '2024-10-01 12:34:56' AND u.Reputation > 0 GROUP BY u.Id, u.DisplayName),
// PostAnalytics AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, ua.DisplayName, ua.UpVotes, ua.DownVotes, ua.BadgeCount,
//        CASE WHEN rp.Score IS NULL THEN 'No Score' WHEN rp.Score > 100 THEN 'Highly Rated' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Moderately Rated' ELSE 'Low Rating' END AS ScoreCategory
//     FROM RankedPosts rp INNER JOIN Users u ON rp.PostId = u.Id LEFT JOIN UserActivity ua ON u.Id = ua.UserId WHERE rp.rn <= 10 AND (rp.ViewCount > 100 OR ua.BadgeCount > 0)),
// PostHistoryAnalytics AS (SELECT p.Title, p.Id AS PostId, ph.PostHistoryTypeId, ph.CreationDate, COUNT(ph.Id) AS ChangeCount FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE ph.PostHistoryTypeId IN (4, 5, 10) AND ph.CreationDate > '2023-10-01 12:34:56' GROUP BY p.Title, p.Id, ph.PostHistoryTypeId, ph.CreationDate)
// SELECT pa.Title, pa.ViewCount, pa.Score, pa.DisplayName, pa.UpVotes, pa.DownVotes, pa.ScoreCategory, COALESCE(ph.ChangeCount, 0) AS RecentChanges
// FROM PostAnalytics pa LEFT JOIN PostHistoryAnalytics ph ON pa.PostId = ph.PostId WHERE (pa.UpVotes > pa.DownVotes OR pa.ScoreCategory = 'Highly Rated')
// ORDER BY pa.Score DESC, pa.ViewCount DESC;
//
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. rn ties go to the smaller post id.
fn q24136(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(ts(2022, 1, 1, 0, 0, 0)).and(creation_date.lt(ts(2024, 10, 1, 12, 34, 56)))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_raw: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let User { creation_date: ucd, reputation, .. } = &db.user;
    let ua = db
        .user
        .with(ucd.le(ts(2024, 10, 1, 12, 34, 56)))
        .with(reputation.gt(0))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.with(post_history_type_id.is_in([4, 5, 10]).and(hd.gt(ts(2023, 10, 1, 12, 34, 56)))).group_by(post.and(post_history_type_id).and(hd)).select(post).fold(0i64, |n, _| n + 1);
    let pr = rel(drain(&pha));
    let by_post: HashIdx<Id<Post>, (((Id<Post>, i64), i64), i64)> = (&pr).map(|(((p, _), _), _)| p).inv().select(&pr).collect();
    type A = (Id<Post>, ((i64, Option<i64>), (Id<User>, Option<[i64; 3]>)));
    let pa = drain((&tp).select(Ident::<Post>::new().and(score.and(view_count.opt()).and(origid.select(&by_raw).select(Ident::<User>::new().and((&ua).opt()))))));
    let pa = rel(drain(rel(pa).filt(|(_, x): (Id<Post>, A)| {
        let (_, ((s, w), (_, a))) = x;
        (w.map_or(false, |w| w > 100) || a.map_or(false, |a| a[2] > 0)) && (a.map_or(false, |a| a[0] > a[1]) || s > 100)
    })).into_iter().map(|x| (x.1).1).collect::<Vec<A>>());
    let v = drain((&pa).select(Same::<A>::new().and(Same::<A>::new().map(|(p, _): A| p).select((&by_post).map(|(_, n)| n).opt()))));
    rows(v.into_iter().map(|(_, ((p, ((s, w), (u, a))), n))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([oint(w), V::I(s)]);
        f.extend(match a {
            Some(a) => [user_col(db, u, "name"), V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if s > 100 { "Highly Rated" } else if s >= 50 { "Moderately Rated" } else { "Low Rating" }));
        f.push(V::I(n.unwrap_or(0)));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId AND c.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '90 days' GROUP BY p.Id, p.Title, p.PostTypeId, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName HAVING SUM(p.Score) > 100 OR COUNT(b.Id) >= 5),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.UserId, ph.CreationDate, RANK() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS ChangeRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12) AND ph.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostWithLinks AS (SELECT p.Id AS PostId, pl.RelatedPostId, lt.Name AS LinkTypeName FROM Posts p JOIN PostLinks pl ON p.Id = pl.PostId
//     LEFT JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id WHERE lt.Name IS NOT NULL)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, tu.UserId, tu.DisplayName, tu.TotalScore, tu.BadgeCount, pld.PostHistoryTypeId,
//        pld.ChangeRank, pwl.RelatedPostId, pwl.LinkTypeName, CASE WHEN pld.PostHistoryTypeId IS NULL THEN 'No history available' ELSE 'History exists' END AS HistoryStatus
// FROM RecentPosts rp JOIN TopUsers tu ON rp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = tu.UserId)
// LEFT JOIN PostHistoryDetails pld ON rp.PostId = pld.PostId AND pld.ChangeRank = 1 LEFT JOIN PostWithLinks pwl ON rp.PostId = pwl.PostId
// WHERE (rp.CommentCount > 5 OR rp.Score > 10) AND (rp.ViewCount IS NOT NULL OR rp.ViewCount > 100) ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// `rp.PostId IN (posts of tu)` is the post's owner being tu. RANK() = 1 keeps every history row at the latest date.
fn q22487(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let cnew = Ident::<Comment>::new().with((&db.comment.creation_date).gt(add_days(t0, -30)));
    let rp = db.post.with(creation_date.gt(add_days(t0, -90))).group_by(Ident::<Post>::new()).select(comments_of(db).select(cnew).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt().and(badges_of(db).opt())).fold([0i64; 3], |a, (s, b)| {
        [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0), a[2] + b.is_some() as i64]
    });
    let tu = (&tu).filt(|a| (a[0] > 0 && a[1] > 100) || a[2] >= 5);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = || db.post_history.with(post_history_type_id.is_in([10, 11, 12]).and(hd.gt(add_years(t0, -1))));
    let md = ph().group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = ph().select(post.and(hd)).inv().collect();
    let v = drain(
        (&rp)
            .and(score.and(view_count))
            .filt(|(c, (s, _)): (i64, (i64, i64))| c > 5 || s > 10)
            .and(owner_user.select(Ident::<User>::new().and(tu)))
            .and(Ident::<Post>::new().and(&md).select(&at).opt())
            .and(links_of(db).opt()),
    );
    rows(v.into_iter().map(|(p, ((((c, _), (u, a)), h), l))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(V::I(c));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), nullable(a[1], a[0]), V::I(a[2])]);
        f.extend(match h {
            Some(h) => [V::I(post_history_type_id.get(h).unwrap()), V::I(1)],
            None => [V::Null, V::Null],
        });
        f.extend(match l {
            Some(l) => [V::I(db.post_link.related_post_id.get(l).unwrap()), V::S(db.link_type.name.get(db.post_link.link_type.get(l).unwrap()).unwrap())],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if h.is_none() { "No history available" } else { "History exists" }));
        row(f)
    }))
}

// WITH RECURSIVE UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, b.Name AS BadgeName, COUNT(b.Id) AS BadgeCount FROM Users u JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName, b.Name),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, COUNT(c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.OwnerUserId),
// RecentActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS RecentPosts, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.LastAccessDate >= cast('2024-10-01' as date) - INTERVAL '60 days'
//     GROUP BY u.Id, u.DisplayName),
// FinalReport AS (SELECT ua.UserId, ua.DisplayName, SUM(ua.RecentPosts) AS TotalRecentPosts, SUM(ua.GoldBadges) AS TotalGoldBadges, SUM(ua.SilverBadges) AS TotalSilverBadges,
//        SUM(ua.BronzeBadges) AS TotalBronzeBadges, SUM(ps.TotalComments) AS TotalCommentsOnRecentPosts, SUM(ps.TotalUpvotes) AS TotalUpvotesOnRecentPosts,
//        SUM(ps.TotalDownvotes) AS TotalDownvotesOnRecentPosts FROM RecentActivity ua LEFT JOIN PostStatistics ps ON ua.UserId = ps.OwnerUserId GROUP BY ua.UserId, ua.DisplayName)
// SELECT fr.UserId, fr.DisplayName, fr.TotalRecentPosts, fr.TotalGoldBadges, fr.TotalSilverBadges, fr.TotalBronzeBadges, fr.TotalCommentsOnRecentPosts,
//        fr.TotalUpvotesOnRecentPosts, fr.TotalDownvotesOnRecentPosts
// FROM FinalReport fr LEFT JOIN UserBadges ub ON fr.UserId = ub.UserId WHERE fr.TotalRecentPosts > 0 ORDER BY fr.TotalUpvotesOnRecentPosts DESC, fr.TotalRecentPosts DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. FinalReport sums each RecentActivity row once per PostStatistics row it joins.
fn q30969(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let ra_users = || db.user.with((&db.user.last_access_date).ge(ts(2024, 8, 2, 0, 0, 0)));
    let ra = ra_users().group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 3], |a, (_, c)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let rpn = ra_users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(|xs| distinct_some(xs.iter().copied()));
    let ps = db.post.with(creation_date.ge(ts(2024, 9, 1, 0, 0, 0))).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let fr = ra_users().group_by(Ident::<User>::new()).select((&ra).and(&rpn).and(posts_of(db).select(&ps).opt())).fold([0i64; 8], |a, ((b, n), p)| {
        let q = p.unwrap_or([0; 3]);
        [a[0] + n, a[1] + b[0], a[2] + b[1], a[3] + b[2], a[4] + p.is_some() as i64, a[5] + q[0], a[6] + q[1], a[7] + q[2]]
    });
    let ub = db.badge.group_by((&db.badge.user).and(&db.badge.name)).select(&db.badge.user).fold(0i64, |n, _| n + 1);
    let ubr = rel(drain(&ub));
    let ub_of: HashIdx<Id<User>, ((Id<User>, Str), i64)> = (&ubr).map(|((u, _), _)| u).inv().select(&ubr).collect();
    let v = drain((&fr).filt(|a| a[0] > 0).and((&ub_of).opt()));
    rows(v.into_iter().map(|(u, (a, _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a[..4].iter().map(|&x| V::I(x)));
        f.extend(if a[4] == 0 { [V::Null, V::Null, V::Null] } else { [V::I(a[5]), V::I(a[6]), V::I(a[7])] });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, MAX(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadge,
//        MAX(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadge, MAX(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadge
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON b.UserId = p.OwnerUserId
//     WHERE p.ViewCount IS NOT NULL AND p.CreationDate < TIMESTAMP '2024-10-01 12:34:56' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// HighScorePosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.GoldBadge, rp.SilverBadge, rp.BronzeBadge,
//        CASE WHEN rp.PostRank = 1 AND rp.Score >= 100 THEN 'Top Performing' ELSE 'Regular' END AS PerformanceCategory FROM RankedPosts rp WHERE rp.Score IS NOT NULL AND rp.Score > 0),
// PostHistoryInfo AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate, SUM(CASE WHEN ph.PostHistoryTypeId IN (6, 4) THEN 1 ELSE 0 END) AS TitleEdits,
//        SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseVotes FROM PostHistory ph GROUP BY ph.PostId),
// FinalPostData AS (SELECT h.PostId, h.Title, h.Score, h.ViewCount, h.CommentCount, h.PerformanceCategory, p.EditCount, p.LastEditDate, p.TitleEdits, p.CloseVotes,
//        CASE WHEN p.CloseVotes > 3 THEN 'Highly Closed' ELSE 'Active' END AS ClosureStatus
//     FROM HighScorePosts h LEFT JOIN PostHistoryInfo p ON h.PostId = p.PostId WHERE h.GoldBadge = 1 OR h.SilverBadge = 1 OR h.BronzeBadge = 1)
// SELECT f.*, COALESCE(NULLIF(h.Title, ''), 'Untitled') AS SafeTitle,
//        CASE WHEN f.ClosureStatus = 'Highly Closed' THEN 'This post has been frequently closed; consider revising.' ELSE 'This post is currently active and well-received.' END AS StatusMessage
// FROM FinalPostData f LEFT JOIN Posts h ON f.PostId = h.Id ORDER BY f.Score DESC, f.ViewCount DESC LIMIT 100;
//
// The sort and the badge test read only base columns (a Gold/Silver/Bronze flag is 1 exactly when the owner has a badge of that class), so the hundred posts are
// picked first and the comment x badge product is driven for those alone. PostRank ties on Score go to the smaller post id.
fn q24854(db: &'static So) -> String {
    let Post { view_count, creation_date, score, owner_user, title, .. } = &db.post;
    let base = || db.post.with(view_count).with(creation_date.lt(ts(2024, 10, 1, 12, 34, 56)));
    let rk = top_per(drain(base().select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(rk.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cls = |c: i64| Ident::<Post>::new().with(owner_user.select(badges_of(db)).select((&db.badge.class).eq(c)));
    let hs = base().with(score.gt(0)).with(cls(1).or(cls(2)).or(cls(3)));
    let v = top_n(drain(hs.select(view_count)), |&(p, w)| (Reverse(score.get(p).unwrap()), Reverse(w), p), 100);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(owner_user.select(badges_of(db)).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phi = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold([0, i64::MIN, 0, 0], |a, (t, d)| [a[0] + 1, a[1].max(d), a[2] + matches!(t, 4 | 6) as i64, a[3] + (t == 10) as i64]);
    let v = drain((&cc).and((&phi).opt()).and(Ident::<Post>::new().with(&first).opt()));
    rows(v.into_iter().map(|(p, ((c, h), fst))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::S(if fst.is_some() && s >= 100 { "Top Performing" } else { "Regular" })]);
        let closed = h.map_or(false, |h| h[3] > 3);
        f.extend(match h {
            Some(h) => [V::I(h[0]), V::T(h[1]), V::I(h[2]), V::I(h[3])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::S(if closed { "Highly Closed" } else { "Active" }));
        f.push(V::S(title.get(p).filter(|t| !t.is_empty()).unwrap_or("Untitled")));
        f.push(V::S(if closed { "This post has been frequently closed; consider revising." } else { "This post is currently active and well-received." }));
        row(f)
    }))
}

// WITH RecursiveUserActivity AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.Score) AS TotalScore,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY SUM(p.Score) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalScore, QuestionCount, AnswerCount FROM RecursiveUserActivity WHERE UserRank <= 10),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COALESCE(ph.UserDisplayName, 'No Edits') AS LastEditor,
//        MAX(ph.CreationDate) AS LastEditDate, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (4, 5)
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ph.UserDisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, LastEditor, LastEditDate FROM PostDetails WHERE PostRank <= 10),
// Awards AS (SELECT b.UserId, b.Name AS BadgeName, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId, b.Name),
// UserAwards AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(a.BadgeName, 'No Badge') AS BadgeName, COALESCE(a.BadgeCount, 0) AS BadgeCount FROM Users u
//     LEFT JOIN Awards a ON u.Id = a.UserId)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.PostCount, tu.TotalScore, tu.QuestionCount, tu.AnswerCount, tp.Title AS TopPostTitle, tp.CreationDate AS TopPostCreationDate,
//        tp.Score AS TopPostScore, tp.ViewCount AS TopPostViewCount, tp.CommentCount AS TopPostCommentCount, tp.LastEditor AS TopPostLastEditor, tp.LastEditDate AS TopPostLastEditDate,
//        ua.BadgeName, ua.BadgeCount
// FROM TopUsers tu JOIN TopPosts tp ON tp.CommentCount = ALL (SELECT CommentCount FROM TopPosts WHERE UserId = tu.UserId)
// LEFT JOIN UserAwards ua ON tu.UserId = ua.UserId ORDER BY tu.TotalScore DESC;
//
// TopPosts has no UserId, so DuckDB binds `UserId = tu.UserId` to tu itself and the subquery is every TopPosts row: a TopPosts row joins every user when
// its CommentCount equals all ten. UserRank partitions by the user, so it is always 1. PostRank ranks (post, editor) groups; only the ten newest posts can
// supply the ten newest groups, so the groups are built for those, and ties go to post id, then editor name.
fn q32097(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let r10 = top_n(drain(db.post.select(creation_date)), |&(p, d)| (Reverse(d), p), 10);
    let r10: MatSet<Id<Post>> = rel(r10.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post_history_type_id, user_display_name, creation_date: hd, .. } = &db.post_history;
    let e45 = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5])));
    type J = ((Id<Post>, Option<Id<Comment>>), Option<Id<PostHistory>>);
    let j: MatSet<J> = (&r10).select(Ident::<Post>::new().and(comments_of(db).opt()).and(e45.opt())).collect();
    let editor = Same::<J>::new().map(|(_, h): J| h).flat_map(|h: Option<Id<PostHistory>>| h).select(user_display_name.opt()).opt().map(|x: Option<Option<Str>>| x.flatten());
    let pd = (&j)
        .group_by(Same::<J>::new().map(|((p, _), _): J| p).and(editor))
        .select(Same::<J>::new().map(|((_, c), _): J| c).and(Same::<J>::new().map(|(_, h): J| h).flat_map(|h: Option<Id<PostHistory>>| h).select(hd).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, d.map_or(m, |d| m.max(d))));
    let tp = rel(top_n(drain(&pd), |&((p, e), _)| (Reverse(creation_date.get(p).unwrap()), p, e), 10));
    let (mn, mx) = (&tp).map(|(_, (n, _))| n).fold_flat((i64::MAX, i64::MIN), |(a, b), n| (a.min(n), b.max(n)));
    let tpa = rel(drain((&tp).filt(move |(_, (n, _))| n == mn && n == mx)).into_iter().map(|x| x.1).collect::<Vec<_>>());
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64],
        None => a,
    });
    let aw = db.badge.group_by((&db.badge.user).and(&db.badge.name)).select(&db.badge.user).fold(0i64, |n, _| n + 1);
    let awr = rel(drain(&aw));
    let aw_of: HashIdx<Id<User>, ((Id<User>, Str), i64)> = (&awr).map(|((u, _), _)| u).inv().select(&awr).collect();
    let mut v = Vec::new();
    (&tu).cross(&tpa).drive(|(u, _), (a, ((p, e), (n, d)))| v.push((u, a, p, e, n, d)));
    let v = rel(v);
    type X = (Id<User>, [i64; 4], Id<Post>, Option<Str>, i64, i64);
    let v = drain((&v).select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.0).select((&aw_of).opt()))));
    rows(v.into_iter().map(|(_, ((u, a, p, e, n, d), w))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend([V::I(n), V::S(e.unwrap_or("No Edits")), tmax(d)]);
        f.extend(match w {
            Some(((_, b), k)) => [V::S(b), V::I(k)],
            None => [V::S("No Badge"), V::I(0)],
        });
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCounts AS (SELECT UserId, COUNT(*) AS TotalBadges, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges GROUP BY UserId),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(BC.TotalBadges, 0) AS TotalBadges, BC.GoldBadges, BC.SilverBadges, BC.BronzeBadges,
//        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U LEFT JOIN UserBadgeCounts BC ON U.Id = BC.UserId WHERE U.Reputation > 0),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, U.Id AS OwnerUserId, U.DisplayName AS OwnerDisplayName,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount,
//        COALESCE((SELECT SUM(V.BountyAmount) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 8), 0) AS TotalBounty
//     FROM Posts P INNER JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AnswerDetails AS (SELECT A.Id AS AnswerId, A.ParentId AS QuestionId, A.Score AS AnswerScore, A.CreationDate AS AnswerCreationDate, D.OwnerUserId, D.OwnerDisplayName,
//        D.CommentCount, D.TotalBounty, ROW_NUMBER() OVER (PARTITION BY A.ParentId ORDER BY A.Score DESC) AS AnswerRank
//     FROM Posts A JOIN PostDetails D ON A.ParentId = D.PostId WHERE A.PostTypeId = 2)
// SELECT TU.DisplayName AS TopUser, TU.TotalBadges, TU.GoldBadges, TU.SilverBadges, TU.BronzeBadges, PD.Title AS QuestionTitle, PD.ViewCount, PD.Score AS QuestionScore,
//        COALESCE(AD.AnswerId, -1) AS AcceptedAnswerId, AD.AnswerScore AS MostSupportedAnswerScore, AD.AnswerCreationDate AS MostSupportedAnswerDate, PD.CommentCount,
//        PD.TotalBounty AS QuestionTotalBounty,
//        CASE WHEN PD.ViewCount > 1000 THEN 'High Lead' WHEN PD.ViewCount BETWEEN 500 AND 1000 THEN 'Moderate Lead' ELSE 'Low Lead' END AS EngagementLevel
// FROM TopUsers TU JOIN PostDetails PD ON TU.UserId = PD.OwnerUserId LEFT JOIN AnswerDetails AD ON PD.PostId = AD.QuestionId AND AD.AnswerRank = 1
// WHERE TU.Rank <= 10 ORDER BY TU.Reputation DESC, PD.ViewCount DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. AnswerRank ties on Score go to the smaller answer id.
fn q34109(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let tu = top_n(drain(db.user.with(rep.gt(0)).select(rep)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let Post { creation_date, post_type_id, score, parent, view_count, .. } = &db.post;
    let recent = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pdp: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(&db.post.owner_user).collect();
    let ans = drain(db.post.with(post_type_id.eq(2)).select(parent.select(&pdp)));
    let a1 = top_per(ans, |&(_, q)| q, |&(a, _)| (Reverse(score.get(a).unwrap()), a), 1, false);
    let ar = rel(a1.into_iter().map(|(a, q)| (q, a)).collect::<Vec<_>>());
    let ad: HashIdx<Id<Post>, (Id<Post>, Id<Post>)> = (&ar).map(|(q, _)| q).inv().select(&ar).collect();
    let cc = (&pdp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let b8 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select(&db.vote.bounty_amount);
    let tb = (&pdp).group_by(Ident::<Post>::new()).select(b8.opt()).fold(0i64, |n, b| n + b.unwrap_or(0));
    let v = drain((&tu).select(Ident::<User>::new().and((&bc).opt()).and(posts_of(db).select(recent).select(Ident::<Post>::new().and(&cc).and(&tb).and((&ad).map(|(_, a)| a).opt())))));
    rows(v.into_iter().map(|(_, ((u, b), (((p, c), t), a)))| {
        let w = view_count.get(p);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(match b {
            Some(b) => [V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])],
            None => [V::I(0), V::Null, V::Null, V::Null],
        });
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend(match a {
            Some(a) => [V::I(db.post.origid.get(a).unwrap()), V::I(score.get(a).unwrap()), V::T(creation_date.get(a).unwrap())],
            None => [V::I(-1), V::Null, V::Null],
        });
        f.extend([V::I(c), V::I(t), V::S(match w { Some(w) if w > 1000 => "High Lead", Some(w) if w >= 500 => "Moderate Lead", _ => "Low Lead" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPosts, COALESCE(SUM(v.BountyAmount) FILTER (WHERE v.VoteTypeId = 9) OVER (PARTITION BY p.OwnerUserId), 0) AS TotalBounty
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1 AND p.ViewCount > 10),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// FinalPostMetrics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerUserId, rp.Score, rp.ViewCount, rp.TotalPosts, rp.TotalBounty, ub.BadgeCount, ub.GoldBadges,
//        ub.SilverBadges, ub.BronzeBadges, rc.CommentCount, rc.LastCommentDate
//     FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN RecentComments rc ON rp.PostId = rc.PostId WHERE rp.Rank <= 3)
// SELECT f.PostId, f.Title, f.CreationDate, f.OwnerUserId, f.Score, f.ViewCount, f.TotalPosts, f.TotalBounty, COALESCE(f.BadgeCount, 0) AS BadgeCount,
//        COALESCE(f.GoldBadges, 0) AS GoldBadges, COALESCE(f.SilverBadges, 0) AS SilverBadges, COALESCE(f.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(f.CommentCount, 0) AS CommentCount, f.LastCommentDate,
//        CASE WHEN f.Score > 100 THEN 'High Score' WHEN f.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory,
//        CASE WHEN f.TotalBounty = 0 THEN 'No bounties offered' WHEN f.TotalBounty < 100 THEN 'Moderate bounties offered' ELSE 'High bounties offered' END AS BountyCategory
// FROM FinalPostMetrics f ORDER BY f.Score DESC, f.ViewCount DESC LIMIT 100;
//
// RankedPosts has a row per question x vote, so Rank, TotalPosts and TotalBounty are over those rows; Rank ties go to post id, then vote id.
fn q22382(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, owner_user, score, .. } = &db.post;
    let base = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).with(view_count.gt(10));
    type J = (Id<Post>, Option<Id<Vote>>);
    let j: MatSet<J> = base.select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    let own = || Same::<J>::new().map(|(p, _): J| p).select(owner_user.opt());
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let b9 = Same::<J>::new().map(|(_, v): J| v).flat_map(|v: Option<Id<Vote>>| v).with(vote_type_id.eq(9)).select(bounty_amount).opt();
    let tot = (&j).group_by(own()).select(b9).fold([0i64; 2], |a, b| [a[0] + 1, a[1] + b.unwrap_or(0)]);
    let top = top_per(drain((&j).select(own())), |&(_, u)| u, |&((p, v), _)| (Reverse(score.get(p).unwrap()), p, v), 3, false);
    type T = (J, Option<Id<User>>);
    let fr = rel(top);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Comment { post, creation_date: cd, .. } = &db.comment;
    let rc = db.comment.group_by(post).select(cd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&fr).select(
        Same::<T>::new()
            .and(Same::<T>::new().map(|(_, u): T| u).select(&tot))
            .and(Same::<T>::new().map(|(_, u): T| u).flat_map(|u: Option<Id<User>>| u).select(&ub).opt())
            .and(Same::<T>::new().map(|((p, _), _): T| p).select((&rc).opt())),
    ));
    let v = top_n(v, |&(i, (((((p, _), _), _), _), _))| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p, i), 100);
    rows(v.into_iter().map(|(_, (((((p, _), _), t), b), c))| {
        let s = score.get(p).unwrap();
        let b = b.unwrap_or([0; 4]);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner_id", "score", "views"]);
        f.extend([V::I(t[0]), V::I(t[1])]);
        f.extend(b.map(V::I));
        f.extend(match c {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        f.push(V::S(if s > 100 { "High Score" } else if s >= 50 { "Medium Score" } else { "Low Score" }));
        f.push(V::S(if t[1] == 0 { "No bounties offered" } else if t[1] < 100 { "Moderate bounties offered" } else { "High bounties offered" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// RecentPostCounts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT UR.DisplayName, UR.Reputation, COALESCE(RPC.PostCount, 0) AS RecentPostCount FROM UserReputation UR LEFT JOIN RecentPostCounts RPC ON UR.UserId = RPC.OwnerUserId
//     WHERE UR.Reputation > 1000),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.Score, (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2) AS UpVoteCount, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3) AS DownVoteCount
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostSummary AS (SELECT PD.Title, PD.Score, PD.CommentCount, PD.UpVoteCount, PD.DownVoteCount, COALESCE(PH.UserDisplayName, 'Unknown User') AS LastEditBy, PD.PostId,
//        ROW_NUMBER() OVER (ORDER BY PD.Score DESC) AS PopularityRank
//     FROM PostDetails PD LEFT JOIN PostHistory PH ON PD.PostId = PH.PostId AND PH.CreationDate = (SELECT MAX(PH2.CreationDate) FROM PostHistory PH2 WHERE PH2.PostId = PD.PostId)
//     WHERE PD.Score > 0),
// UserPostRankings AS (SELECT TU.DisplayName, TU.Reputation, PS.Title, PS.Score, PS.CommentCount, PS.UpVoteCount, PS.DownVoteCount, PS.PopularityRank,
//        CASE WHEN PS.PopularityRank <= 10 THEN 'Top 10' WHEN PS.PopularityRank BETWEEN 11 and 50 THEN 'Top 11-50' ELSE 'Beyond Top 50' END AS PopularityCategory
//     FROM TopUsers TU INNER JOIN PostSummary PS ON TU.DisplayName = PS.LastEditBy)
// SELECT UPR.DisplayName, UPR.Reputation, UPR.Title, UPR.Score, UPR.CommentCount, UPR.UpVoteCount, UPR.DownVoteCount, UPR.PopularityRank, UPR.PopularityCategory,
//        CASE WHEN UPR.Reputation > 5000 THEN 'Platinum Contributor' WHEN UPR.Reputation BETWEEN 3000 AND 4999 THEN 'Gold Contributor' ELSE 'Regular Contributor' END AS ContributorBadge
// FROM UserPostRankings UPR ORDER BY UPR.PopularityRank FETCH FIRST 50 ROWS ONLY;
//
// PopularityRank ties on Score go to post id, then history id.
fn q20870(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let PostHistory { post, creation_date: hd, user_display_name, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    type P = (Id<Post>, Option<Id<PostHistory>>);
    let ps = drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(score.gt(0)).select(Ident::<Post>::new().and(Ident::<Post>::new().and(&md).select(&at).opt())));
    let ps = ranked(ps.into_iter().map(|x| x.1).collect::<Vec<P>>(), |&(p, h)| (Reverse(score.get(p).unwrap()), p, h), false);
    let psr = rel(ps);
    type R = (P, i64);
    let editor = Same::<R>::new().map(|((_, h), _): R| h).flat_map(|h: Option<Id<PostHistory>>| h).select(user_display_name.opt()).opt().map(|x: Option<Option<Str>>| x.flatten().unwrap_or("Unknown User"));
    let by_name: HashIdx<Str, R> = (&psr).select(editor).inv().select(&psr).collect();
    let rep = &db.user.reputation;
    let v = drain(db.user.with(rep.gt(1000)).select((&db.user.display_name).select(&by_name)));
    let v = top_n(v, |&(u, ((p, h), k))| (k, u, p, h), 50);
    let cv = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let vr = rel(v);
    type X = (Id<User>, R);
    let v = drain((&vr).select(Same::<X>::new().and(Same::<X>::new().map(|(_, ((p, _), _)): X| p).select((&cv).and(&vv)))));
    rows(v.into_iter().map(|(_, ((u, ((p, _), k)), (c, w)))| {
        let r = rep.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(c), V::I(w[0]), V::I(w[1]), V::I(k), V::S(if k <= 10 { "Top 10" } else if k <= 50 { "Top 11-50" } else { "Beyond Top 50" })]);
        f.push(V::S(if r > 5000 { "Platinum Contributor" } else if r >= 3000 { "Gold Contributor" } else { "Regular Contributor" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStats AS (SELECT P.Id AS PostId, P.Title, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN V.VoteTypeId = 10 THEN 1 ELSE 0 END) AS DeletionCount, COALESCE(P.ViewCount, 0) AS ViewCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.ViewCount),
// DetailedPostStats AS (SELECT PS.PostId, PS.Title, PS.CommentCount, PS.UpVotes, PS.DownVotes, PS.DeletionCount, PS.ViewCount,
//        CASE WHEN PS.UpVotes > PS.DownVotes THEN 'Positive' WHEN PS.UpVotes < PS.DownVotes THEN 'Negative' ELSE 'Neutral' END AS Sentiment FROM PostStats PS),
// UserBadgeStats AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId),
// FinalStats AS (SELECT U.UserId, U.Reputation, U.ReputationRank, DPS.Title, DPS.CommentCount, DPS.UpVotes, DPS.DownVotes, DPS.ViewCount, UBS.GoldBadges, UBS.SilverBadges,
//        UBS.BronzeBadges, DPS.Sentiment FROM UserReputation U LEFT JOIN DetailedPostStats DPS ON U.UserId = DPS.PostId LEFT JOIN UserBadgeStats UBS ON U.UserId = UBS.UserId)
// SELECT FS.UserId, FS.Reputation, FS.ReputationRank, COALESCE(FS.Title, 'No Posts') AS PostTitle, FS.CommentCount, FS.UpVotes, FS.DownVotes, FS.ViewCount,
//        COALESCE(FS.GoldBadges, 0) AS GoldBadges, COALESCE(FS.SilverBadges, 0) AS SilverBadges, COALESCE(FS.BronzeBadges, 0) AS BronzeBadges, FS.Sentiment,
//        CASE WHEN FS.ViewCount > 100 THEN 'Highly Viewed' WHEN FS.ViewCount BETWEEN 50 AND 100 THEN 'Moderately Viewed' ELSE 'Less Viewed' END AS ViewershipCategory
// FROM FinalStats FS WHERE FS.ReputationRank <= 100 OR FS.UpVotes > 0 ORDER BY FS.Reputation DESC, FS.CommentCount ASC NULLS LAST;
//
// `U.UserId = DPS.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q20752(db: &'static So) -> String {
    let Post { creation_date, view_count, origid, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ps = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let pidx: HashIdx<i64, Id<Post>> = recent().select(origid).inv().collect();
    let ubs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ur = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false));
    type U = ((Id<User>, i64), i64);
    let dps = (&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&ps));
    let v = drain((&ur).select(Same::<U>::new().and(Same::<U>::new().map(|((u, _), _): U| u).select(dps.opt())).and(Same::<U>::new().map(|((u, _), _): U| u).select((&ubs).opt()))));
    type X = (usize, ((U, Option<(Id<Post>, [i64; 3])>), Option<[i64; 3]>));
    let v = drain(rel(v).filt(|(_, ((((_, _), k), d), _)): X| k <= 100 || d.map_or(false, |(_, a)| a[1] > 0)));
    rows(v.into_iter().map(|(_, (_, ((((u, _), k), d), b)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(k));
        match d {
            Some((p, a)) => {
                let w = view_count.get(p).unwrap_or(0);
                f.push(V::S(db.post.title.get(p).unwrap_or("No Posts")));
                f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(w)]);
                let b = b.unwrap_or([0; 3]);
                f.extend(b.map(V::I));
                f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
                f.push(V::S(if w > 100 { "Highly Viewed" } else if w >= 50 { "Moderately Viewed" } else { "Less Viewed" }));
            }
            None => {
                f.extend([V::S("No Posts"), V::Null, V::Null, V::Null, V::Null]);
                let b = b.unwrap_or([0; 3]);
                f.extend(b.map(V::I));
                f.extend([V::Null, V::S("Less Viewed")]);
            }
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, pv.UpVotes, pv.DownVotes,
//        CASE WHEN pv.UpVotes + pv.DownVotes > 0 THEN (pv.UpVotes * 1.0 / (pv.UpVotes + pv.DownVotes) * 100) ELSE NULL END AS VotePercentage, rp.CreationDate, rp.RankByScore
//     FROM RankedPosts rp LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId),
// FilteredPosts AS (SELECT pd.PostId, pd.Title, pd.Score, pd.ViewCount, pd.CommentCount, pd.UpVotes, pd.DownVotes, pd.VotePercentage, pd.CreationDate, pd.RankByScore,
//        COALESCE(NULLIF(pd.VotePercentage, 0), 100) AS AdjustedVotePercentage FROM PostDetails pd WHERE pd.Score > 5 AND pd.RankByScore = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(b.Class) AS TotalBadges, RANK() OVER (ORDER BY SUM(b.Class) DESC) AS BadgeRank FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName HAVING SUM(b.Class) > 0)
// SELECT fp.PostId, fp.Title, fp.Score, fp.ViewCount, fp.CommentCount, fp.UpVotes, fp.DownVotes, fp.AdjustedVotePercentage, tu.UserId, tu.DisplayName, tu.TotalBadges,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = fp.PostId) AS ActualCommentCount,
//        (SELECT CASE WHEN EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = fp.PostId AND v.UserId = -1) THEN 1 ELSE 0 END) AS IsVotedByCommunityUser
// FROM FilteredPosts fp JOIN Users u ON fp.UpVotes > 20 AND u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = fp.PostId) JOIN TopUsers tu ON tu.UserId = u.Id
// ORDER BY fp.AdjustedVotePercentage DESC, fp.CreationDate DESC;
//
// RankByScore ties go to the smaller post id.
fn q23268(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let r1 = top_per(drain(recent().select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let r1: MatSet<Id<Post>> = rel(r1.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let fp = (&r1).with(score.gt(5));
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tu = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold(0i64, |n, c| n + c);
    let comm = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.user_id).eq(-1)));
    let v = drain(fp.select((&pv).filt(|a| a[0] > 20).and(owner_user.select(Ident::<User>::new().and((&tu).filt(|n| n > 0)))).and(&cc).and(Ident::<Post>::new().with(comm).opt())));
    rows(v.into_iter().map(|(p, (((a, (u, b)), c), k))| {
        let pct = if a[0] + a[1] > 0 { a[0] as f64 * 1.0 / (a[0] + a[1]) as f64 * 100.0 } else { 0.0 };
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::F(if pct == 0.0 { 100.0 } else { pct })]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(b), V::I(c), V::I(k.is_some() as i64)]);
        row(f)
    }))
}

// WITH RecursiveUserAccolades AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostsWithRank AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.Score, p.CreationDate, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.PostId) AS TotalPosts, COALESCE(SUM(p.Score), 0) AS TotalScore, AVG(p.Score) AS AvgScore
//     FROM Users u LEFT JOIN PostsWithRank p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsersWithPosts AS (SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalScore, ups.AvgScore, ra.BadgeCount, ra.GoldBadges, ra.SilverBadges, ra.BronzeBadges
//     FROM UserPostStats ups JOIN RecursiveUserAccolades ra ON ups.UserId = ra.UserId WHERE ups.TotalPosts > 0),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// FinalBenchmark AS (SELECT t.UserId, t.DisplayName, t.TotalPosts, t.TotalScore, t.AvgScore, t.BadgeCount, t.GoldBadges, t.SilverBadges, t.BronzeBadges, ua.CommentCount, ua.TotalBounty
//     FROM TopUsersWithPosts t JOIN UserActivity ua ON t.UserId = ua.UserId)
// SELECT fb.DisplayName, fb.TotalPosts, fb.TotalScore, fb.AvgScore, fb.BadgeCount, fb.GoldBadges, fb.SilverBadges, fb.BronzeBadges, fb.CommentCount, fb.TotalBounty,
//        CASE WHEN fb.AvgScore > 20 THEN 'High Performer' WHEN fb.AvgScore BETWEEN 10 AND 20 THEN 'Medium Performer' ELSE 'Low Performer' END AS PerformanceCategory
// FROM FinalBenchmark fb ORDER BY fb.TotalScore DESC LIMIT 10;
//
// The sort reads only TotalScore, so only users reaching the tenth-highest TotalScore can make the cut; the comments x votes product of UserActivity is driven
// for those alone.
fn q30663(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(score);
    let ups = db.user.group_by(Ident::<User>::new()).select(recent).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let tenth = top_n(drain(&ups), |&(u, a)| (Reverse(a[1]), u), 10).last().map_or(i64::MIN, |x| x.1[1]);
    let cand: MatSet<Id<User>> = db.user.with((&ups).filt(move |a| a[1] >= tenth)).collect();
    let ra = (&cand).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ua = (&cand).group_by(Ident::<User>::new()).select(comments_by(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 2], |a, (c, b)| {
        [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]
    });
    let v = top_n(drain((&ups).and(&ra).and(&ua)), |&(u, ((a, _), _))| (Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, ((a, b), c))| {
        let m = a[1] as f64 / a[0] as f64;
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), avg(a[1], a[0])];
        f.extend(b.map(V::I));
        f.extend([V::I(c[0]), V::I(c[1]), V::S(if m > 20.0 { "High Performer" } else if m >= 10.0 { "Medium Performer" } else { "Low Performer" })]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverCount, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostActivity AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        COUNT(p.Id) AS TotalPosts, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(CASE WHEN p.Score > 0 THEN p.Score END, 0)) AS PositiveScoreSum,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(p.Id) DESC) AS ActivityRank FROM Posts p GROUP BY p.OwnerUserId),
// EngagementSummary AS (SELECT u.Id AS UserId, ub.DisplayName, ub.BadgeCount, pa.QuestionCount, pa.AnswerCount, pa.TotalPosts, pa.TotalViews, pa.PositiveScoreSum,
//        CASE WHEN pa.QuestionCount > pa.AnswerCount THEN 'More Questions' WHEN pa.QuestionCount < pa.AnswerCount THEN 'More Answers' ELSE 'Equal Questions and Answers' END AS PostTypeDominance
//     FROM UserBadges ub LEFT JOIN PostActivity pa ON ub.UserId = pa.OwnerUserId INNER JOIN Users u ON ub.UserId = u.Id WHERE (ub.BadgeCount > 0 OR pa.TotalPosts > 0)
//     ORDER BY ub.BadgeCount DESC, pa.TotalPosts DESC)
// SELECT e.UserId, e.DisplayName, COALESCE(e.BadgeCount, 0) AS BadgeCount, COALESCE(e.QuestionCount, 0) AS QuestionCount, COALESCE(e.AnswerCount, 0) AS AnswerCount,
//        COALESCE(e.TotalPosts, 0) AS TotalPosts, COALESCE(e.TotalViews, 0) AS TotalViews, COALESCE(e.PositiveScoreSum, 0) AS PositiveScoreSum, e.PostTypeDominance,
//        CASE WHEN e.TotalViews IS NULL OR e.TotalPosts IS NULL THEN 'Unknown Engagement' ELSE CASE WHEN e.TotalViews > e.TotalPosts * 10 THEN 'High Engagement'
//        WHEN e.TotalViews < e.TotalPosts * 2 THEN 'Low Engagement' ELSE 'Medium Engagement' END END AS EngagementLevel
// FROM EngagementSummary e WHERE (e.QuestionCount + e.AnswerCount) > 0 AND e.TotalViews IS NOT NULL
// ORDER BY CASE WHEN e.PostTypeDominance = 'More Questions' THEN 1 WHEN e.PostTypeDominance = 'More Answers' THEN 2 ELSE 3 END, e.BadgeCount DESC, e.TotalViews DESC;
fn q22407(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, score, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pa = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 5], |a, ((t, w), s)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + 1, a[3] + w.unwrap_or(0), a[4] + s.max(0)]
    });
    let v = drain((&ub).and((&pa).filt(|a| a[0] + a[1] > 0)));
    rows(v.into_iter().map(|(u, (b, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(b));
        f.extend(a.map(V::I));
        f.push(V::S(if a[0] > a[1] { "More Questions" } else if a[0] < a[1] { "More Answers" } else { "Equal Questions and Answers" }));
        f.push(V::S(if a[3] > a[2] * 10 { "High Engagement" } else if a[3] < a[2] * 2 { "Low Engagement" } else { "Medium Engagement" }));
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetVoteCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, COALESCE(SUM(CASE WHEN c.PostId IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.OwnerUserId),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// LatestPostHistory AS (SELECT postId, MAX(CreationDate) AS LastChangedDate FROM PostHistoryDetails GROUP BY postId),
// FullPostDetails AS (SELECT ps.PostId, ps.Title, ps.OwnerUserId, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount, u.DisplayName AS OwnerDisplayName,
//        upc.UpVoteCount AS OwnerUpVoteCount, downc.DownVoteCount AS OwnerDownVoteCount, lp.LastChangedDate,
//        CASE WHEN lp.LastChangedDate IS NULL THEN 'No History' ELSE 'History Exists' END AS HistoryStatus
//     FROM PostStatistics ps JOIN Users u ON ps.OwnerUserId = u.Id LEFT JOIN UserVoteCounts upc ON ps.OwnerUserId = upc.UserId
//     LEFT JOIN UserVoteCounts downc ON ps.OwnerUserId = downc.UserId LEFT JOIN LatestPostHistory lp ON ps.PostId = lp.PostId)
// SELECT fpd.PostId, fpd.Title, fpd.OwnerDisplayName, fpd.CommentCount, fpd.UpVoteCount, fpd.DownVoteCount, fpd.OwnerUpVoteCount, fpd.OwnerDownVoteCount, fpd.LastChangedDate,
//        fpd.HistoryStatus
// FROM FullPostDetails fpd WHERE fpd.CommentCount > 3 OR fpd.UpVoteCount - fpd.DownVoteCount > 10 ORDER BY fpd.CommentCount DESC, fpd.UpVoteCount DESC;
fn q24382(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let vt = &db.vote.vote_type_id;
    let uvc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vt).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ps = db.post.with(owner_user).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(vt).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let lp = db.post_history.with(post_history_type_id.is_in([10, 11, 12])).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let v = drain((&ps).filt(|a| a[0] > 3 || a[1] - a[2] > 10).and(owner_user.select(Ident::<User>::new().and(&uvc))).and((&lp).opt()));
    rows(v.into_iter().map(|(p, ((a, (u, w)), l))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(w[0]), V::I(w[1]), l.map_or(V::Null, V::T), V::S(if l.is_none() { "No History" } else { "History Exists" })]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, AVG(u.Reputation) AS AvgReputation, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY COUNT(p.Id) DESC) AS RN
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, AvgReputation FROM UserPostStats WHERE RN <= 10),
// PostVoteDetails AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// PostInfo AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId, COALESCE(ph.Comment, 'No comments') AS LastEditComment, pvd.Upvotes, pvd.Downvotes
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 4 LEFT JOIN PostVoteDetails pvd ON p.Id = pvd.PostId),
// QualifiedPosts AS (SELECT pi.PostId, pi.Title, pi.CreationDate, pi.ViewCount, pi.OwnerUserId, pi.LastEditComment, pi.Upvotes, pi.Downvotes, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM PostInfo pi LEFT JOIN Comments c ON pi.PostId = c.PostId WHERE pi.Upvotes - pi.Downvotes > 0
//     GROUP BY pi.PostId, pi.Title, pi.CreationDate, pi.ViewCount, pi.OwnerUserId, pi.LastEditComment, pi.Upvotes, pi.Downvotes),
// FinalResults AS (SELECT pu.DisplayName, qp.Title, qp.CreationDate, qp.ViewCount, qp.LastEditComment, qp.CommentCount, qp.Upvotes, qp.Downvotes,
//        DENSE_RANK() OVER (ORDER BY qp.Upvotes DESC) AS VoteRank FROM QualifiedPosts qp INNER JOIN TopUsers pu ON qp.OwnerUserId = pu.UserId)
// SELECT fr.*, CASE WHEN fr.Upvotes = 0 AND fr.Downvotes = 0 THEN 'No Votes' WHEN fr.Upvotes - fr.Downvotes > 0 THEN 'Positive' WHEN fr.Upvotes - fr.Downvotes < 0 THEN 'Negative'
//        ELSE 'Neutral' END AS VoteStatus
// FROM FinalResults fr WHERE fr.VoteRank <= 5 ORDER BY fr.VoteRank;
//
// RN partitions by the user, so every user is in TopUsers. QualifiedPosts has a row per post and distinct type-4 edit comment.
fn q20436(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let pvd = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let e4 = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(4))).select(comment.opt());
    let lec = e4.opt().map(|c: Option<Option<Str>>| c.flatten().unwrap_or("No comments"));
    let qp: MatSet<(Id<Post>, Str)> = db.post.with(owner_user).with((&pvd).filt(|a| a[0] - a[1] > 0)).select(Ident::<Post>::new().and(lec)).collect();
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    type Q = (Id<Post>, Str);
    let v = drain((&qp).select(Same::<Q>::new().and(Same::<Q>::new().map(|(p, _): Q| p).select((&pvd).and(&cc)))));
    let v = ranked(v.into_iter().map(|x| x.1).collect::<Vec<_>>(), |&(_, (a, _))| Reverse(a[0]), true);
    rows(v.into_iter().take_while(|x| x.1 <= 5).map(|(((p, e), (a, c)), k)| {
        let mut f = vec![post_fields(db, p, &["owner"]).remove(0)];
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.extend([V::S(e), V::I(c), V::I(a[0]), V::I(a[1]), V::I(k)]);
        f.push(V::S(if a[0] == 0 && a[1] == 0 { "No Votes" } else if a[0] - a[1] > 0 { "Positive" } else if a[0] - a[1] < 0 { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// Rewritten (rewrites/21765.sql): R.AnswerId carried into CombinedData and the ORDER BY refined with `, AnswerId, LinkTypeDescription`.
// WITH CTE_UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, CASE WHEN U.Reputation IS NULL THEN 'Unknown Reputation' WHEN U.Reputation > 1000 THEN 'High Reputation'
//        ELSE 'Low Reputation' END AS REPUTATION, P.ViewCount AS PostViewCount, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY P.CreationDate DESC) AS PostRowNum
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId),
// CTE_RecentPosts AS (SELECT A.Id AS AnswerId, A.OwnerUserId, A.Title, A.CreationDate, A.AcceptedAnswerId,
//        CASE WHEN A.AcceptedAnswerId IS NULL THEN 'Not Accepted' ELSE 'Accepted' END AS AcceptanceStatus,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = A.Id AND V.VoteTypeId = 2) AS UpVoteCount, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = A.Id AND V.VoteTypeId = 3) AS DownVoteCount
//     FROM Posts A WHERE A.PostTypeId = 2 AND A.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// CTE_PostLinks AS (SELECT PL.PostId, PL.RelatedPostId, PL.LinkTypeId, CASE WHEN PL.LinkTypeId = 1 THEN 'Standard Link' WHEN PL.LinkTypeId = 3 THEN 'Duplicate Link'
//        ELSE 'Other Link Type' END AS LinkTypeDescription FROM PostLinks PL),
// CombinedData AS (SELECT U.DisplayName, U.Reputation, R.AnswerId, R.Title AS RecentPostTitle, R.AcceptanceStatus, R.UpVoteCount, R.DownVoteCount, P.ViewCount AS MostViewedPost, L.LinkTypeDescription
//     FROM CTE_UserReputation U JOIN CTE_RecentPosts R ON U.UserId = R.OwnerUserId JOIN (SELECT OwnerUserId, MAX(ViewCount) AS ViewCount FROM Posts GROUP BY OwnerUserId) AS P
//     ON U.UserId = P.OwnerUserId LEFT JOIN CTE_PostLinks L ON R.AnswerId = L.PostId WHERE U.Reputation IS NOT NULL)
// SELECT DisplayName, Reputation, RecentPostTitle, AcceptanceStatus, UpVoteCount, DownVoteCount, MostViewedPost, LinkTypeDescription,
//        CASE WHEN UpVoteCount - DownVoteCount > 10 THEN 'Highly Liked' WHEN UpVoteCount - DownVoteCount < 0 THEN 'Disliked' ELSE 'Neutral' END AS PostSentiment
// FROM CombinedData WHERE (CASE WHEN UpVoteCount - DownVoteCount > 10 THEN 'Highly Liked' WHEN UpVoteCount - DownVoteCount < 0 THEN 'Disliked' ELSE 'Neutral' END)
//     IN ('Highly Liked', 'Disliked')
// ORDER BY Reputation DESC, UpVoteCount DESC, AnswerId, LinkTypeDescription FETCH FIRST 100 ROWS ONLY;
//
// U.Reputation is the CASE text, so the sort is on that string, and U has one row per post of the user.
fn q21765(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, view_count, accepted_answer_id, .. } = &db.post;
    let vc = db.post.with(post_type_id.eq(2)).with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let mv = db.post.group_by(owner_user).select(view_count.opt()).fold(None::<i64>, |m, w| match (m, w) { (Some(a), Some(b)) => Some(a.max(b)), (a, b) => a.or(b) });
    let rep_txt = |u: Id<User>| if db.user.reputation.get(u).unwrap() > 1000 { "High Reputation" } else { "Low Reputation" };
    let r = (&vc).filt(|a| a[0] - a[1] > 10 || a[0] - a[1] < 0);
    let v = drain(r.and(owner_user.select(Ident::<User>::new().and(posts_of(db)).and(&mv))).and(links_of(db).opt()));
    let ld = |l: Option<Id<PostLink>>| l.map(|l| match db.post_link.link_type_id.get(l).unwrap() { 1 => "Standard Link", 3 => "Duplicate Link", _ => "Other Link Type" });
    let v = top_n(v, |&(p, ((a, ((u, _), _)), l))| (Reverse(rep_txt(u)), Reverse(a[0]), p, ld(l).is_none(), ld(l)), 100);
    rows(v.into_iter().map(|(p, ((a, ((u, _), m)), l))| {
        let mut f = vec![user_col(db, u, "name"), V::S(rep_txt(u)), title(db, p)];
        f.push(V::S(if accepted_answer_id.get(p).is_none() { "Not Accepted" } else { "Accepted" }));
        f.extend([V::I(a[0]), V::I(a[1]), oint(m)]);
        f.push(ld(l).map_or(V::Null, V::S));
        f.push(V::S(if a[0] - a[1] > 10 { "Highly Liked" } else { "Disliked" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT P.Id) AS PostCount,
//        RANK() OVER (ORDER BY COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) DESC) AS VoteRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// CommentStats AS (SELECT C.UserId, COUNT(*) AS CommentCount, MAX(C.CreationDate) AS LastCommentDate FROM Comments C GROUP BY C.UserId),
// BadgeSummary AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId),
// ClosePostStats AS (SELECT PH.UserId, COUNT(DISTINCT P.Id) AS ClosedPostCount, MAX(PH.CreationDate) AS LastCloseDate FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id
//     WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId),
// OverallStats AS (SELECT US.UserId, US.DisplayName, US.Reputation, US.UpVotes, US.DownVotes, COALESCE(CS.CommentCount, 0) AS CommentCount,
//        COALESCE(CS.LastCommentDate, '1900-01-01') AS LastCommentDate, COALESCE(BS.GoldBadges, 0) AS GoldBadges, COALESCE(BS.SilverBadges, 0) AS SilverBadges,
//        COALESCE(BS.BronzeBadges, 0) AS BronzeBadges, COALESCE(CPS.ClosedPostCount, 0) AS ClosedPostCount, COALESCE(CPS.LastCloseDate, '1900-01-01') AS LastCloseDate, US.VoteRank
//     FROM UserStats US LEFT JOIN CommentStats CS ON US.UserId = CS.UserId LEFT JOIN BadgeSummary BS ON US.UserId = BS.UserId LEFT JOIN ClosePostStats CPS ON US.UserId = CPS.UserId)
// SELECT OS.DisplayName, OS.Reputation, OS.UpVotes, OS.DownVotes, OS.CommentCount, OS.GoldBadges + OS.SilverBadges + OS.BronzeBadges AS TotalBadges, OS.ClosedPostCount,
//        CASE WHEN OS.ClosedPostCount > 5 THEN 'Frequent Closer' WHEN OS.ClosedPostCount BETWEEN 1 AND 5 THEN 'Occasional Closer' ELSE 'Rarely Closes' END AS ClosingBehavior,
//        ROW_NUMBER() OVER (ORDER BY OS.Reputation DESC) AS UserRank
// FROM OverallStats OS WHERE OS.Reputation > 1000 AND (OS.CommentCount > 0 OR OS.ClosedPostCount > 0) ORDER BY OS.Reputation DESC, OS.UserId FETCH FIRST 100 ROWS ONLY;
//
// The filter and the sort read only Reputation and the comment/close counts, so the hundred users are picked first and the posts x votes product is driven for
// those alone. UserRank ties on Reputation go to the smaller user id.
fn q21749(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let cs = db.comment.group_by(&db.comment.user).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { user, post, post_history_type_id, .. } = &db.post_history;
    let cps = db.post_history.with(post_history_type_id.eq(10)).group_by(user).select(post).count_distinct();
    let el = drain(db.user.with(rep.gt(1000)).select((&cs).opt().and((&cps).opt())));
    let el = drain(rel(el).filt(|(_, (c, k)): (Id<User>, (Option<i64>, Option<i64>))| c.map_or(false, |c| c > 0) || k.map_or(false, |k| k > 0)));
    let top = top_n(el.into_iter().map(|x| x.1).collect::<Vec<_>>(), |&(u, _)| (Reverse(rep.get(u).unwrap()), db.user.origid.get(u).unwrap()), 100);
    let tu = rel(top);
    type T = (Id<User>, (Option<i64>, Option<i64>));
    let tuk: MatSet<Id<User>> = (&tu).map(|(u, _): T| u).collect();
    let ust = (&tuk).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let bs = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select((&ust).and((&bs).opt())))));
    rows(v.into_iter().map(|(i, ((u, (c, k)), (a, b)))| {
        let k = k.unwrap_or(0);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0)), V::I(b.unwrap_or(0)), V::I(k)]);
        f.push(V::S(if k > 5 { "Frequent Closer" } else if k >= 1 { "Occasional Closer" } else { "Rarely Closes" }));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH RecursiveUserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate, COALESCE(b.BadgeCount, 0) AS BadgeCount,
//        COALESCE(v.VoteCount, 0) AS VoteCount, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS RN
//     FROM Users u LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON u.Id = b.UserId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY UserId) v ON u.Id = v.UserId),
// PostDetails AS (SELECT p.Id AS PostId, p.OwnerUserId, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.OwnerUserId, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount),
// FilteredPosts AS (SELECT pd.PostId, pd.OwnerUserId, pd.PostTypeId, pd.CreationDate, pd.Score, pd.ViewCount, pd.CommentCount, pd.UpVoteCount, pd.DownVoteCount,
//        (pd.UpVoteCount - pd.DownVoteCount) AS NetScore, CASE WHEN pd.PostTypeId = 1 THEN 'Question' WHEN pd.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END AS PostType
//     FROM PostDetails pd WHERE pd.Score >= 0),
// TopUsers AS (SELECT rus.UserId, rus.DisplayName, rus.BadgeCount, rus.VoteCount, RANK() OVER (ORDER BY rus.Reputation DESC) AS UserRank FROM RecursiveUserStats rus WHERE rus.VoteCount > 10),
// UserPostStats AS (SELECT fu.UserId, fu.DisplayName, fp.PostId, fp.PostType, fp.NetScore, fp.CreationDate, COUNT(pv.PostId) AS UserPostVoteCount, COUNT(DISTINCT fc.Id) AS UserCommentCount
//     FROM FilteredPosts fp JOIN TopUsers fu ON fp.OwnerUserId = fu.UserId LEFT JOIN Votes pv ON fp.PostId = pv.PostId AND pv.UserId = fu.UserId
//     LEFT JOIN Comments fc ON fp.PostId = fc.PostId AND fc.UserId = fu.UserId GROUP BY fu.UserId, fu.DisplayName, fp.PostId, fp.PostType, fp.NetScore, fp.CreationDate)
// SELECT ups.DisplayName, COUNT(ups.PostId) AS TotalPosts, SUM(ups.UserPostVoteCount) AS TotalUserVotes, SUM(ups.UserCommentCount) AS TotalComments, AVG(ups.NetScore) AS AverageNetScore
// FROM UserPostStats ups WHERE ups.NetScore > 0 GROUP BY ups.DisplayName HAVING COUNT(ups.PostId) > 5 ORDER BY AverageNetScore DESC LIMIT 10;
//
// NetScore is taken over the post's comment x vote rows, as PostDetails joins them.
fn q20887(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let Vote { vote_type_id, user, .. } = &db.vote;
    let vc = db.vote.with(vote_type_id.eq(2)).group_by(user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let tu = Ident::<User>::new().with((&vc).filt(|n| n > 10));
    let fp: MatSet<Id<Post>> = db.post.with(score.ge(0)).with(owner_user.select(tu)).collect();
    let pd = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(vote_type_id).opt())).fold(0i64, |n, (_, t)| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let pvs = own_votes(db);
    let Comment { user: cu, post: cp, .. } = &db.comment;
    let own_c: HashIdx<Id<Post>, Id<Comment>> = db.comment.with(cu.and(cp.select(owner_user)).filt(|(a, b)| a == b)).select(cp).inv().collect();
    let pvn = (&fp).group_by(Ident::<Post>::new()).select(pvs.opt().and((&own_c).opt())).fold(0i64, |n, (v, _)| n + v.is_some() as i64);
    let pcn = (&fp).group_by(Ident::<Post>::new()).select((&own_c).opt()).buf_fold(|xs| distinct_some(xs.iter().copied()));
    let g = (&fp)
        .with((&pd).filt(|n| n > 0))
        .group_by(owner_user.select(&db.user.display_name))
        .select(Ident::<Post>::new().and(&pd).and(&pvn).and(&pcn))
        .fold([0i64; 4], |a, (((_, n), v), c)| [a[0] + 1, a[1] + v, a[2] + c, a[3] + n]);
    let v = top_n(drain((&g).filt(|a| a[0] > 5)), |&(d, a)| (Reverse(fkey(a[3] as f64 / a[0] as f64)), d), 10);
    rows(v.into_iter().map(|(d, a)| row(vec![V::S(d), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])])))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, p.OwnerUserId, p.AcceptedAnswerId, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// PostStatistics AS (SELECT rp.OwnerUserId, COUNT(rp.PostId) AS TotalPosts, SUM(CASE WHEN rp.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN rp.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(rp.ViewCount) AS AvgViews, MAX(rp.CreationDate) AS LastPostDate FROM RecentPosts rp GROUP BY rp.OwnerUserId),
// PostHistoryAnalysis AS (SELECT ph.UserId, COUNT(ph.Id) AS TotalEdits, SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS Edits,
//        SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS Closures, SUM(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 ELSE 0 END) AS Deletions
//     FROM PostHistory ph GROUP BY ph.UserId),
// FlaggedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.TotalQuestions, 0) AS TotalQuestions,
//        COALESCE(ps.TotalAnswers, 0) AS TotalAnswers, COALESCE(ps.AvgViews, 0) AS AvgViews, COALESCE(pa.TotalEdits, 0) AS TotalEdits, COALESCE(pa.Closures, 0) AS TotalClosures,
//        COALESCE(pa.Deletions, 0) AS TotalDeletions
//     FROM Users u LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId LEFT JOIN PostHistoryAnalysis pa ON u.Id = pa.UserId
//     WHERE u.Reputation > 1000 OR EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = u.Id AND (b.Class = 1 OR b.Class = 2))),
// UserFlags AS (SELECT fu.UserId, fu.DisplayName, fu.Reputation, fu.TotalPosts, fu.TotalQuestions, fu.TotalAnswers, fu.AvgViews, fu.TotalEdits, fu.TotalClosures, fu.TotalDeletions,
//        CASE WHEN fu.TotalPosts < 5 THEN 'Newbie' WHEN fu.TotalPosts BETWEEN 5 AND 20 THEN 'Regular' ELSE 'Veteran' END AS UserType,
//        CASE WHEN fu.Reputation < 5000 THEN 'Low Reputation' WHEN fu.Reputation BETWEEN 5000 AND 10000 THEN 'Moderate Reputation' ELSE 'High Reputation' END AS ReputationCategory
//     FROM FlaggedUsers fu)
// SELECT uf.DisplayName, uf.UserType, uf.ReputationCategory, uf.TotalPosts, uf.TotalQuestions, uf.TotalAnswers, uf.AvgViews, uf.TotalEdits, uf.TotalClosures, uf.TotalDeletions,
//        COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
// FROM UserFlags uf LEFT JOIN Votes v ON uf.UserId = v.UserId
// GROUP BY uf.UserId, uf.DisplayName, uf.Reputation, uf.UserType, uf.ReputationCategory, uf.TotalPosts, uf.TotalQuestions, uf.TotalAnswers, uf.AvgViews, uf.TotalEdits,
//        uf.TotalClosures, uf.TotalDeletions
// ORDER BY uf.TotalPosts DESC, uf.Reputation DESC;
fn q22271(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, view_count, .. } = &db.post;
    let ps = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 5], |a, (t, w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
    });
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let pa = db.post_history.group_by(user).select(post_history_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + matches!(t, 10 | 11) as i64, a[2] + (t == 12) as i64]);
    let gs = Ident::<User>::new().with(badges_of(db).select((&db.badge.class).is_in([1, 2])));
    let rep = &db.user.reputation;
    let fu = db.user.with(Ident::<User>::new().with(rep.gt(1000)).or(gs));
    let tb = fu.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let v = drain((&tb).and((&ps).opt()).and((&pa).opt()));
    rows(v.into_iter().map(|(u, ((b, p), h))| {
        let r = rep.get(u).unwrap();
        let p = p.unwrap_or([0; 5]);
        let h = h.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "name")];
        f.push(V::S(if p[0] < 5 { "Newbie" } else if p[0] <= 20 { "Regular" } else { "Veteran" }));
        f.push(V::S(if r < 5000 { "Low Reputation" } else if r <= 10000 { "Moderate Reputation" } else { "High Reputation" }));
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), if p[3] == 0 { V::F(0.0) } else { avg(p[4], p[3]) }, V::I(h[0]), V::I(h[1]), V::I(h[2]), V::I(b)]);
        row(f)
    }))
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ARRAY_AGG(DISTINCT t.TagName) AS Tags
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN LATERAL (SELECT unnest(string_to_array(p.Tags, '<>')) AS TagName) t ON true
// WHERE p.CreationDate >= DATE '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName ORDER BY p.CreationDate DESC LIMIT 100;
//
// '<>' never occurs in Tags, so the unnest yields the whole Tags string once (NULL for an untagged post), and the ARRAY_AGG is that one value. The
// hundred newest posts are picked first (ties by id) and the comment x vote product is driven for those alone.
fn q10933(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, .. } = &db.post;
    let v = top_n(drain(db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0))).with(owner_user).select(creation_date)), |&(p, d)| (Reverse(d), p), 100);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain((&s).and(tags_str.opt()));
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::L(vec![ostr(t)]));
        row(f)
    }))
}

// WITH RankedQuestions AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, u.DisplayName AS OwnerDisplayName, STRING_AGG(t.TagName, ', ') AS Tags,
//        ROW_NUMBER() OVER (ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN (SELECT p.Id AS PostId, t.TagName FROM Posts p JOIN Tags t ON t.ExcerptPostId = p.Id) t ON t.PostId = p.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.ViewCount, u.DisplayName)
// SELECT PostId, Title, ViewCount, OwnerDisplayName, Tags FROM RankedQuestions WHERE Rank <= 10;
//
// Rank reads only ViewCount, so the ten questions are picked first (ties by id). The STRING_AGG has no ORDER BY; the port joins the names in Tags id order.
fn q14380(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(view_count.opt())), |&(p, w)| (w.is_none(), Reverse(w), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tags = (&tp).group_by(Ident::<Post>::new()).select((&ex).opt()).buf_fold(|xs| -> Option<&'static str> {
        let mut t: Vec<Id<Tag>> = xs.iter().flatten().copied().collect();
        t.sort();
        if t.is_empty() { None } else { Some(Box::leak(t.iter().map(|&t| db.tag.tag_name.get(t).unwrap()).collect::<Vec<_>>().join(", ").into_boxed_str())) }
    });
    let v = drain(&tags);
    rows(v.into_iter().map(|(p, t)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "owner"]);
        f.push(ostr(t));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        SUM(COALESCE(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END, 0)) AS TotalAnswers, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalViews, TotalScore, TotalAnswers, CommentCount, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, PostCount, TotalViews, TotalScore, TotalAnswers, CommentCount FROM TopUsers WHERE Rank <= 10 ORDER BY TotalScore DESC;
fn q13728(db: &'static So) -> String {
    let Post { view_count, score, post_type_id, answer_count, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(score).and(post_type_id).and(answer_count.opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((((w, s), t), n), c)) => [a[0] + w.unwrap_or(0), a[1] + s, a[2] + if t == 1 { n.unwrap_or(0) } else { 0 }, a[3] + c.is_some() as i64],
            None => a,
        });
    let v = top_n(drain((&us).and(user_distinct_posts(db))), |&(u, (a, _))| (Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Tags, u.DisplayName AS OwnerDisplayName, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS TagRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// QuestionTags AS (SELECT rp.PostId, UNNEST(STRING_TO_ARRAY(SUBSTRING(rp.Tags, 2, LENGTH(rp.Tags)-2), '><')) AS Tag FROM RankedPosts rp),
// TagScores AS (SELECT Tag, COUNT(*) AS QuestionsCount, SUM(rp.Score) AS TotalScore FROM QuestionTags qt JOIN RankedPosts rp ON qt.PostId = rp.PostId GROUP BY Tag)
// SELECT ts.Tag, ts.QuestionsCount, ts.TotalScore, CASE WHEN ts.QuestionsCount > 0 THEN ts.TotalScore / ts.QuestionsCount ELSE 0 END AS AverageScore
// FROM TagScores ts ORDER BY AverageScore DESC LIMIT 10;
fn q28155(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, tags_str, .. } = &db.post;
    let rp = db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user);
    let ts = rp.select(tags_str.flat_map(tag_list).and(score)).group_by(Same::<(Str, i64)>::new().map(|(t, _): (Str, i64)| t)).select(Same::<(Str, i64)>::new().map(|(_, s): (Str, i64)| s)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let v = top_n(drain(&ts), |&(t, a)| (Reverse(fkey(a[1] as f64 / a[0] as f64)), t), 10);
    rows(v.into_iter().map(|(t, a)| row(vec![V::S(t), V::I(a[0]), V::I(a[1]), V::F(a[1] as f64 / a[0] as f64)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.UserId END) AS UpVotes,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId = 3 THEN v.UserId END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.PostTypeId, p.CreationDate),
// TopPosts AS (SELECT r.* FROM RankedPosts r WHERE r.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes, COALESCE(b.Name, 'No Badge') AS TopBadge
// FROM TopPosts tp LEFT JOIN Badges b ON tp.PostId = b.UserId AND b.Class = 1 ORDER BY tp.Score DESC;
//
// Rank reads only base columns, so the ten posts per type are picked first (ties by id) and the comment x vote product is driven for those alone.
// `tp.PostId = b.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q9635(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let Vote { vote_type_id, user_id, .. } = &db.vote;
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let ud = |t: i64| (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(t))).select(user_id).opt()).buf_fold(|xs| distinct_some(xs.iter().copied()));
    let (up, dn) = (ud(2), ud(3));
    let gold: HashIdx<i64, Id<Badge>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user_id).inv().collect();
    let v = drain((&cc).and(&up).and(&dn).and(origid.select(&gold).opt()));
    rows(v.into_iter().map(|(p, (((c, u), d), b))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap()))]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalClosedPosts,
//        SUM(COALESCE(V.BountyAmount, 0)) AS TotalBountyAmount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalClosedPosts, TotalBountyAmount, RANK() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserActivity)
// SELECT T.DisplayName, T.TotalPosts, T.TotalQuestions, T.TotalAnswers, T.TotalClosedPosts, T.TotalBountyAmount FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.TotalPosts DESC;
//
// Rank reads only the distinct post count, so the top users are picked first and the posts x votes product is driven for those alone.
fn q5881(db: &'static So) -> String {
    let Post { post_type_id, closed_date, .. } = &db.post;
    let r = ranked(drain(user_distinct_posts(db)), |&(_, n)| Reverse(n), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ua = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(closed_date.opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some(((t, c), b)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let v = drain((&ua).and(user_distinct_posts(db)));
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments,
//        COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
//     GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalComments, TotalBounty, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC, TotalBounty DESC) AS Rank
//     FROM UserActivity)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalComments, TotalBounty FROM TopUsers WHERE Rank <= 10 ORDER BY TotalBounty DESC, TotalPosts DESC;
//
// Rank leads with the distinct post count, so only users at or above the tenth-highest count can make the cut; the product is driven for those alone.
// Ties in Rank go to the smaller user id.
fn q9090(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let dp = user_distinct_posts(db);
    let tenth = top_n(drain(dp), |&(u, n)| (Reverse(n), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with(user_distinct_posts(db).filt(move |n| n >= tenth)).collect();
    let b8 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ua = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(comments_of(db).opt()).and(b8.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some(((t, c), b)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let v = top_n(drain((&ua).and(user_distinct_posts(db))), |&(u, (a, n))| (Reverse(n), Reverse(a[3]), u), 10);
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, U.Reputation AS OwnerReputation,
//        U.DisplayName AS OwnerDisplayName, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id) AS TotalVotes, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS TotalComments
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1),
// TagStats AS (SELECT p.Id AS PostId, STRING_AGG(t.TagName, ', ') AS Tags FROM Posts p JOIN UNNEST(STRING_TO_ARRAY(p.Tags, '<>')) AS t(TagName) ON true GROUP BY p.Id)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.AnswerCount, ps.CommentCount, ps.OwnerReputation, ps.OwnerDisplayName, ps.TotalVotes, ps.TotalComments, ts.Tags
// FROM PostStats ps LEFT JOIN TagStats ts ON ps.PostId = ts.PostId ORDER BY ps.CreationDate DESC LIMIT 100;
//
// '<>' never occurs in Tags, so each tagged post unnests to its whole Tags string and the STRING_AGG is that string. The hundred newest questions are picked
// first (ties by id).
fn q14273(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(creation_date)), |&(p, d)| (Reverse(d), p), 100);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&vc).and(&cc));
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "rep", "owner"]);
        f.extend([V::I(a), V::I(b)]);
        f.extend(post_fields(db, p, &["tags"]));
        row(f)
    }))
}

// Rewritten (rewrites/14246.sql): the float AVG of epoch seconds became the exact-integer mean.
// WITH UserPostStats AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount,
//        SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.Id) / 1e6 AS AvgPostLifeSpan
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// TopUsers AS (SELECT UserId, PostCount, Upvotes, Downvotes, QuestionCount, AnswerCount, AvgPostLifeSpan, RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts FROM UserPostStats)
// SELECT UserId, PostCount, Upvotes, Downvotes, QuestionCount, AnswerCount, AvgPostLifeSpan FROM TopUsers WHERE RankByPosts <= 10;
fn q14246(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, creation_date, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(last_activity_date.and(creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold((0i64, 0i64, 0i64, 0i128), |a, p| match p {
            Some(((l, c), t)) => (a.0 + 1, a.1 + (t == Some(2)) as i64, a.2 + (t == Some(3)) as i64, a.3 + (l - c) as i128),
            None => a,
        });
    let qa = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64]);
    let r = ranked(drain((&ups).and(&qa)), |&(_, (a, _))| Reverse(a.0), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, q)), _)| {
        let mut f = ucols(db, u, &["uid"]);
        f.extend([V::I(a.0), V::I(a.1), V::I(a.2), V::I(q[0]), V::I(q[1]), if a.0 == 0 { V::Null } else { V::F(a.3 as f64 / a.0 as f64 / 1e6) }]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS Rnk FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Body IS NOT NULL),
// FilteredTags AS (SELECT UNNEST(string_to_array(Tags, ',')) AS Tag FROM RankedPosts WHERE Rnk <= 10),
// TagCounts AS (SELECT Tag, COUNT(*) AS TagCount FROM FilteredTags GROUP BY Tag),
// TopTags AS (SELECT Tag, TagCount, RANK() OVER (ORDER BY TagCount DESC) AS Rank FROM TagCounts)
// SELECT tt.Tag, tt.TagCount, p.Title, p.OwnerDisplayName, p.Score, p.ViewCount, p.CreationDate
// FROM TopTags tt JOIN RankedPosts p ON p.Tags LIKE '%' || tt.Tag || '%' WHERE tt.Rank <= 5 ORDER BY tt.TagCount DESC, p.Score DESC;
//
// Tags never contains ',', so each Tag is a post's whole Tags string, and TagCount is the number of Rnk <= 10 rows in that partition: min(10, its questions),
// whichever ten the ROW_NUMBER keeps. The LIKE is a raw substring test on the Tags text.
fn q26871(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let tc = rp().group_by(tags_str).select(Ident::<Post>::new()).fold(0i64, |n, _| (n + 1).min(10));
    let r = ranked(drain(&tc), |&(_, n)| Reverse(n), false);
    let tt = rel(r.into_iter().take_while(|x| x.1 <= 5).map(|x| x.0).collect::<Vec<(Str, i64)>>());
    let tti: HashIdx<Str, (Str, i64)> = (&tt).map(|(t, _)| t).inv().select(&tt).collect();
    let full: MatSet<Str> = rp().select(tags_str).collect();
    let like: HashIdx<Str, (Str, i64)> = (&full).select_where(&tti, |f: Str, t: Str| f.contains(t)).collect();
    let v = drain(rp().select(tags_str.select(&like)));
    rows(v.into_iter().map(|(p, (t, n))| {
        let mut f = vec![V::S(t), V::I(n)];
        f.extend(post_fields(db, p, &["title", "owner", "score", "views", "created"]));
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COALESCE(COUNT(DISTINCT P.Id), 0) AS PostCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, CommentCount, PostCount, (UpVotes - DownVotes) AS NetVotes,
//        RANK() OVER (ORDER BY PostCount DESC, (UpVotes - DownVotes) DESC) AS UserRank FROM UserEngagement)
// SELECT TU.UserId, TU.DisplayName, TU.UpVotes, TU.DownVotes, TU.CommentCount, TU.PostCount, TU.NetVotes FROM TopUsers TU WHERE TU.UserRank <= 10 ORDER BY TU.UserRank;
//
// UserRank leads with the distinct post count, so only users at or above the tenth-highest count can rank in the top ten; the votes x posts x comments
// product is driven for those alone.
fn q9114(db: &'static So) -> String {
    let tenth = top_n(drain(user_distinct_posts(db)), |&(u, n)| (Reverse(n), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with(user_distinct_posts(db).filt(move |n| n >= tenth)).collect();
    let ue = (&cand)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(comments_of(db).opt()).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.flatten().is_some() as i64]);
    let r = ranked(drain((&ue).and(user_distinct_posts(db))), |&(_, (a, n))| (Reverse(n), Reverse(a[0] - a[1])), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH UserScores AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, Upvotes, Downvotes, PostCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserScores WHERE PostCount > 5)
// SELECT U.DisplayName, U.TotalScore, U.Upvotes, U.Downvotes, U.ScoreRank, COUNT(DISTINCT C.Id) AS CommentCount
// FROM TopUsers U JOIN Comments C ON U.UserId = C.UserId JOIN PostHistory PH ON C.PostId = PH.PostId
// WHERE PH.CreationDate BETWEEN TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND TIMESTAMP '2024-10-01 12:34:56'
// GROUP BY U.DisplayName, U.TotalScore, U.Upvotes, U.Downvotes, U.ScoreRank ORDER BY U.ScoreRank LIMIT 10;
fn q7679(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let us = db
        .user
        .with(rep.gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((s, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + s],
            None => a,
        });
    let r = ranked(drain((&us).and(user_distinct_posts(db).filt(|n| n > 5))), |&(_, (a, _))| Reverse(a[2]), false);
    let tu = rel(r);
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let hd = &db.post_history.creation_date;
    let recent = Ident::<Comment>::new().with((&db.comment.post).select(history_of(db).select(hd.between(add_days(t0, -30), t0))));
    let cc = db.user.group_by(Ident::<User>::new()).select(comments_by(db).select(recent)).fold(0i64, |n, _| n + 1);
    type T = ((Id<User>, ([i64; 3], i64)), i64);
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|((u, _), _): T| u).select(&cc))));
    let v = top_n(v, |&(_, (((u, _), k), _))| (k, u), 10);
    rows(v.into_iter().map(|(_, (((u, (a, _)), k), c))| row(vec![user_col(db, u, "name"), V::I(a[2]), V::I(a[0]), V::I(a[1]), V::I(k), V::I(c)])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY u.Location ORDER BY p.Score DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND u.Reputation > 1000),
// RecentPosts AS (SELECT Id, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE PostRank <= 5)
// SELECT rp.Title, rp.CreationDate, rp.OwnerDisplayName, COALESCE(SUM(CASE WHEN c.UserId IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount
// FROM RecentPosts rp LEFT JOIN Comments c ON rp.Id = c.PostId LEFT JOIN Votes v ON rp.Id = v.PostId WHERE rp.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
// GROUP BY rp.Id, rp.Title, rp.CreationDate, rp.OwnerDisplayName ORDER BY rp.CreationDate DESC;
//
// PostRank ties on Score go to the smaller post id; the users without a Location are one partition.
fn q31378(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let hi = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.select(hi).select((&db.user.location).opt())));
    let top = top_per(v, |&(_, l)| l, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cu = comments_of(db).select((&db.comment.user).opt());
    let s = (&tp)
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(cu.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.flatten().is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let v = drain(&s);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.PostCount, ua.QuestionCount, ua.AnswerCount, ua.TotalBounty, ua.BadgeCount,
//        RANK() OVER (ORDER BY ua.PostCount DESC, ua.BadgeCount DESC) AS Rank FROM UserActivity ua)
// SELECT tu.Rank, tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.TotalBounty, tu.BadgeCount FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// Rank leads with the distinct post count, so only users at or above the tenth-highest count can rank in the top ten; the posts x votes x badges product is
// driven for those alone.
fn q7896(db: &'static So) -> String {
    let tenth = top_n(drain(user_distinct_posts(db)), |&(u, n)| (Reverse(n), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with(user_distinct_posts(db).filt(move |n| n >= tenth)).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bv = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([8, 9]))).select(bounty_amount.opt());
    let ua = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(bv.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (t, bo) = p.map_or((0, None), |(t, x)| (t, x.flatten()));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + bo.unwrap_or(0), a[3] + b.is_some() as i64]
        });
    let r = ranked(drain((&ua).and(user_distinct_posts(db))), |&(_, (a, n))| (Reverse(n), Reverse(a[3])), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), k)| {
        row(vec![V::I(k), user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])])
    }))
}

// WITH TaggedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount, p.Tags, p.OwnerUserId, u.DisplayName AS OwnerDisplayName
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Tags IS NOT NULL),
// TagCounts AS (SELECT unnest(string_to_array(trim(both '<>' from Tags), '> <')) AS Tag, PostId FROM TaggedPosts),
// TagStatistics AS (SELECT Tag, COUNT(*) AS PostCount, AVG(ViewCount) AS AvgViewCount, AVG(AnswerCount) AS AvgAnswerCount FROM TagCounts tc JOIN TaggedPosts tp ON tc.PostId = tp.PostId
//     GROUP BY Tag),
// PopularTags AS (SELECT Tag, PostCount, AvgViewCount, AvgAnswerCount, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagStatistics)
// SELECT t.Tag, t.PostCount, t.AvgViewCount, t.AvgAnswerCount FROM PopularTags t WHERE t.TagRank <= 10 ORDER BY t.PostCount DESC;
//
// '> <' never occurs in Tags, so each post's Tag is its Tags string with the outer '<' and '>' trimmed.
fn q27383(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, view_count, answer_count, .. } = &db.post;
    let tag = tags_str.map(|t: Str| -> Str { t.trim_matches(|c| c == '<' || c == '>') });
    let ts = db.post.with(post_type_id.eq(1)).with(owner_user).group_by(tag).select(view_count.opt().and(answer_count.opt())).fold([0i64; 5], |a, (w, n)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + n.is_some() as i64, a[4] + n.unwrap_or(0)]
    });
    let r = ranked(drain(&ts), |&(_, a)| Reverse(a[0]), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((t, a), _)| row(vec![V::S(t), V::I(a[0]), avg(a[2], a[1]), avg(a[4], a[3])])))
}

// WITH RankedUserVotes AS (SELECT v.UserId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN v.VoteTypeId = 10 THEN 1 END) AS Deletions, RANK() OVER (ORDER BY COUNT(v.Id) DESC) AS VoteRank
//     FROM Votes v JOIN Posts p ON v.PostId = p.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY v.UserId),
// ActiveUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, u.LastAccessDate, COUNT(DISTINCT p.Id) AS ActivePosts FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.LastAccessDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY u.Id, u.DisplayName, u.Reputation, u.LastAccessDate)
// SELECT au.DisplayName, au.Reputation, au.ActivePosts, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes, COALESCE(rv.Deletions, 0) AS Deletions,
//        COALESCE(rv.VoteRank, 0) AS VoteRank
// FROM ActiveUsers au LEFT JOIN RankedUserVotes rv ON au.Id = rv.UserId WHERE au.Reputation > 1000 ORDER BY rv.VoteRank, au.DisplayName;
//
// The votes with no UserId form one group of RankedUserVotes, which takes part in VoteRank but joins no user.
fn q5211(db: &'static So) -> String {
    let today = current_date();
    let Vote { user, post, vote_type_id, .. } = &db.vote;
    let recent = Ident::<Post>::new().with((&db.post.creation_date).ge(add_years(today, -1)));
    let rv = db.vote.with(post.select(recent)).group_by(user.opt()).select(vote_type_id).fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (t == 10) as i64]);
    let rr = rel(ranked(drain(&rv), |&(_, a)| Reverse(a[0]), false));
    type R = ((Option<Id<User>>, [i64; 4]), i64);
    let rv_by: HashIdx<Option<Id<User>>, R> = (&rr).map(|((u, _), _): R| u).inv().select(&rr).collect();
    let au = db.user.with((&db.user.last_access_date).ge(add_months(today, -6))).with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&au).and(Ident::<User>::new().map(|u: Id<User>| Some(u)).select(&rv_by).opt()));
    rows(v.into_iter().map(|(u, (n, r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(match r {
            Some(((_, a), k)) => [V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(k)],
            None => [V::I(0), V::I(0), V::I(0), V::I(0)],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerUserId, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.Rank = 1)
// SELECT u.DisplayName AS Owner, tp.Title, tp.Score, tp.ViewCount, COALESCE(b.Name, 'No Badge') AS BadgeName, COUNT(DISTINCT c.Id) AS CommentCount, MAX(v.CreationDate) AS LastVoteDate
// FROM TopPosts tp JOIN Users u ON tp.OwnerUserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId AND b.Class = 1 LEFT JOIN Comments c ON tp.PostId = c.PostId
// LEFT JOIN Votes v ON tp.PostId = v.PostId AND v.VoteTypeId IN (2, 3) GROUP BY u.DisplayName, tp.Title, tp.Score, tp.ViewCount, b.Name
// HAVING SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) > 10 ORDER BY tp.Score DESC, u.DisplayName ASC
//
// The GROUP BY is on the columns, not the post, and names a badge column, so the (post, gold badge) rows are materialised and grouped by
// (DisplayName, Title, Score, ViewCount, badge name). Rank ties on CreationDate go to the smaller post id.
fn q3852(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, title, score, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    type J = (Id<Post>, Option<Id<Badge>>);
    let j: MatSet<J> = (&tp).select(Ident::<Post>::new().and(owner_user.select(gold).opt())).collect();
    let jp = || Same::<J>::new().map(|(p, _): J| p);
    let key = jp().select(owner_user.select(&db.user.display_name)).and(jp().select(title.opt())).and(jp().select(score)).and(jp().select(view_count.opt())).and(
        Same::<J>::new().map(|(_, b): J| b).flat_map(|b: Option<Id<Badge>>| b).select(&db.badge.name).opt(),
    );
    let Vote { vote_type_id, creation_date: vd, .. } = &db.vote;
    let vs = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([2, 3]))).select(vote_type_id.and(vd));
    let g = (&j).group_by(key).select(jp().select(comments_of(db).opt().and(vs.opt()))).buf_fold(|xs| {
        let up = xs.iter().filter(|x| matches!(x.1, Some((2, _)))).count() as i64;
        let last = xs.iter().filter_map(|x| x.1.map(|y| y.1)).max().unwrap_or(i64::MIN);
        (distinct_some(xs.iter().map(|x| x.0)), up, last)
    });
    let v = drain((&g).filt(|(_, up, _)| up > 10));
    rows(v.into_iter().map(|(((((n, t), s), w), b), (c, _, d))| row(vec![V::S(n), ostr(t), V::I(s), oint(w), V::S(b.unwrap_or("No Badge")), V::I(c), tmax(d)])))
}

// WITH UserVoteCounts AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT P.Id) AS PostCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, PostCount, RANK() OVER (ORDER BY UpVotes DESC, PostCount DESC) AS Rank FROM UserVoteCounts WHERE PostCount > 0),
// RecentPostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' THEN 1 END) AS RecentPostCount,
//        AVG(P.Score) AS AvgScore FROM Posts P WHERE P.OwnerUserId IS NOT NULL GROUP BY P.OwnerUserId)
// SELECT U.DisplayName, U.UpVotes, U.DownVotes, U.PostCount, R.RecentPostCount, R.AvgScore FROM TopUsers U JOIN RecentPostStats R ON U.UserId = R.OwnerUserId WHERE U.Rank <= 10
// ORDER BY U.UpVotes DESC, R.AvgScore DESC;
fn q5395(db: &'static So) -> String {
    let Vote { vote_type_id, post, .. } = &db.vote;
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(post)).count_distinct();
    let r = ranked(drain((&uv).and((&pc).filt(|n| n > 0))), |&(_, (a, n))| (Reverse(a[0]), Reverse(n)), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect::<Vec<_>>());
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let cut = add_months(ts(2024, 10, 1, 12, 34, 56), -1);
    let rs = db.post.group_by(owner_user).select(creation_date.and(score)).fold([0i64; 3], |a, (d, s)| [a[0] + (d >= cut) as i64, a[1] + 1, a[2] + s]);
    type T = (Id<User>, ([i64; 2], i64));
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(&rs))));
    rows(v.into_iter().map(|(_, ((u, (a, n)), r))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n), V::I(r[0]), avg(r[2], r[1])])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, p.ViewCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL),
// RecentVotes AS (SELECT V.PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(*) AS TotalVotes
//     FROM Votes V WHERE V.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' GROUP BY V.PostId)
// SELECT RP.Title, RP.OwnerDisplayName, RP.Score, RP.ViewCount, COALESCE(RV.UpVotes, 0) AS UpVotes, COALESCE(RV.DownVotes, 0) AS DownVotes, RP.CreationDate,
//        CASE WHEN RP.PostRank = 1 THEN 'Top Post' ELSE NULL END AS PostStatus
// FROM RankedPosts RP LEFT JOIN RecentVotes RV ON RP.Id = RV.PostId WHERE RP.ViewCount > 1000 ORDER BY RP.Score DESC, RP.CreationDate DESC LIMIT 50;
//
// PostRank ties on Score go to the smaller post id.
fn q3948(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, creation_date, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let first = top_per(drain(qs().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = top_n(drain(qs().with(view_count.gt(1000)).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap()), p), 50);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let Vote { vote_type_id, creation_date: vd, .. } = &db.vote;
    let rv = votes_of(db).select(Ident::<Vote>::new().with(vd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(vote_type_id);
    let s = (&tp).group_by(Ident::<Post>::new()).select(rv.opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&s).and(Ident::<Post>::new().with(&first).opt()));
    rows(v.into_iter().map(|(p, (a, f1))| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::T(creation_date.get(p).unwrap()), if f1.is_some() { V::S("Top Post") } else { V::Null }]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT bh.Id) AS EditCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory bh ON p.Id = bh.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR') AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// PopularPosts AS (SELECT rp.*, ROW_NUMBER() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS Rank FROM RankedPosts rp)
// SELECT pp.PostId, pp.Title, pp.CreationDate, pp.Score, pp.ViewCount, pp.UpVotes, pp.DownVotes, pp.CommentCount, pp.EditCount, pp.Rank FROM PopularPosts pp WHERE pp.Rank <= 10
// ORDER BY pp.Rank;
//
// Rank reads only base columns, so the ten questions are picked first (ties by id) and the votes x comments x history product is driven for those alone.
fn q6680(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 10);
    let rk = rel(v.into_iter().enumerate().map(|(i, x)| (x.0, i as i64 + 1)).collect::<Vec<_>>());
    let tp: MatSet<Id<Post>> = (&rk).map(|(p, _)| p).collect();
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt())).fold([0i64; 2], |a, ((t, _), _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    type K = (Id<Post>, i64);
    let pk = || Same::<K>::new().map(|(p, _): K| p);
    let v = drain((&rk).select(Same::<K>::new().and(pk().select(&ud)).and(pk().select(&cc)).and(pk().select(&hc))));
    rows(v.into_iter().map(|(_, ((((p, k), a), c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(h), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.PostTypeId, p.CreationDate, p.ViewCount, DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.ViewCount, pv.UpVotes, pv.DownVotes FROM RankedPosts rp LEFT JOIN PostVotes pv ON rp.Id = pv.PostId WHERE rp.PostRank = 1)
// SELECT t.TagName, COUNT(DISTINCT tp.Id) AS PostCount, COALESCE(SUM(tp.UpVotes - tp.DownVotes), 0) AS NetVotes, AVG(tp.ViewCount) AS AvgViewCount
// FROM Tags t JOIN Posts p ON p.Tags ILIKE '%' || t.TagName || '%' JOIN TopPosts tp ON tp.Id = p.Id GROUP BY t.TagName HAVING COUNT(DISTINCT tp.Id) > 5 ORDER BY NetVotes DESC NULLS LAST;
//
// The ILIKE is a case-insensitive substring test on the Tags text.
fn q1557(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, view_count, .. } = &db.post;
    let recent = drain(db.post.with(creation_date.ge(ts(2023, 10, 1, 0, 0, 0))).select(post_type_id));
    let r = per_group(ranked(recent, |&(p, t)| (t, Reverse(creation_date.get(p).unwrap())), true), |&(_, t)| t);
    let tp: MatSet<Id<Post>> = rel(r).filt(|(_, k): ((Id<Post>, i64), i64)| k == 1).map(|((p, _), _)| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let full: MatSet<Str> = (&tp).select(tags_str).collect();
    let hit: HashIdx<Str, Id<Tag>> = (&full).select_where((&db.tag.tag_name).inv(), |f: Str, n: Str| f.to_lowercase().contains(&n.to_lowercase())).collect();
    let g = (&tp).select(tags_str.select(&hit).and(Ident::<Post>::new())).group_by(Same::<(Id<Tag>, Id<Post>)>::new().map(|(t, _): (Id<Tag>, Id<Post>)| t))
        .select(Same::<(Id<Tag>, Id<Post>)>::new().map(|(_, p): (Id<Tag>, Id<Post>)| p).select((&pv).opt().and(view_count.opt())))
        .fold([0i64; 5], |a, (v, w)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.map_or(0, |v| v[0] - v[1]), a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]);
    let v = drain((&g).filt(|a| a[0] > 5));
    rows(v.into_iter().map(|(t, a)| row(vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[2]), avg(a[4], a[3])])))
}

// WITH TagUsage AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COUNT(DISTINCT v.UserId) AS VoterCount
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY t.TagName),
// TagStats AS (SELECT TagName, PostCount, AnswerCount, VoterCount, ROUND((AnswerCount::FLOAT / NULLIF(PostCount, 0)) * 100, 2) AS AnswerRate,
//        ROUND((VoterCount::FLOAT / NULLIF(PostCount, 0)) * 100, 2) AS VoterEngagement FROM TagUsage),
// TopTags AS (SELECT TagName, PostCount, AnswerCount, VoterCount, AnswerRate, VoterEngagement, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagStats)
// SELECT tt.TagName, tt.PostCount, tt.AnswerCount, tt.VoterCount, tt.AnswerRate, tt.VoterEngagement FROM TopTags tt WHERE tt.TagRank <= 10 ORDER BY tt.PostCount DESC;
//
// PostCount and AnswerCount count the tag x post x vote rows. FLOAT is f32, so the rates are computed in f32.
fn q27662(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let lt = tag_mentions(db);
    type M = (Id<Post>, Id<Tag>);
    let recent = Same::<M>::new().map(|(p, _): M| p).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let tu = (&lt)
        .with(recent)
        .group_by(Same::<M>::new().map(|(_, t): M| t))
        .select(Same::<M>::new().map(|(p, _): M| p).select(post_type_id.and(votes_of(db).select((&db.vote.user_id).opt()).opt())))
        .buf_fold(|xs| {
            let n = xs.len() as i64;
            let a = xs.iter().filter(|x| x.0 == 2).count() as i64;
            (n, a, distinct_some(xs.iter().map(|x| x.1.flatten())))
        });
    let r = ranked(drain(&tu), |&(_, (n, _, _))| Reverse(n), false);
    let pct = |x: i64, n: i64| V::F(((x as f32 / n as f32 * 100.0f32 * 100.0).round() / 100.0) as f64);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((t, (n, a, c)), _)| row(vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(n), V::I(a), V::I(c), pct(a, n), pct(c, n)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount,
//        RANK() OVER (ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.UpVotes, tp.DownVotes, tp.BadgeCount, COUNT(c.Id) AS CommentCount
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.UpVotes, tp.DownVotes, tp.BadgeCount ORDER BY tp.Score DESC;
//
// Rank reads only Score, so the top questions are picked first and the votes x badges product is driven for those alone.
fn q7355(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let r = ranked(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(score)), |&(_, s)| Reverse(s), false);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(owner_user.select(badges_of(db)).opt())).fold([0i64; 3], |a, (t, b)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and(&cc));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation FROM Users WHERE Reputation > 1000),
// PopularPosts AS (SELECT P.Id, P.Title, P.ViewCount, P.Score, P.AnswerCount, U.Reputation AS UserReputation FROM Posts P JOIN UserReputation U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND P.Score > 0),
// PostTags AS (SELECT P.Id AS PostId, UNNEST(STRING_TO_ARRAY(P.Tags, '><')) AS Tag FROM Posts P WHERE P.PostTypeId = 1),
// TagPopularity AS (SELECT Tag, COUNT(Pt.PostId) AS PostCount FROM PostTags Pt GROUP BY Tag HAVING COUNT(Pt.PostId) > 5),
// PopularityScores AS (SELECT P.Id, P.Title, P.ViewCount, P.Score, P.AnswerCount, (P.ViewCount * 0.2 + P.Score * 0.7 + P.AnswerCount * 0.1) AS PopularityScore
//     FROM PopularPosts P JOIN TagPopularity T ON P.Title ILIKE '%' || T.Tag || '%'),
// RankedPosts AS (SELECT Id, Title, ViewCount, Score, AnswerCount, PopularityScore, ROW_NUMBER() OVER (ORDER BY PopularityScore DESC) AS Rank FROM PopularityScores)
// SELECT R.Title, R.ViewCount, R.Score, R.AnswerCount, R.Rank FROM RankedPosts R WHERE R.Rank <= 10 ORDER BY R.Rank;
//
// Splitting the whole Tags text on '><' leaves the first piece with its '<' and the last with its '>'. PopularityScore is exact decimal, so it is kept in tenths.
// Rows tied on it are ordered by post id, then tag; a post matching several tags gives identical rows but for Rank.
fn q5243(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, post_type_id, tags_str, title, view_count, answer_count, .. } = &db.post;
    let pieces = tags_str.flat_map(|t: Str| t.split("><"));
    let tp = db.post.with(post_type_id.eq(1)).select(pieces).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let tpv = rel(drain((&tp).filt(|n| n > 5)));
    let tpi: HashIdx<Str, (Str, i64)> = (&tpv).map(|(t, _)| t).inv().select(&tpv).collect();
    let hi = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let pp: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user.select(hi)).collect();
    let titles: MatSet<Str> = (&pp).select(title).collect();
    let hit: HashIdx<Str, (Str, i64)> = (&titles).select_where(&tpi, |t: Str, g: Str| t.to_lowercase().contains(&g.to_lowercase())).collect();
    let v = drain((&pp).select(title.select(&hit).map(|x: (Str, i64)| x.0).and(view_count.opt()).and(answer_count.opt())));
    let key = |&(p, ((g, w), a)): &(Id<Post>, ((Str, Option<i64>), Option<i64>))| {
        let sc = match (w, a) { (Some(w), Some(a)) => Some(w * 2 + score.get(p).unwrap() * 7 + a), _ => None };
        (sc.is_none(), Reverse(sc), p, g)
    };
    let v = top_n(v, key, 10);
    rows(v.into_iter().enumerate().map(|(i, (p, ((_, w), a)))| row(vec![harness::views::title(db, p), oint(w), V::I(score.get(p).unwrap()), oint(a), V::I(i as i64 + 1)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank, p.Tags FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// FilteredTags AS (SELECT pt.TagName, COUNT(pt.TagName) AS TagCount FROM Posts p JOIN LATERAL unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS pt(TagName) ON true
//     WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year' GROUP BY pt.TagName HAVING COUNT(pt.TagName) > 5)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.CreationDate AS PostCreationDate, rp.OwnerDisplayName, ft.TagName, ft.TagCount
// FROM RankedPosts rp JOIN FilteredTags ft ON ft.TagName = ANY (string_to_array(substring(rp.Tags, 2, length(rp.Tags) - 2), '><')) WHERE rp.Rank = 1
// ORDER BY rp.ViewCount DESC, ft.TagCount DESC LIMIT 10;
//
// Rank ties on CreationDate go to the smaller post id; rows tied at the cut are ordered by post, then tag.
fn q26382(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, tags_str, view_count, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ft = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let fts = (&ft).filt(|n| n > 5);
    let v = drain((&first).select(tags_str.flat_map(tag_list).select(Same::<Str>::new().and(fts))));
    let v = top_n(v, |&(p, (t, n))| (view_count.get(p).is_none(), Reverse(view_count.get(p)), Reverse(n), p, t), 10);
    rows(v.into_iter().map(|(p, (t, n))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "created", "owner"]);
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TagStatistics AS (SELECT TagName, COUNT(*) AS QuestionCount, SUM(Score) AS TotalScore
//     FROM (SELECT UNNEST(string_to_array(TRANSLATE(Tags, '<>', ''), '><')) AS TagName, Score FROM RankedPosts) AS UnnestedTags GROUP BY TagName),
// TopTags AS (SELECT TagName, QuestionCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS TagRank FROM TagStatistics)
// SELECT tt.TagName, tt.QuestionCount, tt.TotalScore, CASE WHEN tt.QuestionCount > 50 THEN 'Active' WHEN tt.QuestionCount > 10 THEN 'Moderate' ELSE 'Inactive' END AS TagActivityStatus
// FROM TopTags tt WHERE tt.TagRank <= 10 ORDER BY tt.TotalScore DESC;
//
// TRANSLATE deletes every '<' and '>', so there is no '><' left to split on and each post's TagName is its tags run together.
fn q28373(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, tags_str, .. } = &db.post;
    let name = tags_str.map(|t: Str| -> Str { Box::leak(t.replace(['<', '>'], "").into_boxed_str()) });
    let ts = db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).group_by(name).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let r = ranked(drain(&ts), |&(_, a)| Reverse(a[1]), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((t, a), _)| row(vec![V::S(t), V::I(a[0]), V::I(a[1]), V::S(if a[0] > 50 { "Active" } else if a[0] > 10 { "Moderate" } else { "Inactive" })])))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId
//     WHERE U.Reputation > 1000 AND U.CreationDate < (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT *, RANK() OVER (ORDER BY PostCount DESC, QuestionCount DESC, UpVotesReceived DESC) AS Rank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotesReceived, DownVotesReceived, BadgeCount FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Rank leads with the distinct post count, so only users at or above the tenth-highest count can make it; the posts x votes x badges product is driven for those alone.
fn q9455(db: &'static So) -> String {
    let User { reputation, creation_date, .. } = &db.user;
    let el = || db.user.with(reputation.gt(1000)).with(creation_date.lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let dp = el().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tenth = top_n(drain(&dp), |&(u, n)| (Reverse(n), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&dp).filt(move |n| n >= tenth)).collect();
    let ua = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let r = ranked(drain((&ua).and(&dp)), |&(_, (a, n))| (Reverse(n), Reverse(a[0]), Reverse(a[2])), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.Id END) AS UpVoteCount, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 3 THEN v.Id END) AS DownVoteCount,
//        SUM(COALESCE(b.Class, 0)) AS TotalBadgeClass, COUNT(DISTINCT ph.Id) AS EditCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// RankedPosts AS (SELECT pm.*, RANK() OVER (ORDER BY pm.UpVoteCount DESC, pm.CommentCount DESC, pm.CreationDate ASC) AS Rank FROM PostMetrics pm)
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.UpVoteCount, rp.DownVoteCount, rp.CommentCount, rp.EditCount, rp.TotalBadgeClass, rp.Rank FROM RankedPosts rp WHERE rp.Rank <= 10
// ORDER BY rp.Rank;
//
// Rank reads only the distinct counts, which do not need the product, so the ranked posts are picked first and the comments x votes x badges x history
// product is driven for those alone.
fn q6891(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let vt = &db.vote.vote_type_id;
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(vt).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let r = ranked(drain((&vc).and(&cc)), |&(p, (a, c))| (Reverse(a[0]), Reverse(c), creation_date.get(p).unwrap()), false);
    let tr = rel(r.into_iter().take_while(|x| x.1 <= 10).collect::<Vec<_>>());
    type R = ((Id<Post>, ([i64; 2], i64)), i64);
    let tp: MatSet<Id<Post>> = (&tr).map(|((p, _), _): R| p).collect();
    let bc = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(owner_user.select(badges_of(db)).select(&db.badge.class).opt()).and(history_of(db).opt()))
        .fold(0i64, |n, (((_, _), b), _)| n + b.unwrap_or(0));
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let pk = || Same::<R>::new().map(|((p, _), _): R| p);
    let v = drain((&tr).select(Same::<R>::new().and(pk().select(&bc)).and(pk().select(&hc))));
    rows(v.into_iter().map(|(_, ((((p, (a, c)), k), b), h))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(h), V::I(b), V::I(k)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// BestUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, TotalPosts, TotalComments, RANK() OVER (ORDER BY (UpVotes - DownVotes) DESC) AS RankUpDown FROM UserVoteStats)
// SELECT U.DisplayName, U.Reputation, B.TotalPosts, B.TotalComments, B.UpVotes, B.DownVotes, CASE WHEN B.RankUpDown <= 10 THEN 'Top Users' ELSE 'Regular Users' END AS UserCategory
// FROM BestUsers B JOIN Users U ON B.UserId = U.Id WHERE U.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' ORDER BY B.RankUpDown LIMIT 20;
//
// Rows tied on RankUpDown at the cut are ordered by user id.
fn q6948(db: &'static So) -> String {
    let uv = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((_, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
            None => a,
        });
    let tc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let r = ranked(drain((&uv).and(user_distinct_posts(db)).and(&tc)), |&(_, ((a, _), _))| Reverse(a[0] - a[1]), false);
    type R = ((Id<User>, (([i64; 2], i64), i64)), i64);
    let old = Ident::<User>::new().with((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let v = drain(rel(r).select(Same::<R>::new().with(Same::<R>::new().map(|((u, _), _): R| u).select(old))));
    let v = top_n(v.into_iter().map(|x| x.1).collect::<Vec<R>>(), |&((u, _), k)| (k, u), 20);
    rows(v.into_iter().map(|((u, ((a, n), c)), k)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), V::S(if k <= 10 { "Top Users" } else { "Regular Users" })]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore,
//        SUM(COALESCE(CAST(B.Class AS int), 0)) AS TotalBadges, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalScore, TotalBadges, TotalComments, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY TotalPosts DESC) AS PostsRank FROM UserStatistics)
// SELECT T.DisplayName, T.TotalPosts, T.Questions, T.Answers, T.TotalScore, T.TotalBadges, T.TotalComments, T.ScoreRank, T.PostsRank FROM TopUsers T
// WHERE T.ScoreRank <= 10 OR T.PostsRank <= 10 ORDER BY T.ScoreRank, T.PostsRank;
fn q9977(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (t, s) = p.map_or((0, 0), |((t, s), _)| (t, s));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + b.unwrap_or(0)]
        });
    let tc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let r = ranked(drain((&us).and(user_distinct_posts(db)).and(&tc)), |&(_, ((a, _), _))| Reverse(a[2]), false);
    let r = ranked(r, |&((_, ((_, n), _)), _)| Reverse(n), false);
    type R = (((Id<User>, (([i64; 4], i64), i64)), i64), i64);
    let v = drain(rel(r).filt(|((_, s), p): R| s <= 10 || p <= 10));
    rows(v.into_iter().map(|(_, (((u, ((a, n), c)), s), p))| {
        row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(c), V::I(s), V::I(p)])
    }))
}

// Rewritten (rewrites/637.sql): the final ORDER BY refined with `, PS.OwnerDisplayName`.
// WITH RankedPosts AS (SELECT P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY EXTRACT(YEAR FROM P.CreationDate) ORDER BY P.Score DESC, P.ViewCount DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.CreationDate >= DATE '2024-10-01' - INTERVAL '2 years'),
// PostStatistics AS (SELECT RP.OwnerDisplayName, COUNT(DISTINCT RP.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS TotalClosed,
//        COALESCE(SUM(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 ELSE 0 END), 0) AS TotalReopened, AVG(RP.Score) AS AvgScore, AVG(RP.ViewCount) AS AvgViewCount
//     FROM RankedPosts RP LEFT JOIN PostHistory PH ON RP.Id = PH.PostId GROUP BY RP.OwnerDisplayName)
// SELECT PS.OwnerDisplayName, PS.TotalPosts, PS.TotalClosed, PS.TotalReopened, PS.AvgScore, PS.AvgViewCount FROM PostStatistics PS WHERE PS.TotalPosts > 0
// ORDER BY PS.AvgScore DESC, PS.TotalPosts DESC, PS.OwnerDisplayName LIMIT 10;
//
// The averages run over the post x history rows. Rank is never read.
fn q637(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let rp = db.post.with(post_type_id.eq(1).and(creation_date.ge(ts(2022, 10, 1, 0, 0, 0)))).with(owner_user);
    let key = owner_user.select(&db.user.display_name);
    let ps = rp.group_by(key).select(score.and(view_count.opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt())).fold([0i64; 6], |a, ((s, w), t)| {
        [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(11)) as i64, a[2] + 1, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let rp2 = db.post.with(post_type_id.eq(1).and(creation_date.ge(ts(2022, 10, 1, 0, 0, 0)))).with(owner_user);
    let tp = rp2.group_by(owner_user.select(&db.user.display_name)).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&ps).and(&tp)), |&(d, (a, n))| (Reverse(fkey(a[3] as f64 / a[2] as f64)), Reverse(n), d), 10);
    rows(v.into_iter().map(|(d, (a, n))| row(vec![V::S(d), V::I(n), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), avg(a[5], a[4])])))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount,
//        AVG(COALESCE(P.Score, 0)) AS AvgScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, VoteCount, AvgScore, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserStats)
// SELECT TU.DisplayName, TU.PostCount, TU.VoteCount, TU.AvgScore,
//        CASE WHEN TU.AvgScore > 50 THEN 'High Scorer' WHEN TU.AvgScore BETWEEN 20 AND 50 THEN 'Medium Scorer' ELSE 'Low Scorer' END AS ScoreCategory,
//        COALESCE(PH.Comment, 'No comments') AS PostHistoryComment
// FROM TopUsers TU LEFT JOIN PostHistory PH ON TU.UserId = PH.UserId AND PH.CreationDate = (SELECT MAX(PH2.CreationDate) FROM PostHistory PH2 WHERE PH2.UserId = TU.UserId)
// WHERE TU.Rank <= 10 ORDER BY TU.Rank;
//
// Rank reads only the distinct post count, so the top users are picked first and the posts x votes product is driven for those alone.
fn q98(db: &'static So) -> String {
    let r = ranked(drain(user_distinct_posts(db)), |&(_, n)| Reverse(n), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).collect::<Vec<_>>());
    type T = ((Id<User>, i64), i64);
    let tk: MatSet<Id<User>> = (&tu).map(|((u, _), _): T| u).collect();
    let us = (&tk).group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.score).and(votes_of(db).opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, v)) => [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64],
        None => [a[0] + 1, a[1], a[2]],
    });
    let PostHistory { user, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(user).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<User>, i64), Id<PostHistory>> = db.post_history.select(user.and(hd)).inv().collect();
    let uk = || Same::<T>::new().map(|((u, _), _): T| u);
    let v = drain((&tu).select(Same::<T>::new().and(uk().select(&us)).and(uk().and(uk().select(&md)).select(&at).opt())));
    rows(v.into_iter().map(|(_, ((((u, n), _), a), h))| {
        let m = a[1] as f64 / a[0] as f64;
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[2]), avg(a[1], a[0])];
        f.push(V::S(if m > 50.0 { "High Scorer" } else if m >= 20.0 { "Medium Scorer" } else { "Low Scorer" }));
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No comments")));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, MAX(P.CreationDate) AS LastActive,
//        ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS Rank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation)
// SELECT UA.UserId, UA.DisplayName, UA.Reputation, UA.PostCount, UA.Upvotes, UA.Downvotes, UA.LastActive, CASE WHEN UA.Rank <= 10 THEN 'Top User' ELSE 'Regular User' END AS UserCategory,
//        COALESCE(NULLIF(UT.Name, ''), 'No User Type') AS UserType
// FROM UserActivity UA LEFT JOIN (SELECT * FROM (VALUES (1, 'Active'), (2, 'Inactive'), (3, 'Guest')) AS UT(Id, Name)) UT ON UA.Reputation / 100 > UT.Id
// WHERE UA.PostCount > 0 AND (UA.LastActive >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') ORDER BY UA.Reputation DESC;
//
// Rank partitions by the user, so it is always 1. `/` on integers is a float division in DuckDB. The filter reads only MAX(P.CreationDate), so the users are
// picked first and the posts x votes product is driven for those alone.
fn q1959(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let last = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(creation_date)).fold(i64::MIN, |m, d| m.max(d));
    let cut = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let ua: MatSet<Id<User>> = db.user.with((&last).filt(move |d| d >= cut)).collect();
    let s = (&ua).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ut = rel(vec![(1i64, "Active"), (2, "Inactive"), (3, "Guest")]);
    let uti: HashIdx<i64, (i64, &'static str)> = (&ut).map(|(i, _)| i).inv().select(&ut).collect();
    let typ = (&db.user.reputation).select_where(&uti, |r: i64, i: i64| r as f64 / 100.0 > i as f64).opt();
    let v = drain((&s).and(user_distinct_posts(db)).and(&last).and(typ));
    rows(v.into_iter().map(|(u, (((a, n), d), t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::T(d), V::S("Top User"), V::S(t.map_or("No User Type", |t| t.1))]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, AVG(COALESCE(p.Score, 0)) AS AvgScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate > CURRENT_TIMESTAMP - INTERVAL '1 YEAR')
// SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.Questions, ups.Answers, ups.AvgScore, rp.PostId, rp.Title, rp.CreationDate,
//        CASE WHEN rp.ViewCount IS NULL THEN 'No views' ELSE CONCAT(CAST(rp.ViewCount AS TEXT), ' views') END AS FormattedViewCount
// FROM UserPostStats ups LEFT JOIN RecentPosts rp ON ups.UserId = rp.OwnerUserId AND rp.rn = 1 WHERE ups.TotalPosts > 0 ORDER BY ups.AvgScore DESC LIMIT 10;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so CreationDate is compared as a New York instant. rn ties go to the smaller post id, and ties at the cut to the smaller user id.
fn q3497(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, .. } = &db.post;
    let ups = user_posts(db);
    let since = add_years(utc_to_ny(now_utc()), -1);
    let recent = drain(db.post.with(creation_date.filt(move |d: i64| d > since)).select(owner_user));
    let rp = top_per(recent, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rr = rel(rp.into_iter().map(|(p, u)| (u, p)).collect::<Vec<_>>());
    let ri: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let v = top_n(drain((&ups).filt(|a| a[1] > 0).and((&ri).map(|(_, p)| p).opt())), |&(u, (a, _))| (Reverse(fkey(a[4] as f64 / a[1] as f64)), u), 10);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[1])]);
        match p {
            Some(p) => {
                f.extend(post_fields(db, p, &["id", "title", "created"]));
                f.push(match view_count.get(p) { Some(w) => V::Owned(format!("{w} views")), None => V::S("No views") });
            }
            None => f.extend([V::Null, V::Null, V::Null, V::S("No views")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.OwnerDisplayName, tp.CommentCount, tp.VoteCount,
//        (SELECT STRING_AGG(pt.Name, ', ') FROM PostTypes pt JOIN Posts p ON p.PostTypeId = pt.Id WHERE p.Id = tp.PostId) AS PostType
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.CommentCount DESC;
//
// Rank reads only base columns, so the five questions per owner are picked first (ties by id; the ownerless questions are one partition) and the comment x vote
// product is driven for those alone. The STRING_AGG is over the one type of the post.
fn q9492(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let v = drain((&s).and(ptype_name(db)));
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(t)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionsAsked, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionsAsked, TotalViews, TotalScore, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY TotalScore DESC) AS RankByScore
//     FROM UserActivity)
// SELECT UserId, DisplayName, QuestionsAsked, TotalViews, TotalScore, GoldBadges, SilverBadges, BronzeBadges, RankByScore FROM TopUsers WHERE RankByScore <= 10 ORDER BY RankByScore;
//
// The WHERE on p makes the posts join inner.
fn q5866(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, score, .. } = &db.post;
    let rq = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))));
    let ua = db.user.group_by(Ident::<User>::new()).select(rq.select(view_count.opt().and(score)).and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 5], |a, ((w, s), c)| {
        [a[0] + w.unwrap_or(0), a[1] + s, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
    });
    let rq2 = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))));
    let qc = db.user.group_by(Ident::<User>::new()).select(rq2).fold(0i64, |n, _| n + 1);
    let r = ranked(drain((&ua).and(&qc)), |&(_, (a, _))| Reverse(a[1]), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, q)), k)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(q));
        f.extend(a.map(V::I));
        f.push(V::I(k));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(COUNT(DISTINCT c.Id), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 month'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.PostTypeId, p.Score),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.CommentCount, tp.UpvoteCount, tp.DownvoteCount, (tp.UpvoteCount - tp.DownvoteCount) AS NetScore
// FROM TopPosts tp ORDER BY NetScore DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the five posts per type are picked first (ties by id) and the comment x vote product is driven for those alone.
fn q7668(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and(&cc));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostsCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//        SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedQuestions,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UA.*, RANK() OVER (ORDER BY (TotalUpVotes - TotalDownVotes) DESC) AS UserRanking FROM UserActivity UA)
// SELECT TU.DisplayName, TU.PostsCount, TU.AnswersCount, TU.AcceptedQuestions, TU.TotalUpVotes, TU.TotalDownVotes,
//        CASE WHEN TU.UserRanking <= 10 THEN 'Top Contributor' WHEN TU.UserRanking <= 50 THEN 'Regular Contributor' ELSE 'New Contributor' END AS ContributorLevel
// FROM TopUsers TU WHERE TU.PostsCount > 5 ORDER BY TU.UserRanking;
fn q1418(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, .. } = &db.post;
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((t, x), v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1 && x.is_some()) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let r = rel(ranked(drain((&ua).and(user_distinct_posts(db))), |&(_, (a, _))| Reverse(a[2] - a[3]), false));
    type R = ((Id<User>, ([i64; 4], i64)), i64);
    let v = drain((&r).filt(|((_, (_, n)), _): R| n > 5));
    rows(v.into_iter().map(|(_, ((u, (a, n)), k))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        f.push(V::S(if k <= 10 { "Top Contributor" } else if k <= 50 { "Regular Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(DISTINCT V.UserId) AS VoteCount FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON C.PostId = P.Id LEFT JOIN Votes V ON V.PostId = P.Id
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName),
// PostEngagement AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.ViewCount, RP.Score, RP.OwnerDisplayName, RP.CommentCount, RP.VoteCount,
//        RANK() OVER (ORDER BY (RP.VoteCount + RP.CommentCount + RP.Score) DESC) AS EngagementRank FROM RankedPosts RP)
// SELECT PE.PostId, PE.Title, PE.CreationDate, PE.ViewCount, PE.Score, PE.OwnerDisplayName, PE.CommentCount, PE.VoteCount, PE.EngagementRank FROM PostEngagement PE
// WHERE PE.EngagementRank <= 10 ORDER BY PE.EngagementRank;
fn q5795(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vu = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.user_id).opt()).opt()).buf_fold(|xs| distinct_some(xs.iter().map(|x| x.flatten())));
    let r = ranked(drain((&cc).and(&vu)), |&(p, (c, n))| Reverse(n + c + score.get(p).unwrap()), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((p, (c, n)), k)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(c), V::I(n), V::I(k)]);
        row(f)
    }))
}

// WITH RankedUserPosts AS (SELECT p.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, ROW_NUMBER() OVER (ORDER BY COUNT(*) DESC) AS UserRank FROM Posts p GROUP BY p.OwnerUserId),
// TopUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, r.TotalPosts, r.QuestionCount, r.AnswerCount FROM Users u JOIN RankedUserPosts r ON u.Id = r.OwnerUserId WHERE r.UserRank <= 10),
// PostTagCounts AS (SELECT p.Id AS PostId, t.TagName, COUNT(*) AS TagCount FROM Posts p JOIN UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS t(TagName) ON TRUE
//     GROUP BY p.Id, t.TagName),
// TopTags AS (SELECT TagName, SUM(TagCount) AS TotalTagCount FROM PostTagCounts GROUP BY TagName ORDER BY TotalTagCount DESC LIMIT 5)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.QuestionCount, tu.AnswerCount, tg.TagName, tg.TotalTagCount
// FROM TopUsers tu JOIN TopTags tg ON tg.TagName IN (SELECT UNNEST(string_to_array(tg.TagName, ' '))) ORDER BY tu.Reputation DESC, tg.TotalTagCount DESC;
//
// The ownerless posts are one group of RankedUserPosts and take a UserRank. No tag name has a space, so the ON is always true and the join is a cross join.
fn q27309(db: &'static So) -> String {
    let Post { owner_user, post_type_id, tags_str, .. } = &db.post;
    let rup = db.post.group_by(owner_user.opt()).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let top = top_n(drain(&rup), |&(u, a)| (Reverse(a[0]), u), 10);
    type R = (Option<Id<User>>, [i64; 3]);
    let tu = rel(top).select(Same::<R>::new().map(|(u, _): R| u).flat_map(|u: Option<Id<User>>| u).and(Same::<R>::new().map(|(_, a): R| a)));
    let tu = rel(drain(tu).into_iter().map(|x| x.1).collect::<Vec<_>>());
    let tc = db.post.select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let tt = rel(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 5));
    let tg = (&tt).filt(|(t, _): (Str, i64)| t.split(' ').any(|x| x == t));
    let mut v = Vec::new();
    (&tu).cross(tg).drive(|_, ((u, a), (t, n))| v.push((u, a, t, n)));
    rows(v.into_iter().map(|(u, a, t, n)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > (CAST('2024-10-01' AS DATE) - INTERVAL '30 days') GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId, p.Score),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes,
//        CASE WHEN tp.UpVotes > tp.DownVotes THEN 'Positive' WHEN tp.UpVotes < tp.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM TopPosts tp ORDER BY tp.UpVotes DESC, tp.DownVotes ASC;
//
// Rank reads only base columns, so the five posts per type are picked first (ties by id) and the comment x vote product is driven for those alone.
fn q7757(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(ts(2024, 9, 1, 0, 0, 0))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and(&cc));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount,
//        SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount, AVG(COALESCE(p.Score, 0)) AS AvgScore, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Comments c ON p.Id = c.PostId
//     GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, VoteCount, BadgeCount, AvgScore, CommentCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank
//     FROM UserStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, VoteCount, BadgeCount, AvgScore, CommentCount FROM TopUsers WHERE Rank <= 10;
//
// Rank reads only the distinct post count, so the ten users are picked first (ties by id) and the posts x votes x badges x comments product is driven for
// those alone.
fn q13290(db: &'static So) -> String {
    let top = top_n(drain(user_distinct_posts(db)), |&(u, n)| (Reverse(n), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { post_type_id, score, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(votes_of(db).opt().and(comments_of(db).opt()))).opt().and(badges_of(db).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let (t, s, v) = p.map_or((0, 0, false), |((t, s), (v, _))| (t, s, v.is_some()));
            [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + v as i64, a[4] + b.is_some() as i64, a[5] + s]
        });
    let tc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and(user_distinct_posts(db)).and(&tc));
    rows(v.into_iter().map(|(u, ((a, n), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[5], a[0]), V::I(c)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount, RANK() OVER (ORDER BY COUNT(DISTINCT P.Id) DESC) AS PostRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id
//     LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, CommentCount, UpVotes, DownVotes, BadgeCount FROM UserActivity WHERE PostRank <= 50)
// SELECT TU.DisplayName, TU.PostCount, TU.CommentCount, TU.UpVotes, TU.DownVotes, TU.BadgeCount, ROUND((CAST(TU.UpVotes AS DECIMAL) / NULLIF(TU.PostCount, 0)) * 100, 2) AS UpvotePercentage,
//        ROUND((CAST(TU.DownVotes AS DECIMAL) / NULLIF(TU.PostCount, 0)) * 100, 2) AS DownvotePercentage
// FROM TopUsers TU ORDER BY TU.PostCount DESC, TU.UpVotes DESC;
//
// PostRank reads only the distinct post count, so the top users are picked first and the posts x comments x own-votes x badges product is driven for those alone.
fn q5867(db: &'static So) -> String {
    let r = ranked(drain(user_distinct_posts(db)), |&(_, n)| Reverse(n), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 50).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ov = own_votes(db).select(&db.vote.vote_type_id);
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(ov.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, b)| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]
        });
    let tc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and(user_distinct_posts(db)).and(&tc));
    let pct = |x: i64, n: i64| if n == 0 { V::Null } else { V::F((x as f64 / n as f64 * 100.0 * 100.0).round() / 100.0) };
    rows(v.into_iter().map(|(u, ((a, n), c))| row(vec![user_col(db, u, "name"), V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2]), pct(a[0], n), pct(a[1], n)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId
//     GROUP BY u.Id, u.Reputation, u.DisplayName),
// ClosedQuestions AS (SELECT ph.PostId, ph.CreationDate, ph.Comment, ph.UserId AS CloserId, ph.Text FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10)
// SELECT u.DisplayName, u.Reputation, u.TotalBounty, COUNT(DISTINCT rp.PostId) AS TotalQuestions, AVG(rp.Score) AS AvgScore,
//        SUM(CASE WHEN cq.PostId IS NOT NULL THEN 1 ELSE 0 END) AS ClosedQuestionsCount
// FROM UserReputation u JOIN RankedPosts rp ON u.UserId = rp.OwnerUserId LEFT JOIN ClosedQuestions cq ON rp.PostId = cq.PostId WHERE u.Reputation > 1000
// GROUP BY u.DisplayName, u.Reputation, u.TotalBounty HAVING COUNT(DISTINCT rp.PostId) > 5 ORDER BY AvgScore DESC, TotalQuestions DESC LIMIT 10;
//
// The GROUP BY is on (DisplayName, Reputation, TotalBounty), not the user. AvgScore runs over the question x close rows.
fn q1452(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let tb = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let Post { post_type_id, score, .. } = &db.post;
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let cq = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let key = || (&db.user.display_name).and(rep).and(&tb);
    let g = db.user.with(rep.gt(1000)).group_by(key()).select(qs().select(score.and(cq.opt()))).fold([0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c.is_some() as i64]);
    let nq = db.user.with(rep.gt(1000)).group_by(key()).select(qs()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&g).and((&nq).filt(|n| n > 5))), |&(k, (a, n))| (Reverse(fkey(a[1] as f64 / a[0] as f64)), Reverse(n), k), 10);
    rows(v.into_iter().map(|(((d, r), b), (a, n))| row(vec![V::S(d), V::I(r), V::I(b), V::I(n), avg(a[1], a[0]), V::I(a[2])])))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS EditCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// RankedPosts AS (SELECT ps.*, RANK() OVER (ORDER BY ps.Score DESC, ps.ViewCount DESC) AS PopularityRank FROM PostStatistics ps)
// SELECT r.PostId, r.Title, r.CreationDate, r.ViewCount, r.Score, r.UpVotes, r.DownVotes, r.CommentCount, r.EditCount, r.PopularityRank, u.DisplayName AS OwnerDisplayName,
//        u.Reputation AS OwnerReputation
// FROM RankedPosts r JOIN Users u ON r.PostId = u.Id WHERE r.PopularityRank <= 10 ORDER BY r.PopularityRank;
//
// PopularityRank reads only base columns, so the ranked posts are picked first and the votes x comments x history product is driven for those alone.
// `r.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q6434(db: &'static So) -> String {
    let Post { creation_date, score, view_count, origid, .. } = &db.post;
    let r = ranked(drain(db.post.with(creation_date.ge(ts(2024, 9, 1, 0, 0, 0))).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, false);
    let rk = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((p, _), k)| (p, k)).collect::<Vec<_>>());
    let tp: MatSet<Id<Post>> = (&rk).map(|(p, _)| p).collect();
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt())).fold([0i64; 2], |a, ((t, _), _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let by_raw: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type K = (Id<Post>, i64);
    let pk = || Same::<K>::new().map(|(p, _): K| p);
    let v = drain((&rk).select(Same::<K>::new().and(pk().select(&ud)).and(pk().select(&cc)).and(pk().select(&hc)).and(pk().select(origid).select(&by_raw))));
    rows(v.into_iter().map(|(_, (((((p, k), a), c), h), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(h), V::I(k)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RECURSIVE UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// PopularTags AS (SELECT T.TagName, COUNT(P.Id) AS TagPostCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%'
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY T.TagName),
// TopTags AS (SELECT TagName, TagPostCount, RANK() OVER (ORDER BY TagPostCount DESC) AS TagRank FROM PopularTags)
// SELECT U.UserId, U.DisplayName, U.PostCount, U.UpVoteCount, U.DownVoteCount, COALESCE(TT.TagName, 'No Tags') AS PopularTag, TT.TagPostCount
// FROM UserActivity U LEFT JOIN TopTags TT ON U.PostCount > 0 AND TT.TagRank = 1 WHERE U.PostCount > 0 ORDER BY U.UpVoteCount DESC, U.DisplayName;
//
// WITH RECURSIVE, but no CTE refers to itself. The ON names TT only through TagRank = 1, so the rank-1 tags are crossed with the users.
fn q33350(db: &'static So) -> String {
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let lt = tag_mentions(db);
    type M = (Id<Post>, Id<Tag>);
    let recent = Same::<M>::new().map(|(p, _): M| p).select(Ident::<Post>::new().with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let pt = (&lt).with(recent).group_by(Same::<M>::new().map(|(_, t): M| t)).select(Same::<M>::new()).fold(0i64, |n, _| n + 1);
    let r = ranked(drain(&pt), |&(_, n)| Reverse(n), false);
    let t1 = left_all(r.into_iter().take_while(|x| x.1 == 1).map(|x| x.0).collect());
    let us = rel(drain((&ua).and(user_distinct_posts(db))));
    let mut v = Vec::new();
    (&us).cross(&t1).drive(|_, ((u, (a, n)), t)| v.push((u, a, n, t)));
    rows(v.into_iter().map(|(u, a, n, t)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(match t {
            Some((t, c)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(c)],
            None => [V::S("No Tags"), V::Null],
        });
        row(f)
    }))
}

// Rewritten (rewrites/1156.sql): the float AVG of epoch seconds became the exact-integer mean.
// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.Id) / 1e6 AS AvgResponseTime
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation >= 100 GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankedPosts
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > CURRENT_DATE - INTERVAL '90 days')
// SELECT ua.DisplayName, ua.PostCount, ua.Upvotes, ua.Downvotes, ua.AvgResponseTime, tp.Title, tp.CreationDate AS PostCreationDate, tp.Score, tp.OwnerName
// FROM UserActivity ua LEFT JOIN TopPosts tp ON ua.DisplayName = tp.OwnerName WHERE (ua.Upvotes - ua.Downvotes) > 10 AND (tp.RankedPosts <= 5 OR tp.RankedPosts IS NULL)
// ORDER BY ua.Upvotes DESC, ua.AvgResponseTime ASC;
//
// RankedPosts ties on Score go to the smaller post id.
fn q1156(db: &'static So) -> String {
    let Post { last_activity_date, creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let ua = db
        .user
        .with((&db.user.reputation).ge(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(last_activity_date.and(creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold((0i64, 0i64, 0i64, 0i128), |a, p| match p {
            Some(((l, c), t)) => (a.0 + 1, a.1 + (t == Some(2)) as i64, a.2 + (t == Some(3)) as i64, a.3 + (l - c) as i128),
            None => a,
        });
    let v = drain(db.post.with(creation_date.gt(add_days(current_date(), -90))).with(owner_user).select(post_type_id));
    let r = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), false), |&(_, t)| t);
    type R = ((Id<Post>, i64), i64);
    let tr = rel(r);
    let by_name: HashIdx<Str, R> = (&tr).map(|((p, _), _): R| p).select(owner_user).select(&db.user.display_name).inv().select(&tr).collect();
    let v = drain((&ua).filt(|a| a.1 - a.2 > 10).and((&db.user.display_name).select(&by_name).opt()).and(user_distinct_posts(db)));
    let v = drain(rel(v).filt(|(_, ((_, t), _)): (Id<User>, (((i64, i64, i64, i128), Option<R>), i64))| t.map_or(true, |(_, k)| k <= 5)));
    rows(v.into_iter().map(|(_, (u, ((a, t), n)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a.1), V::I(a.2), if a.0 == 0 { V::Null } else { V::F(a.3 as f64 / a.0 as f64 / 1e6) }];
        f.extend(match t {
            Some(((p, _), _)) => post_fields(db, p, &["title", "created", "score", "owner"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COALESCE(SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 END), 0) AS AcceptedAnswers,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.CommentCount, ps.UpVotes, ps.DownVotes, ps.AcceptedAnswers FROM PostStatistics ps WHERE ps.RN <= 5)
// SELECT u.DisplayName, SUM(tp.UpVotes) AS TotalUpVotes, AVG(tp.CommentCount) AS AverageComments, SUM(tp.AcceptedAnswers) AS TotalAcceptedAnswers
// FROM Users u JOIN TopPosts tp ON u.Id = tp.PostId GROUP BY u.DisplayName ORDER BY TotalUpVotes DESC LIMIT 10;
//
// RN reads only base columns, so the five newest posts per owner are picked first (ties by id; the ownerless posts are one partition) and the comment x vote
// product is driven for those alone. `u.Id = tp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q7701(db: &'static So) -> String {
    let Post { creation_date, owner_user, accepted_answer_id, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(accepted_answer_id.opt())).fold([0i64; 3], |a, ((c, t), x)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + x.is_some() as i64]
    });
    let by_raw: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let g = (&tp).group_by(origid.select(&by_raw).select(&db.user.display_name)).select(&ps).fold([0i64; 4], |a, p| [a[0] + p[1], a[1] + p[0], a[2] + 1, a[3] + p[2]]);
    let v = top_n(drain(&g), |&(d, a)| (Reverse(a[0]), d), 10);
    rows(v.into_iter().map(|(d, a)| row(vec![V::S(d), V::I(a[0]), avg(a[1], a[2]), V::I(a[3])])))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, AVG(p.Score) AS AvgScore, COUNT(DISTINCT COALESCE(c.Id, -1)) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, PostCount, Questions, Answers, AvgScore, CommentCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS UserRank
//     FROM UserPostStats WHERE PostCount > 0),
// MaxScores AS (SELECT UserId, MAX(AvgScore) AS MaxAvgScore FROM RankedUsers GROUP BY UserId)
// SELECT ru.DisplayName, ru.PostCount, ru.Questions, ru.Answers, ru.AvgScore, ru.CommentCount, ms.MaxAvgScore, CASE WHEN ru.AvgScore = ms.MaxAvgScore THEN 'Top Scorer' ELSE NULL END AS Status
// FROM RankedUsers ru LEFT JOIN MaxScores ms ON ru.UserId = ms.UserId WHERE ru.UserRank <= 10 OR ms.MaxAvgScore IS NOT NULL ORDER BY ru.UserRank, ru.AvgScore DESC;
//
// MaxScores has one row per user, the user's own AvgScore, which is never NULL once PostCount > 0: every row passes and is a 'Top Scorer'.
// COUNT(DISTINCT COALESCE(c.Id, -1)) counts the comments, plus one for the -1 of any post x comment row without a comment.
fn q776(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score).and(comments_of(db).opt()))).fold([0i64; 6], |a, ((t, s), c)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + c.is_some() as i64, a[5] | c.is_none() as i64]
    });
    let v = drain(&ups);
    rows(v.into_iter().map(|(u, a)| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::I(a[4] + a[5]), avg(a[3], a[0]), V::S("Top Scorer")])
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.QuestionCount, tu.AnswerCount, tu.TotalUpVotes, tu.TotalDownVotes,
//        CASE WHEN tu.ReputationRank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorType
// FROM TopUsers tu WHERE tu.TotalPosts > 0 AND EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = tu.UserId AND p.LastActivityDate > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'))
// ORDER BY tu.Reputation DESC, tu.TotalPosts DESC LIMIT 20;
//
// The filter and the sort read only base columns and the distinct post count, so the twenty users are picked first and the posts x votes product is driven for
// those alone. ReputationRank is over every user.
fn q4548(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let rr = ranked(drain(rep), |&(_, r)| Reverse(r), false);
    let rk = rel(rr.into_iter().map(|((u, _), k)| (u, k)).collect::<Vec<_>>());
    let rki: HashIdx<Id<User>, (Id<User>, i64)> = (&rk).map(|(u, _)| u).inv().select(&rk).collect();
    let active = posts_of(db).select(Ident::<Post>::new().with((&db.post.last_activity_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let el = drain(db.user.with(active).select(user_distinct_posts(db)));
    let top = top_n(el, |&(u, n)| (Reverse(rep.get(u).unwrap()), Reverse(n), u), 20);
    let tr = rel(top);
    type T = (Id<User>, i64);
    let tk: MatSet<Id<User>> = (&tr).map(|(u, _): T| u).collect();
    let us = (&tk).group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt()))).fold([0i64; 4], |a, (t, v)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]
    });
    let uk = || Same::<T>::new().map(|(u, _): T| u);
    let v = drain((&tr).select(Same::<T>::new().and(uk().select(&us)).and(uk().select(&rki))));
    rows(v.into_iter().map(|(_, (((u, n), a), (_, k)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::S(if k <= 10 { "Top Contributor" } else { "Regular Contributor" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COALESCE(SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, Upvotes, Downvotes, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.Upvotes, TU.Downvotes, TU.BadgeCount, RANK() OVER (ORDER BY TU.Upvotes DESC) AS UpvoteRank,
//        RANK() OVER (ORDER BY TU.Downvotes DESC) AS DownvoteRank, RANK() OVER (ORDER BY TU.BadgeCount DESC) AS BadgeRank
// FROM TopUsers TU WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC, TU.PostCount DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the posts x votes x badges product is driven for those alone.
fn q5694(db: &'static So) -> String {
    let rr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(rr.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, b)| {
            let t = p.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]
        });
    let v = ranked(drain((&s).and(user_distinct_posts(db))), |&(_, (a, _))| Reverse(a[0]), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[1]), false);
    let v = ranked(v, |&(((_, (a, _)), _), _)| Reverse(a[2]), false);
    rows(v.into_iter().map(|((((u, (a, n)), ur), dr), br)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(ur), V::I(dr), V::I(br)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVotes FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT t.PostId, t.Title, t.Score, t.ViewCount, t.CommentCount, t.UpVotes, u.DisplayName AS Author, u.Reputation,
//        (SELECT STRING_AGG(pt.Name, ', ') FROM PostTypes pt WHERE pt.Id = (SELECT p.PostTypeId FROM Posts p WHERE p.Id = t.PostId)) AS PostType
// FROM TopPosts t JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts p WHERE p.Id = t.PostId);
//
// Rank ties go to the smaller post id. The STRING_AGG is over the one type of the post.
fn q8372(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let uv = (&tp).group_by(Ident::<Post>::new()).select(up.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = drain((&cc).and(&uv).and(owner_user).and(ptype_name(db)));
    rows(v.into_iter().map(|(p, (((c, n), u), t))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::S(t));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("24710", q24710),
    ("22552", q22552),
    ("24628", q24628),
    ("21706", q21706),
    ("30678", q30678),
    ("24351", q24351),
    ("24593", q24593),
    ("21972", q21972),
    ("24811", q24811),
    ("21017", q21017),
    ("23765", q23765),
    ("21213", q21213),
    ("24730", q24730),
    ("33388", q33388),
    ("24968", q24968),
    ("21224", q21224),
    ("22696", q22696),
    ("22452", q22452),
    ("23186", q23186),
    ("33065", q33065),
    ("30741", q30741),
    ("34350", q34350),
    ("2886", q2886),
    ("34440", q34440),
    ("20864", q20864),
    ("22958", q22958),
    ("24181", q24181),
    ("24136", q24136),
    ("22487", q22487),
    ("30969", q30969),
    ("24854", q24854),
    ("32097", q32097),
    ("34109", q34109),
    ("22382", q22382),
    ("20870", q20870),
    ("20752", q20752),
    ("23268", q23268),
    ("30663", q30663),
    ("22407", q22407),
    ("24382", q24382),
    ("20436", q20436),
    ("21765", q21765),
    ("21749", q21749),
    ("20887", q20887),
    ("22271", q22271),
    ("10933", q10933),
    ("14380", q14380),
    ("13728", q13728),
    ("28155", q28155),
    ("9635", q9635),
    ("5881", q5881),
    ("9090", q9090),
    ("14273", q14273),
    ("14246", q14246),
    ("26871", q26871),
    ("9114", q9114),
    ("7679", q7679),
    ("31378", q31378),
    ("7896", q7896),
    ("27383", q27383),
    ("5211", q5211),
    ("3852", q3852),
    ("5395", q5395),
    ("3948", q3948),
    ("6680", q6680),
    ("1557", q1557),
    ("27662", q27662),
    ("7355", q7355),
    ("5243", q5243),
    ("26382", q26382),
    ("28373", q28373),
    ("9455", q9455),
    ("6891", q6891),
    ("6948", q6948),
    ("9977", q9977),
    ("637", q637),
    ("98", q98),
    ("1959", q1959),
    ("3497", q3497),
    ("9492", q9492),
    ("5866", q5866),
    ("7668", q7668),
    ("1418", q1418),
    ("5795", q5795),
    ("27309", q27309),
    ("7757", q7757),
    ("13290", q13290),
    ("5867", q5867),
    ("1452", q1452),
    ("6434", q6434),
    ("33350", q33350),
    ("1156", q1156),
    ("7701", q7701),
    ("776", q776),
    ("4548", q4548),
    ("5694", q5694),
    ("8372", q8372),
];
