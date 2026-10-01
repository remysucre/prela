use harness::prelude::*;
use std::cmp::Reverse;

fn badge_classes(db: &'static So) -> Fold<Id<User>, [i64; 3]> {
    db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64])
}

// WITH TagCounts AS (SELECT UNNEST(string_to_array(substring(Tags, 2, length(Tags) - 2), '><')) AS Tag, Id AS PostId FROM Posts WHERE PostTypeId = 1),
// TopTags AS (SELECT Tag, COUNT(*) AS Count FROM TagCounts GROUP BY Tag ORDER BY Count DESC LIMIT 10),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END), 0) AS AcceptedAnswers
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId),
// UserSummary AS (SELECT UA.UserId, UA.DisplayName, UA.TotalPosts, UA.TotalAnswers, UA.AcceptedAnswers, COALESCE(UB.GoldBadges, 0) AS GoldBadges,
//        COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges FROM UserActivity UA LEFT JOIN UserBadges UB ON UA.UserId = UB.UserId)
// SELECT U.DisplayName, U.TotalPosts, U.TotalAnswers, U.AcceptedAnswers, U.GoldBadges, U.SilverBadges, U.BronzeBadges, T.Tag, T.Count AS TagCount
// FROM UserSummary U JOIN TopTags T ON U.TotalAnswers > 0 ORDER BY U.TotalPosts DESC, U.AcceptedAnswers DESC, T.Count DESC;
//
// The ON clause names only U, so the answering users and the top tags are crossed.
fn q25027(db: &'static So) -> String {
    let Post { post_type_id, tags_str, accepted_answer_id, .. } = &db.post;
    let freq = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let top = rel(top_n(drain(&freq), |&(t, n)| (Reverse(n), t), 10));
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, x)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1 && x.is_some()) as i64],
        None => a,
    });
    let ub = badge_classes(db);
    let mut v = Vec::new();
    (&ua).filt(|a| a[1] > 0).and((&ub).opt()).cross(&top).drive(|(u, _), ((a, b), (t, n))| v.push((u, a, b.unwrap_or([0; 3]), t, n)));
    rows(v.into_iter().map(|(u, a, b, t, n)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS UserTotalPosts FROM Posts p WHERE p.PostTypeId = 1 AND p.Title IS NOT NULL),
// BadgedUsers AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id HAVING COUNT(b.Id) > 0),
// QuestionStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS QuestionCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, AVG(COALESCE(p.Score, 0)) AS AvgScore
//     FROM Posts p WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId),
// FinalReport AS (SELECT u.DisplayName, u.Reputation, qs.QuestionCount, qs.TotalViews, qs.AvgScore, COALESCE(bu.BadgeCount, 0) AS BadgeCount,
//        CASE WHEN qs.AvgScore IS NULL THEN 'No Score' WHEN qs.AvgScore > 5 THEN 'High Achiever' ELSE 'Needs Improvement' END AS PerformanceCategory
//     FROM Users u LEFT JOIN QuestionStats qs ON u.Id = qs.OwnerUserId LEFT JOIN BadgedUsers bu ON u.Id = bu.UserId WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users))
// SELECT fr.DisplayName, fr.Reputation, fr.QuestionCount, fr.TotalViews, fr.AvgScore, fr.BadgeCount, fr.PerformanceCategory
// FROM FinalReport fr WHERE fr.QuestionCount BETWEEN 5 AND 15 AND fr.TotalViews IS NOT NULL ORDER BY fr.AvgScore DESC NULLS LAST LIMIT 10;
//
// RankedPosts is never read. `Reputation > AVG(Reputation)` is compared exactly, as `Reputation * n > sum`.
fn q22118(db: &'static So) -> String {
    let (s, n) = (&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let Post { post_type_id, owner_user, view_count, score, .. } = &db.post;
    let qs = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(view_count.opt().and(score)).fold([0i64; 3], |a, (w, sc)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + sc]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |k, _| k + 1);
    let v = drain(db.user.with((&db.user.reputation).filt(move |r| r * n > s)).select((&qs).filt(|a| a[0] >= 5 && a[0] <= 15).and((&bc).opt())));
    let v = top_n(v, |&(u, (a, _))| (Reverse(fkey(a[2] as f64 / a[0] as f64)), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), V::I(b.unwrap_or(0))]);
        f.push(V::S(if a[2] as f64 / a[0] as f64 > 5.0 { "High Achiever" } else { "Needs Improvement" }));
        row(f)
    }))
}

// WITH RECURSIVE UserContribution AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostsActivity AS (SELECT p.OwnerUserId, p.CreationDate, CASE WHEN COUNT(*) OVER (PARTITION BY p.OwnerUserId) > 10 THEN 'Active' ELSE 'Inactive' END AS FenstonRanking,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS ActivityRank FROM Posts p)
// SELECT uc.UserId, uc.DisplayName, uc.TotalPosts, uc.TotalQuestions, uc.TotalAnswers, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, uc.TotalBounties, pa.ActivityRank,
//        CASE WHEN uc.TotalPosts > 5 THEN 'Frequent Contributor' WHEN COALESCE(ub.GoldBadges, 0) > 0 THEN 'Expert Contributor' ELSE 'Novice Contributor' END AS ContributorCategory
// FROM UserContribution uc LEFT JOIN UserBadges ub ON uc.UserId = ub.UserId LEFT JOIN PostsActivity pa ON uc.UserId = pa.OwnerUserId
// WHERE uc.TotalPosts > 0 ORDER BY uc.TotalPosts DESC, uc.DisplayName ASC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q34447(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let uc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(bounty.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let dp = user_distinct_posts(db);
    let ub = badge_classes(db);
    let pa = per_group(ranked(drain(db.post.select(owner_user)), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let pa = rel(pa);
    type R = ((Id<Post>, Id<User>), i64);
    let stats = Ident::<User>::new().and(&uc).and((&dp).filt(|n| n > 0)).and((&ub).opt());
    let v = drain((&pa).select(Same::<R>::new().map(|(_, r): R| r).and(Same::<R>::new().map(|((_, u), _): R| u).select(stats))));
    rows(v.into_iter().map(|(_, (r, (((u, a), n), b)))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[1]), V::I(a[0])]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[2]), V::I(r)]);
        f.push(V::S(if n > 5 { "Frequent Contributor" } else if b[0] > 0 { "Expert Contributor" } else { "Novice Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Body, p.Tags, COUNT(a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Body, p.Tags, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// UserRankedPosts AS (SELECT r.*, u.DisplayName AS Author, u.GoldBadges, u.SilverBadges, u.BronzeBadges, u.PostCount, u.TotalViews
//     FROM RankedPosts r JOIN UserStats u ON r.OwnerUserId = u.UserId WHERE r.PostRank <= 5)
// SELECT PostId, Title, Author, CreationDate, Body, Tags, AnswerCount, Upvotes, Downvotes, GoldBadges, SilverBadges, BronzeBadges, PostCount, TotalViews
// FROM UserRankedPosts ORDER BY CreationDate DESC;
//
// PostRank reads only base columns, so each owner's five newest questions are picked first; the answer x vote product is driven for those,
// and the badge x post product for their owners alone.
fn q29394(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let rp = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let us = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(view_count.opt()).opt())).fold([0i64; 5], |a, (c, w)| {
        let w = w.flatten();
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
    });
    let dp = user_distinct_posts(db);
    let v = drain((&rp).and(owner_user.select(Ident::<User>::new().and(&us).and(&dp))));
    rows(v.into_iter().map(|(p, (a, ((u, s), n)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(user_col(db, u, "name"));
        f.extend(post_fields(db, p, &["created", "body", "tags"]));
        f.extend(a.map(V::I));
        f.extend([V::I(s[0]), V::I(s[1]), V::I(s[2]), V::I(n), nullable(s[4], s[3])]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, P.Score, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS UserPostRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY P.Id, P.Title, P.OwnerUserId, P.CreationDate, P.Score),
// ClosedPostDetails AS (SELECT PH.PostId, COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseVoteCount, MIN(PH.CreationDate) AS FirstCloseDate
//     FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId),
// UserBadges AS (SELECT B.UserId, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges,
//        COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges FROM Badges B GROUP BY B.UserId)
// SELECT UR.UserId, UR.DisplayName, UR.Reputation, TP.Title AS PostTitle, TP.CommentCount, CP.CloseVoteCount, CP.FirstCloseDate, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges,
//        CASE WHEN CP.CloseVoteCount > 0 THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM UserReputation UR JOIN TopPosts TP ON UR.UserId = TP.OwnerUserId LEFT JOIN ClosedPostDetails CP ON TP.PostId = CP.PostId LEFT JOIN UserBadges UB ON UR.UserId = UB.UserId
// WHERE UR.Reputation > 1000 AND TP.UserPostRank <= 5 ORDER BY UR.Reputation DESC, TP.Score DESC;
//
// UserPostRank is partitioned by owner, so the reputation filter on the owner can go first without changing it.
fn q332(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let v = drain(db.post.with(creation_date.gt(add_years(t0, -1))).select(owner_user.select(rich)));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(post_history_type_id.and(hd)).fold((0i64, i64::MAX), |(n, m), (t, d)| (n + (t == 10) as i64, m.min(d)));
    let ub = badge_classes(db);
    let v = drain((&cc).and((&cp).opt()).and(owner_user.select(Ident::<User>::new().and((&ub).opt()))));
    rows(v.into_iter().map(|(p, ((c, h), (u, b)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.push(V::I(c));
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if h.map_or(false, |(n, _)| n > 0) { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RecursivePostCTE AS (SELECT p.Id AS PostId, p.OwnerUserId, CASE WHEN p.PostTypeId = 1 THEN p.Title ELSE NULL END AS QuestionTitle, p.CreationDate, p.Score, p.ViewCount,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.ViewCount > 0),
// UserAggregates AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, SUM(p.ViewCount) AS TotalViews,
//        COALESCE(AVG(p.Score), 0) AS AvgScore
//     FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id LEFT JOIN Posts p ON p.OwnerUserId = u.Id AND p.PostTypeId = 1 WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName)
// SELECT ua.DisplayName, ua.BadgeCount, ua.GoldBadges, ua.SilverBadges, ua.BronzeBadges, ua.TotalViews, ua.AvgScore, rp.PostId, rp.QuestionTitle, rp.CreationDate, rp.Score,
//        rp.ViewCount, rp.CommentCount, CASE WHEN rp.RecentPostRank = 1 THEN 'Latest Post' ELSE 'Older Post' END AS PostCategory
// FROM UserAggregates ua LEFT JOIN RecursivePostCTE rp ON ua.UserId = rp.OwnerUserId
// WHERE (ua.BadgeCount > 5 OR ua.TotalViews > 1000) AND (rp.Score >= (SELECT AVG(Score) FROM Posts WHERE PostTypeId = 1) OR rp.PostId IS NULL)
// ORDER BY ua.TotalViews DESC, ua.AvgScore DESC;
//
// RecursivePostCTE has one row per post x comment, and RecentPostRank numbers those rows, so only one row of an owner's newest post is 'Latest Post'
// (which of that post's rows it is cannot be seen: they differ only in the comment, which is not projected). `rp.Score >= AVG(Score)` is compared exactly.
fn q22784(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, score, .. } = &db.post;
    let (qs, qn) = db.post.with(post_type_id.eq(1)).select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let ua = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(view_count.opt().and(score)).opt()))
        .fold([0i64; 8], |a, (c, p)| {
            let (w, s) = p.map_or((None, None), |(w, s)| (w, Some(s)));
            [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + s.is_some() as i64, a[7] + s.unwrap_or(0)]
        });
    let viewed = || db.post.with(view_count.gt(0));
    let cc = viewed().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let jr = drain(viewed().select(owner_user.and(comments_of(db).opt())));
    let jr = per_group(ranked(jr, |&(p, (u, c))| (u, Reverse(creation_date.get(p).unwrap()), p, c), false), |&(_, (u, _))| u);
    let jr = rel(jr.into_iter().map(|((p, (u, _)), r)| (u, (p, r))).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, (Id<Post>, i64))> = (&jr).map(|(u, _)| u).inv().select(&jr).collect();
    let v = drain(
        (&ua)
            .filt(|a| a[0] > 5 || (a[4] > 0 && a[5] > 1000))
            .and((&by_user).map(|(_, x)| x).opt())
            .filt(move |(_, r): ([i64; 8], Option<(Id<Post>, i64)>)| r.map_or(true, |(p, _)| score.get(p).unwrap() * qn >= qs)),
    );
    type J = (Id<User>, ([i64; 8], Option<(Id<Post>, i64)>));
    let v = drain(rel(v).select(Same::<J>::new().and(Same::<J>::new().flat_map(|(_, (_, r)): J| r.map(|(p, _)| p)).select(&cc).opt())));
    rows(v.into_iter().map(|(_, ((u, (a, r)), cn))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4]), if a[6] == 0 { V::F(0.0) } else { avg(a[7], a[6]) }];
        f.extend(match r {
            Some((p, k)) => vec![
                V::I(db.post.origid.get(p).unwrap()),
                if post_type_id.get(p).unwrap() == 1 { ostr(db.post.title.get(p)) } else { V::Null },
                V::T(creation_date.get(p).unwrap()),
                V::I(score.get(p).unwrap()),
                oint(view_count.get(p)),
                oint(cn),
                V::S(if k == 1 { "Latest Post" } else { "Older Post" }),
            ],
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::S("Older Post")],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(*) AS TotalBadges, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges GROUP BY UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// ActiveUsers AS (SELECT u.Id, u.DisplayName, COALESCE(ub.TotalBadges, 0) AS TotalBadges, COALESCE(ps.QuestionCount, 0) AS QuestionCount, COALESCE(ps.AnswerCount, 0) AS AnswerCount,
//        COALESCE(ps.TotalScore, 0) AS TotalScore, COALESCE(ps.TotalViews, 0) AS TotalViews
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
//     WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users) AND (ps.QuestionCount + ps.AnswerCount) > 0),
// RankedUsers AS (SELECT au.*, ROW_NUMBER() OVER (ORDER BY au.TotalScore DESC, au.TotalViews DESC) AS UserRank FROM ActiveUsers au)
// SELECT ru.DisplayName, ru.TotalBadges, ru.QuestionCount, ru.AnswerCount, ru.TotalScore, ru.TotalViews,
//        CASE WHEN ru.TotalBadges >= 10 THEN 'Expert' WHEN ru.TotalBadges >= 5 THEN 'Intermediate' ELSE 'Novice' END AS UserLevel
// FROM RankedUsers ru WHERE ru.UserRank <= 10 ORDER BY ru.UserRank;
//
// `Reputation > AVG(Reputation)` is compared exactly, as `Reputation * n > sum`.
fn q452(db: &'static So) -> String {
    let (s, n) = (&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |k, _| k + 1);
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(post_type_id.and(score).and(view_count.opt()))
        .fold([0i64; 4], |a, ((t, sc), w)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + sc, a[3] + w.unwrap_or(0)]);
    let v = drain(db.user.with((&db.user.reputation).filt(move |r| r * n > s)).select((&ps).filt(|a| a[0] + a[1] > 0).and((&bc).opt())));
    let v = top_n(v, |&(u, (a, _))| (Reverse(a[2]), Reverse(a[3]), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let b = b.unwrap_or(0);
        row(vec![user_col(db, u, "name"), V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::S(if b >= 10 { "Expert" } else if b >= 5 { "Intermediate" } else { "Novice" })])
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostMetrics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AvgScore, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount FROM Posts p GROUP BY p.OwnerUserId),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(pc.PostCount, 0) AS PostCount, COALESCE(pc.TotalViews, 0) AS TotalViews, COALESCE(pc.AvgScore, 0) AS AvgScore,
//        COALESCE(bc.BadgeCount, 0) AS BadgeCount, COALESCE(bc.GoldBadges, 0) AS GoldBadges, COALESCE(bc.SilverBadges, 0) AS SilverBadges, COALESCE(bc.BronzeBadges, 0) AS BronzeBadges
//     FROM Users u LEFT JOIN PostMetrics pc ON u.Id = pc.OwnerUserId LEFT JOIN UserBadgeCounts bc ON u.Id = bc.UserId),
// RankedUsers AS (SELECT UserId, DisplayName, PostCount, TotalViews, AvgScore, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY TotalViews DESC, AvgScore DESC) AS Rank
//     FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, TotalViews, AvgScore, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, Rank FROM RankedUsers WHERE Rank <= 10 ORDER BY Rank;
fn q29322(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let pm = db.post.group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let avgf = |a: [i64; 3]| if a[0] == 0 { 0.0 } else { a[2] as f64 / a[0] as f64 };
    let v = drain((&ub).and((&pm).opt()));
    let v = ranked(v, |&(_, (_, p))| {
        let p = p.unwrap_or([0; 3]);
        (Reverse(p[1]), Reverse(fkey(avgf(p))))
    }, false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (b, p)), r)| {
        let p = p.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p[0]), V::I(p[1]), V::F(avgf(p))]);
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        AVG(COALESCE(p.Score, 0)) AS AvgScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT ph.PostId, ph.UserId, ph.PostHistoryTypeId, COUNT(*) AS HistoryCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph
//     GROUP BY ph.PostId, ph.UserId, ph.PostHistoryTypeId),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, QuestionCount, AnswerCount, AcceptedAnswers, AvgScore, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats),
// PostEditFrequency AS (SELECT phs.PostId, SUM(CASE WHEN phs.PostHistoryTypeId IN (4, 5) THEN 1 ELSE 0 END) AS EditCount, SUM(CASE WHEN phs.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount
//     FROM PostHistoryStats phs GROUP BY phs.PostId)
// SELECT u.UserId, u.DisplayName, u.TotalPosts, u.QuestionCount, u.AnswerCount, u.AcceptedAnswers, u.AvgScore, COALESCE(pe.EditCount, 0) AS EditCount, COALESCE(pe.CloseCount, 0) AS CloseCount,
//        v.UserRank AS Rank
// FROM TopUsers u LEFT JOIN PostEditFrequency pe ON u.UserId = pe.PostId
// JOIN (SELECT UserId, COUNT(*) AS UserRank FROM Votes v WHERE v.VoteTypeId IN (2, 3) GROUP BY UserId) v ON u.UserId = v.UserId
// WHERE u.TotalPosts > 10 ORDER BY u.TotalPosts DESC, u.AvgScore DESC;
//
// `u.UserId = pe.PostId` joins a user id to a post id, so it goes through the raw ids. PostEditFrequency sums over (post, user, type) groups,
// so it counts groups, not history rows.
fn q284(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, score, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, x), s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + x.is_some() as i64, a[4] + s],
        None => a,
    });
    let PostHistory { post, user, post_history_type_id, .. } = &db.post_history;
    let groups: MatSet<(Id<Post>, Option<Id<User>>, i64)> = db.post_history.select(post.and(user.opt()).and(post_history_type_id)).map(|((p, u), t)| (p, u, t)).collect();
    type G = (Id<Post>, Option<Id<User>>, i64);
    let pe = (&groups).group_by(Same::<G>::new().map(|(p, _, _): G| p)).select(Same::<G>::new().map(|(_, _, t): G| t)).fold([0i64; 2], |a, t| [a[0] + matches!(t, 4 | 5) as i64, a[1] + (t == 10) as i64]);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let Vote { vote_type_id, user: vu, .. } = &db.vote;
    let vc = db.vote.with(vote_type_id.is_in([2, 3])).group_by(vu).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&us).filt(|a| a[0] > 10).and((&db.user.origid).select(&pidx).select(&pe).opt()).and(&vc));
    rows(v.into_iter().map(|(u, ((a, e), n))| {
        let e = e.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), V::I(e[0]), V::I(e[1]), V::I(n)]);
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(U.Reputation) AS AverageUserReputation
//     FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' LEFT JOIN Users U ON P.OwnerUserId = U.Id GROUP BY T.TagName),
// TopTags AS (SELECT TagName, PostCount, QuestionCount, AnswerCount, AverageUserReputation, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM TagStatistics),
// PostHistoryStats AS (SELECT PH.PostId, COUNT(PH.Id) AS EditCount, MAX(PH.CreationDate) AS LastEditDate FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (4, 5, 6, 24) GROUP BY PH.PostId),
// TopEditedPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, P.LastActivityDate, COALESCE(E.EditCount, 0) AS EditCount, E.LastEditDate
//     FROM Posts P LEFT JOIN PostHistoryStats E ON P.Id = E.PostId WHERE P.PostTypeId = 1 ORDER BY P.Score DESC, P.ViewCount DESC LIMIT 10)
// SELECT T.TagName, T.PostCount, T.QuestionCount, T.AnswerCount, T.AverageUserReputation, E.PostId, E.Title, E.ViewCount, E.Score, E.LastActivityDate, E.EditCount, E.LastEditDate
// FROM TopTags T JOIN TopEditedPosts E ON E.PostId IN (SELECT P.Id FROM Posts P WHERE P.Tags LIKE '%' || T.TagName || '%')
// WHERE T.Rank <= 5 ORDER BY T.PostCount DESC, E.Score DESC;
fn q26420(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let tsf = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select(post_type_id.and(owner_user.select(&db.user.reputation).opt())))
        .fold([0i64; 5], |a, (t, r)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + r.is_some() as i64, a[4] + r.unwrap_or(0)]);
    let tt = ranked(drain(&tsf), |&(_, a)| Reverse(a[0]), false);
    let tt: MatSet<Id<Tag>> = rel(tt.into_iter().take_while(|x| x.1 <= 5).map(|x| x.0 .0).collect()).map(|t| t).collect();
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let tep = top_n(drain(db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new())), |&(p, _)| (key(p), p), 10);
    let tep: MatSet<Id<Post>> = rel(tep.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let es = db.post_history.with(post_history_type_id.is_in([4, 5, 6, 24])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&tt).select(&tsf).and((&by_tag).map(|(p, _)| p).select(Ident::<Post>::new().with(&tep).and((&es).opt()))));
    rows(v.into_iter().map(|(t, (a, (p, e)))| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3])];
        f.extend(post_fields(db, p, &["id", "title", "views", "score", "activity"]));
        f.extend(match e {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u),
// TopTags AS (SELECT t.TagName, t.Count, RANK() OVER (ORDER BY t.Count DESC) AS TagRank FROM Tags t WHERE t.Count > 100),
// PostsWithVotes AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate),
// PostHistoryAggregates AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastClosedDate,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11, 12) THEN 1 END) AS ClosureEvents FROM PostHistory ph GROUP BY ph.PostId),
// FinalResults AS (SELECT u.UserId, u.DisplayName, tt.TagName, pv.PostId, pv.Title, pv.CreationDate, pv.Upvotes, pv.Downvotes, ph.LastClosedDate, ph.ClosureEvents,
//        CASE WHEN ph.LastClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
//     FROM RankedUsers u CROSS JOIN TopTags tt JOIN PostsWithVotes pv ON u.UserId = pv.PostId LEFT JOIN PostHistoryAggregates ph ON pv.PostId = ph.PostId
//     WHERE u.UserRank <= 10 AND tt.TagRank <= 5)
// SELECT fr.DisplayName AS UserDisplayName, fr.TagName, fr.Title, fr.CreationDate, fr.Upvotes, fr.Downvotes, fr.PostStatus, COALESCE(fr.ClosureEvents, 0) AS NumberOfClosures
// FROM FinalResults fr ORDER BY fr.DisplayName, fr.TagName;
//
// `u.UserId = pv.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q30588(db: &'static So) -> String {
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let tt = ranked(drain(db.tag.with((&db.tag.count).gt(100)).select(&db.tag.count)), |&(_, c)| Reverse(c), false);
    let tt = rel(tt.into_iter().take_while(|x| x.1 <= 5).map(|x| x.0 .0).collect());
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let posts: MatSet<Id<Post>> = (&tu).select((&db.user.origid).select(&pidx)).collect();
    let pv = (&posts).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = (&posts).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd))).fold((i64::MIN, 0i64), |(m, n), (t, d)| {
        (if t == 10 { m.max(d) } else { m }, n + matches!(t, 10 | 11 | 12) as i64)
    });
    let ur = rel(drain((&tu).select((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&pv).and((&pha).opt())))));
    let mut v = Vec::new();
    (&ur).cross(&tt).drive(|_, ((u, ((p, a), h)), t)| v.push((u, t, p, a, h)));
    rows(v.into_iter().map(|(u, t, p, a, h)| {
        let mut f = vec![user_col(db, u, "name"), V::S(db.tag.tag_name.get(t).unwrap())];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if h.map_or(false, |(m, _)| m != i64::MIN) { "Closed" } else { "Open" }));
        f.push(V::I(h.map_or(0, |(_, n)| n)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) as Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation < 100 THEN 'Newbie' WHEN u.Reputation BETWEEN 100 AND 999 THEN 'Intermediary' ELSE 'Expert' END AS ReputationLevel
//     FROM Users u),
// PostVoteDetails AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// FinalStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ur.ReputationLevel, pvd.UpVotes, pvd.DownVotes, (pvd.UpVotes - pvd.DownVotes) AS VoteNet,
//        CASE WHEN ur.ReputationLevel = 'Expert' THEN 'Top Author' ELSE NULL END AS AuthorStatus
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostVoteDetails pvd ON rp.PostId = pvd.PostId WHERE rp.Rank <= 5)
// SELECT fs.PostId, fs.Title, fs.CreationDate, fs.Score, fs.ViewCount, fs.ReputationLevel, fs.UpVotes, fs.DownVotes, fs.VoteNet, fs.AuthorStatus
// FROM FinalStats fs WHERE fs.VoteNet > 0 ORDER BY fs.Score DESC LIMIT 10 OFFSET 0;
fn q963(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain((&pv).filt(|a| a[0] - a[1] > 0).and(owner_user.select(&db.user.reputation))), |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, r))| {
        let lvl = if r < 100 { "Newbie" } else if r <= 999 { "Intermediary" } else { "Expert" };
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::S(lvl), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), if lvl == "Expert" { V::S("Top Author") } else { V::Null }]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '>'), 1) AS TagCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank, u.DisplayName AS AuthorDisplayName, u.Reputation
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.ViewCount, rp.Score, rp.TagCount, rp.AuthorDisplayName, rp.Reputation, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        COALESCE(SUM(v.vote_count), 0) AS TotalVotes
//     FROM RankedPosts rp LEFT JOIN Comments c ON c.PostId = rp.PostId LEFT JOIN (SELECT PostId, COUNT(*) AS vote_count FROM Votes GROUP BY PostId) v ON v.PostId = rp.PostId
//     GROUP BY rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.ViewCount, rp.Score, rp.TagCount, rp.AuthorDisplayName, rp.Reputation)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.Score, ps.TagCount, ps.AuthorDisplayName, ps.Reputation, ps.CommentCount, ps.TotalVotes,
//        CASE WHEN ps.Score >= 10 THEN 'High Score' WHEN ps.Score BETWEEN 5 AND 9 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory,
//        CASE WHEN ps.TagCount > 5 THEN 'Diverse Tags' ELSE 'Limited Tags' END AS TagDiversity
// FROM PostStats ps ORDER BY ps.Score DESC, ps.ViewCount DESC LIMIT 50;
fn q25012(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, tags_str, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).with(owner_user);
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let ps = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and((&vc).opt())).fold([0i64; 2], |a, (c, n)| [a[0] + c.is_some() as i64, a[1] + n.unwrap_or(0)]);
    let v = top_n(drain(&ps), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let tc = tags_str.get(p).map(|t| t[1..t.len() - 1].split('>').count() as i64);
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(oint(tc));
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if s >= 10 { "High Score" } else if (5..=9).contains(&s) { "Medium Score" } else { "Low Score" }));
        f.push(V::S(if tc.map_or(false, |c| c > 5) { "Diverse Tags" } else { "Limited Tags" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.Body, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RowNum FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount FROM Votes v WHERE v.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY v.PostId),
// CommentStatistics AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(b.Class) AS TotalBadges, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName ORDER BY TotalBadges DESC LIMIT 5)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, COALESCE(rv.VoteCount, 0) AS RecentVoteCount, cs.CommentCount, cs.LastCommentDate, tu.UserId, tu.DisplayName, tu.TotalBadges, tu.TotalViews
// FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN CommentStatistics cs ON rp.PostId = cs.PostId LEFT JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId
// WHERE rp.RankScore <= 3 OR (rp.RowNum = 1 AND rp.OwnerUserId IS NOT NULL) ORDER BY rp.CreationDate DESC, rp.Score DESC LIMIT 50;
fn q34867(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let t0 = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let recent = || db.post.with(creation_date.ge(t0));
    let rs = per_group(ranked(drain(recent().select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |&(_, t)| t);
    let top = top_per(drain(recent().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let mut keep: Vec<Id<Post>> = drain(rel(rs).filt(|(_, r)| r <= 3)).into_iter().map(|x| x.1 .0 .0).collect();
    keep.extend(top.into_iter().map(|x| x.0));
    let keep: MatSet<Id<Post>> = rel(keep).map(|p| p).collect();
    let rv = db.vote.with((&db.vote.creation_date).ge(t0)).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let cs = (&keep).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(view_count.opt()).opt()))
        .fold([0i64; 3], |a, (c, w)| [a[0] + c.is_some() as i64, a[1] + c.unwrap_or(0), a[2] + w.flatten().unwrap_or(0)]);
    let tu = top_n(drain(&tu), |&(u, a)| (a[0] == 0, Reverse(a[1]), u), 5);
    let tu = rel(tu);
    let top_users: HashIdx<Id<User>, (Id<User>, [i64; 3])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let v = drain((&keep).select(Ident::<Post>::new().and((&rv).opt()).and((&cs).opt()).and(owner_user.select(&top_users).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(_, (((p, n), c), u))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.push(V::I(n.unwrap_or(0)));
        f.extend(match c {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        f.extend(match u {
            Some((u, a)) => vec![user_col(db, u, "uid"), user_col(db, u, "name"), nullable(a[1], a[0]), V::I(a[2])],
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecursiveCTE AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(An.AnswerCount, 0) AS AnswerCount, COALESCE(Com.CommentCount, 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) An ON p.Id = An.ParentId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) Com ON p.Id = Com.PostId WHERE p.PostTypeId = 1),
// UserScoreSummary AS (SELECT u.Id AS UserId, u.DisplayName, SUM(v.BountyAmount) AS TotalBounty, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.PostId END) AS UniqueUpVotedPosts FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT r.PostId, r.Title, r.CreationDate, r.ViewCount, r.Score, r.AnswerCount, r.CommentCount, u.DisplayName AS UserCreator, u.TotalBounty AS CreatorTotalBounty,
//        u.UpVotes AS CreatorUpVotes, u.UniqueUpVotedPosts AS CreatorUniqueUpVotedPosts, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = r.PostId AND v.VoteTypeId = 3) AS DownVotes
// FROM RecursiveCTE r LEFT JOIN UserScoreSummary u ON r.PostId = u.UserId WHERE r.ViewCount > 1000 ORDER BY r.Score DESC, r.ViewCount DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// The order reads only base columns, so the ten questions are picked first. `r.PostId = u.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q34499(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let tp = top_n(drain(db.post.with(post_type_id.eq(1).and(view_count.gt(1000))).select(view_count)), |&(p, w)| (Reverse(score.get(p).unwrap()), Reverse(w), p), 10);
    let tp: MatSet<Id<Post>> = rel(tp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let answers = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let comments = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let down = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(3))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let users: MatSet<Id<User>> = (&tp).select((&db.post.origid).select(&uidx)).collect();
    let Vote { vote_type_id, bounty_amount, post_id, .. } = &db.vote;
    let uss = (&users).group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id.and(bounty_amount.opt())).opt()).fold([0i64; 3], |a, v| match v {
        Some((t, b)) => [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + (t == 2) as i64],
        None => a,
    });
    let uniq = (&users).group_by(Ident::<User>::new()).select(votes_by(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2))).select(post_id).opt()).buf_fold(distinct_some);
    let v = drain((&answers).and(&comments).and(&down).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&uss).and(&uniq)).opt()));
    rows(v.into_iter().map(|(p, (((a, c), d), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a), V::I(c)]);
        f.extend(match u {
            Some(((u, s), n)) => vec![user_col(db, u, "name"), nullable(s[1], s[0]), V::I(s[2]), V::I(n)],
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::I(d));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS Rank,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY P.Id) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY P.Id) AS DownVotes, P.OwnerUserId
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.PostTypeId = 1 AND P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT R.*, U.DisplayName AS OwnerDisplayName, U.Reputation FROM RankedPosts R JOIN Users U ON R.OwnerUserId = U.Id WHERE R.Rank <= 5),
// PostDetails AS (SELECT TP.PostId, TP.Title, TP.OwnerDisplayName, TP.Reputation, TP.Score, TP.ViewCount, COALESCE(TC.CommentCount, 0) AS TotalComments,
//        COALESCE(TH.Upvotes, 0) - COALESCE(TH.Downvotes, 0) AS NetVotes
//     FROM TopPosts TP LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) TC ON TP.PostId = TC.PostId
//     LEFT JOIN (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS Downvotes FROM Votes GROUP BY PostId) TH ON TP.PostId = TH.PostId)
// SELECT PD.Title, PD.OwnerDisplayName, PD.Reputation, PD.Score, PD.ViewCount, PD.TotalComments, PD.NetVotes FROM PostDetails PD ORDER BY PD.Reputation DESC, PD.Score DESC LIMIT 10;
//
// RankedPosts has one row per post x vote, and Rank numbers those rows, so an owner's five rows can be several votes of one post.
fn q2364(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let jr = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.and(votes_of(db).opt())));
    let top = top_per(jr, |&(_, (u, _))| u, |&(p, (_, v))| (Reverse(score.get(p).unwrap()), p, v), 5, false);
    let posts: MatSet<Id<Post>> = rel(top.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&posts).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let th = (&posts).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    type R = (Id<Post>, (Id<User>, Option<Id<Vote>>));
    let tr = rel(top);
    let v = drain((&tr).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&cc).and(&th)))));
    let v = top_n(v, |&(_, ((p, (u, x)), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p, x), 10);
    rows(v.into_iter().map(|(_, ((p, (u, _)), (c, n)))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(post_fields(db, p, &["score", "views"]));
        f.extend([V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(rp.Score), 0) AS TotalScore, COUNT(rp.Id) AS TotalPosts, SUM(rp.UpVotes) AS TotalUpVotes,
//        SUM(rp.DownVotes) AS TotalDownVotes FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.TotalScore, DENSE_RANK() OVER (ORDER BY ua.TotalScore DESC) AS ScoreRank FROM UserActivity ua WHERE ua.TotalPosts > 0)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalScore, tu.ScoreRank, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId GROUP BY tu.UserId, tu.DisplayName, tu.Reputation, tu.TotalScore, tu.ScoreRank ORDER BY tu.ScoreRank LIMIT 10;
//
// RankedPosts is one row per post whatever its comments and votes, and its vote sums are never read, so the comment x vote product is not driven.
fn q2731(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let ua = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let v = ranked(drain(&ua), |&(_, a)| Reverse(a[1]), true);
    let v = top_n(v, |&((u, _), r)| (r, u), 10);
    let tu: MatSet<Id<User>> = rel(v.iter().map(|x| x.0 .0).collect()).map(|u| u).collect();
    let b = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    type T = ((Id<User>, [i64; 2]), i64);
    let v = drain(rel(v).select(Same::<T>::new().and(Same::<T>::new().map(|((u, _), _): T| u).select(&b))));
    rows(v.into_iter().map(|(_, (((u, a), r), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(r)]);
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, p.AnswerCount, p.CommentCount, p.FavoriteCount, p.Tags,
//        RANK() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.AnswerCount, rp.CommentCount, rp.FavoriteCount, rp.Tags FROM RankedPosts rp WHERE rp.ScoreRank <= 5),
// PostMetrics AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.AnswerCount, tp.CommentCount, tp.FavoriteCount, tp.Tags,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM TopPosts tp LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.AnswerCount, tp.CommentCount, tp.FavoriteCount, tp.Tags)
// SELECT pm.Title, pm.OwnerDisplayName, pm.Score, pm.ViewCount, pm.AnswerCount, pm.CommentCount, pm.FavoriteCount, pm.UpVotes, pm.DownVotes, pm.Tags, CAST(pm.CreationDate AS DATE) AS CreatedOn
// FROM PostMetrics pm ORDER BY pm.Score DESC, pm.ViewCount DESC;
fn q7239(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, tags_str, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pm = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain(&pm).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views", "answers", "comments", "favorites"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["tags"]));
        f.push(V::D(creation_date.get(p).unwrap()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVoteCounts AS (SELECT PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVoteCount
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY PostId),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId = 10) AS CloseCount, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId = 11) AS ReopenCount,
//        COUNT(*) FILTER (WHERE ph.PostHistoryTypeId IN (12, 13)) AS DeleteCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.Id, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerDisplayName, COALESCE(pvc.UpVoteCount, 0) AS UpVoteCount, COALESCE(pvc.DownVoteCount, 0) AS DownVoteCount,
//        COALESCE(phs.CloseCount, 0) AS CloseCount, COALESCE(phs.ReopenCount, 0) AS ReopenCount, COALESCE(phs.DeleteCount, 0) AS DeleteCount,
//        CASE WHEN rp.Rank <= 10 THEN 'Top 10 Post' ELSE 'Other Post' END AS PostCategory, CASE WHEN rp.Score >= 0 THEN 'Positive' ELSE 'Negative' END AS ScoreCategory
// FROM RankedPosts rp LEFT JOIN PostVoteCounts pvc ON rp.Id = pvc.PostId LEFT JOIN PostHistorySummary phs ON rp.Id = phs.PostId ORDER BY rp.Score DESC, rp.ViewCount DESC;
fn q33996(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user);
    let top = top_per(drain(rp().select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let top: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvc = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db)).opt()).fold([0i64; 2], |a, n| [a[0] + (n == Some("UpMod")) as i64, a[1] + (n == Some("DownMod")) as i64]);
    let phs = rp().group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(11)) as i64, a[2] + matches!(t, Some(12 | 13)) as i64]
    });
    let v = drain((&pvc).and(&phs).and(Ident::<Post>::new().with(&top).opt()));
    rows(v.into_iter().map(|(p, ((a, h), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(h[0]), V::I(h[1]), V::I(h[2])]);
        f.push(V::S(if t.is_some() { "Top 10 Post" } else { "Other Post" }));
        f.push(V::S(if score.get(p).unwrap() >= 0 { "Positive" } else { "Negative" }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(*) FILTER (WHERE Class = 1) AS GoldCount, COUNT(*) FILTER (WHERE Class = 2) AS SilverCount, COUNT(*) FILTER (WHERE Class = 3) AS BronzeCount
//     FROM Badges GROUP BY UserId),
// PostStats AS (SELECT OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(ViewCount) AS TotalViews FROM Posts GROUP BY OwnerUserId),
// UserAggregate AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.GoldCount, 0) AS GoldBadges, COALESCE(ub.SilverCount, 0) AS SilverBadges, COALESCE(ub.BronzeCount, 0) AS BronzeBadges,
//        COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.QuestionCount, 0) AS QuestionCount, COALESCE(ps.AnswerCount, 0) AS AnswerCount, COALESCE(ps.TotalViews, 0) AS TotalViews,
//        RANK() OVER (ORDER BY COALESCE(ps.TotalPosts, 0) DESC, u.Reputation DESC) AS PostRank, u.Reputation
//     FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT UserId, DisplayName, GoldBadges, SilverBadges, BronzeBadges, TotalPosts, QuestionCount, AnswerCount, TotalViews, PostRank,
//        CASE WHEN TotalPosts = 0 THEN 'No Posts' WHEN QuestionCount > AnswerCount THEN 'More Questions than Answers' ELSE 'More Answers than Questions' END AS PostBalance,
//        (SELECT COUNT(DISTINCT c.PostId) FROM Comments c WHERE c.UserId = UserId) AS TotalComments
// FROM UserAggregate WHERE Reputation > 1000 ORDER BY TotalViews DESC, PostRank LIMIT 50;
//
// Inside the subquery `UserId` binds to `c.UserId`, so it is `c.UserId = c.UserId`: one uncorrelated count of the posts commented on by any known user.
fn q3975(db: &'static So) -> String {
    let ub = badge_classes(db);
    let Post { owner_user, post_type_id, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 4], |a, (t, w)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0)]);
    let commented: MatSet<i64> = db.comment.with(&db.comment.user).select(&db.comment.post_id).collect();
    let tc = count(&commented);
    let v = drain(db.user.select((&ub).opt().and((&ps).opt())));
    let v = ranked(v, |&(u, (_, p))| (Reverse(p.map_or(0, |a| a[0])), Reverse(db.user.reputation.get(u).unwrap())), false);
    let v = drain(rel(v).filt(|((u, _), _)| db.user.reputation.get(u).unwrap() > 1000));
    let v = top_n(v.into_iter().map(|x| x.1).collect(), |&((u, (_, p)), r)| (Reverse(p.map_or(0, |a| a[3])), r, u), 50);
    rows(v.into_iter().map(|((u, (b, p)), r)| {
        let b = b.unwrap_or([0; 3]);
        let p = p.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[3]), V::I(r)]);
        f.push(V::S(if p[0] == 0 { "No Posts" } else if p[1] > p[2] { "More Questions than Answers" } else { "More Answers than Questions" }));
        f.push(V::I(tc));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, u.DisplayName AS OwnerDisplayName, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// PostWithComments AS (SELECT rp.PostId, rp.Title, rp.Score, rp.OwnerDisplayName, rp.CreationDate, COUNT(c.Id) AS CommentCount FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId
//     GROUP BY rp.PostId, rp.Title, rp.Score, rp.OwnerDisplayName, rp.CreationDate),
// PostsWithScores AS (SELECT pwc.PostId, pwc.Title, pwc.Score, pwc.OwnerDisplayName, pwc.CreationDate, pwc.CommentCount,
//        CASE WHEN pwc.CommentCount > 10 THEN 'High Engagement' WHEN pwc.CommentCount BETWEEN 1 AND 10 THEN 'Moderate Engagement' ELSE 'No Engagement' END AS EngagementLevel FROM PostWithComments pwc)
// SELECT ps.PostId, ps.Title, ps.Score, ps.OwnerDisplayName, ps.CreationDate, ps.CommentCount, ps.EngagementLevel, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes
// FROM PostsWithScores ps LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v
//     ON ps.PostId = v.PostId
// WHERE ps.EngagementLevel <> 'No Engagement' ORDER BY ps.Score DESC, ps.CommentCount DESC LIMIT 50;
fn q84(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)));
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vs = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain((&cc).filt(|n| n >= 1).and(&vs)), |&(p, (n, _))| (Reverse(score.get(p).unwrap()), Reverse(n), p), 50);
    rows(v.into_iter().map(|(p, (n, a))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "owner", "created"]);
        f.extend([V::I(n), V::S(if n > 10 { "High Engagement" } else { "Moderate Engagement" }), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON V.UserId = U.Id GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalBounty, RANK() OVER (ORDER BY TotalPosts DESC, Reputation DESC) AS UserRank FROM UserStats),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalBounty FROM RankedUsers WHERE UserRank <= 10)
// SELECT TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.TotalBounty, (SELECT COUNT(*) FROM Comments C WHERE C.UserId = TU.UserId) AS TotalComments,
//        (SELECT COUNT(*) FROM Badges B WHERE B.UserId = TU.UserId AND B.Class = 1) AS GoldBadges, (SELECT COUNT(*) FROM Badges B WHERE B.UserId = TU.UserId AND B.Class = 2) AS SilverBadges,
//        (SELECT COUNT(*) FROM Badges B WHERE B.UserId = TU.UserId AND B.Class = 3) AS BronzeBadges
// FROM TopUsers TU ORDER BY TU.TotalPosts DESC, TU.Reputation DESC;
//
// UserRank reads only the distinct post count and the reputation, so the top users are picked first and the posts x votes product is driven for those alone.
fn q1886(db: &'static So) -> String {
    let dp = user_distinct_posts(db);
    let v = ranked(drain(&dp), |&(u, n)| (Reverse(n), Reverse(db.user.reputation.get(u).unwrap())), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.flatten().unwrap_or(0)]);
    let cc = (&tu).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = drain((&us).and(&dp).and(&cc).and(&bc));
    rows(v.into_iter().map(|(u, (((a, n), c), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c)]);
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RankByDate,
//        SUM(COALESCE(v.VoteTypeId, 0)) OVER (PARTITION BY p.Id) AS TotalVotes, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.TotalVotes, rp.CommentCount FROM RankedPosts rp WHERE rp.RankByDate <= 10),
// PostRanking AS (SELECT fp.*, CASE WHEN TotalVotes > 50 THEN 'Highly Engaged' WHEN TotalVotes BETWEEN 21 AND 50 THEN 'Moderately Engaged' ELSE 'Low Engagement' END AS EngagementCategory
//     FROM FilteredPosts fp),
// PostHistorySummary AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastEdited, COUNT(*) AS EditCount, SUM(CASE WHEN ph.PostHistoryTypeId IN (1, 4, 5) THEN 1 ELSE 0 END) AS TitleEdits,
//        SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11, 12) THEN 1 ELSE 0 END) AS ClosureStatus FROM PostHistory ph GROUP BY ph.PostId)
// SELECT pr.PostId, pr.Title, pr.CreationDate, pr.ViewCount, pr.CommentCount, pr.TotalVotes, phs.LastEdited, phs.EditCount, phs.TitleEdits, phs.ClosureStatus, pr.EngagementCategory
// FROM PostRanking pr LEFT JOIN PostHistorySummary phs ON pr.PostId = phs.PostId WHERE phs.EditCount > 5 OR phs.ClosureStatus > 0 ORDER BY pr.ViewCount DESC, pr.TotalVotes DESC LIMIT 100;
//
// RankedPosts has one row per post x vote x comment and RankByDate numbers those rows, so the ten rows per post type can be several rows of one post.
fn q21724(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, .. } = &db.post;
    let updown = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let jr = drain(recent().select(post_type_id.and(updown().opt()).and(comments_of(db).opt())));
    let top = top_per(jr, |&(_, ((t, _), _))| t, |&(p, ((_, v), c))| (Reverse(creation_date.get(p).unwrap()), p, v, c), 10, false);
    let posts: MatSet<Id<Post>> = rel(top.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&posts).group_by(Ident::<Post>::new()).select(updown().select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 2], |a, (t, c)| [a[0] + t.unwrap_or(0), a[1] + c.is_some() as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = (&posts).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd))).fold([i64::MIN, 0, 0, 0], |a, (t, d)| {
        [a[0].max(d), a[1] + 1, a[2] + matches!(t, 1 | 4 | 5) as i64, a[3] + matches!(t, 10 | 11 | 12) as i64]
    });
    type R = (Id<Post>, ((i64, Option<Id<Vote>>), Option<Id<Comment>>));
    let tr = rel(top);
    let v = drain((&tr).select(Same::<R>::new().map(|(p, _): R| p).select(Ident::<Post>::new().and(&pv).and((&phs).filt(|h| h[1] > 5 || h[3] > 0)))));
    let v = top_n(v, |&(i, ((p, a), _))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(a[0]), i)
    }, 100);
    rows(v.into_iter().map(|(_, ((p, a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(a[1]), V::I(a[0]), V::T(h[0]), V::I(h[1]), V::I(h[2]), V::I(h[3])]);
        f.push(V::S(if a[0] > 50 { "Highly Engaged" } else if a[0] >= 21 { "Moderately Engaged" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.Score, p.CreationDate, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')),
// RecentBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Date >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months') GROUP BY b.UserId),
// AggregatedUserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(rb.BadgeCount, 0) AS RecentBadges, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        AVG(COALESCE(v.VoteAmount, 0)) AS AverageVotes
//     FROM Users u LEFT JOIN RankedPosts r ON u.Id = r.OwnerUserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteAmount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN RecentBadges rb ON u.Id = rb.UserId GROUP BY u.Id, u.DisplayName, rb.BadgeCount)
// SELECT us.UserId, us.DisplayName, us.RecentBadges, us.TotalViews, us.TotalScore, us.AverageVotes, COUNT(DISTINCT r.PostId) AS NumberOfQuestions, MAX(r.CreationDate) AS LastPostDate
// FROM AggregatedUserStats us LEFT JOIN RankedPosts r ON us.UserId = r.OwnerUserId GROUP BY us.UserId, us.DisplayName, us.RecentBadges, us.TotalViews, us.TotalScore, us.AverageVotes
// HAVING COUNT(DISTINCT r.PostId) > 5 ORDER BY us.TotalViews DESC, us.TotalScore DESC;
//
// The HAVING reads only each user's recent questions, so those users are picked first and the recent-questions x posts x votes product is driven for them alone.
fn q30252(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, view_count, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let recent = Ident::<Post>::new().with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1))));
    let rq = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).group_by(owner_user).select(creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let us: MatSet<Id<User>> = db.user.with((&rq).filt(|(n, _)| n > 5)).collect();
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let agg = (&us)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(recent).opt().and(posts_of(db).select(view_count.opt().and(score).and((&vc).opt())).opt()))
        .fold([0i64; 4], |a, (_, p)| match p {
            Some(((w, s), n)) => [a[0] + w.unwrap_or(0), a[1] + s, a[2] + n.unwrap_or(0), a[3] + 1],
            None => [a[0], a[1], a[2], a[3] + 1],
        });
    let rb = db.badge.with((&db.badge.date).ge(add_months(t0, -6))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&agg).and(&rq).and((&rb).opt()));
    rows(v.into_iter().map(|(u, ((a, (n, d)), b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(a[0]), V::I(a[1]), avg(a[2], a[3]), V::I(n), V::T(d)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        u.DisplayName AS OwnerName, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, u.DisplayName, p.OwnerUserId),
// TopRankedPosts AS (SELECT PostId, Title, ViewCount, Score, CreationDate, OwnerName, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE Rank <= 10),
// CtePostHistory AS (SELECT ph.PostId, ph.UserDisplayName, ph.CreationDate, ph.Comment, ph.PostHistoryTypeId, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12))
// SELECT trp.PostId, trp.Title, trp.ViewCount, trp.Score, trp.CreationDate, trp.OwnerName, trp.CommentCount, trp.UpVotes, trp.DownVotes,
//        COALESCE(cph.Comment, 'No Recent Activity') AS LastActivityComment, cph.CreationDate AS LastActivityDate
// FROM TopRankedPosts trp LEFT JOIN CtePostHistory cph ON trp.PostId = cph.PostId AND cph.HistoryRank = 1 ORDER BY trp.ViewCount DESC, trp.Score DESC;
//
// Rank reads only base columns, so each owner's ten questions are picked first. A tie on the latest history date goes to the larger history id.
fn q30729(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cl = drain(db.post_history.with(post_history_type_id.is_in([10, 11, 12])).select(post));
    let cl = top_per(cl, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let cl = rel(cl.into_iter().map(|(h, p)| (p, h)).collect());
    let last: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&cl).map(|(p, _)| p).inv().select(&cl).collect();
    let v = drain((&cc).and(&vs).and((&last).map(|(_, h)| h).opt()));
    rows(v.into_iter().map(|(p, ((c, a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "created", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [V::S(db.post_history.comment.get(h).unwrap_or("No Recent Activity")), V::T(hd.get(h).unwrap())],
            None => [V::S("No Recent Activity"), V::Null],
        });
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, DENSE_RANK() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges B GROUP BY B.UserId),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.OwnerUserId),
// UserPerformance AS (SELECT U.Id AS UserId, U.DisplayName, PS.TotalPosts, PS.TotalUpvotes, PS.TotalDownvotes, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges,
//        CASE WHEN U.Reputation > 5000 THEN 'Elite' ELSE 'Novice' END AS UserTier FROM Users U LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId LEFT JOIN UserBadges UB ON U.Id = UB.UserId)
// SELECT U.DisplayName, U.TotalPosts, U.TotalUpvotes, U.TotalDownvotes, U.GoldBadges, U.SilverBadges, U.BronzeBadges, U.UserTier, R.UserRank
// FROM UserPerformance U JOIN RankedUsers R ON U.UserId = R.UserId
// WHERE U.TotalPosts IS NOT NULL AND U.TotalUpvotes >= 10 AND NOT EXISTS (SELECT 1 FROM Votes V WHERE V.UserId = U.UserId AND V.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')
// ORDER BY R.UserRank OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
fn q22144(db: &'static So) -> String {
    let ur = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let ur = rel(ur.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    let ps = db.post.group_by(&db.post.owner_user).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ub = badge_classes(db);
    let Vote { user, creation_date, .. } = &db.vote;
    let voters: MatSet<Id<User>> = db.vote.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(user).collect();
    let v = drain(db.user.minus(&voters).select((&ps).filt(|a| a[1] >= 10).and((&ub).opt()).and((&rank).map(|(_, r)| r))));
    let v = top_n(v, |&(u, (_, r))| (r, u), 10);
    rows(v.into_iter().map(|(u, ((a, b), r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::S(if db.user.reputation.get(u).unwrap() > 5000 { "Elite" } else { "Novice" }), V::I(r)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, CTE.ReputationRank, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId JOIN UserReputation CTE ON P.OwnerUserId = CTE.UserId
//     WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId, CTE.ReputationRank),
// PostHistoryDetails AS (SELECT PH.PostId, PH.PostHistoryTypeId, PH.CreationDate, PH.UserId, RANK() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS HistoryRank, PH.Comment
//     FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11, 12, 13, 14, 15, 20))
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.CommentCount, RP.UpVotes, RP.DownVotes, U.DisplayName AS OwnerDisplayName, U.Reputation,
//        CASE WHEN RP.ReputationRank <= 10 THEN 'Top Reputation User' ELSE 'Regular User' END AS UserCategory, COALESCE(PHD.Comment, 'No history') AS RecentActivity,
//        CASE WHEN PHD.HistoryRank = 1 THEN 'Most Recent Change' ELSE 'Earlier Change' END AS ChangeCategory
// FROM RecentPosts RP JOIN Users U ON RP.OwnerUserId = U.Id LEFT JOIN PostHistoryDetails PHD ON RP.PostId = PHD.PostId AND PHD.HistoryRank = 1
// WHERE RP.CommentCount > 0 ORDER BY RP.CreationDate DESC, RP.UpVotes DESC LIMIT 50;
fn q21702(db: &'static So) -> String {
    let ur = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let ur = rel(ur.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let types = || db.post_history.with(post_history_type_id.is_in([10, 11, 12, 13, 14, 15, 20]));
    let md = types().group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = types().select(post.and(hd)).inv().collect();
    let v = drain((&rp).filt(|a| a[0] > 0).and(owner_user.select(Ident::<User>::new().and((&rank).map(|(_, r)| r)))).and(Ident::<Post>::new().and(&md).select(&at).opt()));
    let v = top_n(v, |&(p, ((a, _), h))| (Reverse(creation_date.get(p).unwrap()), Reverse(a[1]), p, h), 50);
    rows(v.into_iter().map(|(p, ((a, (u, r)), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::S(if r <= 10 { "Top Reputation User" } else { "Regular User" }));
        f.push(match h {
            Some(h) => V::S(db.post_history.comment.get(h).unwrap_or("No history")),
            None => V::S("No history"),
        });
        f.push(V::S(if h.is_some() { "Most Recent Change" } else { "Earlier Change" }));
        row(f)
    }))
}

// WITH RecursiveBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount, ROW_NUMBER() OVER (ORDER BY COUNT(*) DESC) AS BadgeRank FROM Badges GROUP BY UserId),
// QuestionPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.ViewCount, COALESCE(v.UpVoteCount, 0) AS UpVoteCount, COALESCE(c.CommentCount, 0) AS CommentCount
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS UpVoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId WHERE p.PostTypeId = 1),
// ClosedQuestions AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate FROM PostHistory ph GROUP BY ph.PostId),
// RankedQuestions AS (SELECT qp.PostId, qp.Title, qp.OwnerUserId, qp.ViewCount, qp.UpVoteCount, qp.CommentCount, ROW_NUMBER() OVER (ORDER BY qp.ViewCount DESC, qp.UpVoteCount DESC) AS Rank
//     FROM QuestionPosts qp LEFT JOIN ClosedQuestions cq ON qp.PostId = cq.PostId WHERE cq.ClosedDate IS NULL),
// UsersWithBadges AS (SELECT u.Id AS UserId, CONCAT(u.DisplayName, ' - Reputation: ', CAST(u.Reputation AS VARCHAR)) AS UserInfo, rb.BadgeCount AS BadgeCount
//     FROM Users u JOIN RecursiveBadgeCounts rb ON u.Id = rb.UserId WHERE rb.BadgeCount > 0)
// SELECT rq.Rank, rq.Title, rq.ViewCount, rq.UpVoteCount, rq.CommentCount, u.UserInfo FROM RankedQuestions rq JOIN UsersWithBadges u ON rq.OwnerUserId = u.UserId WHERE rq.Rank <= 10 ORDER BY rq.Rank;
fn q30712(db: &'static So) -> String {
    let Post { post_type_id, view_count, owner_user, .. } = &db.post;
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).collect();
    let up = votes_of_type(db, 2);
    let v = drain(db.post.with(post_type_id.eq(1)).minus(&closed).select(Ident::<Post>::new().and(&up)));
    let v = top_n(v, |&(p, (_, n))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(n), p)
    }, 10);
    let rq = rel(v.into_iter().enumerate().map(|(i, (p, (_, n)))| (p, n, i as i64 + 1)).collect());
    let tp: MatSet<Id<Post>> = (&rq).map(|(p, _, _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let badged: MatSet<Id<User>> = db.badge.select(&db.badge.user).collect();
    type R = (Id<Post>, i64, i64);
    let v = drain((&rq).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _, _): R| p).select((&cc).and(owner_user.select(Ident::<User>::new().with(&badged)))))));
    rows(v.into_iter().map(|(_, ((p, n, r), (c, u)))| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(n), V::I(c)]);
        f.push(V::Owned(format!("{} - Reputation: {}", db.user.display_name.get(u).unwrap(), db.user.reputation.get(u).unwrap())));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - interval '30 days'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, COUNT(DISTINCT p.Id) AS TotalPosts, RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(*) AS HistoryCount, MAX(ph.CreationDate) AS LastUpdated FROM PostHistory ph GROUP BY ph.PostId),
// ClosedPosts AS (SELECT p.Id AS PostId, p.Title, ph.Comment AS CloseReason, ph.CreationDate AS CloseDate FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10)
// SELECT rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate, rp.Score AS RecentPostScore, rp.ViewCount AS RecentPostViews, tu.DisplayName AS TopUserDisplayName,
//        tu.TotalBounty AS TopUserTotalBounty, phc.HistoryCount AS PostHistoryCount, cp.CloseReason AS ClosedPostReason, cp.CloseDate AS ClosedPostDate
// FROM RecentPosts rp LEFT JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId AND tu.UserRank <= 10 LEFT JOIN PostHistoryCounts phc ON rp.Id = phc.PostId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId
// WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC;
//
// UserRank reads only the distinct post count, so the top users are picked first and the posts x bounty-votes product is driven for those alone.
fn q31748(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let rs = per_group(ranked(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id)), |&(p, t)| (t, Reverse(creation_date.get(p).unwrap())), false), |&(_, t)| t);
    let rp: MatSet<Id<Post>> = rel(drain(rel(rs).filt(|(_, r)| r == 1)).into_iter().map(|x| x.1 .0 .0).collect()).map(|p| p).collect();
    let dp = user_distinct_posts(db);
    let tu = ranked(drain(&dp), |&(_, n)| Reverse(n), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let tb = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let hc = (&rp).group_by(Ident::<Post>::new()).select(history_of(db)).fold(0i64, |n, _| n + 1);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&rp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&tb)).opt()).and((&hc).opt()).and(closes.opt())));
    rows(v.into_iter().map(|(_, (((p, u), n), h))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(match u {
            Some((u, b)) => [user_col(db, u, "name"), V::I(b)],
            None => [V::Null, V::Null],
        });
        f.push(oint(n));
        f.extend(match h {
            Some(h) => [ostr(db.post_history.comment.get(h)), V::T(db.post_history.creation_date.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostAndBadgeSummary AS (SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
//     FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId)
// SELECT pas.PostId, pas.Title, pas.CreationDate, COALESCE(pas.Score, 0) AS Score, COALESCE(pas.ViewCount, 0) AS ViewCount, pas.OwnerDisplayName, COALESCE(pas.BadgeCount, 0) AS BadgeCount,
//        COALESCE(pas.GoldBadges, 0) AS GoldBadges, COALESCE(pas.SilverBadges, 0) AS SilverBadges, COALESCE(pas.BronzeBadges, 0) AS BronzeBadges,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = pas.PostId) AS CommentCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = pas.PostId AND v.VoteTypeId = 2) AS UpvoteCount
// FROM PostAndBadgeSummary pas WHERE (pas.BadgeCount > 0 OR pas.Score > 10) ORDER BY pas.CreationDate DESC LIMIT 50;
fn q1506(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain(
        db.post
            .with(post_type_id.eq(1).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
            .select(score.and(owner_user.select(&ub).opt()))
            .filt(|(s, b): (i64, Option<[i64; 4]>)| b.map_or(false, |b| b[0] > 0) || s > 10),
    );
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    let tp: MatSet<Id<Post>> = rel(v.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64);
    type P = (Id<Post>, (i64, Option<[i64; 4]>));
    let v = drain(rel(v).select(Same::<P>::new().and(Same::<P>::new().map(|(p, _): P| p).select((&cc).and(&uc)))));
    rows(v.into_iter().map(|(_, ((p, (s, b)), (c, n)))| {
        let b = b.unwrap_or([0; 4]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s), V::I(view_count.get(p).unwrap_or(0))]);
        f.extend(post_fields(db, p, &["owner"]));
        f.extend(b.map(V::I));
        f.extend([V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(CASE WHEN p.Score > 0 THEN p.Score ELSE 0 END) AS TotalScore,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY TotalScore DESC, TotalPosts DESC) AS ScoreRank
//     FROM UserStatistics)
// SELECT t.UserId, t.DisplayName, t.TotalPosts, t.TotalQuestions, t.TotalAnswers, t.TotalScore, t.GoldBadges, t.SilverBadges, t.BronzeBadges,
//        (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = t.UserId AND p.CreationDate BETWEEN cast('2024-10-01' as date) - INTERVAL '30 days' AND cast('2024-10-01' as date)) AS RecentPosts,
//        (SELECT COUNT(*) FROM Comments c WHERE c.UserId = t.UserId AND c.CreationDate BETWEEN cast('2024-10-01' as date) - INTERVAL '30 days' AND cast('2024-10-01' as date)) AS RecentComments,
//        (SELECT COUNT(*) FROM Votes v WHERE v.UserId = t.UserId AND v.CreationDate BETWEEN cast('2024-10-01' as date) - INTERVAL '30 days' AND cast('2024-10-01' as date)) AS RecentVotes
// FROM TopUsers t WHERE t.ScoreRank <= 10 ORDER BY t.TotalScore DESC, t.TotalPosts DESC;
fn q25788(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(0));
    let us = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (s, c)| [a[0] + s.map_or(0, |s| s.max(0)), a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]);
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let v = ranked(drain((&us).and(&pc)), |&(_, (a, p))| (Reverse(a[0]), Reverse(p[0])), false);
    let tu: MatSet<Id<User>> = rel(v.iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let (lo, hi) = (add_days(date(2024, 10, 1), -30), date(2024, 10, 1));
    let rp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(creation_date.between(lo, hi))).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let rc = (&tu).group_by(Ident::<User>::new()).select(comments_by(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).between(lo, hi))).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let rv = (&tu).group_by(Ident::<User>::new()).select(votes_by(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).between(lo, hi))).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let v = drain((&us).and(&pc).and(&rp).and(&rc).and(&rv));
    rows(v.into_iter().map(|(u, ((((a, p), x), y), z))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(x), V::I(y), V::I(z)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostsCreated, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostsCreated, QuestionsAsked, AnswersGiven, TotalViews, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserReputation),
// BadgeCounts AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId),
// FinalReport AS (SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.PostsCreated, TU.QuestionsAsked, TU.AnswersGiven, TU.TotalViews, COALESCE(BC.BadgeCount, 0) AS TotalBadges,
//        COALESCE(BC.GoldBadges, 0) AS GoldBadges, COALESCE(BC.SilverBadges, 0) AS SilverBadges, COALESCE(BC.BronzeBadges, 0) AS BronzeBadges, TU.ReputationRank, TU.ViewRank
//     FROM TopUsers TU LEFT JOIN BadgeCounts BC ON TU.UserId = BC.UserId)
// SELECT *, (ReputationRank + ViewRank) AS OverallRank FROM FinalReport WHERE (ReputationRank + ViewRank) <= 10 ORDER BY OverallRank;
fn q27529(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let ur = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0)],
        None => a,
    });
    let v = ranked(drain(&ur), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[3]), false);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    type W = (((Id<User>, [i64; 4]), i64), i64);
    let v = drain(rel(v).filt(|((_, r), w)| r + w <= 10).select(Same::<W>::new().and(Same::<W>::new().map(|(((u, _), _), _): W| u).select((&ub).opt()))));
    rows(v.into_iter().map(|(_, ((((u, a), r), w), b))| {
        let b = b.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        f.extend([V::I(r), V::I(w), V::I(r + w)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        RANK() OVER (ORDER BY COUNT(v.Id) DESC) AS Rank FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
//     GROUP BY p.Id, p.Title, p.PostTypeId, p.CreationDate),
// CombinedStats AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.CommentCount, ps.UpVotes, ps.DownVotes, us.DisplayName AS TopUser, us.Rank AS UserRank
//     FROM PostStats ps JOIN UserVoteStats us ON ps.UpVotes > us.UpVotes WHERE us.Rank <= 10)
// SELECT cs.PostId, cs.Title, cs.CreationDate, cs.CommentCount, cs.UpVotes, cs.DownVotes, COALESCE(NULLIF(cs.TopUser, ''), 'No Votes') AS TopUser,
//        CASE WHEN cs.UpVotes - cs.DownVotes > 0 THEN 'Positive' WHEN cs.UpVotes - cs.DownVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS Sentiment,
//        (SELECT COUNT(*) FROM Posts p2 WHERE p2.CreationDate >= cs.CreationDate AND p2.Id != cs.PostId) AS NewerPostsCount
// FROM CombinedStats cs ORDER BY cs.UpVotes DESC, cs.CommentCount DESC;
//
// `ps.UpVotes > us.UpVotes` is a join on a comparison, so the recent posts and the top voters are crossed and filtered. NewerPostsCount is the number of posts
// not earlier than this one, less the post itself: `N - RANK() OVER (ORDER BY CreationDate)` over all posts.
fn q2145(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let uv = db.user.with((&db.user.reputation).gt(0)).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64],
        None => a,
    });
    let tu = ranked(drain(&uv), |&(_, a)| Reverse(a[0]), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| (u, a[1])).collect());
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let n = count(&db.post.creation_date);
    let rk = ranked(drain(&db.post.creation_date), |&(_, d)| d, false);
    let rk = rel(rk.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let mut v = Vec::new();
    (&ps).and((&rank).map(|(_, r)| r)).cross(&tu).filt(|((a, _), (_, up)): (([i64; 3], i64), (Id<User>, i64))| a[1] > up).drive(|(p, _), ((a, r), (u, _))| v.push((p, a, r, u)));
    rows(v.into_iter().map(|(p, a, r, u)| {
        let name = db.user.display_name.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(if name.is_empty() { "No Votes" } else { name })]);
        f.push(V::S(if a[1] - a[2] > 0 { "Positive" } else if a[1] - a[2] < 0 { "Negative" } else { "Neutral" }));
        f.push(V::I(n - r));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS Downvotes,
//        COUNT(CASE WHEN V.VoteTypeId = 6 THEN 1 END) AS CloseVotes, COUNT(CASE WHEN V.VoteTypeId = 7 THEN 1 END) AS ReopenVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.FavoriteCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (ORDER BY P.Score DESC) AS ScoreRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.Score, PS.OwnerDisplayName, US.Upvotes AS TotalUpvotes, US.Downvotes AS TotalDownvotes, US.CloseVotes AS TotalCloseVotes, US.ReopenVotes AS TotalReopenVotes
//     FROM PostStatistics PS JOIN UserVoteSummary US ON US.UserId IN (SELECT DISTINCT U.Id FROM Votes V JOIN Posts P ON V.PostId = P.Id JOIN Users U ON U.Id = P.OwnerUserId WHERE P.Id = PS.PostId)
//     WHERE PS.ScoreRank <= 10)
// SELECT TP.PostId, TP.Title, TP.Score, TP.OwnerDisplayName, TP.TotalUpvotes, TP.TotalDownvotes, TP.TotalCloseVotes, TP.TotalReopenVotes, (TP.TotalUpvotes - TP.TotalDownvotes) AS NetVotes
// FROM TopPosts TP ORDER BY TP.Score DESC, NetVotes DESC;
//
// The IN subquery is the owner of PS.PostId when that post has a vote, so each top question joins its owner's vote summary if it has been voted on.
fn q9634(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(score)), |&(_, s)| Reverse(s), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 4], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(6)) as i64, a[3] + (t == Some(7)) as i64]
    });
    let voted: MatSet<Id<Post>> = db.vote.select(&db.vote.post).collect();
    let v = drain((&tp).with(&voted).select(owner_user.select(&us)));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(a[0] - a[1]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.CreationDate, P.ViewCount, U.DisplayName AS Owner, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS RankScore,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
//     GROUP BY P.Id, P.Title, P.Score, P.CreationDate, P.ViewCount, U.DisplayName, P.PostTypeId),
// TopPosts AS (SELECT PostId, Title, Score, CreationDate, Owner, RankScore, UpVoteCount, DownVoteCount FROM RankedPosts WHERE RankScore <= 5),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId),
// FinalResult AS (SELECT TP.PostId, TP.Title, TP.Score, TP.CreationDate, TP.Owner, TP.UpVoteCount, TP.DownVoteCount, COALESCE(PC.CommentCount, 0) AS CommentCount,
//        CASE WHEN TP.Score >= 10 THEN 'High Score' WHEN TP.Score BETWEEN 5 AND 9 THEN 'Moderate Score' ELSE 'Low Score' END AS ScoreCategory FROM TopPosts TP LEFT JOIN PostComments PC ON TP.PostId = PC.PostId)
// SELECT FR.*, UP.AverageViewCount FROM FinalResult FR CROSS JOIN (SELECT AVG(ViewCount) AS AverageViewCount FROM Posts) UP WHERE FR.CommentCount > 0 ORDER BY FR.Score DESC, FR.CreationDate ASC LIMIT 100;
fn q1787(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let rs = per_group(ranked(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |&(_, t)| t);
    let tp: MatSet<Id<Post>> = rel(drain(rel(rs).filt(|(_, r)| r <= 5)).into_iter().map(|x| x.1 .0 .0).collect()).map(|p| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let (ws, wn) = view_count.fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let up = whole(db.post.select(Ident::<Post>::new())).fold((ws, wn), |a, _| a);
    let mut v = Vec::new();
    (&vs).and((&cc).filt(|n| n > 0)).cross(&up).drive(|(p, _), ((a, c), (s, n))| v.push((p, a, c, s, n)));
    let v = top_n(v, |&(p, ..)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 100);
    rows(v.into_iter().map(|(p, a, c, s, n)| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if sc >= 10 { "High Score" } else if sc >= 5 { "Moderate Score" } else { "Low Score" }), avg(s, n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.CreationDate, p.Title, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.OwnerUserId, p.CreationDate, p.Title, p.Score, p.ViewCount),
// TopUserPosts AS (SELECT rp.PostId, rp.OwnerUserId, rp.Title, rp.Score, rp.ViewCount, u.DisplayName, u.Reputation, u.Location, COALESCE(r.NumBadges, 0) AS NumBadges
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN (SELECT UserId, COUNT(*) AS NumBadges FROM Badges GROUP BY UserId) r ON u.Id = r.UserId WHERE rp.UserPostRank <= 5),
// UserActivity AS (SELECT p.OwnerUserId AS UserId, SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v JOIN Posts p ON v.PostId = p.Id WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId)
// SELECT up.Title, up.Score, up.ViewCount, up.DisplayName, up.Reputation, up.Location, COALESCE(ua.VoteCount, 0) AS TotalVotes, COALESCE(ua.UpVotes, 0) AS TotalUpVotes,
//        COALESCE(ua.DownVotes, 0) AS TotalDownVotes
// FROM TopUserPosts up LEFT JOIN UserActivity ua ON up.OwnerUserId = ua.UserId ORDER BY up.Reputation DESC, up.Score DESC;
fn q32451(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let ua = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| {
        [a[0] + matches!(t, 2 | 3) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]
    });
    let tr = rel(top);
    type R = (Id<Post>, Id<User>);
    let v = drain((&tr).select(Same::<R>::new().and(Same::<R>::new().map(|(_, u): R| u).select((&ua).opt()))));
    rows(v.into_iter().map(|(_, ((p, u), a))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(ostr(db.user.location.get(u)));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// Rewritten (rewrites/32545.sql): `, p.Id` added to the ROW_NUMBER order, since a question and its self-answer share owner and CreationDate.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS Rank,
//        COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// RecentBadges AS (SELECT u.DisplayName, b.Name AS BadgeName, b.Date AS BadgeDate FROM Users u JOIN Badges b ON u.Id = b.UserId WHERE b.Date >= CURRENT_TIMESTAMP - INTERVAL '30 days'),
// PostHistoryAggregates AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseVotes, COUNT(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeleteVotes,
//        COUNT(*) AS TotalHistoryCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rb.DisplayName AS BadgeOwner, rb.BadgeName, pha.CloseVotes, pha.DeleteVotes, (rp.UpVoteCount - rp.DownVoteCount) AS NetVotes,
//        CASE WHEN pha.TotalHistoryCount > 0 THEN 'Has History' ELSE 'No History' END AS HistoryStatus
// FROM RankedPosts rp LEFT JOIN RecentBadges rb ON rp.OwnerUserId = (SELECT u.Id FROM Users u WHERE u.DisplayName = rb.DisplayName LIMIT 1) LEFT JOIN PostHistoryAggregates pha ON rp.PostId = pha.PostId
// WHERE rp.Rank = 1 AND rp.Score > 5 ORDER BY rp.CreationDate DESC LIMIT 50;
//
// Rank reads only base columns, so each owner's newest post (the ownerless posts as one partition) is picked first and the comment x vote product driven for those.
// The correlated `LIMIT 1` has no ORDER BY; the port takes the smallest user id with that name. It cannot be observed here: RecentBadges is empty, since no badge
// is within 30 days of CURRENT_TIMESTAMP.
fn q32545(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let first = top_per(drain(db.post.select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&rp)
        .with(score.gt(5))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .buf_fold(|rows| [distinct_some(rows.iter().map(|r| r.0)), rows.iter().map(|r| (r.1 == Some(2)) as i64 - (r.1 == Some(3)) as i64).sum()]);
    let since = now_utc() - 30 * DAY_US;
    let rb = rel(drain(db.badge.with((&db.badge.date).filt(move |d| ny_to_utc(d) >= since)).select((&db.badge.user).select(&db.user.display_name).and(&db.badge.name))));
    let first_of: Fold<Str, i64> = db.user.group_by(&db.user.display_name).select(&db.user.origid).fold(i64::MAX, |m, i| m.min(i));
    type B = (Id<Badge>, (Str, Str));
    let by_owner: HashIdx<i64, B> = (&rb).select(Same::<B>::new().map(|(_, (n, _)): B| n).select(&first_of)).inv().select(&rb).collect();
    let PostHistory { post_history_type_id, .. } = &db.post_history;
    let pha = (&rp).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id)).fold([0i64; 3], |a, t| [a[0] + matches!(t, 10 | 11) as i64, a[1] + matches!(t, 12 | 13) as i64, a[2] + 1]);
    let v = drain((&s).and(owner_user.select(&db.user.origid).select(&by_owner).opt()).and((&pha).opt()));
    let v = top_n(v, |&(p, ((_, b), _))| (Reverse(creation_date.get(p).unwrap()), p, b.map(|b| b.0)), 50);
    rows(v.into_iter().map(|(p, ((a, b), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(a[0]));
        f.extend(match b {
            Some((_, (n, bn))) => [V::S(n), V::S(bn)],
            None => [V::Null, V::Null],
        });
        f.extend(match h {
            Some(h) => [V::I(h[0]), V::I(h[1])],
            None => [V::Null, V::Null],
        });
        f.push(V::I(a[1]));
        f.push(V::S(if h.map_or(false, |h| h[2] > 0) { "Has History" } else { "No History" }));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, u.UpVotes, u.DownVotes, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank,
//        ROW_NUMBER() OVER (PARTITION BY u.Location ORDER BY u.Reputation DESC) AS LocationRank, u.Location FROM Users u),
// PostDetails AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.Score, p.ViewCount, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.OwnerUserId, p.Title, p.Score, p.ViewCount, p.CreationDate),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.Score, pd.ViewCount, pd.CommentCount, ROW_NUMBER() OVER (ORDER BY pd.Score DESC, pd.ViewCount DESC) AS ScoreRank FROM PostDetails pd WHERE pd.Score > 0)
// SELECT us.DisplayName AS UserName, us.Reputation, pp.Title AS PostTitle, pp.Score, pp.ViewCount, pp.CommentCount, CASE WHEN us.Location IS NULL THEN 'Unknown Location' ELSE us.Location END AS UserLocation,
//        pht.Name AS PostHistoryType
// FROM UserStatistics us LEFT JOIN Posts p ON us.UserId = p.OwnerUserId LEFT JOIN TopPosts pp ON p.Id = pp.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// WHERE us.ReputationRank <= 50 AND (pp.ScoreRank <= 10 OR pp.ViewCount > 100) AND pp.CommentCount > COALESCE((SELECT AVG(CommentCount) FROM PostDetails), 0)
// ORDER BY us.Reputation DESC, pp.Score DESC;
//
// CommentCount is counted over the comment x vote product, as the SQL does, for every post, since its average is the threshold; that
// average is compared exactly, as `CommentCount * n > sum`.
fn q4592(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let pd = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let (cs, cn) = (&pd).fold_flat((0i64, 0i64), |(s, n), c| (s + c, n + 1));
    let ranks = top_n(drain(db.post.with(score.gt(0)).select(Ident::<Post>::new())), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let top10: MatSet<Id<Post>> = rel(ranks.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ur = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let tu: MatSet<Id<User>> = rel(ur.into_iter().take_while(|x| x.1 <= 50).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let pp = Ident::<Post>::new()
        .with(score.gt(0))
        .and(Ident::<Post>::new().with(&top10).opt())
        .and(view_count.opt())
        .filt(|((_, t), w): ((Id<Post>, Option<Id<Post>>), Option<i64>)| t.is_some() || w.map_or(false, |w| w > 100))
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and((&pd).filt(move |c| c * cn > cs)));
    let v = drain((&tu).select(posts_of(db).select(pp.and(history_of(db).opt()))));
    rows(v.into_iter().map(|(u, ((p, c), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(V::I(c));
        f.push(V::S(db.user.location.get(u).unwrap_or("Unknown Location")));
        f.push(h.map_or(V::Null, |h| V::S(htype_name(db).get(h).unwrap())));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(p.AnswerCount, 0) AS AnswerCount, COALESCE(p.CommentCount, 0) AS CommentCount,
//        COALESCE(p.FavoriteCount, 0) AS FavoriteCount, p.OwnerUserId, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId IN (2, 3)) AS TotalVotes,
//        CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END AS IsAcceptedAnswer FROM Posts p),
// CommentStatistics AS (SELECT p.Id AS PostId, COUNT(c.Id) AS TotalComments FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.FavoriteCount, pd.TotalVotes, pd.IsAcceptedAnswer, ur.DisplayName, ur.ReputationRank
//     FROM PostDetails pd JOIN UserReputation ur ON pd.OwnerUserId = ur.UserId WHERE pd.ViewCount > 100 AND pd.AnswerCount > 0 ORDER BY pd.ViewCount DESC LIMIT 10)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.FavoriteCount, tp.TotalVotes, tp.IsAcceptedAnswer, COALESCE(cs.TotalComments, 0) AS TotalComments,
//        tp.DisplayName AS UserDisplayName, tp.ReputationRank
// FROM TopPosts tp LEFT JOIN CommentStatistics cs ON tp.PostId = cs.PostId ORDER BY tp.ReputationRank, tp.ViewCount DESC;
fn q720(db: &'static So) -> String {
    let Post { owner_user, view_count, answer_count, .. } = &db.post;
    let ur = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let ur = rel(ur.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    let v = drain(db.post.with(view_count.gt(100)).with(answer_count.gt(0)).select(view_count.and(owner_user.select(&rank))));
    let v = top_n(v, |&(p, (w, _))| (Reverse(w), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    type P = (Id<Post>, (i64, (Id<User>, i64)));
    let v = drain(rel(v).select(Same::<P>::new().and(Same::<P>::new().map(|(p, _): P| p).select((&tv).and(&cc)))));
    rows(v.into_iter().map(|(_, ((p, (_, (u, r))), (t, c)))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "answers", "comments"]);
        f.push(V::I(db.post.favorite_count.get(p).unwrap_or(0)));
        f.extend([V::I(t), V::I(db.post.accepted_answer_id.get(p).is_some() as i64), V::I(c), user_col(db, u, "name"), V::I(r)]);
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotesCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotesCount, COALESCE(COUNT(DISTINCT P.Id), 0) AS PostsCount, COALESCE(SUM(COALESCE(P.Score, 0)), 0) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// UserRankings AS (SELECT UserId, DisplayName, Reputation, UpVotesCount, DownVotesCount, PostsCount, TotalScore, RANK() OVER (ORDER BY Reputation DESC, TotalScore DESC) AS UserRank FROM UserMetrics),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, UserRank FROM UserRankings WHERE UserRank <= 10),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, COALESCE(V.UpVotesCount, 0) AS TotalUpVotes, COALESCE(V.DownVotesCount, 0) AS TotalDownVotes,
//        P.Score AS PostScore, P.OwnerUserId
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     LEFT JOIN (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotesCount FROM Votes GROUP BY PostId) V ON P.Id = V.PostId)
// SELECT T.UserRank, T.DisplayName, TD.Title, TD.CreationDate, TD.TotalUpVotes, TD.TotalDownVotes, TD.PostScore FROM TopUsers T JOIN PostDetails TD ON T.UserId = TD.OwnerUserId
// ORDER BY T.UserRank, TD.TotalUpVotes DESC OFFSET 5 ROWS FETCH NEXT 5 ROWS ONLY;
//
// UserRank leads with Reputation, so only users with at least the tenth-highest reputation can rank in the top ten; the posts x votes product is driven for those alone.
fn q4611(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let tenth = top_n(drain(rep), |&(u, r)| (Reverse(r), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with(rep.ge(tenth)).collect();
    let um = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.score).and(votes_of(db).opt())).opt()).fold(0i64, |s, p| s + p.map_or(0, |(s, _)| s));
    let v = ranked(drain(&um), |&(u, s)| (Reverse(rep.get(u).unwrap()), Reverse(s)), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let vs = db.post.with(&db.post.owner_user).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&tu).select(Same::<(Id<User>, i64)>::new().and(Same::<(Id<User>, i64)>::new().map(|(u, _): (Id<User>, i64)| u).select(posts_of(db).select(Ident::<Post>::new().and(&vs))))));
    let v = top_n(v, |&(_, ((_, r), (p, a)))| (r, Reverse(a[0]), p), 10);
    rows(v.into_iter().skip(5).map(|(_, ((u, r), (p, a)))| {
        let mut f = vec![V::I(r), user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(db.post.score.get(p).unwrap())]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostStats AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        MAX(COALESCE(p.LastActivityDate, p.CreationDate)) AS MostRecentActivity
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId),
// PostDetails AS (SELECT ps.PostId, ur.DisplayName AS OwnerDisplayName, ps.TotalComments, ps.UpVotes, ps.DownVotes,
//        CASE WHEN ps.UpVotes - ps.DownVotes > 0 THEN 'Positive' WHEN ps.UpVotes - ps.DownVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment,
//        COUNT(ph.Id) FILTER (WHERE ph.PostHistoryTypeId = 10) AS CloseCount, COUNT(ph.Id) FILTER (WHERE ph.PostHistoryTypeId = 11) AS ReopenCount
//     FROM PostStats ps LEFT JOIN UserReputation ur ON ps.OwnerUserId = ur.UserId LEFT JOIN PostHistory ph ON ps.PostId = ph.PostId WHERE ur.ReputationRank <= 10
//     GROUP BY ps.PostId, ur.DisplayName, ps.TotalComments, ps.UpVotes, ps.DownVotes)
// SELECT pd.OwnerDisplayName, pd.TotalComments, pd.UpVotes, pd.DownVotes, pd.VoteSentiment, pd.CloseCount, pd.ReopenCount, (pd.UpVotes + pd.ReopenCount - pd.CloseCount - pd.DownVotes) AS EngagementScore
// FROM PostDetails pd WHERE pd.CloseCount > pd.ReopenCount ORDER BY EngagementScore DESC LIMIT 10;
//
// The WHERE on ur makes the LEFT JOIN an inner one, so only the ten highest-reputation users' recent posts are aggregated (ties at the tenth place go to the smaller id).
fn q4175(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { creation_date, owner_user, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user.select(Ident::<User>::new().with(&tu)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ph = db.post.with(&ps).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(11)) as i64]);
    let v = drain((&ps).and((&ph).filt(|h| h[0] > h[1])).and(owner_user));
    let score = |a: [i64; 3], h: [i64; 2]| a[1] + h[1] - h[0] - a[2];
    let v = top_n(v, |&(p, ((a, h), _))| (Reverse(score(a, h)), p), 10);
    rows(v.into_iter().map(|(_, ((a, h), u))| {
        let net = a[1] - a[2];
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(if net > 0 { "Positive" } else if net < 0 { "Negative" } else { "Neutral" }), V::I(h[0]), V::I(h[1]), V::I(score(a, h))])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rnk, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostsCount, SUM(COALESCE(b.Class, 0)) AS TotalBadges, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostAnalysis AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, rp.VoteCount, us.UserId, us.DisplayName AS OwnerDisplayName, us.Reputation AS OwnerReputation,
//        us.PostsCount, us.TotalBadges, us.TotalScore FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId WHERE rp.Rnk <= 5)
// SELECT pa.Title, pa.CreationDate, pa.ViewCount, pa.Score, pa.CommentCount, pa.VoteCount, pa.OwnerDisplayName, pa.OwnerReputation, pa.PostsCount, pa.TotalBadges, pa.TotalScore
// FROM PostAnalysis pa ORDER BY pa.Score DESC, pa.ViewCount DESC;
//
// Rnk reads only base columns, so each owner's five posts are picked first; the posts x badges product is driven for their owners alone.
fn q5789(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user)), |&(_, u)| u, |&(p, _)| {
        (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let us = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 2], |a, (s, c)| [a[0] + c.unwrap_or(0), a[1] + s.unwrap_or(0)]);
    let dp = user_distinct_posts(db);
    let v = drain((&cc).and(&vc).and(owner_user.select(Ident::<User>::new().and(&us).and(&dp))));
    rows(v.into_iter().map(|(p, ((c, n), ((u, a), k)))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(k), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS TotalGoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS TotalSilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS TotalBronzeBadges, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.TotalPosts, us.TotalQuestions, us.TotalAcceptedAnswers, us.TotalGoldBadges, us.TotalSilverBadges, us.TotalBronzeBadges, us.ReputationRank
//     FROM UserStatistics us WHERE us.TotalPosts > 0 ORDER BY us.ReputationRank LIMIT 10)
// SELECT tu.DisplayName, tu.TotalQuestions, tu.TotalAcceptedAnswers, tu.TotalGoldBadges, tu.TotalSilverBadges, tu.TotalBronzeBadges, tu.ReputationRank,
//        COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS TotalCloseVotes, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 24 THEN 1 ELSE 0 END), 0) AS TotalSuggestedEdits
// FROM TopUsers tu LEFT JOIN Posts p ON tu.UserId = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// GROUP BY tu.DisplayName, tu.TotalQuestions, tu.TotalAcceptedAnswers, tu.TotalGoldBadges, tu.TotalSilverBadges, tu.TotalBronzeBadges, tu.ReputationRank ORDER BY tu.ReputationRank;
//
// `TotalPosts > 0` holds exactly for the users with a post, and the order is the reputation rank, so the ten users are picked first and the posts x badges product
// is driven for those alone.
fn q9294(db: &'static So) -> String {
    let rk = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let rk = rel(rk.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rk).map(|(u, _)| u).inv().select(&rk).collect();
    let owners: MatSet<Id<User>> = db.post.select(&db.post.owner_user).collect();
    let tu = top_n(drain((&owners).select((&rank).map(|(_, r)| r))), |&(u, r)| (r, u), 10);
    let tu = rel(tu);
    let tus: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let us = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (p, c)| {
            let (q, x) = p.map_or((false, false), |(t, x)| (t == 1, t == 2 && x.is_some()));
            [a[0] + q as i64, a[1] + x as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
        });
    let ph = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(24)) as i64]
    });
    rows(drain((&tu).select(Same::<(Id<User>, i64)>::new().and(Same::<(Id<User>, i64)>::new().map(|(u, _): (Id<User>, i64)| u).select((&us).and(&ph))))).into_iter().map(|(_, ((u, r), (a, h)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([V::I(r), V::I(h[0]), V::I(h[1])]);
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(*) AS TotalPosts, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, AVG(COALESCE(p.Score, 0)) AS AvgScore FROM Posts p GROUP BY p.OwnerUserId),
// UserEngagement AS (SELECT u.Id, u.DisplayName, us.BadgeCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges, ps.TotalPosts, ps.TotalQuestions, ps.TotalAnswers, ps.TotalViews, ps.AvgScore
//     FROM Users u JOIN UserBadgeStats us ON u.Id = us.UserId JOIN PostStats ps ON u.Id = ps.OwnerUserId),
// TopUsers AS (SELECT ue.*, ROW_NUMBER() OVER (ORDER BY ue.TotalPosts DESC, ue.TotalViews DESC) AS Rank FROM UserEngagement ue)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, COALESCE(tu.TotalAnswers, 0) AS TotalAnswers, tu.TotalViews, tu.AvgScore, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges,
//        CASE WHEN tu.BadgeCount IS NULL THEN 'No Badges' ELSE 'Has Badges' END AS BadgeStatus,
//        CASE WHEN tu.TotalPosts > 100 THEN 'Veteran' WHEN tu.TotalPosts BETWEEN 50 AND 100 THEN 'Contributor' ELSE 'Novice' END AS UserLevel
// FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Rank;
fn q21407(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let Post { owner_user, post_type_id, view_count, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 5], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + s]
    });
    let v = top_n(drain((&ps).and(&ub)), |&(u, (a, _))| (Reverse(a[0]), Reverse(a[3]), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0])];
        f.extend(b.map(V::I));
        f.push(V::S("Has Badges"));
        f.push(V::S(if a[0] > 100 { "Veteran" } else if a[0] >= 50 { "Contributor" } else { "Novice" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment, ph.UserDisplayName, pt.Name AS PostHistoryType FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id
//     WHERE pt.Name IN ('Post Closed', 'Post Reopened')),
// UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.OwnerDisplayName, CASE WHEN cb.PostId IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus, ub.BadgeCount, ub.HighestBadgeClass
// FROM RankedPosts rp LEFT JOIN ClosedPosts cb ON rp.PostId = cb.PostId JOIN UserBadgeCounts ub ON rp.OwnerUserId = ub.UserId WHERE ub.BadgeCount > 0
// ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 100;
fn q30124(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold((0i64, i64::MIN), |(n, m), c| (n + 1, m.max(c)));
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user.select(&ub))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(htype_name(db).is_in(["Post Closed", "Post Reopened"])));
    let v = drain((&rp).and(closes.opt()).and(owner_user.select(&ub)));
    let v = top_n(v, |&(p, ((_, h), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, h), 100);
    rows(v.into_iter().map(|(p, ((c, h), (n, m)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::S(if h.is_some() { "Closed" } else { "Open" }), V::I(n), V::I(m)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.ViewCount DESC) AS RankByViews,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount > 0),
// TagsWithHighCount AS (SELECT t.Id AS TagId, t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON position(t.TagName IN p.Tags) > 0 GROUP BY t.Id, t.TagName HAVING COUNT(p.Id) > 100),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation > 1000 ORDER BY UserRank),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, COALESCE(badge.BadgeCount, 0) AS TotalBadges,
//        CASE WHEN rp.RankByViews < 5 OR rp.RankByScore < 5 THEN 'Underperforming' WHEN EXISTS (SELECT 1 FROM TagsWithHighCount t WHERE position(t.TagName IN rp.Title) > 0) THEN 'Trending Topic'
//        ELSE 'Well-Performed' END AS PerformanceCategory, u.DisplayName AS TopUser
// FROM RankedPosts rp LEFT JOIN UserBadges badge ON badge.UserId = rp.PostId CROSS JOIN (SELECT DisplayName FROM TopUsers WHERE UserRank = 1) u
// WHERE rp.RankByViews <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 50;
//
// `badge.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q22537(db: &'static So) -> String {
    let Post { creation_date, view_count, score, post_type, title, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(view_count.gt(0))).select(post_type.and(view_count)));
    let bv = per_group(ranked(v.clone(), |&(p, (t, w))| (t, Reverse(w), p), false), |&(_, (t, _))| t);
    let bs = per_group(ranked(v, |&(p, (t, _))| (t, Reverse(score.get(p).unwrap()), p), false), |&(_, (t, _))| t);
    let rs = rel(bs.into_iter().map(|((p, _), r)| (p, r)).collect());
    let by_score: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rs).map(|(p, _)| p).inv().select(&rs).collect();
    let rv = rel(drain(rel(bv).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((p, _), r))| (p, r)).collect());
    let lt = tag_mentions(db);
    let hot = db.tag.group_by(Ident::<Tag>::new()).select((&lt).map(|(_, t)| t).inv().collect::<HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)>>()).fold(0i64, |n, _| n + 1);
    let hot: HashIdx<Str, Id<Tag>> = db.tag.with((&hot).filt(|n| n > 100)).select(&db.tag.tag_name).inv().collect();
    let titles: MatSet<Str> = (&rv).map(|(p, _)| p).select(title).collect();
    let trending: HashIdx<Str, Id<Tag>> = (&titles).select_where(&hot, |t: Str, n: Str| t.contains(n)).collect();
    let tu = ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 == 1).map(|x| x.0 .0).collect());
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    type R = (Id<Post>, i64);
    let base = Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&by_score).map(|(_, r)| r).and((&db.post.origid).select(&uidx).select(&bc).opt()).and(Ident::<Post>::new().with(title.select(&trending)).opt())));
    let mut v = Vec::new();
    (&rv).select(base).cross(&tu).drive(|(i, _), (((p, rv), ((rs, b), t)), u)| v.push((i, p, rv, rs, b, t, u)));
    let v = top_n(v, |&(i, p, ..)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p).unwrap()), i), 50);
    rows(v.into_iter().map(|(_, p, rv, rs, b, t, u)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.push(V::I(b.unwrap_or(0)));
        f.push(V::S(if rv < 5 || rs < 5 { "Underperforming" } else if t.is_some() { "Trending Topic" } else { "Well-Performed" }));
        f.push(user_col(db, u, "name"));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rn
//     FROM Posts p WHERE p.CreationDate > CURRENT_DATE - INTERVAL '30 days'),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Users u LEFT JOIN Comments c ON c.UserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT ph.PostId, ph.UserId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId, ph.UserId),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsEdited FROM Users u JOIN PostHistory ph ON ph.UserId = u.Id JOIN Posts p ON p.Id = ph.PostId
//     WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ue.DisplayName AS EngagedUser, ue.CommentCount, ue.TotalBounty, phs.EditCount, phs.LastEditDate, mau.PostsEdited AS ActiveUserEdits
// FROM RecentPosts rp LEFT JOIN UserEngagement ue ON rp.OwnerUserId = ue.UserId LEFT JOIN PostHistoryStats phs ON rp.PostId = phs.PostId LEFT JOIN MostActiveUsers mau ON mau.UserId = phs.UserId
// WHERE rp.Rn = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// Rn reads only base columns, so the newest post of each type is picked first and the comments x votes product is driven for their owners alone.
fn q31635(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let first = top_per(drain(db.post.with(creation_date.gt(add_days(current_date(), -30))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&rp).select(owner_user).collect();
    let ue = (&owners)
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let PostHistory { post, user, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let edits = || db.post_history.with(post_history_type_id.is_in([4, 5, 6]));
    let phs = edits().with(post.select(Ident::<Post>::new().with(&rp))).group_by(post.and(user.opt())).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mau = edits().with(user).group_by(user).select(post).buf_fold(|ps| distinct_some(ps.iter().map(|&p| Some(p))));
    let pv = rel(drain((&phs).and(Same::<(Id<Post>, Option<Id<User>>)>::new().flat_map(|(_, u): (Id<Post>, Option<Id<User>>)| u).select(&mau).opt())).into_iter().map(|(k, (a, m))| (k, a, m)).collect());
    let by_post: HashIdx<Id<Post>, ((Id<Post>, Option<Id<User>>), (i64, i64), Option<i64>)> = (&pv).map(|((p, _), _, _)| p).inv().select(&pv).collect();
    let v = drain((&rp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ue)).opt()).and((&by_post).opt())));
    rows(v.into_iter().map(|(_, ((p, u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(match u {
            Some((u, a)) => [user_col(db, u, "name"), V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match h {
            Some((_, (n, d), m)) => [V::I(n), V::T(d), oint(m)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RECURSIVE UserPostCounts AS (SELECT U.Id AS UserId, COUNT(P.Id) AS PostCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id),
// RecentPostHistory AS (SELECT PH.UserId, PH.PostId, PH.CreationDate, P.Title, P.LastActivityDate, RANK() OVER (PARTITION BY PH.UserId ORDER BY PH.CreationDate DESC) AS RecentRank
//     FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// BadgesCounts AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount FROM Badges B GROUP BY B.UserId),
// VoteStatistics AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN V.VoteTypeId = 10 THEN 1 ELSE 0 END) AS DeleteVotes FROM Votes V GROUP BY V.PostId)
// SELECT U.DisplayName, U.Reputation, COALESCE(UPC.PostCount, 0) AS TotalPosts, COALESCE(BC.BadgeCount, 0) AS TotalBadges, COALESCE(RPH.Title, 'No Recent Activity') AS RecentPostTitle,
//        RPH.CreationDate AS RecentPostDate, V.UpVotes, V.DownVotes, V.DeleteVotes,
//        CASE WHEN UPC.PostCount = 0 THEN 'New User' WHEN U.Reputation < 100 THEN 'Regular User' ELSE 'Experienced User' END AS UserType
// FROM Users U LEFT JOIN UserPostCounts UPC ON U.Id = UPC.UserId LEFT JOIN RecentPostHistory RPH ON U.Id = RPH.UserId AND RPH.RecentRank = 1 LEFT JOIN BadgesCounts BC ON U.Id = BC.UserId
// LEFT JOIN VoteStatistics V ON V.PostId = RPH.PostId WHERE U.Reputation > 0 ORDER BY U.Reputation DESC, TotalPosts DESC LIMIT 100;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q34426(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { user, creation_date: hd, post, .. } = &db.post_history;
    let recent = || db.post_history.with(hd.ge(add_days(date(2024, 10, 1), -30))).with(user);
    let md = recent().group_by(user).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<User>, i64), Id<PostHistory>> = recent().select(user.and(hd)).inv().collect();
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 10) as i64]);
    let v = drain(db.user.with((&db.user.reputation).gt(0)).select((&pc).and((&bc).opt()).and(Ident::<User>::new().and(&md).select(&at).select(Ident::<PostHistory>::new().and(post.select(&vs).opt())).opt())));
    let v = top_n(v, |&(u, ((n, _), h))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u, h.map(|h| h.0)), 100);
    rows(v.into_iter().map(|(u, ((n, b), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(b.unwrap_or(0))]);
        f.extend(match h {
            Some((h, x)) => {
                let p = post.get(h).unwrap();
                let mut g = vec![V::S(db.post.title.get(p).unwrap_or("No Recent Activity")), V::T(hd.get(h).unwrap())];
                g.extend(match x {
                    Some(a) => a.map(V::I),
                    None => [V::Null, V::Null, V::Null],
                });
                g
            }
            None => vec![V::S("No Recent Activity"), V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::S(if n == 0 { "New User" } else if db.user.reputation.get(u).unwrap() < 100 { "Regular User" } else { "Experienced User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id, u.Reputation, u.DisplayName, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.DisplayName),
// UserPostStats AS (SELECT ur.DisplayName, ur.Reputation, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges, COUNT(rp.Id) AS PostCount, AVG(rp.Score) AS AvgScore, SUM(rp.ViewCount) AS TotalViews
//     FROM UserReputation ur JOIN RankedPosts rp ON ur.Id = rp.OwnerUserId GROUP BY ur.DisplayName, ur.Reputation, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT ups.DisplayName, ups.Reputation, ups.GoldBadges, ups.SilverBadges, ups.BronzeBadges, ups.PostCount, ups.AvgScore, ups.TotalViews, COALESCE(cp.CloseCount, 0) AS TotalClosedPosts
// FROM UserPostStats ups LEFT JOIN ClosedPosts cp ON ups.PostCount = cp.PostId WHERE ups.Reputation > 1000 ORDER BY ups.TotalViews DESC NULLS LAST LIMIT 20;
//
// UserPostStats groups by the owner's name, reputation and badge counts, not the user id. `ups.PostCount = cp.PostId` joins a count to a post id, through the raw ids.
fn q3051(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let key = owner_user.select((&db.user.display_name).and(&db.user.reputation).and(&ub));
    let ups = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))
        .group_by(key)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    type K = ((Str, i64), [i64; 3]);
    let v = drain(&ups);
    let cps = rel(v);
    type Row = (K, [i64; 4]);
    let v = drain((&cps).select(Same::<Row>::new().and(Same::<Row>::new().map(|(_, a): Row| a[0]).select(&pidx).select(&cp).opt())));
    let v = top_n(v, |&(i, ((_, a), _))| (a[2] == 0, Reverse(a[3]), i), 20);
    rows(v.into_iter().map(|(_, ((((n, r), b), a), c))| {
        let mut f = vec![V::S(n), V::I(r)];
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(v.Id) DESC) AS VoteRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserID, u.Reputation, COALESCE(b.Gold, 0) AS GoldBadges, COALESCE(b.Silver, 0) AS SilverBadges, COALESCE(b.Bronze, 0) AS BronzeBadges
//     FROM Users u LEFT JOIN (SELECT UserId, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS Gold, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS Silver, SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS Bronze
//     FROM Badges GROUP BY UserId) b ON u.Id = b.UserId)
// SELECT rp.PostID, rp.Title, rp.CreationDate, u.DisplayName AS OwnerDisplayName, u.Reputation, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges, rp.CommentCount, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.UpVotes > rp.DownVotes THEN 'Positive' WHEN rp.UpVotes < rp.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserReputation ur ON u.Id = ur.UserID WHERE rp.VoteRank = 1 ORDER BY rp.CommentCount DESC, rp.UpVotes - rp.DownVotes DESC LIMIT 10;
fn q3338(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + t.is_some() as i64]);
    let v = drain((&rp).and(owner_user));
    let v = per_group(ranked(v, |&(_, (a, u))| (u, Reverse(a[3])), false), |&(_, (_, u))| u);
    let ub = badge_classes(db);
    type R = ((Id<Post>, ([i64; 4], Id<User>)), i64);
    let v = drain(rel(v).filt(|(_, r)| r == 1).select(Same::<R>::new().and(Same::<R>::new().map(|((_, (_, u)), _): R| u).select((&ub).opt()))));
    let v = top_n(v.into_iter().map(|(_, (((p, x), _), b))| (p, x, b)).collect(), |&(p, (a, _), _)| (Reverse(a[0]), Reverse(a[1] - a[2]), p), 10);
    rows(v.into_iter().map(|(p, (a, u), b)| {
        let b = b.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// RecentBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Date >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY b.UserId),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers, SUM(COALESCE(p.CommentCount, 0)) AS TotalComments,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY u.Id, u.DisplayName ORDER BY TotalAnswers DESC, TotalComments DESC LIMIT 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, rb.BadgeCount, mau.DisplayName AS ActiveUser, mau.TotalAnswers,
//        mau.TotalComments, mau.TotalUpvotes
// FROM RankedPosts rp LEFT JOIN RecentBadges rb ON rp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = rb.UserId) LEFT JOIN MostActiveUsers mau ON rp.OwnerDisplayName = mau.DisplayName
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// Both LEFT JOINs match on the display name, so an owner's post joins every recent-badge user and every active user with that name.
fn q7122(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, answer_count, comment_count, .. } = &db.post;
    let t0 = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let recent = || db.post.with(creation_date.ge(t0));
    let top = top_per(drain(recent().with(owner_user).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rb = db.badge.with((&db.badge.date).ge(t0)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let mau = recent()
        .with(owner_user)
        .group_by(owner_user)
        .select(answer_count.opt().and(comment_count).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, ((n, c), t)| [a[0] + n.unwrap_or(0), a[1] + c, a[2] + (t == Some(2)) as i64]);
    let mau = rel(top_n(drain(&mau), |&(u, a)| (Reverse(a[0]), Reverse(a[1]), u), 10));
    let mau_name: HashIdx<Str, (Id<User>, [i64; 3])> = (&mau).map(|(u, _)| u).select(&db.user.display_name).inv().select(&mau).collect();
    let rbv = rel(drain(&rb));
    let rb_name: HashIdx<Str, (Id<User>, i64)> = (&rbv).map(|(u, _)| u).select(&db.user.display_name).inv().select(&rbv).collect();
    let name = owner_user.select(&db.user.display_name);
    let v = drain((&tp).select(Ident::<Post>::new().and(name.select(&rb_name).opt()).and(owner_user.select(&db.user.display_name).select(&mau_name).opt())));
    rows(v.into_iter().map(|(_, ((p, b), m))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.push(b.map_or(V::Null, |(_, n)| V::I(n)));
        f.extend(match m {
            Some((u, a)) => vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])],
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank <= 5),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(distinct b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT tp.PostId, tp.Title, ta.TotalBounty, ta.BadgeCount,
//        CASE WHEN tp.ViewCount < 100 THEN 'Low Engagement' WHEN tp.ViewCount BETWEEN 100 AND 1000 THEN 'Moderate Engagement' ELSE 'High Engagement' END AS EngagementLevel
//     FROM TopPosts tp LEFT JOIN UserActivity ta ON tp.PostId = ta.UserId)
// SELECT pd.Title, pd.EngagementLevel, COALESCE(pd.TotalBounty, 0) AS TotalBounty, COALESCE(pd.BadgeCount, 0) AS BadgeCount
// FROM PostDetails pd LEFT JOIN PostHistory ph ON pd.PostId = ph.PostId AND ph.PostHistoryTypeId NOT IN (12, 10)
// WHERE pd.EngagementLevel = 'High Engagement' AND pd.BadgeCount > 0 ORDER BY pd.TotalBounty DESC, pd.BadgeCount DESC, pd.Title;
//
// Rank reads only base columns, so the top posts are picked first. `tp.PostId = ta.UserId` joins a post id to a user id, so it goes through the raw ids;
// a NULL ViewCount falls to the ELSE branch, 'High Engagement'.
fn q23578(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(date(2023, 1, 1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let high = |w: Option<i64>| w.map_or(true, |w| w > 1000);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let users: MatSet<Id<User>> = (&tp).select((&db.post.origid).select(&uidx)).collect();
    let ua = (&users).group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let bc = (&users).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let rest = history_of(db).select(Ident::<PostHistory>::new().minus((&db.post_history.post_history_type_id).is_in([10, 12])));
    let v = drain((&tp).with(view_count.opt().filt(move |w| high(w))).select((&db.post.origid).select(&uidx).select((&ua).and(&bc)).and(rest.opt())));
    rows(v.into_iter().map(|(p, ((b, n), _))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::S("High Engagement"), V::I(b), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate ASC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostVotes AS (SELECT v.PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVotes, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVotes FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
//     GROUP BY v.PostId)
// SELECT p.Id, p.Title, p.CreationDate, u.DisplayName, u.Reputation, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges, pv.UpVotes, pv.DownVotes,
//        CASE WHEN pv.UpVotes > pv.DownVotes THEN 'Positive' WHEN pv.DownVotes > pv.UpVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN PostVotes pv ON p.Id = pv.PostId
// WHERE p.Score > 10 AND EXISTS (SELECT 1 FROM Comments c WHERE c.PostId = p.Id GROUP BY c.PostId HAVING COUNT(*) > 2) AND p.Id IN (SELECT Id FROM RankedPosts WHERE PostRank <= 5)
// ORDER BY p.CreationDate DESC, p.Score DESC;
fn q502(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| {
        (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p)
    }, 5, false);
    let ranked_ids: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let pv = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let v = drain(db.post.with(score.gt(10)).with((&cc).filt(|n| n > 2)).with(&ranked_ids).select(owner_user.select(Ident::<User>::new().and(&ub)).and((&pv).opt())));
    rows(v.into_iter().map(|(p, ((u, b), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(b.map(V::I));
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::S(if a[0] > a[1] { "Positive" } else if a[1] > a[0] { "Negative" } else { "Neutral" })],
            None => [V::Null, V::Null, V::S("Neutral")],
        });
        row(f)
    }))
}

// WITH RecursivePostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COALESCE(v.UpVotes, 0) AS UpVoteCount, COALESCE(v.DownVotes, 0) AS DownVoteCount, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS RecentEditOrder, CASE WHEN p.PostTypeId = 1 THEN ph.Comment ELSE NULL END AS CloseReason
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v
//     ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10),
// PostWithDetails AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.OwnerUserId, ps.UpVoteCount, ps.DownVoteCount, ps.ViewCount, ps.RecentEditOrder, ps.CloseReason,
//        ROW_NUMBER() OVER (PARTITION BY ps.OwnerUserId ORDER BY ps.ViewCount DESC) AS UserPostRank FROM RecursivePostStats ps)
// SELECT u.DisplayName AS OwnerName, p.Title, p.CreationDate, p.UpVoteCount, p.DownVoteCount, p.ViewCount, p.CloseReason,
//        CASE WHEN p.RecentEditOrder >= 1 THEN 'Edited Recently' ELSE 'Not Edited Recently' END AS RecentEditStatus, CASE WHEN p.UserPostRank = 1 THEN 'Top Post' ELSE 'Other Post' END AS PostRank
// FROM PostWithDetails p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CloseReason IS NOT NULL OR (p.UpVoteCount - p.DownVoteCount) > 10
// ORDER BY p.ViewCount DESC, p.CreationDate DESC OFFSET 10 ROWS FETCH NEXT 20 ROWS ONLY;
//
// The WHERE on ph makes the history join inner, so the rows are one per post x close event; UserPostRank numbers those rows (ties go to the smaller history id).
fn q22254(db: &'static So) -> String {
    let Post { owner_user, view_count, creation_date, post_type_id, .. } = &db.post;
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let rows_ = drain(db.post_history.with(post_history_type_id.eq(10)).select(post.and(post.select(owner_user))));
    let ranked_ = per_group(ranked(rows_, |&(h, (p, u))| {
        let w = view_count.get(p);
        (u, w.is_none(), Reverse(w), p, h)
    }, false), |&(_, (_, u))| u);
    type R = ((Id<PostHistory>, (Id<Post>, Id<User>)), i64);
    let reason = |h: Id<PostHistory>, p: Id<Post>| if post_type_id.get(p).unwrap() == 1 { comment.get(h) } else { None };
    let v = drain(rel(ranked_).select(Same::<R>::new().and(Same::<R>::new().map(|((_, (p, _)), _): R| p).select((&vs).opt()))).filt(move |(((h, (p, _)), _), a): (R, Option<[i64; 2]>)| {
        let a = a.unwrap_or([0; 2]);
        reason(h, p).is_some() || a[0] - a[1] > 10
    }));
    let v = top_n(v, |&(_, (((h, (p, _)), _), _))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(creation_date.get(p).unwrap()), p, h)
    }, 30);
    rows(v.into_iter().skip(10).map(|(_, (((h, (p, u)), r), a))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["views"]));
        f.push(ostr(reason(h, p)));
        f.extend([V::S("Edited Recently"), V::S(if r == 1 { "Top Post" } else { "Other Post" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        COALESCE(COUNT(a.Id), 0) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName, p.Score, p.ViewCount),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScores
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId IN (1, 2) GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopTags AS (SELECT t.TagName, COUNT(pt.Id) AS TagCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY t.TagName
//     ORDER BY TagCount DESC LIMIT 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, ur.Reputation AS OwnerReputation, rp.Score, rp.ViewCount, rp.CommentCount, rp.AnswerCount, tt.TagName AS TopTag
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN TopTags tt ON tt.TagCount = rp.ViewCount WHERE rp.PostRank = 1 ORDER BY ur.Reputation DESC, rp.Score DESC;
//
// PostRank reads only base columns, so each owner's newest question is picked first and the comment x answer product is driven for those alone.
fn q26431(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(answers_of(db).opt())).fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    let asked: MatSet<Id<User>> = db.post.with(post_type_id.is_in([1, 2])).select(owner_user).collect();
    let lt = tag_mentions(db);
    let tc = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| db.tag.tag_name.get(t).unwrap())).select(Same::<(Id<Post>, Id<Tag>)>::new()).fold(0i64, |n, _| n + 1);
    let tt = rel(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10));
    let by_count: HashIdx<i64, (Str, i64)> = (&tt).map(|(_, n)| n).inv().select(&tt).collect();
    let v = drain((&s).and(owner_user.select(Ident::<User>::new().with(&asked))).and(view_count.select(&by_count).opt()));
    rows(v.into_iter().map(|(p, ((a, u), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.push(user_col(db, u, "rep"));
        f.extend(post_fields(db, p, &["score", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), t.map_or(V::Null, |(t, _)| V::S(t))]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM Users),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(v.UpVoteCount, 0) AS UpVoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS UpVoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.Score, rp.CommentCount, rp.UpVoteCount, ut.Reputation AS OwnerReputation, ut.Rank AS OwnerRank
//     FROM RecentPosts rp JOIN UserReputation ut ON rp.OwnerDisplayName = ut.DisplayName),
// TopPosts AS (SELECT *, DENSE_RANK() OVER (ORDER BY Score DESC, CommentCount DESC) AS ScoreRank FROM PostDetails)
// SELECT tp.Title, tp.OwnerDisplayName, tp.OwnerReputation, tp.CreationDate, tp.Score, tp.CommentCount, tp.UpVoteCount, CASE WHEN tp.OwnerRank <= 10 THEN 'Top User' ELSE 'Normal User' END AS UserCategory
// FROM TopPosts tp WHERE tp.ScoreRank <= 10 ORDER BY tp.Score DESC, tp.CommentCount DESC;
//
// PostDetails joins on the display name, so a post pairs with every user of its owner's name.
fn q2985(db: &'static So) -> String {
    let ur = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let ur = rel(ur.into_iter().map(|((u, _), r)| (u, r)).collect());
    let by_name: HashIdx<Str, (Id<User>, i64)> = (&ur).map(|(u, _)| u).select(&db.user.display_name).inv().select(&ur).collect();
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64);
    let v = drain((&cc).and(&uc).and(owner_user.select(&db.user.display_name).select(&by_name)));
    let v = ranked(v, |&(p, ((c, _), _))| (Reverse(score.get(p).unwrap()), Reverse(c)), true);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, ((c, n), (u, r))), _)| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.push(user_col(db, u, "rep"));
        f.extend(post_fields(db, p, &["created", "score"]));
        f.extend([V::I(c), V::I(n), V::S(if r <= 10 { "Top User" } else { "Normal User" })]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(v.UpVotes, 0) AS TotalUpVotes, COALESCE(v.DownVotes, 0) AS TotalDownVotes, COUNT(c.Id) AS CommentCount,
//        COUNT(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 END) AS HasAcceptedAnswer, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v
//     ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.CreationDate, v.UpVotes, v.DownVotes, p.OwnerUserId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, MAX(ph.CreationDate) AS LastClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// FinalStats AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.TotalUpVotes, ps.TotalDownVotes, ps.CommentCount, ps.HasAcceptedAnswer, cp.CloseCount, cp.LastClosedDate
//     FROM PostStats ps LEFT JOIN ClosedPosts cp ON ps.PostId = cp.PostId)
// SELECT fs.PostId, fs.Title, fs.CreationDate, fs.TotalUpVotes, fs.TotalDownVotes, fs.CommentCount, fs.HasAcceptedAnswer, COALESCE(fs.CloseCount, 0) AS CloseCount,
//        COALESCE(fs.LastClosedDate, '1900-01-01') AS LastClosedDate
// FROM FinalStats fs WHERE fs.TotalUpVotes - fs.TotalDownVotes > 10 OR fs.CommentCount > 5 ORDER BY fs.CreationDate DESC OFFSET 20 ROWS FETCH NEXT 10 ROWS ONLY;
//
// HasAcceptedAnswer counts the post x comment rows, so it is the comment count (at least one row) when the post has an accepted answer.
fn q3262(db: &'static So) -> String {
    let Post { creation_date, accepted_answer_id, .. } = &db.post;
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ps = db.post.group_by(Ident::<Post>::new()).select(accepted_answer_id.opt().and(comments_of(db).opt())).fold([0i64; 2], |a, (x, c)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&ps).and((&vs).opt()).filt(|(a, v): ([i64; 2], Option<[i64; 2]>)| {
        let v = v.unwrap_or([0; 2]);
        v[0] - v[1] > 10 || a[0] > 5
    }).and((&cp).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 30);
    rows(v.into_iter().skip(20).map(|(p, ((a, v), c))| {
        let v = v.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(v[0]), V::I(v[1]), V::I(a[0]), V::I(a[1])]);
        f.extend(match c {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::T(date(1900, 1, 1))],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, p.Score, p.ViewCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.Reputation, u.Views, COUNT(b.Id) AS BadgeCount, SUM(v.BountyAmount) AS TotalBountySpent
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId AND v.VoteTypeId IN (2,3) GROUP BY u.Id, u.Reputation, u.Views),
// PostHistorySummary AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS IsClosed, MAX(CASE WHEN ph.PostHistoryTypeId IN (11, 13) THEN 1 ELSE 0 END) AS IsReopened,
//        COUNT(ph.Id) AS EditCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.LastActivityDate, rp.Score, rp.ViewCount, CASE WHEN us.UserId IS NULL THEN 'Unregistered' ELSE u.DisplayName END AS OwnerDisplayName,
//        us.Reputation, us.Views, us.BadgeCount, us.TotalBountySpent, CASE WHEN phs.IsClosed = 1 THEN 'Closed' WHEN phs.IsReopened = 1 THEN 'Reopened' ELSE 'Active' END AS PostStatus, phs.EditCount
// FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserStats us ON u.Id = us.UserId LEFT JOIN PostHistorySummary phs ON rp.PostId = phs.PostId
// WHERE rp.OwnerRank = 1 AND rp.Score > 0 ORDER BY rp.CreationDate DESC, us.Reputation DESC;
//
// OwnerRank partitions by the raw OwnerUserId, so the ownerless questions are one partition. It reads only base columns, so each owner's newest question is picked
// first and the badges x votes product is driven for those owners alone.
fn q20348(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, owner_user_id, score, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(date(2024, 10, 1), -1)))).select(owner_user_id.opt())), |&(_, u)| u, |&(p, _)| {
        (Reverse(creation_date.get(p).unwrap()), p)
    }, 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&rp).select(owner_user).collect();
    let bounty = votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select((&db.vote.bounty_amount).opt());
    let us = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt().and(bounty.opt())).fold([0i64; 3], |a, (b, v)| {
        let v = v.flatten();
        [a[0] + b.is_some() as i64, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0)]
    });
    let phs = (&rp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id)).fold([0i64; 3], |a, t| {
        [a[0].max((t == 10) as i64), a[1].max(matches!(t, 11 | 13) as i64), a[2] + 1]
    });
    let v = drain((&rp).with(score.gt(0)).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&us)).opt()).and((&phs).opt())));
    rows(v.into_iter().map(|(_, ((p, u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity", "score", "views"]);
        f.extend(match u {
            Some((u, a)) => vec![user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "uviews"), V::I(a[0]), nullable(a[2], a[1])],
            None => vec![V::S("Unregistered"), V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::S(match h {
            Some(h) if h[0] == 1 => "Closed",
            Some(h) if h[1] == 1 => "Reopened",
            _ => "Active",
        }));
        f.push(h.map_or(V::Null, |h| V::I(h[2])));
        row(f)
    }))
}

// WITH RecursiveUserVotes AS (SELECT v.PostId, v.UserId, ROW_NUMBER() OVER (PARTITION BY v.PostId ORDER BY v.CreationDate DESC) AS VoteRank FROM Votes v WHERE v.VoteTypeId IN (2, 3)),
// PostScoreHistory AS (SELECT p.Id AS PostId, p.Title, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS HistoryCount, MAX(ph.CreationDate) AS LastHistoryDate
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title, p.Score),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(b.Class, 0)) AS BadgeScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 50 GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// RankedPosts AS (SELECT psh.PostId, psh.Title, psh.Score, psh.UpVotes, psh.DownVotes, psh.CommentCount, psh.LastHistoryDate, RANK() OVER (ORDER BY psh.Score DESC, psh.CommentCount DESC) AS PostRank
//     FROM PostScoreHistory psh)
// SELECT up.UserId, up.DisplayName, up.Reputation, up.CreationDate, rps.PostId, rps.Title, rps.Score, rps.UpVotes, rps.DownVotes, rps.CommentCount, rps.PostRank
// FROM ActiveUsers up JOIN RankedPosts rps ON up.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rps.PostId) WHERE rps.PostRank <= 100 ORDER BY up.Reputation DESC, rps.Score DESC;
//
// PostRank reads the score and the distinct comment count, so the top posts are picked first and the votes x comments x history product is driven for those alone.
fn q34082(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain(&cc), |&(p, c)| (Reverse(score.get(p).unwrap()), Reverse(c)), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 100).map(|((p, c), r)| (p, (c, r))).collect());
    let tp: MatSet<Id<Post>> = (&tr).map(|(p, _)| p).collect();
    let psh = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    type T = (Id<Post>, (i64, i64));
    let active = Ident::<User>::new().with((&db.user.reputation).gt(50));
    let v = drain((&tr).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select((&psh).and(owner_user.select(active))))));
    rows(v.into_iter().map(|(_, ((p, (c, r)), (a, u)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated"]);
        f.extend(post_fields(db, p, &["id", "title", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT B.Id) AS BadgeCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, QuestionCount, AnswerCount, UpVotes, DownVotes, TotalViews, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats),
// ActiveTags AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName
//     HAVING COUNT(DISTINCT P.Id) > 0),
// TopTags AS (SELECT TagName, PostCount, TotalViews, RANK() OVER (ORDER BY TotalViews DESC) AS Rank FROM ActiveTags)
// SELECT TU.Rank AS UserRank, TU.DisplayName AS User, TU.Reputation, TU.BadgeCount, TU.QuestionCount, TU.AnswerCount, TU.UpVotes, TU.DownVotes, TU.TotalViews, TT.Rank AS TagRank,
//        TT.TagName AS ActiveTag, TT.PostCount AS TagPostCount, TT.TotalViews AS TagTotalViews
// FROM TopUsers TU CROSS JOIN TopTags TT WHERE TU.Rank <= 10 AND TT.Rank <= 10 ORDER BY TU.Rank, TT.Rank;
//
// Rank reads only Reputation, so the top users are picked first and the badges x posts x votes product is driven for those alone.
fn q27092(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let ur = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(ur.iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ur = rel(ur.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 5], |a, (_, p)| match p {
            Some(((t, w), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + w.unwrap_or(0)],
            None => a,
        });
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let at = db.tag.group_by(Ident::<Tag>::new()).select((&by_tag).map(|(p, _)| p).select(view_count.opt())).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let tt = ranked(drain(&at), |&(_, a)| (a[1] == 0, Reverse(a[2])), false);
    let tt = rel(tt.into_iter().take_while(|x| x.1 <= 10).collect());
    type U = (Id<User>, i64);
    let urs = rel(drain((&ur).select(Same::<U>::new().and(Same::<U>::new().map(|(u, _): U| u).select((&us).and(&bc))))).into_iter().map(|x| x.1).collect());
    let mut v = Vec::new();
    (&urs).cross(&tt).drive(|_, (((u, r), (a, b)), ((t, s), k))| v.push((u, r, a, b, t, s, k)));
    rows(v.into_iter().map(|(u, r, a, b, t, s, k)| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)];
        f.extend(a.map(V::I));
        f.extend([V::I(k), V::S(db.tag.tag_name.get(t).unwrap()), V::I(s[0]), nullable(s[2], s[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3)
//     WHERE p.PostTypeId = 1 AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName)
// SELECT rp.*, rp.Score + rp.CommentCount * 2 + rp.VoteCount * 5 AS PostEngagementScore,
//        CASE WHEN rp.RankScore <= 10 THEN 'Top Posts' WHEN rp.RankScore <= 30 THEN 'Trending Posts' ELSE 'New Posts' END AS PostCategory, r.RecentEngagementScore
// FROM RankedPosts rp LEFT JOIN (SELECT Id, Score + CommentCount * 2 + VoteCount * 5 AS RecentEngagementScore FROM RecentPosts) r ON rp.Id = r.Id
// WHERE rp.Score > 0 ORDER BY PostEngagementScore DESC, rp.CreationDate DESC LIMIT 50;
//
// RankScore partitions by the post itself, so it is always 1.
fn q9734(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let updown = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let agg = |a: [i64; 2], x: (Option<Id<Comment>>, Option<Id<Vote>>)| [a[0] + x.0.is_some() as i64, a[1]];
    let qs = || db.post.with(post_type_id.eq(1));
    let cc = qs().with(score.gt(0)).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(updown().opt())).fold([0i64; 2], agg);
    let vc = qs().with(score.gt(0)).group_by(Ident::<Post>::new()).select(updown().opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let recent = || qs().with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let rc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(updown().opt())).fold([0i64; 2], agg);
    let rv = recent().group_by(Ident::<Post>::new()).select(updown().opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let pes = |p: Id<Post>, c: i64, n: i64| score.get(p).unwrap() + c * 2 + n * 5;
    let v = drain((&cc).and(&vc).and((&rc).and(&rv).opt()));
    let v = top_n(v, |&(p, ((c, n), _))| (Reverse(pes(p, c[0], n)), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((c, n), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c[0]), V::I(n), V::I(1), V::I(pes(p, c[0], n)), V::S("Top Posts"), r.map_or(V::Null, |(c, n)| V::I(pes(p, c[0], n)))]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId IN (4,5) THEN 1 ELSE 0 END) AS TotalWikis, AVG(p.Score) AS AverageScore,
//        RANK() OVER (ORDER BY COUNT(p.Id) DESC) AS PostRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// BadgedUsers AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// UserSummary AS (SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalWikis, ups.AverageScore, bu.BadgeCount, bu.HighestBadgeClass
//     FROM UserPostStats ups LEFT JOIN BadgedUsers bu ON ups.UserId = bu.UserId)
// SELECT us.DisplayName, us.TotalPosts, us.TotalQuestions, us.TotalAnswers, us.TotalWikis, COALESCE(us.BadgeCount, 0) AS BadgeCount,
//        CASE WHEN us.BadgeCount > 0 THEN CASE WHEN us.HighestBadgeClass = 1 THEN 'Gold' WHEN us.HighestBadgeClass = 2 THEN 'Silver' ELSE 'Bronze' END ELSE 'None' END AS HighestBadge, us.AverageScore,
//        (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = us.UserId AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') AS PostsLastYear
// FROM UserSummary us WHERE us.TotalPosts > 10 ORDER BY us.TotalPosts DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
fn q4212(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let ups = db.post.group_by(&db.post.owner_user).select(post_type_id.and(score).and(creation_date)).fold([0i64; 6], |a, ((t, s), d)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64, a[4] + s, a[5] + (d > add_years(ts(2024, 10, 1, 12, 34, 56), -1)) as i64]
    });
    let bu = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold((0i64, i64::MIN), |(n, m), c| (n + 1, m.max(c)));
    let v = top_n(drain((&ups).filt(|a| a[0] > 10).and((&bu).opt())), |&(u, (a, _))| (Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let (n, m) = b.unwrap_or((0, 0));
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(n)];
        f.push(V::S(if n > 0 { if m == 1 { "Gold" } else if m == 2 { "Silver" } else { "Bronze" } } else { "None" }));
        f.extend([avg(a[4], a[0]), V::I(a[5])]);
        row(f)
    }))
}

// WITH RecursiveTagCounts AS (SELECT Tags.TagName, COUNT(Posts.Id) AS PostCount FROM Tags LEFT JOIN Posts ON Tags.Id = Posts.Id GROUP BY Tags.TagName),
// UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostHistoryDetails AS (SELECT PH.PostId, PH.UserId, PH.CreationDate, P.Title, P.AcceptedAnswerId, PH.Comment, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS RevisionNumber
//     FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.PostHistoryTypeId IN (10, 11, 12)),
// MostActiveUsers AS (SELECT U.DisplayName, COUNT(P.Id) AS TotalPosts FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.DisplayName HAVING COUNT(P.Id) > 10),
// LastVotePerPost AS (SELECT V.PostId, V.UserId, V.CreationDate, ROW_NUMBER() OVER (PARTITION BY V.PostId ORDER BY V.CreationDate DESC) AS rn FROM Votes V)
// SELECT U.DisplayName AS UserName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(DISTINCT PH.RevisionNumber) AS TotalRevisions, COALESCE(RTC.PostCount, 0) AS TotalPostsWithTags, R.UPVotes, R.DownVotes,
//        MAX(LV.CreationDate) AS LastVoteDate
// FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostHistoryDetails PH ON P.Id = PH.PostId LEFT JOIN RecursiveTagCounts RTC ON RTC.TagName = SUBSTRING(P.Tags, 2, LENGTH(P.Tags) - 2)
// LEFT JOIN UserReputation R ON U.Id = R.UserId LEFT JOIN LastVotePerPost LV ON P.Id = LV.PostId AND LV.rn = 1
// WHERE U.Reputation > 1000 GROUP BY U.DisplayName, R.UPVotes, R.DownVotes, RTC.PostCount HAVING COUNT(DISTINCT P.Id) > 5 ORDER BY TotalPosts DESC, UserName ASC;
//
// `Tags.Id = Posts.Id` joins a tag id to a post id, through the raw ids. The groups are (name, votes, RTC count), not users. LastVotePerPost's rn = 1 row
// carries its post's latest vote date, which is all MAX reads. A post with k close/reopen/delete events has revision numbers 1..k whichever way ties fall.
fn q34680(db: &'static So) -> String {
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let rtc: Fold<Str, i64> = db.tag.group_by(&db.tag.tag_name).select((&db.tag.origid).select(&pidx).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let r = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = drain(db.post_history.with(post_history_type_id.is_in([10, 11, 12])).select(post));
    let ph = per_group(ranked(ph, |&(h, p)| (p, Reverse(hd.get(h).unwrap()), h), false), |&(_, p)| p);
    let ph = rel(ph.into_iter().map(|((_, p), n)| (p, n)).collect());
    let revs: HashIdx<Id<Post>, (Id<Post>, i64)> = (&ph).map(|(p, _)| p).inv().select(&ph).collect();
    let lv = db.vote.group_by(&db.vote.post).select(&db.vote.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let Post { owner_user, tags_str, .. } = &db.post;
    let up: MatSet<(Id<User>, Id<Post>)> = db.post.select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))).and(Ident::<Post>::new())).map(|x| x).collect();
    type UP = (Id<User>, Id<Post>);
    let key = Same::<UP>::new().map(|(u, _): UP| u).select((&db.user.display_name).and(&r)).and(Same::<UP>::new().map(|(_, p): UP| p).select(tags_str.map(|t: Str| &t[1..t.len() - 1]).select(&rtc).opt()));
    let vals = Same::<UP>::new().map(|(_, p): UP| p).select(Ident::<Post>::new().and((&revs).map(|(_, n)| n).opt()).and((&lv).opt()));
    let g = (&up).group_by(key).select(vals).buf_fold(|rows| {
        let posts = distinct_some(rows.iter().map(|r| Some(r.0 .0)));
        let mut ns: Vec<i64> = rows.iter().filter_map(|r| r.0 .1).collect();
        ns.sort_unstable();
        ns.dedup();
        let last = rows.iter().filter_map(|r| r.1).max();
        (posts, if ns.is_empty() { None } else { Some(ns.iter().sum::<i64>()) }, last)
    });
    let v = drain((&g).filt(|(n, _, _)| n > 5));
    rows(v.into_iter().map(|(((name, a), t), (n, s, d))| row(vec![V::S(name), V::I(n), oint(s), V::I(t.unwrap_or(0)), V::I(a[0]), V::I(a[1]), ots(d)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COALESCE(SUBSTRING(p.Body, 1, 100), '[No Content]') AS ShortBody, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '6 months'),
// PopularPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.ViewCount, rp.ShortBody, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostEngagements AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        SUM(CASE WHEN v.VoteTypeId = 6 THEN 1 ELSE 0 END) AS CloseVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT pp.PostId, pp.Title, pp.Score, pp.CreationDate, pp.ViewCount, pp.ShortBody, pp.CommentCount, COALESCE(pe.Upvotes, 0) AS Upvotes, COALESCE(pe.Downvotes, 0) AS Downvotes,
//        COALESCE(pe.CloseVotes, 0) AS CloseVotes, CASE WHEN COALESCE(pe.Upvotes, 0) > COALESCE(pe.Downvotes, 0) THEN 'Positive' WHEN COALESCE(pe.Upvotes, 0) < COALESCE(pe.Downvotes, 0) THEN 'Negative'
//        ELSE 'Neutral' END AS EngagementStatus, COALESCE(pht.Name, 'No History') AS PostHistoryType
// FROM PopularPosts pp LEFT JOIN PostEngagements pe ON pp.PostId = pe.PostId LEFT JOIN PostHistory ph ON pp.PostId = ph.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// WHERE pp.ViewCount > 100 ORDER BY pp.Score DESC, pp.CreationDate DESC;
fn q32369(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, body, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_months(current_date(), -6))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let pp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pe = (&pp).with(view_count.gt(100)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(6)) as i64]
    });
    let cc = (&pp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&pe).and(&cc).and(history_of(db).select(htype_name(db)).opt()));
    rows(v.into_iter().map(|(p, ((a, c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "views"]);
        f.push(V::Owned(body.get(p).unwrap().chars().take(100).collect()));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        f.push(V::S(h.unwrap_or("No History")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS HistoryCount,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount),
// TagStatistics AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END, 0)) AS CloseReopenEvents
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, TotalViews, CloseReopenEvents, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagStatistics)
// SELECT rp.Title, rp.Body, rp.CreationDate, rp.ViewCount, rp.UpVotes, rp.DownVotes, rp.CommentCount, rp.HistoryCount, tt.TagName, tt.PostCount AS TagPostCount, tt.TotalViews AS TagTotalViews,
//        tt.CloseReopenEvents AS TagCloseReopenEvents
// FROM RankedPosts rp JOIN Posts p ON rp.PostId = p.Id JOIN TopTags tt ON p.Tags LIKE '%' || tt.TagName || '%' WHERE rp.Rank <= 10 ORDER BY rp.Rank;
fn q29665(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let top = top_n(drain(&rp), |&(p, a)| (Reverse(a[0]), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let by_post: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&lt).map(|(p, _)| p).inv().collect();
    let ts = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select(view_count.opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt())))
        .fold([0i64; 2], |a, (w, t)| [a[0] + w.unwrap_or(0), a[1] + matches!(t, Some(10 | 11)) as i64]);
    let pc = db.tag.group_by(Ident::<Tag>::new()).select((&by_tag).map(|(p, _)| p)).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(Ident::<Post>::new().and(&rp).and(&cc).and(&hc).and((&by_post).map(|(_, t)| t).select(Ident::<Tag>::new().and(&pc).and(&ts)))));
    rows(v.into_iter().map(|(_, ((((p, a), c), h), ((t, n), s)))| {
        let mut f = post_fields(db, p, &["title", "body", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(h), V::S(db.tag.tag_name.get(t).unwrap()), V::I(n), V::I(s[0]), V::I(s[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions, COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes - DownVotes AS NetVotes, TotalPosts, TotalQuestions, TotalAnswers, RANK() OVER (ORDER BY UpVotes DESC) AS VoteRank,
//        DENSE_RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserStats WHERE TotalPosts > 0),
// PostSummary AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, AVG(P.Score) AS AverageScore, MIN(P.CreationDate) AS FirstPostDate, MAX(P.CreationDate) AS LastPostDate FROM Posts P GROUP BY P.OwnerUserId)
// SELECT U.DisplayName, COALESCE(S.PostCount, 0) AS PostCount, COALESCE(S.AverageScore, 0) AS AverageScore, U.UpVotes, U.DownVotes, U.TotalQuestions, U.TotalAnswers, COALESCE(T.NetVotes, 0) AS NetVotes,
//        T.VoteRank, T.PostRank, COALESCE(COUNT(DISTINCT C.Id), 0) AS CommentCount
// FROM UserStats U LEFT JOIN PostSummary S ON U.UserId = S.OwnerUserId LEFT JOIN TopUsers T ON U.UserId = T.UserId LEFT JOIN Comments C ON U.UserId = C.UserId
// GROUP BY U.DisplayName, S.PostCount, S.AverageScore, U.UpVotes, U.DownVotes, U.TotalQuestions, U.TotalAnswers, T.NetVotes, T.VoteRank, T.PostRank
// HAVING (COALESCE(T.NetVotes, 0) >= 10 OR SUM(COALESCE(C.Score, 0)) >= 50) ORDER BY T.VoteRank, PostCount DESC LIMIT 100;
//
// The outer GROUP BY is on the display name and the aggregates, not the user id, so users that agree on all of them are one group; the average is keyed as its
// exact (count, sum) pair. Each comment belongs to one user, so a group's distinct comments are its users' comments.
fn q176(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((_, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s],
        None => a,
    });
    let base = drain((&us).and(&pc));
    let t = ranked(drain(rel(base).filt(|(_, (_, p))| p[0] > 0)).into_iter().map(|x| x.1).collect(), |&(_, (a, _))| Reverse(a[0]), false);
    let t = ranked(t, |&((_, (_, p)), _)| Reverse(p[0]), true);
    let t = rel(t.into_iter().map(|(((u, (a, _)), vr), pr)| (u, (a[0] - a[1], vr, pr))).collect());
    let tu: HashIdx<Id<User>, (Id<User>, (i64, i64, i64))> = (&t).map(|(u, _)| u).inv().select(&t).collect();
    type K = (Str, Option<(i64, i64)>, [i64; 4], Option<(i64, i64, i64)>);
    let rowsk = rel(drain((&us).and(&pc).and((&tu).map(|(_, x)| x).opt())).into_iter().map(|(u, ((a, p), x))| {
        (u, (db.user.display_name.get(u).unwrap(), if p[0] == 0 { None } else { Some((p[0], p[3])) }, [a[0], a[1], p[1], p[2]], x))
    }).collect());
    type R = (Id<User>, K);
    let g = (&rowsk)
        .group_by(Same::<R>::new().map(|(_, k): R| k))
        .select(Same::<R>::new().map(|(u, _): R| u).select(comments_by(db).select(&db.comment.score).opt()))
        .fold([0i64; 2], |a, s| [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0)]);
    let v = drain(rel(drain(&g)).filt(|((_, _, _, x), a): (K, [i64; 2])| x.map_or(0, |x| x.0) >= 10 || a[1] >= 50));
    let v = top_n(v.into_iter().map(|x| x.1).collect(), |&((n, s, a, x), _)| (x.map_or(i64::MAX, |x| x.1), Reverse(s.map_or(0, |s| s.0)), n, s, a, x), 100);
    rows(v.into_iter().map(|((n, s, a, x), c)| {
        let mut f = vec![V::S(n), V::I(s.map_or(0, |s| s.0)), s.map_or(V::F(0.0), |(k, t)| avg(t, k))];
        f.extend(a.map(V::I));
        f.extend(match x {
            Some((net, vr, pr)) => [V::I(net), V::I(vr), V::I(pr)],
            None => [V::I(0), V::Null, V::Null],
        });
        f.push(V::I(c[0]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVoteCounts AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        (SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END)) AS NetVotes FROM Votes v GROUP BY v.PostId),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes, COALESCE(pvc.NetVotes, 0) AS NetVotes, rp.Score, rp.OwnerUserId
//     FROM RankedPosts rp LEFT JOIN PostVoteCounts pvc ON rp.PostId = pvc.PostId WHERE rp.rn <= 5),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(ps.NetVotes) AS TotalVotes, COUNT(ps.PostId) AS PostCount FROM Users u JOIN PostStats ps ON u.Id = ps.OwnerUserId GROUP BY u.Id, u.DisplayName
//     HAVING SUM(ps.NetVotes) > 10)
// SELECT tu.UserId, tu.DisplayName, tu.TotalVotes, tu.PostCount, ps.Title, ps.UpVotes, ps.DownVotes, ps.NetVotes
// FROM TopUsers tu JOIN PostStats ps ON tu.UserId = ps.OwnerUserId WHERE ps.NetVotes = (SELECT MAX(NetVotes) FROM PostStats ps_inner WHERE ps_inner.OwnerUserId = ps.OwnerUserId)
// ORDER BY tu.TotalVotes DESC, ps.CreationDate DESC;
fn q22777(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let ps: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvc = (&ps).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let tu = (&ps).group_by(owner_user).select(&pvc).fold([0i64; 3], |a, v| [a[0] + v[0] - v[1], a[1] + 1, a[2].max(v[0] - v[1])]);
    let tu = db.user.with((&tu).filt(|a| a[0] > 10)).select(&tu);
    let v = drain((&pvc).and(owner_user.select(Ident::<User>::new().and(tu))).filt(|(v, (_, a)): ([i64; 2], (Id<User>, [i64; 3]))| v[0] - v[1] == a[2]));
    rows(v.into_iter().map(|(p, (a, (u, t)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(t[0]), V::I(t[1])]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount,
//        COUNT(V.Id) AS TotalVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostVoteCounts AS (SELECT P.Id AS PostId, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id),
// PostDetail AS (SELECT P.Id, P.Title, P.ViewCount, COALESCE(PAC.TotalVotes, 0) AS PostTotalVotes, COALESCE(PAC.UpVotes, 0) AS PostUpVotes, COALESCE(PAC.DownVotes, 0) AS PostDownVotes,
//        DENSE_RANK() OVER (PARTITION BY P.PostTypeId ORDER BY COALESCE(PAC.TotalVotes, 0) DESC) AS VoteRank FROM Posts P LEFT JOIN PostVoteCounts PAC ON P.Id = PAC.PostId)
// SELECT UVC.UserId, UVC.DisplayName, SUM(PD.PostTotalVotes) AS TotalVotesForUser, AVG(PD.PostUpVotes) AS AvgUpVotes, AVG(PD.PostDownVotes) AS AvgDownVotes,
//        (SELECT COUNT(DISTINCT B.Id) FROM Badges B WHERE B.UserId = UVC.UserId AND B.Class = 1) AS GoldBadges, (SELECT COUNT(DISTINCT B.Id) FROM Badges B WHERE B.UserId = UVC.UserId AND B.Class = 2) AS SilverBadges,
//        (SELECT COUNT(DISTINCT B.Id) FROM Badges B WHERE B.UserId = UVC.UserId AND B.Class = 3) AS BronzeBadges
// FROM UserVoteCounts UVC JOIN PostDetail PD ON UVC.UserId = PD.Id WHERE UVC.TotalVotes > 100 GROUP BY UVC.UserId, UVC.DisplayName HAVING SUM(PD.PostTotalVotes) > 200 ORDER BY TotalVotesForUser DESC;
//
// `UVC.UserId = PD.Id` joins a user id to a post id, so it goes through the raw ids.
fn q31238(db: &'static So) -> String {
    let uvc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let pvc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let g = db
        .user
        .with((&uvc).filt(|n| n > 100))
        .group_by(Ident::<User>::new())
        .select((&db.user.origid).select(&pidx).select(&pvc))
        .fold([0i64; 4], |a, v| [a[0] + v[0], a[1] + v[1], a[2] + v[2], a[3] + 1]);
    let bc = db.user.with(&g).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = drain((&g).filt(|a| a[0] > 200).and(&bc));
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), avg(a[1], a[3]), avg(a[2], a[3])]);
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.Score, p.AnswerCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.Reputation),
// PostStatistics AS (SELECT rp.Title, ur.UserId, ur.Reputation, ur.UpVotes, ur.DownVotes, rp.Score, rp.AnswerCount, COUNT(c.Id) AS CommentCount, MAX(ph.CreationDate) AS LastEditedDate
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN Comments c ON rp.Id = c.PostId LEFT JOIN PostHistory ph ON rp.Id = ph.PostId WHERE rp.UserPostRank <= 3
//     GROUP BY rp.Title, ur.UserId, ur.Reputation, ur.UpVotes, ur.DownVotes, rp.Score, rp.AnswerCount)
// SELECT ps.Title, ps.Reputation, ps.UpVotes - ps.DownVotes AS NetVotes, ps.Score, ps.AnswerCount, ps.CommentCount, COALESCE(NULLIF(ps.LastEditedDate, rp.CreationDate), rp.CreationDate) AS RecentActivity
// FROM PostStatistics ps JOIN RankedPosts rp ON ps.Title = rp.Title WHERE (ps.UpVotes - ps.DownVotes) > 10 ORDER BY ps.Score DESC LIMIT 100 OFFSET 0;
//
// UserPostRank reads only base columns, so each owner's three newest posts are picked first; the votes x badges product is driven for their owners alone.
// PostStatistics groups by (Title, user, Score, AnswerCount), so one user's posts that agree on all of them are one group.
fn q1693(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, answer_count, title, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)));
    let top = top_per(drain(rp().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).collect();
    let ur = (&owners).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (t, _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let key = title.opt().and(owner_user).and(score).and(answer_count.opt());
    let ps = (&tp)
        .with(owner_user.select(Ident::<User>::new().with(&owners)))
        .group_by(key)
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, m.max(d.unwrap_or(i64::MIN))));
    let by_title: HashIdx<Str, Id<Post>> = rp().select(title).inv().collect();
    type K = (((Option<Str>, Id<User>), i64), Option<i64>);
    let psv = rel(drain(&ps));
    let v = drain((&psv).select(Same::<(K, (i64, i64))>::new().and(Same::<(K, (i64, i64))>::new().flat_map(|((((t, _), _), _), _): (K, (i64, i64))| t).select(&by_title)).and(Same::<(K, (i64, i64))>::new().map(|((((_, u), _), _), _): (K, (i64, i64))| u).select(&ur))).filt(|(_, a): (((K, (i64, i64)), Id<Post>), [i64; 2])| a[0] - a[1] > 10));
    let v = top_n(v, |&(i, ((((((_, _), s), _), _), p), _))| (Reverse(s), i, p), 100);
    rows(v.into_iter().map(|(_, ((((((t, u), s), n), (c, m)), p), a))| {
        let cd = creation_date.get(p).unwrap();
        row(vec![ostr(t), user_col(db, u, "rep"), V::I(a[0] - a[1]), V::I(s), oint(n), V::I(c), V::T(if m == i64::MIN || m == cd { cd } else { m })])
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, CASE WHEN Reputation >= 1000 THEN 'High Rep' WHEN Reputation < 1000 AND Reputation >= 100 THEN 'Medium Rep' ELSE 'Low Rep' END AS ReputationCategory
//     FROM Users),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 END), 0) AS Downvotes,
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, RANK() OVER (ORDER BY P.CreationDate DESC) AS RecentRank
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.Score),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, P.Title AS ClosedTitle, CRT.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes CRT ON PH.Comment = CAST(CRT.Id AS VARCHAR)
//     JOIN Posts P ON PH.PostId = P.Id WHERE PH.PostHistoryTypeId = 10)
// SELECT UR.UserId, UR.Reputation, UR.ReputationCategory, PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.Upvotes, PS.Downvotes, PS.CommentCount, CP.ClosedTitle, CP.CloseReason,
//        DENSE_RANK() OVER (PARTITION BY UR.ReputationCategory ORDER BY PS.Score DESC) AS CategoryScoreRank, CASE WHEN CP.ClosedTitle IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus
// FROM UserReputation UR JOIN Posts P ON UR.UserId = P.OwnerUserId JOIN PostStats PS ON P.Id = PS.PostId LEFT JOIN ClosedPosts CP ON P.Id = CP.PostId
// WHERE UR.Reputation > 0 AND COALESCE(PS.Upvotes - PS.Downvotes, 0) > 5 AND PS.RecentRank <= 100 ORDER BY UR.Reputation DESC, PS.Score DESC;
//
// RecentRank reads only CreationDate, so the hundred newest posts are picked first and the votes x comments product is driven for those alone.
fn q21261(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, title, .. } = &db.post;
    let rr = ranked(drain(creation_date), |&(_, d)| Reverse(d), false);
    let tp: MatSet<Id<Post>> = rel(rr.into_iter().take_while(|x| x.1 <= 100).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let ps = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 3], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]
    });
    let reason: HashIdx<Str, Str> = (&db.close_reason_type.origid).map(|i: i64| &*Box::leak(i.to_string().into_boxed_str())).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).select(comment.select(&reason)));
    let v = drain((&ps).filt(|a| a[0] - a[1] > 5).and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(0)))).and(cp.opt()));
    let cat = |u: Id<User>| {
        let r = db.user.reputation.get(u).unwrap();
        if r >= 1000 { "High Rep" } else if r >= 100 { "Medium Rep" } else { "Low Rep" }
    };
    let v = per_group(ranked(v, |&(p, ((_, u), _))| (cat(u), Reverse(score.get(p).unwrap())), true), |&(_, ((_, u), _))| cat(u));
    rows(v.into_iter().map(|((p, ((a, u), c)), r)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::S(cat(u)));
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match c {
            Some(n) => [ostr(title.get(p)), V::S(n)],
            None => [V::Null, V::Null],
        });
        f.push(V::I(r));
        f.push(V::S(if c.is_some() && title.get(p).is_some() { "Closed" } else { "Active" }));
        row(f)
    }))
}

// WITH PostStats AS (SELECT Posts.Id AS PostId, Posts.Title, Users.DisplayName AS Owner, COUNT(DISTINCT Comments.Id) AS TotalComments, COUNT(DISTINCT Votes.Id) FILTER (WHERE Votes.VoteTypeId = 2) AS Upvotes,
//        COUNT(DISTINCT Votes.Id) FILTER (WHERE Votes.VoteTypeId = 3) AS Downvotes, MAX(Comments.CreationDate) AS LastCommentDate
//     FROM Posts LEFT JOIN Users ON Posts.OwnerUserId = Users.Id LEFT JOIN Comments ON Posts.Id = Comments.PostId LEFT JOIN Votes ON Posts.Id = Votes.PostId
//     WHERE Posts.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY Posts.Id, Posts.Title, Users.DisplayName),
// PostHistoryDetails AS (SELECT PH.PostId, PHT.Name AS ChangeType, COUNT(*) AS ChangeCount, ARRAY_AGG(DISTINCT PH.UserDisplayName) AS UsersInvolved
//     FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id WHERE PH.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' GROUP BY PH.PostId, PHT.Name),
// CombinedResults AS (SELECT PS.PostId, PS.Title, PS.Owner, PS.TotalComments, PS.Upvotes, PS.Downvotes, PS.LastCommentDate, COALESCE(PHD.ChangeCount, 0) AS TotalChanges,
//        COALESCE(PHD.UsersInvolved, ARRAY[]::text[]) AS UsersInvolved FROM PostStats PS LEFT JOIN PostHistoryDetails PHD ON PS.PostId = PHD.PostId)
// SELECT CR.*, (CR.Upvotes - CR.Downvotes) AS NetVotes, CASE WHEN CR.TotalComments > 10 THEN 'Hot Topic' WHEN CR.TotalComments BETWEEN 5 AND 10 THEN 'Moderate Interest' ELSE 'Low Interest' END AS InterestLevel
// FROM CombinedResults CR ORDER BY NetVotes DESC, LastCommentDate DESC;
//
// Each post's distinct counts are its own comments and votes, so they need no product. ARRAY_AGG has no ORDER BY; the port sorts the names (NULL first), which
// cannot be observed here: no array in the answer has more than one element.
fn q3445(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let cs = recent().group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    let vs = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post, creation_date: hd, user_display_name, .. } = &db.post_history;
    let phd = db
        .post_history
        .with(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))
        .group_by(post.and(htype_name(db)))
        .select(user_display_name.opt())
        .buf_fold(|names| {
            let mut d: Vec<Option<Str>> = names.iter().copied().collect();
            d.sort();
            d.dedup();
            (names.len() as i64, &*Box::leak(d.into_boxed_slice()))
        });
    let pv = rel(drain(&phd));
    type H = ((Id<Post>, Str), (i64, &'static [Option<Str>]));
    let by_post: HashIdx<Id<Post>, H> = (&pv).map(|((p, _), _): H| p).inv().select(&pv).collect();
    let v = drain((&cs).and(&vs).and((&by_post).opt()));
    rows(v.into_iter().map(|(p, (((n, m), a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), tmax(m)]);
        f.extend(match h {
            Some((_, (k, us))) => [V::I(k), V::L(us.iter().map(|&u| ostr(u)).collect())],
            None => [V::I(0), V::L(vec![])],
        });
        f.push(V::I(a[0] - a[1]));
        f.push(V::S(if n > 10 { "Hot Topic" } else if n >= 5 { "Moderate Interest" } else { "Low Interest" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPosts FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserProfiles AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(u.Reputation, 0) as Reputation, COALESCE(b.BadgeCount, 0) as BadgeCount FROM Users u
//     LEFT JOIN (SELECT UserId, COUNT(*) as BadgeCount FROM Badges GROUP BY UserId) b ON u.Id = b.UserId),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostMetrics AS (SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, up.DisplayName, up.Reputation, up.BadgeCount, COALESCE(pc.CommentCount, 0) AS CommentCount, rp.Score, rp.TotalPosts
//     FROM RankedPosts rp JOIN UserProfiles up ON rp.OwnerUserId = up.UserId LEFT JOIN PostComments pc ON rp.Id = pc.PostId)
// SELECT pm.PostId, pm.Title, pm.CreationDate, pm.DisplayName, pm.Reputation, pm.BadgeCount, pm.CommentCount, pm.Score, pm.TotalPosts,
//        CASE WHEN pm.Score >= 50 THEN 'High Score' WHEN pm.Score BETWEEN 20 AND 49 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory,
//        CASE WHEN pm.Reputation >= 1000 THEN 'Influencer' ELSE 'Newbie' END AS UserCategory
// FROM PostMetrics pm WHERE pm.CommentCount > 5 ORDER BY pm.Score DESC, pm.CreationDate ASC;
fn q328(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let tp = recent().group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&cc).filt(|n| n > 5).and(owner_user.select(Ident::<User>::new().and(&tp).and((&bc).opt()))));
    rows(v.into_iter().map(|(p, (c, ((u, t), b)))| {
        let s = score.get(p).unwrap();
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b.unwrap_or(0)), V::I(c), V::I(s), V::I(t)]);
        f.push(V::S(if s >= 50 { "High Score" } else if s >= 20 { "Medium Score" } else { "Low Score" }));
        f.push(V::S(if r >= 1000 { "Influencer" } else { "Newbie" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        MAX(P.CreationDate) AS LastPostDate, DENSE_RANK() OVER (ORDER BY COUNT(P.Id) DESC) AS UserRank FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, LastPostDate FROM UserActivity WHERE UserRank <= 10),
// RecentComments AS (SELECT C.UserId, COUNT(C.Id) AS CommentCount, MAX(C.CreationDate) AS LastCommentDate FROM Comments C WHERE C.CreationDate >= DATE '2024-10-01' - INTERVAL '30 days' GROUP BY C.UserId),
// UserDetails AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(R.CommentCount, 0) AS RecentCommentCount, COALESCE(R.LastCommentDate, DATE '1970-01-01') AS LastCommentDate
//     FROM Users U LEFT JOIN RecentComments R ON U.Id = R.UserId),
// FinalResults AS (SELECT T.DisplayName, T.PostCount, T.QuestionCount, T.AnswerCount, T.LastPostDate, UD.Reputation, UD.RecentCommentCount,
//        CASE WHEN T.LastPostDate > UD.LastCommentDate THEN 'Active' WHEN T.LastPostDate < UD.LastCommentDate THEN 'Commenting More' ELSE 'Equally Active' END AS ActivityStatus
//     FROM TopUsers T JOIN UserDetails UD ON T.UserId = UD.UserId)
// SELECT *, CONCAT(DisplayName, ' has ', PostCount, ' posts (', QuestionCount, ' questions and ', AnswerCount, ' answers) | Status: ', ActivityStatus) AS Summary
// FROM FinalResults ORDER BY PostCount DESC, DisplayName;
fn q24853(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = ranked(drain(&ups), |&(_, a)| Reverse(a[1]), true);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let Comment { user, creation_date, .. } = &db.comment;
    let rc = db.comment.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).group_by(user).select(creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    type T = (Id<User>, [i64; 10]);
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select((&rc).opt()))));
    rows(v.into_iter().map(|(_, ((u, a), r))| {
        let (n, last) = r.unwrap_or((0, 0));
        let lp = a[7];
        let st = if a[1] == 0 { "Equally Active" } else if lp > last { "Active" } else if lp < last { "Commenting More" } else { "Equally Active" };
        let name = db.user.display_name.get(u).unwrap();
        let mut f = vec![V::S(name), V::I(a[1]), V::I(a[2]), V::I(a[3]), tmax(lp), user_col(db, u, "rep"), V::I(n), V::S(st)];
        f.push(V::Owned(format!("{name} has {} posts ({} questions and {} answers) | Status: {st}", a[1], a[2], a[3])));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 1 THEN P.Score ELSE 0 END) AS TotalScore, SUM(CASE WHEN P.PostTypeId = 1 THEN P.ViewCount ELSE 0 END) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalScore, TotalViews, RANK() OVER (ORDER BY TotalScore DESC, Reputation DESC) AS ScoreRank
//     FROM UserStats WHERE PostCount > 0),
// RecentPostActivity AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, COUNT(CI.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P LEFT JOIN Comments CI ON P.Id = CI.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY P.Id, P.OwnerUserId, P.Title, P.CreationDate)
// SELECT TU.DisplayName, TU.ScoreRank, TU.TotalScore, TU.QuestionCount, TU.AnswerCount, RPA.Title, RPA.CreationDate AS RecentPostDate, RPA.CommentCount, RPA.UpVotes, RPA.DownVotes
// FROM TopUsers TU JOIN RecentPostActivity RPA ON TU.UserId = RPA.OwnerUserId WHERE TU.ScoreRank <= 10 ORDER BY TU.ScoreRank, RPA.CreationDate DESC;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so CreationDate is compared as a New York wall-clock time.
fn q6083(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let us = db.user.with((&db.user.reputation).gt(0)).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score))).fold([0i64; 4], |a, (t, s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { s } else { 0 }]
    });
    let v = ranked(drain(&us), |&(u, a)| (Reverse(a[3]), Reverse(db.user.reputation.get(u).unwrap())), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| (u, (a, r))).collect());
    let top: HashIdx<Id<User>, (Id<User>, ([i64; 4], i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let since = now_utc() - 30 * DAY_US;
    let rpa = db
        .post
        .with(creation_date.filt(move |d| ny_to_utc(d) >= since))
        .with(owner_user.select(&top))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&rpa).and(owner_user.select(&top)));
    rows(v.into_iter().map(|(p, (c, (u, (a, r))))| {
        let mut f = vec![user_col(db, u, "name"), V::I(r), V::I(a[3]), V::I(a[1]), V::I(a[2])];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend(c.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStats AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title),
// MostActiveUsers AS (SELECT u.Id, u.DisplayName, COUNT(p.Id) AS PostsCount FROM Users u INNER JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName HAVING COUNT(p.Id) > 10),
// ClosedPostStats AS (SELECT ph.PostId, ph.CreationDate, pt.Name AS PostHistoryType FROM PostHistory ph INNER JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id WHERE ph.PostHistoryTypeId IN (10, 11)),
// FinalStats AS (SELECT ps.PostId, ps.Title, ps.CommentCount, ps.UpVotes, ps.DownVotes, COALESCE(cps.CreationDate, NULL) AS LastClosedDate, u.DisplayName AS ActiveUser
//     FROM PostStats ps LEFT JOIN ClosedPostStats cps ON ps.PostId = cps.PostId LEFT JOIN MostActiveUsers u ON ps.UpVotes > 0)
// SELECT fs.PostId, fs.Title, fs.CommentCount, fs.UpVotes, fs.DownVotes, fs.LastClosedDate, COALESCE(fs.ActiveUser, 'N/A') AS ActiveUserDetails,
//        CASE WHEN fs.UpVotes - fs.DownVotes > 0 THEN 'Positive' WHEN fs.UpVotes - fs.DownVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM FinalStats fs WHERE fs.CommentCount > 5 ORDER BY fs.UpVotes DESC, fs.DownVotes ASC;
//
// `LEFT JOIN MostActiveUsers u ON ps.UpVotes > 0` names only ps: a post with upvotes is crossed with every active user, any other keeps one NULL row.
fn q987(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let cps = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let fs = rel(drain((&ps).filt(|a| a[0] > 5).and(cps.select(&db.post_history.creation_date).opt())));
    let pc = db.post.group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let mau = rel(drain(db.user.with((&pc).filt(|n| n > 10)).select(&db.user.display_name)));
    let out = |p: Id<Post>, a: [i64; 3], d: Option<i64>, n: V| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), ots(d), n]);
        f.push(V::S(if a[1] - a[2] > 0 { "Positive" } else if a[1] - a[2] < 0 { "Negative" } else { "Neutral" }));
        row(f)
    };
    let mut v = Vec::new();
    (&fs).filt(|(_, (a, _)): (Id<Post>, ([i64; 3], Option<i64>))| a[1] > 0).cross(&mau).drive(|_, ((p, (a, d)), (_, n))| v.push(out(p, a, d, V::S(n))));
    (&fs).filt(|(_, (a, _)): (Id<Post>, ([i64; 3], Option<i64>))| a[1] <= 0).drive(|_, (p, (a, d))| v.push(out(p, a, d, V::S("N/A"))));
    rows(v)
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS UpVotedPosts, SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS DownVotedPosts
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// BadgeCounts AS (SELECT B.UserId, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId),
// UserPerformance AS (SELECT US.UserId, US.DisplayName, US.Reputation, US.PostCount, US.QuestionCount, US.AnswerCount, US.UpVotedPosts, US.DownVotedPosts, COALESCE(BC.TotalBadges, 0) AS TotalBadges,
//        COALESCE(BC.GoldBadges, 0) AS GoldBadges, COALESCE(BC.SilverBadges, 0) AS SilverBadges, COALESCE(BC.BronzeBadges, 0) AS BronzeBadges FROM UserStats US LEFT JOIN BadgeCounts BC ON US.UserId = BC.UserId),
// RankedPerformers AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserPerformance)
// SELECT RP.Rank, RP.DisplayName, RP.Reputation, RP.PostCount, RP.QuestionCount, RP.AnswerCount, RP.UpVotedPosts, RP.DownVotedPosts, RP.TotalBadges, RP.GoldBadges, RP.SilverBadges, RP.BronzeBadges
// FROM RankedPerformers RP WHERE RP.Rank <= 10 ORDER BY RP.Rank;
fn q5152(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + (s < 0) as i64],
        None => a,
    });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = ranked(drain((&us).and((&bc).opt())), |&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, b)), r)| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(a.map(V::I));
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserActivity),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, TotalUpVotes, TotalDownVotes FROM RankedUsers WHERE UserRank <= 10)
// SELECT tu.DisplayName, tu.Reputation, tu.QuestionCount, tu.AnswerCount, tu.TotalUpVotes, tu.TotalDownVotes,
//        CASE WHEN tu.TotalUpVotes > tu.TotalDownVotes THEN 'Positive Contributor' WHEN tu.TotalUpVotes < tu.TotalDownVotes THEN 'Negative Contributor' ELSE 'Neutral Contributor' END AS ContributorStatus,
//        (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = tu.UserId AND p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL) AS AcceptedQuestions
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId WHERE b.Class = 1
// GROUP BY tu.UserId, tu.DisplayName, tu.Reputation, tu.QuestionCount, tu.AnswerCount, tu.TotalUpVotes, tu.TotalDownVotes ORDER BY tu.Reputation DESC;
//
// UserRank reads only Reputation, so the top users are picked first and the posts x votes product is driven for those alone. The WHERE on b keeps the users with a
// gold badge, once each.
fn q207(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ua = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
        None => a,
    });
    let aq = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)).with(accepted_answer_id)).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let gold: MatSet<Id<User>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).collect();
    let v = drain(db.user.with(&gold).select((&ua).and(&aq)));
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[2] > a[3] { "Positive Contributor" } else if a[2] < a[3] { "Negative Contributor" } else { "Neutral Contributor" }));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
//        AVG(P.Score) AS AvgPostScore, SUM(COALESCE(CAST(P.ViewCount AS BIGINT), 0)) AS TotalViews FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, PositivePosts, NegativePosts, AvgPostScore, TotalViews, RANK() OVER (ORDER BY TotalViews DESC) AS Rank FROM UserPostStats WHERE PostCount > 0),
// PostHistoryDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, PH.UserId, PH.UserDisplayName, PH.CreationDate AS HistoryDate, PHT.Name AS HistoryType, COUNT(c.Id) AS CommentCount
//     FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id LEFT JOIN Comments c ON c.PostId = P.Id WHERE PH.UserId IS NOT NULL
//     GROUP BY P.Id, P.Title, P.CreationDate, PH.UserId, PH.UserDisplayName, PH.CreationDate, PHT.Name)
// SELECT TU.Rank, TU.DisplayName, TU.PostCount, TU.PositivePosts, TU.NegativePosts, TU.AvgPostScore, TU.TotalViews, PHD.PostId, PHD.Title, PHD.CreationDate, PHD.HistoryDate, PHD.HistoryType, PHD.CommentCount,
//        COALESCE(PHD.UserDisplayName, 'Anonymous') AS PostEditor
// FROM TopUsers TU LEFT JOIN PostHistoryDetails PHD ON TU.UserId = PHD.UserId WHERE TU.Rank <= 10 OR PHD.HistoryType IN ('Post Closed', 'Post Reopened') ORDER BY TU.Rank, PHD.HistoryDate DESC;
//
// PostHistoryDetails groups history rows that agree on (post, user, name, date, type), and its COUNT(c.Id) runs over those rows x the post's comments.
fn q30971(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt()))).fold([0i64; 5], |a, (s, w)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + s, a[4] + w.unwrap_or(0)]
    });
    let tu = ranked(drain(&ups), |&(_, a)| Reverse(a[4]), false);
    let tu = rel(tu);
    let PostHistory { post, user, user_display_name, creation_date: hd, .. } = &db.post_history;
    let key = post.and(user).and(user_display_name.opt()).and(hd).and(htype_name(db));
    let phd = db.post_history.with(user).group_by(key).select(post.select(comments_of(db).opt())).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = rel(drain(&phd));
    type K = ((((Id<Post>, Id<User>), Option<Str>), i64), Str);
    let by_user: HashIdx<Id<User>, (K, i64)> = (&pv).map(|(((((_, u), _), _), _), _): (K, i64)| u).inv().select(&pv).collect();
    type T = ((Id<User>, [i64; 5]), i64);
    let v = drain(
        (&tu)
            .select(Same::<T>::new().and(Same::<T>::new().map(|((u, _), _): T| u).select((&by_user).opt())))
            .filt(|((_, r), h): (T, Option<(K, i64)>)| r <= 10 || h.map_or(false, |(k, _)| matches!(k.1, "Post Closed" | "Post Reopened"))),
    );
    rows(v.into_iter().map(|(_, (((u, a), r), h))| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::I(a[4])];
        f.extend(match h {
            Some((((((p, _), n), d), t), c)) => {
                let mut g = post_fields(db, p, &["id", "title", "created"]);
                g.extend([V::T(d), V::S(t), V::I(c), V::S(n.unwrap_or("Anonymous"))]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::S("Anonymous")],
        });
        row(f)
    }))
}

// WITH UserAggregation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//        SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativeScorePosts FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     WHERE U.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentVotes AS (SELECT V.UserId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived, COUNT(V.Id) AS TotalVotes FROM Votes V
//     WHERE V.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY V.UserId),
// PostHistoryAggregation AS (SELECT PH.UserId, COUNT(PH.Id) AS HistoryCount, ARRAY_AGG(DISTINCT PH.PostId) AS RelatedPostIds, MAX(PH.CreationDate) AS LastActivity FROM PostHistory PH
//     WHERE PH.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY PH.UserId)
// SELECT UA.UserId, UA.DisplayName, UA.Reputation, UA.PostCount, UA.PositiveScorePosts, UA.NegativeScorePosts, COALESCE(RV.UpVotesReceived, 0) AS UpVotesReceived, COALESCE(RV.TotalVotes, 0) AS TotalVotes,
//        COALESCE(PH.HistoryCount, 0) AS PostHistoryCount, COALESCE(PH.LastActivity, '1970-01-01 00:00:00') AS LastPostHistoryActivity,
//        CASE WHEN UA.Reputation < 100 THEN 'New Contributor' WHEN UA.Reputation BETWEEN 100 AND 1000 THEN 'Active Contributor' ELSE 'Top Contributor' END AS ContributionLevel
// FROM UserAggregation UA LEFT OUTER JOIN RecentVotes RV ON UA.UserId = RV.UserId LEFT JOIN PostHistoryAggregation PH ON UA.UserId = PH.UserId WHERE UA.PostCount > 0
// ORDER BY UA.Reputation DESC, UA.DisplayName FETCH FIRST 10 ROWS ONLY;
//
// RelatedPostIds is never read.
fn q23966(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ua = db.user.with((&db.user.creation_date).lt(add_years(t0, -1))).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.score)).fold([0i64; 3], |a, s| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64]
    });
    let Vote { user, vote_type_id, creation_date, .. } = &db.vote;
    let rv = db.vote.with(creation_date.gt(add_months(t0, -6))).group_by(user).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + 1]);
    let PostHistory { user: hu, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.with(hd.gt(add_years(t0, -1))).group_by(hu).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&ua).and((&rv).opt()).and((&pha).opt()));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), db.user.display_name.get(u).unwrap(), u), 10);
    rows(v.into_iter().map(|(u, ((a, r), h))| {
        let rep = db.user.reputation.get(u).unwrap();
        let r = r.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(r[0]), V::I(r[1])]);
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::T(0)],
        });
        f.push(V::S(if rep < 100 { "New Contributor" } else if rep <= 1000 { "Active Contributor" } else { "Top Contributor" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges GROUP BY UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
//        COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(ps.CommentCount, 0) AS TotalComments, COALESCE(ps.TotalScore, 0) AS TotalScore, COALESCE(ps.QuestionCount, 0) AS TotalQuestions, COALESCE(ps.AnswerCount, 0) AS TotalAnswers
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, GoldBadges, SilverBadges, BronzeBadges, TotalComments, TotalScore, TotalQuestions, TotalAnswers,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC, TotalScore DESC) AS UserRank FROM UserReputation)
// SELECT UserId, DisplayName, Reputation, GoldBadges, SilverBadges, BronzeBadges, TotalComments, TotalScore, TotalQuestions, TotalAnswers FROM RankedUsers WHERE UserRank <= 10 ORDER BY UserRank;
fn q500(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(comments_of(db).opt())).fold([0i64; 4], |a, ((t, s), c)| {
        [a[0] + c.is_some() as i64, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64]
    });
    let ub = badge_classes(db);
    let v = top_n(drain(db.user.select((&ub).opt().and((&ps).opt()))), |&(u, (_, p))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(p.map_or(0, |p| p[1])), u), 10);
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend(p.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// Rewritten (rewrites/4705.sql): `, p.Id` added to the ActivityRank order, since an owner's posts can share a LastActivityDate.
// WITH UserVoteSummary AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// RecentPostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC, p.Id) AS ActivityRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CAST('2024-10-01' AS DATE) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.LastActivityDate, p.OwnerUserId),
// ClosedPosts AS (SELECT p.Id AS ClosedPostId, ph.UserDisplayName, ph.CreationDate AS CloseDate, ph.Comment AS CloseReason FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId = 10)
// SELECT uvs.DisplayName AS UserName, uvs.TotalVotes, uvs.UpVotes, uvs.DownVotes, rpa.PostId, rpa.Title AS RecentPostTitle, rpa.CreationDate AS PostCreationDate, rpa.LastActivityDate AS PostLastActivity,
//        rpa.UpVoteCount, rpa.DownVoteCount, cp.ClosedPostId, cp.CloseDate, cp.CloseReason
// FROM UserVoteSummary uvs JOIN RecentPostActivity rpa ON rpa.ActivityRank = 1 LEFT JOIN ClosedPosts cp ON rpa.PostId = cp.ClosedPostId
// WHERE rpa.UpVoteCount > 3 AND uvs.TotalVotes > 10 ORDER BY uvs.TotalVotes DESC, rpa.LastActivityDate DESC;
//
// The JOIN's ON names only rpa, so the voters and the rank-1 posts are crossed. ActivityRank partitions by the raw OwnerUserId, the ownerless posts as one partition.
fn q4705(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, last_activity_date, .. } = &db.post;
    let first = top_per(drain(db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).select(owner_user_id.opt())), |&(_, u)| u, |&(p, _)| {
        (Reverse(last_activity_date.get(p).unwrap()), p)
    }, 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rpa = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let rpa = rel(drain((&rpa).filt(|a| a[0] > 3).and(closes.opt())));
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let mut v = Vec::new();
    (&uvs).filt(|a| a[0] > 10).cross(&rpa).drive(|(u, _), (a, (p, (b, h)))| v.push((u, a, p, b, h)));
    rows(v.into_iter().map(|(u, a, p, b, h)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(post_fields(db, p, &["id", "title", "created", "activity"]));
        f.extend([V::I(b[0]), V::I(b[1])]);
        f.extend(match h {
            Some(h) => [V::I(db.post.origid.get(p).unwrap()), V::T(db.post_history.creation_date.get(h).unwrap()), ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// Rewritten (rewrites/5383.sql): `, p.Id` added to the Rank order, since posts tie on score and views at the fifth place.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months'),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// PostDetails AS (SELECT tp.*, COALESCE(c.CommentCount, 0) AS TotalComments, COALESCE(v.UpVoteCount, 0) AS TotalUpVotes FROM TopPosts tp
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON tp.PostId = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) FILTER (WHERE VoteTypeId = 2) AS UpVoteCount FROM Votes GROUP BY PostId) v ON tp.PostId = v.PostId)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.Score, pd.TotalComments, pd.TotalUpVotes, coalesce(h.ResponsesCount, 0) AS TotalResponses,
//        CASE WHEN pd.Score > 10 THEN 'Hot' WHEN pd.ViewCount > 1000 THEN 'Trending' ELSE 'Regular' END AS PostCategory
// FROM PostDetails pd LEFT JOIN (SELECT p.Id AS PostId, COUNT(*) AS ResponsesCount FROM Posts p WHERE p.PostTypeId = 2 AND p.ParentId IN (SELECT PostId FROM PostDetails) GROUP BY p.Id) h ON pd.PostId = h.PostId
// ORDER BY pd.Score DESC, pd.ViewCount DESC;
//
// h is grouped by the answer's own id, so it matches a top post that is an answer whose question is also a top post, with a count of 1.
fn q5383(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, parent, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).with(owner_user).select(post_type_id)), |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64);
    let h = (&tp).with(post_type_id.eq(2)).with(parent.select(Ident::<Post>::new().with(&tp))).group_by(Ident::<Post>::new()).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&cc).and(&uc).and((&h).opt()));
    rows(v.into_iter().map(|(p, ((c, u), r))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(u), V::I(r.unwrap_or(0))]);
        f.push(V::S(if s > 10 { "Hot" } else if view_count.get(p).map_or(false, |w| w > 1000) { "Trending" } else { "Regular" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.AnswerCount, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PopularPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, ur.Reputation, ur.UpVotes, ur.DownVotes, RANK() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS post_rank, rp.ViewCount
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId)
// SELECT pp.Title, pp.CreationDate, pp.Score, pp.Reputation, pp.UpVotes, pp.DownVotes,
//        CASE WHEN pp.Score IS NULL OR pp.Score < 0 THEN 'Score is negative or NULL' WHEN pp.UpVotes > pp.DownVotes THEN 'More Upvotes than Downvotes' ELSE 'Mixed votes' END AS VoteSummary,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = pp.Id) AS CommentCount
// FROM PopularPosts pp WHERE pp.post_rank <= 10 ORDER BY pp.Score DESC, pp.ViewCount DESC;
//
// post_rank reads only base columns, so the top questions are picked first and the posts x votes x badges product is driven for their owners alone.
fn q3959(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ur = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&ur))));
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([user_col(db, u, "rep"), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if a[0] > a[1] { "More Upvotes than Downvotes" } else { "Mixed votes" }));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH RecursiveUserBadges AS (SELECT U.Id AS UserId, U.DisplayName, B.Class, B.Name AS BadgeName, DENSE_RANK() OVER (PARTITION BY U.Id ORDER BY B.Date DESC) AS BadgeRank
//     FROM Users U JOIN Badges B ON U.Id = B.UserId WHERE B.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopUsers AS (SELECT UserId, DisplayName, COUNT(*) AS TotalBadges, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM RecursiveUserBadges GROUP BY UserId, DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(P.Score) AS TotalScore,
//        SUM(COALESCE(P.ViewCount, 0)) AS TotalViews FROM Posts P GROUP BY P.OwnerUserId),
// CombinedStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(TU.TotalBadges, 0) AS TotalBadges, COALESCE(TU.GoldBadges, 0) AS GoldBadges, COALESCE(TU.SilverBadges, 0) AS SilverBadges,
//        COALESCE(TU.BronzeBadges, 0) AS BronzeBadges, COALESCE(PS.QuestionCount, 0) AS QuestionCount, COALESCE(PS.AnswerCount, 0) AS AnswerCount, COALESCE(PS.TotalScore, 0) AS TotalScore,
//        COALESCE(PS.TotalViews, 0) AS TotalViews FROM Users U LEFT JOIN TopUsers TU ON U.Id = TU.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId)
// SELECT UserId, DisplayName, TotalBadges, GoldBadges, SilverBadges, BronzeBadges, QuestionCount, AnswerCount, TotalScore, TotalViews FROM CombinedStats
// WHERE TotalBadges > 0 OR QuestionCount > 0 ORDER BY TotalScore DESC, TotalBadges DESC LIMIT 10;
fn q32156(db: &'static So) -> String {
    let tu = db.badge.with((&db.badge.date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| {
        [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]
    });
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 4], |a, ((t, s), w)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.unwrap_or(0)]
    });
    let v = drain(db.user.select((&tu).opt().and((&ps).opt())).filt(|(b, p): (Option<[i64; 4]>, Option<[i64; 4]>)| b.map_or(0, |b| b[0]) > 0 || p.map_or(0, |p| p[0]) > 0));
    let v = top_n(v, |&(u, (b, p))| (Reverse(p.map_or(0, |p| p[2])), Reverse(b.map_or(0, |b| b[0])), u), 10);
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.extend(p.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, pgnt.Rank, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RowNum,
//        CASE WHEN p.Score > 0 THEN 'Positive' WHEN p.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreCategory, COALESCE(NULLIF(SUBSTRING(p.Body, 1, 100), ''), 'No Content') AS ShortBody
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentsCount, SUM(Score) AS CommentsScore FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT Id, ROW_NUMBER() OVER (ORDER BY CreationDate DESC) AS Rank FROM Posts) pgnt ON p.Id = pgnt.Id
//     WHERE p.ViewCount > (SELECT AVG(ViewCount) FROM Posts) AND (p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year'))
// SELECT rp.PostId, rp.Title, rp.ShortBody, rp.Score, rp.ScoreCategory, rp.CreationDate, COALESCE(c.CommentsCount, 0) AS TotalComments,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 2) AS UpVotesCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 3) AS DownVotesCount,
//        (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = rp.PostId AND ph.PostHistoryTypeId = 10) AS CloseVotesCount, CASE WHEN rp.RowNum = 1 THEN 'Most Recent Post' ELSE NULL END AS PostFlag
// FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentsCount FROM Comments GROUP BY PostId) c ON rp.PostId = c.PostId
// WHERE EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId IN (1, 2)) ORDER BY rp.Rank, rp.Score DESC LIMIT 50;
//
// `ViewCount > AVG(ViewCount)` is compared exactly, as `ViewCount * n > sum`; the global Rank orders by CreationDate descending, so the port sorts on that.
fn q21669(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, body, .. } = &db.post;
    let (s, n) = view_count.fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let rp = || db.post.with(view_count.filt(move |w| w * n > s)).with(creation_date.gt(add_years(date(2024, 10, 1), -1)));
    let first = top_per(drain(rp().select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let voted: MatSet<Id<Post>> = db.vote.with((&db.vote.vote_type_id).is_in([1, 2])).select(&db.vote.post).collect();
    let cs = rp().with(&voted).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vs = rp().with(&voted).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cv = rp().with(&voted).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10))).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&cs).and(&vs).and(&cv).and(Ident::<Post>::new().with(&first).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (((c, a), k), fl))| {
        let sc = score.get(p).unwrap();
        let short: String = body.get(p).unwrap().chars().take(100).collect();
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(if short.is_empty() { V::S("No Content") } else { V::Owned(short) });
        f.extend([V::I(sc), V::S(if sc > 0 { "Positive" } else if sc < 0 { "Negative" } else { "Neutral" })]);
        f.extend(post_fields(db, p, &["created"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(k), if fl.is_some() { V::S("Most Recent Post") } else { V::Null }]);
        row(f)
    }))
}

// WITH RecentPostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, P.Score, P.AnswerCount, ROW_NUMBER() OVER (PARTITION BY U.Location ORDER BY P.CreationDate DESC) AS RecentRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// AnswerStatistics AS (SELECT P.Id AS QuestionId, COUNT(A.Id) AS TotalAnswers, AVG(A.Score) AS AvgAnswerScore FROM Posts P LEFT JOIN Posts A ON P.Id = A.ParentId WHERE P.PostTypeId = 1 GROUP BY P.Id),
// HighScoreQuestions AS (SELECT R.PostId, R.Title, R.OwnerDisplayName, R.CreationDate, R.Score, A.TotalAnswers, A.AvgAnswerScore FROM RecentPostStats R JOIN AnswerStatistics A ON R.PostId = A.QuestionId
//     WHERE R.Score > (SELECT AVG(Score) FROM Posts WHERE PostTypeId = 1)),
// ClosedPosts AS (SELECT P.Id AS ClosedPostId, P.Title, PH.CreationDate AS ClosedDate, PH.Comment AS CloseReason FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId WHERE PH.PostHistoryTypeId = 10)
// SELECT Q.Title AS QuestionTitle, Q.OwnerDisplayName, Q.CreationDate AS QuestionDate, Q.Score AS QuestionScore, Q.TotalAnswers, Q.AvgAnswerScore, C.ClosedPostId, C.ClosedDate,
//        COALESCE(C.CloseReason, 'Not Closed') AS CloseReason
// FROM HighScoreQuestions Q LEFT JOIN ClosedPosts C ON Q.PostId = C.ClosedPostId WHERE (C.ClosedPostId IS NULL OR C.ClosedDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '7 days')
// ORDER BY Q.Score DESC, Q.CreationDate ASC;
//
// `Score > AVG(Score)` is compared exactly, as `Score * n > sum`.
fn q31536(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let (s, n) = db.post.with(post_type_id.eq(1)).select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let hq = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(t0, -30)))).with(owner_user).with(score.filt(move |x| x * n > s));
    let asg = hq.group_by(Ident::<Post>::new()).select(children_of(db).select(score).opt()).fold([0i64; 2], |a, x| match x {
        Some(x) => [a[0] + 1, a[1] + x],
        None => a,
    });
    let hd = &db.post_history.creation_date;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&asg).and(closes.opt()).filt(move |(_, h): ([i64; 2], Option<Id<PostHistory>>)| h.map_or(true, |h| hd.get(h).unwrap() >= add_days(t0, -7))));
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score"]);
        f.extend([V::I(a[0]), avg(a[1], a[0])]);
        f.extend(match h {
            Some(h) => [V::I(db.post.origid.get(p).unwrap()), V::T(hd.get(h).unwrap()), V::S(db.post_history.comment.get(h).unwrap_or("Not Closed"))],
            None => [V::Null, V::Null, V::S("Not Closed")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PopularUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS post_count, COALESCE(SUM(v.BountyAmount), 0) AS total_bounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS comment_count FROM Comments c WHERE c.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' GROUP BY c.PostId),
// PostStats AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, COALESCE(rc.comment_count, 0) AS comment_count, u.DisplayName AS owner_display_name, rp.OwnerUserId
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN RecentComments rc ON rp.Id = rc.PostId WHERE rp.rn = 1)
// SELECT ps.Id, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.AnswerCount, ps.comment_count, pu.DisplayName AS popular_user_name, pu.Reputation AS popular_user_reputation, pu.post_count, pu.total_bounty
// FROM PostStats ps LEFT JOIN PopularUsers pu ON ps.OwnerUserId = pu.Id ORDER BY ps.Score DESC, ps.ViewCount DESC LIMIT 10;
//
// The order reads only base columns, so the ten posts are picked first and the posts x bounty-votes product is driven for their owners alone.
fn q1382(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let first = top_per(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let v = top_n(first, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let pu = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let dp = user_distinct_posts(db);
    let rc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).ge(add_months(t0, -1)))).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&rc).and(owner_user.select(Ident::<User>::new().and(&pu).and(&dp)).opt()));
    rows(v.into_iter().map(|(p, (c, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(c));
        f.extend(match u {
            Some(((u, b), n)) => vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(n), V::I(b)],
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CreationDate, OwnerDisplayName FROM RankedPosts WHERE rn <= 5),
// PostVoteCounts AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId),
// PostStatistics AS (SELECT t.PostId, t.Title, t.Score, t.ViewCount, t.CreationDate, t.OwnerDisplayName, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes,
//        CASE WHEN COALESCE(v.UpVotes, 0) + COALESCE(v.DownVotes, 0) = 0 THEN NULL ELSE (COALESCE(v.UpVotes, 0) * 100.0) / (COALESCE(v.UpVotes, 0) + COALESCE(v.DownVotes, 0)) END AS UpVotePercentage
//     FROM TopPosts t LEFT JOIN PostVoteCounts v ON t.PostId = v.PostId)
// SELECT ps.OwnerDisplayName, ps.Title, ps.CreationDate, ps.UpVotePercentage, ps.ViewCount, CASE WHEN ps.Score > 10 THEN 'High' WHEN ps.Score BETWEEN 1 AND 10 THEN 'Medium' ELSE 'Low' END AS ScoreTier,
//        (SELECT COUNT(*) FROM PostHistory pH WHERE pH.PostId = ps.PostId AND pH.CreationDate > CURRENT_DATE - INTERVAL '6 MONTH') AS RecentEdits
// FROM PostStatistics ps ORDER BY ps.UpVotePercentage DESC NULLS LAST, ps.ViewCount DESC;
fn q2136(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let since = add_months(current_date(), -6);
    let re = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).gt(since))).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&vs).and(&re));
    rows(v.into_iter().map(|(p, (a, e))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["owner", "title", "created"]);
        f.push(if a[0] + a[1] == 0 { V::Null } else { V::F(a[0] as f64 * 100.0 / (a[0] + a[1]) as f64) });
        f.extend(post_fields(db, p, &["views"]));
        f.extend([V::S(if s > 10 { "High" } else if s >= 1 { "Medium" } else { "Low" }), V::I(e)]);
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.Id, ph.PostId, ph.CreationDate, ph.UserId, ph.Comment, ph.PostHistoryTypeId, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC, ph.Id) AS rn
//     FROM PostHistory ph),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostsWithBadges AS (SELECT p.Id AS PostId, COUNT(b.Id) AS BadgeCount, MAX(p.CreationDate) AS LatestActivityDate FROM Posts p LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' GROUP BY p.Id),
// ClosedPosts AS (SELECT p.Id AS PostId, ph.Comment AS CloseReason, ph.CreationDate AS CloseDate, ph.UserDisplayName AS ClosedBy FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10),
// RankedPosts AS (SELECT p.Title, p.ViewCount, pw.BadgeCount, rp.UserId, ur.ReputationRank, cp.CloseReason, cp.CloseDate, cp.ClosedBy FROM Posts p JOIN PostsWithBadges pw ON p.Id = pw.PostId
//     JOIN RecursivePostHistory rp ON p.Id = rp.PostId AND rp.rn = 1 LEFT JOIN ClosedPosts cp ON p.Id = cp.PostId LEFT JOIN UserReputation ur ON rp.UserId = ur.UserId)
// SELECT Title, ViewCount, BadgeCount, ReputationRank, CloseReason, ClosedBy, CASE WHEN CloseDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus
// FROM RankedPosts WHERE BadgeCount > 0 OR CloseReason IS NOT NULL ORDER BY ViewCount DESC, ReputationRank ASC LIMIT 100;
//
// Rewritten (rewrites/30253.sql): `, ph.Id` added to the rn order, since several history rows of a post share its latest date.
fn q30253(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let ur = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let ur = rel(ur.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pw = recent().group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { post, creation_date: hd, user, .. } = &db.post_history;
    let last = top_per(drain(db.post_history.with(post.select(Ident::<Post>::new().with(&pw))).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let last = rel(last.into_iter().map(|(h, p)| (p, h)).collect());
    let lh: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&last).map(|(p, _)| p).inv().select(&last).collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let rr = (&lh).map(|(_, h)| h).select(user.select((&rank).map(|(_, r)| r)).opt());
    let v = drain((&pw).and(rr).and(closes.opt()).filt(|((b, _), c): ((i64, Option<i64>), Option<Id<PostHistory>>)| b > 0 || c.map_or(false, |c| db.post_history.comment.get(c).is_some())));
    let v = top_n(v, |&(p, ((_, r), c))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), r.map_or(i64::MAX, |r| r), p, c)
    }, 100);
    rows(v.into_iter().map(|(p, ((b, r), c))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([V::I(b), oint(r)]);
        f.extend(match c {
            Some(c) => [ostr(db.post_history.comment.get(c)), ostr(db.post_history.user_display_name.get(c)), V::S("Closed")],
            None => [V::Null, V::Null, V::S("Active")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// RecentPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges FROM RankedPosts rp LEFT JOIN UserBadges ub
//     ON rp.OwnerUserId = ub.UserId WHERE rp.Rank <= 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(rp.CommentCount, 0) AS TotalComments, COALESCE(rp.GoldBadges, 0) AS TotalGoldBadges,
//        COALESCE(rp.SilverBadges, 0) AS TotalSilverBadges, COALESCE(rp.BronzeBadges, 0) AS TotalBronzeBadges, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM RecentPosts rp LEFT JOIN Votes v ON rp.PostId = v.PostId GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.GoldBadges, rp.SilverBadges, rp.BronzeBadges
// ORDER BY rp.CreationDate DESC LIMIT 50;
//
// Rank partitions by the raw OwnerUserId (the ownerless posts as one partition) and reads only base columns, so each owner's ten newest posts are picked first.
fn q33873(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user_id.opt())), |&(_, u)| u, |&(p, _)| {
        (Reverse(creation_date.get(p).unwrap()), p)
    }, 10, false);
    let top = top_n(top, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let ub = badge_classes(db);
    let v = drain((&cc).and(&vs).and(owner_user.select(&ub).opt()));
    rows(v.into_iter().map(|(p, ((c, a), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(c));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostStats AS (SELECT p.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalClosed, SUM(CASE WHEN ph.PostHistoryTypeId = 24 THEN 1 ELSE 0 END) AS TotalEdited
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.TotalClosed, 0) AS TotalClosed, COALESCE(ps.TotalEdited, 0) AS TotalEdited
//     FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId),
// TopUsers AS (SELECT ur.UserId, ur.Reputation, ur.TotalPosts, ur.TotalClosed, ur.TotalEdited, RANK() OVER (ORDER BY ur.Reputation DESC) AS UserRank FROM UserReputation ur WHERE ur.Reputation > 1000),
// FinalOutput AS (SELECT tu.UserId, tu.Reputation, tu.TotalPosts, tu.TotalClosed, tu.TotalEdited, tp.PostId AS TopPostId, tp.Title AS TopPostTitle, tp.ViewCount AS TopPostViewCount
//     FROM TopUsers tu LEFT JOIN RankedPosts tp ON tu.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId LIMIT 1) WHERE tu.UserRank <= 10)
// SELECT UserId, Reputation, TotalPosts, TotalClosed, TotalEdited, TopPostId, TopPostTitle, TopPostViewCount FROM FinalOutput ORDER BY Reputation DESC;
//
// The subquery looks the post up by its id, so it is the post's owner: every recent post of a top user joins. TotalPosts counts post x history rows.
fn q1723(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let rep = &db.user.reputation;
    let v = ranked(drain(db.user.with(rep.gt(1000)).select(rep)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ps = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(history_of(db).select(&db.post_history.post_history_type_id).opt())).fold([0i64; 3], |a, t| {
        [a[0] + 1, a[1] + (t == Some(10)) as i64, a[2] + (t == Some(24)) as i64]
    });
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let v = drain((&tu).select((&ps).opt().and(recent.opt())));
    rows(v.into_iter().map(|(u, (a, p))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "views"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RECURSIVE UserEngagement AS (SELECT UserId, COUNT(DISTINCT PostId) AS PostCount, COUNT(DISTINCT CASE WHEN VoteTypeId = 2 THEN PostId END) AS Upvotes,
//        COUNT(DISTINCT CASE WHEN VoteTypeId = 3 THEN PostId END) AS Downvotes, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS Score
//     FROM Votes v JOIN Posts p ON v.PostId = p.Id GROUP BY UserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, ue.PostCount, ue.Upvotes, ue.Downvotes, ue.Score, RANK() OVER (ORDER BY ue.Score DESC) AS ScoreRank FROM Users u JOIN UserEngagement ue ON u.Id = ue.UserId
//     WHERE ue.Score > 0),
// ActivePosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT CASE WHEN p.Id IN (SELECT DISTINCT PostId FROM Votes WHERE VoteTypeId = 2) THEN v.UserId END) AS UpvotedByCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// PostHistorySummary AS (SELECT ph.PostId, ph.PostHistoryTypeId, MAX(ph.CreationDate) AS LastChange FROM PostHistory ph GROUP BY ph.PostId, ph.PostHistoryTypeId)
// SELECT tu.DisplayName AS TopUser, tu.Score, tu.PostCount, ap.Title AS ActivePostTitle, ap.CommentCount, ap.UpvotedByCount, ph.LastChange
// FROM TopUsers tu JOIN ActivePosts ap ON tu.UserId = ap.OwnerUserId LEFT JOIN PostHistorySummary ph ON ap.PostId = ph.PostId AND ph.PostHistoryTypeId IN (10, 11)
// WHERE tu.ScoreRank <= 10 ORDER BY tu.Score DESC, ap.CreationDate DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q34673(db: &'static So) -> String {
    let Vote { user, post, vote_type_id, .. } = &db.vote;
    let ue = db.vote.with(user).with(post).group_by(user).select(post.and(vote_type_id)).buf_fold(|rs| {
        let d = |t: Option<i64>| distinct_some(rs.iter().map(|&(p, v)| if t.map_or(true, |t| t == v) { Some(p) } else { None }));
        [d(None), d(Some(2)), d(Some(3)), rs.iter().map(|&(_, v)| (v == 2) as i64 - (v == 3) as i64).sum()]
    });
    let tr = ranked(drain(db.user.select((&ue).filt(|a| a[3] > 0))), |&(_, a)| Reverse(a[3]), false);
    let tu = rel(tr.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let Post { creation_date, owner_user, .. } = &db.post;
    let up = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2)));
    let ap = db
        .post
        .with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(up.select((&db.vote.user).opt()).opt()))
        .buf_fold(|rs| (rs.iter().filter(|r| r.0.is_some()).count() as i64, distinct_some(rs.iter().map(|r| r.1.flatten()))));
    let PostHistory { post: hp, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(hp.and(post_history_type_id)).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let pv = rel(drain(&phs));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&pv).map(|((p, _), _)| p).inv().select(&pv).collect();
    type T = (Id<User>, [i64; 4]);
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(posts_of(db).select(Ident::<Post>::new().and(&ap).and((&by_post).opt()))))));
    rows(v.into_iter().map(|(_, ((u, a), ((p, (c, n)), h)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[3]), V::I(a[0])];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(c), V::I(n), h.map_or(V::Null, |(_, d)| V::T(d))]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END), 0) AS AcceptedAnswersCount,
//        MAX(U.LastAccessDate) AS LastAccessed FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostAnalytics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, COALESCE(COUNT(C.ID), 0) AS CommentCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score),
// TopContributors AS (SELECT U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS Rank FROM UserStatistics U WHERE U.Reputation > 1000)
// SELECT UStats.UserId, UStats.DisplayName, UStats.Reputation, P.Title AS TopQuestionTitle, P.Score AS QuestionScore, P.ViewCount AS QuestionViews, P.CommentCount AS QuestionComments,
//        P.TotalBounty AS QuestionBounty, T.Rank
// FROM UserStatistics UStats INNER JOIN PostAnalytics P ON UStats.UserId = (SELECT OwnerUserId FROM Posts WHERE PostTypeId = 1 ORDER BY Score DESC LIMIT 1)
// LEFT JOIN TopContributors T ON UStats.DisplayName = T.DisplayName WHERE UStats.Reputation > 500 ORDER BY UStats.Reputation DESC, QuestionViews DESC LIMIT 10;
//
// The subquery is uncorrelated (the owner of the single top-scoring question, score 1572), so the ON names only UStats: that user is crossed with every question.
// The order reads only base columns, so the ten questions are picked before their comment x bounty product is driven.
fn q1185(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, view_count, .. } = &db.post;
    let best = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), p), 1);
    let bu: MatSet<Id<User>> = rel(best.into_iter().map(|x| x.0).collect()).select(owner_user).collect();
    let us: MatSet<Id<User>> = (&bu).with((&db.user.reputation).gt(500)).collect();
    let qs = top_n(drain(db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new())), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 10);
    let qs: MatSet<Id<Post>> = rel(qs.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let pa = (&qs).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let rep = &db.user.reputation;
    let tc = ranked(drain(db.user.with(rep.gt(1000)).select(rep)), |&(_, r)| Reverse(r), false);
    let tc = rel(tc.into_iter().map(|((u, _), r)| (u, r)).collect());
    let by_name: HashIdx<Str, (Id<User>, i64)> = (&tc).map(|(u, _)| u).select(&db.user.display_name).inv().select(&tc).collect();
    let ur = (&us).select(Ident::<User>::new().and((&db.user.display_name).select(&by_name).opt()));
    let mut v = Vec::new();
    ur.cross(&pa).drive(|(_, p), ((u, t), a)| v.push((u, t, p, a)));
    let v = top_n(v, |&(u, t, p, _)| {
        let w = view_count.get(p);
        (Reverse(rep.get(u).unwrap()), w.is_none(), Reverse(w), p, t.map(|x| x.0))
    }, 10);
    rows(v.into_iter().map(|(u, t, p, a)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), t.map_or(V::Null, |(_, r)| V::I(r))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) as rn
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND p.Score > 0),
// PostDetails AS (SELECT rp.Id, rp.Title, rp.CreationDate, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.Id AND v.VoteTypeId = 2), 0) AS TotalUpvotes,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.Id AND v.VoteTypeId = 3), 0) AS TotalDownvotes,
//        CASE WHEN rp.CommentCount > 10 THEN 'Highly Discussed' WHEN rp.CommentCount > 0 THEN 'Moderately Discussed' ELSE 'Less Discussed' END AS DiscussionLevel
//     FROM RankedPosts rp LEFT JOIN Votes v ON rp.Id = v.PostId GROUP BY rp.Id, rp.Title, rp.CreationDate, rp.CommentCount)
// SELECT pd.Title, pd.CreationDate, pd.TotalBounty, pd.TotalUpvotes, pd.TotalDownvotes, pd.DiscussionLevel, p.ConversionRate,
//        (CASE WHEN pd.TotalUpvotes > pd.TotalDownvotes THEN 'Positive' WHEN pd.TotalUpvotes < pd.TotalDownvotes THEN 'Negative' ELSE 'Neutral' END) AS Sentiment
// FROM PostDetails pd LEFT JOIN (SELECT PostId, (SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) * 1.0 / NULLIF(COUNT(*), 0)) * 100 AS ConversionRate FROM Votes GROUP BY PostId) p ON pd.Id = p.PostId
// WHERE pd.DiscussionLevel = 'Highly Discussed' ORDER BY pd.TotalBounty DESC, pd.CreationDate DESC;
//
// rn is never read.
fn q3434(db: &'static So) -> String {
    let Post { creation_date, score, comment_count, .. } = &db.post;
    let pd = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)).and(score.gt(0)).and(comment_count.gt(10)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt())
        .fold([0i64; 4], |a, v| match v {
            Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + 1],
            None => a,
        });
    let v = drain(&pd);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S("Highly Discussed")]);
        f.push(if a[3] == 0 { V::Null } else { V::F(a[1] as f64 * 1.0 / a[3] as f64 * 100.0) });
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RECURSIVE UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY SUM(COALESCE(p.Score, 0)) DESC) AS Rank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostHistorySummary AS (SELECT ph.UserId, COUNT(ph.Id) AS TotalPostHistoryRecords, SUM(CASE WHEN pht.Name = 'Post Closed' THEN 1 ELSE 0 END) AS TotalPostClosed,
//        SUM(CASE WHEN pht.Name = 'Post Reopened' THEN 1 ELSE 0 END) AS TotalPostReopened FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.UserId),
// UserCombinedStats AS (SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalAcceptedAnswers, ups.TotalScore,
//        COALESCE(pSummary.TotalPostHistoryRecords, 0) AS TotalPostHistoryRecords, COALESCE(pSummary.TotalPostClosed, 0) AS TotalPostClosed, COALESCE(pSummary.TotalPostReopened, 0) AS TotalPostReopened
//     FROM UserPostStats ups LEFT JOIN PostHistorySummary pSummary ON ups.UserId = pSummary.UserId)
// SELECT ucs.DisplayName, ucs.TotalPosts, ucs.TotalQuestions, ucs.TotalAnswers, ucs.TotalAcceptedAnswers, ucs.TotalScore, ucs.TotalPostHistoryRecords, ucs.TotalPostClosed, ucs.TotalPostReopened
// FROM UserCombinedStats ucs WHERE ucs.TotalPosts > 10 ORDER BY ucs.TotalScore DESC LIMIT 10;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q32459(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, score, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, x), s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + x.is_some() as i64, a[4] + s],
        None => a,
    });
    let PostHistory { user, .. } = &db.post_history;
    let phs = db.post_history.group_by(user).select(htype_name(db)).fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "Post Closed") as i64, a[2] + (n == "Post Reopened") as i64]);
    let v = top_n(drain((&us).filt(|a| a[0] > 10).and((&phs).opt())), |&(u, (a, _))| (Reverse(a[4]), u), 10);
    rows(v.into_iter().map(|(u, (a, h))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(h.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVoteCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Rank, rp.UpVoteCount, rp.DownVoteCount, rp.TotalBounty FROM RankedPosts rp
//     WHERE rp.Rank <= 5 AND (rp.UpVoteCount - rp.DownVoteCount) > 10),
// PostDetails AS (SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = fp.PostId) AS CommentCount,
//        (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = fp.PostId AND ph.PostHistoryTypeId IN (10, 11, 12)) AS ClosureCount, fp.UpVoteCount, fp.DownVoteCount, fp.TotalBounty FROM FilteredPosts fp)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.CommentCount, pd.ClosureCount, pd.UpVoteCount, pd.DownVoteCount, pd.TotalBounty, COALESCE(pd.ClosureCount, 0) AS NonClosureCount
// FROM PostDetails pd LEFT JOIN Badges b ON pd.PostId = b.UserId WHERE pd.TotalBounty > 0 OR pd.ClosureCount IS NULL ORDER BY pd.Score DESC, pd.ViewCount DESC LIMIT 50 OFFSET 10;
//
// Rank reads only Score, so the top five posts per type are picked first (ties at the fifth place go to the smaller id; the answer is empty either way).
// `pd.PostId = b.UserId` joins a post id to a user id, through the raw ids. ClosureCount is a COUNT(*), never NULL.
fn q20576(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).fold([0i64; 3], |a, v| match v {
        Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + b.unwrap_or(0)],
        None => a,
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11, 12]))).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&rp).filt(|a| a[0] - a[1] > 10 && a[2] > 0).and(&cc).and(&hc).and((&db.post.origid).select(&uidx).select(badges_of(db)).opt()));
    let v = top_n(v, |&(p, (_, b))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, b)
    }, 60);
    rows(v.into_iter().skip(10).map(|(p, (((a, c), h), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(h), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(h)]);
        row(f)
    }))
}

// WITH RECURSIVE UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, b.Name AS BadgeName, b.Class, b.Date, RANK() OVER (PARTITION BY u.Id ORDER BY b.Date DESC) AS BadgeRank
//     FROM Users u JOIN Badges b ON u.Id = b.UserId WHERE b.Class <= 2),
// RecentPosts AS (SELECT p.OwnerUserId, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// ClosedPosts AS (SELECT ph.UserId, ph.PostId, ph.CreationDate, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId, ph.PostId, ph.CreationDate),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(rp.PostCount, 0) AS RecentPostCount, COALESCE(cp.CloseCount, 0) AS ClosedPostCount, COUNT(DISTINCT ub.BadgeName) AS BadgeCount
//     FROM Users u LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS PostCount FROM RecentPosts WHERE RecentPostRank = 1 GROUP BY OwnerUserId) rp ON u.Id = rp.OwnerUserId
//     LEFT JOIN ClosedPosts cp ON u.Id = cp.UserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId GROUP BY u.Id, u.DisplayName, rp.PostCount, cp.CloseCount)
// SELECT ue.UserId, ue.DisplayName, ue.RecentPostCount, ue.ClosedPostCount, ue.BadgeCount, RANK() OVER (ORDER BY ue.RecentPostCount DESC, ue.ClosedPostCount) AS EngagementRank
// FROM UserEngagement ue WHERE ue.RecentPostCount > 0 ORDER BY EngagementRank, ue.DisplayName;
//
// WITH RECURSIVE, but no CTE refers to itself. Exactly one RecentPosts row per owner has rank 1, so rp.PostCount is 1 for every user with a recent post.
// The groups are (user, CloseCount): a user with close events of several multiplicities is several rows.
fn q34354(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let first = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(first).map(|(_, u)| u).group_by(Same::<Id<User>>::new()).select(Same::<Id<User>>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { user, post, creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).with(user).group_by(user.and(post).and(hd)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cps: MatSet<(Id<User>, i64)> = (&cp).and(Same::<((Id<User>, Id<Post>), i64)>::new()).map(|(n, ((u, _), _))| (u, n)).collect();
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&cps).map(|(u, _)| u).inv().select(&cps).collect();
    let bn = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).le(2))).select(&db.badge.name).opt()).buf_fold(|ns| distinct_some(ns.iter().copied()));
    let v = drain((&rp).filt(|n| n > 0).and((&by_user).map(|(_, n)| n).opt()).and(&bn));
    let v = ranked(v, |&(_, ((r, c), _))| (Reverse(r), c.unwrap_or(0)), false);
    rows(v.into_iter().map(|((u, ((r, c), b)), k)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(r), V::I(c.unwrap_or(0)), V::I(b), V::I(k)])))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("25027", q25027),
    ("22118", q22118),
    ("34447", q34447),
    ("29394", q29394),
    ("332", q332),
    ("22784", q22784),
    ("452", q452),
    ("29322", q29322),
    ("284", q284),
    ("26420", q26420),
    ("30588", q30588),
    ("963", q963),
    ("25012", q25012),
    ("34867", q34867),
    ("34499", q34499),
    ("2364", q2364),
    ("2731", q2731),
    ("7239", q7239),
    ("33996", q33996),
    ("3975", q3975),
    ("84", q84),
    ("1886", q1886),
    ("21724", q21724),
    ("30252", q30252),
    ("30729", q30729),
    ("22144", q22144),
    ("21702", q21702),
    ("30712", q30712),
    ("31748", q31748),
    ("1506", q1506),
    ("25788", q25788),
    ("27529", q27529),
    ("2145", q2145),
    ("9634", q9634),
    ("1787", q1787),
    ("32451", q32451),
    ("32545", q32545),
    ("4592", q4592),
    ("720", q720),
    ("4611", q4611),
    ("4175", q4175),
    ("5789", q5789),
    ("9294", q9294),
    ("21407", q21407),
    ("30124", q30124),
    ("22537", q22537),
    ("31635", q31635),
    ("34426", q34426),
    ("3051", q3051),
    ("3338", q3338),
    ("7122", q7122),
    ("23578", q23578),
    ("502", q502),
    ("22254", q22254),
    ("26431", q26431),
    ("2985", q2985),
    ("3262", q3262),
    ("20348", q20348),
    ("34082", q34082),
    ("27092", q27092),
    ("9734", q9734),
    ("4212", q4212),
    ("34680", q34680),
    ("32369", q32369),
    ("29665", q29665),
    ("176", q176),
    ("22777", q22777),
    ("31238", q31238),
    ("1693", q1693),
    ("21261", q21261),
    ("3445", q3445),
    ("328", q328),
    ("24853", q24853),
    ("6083", q6083),
    ("987", q987),
    ("5152", q5152),
    ("207", q207),
    ("30971", q30971),
    ("23966", q23966),
    ("500", q500),
    ("4705", q4705),
    ("5383", q5383),
    ("3959", q3959),
    ("32156", q32156),
    ("21669", q21669),
    ("31536", q31536),
    ("1382", q1382),
    ("2136", q2136),
    ("30253", q30253),
    ("33873", q33873),
    ("1723", q1723),
    ("34673", q34673),
    ("1185", q1185),
    ("3434", q3434),
    ("32459", q32459),
    ("20576", q20576),
    ("34354", q34354),
];
