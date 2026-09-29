use harness::prelude::*;
use std::cmp::Reverse;

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.rn <= 5),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.UpVotes, fp.DownVotes, ub.BadgeCount,
//        CASE WHEN ub.HighestBadgeClass IS NULL THEN 'No Badge' ELSE CASE WHEN ub.HighestBadgeClass = 1 THEN 'Gold' WHEN ub.HighestBadgeClass = 2 THEN 'Silver'
//        WHEN ub.HighestBadgeClass = 3 THEN 'Bronze' ELSE 'Unknown' END END AS HighestBadge,
//        CASE WHEN fp.ViewCount > 100 THEN 'Popular' WHEN fp.ViewCount BETWEEN 50 AND 100 THEN 'Moderate' ELSE 'Less Viewed' END AS ViewPopularity
// FROM FilteredPosts fp LEFT JOIN Posts p ON p.Id = fp.PostId LEFT JOIN Users u ON u.Id = p.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// WHERE fp.UpVotes > fp.DownVotes OR fp.Score > 10 ORDER BY fp.CreationDate DESC;
//
// rn reads only base columns, so each owner's five newest questions are picked first and the votes are joined for those alone.
fn q1802(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let v = drain((&vc).and(score).filt(|(a, s): ([i64; 2], i64)| a[0] > a[1] || s > 10).and(owner_user.select(&ub).opt()));
    rows(v.into_iter().map(|(p, ((a, _), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), b.map_or(V::Null, |b| V::I(b.0))]);
        f.push(V::S(match b {
            Some((n, m)) if n > 0 => match m {
                1 => "Gold",
                2 => "Silver",
                3 => "Bronze",
                _ => "Unknown",
            },
            _ => "No Badge",
        }));
        let w = view_count.get(p);
        f.push(V::S(match w {
            Some(w) if w > 100 => "Popular",
            Some(w) if w >= 50 => "Moderate",
            _ => "Less Viewed",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.LastActivityDate, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC) AS OwnerPostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.LastActivityDate, p.OwnerUserId),
// UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostSummaries AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.LastActivityDate, rp.CommentCount, rp.AnswerCount, um.DisplayName AS OwnerDisplayName, um.Reputation AS OwnerReputation
//     FROM RankedPosts rp JOIN Users um ON rp.OwnerUserId = um.Id WHERE rp.OwnerPostRank <= 3)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.LastActivityDate, ps.CommentCount, ps.AnswerCount, ps.OwnerDisplayName, ps.OwnerReputation, COALESCE(pm.VoteCount, 0) AS VoteCount
// FROM PostSummaries ps LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) pm ON ps.PostId = pm.PostId
// ORDER BY ps.OwnerReputation DESC, ps.CreationDate DESC LIMIT 100;
//
// UserMetrics is never referenced, so it is not computed. The two COUNT(DISTINCT) are one fold each over one row per post.
fn q28589(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, last_activity_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(last_activity_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(&ac).and(&vc));
    let rep = |p: Id<Post>| db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
    let v = top_n(v, |&(p, _)| (Reverse(rep(p)), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((c, a), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity"]);
        f.extend([V::I(c), V::I(a)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedPostScores AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS ScoreRank
//     FROM Posts P WHERE P.CreationDate > CURRENT_DATE - INTERVAL '1 year'),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// UserActivity AS (SELECT C.UserId, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN C.CreationDate > CURRENT_DATE - INTERVAL '6 months' THEN 1 ELSE 0 END) AS RecentComments FROM Comments C GROUP BY C.UserId),
// PostDetails AS (SELECT PS.PostId, PS.Title, PS.ScoreRank, U.Reputation, COALESCE(UA.CommentCount, 0) AS CommentCount, COALESCE(UA.RecentComments, 0) AS RecentComments,
//        CASE WHEN PS.ScoreRank = 1 THEN 'Top' ELSE 'Others' END AS PostCategory
//     FROM RankedPostScores PS JOIN Posts P ON P.Id = PS.PostId JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN UserActivity UA ON U.Id = UA.UserId)
// SELECT PD.Title, PD.ScoreRank, PD.Reputation AS UserReputation, PD.CommentCount, PD.RecentComments, PD.PostCategory,
//        CASE WHEN PD.Reputation > 1000 THEN 'High Reputation User' WHEN PD.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation User' ELSE 'Low Reputation User' END AS ReputationCategory
// FROM PostDetails PD WHERE PD.ScoreRank <= 10 ORDER BY PD.ScoreRank;
//
// UserReputation is never referenced, so it is not computed.
fn q34720(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let today = current_date();
    let v = drain(db.post.with(creation_date.gt(add_years(today, -1))).select(post_type_id));
    let r = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |&(_, t)| t);
    let rk = rel(r.into_iter().filter(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let recent = add_months(today, -6);
    let ua = db.user.group_by(Ident::<User>::new()).select(comments_by(db).select(&db.comment.creation_date).opt()).fold([0i64; 2], |a, d| match d {
        Some(d) => [a[0] + 1, a[1] + (d > recent) as i64],
        None => a,
    });
    let v = drain((&by_post).and(owner_user.select(Ident::<User>::new().and(&ua))));
    rows(v.into_iter().map(|(p, ((_, r), (u, a)))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(r), V::I(rep), V::I(a[0]), V::I(a[1]), V::S(if r == 1 { "Top" } else { "Others" })]);
        f.push(V::S(if rep > 1000 { "High Reputation User" } else if rep >= 500 { "Medium Reputation User" } else { "Low Reputation User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostVoteCounts AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// ClosedPosts AS (SELECT p.Id, ph.CreationDate AS ClosedDate, ph.UserDisplayName, ph.Comment FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes, COALESCE(b.BadgeCount, 0) AS UserBadgeCount,
//        COALESCE(b.GoldBadges, 0) AS UserGoldBadges, COALESCE(b.SilverBadges, 0) AS UserSilverBadges, COALESCE(b.BronzeBadges, 0) AS UserBronzeBadges,
//        cp.ClosedDate, cp.UserDisplayName AS ClosedBy, cp.Comment AS CloseReason
// FROM RankedPosts rp LEFT JOIN PostVoteCounts v ON rp.PostId = v.PostId LEFT JOIN UserBadges b ON rp.OwnerUserId = b.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.Id
// WHERE rp.rn = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 100;
fn q1856(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&vc).and(owner_user.select(&ub).opt()).and(closes.opt()));
    let v = top_n(v, |&(p, (_, h))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, h)
    }, 100);
    rows(v.into_iter().map(|(p, ((a, b), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.extend(match h {
            Some(h) => [V::T(db.post_history.creation_date.get(h).unwrap()), ostr(db.post_history.user_display_name.get(h)), ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC, P.CreationDate DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerReputation FROM RankedPosts RP WHERE RP.PostRank = 1),
// PostVotes AS (SELECT V.PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes V GROUP BY V.PostId),
// PostComments AS (SELECT C.PostId, COUNT(*) AS CommentCount FROM Comments C GROUP BY C.PostId),
// FinalResult AS (SELECT TP.PostId, TP.Title, TP.CreationDate, TP.Score, TP.ViewCount, TP.OwnerReputation, COALESCE(PV.UpVotes, 0) AS UpVotes, COALESCE(PV.DownVotes, 0) AS DownVotes,
//        COALESCE(PC.CommentCount, 0) AS CommentCount FROM TopPosts TP LEFT JOIN PostVotes PV ON TP.PostId = PV.PostId LEFT JOIN PostComments PC ON TP.PostId = PC.PostId)
// SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerReputation, UpVotes, DownVotes, CommentCount,
//        CASE WHEN Score > 100 THEN 'Highly Engaging' WHEN Score BETWEEN 50 AND 100 THEN 'Moderately Engaging' ELSE 'Less Engaging' END AS EngagementLevel
// FROM FinalResult ORDER BY Score DESC, ViewCount DESC LIMIT 50;
fn q33603(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&vc).and(&cc)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, (a, c))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.push(V::S(if s > 100 { "Highly Engaging" } else if s >= 50 { "Moderately Engaging" } else { "Less Engaging" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN 1 ELSE 0 END) AS QuestionAnswerCount,
//        SUM(CASE WHEN p.ViewCount > 1000 THEN 1 ELSE 0 END) AS HighViewCountPosts, SUM(CASE WHEN p.AnswerCount > 0 THEN 1 ELSE 0 END) AS AnsweredPostsCount, u.Reputation,
//        RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// UserPosts AS (SELECT u.Id AS UserId, SUM(COALESCE(bp.PostCount, 0)) AS TotalPosts, SUM(COALESCE(ba.BadgeCount, 0)) AS TotalBadges, SUM(COALESCE(ba.GoldBadges, 0)) AS TotalGoldBadges,
//        SUM(COALESCE(ba.SilverBadges, 0)) AS TotalSilverBadges, SUM(COALESCE(ba.BronzeBadges, 0)) AS TotalBronzeBadges
//     FROM Users u LEFT JOIN UserActivity bp ON u.Id = bp.UserId LEFT JOIN UserBadges ba ON u.Id = ba.UserId GROUP BY u.Id)
// SELECT up.UserId, u.DisplayName, up.TotalPosts, up.TotalBadges, up.TotalGoldBadges, up.TotalSilverBadges, up.TotalBronzeBadges, ua.PostCount, ua.QuestionAnswerCount,
//        ua.HighViewCountPosts, ua.AnsweredPostsCount, ua.Reputation, ua.ReputationRank
// FROM UserPosts up JOIN Users u ON up.UserId = u.Id JOIN UserActivity ua ON up.UserId = ua.UserId WHERE up.TotalPosts > 10 ORDER BY ua.Reputation DESC, up.TotalPosts DESC;
//
// UserActivity and UserBadges have one row per user, so UserPosts' sums are over one joined row each.
fn q8566(db: &'static So) -> String {
    let Post { post_type_id, view_count, answer_count, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(answer_count.opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((t, w), n)) => [a[0] + 1, a[1] + matches!(t, 1 | 2) as i64, a[2] + (w.unwrap_or(0) > 1000) as i64, a[3] + (n.unwrap_or(0) > 0) as i64],
            None => a,
        });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = ranked(drain((&ua).and((&ub).opt())), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = drain(rel(v).filt(|((_, (a, _)), _)| a[0] > 10));
    rows(v.into_iter().map(|(_, ((u, (a, b)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(a[0]));
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.extend(a.map(V::I));
        f.extend([user_col(db, u, "rep"), V::I(r)]);
        row(f)
    }))
}

// WITH RecursivePostHierarchy AS (SELECT p.Id AS QuestionId, p.Title AS QuestionTitle, p.OwnerUserId, a.Id AS AnswerId, a.Score AS AnswerScore,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY a.Score DESC) AS Rank FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation),
// PostInsights AS (SELECT ph.QuestionId, ph.QuestionTitle, ph.OwnerUserId, COUNT(ph.AnswerId) AS TotalAnswers, AVG(ph.AnswerScore) AS AverageAnswerScore, ur.Reputation AS UserReputation,
//        ur.UpvoteCount, ur.DownvoteCount FROM RecursivePostHierarchy ph JOIN UserReputation ur ON ph.OwnerUserId = ur.UserId
//     GROUP BY ph.QuestionId, ph.QuestionTitle, ph.OwnerUserId, ur.Reputation, ur.UpvoteCount, ur.DownvoteCount)
// SELECT pi.QuestionId, pi.QuestionTitle, pi.TotalAnswers, pi.AverageAnswerScore,
//        CASE WHEN pi.UserReputation >= 1000 THEN 'Top Contributor' WHEN pi.UserReputation BETWEEN 500 AND 999 THEN 'Active Contributor' ELSE 'New Contributor' END AS ContributorCategory,
//        pi.UpvoteCount, pi.DownvoteCount,
//        CASE WHEN pi.TotalAnswers = 0 THEN 'No Answers' WHEN pi.AverageAnswerScore IS NULL THEN 'Answers have no score' ELSE 'Answers available' END AS AnswerStatus
// FROM PostInsights pi WHERE pi.TotalAnswers > 0 ORDER BY pi.AverageAnswerScore DESC, pi.TotalAnswers DESC LIMIT 50;
//
// The window's Rank is never read.
fn q32898(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let pi = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).select(score).opt())
        .fold([0i64; 2], |a, s| match s {
            Some(s) => [a[0] + 1, a[1] + s],
            None => a,
        });
    let ur = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&pi).filt(|a| a[0] > 0).and(owner_user.select(Ident::<User>::new().and(&ur))));
    let v = top_n(v, |&(p, (a, _))| (Reverse(fkey(a[1] as f64 / a[0] as f64)), Reverse(a[0]), p), 50);
    rows(v.into_iter().map(|(p, (a, (u, c)))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), avg(a[1], a[0])]);
        f.push(V::S(if rep >= 1000 { "Top Contributor" } else if rep >= 500 { "Active Contributor" } else { "New Contributor" }));
        f.extend([V::I(c[0]), V::I(c[1]), V::S("Answers available")]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS OwnerPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, OwnerDisplayName FROM RankedPosts WHERE OwnerPostRank <= 3),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, SUM(c.Score) AS TotalCommentScore FROM Comments c GROUP BY c.PostId),
// FinalPostMetrics AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, COALESCE(pc.CommentCount, 0) AS CommentCount,
//        COALESCE(pc.TotalCommentScore, 0) AS TotalCommentScore, tp.OwnerDisplayName FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId)
// SELECT fpm.PostId, fpm.Title, fpm.CreationDate, fpm.Score, fpm.ViewCount, fpm.AnswerCount, fpm.CommentCount, fpm.TotalCommentScore, fpm.OwnerDisplayName,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM FinalPostMetrics fpm LEFT JOIN Votes v ON fpm.PostId = v.PostId
// GROUP BY fpm.PostId, fpm.Title, fpm.CreationDate, fpm.Score, fpm.ViewCount, fpm.AnswerCount, fpm.CommentCount, fpm.TotalCommentScore, fpm.OwnerDisplayName
// ORDER BY fpm.Score DESC, fpm.ViewCount DESC LIMIT 10;
fn q5235(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let vc = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain((&pc).and(&vc)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(c[0]), V::I(c[1])]);
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(sub_query.CommentCount, 0)) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) sub_query ON p.Id = sub_query.PostId GROUP BY u.Id, u.DisplayName),
// UserRanked AS (SELECT ua.*, RANK() OVER (ORDER BY ua.Upvotes DESC, ua.TotalViews DESC, ua.PostCount DESC) AS UserRank FROM UserActivity ua),
// ClosedPosts AS (SELECT p.Id, ph.PostHistoryTypeId, ph.CreationDate, ph.UserDisplayName, ph.Comment, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS LatestHistory
//     FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11))
// SELECT ur.UserId, ur.DisplayName, ur.PostCount, ur.Upvotes, ur.Downvotes, ur.TotalViews, ur.TotalComments, ur.UserRank, COUNT(DISTINCT cp.Id) AS ClosedPostCount,
//        AVG(CASE WHEN cp.LatestHistory = 1 THEN 1 ELSE 0 END) AS ReopenedPosts
// FROM UserRanked ur LEFT JOIN ClosedPosts cp ON ur.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = cp.Id)
// GROUP BY ur.UserId, ur.DisplayName, ur.PostCount, ur.Upvotes, ur.Downvotes, ur.TotalViews, ur.TotalComments, ur.UserRank
// HAVING ur.UserRank <= 10 OR (COUNT(DISTINCT cp.Id) > 0 AND ur.TotalViews > 100) ORDER BY ur.UserRank;
//
// The correlated subquery is the post's owner, so ClosedPosts joins each user through the posts they own.
fn q20426(db: &'static So) -> String {
    let Post { view_count, .. } = &db.post;
    let ccount = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&ccount).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((((_, w), t), c)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + w.unwrap_or(0), a[3] + c.unwrap_or(0)],
            None => a,
        });
    let pc = user_distinct_posts(db);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cl = drain(db.post_history.with(post_history_type_id.is_in([10, 11])).select(post));
    let latest = top_per(cl, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let latest: MatSet<Id<PostHistory>> = rel(latest.into_iter().map(|x| x.0).collect()).map(|h| h).collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11])));
    let cp = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(closes.select(Ident::<PostHistory>::new().with(&latest).opt()))).opt())
        .buf_fold(|rows| {
            let n = rows.len() as i64;
            let first = rows.iter().filter(|r| matches!(r, Some((_, Some(_))))).count() as i64;
            [n, first, distinct_some(rows.iter().map(|r| r.map(|(p, _)| p)))]
        });
    let v = ranked(drain((&ua).and(&pc)), |&(_, (a, n))| (Reverse(a[0]), Reverse(a[2]), Reverse(n)), false);
    let v = rel(v);
    let v = drain((&v).select(Same::<((Id<User>, ([i64; 4], i64)), i64)>::new().and(Same::<((Id<User>, ([i64; 4], i64)), i64)>::new().map(|((u, _), _)| u).select((&cp).opt()))));
    let v = drain(rel(v.into_iter().map(|x| x.1).collect()).filt(|(((_, (a, _)), r), c): (((Id<User>, ([i64; 4], i64)), i64), Option<[i64; 3]>)| {
        r <= 10 || (c.map_or(0, |c| c[2]) > 0 && a[2] > 100)
    }));
    rows(v.into_iter().map(|(_, (((u, (a, n)), r), c))| {
        let (rows_n, first, distinct) = match c {
            Some(c) => (c[0], c[1], c[2]),
            None => (1, 0, 0),
        };
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(r), V::I(distinct), V::F(first as f64 / rows_n as f64)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(NULLIF(p.OwnerDisplayName, ''), 'Anonymous') AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > (SELECT AVG(Score) FROM Posts WHERE PostTypeId = 1)),
// PostVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostAnalytics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, pv.UpVotes, pv.DownVotes, (pv.UpVotes - pv.DownVotes) AS NetVotes,
//        CASE WHEN pv.UpVotes >= pv.DownVotes THEN 'Positive' ELSE 'Negative' END AS VoteSentiment FROM RankedPosts rp LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId)
// SELECT pa.Title, pa.OwnerDisplayName, pa.CreationDate, pa.Score, pa.ViewCount, pa.UpVotes, pa.DownVotes, pa.NetVotes, pa.VoteSentiment, COUNT(DISTINCT c.Id) AS CommentCount,
//        MAX(b.Class) AS HighestBadgeClass, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostsCount
// FROM PostAnalytics pa LEFT JOIN Comments c ON pa.PostId = c.PostId LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pa.PostId LIMIT 1)
// LEFT JOIN PostLinks pl ON pa.PostId = pl.PostId WHERE pa.NetVotes > 0
// GROUP BY pa.Title, pa.OwnerDisplayName, pa.CreationDate, pa.Score, pa.ViewCount, pa.UpVotes, pa.DownVotes, pa.NetVotes, pa.VoteSentiment HAVING COUNT(DISTINCT c.Id) > 5
// ORDER BY pa.Score DESC, pa.ViewCount DESC;
//
// The correlated LIMIT 1 looks a post up by its id, so it is the post's owner. The GROUP BY names the columns, not the post, so posts are grouped by that tuple.
fn q841(db: &'static So) -> String {
    let Post { post_type_id, score, title, owner_display_name, creation_date, view_count, owner_user, .. } = &db.post;
    let (sum, n) = db.post.with(post_type_id.eq(1)).select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let mean = sum as f64 / n as f64;
    let pv = db
        .post
        .with(post_type_id.eq(1).and(score.filt(move |s: i64| s as f64 > mean)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id))
        .fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    type K = (Option<Str>, Str, i64, i64, Option<i64>, i64, i64);
    let keyed: Vec<(K, Id<Post>)> = drain((&pv).filt(|a| a[0] - a[1] > 0))
        .into_iter()
        .map(|(p, a)| {
            let name = owner_display_name.get(p).filter(|s| !s.is_empty()).unwrap_or("Anonymous");
            ((title.get(p), name, creation_date.get(p).unwrap(), score.get(p).unwrap(), view_count.get(p), a[0], a[1]), p)
        })
        .collect();
    let kr = rel(keyed);
    let g = (&kr)
        .group_by(Same::<(K, Id<Post>)>::new().map(|(k, _): (K, Id<Post>)| k))
        .select(
            Same::<(K, Id<Post>)>::new()
                .map(|(_, p): (K, Id<Post>)| p)
                .select(comments_of(db).opt().and(owner_user.select(badges_of(db)).select(&db.badge.class).opt()).and(links_of(db).select(&db.post_link.related_post_id).opt())),
        )
        .buf_fold(|rows| {
            let c = distinct_some(rows.iter().map(|r| r.0 .0));
            let b = rows.iter().filter_map(|r| r.0 .1).max();
            (c, b, distinct_some(rows.iter().map(|r| r.1)))
        });
    let v = drain((&g).filt(|(c, _, _): (i64, Option<i64>, i64)| c > 5));
    rows(v.into_iter().map(|((t, name, d, s, w, up, down), (c, b, l))| {
        row(vec![ostr(t), V::S(name), V::T(d), V::I(s), oint(w), V::I(up), V::I(down), V::I(up - down), V::S(if up >= down { "Positive" } else { "Negative" }), V::I(c), oint(b), V::I(l)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1),
// EnhancedUserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(b.Id) AS BadgeCount, COUNT(DISTINCT p.Id) AS TotalQuestions,
//        MAX(u.Reputation) AS MaxReputation FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId AND v.VoteTypeId IN (8, 9) LEFT JOIN Badges b ON u.Id = b.UserId
//     LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName),
// UserRanks AS (SELECT UserId, DisplayName, DENSE_RANK() OVER (ORDER BY TotalBounty DESC) AS BountyRank, DENSE_RANK() OVER (ORDER BY MaxReputation DESC) AS ReputationRank FROM EnhancedUserStats),
// PostHistorySummary AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS HistoryCount, MAX(ph.CreationDate) AS LastModifiedDate FROM PostHistory ph GROUP BY ph.PostId, ph.PostHistoryTypeId)
// SELECT up.UserId, up.DisplayName, up.TotalBounty, up.BadgeCount, up.TotalQuestions, ur.BountyRank, ur.ReputationRank, iron.Id AS PostId, iron.Title, iron.CreationDate, iron.Score,
//        iron.ViewCount, phs.HistoryCount, phs.LastModifiedDate
// FROM EnhancedUserStats up JOIN UserRanks ur ON up.UserId = ur.UserId INNER JOIN RankedPosts iron ON up.UserId = iron.OwnerUserId AND iron.Rank <= 5
// LEFT JOIN PostHistorySummary phs ON iron.Id = phs.PostId
// WHERE (up.TotalBounty > 0 OR up.BadgeCount > 0) AND ur.BountyRank < 10 AND (phs.HistoryCount > 1 OR phs.LastModifiedDate IS NOT NULL) ORDER BY up.TotalBounty DESC, ur.ReputationRank ASC;
fn q20632(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let bounty = votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let asked = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(bounty.opt().and(badges_of(db).opt()).and(asked().opt()))
        .fold([0i64; 2], |a, ((v, b), _)| [a[0] + v.flatten().unwrap_or(0), a[1] + b.is_some() as i64]);
    let qc = db.user.group_by(Ident::<User>::new()).select(asked().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&us).and(&qc)), |&(_, (a, _))| Reverse(a[0]), true);
    let v = ranked(v, |&((u, _), _)| Reverse(db.user.reputation.get(u).unwrap()), true);
    let keep = rel(drain(rel(v).filt(|(((_, (a, _)), br), _)| (a[0] > 0 || a[1] > 0) && br < 10)).into_iter().map(|x| x.1).collect());
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp = rel(top);
    let by_owner: HashIdx<Id<User>, (Id<Post>, Id<User>)> = (&tp).map(|(_, u)| u).inv().select(&tp).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post.and(post_history_type_id)).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pr = rel(drain(&phs));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, i64), (i64, i64))> = (&pr).map(|((p, _), _)| p).inv().select(&pr).collect();
    type R = (((Id<User>, ([i64; 2], i64)), i64), i64);
    let v = drain((&keep).select(Same::<R>::new().and(Same::<R>::new().map(|(((u, _), _), _): R| u).select((&by_owner).map(|(p, _)| p).select(Ident::<Post>::new().and(&by_post))))));
    rows(v.into_iter().map(|(_, ((((u, (a, q)), br), rr), (p, (_, (n, d)))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(q), V::I(br), V::I(rr)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(n), V::T(d)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation >= 1000 THEN 'High Reputation' WHEN u.Reputation >= 500 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationLevel FROM Users u),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        PERCENT_RANK() OVER (ORDER BY COUNT(c.Id) DESC) AS CommentRank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title),
// PostHistoryDetails AS (SELECT ph.PostId, MAX(CASE WHEN pht.Name = 'Post Closed' THEN ph.CreationDate END) AS LastClosedDate, MAX(CASE WHEN pht.Name = 'Post Reopened' THEN ph.CreationDate END) AS LastReopenedDate,
//        COUNT(CASE WHEN pht.Name IN ('Edit Body', 'Edit Title') THEN 1 END) AS EditCount FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT p.Title, ps.CommentCount, ps.UpVotes, ps.DownVotes, CASE WHEN (ps.UpVotes - ps.DownVotes) > 0 THEN 'Positive' WHEN (ps.UpVotes - ps.DownVotes) < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment,
//        ur.ReputationLevel, COALESCE(PHD.LastClosedDate, PHD.LastReopenedDate) AS LastStatusChange, PHD.EditCount
// FROM PostStatistics ps JOIN Posts p ON ps.PostId = p.Id JOIN UserReputation ur ON p.OwnerUserId = ur.UserId LEFT JOIN PostHistoryDetails PHD ON p.Id = PHD.PostId
// WHERE ur.ReputationLevel <> 'Low Reputation' AND ps.CommentRank < 0.25 AND (COALESCE(PHD.LastClosedDate, PHD.LastReopenedDate) IS NOT NULL OR ps.CommentCount > 5)
// ORDER BY ps.CommentCount DESC, ps.UpVotes DESC;
fn q23109(db: &'static So) -> String {
    let s = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    let n = v.len() as f64;
    let pr = rel(v.into_iter().map(|((p, a), r)| (p, a, (r - 1) as f64 / (n - 1.0))).collect());
    let by_post: HashIdx<Id<Post>, (Id<Post>, [i64; 3], f64)> = (&pr).filt(|(_, _, r): (Id<Post>, [i64; 3], f64)| r < 0.25).map(|(p, _, _)| p).inv().select(&pr).collect();
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let phd = db.post_history.group_by(post).select(htype_name(db).and(hd)).fold((i64::MIN, i64::MIN, 0i64), |(c, r, e), (t, d)| {
        (if t == "Post Closed" { c.max(d) } else { c }, if t == "Post Reopened" { r.max(d) } else { r }, e + matches!(t, "Edit Body" | "Edit Title") as i64)
    });
    let owner_ok = (&db.post.owner_user).select(Ident::<User>::new().with((&db.user.reputation).ge(500)));
    let v = drain(
        (&by_post)
            .and(owner_ok)
            .and((&phd).opt())
            .filt(|(((_, a, _), _), h): (((Id<Post>, [i64; 3], f64), Id<User>), Option<(i64, i64, i64)>)| h.map_or(false, |h| h.0 != i64::MIN || h.1 != i64::MIN) || a[0] > 5),
    );
    rows(v.into_iter().map(|(p, (((_, a, _), u), h))| {
        let rep = db.user.reputation.get(u).unwrap();
        let net = a[1] - a[2];
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(if net > 0 { "Positive" } else if net < 0 { "Negative" } else { "Neutral" })]);
        f.push(V::S(if rep >= 1000 { "High Reputation" } else { "Medium Reputation" }));
        f.extend(match h {
            Some((c, r, e)) => [if c != i64::MIN { V::T(c) } else { tmax(r) }, V::I(e)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RECURSIVE UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN VT.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN VT.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY COUNT(P.Id) DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes VT ON P.Id = VT.PostId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, UpVoteCount, DownVoteCount FROM UserActivity WHERE UserRank <= 10),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(COUNT(C.Id), 0) AS CommentCount, COALESCE(MAX(PH.PostHistoryTypeId), 0) AS LastActionType,
//        MAX(PH.CreationDate) AS LastActionDate FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.Score),
// FilteredPosts AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.CommentCount, PS.LastActionType, PS.LastActionDate, T.TagName FROM PostStats PS LEFT JOIN Tags T ON PS.PostId = T.ExcerptPostId
//     WHERE PS.Score >= 10 AND (PS.CommentCount > 0 OR PS.LastActionType IN (10, 11))),
// RankedPosts AS (SELECT FP.*, RANK() OVER (ORDER BY FP.Score DESC, FP.CommentCount DESC) AS PostRank FROM FilteredPosts FP)
// SELECT TU.DisplayName, TU.Reputation, RP.Title, RP.Score, RP.CommentCount, RP.LastActionDate FROM TopUsers TU JOIN RankedPosts RP ON TU.UserId = RP.PostId
// WHERE RP.PostRank <= 5 ORDER BY TU.Reputation DESC, RP.Score DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. UserRank partitions by the user, one row each, so every user with Reputation > 0 is in TopUsers, and only its name and
// reputation are read, so its vote counts are not computed. `TU.UserId = RP.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q31375(db: &'static So) -> String {
    let Post { score, origid, .. } = &db.post;
    let ps = db
        .post
        .with(score.ge(10))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select((&db.post_history.post_history_type_id).and(&db.post_history.creation_date)).opt()))
        .fold((0i64, 0i64, i64::MIN), |(c, t, d), (x, h)| match h {
            Some((ht, hd)) => (c + x.is_some() as i64, t.max(ht), d.max(hd)),
            None => (c + x.is_some() as i64, t, d),
        });
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let v = drain((&ps).filt(|(c, t, _): (i64, i64, i64)| c > 0 || t == 10 || t == 11).and((&excerpt).opt()));
    let v = ranked(v, |&(p, ((c, _, _), _))| (Reverse(score.get(p).unwrap()), Reverse(c)), false);
    let top = rel(v.into_iter().take_while(|x| x.1 <= 5).map(|((p, (a, _)), _)| (p, a)).collect());
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let tu = Ident::<User>::new().with((&db.user.reputation).gt(0));
    let v = drain((&top).select(Same::<(Id<Post>, (i64, i64, i64))>::new().and(Same::<(Id<Post>, (i64, i64, i64))>::new().map(|(p, _)| p).select(origid).select(&uidx).select(tu))));
    rows(v.into_iter().map(|(_, ((p, (c, _, d)), u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(c), tmax(d)]);
        row(f)
    }))
}

// WITH RecursivePostCTE AS (SELECT Id, ParentId, Title, Score, CreationDate, ROW_NUMBER() OVER (PARTITION BY ParentId ORDER BY Score DESC) as Rank FROM Posts WHERE PostTypeId = 1),
// PostVotes AS (SELECT P.Id AS PostId, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.PostTypeId IN (1, 2) GROUP BY P.Id),
// PostHistoryDetails AS (SELECT PH.PostId, MAX(CASE WHEN PHT.Name = 'Edit Title' THEN PH.CreationDate END) AS LastTitleEdit, MAX(CASE WHEN PHT.Name = 'Post Closed' THEN PH.CreationDate END) AS ClosedDate,
//        MAX(CASE WHEN PHT.Name = 'Post Reopened' THEN PH.CreationDate END) AS ReopenedDate FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id GROUP BY PH.PostId),
// UserBadgeCount AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount FROM Badges B GROUP BY B.UserId)
// SELECT P.Id AS PostId, P.Title, P.Score AS PostScore, PH.LastTitleEdit, PH.ClosedDate, PH.ReopenedDate, PV.TotalVotes, PV.UpVotes, PV.DownVotes, COALESCE(UBC.BadgeCount, 0) AS UserBadgeCount,
//        RP.Rank AS TopAnswerRank
// FROM Posts P LEFT JOIN PostVotes PV ON P.Id = PV.PostId LEFT JOIN PostHistoryDetails PH ON P.Id = PH.PostId LEFT JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN UserBadgeCount UBC ON U.Id = UBC.UserId LEFT JOIN RecursivePostCTE RP ON P.AcceptedAnswerId = RP.Id
// WHERE P.CreationDate > '2022-01-01' AND (P.Score > 5 OR P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days') ORDER BY P.Score DESC, P.CreationDate DESC;
//
// RecursivePostCTE does not recurse. Its ROW_NUMBER ties on Score are broken by Id; no row of the answer reads one.
fn q30645(db: &'static So) -> String {
    let Post { post_type_id, parent, score, creation_date, owner_user, accepted_answer, .. } = &db.post;
    let rn = per_group(ranked(drain(db.post.with(post_type_id.eq(1)).select(parent.opt())), |&(p, g)| (g, Reverse(score.get(p).unwrap()), p), false), |&(_, g)| g);
    let rr = rel(rn.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank_of: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rr).map(|(p, _)| p).inv().select(&rr).collect();
    let pv = db
        .post
        .with(post_type_id.is_in([1, 2]))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let phd = db.post_history.group_by(&db.post_history.post).select(htype_name(db).and(&db.post_history.creation_date)).fold([i64::MIN; 3], |a, (t, d)| {
        [if t == "Edit Title" { a[0].max(d) } else { a[0] }, if t == "Post Closed" { a[1].max(d) } else { a[1] }, if t == "Post Reopened" { a[2].max(d) } else { a[2] }]
    });
    let ubc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let base = db.post.with(creation_date.gt(ts(2022, 1, 1, 0, 0, 0)).and(score.gt(5).or(creation_date.ge(add_days(date(2024, 10, 1), -30)))));
    let v = drain(base.select((&pv).opt().and((&phd).opt()).and(owner_user.select(&ubc).opt()).and(accepted_answer.select(&rank_of).opt())));
    rows(v.into_iter().map(|(p, (((a, h), b), r))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend(match h {
            Some(h) => h.map(tmax),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::I(b.unwrap_or(0)), r.map_or(V::Null, |r| V::I(r.1))]);
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.PostHistoryTypeId, ph.UserId, ROW_NUMBER() OVER(PARTITION BY ph.PostId ORDER BY ph.CreationDate) AS HistoryRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12, 13)),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalClosedPosts,
//        SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS TotalReopenedPosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN RecursivePostHistory ph ON p.Id = ph.PostId GROUP BY u.Id, u.DisplayName),
// ClosedPostStats AS (SELECT ph.UserId, COUNT(DISTINCT ph.PostId) AS ClosedPostCount, COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.PostId END) AS NewlyClosedPosts,
//        COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.PostId END) AS ReopenedPostsCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.UserId)
// SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalQuestions, ua.TotalAnswers, COALESCE(cps.ClosedPostCount, 0) AS TotalClosedPosts, COALESCE(cps.NewlyClosedPosts, 0) AS NewlyClosedPosts,
//        COALESCE(cps.ReopenedPostsCount, 0) AS ReopenedPostsCount, (CAST(ua.TotalAnswers AS FLOAT) / NULLIF(ua.TotalQuestions, 0)) * 100 AS AnswerToQuestionRatio,
//        (ua.TotalClosedPosts - ua.TotalReopenedPosts) AS NetClosedPosts
// FROM UserActivity ua LEFT JOIN ClosedPostStats cps ON ua.UserId = cps.UserId ORDER BY ua.TotalPosts DESC LIMIT 100;
fn q32925(db: &'static So) -> String {
    let t = &db.post_history.post_history_type_id;
    let rph = history_of(db).select(Ident::<PostHistory>::new().with(t.is_in([10, 11, 12, 13]))).select(t);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(rph.opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((pt, h)) => [a[0] + (pt == 1) as i64, a[1] + (pt == 2) as i64, a[2] + (h == Some(10)) as i64, a[3] + (h == Some(11)) as i64],
            None => a,
        });
    let cps = db.post_history.with(t.is_in([10, 11])).group_by(&db.post_history.user).select((&db.post_history.post).and(t)).buf_fold(|rows| {
        [
            distinct_some(rows.iter().map(|r| Some(r.0))),
            distinct_some(rows.iter().map(|r| Some(r.0).filter(|_| r.1 == 10))),
            distinct_some(rows.iter().map(|r| Some(r.0).filter(|_| r.1 == 11))),
        ]
    });
    let v = drain((&ua).and(user_distinct_posts(db)).and((&cps).opt()));
    let v = top_n(v, |&(u, ((_, n), _))| (Reverse(n), u), 100);
    rows(v.into_iter().map(|(u, ((a, n), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(c.unwrap_or([0; 3]).map(V::I));
        f.push(if a[0] == 0 { V::Null } else { V::F(((a[1] as f32 / a[0] as f32) * 100.0) as f64) });
        f.push(V::I(a[2] - a[3]));
        row(f)
    }))
}

// WITH RecursiveCTE AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate) AS OwnerPostNumber
//     FROM Posts p WHERE p.PostTypeId = 1),
// RecentUserPosts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(r.PostId) AS RecentPostCount, AVG(EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - r.CreationDate)) / 3600) AS AvgAgeInHours
//     FROM Users u LEFT JOIN RecursiveCTE r ON u.Id = r.OwnerUserId WHERE u.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// JoinWithVotes AS (SELECT r.*, v.VoteCount, COALESCE(v.UpVoteCount, 0) AS UpVoteCount, COALESCE(v.DownVoteCount, 0) AS DownVoteCount FROM RecentUserPosts r
//     LEFT JOIN (SELECT p.OwnerUserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p JOIN Votes v ON p.Id = v.PostId GROUP BY p.OwnerUserId) v ON r.UserId = v.OwnerUserId),
// UserWithBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT j.UserId, j.DisplayName, j.RecentPostCount, j.AvgAgeInHours, COALESCE(b.BadgeCount, 0) AS BadgeCount, j.VoteCount, j.UpVoteCount, j.DownVoteCount
// FROM JoinWithVotes j LEFT JOIN UserWithBadges b ON j.UserId = b.UserId WHERE j.RecentPostCount > 5 ORDER BY j.RecentPostCount DESC, j.AvgAgeInHours ASC;
fn q32729(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.creation_date).gt(add_years(t0, -1)));
    let asked = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(creation_date);
    let rup = users().group_by(Ident::<User>::new()).select(asked.opt()).fold((0i64, 0.0f64), |(n, s), d| match d {
        Some(d) => (n + 1, s + secs(t0 - d) / 3600.0),
        None => (n, s),
    });
    let vv = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id)))
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let bc = users().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&rup).filt(|(n, _): (i64, f64)| n > 5).and((&vv).opt()).and(&bc));
    rows(v.into_iter().map(|(u, (((n, s), a), b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::F(s / n as f64), V::I(b)]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2])],
            None => [V::Null, V::I(0), V::I(0)],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.Body, p.CreationDate, p.OwnerUserId,
//        CASE WHEN p.PostTypeId = 1 THEN 'Question' WHEN p.PostTypeId = 2 THEN 'Answer' WHEN p.PostTypeId = 4 THEN 'TagWikiExcerpt' ELSE 'Other' END AS PostType,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// FilteredQuestions AS (SELECT rp.PostId, rp.Title, rp.Tags, rp.Body, ur.DisplayName, ur.Reputation, ur.PostCount, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges
//     FROM RankedPosts rp INNER JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.PostType = 'Question' AND (LOWER(rp.Body) LIKE '%performance%' OR LOWER(rp.Body) LIKE '%benchmark%'))
// SELECT fq.PostId, fq.Title, fq.Tags, fq.Body, fq.DisplayName AS AuthorName, fq.Reputation AS AuthorReputation, fq.PostCount AS TotalPosts, fq.GoldBadges AS GoldBadgeCount,
//        fq.SilverBadges AS SilverBadgeCount, fq.BronzeBadges AS BronzeBadgeCount
// FROM FilteredQuestions fq ORDER BY fq.Reputation DESC, fq.PostId;
//
// PostRank is never read. UserReputation is computed only for the owners of the matching questions, the only users the join reads.
fn q26704(db: &'static So) -> String {
    let Post { post_type_id, creation_date, body, owner_user, .. } = &db.post;
    let fq = || {
        db.post
            .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)))
            .with(body.filt(|b: Str| {
                let l = b.to_lowercase();
                l.contains("performance") || l.contains("benchmark")
            }))
    };
    let owners: MatSet<Id<User>> = fq().select(owner_user).collect();
    let ur = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (_, c)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain(fq().select(owner_user.select(Ident::<User>::new().and(&pc).and(&ur))));
    rows(v.into_iter().map(|(p, ((u, n), a))| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "body"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.Reputation),
// PostScoreRanked AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, RANK() OVER (ORDER BY P.Score DESC) AS ScoreRank FROM Posts P WHERE P.PostTypeId = 1),
// RecentPostHistory AS (SELECT PH.PostId, PH.UserId, PH.PostHistoryTypeId, PH.CreationDate, RANK() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS RecentChangeRank
//     FROM PostHistory PH WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - interval '30 days'),
// ActivePostLinks AS (SELECT PL.PostId, PL.RelatedPostId, COUNT(*) AS LinkCount FROM PostLinks PL INNER JOIN Posts P ON PL.PostId = P.Id
//     WHERE P.LastActivityDate >= cast('2024-10-01 12:34:56' as timestamp) - interval '1 year' GROUP BY PL.PostId, PL.RelatedPostId)
// SELECT U.DisplayName, U.Reputation, UVS.TotalVotes, UVS.Upvotes, UVS.Downvotes, PS.PostId, PS.Title, PS.Score, PS.ScoreRank, RP.RecentChangeRank, COALESCE(AL.LinkCount, 0) AS ActiveLinks
// FROM Users U INNER JOIN UserVoteStats UVS ON U.Id = UVS.UserId INNER JOIN PostScoreRanked PS ON U.Id = PS.PostId
// LEFT JOIN RecentPostHistory RP ON PS.PostId = RP.PostId AND RP.RecentChangeRank = 1 LEFT JOIN ActivePostLinks AL ON PS.PostId = AL.PostId
// WHERE U.Reputation >= 1000 AND PS.Score > (SELECT AVG(Score) FROM Posts WHERE PostTypeId = 1) ORDER BY U.Reputation DESC, PS.Score DESC;
//
// `U.Id = PS.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q30110(db: &'static So) -> String {
    let Post { post_type_id, score, last_activity_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let (sum, n) = db.post.with(post_type_id.eq(1)).select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let mean = sum as f64 / n as f64;
    let r = ranked(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(_, s)| Reverse(s), false);
    let rr = rel(r.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank_of: HashIdx<i64, (Id<Post>, i64)> = (&rr).map(|(p, _)| p).select(&db.post.origid).inv().select(&rr).collect();
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let recent = || db.post_history.with(hd.ge(add_days(t0, -30)));
    let md = recent().group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = recent().select(post.and(hd)).inv().collect();
    let PostLink { post: lp, related_post_id, .. } = &db.post_link;
    let apl = db.post_link.with(lp.select(Ident::<Post>::new().with(last_activity_date.ge(add_years(t0, -1))))).group_by(lp.and(related_post_id)).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let ar = rel(drain(&apl));
    let links: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&ar).map(|((p, _), _)| p).inv().select(&ar).collect();
    let v = drain(
        db.user
            .with((&db.user.reputation).ge(1000))
            .select((&uvs).and((&db.user.origid).select(&rank_of).select(Same::<(Id<Post>, i64)>::new().with(Same::<(Id<Post>, i64)>::new().map(|(p, _)| p).select(score).filt(move |s: i64| s as f64 > mean))))),
    );
    let rv = rel(v);
    type R = (Id<User>, ([i64; 3], (Id<Post>, i64)));
    let v = drain((&rv).select(
        Same::<R>::new().and(Same::<R>::new().map(|(_, (_, (p, _))): R| p).select(Ident::<Post>::new().and(&md).select(&at).opt().and((&links).map(|(_, n)| n).opt()))),
    ));
    rows(v.into_iter().map(|(_, ((u, (a, (p, r))), (h, l)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "score"]));
        f.extend([V::I(r), if h.is_some() { V::I(1) } else { V::Null }, V::I(l.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.OwnerUserId, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY P.CreationDate DESC) AS RowNum
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
//     GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.OwnerUserId),
// PostMetrics AS (SELECT PD.PostId, PD.Title, PD.CreationDate, PD.Score, PD.CommentCount, PD.UpVotes, PD.DownVotes, COALESCE(UR.Reputation, 0) AS OwnerReputation, COALESCE(UR.BadgeCount, 0) AS OwnerBadgeCount,
//        COALESCE(UR.GoldBadges, 0) AS OwnerGoldBadges, COALESCE(UR.SilverBadges, 0) AS OwnerSilverBadges, COALESCE(UR.BronzeBadges, 0) AS OwnerBronzeBadges
//     FROM PostDetails PD LEFT JOIN UserReputation UR ON PD.OwnerUserId = UR.UserId WHERE PD.RowNum = 1)
// SELECT PostId, Title, CreationDate, Score, CommentCount, UpVotes, DownVotes, OwnerReputation, OwnerBadgeCount, OwnerGoldBadges, OwnerSilverBadges, OwnerBronzeBadges
// FROM PostMetrics ORDER BY Score DESC, CreationDate DESC FETCH FIRST 10 ROWS ONLY;
//
// RowNum partitions by the post, one row each. The order reads only base columns, so the ten posts are picked first and the comment x vote product is driven for those alone.
fn q573(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let top = top_n(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pd = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let v = drain((&pd).and(owner_user.select(Ident::<User>::new().and(&ur)).opt()));
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(a.map(V::I));
        match u {
            Some((u, b)) => {
                f.push(user_col(db, u, "rep"));
                f.extend(b.map(V::I));
            }
            None => f.extend([0; 5].map(V::I)),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// UserVoteStats AS (SELECT v.UserId, SUM(CASE WHEN vt.Id = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN vt.Id = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId),
// CombinedStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(rb.BadgeCount, 0) AS BadgeCount, COALESCE(us.TotalUpVotes, 0) AS TotalUpVotes, COALESCE(us.TotalDownVotes, 0) AS TotalDownVotes,
//        COUNT(rp.PostId) AS RecentPostsCount FROM Users u LEFT JOIN UserBadges rb ON u.Id = rb.UserId LEFT JOIN UserVoteStats us ON u.Id = us.UserId
//     LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId AND rp.rn <= 5 GROUP BY u.Id, u.DisplayName, rb.BadgeCount, us.TotalUpVotes, us.TotalDownVotes)
// SELECT cs.UserId, cs.DisplayName, cs.BadgeCount, cs.TotalUpVotes, cs.TotalDownVotes, cs.RecentPostsCount, (cs.TotalUpVotes - cs.TotalDownVotes) AS VoteNet,
//        CASE WHEN cs.BadgeCount > 0 THEN 'Active' ELSE 'Inactive' END AS UserStatus
// FROM CombinedStats cs WHERE cs.RecentPostsCount > 0 ORDER BY VoteNet DESC, cs.RecentPostsCount DESC;
//
// The comment count is never read.
fn q1205(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rc = (&tp).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let vs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&rc).and(&bc).and((&vs).opt()));
    rows(v.into_iter().map(|(u, ((n, b), s))| {
        let s = s.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(s[0]), V::I(s[1]), V::I(n), V::I(s[0] - s[1]), V::S(if b > 0 { "Active" } else { "Inactive" })]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (ORDER BY p.ViewCount DESC) AS PopularityRank FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// PostHistoryDetails AS (SELECT ph.Id AS HistoryId, ph.PostId, p.Title AS PostTitle, p.Body AS PostBody, p.OwnerDisplayName AS Author, MAX(ph.CreationDate) AS LastEdited
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id GROUP BY ph.Id, ph.PostId, p.Title, p.Body, p.OwnerDisplayName),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName)
// SELECT ub.DisplayName AS UserName, ub.BadgeCount, pp.Title AS PopularPostTitle, pp.Score AS PopularPostScore, pp.ViewCount AS PopularPostViews, pp.AnswerCount AS PopularPostAnswers,
//        pp.CommentCount AS PopularPostComments, phd.LastEdited, ua.PostsCount, ua.UpVotes, ua.DownVotes
// FROM UserBadges ub JOIN PopularPosts pp ON ub.UserId = pp.PostId JOIN PostHistoryDetails phd ON pp.PostId = phd.PostId JOIN UserActivity ua ON ub.UserId = ua.UserId
// WHERE pp.PopularityRank <= 10 ORDER BY ub.BadgeCount DESC, pp.ViewCount DESC;
//
// `ub.UserId = pp.PostId` joins a user id to a post id, so it goes through the raw ids. PostHistoryDetails groups by the history row, so LastEdited is each row's own date.
fn q7994(db: &'static So) -> String {
    let Post { creation_date, view_count, origid, .. } = &db.post;
    let top = top_n(drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(view_count.opt())), |&(p, w)| (w.is_none(), Reverse(w), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(Some(2))) as i64, a[1] + (t == Some(Some(3))) as i64]);
    let pc = user_distinct_posts(db);
    let v = drain((&tp).select(origid.select(&uidx).select(Ident::<User>::new().and(&bc).and(&ua).and(&pc)).and(history_of(db).select(&db.post_history.creation_date))));
    rows(v.into_iter().map(|(p, ((((u, b), a), n), d))| {
        let mut f = vec![user_col(db, u, "name"), V::I(b)];
        f.extend(post_fields(db, p, &["title", "score", "views", "answers", "comments"]));
        f.extend([V::T(d), V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount, COUNT(DISTINCT P.Id) AS PostCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// BadgeCounts AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadgeCount, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadgeCount,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadgeCount FROM Badges B GROUP BY B.UserId),
// PostActivity AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, COALESCE(COUNT(Cm.Id), 0) AS CommentCount, COALESCE(MAX(PH.CreationDate), P.CreationDate) AS LastActivityDate
//     FROM Posts P LEFT JOIN Comments Cm ON P.Id = Cm.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount),
// RankedPosts AS (SELECT PA.*, ROW_NUMBER() OVER (ORDER BY PA.ViewCount DESC, PA.LastActivityDate DESC) AS Rank FROM PostActivity PA)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.UpvoteCount, U.DownvoteCount, U.PostCount, COALESCE(BC.GoldBadgeCount, 0) AS GoldBadgeCount, COALESCE(BC.SilverBadgeCount, 0) AS SilverBadgeCount,
//        COALESCE(BC.BronzeBadgeCount, 0) AS BronzeBadgeCount, RP.PostId, RP.Title, RP.CreationDate, RP.ViewCount, RP.CommentCount, RP.Rank
// FROM UserStats U LEFT JOIN BadgeCounts BC ON U.UserId = BC.UserId LEFT JOIN RankedPosts RP ON U.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = RP.PostId)
// WHERE U.Reputation > 1000 ORDER BY U.Reputation DESC, RP.Rank ASC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
//
// The correlated subquery is the post's owner, so RankedPosts joins each user through the posts they own. UserStats is computed only for the users with Reputation > 1000.
// Rank ties on (ViewCount, LastActivityDate) are broken by Id; the query leaves them open.
fn q4808(db: &'static So) -> String {
    let Post { view_count, creation_date, .. } = &db.post;
    let pa = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, m.max(d.unwrap_or(i64::MIN))));
    let r = drain(&pa);
    let last = |p: Id<Post>, m: i64| if m == i64::MIN { creation_date.get(p).unwrap() } else { m };
    let r = ranked(r, |&(p, (_, m))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(last(p, m)), p)
    }, false);
    let rr = rel(r.into_iter().map(|((p, (n, _)), k)| (p, (n, k))).collect());
    let rank_of: HashIdx<Id<Post>, (Id<Post>, (i64, i64))> = (&rr).map(|(p, _)| p).inv().select(&rr).collect();
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let us = users().group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt())).fold([0i64; 2], |a, (t, _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bcs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&us).and(&pc).and((&bcs).opt()).and(posts_of(db).select(&rank_of).opt()));
    let v = top_n(v, |&(u, (_, r))| (Reverse(db.user.reputation.get(u).unwrap()), r.is_none(), r.map(|x| x.1 .1)), 20);
    rows(v.into_iter().skip(10).map(|(u, (((a, n), b), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend(match r {
            Some((p, (c, k))) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "views"]);
                g.extend([V::I(c), V::I(k)]);
                g
            }
            None => (0..6).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostsWithComments AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title),
// FinalResults AS (SELECT up.UserId, CASE WHEN up.Reputation < 1000 THEN 'Bronze Level' WHEN up.Reputation BETWEEN 1000 AND 5000 THEN 'Silver Level' ELSE 'Gold Level' END AS ReputationLevel,
//        rp.Title AS LastQuestion, rp.CreationDate AS LastQuestionDate, rp.Score AS LastQuestionScore, COALESCE(pwc.CommentCount, 0) AS LastQuestionComments
//     FROM UserReputation up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId LEFT JOIN PostsWithComments pwc ON rp.PostId = pwc.PostId WHERE rp.rn = 1)
// SELECT fr.UserId, fr.ReputationLevel, fr.LastQuestion, fr.LastQuestionDate, fr.LastQuestionScore, fr.LastQuestionComments
// FROM FinalResults fr ORDER BY fr.ReputationLevel DESC, fr.LastQuestionScore DESC LIMIT 100;
//
// The badge counts are never read.
fn q3854(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let level = |p: Id<Post>| {
        let r = db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
        if r < 1000 { "Bronze Level" } else if r <= 5000 { "Silver Level" } else { "Gold Level" }
    };
    let v = top_n(drain(&cc), |&(p, _)| (Reverse(level(p)), Reverse(score.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, c)| {
        let mut f = post_fields(db, p, &["uid"]);
        f.push(V::S(level(p)));
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, SUM(b.Class) AS TotalBadgeClass, AVG(u.Reputation) AS AverageReputation FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopUsers AS (SELECT ur.UserId, ur.AverageReputation, ur.TotalBadgeClass FROM UserReputation ur WHERE ur.AverageReputation > 1000 AND ur.TotalBadgeClass > 2),
// PostSummary AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(vs.VoteCount, 0) AS VoteCount FROM RankedPosts rp
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON rp.PostId = c.PostId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) vs ON rp.PostId = vs.PostId)
// SELECT ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.CommentCount, ps.VoteCount,
//        CASE WHEN ps.ViewCount > 1000 THEN 'High Traffic' WHEN ps.ViewCount BETWEEN 500 AND 1000 THEN 'Moderate Traffic' ELSE 'Low Traffic' END AS TrafficCategory, u.DisplayName
// FROM PostSummary ps JOIN Users u ON ps.PostId = u.Id WHERE EXISTS (SELECT 1 FROM TopUsers tu WHERE tu.UserId = u.Id) ORDER BY ps.Score DESC, ps.ViewCount DESC FETCH FIRST 10 ROWS ONLY;
//
// `ps.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. UserRank is never read. AVG(u.Reputation) over a user's rows is its reputation.
fn q3765(db: &'static So) -> String {
    let Post { creation_date, score, view_count, origid, .. } = &db.post;
    let ur = db.user.group_by(Ident::<User>::new()).select((&db.user.reputation).and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 4], |a, (r, c)| {
        [a[0] + 1, a[1] + r, a[2] + c.is_some() as i64, a[3] + c.unwrap_or(0)]
    });
    let tu = Ident::<User>::new().with((&ur).filt(|a| a[1] as f64 / a[0] as f64 > 1000.0 && a[2] > 0 && a[3] > 2));
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let base: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(origid.select(&uidx).select(tu)).collect();
    let cc = (&base).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&base).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(&vc).and(origid.select(&uidx)));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, ((c, n), u))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n)]);
        f.push(V::S(match view_count.get(p) {
            Some(w) if w > 1000 => "High Traffic",
            Some(w) if w >= 500 => "Moderate Traffic",
            _ => "Low Traffic",
        }));
        f.push(user_col(db, u, "name"));
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON V.PostId = P.Id GROUP BY U.Id, U.Reputation),
// RankedUsers AS (SELECT UserId, Reputation, PostCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation),
// ActiveQuestions AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(P.AcceptedAnswerId, 0) AS AcceptedAnswerId, (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2) AS UpVoteCount, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3) AS DownVoteCount
//     FROM Posts P WHERE P.PostTypeId = 1 AND P.ClosedDate IS NULL),
// FinalResults AS (SELECT U.ReputationRank, AQ.PostId, AQ.Title, AQ.CreationDate, AQ.AcceptedAnswerId, AQ.CommentCount, AQ.UpVoteCount, AQ.DownVoteCount, U.Reputation,
//        CASE WHEN AQ.UpVoteCount > AQ.DownVoteCount THEN 'Positive' ELSE 'Negative' END AS PostSentiment
//     FROM ActiveQuestions AQ JOIN RankedUsers U ON U.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = AQ.PostId))
// SELECT FR.ReputationRank, FR.Title, FR.CreationDate, FR.Reputation, FR.CommentCount, FR.UpVoteCount, FR.DownVoteCount, FR.PostSentiment
// FROM FinalResults FR WHERE FR.Reputation > 1000 ORDER BY FR.ReputationRank, FR.CreationDate DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. Only Reputation and its rank are read from RankedUsers, so its counts are not computed. The correlated subquery is the post's owner.
fn q32215(db: &'static So) -> String {
    let Post { post_type_id, closed_date, owner_user, .. } = &db.post;
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank_of: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let aq = || db.post.with(post_type_id.eq(1)).minus(closed_date).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))));
    let cc = aq().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = aq().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&cc).and(&vc).and(owner_user.select(&rank_of)));
    rows(v.into_iter().map(|(p, ((c, a), (u, r)))| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([user_col(db, u, "rep"), V::I(c), V::I(a[0]), V::I(a[1]), V::S(if a[0] > a[1] { "Positive" } else { "Negative" })]);
        row(f)
    }))
}

// WITH RECURSIVE TopUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, u.CreationDate, RANK() OVER (ORDER BY u.Reputation DESC) as UserRank FROM Users u WHERE u.Reputation > 0),
// PostApproval AS (SELECT p.Id AS PostId, p.Title, p.Score, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, MAX(v.CreationDate) AS LastVoteDate,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.OwnerUserId, u.DisplayName),
// PostHistorySummary AS (SELECT p.Id, COUNT(ph.Id) AS EditCount, MIN(ph.CreationDate) AS FirstEditDate, MAX(ph.CreationDate) AS LastEditDate FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY p.Id)
// SELECT tu.UserRank, tu.DisplayName AS TopUser, pa.PostId, pa.Title AS PostTitle, pa.Score, pa.CommentCount, phs.EditCount, phs.FirstEditDate, phs.LastEditDate, pa.LastVoteDate, pa.UpVoteCount,
//        pa.DownVoteCount, CASE WHEN (pa.UpVoteCount - pa.DownVoteCount) > 0 THEN 'Positive' WHEN (pa.UpVoteCount - pa.DownVoteCount) < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM TopUsers tu LEFT JOIN PostApproval pa ON tu.Id = pa.OwnerUserId LEFT JOIN PostHistorySummary phs ON pa.PostId = phs.Id WHERE tu.UserRank <= 10 ORDER BY tu.UserRank, pa.Score DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. UserRank reads only Reputation, so the top users are picked first.
fn q32829(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let r = ranked(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let Vote { vote_type_id, creation_date: vd, .. } = &db.vote;
    let pa = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id.and(vd)).opt()))
        .fold([0, 0, i64::MIN, 0, 0], |a, (c, v)| match v {
            Some((t, d)) => [a[0] + c.is_some() as i64, a[1], a[2].max(d), a[3] + (t == 2) as i64, a[4] + (t == 3) as i64],
            None => [a[0] + c.is_some() as i64, a[1], a[2], a[3], a[4]],
        });
    let cd = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6]))).select(hd);
    let phs = db.post.group_by(Ident::<Post>::new()).select(phs).fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), d| (n + 1, lo.min(d), hi.max(d)));
    type R = (Id<User>, i64);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(posts_of(db).select(Ident::<Post>::new().and(&pa).and(&cd).and((&phs).opt())).opt()))));
    rows(v.into_iter().map(|(_, ((u, r), x))| {
        let mut f = vec![V::I(r), user_col(db, u, "name")];
        match x {
            Some((((p, a), c), h)) => {
                f.extend(post_fields(db, p, &["id", "title", "score"]));
                f.push(V::I(c));
                f.extend(match h {
                    Some((n, lo, hi)) => [V::I(n), V::T(lo), V::T(hi)],
                    None => [V::Null, V::Null, V::Null],
                });
                let net = a[3] - a[4];
                f.extend([tmax(a[2]), V::I(a[3]), V::I(a[4]), V::S(if net > 0 { "Positive" } else if net < 0 { "Negative" } else { "Neutral" })]);
            }
            None => {
                f.extend((0..10).map(|_| V::Null));
                f.push(V::S("Neutral"));
            }
        }
        row(f)
    }))
}

// WITH Recent_Posts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// Top_Users AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY SUM(p.Score) DESC) AS ScoreRank FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(p.Id) > 5),
// Post_History_Details AS (SELECT ph.PostId, ph.UserDisplayName, ph.CreationDate AS EditDate, ph.Comment AS EditComment, pt.Name AS PostHistoryType FROM PostHistory ph
//     JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id WHERE pt.Id IN (2, 5, 10))
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.UpVotes AS TotalUpVotes, rp.DownVotes AS TotalDownVotes, rp.CommentCount, tu.DisplayName AS TopUser,
//        tu.Reputation AS UserReputation, pd.EditDate, pd.EditComment, pd.PostHistoryType
// FROM Recent_Posts rp JOIN Top_Users tu ON rp.UserPostRank = 1 AND rp.OwnerUserId = tu.UserId LEFT JOIN Post_History_Details pd ON rp.PostId = pd.PostId
// WHERE rp.Score > 0 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 100;
//
// ScoreRank is never read. UserPostRank reads only base columns, so each owner's newest recent post is picked first and the vote x comment product is driven for those alone.
fn q34199(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let rp = (&tp)
        .with(score.gt(0))
        .with(owner_user.select(Ident::<User>::new().with((&pc).filt(|n| n > 5))))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let pd = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([2, 5, 10])));
    let v = top_n(drain((&rp).and(pd.opt())), |&(p, (_, h))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, h)
    }, 100);
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend(match h {
            Some(h) => [V::T(db.post_history.creation_date.get(h).unwrap()), ostr(db.post_history.comment.get(h)), V::S(htype_name(db).get(h).unwrap())],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS TotalPosts,
//        SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, COUNT(c.Id) AS TotalComments,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// PostEngagement AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.PositivePosts, us.NegativePosts, us.TotalComments, ROW_NUMBER() OVER (ORDER BY us.Reputation DESC) AS Rank FROM UserStats us)
// SELECT tu.Rank, tu.DisplayName, tu.Reputation, pe.Title AS TopPostTitle, pe.Score AS TopPostScore, pe.CommentCount AS TopPostComments
// FROM TopUsers tu JOIN PostEngagement pe ON tu.UserId = pe.PostId WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// UserStats has one row per user and only its name and reputation are read, so its counts are not computed. `tu.UserId = pe.PostId` joins a user id to a post id,
// so it goes through the raw ids. The rank reads only Reputation, so the ten users are picked first.
fn q9106(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(tu.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let pidx: HashIdx<i64, Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(&db.post.origid).inv().collect();
    type R = (Id<User>, i64);
    let pe = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&pe)))));
    rows(v.into_iter().map(|(_, ((u, r), (p, c)))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(post_fields(db, p, &["title", "score"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.Score, COALESCE(P.ViewCount, 0) AS ViewCount, COALESCE(P.AnswerCount, 0) AS AnswerCount, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        MAX(B.Date) AS LastBadgeDate, P.CreationDate FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON P.OwnerUserId = B.UserId
//     GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.AnswerCount, P.CreationDate),
// OverallStatistics AS (SELECT U.UserId, U.DisplayName, U.UpVotes, U.DownVotes, SUM(PS.Score) AS TotalScore, SUM(PS.ViewCount) AS TotalViews, SUM(PS.AnswerCount) AS TotalAnswers,
//        SUM(PS.CommentCount) AS TotalComments, COUNT(PS.PostId) AS TotalPosts FROM UserVoteSummary U LEFT JOIN PostStatistics PS ON U.UserId = PS.PostId GROUP BY U.UserId, U.DisplayName, U.UpVotes, U.DownVotes)
// SELECT O.DisplayName AS UserName, O.UpVotes, O.DownVotes, O.TotalScore, O.TotalViews, O.TotalAnswers, O.TotalComments, O.TotalPosts,
//        CASE WHEN O.TotalPosts = 0 THEN 'No Posts' WHEN O.TotalScore > 100 THEN 'Highly Active' ELSE 'Moderate Activity' END AS ActivityLevel, RANK() OVER (ORDER BY O.TotalScore DESC) AS ScoreRank
// FROM OverallStatistics O WHERE O.TotalViews IS NOT NULL AND O.TotalScore IS NOT NULL ORDER BY O.TotalPosts DESC, O.TotalScore DESC OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY;
//
// `U.UserId = PS.PostId` joins a user id to a post id, so it goes through the raw ids; the WHERE keeps only the users that matched a post.
fn q22034(db: &'static So) -> String {
    let Post { score, view_count, answer_count, owner_user, origid, .. } = &db.post;
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let ps = db
        .post
        .with(origid.select(&uidx))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(owner_user.select(badges_of(db)).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain(db.user.select((&uv).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&ps)))));
    let v = ranked(v, |&(_, (_, (p, _)))| Reverse(score.get(p).unwrap()), false);
    let v = top_n(v, |&((u, (_, (p, _))), _)| (Reverse(score.get(p).unwrap()), u), 15);
    rows(v.into_iter().skip(5).map(|((u, (a, (p, c))), r)| {
        let s = score.get(p).unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(s), V::I(view_count.get(p).unwrap_or(0)), V::I(answer_count.get(p).unwrap_or(0)), V::I(c), V::I(1)];
        f.push(V::S(if s > 100 { "Highly Active" } else { "Moderate Activity" }));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS VersionNumber
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12, 13)),
// UserReputation AS (SELECT u.Id, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStatistics AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, AVG(v.BountyAmount) AS AverageBounty, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id)
// SELECT p.Title, p.CreationDate AS PostCreationDate, ph.PostHistoryTypeId, ph.CreationDate AS HistoryDate, u.DisplayName AS UserDisplayName, u.Reputation, up.GoldBadges, up.SilverBadges, up.BronzeBadges,
//        ps.CommentCount, ps.AverageBounty, ps.UpVotes, ps.DownVotes
// FROM RecursivePostHistory ph JOIN Posts p ON p.Id = ph.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserReputation up ON u.Id = up.Id LEFT JOIN PostStatistics ps ON p.Id = ps.PostId
// WHERE ph.VersionNumber = 1 AND (ph.PostHistoryTypeId = 10 OR ph.PostHistoryTypeId = 11) ORDER BY p.CreationDate DESC, ph.CreationDate DESC LIMIT 50;
//
// RecursivePostHistory does not recurse. The order reads only base columns, so the fifty rows are picked first and PostStatistics is driven for those posts alone.
fn q20306(db: &'static So) -> String {
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let v = drain(db.post_history.with(post_history_type_id.is_in([10, 11, 12, 13])).select(post));
    let latest = rel(top_per(v, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false));
    let latest = drain((&latest).filt(|(h, _): (Id<PostHistory>, Id<Post>)| matches!(post_history_type_id.get(h).unwrap(), 10 | 11)));
    let created = &db.post.creation_date;
    let top = top_n(latest.into_iter().map(|x| x.1).collect(), |&(h, p)| (Reverse(created.get(p).unwrap()), Reverse(hd.get(h).unwrap()), h), 50);
    let tv = rel(top);
    let tps: MatSet<Id<Post>> = (&tv).map(|(_, p)| p).collect();
    let ps = (&tps)
        .with(created.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()))
        .fold([0i64; 5], |a, (c, v)| {
            let (t, b) = v.map_or((0, None), |(t, b)| (t, b));
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0), a[3] + (t == 2) as i64, a[4] + (t == 3) as i64]
        });
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    type R = (Id<PostHistory>, Id<Post>);
    let v = drain((&tv).select(Same::<R>::new().and(Same::<R>::new().map(|(_, p): R| p).select((&db.post.owner_user).select(Ident::<User>::new().and(&ur)).opt().and((&ps).opt())))));
    rows(v.into_iter().map(|(_, ((h, p), (u, s)))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::I(post_history_type_id.get(h).unwrap()), V::T(hd.get(h).unwrap())]);
        f.extend(match u {
            Some((u, b)) => {
                let mut g = ucols(db, u, &["name", "rep"]);
                g.extend(b.map(V::I));
                g
            }
            None => (0..5).map(|_| V::Null).collect(),
        });
        f.extend(match s {
            Some(a) => [V::I(a[0]), avg(a[2], a[1]), V::I(a[3]), V::I(a[4])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, p.AnswerCount, p.CommentCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn FROM Posts p WHERE p.PostTypeId = 1),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// ClosedPostHistory AS (SELECT p.Id AS ClosedPostId, ph.CreationDate AS CloseDate, ph.UserDisplayName AS Closer FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10),
// RecentUserScores AS (SELECT u.Id AS UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostSummary AS (SELECT rp.Title, rp.ViewCount, ub.BadgeCount, cp.CloseDate, cp.Closer, rus.UpVotes, rus.DownVotes FROM RankedPosts rp JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId
//     LEFT JOIN ClosedPostHistory cp ON rp.PostId = cp.ClosedPostId JOIN RecentUserScores rus ON rp.OwnerUserId = rus.UserId WHERE rp.rn = 1)
// SELECT ps.Title, ps.ViewCount, ps.BadgeCount, ps.CloseDate, ps.Closer, COALESCE(ps.UpVotes, 0) AS TotalUpVotes, COALESCE(ps.DownVotes, 0) AS TotalDownVotes,
//        CASE WHEN ps.CloseDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus
// FROM PostSummary ps ORDER BY ps.ViewCount DESC FETCH FIRST 50 ROWS ONLY;
fn q33066(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let rus = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&tp).select(owner_user.select((&bc).and(&rus)).and(closes.opt())));
    let v = top_n(v, |&(p, (_, h))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p, h)
    }, 50);
    rows(v.into_iter().map(|(p, ((b, a), h))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.push(V::I(b));
        f.extend(match h {
            Some(h) => [V::T(db.post_history.creation_date.get(h).unwrap()), ostr(db.post_history.user_display_name.get(h))],
            None => [V::Null, V::Null],
        });
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if h.is_some() { "Closed" } else { "Active" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, u.DisplayName AS UserDisplayName, COALESCE(p2.Title, 'No Accepted Answer') AS AcceptedAnswerTitle,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS RankByTags FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts p2 ON p.AcceptedAnswerId = p2.Id WHERE p.PostTypeId = 1),
// StringMetrics AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS UserDisplayName, p.ViewCount, p.Score, p.CreationDate, LENGTH(p.Tags) AS TagLength, LENGTH(p.Title) AS TitleLength,
//        LENGTH(p.Body) AS BodyLength FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// PostBenchmark AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.UserDisplayName, sm.TagLength, sm.TitleLength, sm.BodyLength, (sm.TagLength + sm.TitleLength + sm.BodyLength) AS TotalStringLength,
//        DENSE_RANK() OVER (ORDER BY (sm.TagLength + sm.TitleLength + sm.BodyLength) DESC) AS DensityRank, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM RankedPosts rp JOIN StringMetrics sm ON rp.PostId = sm.PostId LEFT JOIN Comments c ON rp.PostId = c.PostId
//     GROUP BY rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.UserDisplayName, sm.TagLength, sm.TitleLength, sm.BodyLength)
// SELECT pb.*, CASE WHEN pb.DensityRank <= 10 THEN 'Top 10 Posts by String Length' WHEN pb.CommentCount > 50 THEN 'Highly Commented Post' ELSE 'Regular Post' END AS PostTypeCategory
// FROM PostBenchmark pb WHERE pb.Score > 0 ORDER BY pb.ViewCount DESC, pb.Score DESC FETCH FIRST 20 ROWS ONLY;
//
// RankByTags and AcceptedAnswerTitle are never read.
fn q26464(db: &'static So) -> String {
    let Post { post_type_id, tags_str, title, body, score, view_count, .. } = &db.post;
    let len = |s: Option<Str>| s.map(|s| s.chars().count() as i64);
    let lens = |p: Id<Post>| (len(tags_str.get(p)), len(title.get(p)), body.get(p).unwrap().chars().count() as i64);
    let total = |p: Id<Post>| {
        let (t, ti, b) = lens(p);
        Some(t? + ti? + b)
    };
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().map(total))), |&(_, t)| (t.is_none(), Reverse(t)), true);
    let rr = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let pos = (&rr).filt(|(p, _): (Id<Post>, i64)| score.get(p).unwrap() > 0);
    let v = top_n(drain(pos), |&(_, (p, _))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(score.get(p).unwrap()), p)
    }, 20);
    let tv = rel(v.into_iter().map(|x| x.1).collect());
    type R = (Id<Post>, i64);
    let cc = (&tv).group_by(Same::<R>::new()).select(Same::<R>::new().map(|(p, _): R| p).select(comments_of(db).opt())).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain(&cc).into_iter().map(|((p, r), c)| {
        let (t, ti, b) = lens(p);
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "owner"]);
        f.extend([oint(t), oint(ti), V::I(b), oint(total(p)), V::I(r), V::I(c)]);
        f.push(V::S(if r <= 10 { "Top 10 Posts by String Length" } else if c > 50 { "Highly Commented Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RECURSIVE UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, b.Name AS BadgeName, b.Class, b.Date, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY b.Date DESC) AS BadgeRank
//     FROM Users u JOIN Badges b ON u.Id = b.UserId),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(*) AS TotalPosts, AVG(p.Score) AS AverageScore, SUM(p.ViewCount) AS TotalViews, COUNT(c.Id) AS TotalComments,
//        COALESCE(MAX(v.CreationDate), '1900-01-01') AS LastVoteDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.OwnerUserId),
// ClosedPosts AS (SELECT p.OwnerUserId, COUNT(*) AS ClosedPostCount, AVG(p.Score) AS AvgClosedPostScore FROM Posts p WHERE p.ClosedDate IS NOT NULL GROUP BY p.OwnerUserId),
// MostActiveUsers AS (SELECT ps.OwnerUserId, us.DisplayName, ps.TotalPosts, ps.AverageScore, ps.TotalViews, ps.TotalComments, COALESCE(cp.ClosedPostCount, 0) AS ClosedPostCount,
//        COALESCE(cp.AvgClosedPostScore, 0) AS AvgClosedPostScore, RANK() OVER (ORDER BY ps.TotalPosts DESC) AS OverallRank
//     FROM PostStatistics ps LEFT JOIN ClosedPosts cp ON ps.OwnerUserId = cp.OwnerUserId JOIN Users us ON ps.OwnerUserId = us.Id)
// SELECT mu.DisplayName, mu.TotalPosts, mu.AverageScore, mu.TotalViews, mu.TotalComments, mu.ClosedPostCount, mu.AvgClosedPostScore, COUNT(DISTINCT ub.BadgeName) AS UniqueBadgeCount
// FROM MostActiveUsers mu LEFT JOIN UserBadges ub ON mu.OwnerUserId = ub.UserId AND ub.BadgeRank = 1 WHERE mu.TotalPosts > 10
// GROUP BY mu.DisplayName, mu.TotalPosts, mu.AverageScore, mu.TotalViews, mu.TotalComments, mu.ClosedPostCount, mu.AvgClosedPostScore
// ORDER BY mu.TotalPosts DESC, UniqueBadgeCount DESC FETCH FIRST 10 ROWS ONLY;
//
// WITH RECURSIVE, but no CTE refers to itself. OverallRank is never read. The GROUP BY names the columns, not the user, so users are grouped by that tuple.
fn q33086(db: &'static So) -> String {
    let Post { score, view_count, closed_date, .. } = &db.post;
    let ps = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).opt())))
        .fold([0i64; 5], |a, (((s, w), c), _)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c.is_some() as i64]);
    let cp = db.post.with(closed_date).group_by(&db.post.owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let Badge { user, date: bd, .. } = &db.badge;
    let lb = top_per(drain(db.badge.select(user)), |&(_, u)| u, |&(b, _)| (Reverse(bd.get(b).unwrap()), b), 1, false);
    let lb: MatSet<Id<Badge>> = rel(lb.into_iter().map(|x| x.0).collect()).map(|b| b).collect();
    let v = drain((&ps).filt(|a| a[0] > 10).and((&cp).opt()));
    type K = (Str, i64, i64, Option<i64>, i64, i64, i64);
    let keyed: Vec<(K, Id<User>)> = v
        .into_iter()
        .map(|(u, (a, c))| {
            let (cn, cs) = c.map_or((0, 0), |c| (c[0], fkey(c[1] as f64 / c[0] as f64)));
            ((db.user.display_name.get(u).unwrap(), a[0], fkey(a[1] as f64 / a[0] as f64), if a[2] > 0 { Some(a[3]) } else { None }, a[4], cn, cs), u)
        })
        .collect();
    let kr = rel(keyed);
    let g = (&kr)
        .group_by(Same::<(K, Id<User>)>::new().map(|(k, _): (K, Id<User>)| k))
        .select(Same::<(K, Id<User>)>::new().map(|(_, u): (K, Id<User>)| u).and(Same::<(K, Id<User>)>::new().map(|(_, u): (K, Id<User>)| u).select(badges_of(db).select(Ident::<Badge>::new().with(&lb)).select(&db.badge.name).opt())))
        .buf_fold(|rows| (rows[0].0, distinct_some(rows.iter().map(|r| r.1))));
    let v = top_n(drain(&g), |&(k, (u, n))| (Reverse(k.1), Reverse(n), u), 10);
    rows(v.into_iter().map(|((name, n, _, w, c, cn, _), (u, b))| {
        let a = ps.get(u).unwrap();
        let cps = cp.get(u);
        row(vec![V::S(name), V::I(n), avg(a[1], a[0]), oint(w), V::I(c), V::I(cn), cps.map_or(V::F(0.0), |c| avg(c[1], c[0])), V::I(b)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PostVoteCounts AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId),
// PostComments AS (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId),
// AggregatedData AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.OwnerDisplayName, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes,
//        COALESCE(pc.CommentCount, 0) AS CommentCount, rp.RankScore FROM RankedPosts rp LEFT JOIN PostVoteCounts pvc ON rp.PostId = pvc.PostId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId)
// SELECT ad.PostId, ad.Title, ad.CreationDate, ad.Score, ad.ViewCount, ad.AnswerCount, ad.OwnerDisplayName, ad.UpVotes, ad.DownVotes, ad.CommentCount,
//        CASE WHEN ad.Score > 0 THEN 'Popular' WHEN ad.Score < 0 THEN 'Unpopular' ELSE 'Neutral' END AS PopularityStatus,
//        CASE WHEN ad.UpVotes > ad.DownVotes THEN 'More Upvotes' WHEN ad.UpVotes < ad.DownVotes THEN 'More Downvotes' ELSE 'Equal Votes' END AS VoteSummary
// FROM AggregatedData ad WHERE ad.RankScore <= 10 ORDER BY ad.CreationDate DESC;
fn q34096(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&vc).and(&cc)).into_iter().map(|(p, (a, c))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.push(V::S(if s > 0 { "Popular" } else if s < 0 { "Unpopular" } else { "Neutral" }));
        f.push(V::S(if a[0] > a[1] { "More Upvotes" } else if a[0] < a[1] { "More Downvotes" } else { "Equal Votes" }));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount, COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END), 0) AS WikiCount, COALESCE(SUM(p.ViewCount), 0) AS TotalViews, COALESCE(SUM(p.Score), 0) AS TotalScore,
//        RANK() OVER (ORDER BY COALESCE(SUM(p.Score), 0) DESC) AS ScoreRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionCount, AnswerCount, WikiCount, TotalViews, TotalScore, ScoreRank FROM UserPostStats WHERE ScoreRank <= 10),
// PostHistorySummary AS (SELECT post.Id AS PostId, pt.Name AS PostType, COUNT(ph.PostId) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM Posts post JOIN PostHistory ph ON post.Id = ph.PostId
//     JOIN PostHistoryTypes pt ON pt.Id = ph.PostHistoryTypeId WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY post.Id, pt.Name),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COALESCE(ph.EditCount, 0) AS EditCount, ph.LastEditDate FROM Posts p LEFT JOIN PostHistorySummary ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'))
// SELECT u.DisplayName AS UserDisplayName, u.QuestionCount, u.AnswerCount, u.WikiCount, r.PostId, r.Title, r.CreationDate, r.EditCount, r.LastEditDate,
//        CASE WHEN r.EditCount > 0 THEN 'Edited' ELSE 'Unedited' END AS EditStatus
// FROM TopUsers u JOIN RecentPosts r ON u.UserId = r.OwnerUserId ORDER BY u.ScoreRank, u.TotalScore DESC, r.CreationDate DESC;
fn q31826(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, s)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 3 | 4 | 5) as i64, a[3] + s],
        None => a,
    });
    let r = ranked(drain(&ups), |&(_, a)| Reverse(a[3]), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, [i64; 4])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post.and(htype_name(db))).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pr = rel(drain(&phs));
    let hp: HashIdx<Id<Post>, ((Id<Post>, Str), (i64, i64))> = (&pr).map(|((p, _), _)| p).inv().select(&pr).collect();
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user.select(&by_user).and((&hp).opt())));
    rows(v.into_iter().map(|(p, ((u, a), h))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        let (n, d) = h.map_or((0, None), |(_, (n, d))| (n, Some(d)));
        f.extend([V::I(n), ots(d), V::S(if n > 0 { "Edited" } else { "Unedited" })]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// QuestionActivity AS (SELECT p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, AVG(EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - p.CreationDate)) / 3600) AS AvgHoursToFirstVote
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId),
// UserPostStats AS (SELECT us.UserId, us.DisplayName, us.PostCount, us.QuestionCount, us.AnswerCount, COALESCE(qa.CommentCount, 0) AS CommentCount, COALESCE(qa.VoteCount, 0) AS VoteCount,
//        COALESCE(qa.AvgHoursToFirstVote, 0) AS AvgHoursToFirstVote, (us.GoldBadges + us.SilverBadges + us.BronzeBadges) AS TotalBadges FROM UserStatistics us LEFT JOIN QuestionActivity qa ON us.UserId = qa.OwnerUserId),
// RankedUsers AS (SELECT *, RANK() OVER (ORDER BY PostCount DESC, AvgHoursToFirstVote ASC) AS Rank FROM UserPostStats)
// SELECT u.UserId, u.DisplayName, u.PostCount, u.QuestionCount, u.AnswerCount, u.CommentCount, u.VoteCount, u.AvgHoursToFirstVote, u.TotalBadges
// FROM RankedUsers u WHERE u.Rank <= 10 AND u.TotalBadges > (SELECT AVG(TotalBadges) FROM UserPostStats) ORDER BY u.PostCount DESC;
fn q22940(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, creation_date, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (t, c)| [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + matches!(c, Some(1 | 2 | 3)) as i64]);
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let qa = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(creation_date.and(comments_of(db).opt()).and(up.opt())))
        .fold((0i64, 0i64, 0i64, 0.0f64), |(n, c, v, s), ((d, x), y)| (n + 1, c + x.is_some() as i64, v + y.is_some() as i64, s + secs(t0 - d) / 3600.0));
    let v = drain((&us).and((&qa).opt()));
    let (sum, n) = (&us).fold_flat((0i64, 0i64), |(s, n), a| (s + a[3], n + 1));
    let mean = sum as f64 / n as f64;
    let hours = |q: Option<(i64, i64, i64, f64)>| q.map_or(0.0, |q| q.3 / q.0 as f64);
    let v = ranked(v, |&(_, (a, q))| (Reverse(a[0]), fkey(hours(q))), false);
    let v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).collect();
    let v = drain(rel(v).filt(move |((_, (a, _)), _)| a[3] as f64 > mean));
    rows(v.into_iter().map(|(_, ((u, (a, q)), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        let (c, vc) = q.map_or((0, 0), |q| (q.1, q.2));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c), V::I(vc), V::F(hours(q)), V::I(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, p.Tags, u.DisplayName AS Author, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, u.DisplayName, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, p.Tags),
// FilteredPostHistory AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate AS ChangeDate, MAX(ph.CreationDate) OVER (PARTITION BY ph.PostId) AS LastChangeDate FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (10, 11, 52, 53)),
// FinalResult AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.ViewCount, rp.Score, rp.Author, rp.CommentCount, rp.UpVotes, rp.DownVotes, MAX(fph.ChangeDate) AS LastHistoryChange,
//        MAX(fph.LastChangeDate) AS LastChange FROM RankedPosts rp LEFT JOIN FilteredPostHistory fph ON rp.PostId = fph.PostId
//     GROUP BY rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.ViewCount, rp.Score, rp.Author, rp.CommentCount, rp.UpVotes, rp.DownVotes)
// SELECT PostId, Title, Body, CreationDate, ViewCount, Score, Author, CommentCount, UpVotes, DownVotes, LastHistoryChange, LastChange
// FROM FinalResult WHERE LastHistoryChange IS NOT NULL ORDER BY Score DESC, ViewCount DESC LIMIT 10;
//
// PostRank is never read. The order reads only base columns, so the ten questions are picked first and the comment x vote product is driven for those alone.
fn q27852(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let lc = db.post_history.with(post_history_type_id.is_in([10, 11, 52, 53])).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let v = drain(db.post.with(post_type_id.eq(1)).select(&lc));
    let top = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&s).and(&lc)).into_iter().map(|(p, (a, d))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "views", "score", "owner"]);
        f.extend(a.map(V::I));
        f.extend([V::T(d), V::T(d)]);
        row(f)
    }))
}

// Rewritten (rewrites/2894.sql): the ROW_NUMBER order is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id) AS Rank,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVotes, COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVotes
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.Rank, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.Score > 100 THEN 'High Score' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory FROM RankedPosts rp WHERE rp.Rank <= 10),
// CommentCounts AS (SELECT p.Id AS PostId, COUNT(c.Id) AS TotalComments FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// FinalMetrics AS (SELECT pm.PostId, pm.Title, pm.Score, pm.ViewCount, pm.CreationDate, pm.Rank, pm.UpVotes, pm.DownVotes, pm.ScoreCategory, COALESCE(cc.TotalComments, 0) AS TotalComments,
//        (pm.UpVotes - pm.DownVotes) AS NetVotes FROM PostMetrics pm LEFT JOIN CommentCounts cc ON pm.PostId = cc.PostId)
// SELECT fm.PostId, fm.Title, fm.Score, fm.ViewCount, fm.CreationDate, fm.Rank, fm.UpVotes, fm.DownVotes, fm.ScoreCategory, fm.TotalComments, fm.NetVotes,
//        CASE WHEN fm.NetVotes >= 0 THEN 'Positive Engagement' ELSE 'Negative Engagement' END AS EngagementCategory
// FROM FinalMetrics fm WHERE fm.ScoreCategory = 'High Score' OR fm.TotalComments > 5 ORDER BY fm.ViewCount DESC, fm.Score DESC;
fn q2894(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let r = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), false), |&(_, t)| t);
    let tr = rel(r.into_iter().filter(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    type R = (Id<Post>, i64);
    let pm = (&tr)
        .group_by(Same::<R>::new())
        .select(Same::<R>::new().map(|(p, _): R| p).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tr).group_by(Same::<R>::new()).select(Same::<R>::new().map(|(p, _): R| p).select(comments_of(db).opt())).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&pm).and(&cc).and(Same::<R>::new().map(|(p, _): R| p).select(score)).filt(|((_, c), s): (([i64; 2], i64), i64)| s > 100 || c > 5));
    rows(v.into_iter().map(|((p, r), ((a, c), _))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.extend([V::I(r), V::I(a[0]), V::I(a[1]), V::S(if s > 100 { "High Score" } else if s >= 50 { "Medium Score" } else { "Low Score" }), V::I(c), V::I(a[0] - a[1])]);
        f.push(V::S(if a[0] - a[1] >= 0 { "Positive Engagement" } else { "Negative Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT r.PostId, r.Title, r.Score, r.CreationDate, r.OwnerDisplayName FROM RankedPosts r WHERE r.PostRank = 1),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostsWithComments AS (SELECT t.PostId, t.Title, t.Score, t.CreationDate, t.OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount FROM TopPosts t LEFT JOIN PostComments pc ON t.PostId = pc.PostId),
// VotesSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// FinalResult AS (SELECT p.PostId, p.Title, p.Score, p.CreationDate, p.OwnerDisplayName, p.CommentCount, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes,
//        (p.Score + COALESCE(vs.UpVotes, 0) - COALESCE(vs.DownVotes, 0)) AS NetScore FROM PostsWithComments p LEFT JOIN VotesSummary vs ON p.PostId = vs.PostId)
// SELECT *, CASE WHEN NetScore > 100 THEN 'High Engagement' WHEN NetScore BETWEEN 51 AND 100 THEN 'Medium Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM FinalResult ORDER BY NetScore DESC, CreationDate DESC;
fn q838(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vs)).into_iter().map(|(p, (c, a))| {
        let net = score.get(p).unwrap() + a[0] - a[1];
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(net)]);
        f.push(V::S(if net > 100 { "High Engagement" } else if net >= 51 { "Medium Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers,
//        SUM(COALESCE(p.CommentCount, 0)) AS TotalComments, SUM(COALESCE(p.FavoriteCount, 0)) AS TotalFavorites FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TagPopularity AS (SELECT Tag, COUNT(DISTINCT pt.PostId) AS PostCount, SUM(COALESCE(up.TotalViews, 0)) AS TotalViewsPerTag, SUM(COALESCE(up.TotalAnswers, 0)) AS TotalAnswersPerTag,
//        SUM(COALESCE(up.TotalComments, 0)) AS TotalCommentsPerTag, SUM(COALESCE(up.TotalFavorites, 0)) AS TotalFavoritesPerTag FROM PostTags pt LEFT JOIN UserPostStats up ON pt.PostId = up.UserId GROUP BY Tag)
// SELECT tp.Tag, tp.PostCount, tp.TotalViewsPerTag, tp.TotalAnswersPerTag, tp.TotalCommentsPerTag, tp.TotalFavoritesPerTag,
//        CASE WHEN tp.PostCount > 0 THEN ROUND(tp.TotalViewsPerTag * 1.0 / tp.PostCount, 2) ELSE 0 END AS AverageViewsPerPost,
//        CASE WHEN tp.PostCount > 0 THEN ROUND(tp.TotalAnswersPerTag * 1.0 / tp.PostCount, 2) ELSE 0 END AS AverageAnswersPerPost,
//        CASE WHEN tp.PostCount > 0 THEN ROUND(tp.TotalCommentsPerTag * 1.0 / tp.PostCount, 2) ELSE 0 END AS AverageCommentsPerPost,
//        CASE WHEN tp.PostCount > 0 THEN ROUND(tp.TotalFavoritesPerTag * 1.0 / tp.PostCount, 2) ELSE 0 END AS AverageFavoritesPerPost
// FROM TagPopularity tp ORDER BY tp.TotalViewsPerTag DESC, tp.Tag;
//
// `pt.PostId = up.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q25771(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, answer_count, comment_count, favorite_count, origid, .. } = &db.post;
    let ups = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(answer_count.opt()).and(comment_count).and(favorite_count.opt())))
        .fold([0i64; 4], |a, (((w, n), c), f)| [a[0] + w.unwrap_or(0), a[1] + n.unwrap_or(0), a[2] + c, a[3] + f.unwrap_or(0)]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pt = rel(drain(db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list))));
    type R = (Id<Post>, Str);
    let g = (&pt)
        .group_by(Same::<R>::new().map(|(_, t): R| t))
        .select(Same::<R>::new().map(|(p, _): R| p).and(Same::<R>::new().map(|(p, _): R| p).select(origid).select(&uidx).select(&ups).opt()))
        .buf_fold(|rows| {
            let mut s = [0i64; 4];
            for r in rows.iter() {
                let a = r.1.unwrap_or([0; 4]);
                for i in 0..4 {
                    s[i] += a[i];
                }
            }
            (distinct_some(rows.iter().map(|r| Some(r.0))), s)
        });
    let r2 = |x: i64, n: i64| V::F((x as f64 / n as f64 * 100.0).round() / 100.0);
    rows(drain(&g).into_iter().map(|(t, (n, s))| {
        let mut f = vec![V::S(t), V::I(n)];
        f.extend(s.map(V::I));
        f.extend(s.map(|x| r2(x, n)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.Tags ORDER BY p.ViewCount DESC) AS TagRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u JOIN Votes v ON u.Id = v.UserId WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// PostHistoryAnalysis AS (SELECT ph.PostId, COUNT(DISTINCT ph.UserId) AS EditorsCount, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseOpenCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (24) THEN 1 END) AS EditSuggestionCount FROM PostHistory ph WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.Score, rp.OwnerDisplayName, mau.DisplayName AS ActiveUserDisplayName, mau.VoteCount, mau.Upvotes, mau.Downvotes,
//        pha.EditorsCount, pha.CloseOpenCount, pha.EditSuggestionCount
// FROM RankedPosts rp JOIN MostActiveUsers mau ON rp.OwnerDisplayName = mau.DisplayName LEFT JOIN PostHistoryAnalysis pha ON rp.PostId = pha.PostId WHERE rp.TagRank = 1 ORDER BY rp.ViewCount DESC LIMIT 50;
fn q26803(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, owner_user, creation_date, tags_str, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).with(owner_user).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let Vote { user, vote_type_id, creation_date: vd, .. } = &db.vote;
    let mau = db.vote.with(vd.ge(add_years(t0, -1))).group_by(user).select(vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mr = rel(drain(&mau));
    let by_name: HashIdx<Str, (Id<User>, [i64; 3])> = (&mr).map(|(u, _)| u).select(&db.user.display_name).inv().select(&mr).collect();
    let PostHistory { post, post_history_type_id, user: hu, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.with(hd.ge(add_years(t0, -1))).group_by(post).select(hu.opt().and(post_history_type_id)).buf_fold(|rows| {
        [distinct_some(rows.iter().map(|r| r.0)), rows.iter().filter(|r| matches!(r.1, 10 | 11)).count() as i64, rows.iter().filter(|r| r.1 == 24).count() as i64]
    });
    let v = drain((&tp).select(owner_user.select(&db.user.display_name).select(&by_name).and((&pha).opt())));
    let v = top_n(v, |&(p, ((u, _), _))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p, u)
    }, 50);
    rows(v.into_iter().map(|(p, ((u, a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "views", "answers", "score", "owner"]);
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        f.extend(match h {
            Some(h) => h.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id, U.DisplayName, U.Reputation, U.CreationDate, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// ClosedPosts AS (SELECT P.Id AS PostId, P.Title, PH.CreationDate AS ClosedDate, P.OwnerUserId, PH.Comment AS CloseReason FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId AND PH.PostHistoryTypeId = 10
//     WHERE P.PostTypeId = 1),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS TotalBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostDetails AS (SELECT P.Id, P.Title, P.Score, P.ViewCount, COALESCE(CP.ClosedDate, TIMESTAMP '2024-10-01 12:34:56') AS ClosedDate, COALESCE(CP.CloseReason, 'Not Closed') AS CloseReason, U.Reputation,
//        P.OwnerUserId FROM Posts P LEFT JOIN ClosedPosts CP ON P.Id = CP.PostId JOIN UserReputation U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// FinalResults AS (SELECT PD.Title, PD.Score, PD.ViewCount, PD.ClosedDate, PD.CloseReason, U.Reputation, UB.TotalBadges,
//        RANK() OVER (PARTITION BY CASE WHEN PD.ViewCount > 1000 THEN 'High' WHEN PD.ViewCount BETWEEN 100 AND 1000 THEN 'Medium' ELSE 'Low' END ORDER BY PD.Score DESC) AS ScoreRank
//     FROM PostDetails PD JOIN UserBadges UB ON PD.OwnerUserId = UB.UserId JOIN Users U ON PD.OwnerUserId = U.Id)
// SELECT Title, Score, ViewCount, ClosedDate, CloseReason, Reputation, TotalBadges, ScoreRank FROM FinalResults WHERE ScoreRank <= 5 ORDER BY ClosedDate DESC, Score DESC;
//
// ReputationRank is never read.
fn q2640(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let asked_closes = Ident::<Post>::new().with(post_type_id.eq(1)).select(closes);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).select(asked_closes.opt()));
    let cat = |p: Id<Post>| match view_count.get(p) {
        Some(w) if w > 1000 => 0,
        Some(w) if w >= 100 => 1,
        _ => 2,
    };
    let r = per_group(ranked(v, |&(p, _)| (cat(p), Reverse(score.get(p).unwrap())), false), |&(p, _)| cat(p));
    let fr = rel(r.into_iter().filter(|x| x.1 <= 5).collect());
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type R = ((Id<Post>, Option<Id<PostHistory>>), i64);
    let v = drain((&fr).select(Same::<R>::new().and(Same::<R>::new().map(|((p, _), _): R| p).select(owner_user.select(Ident::<User>::new().and(&bc))))));
    rows(v.into_iter().map(|(_, (((p, h), r), (u, b)))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend(match h {
            Some(h) => [V::T(db.post_history.creation_date.get(h).unwrap()), db.post_history.comment.get(h).map_or(V::S("Not Closed"), V::S)],
            None => [V::T(t0), V::S("Not Closed")],
        });
        f.extend([user_col(db, u, "rep"), V::I(b), V::I(r)]);
        row(f)
    }))
}

// Rewritten (rewrites/8453.sql): the ROW_NUMBER order is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.Id) AS Rank,
//        COUNT(c.Id) AS CommentCount, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName, pt.Name),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' GROUP BY u.Id, u.DisplayName HAVING COUNT(p.Id) > 5),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(DISTINCT ph.PostHistoryTypeId) AS EditCount FROM PostHistory ph WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.Score, rp.ViewCount, rp.CommentCount, rp.Upvotes, rp.Downvotes, pah.EditCount, mau.UserId, mau.DisplayName AS MostActiveUser, mau.PostCount, mau.TotalScore
// FROM RankedPosts rp LEFT JOIN PostHistorySummary pah ON rp.PostId = pah.PostId LEFT JOIN MostActiveUsers mau ON mau.PostCount = (SELECT MAX(PostCount) FROM MostActiveUsers)
// WHERE rp.Rank <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// Rank reads only base columns, so the top posts are picked first. The second ON names only mau, so the posts are crossed with the users at the maximum PostCount.
fn q8453(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(hd.ge(add_months(t0, -6))).group_by(post).select(post_history_type_id).buf_fold(|rows| distinct_some(rows.iter().map(|&t| Some(t))));
    let mau = db.user.with((&db.user.creation_date).lt(add_months(t0, -1))).group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let most = (&mau).filt(|a| a[0] > 5).fold_flat(i64::MIN, |m, a| m.max(a[0]));
    let top_users = left_all(drain((&mau).filt(move |a| a[0] > 5 && a[0] == most)));
    let mut v = Vec::new();
    (&rp).and((&phs).opt()).cross(&top_users).drive(|(p, _), ((a, e), m)| v.push((p, a, e, m)));
    rows(v.into_iter().map(|(p, a, e, m)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(oint(e));
        f.extend(match m {
            Some((u, s)) => vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(s[0]), V::I(s[1])],
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserRanked AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostsWithAnswers AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(p.AnswerCount, 0) AS AnswerCount, u.DisplayName AS OwnerDisplayName, p.Score, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, p.AnswerCount),
// PopularPosts AS (SELECT p.PostId, p.Title, p.CreationDate, p.AnswerCount, p.OwnerDisplayName, p.Score, p.CommentCount,
//        CASE WHEN p.Score > 5 THEN 'Hot' WHEN p.Score BETWEEN 1 AND 5 THEN 'Moderate' ELSE 'Cold' END AS Popularity FROM PostsWithAnswers p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostHistoryWithCloseReason AS (SELECT ph.PostId, ph.CreationDate AS HistoryDate, p.Title, p.OwnerDisplayName, ph.Comment AS CloseReason FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11)),
// FinalResult AS (SELECT pp.Title, pp.OwnerDisplayName, pp.Score, pp.AnswerCount, pp.CommentCount, pp.Popularity, COALESCE(ph.CloseReason, 'N/A') AS LastCloseReason
//     FROM PopularPosts pp LEFT JOIN PostHistoryWithCloseReason ph ON pp.PostId = ph.PostId)
// SELECT f.Title, f.OwnerDisplayName, f.Score, f.AnswerCount, f.CommentCount, f.Popularity, f.LastCloseReason, ur.ReputationRank
// FROM FinalResult f JOIN UserRanked ur ON f.OwnerDisplayName = ur.DisplayName WHERE ur.ReputationRank <= 10 ORDER BY f.Score DESC, f.CommentCount DESC;
fn q1472(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, answer_count, .. } = &db.post;
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let by_name: HashIdx<Str, (Id<User>, i64)> = (&tu).map(|(u, _)| u).select(&db.user.display_name).inv().select(&tu).collect();
    let pp = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let cc = pp.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let v = drain((&cc).and(closes.opt()).and(owner_user.select(&db.user.display_name).select(&by_name)));
    rows(v.into_iter().map(|(p, ((c, h), (_, r)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "owner", "score"]);
        f.extend([V::I(answer_count.get(p).unwrap_or(0)), V::I(c), V::S(if s > 5 { "Hot" } else if s >= 1 { "Moderate" } else { "Cold" })]);
        f.push(h.and_then(|h| db.post_history.comment.get(h)).map_or(V::S("N/A"), V::S));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId = 1 THEN P.Score ELSE 0 END) AS TotalQuestionScore,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN P.Score ELSE 0 END) AS TotalAnswerScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// ActiveUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, Questions, Answers, TotalQuestionScore, TotalAnswerScore FROM UserStats WHERE PostCount > 0),
// TopPosters AS (SELECT *, RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM ActiveUsers),
// TopQuestions AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TopQuestionCount FROM Users U INNER JOIN Posts P ON U.Id = P.OwnerUserId WHERE P.PostTypeId = 1 GROUP BY U.Id, U.DisplayName),
// TopAnswers AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TopAnswerCount FROM Users U INNER JOIN Posts P ON U.Id = P.OwnerUserId WHERE P.PostTypeId = 2 GROUP BY U.Id, U.DisplayName)
// SELECT T.UserId, T.DisplayName, T.Reputation, T.PostCount, T.Questions, T.Answers, T.TotalQuestionScore, T.TotalAnswerScore, COALESCE(Q.TopQuestionCount, 0) AS TopQuestionCount,
//        COALESCE(A.TopAnswerCount, 0) AS TopAnswerCount
// FROM TopPosters T LEFT JOIN TopQuestions Q ON T.UserId = Q.UserId LEFT JOIN TopAnswers A ON T.UserId = A.UserId WHERE T.PostRank <= 10 ORDER BY T.PostCount DESC, T.Reputation DESC;
fn q7053(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score))).fold([0i64; 5], |a, (t, s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { s } else { 0 }, a[4] + if t == 2 { s } else { 0 }]
    });
    let tq = db.post.with(post_type_id.eq(1)).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let ta = db.post.with(post_type_id.eq(2)).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let r = ranked(drain((&us).filt(|a| a[0] > 0).and((&tq).opt()).and((&ta).opt())), |&(_, ((a, _), _))| Reverse(a[0]), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, q), n)), _)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(q.unwrap_or(0)), V::I(n.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, us.DisplayName AS OwnerDisplayName, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount, us.TotalPosts, us.GoldBadges, us.SilverBadges,
//        us.BronzeBadges FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId WHERE rp.UserPostRank <= 5)
// SELECT pd.Title, pd.CreationDate, pd.OwnerDisplayName, pd.CommentCount, pd.UpvoteCount, pd.DownvoteCount, pd.TotalPosts, pd.GoldBadges, pd.SilverBadges, pd.BronzeBadges
// FROM PostDetails pd WHERE pd.CommentCount > 0 ORDER BY pd.UpvoteCount DESC LIMIT 10;
//
// UserPostRank reads only base columns, so each owner's five newest posts are picked first; UserStats is computed for their owners, the only users the join reads.
fn q2851(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (_, c)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&rp).filt(|a| a[0] > 0).and(owner_user.select(Ident::<User>::new().and(&pc).and(&us))));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[1]), p), 10);
    rows(v.into_iter().map(|(p, (a, ((u, n), b)))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        f.push(V::I(n));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostRankings AS (SELECT P.Id AS PostId, P.Title, P.Score, P.CreationDate, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS ScoreRank,
//        ROW_NUMBER() OVER (ORDER BY P.CreationDate DESC) AS RecentPostRank FROM Posts P WHERE P.Score IS NOT NULL),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, COUNT(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseCount FROM PostHistory PH GROUP BY PH.PostId, PH.CreationDate),
// FinalOutput AS (SELECT U.DisplayName, U.BadgeCount, U.GoldBadges, U.SilverBadges, U.BronzeBadges, P.Title, P.Score, P.CreationDate, COALESCE(CP.CloseCount, 0) AS TotalCloseCount,
//        CASE WHEN COALESCE(CP.CloseCount, 0) > 0 THEN 'Closed' ELSE 'Active' END AS PostStatus, P.ScoreRank, P.RecentPostRank
//     FROM UserBadges U JOIN PostRankings P ON U.UserId = P.PostId LEFT JOIN ClosedPosts CP ON P.PostId = CP.PostId)
// SELECT DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, Title, Score, CreationDate, TotalCloseCount, PostStatus,
//        CASE WHEN ScoreRank = 1 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory, CASE WHEN RecentPostRank <= 10 THEN 'Recent Top 10' ELSE 'Older Post' END AS RecencyCategory
// FROM FinalOutput WHERE (TotalCloseCount IS NULL OR TotalCloseCount < 2) ORDER BY Score DESC, CreationDate DESC;
//
// `U.UserId = P.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q21486(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, origid, .. } = &db.post;
    let sr = per_group(ranked(drain(db.post.select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |&(_, t)| t);
    let sr = rel(sr.into_iter().map(|((p, _), r)| (p, r)).collect());
    let score_rank: HashIdx<Id<Post>, (Id<Post>, i64)> = (&sr).map(|(p, _)| p).inv().select(&sr).collect();
    let recent = top_n(drain(creation_date), |&(p, d)| (Reverse(d), p), 10);
    let recent: MatSet<Id<Post>> = rel(recent.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.group_by(post.and(hd)).select(post_history_type_id).fold(0i64, |n, t| n + matches!(t, 10 | 11) as i64);
    let cr = rel(drain(&cp));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&cr).map(|((p, _), _)| p).inv().select(&cr).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain(
        db.post
            .select(origid.select(&uidx).select(Ident::<User>::new().and(&ub)).and(&score_rank).and(Ident::<Post>::new().with(&recent).opt()).and((&by_post).map(|(_, n)| n).opt()))
            .filt(|(_, n): ((((Id<User>, [i64; 4]), (Id<Post>, i64)), Option<Id<Post>>), Option<i64>)| n.unwrap_or(0) < 2),
    );
    rows(v.into_iter().map(|(p, ((((u, b), (_, r)), rc), n))| {
        let n = n.unwrap_or(0);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["title", "score", "created"]));
        f.extend([V::I(n), V::S(if n > 0 { "Closed" } else { "Active" }), V::S(if r == 1 { "Top Post" } else { "Regular Post" })]);
        f.push(V::S(if rc.is_some() { "Recent Top 10" } else { "Older Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '7 days'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT b.Id) AS TotalBadges, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Comments c ON u.Id = c.UserId GROUP BY u.Id, u.DisplayName),
// RecentPostLinks AS (SELECT pl.PostId, pl.RelatedPostId FROM PostLinks pl WHERE pl.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT ups.UserId, ups.DisplayName, ups.TotalBounty, ups.TotalBadges, ups.TotalComments, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount,
//        COALESCE(cp.CloseCount, 0) AS CloseCount, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostsCount
// FROM UserStats ups JOIN RankedPosts rp ON ups.UserId = rp.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId LEFT JOIN RecentPostLinks pl ON rp.PostId = pl.PostId
// WHERE ups.TotalBadges > 0 AND rp.RankScore <= 5
// GROUP BY ups.UserId, ups.DisplayName, ups.TotalBounty, ups.TotalBadges, ups.TotalComments, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, cp.CloseCount
// ORDER BY rp.Score DESC, ups.TotalBounty DESC;
//
// `ups.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids; UserStats is computed for those users alone. The outer GROUP BY has one row per (user, post).
fn q2234(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { owner_user, creation_date, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(t0, -7))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let users: MatSet<Id<User>> = (&tp).select(origid).select(&uidx).collect();
    let bounty = votes_by(db).select((&db.vote.bounty_amount).opt());
    let us = (&users).group_by(Ident::<User>::new()).select(bounty.opt().and(badges_of(db).opt()).and(comments_by(db).opt())).buf_fold(|rows| {
        (rows.iter().map(|r| r.0 .0.flatten().unwrap_or(0)).sum::<i64>(), distinct_some(rows.iter().map(|r| r.0 .1)), distinct_some(rows.iter().map(|r| r.1)))
    });
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let PostLink { post: lp, related_post_id, creation_date: ld, .. } = &db.post_link;
    let rl = db.post_link.with(ld.gt(add_months(t0, -1))).group_by(lp).select(related_post_id).buf_fold(|rows| distinct_some(rows.iter().map(|&r| Some(r))));
    let v = drain((&tp).select(origid.select(&uidx).select(Ident::<User>::new().and((&us).filt(|(_, b, _): (i64, i64, i64)| b > 0))).and((&cp).opt()).and((&rl).opt())));
    rows(v.into_iter().map(|(p, (((u, (b, n, c)), k), l))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(n), V::I(c)]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views", "answers", "comments"]));
        f.extend([V::I(k.unwrap_or(0)), V::I(l.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, AVG(P.Score) AS AvgScore, SUM(P.ViewCount) AS TotalViews, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers, RANK() OVER (ORDER BY COUNT(P.Id) DESC) AS PostRank FROM Posts P GROUP BY P.OwnerUserId),
// UserPostBadgeStats AS (SELECT U.UserId, U.DisplayName, COALESCE(UB.TotalBadges, 0) AS TotalBadges, COALESCE(PS.TotalPosts, 0) AS TotalPosts, PS.AvgScore, PS.TotalViews, PS.TotalQuestions, PS.TotalAnswers
//     FROM (SELECT DISTINCT UserId, DisplayName FROM UserBadges) U LEFT JOIN UserBadges UB ON U.UserId = UB.UserId LEFT JOIN PostStats PS ON U.UserId = PS.OwnerUserId),
// RankedUsers AS (SELECT DisplayName, TotalBadges, TotalPosts, AvgScore, TotalViews, TotalQuestions, TotalAnswers, RANK() OVER (ORDER BY TotalBadges DESC, TotalPosts DESC) AS BadgePostRank FROM UserPostBadgeStats)
// SELECT R.DisplayName, R.TotalBadges, R.TotalPosts, R.AvgScore, R.TotalViews, (R.TotalAnswers * 1.0 / NULLIF(R.TotalQuestions, 0)) AS AnswerToQuestionRatio, R.BadgePostRank
// FROM RankedUsers R WHERE R.TotalPosts > 50 AND (R.TotalBadges > 3 OR R.AvgScore > 10) ORDER BY R.BadgePostRank, AnswerToQuestionRatio DESC LIMIT 10;
//
// PostRank is never read. UserBadges has one row per user, so the DISTINCT and the self-join keep one row per user.
fn q20948(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()))).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + (t == 1) as i64, a[5] + (t == 2) as i64]
    });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain((&bc).and((&ps).opt())), |&(_, (b, p))| (Reverse(b), Reverse(p.map_or(0, |a| a[0]))), false);
    let v = drain(rel(v).filt(|((_, (b, p)), _)| {
        let a = p.unwrap_or([0; 6]);
        a[0] > 50 && (b > 3 || a[1] as f64 / a[0] as f64 > 10.0)
    }));
    let ratio = |a: [i64; 6]| if a[4] == 0 { None } else { Some(a[5] as f64 / a[4] as f64) };
    let v = top_n(v.into_iter().map(|x| x.1).collect(), |&((u, (_, p)), r)| {
        let q = ratio(p.unwrap());
        (r, q.is_none(), Reverse(q.map(fkey)), u)
    }, 10);
    rows(v.into_iter().map(|((u, (b, p)), r)| {
        let a = p.unwrap();
        row(vec![user_col(db, u, "name"), V::I(b), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), ofloat(ratio(a)), V::I(r)])
    }))
}

// WITH RECURSIVE UserBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount, RANK() OVER (ORDER BY COUNT(*) DESC) AS BadgeRank FROM Badges GROUP BY UserId),
// PostStatistics AS (SELECT p.Id AS PostId, p.OwnerUserId, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
//        COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.OwnerUserId),
// RankedPosts AS (SELECT ps.PostId, ps.OwnerUserId, ps.UpVotes, ps.DownVotes, ps.CommentCount, (ps.UpVotes - ps.DownVotes) AS Score,
//        RANK() OVER (ORDER BY (ps.UpVotes - ps.DownVotes + ps.TotalBounty) DESC) AS PostRank FROM PostStatistics ps),
// UserPostStats AS (SELECT u.Id AS UserId, COALESCE(SUM(CASE WHEN rp.OwnerUserId = u.Id THEN 1 ELSE 0 END), 0) AS TotalPosts, COALESCE(SUM(rp.UpVotes), 0) AS TotalUpVotes,
//        COALESCE(SUM(rp.DownVotes), 0) AS TotalDownVotes, MAX(ub.BadgeRank) AS HighestBadgeRank FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId GROUP BY u.Id)
// SELECT u.DisplayName, ups.TotalPosts, ups.TotalUpVotes, ups.TotalDownVotes, ups.HighestBadgeRank,
//        (CASE WHEN ups.HighestBadgeRank IS NULL THEN 'No Badges' WHEN ups.HighestBadgeRank = 1 THEN 'Gold' WHEN ups.HighestBadgeRank = 2 THEN 'Silver' ELSE 'Bronze' END) AS HighestBadgeRankName
// FROM Users u LEFT JOIN UserPostStats ups ON u.Id = ups.UserId WHERE (ups.TotalPosts > 10 OR ups.TotalUpVotes > 100) ORDER BY ups.TotalPosts DESC, ups.TotalUpVotes DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. PostRank is never read.
fn q32850(db: &'static So) -> String {
    let ubc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let br = rel(ranked(drain(&ubc), |&(_, n)| Reverse(n), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let badge_rank: HashIdx<Id<User>, (Id<User>, i64)> = (&br).map(|(u, _)| u).inv().select(&br).collect();
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&ps).opt()).fold([0i64; 3], |a, p| match p {
        Some(p) => [a[0] + 1, a[1] + p[0], a[2] + p[1]],
        None => a,
    });
    let v = drain((&ups).filt(|a| a[0] > 10 || a[1] > 100).and((&badge_rank).opt()));
    rows(v.into_iter().map(|(u, (a, r))| {
        let r = r.map(|x| x.1);
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), oint(r), V::S(match r {
            None => "No Badges",
            Some(1) => "Gold",
            Some(2) => "Silver",
            _ => "Bronze",
        })])
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, MAX(B.Date) AS LastBadgeDate FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(P.ViewCount) AS TotalViews, AVG(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS AverageScore, COUNT(CASE WHEN P.ClosedDate IS NOT NULL THEN 1 END) AS ClosedPostCount
//     FROM Posts P GROUP BY P.OwnerUserId),
// VoteDetails AS (SELECT V.UserId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Votes V GROUP BY V.UserId),
// CombinedStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(BC.BadgeCount, 0) AS BadgeCount, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.QuestionCount, 0) AS QuestionCount,
//        COALESCE(PS.AnswerCount, 0) AS AnswerCount, COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(PS.ClosedPostCount, 0) AS ClosedPostCount, COALESCE(VD.VoteCount, 0) AS VoteCount,
//        COALESCE(VD.Upvotes, 0) AS Upvotes, COALESCE(VD.Downvotes, 0) AS Downvotes, ROW_NUMBER() OVER (ORDER BY COALESCE(PS.TotalPosts, 0) DESC, COALESCE(BC.BadgeCount, 0) DESC) AS Rank
//     FROM Users U LEFT JOIN UserBadgeCounts BC ON U.Id = BC.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId LEFT JOIN VoteDetails VD ON U.Id = VD.UserId)
// SELECT UserId, DisplayName, BadgeCount, TotalPosts, QuestionCount, AnswerCount, TotalViews, ClosedPostCount, VoteCount, Upvotes, Downvotes, Rank FROM CombinedStats WHERE Rank <= 10 ORDER BY Rank;
fn q331(db: &'static So) -> String {
    let Post { post_type_id, view_count, closed_date, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(closed_date.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, w), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + c.is_some() as i64],
        None => a,
    });
    let vd = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let v = top_n(drain((&bc).and(&ps).and((&vd).opt())), |&(u, ((b, a), _))| (Reverse(a[0]), Reverse(b), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, ((b, a), d)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(b));
        f.extend(a.map(V::I));
        f.extend(d.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, P.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.PostTypeId = 1 AND P.Score > 5),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(P.ViewCount) AS TotalViews, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// CloseReasons AS (SELECT PH.PostId, C.Name AS CloseReason, COUNT(PH.Id) AS CloseCount FROM PostHistory PH JOIN CloseReasonTypes C ON CAST(PH.Comment AS INT) = C.Id WHERE PH.PostHistoryTypeId = 10
//     GROUP BY PH.PostId, C.Name),
// FinalStats AS (SELECT U.UserId, U.DisplayName, U.Reputation, U.QuestionCount, U.TotalViews, COALESCE(CR.CloseReason, 'No Close Reason') AS CloseReason, COALESCE(CR.CloseCount, 0) AS CloseCount
//     FROM UserStats U LEFT JOIN CloseReasons CR ON U.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = CR.PostId LIMIT 1))
// SELECT R.PostId, R.Title, R.Score, R.ViewCount, F.DisplayName, F.Reputation, F.QuestionCount, F.TotalViews, F.CloseReason, F.CloseCount,
//        CASE WHEN F.CloseCount > 0 THEN 'Closed Posts' ELSE 'Active Posts' END AS PostStatus
// FROM RankedPosts R JOIN FinalStats F ON R.OwnerUserId = F.UserId WHERE R.PostRank = 1 ORDER BY R.Score DESC, R.ViewCount DESC;
//
// The correlated LIMIT 1 looks a post up by its id, so it is the post's owner. UserStats is computed for the owners of the picked questions, the only users the join reads.
fn q659(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(5))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let asked = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let us = (&owners).group_by(Ident::<User>::new()).select(asked.select(view_count.opt().and(votes_of(db).opt())).opt()).fold([0i64; 2], |a, x| match x {
        Some((w, _)) => [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0)],
        None => a,
    });
    let qc = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).opt())
        .fold(0i64, |n, p| n + p.is_some() as i64);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .select(Ident::<PostHistory>::new())
        .fold(0i64, |n, _| n + 1);
    let crr = rel(drain(&cr));
    let by_owner: HashIdx<Id<User>, ((Id<Post>, Str), i64)> = (&crr).map(|((p, _), _)| p).select(owner_user).inv().select(&crr).collect();
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&qc).and(&us).and((&by_owner).opt()))));
    rows(v.into_iter().map(|(p, (((u, q), a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(q), nullable(a[1], a[0])]);
        let (r, n) = c.map_or(("No Close Reason", 0), |((_, r), n)| (r, n));
        f.extend([V::S(r), V::I(n), V::S(if n > 0 { "Closed Posts" } else { "Active Posts" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Location, SUM(CASE WHEN p.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalPosts, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Location)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.Location, us.TotalPosts, rs.PostId, rs.Title, rs.CreationDate, rs.Score, rs.ViewCount, rs.CommentCount, rs.UpVotes, rs.DownVotes,
//        CASE WHEN us.Reputation >= 1000 THEN 'Experienced' ELSE 'Novice' END AS UserLevel, EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = us.UserId AND p.Score > 0) AS HasPositivePosts
// FROM UserStatistics us JOIN RankedPosts rs ON us.UserId = rs.PostId WHERE us.TotalPosts > 0 AND (us.Reputation > 500 OR rs.CommentCount > 5) ORDER BY us.Reputation DESC, rs.Score DESC LIMIT 100;
//
// `us.UserId = rs.PostId` joins a user id to a post id, so it goes through the raw ids; UserStatistics is computed for those users alone. UserPostRank is never read.
fn q4968(db: &'static So) -> String {
    let Post { creation_date, score, origid, .. } = &db.post;
    let recent: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).collect();
    let rs = (&recent)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let users: MatSet<Id<User>> = (&recent).select(origid).select(&uidx).collect();
    let us = (&users).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).opt())).fold(0i64, |n, (p, _)| n + p.is_some() as i64);
    let positive: MatSet<Id<User>> = db.post.with(score.gt(0)).select(&db.post.owner_user).collect();
    let v = drain(
        (&rs)
            .and(origid.select(&uidx).select(Ident::<User>::new().and((&us).filt(|n| n > 0)).and(Ident::<User>::new().with(&positive).opt())))
            .filt(|(a, ((u, _), _)): ([i64; 3], ((Id<User>, i64), Option<Id<User>>))| db.user.reputation.get(u).unwrap() > 500 || a[0] > 5),
    );
    let v = top_n(v, |&(p, (_, ((u, _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (a, ((u, n), pos)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(ostr(db.user.location.get(u)));
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend(a.map(V::I));
        f.extend([V::S(if db.user.reputation.get(u).unwrap() >= 1000 { "Experienced" } else { "Novice" }), V::B(pos.is_some())]);
        row(f)
    }))
}

// WITH PostTagStatistics AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, p.CreationDate, STRING_AGG(DISTINCT t.TagName, ', ') AS TagsList, COUNT(DISTINCT mh.UserId) AS TotalModerationHistory,
//        COALESCE(SUM(CASE WHEN mh.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS TotalClosures, COALESCE(SUM(CASE WHEN mh.PostHistoryTypeId = 11 THEN 1 ELSE 0 END), 0) AS TotalReopens
//     FROM Posts p LEFT JOIN Tags t ON POSITION(t.TagName IN p.Tags) > 0 LEFT JOIN PostHistory mh ON mh.PostId = p.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.ViewCount, p.AnswerCount, p.CreationDate),
// UserPostEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(p.ViewCount) AS TotalViews, SUM(CASE WHEN p.AnswerCount > 0 THEN 1 ELSE 0 END) AS TotalAnswered,
//        SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS HighEngagementPosts FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// BenchmarkingResults AS (SELECT pts.PostId, pts.Title, pts.TagsList, ups.DisplayName AS UserDisplayName, ups.TotalPosts, ups.TotalViews, ups.TotalAnswered, ups.HighEngagementPosts,
//        pts.TotalModerationHistory, pts.TotalClosures, pts.TotalReopens FROM PostTagStatistics pts JOIN UserPostEngagement ups ON pts.PostId = ups.TotalPosts ORDER BY pts.ViewCount DESC, ups.TotalPosts DESC)
// SELECT BenchmarkingResults.* FROM BenchmarkingResults WHERE TotalViews > 500 ORDER BY TotalAnswered DESC;
//
// `pts.PostId = ups.TotalPosts` joins a post id to a count, so it goes through the raw post id; PostTagStatistics is computed for the posts that join. The DISTINCT tag list is
// sorted by name; the query leaves its order open.
fn q25546(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, creation_date, view_count, answer_count, tags_str, origid, .. } = &db.post;
    let upe = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(t0, -1)))).select(view_count.opt().and(answer_count.opt())))
        .fold([0i64; 5], |a, (w, n)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + (n.unwrap_or(0) > 0) as i64, a[4] + (w.unwrap_or(0) > 100) as i64]);
    let ur = rel(drain(&upe));
    let by_count: HashIdx<i64, (Id<User>, [i64; 5])> = (&ur).map(|(_, a)| a[0]).inv().select(&ur).collect();
    let pts: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).with(origid.select(&by_count)).collect();
    let strs: MatSet<Str> = (&pts).select(tags_str).collect();
    let hit: HashIdx<Str, Id<Tag>> = (&strs).select_where((&db.tag.tag_name).inv(), |s: Str, n: Str| s.contains(n)).collect();
    let PostHistory { user: hu, post_history_type_id, .. } = &db.post_history;
    let st = (&pts)
        .group_by(Ident::<Post>::new())
        .select(tags_str.select(&hit).select(&db.tag.tag_name).opt().and(history_of(db).select(hu.opt().and(post_history_type_id)).opt()))
        .buf_fold(|rows| {
            let mut names: Vec<Str> = rows.iter().filter_map(|r| r.0).collect();
            names.sort_unstable();
            names.dedup();
            let list: Option<&'static str> = if names.is_empty() { None } else { Some(Box::leak(names.join(", ").into_boxed_str())) };
            let t = |k: i64| rows.iter().filter(|r| r.1.map(|h| h.1) == Some(k)).count() as i64;
            (list, distinct_some(rows.iter().map(|r| r.1.and_then(|h| h.0))), t(10), t(11))
        });
    let v = drain((&st).and(origid.select(&by_count).select(Same::<(Id<User>, [i64; 5])>::new().with(Same::<(Id<User>, [i64; 5])>::new().map(|(_, a)| a).filt(|a: [i64; 5]| a[1] > 0 && a[2] > 500)))));
    rows(v.into_iter().map(|(p, ((l, m, c, r), (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([ostr(l), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::I(a[4]), V::I(m), V::I(c), V::I(r)]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Owner, pt.Name AS PostType, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CAST('2024-10-01' AS DATE) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, pt.Name),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, u.Reputation FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation
//     HAVING SUM(CASE WHEN b.Class IN (1, 2, 3) THEN 1 ELSE 0 END) > 0),
// PostEngagement AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Owner, rp.PostType, rp.CommentCount, rp.UpVotes, rp.DownVotes, tu.UserId, tu.DisplayName AS TopUser,
//        ROW_NUMBER() OVER (PARTITION BY rp.PostId ORDER BY rp.UpVotes DESC) AS VoteRank FROM RecentPosts rp JOIN TopUsers tu ON rp.Owner = tu.DisplayName)
// SELECT pe.PostId, pe.Title, pe.CreationDate, pe.Owner, pe.PostType, pe.CommentCount, (pe.UpVotes - pe.DownVotes) AS NetVotes, COALESCE(pe.TopUser, 'No Top Contributor') AS TopContributor
// FROM PostEngagement pe WHERE pe.VoteRank = 1 ORDER BY NetVotes DESC, pe.CreationDate DESC LIMIT 10;
//
// VoteRank keeps one of the TopUsers rows that share the owner's name; they differ only in UserId, which is not projected, so the row is the post joined to that name.
fn q1597(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let named: MatSet<Str> = db.user.with(badges_of(db).select(&db.badge.class).filt(|c: i64| matches!(c, 1 | 2 | 3))).select(&db.user.display_name).collect();
    let rp = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .with(owner_user.select(&db.user.display_name).with(&named))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain(&rp);
    let v = top_n(v, |&(p, a)| (Reverse(a[1] - a[2]), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "type"]);
        f.extend([V::I(a[0]), V::I(a[1] - a[2])]);
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, UpVotes, DownVotes, ReputationRank FROM UserActivity WHERE ReputationRank <= 10),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(P.Score) AS TotalScore, MAX(P.CreationDate) AS LastPostDate, MIN(P.CreationDate) AS FirstPostDate FROM Posts P GROUP BY P.OwnerUserId),
// ClosedPosts AS (SELECT H.UserId, COUNT(H.PostId) AS TotalClosedPosts FROM PostHistory H WHERE H.PostHistoryTypeId IN (10, 11) GROUP BY H.UserId)
// SELECT T.DisplayName, T.Reputation, T.TotalPosts, T.TotalQuestions, T.TotalAnswers, COALESCE(S.TotalPosts, 0) AS UserPostStatistics, COALESCE(S.TotalScore, 0) AS UserTotalScore,
//        COALESCE(C.TotalClosedPosts, 0) AS UserTotalClosedPosts, CASE WHEN C.TotalClosedPosts > 0 THEN 'Has Closed Posts' ELSE 'No Closed Posts' END AS ClosedPostStatus
// FROM TopUsers T LEFT JOIN PostStatistics S ON T.UserId = S.OwnerUserId LEFT JOIN ClosedPosts C ON T.UserId = C.UserId ORDER BY T.Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the post x vote product is driven for those alone.
fn q1804(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ua = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 2], |a, x| match x {
        Some((t, _)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64],
        None => a,
    });
    let ps = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&ua).and((&ps).opt()).and((&cp).opt()));
    rows(v.into_iter().map(|(u, ((a, s), c))| {
        let s = s.unwrap_or([0; 2]);
        let c = c.unwrap_or(0);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(s[0]), V::I(a[0]), V::I(a[1]), V::I(s[0]), V::I(s[1]), V::I(c), V::S(if c > 0 { "Has Closed Posts" } else { "No Closed Posts" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// BadgeStats AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadgeCount, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadgeCount, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadgeCount
//     FROM Badges b GROUP BY b.UserId),
// FinalStats AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.CreationDate, us.PostCount, us.AnswerCount, us.QuestionCount, us.CommentCount, us.UpvoteCount, us.DownvoteCount,
//        COALESCE(bs.GoldBadgeCount, 0) AS GoldBadgeCount, COALESCE(bs.SilverBadgeCount, 0) AS SilverBadgeCount, COALESCE(bs.BronzeBadgeCount, 0) AS BronzeBadgeCount
//     FROM UserStats us LEFT JOIN BadgeStats bs ON us.UserId = bs.UserId)
// SELECT f.UserId, f.DisplayName, f.Reputation, f.CreationDate, f.PostCount, f.AnswerCount, f.QuestionCount, f.CommentCount, f.UpvoteCount, f.DownvoteCount, f.GoldBadgeCount, f.SilverBadgeCount,
//        f.BronzeBadgeCount, RANK() OVER (ORDER BY f.Reputation DESC) AS ReputationRank FROM FinalStats f ORDER BY ReputationRank FETCH FIRST 100 ROWS ONLY;
//
// The rank and the cut read only Reputation, so the hundred users are picked first and the post x comment x vote product is driven for those alone.
fn q6345(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let top = rel(top_n(r, |&((u, rep), _)| (Reverse(rep), u), 100).into_iter().map(|((u, _), k)| (u, k)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&top).map(|(u, _)| u).inv().select(&top).collect();
    let tu: MatSet<Id<User>> = (&top).map(|(u, _)| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some(((t, _), v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    rows(drain((&us).and(&pc).and(&cc).and((&bs).opt()).and(&by_user)).into_iter().map(|(u, ((((a, p), c), b), (_, k)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1]), V::I(c), V::I(a[2]), V::I(a[3])]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(k));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.AnswerCount, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// TopPosters AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.Views, ROW_NUMBER() OVER (ORDER BY us.Reputation DESC) AS Ranking FROM UserStats us WHERE us.TotalPosts > 0),
// ClusteredHistory AS (SELECT ph.PostId, COUNT(*) AS EditCount, MIN(ph.CreationDate) AS FirstEditDate, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5) GROUP BY ph.PostId)
// SELECT up.UserId, up.DisplayName, up.Reputation, up.TotalPosts, up.Views, COALESCE(rp.PostId, 0) AS TopPostId, COALESCE(rp.Title, 'No Posts') AS TopPostTitle, COALESCE(rp.Score, 0) AS TopPostScore,
//        COALESCE(ch.EditCount, 0) AS TotalEdits, COALESCE(ch.FirstEditDate, '1970-01-01') AS FirstEditDate, COALESCE(ch.LastEditDate, '1970-01-01') AS LastEditDate
// FROM TopPosters up LEFT JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId AND rp.PostRank = 1 LEFT JOIN ClusteredHistory ch ON rp.PostId = ch.PostId WHERE up.Ranking <= 10 ORDER BY up.Reputation DESC;
//
// Only TotalPosts is read from UserStats, so its vote counts are not computed. Ranking reads only Reputation, so the ten posters are picked first.
fn q33546(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, title, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let tp = top_n(drain((&pc).and(&db.user.reputation)), |&(u, (_, r))| (Reverse(r), u), 10);
    let tp = rel(tp.into_iter().map(|(u, (n, _))| (u, n)).collect());
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tr = rel(top);
    let best: HashIdx<Id<User>, (Id<Post>, Id<User>)> = (&tr).map(|(_, u)| u).inv().select(&tr).collect();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ch = db.post_history.with(post_history_type_id.is_in([4, 5])).group_by(&db.post_history.post).select(hd).fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), d| (n + 1, lo.min(d), hi.max(d)));
    type R = (Id<User>, i64);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&best).map(|(p, _)| p).select(Ident::<Post>::new().and((&ch).opt())).opt()))));
    rows(v.into_iter().map(|(_, ((u, n), x))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), user_col(db, u, "uviews")]);
        match x {
            Some((p, h)) => {
                f.extend([V::I(db.post.origid.get(p).unwrap()), title.get(p).map_or(V::S("No Posts"), V::S), V::I(score.get(p).unwrap())]);
                let (e, lo, hi) = h.unwrap_or((0, 0, 0));
                f.extend([V::I(e), V::T(lo), V::T(hi)]);
            }
            None => f.extend([V::I(0), V::S("No Posts"), V::I(0), V::I(0), V::T(0), V::T(0)]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.Score, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId, p.Title, p.Score, p.PostTypeId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(u.UpVotes) AS TotalUpVotes, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, RANK() OVER (ORDER BY SUM(u.Reputation) DESC) AS UserRank FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName HAVING SUM(u.Reputation) > 100),
// PostHistoryStats AS (SELECT p.Id AS PostId, COUNT(ph.Id) AS HistoryCount, MAX(ph.CreationDate) AS LastEdited FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CommentCount, COALESCE(pv.HistoryCount, 0) AS HistoryCount, pv.LastEdited FROM RankedPosts rp LEFT JOIN PostHistoryStats pv ON rp.PostId = pv.PostId
//     WHERE rp.Rank <= 10)
// SELECT fp.PostId, fp.Title, fp.Score, fp.CommentCount, fp.HistoryCount, u.DisplayName, u.TotalUpVotes, u.GoldBadges, u.SilverBadges, u.BronzeBadges
// FROM FilteredPosts fp JOIN TopUsers u ON fp.CommentCount > 5 AND fp.PostId IN (SELECT v.PostId FROM Votes v WHERE v.UserId = u.UserId AND v.VoteTypeId = 2) ORDER BY fp.Score DESC, u.TotalUpVotes DESC;
//
// UserRank is never read. The ROW_NUMBER ties on Score are broken by Id; the query leaves them open.
fn q31131(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = db.post_history.group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let tu = db.user.group_by(Ident::<User>::new()).select((&db.user.up_votes).and(&db.user.reputation).and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 5], |a, ((up, r), c)| {
        [a[0] + up, a[1] + r, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
    });
    let voter_ids = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).select(&db.vote.user).select(Ident::<User>::new().with((&tu).filt(|a| a[1] > 100)));
    let v = drain((&cc).filt(|n| n > 5).and((&hc).opt()).and(voter_ids.select(Ident::<User>::new().and(&tu))));
    rows(v.into_iter().map(|(p, ((c, h), (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(c), V::I(h.unwrap_or(0)), user_col(db, u, "name"), V::I(a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        row(f)
    }))
}

// WITH RecursivePostHistories AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.UserId, ph.CreationDate, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// AggregatedVotes AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// FilteredPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, COALESCE(up.Reputation, 0) AS UserReputation, COALESCE(av.VoteCount, 0) AS TotalVotes, COALESCE(av.UpVoteCount, 0) AS UpVotes,
//        COALESCE(av.DownVoteCount, 0) AS DownVotes FROM Posts p LEFT JOIN Users up ON p.OwnerUserId = up.Id LEFT JOIN AggregatedVotes av ON p.Id = av.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// PostCloseStats AS (SELECT rph.PostId, MAX(CASE WHEN rph.PostHistoryTypeId = 10 THEN rph.CreationDate END) AS ClosedDate, MAX(CASE WHEN rph.PostHistoryTypeId = 11 THEN rph.CreationDate END) AS ReopenedDate
//     FROM RecursivePostHistories rph WHERE rph.HistoryRank = 1 GROUP BY rph.PostId)
// SELECT fp.PostId, fp.Title, fp.Body, fp.CreationDate, fp.UserReputation, fp.TotalVotes, fp.UpVotes, fp.DownVotes, pcs.ClosedDate, pcs.ReopenedDate, (fp.UserReputation / NULLIF(fp.TotalVotes, 0)) AS ReputationPerVote
// FROM FilteredPosts fp LEFT JOIN PostCloseStats pcs ON fp.PostId = pcs.PostId ORDER BY fp.TotalVotes DESC, fp.CreationDate DESC LIMIT 100;
//
// RecursivePostHistories does not recurse. Its ROW_NUMBER ties on the date are broken by Id.
fn q31544(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let fp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let v = drain(db.post_history.with(post_history_type_id.is_in([10, 11, 12])).select(post));
    let latest = top_per(v, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let lr = rel(latest.into_iter().map(|(h, p)| (p, h)).collect());
    let last: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&lr).map(|(p, _)| p).inv().select(&lr).collect();
    let v = drain((&fp).and(owner_user.select(&db.user.reputation).opt()).and((&last).map(|(_, h)| h).opt()));
    let v = top_n(v, |&(p, ((a, _), _))| (Reverse(a[0]), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((a, r), h))| {
        let r = r.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "body", "created"]);
        f.extend([V::I(r), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        let t = h.map(|h| (post_history_type_id.get(h).unwrap(), hd.get(h).unwrap()));
        f.extend([ots(t.filter(|t| t.0 == 10).map(|t| t.1)), ots(t.filter(|t| t.0 == 11).map(|t| t.1))]);
        f.push(if a[0] == 0 { V::Null } else { V::F(r as f64 / a[0] as f64) });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) FILTER (WHERE c.Score > 0) AS PositiveCommentCount, COUNT(c.Id) FILTER (WHERE c.Score < 0) AS NegativeCommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// RecentVoteCounts AS (SELECT Vote.PostId, COUNT(*) FILTER (WHERE vt.Name = 'UpMod') AS UpVoteCount, COUNT(*) FILTER (WHERE vt.Name = 'DownMod') AS DownVoteCount FROM Votes Vote
//     JOIN VoteTypes vt ON Vote.VoteTypeId = vt.Id WHERE Vote.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY Vote.PostId),
// FinalPostStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerUserId, rp.PostRank, COALESCE(rvc.UpVoteCount, 0) AS UpVoteCount, COALESCE(rvc.DownVoteCount, 0) AS DownVoteCount,
//        rp.PositiveCommentCount, rp.NegativeCommentCount FROM RankedPosts rp LEFT JOIN RecentVoteCounts rvc ON rp.PostId = rvc.PostId
//     WHERE (rp.Score > 10 OR rp.PositiveCommentCount > 5) AND (rvc.DownVoteCount IS NULL OR rvc.UpVoteCount > rvc.DownVoteCount))
// SELECT fps.PostId, fps.Title, fps.CreationDate, fps.Score, fps.UpVoteCount, fps.DownVoteCount, fps.PositiveCommentCount, fps.NegativeCommentCount
// FROM FinalPostStats fps JOIN Users u ON fps.OwnerUserId = u.Id LEFT JOIN Badges b ON b.UserId = u.Id WHERE fps.PostRank <= 3 AND (b.Class IN (1, 2) OR b.Id IS NULL) ORDER BY fps.Score DESC, fps.CreationDate DESC;
//
// PostRank reads only base columns, so each owner's three newest posts are picked first; the ownerless posts rank among themselves and are then dropped by the join to Users.
fn q22826(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64],
        None => a,
    });
    let rvc = db.vote.with((&db.vote.creation_date).ge(add_days(t0, -30))).group_by(&db.vote.post).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let fps = (&pc)
        .and(score)
        .and((&rvc).opt())
        .filt(|((c, s), r): (([i64; 2], i64), Option<[i64; 2]>)| (s > 10 || c[0] > 5) && r.map_or(true, |r| r[0] > r[1]));
    let v = drain(fps.and(owner_user.select(badges_of(db).select(&db.badge.class).opt()).filt(|c: Option<i64>| c.map_or(true, |c| c == 1 || c == 2))));
    rows(v.into_iter().map(|(p, (((c, _), r), _))| {
        let r = r.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(r[0]), V::I(r[1]), V::I(c[0]), V::I(c[1])]);
        row(f)
    }))
}

// WITH RECURSIVE UserVotes AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS VoteBalance FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostAggregates AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END), 0) AS ClosureCount, COALESCE(VB.TotalUpVotes, 0) AS TotalUpVotes, COALESCE(VB.TotalDownVotes, 0) AS TotalDownVotes,
//        COALESCE(VB.VoteBalance, 0) AS VoteBalance FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId LEFT JOIN UserVotes VB ON P.OwnerUserId = VB.UserId
//     GROUP BY P.Id, P.Title, P.Score, P.ViewCount, VB.TotalUpVotes, VB.TotalDownVotes, VB.VoteBalance),
// RankedPosts AS (SELECT PA.PostId, PA.Title, PA.Score, PA.ViewCount, PA.CommentCount, PA.ClosureCount, PA.TotalUpVotes, PA.TotalDownVotes, PA.VoteBalance,
//        ROW_NUMBER() OVER (ORDER BY PA.Score DESC, PA.ViewCount DESC) AS Rank FROM PostAggregates PA WHERE PA.Score > 0 AND PA.ClosureCount < 5)
// SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, RP.CommentCount, RP.ClosureCount, RP.TotalUpVotes, RP.TotalDownVotes, RP.VoteBalance,
//        CASE WHEN RP.VoteBalance > 0 THEN 'Positive' WHEN RP.VoteBalance < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteStatus
// FROM RankedPosts RP WHERE RP.Rank <= 100 ORDER BY RP.Score DESC, RP.ViewCount DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q30688(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let pa = db
        .post
        .with(score.gt(0))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + matches!(t, Some(10 | 11)) as i64]);
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&pa).filt(|a| a[1] < 5).and(owner_user.select(&uv).opt()));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 100);
    rows(v.into_iter().map(|(p, (a, u))| {
        let u = u.unwrap_or([0; 2]);
        let b = u[0] - u[1];
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(u[0]), V::I(u[1]), V::I(b), V::S(if b > 0 { "Positive" } else if b < 0 { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Body, p.Score, u.DisplayName AS OwnerName, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        COALESCE((SELECT SUM(v.BountyAmount) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId IN (8, 9)), 0) AS TotalBounty FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostVotes AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerName, rp.CommentCount, pv.UpVotes, pv.DownVotes,
//        CASE WHEN pv.UpVotes = 0 AND pv.DownVotes = 0 THEN 'No votes' WHEN pv.UpVotes > pv.DownVotes THEN 'Popular' ELSE 'Less popular' END AS Popularity,
//        (rp.Score + COALESCE(rp.TotalBounty, 0)) AS TotalScore FROM RecentPosts rp LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId),
// RankedPosts AS (SELECT *, RANK() OVER (ORDER BY TotalScore DESC) AS Rank FROM TopPosts)
// SELECT p.PostId, p.Title, p.OwnerName, p.CommentCount, p.UpVotes, p.DownVotes, p.Popularity, p.Rank,
//        CASE WHEN p.Rank <= 5 THEN 'Top 5 Posts' WHEN p.Rank <= 10 THEN 'Top 10 Posts' ELSE 'Other Posts' END AS PostCategory
// FROM RankedPosts p WHERE p.Popularity = 'Popular' OR p.CommentCount > 5 ORDER BY p.Rank ASC;
fn q24597(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let rp = || db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let tb = rp().group_by(Ident::<Post>::new()).select(bounty.opt()).fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pop = |v: Option<[i64; 2]>| match v {
        Some(v) if v[0] == 0 && v[1] == 0 => "No votes",
        Some(v) if v[0] > v[1] => "Popular",
        _ => "Less popular",
    };
    let v = ranked(drain((&cc).and(&tb).and((&pv).opt())), |&(p, ((_, b), _))| Reverse(score.get(p).unwrap() + b), false);
    let v = drain(rel(v).filt(move |((_, ((c, _), w)), _)| pop(w) == "Popular" || c > 5));
    rows(v.into_iter().map(|(_, ((p, ((c, _), w)), r))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(c), oint(w.map(|w| w[0])), oint(w.map(|w| w[1])), V::S(pop(w)), V::I(r)]);
        f.push(V::S(if r <= 5 { "Top 5 Posts" } else if r <= 10 { "Top 10 Posts" } else { "Other Posts" }));
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.PostHistoryTypeId, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS RevisionNumber
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, SUM(u.Reputation) AS TotalReputation, COUNT(b.Id) AS BadgeCount, COUNT(DISTINCT ph.PostId) AS ClosedPosts FROM Users u
//     LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN RecursivePostHistory ph ON p.Id = ph.PostId GROUP BY u.Id, u.DisplayName),
// PostVoteStats AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        AVG(CASE WHEN VoteTypeId IN (2, 3) THEN VoteTypeId END) AS AvgVoteType FROM Votes GROUP BY PostId),
// CombinedStats AS (SELECT u.UserId, u.DisplayName, u.TotalReputation, u.BadgeCount, u.ClosedPosts, p.Id AS PostId, p.Title, p.CreationDate AS PostCreationDate, COALESCE(v.TotalUpVotes, 0) AS PostUpVotes,
//        COALESCE(v.TotalDownVotes, 0) AS PostDownVotes, COALESCE(v.AvgVoteType, 0) AS PostAvgVote FROM UserReputation u JOIN Posts p ON u.UserId = p.OwnerUserId LEFT JOIN PostVoteStats v ON p.Id = v.PostId
//     ORDER BY u.TotalReputation DESC, p.CreationDate DESC)
// SELECT cs.DisplayName AS UserDisplayName, cs.TotalReputation, cs.BadgeCount, cs.ClosedPosts, cs.Title AS PostTitle, cs.PostCreationDate, cs.PostUpVotes, cs.PostDownVotes, cs.PostAvgVote
// FROM CombinedStats cs WHERE cs.ClosedPosts > 0 ORDER BY cs.TotalReputation DESC, cs.PostCreationDate DESC LIMIT 100;
//
// RevisionNumber is never read. The ClosedPosts distinct count is a fold over one row per post.
fn q30959(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let rph = || history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11, 12])));
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(badges_of(db).opt()).and(posts_of(db).select(rph().opt()).opt()))
        .fold([0i64; 2], |a, ((r, b), _)| [a[0] + r, a[1] + b.is_some() as i64]);
    let cl = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(rph()))).fold(0i64, |n, _| n + 1);
    let vt = &db.vote.vote_type_id;
    let pvs = db.vote.group_by(&db.vote.post).select(vt).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + if t == 2 || t == 3 { t } else { 0 }]);
    let v = drain((&ur).and(&cl).and(posts_of(db).select(Ident::<Post>::new().and((&pvs).opt()))));
    let v = top_n(v, |&(u, ((a, _), (p, _)))| (Reverse(a[0]), Reverse(creation_date.get(p).unwrap()), u, p), 100);
    rows(v.into_iter().map(|(u, ((a, c), (p, s)))| {
        let s = s.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(c)];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(s[0]), V::I(s[1]), if s[0] + s[1] == 0 { V::F(0.0) } else { avg(s[2], s[0] + s[1]) }]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')),
// UsersWithBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount, COUNT(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeleteUndeleteCount,
//        MAX(ph.CreationDate) AS LastEdited FROM PostHistory ph GROUP BY ph.PostId),
// HighScoringPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, ub.BadgeCount FROM RecentPosts rp JOIN UsersWithBadges ub ON rp.OwnerUserId = ub.UserId WHERE rp.Score > 10 AND ub.BadgeCount > 5),
// FinalSummary AS (SELECT hsp.PostId, hsp.Title, hsp.Score, hsp.ViewCount, hsp.AnswerCount, ph.CloseReopenCount, ph.DeleteUndeleteCount, ph.LastEdited FROM HighScoringPosts hsp
//     JOIN PostHistorySummary ph ON hsp.PostId = ph.PostId)
// SELECT fs.PostId, fs.Title, fs.Score, fs.ViewCount, fs.AnswerCount, fs.CloseReopenCount, fs.DeleteUndeleteCount, fs.LastEdited,
//        CASE WHEN fs.CloseReopenCount > 2 THEN 'Frequently Closed/Reopened' ELSE 'Rarely Closed/Reopened' END AS ClosureStatus
// FROM FinalSummary fs ORDER BY fs.Score DESC, fs.ViewCount DESC;
//
// rn is never read.
fn q32360(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold([0, 0, i64::MIN], |a, (t, d)| [a[0] + matches!(t, 10 | 11) as i64, a[1] + matches!(t, 12 | 13) as i64, a[2].max(d)]);
    let hs = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(score.gt(10))).with(owner_user.select((&bc).filt(|n| n > 5)));
    rows(drain(hs.select(&phs)).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::T(a[2]), V::S(if a[0] > 2 { "Frequently Closed/Reopened" } else { "Rarely Closed/Reopened" })]);
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges GROUP BY UserId),
// PostStats AS (SELECT OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        COUNT(DISTINCT Tags) AS UniqueTagsUsed FROM Posts GROUP BY OwnerUserId),
// MostActiveUsers AS (SELECT u.Id, u.DisplayName, COALESCE(b.GoldBadges, 0) AS GoldBadges, COALESCE(b.SilverBadges, 0) AS SilverBadges, COALESCE(b.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(p.TotalPosts, 0) AS TotalPosts, COALESCE(p.TotalQuestions, 0) AS TotalQuestions, COALESCE(p.TotalAnswers, 0) AS TotalAnswers, COALESCE(p.UniqueTagsUsed, 0) AS UniqueTagsUsed,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(p.TotalPosts, 0) DESC) AS UserRank FROM Users u LEFT JOIN UserBadgeStats b ON u.Id = b.UserId LEFT JOIN PostStats p ON u.Id = p.OwnerUserId),
// UserCounts AS (SELECT u.Id, u.DisplayName, COUNT(c.Id) AS CommentCount FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId GROUP BY u.Id, u.DisplayName)
// SELECT m.Id, m.DisplayName, m.GoldBadges, m.SilverBadges, m.BronzeBadges, m.TotalPosts, m.TotalQuestions, m.TotalAnswers, m.UniqueTagsUsed, COALESCE(c.CommentCount, 0) AS CommentCount,
//        CASE WHEN m.TotalQuestions > 0 THEN ROUND(CAST(m.TotalAnswers AS DECIMAL) / NULLIF(m.TotalQuestions, 0), 2) ELSE NULL END AS AnswerToQuestionRatio
// FROM MostActiveUsers m LEFT JOIN UserCounts c ON m.Id = c.Id WHERE m.UserRank <= 10 ORDER BY m.UserRank;
fn q1427(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let ps = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(tags_str.opt()))).buf_fold(|rows| {
        [rows.len() as i64, rows.iter().filter(|r| r.0 == 1).count() as i64, rows.iter().filter(|r| r.0 == 2).count() as i64, distinct_some(rows.iter().map(|r| r.1))]
    });
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let cc = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&cc).and((&ps).opt()).and((&bs).opt())), |&(u, ((_, p), _))| (Reverse(p.map_or(0, |a| a[0])), u), 10);
    rows(v.into_iter().map(|(u, ((c, p), b))| {
        let p = p.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend(p.map(V::I));
        f.extend([V::I(c), if p[1] > 0 { V::F((p[2] as f64 / p[1] as f64 * 100.0).round() / 100.0) } else { V::Null }]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, p.AcceptedAnswerId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank,
//        COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, p.AcceptedAnswerId),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// LastActivity AS (SELECT p.OwnerUserId, MAX(p.LastActivityDate) AS LastPostActivity FROM Posts p GROUP BY p.OwnerUserId)
// SELECT u.Id AS OwnerUserId, u.DisplayName, COALESCE(b.GoldBadges, 0) AS GoldBadges, COALESCE(b.SilverBadges, 0) AS SilverBadges, COALESCE(b.BronzeBadges, 0) AS BronzeBadges,
//        COUNT(DISTINCT rp.PostId) AS TotalPosts, COUNT(DISTINCT CASE WHEN rp.UserPostRank = 1 THEN rp.PostId END) AS AcceptedAnswersCount, SUM(rp.UpVoteCount) AS TotalUpVotes,
//        SUM(rp.DownVoteCount) AS TotalDownVotes, MAX(la.LastPostActivity) AS LastActivity, SUM(rp.CommentCount) AS TotalComments
// FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId LEFT JOIN UserBadges b ON u.Id = b.UserId LEFT JOIN LastActivity la ON u.Id = la.OwnerUserId
// WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, b.GoldBadges, b.SilverBadges, b.BronzeBadges ORDER BY TotalPosts DESC, u.DisplayName ASC LIMIT 10;
//
// The order reads the question count and the name, so the ten users are picked first and the comment x vote product is driven for their questions alone.
fn q30171(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, last_activity_date, .. } = &db.post;
    let asked = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let users = || db.user.with((&db.user.reputation).gt(0));
    let qc = users().group_by(Ident::<User>::new()).select(asked().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let top = top_n(drain(&qc), |&(u, n)| (Reverse(n), db.user.display_name.get(u).unwrap(), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user.select(Ident::<User>::new().with(&tu)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pcn = db.post.with(post_type_id.eq(1)).with(owner_user.select(Ident::<User>::new().with(&tu))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(asked().select((&pv).and(&pcn).and(Ident::<Post>::new().with(&first).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((v, c), f)) => [a[0] + 1, a[1] + f.is_some() as i64, a[2] + v[0], a[3] + v[1], a[4] + c],
            None => a,
        });
    let la = db.post.group_by(owner_user).select(last_activity_date).fold(i64::MIN, |m, d| m.max(d));
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = top_n(drain((&ua).and((&bs).opt()).and((&la).opt())), |&(u, ((a, _), _))| (Reverse(a[0]), db.user.display_name.get(u).unwrap(), u), 10);
    rows(v.into_iter().map(|(u, ((a, b), l))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), nullable(a[2], a[0]), nullable(a[3], a[0]), ots(l), nullable(a[4], a[0])]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// UserPostStats AS (SELECT UserId, DisplayName, Reputation, BadgeCount, UpVotes, DownVotes, PostCount, TotalScore, RANK() OVER (ORDER BY Reputation DESC, TotalScore DESC) AS ReputationRank FROM UserReputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, UpVotes, DownVotes, PostCount, TotalScore FROM UserPostStats WHERE ReputationRank <= 10),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, p.Score, u.Id AS OwnerUserId, u.DisplayName AS OwnerDisplayName FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.Score > 0 ORDER BY p.ViewCount DESC LIMIT 5)
// SELECT tu.DisplayName AS TopUser, tu.Reputation AS UserReputation, tu.BadgeCount AS TotalBadges, tu.UpVotes AS TotalUpVotes, tu.DownVotes AS TotalDownVotes, pp.Title AS PopularPostTitle,
//        pp.ViewCount AS PopularPostViews, pp.AnswerCount AS PopularPostAnswers, pp.Score AS PopularPostScore
// FROM TopUsers tu LEFT JOIN PopularPosts pp ON tu.UserId = pp.OwnerUserId ORDER BY tu.Reputation DESC;
//
// ReputationRank leads with Reputation, so only users with at least the tenth-highest reputation can rank in the top ten; the badge x vote x post product is driven for those alone.
fn q9060(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let tenth = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&db.user.reputation).ge(tenth)).collect();
    let ur = (&cand)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()).and(posts_of(db).select(score).opt()))
        .fold([0i64; 4], |a, ((b, t), s)| [a[0] + b.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + s.unwrap_or(0)]);
    let v = ranked(drain(&ur), |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[3])), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let pp = top_n(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(view_count.opt())), |&(p, w)| (w.is_none(), Reverse(w), p), 5);
    let pr: MatSet<Id<Post>> = rel(pp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_owner: HashIdx<Id<User>, Id<Post>> = (&pr).select(owner_user).inv().collect();
    type R = (Id<User>, [i64; 4]);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(&by_owner).opt())));
    rows(v.into_iter().map(|(_, ((u, a), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "views", "answers", "score"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS ScoreRank,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY p.Id) AS UpVoteCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY p.Id) AS DownVoteCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN bh.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN bh.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN bh.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Badges bh ON u.Id = bh.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseEvents, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenEvents,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (6, 4) THEN 1 END) AS EditEvents FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.UpVoteCount, rp.DownVoteCount, ua.DisplayName, ua.PostCount, ua.CommentCount, phs.CloseEvents, phs.ReopenEvents, phs.EditEvents
// FROM RankedPosts rp JOIN UserActivity ua ON rp.PostId IN (SELECT AcceptedAnswerId FROM Posts WHERE Id = rp.PostId) LEFT JOIN PostHistoryStats phs ON rp.PostId = phs.PostId
// WHERE rp.ScoreRank <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// RankedPosts has no GROUP BY, so ScoreRank numbers the post x vote rows; its ties are broken by the vote id. The ON clause reads only rp (a post that is its own accepted
// answer), so the rows that pass it are crossed with every user.
fn q21303(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, accepted_answer_id, origid, .. } = &db.post;
    let rows_v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id.and(votes_of(db).opt())));
    let r = top_per(rows_v, |&(_, (t, _))| t, |&(p, (_, v))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, v), 5, false);
    let rp: MatSet<Id<Post>> = rel(r.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let self_acc = (&rp).with(accepted_answer_id.and(origid).filt(|(a, i): (i64, i64)| a == i));
    let pv = self_acc.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt()))
        .fold([0i64; 3], |a, (c, _)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let pc = user_distinct_posts(db);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let phs = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2] + matches!(t, 4 | 6) as i64]);
    let users = (&ua).and(&pc).and(&cc);
    let mut v = Vec::new();
    (&pv).and((&phs).opt()).cross(users).drive(|(p, u), ((a, h), ((_, n), c))| v.push((p, a, h, u, n, c)));
    rows(v.into_iter().map(|(p, a, h, u, n, c)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), user_col(db, u, "name"), V::I(n), V::I(c)]);
        f.extend(match h {
            Some(h) => h.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotes, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount,
//        ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RecentPostRank FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1),
// BadgedUsers AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Date) AS LatestBadgeDate FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// RecentActiveUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.LastAccessDate DESC) AS RecentActivityRank FROM Users u
//     WHERE u.LastAccessDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'))
// SELECT ps.Title AS PostTitle, ps.CreationDate AS PostCreationDate, ps.ViewCount AS PostViewCount, ps.UpVotes AS PostUpVotes, ps.DownVotes AS PostDownVotes, ps.CommentCount AS PostCommentCount,
//        bu.BadgeCount AS UserBadgeCount, rau.DisplayName AS RecentActiveUserDisplayName, CASE WHEN ps.RecentPostRank <= 10 THEN 'Recent Top 10 Questions' ELSE 'Older Questions' END AS PostCategory,
//        COALESCE(rau.RecentActivityRank, 0) AS UserActivityRank
// FROM PostStats ps FULL OUTER JOIN BadgedUsers bu ON ps.PostId = bu.UserId FULL OUTER JOIN RecentActiveUsers rau ON bu.UserId = rau.Id
// WHERE (ps.UpVotes - ps.DownVotes) > 0 AND (COALESCE(bu.BadgeCount, 0) > 2 OR COALESCE(rau.Reputation, 0) > 100)
// ORDER BY ps.CreationDate DESC, COALESCE(bu.BadgeCount, 0) DESC, COALESCE(rau.Reputation, 0) DESC;
//
// The WHERE needs a PostStats row, so both FULL OUTER JOINs keep only their left rows: PostStats LEFT JOIN BadgedUsers LEFT JOIN RecentActiveUsers. `ps.PostId = bu.UserId`
// joins a post id to a user id, so it goes through the raw ids. PostStats has no GROUP BY, so it has one row per question x vote x comment; RecentPostRank numbers those rows,
// and its ties are broken by the vote and comment ids.
fn q24053(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, creation_date, origid, .. } = &db.post;
    let pw = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 3], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]
    });
    type R = (Id<Post>, (Option<Id<Vote>>, Option<Id<Comment>>));
    let prod = drain(db.post.with(post_type_id.eq(1)).select(votes_of(db).opt().and(comments_of(db).opt())));
    let recent = top_n(prod.clone(), |&(p, (v, c))| (Reverse(creation_date.get(p).unwrap()), p, v, c), 10);
    let recent_set: MatSet<R> = rel(recent).map(|x| x).collect();
    let bu = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ra = ranked(drain(db.user.with((&db.user.last_access_date).gt(add_days(t0, -30))).select(&db.user.last_access_date)), |&(_, d)| Reverse(d), false);
    let ra = rel(ra.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank_of: HashIdx<Id<User>, (Id<User>, i64)> = (&ra).map(|(u, _)| u).inv().select(&ra).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pr = rel(prod);
    let post = || Same::<R>::new().map(|(p, _): R| p);
    let v = drain(
        (&pr)
            .select(Same::<R>::new().and(Same::<R>::new().with(&recent_set).opt()).and(post().select(&pw)).and(post().select(origid).select(&uidx).select(Ident::<User>::new().and(&bu).and((&rank_of).opt())).opt()))
            .filt(|(((_, _), a), u): (((R, Option<R>), [i64; 3]), Option<((Id<User>, i64), Option<(Id<User>, i64)>)>)| {
                let (b, rep) = u.map_or((0, 0), |((_, b), r)| (b, r.map_or(0, |(u, _)| db.user.reputation.get(u).unwrap())));
                a[0] - a[1] > 0 && (b > 2 || rep > 100)
            }),
    );
    rows(v.into_iter().map(|(_, ((((p, _), rc), a), u))| {
        let mut f = post_fields(db, p, &["title", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        let cat = V::S(if rc.is_some() { "Recent Top 10 Questions" } else { "Older Questions" });
        match u {
            Some(((_, b), r)) => f.extend([V::I(b), r.map_or(V::Null, |(u, _)| user_col(db, u, "name")), cat, V::I(r.map_or(0, |(_, k)| k))]),
            None => f.extend([V::Null, V::Null, cat, V::I(0)]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.UserId, ph.Comment, ph.Text, RANK() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS CloseRank FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10),
// PostScores AS (SELECT p.Id, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        (COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0)) AS NetScore FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, COALESCE(cp.Comment, 'No Close Comment') AS CloseComment, ps.UpVotes, ps.DownVotes, ps.NetScore, ub.GoldBadges,
//        ub.SilverBadges, ub.BronzeBadges
// FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId AND cp.CloseRank = 1 LEFT JOIN PostScores ps ON rp.PostId = ps.Id
// LEFT JOIN UserBadges ub ON rp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = ub.UserId) WHERE rp.ScoreRank <= 5 ORDER BY rp.Score DESC;
//
// The correlated subquery is the badge owner's name, so UserBadges joins every user whose name is the post owner's.
fn q3787(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(post.and(hd)).inv().collect();
    let ps = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ubr = rel(drain(&ub));
    let by_name: HashIdx<Str, (Id<User>, [i64; 3])> = (&ubr).map(|(u, _)| u).select(&db.user.display_name).inv().select(&ubr).collect();
    let v = drain((&ps).and(Ident::<Post>::new().and(&md).select(&at).opt()).and(owner_user.select(&db.user.display_name).select(&by_name).opt()));
    rows(v.into_iter().map(|(p, ((a, h), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.push(h.and_then(|h| db.post_history.comment.get(h)).map_or(V::S("No Close Comment"), V::S));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        f.extend(match b {
            Some((_, b)) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// AggregatedVotes AS (SELECT p.Id AS PostId, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount FROM Posts p
//     LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadgeCount, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadgeCount, COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadgeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT u.DisplayName, rp.PostId, rp.Title, rp.Score, av.UpVoteCount, av.DownVoteCount, ubc.GoldBadgeCount, ubc.SilverBadgeCount, ubc.BronzeBadgeCount,
//        COALESCE(ubc.GoldBadgeCount + ubc.SilverBadgeCount * 0.5 + ubc.BronzeBadgeCount * 0.25, 0) AS BadgeScore,
//        CASE WHEN rp.UserPostRank = 1 THEN 'Newest Post by User' ELSE 'Older Posts by User' END AS PostStatus,
//        CASE WHEN COALESCE(av.UpVoteCount - av.DownVoteCount, 0) > 0 THEN 'Positive Engagement' WHEN COALESCE(av.UpVoteCount - av.DownVoteCount, 0) < 0 THEN 'Negative Engagement' ELSE 'Neutral Engagement' END AS EngagementType
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN AggregatedVotes av ON rp.PostId = av.PostId LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId
// WHERE COALESCE(av.UpVoteCount, 0) > 5 OR ubc.GoldBadgeCount > 0 ORDER BY rp.CreationDate DESC LIMIT 50;
fn q23064(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).with(owner_user);
    let first = top_per(drain(base().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let av = base().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = drain((&av).and(owner_user.select(&ub)).filt(|(a, b): ([i64; 2], [i64; 3])| a[0] > 5 || b[0] > 0).and(Ident::<Post>::new().with(&first).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((a, b), f1))| {
        let net = a[0] - a[1];
        let mut f = post_fields(db, p, &["owner", "id", "title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(b.map(V::I));
        f.push(V::F(b[0] as f64 + b[1] as f64 * 0.5 + b[2] as f64 * 0.25));
        f.push(V::S(if f1.is_some() { "Newest Post by User" } else { "Older Posts by User" }));
        f.push(V::S(if net > 0 { "Positive Engagement" } else if net < 0 { "Negative Engagement" } else { "Neutral Engagement" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COUNT(DISTINCT p.Id) AS PostCount, COALESCE(SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes vt ON p.Id = vt.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Views, PostCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats WHERE PostCount > 0),
// PostAnalysis AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS CloseCount,
//        COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END), 0) AS ReopenCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate),
// FilteredPosts AS (SELECT pa.PostId, pa.Title, pa.Body, pa.CreationDate, pa.CommentCount, pa.CloseCount, pa.ReopenCount, RANK() OVER (ORDER BY pa.CommentCount DESC) AS CommentRank FROM PostAnalysis pa
//     WHERE pa.CloseCount > 0)
// SELECT tu.Rank, tu.DisplayName, tu.Reputation, fp.Title, fp.Body, fp.CreationDate, fp.CommentCount, fp.CloseCount, fp.ReopenCount
// FROM TopUsers tu JOIN FilteredPosts fp ON tu.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = fp.PostId) WHERE tu.Rank <= 10 ORDER BY tu.Rank, fp.CommentCount DESC;
//
// The correlated subquery is the post's owner. Rank reads only Reputation, so the ten posters are picked first and PostAnalysis is driven for their posts alone;
// CommentRank and the vote counts are never read.
fn q4448(db: &'static So) -> String {
    let posters: MatSet<Id<User>> = db.post.select(&db.post.owner_user).collect();
    let r = ranked(drain((&posters).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let owned: MatSet<Id<Post>> = (&tu).map(|(u, _)| u).select(posts_of(db)).collect();
    let pa = (&owned)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(10)) as i64, a[2] + (t == Some(11)) as i64]);
    type R = (Id<User>, i64);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(posts_of(db)).select(Ident::<Post>::new().and((&pa).filt(|a| a[1] > 0))))));
    rows(v.into_iter().map(|(_, ((u, r), (p, a)))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(post_fields(db, p, &["title", "body", "created"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCount AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank FROM Posts P
//     WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopComments AS (SELECT C.UserId, COUNT(C.Id) AS CommentCount FROM Comments C WHERE C.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY C.UserId),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(UB.BadgeCount, 0) AS TotalBadges, COALESCE(RP.PostCount, 0) AS RecentPostCount, COALESCE(TC.CommentCount, 0) AS RecentCommentCount
//     FROM Users U LEFT JOIN UserBadgeCount UB ON U.Id = UB.UserId LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS PostCount FROM RecentPosts WHERE PostRank = 1 GROUP BY OwnerUserId) RP ON U.Id = RP.OwnerUserId
//     LEFT JOIN TopComments TC ON U.Id = TC.UserId)
// SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.TotalBadges, TU.RecentPostCount, TU.RecentCommentCount,
//        CASE WHEN TU.Reputation >= 1000 THEN 'Expert' WHEN TU.Reputation >= 500 THEN 'Intermediate' ELSE 'Novice' END AS UserLevel
// FROM TopUsers TU WHERE TU.RecentPostCount > 0 OR TU.RecentCommentCount > 0 ORDER BY TU.Reputation DESC, TU.TotalBadges DESC LIMIT 10;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q32642(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { owner_user, creation_date, .. } = &db.post;
    let first = top_per(drain(db.post.with(creation_date.gt(add_days(t0, -30))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&first).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let tc = db.comment.with((&db.comment.creation_date).gt(add_days(t0, -30))).group_by(&db.comment.user).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.select((&ub).opt().and((&rp).opt()).and((&tc).opt())).filt(|((_, r), c): ((Option<i64>, Option<i64>), Option<i64>)| r.unwrap_or(0) > 0 || c.unwrap_or(0) > 0));
    let v = top_n(v, |&(u, ((b, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(b.unwrap_or(0)), u), 10);
    rows(v.into_iter().map(|(u, ((b, r), c))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(r.unwrap_or(0)), V::I(c.unwrap_or(0)), V::S(if rep >= 1000 { "Expert" } else if rep >= 500 { "Intermediate" } else { "Novice" })]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// PostAggregates AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(p.Score) AS TotalScore, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// ClosedPostCounts AS (SELECT ph.UserId, COUNT(ph.PostId) AS ClosedPosts FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE pht.Name = 'Post Closed' GROUP BY ph.UserId),
// UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(pa.TotalPosts, 0) AS TotalPosts, COALESCE(pa.TotalScore, 0) AS TotalScore, COALESCE(pa.AvgViewCount, 0) AS AvgViewCount, COALESCE(cpc.ClosedPosts, 0) AS ClosedPosts
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostAggregates pa ON u.Id = pa.OwnerUserId LEFT JOIN ClosedPostCounts cpc ON u.Id = cpc.UserId),
// RankedUsers AS (SELECT up.*, RANK() OVER (ORDER BY up.TotalScore DESC, up.TotalPosts DESC) AS PerformanceRank FROM UserPerformance up)
// SELECT ru.UserId, ru.DisplayName, ru.GoldBadges, ru.SilverBadges, ru.BronzeBadges, ru.TotalPosts, ru.TotalScore, ru.AvgViewCount, ru.ClosedPosts, ru.PerformanceRank
// FROM RankedUsers ru WHERE ru.PerformanceRank <= 10 ORDER BY ru.PerformanceRank;
fn q1794(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let pa = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(&db.post.owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let cpc = db.post_history.with(htype_name(db).filt(|n: Str| n == "Post Closed")).group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = ranked(drain(db.user.select((&ub).opt().and((&pa).opt()).and((&cpc).opt()))), |&(_, ((_, a), _))| {
        let a = a.unwrap_or([0; 3]);
        (Reverse(a[1]), Reverse(a[0]))
    }, false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((b, a), c)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        match a {
            Some(a) => f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[0])]),
            None => f.extend([V::I(0), V::I(0), V::F(0.0)]),
        }
        f.extend([V::I(c.unwrap_or(0)), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.Score > 0),
// RecentVotes AS (SELECT v.PostId, v.VoteTypeId, COUNT(v.Id) AS VoteCount FROM Votes v WHERE v.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY v.PostId, v.VoteTypeId),
// PostHistoryAggregates AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS IsEdited,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS IsClosed, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS IsReopened FROM PostHistory ph GROUP BY ph.PostId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.OwnerDisplayName, pha.EditCount, pha.IsEdited, pha.IsClosed, pha.IsReopened, COALESCE(rv.VoteCount, 0) AS RecentVoteCount
//     FROM RankedPosts rp LEFT JOIN PostHistoryAggregates pha ON rp.PostId = pha.PostId LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId AND rv.VoteTypeId = 2 WHERE rp.rn = 1)
// SELECT fp.PostId, fp.Title, fp.Score, fp.CreationDate, fp.OwnerDisplayName, fp.EditCount, fp.IsEdited, fp.IsClosed, fp.IsReopened, fp.RecentVoteCount,
//        CASE WHEN fp.IsClosed = 1 THEN 'Closed' WHEN fp.IsReopened = 1 THEN 'Reopened' ELSE 'Active' END AS PostStatus, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = fp.PostId) AS CommentCount
// FROM FilteredPosts fp ORDER BY fp.Score DESC, fp.CreationDate DESC FETCH FIRST 10 ROWS ONLY;
fn q34071(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let first = top_per(drain(db.post.with(score.gt(0)).with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let v = top_n(first, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pha = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 4], |a, t| {
        [a[0] + 1, a[1].max(matches!(t, 4 | 5 | 6) as i64), a[2].max((t == 10) as i64), a[3].max((t == 11) as i64)]
    });
    let rv = db.vote.with((&db.vote.creation_date).ge(add_days(date(2024, 10, 1), -30)).and((&db.vote.vote_type_id).eq(2))).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&cc).and((&pha).opt()).and((&rv).opt())).into_iter().map(|(p, ((c, h), r))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner"]);
        f.extend(match h {
            Some(h) => h.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::I(r.unwrap_or(0)));
        f.push(V::S(match h {
            Some(h) if h[2] == 1 => "Closed",
            Some(h) if h[3] == 1 => "Reopened",
            _ => "Active",
        }));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AverageScores AS (SELECT OwnerUserId, AVG(Score) AS AvgScore FROM Posts GROUP BY OwnerUserId),
// TopBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(rp.RecentPostCount, 0) AS RecentPostCount, COALESCE(ascores.AvgScore, 0) AS AverageScore, COALESCE(tb.BadgeCount, 0) AS GoldBadgeCount,
//        CASE WHEN u.Reputation > 1000 THEN 'High Reputation' WHEN u.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
//     FROM Users u LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS RecentPostCount FROM RecentPosts GROUP BY OwnerUserId) rp ON u.Id = rp.OwnerUserId
//     LEFT JOIN AverageScores ascores ON u.Id = ascores.OwnerUserId LEFT JOIN TopBadges tb ON u.Id = tb.UserId)
// SELECT us.DisplayName, us.RecentPostCount, us.AverageScore, us.GoldBadgeCount, us.ReputationCategory, COUNT(v.Id) AS TotalVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM UserStats us LEFT JOIN Votes v ON us.UserId = v.UserId GROUP BY us.UserId, us.DisplayName, us.RecentPostCount, us.AverageScore, us.GoldBadgeCount, us.ReputationCategory
// HAVING COUNT(v.Id) >= 10 ORDER BY us.AverageScore DESC, us.RecentPostCount DESC;
//
// UserPostRank is never read.
fn q32727(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let rp = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let asc = db.post.group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let tb = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let vv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&vv).filt(|a| a[0] >= 10).and((&rp).opt()).and((&asc).opt()).and((&tb).opt()));
    rows(v.into_iter().map(|(u, (((a, r), s), g))| {
        let rep = db.user.reputation.get(u).unwrap();
        row(vec![
            user_col(db, u, "name"),
            V::I(r.unwrap_or(0)),
            s.map_or(V::F(0.0), |s| avg(s[1], s[0])),
            V::I(g.unwrap_or(0)),
            V::S(if rep > 1000 { "High Reputation" } else if rep >= 500 { "Medium Reputation" } else { "Low Reputation" }),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
        ])
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') AND p.Score > 0 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, us.QuestionCount, us.AnswerCount, us.UpVotes, us.DownVotes, ROW_NUMBER() OVER (ORDER BY us.Reputation DESC) AS Rank FROM UserStats us)
// SELECT tu.Rank, tu.DisplayName, tu.Reputation, pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.Score, pd.CommentCount
// FROM TopUsers tu JOIN PostDetails pd ON tu.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pd.PostId LIMIT 1) WHERE tu.Rank <= 10 ORDER BY tu.Rank, pd.Score DESC;
//
// UserStats has one row per user with Reputation > 1000 and only its name and reputation are read, so its counts are not computed. The correlated LIMIT 1 looks a post up by its id,
// so it is the post's owner.
fn q5210(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(tu.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let pd = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt())
        .fold(0i64, |n, c| n + c.is_some() as i64);
    type R = (Id<User>, i64);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(posts_of(db)).select(Ident::<Post>::new().and(&pd)))));
    rows(v.into_iter().map(|(_, ((u, r), (p, c)))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AverageViews, MAX(P.LastActivityDate) AS LastPostDate FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// UserRanking AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(UB.BadgeCount, 0) AS TotalBadges, COALESCE(PS.PostCount, 0) AS TotalPosts, COALESCE(PS.TotalScore, 0) AS TotalScore,
//        COALESCE(PS.AverageViews, 0) AS AverageViews, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId),
// ClosedPosts AS (SELECT P.Id, PH.UserDisplayName AS ClosedBy, PH.CreationDate AS ClosedDate, P.Title, P.Score, (SELECT COUNT(C.Id) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount
//     FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId AND PH.PostHistoryTypeId = 10)
// SELECT UR.Rank, UR.DisplayName, UR.TotalBadges, UR.TotalPosts, UR.TotalScore, UR.AverageViews, CP.Title, CP.ClosedBy, CP.ClosedDate, CP.Score, CP.CommentCount
// FROM UserRanking UR LEFT JOIN ClosedPosts CP ON UR.UserId = CP.Id WHERE UR.Rank <= 50 ORDER BY UR.TotalScore DESC, UR.TotalPosts DESC NULLS LAST OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// `UR.UserId = CP.Id` joins a user id to a post id, so it goes through the raw ids. Rank reads only Reputation, so the fifty users are picked first.
fn q22849(db: &'static So) -> String {
    let Post { creation_date, score, view_count, last_activity_date, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 50);
    let tu = rel(tu.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(&db.post.owner_user)
        .select(score.and(view_count.opt()).and(last_activity_date))
        .fold([0, 0, 0, 0, i64::MIN], |a, ((s, w), d)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4].max(d)]);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    type R = (Id<User>, i64);
    let u = || Same::<R>::new().map(|(u, _): R| u);
    let v = drain((&tu).select(Same::<R>::new().and(u().select((&ub).opt())).and(u().select((&ps).opt())).and(u().select(&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(closes).and((&cc).opt())).opt())));
    let key = |p: Option<[i64; 5]>| p.map_or((0, 0), |a| (a[1], a[0]));
    let v = top_n(v, |&(_, ((((u, _), _), p), c))| (Reverse(key(p).0), Reverse(key(p).1), u, c), 10);
    rows(v.into_iter().map(|(_, ((((u, r), b), p), c))| {
        let a = p.unwrap_or([0; 5]);
        let mut f = vec![V::I(r), user_col(db, u, "name"), V::I(b.unwrap_or(0)), V::I(a[0]), V::I(a[1]), if a[2] == 0 { V::F(0.0) } else { avg(a[3], a[2]) }];
        match c {
            Some(((p, h), n)) => {
                f.extend(post_fields(db, p, &["title"]));
                f.extend([ostr(db.post_history.user_display_name.get(h)), V::T(db.post_history.creation_date.get(h).unwrap())]);
                f.extend(post_fields(db, p, &["score"]));
                f.push(V::I(n.unwrap_or(0)));
            }
            None => f.extend((0..5).map(|_| V::Null)),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, MAX(b.Class) OVER (PARTITION BY p.OwnerUserId) AS MaxBadgeClass
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PostAnalytics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score,
//        CASE WHEN rp.CommentCount > 10 THEN 'Highly Discussed' WHEN rp.CommentCount BETWEEN 5 AND 10 THEN 'Moderately Discussed' ELSE 'Less Discussed' END AS DiscussionLevel,
//        COALESCE(rp.MaxBadgeClass, 0) AS UserBadgeClass FROM RankedPosts rp WHERE rp.ScoreRank <= 10),
// InvalidVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId NOT IN (2, 3) THEN 1 ELSE 0 END) AS InvalidVoteCount FROM Votes v
//     WHERE v.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId),
// FinalResult AS (SELECT pa.PostId, pa.Title, pa.CreationDate, pa.Score, pa.DiscussionLevel, pa.UserBadgeClass, COALESCE(iv.InvalidVoteCount, 0) AS InvalidVoteCount
//     FROM PostAnalytics pa LEFT JOIN InvalidVotes iv ON pa.PostId = iv.PostId)
// SELECT fr.PostId, fr.Title, fr.CreationDate, fr.Score, fr.DiscussionLevel, fr.UserBadgeClass, CASE WHEN fr.InvalidVoteCount > 5 THEN 'Needs Attention' ELSE 'Normal' END AS PostStatus
// FROM FinalResult fr WHERE fr.UserBadgeClass = 1 ORDER BY fr.Score DESC, fr.CreationDate DESC;
//
// RankedPosts has no GROUP BY, so ScoreRank numbers the post x comment x badge rows; its ties are broken by the ids. The windowed COUNT and MAX are per post and per owner
// over those same rows.
fn q23016(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(t0, -1)).and(post_type_id.is_in([1, 2])));
    let prod = drain(base().select(post_type_id.and(comments_of(db).opt()).and(owner_user.select(badges_of(db)).opt())));
    let top = top_per(prod, |&(_, ((t, _), _))| t, |&(p, ((_, c), b))| (Reverse(score.get(p).unwrap()), p, c, b), 10, false);
    let tr = rel(top);
    let cw = base().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(owner_user.select(badges_of(db)).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let mc = db.post.with(creation_date.ge(add_years(t0, -1)).and(post_type_id.is_in([1, 2]))).group_by(owner_user).select(comments_of(db).opt().and(owner_user.select(badges_of(db)).select(&db.badge.class).opt())).fold(0i64, |m, (_, c)| m.max(c.unwrap_or(0)));
    let iv = db.vote.with((&db.vote.creation_date).lt(add_days(t0, -30))).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |n, t| n + !matches!(t, 2 | 3) as i64);
    type R = (Id<Post>, ((i64, Option<Id<Comment>>), Option<Id<Badge>>));
    let p = || Same::<R>::new().map(|(p, _): R| p);
    let v = drain((&tr).select(Same::<R>::new().and(p().select(&cw)).and(p().select(owner_user.select(&mc)).opt().filt(|m: Option<i64>| m.unwrap_or(0) == 1)).and(p().select((&iv).opt()))));
    rows(v.into_iter().map(|(_, ((((p, _), c), m), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::S(if c > 10 { "Highly Discussed" } else if c >= 5 { "Moderately Discussed" } else { "Less Discussed" }));
        f.push(V::I(m.unwrap_or(0)));
        f.push(V::S(if n.unwrap_or(0) > 5 { "Needs Attention" } else { "Normal" }));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// AggregatedStatistics AS (SELECT UserId, Reputation, TotalPosts, QuestionsCount, AnswersCount, GoldBadges, SilverBadges, BronzeBadges, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStatistics),
// MetricCalculations AS (SELECT UserId, Reputation, TotalPosts, QuestionsCount, AnswersCount, GoldBadges, SilverBadges, BronzeBadges,
//        CASE WHEN TotalPosts = 0 THEN 0 ELSE CAST(QuestionsCount AS FLOAT) / TotalPosts END AS QuestionRatio, CASE WHEN TotalPosts = 0 THEN 0 ELSE CAST(AnswersCount AS FLOAT) / TotalPosts END AS AnswerRatio
//     FROM AggregatedStatistics)
// SELECT U.DisplayName, U.Id, U.Reputation, M.TotalPosts, M.QuestionsCount, M.AnswersCount, M.GoldBadges, M.SilverBadges, M.BronzeBadges, M.QuestionRatio, M.AnswerRatio,
//        (SELECT COUNT(*) FROM Votes V WHERE V.UserId = U.Id AND V.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') AS RecentVoteCount
// FROM Users U JOIN MetricCalculations M ON U.Id = M.UserId WHERE (M.QuestionRatio > 0.5 OR M.AnswerRatio > 0.5) AND (M.GoldBadges + M.SilverBadges + M.BronzeBadges > 0)
// ORDER BY M.Reputation DESC, U.DisplayName FETCH FIRST 10 ROWS ONLY;
//
// ReputationRank is never read. CAST(.. AS FLOAT) is a 4-byte float, so the ratios are computed in f32.
fn q23193(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 5], |a, (t, c)| {
        [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
    });
    let ratio = |x: i64, n: i64| if n == 0 { 0.0f32 } else { x as f32 / n as f32 };
    let rv = db.vote.with((&db.vote.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(
        (&us)
            .and(user_distinct_posts(db))
            .filt(move |(a, n): ([i64; 5], i64)| (ratio(a[0], n) > 0.5 || ratio(a[1], n) > 0.5) && a[2] + a[3] + a[4] > 0)
            .and((&rv).opt()),
    );
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), db.user.display_name.get(u).unwrap(), u), 10);
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["name", "uid", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::F(ratio(a[0], n) as f64), V::F(ratio(a[1], n) as f64), V::I(r.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes,
//        SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostEngagement AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN p.PostTypeId = 2 THEN p.Score ELSE 0 END) AS AnswerScore,
//        MAX(p.CreationDate) AS LastActiveDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title),
// ClosedPosts AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS ClosureCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// RankedPosts AS (SELECT pe.PostId, pe.Title, pe.CommentCount, pe.TotalViews, cp.ClosureCount, RANK() OVER (ORDER BY pe.TotalViews DESC, pe.AnswerScore DESC) AS ViewRank
//     FROM PostEngagement pe LEFT JOIN ClosedPosts cp ON pe.PostId = cp.PostId WHERE pe.CommentCount > 5)
// SELECT ups.UserId, ups.DisplayName, rp.PostId, rp.Title, rp.CommentCount, rp.TotalViews, rp.ClosureCount, ups.Upvotes, ups.Downvotes, ups.TotalBounty,
//        CONCAT('User: ', ups.DisplayName, ' engaged with Post: "', rp.Title, '" (', rp.CommentCount, ' comments, ', rp.TotalViews, ' views)') AS EngagementSummary
// FROM UserVoteStats ups JOIN Posts p ON ups.UserId = p.OwnerUserId JOIN RankedPosts rp ON p.Id = rp.PostId
// WHERE (ups.Upvotes - ups.Downvotes) > 0 AND rp.ViewRank <= 10 AND (rp.ClosureCount IS NULL OR rp.ClosureCount < 2) ORDER BY ups.TotalBounty DESC, rp.TotalViews DESC;
fn q23454(db: &'static So) -> String {
    let Post { view_count, post_type_id, score, owner_user, .. } = &db.post;
    let pe = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(view_count.opt()).and(post_type_id).and(score)).fold([0i64; 3], |a, (((c, w), t), s)| {
        [a[0] + c.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + if t == 2 { s } else { 0 }]
    });
    let cp = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold(0i64, |n, t| n + (t == 10) as i64);
    let r = ranked(drain((&pe).filt(|a| a[0] > 5).and((&cp).opt())), |&(_, (a, _))| (Reverse(a[1]), Reverse(a[2])), false);
    let tr = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).fold([0i64; 3], |a, v| match v {
        Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + b.unwrap_or(0)],
        None => a,
    });
    type R = (Id<Post>, ([i64; 3], Option<i64>));
    let v = drain(
        (&tr)
            .select(Same::<R>::new().filt(|(_, (_, c)): R| c.map_or(true, |c| c < 2)).and(Same::<R>::new().map(|(p, _): R| p).select(owner_user).select(Ident::<User>::new().and((&uvs).filt(|a| a[0] - a[1] > 0))))),
    );
    rows(v.into_iter().map(|(_, ((p, (a, c)), (u, s)))| {
        let name = db.user.display_name.get(u).unwrap();
        let t = db.post.title.get(p).unwrap_or("");
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(a[0]), V::I(a[1]), oint(c), V::I(s[0]), V::I(s[1]), V::I(s[2])]);
        f.push(V::Owned(format!("User: {name} engaged with Post: \"{t}\" ({} comments, {} views)", a[0], a[1])));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldCount, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverCount, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeCount
//     FROM Badges b GROUP BY b.UserId),
// PostCounts AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY p.OwnerUserId),
// ClosedPosts AS (SELECT ph.UserId, COUNT(*) AS ClosedPostCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.UserId),
// UserStats AS (SELECT u.Id, u.DisplayName, COALESCE(ub.GoldCount, 0) AS GoldCount, COALESCE(ub.SilverCount, 0) AS SilverCount, COALESCE(ub.BronzeCount, 0) AS BronzeCount,
//        COALESCE(pc.QuestionCount, 0) AS QuestionCount, COALESCE(pc.AnswerCount, 0) AS AnswerCount, COALESCE(pc.TotalBounties, 0) AS TotalBounties, COALESCE(cp.ClosedPostCount, 0) AS ClosedPostCount
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostCounts pc ON u.Id = pc.OwnerUserId LEFT JOIN ClosedPosts cp ON u.Id = cp.UserId),
// FinalStats AS (SELECT us.DisplayName, us.QuestionCount, us.AnswerCount, us.TotalBounties, us.ClosedPostCount, ROW_NUMBER() OVER (ORDER BY us.QuestionCount DESC, us.AnswerCount DESC) AS Rank FROM UserStats us)
// SELECT fs.DisplayName, fs.QuestionCount, fs.AnswerCount, fs.TotalBounties, fs.ClosedPostCount,
//        CASE WHEN fs.ClosedPostCount > 10 THEN 'Expert' WHEN fs.ClosedPostCount BETWEEN 5 AND 10 THEN 'Intermediate' ELSE 'Novice' END AS UserLevel
// FROM FinalStats fs WHERE fs.ClosedPostCount IS NOT NULL ORDER BY fs.Rank LIMIT 100;
//
// The badge counts are never read.
fn q4835(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt()).fold([0i64; 3], |a, x| match x {
        Some((t, b)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let cp = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&pc).and((&cp).opt())), |&(u, (a, _))| (Reverse(a[0]), Reverse(a[1]), u), 100);
    rows(v.into_iter().map(|(u, (a, c))| {
        let c = c.unwrap_or(0);
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c), V::S(if c > 10 { "Expert" } else if c >= 5 { "Intermediate" } else { "Novice" })])
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, U.CreationDate, ROW_NUMBER() OVER (PARTITION BY CASE WHEN U.Reputation IS NULL THEN 'NULL' ELSE 'NOT_NULL' END ORDER BY U.Reputation DESC) AS Rank,
//        CASE WHEN U.Reputation IS NULL THEN 'Unranked' ELSE 'Ranked' END AS ReputationStatus FROM Users U WHERE U.Reputation IS NOT NULL OR U.Reputation IS NULL),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges B GROUP BY B.UserId),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, AVG(P.Score) AS AvgScore FROM Posts P GROUP BY P.OwnerUserId),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(P.TotalPosts, 0) AS TotalPosts, COALESCE(B.GoldBadges, 0) AS GoldBadges, COALESCE(B.SilverBadges, 0) AS SilverBadges,
//        COALESCE(B.BronzeBadges, 0) AS BronzeBadges, COALESCE(P.NegativePosts, 0) AS NegativePosts, COALESCE(P.AvgScore, 0) AS AvgScore
//     FROM Users U LEFT JOIN PostStatistics P ON U.Id = P.OwnerUserId LEFT JOIN UserBadges B ON U.Id = B.UserId)
// SELECT UA.DisplayName, UA.TotalPosts, UA.GoldBadges, UA.SilverBadges, UA.BronzeBadges, UA.NegativePosts, UA.AvgScore,
//        CASE WHEN UA.AvgScore < 0 THEN 'Needs Improvement' WHEN UA.AvgScore = 0 THEN 'Neutral' ELSE 'Good Score' END AS ScoreEvaluation, R.Rank, R.ReputationStatus
// FROM UserActivity UA JOIN RankedUsers R ON UA.UserId = R.UserId WHERE UA.TotalPosts > (SELECT AVG(TotalPosts) FROM PostStatistics HAVING COUNT(*) > 1) OR UA.GoldBadges > 2
// ORDER BY UA.TotalPosts DESC, R.Reputation DESC OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY;
//
// Reputation is never NULL, so RankedUsers is one partition. The ROW_NUMBER ties on Reputation are broken by Id; the query leaves them open.
fn q22713(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let rr = rel(ranked(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank_of: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let ps = db.post.group_by(owner_user.opt()).select(score).fold([0i64; 3], |a, s| [a[0] + 1, a[1] + (s < 0) as i64, a[2] + s]);
    let (gs, gn) = (&ps).fold_flat((0i64, 0i64), |(s, n), a| (s + a[0], n + 1));
    let mean = if gn > 1 { Some(gs as f64 / gn as f64) } else { None };
    let pu = db.post.group_by(owner_user).select(score).fold([0i64; 3], |a, s| [a[0] + 1, a[1] + (s < 0) as i64, a[2] + s]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain(
        db.user
            .select((&pu).opt().and((&ub).opt()).and(&rank_of))
            .filt(move |((p, b), _): ((Option<[i64; 3]>, Option<[i64; 3]>), (Id<User>, i64))| mean.map_or(false, |m| p.map_or(0, |a| a[0]) as f64 > m) || b.map_or(0, |b| b[0]) > 2),
    );
    let v = top_n(v, |&(u, ((p, _), _))| (Reverse(p.map_or(0, |a| a[0])), Reverse(db.user.reputation.get(u).unwrap()), u), 15);
    rows(v.into_iter().skip(5).map(|(u, ((p, b), (_, r)))| {
        let p = p.unwrap_or([0; 3]);
        let avg_v = if p[0] == 0 { 0.0 } else { p[2] as f64 / p[0] as f64 };
        let mut f = vec![user_col(db, u, "name"), V::I(p[0])];
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend([V::I(p[1]), if p[0] == 0 { V::F(0.0) } else { avg(p[2], p[0]) }]);
        f.push(V::S(if avg_v < 0.0 { "Needs Improvement" } else if avg_v == 0.0 { "Neutral" } else { "Good Score" }));
        f.extend([V::I(r), V::S("Ranked")]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount, p.OwnerUserId,
//        ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RowNum FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, rp.AnswerCount, ur.UserId, ur.Reputation, ur.BadgeCount,
//        CASE WHEN ur.Reputation < 100 THEN 'Newbie' WHEN ur.Reputation BETWEEN 100 AND 1000 THEN 'Intermediate' ELSE 'Expert' END AS UserLevel FROM RecentPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId)
// SELECT pm.PostId, pm.Title, pm.Score, pm.ViewCount, pm.CommentCount, pm.AnswerCount, pm.Reputation, pm.UserLevel, COALESCE(MAX(v.UserId) FILTER (WHERE vt.Name = 'UpMod'), -1) AS LastUpVoter,
//        COALESCE(MAX(v.UserId) FILTER (WHERE vt.Name = 'DownMod'), -1) AS LastDownVoter, CASE WHEN pm.CommentCount > 10 THEN 'High Interaction' ELSE 'Low Interaction' END AS InteractionLevel
// FROM PostMetrics pm LEFT JOIN Votes v ON pm.PostId = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
// WHERE pm.AnswerCount = (SELECT MAX(AnswerCount) FROM PostMetrics) AND pm.UserLevel = 'Expert'
// GROUP BY pm.PostId, pm.Title, pm.Score, pm.ViewCount, pm.CommentCount, pm.AnswerCount, pm.Reputation, pm.UserLevel ORDER BY pm.Score DESC LIMIT 10;
//
// RowNum and BadgeCount are never read.
fn q22926(db: &'static So) -> String {
    let Post { creation_date, owner_user, answer_count, score, .. } = &db.post;
    let pm = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let most = pm().select(answer_count).fold_flat(i64::MIN, |m, a| m.max(a));
    let base = pm().with(answer_count.filt(move |a: i64| a == most)).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))));
    let vs = base.group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db).and((&db.vote.user_id).opt())).opt()).fold([-1i64; 2], |a, v| match v {
        Some(("UpMod", Some(u))) => [a[0].max(u), a[1]],
        Some(("DownMod", Some(u))) => [a[0], a[1].max(u)],
        _ => a,
    });
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&vs).and((&cc).opt())), |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let c = c.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c)]);
        f.extend(post_fields(db, p, &["answers", "rep"]));
        f.extend([V::S("Expert"), V::I(a[0]), V::I(a[1]), V::S(if c > 10 { "High Interaction" } else { "Low Interaction" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        MAX(u.Reputation) AS MaxReputation, MIN(u.CreationDate) AS AccountCreated, AVG(COALESCE(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS AverageUpvotes,
//        AVG(COALESCE(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS AverageDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id GROUP BY u.Id, u.DisplayName),
// BadgeStats AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// FinalStats AS (SELECT us.UserId, us.DisplayName, us.PostCount, us.QuestionCount, us.AnswerCount, us.MaxReputation, us.AccountCreated, COALESCE(bs.BadgeCount, 0) AS BadgeCount, COALESCE(bs.GoldBadges, 0) AS GoldBadges,
//        COALESCE(bs.SilverBadges, 0) AS SilverBadges, COALESCE(bs.BronzeBadges, 0) AS BronzeBadges, us.AverageUpvotes, us.AverageDownvotes FROM UserStats us LEFT JOIN BadgeStats bs ON us.UserId = bs.UserId)
// SELECT fs.DisplayName, fs.PostCount, fs.QuestionCount, fs.AnswerCount, fs.MaxReputation, fs.AccountCreated, fs.BadgeCount, fs.GoldBadges, fs.SilverBadges, fs.BronzeBadges, fs.AverageUpvotes,
//        fs.AverageDownvotes, ROW_NUMBER() OVER (ORDER BY fs.MaxReputation DESC) AS Rank
// FROM FinalStats fs WHERE fs.PostCount > 0 ORDER BY fs.MaxReputation DESC, fs.PostCount DESC LIMIT 10;
//
// `v.UserId = u.Id` keeps a user's own votes on their own posts. The order leads with Reputation, so the ten posters are picked first.
fn q8986(db: &'static So) -> String {
    let posters: MatSet<Id<User>> = db.post.select(&db.post.owner_user).collect();
    let pc = (&posters).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let tu = top_n(drain(&pc), |&(u, n)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u), 10);
    let tu = rel(tu.into_iter().enumerate().map(|(i, (u, n))| (u, (n, i as i64 + 1))).collect());
    let ov = own_votes(db);
    let tus: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let st = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and((&ov).select(&db.vote.vote_type_id).opt()))).fold([0i64; 5], |a, (t, v)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64]
    });
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    type R = (Id<User>, (i64, i64));
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&st).and((&bs).opt())))));
    rows(v.into_iter().map(|(_, ((u, (n, r)), (a, b)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[1]), V::I(a[2])];
        f.extend(ucols(db, u, &["rep", "ucreated"]));
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.extend([avg(a[3], a[0]), avg(a[4], a[0]), V::I(r)]);
        row(f)
    }))
}

// WITH TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u WHERE u.Reputation > 1000),
// PostSummary AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, MAX(p.CreationDate) AS LastPostDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId),
// ClosedPosts AS (SELECT p.Id AS ClosedPostId, p.Title, MAX(ph.CreationDate) AS ClosureDate, (SELECT COUNT(*) FROM PostHistory ph2 WHERE ph2.PostId = p.Id AND ph2.PostHistoryTypeId = 10) AS CloseCount
//     FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10 GROUP BY p.Id, p.Title),
// ReputationBreakdown AS (SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 3 WHEN b.Class = 2 THEN 2 WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BadgePoints FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT tu.DisplayName, tu.Reputation, ps.CommentCount, ps.VoteCount, cp.Title AS ClosedPostTitle, cp.ClosureDate, rb.BadgePoints,
//        CASE WHEN ps.VoteCount > 10 THEN 'Highly Voted' WHEN ps.VoteCount BETWEEN 5 AND 10 THEN 'Moderately Voted' ELSE 'Low Voted' END AS VoteCategory,
//        CASE WHEN rb.BadgePoints > 5 THEN 'Active Contributor' ELSE 'New Contributor' END AS ContributorType
// FROM TopUsers tu JOIN PostSummary ps ON tu.UserId = ps.OwnerUserId LEFT JOIN ClosedPosts cp ON ps.PostId = cp.ClosedPostId JOIN ReputationBreakdown rb ON tu.UserId = rb.UserId
// WHERE ps.CommentCount > 3 AND (cp.CloseCount IS NULL OR cp.CloseCount < 3) ORDER BY tu.Reputation DESC, ps.VoteCount DESC;
//
// Rank is never read.
fn q3379(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let rb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |n, c| n + match c {
        Some(1) => 3,
        Some(2) => 2,
        Some(3) => 1,
        _ => 0,
    });
    let v = drain((&ps).filt(|a| a[0] > 3).and((&cp).opt().filt(|c: Option<(i64, i64)>| c.map_or(true, |c| c.0 < 3))).and(owner_user.select(Ident::<User>::new().and(&rb))));
    rows(v.into_iter().map(|(p, ((a, c), (u, b)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        match c {
            Some((_, d)) => f.extend([post_fields(db, p, &["title"]).remove(0), V::T(d)]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(V::I(b));
        f.push(V::S(if a[1] > 10 { "Highly Voted" } else if a[1] >= 5 { "Moderately Voted" } else { "Low Voted" }));
        f.push(V::S(if b > 5 { "Active Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH RECURSIVE UserPostHierarchy AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN P.Score <= 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(P.ViewCount) AS TotalViews, SUM(P.AnswerCount) AS TotalAnswers FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// RankedUsers AS (SELECT *, RANK() OVER (ORDER BY PostCount DESC, TotalViews DESC) AS UserRank FROM UserPostHierarchy),
// PostScoreHistory AS (SELECT PH.CreationDate, P.Id AS PostId, PH.Comment AS CloseReason, PH.UserDisplayName AS Editor, PH.PostHistoryTypeId,
//        CASE WHEN PH.PostHistoryTypeId = 10 THEN 'Closed' WHEN PH.PostHistoryTypeId = 11 THEN 'Reopened' ELSE 'Other' END AS Action, P.Score FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id
//     WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AggregatePostHistory AS (SELECT PostId, COUNT(*) AS HistoryCount, COUNT(DISTINCT CloseReason) AS DistinctCloseReasons, SUM(CASE WHEN Action = 'Closed' THEN 1 ELSE 0 END) AS CloseCount,
//        SUM(CASE WHEN Action = 'Reopened' THEN 1 ELSE 0 END) AS ReopenCount FROM PostScoreHistory GROUP BY PostId)
// SELECT RU.UserId, RU.DisplayName, RU.PostCount, RU.PositivePosts, RU.NegativePosts, RU.TotalViews, RU.TotalAnswers, APH.PostId, APH.HistoryCount, APH.DistinctCloseReasons, APH.CloseCount, APH.ReopenCount,
//        CASE WHEN RU.PostCount > 10 THEN 'Active Contributor' WHEN RU.PostCount BETWEEN 5 AND 10 THEN 'Moderately Active' ELSE 'Occasional User' END AS UserActivityLevel
// FROM RankedUsers RU LEFT JOIN AggregatePostHistory APH ON RU.UserId = APH.PostId WHERE RU.UserRank <= 50 ORDER BY RU.UserRank, RU.TotalViews DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. `RU.UserId = APH.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q32673(db: &'static So) -> String {
    let Post { score, view_count, answer_count, .. } = &db.post;
    let uph = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt()).and(answer_count.opt())).opt()).fold([0i64; 7], |a, p| match p {
        Some(((s, w), n)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s <= 0) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + n.is_some() as i64, a[6] + n.unwrap_or(0)],
        None => a,
    });
    let views = |a: [i64; 7]| if a[3] > 0 { Some(a[4]) } else { None };
    let r = ranked(drain(&uph), |&(_, a)| (Reverse(a[0]), views(a).is_none(), Reverse(views(a))), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 50).collect());
    let PostHistory { post, comment, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let aph = db.post_history.with(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post).select(comment.opt().and(post_history_type_id)).buf_fold(|rows| {
        [rows.len() as i64, distinct_some(rows.iter().map(|r| r.0)), rows.iter().filter(|r| r.1 == 10).count() as i64, rows.iter().filter(|r| r.1 == 11).count() as i64]
    });
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    type R = ((Id<User>, [i64; 7]), i64);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|((u, _), _): R| u).select(&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&aph)).opt())));
    rows(v.into_iter().map(|(_, (((u, a), _), h))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), nullable(a[1], a[0]), nullable(a[2], a[0]), nullable(a[4], a[3]), nullable(a[6], a[5])]);
        match h {
            Some((p, h)) => {
                f.push(V::I(db.post.origid.get(p).unwrap()));
                f.extend(h.map(V::I));
            }
            None => f.extend((0..5).map(|_| V::Null)),
        }
        f.push(V::S(if a[0] > 10 { "Active Contributor" } else if a[0] >= 5 { "Moderately Active" } else { "Occasional User" }));
        row(f)
    }))
}

// WITH TagStats AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 AND ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS ClosedQuestionCount
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN PostHistory ph ON ph.PostId = p.Id GROUP BY t.TagName),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS ContributedPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgesReceived
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// TopTags AS (SELECT TagName, PostCount, QuestionCount, AnswerCount, ClosedQuestionCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagStats),
// TopUsers AS (SELECT UserId, DisplayName, ContributedPosts, QuestionsAsked, AnswersGiven, BadgesReceived, ROW_NUMBER() OVER (ORDER BY ContributedPosts DESC) AS UserRank FROM ActiveUsers)
// SELECT t.TagName, t.PostCount, t.QuestionCount, t.AnswerCount, t.ClosedQuestionCount, u.DisplayName AS TopContributor, u.ContributedPosts, u.QuestionsAsked, u.AnswersGiven, u.BadgesReceived
// FROM TopTags t JOIN TopUsers u ON t.QuestionCount > 0 AND u.QuestionsAsked = (SELECT MAX(QuestionsAsked) FROM ActiveUsers WHERE QuestionsAsked > 0) WHERE t.TagRank <= 10 ORDER BY t.PostCount DESC;
//
// The ON clause names u only through an uncorrelated scalar, so the top tags are crossed with the users at the maximum QuestionsAsked. UserRank is never read.
fn q29472(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts = db
        .tag
        .with(&by_tag)
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select(Ident::<Post>::new().and(&db.post.post_type_id).and(history_of(db).select(&db.post_history.post_history_type_id).opt())))
        .buf_fold(|rows| {
            let q = rows.iter().filter(|r| r.0 .1 == 1).count() as i64;
            let a = rows.iter().filter(|r| r.0 .1 == 2).count() as i64;
            let c = rows.iter().filter(|r| r.0 .1 == 1 && r.1 == Some(10)).count() as i64;
            [distinct_some(rows.iter().map(|r| Some(r.0 .0))), q, a, c]
        });
    let tt = top_n(drain(&ts), |&(t, a)| (Reverse(a[0]), t), 10);
    let tt = rel(tt);
    let au = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.is_some() as i64]);
    let most = (&au).filt(|a| a[0] > 0).fold_flat(i64::MIN, |m, a| m.max(a[0]));
    let dp = db.user.with((&db.user.reputation).gt(100)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let top_users = rel(drain((&au).filt(move |a| a[0] == most).and(&dp)));
    let mut v = Vec::new();
    (&tt).filt(|(_, a): (Str, [i64; 4])| a[1] > 0).cross(&top_users).drive(|_, ((t, a), (u, (s, n)))| v.push((t, a, u, s, n)));
    rows(v.into_iter().map(|(t, a, u, s, n)| {
        let mut f = vec![V::S(t)];
        f.extend(a.map(V::I));
        f.extend([user_col(db, u, "name"), V::I(n), V::I(s[0]), V::I(s[1]), V::I(s[2])]);
        row(f)
    }))
}

// WITH UserPostStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.Score >= 0 THEN 1 ELSE 0 END) AS PositiveScoreCount, SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScoreCount, ROW_NUMBER() OVER (ORDER BY COUNT(p.Id) DESC) AS PostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// FilteredBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// UserDetails AS (SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.QuestionCount, ups.AnswerCount, ups.PositiveScoreCount, ups.NegativeScoreCount, COALESCE(b.GoldBadges, 0) AS GoldBadges,
//        COALESCE(b.SilverBadges, 0) AS SilverBadges, COALESCE(b.BronzeBadges, 0) AS BronzeBadges FROM UserPostStatistics ups LEFT JOIN FilteredBadges b ON ups.UserId = b.UserId)
// SELECT ud.DisplayName, ud.TotalPosts, ud.QuestionCount, ud.AnswerCount, ud.PositiveScoreCount, ud.NegativeScoreCount, ud.GoldBadges, ud.SilverBadges, ud.BronzeBadges,
//        CASE WHEN (ud.GoldBadges + ud.SilverBadges + ud.BronzeBadges) > 0 THEN 'Has Badges' ELSE 'No Badges' END AS BadgeStatus,
//        CASE WHEN ud.TotalPosts IS NULL OR ud.TotalPosts = 0 THEN 'No Activity' WHEN ud.AnswerCount > 0 AND ud.QuestionCount > 0 THEN 'Active Contributor' ELSE 'Lurker' END AS UserActivityStatus
// FROM UserDetails ud WHERE ud.TotalPosts > 10 ORDER BY ud.TotalPosts DESC, ud.DisplayName ASC;
//
// PostRank is never read.
fn q21370(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score))).fold([0i64; 5], |a, (t, s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s >= 0) as i64, a[4] + (s < 0) as i64]
    });
    let fb = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    rows(drain((&ups).filt(|a| a[0] > 10).and((&fb).opt())).into_iter().map(|(u, (a, b))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        f.push(V::S(if b[0] + b[1] + b[2] > 0 { "Has Badges" } else { "No Badges" }));
        f.push(V::S(if a[2] > 0 && a[1] > 0 { "Active Contributor" } else { "Lurker" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, up.Reputation AS OwnerReputation, COUNT(a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, SUM(v.BountyAmount) AS TotalBounty
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId JOIN Users up ON p.OwnerUserId = up.Id LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, up.Reputation),
// RecentActivity AS (SELECT p.Id AS PostId, p.LastActivityDate, COUNT(c.Id) AS CommentCount, MAX(ph.CreationDate) AS LastEditDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.LastActivityDate),
// DetailedPostStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerReputation, ra.CommentCount, ra.LastActivityDate, ra.LastEditDate, rp.TotalBounty, rp.AnswerCount,
//        COALESCE(rp.TotalBounty, 0) + (CASE WHEN ra.LastActivityDate > CURRENT_TIMESTAMP - INTERVAL '6 months' THEN 1 ELSE 0 END) AS ActivityScore FROM RankedPosts rp JOIN RecentActivity ra ON rp.PostId = ra.PostId),
// FilteredPosts AS (SELECT dps.*, DENSE_RANK() OVER (ORDER BY ActivityScore DESC) AS ActivityRank FROM DetailedPostStats dps WHERE dps.OwnerReputation > 100 AND (dps.AnswerCount > 5 OR dps.TotalBounty > 0))
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.OwnerReputation, fp.CommentCount, fp.LastActivityDate, fp.LastEditDate, fp.TotalBounty, fp.AnswerCount, fp.ActivityScore, fp.ActivityRank
// FROM FilteredPosts fp WHERE fp.ActivityRank <= 10 ORDER BY fp.ActivityScore DESC;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so LastActivityDate is compared in the session zone (America/New_York). UserPostRank is never read. RecentActivity is computed only for the
// posts that pass FilteredPosts' WHERE, the only ones the join reads.
fn q34132(db: &'static So) -> String {
    let Post { post_type_id, owner_user, last_activity_date, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100))))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(bounty.opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let keep: MatSet<Id<Post>> = db.post.with((&rp).filt(|a| a[0] > 5 || (a[1] > 0 && a[2] > 0))).collect();
    let ra = (&keep)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, m.max(d.unwrap_or(i64::MIN))));
    let since = add_months(utc_to_ny(now_utc()), -6);
    let score = |p: Id<Post>, a: [i64; 3]| a[2] + (last_activity_date.get(p).unwrap() > since) as i64;
    let v = ranked(drain((&rp).and(&ra)), |&(p, (a, _))| Reverse(score(p, a)), true);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, (a, (c, d))), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "rep"]);
        f.extend([V::I(c), V::T(last_activity_date.get(p).unwrap()), tmax(d), nullable(a[2], a[1]), V::I(a[0]), V::I(score(p, a)), V::I(r)]);
        row(f)
    }))
}

// WITH TagArray AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotesCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotesCount,
//        SUM(CASE WHEN v.UserId IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotesGiven FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PopularTags AS (SELECT ta.Tag, COUNT(ta.PostId) AS TagUsageCount FROM TagArray ta GROUP BY ta.Tag ORDER BY TagUsageCount DESC LIMIT 10),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName, COALESCE(v.UpVotes, 0) AS UpVotes,
//        COALESCE(v.DownVotes, 0) AS DownVotes FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.OwnerDisplayName, COALESCE(pt.Tag, 'Uncategorized') AS Tag, pd.UpVotes, pd.DownVotes,
//        us.UpVotesCount, us.DownVotesCount, us.TotalVotesGiven
// FROM PostDetails pd LEFT JOIN PopularTags pt ON pd.Title ILIKE '%' || pt.Tag || '%' LEFT JOIN UserVoteStats us ON pd.OwnerDisplayName = us.DisplayName
// ORDER BY pd.ViewCount DESC, pd.CreationDate DESC LIMIT 50;
//
// Every question yields at least one row, and the order reads only its own columns, so the fifty first questions are picked before the joins. The title match goes through
// select_where over those titles, lowercased as ILIKE does.
fn q28793(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, creation_date, title, owner_user, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(creation_date.get(p).unwrap()), p)
    };
    let freq = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let pt = rel(top_n(drain(&freq), |&(t, n)| (Reverse(n), t), 10));
    let ptk: HashIdx<Str, (Str, i64)> = (&pt).map(|(t, _)| t).inv().select(&pt).collect();
    let tp = top_n(drain(db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new())), |&(p, _)| key(p), 50);
    let tp: MatSet<Id<Post>> = rel(tp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let titles: MatSet<Str> = (&tp).select(title).collect();
    let hit: HashIdx<Str, (Str, i64)> = (&titles).select_where(&ptk, |s: Str, t: Str| s.to_lowercase().contains(&t.to_lowercase())).collect();
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]);
    let ur = rel(drain(&uv));
    let by_name: HashIdx<Str, (Id<User>, [i64; 3])> = (&ur).map(|(u, _)| u).select(&db.user.display_name).inv().select(&ur).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let name = owner_user.select(&db.user.display_name).opt().map(|n: Option<Str>| n.unwrap_or("Community User"));
    let v = drain((&pv).and(title.select(&hit).opt()).and(name.select((&by_name).opt())));
    let v = top_n(v, |&(p, ((_, t), u))| (key(p), t.map(|x| x.0), u.map(|x| x.0)), 50);
    rows(v.into_iter().map(|(p, ((a, t), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "answers", "comments"]);
        f.push(V::S(owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.push(V::S(t.map_or("Uncategorized", |x| x.0)));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match u {
            Some((_, s)) => s.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, p.PostTypeId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RecentRank FROM Posts p WHERE p.PostTypeId IN (1, 2)),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopContributors AS (SELECT us.UserId, us.DisplayName, us.TotalPosts, us.QuestionsCount, us.AnswersCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges, COUNT(DISTINCT ph.Id) AS PostHistoryCount,
//        AVG(COALESCE(pt.ViewCount, 0)) AS AvgPostViews FROM UserStats us LEFT JOIN Posts p ON us.UserId = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN RankedPosts pt ON p.Id = pt.PostId
//     GROUP BY us.UserId, us.DisplayName, us.TotalPosts, us.QuestionsCount, us.AnswersCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges)
// SELECT tc.DisplayName, tc.TotalPosts, tc.QuestionsCount, tc.AnswersCount, tc.GoldBadges, tc.SilverBadges, tc.BronzeBadges, tc.PostHistoryCount, tc.AvgPostViews
// FROM TopContributors tc WHERE tc.TotalPosts > 10 ORDER BY tc.TotalPosts DESC, tc.AvgPostViews DESC;
//
// RankedPosts is one row per question or answer and its ranks are never read, so it is the post itself, restricted to types 1 and 2.
fn q7963(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (t, c)| [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + (c == Some(1)) as i64, a[4] + (c == Some(2)) as i64, a[5] + (c == Some(3)) as i64]);
    let pt_views = Ident::<Post>::new().with(post_type_id.is_in([1, 2])).select(view_count.opt()).opt().map(|w: Option<Option<i64>>| w.flatten().unwrap_or(0));
    let tc = db
        .user
        .with((&us).filt(|a| a[0] > 10))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(history_of(db).opt().and(pt_views)).opt())
        .buf_fold(|rows| {
            let n = rows.len() as i64;
            let s: i64 = rows.iter().map(|r| r.map_or(0, |x| x.1)).sum();
            (distinct_some(rows.iter().map(|r| r.and_then(|x| x.0))), s, n)
        });
    let v = drain((&us).and(&tc));
    rows(v.into_iter().map(|(u, (a, (h, s, n)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([V::I(h), avg(s, n)]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, p.OwnerUserId, COALESCE(u.DisplayName, 'Deleted User') AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.PostTypeId, p.OwnerUserId, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment, CASE WHEN ph.PostHistoryTypeId = 10 THEN 'Post Closed' WHEN ph.PostHistoryTypeId = 11 THEN 'Post Reopened' ELSE 'Other Reason' END AS CloseReason
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.OwnerDisplayName, rp.CreationDate, COALESCE(cp.CloseReason, 'Not Closed') AS CloseStatus
//     FROM RecentPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.rn <= 5)
// SELECT tp.Title, tp.CommentCount, tp.UpVotes, tp.DownVotes, tp.OwnerDisplayName, tp.CreationDate, CASE WHEN tp.CloseStatus <> 'Not Closed' THEN 'Closed: ' || tp.CloseStatus ELSE 'Open' END AS Status,
//        CASE WHEN (tp.UpVotes - tp.DownVotes) < 0 THEN 'Negative Feedback' ELSE 'Feedback OK' END AS FeedbackAssessment
// FROM TopPosts tp ORDER BY tp.UpVotes - tp.DownVotes DESC, tp.CommentCount DESC;
//
// rn reads only base columns, so each type's five newest posts are picked first and the comment x vote product is driven for those alone.
fn q23777(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11]))).select(&db.post_history.post_history_type_id);
    rows(drain((&rp).and(closes.opt())).into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend(a.map(V::I));
        f.push(V::S(owner_user.get(p).map_or("Deleted User", |u| db.user.display_name.get(u).unwrap())));
        f.extend(post_fields(db, p, &["created"]));
        f.push(V::S(match c {
            Some(10) => "Closed: Post Closed",
            Some(_) => "Closed: Post Reopened",
            None => "Open",
        }));
        f.push(V::S(if a[1] - a[2] < 0 { "Negative Feedback" } else { "Feedback OK" }));
        row(f)
    }))
}

// Rewritten (rewrites/25687.sql): the final ORDER BY is tie-broken on PostId, tag.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, p.Tags, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TagStatistics AS (SELECT tag, COUNT(*) AS PostCount, AVG(ViewCount) AS AvgViewCount, AVG(Score) AS AvgScore FROM (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS tag,
//        p.ViewCount, p.Score FROM Posts p WHERE p.PostTypeId = 1) AS TagData GROUP BY tag),
// TopAuthors AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName
//     HAVING COUNT(p.Id) > 5),
// FinalBenchmark AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, ts.tag, ts.PostCount, ts.AvgViewCount, ts.AvgScore, ta.UserId, ta.DisplayName AS TopAuthorName,
//        ta.PostCount AS TopAuthorPostCount, ta.TotalScore AS TopAuthorTotalScore FROM RankedPosts rp JOIN TagStatistics ts ON POSITION(ts.tag IN rp.Tags) > 0 JOIN TopAuthors ta ON rp.OwnerUserId = ta.UserId
//     WHERE rp.PostRank = 1)
// SELECT PostId, Title, OwnerDisplayName, CreationDate, tag, PostCount, AvgViewCount, AvgScore, UserId AS TopAuthorId, TopAuthorName, TopAuthorPostCount, TopAuthorTotalScore
// FROM FinalBenchmark ORDER BY AvgScore DESC, AvgViewCount DESC, PostId, tag LIMIT 50;
//
// POSITION(ts.tag IN rp.Tags) is a substring test on the whole Tags text, run with select_where over the distinct Tags strings of the ranked posts.
fn q25687(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, tags_str, view_count, origid, .. } = &db.post;
    let ta = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user.select((&ta).filt(|a| a[0] > 5))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 1, true);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tsg = db
        .post
        .with(post_type_id.eq(1))
        .select(tags_str.flat_map(tag_list).and(view_count.opt()).and(score))
        .group_by(Same::<((Str, Option<i64>), i64)>::new().map(|((t, _), _): ((Str, Option<i64>), i64)| t))
        .select(Same::<((Str, Option<i64>), i64)>::new())
        .fold([0i64; 4], |a, ((_, w), s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let tr = rel(drain(&tsg));
    let tk: HashIdx<Str, (Str, [i64; 4])> = (&tr).map(|(t, _)| t).inv().select(&tr).collect();
    let strs: MatSet<Str> = (&rp).select(tags_str).collect();
    let hit: HashIdx<Str, (Str, [i64; 4])> = (&strs).select_where(&tk, |s: Str, t: Str| s.contains(t)).collect();
    let v = drain((&rp).select(tags_str.select(&hit).and(owner_user.select(Ident::<User>::new().and(&ta)))));
    let avgf = |s: i64, n: i64| s as f64 / n as f64;
    let v = top_n(v, |&(p, ((t, a), _))| (Reverse(fkey(avgf(a[3], a[0]))), a[1] == 0, Reverse(fkey(avgf(a[2], a[1]))), origid.get(p).unwrap(), t), 50);
    rows(v.into_iter().map(|(p, ((t, a), (u, s)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([V::S(t), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0])]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(s[0]), V::I(s[1])]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, CASE WHEN Reputation < 100 THEN 'Low' WHEN Reputation < 1000 THEN 'Medium' ELSE 'High' END AS ReputationCategory FROM Users),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, COALESCE(a.Score, 0) AS AcceptedAnswerScore, p.Score AS PostScore, COUNT(c.Id) AS CommentCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes FROM Posts p LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id
//     LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.Score, a.Score),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, ur.ReputationCategory, SUM(ps.UpVotes) AS TotalUpVotes, SUM(ps.DownVotes) AS TotalDownVotes,
//        DENSE_RANK() OVER (PARTITION BY ur.ReputationCategory ORDER BY SUM(ps.UpVotes) DESC) AS Rank FROM UserReputation ur JOIN PostStats ps ON ur.UserId = ps.OwnerUserId JOIN Users u ON ps.OwnerUserId = u.Id
//     GROUP BY u.Id, u.DisplayName, ur.ReputationCategory),
// PostAnalytics AS (SELECT ps.PostId, ps.Title, ps.PostScore, ps.CommentCount, u.DisplayName, COALESCE(tu.Rank, 0) AS UserRank, (ps.UpVotes - ps.DownVotes) AS NetVotes,
//        ROUND((COALESCE(ps.CommentCount, 0) + ps.UpVotes - ps.DownVotes) * 1.0 / NULLIF(ps.CommentCount + 1, 0), 2) AS EngagementScore FROM PostStats ps LEFT JOIN Users u ON ps.OwnerUserId = u.Id
//     LEFT JOIN TopUsers tu ON u.Id = tu.UserId)
// SELECT pa.PostId, pa.Title, pa.PostScore, pa.CommentCount, pa.DisplayName, pa.UserRank, pa.NetVotes, pa.EngagementScore FROM PostAnalytics pa WHERE pa.UserRank <= 5 ORDER BY pa.PostScore DESC, pa.NetVotes DESC;
//
// The accepted-answer join matches at most one row and its score is never read, so it does not change the counts.
fn q3676(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tu = db.post.with(&ps).group_by(owner_user).select(&ps).fold(0i64, |n, a| n + a[1]);
    let cat = |u: Id<User>| {
        let r = db.user.reputation.get(u).unwrap();
        if r < 100 { 0 } else if r < 1000 { 1 } else { 2 }
    };
    let r = per_group(ranked(drain(&tu), |&(u, s)| (cat(u), Reverse(s)), true), |&(u, _)| cat(u));
    let rr = rel(r.into_iter().map(|((u, _), k)| (u, k)).collect());
    let rank_of: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let v = drain((&ps).and(owner_user.select(&rank_of).opt()).filt(|(_, r): ([i64; 3], Option<(Id<User>, i64)>)| r.map_or(0, |x| x.1) <= 5));
    rows(v.into_iter().map(|(p, (a, r))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.push(V::I(a[0]));
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::I(r.map_or(0, |x| x.1)), V::I(a[1] - a[2]), V::F((((a[0] + a[1] - a[2]) as f64 * 1.0 / (a[0] + 1) as f64) * 100.0).round() / 100.0)]);
        row(f)
    }))
}

// WITH RecentPostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.AnswerCount, P.CommentCount, U.Reputation AS OwnerReputation, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, DENSE_RANK() OVER (ORDER BY P.ViewCount DESC) AS RankByViews FROM Posts P INNER JOIN Users U ON P.OwnerUserId = U.Id
//     LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.AnswerCount, P.CommentCount, U.Reputation),
// TopPosts AS (SELECT RPS.PostId, RPS.Title, RPS.ViewCount, RPS.AnswerCount, RPS.CommentCount, RPS.OwnerReputation, RPS.DownVotes, RPS.UpVotes, AVG(RPS.OwnerReputation) OVER () AS AvgOwnerReputation
//     FROM RecentPostStats RPS WHERE RPS.RankByViews <= 10),
// PostComments AS (SELECT C.PostId, COUNT(*) AS TotalComments FROM Comments C GROUP BY C.PostId),
// FinalResult AS (SELECT TP.PostId, TP.Title, TP.ViewCount, TP.AnswerCount, TP.CommentCount, TP.OwnerReputation, COALESCE(PC.TotalComments, 0) AS TotalComments, TP.DownVotes, TP.UpVotes,
//        TP.OwnerReputation - TP.AvgOwnerReputation AS ReputationDifference FROM TopPosts TP LEFT OUTER JOIN PostComments PC ON TP.PostId = PC.PostId)
// SELECT FR.PostId, FR.Title, FR.ViewCount, FR.AnswerCount, FR.CommentCount, FR.OwnerReputation, FR.TotalComments, FR.DownVotes, FR.UpVotes,
//        CASE WHEN FR.ReputationDifference > 0 THEN 'Above Average' WHEN FR.ReputationDifference < 0 THEN 'Below Average' ELSE 'Average' END AS ReputationStatus
// FROM FinalResult FR ORDER BY FR.ViewCount DESC;
fn q32518(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(current_date(), -6))).with(owner_user).select(view_count.opt()));
    let r = ranked(v, |&(_, w)| (w.is_none(), Reverse(w)), true);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let (rs, rn) = (&tp).select(owner_user.select(&db.user.reputation)).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(3)) as i64, a[1] + (t == Some(2)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&vc).and(&cc)).into_iter().map(|(p, (a, c))| {
        let rep = db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
        let d = rep as f64 - rs as f64 / rn as f64;
        let mut f = post_fields(db, p, &["id", "title", "views", "answers", "comments", "rep"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(if d > 0.0 { "Above Average" } else if d < 0.0 { "Below Average" } else { "Average" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE 0 END) AS TotalQuestionScore, SUM(CASE WHEN p.PostTypeId = 2 THEN p.Score ELSE 0 END) AS TotalAnswerScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalQuestionScore, TotalAnswerScore, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, pt.Name AS PostTypeName, COUNT(c.Id) AS CommentCount FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, pt.Name),
// TopRecentPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, PostTypeName, CommentCount, ROW_NUMBER() OVER (ORDER BY CreationDate DESC) AS RecentRank FROM RecentPosts)
// SELECT tu.DisplayName AS TopUser, tu.Reputation AS UserReputation, trp.Title AS RecentPostTitle, trp.CreationDate AS RecentPostDate, trp.OwnerDisplayName, trp.PostTypeName, trp.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY tu.UserId ORDER BY trp.CreationDate DESC) AS UserPostRank
// FROM TopUsers tu JOIN TopRecentPosts trp ON tu.DisplayName = trp.OwnerDisplayName WHERE tu.Rank <= 10 AND trp.RecentRank <= 5 ORDER BY tu.Rank, trp.CreationDate DESC;
//
// Only the name and reputation are read from TopUsers, so its counts are not computed. Both ROW_NUMBERs read only base columns, so the ten users and the five newest posts
// are picked first; the RecentRank ties on CreationDate are broken by Id.
fn q6128(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(tu.into_iter().enumerate().map(|(i, (u, _))| (db.user.display_name.get(u).unwrap(), (u, i as i64 + 1))).collect());
    let by_name: HashIdx<Str, (Str, (Id<User>, i64))> = (&tu).map(|(n, _)| n).inv().select(&tu).collect();
    let rp = top_n(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(creation_date)), |&(p, d)| (Reverse(d), p), 5);
    let rp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(&db.user.display_name).select(&by_name)));
    let v = ranked(v, |&(p, (_, (_, (u, _))))| (u, Reverse(creation_date.get(p).unwrap()), p), false);
    let v = per_group(v, |&(_, (_, (_, (u, _))))| u);
    rows(v.into_iter().map(|((p, (c, (_, (u, _)))), r)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "owner", "type"]));
        f.extend([V::I(c), V::I(r)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("1802", q1802),
    ("28589", q28589),
    ("34720", q34720),
    ("1856", q1856),
    ("33603", q33603),
    ("8566", q8566),
    ("32898", q32898),
    ("5235", q5235),
    ("20426", q20426),
    ("841", q841),
    ("20632", q20632),
    ("23109", q23109),
    ("31375", q31375),
    ("30645", q30645),
    ("32925", q32925),
    ("32729", q32729),
    ("26704", q26704),
    ("30110", q30110),
    ("573", q573),
    ("1205", q1205),
    ("7994", q7994),
    ("4808", q4808),
    ("3854", q3854),
    ("3765", q3765),
    ("32215", q32215),
    ("32829", q32829),
    ("34199", q34199),
    ("9106", q9106),
    ("22034", q22034),
    ("20306", q20306),
    ("33066", q33066),
    ("26464", q26464),
    ("33086", q33086),
    ("34096", q34096),
    ("31826", q31826),
    ("22940", q22940),
    ("27852", q27852),
    ("2894", q2894),
    ("838", q838),
    ("25771", q25771),
    ("26803", q26803),
    ("2640", q2640),
    ("8453", q8453),
    ("1472", q1472),
    ("7053", q7053),
    ("2851", q2851),
    ("21486", q21486),
    ("2234", q2234),
    ("20948", q20948),
    ("32850", q32850),
    ("331", q331),
    ("659", q659),
    ("4968", q4968),
    ("25546", q25546),
    ("1597", q1597),
    ("1804", q1804),
    ("6345", q6345),
    ("33546", q33546),
    ("31131", q31131),
    ("31544", q31544),
    ("22826", q22826),
    ("30688", q30688),
    ("24597", q24597),
    ("30959", q30959),
    ("32360", q32360),
    ("1427", q1427),
    ("30171", q30171),
    ("9060", q9060),
    ("21303", q21303),
    ("24053", q24053),
    ("3787", q3787),
    ("23064", q23064),
    ("4448", q4448),
    ("32642", q32642),
    ("1794", q1794),
    ("34071", q34071),
    ("32727", q32727),
    ("5210", q5210),
    ("22849", q22849),
    ("23016", q23016),
    ("23193", q23193),
    ("23454", q23454),
    ("4835", q4835),
    ("22713", q22713),
    ("22926", q22926),
    ("8986", q8986),
    ("3379", q3379),
    ("32673", q32673),
    ("29472", q29472),
    ("21370", q21370),
    ("34132", q34132),
    ("28793", q28793),
    ("7963", q7963),
    ("23777", q23777),
    ("25687", q25687),
    ("3676", q3676),
    ("32518", q32518),
    ("6128", q6128),
];
