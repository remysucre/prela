use harness::prelude::*;
use std::cmp::Reverse;


type UP = (Id<User>, [i64; 10]);

/// One window over every row of a fold, the whole fold as the single partition.
fn win<D, S, K, O, C>(s: &Fold<D, S>, f: fn(&[(K, (D, S))], &mut Vec<i64>), order: O, cmp: C) -> Window<(), (D, S), i64>
where
    D: Copy + Eq + std::hash::Hash + 'static,
    S: Copy + Eq + std::hash::Hash,
    K: Copy,
    O: Fn((D, S)) -> K,
    C: Fn(&K, &K) -> std::cmp::Ordering,
{
    whole(s).select(Same::<D>::new().and(s)).window(f, order, cmp)
}

/// RANK() OVER (ORDER BY TotalScore DESC) over `user_posts`, NULL scores last, with rank <= 10.
fn score_top(db: &'static So) -> Vec<(UP, i64)> {
    let w = win(&user_posts(db), rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    top_n(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1).collect(), |&(_, r)| r, 0)
}

/// Users with RANK() by total score (NULLs last) and by views (or post count), either at most 10, by (score rank, other rank).
fn score_and(db: &'static So, second_views: bool) -> Vec<((UP, i64), i64)> {
    let w = win(&user_posts(db), rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    let w = (&w).window(rank, move |((_, a), _): (UP, i64)| if second_views { (a[5] == 0, Reverse(a[6])) } else { (false, Reverse(a[1])) }, asc);
    top_n(drain((&w).filt(|((_, s), w)| s <= 10 || w <= 10)).into_iter().map(|x| x.1).collect(), |&((_, s), w)| (s, w), 0)
}

fn tag_mentions_ci(db: &'static So) -> MatSet<(Id<Post>, Id<Tag>)> {
    let elems: MatSet<Str> = (&db.post.tags_str).flat_map(tag_list).collect();
    let contains: HashIdx<Str, Id<Tag>> = (&elems).select_where((&db.tag.tag_name).inv(), |e: Str, n: Str| e.to_lowercase().contains(&n.to_lowercase())).collect();
    db.post.select(Ident::<Post>::new().and((&db.post.tags_str).flat_map(tag_list).select(&contains))).collect()
}

// WITH RECURSIVE PostHierarchy AS (... questions, then their answers, recursively ...)
// SELECT u.DisplayName, u.Reputation, p.Title AS QuestionTitle, COUNT(c.Id) AS CommentCount, AVG(v.BountyAmount) AS AverageBounty,
//        MAX(CASE WHEN bh.Class = 1 THEN 1 ELSE 0 END) AS GoldBadge, MAX(CASE WHEN bh.Class = 2 THEN 1 ELSE 0 END) AS SilverBadge,
//        MAX(CASE WHEN bh.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadge, bh.Date AS BadgeDate, p.CreationDate AS CreatedAt
// FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) LEFT JOIN Badges bh ON u.Id = bh.UserId
// LEFT JOIN (SELECT DISTINCT PostId FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11)) PHFiltered ON p.Id = PHFiltered.PostId
// WHERE p.CreationDate BETWEEN '2022-01-01' AND '2023-10-01'
// GROUP BY u.DisplayName, u.Reputation, p.Title, bh.Date, p.CreationDate HAVING COUNT(c.Id) > 5 ORDER BY u.Reputation DESC, CommentCount DESC;
//
// The recursive CTE is never referenced, and PHFiltered has one row per post
// at most and nothing selected from it, so neither changes the answer. The
// WHERE on p makes the posts join an inner one.
fn q34957(db: &'static So) -> String {
    let Post { owner_user, title, creation_date, .. } = &db.post;
    let bounty: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).is_in([8, 9])).select(&db.vote.post).inv().collect();
    type R = (Id<Post>, Option<(i64, i64)>);
    let rows_ = owned(db)
        .with(creation_date.between(date(2022, 1, 1), date(2023, 10, 1)))
        .select(Ident::<Post>::new().and(owner_user.select(badges_of(db).select((&db.badge.date).and(&db.badge.class))).opt()));
    let post = || Same::<R>::new().map(|(p, _): R| p);
    let badge = || Same::<R>::new().map(|(_, b): R| b);
    let key = post().select(owner_user.select((&db.user.display_name).and(&db.user.reputation)).and(title.opt()).and(creation_date)).and(badge().map(|b: Option<(i64, i64)>| b.map(|x| x.0)));
    let per = post().select(comments_of(db).opt().and((&bounty).select((&db.vote.bounty_amount).opt()).opt())).and(badge().map(|b: Option<(i64, i64)>| b.map(|x| x.1)));
    let g = rows_.group_by(key).select(per).fold([0i64; 6], |a, ((c, v), cl)| {
        let v = v.flatten();
        [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3].max((cl == Some(1)) as i64), a[4].max((cl == Some(2)) as i64), a[5].max((cl == Some(3)) as i64)]
    });
    let mut out = Vec::new();
    (&g).filt(|a| a[0] > 5).drive(|((((n, r), t), c), d), a| {
        out.push(row(vec![V::S(n), V::I(r), ostr(t), V::I(a[0]), avg(a[2], a[1]), V::I(a[3]), V::I(a[4]), V::I(a[5]), ots(d), V::T(c)]))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount,
//            SUM(CASE WHEN v.CreationDate IS NOT NULL AND vt.Id = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.CreationDate IS NOT NULL AND vt.Id = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
//     GROUP BY u.Id, u.DisplayName),
// PostSummary AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(p.Score, 0) AS Score, p.AnswerCount, p.CommentCount, COALESCE(p.FavoriteCount, 0) AS FavoriteCount,
//                        COALESCE(p.ClosedDate, p.CreationDate) AS CloseDate, p.OwnerUserId FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 month')
// SELECT u.DisplayName, ups.PostCount, ups.CommentCount, ups.UpVoteCount, ups.DownVoteCount, ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.Score, ps.AnswerCount,
//        ps.CommentCount AS PostCommentCount, ps.FavoriteCount, ps.CloseDate
// FROM UserPostStats ups JOIN Users u ON ups.UserId = u.Id JOIN PostSummary ps ON u.Id = ps.OwnerUserId ORDER BY ups.PostCount DESC, ups.UpVoteCount DESC;
//
// CURRENT_DATE is the day the query runs; the data ends in 2024, so
// PostSummary is empty.
fn q11476(db: &'static So) -> String {
    let ps: HashIdx<Id<User>, Id<Post>> = owned(db).with((&db.post.creation_date).ge(add_months(current_date(), -1))).select(&db.post.owner_user).inv().collect();
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold((0i64, 0i64), |(u, d), (_, t)| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let dc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).count_distinct();
    let v = drain(db.user.select(Ident::<User>::new().and((&dp).opt()).and((&dc).opt()).and((&ups).opt())).and(&ps));
    let v = top_n(v, |&(_, ((((_, n), _), u), _))| (Reverse(n), Reverse(u.map_or(0, |u| u.0))), 0);
    rows(v.iter().map(|&(_, ((((u, n), c), x), p))| {
        let x = x.unwrap_or((0, 0));
        let mut f = vec![user_col(db, u, "name"), V::I(n.unwrap_or(0)), V::I(c.unwrap_or(0)), V::I(x.0), V::I(x.1)];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments"]));
        f.push(V::I(db.post.favorite_count.get(p).unwrap_or(0)));
        f.push(V::T(db.post.closed_date.get(p).unwrap_or(db.post.creation_date.get(p).unwrap())));
        row(f)
    }))
}

// WITH UserScores AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COALESCE(UP.NumPosts, 0) AS NumPosts, COALESCE(UP.NumAnswers, 0) AS NumAnswers,
//            COALESCE(UP.NumComments, 0) AS NumComments, COALESCE(B.NumBadges, 0) AS NumBadges
//     FROM Users U
//     LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS NumPosts, SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS NumAnswers, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS NumQuestions,
//                       SUM(CASE WHEN PostTypeId NOT IN (1,2) THEN 1 ELSE 0 END) AS NumComments FROM Posts GROUP BY OwnerUserId) UP ON U.Id = UP.OwnerUserId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS NumBadges FROM Badges GROUP BY UserId) B ON U.Id = B.UserId),
// UserActivity AS (SELECT UserId, COUNT(*) AS ActivityCount, MAX(CreationDate) AS LastActivityDate
//                  FROM (SELECT UserId, CreationDate FROM Votes UNION ALL SELECT UserId, CreationDate FROM Comments UNION ALL SELECT OwnerUserId AS UserId, CreationDate FROM Posts) AS Activities
//                  GROUP BY UserId)
// SELECT US.DisplayName, US.Reputation, US.NumPosts, US.NumAnswers, US.NumComments, US.NumBadges, UA.ActivityCount, UA.LastActivityDate
// FROM UserScores US JOIN UserActivity UA ON US.UserId = UA.UserId WHERE US.Reputation > 1000 ORDER BY US.Reputation DESC, UA.ActivityCount DESC LIMIT 100;
fn q7762(db: &'static So) -> String {
    let up = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + !matches!(t, 1 | 2) as i64]);
    let bc = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let acts = (&db.vote.user).inv().select(&db.vote.creation_date).union((&db.comment.user).inv().select(&db.comment.creation_date)).union((&db.post.owner_user).inv().select(&db.post.creation_date));
    let ua = acts.fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select(Ident::<User>::new().and((&up).opt()).and((&bc).opt()).and(&ua)));
    let v = top_n(v, |&(_, (((u, _), _), (n, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n)), 100);
    rows(v.iter().map(|&(_, (((u, p), b), (n, m)))| {
        let p = p.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(b.unwrap_or(0)), V::I(n), V::T(m)]);
        row(f)
    }))
}

// WITH RECURSIVE RecursiveCTE AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, P.PostTypeId, 1 AS Level FROM Posts P WHERE P.PostTypeId = 1
//                                 UNION ALL SELECT A.Id, A.Title, A.OwnerUserId, A.CreationDate, A.PostTypeId, R.Level + 1 FROM Posts A INNER JOIN RecursiveCTE R ON A.ParentId = R.PostId
//                                 WHERE R.PostTypeId = 1),
// VoteCounts AS (SELECT PostId, COUNT(*) FILTER (WHERE VoteTypeId = 2) AS Upvotes, COUNT(*) FILTER (WHERE VoteTypeId = 3) AS Downvotes FROM Votes GROUP BY PostId),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, ... FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostScore AS (SELECT P.Id AS PostId, (COALESCE(VC.Upvotes, 0) - COALESCE(VC.Downvotes, 0)) AS Score, P.CreationDate FROM Posts P LEFT JOIN VoteCounts VC ON P.Id = VC.PostId)
// SELECT R.Level, R.Title, U.DisplayName AS Owner, COALESCE(UB.BadgeCount, 0) AS TotalBadges, PS.Score, R.CreationDate
// FROM RecursiveCTE R JOIN Users U ON R.OwnerUserId = U.Id LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostScore PS ON R.PostId = PS.PostId
// WHERE R.Level = 1 AND PS.Score >= 0 ORDER BY PS.Score DESC, R.CreationDate DESC;
//
// The recursive step only produces rows with Level >= 2, so `WHERE R.Level
// = 1` keeps exactly the base case: the questions. That follows from the
// query, not the data, so no fixpoint is needed.
fn q33552(db: &'static So) -> String {
    let net = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |s, t| s + (t == 2) as i64 - (t == 3) as i64);
    let bc = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    owned(db)
        .with((&db.post.post_type_id).eq(1))
        .select(Ident::<Post>::new().and((&net).opt().map(|s| s.unwrap_or(0)).filt(|s| s >= 0)).and((&db.post.owner_user).select((&bc).opt())))
        .drive(|_, ((p, s), b)| {
            out.push(row(vec![V::I(1), title(db, p), post_fields(db, p, &["owner"]).remove(0), V::I(b.unwrap_or(0)), V::I(s), V::T(db.post.creation_date.get(p).unwrap())]))
        });
    rows(out)
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId = 2 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//            SUM(COALESCE(B.Class, 0)) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostActivity AS (SELECT PH.UserId, PH.PostId, PH.CreationDate, PH.PostHistoryTypeId, COUNT(*) AS ChangeCount FROM PostHistory PH
//                  WHERE PH.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') GROUP BY PH.UserId, PH.PostId, PH.CreationDate, PH.PostHistoryTypeId),
// Result AS (SELECT US.*, PA.ChangeCount FROM UserStats US LEFT JOIN PostActivity PA ON US.UserId = PA.UserId)
// SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, AcceptedAnswers, TotalBadges, SUM(ChangeCount) AS TotalPostChanges
// FROM Result GROUP BY UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, AcceptedAnswers, TotalBadges ORDER BY TotalPosts DESC, Reputation DESC LIMIT 50;
fn q9497(db: &'static So) -> String {
    let pos = || db.user.with((&db.user.reputation).gt(0));
    let us = pos()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&db.post.accepted_answer_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (p, c)| {
            let (t, x) = p.map_or((0, false), |(t, x)| (t, x.is_some()));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 2 && x) as i64, a[3] + c.unwrap_or(0)]
        });
    let dp = pos().group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let PostHistory { user, post, creation_date, post_history_type_id, .. } = &db.post_history;
    let pa = db
        .post_history
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(user.and(post).and(creation_date).and(post_history_type_id))
        .fold(0i64, |n, _| n + 1);
    type K = (((Id<User>, Id<Post>), i64), i64);
    let per_user = whole(&pa).select(Same::new().and(&pa)).group_by(Same::<(K, i64)>::new().map(|((((u, _), _), _), _)| u)).fold(0i64, |s, (_, n)| s + n);
    let v = drain((&us).and((&dp).opt()).and((&per_user).opt()));
    let v = top_n(v, |&(u, ((_, n), _))| (Reverse(n.unwrap_or(0)), Reverse(db.user.reputation.get(u).unwrap())), 50);
    rows(v.iter().map(|&(u, ((a, n), c))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), oint(c)]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT Id, Title, ParentId, 1 AS Level FROM Posts WHERE PostTypeId = 1
//                                  UNION ALL SELECT p.Id, p.Title, p.ParentId, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.Id),
// UserVotes AS (SELECT UserId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY UserId),
// PostAnalytics AS (SELECT p.Id, p.Title, ph.Level, p.CreationDate, u.DisplayName AS OwnerDisplayName, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes,
//                          (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
//                          (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = p.Id AND ph.PostHistoryTypeId = 12) AS DeleteCount
//                   FROM Posts p LEFT JOIN PostHierarchy ph ON p.Id = ph.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserVotes v ON p.OwnerUserId = v.UserId)
// SELECT pa.Title, pa.OwnerDisplayName, pa.CreationDate, pa.Level, pa.UpVotes, pa.DownVotes, pa.CommentCount, pa.DeleteCount, (pa.UpVotes - pa.DownVotes) AS VoteBalance,
//        CASE WHEN pa.DeleteCount > 0 THEN 'Deleted' ELSE 'Active' END AS PostStatus
// FROM PostAnalytics pa WHERE pa.Level = 1 AND pa.CommentCount > 0 ORDER BY pa.CreationDate DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// Level 1 rows come only from the base case, so `pa.Level = 1` keeps each
// question once and none of the recursive rows; no fixpoint is needed.
fn q30398(db: &'static So) -> String {
    let uv = db.vote.group_by(&db.vote.user_id).select(&db.vote.vote_type_id).fold((0i64, 0i64), |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64));
    let cp = comments_per_post(db);
    let dels = db.post_history.with((&db.post_history.post_history_type_id).eq(12)).select(&db.post_history.post).inv().dense_fold_outer(db.post.id.n, 0i64, |n, _| n + 1);
    let q = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .select(Ident::<Post>::new().and((&cp).filt(|c| c > 0)).and(&dels).and((&db.post.owner_user_id).select(&uv).opt()));
    let v = top_n(drain(q), |&(p, _)| Reverse(db.post.creation_date.get(p).unwrap()), 10);
    rows(v.iter().map(|&(_, (((p, c), d), x))| {
        let (u, dn) = x.unwrap_or((0, 0));
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        f.extend([V::I(1), V::I(u), V::I(dn), V::I(c), V::I(d), V::I(u - dn), V::S(if d > 0 { "Deleted" } else { "Active" })]);
        row(f)
    }))
}


// WITH RecentUserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(c.Id, 0)) AS CommentCount,
//            SUM(COALESCE(v.Id, 0)) AS VoteCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON c.UserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id
//     WHERE u.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, ... GoldBadges, SilverBadges, BronzeBadges FROM Badges b GROUP BY b.UserId),
// CombinedData AS (SELECT rua.*, COALESCE(ub.BadgeCount, 0) AS BadgeCount, ... FROM RecentUserActivity rua LEFT JOIN UserBadges ub ON rua.UserId = ub.UserId)
// SELECT * FROM CombinedData ORDER BY TotalViews DESC, PostCount DESC LIMIT 100;
//
// The comment and vote sums add their Ids, over the posts x comments x
// votes product, as written.
fn q8242(db: &'static So) -> String {
    let recent = || db.user.with((&db.user.creation_date).gt(add_years(date(2024, 10, 1), -1)));
    let rua = recent()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and((&db.post.accepted_answer_id).opt()).and(&db.post.creation_date)).opt().and(comments_by(db).select(&db.comment.origid).opt()).and(votes_by(db).select(&db.vote.origid).opt()))
        .fold([0, 0, 0, 0, i64::MIN], |a, ((p, c), v)| {
            let (w, x, d) = p.map_or((0, 0, i64::MIN), |((w, x), d)| (w.unwrap_or(0), x.is_some() as i64, d));
            [a[0] + w, a[1] + c.unwrap_or(0), a[2] + v.unwrap_or(0), a[3] + x, a[4].max(d)]
        });
    let dp = recent().group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = top_n(drain((&rua).and((&dp).opt()).and((&ub).opt())), |&(_, ((a, n), _))| (Reverse(a[0]), Reverse(n.unwrap_or(0))), 100);
    rows(v.iter().map(|&(u, ((a, n), b))| {
        let b = b.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), tmax(a[4]), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        row(f)
    }))
}

// WITH UserTags AS (
//     SELECT U.Id AS UserId, U.DisplayName, T.TagName, COUNT(P.Id) AS PostCount
//     FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId
//     JOIN (SELECT P.Id, unnest(string_to_array(substring(P.Tags, 2, length(P.Tags) - 2), '><')) AS TagName FROM Posts P WHERE P.PostTypeId = 1) AS T ON P.Id = T.Id
//     GROUP BY U.Id, U.DisplayName, T.TagName),
// TopTags AS (SELECT TagName, SUM(PostCount) AS TotalPosts FROM UserTags GROUP BY TagName ORDER BY TotalPosts DESC LIMIT 10),
// PostStatistics AS (
//     SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, T.TagName, U.DisplayName AS OwnerName,
//            (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount
//     FROM Posts P LEFT JOIN UserTags U ON P.OwnerUserId = U.UserId
//     JOIN (SELECT UserId, TagName FROM UserTags WHERE TagName IN (SELECT TagName FROM TopTags)) T ON P.OwnerUserId = T.UserId
//     WHERE P.PostTypeId = 1)
// SELECT PS.Title, PS.CreationDate, PS.ViewCount, PS.Score, PS.CommentCount, PS.TagName, COUNT(V.Id) AS VoteCount, AVG(U.Reputation) AS AvgReputation
// FROM PostStatistics PS LEFT JOIN Votes V ON PS.PostId = V.PostId LEFT JOIN Users U ON PS.OwnerName = U.DisplayName
// GROUP BY PS.Title, PS.CreationDate, PS.ViewCount, PS.Score, PS.CommentCount, PS.TagName ORDER BY VoteCount DESC, PS.Score DESC;
//
// For each question, PostStatistics has one row per (tag its owner has
// used, top tag its owner has used): the LEFT JOIN on UserTags does not
// narrow to the question's own tags. The final join on OwnerName takes
// every user with that display name.
fn q26657(db: &'static So) -> String {
    let Post { owner_user, tags_str, title, creation_date, view_count, score, .. } = &db.post;
    let q = || owned(db).with((&db.post.post_type_id).eq(1));
    let ut: MatSet<(Id<User>, Str)> = q().select(owner_user.and(tags_str.flat_map(tag_list))).collect();
    let tt = q().select(tags_str.flat_map(tag_list)).group_by(Same::new()).fold(0i64, |n, _| n + 1);
    let top: MatSet<Str> = rel(top_n(drain(&tt), |&(_, n)| Reverse(n), 10)).map(|(t, _)| t).collect();
    let by_user: HashIdx<Id<User>, (Id<User>, Str)> = (&ut).map(|(u, _)| u).inv().collect();
    let by_user_top: HashIdx<Id<User>, Str> = (&ut).with(Same::<(Id<User>, Str)>::new().map(|(_, t)| t).with(&top)).map(|(u, _)| u).inv().map(|(_, t)| t).collect();
    let same_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let cp = comments_per_post(db);
    type PT = (Id<Post>, Str);
    let post = || Same::<PT>::new().map(|(p, _): PT| p);
    let key = post().select(title.opt().and(creation_date).and(view_count.opt()).and(score).and(&cp)).and(Same::<PT>::new().map(|(_, t): PT| t));
    let product = post().select(owner_user.select(&by_user).and(votes_of(db).opt()).and(owner_user.select(&db.user.display_name).select((&same_name).select(&db.user.reputation)).opt()));
    let g = q()
        .select(Ident::<Post>::new().and(owner_user.select(&by_user_top)))
        .group_by(key)
        .select(product)
        .fold((0i64, 0i64, 0i64), |(v, n, r), ((_, x), y)| (v + x.is_some() as i64, n + y.is_some() as i64, r + y.unwrap_or(0)));
    let mut out = Vec::new();
    (&g).drive(|(((((t, d), w), s), c), tag), (v, n, r)| out.push(row(vec![ostr(t), V::T(d), oint(w), V::I(s), V::I(c), V::S(tag), V::I(v), avg(r, n)])));
    rows(out)
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadgeCount, ... FROM Badges GROUP BY UserId),
// PostSummary AS (SELECT P.Id AS PostId, P.Title, COALESCE(P.AcceptedAnswerId, 0) AS AcceptedAnswerId, P.CreationDate, P.Score,
//                        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount, ... UpVoteCount, DownVoteCount FROM Posts P),
// UserPostMetrics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, SUM(CASE WHEN PS.Score IS NOT NULL THEN PS.Score ELSE 0 END) AS TotalPostScore,
//                            COUNT(DISTINCT PS.PostId) AS TotalPosts, COUNT(DISTINCT CASE WHEN PS.AcceptedAnswerId IS NOT NULL THEN PS.PostId END) AS AcceptedAnswers
//                     FROM Users U LEFT JOIN PostSummary PS ON U.Id = PS.AcceptedAnswerId GROUP BY U.Id, U.DisplayName, U.Reputation)
// SELECT U.DisplayName, U.Reputation, COALESCE(UBC.GoldBadgeCount, 0) AS GoldBadges, ..., UPM.TotalPosts, UPM.AcceptedAnswers, UPM.TotalPostScore,
//        CASE WHEN U.LastAccessDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN 'Inactive' ELSE 'Active' END AS UserStatus
// FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN UserPostMetrics UPM ON U.Id = UPM.UserId ORDER BY U.Reputation DESC, U.DisplayName ASC LIMIT 100;
//
// UserPostMetrics joins a user Id against a post's (coalesced) accepted
// answer Id, as written.
fn q2232(db: &'static So) -> String {
    let ubc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let by_acc: HashIdx<i64, Id<Post>> = db.post.select((&db.post.accepted_answer_id).opt().map(|x: Option<i64>| x.unwrap_or(0))).inv().collect();
    let upm = db.user.group_by(Ident::<User>::new()).select((&db.user.origid).select((&by_acc).select(&db.post.score)).opt()).fold(0i64, |s, x| s + x.unwrap_or(0));
    let upd = db.user.group_by(Ident::<User>::new()).select((&db.user.origid).select(&by_acc)).count_distinct();
    let v = drain(db.user.select(Ident::<User>::new().and((&ubc).opt()).and(&upm).and((&upd).opt())));
    let v = top_n(v, |&(_, (((u, _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), db.user.display_name.get(u).unwrap()), 100);
    let old = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    rows(v.iter().map(|&(_, (((u, b), s), n))| {
        let b = b.unwrap_or([0; 3]);
        let n = n.unwrap_or(0);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(n), V::I(n), V::I(s), V::S(if db.user.last_access_date.get(u).unwrap() < old { "Inactive" } else { "Active" })]);
        row(f)
    }))
}

// WITH RECURSIVE RecursiveCTE AS (SELECT P.Id AS PostId, P.Title, P.Score, P.CreationDate, 0 AS Level, P.OwnerUserId FROM Posts P WHERE P.PostTypeId = 1
//                                 UNION ALL SELECT A.Id, A.Title, A.Score, A.CreationDate, R.Level + 1, A.OwnerUserId FROM Posts A INNER JOIN RecursiveCTE R ON A.ParentId = R.PostId
//                                 WHERE A.PostTypeId = 2),
// PostScores AS (SELECT P.Id, P.Title, P.OwnerUserId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//                       COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, P.Score AS CurrentScore,
//                       (COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) - COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END)) AS NetScore, R.Level
//                FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN RecursiveCTE R ON P.Id = R.PostId
//                GROUP BY P.Id, P.Title, P.OwnerUserId, P.Score, R.Level),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// FinalOutput AS (SELECT U.Id AS UserId, U.DisplayName, PS.Title, PS.CurrentScore, PS.NetScore, PS.CommentCount, UB.BadgeCount, PS.Level
//                 FROM PostScores PS JOIN Users U ON PS.OwnerUserId = U.Id JOIN UserBadges UB ON U.Id = UB.UserId)
// SELECT UserId, DisplayName, Title, CurrentScore, NetScore, CommentCount, BadgeCount, Level FROM FinalOutput WHERE Level = 0 ORDER BY NetScore DESC, CurrentScore DESC LIMIT 100;
//
// The recursive step only adds answers, at Level >= 1, so `Level = 0` keeps
// exactly the base case: each question once. No fixpoint is needed.
fn q32194(db: &'static So) -> String {
    let ps = owned(db)
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&ps).and((&db.post.owner_user).select(&bc)));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[0] - a[1]), Reverse(db.post.score.get(p).unwrap())), 100);
    rows(v.iter().map(|&(p, (a, b))| {
        let mut f = post_fields(db, p, &["uid", "owner", "title", "score"]);
        f.extend([V::I(a[0] - a[1]), V::I(a[2]), V::I(b), V::I(0)]);
        row(f)
    }))
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedQuestions,
//            SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE u.CreationDate < timestamp '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, ... FROM Badges b GROUP BY b.UserId),
// CombinedActivity AS (SELECT ua.*, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
//                      FROM UserActivity ua LEFT JOIN UserBadges ub ON ua.UserId = ub.UserId)
// SELECT ca.*, (TotalUpVotes - TotalDownVotes) AS NetVotes, (TotalQuestions * 2 + TotalAnswers) AS EngagementScore, (GoldBadges * 3 + SilverBadges * 2 + BronzeBadges) AS BadgesScore
// FROM CombinedActivity ca ORDER BY EngagementScore DESC, NetVotes DESC LIMIT 10;
fn q8433(db: &'static So) -> String {
    let old = || db.user.with((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ua = old()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&db.post.accepted_answer_id).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 5], |a, ((t, x), v)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 1 && x.is_some()) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64]);
    let dp = old().group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain(old().select(Ident::<User>::new().and((&dp).opt()).and((&ua).opt()).and((&ub).opt())));
    let eng = |a: [i64; 5]| a[0] * 2 + a[1];
    let v = top_n(v, |&(_, ((_, a), _)): &(Id<User>, (((Id<User>, Option<i64>), Option<[i64; 5]>), Option<[i64; 3]>))| {
        let a = a.unwrap_or([0; 5]);
        (Reverse(eng(a)), Reverse(a[3] - a[4]))
    }, 10);
    rows(v.iter().map(|&(_, (((u, n), a), b))| {
        let a = a.unwrap_or([0; 5]);
        let b = b.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(a[3] - a[4]), V::I(eng(a)), V::I(b[0] * 3 + b[1] * 2 + b[2])]);
        row(f)
    }))
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
//            SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//            AVG(EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - u.CreationDate)) / (60 * 60 * 24)) AS AvgAccountAgeDays
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, CommentCount, UpVoteCount, DownVoteCount, AvgAccountAgeDays FROM UserActivity
//              WHERE PostCount > 0 ORDER BY PostCount DESC LIMIT 10),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, ... FROM Badges b GROUP BY b.UserId)
// SELECT tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.CommentCount, tu.UpVoteCount, tu.DownVoteCount, COALESCE(ub.BadgeCount, 0) AS BadgeCount, ...,
//        tu.AvgAccountAgeDays
// FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId ORDER BY tu.PostCount DESC;
fn q8625(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.creation_date).and(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold(([0i64; 5], 0.0f64, 0i64), |(a, s, n), (cd, p)| {
            let age = secs(now - cd) / (60.0 * 60.0 * 24.0);
            let a = match p {
                Some(((t, c), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
                None => a,
            };
            (a, s + age, n + 1)
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let tu = top_n(drain((&dp).filt(|n| n > 0).and(&ua)), |&(_, (n, _))| Reverse(n), 10);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let mut out = Vec::new();
    let tu = rel(tu);
    (&tu).and((&tu).map(|(u, _)| u).select((&ub).opt())).drive(|_, ((u, (n, (a, s, k))), b)| {
        let b = b.unwrap_or([0; 4]);
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.iter().map(|&x| V::I(x)));
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3]), V::F(s / k as f64)]);
        out.push(row(f))
    });
    rows(out)
}


// WITH UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) as GoldBadges, ... FROM Badges b GROUP BY b.UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//                      COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers
//               FROM Posts p GROUP BY p.OwnerUserId),
// RecentActivity AS (SELECT p.OwnerUserId, MAX(p.LastActivityDate) AS LastActive FROM Posts p WHERE p.LastActivityDate IS NOT NULL GROUP BY p.OwnerUserId),
// QualifiedUsers AS (SELECT u.Id, u.DisplayName, COALESCE(ub.GoldBadges, 0) + COALESCE(ub.SilverBadges, 0) + COALESCE(ub.BronzeBadges, 0) AS TotalBadges,
//                           ps.TotalPosts, ps.TotalViews, ps.TotalScore, ra.LastActive
//                    FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN RecentActivity ra ON u.Id = ra.OwnerUserId
//                    WHERE u.Reputation > 100 AND ((ps.TotalQuestions > 5 AND ps.TotalAnswers > 10) OR (ub.GoldBadges > 0)))
// SELECT q.DisplayName, q.TotalBadges, COALESCE(q.TotalPosts, 0) AS TotalPosts, COALESCE(q.TotalViews, 0) AS TotalViews, COALESCE(q.TotalScore, 0) AS TotalScore, q.LastActive,
//        CASE WHEN q.LastActive IS NULL THEN 'Never Active' WHEN q.LastActive < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN 'Inactive for over a year'
//             ELSE 'Active Recently' END AS ActivityStatus
// FROM QualifiedUsers q ORDER BY q.TotalBadges DESC, q.TotalScore DESC, q.LastActive DESC;
fn q21570(db: &'static So) -> String {
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ups = user_posts(db);
    let ra = owned(db).group_by(&db.post.owner_user).select(&db.post.last_activity_date).fold(i64::MIN, |m, d| m.max(d));
    let old = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let mut out = Vec::new();
    db.user
        .with((&db.user.reputation).gt(100))
        .select(Ident::<User>::new().and((&ub).opt()).and((&ups).filt(|a| a[1] > 0).opt()).and((&ra).opt()))
        .filt(|(((_, b), p), _)| p.is_some_and(|p| p[2] > 5 && p[3] > 10) || b.is_some_and(|b| b[0] > 0))
        .drive(|_, (((u, b), p), r)| {
            let b = b.unwrap_or([0; 3]);
            let p = p.unwrap_or([0; 10]);
            let status = match r {
                None => "Never Active",
                Some(r) if r < old => "Inactive for over a year",
                _ => "Active Recently",
            };
            out.push(row(vec![user_col(db, u, "name"), V::I(b[0] + b[1] + b[2]), V::I(p[1]), V::I(p[6]), V::I(p[4]), ots(r), V::S(status)]))
        });
    rows(out)
}

// WITH RECURSIVE RecursiveCTE AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//                                 UNION ALL SELECT a.Id, a.Title, a.CreationDate, a.Score, a.OwnerUserId, r.Level + 1 FROM Posts a INNER JOIN RecursiveCTE r ON a.ParentId = r.PostId
//                                 WHERE a.PostTypeId = 2),
// PostVotes AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.DisplayName),
// PostHistorySummary AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId, ph.PostHistoryTypeId)
// SELECT q.PostId AS QuestionId, q.Title AS QuestionTitle, q.CreationDate AS QuestionDate, q.Score AS QuestionScore, u.DisplayName AS Owner, COALESCE(v.UpVotes, 0) AS TotalUpVotes,
//        COALESCE(v.DownVotes, 0) AS TotalDownVotes, COALESCE(v.TotalVotes, 0) AS TotalVotes, COALESCE(b.BadgeCount, 0) AS OwnerBadges, COALESCE(phs.EditCount, 0) AS TotalEdits,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = q.PostId) AS CommentCount, (SELECT COUNT(*) FROM Posts a WHERE a.ParentId = q.PostId) AS AnswerCount
// FROM RecursiveCTE q LEFT JOIN Users u ON q.OwnerUserId = u.Id LEFT JOIN PostVotes v ON q.PostId = v.PostId LEFT JOIN UserReputation b ON u.Id = b.UserId
// LEFT JOIN PostHistorySummary phs ON q.PostId = phs.PostId WHERE q.Level = 1 ORDER BY q.Score DESC, q.CreationDate DESC LIMIT 100;
//
// The recursive step only adds answers at Level >= 2, so `q.Level = 1`
// keeps exactly the base case, the questions. No fixpoint is needed.
fn q31841(db: &'static So) -> String {
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let bc = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let phs = db.post_history.with((&db.post_history.post_history_type_id).is_in([4, 5, 6])).group_by((&db.post_history.post).and(&db.post_history.post_history_type_id)).fold(0i64, |n, _| n + 1);
    let keys: MatSet<(Id<Post>, i64)> = whole(&phs).collect();
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64)> = (&keys).map(|(p, _)| p).inv().collect();
    let (cp, ap) = (comments_per_post(db), answers_per_post(db));
    let q = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(&bc).opt()).and((&by_post).select(&phs).opt()).and(&cp).and(&ap));
    let v = top_n(drain(q), |&(p, _)| (Reverse(db.post.score.get(p).unwrap()), Reverse(db.post.creation_date.get(p).unwrap())), 100);
    rows(v.iter().map(|&(_, (((((p, x), b), e), c), a))| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(x[0]), V::I(x[1]), V::I(x[2]), V::I(b.unwrap_or(0)), V::I(e.unwrap_or(0)), V::I(c), V::I(a)]);
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount, ... FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostAnalytics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, COALESCE(NULLIF(P.AcceptedAnswerId, -1), 0) AS AcceptedAnswer,
//                          COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//                          SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//                   FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '6 months'
//                   GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId, P.AcceptedAnswerId),
// TopAuthors AS (SELECT U.Id, U.DisplayName, COUNT(P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS TotalComments FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId
//                LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName)
// SELECT U.DisplayName AS Author, U.Reputation, UBC.GoldBadgeCount, UBC.SilverBadgeCount, UBC.BronzeBadgeCount, TA.PostCount, TA.TotalComments, PA.Title, PA.CommentCount,
//        PA.UpVoteCount, PA.DownVoteCount, PA.CreationDate, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = PA.PostId AND V.VoteTypeId = 2) AS TotalUpVotesOfPost,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = PA.PostId AND V.VoteTypeId = 3) AS TotalDownVotesOfPost
// FROM UserBadgeCounts UBC JOIN Users U ON UBC.UserId = U.Id JOIN TopAuthors TA ON U.Id = TA.Id JOIN PostAnalytics PA ON PA.OwnerUserId = U.Id
// WHERE U.Reputation > 1000 AND (PA.CommentCount > 5 OR PA.UpVoteCount > PA.DownVoteCount) ORDER BY U.Reputation DESC, PA.UpVoteCount DESC;
//
// RECURSIVE is written but nothing recurses. CURRENT_DATE is the day the
// query runs; the data ends in 2024, so PostAnalytics is empty.
fn q30968(db: &'static So) -> String {
    let ubc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let ta = owned(db).group_by(&db.post.owner_user).select(comments_of(db).opt()).fold(0i64, |n, _| n + 1);
    let tc = owned(db).group_by(&db.post.owner_user).select(comments_of(db)).count_distinct();
    let pa = owned(db)
        .with((&db.post.creation_date).ge(add_months(current_date(), -6)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let by_owner: HashIdx<Id<User>, Id<Post>> = db.post.with((&pa).filt(|a| a[0] > 5 || a[1] > a[2])).select(&db.post.owner_user).inv().collect();
    let (up, dn) = (votes_of_type(db, 2), votes_of_type(db, 3));
    let mut out = Vec::new();
    db.user
        .with((&db.user.reputation).gt(1000))
        .select(Ident::<User>::new().and(&ubc).and(&ta).and((&tc).opt()).and((&by_owner).select(Ident::<Post>::new().and(&pa).and(&up).and(&dn))))
        .drive(|_, ((((u, b), n), c), (((p, a), x), y))| {
            let mut f = ucols(db, u, &["name", "rep"]);
            f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(n), V::I(c.unwrap_or(0))]);
            f.push(title(db, p));
            f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::T(db.post.creation_date.get(p).unwrap()), V::I(x), V::I(y)]);
            out.push(row(f))
        });
    rows(out)
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(v.BountyAmount) AS TotalBounty, SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3, 6, 12) LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStats AS (SELECT p.PostTypeId, COUNT(*) AS PostCount, AVG(p.Score) AS AvgScore, AVG(p.ViewCount) AS AvgViewCount FROM Posts p
//               WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.PostTypeId),
// BadgeStats AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges, ... GoldBadges, SilverBadges, BronzeBadges FROM Badges b GROUP BY b.UserId),
// FinalStats AS (SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalAnswers, ua.TotalQuestions, ua.TotalBounty, ua.TotalComments, COALESCE(bs.TotalBadges, 0) AS TotalBadges, ...,
//                       ps.PostCount, ps.AvgScore, ps.AvgViewCount
//                FROM UserActivity ua LEFT JOIN BadgeStats bs ON ua.UserId = bs.UserId
//                LEFT JOIN (SELECT PostTypeId, SUM(PostCount) AS PostCount, AVG(AvgScore) AS AvgScore, AVG(AvgViewCount) AS AvgViewCount FROM PostStats GROUP BY PostTypeId) ps ON true)
// SELECT * FROM FinalStats ORDER BY TotalPosts DESC, TotalAnswers DESC, TotalBounty DESC LIMIT 100;
//
// ps has one PostStats row per group, so its AVGs average one value each.
fn q5990(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let bv: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).is_in([2, 3, 6, 12])).select(&db.vote.post).inv().collect();
    let ua = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&bv).select((&db.vote.bounty_amount).opt()).opt()).and(comments_of(db).opt())))
        .fold([0i64; 5], |a, ((t, b), c)| {
            let b = b.flatten();
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0), a[4] + c.is_some() as i64]
        });
    let dp = rich().group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let ps = db
        .post
        .with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(&db.post.post_type_id)
        .select((&db.post.score).and((&db.post.view_count).opt()))
        .fold([0i64; 4], |a, (s, v)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]);
    let pst: HashIdx<(), (i64, [i64; 4])> = whole(&ps).select(Same::<i64>::new().and(&ps)).collect();
    let v = drain(rich().select(Ident::<User>::new().and((&dp).opt()).and((&ua).opt()).and((&bs).opt()).and(Ident::<User>::new().map(|_| ()).select(&pst).opt())));
    let v = top_n(v, |&(_, ((((_, n), a), _), _))| {
        let a = a.unwrap_or([0; 5]);
        (Reverse(n.unwrap_or(0)), Reverse(a[0]), a[2] == 0, Reverse(a[3]))
    }, 100);
    rows(v.iter().map(|&(_, ((((u, n), a), b), t))| {
        let a = a.unwrap_or([0; 5]);
        let b = b.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[4]), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        f.extend(match t {
            Some((_, p)) => [V::I(p[0]), avg(p[1], p[0]), avg(p[3], p[2])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH User_Aggregates AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes, COUNT(DISTINCT B.Id) AS BadgeCount,
//            SUM(CASE WHEN PT.Name = 'Question' AND P.Score > 0 THEN 1 ELSE 0 END) AS PositiveQuestions, SUM(CASE WHEN PT.Name = 'Answer' AND P.Score < 0 THEN 1 ELSE 0 END) AS NegativeAnswers
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostTypes PT ON P.PostTypeId = PT.Id
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes),
// PostHistory_Summary AS (SELECT PH.UserId, COUNT(CASE WHEN PHT.Name = 'Post Closed' THEN 1 END) AS ClosedPosts, COUNT(CASE WHEN PHT.Name = 'Post Reopened' THEN 1 END) AS ReopenedPosts,
//                                COUNT(*) AS TotalEdits, COUNT(DISTINCT PH.PostId) AS UniquePosts
//                         FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id GROUP BY PH.UserId),
// Post_Stats AS (SELECT P.OwnerUserId, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(P.Score) AS TotalScore, COALESCE(MAX(P.CreationDate), '1970-01-01') AS LatestPostDate
//                FROM Posts P GROUP BY P.OwnerUserId),
// Combined AS (SELECT UA.UserId, UA.DisplayName, UA.Reputation, UA.Views, UA.BadgeCount, PHS.ClosedPosts, PHS.ReopenedPosts, PS.TotalPosts, PS.TotalScore, PS.LatestPostDate
//              FROM User_Aggregates UA LEFT JOIN PostHistory_Summary PHS ON UA.UserId = PHS.UserId LEFT JOIN Post_Stats PS ON UA.UserId = PS.OwnerUserId)
// SELECT C.UserId, C.DisplayName, C.Reputation, C.Views, C.BadgeCount, COALESCE(C.ClosedPosts, 0) AS ClosedPosts, COALESCE(C.ReopenedPosts, 0) AS ReopenedPosts,
//        COALESCE(C.TotalPosts, 0) AS TotalPosts, COALESCE(C.TotalScore, 0) AS TotalScore,
//        CASE WHEN C.LatestPostDate IS NOT NULL THEN EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - C.LatestPostDate)) / 86400 ELSE NULL END AS DaysSinceLastPost
// FROM Combined C WHERE (C.Reputation > 100 OR C.BadgeCount > 5) AND (C.Views IS NOT NULL OR C.TotalPosts > 5)
// ORDER BY C.Reputation DESC, C.Views DESC, C.BadgeCount DESC LIMIT 50;
//
// Users.Views is NOT NULL in the schema, so the second condition always holds.
fn q22641(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).count_distinct();
    let phs = db.post_history.group_by(&db.post_history.user).select(htype_name(db)).fold((0i64, 0i64), |(c, r), n| (c + (n == "Post Closed") as i64, r + (n == "Post Reopened") as i64));
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and(&db.post.creation_date)).fold((0i64, 0i64, i64::MIN), |(n, s, m), (x, d)| (n + 1, s + x, m.max(d)));
    let c = db
        .user
        .select(Ident::<User>::new().and(&db.user.reputation).and((&bc).opt().map(|b: Option<i64>| b.unwrap_or(0))).and((&phs).opt()).and((&ps).opt()))
        .filt(|((((_, r), b), _), _): ((((Id<User>, i64), i64), Option<(i64, i64)>), Option<(i64, i64, i64)>)| r > 100 || b > 5);
    let v = top_n(drain(c), |&(_, ((((u, r), b), _), _))| (Reverse(r), Reverse(db.user.views.get(u).unwrap()), Reverse(b)), 50);
    rows(v.iter().map(|&(_, ((((u, _), b), h), p))| {
        let h = h.unwrap_or((0, 0));
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
        f.extend([V::I(b), V::I(h.0), V::I(h.1)]);
        f.extend(match p {
            Some((n, s, m)) => [V::I(n), V::I(s), V::F(secs(now - m) / 86400.0)],
            None => [V::I(0), V::I(0), V::Null],
        });
        row(f)
    }))
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ARRAY_AGG(DISTINCT t.TagName) AS Tags
// FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN UNNEST(string_to_array(p.Tags, ',')) AS tag(tag) ON TRUE LEFT JOIN Tags t ON t.TagName = TRIM(BOTH ' ' FROM tag.tag)
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY p.Id, p.Title, p.CreationDate, p.LastActivityDate, p.Score, p.ViewCount ORDER BY p.LastActivityDate DESC LIMIT 100;
//
// Tags has no commas, so each split is the whole Tags string, which names no
// tag: the arrays come out [NULL], and their missing ORDER BY never shows.
fn q14489(db: &'static So) -> String {
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let recent = || db.post.with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let dc = recent().group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let dv = recent().group_by(Ident::<Post>::new()).select(votes_of(db)).count_distinct();
    let tags = recent()
        .group_by(Ident::<Post>::new())
        .select((&db.post.tags_str).flat_map(|t: Str| t.split(',').map(|x| x.trim_matches(' ')).collect::<Vec<_>>()).select((&names).select(&db.tag.tag_name).opt()).opt())
        .buf_fold(|v| {
            let mut t: Vec<Option<Str>> = v.iter().map(|x| x.flatten()).collect();
            t.sort_unstable();
            t.dedup();
            &*Box::leak(t.into_boxed_slice())
        });
    let v = top_n(drain(recent().select(Ident::<Post>::new().and((&dc).opt()).and((&dv).opt()).and(&tags))), |&(p, _)| Reverse(db.post.last_activity_date.get(p).unwrap()), 100);
    rows(v.iter().map(|&(_, (((p, c), n), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity", "score", "views"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(n.unwrap_or(0))]);
        f.push(V::L(t.iter().map(|x: &Option<Str>| x.map_or(V::Null, V::S)).collect()));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, TotalViews
// FROM TopUsers WHERE ScoreRank <= 10 ORDER BY TotalScore DESC;
fn q10382(db: &'static So) -> String {
    rows(score_top(db).into_iter().map(|((u, a), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), nullable(a[6], a[5])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViews, TotalScore, ScoreRank
// FROM TopUsers WHERE ScoreRank <= 10 ORDER BY ScoreRank;
fn q10453(db: &'static So) -> String {
    rows(score_top(db).into_iter().map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStatistics)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViews, TotalScore, ScoreRank
// FROM TopUsers WHERE ScoreRank <= 10 ORDER BY ScoreRank;
fn q13159(db: &'static So) -> String {
    q10453(db)
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts, AVG(P.Score) AS AverageScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpvotedPosts, AverageScore
// FROM TopUsers WHERE Rank <= 10;
fn q12693(db: &'static So) -> String {
    let v = top_n(drain(&user_posts(db)), |&(_, a)| Reverse(a[1]), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[8]), avg(a[4], a[1])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// SortedUserStats AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, ScoreRank
// FROM SortedUserStats WHERE ScoreRank <= 10;
fn q13673(db: &'static So) -> String {
    q10453(db)
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        AVG(p.Score) AS AvgPostScore, AVG(p.ViewCount) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, AvgPostScore, AvgViewCount
// FROM TopUsers WHERE RankByPosts <= 10 ORDER BY RankByPosts;
fn q10336(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| Reverse(a[1]), asc);
    let v = top_n(drain((&w).filt(|(_, r)| r <= 10)), |&(_, (_, r))| r, 0);
    rows(v.into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[1]), avg(a[6], a[5])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM RankedPosts)
// SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, VoteCount
// FROM TopPosts WHERE Rank <= 10 ORDER BY Score DESC, ViewCount DESC;
//
// CURRENT_DATE is the day the query runs; the data ends in 2024, so it is empty.
fn q13368(db: &'static So) -> String {
    let cv = db
        .post
        .with((&db.post.creation_date).ge(add_years(current_date(), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(c, n), (x, y)| (c + x.is_some() as i64, n + y.is_some() as i64));
    let v = top_n(drain(&cv), |&(p, _)| {
        let w = db.post.view_count.get(p);
        (Reverse(db.post.score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10);
    rows(v.into_iter().map(|(p, (c, n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, COUNT(p.Id) AS TotalPosts,
//        COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// TopUsers AS (SELECT ..., DENSE_RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT u.DisplayName, t.TotalPosts, t.TotalQuestions, t.TotalAnswers, t.TotalScore, t.TotalViews, t.ScoreRank
// FROM TopUsers t JOIN Users u ON t.UserId = u.Id WHERE t.ScoreRank <= 10 ORDER BY t.TotalScore DESC;
fn q14842(db: &'static So) -> String {
    let w = win(&user_posts(db), dense_rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), nullable(a[6], a[5]), V::I(r)])
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(p.Id) AS PostCount, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY PostCount DESC, UpVoteCount DESC) AS Rank FROM UserPostStats)
// SELECT UserId, Reputation, PostCount, CommentCount, UpVoteCount, DownVoteCount FROM TopUsers WHERE Rank <= 10;
fn q11020(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((c, t)) => [a[0] + 1, a[1] + c.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    let w = win(&s, rank, |(_, a)| (Reverse(a[0]), Reverse(a[2])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//        SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionsCount, AnswersCount, TotalScore, AvgViewCount, ScoreRank
// FROM TopUsers WHERE ScoreRank <= 10 ORDER BY ScoreRank;
fn q10815(db: &'static So) -> String {
    rows(score_top(db).into_iter().map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), avg(a[6], a[5]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN p.Score ELSE 0 END) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, AvgViewCount, Rank
// FROM TopUsers WHERE Rank <= 10;
fn q13601(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 || t == 2 { s } else { 0 }, a[4] + v.is_some() as i64, a[5] + v.unwrap_or(0)],
            None => a,
        });
    let w = win(&s, row_number, |(_, a)| Reverse(a[3]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        AVG(p.Score) AS AverageScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AverageScore, TotalViews, RankByPosts
// FROM TopUsers WHERE RankByPosts <= 10 ORDER BY RankByPosts;
fn q14550(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| Reverse(a[1]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[1]), nullable(a[6], a[5]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPosts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViews, TotalScore,
//        DENSE_RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPosts WHERE PostCount > 10)
// SELECT TU.DisplayName, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.TotalViews, TU.TotalScore
// FROM TopUsers TU WHERE TU.ScoreRank <= 10 ORDER BY TU.TotalScore DESC;
fn q6480(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())))
        .fold([0i64; 6], |a, ((t, s), v)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + v.is_some() as i64, a[5] + v.unwrap_or(0)]);
    let w = whole((&s).filt(|a| a[0] > 10)).select(Ident::<User>::new().and(&s)).window(dense_rank, |(_, a)| Reverse(a[3]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[5], a[4]), V::I(a[3])])
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts, AVG(p.ViewCount) AS AverageViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, PositiveScorePosts, AverageViewCount
// FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q12077(db: &'static So) -> String {
    let v = top_n(drain(&user_posts(db)), |&(_, a)| Reverse(a[1]), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[8]), avg(a[6], a[5])]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount,
//        SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
//        AVG(P.ViewCount) AS AverageViewCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation)
// SELECT UserId, DisplayName, Reputation, PostCount, PositivePosts, NegativePosts, AverageViewCount, ReputationRank
// FROM TopUsers WHERE ReputationRank <= 10 ORDER BY ReputationRank;
fn q12200(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((s, v)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + v.is_some() as i64, a[4] + v.unwrap_or(0)],
            None => a,
        });
    let w = whole(&s).select(Ident::<User>::new().and(&db.user.reputation).and(&s)).window(rank, |((_, r), _)| Reverse(r), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, (((u, _), a), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), V::I(r)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount,
//        COUNT(DISTINCT C.Id) AS CommentCount, SUM(V.BountyAmount) AS TotalBounties
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId
//     LEFT JOIN Votes V ON U.Id = V.UserId
//     WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// HighScorers AS (SELECT ..., RANK() OVER (ORDER BY PostCount DESC, TotalBounties DESC) AS Rank FROM UserActivity)
// SELECT H.UserId, H.DisplayName, H.Reputation, H.PostCount, H.CommentCount, H.TotalBounties
// FROM HighScorers H WHERE H.Rank <= 10 ORDER BY H.Rank;
fn q9184(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let dp = rich().group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let dc = rich().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).count_distinct();
    let b = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt()).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold(None, |a: Option<i64>, (_, v)| match v.flatten() {
            Some(x) => Some(a.unwrap_or(0) + x),
            None => a,
        });
    let w = whole(rich()).select(Ident::<User>::new().and((&dp).opt()).and((&dc).opt()).and(&b)).window(rank, |(((_, p), _), b)| (Reverse(p.unwrap_or(0)), b.is_none(), Reverse(b)), asc);
    let v = top_n(drain((&w).filt(|(_, r)| r <= 10)), |&(_, (_, r))| r, 0);
    rows(v.into_iter().map(|(_, ((((u, p), c), b), _))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(p.unwrap_or(0)), V::I(c.unwrap_or(0)), b.map_or(V::Null, V::I)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.Reputation AS OwnerReputation,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.Reputation),
// TopPosts AS (SELECT ..., RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM RankedPosts)
// SELECT Id, Title, CreationDate, ViewCount, Score, OwnerReputation, CommentCount, AnswerCount
// FROM TopPosts WHERE Rank <= 100;
fn q14294(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let w = whole(db.post.with(post_type_id.eq(1))).select(Ident::<Post>::new().and(score).and(view_count.opt())).window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 100).map(|(((p, _), _), _)| p).collect();
    let dc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let da = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db)).count_distinct();
    rows(drain((&tp).select(Ident::<Post>::new().and((&dc).opt()).and((&da).opt()))).into_iter().map(|(_, ((p, c), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "rep"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0))]);
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY Score DESC) AS Rank FROM PostStatistics)
// SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, UpVotes, DownVotes FROM TopPosts WHERE Rank <= 10;
fn q14976(db: &'static So) -> String {
    let s = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, _)| Reverse(db.post.score.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPosts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserPosts)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViews, TotalScore, ScoreRank, PostRank
// FROM TopUsers WHERE ScoreRank <= 10 OR PostRank <= 10 ORDER BY ScoreRank, PostRank;
fn q10770(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    let w = (&w).window(rank, |((_, a), _): (UP, i64)| Reverse(a[1]), asc);
    let v = top_n(drain((&w).filt(|((_, s), p)| s <= 10 || p <= 10)), |&(_, ((_, s), p))| (s, p), 0);
    rows(v.into_iter().map(|(_, (((u, a), s), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1]), V::I(s), V::I(p)]);
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// TopUsers AS (SELECT UserId, PostCount, QuestionCount, AnswerCount, RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts,
//        RANK() OVER (ORDER BY QuestionCount DESC) AS RankByQuestions, RANK() OVER (ORDER BY AnswerCount DESC) AS RankByAnswers
//     FROM UserPostCounts)
// SELECT u.DisplayName, u.Reputation, t.PostCount, t.QuestionCount, t.AnswerCount, t.RankByPosts, t.RankByQuestions, t.RankByAnswers
// FROM TopUsers t JOIN Users u ON t.UserId = u.Id WHERE t.RankByPosts <= 10 ORDER BY t.RankByPosts;
fn q11348(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| Reverse(a[1]), asc);
    let w = (&w).window(rank, |((_, a), _): (UP, i64)| Reverse(a[2]), asc);
    let w = (&w).window(rank, |(((_, a), _), _): ((UP, i64), i64)| Reverse(a[3]), asc);
    let v = top_n(drain((&w).filt(|(((_, p), _), _)| p <= 10)), |&(_, (((_, p), _), _))| p, 0);
    rows(v.into_iter().map(|(_, ((((u, a), p), q), n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(p), V::I(q), V::I(n)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts, SUM(P.ViewCount) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, PositiveScorePosts, TotalViews, RankByPosts
// FROM TopUsers WHERE RankByPosts <= 10 ORDER BY RankByPosts;
fn q14685(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| Reverse(a[1]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[8]), nullable(a[6], a[5]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts, SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScorePosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, PositiveScorePosts, NegativeScorePosts FROM TopUsers WHERE Rank <= 10;
fn q11095(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + (s < 0) as i64],
            None => a,
        });
    let v = top_n(drain(&s), |&(_, a)| Reverse(a[0]), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank, u.DisplayName AS OwnerDisplayName,
//        COALESCE(p.AnswerCount, 0) AS AnswerCount, COALESCE(COUNT(c.Id), 0) AS CommentCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName, p.Score, p.AnswerCount),
// HighScorePosts AS (SELECT r.OwnerUserId, r.OwnerDisplayName, COUNT(r.Id) AS PostCount, SUM(r.Score) AS TotalScore
//     FROM RankedPosts r WHERE r.Rank = 1 GROUP BY r.OwnerUserId, r.OwnerDisplayName)
// SELECT u.Id, u.DisplayName, u.Reputation, u.CreationDate, hs.PostCount, hs.TotalScore
// FROM Users u JOIN HighScorePosts hs ON u.Id = hs.OwnerUserId WHERE u.Reputation > 1000
// ORDER BY hs.TotalScore DESC, hs.PostCount DESC LIMIT 10;
//
// Rank = 1 keeps one post per owner, whichever of its top-scoring posts, so
// PostCount is 1 and TotalScore is the owner's best score.
fn q9856(db: &'static So) -> String {
    let w = owned(db).group_by(&db.post.owner_user).select(Ident::<Post>::new().and(&db.post.score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let hs = (&w).filt(|(_, r)| r == 1).fold((0i64, 0i64), |(n, t), ((_, s), _)| (n + 1, t + s));
    let v = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(Ident::<User>::new().and(&hs))), |&(_, (_, (n, s)))| (Reverse(s), Reverse(n)), 10);
    rows(v.into_iter().map(|(_, (u, (n, s)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated"]);
        f.extend([V::I(n), V::I(s)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT Users.Id AS UserId, Users.Reputation, COUNT(Posts.Id) AS TotalPosts,
//        COUNT(CASE WHEN Posts.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN Posts.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        SUM(Posts.Score) AS TotalScore, AVG(Posts.ViewCount) AS AvgViewCount
//     FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId GROUP BY Users.Id, Users.Reputation),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC, Reputation DESC) AS UserRank FROM UserPostStats)
// SELECT UserId, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, AvgViewCount, UserRank
// FROM TopUsers WHERE UserRank <= 10 ORDER BY UserRank;
fn q11629(db: &'static So) -> String {
    let up = user_posts(db);
    let w = whole(&up).select(Ident::<User>::new().and(&db.user.reputation).and(&up)).window(rank, |((_, r), a)| (a[1] == 0, Reverse(a[4]), Reverse(r)), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, (((u, _), a), r))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), avg(a[6], a[5]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.AnswerCount, 0)) AS TotalAnswers, SUM(COALESCE(P.CommentCount, 0)) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalScore DESC, UserId) AS ScoreRank,
//        ROW_NUMBER() OVER (ORDER BY TotalViews DESC, UserId) AS ViewRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, TotalScore, TotalViews, TotalAnswers, TotalComments, ScoreRank, ViewRank
// FROM TopUsers WHERE ScoreRank <= 10 OR ViewRank <= 10 ORDER BY ScoreRank, ViewRank;
//
// rewrites/10835.sql: both windows tie-broken on UserId.
fn q10835(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count)).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((((s, v), n), c)) => [a[0] + 1, a[1] + s, a[2] + v.unwrap_or(0), a[3] + n.unwrap_or(0), a[4] + c],
            None => a,
        });
    let w = whole(&s).select(Ident::<User>::new().and(&db.user.origid).and(&s)).window(row_number, |((_, id), a)| (Reverse(a[1]), id), asc);
    let w = (&w).window(row_number, |(((_, id), a), _): (((Id<User>, i64), [i64; 5]), i64)| (Reverse(a[2]), id), asc);
    let v = top_n(drain((&w).filt(|((_, s), w)| s <= 10 || w <= 10)), |&(_, ((_, s), w))| (s, w), 0);
    rows(v.into_iter().map(|(_, ((((u, _), a), s), w))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        u.Reputation AS OwnerReputation
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.Reputation),
// TopPosts AS (SELECT *, ROW_NUMBER() OVER (ORDER BY Score DESC) AS Rank FROM PostStats)
// SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerReputation, UpVotes, DownVotes
// FROM TopPosts WHERE Rank <= 10;
fn q13883(db: &'static So) -> String {
    let s = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, _)| Reverse(db.post.score.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(a[0]));
        f.extend(post_fields(db, p, &["rep"]));
        f.extend([V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH TagStats AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId IN (10, 11, 12, 13) THEN 1 ELSE 0 END) AS CloseActionCount,
//        AVG(u.Reputation) AS AvgUserReputation
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Users u ON p.OwnerUserId = u.Id GROUP BY t.TagName),
// TopTags AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM TagStats)
// SELECT Rank, TagName, PostCount, QuestionCount, AnswerCount, CloseActionCount, AvgUserReputation
// FROM TopTags WHERE Rank <= 10 ORDER BY Rank;
fn q28410(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let s = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select((&db.post.post_type_id).and((&db.post.owner_user).select(&db.user.reputation).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some((t, r)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (10..=13).contains(&t) as i64, a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)],
            None => a,
        });
    let w = win(&s, row_number, |(_, a)| Reverse(a[0]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((t, a), r))| row(vec![V::I(r), V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4])])))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts, AVG(p.Score) AS AvgPostScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS UserRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, PositiveScorePosts, AvgPostScore
// FROM TopUsers WHERE UserRank <= 10 ORDER BY UserRank;
fn q10585(db: &'static So) -> String {
    q12693(db)
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts,
//        COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AvgScore, AVG(P.ViewCount) AS AvgViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT *, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewsRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, AvgScore, AvgViews, ScoreRank, ViewsRank
// FROM TopUsers WHERE ScoreRank <= 10 OR ViewsRank <= 10 ORDER BY ScoreRank, ViewsRank;
fn q11036(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    let w = (&w).window(rank, |((_, a), _): (UP, i64)| (a[5] == 0, Reverse(a[6])), asc);
    let v = top_n(drain((&w).filt(|((_, s), w)| s <= 10 || w <= 10)), |&(_, ((_, s), w))| (s, w), 0);
    rows(v.into_iter().map(|(_, (((u, a), s), w))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), nullable(a[6], a[5]), avg(a[4], a[1]), avg(a[6], a[5]), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' AND p.PostTypeId = 1),
// TopPosts AS (SELECT Id, Title, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount, AVG(c.Score) AS AverageCommentScore, SUM(v.BountyAmount) AS TotalBounty
// FROM TopPosts tp LEFT JOIN Comments c ON tp.Id = c.PostId LEFT JOIN Votes v ON tp.Id = v.PostId AND v.VoteTypeId IN (8, 9)
// GROUP BY tp.OwnerDisplayName ORDER BY TotalBounty DESC, AverageCommentScore DESC;
fn q7583(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let bounty: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).is_in([8, 9])).select(&db.vote.post).inv().collect();
    let s = (&tp)
        .group_by(owner_user.select(&db.user.display_name))
        .select(comments_of(db).select(&db.comment.score).opt().and((&bounty).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64, None), |(n, cs, b): (i64, i64, Option<i64>), (c, v)| {
            let b = match v.flatten() {
                Some(x) => Some(b.unwrap_or(0) + x),
                None => b,
            };
            (n + c.is_some() as i64, cs + c.unwrap_or(0), b)
        });
    let mut v = drain(&s);
    v.sort_by(|a, b| {
        let k = |&(_, (n, _, t)): &(Str, (i64, i64, Option<i64>))| (t.is_none(), Reverse(t), n == 0);
        k(a).cmp(&k(b)).then_with(|| {
            let f = |&(_, (n, s, _)): &(Str, (i64, i64, Option<i64>))| s as f64 / n as f64;
            f(b).partial_cmp(&f(a)).unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    rows(v.into_iter().map(|(u, (n, s, t))| row(vec![V::S(u), V::I(n), avg(s, n), t.map_or(V::Null, V::I)])))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(v.Id) AS VoteCount,
//        COUNT(c.Id) AS CommentCount, COUNT(b.Id) AS BadgeCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT pm.PostId, pm.Title, pm.CreationDate, pm.ViewCount, pm.Score, pm.VoteCount, pm.CommentCount,
//        ROW_NUMBER() OVER (ORDER BY pm.Score DESC) AS Rank FROM PostMetrics pm)
// SELECT PostId, Title, CreationDate, ViewCount, Score, VoteCount, CommentCount FROM TopPosts WHERE Rank <= 10;
//
// CURRENT_DATE is the day the query runs; the data ends in 2024, so it is empty.
fn q13484(db: &'static So) -> String {
    let by_user: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let s = db
        .post
        .with((&db.post.creation_date).ge(add_years(current_date(), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).opt().and(comments_of(db).opt()).and((&db.post.owner_user_id).select(&by_user).opt()))
        .fold((0i64, 0i64), |(n, m), ((v, c), _)| (n + v.is_some() as i64, m + c.is_some() as i64));
    let v = top_n(drain(&s), |&(p, _)| Reverse(db.post.score.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, (n, m))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(n), V::I(m)]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        u.DisplayName AS OwnerDisplayName
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// TopPosts AS (SELECT ..., RANK() OVER (ORDER BY Score DESC) AS Rank FROM RecentPosts)
// SELECT PostId, Title, CreationDate, Score, CommentCount, VoteCount, OwnerDisplayName FROM TopPosts WHERE Rank <= 10 ORDER BY Rank;
fn q14133(db: &'static So) -> String {
    let s = db
        .post
        .with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    let w = whole(&s).select(Ident::<Post>::new().and(&db.post.score).and(&s)).window(rank, |((_, sc), _)| Reverse(sc), asc);
    let v = top_n(drain((&w).filt(|(_, r)| r <= 10)), |&(_, (_, r))| r, 0);
    rows(v.into_iter().map(|(_, (((p, _), (n, m)), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(n), V::I(m)]);
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS TotalUpvotedPosts, AVG(p.Score) AS AvgScorePerPost
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS PostsRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotedPosts, AvgScorePerPost, PostsRank
// FROM TopUsers WHERE PostsRank <= 10 ORDER BY PostsRank;
fn q11230(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| Reverse(a[1]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[8]), avg(a[4], a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 month'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName),
// PostStats AS (SELECT ..., RANK() OVER (ORDER BY Score DESC) AS Rank FROM RecentPosts)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.Score, ps.OwnerDisplayName, ps.CommentCount, ps.Rank
// FROM PostStats ps WHERE ps.Rank <= 10 ORDER BY ps.Rank;
fn q14205(db: &'static So) -> String {
    let s = db
        .post
        .with((&db.post.creation_date).ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt())
        .fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(&s).select(Ident::<Post>::new().and(&db.post.score).and(&s)).window(rank, |((_, sc), _)| Reverse(sc), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, (((p, _), n), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(n), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewsPerPost, AVG(P.Score) AS AvgScorePerPost
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT *, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewsRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, AvgViewsPerPost, AvgScorePerPost, ScoreRank, ViewsRank
// FROM TopUsers ORDER BY ScoreRank, ViewsRank;
fn q14310(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    let w = (&w).window(rank, |((_, a), _): (UP, i64)| (a[5] == 0, Reverse(a[6])), asc);
    rows(drain(&w).into_iter().map(|(_, (((u, a), s), w))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1]), avg(a[6], a[5]), avg(a[4], a[1]), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers,
//        SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewsRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalScore, TotalViews, ScoreRank, ViewsRank
// FROM TopUsers WHERE ScoreRank <= 10 OR ViewsRank <= 10 ORDER BY ScoreRank, ViewsRank;
fn q11043(db: &'static So) -> String {
    rows(score_and(db, true).into_iter().map(|(((u, a), s), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), nullable(a[6], a[5]), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS VoteRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2020-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId),
// TopPosts AS (SELECT * FROM RankedPosts WHERE VoteRank <= 10)
// SELECT t.Title, t.CreationDate, u.DisplayName AS Owner, t.CommentCount, t.UpVotes, t.DownVotes, pt.Name AS PostType
// FROM TopPosts t JOIN Users u ON t.OwnerUserId = u.Id JOIN PostTypes pt ON t.PostTypeId = pt.Id
// ORDER BY t.PostTypeId, t.UpVotes DESC;
fn q7248(db: &'static So) -> String {
    let recent = || db.post.with((&db.post.creation_date).ge(date(2020, 1, 1)));
    let s = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = recent().group_by(&db.post.post_type_id).select(Ident::<Post>::new().and(&s).and((&db.post.owner_user).opt())).window(rank, |((_, a), _)| Reverse(a[1]), asc);
    let mut out = Vec::new();
    (&w).filt(|((_, u), r)| r <= 10 && u.is_some()).drive(|_, (((p, a), u), _)| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([user_col(db, u.unwrap(), "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["type"]));
        out.push(row(f));
    });
    rows(out)
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, CommentCount, UpVotes, DownVotes, LastPostDate FROM TopUsers WHERE PostRank <= 10 ORDER BY PostCount DESC;
fn q12314(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.creation_date).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some(((d, c), t)) => [a[0] + 1, a[1] + c.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64, a[4].max(d)],
            None => a,
        });
    let w = win(&s, rank, |(_, a)| Reverse(a[0]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), tmax(a[4])]);
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY PostCount DESC) AS PostRank, RANK() OVER (ORDER BY QuestionCount DESC) AS QuestionRank,
//        RANK() OVER (ORDER BY AnswerCount DESC) AS AnswerRank FROM UserPostCounts)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, PostRank, QuestionRank, AnswerRank
// FROM TopUsers WHERE PostRank <= 10 OR QuestionRank <= 10 OR AnswerRank <= 10 ORDER BY PostRank, QuestionRank, AnswerRank;
fn q13064(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| Reverse(a[1]), asc);
    let w = (&w).window(rank, |((_, a), _): (UP, i64)| Reverse(a[2]), asc);
    let w = (&w).window(rank, |(((_, a), _), _): ((UP, i64), i64)| Reverse(a[3]), asc);
    let v = top_n(drain((&w).filt(|(((_, p), q), n)| p <= 10 || q <= 10 || n <= 10)), |&(_, (((_, p), q), n))| (p, q, n), 0);
    rows(v.into_iter().map(|(_, ((((u, a), p), q), n))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(p), V::I(q), V::I(n)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS Wikis, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, Wikis, TotalViews, TotalScore, ScoreRank
// FROM TopUsers WHERE ScoreRank <= 10 ORDER BY ScoreRank;
fn q12521(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((t, s), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (3..=5).contains(&t) as i64, a[4] + v.is_some() as i64, a[5] + v.unwrap_or(0), a[6] + s],
            None => a,
        });
    let w = win(&s, rank, |(_, a)| (a[0] == 0, Reverse(a[6])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4]), nullable(a[6], a[0]), V::I(r)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(b.Class) AS BadgeScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, BadgeScore,
//        RANK() OVER (ORDER BY Reputation DESC, BadgeScore DESC) AS UserRank FROM UserReputation),
// UserPosts AS (SELECT t.UserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers
//     FROM TopUsers t JOIN Posts p ON t.UserId = p.OwnerUserId GROUP BY t.UserId)
// SELECT t.DisplayName, t.Reputation, t.PostCount, t.BadgeScore, up.TotalPosts, up.Questions, up.Answers
// FROM TopUsers t JOIN UserPosts up ON t.UserId = up.UserId WHERE t.UserRank <= 10 ORDER BY t.UserRank;
fn q9759(db: &'static So) -> String {
    let s = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold((0i64, None), |(n, b): (i64, Option<i64>), (p, c)| {
            (n + p.is_some() as i64, match c {
                Some(c) => Some(b.unwrap_or(0) + c),
                None => b,
            })
        });
    let ups = user_posts(db);
    let w = whole(&s)
        .select(Ident::<User>::new().and(&db.user.reputation).and(&s).and((&ups).filt(|a| a[1] > 0).opt()))
        .window(rank, |(((_, r), (_, b)), _)| (Reverse(r), b.is_none(), Reverse(b)), asc);
    let v = top_n(drain((&w).filt(|((_, a), r)| r <= 10 && a.is_some())), |&(_, (_, r))| r, 0);
    rows(v.into_iter().map(|(_, ((((u, _), (n, b)), a), _))| {
        let a = a.unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), b.map_or(V::Null, V::I), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalViews, TotalScore, PostRank, ScoreRank
// FROM TopUsers WHERE PostRank <= 10 OR ScoreRank <= 10 ORDER BY PostRank, ScoreRank;
fn q10939(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| Reverse(a[1]), asc);
    let w = (&w).window(rank, |((_, a), _): (UP, i64)| (a[1] == 0, Reverse(a[4])), asc);
    let v = top_n(drain((&w).filt(|((_, p), s)| p <= 10 || s <= 10)), |&(_, ((_, p), s))| (p, s), 0);
    rows(v.into_iter().map(|(_, (((u, a), p), s))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1]), V::I(p), V::I(s)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS TotalPositivePosts, AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalPositivePosts, AverageScore, AverageViewCount
// FROM TopUsers WHERE Rank <= 10;
fn q12286(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((t, s), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + s, a[5] + v.is_some() as i64, a[6] + v.unwrap_or(0)],
            None => a,
        });
    let v = top_n(drain(&s), |&(_, a)| Reverse(a[0]), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), avg(a[6], a[5])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN p.Score ELSE 0 END) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, QuestionCount, AnswerCount, TotalScore, AvgViewCount, LastPostDate
// FROM TopUsers WHERE ScoreRank <= 10;
fn q14703(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(creation_date)).opt())
        .fold([0, 0, 0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some((((t, s), v), d)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 || t == 2 { s } else { 0 }, a[4] + v.is_some() as i64, a[5] + v.unwrap_or(0), a[6].max(d)],
            None => a,
        });
    let w = win(&s, rank, |(_, a)| Reverse(a[3]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4]), tmax(a[6])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalPosts DESC) AS PostsRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, ScoreRank, PostsRank
// FROM TopUsers WHERE ScoreRank <= 10 OR PostsRank <= 10 ORDER BY ScoreRank, PostsRank;
fn q10636(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    let w = (&w).window(rank, |((_, a), _): (UP, i64)| Reverse(a[1]), asc);
    let v = top_n(drain((&w).filt(|((_, s), p)| s <= 10 || p <= 10)), |&(_, ((_, s), p))| (s, p), 0);
    rows(v.into_iter().map(|(_, (((u, a), s), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), nullable(a[6], a[5]), V::I(s), V::I(p)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// TopPosts AS (SELECT rp.*, ROW_NUMBER() OVER (ORDER BY rp.Score DESC, rp.CommentCount DESC) AS TopRank FROM RankedPosts rp WHERE rp.Rank <= 100)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes, u.DisplayName AS Author, u.Reputation
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id ORDER BY tp.TopRank LIMIT 10;
fn q9246(db: &'static So) -> String {
    let s = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let sc = |p: Id<Post>| db.post.score.get(p).unwrap();
    let tp: MatSet<Id<Post>> = rel(top_n(drain(&s), |&(p, _)| Reverse(sc(p)), 100)).map(|(p, _)| p).collect();
    let uids: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = top_n(drain((&tp).select(Ident::<Post>::new().and(&s).and((&db.post.origid).select(&uids)))), |&(_, ((p, a), _))| (Reverse(sc(p)), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(_, ((p, a), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 3)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.AnswerCount
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q8021(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, score, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score).and(creation_date)).window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|(((p, _), _), _)| p).collect();
    let dc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let da = (&tp).group_by(Ident::<Post>::new()).select(children_of(db)).count_distinct();
    let mut v = drain((&tp).select(Ident::<Post>::new().and((&dc).opt()).and((&da).opt())));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((p, c), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT u.UserId, u.DisplayName, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalScore, u.AvgViewCount, u.LastPostDate
// FROM TopUsers u WHERE u.ScoreRank <= 10 ORDER BY u.TotalScore DESC;
fn q10526(db: &'static So) -> String {
    rows(score_top(db).into_iter().map(|((u, a), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), avg(a[6], a[5]), tmax(a[7])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT tu.UserId, tu.Reputation, tu.BadgeCount, tu.QuestionCount, tu.AnswerCount, tu.UpVotes, tu.DownVotes
// FROM TopUsers tu WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC;
//
// The rank only reads Reputation, so the ten users are picked before their
// joined rows are aggregated.
fn q14556(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 5], |a, (b, p)| {
            let (t, v) = match p {
                Some((t, v)) => (Some(t), v),
                None => (None, None),
            };
            [a[0] + b.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64]
        });
    let mut v = drain(&s);
    v.sort_by_key(|&(u, _)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, COALESCE(SUM(p.Score), 0) AS TotalScore,
//        COALESCE(SUM(p.ViewCount), 0) AS TotalViews, COALESCE(SUM(p.AnswerCount), 0) AS TotalAnswers, COALESCE(SUM(p.CommentCount), 0) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS RankByScore, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, TotalScore, TotalViews, TotalAnswers, TotalComments, RankByScore, RankByViews
// FROM TopUsers WHERE RankByScore <= 10 OR RankByViews <= 10 ORDER BY RankByScore, RankByViews;
fn q13032(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count)).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((((s, v), n), c)) => [a[0] + 1, a[1] + s, a[2] + v.unwrap_or(0), a[3] + n.unwrap_or(0), a[4] + c],
            None => a,
        });
    let w = win(&s, rank, |(_, a)| Reverse(a[1]), asc);
    let w = (&w).window(rank, |((_, a), _): ((Id<User>, [i64; 5]), i64)| Reverse(a[2]), asc);
    let v = top_n(drain((&w).filt(|((_, s), w)| s <= 10 || w <= 10)), |&(_, ((_, s), w))| (s, w), 0);
    rows(v.into_iter().map(|(_, (((u, a), s), w))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(p.Id) AS PostCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation WHERE PostCount > 0)
// SELECT tu.UserId, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.WikiCount, b.Name AS BadgeName, b.Class AS BadgeClass
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId AND b.Class = 1 WHERE tu.Rank <= 10 ORDER BY tu.Reputation DESC;
fn q9700(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id))
        .fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64]);
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let tu = rel(top_n(drain(&s), |&(u, _)| Reverse(rep(u)), 10));
    let gold: HashIdx<Id<User>, Id<Badge>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).inv().collect();
    let v = drain((&tu).and((&tu).map(|(u, _)| u).select((&gold).opt())));
    rows(v.into_iter().map(|(_, ((u, a), b))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match b {
            Some(b) => [V::S(db.badge.name.get(b).unwrap()), V::I(1)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerName,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPostStats AS (SELECT rp.OwnerName, COUNT(*) AS TotalPosts, SUM(rp.Score) AS TotalScore, AVG(rp.Score) AS AvgScore, SUM(rp.ViewCount) AS TotalViews
//     FROM RankedPosts rp WHERE rp.PostRank <= 5 GROUP BY rp.OwnerName)
// SELECT t.OwnerName, t.TotalPosts, t.TotalScore, t.AvgScore, t.TotalViews, b.Name AS BadgeName, b.Class AS BadgeClass
// FROM TopPostStats t LEFT JOIN Badges b ON t.OwnerName = (SELECT DisplayName FROM Users WHERE Id = b.UserId)
// ORDER BY t.TotalScore DESC, t.OwnerName;
fn q8207(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let st = (&tp)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and((&db.post.view_count).opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let by_name: HashIdx<Str, Id<Badge>> = db.badge.select((&db.badge.user).select(&db.user.display_name)).inv().collect();
    let mut v = drain((&st).and((&by_name).select((&db.badge.name).and(&db.badge.class)).opt()));
    v.sort_by_key(|&(n, (a, _))| (Reverse(a[1]), n));
    rows(v.into_iter().map(|(n, (a, b))| {
        let mut f = vec![V::S(n), V::I(a[0]), V::I(a[1]), avg(a[1], a[0]), nullable(a[3], a[2])];
        f.extend(match b {
            Some((bn, c)) => [V::S(bn), V::I(c)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// FilteredPosts AS (SELECT rp.*, (UpVotes - DownVotes) AS NetVotes FROM RankedPosts rp WHERE Rank <= 10)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Author, fp.CommentCount, fp.NetVotes FROM FilteredPosts fp
// ORDER BY fp.NetVotes DESC, fp.CreationDate DESC;
//
// The rank only reads CreationDate, so the twenty posts are picked before
// their joined rows are aggregated.
fn q9677(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let w = db.post.with(post_type_id.is_in([1, 2])).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(n, net), (c, t)| (n + c.is_some() as i64, net + (t == Some(2)) as i64 - (t == Some(3)) as i64));
    let mut v = drain(&s);
    v.sort_by_key(|&(p, (_, net))| (Reverse(net), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (n, net))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(n), V::I(net)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts,
//        COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts, RANK() OVER (ORDER BY TotalScore DESC) AS RankByScore FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, RankByPosts, RankByScore
// FROM TopUsers WHERE RankByPosts <= 10 OR RankByScore <= 10 ORDER BY RankByPosts, RankByScore;
fn q13161(db: &'static So) -> String {
    q10939(db)
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(P.Id) AS TotalPosts,
//        COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        SUM(P.Score) AS TotalPostScore, SUM(P.ViewCount) AS TotalPostViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPostScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalPostViews DESC) AS ViewsRank FROM UserPostStats)
// SELECT UserId, Reputation, TotalPosts, QuestionCount, AnswerCount, TotalPostScore, TotalPostViews, ScoreRank, ViewsRank
// FROM TopUsers WHERE ScoreRank <= 10 OR ViewsRank <= 10 ORDER BY ScoreRank, ViewsRank;
fn q14760(db: &'static So) -> String {
    rows(score_and(db, true).into_iter().map(|(((u, a), s), w)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), nullable(a[6], a[5]), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, P.CreationDate, COALESCE(UPVotes, 0) AS UpVotes,
//        COALESCE(DownVotes, 0) AS DownVotes, COUNT(C.Id) AS CommentCount, COUNT(V.Id) AS VoteCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Users U ON P.OwnerUserId = U.Id
//     GROUP BY P.Id, P.Title, P.ViewCount, P.Score, P.CreationDate, U.UpVotes, U.DownVotes),
// TopPosts AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS Ranked FROM PostMetrics)
// SELECT PostId, Title, ViewCount, Score, CreationDate, UpVotes, DownVotes, CommentCount, VoteCount FROM TopPosts WHERE Ranked <= 100;
//
// UpVotes and DownVotes bind to the owner's Users columns. The rank only
// reads Score and ViewCount, so the hundred posts are picked first.
fn q11987(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let v = top_n(drain(db.post.select(score.and(view_count.opt()))), |&(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), 100);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    rows(drain(&s).into_iter().map(|(p, (n, m))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "created"]);
        let u = owner_user.get(p);
        f.extend([V::I(u.map_or(0, |u| db.user.up_votes.get(u).unwrap())), V::I(u.map_or(0, |u| db.user.down_votes.get(u).unwrap())), V::I(n), V::I(m)]);
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT T.TagName, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
//        SUM(COALESCE(P.AnswerCount, 0)) AS TotalAnswers, SUM(COALESCE(P.CommentCount, 0)) AS TotalComments, AVG(COALESCE(V.VoteCount, 0)) AS AverageVotes
//     FROM Tags T LEFT JOIN Posts P ON P.Tags ILIKE '%' || T.TagName || '%'
//     LEFT JOIN (SELECT PostId, COUNT(Id) AS VoteCount FROM Votes GROUP BY PostId) V ON P.Id = V.PostId
//     GROUP BY T.TagName),
// TopTags AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM TagStatistics)
// SELECT T.TagName, T.PostCount, T.TotalViews, T.TotalAnswers, T.TotalComments, T.AverageVotes FROM TopTags T WHERE T.Rank <= 10 ORDER BY T.PostCount DESC;
fn q29870(db: &'static So) -> String {
    let lt = tag_mentions_ci(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db)).fold(0i64, |n, _| n + 1);
    let Post { view_count, answer_count, comment_count, .. } = &db.post;
    let s = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select(view_count.opt().and(answer_count.opt()).and(comment_count).and((&vc).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some((((w, n), c), v)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + n.unwrap_or(0), a[3] + c, a[4] + v.unwrap_or(0), a[5] + 1],
            None => [a[0], a[1], a[2], a[3], a[4], a[5] + 1],
        });
    let v = top_n(drain(&s), |&(_, a)| Reverse(a[0]), 10);
    rows(v.into_iter().map(|(t, a)| row(vec![V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[5])])))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AvgScorePerPost, AVG(p.ViewCount) AS AvgViewsPerPost
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, AvgScorePerPost, AvgViewsPerPost
// FROM TopUsers WHERE ScoreRank <= 10;
fn q13949(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + v.is_some() as i64, a[5] + v.unwrap_or(0)],
            None => a,
        });
    let w = win(&s, rank, |(_, a)| (a[0] == 0, Reverse(a[3])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[0]), nullable(a[5], a[4]), avg(a[3], a[0]), avg(a[5], a[4])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS RankByScore, RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, AvgViewCount, RankByScore, RankByPosts
// FROM TopUsers WHERE RankByScore <= 10 OR RankByPosts <= 10 ORDER BY RankByScore, RankByPosts;
fn q12385(db: &'static So) -> String {
    rows(score_and(db, false).into_iter().map(|(((u, a), s), p)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), avg(a[6], a[5]), V::I(s), V::I(p)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AverageViewsPerPost
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalViews, TotalScore, AverageViewsPerPost, ScoreRank, ViewRank
// FROM TopUsers WHERE ScoreRank <= 10 OR ViewRank <= 10 ORDER BY ScoreRank, ViewRank;
fn q11405(db: &'static So) -> String {
    rows(score_and(db, true).into_iter().map(|(((u, a), s), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1]), avg(a[6], a[5]), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName AS UserName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS RankByScore, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews FROM UserPostStats)
// SELECT UserId, UserName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, RankByScore, RankByViews
// FROM TopUsers WHERE RankByScore <= 10 OR RankByViews <= 10 ORDER BY RankByScore, RankByViews;
fn q14651(db: &'static So) -> String {
    rows(score_and(db, true).into_iter().map(|(((u, a), s), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1]), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS RankByScore, RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, AvgViewCount, RankByScore, RankByPosts
// FROM TopUsers WHERE RankByScore <= 10 OR RankByPosts <= 10 ORDER BY RankByScore, RankByPosts;
fn q10608(db: &'static So) -> String {
    q12385(db)
}

// WITH PostStats AS (SELECT P.Id AS PostId, P.Title, COUNT(C.Id) AS CommentCount, COUNT(V.Id) AS VoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        P.CreationDate, P.OwnerUserId, U.DisplayName AS OwnerDisplayName
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.PostTypeId IN (1, 2) GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId, U.DisplayName),
// RankedPosts AS (SELECT PS.*, ROW_NUMBER() OVER (ORDER BY PS.VoteCount DESC, PS.CommentCount DESC) AS Rank FROM PostStats PS)
// SELECT RP.Rank, RP.Title, RP.CommentCount, RP.UpVoteCount, RP.DownVoteCount, RP.CreationDate, RP.OwnerDisplayName
// FROM RankedPosts RP WHERE RP.Rank <= 10 ORDER BY RP.Rank;
fn q9103(db: &'static So) -> String {
    let s = db
        .post
        .with((&db.post.post_type_id).is_in([1, 2]))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + t.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let w = win(&s, row_number, |(_, a)| (Reverse(a[1]), Reverse(a[0])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((p, a), r))| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["created", "owner"]));
        row(f)
    }))
}

// SELECT u.DisplayName AS UserName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, p.Title AS LastPostTitle, MAX(p.CreationDate) AS LastPostDate,
//        AVG(COALESCE(voteCount.UpVotes, 0)) AS AvgUpVotes, AVG(COALESCE(voteCount.DownVotes, 0)) AS AvgDownVotes, u.Reputation, u.Views,
//        ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS Rank
// FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//            FROM Votes GROUP BY PostId) AS voteCount ON p.Id = voteCount.PostId
// WHERE u.Reputation > 1000 GROUP BY u.DisplayName, p.Title, u.Reputation, u.Views HAVING COUNT(DISTINCT p.Id) > 0
// ORDER BY TotalPosts DESC, LastPostDate DESC LIMIT 10;
fn q7247(db: &'static So) -> String {
    let ud = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold((0i64, 0i64), |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64));
    let User { display_name, reputation, views, .. } = &db.user;
    let Post { title, post_type_id, creation_date, .. } = &db.post;
    type R = (Id<User>, Option<(((Option<Str>, i64), i64), Option<(i64, i64)>)>);
    let user = || Same::<R>::new().map(|(u, _): R| u);
    let post = || Same::<R>::new().map(|(_, p): R| p);
    let s = db
        .user
        .with(reputation.gt(1000))
        .select(Ident::<User>::new().and(posts_of(db).select(title.opt().and(post_type_id).and(creation_date).and((&ud).opt())).opt()))
        .group_by(user().select(display_name.and(reputation).and(views)).and(post().map(|p| p.and_then(|(((t, _), _), _)| t))))
        .select(post())
        .fold([0, 0, 0, i64::MIN, 0, 0, 0], |a, p| match p {
            Some((((_, t), d), v)) => {
                let (u, w) = v.unwrap_or((0, 0));
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(d), a[4] + u, a[5] + w, a[6] + 1]
            }
            None => [a[0], a[1], a[2], a[3], a[4], a[5], a[6] + 1],
        });
    let w = whole((&s).filt(|a| a[0] > 0)).select(Same::new().and(&s)).window(row_number, |(_, a): (_, [i64; 7])| Reverse(a[0]), asc);
    let v = top_n(drain(&w), |&(_, ((_, a), _))| (Reverse(a[0]), Reverse(a[3])), 10);
    rows(v.into_iter().map(|(_, (((((n, rep), vw), title), a), r))| {
        row(vec![V::S(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), ostr(title), tmax(a[3]), avg(a[4], a[6]), avg(a[5], a[6]), V::I(rep), V::I(vw), V::I(r)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 0
//     GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.Title, tp.Score, tp.CommentCount, tp.VoteCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
// FROM TopPosts tp JOIN Users u ON u.Id = tp.PostId ORDER BY tp.Score DESC, tp.CommentCount DESC;
//
// The rank only reads Score and CreationDate, so the posts are picked before
// their joined rows are aggregated.
fn q8413(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    let uids: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let mut v = drain((&s).and((&db.post.origid).select(&uids)));
    v.sort_by_key(|&(p, ((n, _), _))| (Reverse(score.get(p).unwrap()), Reverse(n)));
    rows(v.into_iter().map(|(p, ((n, m), u))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(n), V::I(m)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount,
//        U.DisplayName AS OwnerDisplayName, COUNT(V.Id) AS VoteCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year'
//     GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, U.DisplayName),
// TopPosts AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostStats)
// SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerDisplayName, VoteCount FROM TopPosts WHERE Rank <= 10;
//
// CURRENT_DATE is the day the query runs; the data ends in 2024, so it is empty.
fn q12024(db: &'static So) -> String {
    let s = db
        .post
        .with((&db.post.creation_date).ge(add_years(current_date(), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).opt())
        .fold(0i64, |n, v| n + v.is_some() as i64);
    let v = top_n(drain(&s), |&(p, _)| {
        let w = db.post.view_count.get(p);
        (Reverse(db.post.score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10);
    rows(v.into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.push(V::I(n));
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT ..., RANK() OVER (ORDER BY Score DESC) AS Rank FROM PostStatistics)
// SELECT PostId, Title, CreationDate, ViewCount, Score, CommentCount, VoteCount, UpVotes, DownVotes, Rank
// FROM TopPosts WHERE Rank <= 10 ORDER BY Rank;
fn q14143(db: &'static So) -> String {
    let w = whole(db.post.with((&db.post.post_type_id).eq(1))).select(Ident::<Post>::new().and(&db.post.score)).window(rank, |(_, s)| Reverse(s), asc);
    type T = ((Id<Post>, i64), i64);
    let tp: MatSet<T> = (&w).filt(|(_, r)| r <= 10).collect();
    let g = (&tp)
        .group_by(Same::<T>::new())
        .select(Same::<T>::new().map(|((p, _), _): T| p).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + t.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let v = top_n(drain(&g), |&(((_, _), r), _)| r, 0);
    rows(v.into_iter().map(|(((p, _), r), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY PostCount DESC) AS PostRank, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostCounts)
// SELECT u.DisplayName, uc.PostCount, uc.QuestionCount, uc.AnswerCount, uc.TotalViews, uc.TotalScore, t.PostRank, t.ScoreRank
// FROM Users u JOIN UserPostCounts uc ON u.Id = uc.UserId JOIN TopUsers t ON u.Id = t.UserId
// WHERE t.PostRank <= 10 OR t.ScoreRank <= 10 ORDER BY t.PostRank, t.ScoreRank;
fn q12682(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| Reverse(a[1]), asc);
    let w = (&w).window(rank, |((_, a), _): (UP, i64)| (a[1] == 0, Reverse(a[4])), asc);
    let v = top_n(drain((&w).filt(|((_, p), s)| p <= 10 || s <= 10)), |&(_, ((_, p), s))| (p, s), 0);
    rows(v.into_iter().map(|(_, (((u, a), p), s))| {
        row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1]), V::I(p), V::I(s)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) AS VoteCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT rp.*, ROW_NUMBER() OVER (ORDER BY CommentCount DESC, VoteCount DESC) AS OverallRank FROM RankedPosts rp WHERE rp.PostRank = 1)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.CommentCount, tp.VoteCount
// FROM TopPosts tp WHERE tp.OverallRank <= 10 ORDER BY tp.CommentCount DESC, tp.VoteCount DESC;
fn q9129(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let up: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.post).inv().collect();
    let dc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let dv = (&tp).group_by(Ident::<Post>::new()).select(&up).count_distinct();
    let v = top_n(drain((&tp).select(Ident::<Post>::new().and((&dc).opt()).and((&dv).opt()))), |&(_, ((_, c), n))| (Reverse(c.unwrap_or(0)), Reverse(n.unwrap_or(0))), 10);
    rows(v.into_iter().map(|(_, ((p, c), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(n.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, AVG(p.Score) AS AverageScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats)
// SELECT t1.DisplayName, t1.TotalPosts, t1.TotalQuestions, t1.TotalAnswers, t1.PositivePosts, t1.AverageScore,
//        t2.DisplayName AS TopAnswerer, t2.TotalAnswers AS TopAnswersCount
// FROM TopUsers t1 LEFT JOIN TopUsers t2 ON t1.TotalAnswers < t2.TotalAnswers WHERE t1.PostRank <= 10 ORDER BY t1.TotalPosts DESC;
fn q9265(db: &'static So) -> String {
    let ups = user_posts(db);
    let ans: HashIdx<i64, Id<User>> = (&ups).map(|a: [i64; 10]| a[3]).inv().collect();
    let w = win(&ups, rank, |(_, a)| Reverse(a[1]), asc);
    type T = (UP, i64);
    let tu: MatSet<T> = (&w).filt(|(_, r)| r <= 10).collect();
    let mut v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|((_, a), _): T| a[3]).select_where(&ans, |x: i64, y: i64| x < y).select(Ident::<User>::new().and(&ups)).opt())));
    v.sort_by_key(|&(_, (((_, a), _), _))| Reverse(a[1]));
    rows(v.into_iter().map(|(_, (((u, a), _), t))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[8]), avg(a[4], a[1])];
        f.extend(match t {
            Some((t, b)) => [user_col(db, t, "name"), V::I(b[3])],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, rp.VoteCount, rp.UserRank
//     FROM RankedPosts rp WHERE rp.UserRank = 1)
// SELECT u.DisplayName AS Owner, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.CommentCount, tp.VoteCount
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id ORDER BY tp.Score DESC, tp.ViewCount DESC LIMIT 10;
//
// The ranks only read Score and ViewCount, so the ten posts are picked
// before their joined rows are aggregated.
fn q7504(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let uids: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = top_n(drain((&tp).select(score.and(view_count.opt()).and((&db.post.origid).select(&uids)))), |&(_, ((s, w), _))| (Reverse(s), w.is_none(), Reverse(w)), 10);
    type T = (Id<Post>, Id<User>);
    let t10: MatSet<T> = rel(v.into_iter().map(|(p, (_, u))| (p, u)).collect()).map(|x| x).collect();
    let post = || Same::<T>::new().map(|(p, _): T| p);
    let c = (&t10).group_by(Same::<T>::new()).select(post().select(comments_of(db).opt().and(votes_of(db).opt()))).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let d = (&t10).group_by(Same::<T>::new()).select(post().select(votes_of(db))).count_distinct();
    rows(drain((&c).and((&d).opt())).into_iter().map(|((p, u), (n, m))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "views", "score"]));
        f.extend([V::I(n), V::I(m.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank, RANK() OVER (ORDER BY PostCount DESC) AS PostCountRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalViews, TotalScore, ReputationRank, PostCountRank
// FROM TopUsers WHERE ReputationRank <= 10 OR PostCountRank <= 10 ORDER BY ReputationRank, PostCountRank;
fn q12876(db: &'static So) -> String {
    let up = user_posts(db);
    let w = whole(&up).select(Ident::<User>::new().and(&db.user.reputation).and(&up)).window(rank, |((_, r), _)| Reverse(r), asc);
    let w = (&w).window(rank, |((_, a), _): (((Id<User>, i64), [i64; 10]), i64)| Reverse(a[1]), asc);
    let v = top_n(drain((&w).filt(|((_, r), p)| r <= 10 || p <= 10)), |&(_, ((_, r), p))| (r, p), 0);
    rows(v.into_iter().map(|(_, ((((u, _), a), r), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1]), V::I(r), V::I(p)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS TotalUpvotedPosts,
//        AVG(P.Score) AS AverageScore, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id),
// TopActiveUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS UserRank FROM UserPostStats)
// SELECT U.DisplayName, U.Reputation, TA.TotalPosts, TA.TotalQuestions, TA.TotalAnswers, TA.TotalUpvotedPosts, TA.AverageScore, TA.LastPostDate
// FROM TopActiveUsers TA JOIN Users U ON TA.UserId = U.Id WHERE TA.UserRank <= 10 ORDER BY TA.UserRank;
fn q13932(db: &'static So) -> String {
    let w = win(&user_posts(db), rank, |(_, a)| Reverse(a[1]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[8]), avg(a[4], a[1]), tmax(a[7])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount, RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// TopRankedPosts AS (SELECT *, ROW_NUMBER() OVER (PARTITION BY OwnerDisplayName ORDER BY Score DESC) AS OwnerRank FROM RankedPosts WHERE ScoreRank <= 10)
// SELECT trp.OwnerDisplayName, trp.Title, trp.CreationDate, trp.Score, trp.CommentCount, trp.AnswerCount
// FROM TopRankedPosts trp WHERE trp.OwnerRank <= 5 ORDER BY trp.Score DESC, trp.CreationDate DESC;
//
// ScoreRank only reads Score and CreationDate, so the posts are picked
// before their joined rows are aggregated.
fn q8701(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let w = whole(db.post.with(post_type_id.eq(1))).select(Ident::<Post>::new().and(score).and(creation_date)).window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let w = (&top).group_by(owner_user.select(&db.user.display_name).opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let dc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let da = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db)).count_distinct();
    let mut v = drain((&tp).select(Ident::<Post>::new().and((&dc).opt()).and((&da).opt())));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((p, c), a))| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "score"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY BadgeCount DESC) AS BadgeRank FROM UserBadgeStats)
// SELECT u.Id, u.DisplayName, p.Title, p.CreationDate, p.Score, p.ViewCount, p.Tags, t.BadgeRank, t.GoldBadges, t.SilverBadges, t.BronzeBadges
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id JOIN TopUsers t ON u.Id = t.UserId
// WHERE p.PostTypeId = 1 AND t.BadgeRank <= 10 AND p.Score > 10 ORDER BY t.BadgeRank, p.Score DESC;
fn q5372(db: &'static So) -> String {
    let bs = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt())
        .fold([0i64; 4], |a, c| match c {
            Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
            None => a,
        });
    let w = win(&bs, rank, |(_, a)| Reverse(a[0]), asc);
    type T = ((Id<User>, [i64; 4]), i64);
    let tu: MatSet<T> = (&w).filt(|(_, r)| r <= 10).collect();
    let Post { post_type_id, score, .. } = &db.post;
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|((u, _), _): T| u).select(posts_of(db).with(post_type_id.eq(1).and(score.gt(10)))))));
    let v = top_n(v, |&(_, ((_, r), p))| (r, Reverse(score.get(p).unwrap())), 0);
    rows(v.into_iter().map(|(_, (((u, a), r), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views", "tags"]));
        f.extend([V::I(r), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS OwnerName, COUNT(a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, u.DisplayName),
// PostRanking AS (SELECT rp.PostId, rp.Title, rp.OwnerName, rp.AnswerCount, rp.UpVotes, rp.DownVotes,
//        RANK() OVER (ORDER BY rp.AnswerCount DESC, rp.UpVotes DESC, rp.CreationDate DESC) AS Rank FROM RankedPosts rp)
// SELECT pr.Rank, pr.Title, pr.OwnerName, pr.AnswerCount, pr.UpVotes, pr.DownVotes FROM PostRanking pr WHERE pr.Rank <= 10 ORDER BY pr.Rank;
fn q26237(db: &'static So) -> String {
    let s = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = whole(&s).select(Ident::<Post>::new().and(&db.post.creation_date).and(&s)).window(rank, |((_, d), a)| (Reverse(a[0]), Reverse(a[1]), Reverse(d)), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, (((p, _), a), r))| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["title", "owner"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount,
//        u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, PH.PHCount, COALESCE(AVG(v.BountyAmount), 0) AS AvgBounty
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN (SELECT PostId, COUNT(*) AS PHCount FROM PostHistory GROUP BY PostId) PH ON p.Id = PH.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName, u.Reputation, PH.PHCount)
// SELECT PostId, Title, CreationDate, ViewCount, Score, AnswerCount, CommentCount, OwnerDisplayName, OwnerReputation, PHCount, AvgBounty,
//        RANK() OVER (ORDER BY ViewCount DESC) AS ViewRank, RANK() OVER (ORDER BY Score DESC) AS ScoreRank
// FROM PostMetrics ORDER BY ViewCount DESC, Score DESC;
fn q11686(db: &'static So) -> String {
    let ph = db.post_history.group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let bounty: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(8)).select(&db.vote.post).inv().collect();
    let ab = db
        .post
        .group_by(Ident::<Post>::new())
        .select((&bounty).select(&db.vote.bounty_amount).opt())
        .fold((0i64, 0i64), |(n, s), b| (n + b.is_some() as i64, s + b.unwrap_or(0)));
    let Post { view_count, score, .. } = &db.post;
    type X = (((Id<Post>, Option<i64>), i64), ((i64, i64), Option<i64>));
    let w = whole(&ab).select(Ident::<Post>::new().and(view_count.opt()).and(score).and((&ab).and((&ph).opt()))).window(rank, |(((_, w), _), _): X| (w.is_none(), Reverse(w)), asc);
    let w = (&w).window(rank, |((((_, _), s), _), _): (X, i64)| Reverse(s), asc);
    let v = top_n(drain(&w), |&(_, (((((_, w), s), _), _), _))| (w.is_none(), Reverse(w), Reverse(s)), 0);
    rows(v.into_iter().map(|(_, (((((p, _), _), ((n, s), h)), w), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "rep"]);
        f.extend([h.map_or(V::Null, V::I), if n == 0 { V::F(0.0) } else { V::F(s as f64 / n as f64) }, V::I(w), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS TotalComments,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.TotalComments, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.*, (tp.UpVotes - tp.DownVotes) AS NetVotes FROM TopPosts tp ORDER BY tp.CreationDate DESC;
//
// The rank only reads CreationDate, so the posts are picked before their
// joined rows are aggregated.
fn q6653(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(date(2023, 1, 1))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain(&s);
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(a[1] - a[2]));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("34957", q34957),
    ("11476", q11476),
    ("7762", q7762),
    ("33552", q33552),
    ("9497", q9497),
    ("30398", q30398),
    ("8242", q8242),
    ("26657", q26657),
    ("2232", q2232),
    ("32194", q32194),
    ("8433", q8433),
    ("8625", q8625),
    ("21570", q21570),
    ("31841", q31841),
    ("30968", q30968),
    ("5990", q5990),
    ("22641", q22641),
    ("14489", q14489),
    ("10382", q10382),
    ("10453", q10453),
    ("13159", q13159),
    ("12693", q12693),
    ("13673", q13673),
    ("10336", q10336),
    ("13368", q13368),
    ("14842", q14842),
    ("11020", q11020),
    ("10815", q10815),
    ("13601", q13601),
    ("14550", q14550),
    ("6480", q6480),
    ("12077", q12077),
    ("12200", q12200),
    ("9184", q9184),
    ("14294", q14294),
    ("14976", q14976),
    ("10770", q10770),
    ("11348", q11348),
    ("14685", q14685),
    ("11095", q11095),
    ("9856", q9856),
    ("11629", q11629),
    ("10835", q10835),
    ("13883", q13883),
    ("28410", q28410),
    ("10585", q10585),
    ("11036", q11036),
    ("7583", q7583),
    ("13484", q13484),
    ("14133", q14133),
    ("11230", q11230),
    ("14205", q14205),
    ("14310", q14310),
    ("11043", q11043),
    ("7248", q7248),
    ("12314", q12314),
    ("13064", q13064),
    ("12521", q12521),
    ("9759", q9759),
    ("10939", q10939),
    ("12286", q12286),
    ("14703", q14703),
    ("10636", q10636),
    ("9246", q9246),
    ("8021", q8021),
    ("10526", q10526),
    ("14556", q14556),
    ("13032", q13032),
    ("9700", q9700),
    ("8207", q8207),
    ("9677", q9677),
    ("13161", q13161),
    ("14760", q14760),
    ("11987", q11987),
    ("29870", q29870),
    ("13949", q13949),
    ("12385", q12385),
    ("11405", q11405),
    ("14651", q14651),
    ("10608", q10608),
    ("9103", q9103),
    ("7247", q7247),
    ("8413", q8413),
    ("12024", q12024),
    ("14143", q14143),
    ("12682", q12682),
    ("9129", q9129),
    ("9265", q9265),
    ("7504", q7504),
    ("12876", q12876),
    ("13932", q13932),
    ("8701", q8701),
    ("5372", q5372),
    ("26237", q26237),
    ("11686", q11686),
    ("6653", q6653),
];
