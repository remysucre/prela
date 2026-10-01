use harness::prelude::*;
use std::cmp::Reverse;

fn top_k<X, K: Ord, T: Ord>(mut v: Vec<X>, sql: impl Fn(&X) -> K, tie: impl Fn(&X) -> T, n: usize) -> Vec<X> {
    v.sort_by(|a, b| sql(a).cmp(&sql(b)).then_with(|| tie(a).cmp(&tie(b))));
    if n > 0 && n < v.len() && sql(&v[n - 1]) == sql(&v[n]) {
        eprintln!("tie at the LIMIT cut");
    }
    if n > 0 {
        v.truncate(n);
    }
    v
}

// WITH UserVoteCounts AS (SELECT UserId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(*) AS TotalVotes
//     FROM Votes GROUP BY UserId),
// PostEditHistory AS (SELECT ph.PostId, MAX(CASE WHEN PostHistoryTypeId IN (4, 5, 6) THEN CreationDate END) AS LastEditDate, COUNT(CASE WHEN PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
//        COUNT(CASE WHEN PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount FROM PostHistory ph GROUP BY ph.PostId),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u JOIN UserVoteCounts uvc ON uvc.UserId = u.Id
//     WHERE uvc.TotalVotes > 0),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'))
// SELECT TOP.Title, TOP.CreationDate, TOP.ViewCount, COALESCE(uh.UpVotes, 0) AS UpVotes, COALESCE(uh.DownVotes, 0) AS DownVotes, pe.LastEditDate,
//        CASE WHEN pe.CloseCount > 0 THEN 'Closed' WHEN pe.ReopenCount > 0 THEN 'Reopened' ELSE 'Active' END AS PostStatus,
//        CONCAT(u.DisplayName, CASE WHEN u.DisplayName IS NOT NULL THEN CONCAT(' (+', u.Reputation, ')') ELSE '' END) AS UserWithReputation
// FROM TopPosts TOP LEFT JOIN PostEditHistory pe ON TOP.PostId = pe.PostId LEFT JOIN UserVoteCounts uh ON uh.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = TOP.PostId)
// LEFT JOIN MostActiveUsers u ON u.UserId = uh.UserId
// WHERE (uh.TotalVotes IS NULL OR uh.TotalVotes > 4) AND (TOP.ScoreRank <= 10 OR TOP.Score IS NOT NULL) ORDER BY TOP.Score DESC, TOP.CreationDate DESC LIMIT 20 OFFSET 0;
//
// Score is never NULL, so the ScoreRank test always holds and every recent post is kept. UserVoteCounts is keyed by the raw Votes.UserId, so the owner joins it by raw id.
fn q20120(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, .. } = &db.post;
    let Vote { user_id, vote_type_id, .. } = &db.vote;
    let uvc = db.vote.group_by(user_id).select(vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pe = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold((i64::MIN, 0i64, 0i64), |(m, c, r), (t, d)| {
        (if matches!(t, 4 | 5 | 6) { m.max(d) } else { m }, c + (t == 10) as i64, r + (t == 11) as i64)
    });
    let uh = owner_user_id.select(Same::<i64>::new().and(&uvc).and((&uidx).opt())).opt();
    let v = drain(
        db.post
            .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
            .select((&pe).opt().and(uh))
            .filt(|(_, u): (Option<(i64, i64, i64)>, Option<((i64, [i64; 3]), Option<Id<User>>)>)| u.map_or(true, |((_, a), _)| a[2] > 4)),
    );
    let v = top_k(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), |&(p, _)| p, 20);
    rows(v.into_iter().map(|(p, (e, h))| {
        let mut f = post_fields(db, p, &["title", "created", "views"]);
        let a = h.map_or([0; 3], |((_, a), _)| a);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(e.map_or(V::Null, |(m, _, _)| tmax(m)));
        f.push(V::S(match e {
            Some((_, c, _)) if c > 0 => "Closed",
            Some((_, _, r)) if r > 0 => "Reopened",
            _ => "Active",
        }));
        f.push(match h.and_then(|(_, u)| u) {
            Some(u) => V::Owned(format!("{} (+{})", db.user.display_name.get(u).unwrap(), db.user.reputation.get(u).unwrap())),
            None => V::S(""),
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// PostLinkCounts AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS LinkCount FROM PostLinks pl GROUP BY pl.PostId),
// FinalMetrics AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, COALESCE(pc.CommentCount, 0) AS CommentCount, pc.LastCommentDate, COALESCE(plc.LinkCount, 0) AS LinkCount, ub.BadgeCount,
//        ub.GoldBadgeCount, ub.SilverBadgeCount, ub.BronzeBadgeCount
//     FROM RankedPosts rp LEFT JOIN PostComments pc ON rp.PostId = pc.PostId LEFT JOIN PostLinkCounts plc ON rp.PostId = plc.PostId LEFT JOIN Users u ON rp.PostId = u.Id
//     LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE rp.Rank <= 5)
// SELECT fm.PostId, fm.Title, fm.Score, fm.ViewCount, fm.CommentCount, fm.LastCommentDate, fm.LinkCount, fm.BadgeCount, fm.GoldBadgeCount, fm.SilverBadgeCount, fm.BronzeBadgeCount
// FROM FinalMetrics fm ORDER BY fm.Score DESC, fm.ViewCount DESC LIMIT 10;
//
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. Rank reads only base columns, so the top five per type are picked first.
fn q30915(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, origid, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (key(p), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    let lc = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).opt()).fold(0i64, |n, l| n + l.is_some() as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&pc).and(&lc).and(origid.select(&uidx).select(&ub).opt()));
    let v = top_k(v, |&(p, _)| key(p), |&(p, _)| p, 10);
    rows(v.into_iter().map(|(p, (((n, m), l), b))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(n), tmax(m), V::I(l)]);
        match b {
            Some(b) => f.extend(b.map(V::I)),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, DisplayName, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// MostActivePosts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, SUM(P.AnswerCount) AS TotalAnswers, AVG(P.Score) AS AverageScore
//     FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// UserBadgeSummary AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId),
// PostHistorySummary AS (SELECT PH.UserId, COUNT(PH.Id) AS EditCount, COUNT(DISTINCT PH.PostId) AS EditedPosts, MAX(PH.CreationDate) AS LastEditDate FROM PostHistory PH
//     WHERE PH.PostHistoryTypeId IN (4, 5, 24) GROUP BY PH.UserId)
// SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(URS.ReputationRank, 0) AS ReputationRank, COALESCE(MAP.PostCount, 0) AS PostCount, COALESCE(MAP.TotalViews, 0) AS TotalViews,
//        COALESCE(MAP.TotalAnswers, 0) AS TotalAnswers, COALESCE(MAP.AverageScore, 0) AS AverageScore, COALESCE(UBS.BadgeCount, 0) AS BadgeCount, COALESCE(UBS.GoldBadges, 0) AS GoldBadges,
//        COALESCE(UBS.SilverBadges, 0) AS SilverBadges, COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges, COALESCE(PHS.EditCount, 0) AS EditCount, COALESCE(PHS.EditedPosts, 0) AS EditedPosts,
//        COALESCE(PHS.LastEditDate, NULL) AS LastEditDate
// FROM Users U LEFT JOIN UserReputation URS ON U.Id = URS.UserId LEFT JOIN MostActivePosts MAP ON U.Id = MAP.OwnerUserId LEFT JOIN UserBadgeSummary UBS ON U.Id = UBS.UserId
// LEFT JOIN PostHistorySummary PHS ON U.Id = PHS.UserId WHERE U.Reputation > 1000 ORDER BY U.Reputation DESC, U.DisplayName ASC LIMIT 100;
fn q3839(db: &'static So) -> String {
    let User { reputation, display_name, .. } = &db.user;
    let rr = rel(ranked(drain(reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let Post { creation_date, owner_user, view_count, answer_count, score, .. } = &db.post;
    let map = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(view_count.opt().and(answer_count.opt()).and(score))
        .fold([0i64; 6], |a, ((w, n), s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + n.is_some() as i64, a[4] + n.unwrap_or(0), a[5] + s]);
    let ubs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let PostHistory { user, post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let edits = || db.post_history.with(post_history_type_id.is_in([4, 5, 24]));
    let phs = edits().group_by(user).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let phd = edits().group_by(user).select(post).count_distinct();
    let v = drain(db.user.with(reputation.gt(1000)).select((&rank).map(|(_, r)| r).and((&map).opt()).and((&ubs).opt()).and((&phs).and(&phd).opt())));
    let v = top_k(v, |&(u, _)| (Reverse(reputation.get(u).unwrap()), display_name.get(u).unwrap()), |&(u, _)| u, 100);
    rows(v.into_iter().map(|(u, (((r, m), b), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(r));
        f.extend(match m {
            Some(a) => [V::I(a[0]), V::I(a[2]), V::I(a[4]), avg(a[5], a[0])],
            None => [V::I(0), V::I(0), V::I(0), V::F(0.0)],
        });
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.extend(match h {
            Some(((n, m), d)) => [V::I(n), V::I(d), V::T(m)],
            None => [V::I(0), V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH RecursiveUserVotes AS (SELECT U.Id AS UserId, U.DisplayName, V.VoteTypeId, COUNT(V.Id) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY COUNT(V.Id) DESC) AS VoteRank
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, V.VoteTypeId HAVING COUNT(V.Id) > 0),
// ActivePosts AS (SELECT P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, COALESCE(AP.AnnualScore, 0) AS AnnualScore, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        RANK() OVER(ORDER BY P.ViewCount DESC) AS PopularityRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId
//     LEFT JOIN (SELECT PostId, SUM(Score) AS AnnualScore FROM Posts WHERE CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY PostId) AP ON P.Id = AP.PostId
//     WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '2 years' GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, AP.AnnualScore),
// UserTopVotes AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (ORDER BY SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) DESC) AS UserRank FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName)
// SELECT UP.UserId, UP.DisplayName, UP.UpVotes, UP.DownVotes, AP.Id AS PostId, AP.Title, AP.CreationDate, AP.ViewCount, AP.Score, AP.AnnualScore, AP.CommentCount, UP.UserRank, AP.PopularityRank
// FROM UserTopVotes UP JOIN ActivePosts AP ON UP.UserId = AP.Id WHERE UP.UserRank <= 10 AND AP.PopularityRank <= 50 ORDER BY UP.UpVotes DESC, AP.ViewCount DESC;
//
// RecursiveUserVotes is never read. Posts has no PostId, so DuckDB binds the AP subquery's PostId to C.PostId and runs it once per joined comment: it is one row, the
// sum over the last year's posts, when there is any such post, and it matches exactly the rows that have a comment. `UP.UserId = AP.Id` joins a user id to a post
// id, so it goes through the raw ids. A tie in UserRank goes to the smaller user id.
fn q30911(db: &'static So) -> String {
    let Post { creation_date, score, view_count, origid, .. } = &db.post;
    let cd = current_date();
    let annual = db.post.with(creation_date.ge(add_years(cd, -1))).select(score).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let ap = db.post.with(creation_date.ge(add_years(cd, -2))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pr = ranked(drain(&ap), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, false);
    let pr = rel(pr.into_iter().take_while(|x| x.1 <= 50).map(|((p, n), r)| (p, n, r)).collect());
    let by_raw: HashIdx<i64, (Id<Post>, i64, i64)> = (&pr).map(|(p, _, _)| p).select(origid).inv().select(&pr).collect();
    let ut = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + matches!(t, Some(2 | 3)) as i64, a[1] + (t == Some(3)) as i64]);
    let ur = top_k(drain(&ut), |&(_, a)| Reverse(a[0]), |&(u, _)| u, 10);
    let ur = rel(ur.into_iter().enumerate().map(|(i, (u, a))| (u, (a, i as i64 + 1))).collect());
    let v = drain((&ur).select(Same::<(Id<User>, ([i64; 2], i64))>::new().and(Same::<(Id<User>, ([i64; 2], i64))>::new().map(|(u, _): (Id<User>, ([i64; 2], i64))| u).select(&db.user.origid).select(&by_raw))));
    rows(v.into_iter().map(|(_, ((u, (a, r)), (p, n, pr)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(if n > 0 && annual.0 > 0 { annual.1 } else { 0 }), V::I(n), V::I(r), V::I(pr)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountySpent, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName, u.Reputation),
// PopularTags AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, AVG(p.ViewCount) AS AvgViews FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// RecentPostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(ph.EditCount, 0) AS EditCount
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS EditCount FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6) GROUP BY PostId) ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days')
// SELECT us.DisplayName, us.Reputation, us.PostCount, us.QuestionCount, us.AnswerCount, pt.TagName, pt.PostCount AS TagPostCount, pt.AvgViews, rpa.Title, rpa.CreationDate, rpa.CommentCount,
//        rpa.EditCount
// FROM UserStatistics us JOIN PopularTags pt ON us.PostCount > 5 LEFT JOIN RecentPostActivity rpa ON us.UserId = rpa.PostId WHERE us.Rank <= 100 ORDER BY us.Reputation DESC, pt.PostCount DESC;
//
// Rank reads only Reputation, so the hundred users are picked first (a tie goes to the smaller user id) and the posts x votes product is driven for those alone.
// The ON condition names only us, so users and tags are crossed. `us.UserId = rpa.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q1986(db: &'static So) -> String {
    let top = top_k(drain(&db.user.reputation), |&(_, r)| Reverse(r), |&(u, _)| u, 100);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9])));
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt())
        .fold([0i64; 2], |a, x| match x {
            Some((t, _)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64],
            None => a,
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let now = now_utc();
    let recent = Ident::<Post>::new().with((&db.post.creation_date).filt(move |d: i64| ny_to_utc(d) >= now - 30 * DAY_US));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let ec = db.post_history.with((&db.post_history.post_history_type_id).is_in([4, 5, 6])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let rpa = (&db.user.origid).select(&pidx).select(recent.and((&cc).opt()).and((&ec).opt())).opt();
    let ud = rel(drain((&us).and((&pc).filt(|n| n > 5)).and(rpa)));
    let ts_ = tag_stats(db);
    let pt = (&ts_).filt(|a: [i64; 6]| a[0] > 0);
    let mut v = Vec::new();
    (&ud).cross(pt).drive(|(_, t), ((u, ((a, n), r)), s)| v.push((u, a, n, r, t, s)));
    rows(v.into_iter().map(|(u, a, n, r, t, s)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::S(db.tag.tag_name.get(t).unwrap()), V::I(s[0]), avg(s[2], s[1])]);
        f.extend(match r {
            Some(((p, c), e)) => {
                let mut g = post_fields(db, p, &["title", "created"]);
                g.extend([V::I(c.unwrap_or(0)), V::I(e.unwrap_or(0))]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.PostRank, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount FROM RankedPosts rp WHERE rp.PostRank <= 5),
// PostHistoryStats AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastClosed, MAX(CASE WHEN ph.PostHistoryTypeId = 12 THEN ph.CreationDate END) AS LastDeleted,
//        COUNT(*) FILTER (WHERE ph.PostHistoryTypeId IN (10, 11, 12)) AS CloseEditCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpvoteCount, tp.DownvoteCount, phs.LastClosed, phs.LastDeleted, phs.CloseEditCount,
//        CASE WHEN phs.LastClosed IS NOT NULL THEN 'Closed' WHEN phs.LastDeleted IS NOT NULL THEN 'Deleted' ELSE 'Active' END AS PostStatus
// FROM TopPosts tp LEFT OUTER JOIN PostHistoryStats phs ON tp.PostId = phs.PostId WHERE tp.ViewCount > (SELECT AVG(ViewCount) FROM Posts)
// ORDER BY tp.Score DESC, tp.CommentCount DESC, CASE WHEN phs.LastClosed IS NULL THEN 1 ELSE 0 END, tp.CreationDate DESC LIMIT 10;
//
// PostRank reads only Score, so the top five per type are picked first (a tie goes to the smaller post id) and the comment x vote product is driven for those alone.
fn q21467(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let (n, s) = view_count.fold_flat((0i64, 0i64), |(n, s), w| (n + 1, s + w));
    let mean = s as f64 / n as f64;
    let rp = (&tp)
        .with(view_count.filt(move |w: i64| w as f64 > mean))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold((i64::MIN, i64::MIN, 0i64), |(c, d, n), (t, x)| {
        (if t == 10 { c.max(x) } else { c }, if t == 12 { d.max(x) } else { d }, n + matches!(t, 10 | 11 | 12) as i64)
    });
    let v = drain((&rp).and((&phs).opt()));
    let v = top_k(v, |&(p, (a, h))| (Reverse(score.get(p).unwrap()), Reverse(a[0]), h.map_or(true, |h| h.0 == i64::MIN), Reverse(creation_date.get(p).unwrap())), |&(p, _)| p, 10);
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        let (c, d, n) = h.map_or((i64::MIN, i64::MIN, 0), |h| h);
        f.extend([tmax(c), tmax(d), if h.is_some() { V::I(n) } else { V::Null }]);
        f.push(V::S(if c != i64::MIN { "Closed" } else if d != i64::MIN { "Deleted" } else { "Active" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.LastActivityDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL),
// RecentUserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount, MAX(v.CreationDate) AS LastVoteDate
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     WHERE u.Reputation > 1000 AND u.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY u.Id, u.DisplayName),
// PostHistories AS (SELECT ph.PostId, MAX(CASE WHEN pht.Name = 'Post Closed' THEN ph.CreationDate END) AS LastClosed, MAX(CASE WHEN pht.Name = 'Post Reopened' THEN ph.CreationDate END) AS LastReopened
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT up.DisplayName AS UserName, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, ru.CommentCount AS UserCommentCount, ru.BadgeCount, ph.LastClosed, ph.LastReopened,
//        CASE WHEN ph.LastClosed IS NOT NULL AND (ph.LastReopened IS NULL OR ph.LastReopened < ph.LastClosed) THEN 'Closed' WHEN ph.LastReopened IS NOT NULL THEN 'Reopened' ELSE 'Active' END AS PostStatus
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id LEFT JOIN RecentUserActivity ru ON up.Id = ru.UserId LEFT JOIN PostHistories ph ON rp.PostId = ph.PostId
// WHERE rp.Rank = 1 ORDER BY rp.Score DESC, rp.LastActivityDate DESC LIMIT 50;
//
// The two COUNT(DISTINCT)s undo the fan-outs they count, so each is one fold per user; MAX(v.CreationDate) is the same over the product as over the user's votes.
// A tie for an owner's best question goes to the smaller post id.
fn q34118(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, last_activity_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let User { reputation, last_access_date, .. } = &db.user;
    let ru = || db.user.with(reputation.gt(1000).and(last_access_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).group_by(Ident::<User>::new());
    let cc = ru().select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = ru().select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let hn = htype_name(db);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.group_by(post).select(hn.and(hd)).fold((i64::MIN, i64::MIN), |(c, r), (n, d)| {
        (if n == "Post Closed" { c.max(d) } else { c }, if n == "Post Reopened" { r.max(d) } else { r })
    });
    let v = drain((&tp).select(owner_user.select((&cc).and(&bc).opt())).and((&ph).opt()));
    let v = top_k(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(last_activity_date.get(p).unwrap())), |&(p, _)| p, 50);
    rows(v.into_iter().map(|(p, (r, h))| {
        let mut f = post_fields(db, p, &["owner", "title", "score", "views", "answers", "comments"]);
        f.extend(match r {
            Some((c, b)) => [V::I(c), V::I(b)],
            None => [V::Null, V::Null],
        });
        let (c, o) = h.unwrap_or((i64::MIN, i64::MIN));
        f.extend([tmax(c), tmax(o)]);
        f.push(V::S(if c != i64::MIN && (o == i64::MIN || o < c) { "Closed" } else if o != i64::MIN { "Reopened" } else { "Active" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(CASE WHEN p.PostTypeId IN (10, 12) THEN 1 ELSE 0 END) AS ClosedPostsCount, SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// QuestionStats AS (SELECT p.Id AS QuestionId, p.Title, p.LastActivityDate, COUNT(DISTINCT c.Id) AS CommentsCount, MAX(b.Date) AS LastBadgeDate, DENSE_RANK() OVER (ORDER BY p.LastActivityDate DESC) AS ActivityRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.LastActivityDate),
// FinalStats AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.QuestionsCount, us.AnswersCount, us.ClosedPostsCount, us.TotalVotes, qs.QuestionId, qs.Title, qs.LastActivityDate,
//        qs.CommentsCount, qs.LastBadgeDate, qs.ActivityRank FROM UserStats us LEFT JOIN QuestionStats qs ON us.UserId = qs.QuestionId)
// SELECT fs.DisplayName, fs.Reputation, fs.TotalPosts, fs.QuestionsCount, fs.AnswersCount, fs.ClosedPostsCount, fs.TotalVotes, fs.Title AS QuestionTitle, fs.LastActivityDate, fs.CommentsCount,
//        CASE WHEN fs.LastBadgeDate IS NOT NULL THEN 'Yes' ELSE 'No' END AS HasBadge, COALESCE(fs.ActivityRank, 999) AS ActivityRank
// FROM FinalStats fs WHERE fs.TotalPosts > 5 ORDER BY fs.Reputation DESC, fs.LastActivityDate DESC LIMIT 10;
//
// `us.UserId = qs.QuestionId` joins a user id to a post id, so it goes through the raw ids. The vote subquery has one row per post, so it does not multiply the
// posts; COUNT(DISTINCT c.Id) undoes the comment fan-out and MAX(b.Date) is the same over the product as over the owner's badges.
fn q4593(db: &'static So) -> String {
    let Post { post_type_id, owner_user, last_activity_date, origid, .. } = &db.post;
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and((&vc).opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 10 | 12) as i64, a[4] + v.unwrap_or(0)],
        None => a,
    });
    let qs = || db.post.with(post_type_id.eq(1));
    let ar = ranked(drain(qs().select(last_activity_date)), |&(_, d)| Reverse(d), true);
    let ar = rel(ar.into_iter().map(|((p, _), r)| (p, r)).collect());
    let arank: HashIdx<Id<Post>, (Id<Post>, i64)> = (&ar).map(|(p, _)| p).inv().select(&ar).collect();
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let lb = db.badge.group_by(&db.badge.user).select(&db.badge.date).fold(i64::MIN, |m, d| m.max(d));
    let q = Ident::<Post>::new().and(&cc).and((&arank).map(|(_, r)| r)).and(owner_user.select(&lb).opt());
    let pidx: HashIdx<i64, Id<Post>> = qs().select(origid).inv().collect();
    let v = drain((&us).filt(|a: [i64; 5]| a[0] > 5).and((&db.user.origid).select(&pidx).select(q).opt()));
    let v = top_k(v, |&(u, (_, q))| {
        let d = q.map(|(((p, _), _), _)| last_activity_date.get(p).unwrap());
        (Reverse(db.user.reputation.get(u).unwrap()), d.is_none(), Reverse(d))
    }, |&(u, _)| u, 10);
    rows(v.into_iter().map(|(u, (a, q))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match q {
            Some((((p, c), r), b)) => {
                let mut g = post_fields(db, p, &["title", "activity"]);
                g.extend([V::I(c), V::S(if b.is_some() { "Yes" } else { "No" }), V::I(r)]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::S("No"), V::I(999)],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT B.Id) AS BadgeCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.AcceptedAnswerId, COALESCE(A.OwnerDisplayName, 'No Accepted Answer') AS AcceptedAnswerOwner, P.Score, P.ViewCount,
//        COUNT(C.ID) AS CommentCount, COUNT(H.Id) AS EditHistoryCount, RANK() OVER (ORDER BY P.Score DESC) AS ScoreRank
//     FROM Posts P LEFT JOIN Posts A ON P.AcceptedAnswerId = A.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory H ON P.Id = H.PostId
//     WHERE P.CreationDate > DATE '2024-10-01' - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.CreationDate, P.AcceptedAnswerId, A.OwnerDisplayName, P.Score, P.ViewCount),
// CombinedStats AS (SELECT U.DisplayName AS UserName, P.Title AS PostTitle, P.CreationDate AS PostCreationDate, P.AcceptedAnswerOwner, P.Score, P.ViewCount, P.CommentCount, P.EditHistoryCount,
//        U.ReputationRank, U.BadgeCount, U.UpVotes, U.DownVotes FROM UserStats U INNER JOIN PostDetails P ON U.UserId = P.PostId)
// SELECT UserName, PostTitle, PostCreationDate, AcceptedAnswerOwner, Score, ViewCount, CommentCount, EditHistoryCount, ReputationRank, BadgeCount, UpVotes, DownVotes
// FROM CombinedStats WHERE (ReputationRank <= 10 OR BadgeCount > 5) ORDER BY Score DESC, UserName ASC FETCH FIRST 50 ROWS ONLY;
//
// `U.UserId = P.PostId` joins a user id to a post id, so it goes through the raw ids, and the badges x votes product is driven only for the users it reaches.
// ScoreRank is never read.
fn q317(db: &'static So) -> String {
    let Post { creation_date, accepted_answer, owner_display_name, score, .. } = &db.post;
    let pd = db
        .post
        .with(creation_date.gt(add_years(date(2024, 10, 1), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).opt()))
        .fold([0i64; 2], |a, (c, h)| [a[0] + c.is_some() as i64, a[1] + h.is_some() as i64]);
    let pds: MatSet<Id<Post>> = db.post.with(&pd).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&pds).select(&db.post.origid).inv().collect();
    let hit: MatSet<Id<User>> = db.user.with((&db.user.origid).select(&pidx)).collect();
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let bd = (&hit).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let uv = (&hit)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let us = (&bd).and(&uv).and((&rank).map(|(_, r)| r)).filt(|((b, _), r): ((i64, [i64; 2]), i64)| r <= 10 || b > 5);
    let v = drain(us.and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&pd).and(accepted_answer.select(owner_display_name).opt()))));
    let v = top_k(v, |&(u, (_, ((p, _), _)))| (Reverse(score.get(p).unwrap()), db.user.display_name.get(u).unwrap()), |&(u, _)| u, 50);
    rows(v.into_iter().map(|(u, (((b, a), r), ((p, c), o)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.push(V::S(o.unwrap_or("No Accepted Answer")));
        f.extend(post_fields(db, p, &["score", "views"]));
        f.extend([V::I(c[0]), V::I(c[1]), V::I(r), V::I(b), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND (p.Title IS NOT NULL OR p.Body IS NOT NULL) GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(b.Class, 0) AS BadgeClass FROM Users u LEFT JOIN (SELECT UserId, MAX(Class) AS Class FROM Badges GROUP BY UserId) b ON u.Id = b.UserId),
// TopPosts AS (SELECT rp.*, ur.Reputation, ur.BadgeClass FROM RankedPosts rp INNER JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.PostRank <= 5 AND ur.Reputation > 1000)
// SELECT tp.Title, tp.CreationDate, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount,
//        CASE WHEN tp.BadgeClass = 1 THEN 'Gold Badge Holder' WHEN tp.BadgeClass = 2 THEN 'Silver Badge Holder' WHEN tp.BadgeClass = 3 THEN 'Bronze Badge Holder' ELSE 'No Badges' END AS BadgeStatus,
//        CASE WHEN tp.CommentCount IS NULL THEN 'No Comments' ELSE 'Comments Available' END AS CommentStatus,
//        CASE WHEN tp.UpVoteCount - tp.DownVoteCount > 0 THEN 'Positive Reception' WHEN tp.UpVoteCount - tp.DownVoteCount < 0 THEN 'Negative Reception' ELSE 'Neutral Reception' END AS ReceptionStatus
// FROM TopPosts tp WHERE tp.Title LIKE '%SQL%' ORDER BY tp.CreationDate DESC;
//
// Body is never NULL, so every recent post is ranked. PostRank reads only CreationDate, so the five newest per type are picked first (a tie goes to the smaller
// post id) and the comment x vote product is driven for those alone.
fn q22840(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, title, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let mc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold(0i64, |m, c| m.max(c));
    let ur = Ident::<User>::new().with((&db.user.reputation).gt(1000)).and((&mc).opt());
    let rp = (&tp)
        .with(title.filt(|t: Str| t.contains("SQL")))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&rp).and(owner_user.select(ur)));
    v.sort_by_key(|&(p, _)| (Reverse(creation_date.get(p).unwrap()), p));
    rows(v.into_iter().map(|(p, (a, (_, b)))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend(a.map(V::I));
        f.push(V::S(match b.unwrap_or(0) {
            1 => "Gold Badge Holder",
            2 => "Silver Badge Holder",
            3 => "Bronze Badge Holder",
            _ => "No Badges",
        }));
        f.push(V::S("Comments Available"));
        f.push(V::S(if a[1] - a[2] > 0 { "Positive Reception" } else if a[1] - a[2] < 0 { "Negative Reception" } else { "Neutral Reception" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS pn_rank
//     FROM Posts p WHERE p.PostTypeId = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(u.UpVotes, 0)) AS TotalUpVotes,
//        SUM(COALESCE(u.DownVotes, 0)) AS TotalDownVotes, ROW_NUMBER() OVER(ORDER BY SUM(COALESCE(p.Score, 0)) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, MIN(ph.CreationDate) AS FirstClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.CreationDate),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(c.Id) AS CommentCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, ROW_NUMBER() OVER(ORDER BY COUNT(c.Id) DESC) AS ActiveRank
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT tu.UserRank, tu.DisplayName, tu.QuestionCount, tu.TotalScore, tu.TotalUpVotes, tu.TotalDownVotes, ap.PostId, ap.Title, ap.CreationDate AS QuestionCreationDate, cp.FirstClosedDate,
//        au.CommentCount, au.TotalBounty, CASE WHEN cp.FirstClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM TopUsers tu JOIN RankedPosts ap ON tu.UserId = ap.OwnerUserId AND ap.pn_rank = 1 LEFT JOIN ClosedPosts cp ON ap.PostId = cp.PostId JOIN ActiveUsers au ON tu.UserId = au.UserId
// WHERE tu.UserRank <= 10 ORDER BY tu.UserRank;
//
// ClosedPosts has one row per (post, close date), so a post closed on several dates joins once per date. ActiveRank is never read, so the comment x vote product
// is driven only for the ten users. Ties in UserRank and pn_rank go to the smaller id.
fn q30061(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let asked = Ident::<Post>::new().with(post_type_id.eq(1));
    let tu = db.user.group_by(Ident::<User>::new()).select((&db.user.up_votes).and(&db.user.down_votes).and(posts_of(db).select(asked).select(score).opt())).fold([0i64; 4], |a, ((up, dn), s)| {
        [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0), a[2] + up, a[3] + dn]
    });
    let top = top_k(drain(&tu), |&(_, a)| Reverse(a[1]), |&(u, _)| u, 10);
    let tr = rel(top.into_iter().enumerate().map(|(i, (u, a))| (u, (a, i as i64 + 1))).collect());
    let tidx: HashIdx<Id<User>, (Id<User>, ([i64; 4], i64))> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let tset: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let au = (&tset)
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let best = top_per(drain(db.post.with(post_type_id.eq(1)).with(owner_user.select(&tset)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let best = rel(best.into_iter().map(|(p, u)| (u, p)).collect());
    let bidx: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&best).map(|(u, _)| u).inv().select(&best).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post.and(hd)).select(hd).fold(i64::MAX, |m, d| m.min(d));
    let cv = rel(drain(&cp));
    let cidx: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&cv).map(|((p, _), _)| p).inv().select(&cv).collect();
    let v = drain((&tidx).map(|(_, x)| x).and(&au).and((&bidx).map(|(_, p)| p).select(Ident::<Post>::new().and((&cidx).map(|(_, m)| m).opt()))));
    let v = top_k(v, |&(_, (((_, r), _), _))| r, |&(_, (_, (_, c)))| c, 0);
    rows(v.into_iter().map(|(u, (((a, r), b), (p, c)))| {
        let mut f = vec![V::I(r), user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([ots(c), V::I(b[0]), V::I(b[1]), V::S(if c.is_some() { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId = 2 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, AcceptedAnswers, Upvotes, Downvotes, Rank FROM UserStatistics WHERE Rank <= 10),
// PostEngagement AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, C.UserId AS CommentUserId, COUNT(C.Id) AS CommentCount, MAX(C.CreationDate) AS LastCommentDate,
//        CASE WHEN PH.PostId IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId AND PH.PostHistoryTypeId IN (10, 11)
//     GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, C.UserId, PH.PostId)
// SELECT U.DisplayName AS TopUser, U.Reputation AS UserReputation, P.Title AS PostTitle, P.Score AS PostScore, P.ViewCount AS PostViews, P.CommentCount AS TotalComments, P.LastCommentDate, P.PostStatus
// FROM TopUsers U JOIN PostEngagement P ON P.CommentUserId = U.UserId ORDER BY U.Reputation DESC, P.Score DESC;
//
// Rank reads only Reputation, so the ten users are picked first (a tie goes to the smaller user id); none of their aggregates is projected, so they are not
// computed. PostEngagement is grouped by (post, commenting user), so it is built over those users' comments, each crossed with its post's close/reopen rows.
fn q522(db: &'static So) -> String {
    let top = top_k(drain(&db.user.reputation), |&(_, r)| Reverse(r), |&(u, _)| u, 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Comment { post, user, creation_date, .. } = &db.comment;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let pe = db
        .comment
        .with(user.select(&tu))
        .group_by(post.and(user))
        .select(creation_date.and(post.select(closes).opt()))
        .fold((0i64, i64::MIN, false), |(n, m, c), (d, h)| (n + 1, m.max(d), c || h.is_some()));
    let v = drain(&pe);
    rows(v.into_iter().map(|((p, u), (n, m, c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(n), V::T(m), V::S(if c { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH RecursiveUserBadges AS (SELECT U.Id AS UserId, U.DisplayName, B.Class, B.Date, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY B.Date DESC) AS BadgeRank FROM Users U JOIN Badges B ON U.Id = B.UserId),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.PostTypeId, P.CreationDate, P.Title, P.Score, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '366 days'),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, PH.Comment, PH.UserId, P.Title, PH.PostHistoryTypeId, RANK() OVER (PARTITION BY PH.UserId ORDER BY PH.CreationDate DESC) AS ClosePostRank
//     FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.PostHistoryTypeId IN (10, 11)),
// TopTags AS (SELECT T.TagName, T.Count, ROW_NUMBER() OVER (ORDER BY T.Count DESC) AS TagRank FROM Tags T WHERE T.Count > 0)
// SELECT U.DisplayName, COALESCE(MIN(ST.BestPostTitle), 'No Recent Posts') AS BestRecentPost, COALESCE(SUM(CASE WHEN R.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN R.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN R.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges, COUNT(DISTINCT CP.PostId) AS TotalClosedPosts
// FROM Users U LEFT JOIN RecursiveUserBadges R ON U.Id = R.UserId AND R.BadgeRank <= 5 LEFT JOIN RecentPosts RP ON U.Id = RP.OwnerUserId AND RP.RecentPostRank = 1
// LEFT JOIN (SELECT OwnerUserId, MAX(Title) AS BestPostTitle FROM RecentPosts WHERE PostTypeId = 1 GROUP BY OwnerUserId) ST ON U.Id = ST.OwnerUserId LEFT JOIN ClosedPosts CP ON U.Id = CP.UserId
// WHERE U.Reputation > 500 AND U.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY U.DisplayName HAVING COUNT(DISTINCT CP.PostId) > 0 ORDER BY U.DisplayName;
//
// TopTags is never read. The query groups by DisplayName, so users who share a name are one group. A tie in BadgeRank goes to the smaller badge id.
fn q31680(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Badge { user: bu, date: bd, class, .. } = &db.badge;
    let rb = top_per(drain(db.badge.select(bu)), |&(_, u)| u, |&(b, _)| (Reverse(bd.get(b).unwrap()), b), 5, false);
    let rb = rel(rb.into_iter().map(|(b, u)| (u, b)).collect());
    let r: HashIdx<Id<User>, (Id<User>, Id<Badge>)> = (&rb).map(|(u, _)| u).inv().select(&rb).collect();
    let Post { creation_date, owner_user, post_type_id, title, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(t0, -366))).with(owner_user);
    let rp = top_per(drain(recent().select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let rpi: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let st = recent().with(post_type_id.eq(1)).group_by(owner_user).select(title.opt()).fold(None::<Str>, |m, t| m.max(t));
    let PostHistory { user: hu, post_history_type_id, post, .. } = &db.post_history;
    let cp: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(post_history_type_id.is_in([10, 11])).select(hu).inv().collect();
    let User { reputation, creation_date: uc, display_name, .. } = &db.user;
    let users = || db.user.with(reputation.gt(500).and(uc.lt(add_years(t0, -1)))).group_by(display_name);
    let agg = users()
        .select((&r).map(|(_, b)| b).select(class).opt().and((&rpi).opt()).and((&st).opt()).and((&cp).opt()))
        .fold((None::<Str>, [0i64; 3]), |(m, a), (((c, _), s), _)| {
            let s = s.flatten();
            let m = match (m, s) {
                (Some(x), Some(y)) => Some(x.min(y)),
                (x, None) | (None, x) => x,
            };
            (m, [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64])
        });
    let dc = users().select((&cp).select(post)).count_distinct();
    let mut v = drain((&agg).and(&dc));
    v.sort_by_key(|&(n, _)| n);
    rows(v.into_iter().map(|(n, ((m, a), d))| row(vec![V::S(n), V::S(m.unwrap_or("No Recent Posts")), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(d)])))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
//        SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// UserRankings AS (SELECT ua.UserId, ua.DisplayName, ROW_NUMBER() OVER (ORDER BY ua.PostCount DESC) AS UserRank, RANK() OVER (ORDER BY ua.BadgeCount DESC) AS BadgeRank,
//        CASE WHEN ua.PostCount = 0 THEN NULL ELSE ROUND((ua.PositivePosts * 1.0 / NULLIF(ua.PostCount, 0)) * 100, 2) END AS PositivePostPercentage FROM UserActivity ua),
// RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId, CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END AS HasAcceptedAnswer,
//        DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankByUser FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// UserPostStatistics AS (SELECT u.DisplayName, COUNT(rp.Id) AS RecentPostCount, SUM(rp.ViewCount) AS TotalRecentViews, AVG(rp.ViewCount) AS AvgViewCount, SUM(rp.HasAcceptedAnswer) AS TotalAcceptedAnswers
//     FROM Users u LEFT JOIN RecentPosts rp ON u.Id = rp.OwnerUserId GROUP BY u.DisplayName)
// SELECT ur.UserRank, ur.DisplayName, ur.BadgeRank, ur.PositivePostPercentage, ups.RecentPostCount, ups.TotalRecentViews, ups.AvgViewCount, ups.TotalAcceptedAnswers
// FROM UserRankings ur LEFT JOIN UserPostStatistics ups ON ur.DisplayName = ups.DisplayName WHERE ur.UserRank <= 10 ORDER BY ur.UserRank;
//
// PostCount, PositivePosts and BadgeCount are counted over the posts x badges product. UserPostStatistics groups by DisplayName, so users who share a name are
// one group. A tie in UserRank goes to the smaller user id.
fn q20493(db: &'static So) -> String {
    let Post { score, creation_date, view_count, accepted_answer_id, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt().and(badges_of(db).opt())).fold([0i64; 3], |a, (s, b)| {
        [a[0] + s.is_some() as i64, a[1] + (s.unwrap_or(0) > 0) as i64, a[2] + b.is_some() as i64]
    });
    let br = rel(ranked(drain(&ua), |&(_, a)| Reverse(a[2]), false).into_iter().map(|((u, a), r)| (u, (a, r))).collect());
    let bri: HashIdx<Id<User>, (Id<User>, ([i64; 3], i64))> = (&br).map(|(u, _)| u).inv().select(&br).collect();
    let recent = Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let ups = db
        .user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(recent).select(view_count.opt().and(accepted_answer_id.opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((w, x)) => [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + x.is_some() as i64],
            None => a,
        });
    let top = top_k(drain(&ua), |&(_, a)| Reverse(a[0]), |&(u, _)| u, 10);
    let tr = rel(top.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let v = drain((&tr).select(Same::<(Id<User>, i64)>::new().and(Same::<(Id<User>, i64)>::new().map(|(u, _): (Id<User>, i64)| u).select((&bri).map(|(_, x)| x).and((&db.user.display_name).select(&ups))))));
    rows(v.into_iter().map(|(_, ((u, rank), ((a, b), s)))| {
        let pct = if a[0] == 0 { V::Null } else { V::F((a[1] as f64 * 1.0 / a[0] as f64 * 100.0 * 100.0).round() / 100.0) };
        row(vec![V::I(rank), user_col(db, u, "name"), V::I(b), pct, V::I(s[0]), nullable(s[2], s[1]), avg(s[2], s[1]), if s[0] == 0 { V::Null } else { V::I(s[3]) }])
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes,
//        ROW_NUMBER() OVER (ORDER BY SUM(u.UpVotes) - SUM(u.DownVotes) DESC) AS UserRank FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostSummary AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//        AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AvgPostAgeInSeconds FROM Posts p GROUP BY p.OwnerUserId),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.BadgeCount, ps.TotalPosts, ps.QuestionsCount, ps.AnswersCount, (us.TotalUpVotes - us.TotalDownVotes) AS NetVotes
//     FROM UserStats us JOIN PostSummary ps ON us.UserId = ps.OwnerUserId WHERE us.UserRank <= 10)
// SELECT tu.DisplayName, tu.BadgeCount, tu.TotalPosts, tu.QuestionsCount, tu.AnswersCount, (CASE WHEN tu.NetVotes IS NULL THEN 'No Votes' ELSE CONCAT(CAST(tu.NetVotes AS TEXT), ' Net Votes') END) AS VoteSummary,
//        COALESCE(p.Title, 'No Recent Post') AS RecentPostTitle, (SELECT COUNT(*) FROM Comments c WHERE c.UserId = tu.UserId) AS CommentCount
// FROM TopUsers tu LEFT JOIN Posts p ON tu.UserId = p.OwnerUserId AND p.CreationDate = (SELECT MAX(CreationDate) FROM Posts WHERE OwnerUserId = tu.UserId) ORDER BY tu.BadgeCount DESC, tu.NetVotes DESC;
//
// SUM(u.UpVotes) is summed over the badge rows, so it is UpVotes once per badge. AvgPostAgeInSeconds is never read. Several of a user's posts can share the
// latest CreationDate, and each joins. A tie in UserRank goes to the smaller user id.
fn q3121(db: &'static So) -> String {
    let User { up_votes, down_votes, .. } = &db.user;
    let us = db.user.group_by(Ident::<User>::new()).select(up_votes.and(down_votes).and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 6], |a, ((up, dn), c)| {
        [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64, a[4] + up, a[5] + dn]
    });
    let top = top_k(drain(&us), |&(_, a)| Reverse(a[4] - a[5]), |&(u, _)| u, 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { owner_user, post_type_id, creation_date, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(creation_date)).fold([0i64, 0, 0, i64::MIN], |a, (t, d)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(d)]);
    let at: HashIdx<(Id<User>, i64), Id<Post>> = db.post.with(owner_user).select(owner_user.and(creation_date)).inv().collect();
    let cc = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&tu).select((&us).and(&ps).and(&cc).and(Ident::<User>::new().and((&ps).map(|a: [i64; 4]| a[3])).select(&at).opt())));
    let mut v = v;
    v.sort_by_key(|&(u, (((a, _), _), p))| (Reverse(a[0]), Reverse(a[4] - a[5]), u, p));
    rows(v.into_iter().map(|(u, (((a, s), c), p))| {
        row(vec![
            user_col(db, u, "name"),
            V::I(a[0]),
            V::I(s[0]),
            V::I(s[1]),
            V::I(s[2]),
            V::Owned(format!("{} Net Votes", a[4] - a[5])),
            V::S(p.and_then(|p| db.post.title.get(p)).unwrap_or("No Recent Post")),
            V::I(c),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, AVG(COALESCE(b.Class, 0)) AS AverageBadgeClass,
//        SUM(CASE WHEN b.Class IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId = p.Id LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// RecentActivities AS (SELECT ph.UserId, ph.PostId, ph.CreationDate, RANK() OVER (PARTITION BY ph.UserId ORDER BY ph.CreationDate DESC) AS ActivityRank FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// ConsolidatedData AS (SELECT u.UserId, u.DisplayName, u.QuestionCount, u.TotalBounty, u.AverageBadgeClass, u.BadgeCount, ra.PostId, ra.CreationDate AS RecentActivityDate, ra.ActivityRank
//     FROM UserStats u LEFT JOIN RecentActivities ra ON u.UserId = ra.UserId)
// SELECT cd.UserId, cd.DisplayName, cd.QuestionCount, cd.TotalBounty, cd.AverageBadgeClass, cd.BadgeCount, COUNT(DISTINCT rp.PostId) AS TotalPosts, MAX(cd.RecentActivityDate) AS LastActivityDate
// FROM ConsolidatedData cd LEFT JOIN RankedPosts rp ON cd.UserId = rp.OwnerUserId AND rp.Rank <= 1 WHERE cd.QuestionCount > 0
// GROUP BY cd.UserId, cd.DisplayName, cd.QuestionCount, cd.TotalBounty, cd.AverageBadgeClass, cd.BadgeCount HAVING COUNT(DISTINCT rp.PostId) > 0 ORDER BY cd.TotalBounty DESC, cd.QuestionCount DESC;
//
// The vote join keeps only votes the owner cast on their own post. RankedPosts at Rank <= 1 is one post per owner, so its distinct count is 1 exactly for the
// owners of a positive question; MAX(RecentActivityDate) is the same over the joined rows as over the user's history.
fn q33306(db: &'static So) -> String {
    let ov = own_votes(db);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&ov).select((&db.vote.bounty_amount).opt()).opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (p, c)| [a[0] + 1, a[1] + p.flatten().flatten().unwrap_or(0), a[2] + c.unwrap_or(0), a[3] + c.is_some() as i64]);
    let qc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let rp: MatSet<Id<User>> = db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user).collect();
    let PostHistory { user, post_history_type_id, creation_date, .. } = &db.post_history;
    let ra = db.post_history.with(post_history_type_id.is_in([10, 11, 12])).group_by(user).select(creation_date).fold(i64::MIN, |m, d| m.max(d));
    let mut v = drain(db.user.with(&rp).select((&us).and((&qc).filt(|n| n > 0)).and((&ra).opt())));
    v.sort_by_key(|&(u, ((a, n), _))| (Reverse(a[1]), Reverse(n), u));
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[1]), avg(a[2], a[0]), V::I(a[3]), V::I(1), ots(r)]);
        row(f)
    }))
}

// WITH RECURSIVE UserPostCount AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// RecentVotes AS (SELECT v.UserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.UserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(upc.PostCount, 0) AS PostCount, COALESCE(rv.VoteCount, 0) AS RecentVoteCount, COALESCE(rv.UpVotes, 0) AS UpVoteCount,
//        COALESCE(rv.DownVotes, 0) AS DownVoteCount, CASE WHEN u.Reputation < 100 THEN 'Newbie' WHEN u.Reputation BETWEEN 100 AND 500 THEN 'Intermediate' ELSE 'Expert' END AS UserLevel
//     FROM Users u LEFT JOIN UserPostCount upc ON u.Id = upc.UserId LEFT JOIN RecentVotes rv ON u.Id = rv.UserId),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerUserId, p.CreationDate, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '90 days')
// SELECT ur.UserId, ur.Reputation, ur.PostCount, ur.RecentVoteCount, ur.UpVoteCount, ur.DownVoteCount, ur.UserLevel, ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.AnswerCount, ps.CommentCount,
//        ps.CreationDate, ps.PostRank
// FROM UserReputation ur LEFT JOIN PostStats ps ON ur.UserId = ps.OwnerUserId WHERE (ur.Reputation > 0 OR ur.PostCount > 0) AND (ps.PostRank IS NULL OR ps.PostRank < 5)
// ORDER BY ur.Reputation DESC, ps.CreationDate DESC LIMIT 50;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q32655(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let upc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let Vote { creation_date: vd, user: vu, vote_type_id, .. } = &db.vote;
    let rv = db.vote.with(vd.ge(add_days(t0, -30))).group_by(vu).select(vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let Post { creation_date, owner_user, .. } = &db.post;
    let ps = drain(db.post.with(creation_date.ge(add_days(t0, -90))).with(owner_user).select(owner_user));
    let ps = per_group(ranked(ps, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap())), false), |&(_, u)| u);
    let ps = rel(ps.into_iter().map(|((p, u), r)| (u, (p, r))).collect());
    let psi: HashIdx<Id<User>, (Id<User>, (Id<Post>, i64))> = (&ps).map(|(u, _)| u).inv().select(&ps).collect();
    let rep = &db.user.reputation;
    let v = drain(
        db.user
            .select(rep.and(&upc).and((&rv).opt()).and((&psi).map(|(_, x)| x).opt()))
            .filt(|(((r, n), _), p): (((i64, i64), Option<[i64; 3]>), Option<(Id<Post>, i64)>)| (r > 0 || n > 0) && p.map_or(true, |(_, k)| k < 5)),
    );
    let v = top_k(v, |&(_, (((r, _), _), p))| {
        let d = p.map(|(p, _)| creation_date.get(p).unwrap());
        (Reverse(r), d.is_none(), Reverse(d))
    }, |&(u, (_, p))| (u, p.map(|x| x.0)), 50);
    rows(v.into_iter().map(|(u, (((r, n), a), p))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "uid"), V::I(r), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.push(V::S(if r < 100 { "Newbie" } else if r <= 500 { "Intermediate" } else { "Expert" }));
        match p {
            Some((p, k)) => {
                f.extend(post_fields(db, p, &["id", "title", "score", "views", "answers", "comments", "created"]));
                f.push(V::I(k));
            }
            None => f.extend((0..8).map(|_| V::Null)),
        }
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND p.PostTypeId = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '365 days' AND p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(DISTINCT p.Id) > 5
//     ORDER BY u.Reputation DESC, QuestionCount DESC LIMIT 10),
// CombinedData AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, tu.DisplayName, tu.Reputation, tu.TotalBounty, ROW_NUMBER() OVER (ORDER BY rp.CreationDate DESC) AS OverallRank
//     FROM RecentPosts rp JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId)
// SELECT cd.PostId, cd.Title, cd.CreationDate, cd.Score, cd.ViewCount, cd.AnswerCount, cd.DisplayName, cd.Reputation, cd.TotalBounty,
//        CASE WHEN cd.Score > 10 THEN 'High Score' WHEN cd.Score BETWEEN 1 AND 10 THEN 'Moderate Score' ELSE 'Low or No Score' END AS ScoreCategory,
//        CASE WHEN EXISTS (SELECT 1 FROM Posts WHERE AcceptedAnswerId = cd.PostId) THEN 'Has Accepted Answer' ELSE 'No Accepted Answer' END AS AnswerStatus
// FROM CombinedData cd WHERE cd.OverallRank <= 50 ORDER BY cd.CreationDate DESC LIMIT 20;
//
// OverallRank <= 50 then LIMIT 20 in the same order is the twenty newest; a tie goes to the smaller post id.
fn q34736(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, post_type_id, owner_user, score, accepted_answer, .. } = &db.post;
    let yq = || db.post.with(creation_date.ge(add_days(t0, -365)).and(post_type_id.eq(1))).with(owner_user).group_by(owner_user);
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let tb = yq().select(bounty.opt()).fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let qc = yq().select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = top_k(drain((&tb).and((&qc).filt(|n| n > 5))), |&(u, (_, n))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n)), |&(u, _)| u, 10);
    let tu = rel(top.into_iter().map(|(u, (b, _))| (u, b)).collect());
    let ti: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let acc: MatSet<Id<Post>> = db.post.select(accepted_answer).collect();
    let v = drain(
        db.post
            .with(creation_date.ge(add_days(t0, -30)).and(post_type_id.eq(1)))
            .select(owner_user.select(&ti).and(Ident::<Post>::new().with(&acc).opt())),
    );
    let v = top_k(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), |&(p, _)| p, 20);
    rows(v.into_iter().map(|(p, ((u, b), a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(b));
        f.push(V::S(if s > 10 { "High Score" } else if s >= 1 { "Moderate Score" } else { "Low or No Score" }));
        f.push(V::S(if a.is_some() { "Has Accepted Answer" } else { "No Accepted Answer" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Score, P.Title, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS Rank,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotesCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, P.OwnerUserId, P.Score, P.Title, P.CreationDate),
// UsersWithBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, MAX(B.Class) AS HighestBadgeLevel FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// HighScoringPosts AS (SELECT RP.PostId, RP.Score, RP.Title, UB.BadgeCount, UB.HighestBadgeLevel FROM RankedPosts RP INNER JOIN UsersWithBadges UB ON RP.OwnerUserId = UB.UserId
//     WHERE RP.Score > 100 AND RP.Rank <= 5)
// SELECT HSP.PostId, HSP.Title, HSP.Score, HSP.BadgeCount, HSP.HighestBadgeLevel, COALESCE(HSP.BadgeCount, 0) AS BadgeCountPresent,
//        CASE WHEN HSP.HighestBadgeLevel IS NULL THEN 'No badges' ELSE CASE WHEN HSP.HighestBadgeLevel = 1 THEN 'Gold' WHEN HSP.HighestBadgeLevel = 2 THEN 'Silver' ELSE 'Bronze' END END AS HighestBadgeName,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = HSP.PostId) AS CommentCount,
//        (SELECT COUNT(*) FROM PostHistory PH WHERE PH.PostId = HSP.PostId AND PH.PostHistoryTypeId IN (10, 11)) AS CloseReopenCount,
//        CASE WHEN HSP.Score > (SELECT AVG(Score) FROM Posts) THEN 'Above Average' ELSE 'Below Average' END AS ScoreComparison
// FROM HighScoringPosts HSP ORDER BY HSP.Score DESC LIMIT 10;
//
// The vote counts are never read, so they are not computed. Rank reads only CreationDate, so each owner's five newest posts are picked first (a tie goes to
// the smaller post id).
fn q20026(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let cr = (&tp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let (n, s) = score.fold_flat((0i64, 0i64), |(n, t), x| (n + 1, t + x));
    let mean = s as f64 / n as f64;
    let v = drain((&tp).with(score.gt(100)).select(owner_user.select(&ub).and(&cc).and(&cr)));
    let v = top_k(v, |&(p, _)| Reverse(score.get(p).unwrap()), |&(p, _)| p, 10);
    rows(v.into_iter().map(|(p, (((b, m), c), r))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        let hl = if b == 0 { V::Null } else { V::I(m) };
        f.extend([V::I(b), hl, V::I(b)]);
        f.push(V::S(if b == 0 { "No badges" } else if m == 1 { "Gold" } else if m == 2 { "Silver" } else { "Bronze" }));
        f.extend([V::I(c), V::I(r)]);
        f.push(V::S(if score.get(p).unwrap() as f64 > mean { "Above Average" } else { "Below Average" }));
        row(f)
    }))
}

// WITH UserRankings AS (SELECT Id AS UserId, DisplayName, Reputation, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM Users),
// PostActivity AS (SELECT p.Id AS PostId, p.OwnerUserId, p.CreationDate, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, COALESCE(MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END), NULL) AS ClosedDate,
//        COALESCE(MAX(CASE WHEN ph.PostHistoryTypeId = 12 THEN ph.CreationDate END), NULL) AS DeletedDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.OwnerUserId, p.CreationDate),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT pa.PostId) AS ActivePostCount, AVG(COALESCE(pa.CommentCount, 0)) AS AvgCommentCount
//     FROM Users u JOIN PostActivity pa ON u.Id = pa.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostInsights AS (SELECT pa.PostId, pa.OwnerUserId, pa.CommentCount, pa.UpVoteCount, pa.DownVoteCount, pa.ClosedDate, pa.DeletedDate,
//        ROW_NUMBER() OVER (ORDER BY pa.UpVoteCount DESC, pa.CommentCount DESC) AS PostRank FROM PostActivity pa)
// SELECT ur.UserId, ur.DisplayName, ur.Rank, au.ActivePostCount, au.AvgCommentCount, pi.PostId, pi.CommentCount, pi.UpVoteCount, pi.DownVoteCount,
//        CASE WHEN pi.ClosedDate IS NOT NULL THEN 'Closed' WHEN pi.DeletedDate IS NOT NULL THEN 'Deleted' ELSE 'Active' END AS PostStatus
// FROM UserRankings ur LEFT JOIN ActiveUsers au ON ur.UserId = au.UserId LEFT JOIN PostInsights pi ON au.UserId = pi.OwnerUserId WHERE ur.Rank <= 100 AND au.ActivePostCount > 0
// ORDER BY ur.Rank, pi.UpVoteCount DESC;
//
// PostRank is never read. `au.ActivePostCount > 0` keeps only users with posts, so the comments x votes x history product is driven for the posts of the
// users at DENSE_RANK <= 100 alone.
fn q22496(db: &'static So) -> String {
    let dr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let dr = rel(dr.into_iter().take_while(|x| x.1 <= 100).map(|((u, _), r)| (u, r)).collect());
    let du: HashIdx<Id<User>, (Id<User>, i64)> = (&dr).map(|(u, _)| u).inv().select(&dr).collect();
    let Post { owner_user, .. } = &db.post;
    let pa = db
        .post
        .with(owner_user.select(&du))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 5], |a, ((c, v), h)| {
            [a[0] + c.is_some() as i64, a[1] + (v == Some(2)) as i64, a[2] + (v == Some(3)) as i64, a[3] | (h == Some(10)) as i64, a[4] | (h == Some(12)) as i64]
        });
    let au = db.post.with(&pa).group_by(owner_user).select(&pa).fold([0i64; 2], |a, x| [a[0] + 1, a[1] + x[0]]);
    let v = drain(db.post.with(&pa).select(owner_user.select((&du).map(|(_, r)| r).and(&au)).and(&pa)));
    rows(v.into_iter().map(|(p, ((r, a), x))| {
        let mut f = post_fields(db, p, &["owner_id", "owner"]);
        f.extend([V::I(r), V::I(a[0]), avg(a[1], a[0])]);
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(x[0]), V::I(x[1]), V::I(x[2])]);
        f.push(V::S(if x[3] == 1 { "Closed" } else if x[4] == 1 { "Deleted" } else { "Active" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RecentVotes AS (SELECT v.PostId, COUNT(*) AS VoteCount FROM Votes v WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY v.PostId),
// PostComments AS (SELECT c.PostId, COUNT(*) AS CommentCount FROM Comments c WHERE c.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '3 months' GROUP BY c.PostId),
// PostHistoryInfo AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastEditDate, MIN(ph.CreationDate) AS FirstEditDate, COUNT(*) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 24) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, COALESCE(rv.VoteCount, 0) AS VoteCount, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(phi.LastEditDate, '1970-01-01') AS LastEditDate,
//        COALESCE(phi.FirstEditDate, '1970-01-01') AS FirstEditDate, COALESCE(phi.EditCount, 0) AS EditCount,
//        CASE WHEN rp.Score >= 10 THEN 'High' WHEN rp.Score BETWEEN 5 AND 9 THEN 'Medium' ELSE 'Low' END AS PopularityLevel,
//        CASE WHEN rp.ViewCount IS NULL THEN 'Views data missing' WHEN rp.ViewCount < 50 THEN 'Low views' WHEN rp.ViewCount <= 200 THEN 'Moderate views' ELSE 'High views' END AS ViewCategory
// FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId LEFT JOIN PostHistoryInfo phi ON rp.PostId = phi.PostId
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// A tie on CreationDate inside Rank goes to the smaller post id.
fn q3462(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rv = db.vote.with((&db.vote.creation_date).ge(add_months(t0, -6))).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let pc = db.comment.with((&db.comment.creation_date).ge(add_months(t0, -3))).group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phi = db.post_history.with(post_history_type_id.is_in([4, 5, 24])).group_by(post).select(hd).fold((i64::MIN, i64::MAX, 0i64), |(a, b, n), d| (a.max(d), b.min(d), n + 1));
    let mut v = drain((&tp).select((&rv).opt().and((&pc).opt()).and((&phi).opt())));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p));
    rows(v.into_iter().map(|(p, ((r, c), h))| {
        let s = score.get(p).unwrap();
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(r.unwrap_or(0)), V::I(c.unwrap_or(0))]);
        f.extend(match h {
            Some((a, b, n)) => [V::T(a), V::T(b), V::I(n)],
            None => [V::T(0), V::T(0), V::I(0)],
        });
        f.push(V::S(if s >= 10 { "High" } else if s >= 5 { "Medium" } else { "Low" }));
        f.push(V::S(match w {
            None => "Views data missing",
            Some(w) if w < 50 => "Low views",
            Some(w) if w <= 200 => "Moderate views",
            _ => "High views",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS UpVotesCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS DownVotesCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.Score > 0 AND p.CreationDate > DATE '2024-10-01' - INTERVAL '30 days'),
// RecentVotes AS (SELECT v.PostId, COUNT(*) AS VoteTotal, MAX(v.CreationDate) AS LastVoteDate FROM Votes v WHERE v.CreationDate > DATE '2024-10-01' - INTERVAL '14 days' GROUP BY v.PostId),
// FilteredBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Class = 1 AND b.Date >= DATE '2024-10-01' - INTERVAL '1 year' GROUP BY b.UserId),
// Final AS (SELECT rp.PostId, rp.Title, rp.Score, rp.UserPostRank, rv.VoteTotal, fb.BadgeCount, CASE WHEN rv.LastVoteDate IS NULL THEN 'No recent votes' ELSE 'Recently voted' END AS VotingStatus,
//        CASE WHEN rp.UpVotesCount - rp.DownVotesCount > 0 THEN 'Positive' ELSE 'Negative' END AS ScoreStatus
//     FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN FilteredBadges fb ON rp.AcceptedAnswerId = fb.UserId WHERE rp.UserPostRank <= 5 AND (fb.BadgeCount IS NULL OR fb.BadgeCount > 0))
// SELECT f.PostId, f.Title, f.Score, f.VoteTotal, f.VotingStatus, f.ScoreStatus FROM Final f WHERE f.Score > (SELECT AVG(Score) FROM Posts WHERE CreationDate > DATE '2024-10-01' - INTERVAL '30 days')
// ORDER BY f.Score DESC, f.VoteTotal DESC LIMIT 10;
//
// RankedPosts has no GROUP BY, so UserPostRank numbers the post x vote rows; the rows of one post tie, and a tie goes to the smaller (post, vote) id. The
// FilteredBadges join matches at most one row and its test always holds, so it neither filters nor multiplies and is not computed.
fn q23062(db: &'static So) -> String {
    let t0 = date(2024, 10, 1);
    let Post { score, creation_date, owner_user, .. } = &db.post;
    let recent = || db.post.with(score.gt(0).and(creation_date.gt(add_days(t0, -30))));
    let rows_ = drain(recent().select(owner_user.opt().and(votes_of(db).opt())));
    let top = top_per(rows_, |&(_, (u, _))| u, |&(p, (_, v))| (Reverse(creation_date.get(p).unwrap()), p, v), 5, false);
    let (n, s) = db.post.with(creation_date.gt(add_days(t0, -30))).select(score).fold_flat((0i64, 0i64), |(n, t), x| (n + 1, t + x));
    let ud = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |a, t| a + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let rv = db.vote.with((&db.vote.creation_date).gt(add_days(t0, -14))).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    type R = (Id<Post>, (Option<Id<User>>, Option<Id<Vote>>));
    let fr = rel(top);
    let v = drain(
        (&fr)
            .filt(move |(p, _): R| score.get(p).unwrap() * n > s)
            .select(Same::<R>::new().map(|(p, _): R| p).select(Ident::<Post>::new().and(&ud).and((&rv).opt()))),
    );
    let v = top_k(v, |&(_, ((p, _), r))| (Reverse(score.get(p).unwrap()), r.is_none(), Reverse(r)), |&(i, _)| i, 10);
    rows(v.into_iter().map(|(_, ((p, d), r))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([oint(r), V::S(if r.is_some() { "Recently voted" } else { "No recent votes" }), V::S(if d > 0 { "Positive" } else { "Negative" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.ViewCount, p.Score, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.PostTypeId, p.CreationDate, p.ViewCount, p.Score),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopPosts AS (SELECT rp.*, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, RANK() OVER (ORDER BY rp.Score DESC) AS OverallRank
//     FROM RankedPosts rp JOIN Users u ON rp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = u.Id) LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE rp.ViewCount > 100 AND rp.Rank <= 10),
// RecentActivity AS (SELECT PostId, MAX(CreationDate) AS LastActivityDate FROM Comments WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY PostId),
// PostAdditionalInfo AS (SELECT tp.*, ra.LastActivityDate, CASE WHEN ra.LastActivityDate IS NOT NULL THEN 'Active' ELSE 'Inactive' END AS PostStatus FROM TopPosts tp LEFT JOIN RecentActivity ra ON tp.PostId = ra.PostId)
// SELECT p.PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.BadgeCount, p.GoldBadges, p.SilverBadges, p.BronzeBadges, p.LastActivityDate, p.PostStatus
// FROM PostAdditionalInfo p ORDER BY p.OverallRank, p.Score DESC;
//
// Rank reads only Score, so the top ten per type are picked first (a tie goes to the smaller post id); CommentCount is never read. The IN subquery is the owner join.
fn q33707(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ra = db.comment.with((&db.comment.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(&db.comment.post).select(&db.comment.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let mut v = drain((&tp).with(view_count.gt(100)).select(owner_user.select(&ub).and((&ra).opt())));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), p));
    rows(v.into_iter().map(|(p, (b, r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(b.map(V::I));
        f.extend([ots(r), V::S(if r.is_some() { "Active" } else { "Inactive" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER(PARTITION BY pt.Name ORDER BY p.Score DESC, p.ViewCount DESC) AS rn,
//        COUNT(*) OVER(PARTITION BY pt.Name) AS total_posts FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > (SELECT AVG(Score) FROM Posts WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, (SELECT SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) FROM Votes v WHERE v.PostId = rp.PostId) AS UpVotes,
//        (SELECT SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) FROM Votes v WHERE v.PostId = rp.PostId) AS DownVotes, NULLIF((SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId), 0) AS CommentCount
//     FROM RankedPosts rp WHERE rp.rn <= 5),
// Combined AS (SELECT fp.PostId, fp.Title, fp.CreationDate, fp.ViewCount, fp.Score, fp.UpVotes, fp.DownVotes, fp.CommentCount,
//        CASE WHEN fp.CommentCount IS NULL THEN 'No Comments' WHEN fp.CommentCount > 10 THEN 'Active Discussion' ELSE 'Few Comments' END AS CommentStatus FROM FilteredPosts fp)
// SELECT cb.Title, cb.CreationDate, cb.ViewCount, COALESCE(cb.UpVotes, 0) AS UpVotes, COALESCE(cb.DownVotes, 0) AS DownVotes, cb.CommentCount, cb.CommentStatus,
//        CASE WHEN cb.Score IS NULL THEN 'No Score' WHEN cb.Score > 100 THEN 'Highly Rated' ELSE 'Moderately Rated' END AS PostRating
// FROM Combined cb LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = cb.PostId) WHERE b.Id IS NULL OR b.Class < 3 ORDER BY cb.Score DESC, cb.ViewCount DESC LIMIT 20;
//
// total_posts is never read. The rn ties at the cut go to the smaller post id. The Badges join repeats a post once per badge of its owner, then keeps only
// the rows with no badge or a gold or silver one.
fn q22774(db: &'static So) -> String {
    let Post { creation_date, score, view_count, post_type, owner_user, .. } = &db.post;
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let (n, s) = db.post.with(creation_date.ge(since)).select(score).fold_flat((0i64, 0i64), |(n, t), x| (n + 1, t + x));
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let v = drain(db.post.with(creation_date.ge(since).and(score.filt(move |x: i64| x * n > s))).select(post_type));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (key(p), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ob = owner_user.select(badges_of(db).select(Ident::<Badge>::new().and(&db.badge.class))).opt();
    let v = drain((&vs).and(&cc).and(ob).filt(|(_, b): (([i64; 3], i64), Option<(Id<Badge>, i64)>)| b.map_or(true, |(_, c)| c < 3)));
    let v = top_k(v, |&(p, _)| key(p), |&(p, (_, b))| (p, b.map(|x| x.0)), 20);
    rows(v.into_iter().map(|(p, ((a, c), _))| {
        let mut f = post_fields(db, p, &["title", "created", "views"]);
        let x = |k: i64| if a[0] == 0 { V::I(0) } else { V::I(k) };
        f.extend([x(a[1]), x(a[2]), if c == 0 { V::Null } else { V::I(c) }]);
        f.push(V::S(if c == 0 { "No Comments" } else if c > 10 { "Active Discussion" } else { "Few Comments" }));
        f.push(V::S(if score.get(p).unwrap() > 100 { "Highly Rated" } else { "Moderately Rated" }));
        row(f)
    }))
}

// WITH UserScoreSummary AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, QuestionCount, AnswerCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserScoreSummary),
// RecentPostActivity AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(COUNT(C.CommentId), 0) AS CommentCount, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank,
//        P.OwnerUserId FROM Posts P LEFT JOIN (SELECT DISTINCT PostId, Id AS CommentId FROM Comments) C ON P.Id = C.PostId
//     WHERE P.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days') GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId),
// AggregatedPostHistory AS (SELECT PH.PostId, PH.PostHistoryTypeId, COUNT(*) AS HistoryCount FROM PostHistory PH GROUP BY PH.PostId, PH.PostHistoryTypeId)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, U.QuestionCount, U.AnswerCount, RPA.PostId, RPA.Title, RPA.CreationDate AS RecentPostDate, RPA.CommentCount,
//        APH.HistoryCount AS EditHistoryCount, U.ReputationRank
// FROM TopUsers U LEFT JOIN RecentPostActivity RPA ON U.UserId = RPA.OwnerUserId LEFT JOIN AggregatedPostHistory APH ON RPA.PostId = APH.PostId AND APH.PostHistoryTypeId IN (4, 5, 6)
// WHERE U.Reputation > 1000 ORDER BY U.Reputation DESC, RPA.CreationDate DESC LIMIT 10;
//
// Every user contributes at least one row, and the rows come in reputation order, so only the users at RANK <= 10 can reach the LIMIT; the post x vote product
// is driven for those alone. A tie at the cut goes to the smaller (user, post, history type).
fn q31772(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let rr = ranked(drain(db.user.with(reputation.gt(1000)).select(reputation)), |&(_, r)| Reverse(r), false);
    let rr = rel(rr.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let ri: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let tu: MatSet<Id<User>> = (&rr).map(|(u, _)| u).collect();
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64],
            None => a,
        });
    let recent = Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let rc = db.post.with(owner_user.select(&tu)).with(recent).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let aph = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post.and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let av = rel(drain(&aph));
    let ai: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&av).map(|((p, _), _)| p).inv().select(&av).collect();
    let rpa = posts_of(db).select(Ident::<Post>::new().and(&rc).and((&ai).opt()));
    let v = drain((&tu).select((&us).and((&ri).map(|(_, r)| r)).and(rpa.opt())));
    let v = top_k(v, |&(u, (_, p))| {
        let d = p.map(|((p, _), _)| creation_date.get(p).unwrap());
        (Reverse(reputation.get(u).unwrap()), d.is_none(), Reverse(d))
    }, |&(u, (_, p))| (u, p.map(|((p, _), h)| (p, h.map(|((_, t), _)| t)))), 10);
    rows(v.into_iter().map(|(u, ((a, r), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        match p {
            Some(((p, c), h)) => {
                f.extend(post_fields(db, p, &["id", "title", "created"]));
                f.extend([V::I(c), h.map_or(V::Null, |(_, n)| V::I(n))]);
            }
            None => f.extend((0..5).map(|_| V::Null)),
        }
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId,
//        COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id AND c.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '7 days'), 0) AS RecentCommentCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RankByCreation FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(b.Class, 0)) AS TotalBadgeCount, COUNT(v.Id) AS TotalVotes, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id LEFT JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.DisplayName),
// AggregateStats AS (SELECT p.PostId, p.Title, p.CreationDate, u.UserId, u.DisplayName, u.TotalBadgeCount, u.TotalVotes, u.PostCount,
//        CASE WHEN p.RecentCommentCount > 0 THEN 'Has Recent Comments' ELSE 'No Recent Comments' END AS CommentStatus
//     FROM RankedPosts p JOIN UserStats u ON p.OwnerUserId = u.UserId WHERE (u.TotalVotes > 10 OR u.TotalBadgeCount >= 2) ORDER BY p.CreationDate DESC LIMIT 100)
// SELECT a.PostId, a.Title, a.CreationDate, a.DisplayName, a.TotalBadgeCount, a.CommentStatus, LEAD(a.Title) OVER (ORDER BY a.CreationDate) AS NextPostTitle,
//        DENSE_RANK() OVER (PARTITION BY a.Month ORDER BY a.CreationDate) AS PostRankInMonth
// FROM (SELECT *, DATE_TRUNC('month', CreationDate) AS Month FROM AggregateStats) a
// WHERE (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = a.UserId AND p.CreationDate < a.CreationDate) >= 2 OR (SELECT COUNT(*) FROM Votes v WHERE v.UserId = a.UserId) = 0
// ORDER BY a.CreationDate DESC;
//
// The badges x votes x posts product is driven only for the owners of the recent posts. The correlated count of a user's earlier posts is a theta join,
// `select_where` against every (owner, date, post). LEAD and DENSE_RANK are prela windows over the surviving rows; a tie on CreationDate goes to the smaller post id.
fn q23639(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, owner_user, title, .. } = &db.post;
    let recent = || db.post.with(creation_date.gt(add_days(t0, -30))).with(owner_user);
    let owners: MatSet<Id<User>> = recent().select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(votes_by(db).opt()).and(posts_of(db).opt()))
        .fold([0i64; 2], |a, ((c, v), _)| [a[0] + c.unwrap_or(0), a[1] + v.is_some() as i64]);
    let rcc = db.comment.with((&db.comment.creation_date).gt(add_days(t0, -7))).group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(recent().select(owner_user.select((&us).filt(|a: [i64; 2]| a[1] > 10 || a[0] >= 2)).and((&rcc).opt())));
    let v = top_k(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), |&(p, _)| p, 100);
    type A = (Id<Post>, ([i64; 2], Option<i64>));
    let ag = rel(v);
    let all: MatSet<(Id<User>, i64, Id<Post>)> = db.post.with(owner_user).select(owner_user.and(creation_date).and(Ident::<Post>::new())).map(|((u, d), p)| (u, d, p)).collect();
    let at = Same::<A>::new().map(|(p, _): A| p).select(owner_user.and(creation_date));
    let before = (&ag)
        .group_by(Same::<A>::new())
        .select(at.select_where(&all, |(u, d): (Id<User>, i64), (u2, d2, _): (Id<User>, i64, Id<Post>)| u2 == u && d2 < d).opt())
        .fold(0i64, |n, x| n + x.is_some() as i64);
    let vc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    type K = (A, (i64, i64));
    let kept = drain((&ag).select(Same::<A>::new().and((&before).and(Same::<A>::new().map(|(p, _): A| p).select(owner_user).select(&vc)))).filt(|(_, (b, n)): K| b >= 2 || n == 0));
    let surv = rel(kept.into_iter().map(|(_, (a, _))| a).collect::<Vec<A>>());
    type L = (i64, Id<Post>, Option<Str>);
    let lead_w = (&surv)
        .group_by(Same::<A>::new().map(|_: A| 0u8))
        .select(Same::<A>::new())
        .window(lead, |(p, _): A| -> L { (creation_date.get(p).unwrap(), p, title.get(p)) }, asc);
    let dr = (&surv)
        .group_by(Same::<A>::new().map(|(p, _): A| trunc_month(creation_date.get(p).unwrap())))
        .select(Same::<A>::new())
        .window(dense_rank, |(p, _): A| creation_date.get(p).unwrap(), asc);
    let lr = rel(drain(&lead_w).into_iter().map(|x| x.1).collect::<Vec<(A, Option<L>)>>());
    let li: HashIdx<Id<Post>, (A, Option<L>)> = (&lr).map(|((p, _), _): (A, Option<L>)| p).inv().select(&lr).collect();
    let drl = rel(drain(&dr).into_iter().map(|x| x.1).collect::<Vec<(A, i64)>>());
    let di: HashIdx<Id<Post>, (A, i64)> = (&drl).map(|((p, _), _): (A, i64)| p).inv().select(&drl).collect();
    let v = drain((&surv).select(Same::<A>::new().and(Same::<A>::new().map(|(p, _): A| p).select((&li).map(|(_, l)| l).and((&di).map(|(_, r)| r))))));
    let mut v = v;
    v.sort_by_key(|&(_, ((p, _), _))| (Reverse(creation_date.get(p).unwrap()), p));
    rows(v.into_iter().map(|(_, ((p, (a, c)), (l, r)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::S(if c.unwrap_or(0) > 0 { "Has Recent Comments" } else { "No Recent Comments" })]);
        f.push(ostr(l.and_then(|x| x.2)));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionsAnswered FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// QuestionActivity AS (SELECT P.Id AS PostId, P.Title, P.AcceptedAnswerId, COALESCE(PH.CreationDate, P.CreationDate) AS ActivityDate, ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY PH.CreationDate DESC) AS LatestEditRank
//     FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId WHERE P.PostTypeId = 1),
// FilteredQuestions AS (SELECT QA.PostId, QA.Title, QA.ActivityDate, U.UserId, U.DisplayName, U.TotalUpVotes, U.TotalDownVotes, U.QuestionsAnswered,
//        CASE WHEN AQ.Id IS NOT NULL THEN 'Accepted' ELSE 'Not Accepted' END AS AcceptanceStatus
//     FROM QuestionActivity QA JOIN UserVoteSummary U ON QA.PostId = U.UserId LEFT JOIN Posts AQ ON QA.AcceptedAnswerId = AQ.Id WHERE QA.LatestEditRank = 1),
// FinalSummary AS (SELECT FQ.Title, FQ.DisplayName, FQ.TotalUpVotes, FQ.TotalDownVotes, FQ.QuestionsAnswered, FQ.ActivityDate, FQ.AcceptanceStatus,
//        CASE WHEN FQ.TotalUpVotes IS NULL OR FQ.TotalDownVotes IS NULL THEN 'Data Incomplete' ELSE 'Complete Data' END AS DataCompleteness FROM FilteredQuestions FQ)
// SELECT Title, DisplayName, TotalUpVotes, TotalDownVotes, QuestionsAnswered, ActivityDate, AcceptanceStatus, DataCompleteness FROM FinalSummary WHERE DataCompleteness = 'Complete Data'
// ORDER BY QuestionsAnswered DESC, TotalUpVotes DESC;
//
// `QA.PostId = U.UserId` joins a post id to a user id, so it goes through the raw ids. LatestEditRank = 1 is the latest history row (DESC puts NULLs last), so
// ActivityDate is the post's latest history date, or its creation date when it has none. The SUMs are never NULL, so every row is 'Complete Data'.
fn q21827(db: &'static So) -> String {
    let Post { post_type_id, creation_date, accepted_answer, origid, .. } = &db.post;
    let ud = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let qa = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.post).select(Ident::<Post>::new().with(post_type_id.eq(1)))).count_distinct();
    let lh = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain(
        db.post
            .with(post_type_id.eq(1))
            .select(origid.select(&uidx).select(Ident::<User>::new().and(&ud).and((&qa).opt())).and((&lh).opt()).and(accepted_answer.opt())),
    );
    rows(v.into_iter().map(|(p, ((((u, a), q), h), x))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(q.unwrap_or(0))]);
        f.push(V::T(h.unwrap_or(creation_date.get(p).unwrap())));
        f.push(V::S(if x.is_some() { "Accepted" } else { "Not Accepted" }));
        f.push(V::S("Complete Data"));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '3 MONTH'),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(b.Class), 0) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ur.DisplayName, ur.Reputation, ur.TotalBadges FROM RecentPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.rn = 1),
// TopPosts AS (SELECT ps.*, DENSE_RANK() OVER (ORDER BY ps.Score DESC) AS ScoreRank FROM PostStatistics ps WHERE ps.Reputation > 1000)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.DisplayName, tp.Reputation, tp.TotalBadges, COALESCE(c.CommentCount, 0) AS TotalComments, COALESCE(v.UpVotes, 0) AS TotalUpVotes,
//        COALESCE(v.DownVotes, 0) AS TotalDownVotes, CASE WHEN tp.ScoreRank <= 10 THEN 'Top 10' WHEN tp.ScoreRank <= 50 THEN 'Top 50' ELSE 'Other' END AS RankCategory
// FROM TopPosts tp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON tp.PostId = c.PostId
// LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON tp.PostId = v.PostId
// WHERE tp.Reputation IS NOT NULL AND tp.ViewCount IS NOT NULL ORDER BY RankCategory, tp.Score DESC;
//
// A tie on CreationDate inside rn goes to the smaller post id.
fn q30509(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(current_date(), -3))).with(owner_user).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + c.unwrap_or(0));
    let ps: MatSet<Id<Post>> = (&first).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).collect();
    let sr = rel(ranked(drain((&ps).select(score)), |&(_, s)| Reverse(s), true).into_iter().map(|((p, _), r)| (p, r)).collect());
    let sri: HashIdx<Id<Post>, (Id<Post>, i64)> = (&sr).map(|(p, _)| p).inv().select(&sr).collect();
    let cc = (&ps).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&ps).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&ps).with(view_count).select((&sri).map(|(_, r)| r).and(&cc).and(&vc).and(owner_user.select(&tb))));
    let cat = |r: i64| if r <= 10 { "Top 10" } else if r <= 50 { "Top 50" } else { "Other" };
    let mut v = v;
    v.sort_by_key(|&(p, (((r, _), _), _))| (cat(r), Reverse(score.get(p).unwrap()), p));
    rows(v.into_iter().map(|(p, (((r, c), a), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner", "rep"]);
        f.extend([V::I(b), V::I(c), V::I(a[0]), V::I(a[1]), V::S(cat(r))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpvoteCount,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS OwnerPostRank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND P.Score > 0
//     GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName, P.OwnerUserId),
// EnhancedHistory AS (SELECT PH.Id AS HistoryId, PH.PostId, PH.CreationDate, PH.Comment, P.Title AS PostTitle, P.OwnerUserId, PH.PostHistoryTypeId,
//        COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = PH.PostId), 0) AS CommentCount
//     FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' AND PH.PostHistoryTypeId IN (10, 11)),
// FinalReport AS (SELECT R.OwnerDisplayName, R.Title AS PostTitle, R.Score AS PostScore, R.ViewCount, E.HistoryId, E.Comment, E.PostHistoryTypeId, R.OwnerPostRank
//     FROM RankedPosts R FULL OUTER JOIN EnhancedHistory E ON R.PostId = E.PostId WHERE R.OwnerPostRank <= 5 OR E.HistoryId IS NOT NULL)
// SELECT OwnerDisplayName, PostTitle, PostScore, ViewCount, HistoryId, Comment, CASE WHEN PostHistoryTypeId IS NULL THEN 'N/A' ELSE (SELECT Name FROM PostHistoryTypes WHERE Id = PostHistoryTypeId) END AS ChangeType
// FROM FinalReport ORDER BY PostScore DESC, OwnerDisplayName;
//
// The vote counts and CommentCount are never read. The FULL OUTER JOIN is the LEFT JOIN of RankedPosts to the history rows, unioned in prela with the history
// rows whose post is not in RankedPosts. The ownerless posts rank as one partition; a tie on CreationDate goes to the smaller post id.
fn q302(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(t0, -1)).and(score.gt(0))).select(owner_user.opt()));
    let rk = per_group(ranked(v, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let rk = rel(rk.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rset: MatSet<Id<Post>> = (&rk).map(|(p, _)| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let eh: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(hd.gt(add_months(t0, -6)).and(post_history_type_id.is_in([10, 11]))).select(post).inv().collect();
    type F = (Option<Id<Post>>, Option<Id<PostHistory>>);
    let left = rel(
        drain((&rk).select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(p, _): (Id<Post>, i64)| p).select((&eh).opt()))).filt(|((_, r), h): ((Id<Post>, i64), Option<Id<PostHistory>>)| r <= 5 || h.is_some()))
            .into_iter()
            .map(|(_, ((p, _), h))| -> F { (Some(p), h) })
            .collect(),
    );
    let right = rel(drain(db.post_history.with(hd.gt(add_months(t0, -6)).and(post_history_type_id.is_in([10, 11]))).with(post.select(Ident::<Post>::new().minus(&rset)))).into_iter().map(|(h, _)| -> F { (None, Some(h)) }).collect());
    let mut v = drain((&left).union(&right));
    let name = |p: Option<Id<Post>>| p.and_then(|p| owner_user.get(p)).map(|u| db.user.display_name.get(u).unwrap());
    v.sort_by_key(|&(_, (p, h))| (p.map_or(true, |_| false), Reverse(p.map(|p| score.get(p).unwrap())), name(p), p, h));
    rows(v.into_iter().map(|(_, (p, h))| {
        let mut f = match p {
            Some(p) => post_fields(db, p, &["owner", "title", "score", "views"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        };
        match h {
            Some(h) => f.extend([V::I(db.post_history.origid.get(h).unwrap()), ostr(db.post_history.comment.get(h)), V::S(htype_name(db).get(h).unwrap())]),
            None => f.extend([V::Null, V::Null, V::S("N/A")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.ViewCount, p.Score, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.Score IS NOT NULL AND p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year'),
// EligibleBadges AS (SELECT UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges GROUP BY UserId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.ViewCount, rp.Score, eb.BadgeCount FROM RankedPosts rp
//     LEFT JOIN EligibleBadges eb ON rp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = eb.UserId) WHERE rp.RankScore <= 5),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate AS CloseDate, c.Name AS CloseReason FROM PostHistory ph JOIN CloseReasonTypes c ON ph.Comment::int = c.Id WHERE ph.PostHistoryTypeId = 10),
// Analysis AS (SELECT fp.PostId, fp.Title, fp.OwnerDisplayName, fp.CreationDate, fp.ViewCount, fp.Score, fp.BadgeCount, cp.CloseDate, cp.CloseReason FROM FilteredPosts fp LEFT JOIN ClosedPosts cp ON fp.PostId = cp.PostId)
// SELECT *, COALESCE(CAST(CASE WHEN BadgeCount > 10 THEN 'Expert' WHEN BadgeCount BETWEEN 5 AND 10 THEN 'Intermediate' ELSE 'Beginner' END AS VARCHAR), 'No Badges') AS UserExpertise,
//        CASE WHEN CloseReason IS NOT NULL THEN 'Yes' ELSE 'No' END AS IsClosed
// FROM Analysis WHERE Score > 10 ORDER BY CreationDate DESC;
//
// The badge join matches every badge holder whose display name is the owner's, so a shared name repeats the post once per such user. A NULL BadgeCount
// falls through the CASE to 'Beginner', so 'No Badges' never appears.
fn q23456(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1))).with(owner_user).select(post_type_id));
    let rp = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |&(_, t)| t);
    let rp: MatSet<Id<Post>> = rel(rp.into_iter().filter(|x| x.1 <= 5).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let eb = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).with(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)).select(post).inv().collect();
    let v = drain((&rp).with(score.gt(10)).select(owner_user.select(&db.user.display_name).select(&by_name).select(&eb).opt().and((&cp).select(Ident::<PostHistory>::new().and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))).opt())));
    let mut v = v;
    v.sort_by_key(|&(p, _)| (Reverse(creation_date.get(p).unwrap()), p));
    rows(v.into_iter().map(|(p, (b, c))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "score"]);
        f.push(oint(b));
        match c {
            Some((h, r)) => f.extend([V::T(db.post_history.creation_date.get(h).unwrap()), V::S(r)]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(V::S(match b {
            Some(b) if b > 10 => "Expert",
            Some(b) if b >= 5 => "Intermediate",
            _ => "Beginner",
        }));
        f.push(V::S(if c.is_some() { "Yes" } else { "No" }));
        row(f)
    }))
}

// Rewritten (rewrites/20757.sql): the final ORDER BY is refined with `, U.Id, CP.CreationDate`.
// WITH UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS TotalBadges, STRING_AGG(B.Name, ', ') AS BadgeNames FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.Reputation > 1000 GROUP BY U.Id),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END) AS TotalAnswers, AVG(P.ViewCount) AS AvgViews,
//        AVG(P.Score) AS AvgScore FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, STRING_AGG(CASE WHEN C.PostId IS NOT NULL THEN 'Closed' ELSE 'Active' END, ' | ') AS PostStatus
//     FROM PostHistory PH LEFT JOIN Comments C ON PH.PostId = C.PostId WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId, PH.CreationDate),
// UserPostBadgeStats AS (SELECT U.Id AS UserId, UB.TotalBadges, PS.TotalPosts, PS.TotalAnswers, PS.AvgViews, PS.AvgScore, CP.PostStatus FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId
//     LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId LEFT JOIN ClosedPosts CP ON PS.OwnerUserId = CP.PostId WHERE U.Location IS NOT NULL OR U.AboutMe IS NOT NULL)
// SELECT U.DisplayName, COALESCE(UB.TotalBadges, 0) AS BadgeCount, COALESCE(PS.TotalPosts, 0) AS PostsCount, COALESCE(PS.TotalAnswers, 0) AS AnswersCount, COALESCE(PS.AvgViews, 0) AS AverageViews,
//        COALESCE(PS.AvgScore, 0) AS AverageScore, CASE WHEN CP.PostStatus LIKE '%Closed%' THEN 'Discontinued Activity' ELSE 'Active Participation' END AS UserActivityStatus
// FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId LEFT JOIN ClosedPosts CP ON PS.OwnerUserId = CP.PostId
// WHERE U.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 years' ORDER BY U.Reputation DESC, U.Id, CP.CreationDate LIMIT 100;
//
// UserPostBadgeStats is never read. PostStatus contains 'Closed' exactly when the post has a comment, whatever order STRING_AGG uses. `PS.OwnerUserId =
// CP.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q20757(db: &'static So) -> String {
    let User { reputation, creation_date: uc, origid, .. } = &db.user;
    let ub = db.user.with(reputation.gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { creation_date, owner_user, post_type_id, answer_count, view_count, score, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(owner_user)
        .select(post_type_id.and(answer_count.opt()).and(view_count.opt()).and(score))
        .fold([0i64; 5], |a, (((t, n), w), s)| [a[0] + 1, a[1] + if t == 1 { n.unwrap_or(0) } else { 0 }, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post.and(hd)).select(post.select(comments_of(db)).opt()).fold(false, |c, x| c || x.is_some());
    let cv = rel(drain(&cp));
    let cpi: HashIdx<Id<Post>, ((Id<Post>, i64), bool)> = (&cv).map(|((p, _), _)| p).inv().select(&cv).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let cj = Ident::<User>::new().with(&ps).select(origid).select(&pidx).select(&cpi).opt();
    let v = drain(db.user.with(uc.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -2))).select((&ub).opt().and((&ps).opt()).and(cj)));
    let v = top_k(v, |&(u, (_, c))| (Reverse(reputation.get(u).unwrap()), u, c.map_or(true, |_| false), c.map(|((_, d), _)| d)), |_| 0, 100);
    rows(v.into_iter().map(|(u, ((b, p), c))| {
        let a = p.unwrap_or([0; 5]);
        let fz = |s: i64, n: i64| if n == 0 { V::F(0.0) } else { avg(s, n) };
        row(vec![
            user_col(db, u, "name"),
            V::I(b.unwrap_or(0)),
            V::I(a[0]),
            V::I(a[1]),
            fz(a[3], a[2]),
            fz(a[4], a[0]),
            V::S(if c.map_or(false, |(_, x)| x) { "Discontinued Activity" } else { "Active Participation" }),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostActivity AS (SELECT ph.PostId, COUNT(DISTINCT ph.UserId) AS EditCount, MAX(ph.CreationDate) AS LastEditDate, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount FROM PostHistory ph GROUP BY ph.PostId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, us.TotalBounties, us.TotalUpVotes, us.TotalDownVotes, pa.EditCount, pa.LastEditDate, pa.CloseCount
//     FROM RankedPosts rp JOIN UserStats us ON us.UserId = rp.OwnerUserId JOIN PostActivity pa ON pa.PostId = rp.PostId WHERE rp.ScoreRank = 1 AND (us.TotalBounties > 0 OR us.TotalUpVotes > us.TotalDownVotes))
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.AnswerCount, fp.TotalBounties, fp.TotalUpVotes, fp.TotalDownVotes, CASE WHEN fp.CloseCount > 0 THEN 'Closed' ELSE 'Open' END AS Status,
//        CONCAT(CAST(fp.LastEditDate AS TEXT), ' ', COALESCE(fp.EditCount, 0)::TEXT, ' edits') AS EditInfo
// FROM FilteredPosts fp ORDER BY fp.Score DESC, fp.CreationDate DESC LIMIT 100;
fn q20653(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).fold([0i64; 3], |a, x| match x {
        Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, user, .. } = &db.post_history;
    let pa = db.post_history.group_by(post).select(hd.and(post_history_type_id)).fold((i64::MIN, 0i64), |(m, c), (d, t)| (m.max(d), c + (t == 10) as i64));
    let ed = db.post_history.group_by(post).select(user).count_distinct();
    let v = drain((&tp).select(owner_user.select((&us).filt(|a: [i64; 3]| a[0] > 0 || a[1] > a[2])).and(&pa).and((&ed).opt())));
    let v = top_k(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), |&(p, _)| p, 100);
    rows(v.into_iter().map(|(p, ((a, (m, c)), e))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend(a.map(V::I));
        f.push(V::S(if c > 0 { "Closed" } else { "Open" }));
        f.push(V::Owned(format!("{} {} edits", ts_text(m), e.unwrap_or(0))));
        row(f)
    }))
}

// WITH LatestUserBadges AS (SELECT b.UserId, b.Name AS BadgeName, b.Class, ROW_NUMBER() OVER (PARTITION BY b.UserId ORDER BY b.Date DESC) AS BadgeRank FROM Badges b WHERE b.Class = 1),
// ActiveUserPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.PostTypeId, p.CreationDate, EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - p.CreationDate)) / 3600 AS AgeInHours, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' GROUP BY p.Id, p.OwnerUserId, p.PostTypeId, p.CreationDate),
// UserPostMetrics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT ap.PostId) AS TotalPosts, COALESCE(SUM(CASE WHEN ap.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions,
//        COALESCE(SUM(CASE WHEN ap.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers, AVG(ap.AgeInHours) AS AvgPostAge, MAX(ap.CommentCount) AS MaxCommentsPerPost
//     FROM Users u LEFT JOIN ActiveUserPosts ap ON u.Id = ap.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostsRanked AS (SELECT upm.UserId, upm.DisplayName, upm.TotalPosts, upm.TotalQuestions, upm.TotalAnswers, upm.AvgPostAge, upm.MaxCommentsPerPost, lb.BadgeName,
//        ROW_NUMBER() OVER (ORDER BY upm.TotalPosts DESC, upm.MaxCommentsPerPost DESC) AS UserRank FROM UserPostMetrics upm LEFT JOIN LatestUserBadges lb ON upm.UserId = lb.UserId AND lb.BadgeRank = 1)
// SELECT pr.UserId, pr.DisplayName, pr.TotalPosts, pr.TotalQuestions, pr.TotalAnswers, pr.AvgPostAge, pr.MaxCommentsPerPost, COALESCE(pr.BadgeName, 'No Gold Badge') AS TopBadge,
//        CASE WHEN pr.AvgPostAge < 24 THEN 'New Posters' WHEN pr.AvgPostAge BETWEEN 24 AND 720 THEN 'Regular Posters' ELSE 'Old Posters' END AS PosterCategory
// FROM PostsRanked pr WHERE pr.UserRank <= 10 ORDER BY pr.TotalPosts DESC, pr.MaxCommentsPerPost DESC;
//
// A tie in BadgeRank goes to the larger badge id, and in UserRank to the smaller user id.
fn q23451(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let ap = db.post.with(creation_date.ge(add_months(t0, -6))).with(owner_user).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let upm = db
        .post
        .with(&ap)
        .group_by(owner_user)
        .select(post_type_id.and(creation_date).and(&ap))
        .fold(([0i64, 0, 0, i64::MIN], 0.0f64), |(a, s), ((t, d), c)| ([a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(c)], s + (t0 - d) as f64 / 1e6 / 3600.0));
    let top = top_k(drain(&upm), |&(_, (a, _))| (Reverse(a[0]), Reverse(a[3])), |&(u, _)| u, 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Badge { user, date, class, name, .. } = &db.badge;
    let lb = top_per(drain(db.badge.with(class.eq(1)).select(user)), |&(_, u)| u, |&(b, _)| (Reverse(date.get(b).unwrap()), Reverse(b)), 1, false);
    let lb = rel(lb.into_iter().map(|(b, u)| (u, b)).collect());
    let lbi: HashIdx<Id<User>, (Id<User>, Id<Badge>)> = (&lb).map(|(u, _)| u).inv().select(&lb).collect();
    let v = drain((&tu).select((&upm).and((&lbi).map(|(_, b)| b).select(name).opt())));
    let v = top_k(v, |&(_, ((a, _), _))| (Reverse(a[0]), Reverse(a[3])), |&(u, _)| u, 0);
    rows(v.into_iter().map(|(u, ((a, s), b))| {
        let age = s / a[0] as f64;
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(age), V::I(a[3]), V::S(b.unwrap_or("No Gold Badge"))]);
        f.push(V::S(if age < 24.0 { "New Posters" } else if age <= 720.0 { "Regular Posters" } else { "Old Posters" }));
        row(f)
    }))
}

// WITH TagCount AS (SELECT TRIM(tag) AS TagName, COUNT(*) AS PostCount FROM (SELECT UNNEST(string_to_array(SUBSTRING(Tags FROM 2 FOR LENGTH(Tags) - 2), '><')) AS tag FROM Posts WHERE PostTypeId = 1) AS extracted_tags
//     GROUP BY TRIM(tag)),
// TopTags AS (SELECT TagName, PostCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM TagCount WHERE PostCount > 1),
// MostActiveUsers AS (SELECT Users.DisplayName, Users.Reputation, COUNT(Posts.Id) AS QuestionsAnswered, SUM(COALESCE(Posts.AnswerCount, 0)) AS TotalAnswers, SUM(COALESCE(Posts.Score, 0)) AS TotalScore
//     FROM Users JOIN Posts ON Users.Id = Posts.OwnerUserId WHERE Posts.PostTypeId = 2 GROUP BY Users.DisplayName, Users.Reputation),
// TagUsage AS (SELECT Posts.Id AS PostId, Posts.Title, Posts.CreationDate, UNNEST(string_to_array(SUBSTRING(Posts.Tags FROM 2 FOR LENGTH(Posts.Tags) - 2), '><')) AS TagName, Users.DisplayName AS Owner
//     FROM Posts JOIN Users ON Posts.OwnerUserId = Users.Id WHERE Posts.PostTypeId = 1)
// SELECT TopTags.TagName, TopTags.PostCount, MostActiveUsers.DisplayName, MostActiveUsers.Reputation, MostActiveUsers.QuestionsAnswered, MostActiveUsers.TotalAnswers, MostActiveUsers.TotalScore,
//        COUNT(TagUsage.PostId) AS TagPostCount, MIN(TagUsage.CreationDate) AS EarliestPostDate, MAX(TagUsage.CreationDate) AS LatestPostDate
// FROM TopTags JOIN TagUsage ON TopTags.TagName = TagUsage.TagName JOIN MostActiveUsers ON TagUsage.Owner = MostActiveUsers.DisplayName
// GROUP BY TopTags.TagName, TopTags.PostCount, MostActiveUsers.DisplayName, MostActiveUsers.Reputation, MostActiveUsers.QuestionsAnswered, MostActiveUsers.TotalAnswers, MostActiveUsers.TotalScore
// ORDER BY TopTags.PostCount DESC, MostActiveUsers.TotalScore DESC LIMIT 10;
//
// Rank is never read. The joined (question, tag, answerer group) rows are materialised and grouped, since the GROUP BY key is not a function of the question.
fn q28524(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user, creation_date, answer_count, score, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let tc = qs().select(tags_str.flat_map(tag_list).map(|t: Str| t.trim())).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let top: MatSet<Str> = (&tc).filt(|n| n > 1).map(|_| ()).inv().collect();
    let User { display_name, reputation, .. } = &db.user;
    let mau = db.post.with(post_type_id.eq(2)).with(owner_user).group_by(owner_user.select(display_name.and(reputation))).select(answer_count.opt().and(score)).fold([0i64; 3], |a, (n, s)| [a[0] + 1, a[1] + n.unwrap_or(0), a[2] + s]);
    let mv = rel(drain(&mau));
    let by_name: HashIdx<Str, ((Str, i64), [i64; 3])> = (&mv).map(|((n, _), _)| n).inv().select(&mv).collect();
    let rows_: MatSet<(Id<Post>, Str, (Str, i64))> = qs()
        .with(owner_user)
        .select(Ident::<Post>::new().and(tags_str.flat_map(tag_list).select(Same::<Str>::new().with(&top))).and(owner_user.select(display_name).select(&by_name)))
        .map(|((p, t), (k, _))| (p, t, k))
        .collect();
    type X = (Id<Post>, Str, (Str, i64));
    let g = (&rows_).group_by(Same::<X>::new().map(|(_, t, k): X| (t, k))).select(Same::<X>::new().map(|(p, _, _): X| p).select(creation_date)).fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), d| (n + 1, lo.min(d), hi.max(d)));
    let v = drain((&g).and(Same::<(Str, (Str, i64))>::new().map(|(t, _): (Str, (Str, i64))| t).select(&tc)).and(Same::<(Str, (Str, i64))>::new().map(|(_, k): (Str, (Str, i64))| k).select(&mau)));
    let v = top_k(v, |&(_, ((_, c), a))| (Reverse(c), Reverse(a[2])), |&((t, k), _)| (t, k), 10);
    rows(v.into_iter().map(|((t, (n, r)), (((k, lo, hi), c), a))| row(vec![V::S(t), V::I(c), V::S(n), V::I(r), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(k), V::T(lo), V::T(hi)])))
}

// WITH RecursivePostHistory AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, ph.CreationDate AS HistoryCreationDate, ph.PostHistoryTypeId, ph.UserId,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS rn FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId),
// ActiveUserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id),
// TopQuestionsWithAnswerInfo AS (SELECT p.Id AS QuestionId, p.Title, p.AcceptedAnswerId, COALESCE(a.Score, 0) AS AcceptedAnswerScore, COALESCE(a.ViewCount, 0) AS AcceptedAnswerViewCount
//     FROM Posts p LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id WHERE p.PostTypeId = 1),
// RecentPostActivity AS (SELECT p.Id AS PostId, p.Title, MAX(c.CreationDate) AS LastCommentDate, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title)
// SELECT p.Id AS PostId, p.Title, CASE WHEN a.UserId IS NOT NULL THEN 'Badge Holder' ELSE 'Regular User' END AS UserType, COALESCE(q.AcceptedAnswerScore, 0) AS AcceptedAnswerScore,
//        COALESCE(q.AcceptedAnswerViewCount, 0) AS AcceptedAnswerViewCount, ph.HistoryCreationDate, a.BadgeCount, ra.CommentCount, ra.LastCommentDate
// FROM Posts p LEFT JOIN ActiveUserBadgeCounts a ON p.OwnerUserId = a.UserId LEFT JOIN TopQuestionsWithAnswerInfo q ON p.Id = q.QuestionId LEFT JOIN RecursivePostHistory ph ON p.Id = ph.PostId AND ph.rn = 1
// LEFT JOIN RecentPostActivity ra ON p.Id = ra.PostId WHERE p.PostTypeId = 1 AND p.ViewCount > 100 ORDER BY ra.CommentCount DESC, q.AcceptedAnswerScore DESC, p.CreationDate DESC LIMIT 100;
//
// Not recursive despite the name. rn = 1 is a latest history row, and HistoryCreationDate is its date whichever tied row it is.
fn q30261(db: &'static So) -> String {
    let Post { post_type_id, view_count, owner_user, accepted_answer, creation_date, score, .. } = &db.post;
    let ab = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let lh = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let ra = db
        .post
        .with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select(&db.comment.creation_date).opt())
        .fold((0i64, i64::MIN), |(n, m), d| match d {
            Some(d) => (n + 1, m.max(d)),
            None => (n, m),
        });
    let v = drain(db.post.with(post_type_id.eq(1).and(view_count.gt(100))).select(owner_user.select(&ab).opt().and(accepted_answer.select(score.and(view_count.opt())).opt()).and((&lh).opt()).and((&ra).opt())));
    let v = top_k(v, |&(p, (((_, q), _), r))| (r.map_or(true, |_| false), Reverse(r.map(|r| r.0)), Reverse(q.map_or(0, |q| q.0)), Reverse(creation_date.get(p).unwrap())), |&(p, _)| p, 100);
    rows(v.into_iter().map(|(p, (((a, q), h), r))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(if a.is_some() { "Badge Holder" } else { "Regular User" }));
        f.extend([V::I(q.map_or(0, |q| q.0)), V::I(q.and_then(|q| q.1).unwrap_or(0)), ots(h), oint(a)]);
        f.extend(match r {
            Some((n, m)) => [V::I(n), tmax(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank,
//        ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RecentRowNum
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// UserEngagement AS (SELECT u.Id AS UserId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// FilteredEngagement AS (SELECT ue.UserId, ue.Upvotes, ue.Downvotes, ue.GoldBadges, ue.SilverBadges, (ue.Upvotes - ue.Downvotes) AS NetEngagement,
//        CASE WHEN ue.Upvotes > 50 THEN 'High Engagement' WHEN ue.Upvotes BETWEEN 21 AND 50 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
//     FROM UserEngagement ue WHERE ue.Upvotes IS NOT NULL OR ue.Downvotes IS NOT NULL)
// SELECT p.PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.CommentCount, fe.Upvotes, fe.Downvotes, fe.NetEngagement, fe.EngagementLevel
// FROM RankedPosts p LEFT JOIN FilteredEngagement fe ON p.PostId = (SELECT p2.Id FROM Posts p2 WHERE p2.OwnerUserId = fe.UserId ORDER BY p2.Score DESC LIMIT 1)
// WHERE p.ScoreRank <= 5 AND p.RecentRowNum <= 100 ORDER BY p.Score DESC, fe.NetEngagement DESC;
//
// The correlated `ORDER BY Score DESC LIMIT 1` is an arg-max fold per user; a tie on Score goes to the smaller post id. The users x votes x badges product is
// driven only for the users whose top post survives the rank filters. A RecentRowNum tie goes to the smaller post id.
fn q20911(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let sr = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |&(_, t)| t);
    let rn = ranked(sr, |&((p, _), _)| (Reverse(creation_date.get(p).unwrap()), p), false);
    let rp = rel(rn.into_iter().map(|(((p, _), s), r)| (p, s, r)).collect());
    let keep: MatSet<Id<Post>> = (&rp).filt(|(_, s, r): (Id<Post>, i64, i64)| s <= 5 && r <= 100).map(|(p, _, _)| p).collect();
    let best = db.post.with(owner_user).group_by(owner_user).select(score.and(Ident::<Post>::new())).fold((i64::MIN, None::<Id<Post>>), |(s, b), (x, p)| {
        if x > s || (x == s && b.map_or(true, |b| p < b)) { (x, Some(p)) } else { (s, b) }
    });
    let top_of: HashIdx<Id<Post>, Id<User>> = (&best).flat_map(|(_, b): (i64, Option<Id<Post>>)| b).inv().collect();
    let hit: MatSet<Id<User>> = (&keep).select(&top_of).collect();
    let ue = (&hit)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&keep).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain((&cc).and((&top_of).select(&ue).opt()));
    v.sort_by_key(|&(p, (_, e))| (Reverse(score.get(p).unwrap()), e.is_none(), Reverse(e.map(|a| a[0] - a[1])), p));
    rows(v.into_iter().map(|(p, (c, e))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(c));
        f.extend(match e {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::S(if a[0] > 50 { "High Engagement" } else if a[0] >= 21 { "Moderate Engagement" } else { "Low Engagement" })],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecursivePostStats AS (SELECT P.Id AS PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, MAX(COALESCE(P.LastActivityDate, P.CreationDate)) AS LastActiveDate
//     FROM Posts P LEFT JOIN Votes V ON V.PostId = P.Id LEFT JOIN Comments C ON C.PostId = P.Id GROUP BY P.Id),
// PopularPosts AS (SELECT PS.PostId, PS.UpVotes, PS.DownVotes, PS.CommentCount, PS.LastActiveDate, PS.UpVotes - PS.DownVotes AS NetVotes, RANK() OVER (ORDER BY PS.UpVotes DESC, PS.CommentCount DESC) AS PopularityRank
//     FROM RecursivePostStats PS WHERE PS.UpVotes > 0),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, SUM(COALESCE(B.Class, 0)) AS TotalBadgeScore, RANK() OVER (ORDER BY SUM(COALESCE(B.Class, 0)) DESC) AS UserRank
//     FROM Users U LEFT JOIN Badges B ON B.UserId = U.Id GROUP BY U.Id, U.DisplayName),
// FinalReport AS (SELECT PP.PostId, PP.UpVotes, PP.DownVotes, PP.CommentCount, PP.NetVotes, PP.PopularityRank, TU.DisplayName AS TopUser, TU.TotalBadgeScore, PP.LastActiveDate
//     FROM PopularPosts PP JOIN TopUsers TU ON PP.PopularityRank = TU.UserRank)
// SELECT FR.PostId, FR.UpVotes, FR.DownVotes, FR.CommentCount, FR.NetVotes, FR.PopularityRank, COALESCE(FR.TopUser, 'No Users') AS TopUser, COALESCE(FR.TotalBadgeScore, 0) AS TotalBadgeScore,
//        CASE WHEN FR.LastActiveDate < (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') THEN 'Inactive'
//             WHEN FR.LastActiveDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') AND FR.LastActiveDate < (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month') THEN 'Somewhat Active' ELSE 'Active' END AS ActivityStatus
// FROM FinalReport FR ORDER BY FR.PopularityRank;
//
// Not recursive despite the name. The two ranks are joined by value, through an index on UserRank.
fn q31432(db: &'static So) -> String {
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let pp = ranked(drain((&ps).filt(|a: [i64; 3]| a[0] > 0)), |&(_, a)| (Reverse(a[0]), Reverse(a[2])), false);
    let tu = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + c.unwrap_or(0));
    let ur = rel(ranked(drain(&tu), |&(_, s)| Reverse(s), false).into_iter().map(|((u, s), r)| (r, (u, s))).collect());
    let by_rank: HashIdx<i64, (i64, (Id<User>, i64))> = (&ur).map(|(r, _)| r).inv().select(&ur).collect();
    let ppr = rel(pp.into_iter().map(|((p, a), r)| (p, a, r)).collect());
    type P = (Id<Post>, [i64; 3], i64);
    let v = drain((&ppr).select(Same::<P>::new().and(Same::<P>::new().map(|(_, _, r): P| r).select(&by_rank))));
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    rows(v.into_iter().map(|(_, ((p, a, r), (_, (u, s))))| {
        let d = db.post.last_activity_date.get(p).unwrap();
        let mut f = post_fields(db, p, &["id"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[0] - a[1]), V::I(r), user_col(db, u, "name"), V::I(s)]);
        f.push(V::S(if d < add_years(t0, -1) { "Inactive" } else if d < add_months(t0, -1) { "Somewhat Active" } else { "Active" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.AcceptedAnswerId, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank,
//        COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.AcceptedAnswerId, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.AcceptedAnswerId, rp.OwnerUserId, rp.UserRank, rp.CommentCount, COALESCE(rp.UpVotes - rp.DownVotes, 0) AS NetVotes
//     FROM RankedPosts rp WHERE rp.UserRank <= 3),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT tp.PostId, tp.Title, tp.Score, tp.CreationDate, u.DisplayName AS OwnerDisplayName, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, tp.CommentCount, tp.NetVotes,
//        CASE WHEN tp.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Not Accepted' END AS AnswerStatus,
//        CASE WHEN EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = tp.PostId AND v.VoteTypeId = 6) THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM TopPosts tp JOIN Users u ON tp.OwnerUserId = u.Id JOIN UserBadges ub ON u.Id = ub.UserId WHERE (ub.GoldBadges + ub.SilverBadges + ub.BronzeBadges) > 0
// ORDER BY tp.NetVotes DESC, tp.CreationDate ASC LIMIT 10;
//
// UserRank reads only Score, so each owner's top three are picked first and the comment x vote product is driven for those alone.
fn q24897(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, accepted_answer_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 3, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let closed: MatSet<Id<Post>> = db.vote.with((&db.vote.vote_type_id).eq(6)).select(&db.vote.post).collect();
    let v = drain((&rp).and(owner_user.select(&ub)).and(Ident::<Post>::new().with(&closed).opt()));
    let v = top_k(v, |&(p, ((a, _), _))| (Reverse(a[1] - a[2]), creation_date.get(p).unwrap()), |&(p, _)| p, 10);
    rows(v.into_iter().map(|(p, ((a, b), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1] - a[2])]);
        f.push(V::S(if accepted_answer_id.get(p).is_some() { "Accepted" } else { "Not Accepted" }));
        f.push(V::S(if c.is_some() { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        SUM(CASE WHEN P.LastActivityDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' THEN 1 ELSE 0 END) AS RecentPosts FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// PostVoteStats AS (SELECT P.Id AS PostId, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id),
// RecentPostComments AS (SELECT C.PostId, COUNT(C.Id) AS TotalComments FROM Comments C WHERE C.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 week' GROUP BY C.PostId),
// PostHistoryInfo AS (SELECT PH.PostId, PH.UserDisplayName, PH.CreationDate, PH.Comment FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11, 12)),
// RankedUsers AS (SELECT UPS.UserId, UPS.DisplayName, UPS.TotalPosts, UPS.TotalAnswers, UPS.AcceptedAnswers, UPS.RecentPosts, PVS.TotalVotes, PVS.UpVotes, PVS.DownVotes,
//        RANK() OVER (ORDER BY UPS.TotalPosts DESC) AS PostRank FROM UserPostStats UPS JOIN PostVoteStats PVS ON UPS.UserId = PVS.PostId)
// SELECT RU.UserId, RU.DisplayName, RU.TotalPosts, RU.TotalAnswers, RU.AcceptedAnswers, RU.RecentPosts, COALESCE(RC.TotalComments, 0) AS CommentsInLastWeek,
//        COALESCE(PH.UserDisplayName, 'No History') AS LastActionUser, PH.CreationDate AS LastActionDate, PH.Comment AS LastActionComment
// FROM RankedUsers RU LEFT JOIN RecentPostComments RC ON RU.UserId = RC.PostId LEFT JOIN PostHistoryInfo PH ON RU.UserId = PH.PostId WHERE RU.RecentPosts > 0 ORDER BY RU.PostRank, RU.UserId;
//
// Every join here matches a user id to a post id, so each goes through the raw ids; PostVoteStats has a row for every post, so the inner join keeps the users
// whose id is some post's id. The vote counts are never read.
fn q2575(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, last_activity_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(last_activity_date)).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((t, x), d)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1 && x.is_some()) as i64, a[3] + (d > add_months(t0, -1)) as i64],
            None => a,
        });
    let rc = db.comment.with((&db.comment.creation_date).gt(add_days(t0, -7))).group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let ph = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11, 12])));
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let v = drain(db.user.select((&ups).filt(|a: [i64; 4]| a[3] > 0).and((&db.user.origid).select(&pidx).select((&rc).opt().and(ph.opt())))));
    rows(v.into_iter().map(|(u, (a, (c, h)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::I(c.unwrap_or(0)));
        f.extend(match h {
            Some(h) => [V::S(db.post_history.user_display_name.get(h).unwrap_or("No History")), V::T(db.post_history.creation_date.get(h).unwrap()), ostr(db.post_history.comment.get(h))],
            None => [V::S("No History"), V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END)) AS TotalUpvotes,
//        SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END)) AS TotalDownvotes, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName),
// RecentActivities AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS RecentPosts, COUNT(C.Id) AS RecentComments, COUNT(DISTINCT H.Id) AS PostEdits
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' LEFT JOIN Comments C ON P.Id = C.PostId AND C.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'
//     LEFT JOIN PostHistory H ON P.Id = H.PostId AND H.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName),
// RankedUserEngagement AS (SELECT UE.UserId, UE.DisplayName, UE.TotalViews, UE.TotalUpvotes, UE.TotalDownvotes, UE.TotalPosts, UE.TotalComments, R.RecentPosts, R.RecentComments, R.PostEdits,
//        RANK() OVER (ORDER BY UE.TotalViews DESC) AS ViewRank, RANK() OVER (ORDER BY UE.TotalUpvotes DESC) AS UpvoteRank FROM UserEngagement UE LEFT JOIN RecentActivities R ON UE.UserId = R.UserId)
// SELECT UserId, DisplayName, TotalViews, TotalUpvotes, TotalDownvotes, TotalPosts, TotalComments, RecentPosts, RecentComments, PostEdits,
//        CASE WHEN ViewRank <= 10 THEN 'Top Engaged Users' WHEN UpvoteRank <= 10 THEN 'Top Upvote Users' ELSE 'Regular Users' END AS UserCategory
// FROM RankedUserEngagement WHERE TotalComments > 10 ORDER BY TotalViews DESC, TotalUpvotes DESC FETCH FIRST 100 ROWS ONLY;
//
// The SUMs run over the posts x votes x comments product; the two COUNT(DISTINCT)s undo it and are one fold each. The recent side compares TIMESTAMPs with
// CURRENT_TIMESTAMP, so both are UTC instants.
fn q3436(db: &'static So) -> String {
    let Post { view_count, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(0)).group_by(Ident::<User>::new());
    let ue = users()
        .select(posts_of(db).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some(((w, t), _)) => [a[0] + w.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
            None => a,
        });
    let dp = users().select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let dc = users().select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let now = now_utc();
    let since = move |d: i64| ny_to_utc(d) >= now - 30 * DAY_US;
    let rp = || Ident::<Post>::new().with(creation_date.filt(since));
    let rcm = comments_of(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).filt(since)));
    let rh = || history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).filt(since)));
    let ra = users().select(posts_of(db).select(rp()).select(rcm.opt().and(rh().opt())).opt()).fold([0i64; 2], |a, p| match p {
        Some((c, _)) => [a[0] + 1, a[1] + c.is_some() as i64],
        None => a,
    });
    let re = users().select(posts_of(db).select(rp()).select(rh()).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let all = drain((&ue).and(&dp).and(&dc).and(&ra).and(&re));
    let vr = ranked(all, |&(_, ((((a, _), _), _), _))| Reverse(a[0]), false);
    let vr = ranked(vr, |&((_, ((((a, _), _), _), _)), _)| Reverse(a[1]), false);
    type X = (((([i64; 3], i64), i64), [i64; 2]), i64);
    type R = (((Id<User>, X), i64), i64);
    let kept = drain(rel(vr).filt(|(((_, ((((_, _), c), _), _)), _), _): R| c > 10)).into_iter().map(|x| x.1).collect::<Vec<R>>();
    let v = top_k(kept, |&(((_, ((((a, _), _), _), _)), _), _)| (Reverse(a[0]), Reverse(a[1])), |&(((u, _), _), _)| u, 100);
    rows(v.into_iter().map(|(((u, ((((a, p), c), r), e)), w), x)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(p), V::I(c), V::I(r[0]), V::I(r[1]), V::I(e)]);
        f.push(V::S(if w <= 10 { "Top Engaged Users" } else if x <= 10 { "Top Upvote Users" } else { "Regular Users" }));
        row(f)
    }))
}

// WITH TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U WHERE U.Reputation > 1000),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges B GROUP BY B.UserId),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.ViewCount, DENSE_RANK() OVER (ORDER BY P.ViewCount DESC) AS Rank FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '7 days' AND P.ViewCount IS NOT NULL),
// ClosedPosts AS (SELECT PH.PostId, COUNT(PH.Id) AS CloseCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId),
// JoinedData AS (SELECT U.DisplayName, U.Reputation, COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges, PP.PostId, PP.Title,
//        PP.ViewCount, C.CloseCount FROM TopUsers U LEFT JOIN UserBadges UB ON U.UserId = UB.UserId INNER JOIN PopularPosts PP ON U.UserId = PP.OwnerUserId LEFT JOIN ClosedPosts C ON PP.PostId = C.PostId)
// SELECT JD.DisplayName, JD.Reputation, JD.GoldBadges, JD.SilverBadges, JD.BronzeBadges, JD.Title, JD.ViewCount, JD.CloseCount, CASE WHEN JD.CloseCount IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus,
//        CASE WHEN JD.Reputation > 2000 THEN 'Elite User' WHEN JD.Reputation BETWEEN 1000 AND 2000 THEN 'Experienced User' ELSE 'Novice User' END AS UserCategory
// FROM JoinedData JD WHERE JD.ViewCount > 50 ORDER BY JD.ViewCount DESC, JD.Reputation DESC LIMIT 10;
//
// UserRank and Rank are never read.
fn q20889(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, .. } = &db.post;
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let tu = Ident::<User>::new().with((&db.user.reputation).gt(1000)).and((&ub).opt());
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -7)).and(view_count.gt(50))).select(owner_user.select(tu).and((&cp).opt())));
    let v = top_k(v, |&(p, ((u, _), _))| (Reverse(view_count.get(p)), Reverse(db.user.reputation.get(u).unwrap())), |&(p, _)| p, 10);
    rows(v.into_iter().map(|(p, ((u, b), c))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([oint(c), V::S(if c.is_some() { "Closed" } else { "Open" }), V::S(if r > 2000 { "Elite User" } else if r >= 1000 { "Experienced User" } else { "Novice User" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.CommentCount, CASE WHEN rp.Rank = 1 THEN 'Top' WHEN rp.Rank <= 5 THEN 'High' ELSE 'Low' END AS Popularity FROM RankedPosts rp),
// VoteData AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT ps.Title, ps.Score, ps.ViewCount, ps.CommentCount, COALESCE(vd.UpVotes, 0) AS UpVotes, COALESCE(vd.DownVotes, 0) AS DownVotes,
//        CASE WHEN ps.Popularity = 'Top' AND ps.CommentCount > 5 THEN 'Engaging' WHEN ps.Popularity = 'High' AND ps.Score > 100 THEN 'Highly Engaging' ELSE 'Needs Improvement' END AS EngagementLevel,
//        CASE WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = ps.PostId) AND b.Class = 1) THEN 'Gold Member'
//             WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = ps.PostId) AND b.Class = 2) THEN 'Silver Member' ELSE 'Regular User' END AS UserMembershipLevel
// FROM PostStats ps LEFT JOIN VoteData vd ON ps.PostId = vd.PostId WHERE ps.ViewCount > 10 ORDER BY ps.Score DESC, ps.CommentCount DESC LIMIT 100;
//
// A tie in Rank goes to the smaller post id.
fn q20680(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let rk = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), false), |&(_, t)| t);
    let rk = rel(rk.into_iter().map(|((p, _), r)| (p, r)).collect());
    let ri: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let ps = db.post.with(&ri).with(view_count.gt(10));
    let cc = ps.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vd = db.post.with(&ri).with(view_count.gt(10)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let has = |k: i64| -> MatSet<Id<User>> { db.badge.with((&db.badge.class).eq(k)).select(&db.badge.user).collect() };
    let (gold, silver) = (has(1), has(2));
    let v = drain((&cc).and(&vd).and((&ri).map(|(_, r)| r)).and(owner_user.select(Ident::<User>::new().with(&gold)).opt()).and(owner_user.select(Ident::<User>::new().with(&silver)).opt()));
    let v = top_k(v, |&(p, ((((c, _), _), _), _))| (Reverse(score.get(p).unwrap()), Reverse(c)), |&(p, _)| p, 100);
    rows(v.into_iter().map(|(p, ((((c, a), r), g), s))| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if r == 1 && c > 5 { "Engaging" } else if r > 1 && r <= 5 && sc > 100 { "Highly Engaging" } else { "Needs Improvement" }));
        f.push(V::S(if g.is_some() { "Gold Member" } else if s.is_some() { "Silver Member" } else { "Regular User" }));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
//        COUNT(v.Id) AS TotalVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostActivity AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AcceptedAnswerId, COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount, COALESCE(AVG(p.Score), 0) AS AverageScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.ViewCount, p.AcceptedAnswerId),
// RecentPostHistory AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS rn FROM PostHistory ph
//     WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// RankingPosts AS (SELECT pa.PostId, pa.Title, pa.ViewCount, pa.CommentCount, pa.AverageScore, DENSE_RANK() OVER (ORDER BY pa.ViewCount DESC, pa.AverageScore DESC) AS Rank FROM PostActivity pa WHERE pa.CommentCount > 5)
// SELECT u.DisplayName AS UserName, u.UpVotesCount, u.DownVotesCount, u.TotalVotes, r.Title, r.ViewCount, r.CommentCount, r.AverageScore, r.Rank AS PostRank,
//        CASE WHEN r.ViewCount > 1000 THEN 'Highly Viewed' WHEN r.ViewCount BETWEEN 500 AND 1000 THEN 'Moderately Viewed' ELSE 'Low Views' END AS ViewCategory,
//        CASE WHEN php.PostHistoryTypeId IS NOT NULL THEN 'Edited/Closed' ELSE 'No Recent Edits' END AS PostStatus
// FROM UserVoteStats u INNER JOIN RankingPosts r ON u.UserId = r.PostId LEFT JOIN RecentPostHistory php ON r.PostId = php.PostId AND php.rn = 1
// WHERE (u.UpVotesCount - u.DownVotesCount) > 10 ORDER BY r.Rank, u.DisplayName;
//
// `u.UserId = r.PostId` joins a user id to a post id, so it goes through the raw ids. AverageScore averages the post's own Score over its comment rows, so it is
// that Score. rn = 1 exists exactly when the post has history in the last 30 days.
fn q30454(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1],
        None => a,
    });
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rk = ranked(drain((&cc).filt(|n| n > 5)), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(score.get(p).unwrap()))
    }, true);
    let rk = rel(rk.into_iter().map(|((p, c), r)| (p, (c, r))).collect());
    let by_raw: HashIdx<i64, (Id<Post>, (i64, i64))> = (&rk).map(|(p, _)| p).select(&db.post.origid).inv().select(&rk).collect();
    let recent: MatSet<Id<Post>> = db.post_history.with((&db.post_history.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(&db.post_history.post).collect();
    type T = (Id<Post>, (i64, i64));
    let r = (&db.user.origid).select(&by_raw).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select(Ident::<Post>::new().with(&recent)).opt()));
    let mut v = drain(db.user.select((&uv).filt(|a: [i64; 3]| a[0] - a[1] > 10).and(r)));
    v.sort_by_key(|&(u, (_, ((_, (_, r)), _)))| (r, db.user.display_name.get(u).unwrap(), u));
    rows(v.into_iter().map(|(u, (a, ((p, (c, r)), h)))| {
        let w = view_count.get(p);
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(c), V::F(score.get(p).unwrap() as f64), V::I(r)]);
        f.push(V::S(match w {
            Some(w) if w > 1000 => "Highly Viewed",
            Some(w) if w >= 500 => "Moderately Viewed",
            _ => "Low Views",
        }));
        f.push(V::S(if h.is_some() { "Edited/Closed" } else { "No Recent Edits" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserStats),
// HighReputationUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes FROM RankedUsers WHERE UserRank <= 100),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.UserDisplayName, ph.Comment, ph.Text, p.Title, ph.UserId FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id
//     WHERE ph.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND p.OwnerUserId IS NOT NULL)
// SELECT ru.DisplayName, ru.Reputation, ru.TotalPosts, ru.TotalQuestions, ru.TotalAnswers, ru.TotalUpVotes, ru.TotalDownVotes, p.Title AS PostTitle, phd.CreationDate AS HistoryDate, phd.Comment AS HistoryComment,
//        phd.Text AS HistoryText
// FROM HighReputationUsers ru LEFT JOIN PostHistoryDetails phd ON ru.UserId = phd.UserId LEFT JOIN Posts p ON phd.PostId = p.Id WHERE (p.ViewCount > 100 OR phd.PostHistoryTypeId IN (10, 11, 12))
// ORDER BY ru.Reputation DESC, phd.CreationDate DESC;
//
// UserRank reads only Reputation, so the users at RANK <= 100 are picked first and the posts x votes product is driven for those alone.
fn q31374(db: &'static So) -> String {
    let rr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(rr.into_iter().take_while(|x| x.1 <= 100).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let dp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let PostHistory { user, post, creation_date, post_history_type_id, .. } = &db.post_history;
    let phd: HashIdx<Id<User>, Id<PostHistory>> = db
        .post_history
        .with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(post.select(Ident::<Post>::new().with(&db.post.owner_user)))
        .select(user)
        .inv()
        .collect();
    type H = (Id<PostHistory>, ((Id<Post>, Option<i64>), i64));
    let hv = (&phd).select(Ident::<PostHistory>::new().and(post.and(post.select(&db.post.view_count).opt()).and(post_history_type_id))).filt(|(_, ((_, w), t)): H| w.map_or(false, |w| w > 100) || matches!(t, 10 | 11 | 12));
    let mut v = drain((&tu).select((&dp).and(&us).and(hv)));
    v.sort_by_key(|&(u, (_, (h, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(h).unwrap()), u, h));
    rows(v.into_iter().map(|(u, ((n, a), (h, ((p, _), _))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::T(creation_date.get(h).unwrap()), ostr(db.post_history.comment.get(h)), ostr(db.post_history.text.get(h))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(b.Class), 0) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS PostClosure FROM PostHistory ph GROUP BY ph.PostId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ur.Reputation, ur.TotalBadges, COALESCE(phs.CloseCount, 0) AS TotalCloseActions, COALESCE(phs.DeleteCount, 0) AS TotalDeleteActions,
//        ua.DisplayName, ua.TotalBounty, ua.PostCount, CASE WHEN rp.PostRank = 1 THEN 'Latest Post' ELSE 'Older Post' END AS PostStatus, CASE WHEN phs.PostClosure > 0 THEN 'Closed' ELSE 'Open' END AS ClosureStatus
// FROM RankedPosts rp LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostHistorySummary phs ON rp.PostId = phs.PostId JOIN UserActivity ua ON rp.OwnerUserId = ua.UserId
// WHERE rp.ViewCount > (SELECT AVG(ViewCount) FROM Posts) ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// TotalBounty is summed over the votes x posts product, so it is driven for the owners of the posts that pass the ViewCount test alone. A PostRank tie goes to
// the smaller post id.
fn q22467(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, .. } = &db.post;
    let (n, s) = view_count.fold_flat((0i64, 0i64), |(n, t), w| (n + 1, t + w));
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(owner_user).select(owner_user));
    let rk = per_group(ranked(v, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let rk = rel(rk.into_iter().map(|((p, _), r)| (p, r)).collect());
    let ri: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let hot: MatSet<Id<Post>> = db.post.with(&ri).with(view_count.filt(move |w: i64| w * n > s)).collect();
    let owners: MatSet<Id<User>> = (&hot).select(owner_user).collect();
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(posts_of(db).opt()))
        .fold(0i64, |t, (b, _)| t + b.flatten().unwrap_or(0));
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tb = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |t, c| t + c.unwrap_or(0));
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 12) as i64, a[2] + matches!(t, 10 | 11) as i64]);
    let mut v = drain((&hot).select((&ri).map(|(_, r)| r).and(owner_user.select(Ident::<User>::new().and(&tb).and(&ua).and(&pc))).and((&phs).opt())));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p));
    rows(v.into_iter().map(|(p, ((r, (((u, b), t), c)), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        let h = h.unwrap_or([0; 3]);
        f.extend([user_col(db, u, "rep"), V::I(b), V::I(h[0]), V::I(h[1]), user_col(db, u, "name"), V::I(t), V::I(c)]);
        f.push(V::S(if r == 1 { "Latest Post" } else { "Older Post" }));
        f.push(V::S(if h[2] > 0 { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankPerUser
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotesReceived, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotesReceived,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT ph.UserId, COUNT(ph.Id) AS HistoryCount, SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseReopenCount FROM PostHistory ph GROUP BY ph.UserId),
// CombinedResults AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ue.UpVotesReceived, 0) AS TotalUpVotes, COALESCE(ue.DownVotesReceived, 0) AS TotalDownVotes, COALESCE(up.RankPerUser, NULL) AS PostRank,
//        COALESCE(ps.HistoryCount, 0) AS PostHistoryCount, COALESCE(ps.CloseReopenCount, 0) AS CloseReopenCount
//     FROM Users u LEFT JOIN UserEngagement ue ON u.Id = ue.UserId LEFT JOIN RankedPosts up ON u.Id = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = up.PostId) LEFT JOIN PostHistoryStats ps ON u.Id = ps.UserId)
// SELECT UserId, DisplayName, TotalUpVotes, TotalDownVotes, PostRank, PostHistoryCount, CloseReopenCount FROM CombinedResults WHERE (TotalUpVotes - TotalDownVotes) > 10 OR PostHistoryCount > 5
// ORDER BY TotalUpVotes DESC, CloseReopenCount DESC;
//
// The scalar subquery is the post's owner, so each user joins all their recent questions. Up/DownVotesReceived are summed over the votes x comments x badges
// product. A RankPerUser tie goes to the smaller post id.
fn q3730(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let rk = per_group(ranked(v, |&(p, u)| (u, Reverse(score.get(p).unwrap()), p), false), |&(_, u)| u);
    let rk = rel(rk.into_iter().map(|((_, u), r)| (u, r)).collect());
    let ri: HashIdx<Id<User>, (Id<User>, i64)> = (&rk).map(|(u, _)| u).inv().select(&rk).collect();
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(comments_by(db).opt()).and(badges_of(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ps = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + matches!(t, 10 | 11) as i64]);
    let v = drain(
        db.user
            .select((&ue).and((&ps).opt()).and((&ri).map(|(_, r)| r).opt()))
            .filt(|((a, h), _): (([i64; 2], Option<[i64; 2]>), Option<i64>)| a[0] - a[1] > 10 || h.map_or(0, |h| h[0]) > 5),
    );
    rows(v.into_iter().map(|(u, ((a, h), r))| {
        let h = h.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), oint(r), V::I(h[0]), V::I(h[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(CAST(P.Score AS INT), 0)) AS TotalScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserActivity),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerName, P.Score, P.ViewCount, COUNT(C) AS CommentCount FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id
//     LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate, U.DisplayName, P.Score, P.ViewCount),
// VoteStats AS (SELECT P.Id AS PostId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.TotalScore, RPA.PostId, RPA.Title AS RecentPostTitle, RPA.CreationDate AS RecentPostDate, RPA.OwnerName,
//        RPA.Score AS PostScore, RPA.ViewCount, RPA.CommentCount, VS.VoteCount, VS.UpVotes, VS.DownVotes
// FROM TopUsers TU JOIN RecentPosts RPA ON TU.PostCount > 0 LEFT JOIN VoteStats VS ON RPA.PostId = VS.PostId WHERE TU.ScoreRank <= 10 ORDER BY TU.TotalScore DESC, RPA.CreationDate DESC;
//
// COUNT(C) counts the row a LEFT JOIN keeps for a post with no comment, since C is never NULL as a whole. The ON condition names only TU, so the users and the
// recent posts are crossed.
fn q621(db: &'static So) -> String {
    let ups = user_posts(db);
    let tr = ranked(drain(&ups), |&(_, a)| Reverse(a[4]), false);
    let tu = rel(drain(rel(tr).filt(|((_, a), r): ((Id<User>, [i64; 10]), i64)| r <= 10 && a[1] > 0)).into_iter().map(|x| x.1 .0).collect::<Vec<(Id<User>, [i64; 10])>>());
    let Post { creation_date, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt())
        .fold(0i64, |n, _| n + 1);
    let vs = db.post.with(&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let mut v = Vec::new();
    (&tu).cross((&rp).and(&vs)).drive(|(_, p), ((u, a), (c, s))| v.push((u, a, p, c, s)));
    rows(v.into_iter().map(|(u, a, p, c, s)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]));
        f.extend([V::I(c), V::I(s[0]), V::I(s[1]), V::I(s[2])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore, AVG(COALESCE(p.AnswerCount, 0)) AS AvgAnswers, RANK() OVER (ORDER BY SUM(p.ViewCount) DESC) AS ViewRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// CloseReasonCounts AS (SELECT ph.UserId, COUNT(*) AS ClosedPostCount, SUM(CASE WHEN ph.Comment IS NOT NULL THEN 1 ELSE 0 END) AS CommentedClosedPosts FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId),
// CombinedStats AS (SELECT ups.UserId, ups.DisplayName, ups.PostCount, ups.TotalViews, ups.TotalScore, ups.AvgAnswers, COALESCE(crc.ClosedPostCount, 0) AS ClosedPostCount,
//        COALESCE(crc.CommentedClosedPosts, 0) AS CommentedClosedPosts, CASE WHEN ups.ViewRank <= 10 THEN 'Top User' ELSE 'Regular User' END AS UserType FROM UserPostStats ups LEFT JOIN CloseReasonCounts crc ON ups.UserId = crc.UserId),
// InactiveUserPostCount AS (SELECT u.Id AS UserId, COUNT(p.Id) AS InactivePostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.CreationDate < (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')
//     WHERE u.LastAccessDate < (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') GROUP BY u.Id)
// SELECT cs.UserId, cs.DisplayName, cs.PostCount, cs.TotalViews, cs.TotalScore, cs.AvgAnswers, cs.ClosedPostCount, cs.CommentedClosedPosts, COALESCE(iup.InactivePostCount, 0) AS InactivePostCount
// FROM CombinedStats cs LEFT JOIN InactiveUserPostCount iup ON cs.UserId = iup.UserId WHERE cs.TotalScore > 10 OR cs.ClosedPostCount > 0 ORDER BY cs.TotalScore DESC, cs.PostCount DESC;
//
// ViewRank and UserType are never read. AvgAnswers averages over the joined rows, so a user with no posts has one row, and 0.
fn q23216(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { view_count, score, answer_count, creation_date, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt().and(score).and(answer_count.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((w, s), n)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + n.unwrap_or(0), a[4] + 1],
        None => [a[0], a[1], a[2], a[3], a[4] + 1],
    });
    let PostHistory { user, post_history_type_id, comment, .. } = &db.post_history;
    let crc = db.post_history.with(post_history_type_id.eq(10)).group_by(user).select(comment.opt()).fold([0i64; 2], |a, c| [a[0] + 1, a[1] + c.is_some() as i64]);
    let old = Ident::<Post>::new().with(creation_date.lt(add_years(t0, -1)));
    let iup = db.user.with((&db.user.last_access_date).lt(add_years(t0, -1))).group_by(Ident::<User>::new()).select(posts_of(db).select(old).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain(db.user.select((&ups).and((&crc).opt()).and((&iup).opt())).filt(|((a, c), _): (([i64; 5], Option<[i64; 2]>), Option<i64>)| a[2] > 10 || c.map_or(0, |c| c[0]) > 0));
    rows(v.into_iter().map(|(u, ((a, c), i))| {
        let c = c.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[4]), V::I(c[0]), V::I(c[1]), V::I(i.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, COUNT(A.Id) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY P.CreationDate DESC) AS rn
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2 LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.PostTypeId = 1
//     GROUP BY P.Id, P.Title, P.CreationDate, U.DisplayName),
// TopPosts AS (SELECT RP.Id, RP.Title, RP.CreationDate, RP.OwnerDisplayName, RP.AnswerCount, RP.UpVotes, RP.DownVotes FROM RankedPosts RP WHERE RP.rn = 1 ORDER BY RP.CreationDate DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY),
// RecentVotes AS (SELECT V.PostId, COUNT(V.Id) AS RecentVoteCount FROM Votes V WHERE V.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY V.PostId),
// PostsWithRecentVotes AS (SELECT TP.*, COALESCE(RV.RecentVoteCount, 0) AS RecentVoteCount FROM TopPosts TP LEFT JOIN RecentVotes RV ON TP.Id = RV.PostId),
// ClosedPosts AS (SELECT P.Id, P.Title, PH.CreationDate FROM Posts P JOIN PostHistory PH ON PH.PostId = P.Id WHERE PH.PostHistoryTypeId = 10 GROUP BY P.Id, P.Title, PH.CreationDate)
// SELECT P.Title AS "Post Title", P.OwnerDisplayName AS "Posted By", P.CreationDate AS "Creation Date", P.AnswerCount AS "Answer Count", P.UpVotes AS "Up Votes", P.DownVotes AS "Down Votes",
//        P.RecentVoteCount AS "Recent Votes", CASE WHEN CP.Id IS NOT NULL THEN 'Yes' ELSE 'No' END AS "Is Closed"
// FROM PostsWithRecentVotes P LEFT JOIN ClosedPosts CP ON P.Id = CP.Id ORDER BY P.RecentVoteCount DESC, P.CreationDate DESC;
//
// rn is 1 for every question (one row per partition). TopPosts reads only CreationDate, so the ten newest questions are picked first (a tie goes to the
// smaller post id) and the answers x votes product is driven for those alone. ClosedPosts has one row per close date, and each joins.
fn q33983(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let top = top_k(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(_, d)| Reverse(d), |&(p, _)| p, 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (x, t)| [a[0] + x.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let now = now_utc();
    let rv = db.vote.with((&db.vote.creation_date).filt(move |d: i64| ny_to_utc(d) >= now - 30 * DAY_US)).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post.and(hd)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cv = rel(drain(&cp).into_iter().map(|((p, d), _)| (p, d)).collect());
    let ci: HashIdx<Id<Post>, (Id<Post>, i64)> = (&cv).map(|(p, _)| p).inv().select(&cv).collect();
    let mut v = drain((&rp).and((&rv).opt()).and((&ci).opt()));
    v.sort_by_key(|&(p, ((_, r), _))| (Reverse(r.unwrap_or(0)), Reverse(creation_date.get(p).unwrap()), p));
    rows(v.into_iter().map(|(p, ((a, r), c))| {
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        f.extend(a.map(V::I));
        f.extend([V::I(r.unwrap_or(0)), V::S(if c.is_some() { "Yes" } else { "No" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, (SELECT COUNT(*) FROM Badges b WHERE b.UserId = u.Id) AS BadgeCount, (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = u.Id AND p.PostTypeId = 1) AS QuestionCount
//     FROM Users u WHERE u.Reputation > 1000),
// PostHistorySummary AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS HistoryCount, MAX(ph.CreationDate) AS LastUpdate FROM PostHistory ph WHERE ph.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 years'
//     GROUP BY ph.PostId, ph.PostHistoryTypeId),
// PostCommentsCount AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// CombinedData AS (SELECT rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ur.Reputation, ur.BadgeCount, ur.QuestionCount, phs.HistoryCount, phs.LastUpdate, pcc.CommentCount
//     FROM RankedPosts rp LEFT JOIN UserReputation ur ON rp.Id = ur.UserId LEFT JOIN PostHistorySummary phs ON rp.Id = phs.PostId LEFT JOIN PostCommentsCount pcc ON rp.Id = pcc.PostId)
// SELECT Title, CreationDate, ViewCount, Score, Reputation, BadgeCount, QuestionCount, COALESCE(CommentCount, 0) AS CommentCount, COALESCE(HistoryCount, 0) AS HistoryEvents,
//        CASE WHEN LastUpdate IS NOT NULL THEN 'Updated' ELSE 'Never Updated' END AS UpdateStatus
// FROM CombinedData WHERE (Reputation > 0 OR BadgeCount > 0) ORDER BY Score DESC, CreationDate DESC LIMIT 50;
//
// Rank is never read. `rp.Id = ur.UserId` joins a post id to a user id, so it goes through the raw ids; the WHERE needs a matching user.
fn q33605(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, creation_date, score, origid, owner_user, .. } = &db.post;
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let qc = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let uidx: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.origid).inv().collect();
    let ur = origid.select(&uidx).select(Ident::<User>::new().and((&bc).opt()).and((&qc).opt()));
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(hd.gt(add_years(t0, -2))).group_by(post.and(post_history_type_id)).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pv = rel(drain(&phs));
    let pi: HashIdx<Id<Post>, ((Id<Post>, i64), (i64, i64))> = (&pv).map(|((p, _), _)| p).inv().select(&pv).collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.gt(add_days(t0, -30)))).select(ur.and((&pi).opt()).and((&cc).opt())));
    let v = top_k(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), |&(p, ((_, h), _))| (p, h.map(|((_, t), _)| t)), 50);
    rows(v.into_iter().map(|(p, ((((u, b), q), h), c))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend([user_col(db, u, "rep"), V::I(b.unwrap_or(0)), V::I(q.unwrap_or(0)), V::I(c.unwrap_or(0))]);
        f.extend(match h {
            Some((_, (n, _))) => [V::I(n), V::S("Updated")],
            None => [V::I(0), V::S("Never Updated")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank,
//        COALESCE(COUNT(DISTINCT c.Id) FILTER (WHERE c.Score > 0), 0) AS PositiveComments
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate AS ClosedDate, ph.Comment AS CloseReason FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// VoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END) AS Score FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, ur.Reputation, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges, COALESCE(vs.Score, 0) AS VoteScore, cp.ClosedDate, cp.CloseReason,
//        CASE WHEN rp.PostTypeId = 1 THEN 'Question' WHEN rp.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END AS PostType,
//        CASE WHEN rp.PositiveComments > 0 THEN 'Has Positive Comments' ELSE 'No Positive Comments' END AS CommentStatus
// FROM RankedPosts rp LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN VoteSummary vs ON rp.PostId = vs.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE rp.Rank <= 5 ORDER BY rp.PostTypeId, rp.CreationDate DESC;
//
// Rank reads only CreationDate, so the five newest per type are picked first (a tie goes to the smaller post id).
fn q22735(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold(0i64, |n, s| n + (s.unwrap_or(0) > 0) as i64);
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |s, t| s + (t == 2) as i64 - (t == 3) as i64);
    let cp = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let mut v = drain((&pc).and(owner_user.select(Ident::<User>::new().and(&ur)).opt()).and((&vs).opt()).and(cp.opt()));
    v.sort_by_key(|&(p, _)| (post_type_id.get(p).unwrap(), Reverse(creation_date.get(p).unwrap()), p));
    rows(v.into_iter().map(|(p, (((n, u), s), h))| {
        let t = post_type_id.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        match u {
            Some((u, b)) => {
                f.push(user_col(db, u, "rep"));
                f.extend(b.map(V::I));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        f.push(V::I(s.unwrap_or(0)));
        match h {
            Some(h) => f.extend([V::T(db.post_history.creation_date.get(h).unwrap()), ostr(db.post_history.comment.get(h))]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(V::S(if t == 1 { "Question" } else if t == 2 { "Answer" } else { "Other" }));
        f.push(V::S(if n > 0 { "Has Positive Comments" } else { "No Positive Comments" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, COALESCE(SUM(CASE WHEN vt.Id = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN vt.Id = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.OwnerUserId, p.CreationDate),
// UserBadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Badges b GROUP BY b.UserId),
// PostHistoryDetails AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId),
// DistinctTagCounts AS (SELECT p.Id AS PostId, COUNT(DISTINCT t.TagName) AS UniqueTagCount FROM Posts p JOIN Tags t ON t.WikiPostId = p.Id GROUP BY p.Id)
// SELECT up.DisplayName, up.Reputation, p.Title, p.ViewCount, p.Score, phd.EditCount, pt.UniqueTagCount, ub.BadgeCount AS TotalBadges, ub.HighestBadgeClass,
//        CASE WHEN p.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus, CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted Answer Exists' ELSE 'No Accepted Answer' END AS AnswerStatus
// FROM Users up JOIN Posts p ON up.Id = p.OwnerUserId LEFT JOIN PostHistoryDetails phd ON p.Id = phd.PostId LEFT JOIN DistinctTagCounts pt ON p.Id = pt.PostId LEFT JOIN UserBadgeCounts ub ON up.Id = ub.UserId
// WHERE up.Reputation > 100 AND p.ViewCount IS NOT NULL AND (p.Score > 0 OR p.ViewCount > 100)
// ORDER BY CASE WHEN ub.HighestBadgeClass = 1 THEN 1 WHEN ub.HighestBadgeClass = 2 THEN 2 ELSE 3 END, p.Score DESC;
//
// RankedPosts is never read.
fn q23317(db: &'static So) -> String {
    let Post { owner_user, view_count, score, closed_date, accepted_answer_id, .. } = &db.post;
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let phd = db.post_history.with((&db.post_history.post_history_type_id).is_in([4, 5, 6])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let dt = db.tag.with(&db.tag.wiki_post).group_by(&db.tag.wiki_post).select(&db.tag.tag_name).count_distinct();
    let users = Ident::<User>::new().with((&db.user.reputation).gt(100)).and((&ub).opt());
    let v = drain(db.post.with(view_count.filt(|w: i64| w > 100).or(score.gt(0).and(view_count))).select(owner_user.select(users).and((&phd).opt()).and((&dt).opt())));
    rows(v.into_iter().map(|(p, (((u, b), e), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend([oint(e), oint(t), oint(b.map(|b| b.0)), oint(b.map(|b| b.1))]);
        f.push(V::S(if closed_date.get(p).is_some() { "Closed" } else { "Open" }));
        f.push(V::S(if accepted_answer_id.get(p).is_some() { "Accepted Answer Exists" } else { "No Accepted Answer" }));
        row(f)
    }))
}

// WITH UserRankings AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank, COUNT(B.Id) AS BadgeCount,
//        SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.ViewCount, P.Score, (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostNum FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// AggregatedPostData AS (SELECT UR.DisplayName, COUNT(RP.PostId) AS RecentPostCount, SUM(RP.ViewCount) AS TotalViews, SUM(RP.Score) AS TotalScore
//     FROM UserRankings UR JOIN RecentPosts RP ON UR.UserId = RP.OwnerUserId GROUP BY UR.DisplayName)
// SELECT UR.DisplayName, UR.Reputation, UR.ReputationRank, APD.RecentPostCount, APD.TotalViews, APD.TotalScore,
//        CASE WHEN APD.RecentPostCount IS NULL THEN 'No Recent Posts' ELSE 'Active Contributor' END AS ContributionStatus,
//        COALESCE(REPLACE(CAST(NULLIF(UR.GoldBadges, 0) AS VARCHAR), '0', 'No Gold Badges'), 'Gold badges: ' || UR.GoldBadges) AS GoldBadges,
//        COALESCE(REPLACE(CAST(NULLIF(UR.SilverBadges, 0) AS VARCHAR), '0', 'No Silver Badges'), 'Silver badges: ' || UR.SilverBadges) AS SilverBadges,
//        COALESCE(REPLACE(CAST(NULLIF(UR.BronzeBadges, 0) AS VARCHAR), '0', 'No Bronze Badges'), 'Bronze badges: ' || UR.BronzeBadges) AS BronzeBadges
// FROM UserRankings UR LEFT JOIN AggregatedPostData APD ON UR.DisplayName = APD.DisplayName ORDER BY UR.Reputation DESC, APD.RecentPostCount DESC NULLS LAST;
//
// The CommentCount subquery and PostNum are never read. AggregatedPostData groups by DisplayName, so users who share a name share its row.
fn q20959(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let ri: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let Post { creation_date, owner_user, view_count, score, .. } = &db.post;
    let apd = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user)
        .group_by(owner_user.select(&db.user.display_name))
        .select(view_count.opt().and(score))
        .fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let v = drain((&ub).and((&ri).map(|(_, r)| r)).and((&db.user.display_name).select(&apd).opt()));
    let badge = |n: i64, word: &str, lead: &str| -> V {
        if n == 0 { V::Owned(format!("{lead} badges: 0")) } else { V::Owned(n.to_string().replace('0', &format!("No {word} Badges"))) }
    };
    rows(v.into_iter().map(|(u, ((b, r), a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(r));
        f.extend(match a {
            Some(a) => [V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::S("Active Contributor")],
            None => [V::Null, V::Null, V::Null, V::S("No Recent Posts")],
        });
        f.extend([badge(b[0], "Gold", "Gold"), badge(b[1], "Silver", "Silver"), badge(b[2], "Bronze", "Bronze")]);
        row(f)
    }))
}

// WITH RecursiveBadgeCounts AS (SELECT UserId, COUNT(Id) AS BadgeCount, MAX(Date) AS LastBadgeDate FROM Badges WHERE Date >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY UserId),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(P.ViewCount), 0) AS TotalViews, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionsAsked,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, DENSE_RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')),
// HighlightedUsers AS (SELECT UA.UserId, UA.DisplayName, UA.TotalViews, UA.QuestionsAsked, UA.TotalUpvotes, UA.TotalDownvotes, BC.BadgeCount, BC.LastBadgeDate
//     FROM UserActivity UA LEFT JOIN RecursiveBadgeCounts BC ON UA.UserId = BC.UserId WHERE UA.TotalViews >= 100 AND (UA.QuestionsAsked > 5 OR UA.TotalUpvotes > 10))
// SELECT HU.UserId, HU.DisplayName, HU.TotalViews, HU.QuestionsAsked, HU.TotalUpvotes, HU.TotalDownvotes, HU.BadgeCount, HU.LastBadgeDate, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate
// FROM HighlightedUsers HU LEFT JOIN RecentPosts RP ON HU.UserId = RP.OwnerUserId AND RP.PostRank = 1
// WHERE HU.BadgeCount IS NULL OR HU.LastBadgeDate < (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '6 months') ORDER BY HU.TotalViews DESC, HU.TotalUpvotes DESC;
//
// Not recursive despite the name. TotalViews and QuestionsAsked are summed over the posts x votes product. PostRank = 1 is every post at the owner's latest instant.
fn q22494(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Badge { user, date, .. } = &db.badge;
    let bc = db.badge.with(date.ge(add_years(t0, -1))).group_by(user).select(date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let Post { view_count, post_type_id, creation_date, owner_user, .. } = &db.post;
    let ua = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((w, t), v)) => [a[0] + w.unwrap_or(0), a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let rp = drain(db.post.with(creation_date.ge(add_days(t0, -30))).with(owner_user).select(owner_user));
    let rp = top_per(rp, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let rpi: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let cut = add_months(t0, -6);
    let hu = (&ua).filt(|a: [i64; 4]| a[0] >= 100 && (a[1] > 5 || a[2] > 10)).and((&bc).opt()).filt(move |(_, b): ([i64; 4], Option<(i64, i64)>)| b.map_or(true, |(_, d)| d < cut));
    let mut v = drain(hu.and((&rpi).map(|(_, p)| p).opt()));
    v.sort_by_key(|&(u, ((a, _), p))| (Reverse(a[0]), Reverse(a[2]), u, p));
    rows(v.into_iter().map(|(u, ((a, b), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(match b {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "created"])),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// PostHistoryData AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.UserId, MIN(ph.CreationDate) AS FirstChangeDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12, 13) GROUP BY ph.PostId, ph.PostHistoryTypeId, ph.UserId),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, COALESCE(SUM(CASE WHEN pd.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS CloseCount,
//        COALESCE(SUM(CASE WHEN pd.PostHistoryTypeId = 11 THEN 1 ELSE 0 END), 0) AS ReopenCount, MAX(pd.FirstChangeDate) AS LastChangeDate
//     FROM RankedPosts rp LEFT JOIN PostHistoryData pd ON rp.PostId = pd.PostId GROUP BY rp.PostId, rp.Title, rp.Score, rp.ViewCount)
// SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.CloseCount, ps.ReopenCount,
//        CASE WHEN ps.CloseCount > ps.ReopenCount THEN 'More Closed' WHEN ps.ReopenCount > ps.CloseCount THEN 'More Reopened' ELSE 'Equal' END AS ClosureStatus,
//        (CASE WHEN ps.ViewCount IS NULL THEN 'No Views Yet' ELSE CONCAT('Views: ', CAST(ps.ViewCount AS VARCHAR)) END) AS ViewStatus, DENSE_RANK() OVER (ORDER BY ps.Score DESC) AS ScoreRank
// FROM PostStatistics ps WHERE ps.Score > 0 ORDER BY ps.Score DESC, ps.ViewCount DESC NULLS LAST LIMIT 10 OFFSET 0;
//
// RankedPosts' Rank and counts are never read, so its comment x vote product is not driven; it is one row per recent post. ScoreRank is taken over every
// surviving row, before the LIMIT. A tie at the cut goes to the smaller post id.
fn q21267(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let PostHistory { post, post_history_type_id, user, .. } = &db.post_history;
    let pd = db.post_history.with(post_history_type_id.is_in([10, 11, 12, 13])).group_by(post.and(post_history_type_id).and(user.opt())).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pv = rel(drain(&pd).into_iter().map(|(((p, t), _), _)| (p, t)).collect());
    let pvi: HashIdx<Id<Post>, (Id<Post>, i64)> = (&pv).map(|(p, _)| p).inv().select(&pv).collect();
    let ps = db.post.group_by(Ident::<Post>::new()).select((&pvi).map(|(_, t)| t).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(11)) as i64]);
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).select(&ps));
    let v = ranked(v, |&(p, _)| Reverse(score.get(p).unwrap()), true);
    let v = top_k(v, |&((p, _), _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, |&((p, _), _)| p, 10);
    rows(v.into_iter().map(|((p, a), r)| {
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if a[0] > a[1] { "More Closed" } else if a[1] > a[0] { "More Reopened" } else { "Equal" }));
        f.push(match w {
            Some(w) => V::Owned(format!("Views: {w}")),
            None => V::S("No Views Yet"),
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotesCount,
//        COUNT(V.Id) AS TotalVotesCount, RANK() OVER (ORDER BY COUNT(V.Id) DESC) AS VoteRank FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostMetrics AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, COALESCE(SUM(CASE WHEN C.PostId IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN PH.PostId IS NOT NULL AND PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS CloseCount, COUNT(DISTINCT PH.Id) AS EditHistoryCount, P.Score,
//        CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END AS HasAcceptedAnswerFlag
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.Id, P.Title, P.OwnerUserId, P.Score, P.AcceptedAnswerId),
// UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPostsCreated, SUM(PM.CommentCount) AS TotalCommentsOnPosts, SUM(PM.CloseCount) AS TotalPostsClosed,
//        SUM(PM.EditHistoryCount) AS TotalPostEdits, AVG(PM.Score) AS AvgPostScore, SUM(PM.HasAcceptedAnswerFlag) AS TotalAcceptedAnswers
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostMetrics PM ON PM.PostId = P.Id GROUP BY U.Id, U.DisplayName)
// SELECT UDS.UserId, UDS.DisplayName, UDS.TotalPostsCreated, UDS.TotalCommentsOnPosts, UDS.TotalPostsClosed, UDS.TotalPostEdits, UDS.AvgPostScore, UDS.TotalAcceptedAnswers,
//        UVS.UpVotesCount, UVS.DownVotesCount, UVS.TotalVotesCount, UVS.VoteRank
// FROM UserPostStats UDS JOIN UserVoteStats UVS ON UDS.UserId = UVS.UserId WHERE UDS.TotalPostsCreated > 5 AND UVS.TotalVotesCount > 10 ORDER BY UDS.TotalPostsCreated DESC, UVS.VoteRank ASC;
//
// The WHERE reads only the post count and the vote count, so those users are picked first and PostMetrics' comments x history product is driven for their
// posts alone. VoteRank is taken over every user.
fn q20152(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1],
        None => a,
    });
    let vr = rel(ranked(drain(&uv), |&(_, a)| Reverse(a[2]), false).into_iter().map(|((u, a), r)| (u, (a, r))).collect());
    let vri: HashIdx<Id<User>, (Id<User>, ([i64; 3], i64))> = (&vr).map(|(u, _)| u).inv().select(&vr).collect();
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let hit: MatSet<Id<User>> = db.user.with((&pc).filt(|n| n > 5)).with((&vri).map(|(_, (a, _))| a).filt(|a: [i64; 3]| a[2] > 10)).collect();
    let Post { owner_user, score, accepted_answer_id, .. } = &db.post;
    let theirs = || db.post.with(owner_user.select(&hit));
    let pm = theirs()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(10)) as i64]);
    let eh = theirs().group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let ups = (&hit).group_by(Ident::<User>::new()).select(posts_of(db).select((&pm).and(&eh).and(score).and(accepted_answer_id.opt()))).fold([0i64; 6], |a, (((m, e), s), x)| {
        [a[0] + 1, a[1] + m[0], a[2] + m[1], a[3] + e, a[4] + s, a[5] + x.is_some() as i64]
    });
    let mut v = drain((&ups).and((&vri).map(|(_, x)| x)));
    v.sort_by_key(|&(u, (a, (_, r)))| (Reverse(a[0]), r, u));
    rows(v.into_iter().map(|(u, (a, (b, r)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), V::I(a[5]), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// ClosedPostHistory AS (SELECT ph.PostId, ph.CreationDate AS CloseDate, ph.Comment AS CloseReason, ph.UserId AS CloserUserId, ph.UserDisplayName AS CloserDisplayName FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// ProcessedVotes AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, COUNT(*) AS TotalVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT R.Id, R.Title, R.Score, (P.TotalUpVotes::FLOAT / NULLIF(P.TotalVotes, 0)) AS UpVoteRatio, U.UserId, U.PostCount, U.UpVotes, U.DownVotes, U.AvgViewCount, COALESCE(ch.CloseDate, NULL) AS LastClosedDate,
//        COALESCE(ch.CloseReason, 'N/A') AS CloseReason, COALESCE(ch.CloserUserId, -1) AS CloserUserId, COALESCE(ch.CloserDisplayName, 'System') AS CloserDisplayName
// FROM RankedPosts R JOIN UserStats U ON R.OwnerUserId = U.UserId LEFT JOIN ProcessedVotes P ON R.Id = P.PostId LEFT JOIN ClosedPostHistory ch ON R.Id = ch.PostId
// WHERE R.ScoreRank <= 5 AND U.PostCount > 0 AND (UPPER(R.Title) LIKE '%SQL%' OR R.Score > 10) ORDER BY R.Score DESC, U.AvgViewCount DESC;
//
// UserStats is driven only for the owners of the surviving posts. AvgViewCount averages integers over the posts x votes rows, so it is exact. `::FLOAT` is a
// 4-byte float, so the ratio is computed in f32.
fn q21870(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, title, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp: MatSet<Id<Post>> = (&tp).with(Ident::<Post>::new().with(title.filt(|t: Str| t.to_uppercase().contains("SQL"))).or(Ident::<Post>::new().with(score.gt(10)))).collect();
    let owners: MatSet<Id<User>> = (&rp).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((w, t)) => [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + w.unwrap_or(0)],
        None => [a[0] + 1, a[1], a[2], a[3]],
    });
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pv = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + 1]);
    let ch = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let v = drain((&rp).select(owner_user.select(Ident::<User>::new().and(&us).and(&pc)).and(&pv).and(ch.opt())));
    rows(v.into_iter().map(|(p, ((((u, a), n), c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.push(V::F((c[0] as f32 / c[1] as f32) as f64));
        f.extend([user_col(db, u, "uid"), V::I(n), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])]);
        match h {
            Some(h) => {
                let PostHistory { creation_date: hd, comment, user_id, user_display_name, .. } = &db.post_history;
                f.extend([V::T(hd.get(h).unwrap()), V::S(comment.get(h).unwrap_or("N/A")), V::I(user_id.get(h).unwrap_or(-1)), V::S(user_display_name.get(h).unwrap_or("System"))]);
            }
            None => f.extend([V::Null, V::S("N/A"), V::I(-1), V::S("System")]),
        }
        row(f)
    }))
}

// Rewritten (rewrites/1309.sql): the rn window is refined with `, p.Id`.
// WITH LatestPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.AcceptedAnswerId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS rn
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostStats AS (SELECT p.Id, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes, COALESCE(mc.CommentCount, 0) AS CommentCount FROM Posts p
//     LEFT JOIN (SELECT postId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY postId) v ON p.Id = v.postId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) mc ON p.Id = mc.PostId)
// SELECT up.UserId, up.Reputation, lp.Title, lp.CreationDate AS PostDate, ps.UpVotes, ps.DownVotes, ps.CommentCount, (CASE WHEN lp.AcceptedAnswerId IS NOT NULL THEN 'Yes' ELSE 'No' END) AS IsAcceptedAnswer,
//        (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = up.UserId AND p.PostTypeId = 2) AS AnswerCount, up.BadgeCount, up.GoldBadges, up.SilverBadges, up.BronzeBadges
// FROM UserReputation up JOIN LatestPosts lp ON up.UserId = lp.OwnerUserId LEFT JOIN PostStats ps ON lp.Id = ps.Id
// WHERE up.Reputation > (SELECT AVG(Reputation) FROM Users) AND lp.rn = 1 ORDER BY up.Reputation DESC, PostDate DESC LIMIT 100;
fn q1309(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let (n, s) = reputation.fold_flat((0i64, 0i64), |(n, t), r| (n + 1, t + r));
    let Post { creation_date, owner_user, post_type_id, accepted_answer_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user));
    let lp = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let lp = rel(lp.into_iter().map(|(p, u)| (u, p)).collect());
    let lpi: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&lp).map(|(u, _)| u).inv().select(&lp).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let ac = db.post.with(post_type_id.eq(2)).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let lpq = (&lpi).map(|(_, p)| p).select(Ident::<Post>::new().and((&vs).opt()).and((&cc).opt()));
    let v = drain(db.user.with(reputation.filt(move |r: i64| r * n > s)).select(lpq.and(&ub).and((&ac).opt())));
    let v = top_k(v, |&(u, ((((p, _), _), _), _))| (Reverse(reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())), |&(u, _)| u, 100);
    rows(v.into_iter().map(|(u, ((((p, vv), c), b), a))| {
        let vv = vv.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(vv[0]), V::I(vv[1]), V::I(c.unwrap_or(0)), V::S(if accepted_answer_id.get(p).is_some() { "Yes" } else { "No" }), V::I(a.unwrap_or(0))]);
        f.extend(b.map(V::I));
        row(f)
    }))
}

// Rewritten (rewrites/24866.sql): the final ORDER BY is refined with `, UserId, TagName`.
// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
//        COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN P.Id END) AS AnsweredQuestions,
//        DENSE_RANK() OVER (ORDER BY SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) DESC) AS VoteRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// ClosedPostCount AS (SELECT PH.UserId, COUNT(PH.Id) AS ClosedPosts FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId),
// TagStats AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%'
//     LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY T.TagName HAVING COUNT(DISTINCT P.Id) > 5),
// FinalResults AS (SELECT UVS.UserId, UVS.DisplayName, UVS.UpVotesCount, UVS.DownVotesCount, COALESCE(CPC.ClosedPosts, 0) AS ClosedPosts, UVS.TotalPosts, UVS.AnsweredQuestions, UVS.VoteRank, TS.TagName,
//        TS.PostCount, TS.CommentCount FROM UserVoteStats UVS LEFT JOIN ClosedPostCount CPC ON UVS.UserId = CPC.UserId LEFT JOIN TagStats TS ON UVS.VoteRank = 1 ORDER BY UVS.VoteRank, TS.PostCount DESC)
// SELECT UserId, DisplayName, UpVotesCount, DownVotesCount, ClosedPosts, TotalPosts, AnsweredQuestions, TagName, PostCount, CommentCount,
//        CASE WHEN ClosedPosts > TotalPosts / 2 THEN 'Major Contributor to Closed Posts' WHEN UpVotesCount >= DownVotesCount THEN 'Positive Impact' ELSE 'Neutral Contribution' END AS ContributorImpact
// FROM FinalResults WHERE VoteRank <= 10 ORDER BY ContributorImpact, UpVotesCount DESC, DownVotesCount ASC, UserId, TagName LIMIT 100;
//
// `TS ON UVS.VoteRank = 1` names only UVS, so the rank-1 users are crossed with every tag and the others keep one NULL row. `TotalPosts / 2` is a float
// division in DuckDB, compared exactly as `2 * ClosedPosts > TotalPosts`.
fn q24866(db: &'static So) -> String {
    let Post { accepted_answer_id, .. } = &db.post;
    let uv = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let aq = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(accepted_answer_id)).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let vr = ranked(drain(&uv), |&(_, a)| Reverse(a[0] - a[1]), true);
    let vr = rel(vr.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| (u, (a, r))).collect());
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let cpc = db.post_history.with(post_history_type_id.eq(10)).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let tagp = || db.tag.group_by(Ident::<Tag>::new()).select((&by_tag).map(|(p, _)| p));
    let tpc = tagp().fold(0i64, |n, _| n + 1);
    let tcc = tagp().select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ts_ = left_all(drain((&tpc).filt(|n| n > 5).and(&tcc)).into_iter().map(|(t, (n, c))| (t, n, c)).collect());
    let none = rel(vec![None::<(Id<Tag>, i64, i64)>]);
    type U = (Id<User>, ([i64; 2], i64));
    let base = (&vr).select(Same::<U>::new().and(Same::<U>::new().map(|(u, _): U| u).select((&tp).and(&aq).and((&cpc).opt()))));
    let top = base.filt(|((_, (_, r)), _): (U, ((i64, i64), Option<i64>))| r == 1);
    let rest = (&vr).select(Same::<U>::new().and(Same::<U>::new().map(|(u, _): U| u).select((&tp).and(&aq).and((&cpc).opt())))).filt(|((_, (_, r)), _): (U, ((i64, i64), Option<i64>))| r != 1);
    let mut v = Vec::new();
    top.cross(&ts_).drive(|_, (x, t)| v.push((x, t)));
    rest.cross(&none).drive(|_, (x, t)| v.push((x, t)));
    let impact = |((_, (a, _)), ((n, _), c)): (U, ((i64, i64), Option<i64>))| {
        let c = c.unwrap_or(0);
        if 2 * c > n { "Major Contributor to Closed Posts" } else if a[0] >= a[1] { "Positive Impact" } else { "Neutral Contribution" }
    };
    let v = top_k(v, |&(x, t)| (impact(x), Reverse(x.0 .1 .0[0]), x.0 .1 .0[1], x.0 .0, t.is_none(), t.map(|t| db.tag.tag_name.get(t.0).unwrap())), |_| 0, 100);
    rows(v.into_iter().map(|(x, t)| {
        let ((u, (a, _)), ((n, q), c)) = x;
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0)), V::I(n), V::I(q)]);
        f.extend(match t {
            Some((t, pc, cc)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(pc), V::I(cc)],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(impact(x)));
        row(f)
    }))
}

// WITH StringBenchmark AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId, p.CreationDate, ph.UserDisplayName AS EditedBy,
//        ph.CreationDate AS EditDate, pt.Name AS PostType, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (4, 5) LEFT JOIN Comments c ON p.Id = c.PostId JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
//     GROUP BY p.Id, u.DisplayName, ph.UserDisplayName, ph.CreationDate, p.Title, p.Body, p.Tags, p.CreationDate, p.AcceptedAnswerId, pt.Name),
// ProcessedTags AS (SELECT PostId, unnest(string_to_array(substring(Tags, 2, length(Tags)-2), '><')) AS Tag FROM StringBenchmark),
// TopTags AS (SELECT Tag, COUNT(*) AS UsageCount FROM ProcessedTags GROUP BY Tag ORDER BY UsageCount DESC LIMIT 10),
// PostEngagement AS (SELECT sb.PostId, sb.Title, sb.OwnerDisplayName, sb.PostType, sb.CommentCount, COALESCE(VoteCounts.UpVotes, 0) AS UpVotes, COALESCE(VoteCounts.DownVotes, 0) AS DownVotes
//     FROM StringBenchmark sb LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) VoteCounts
//     ON sb.PostId = VoteCounts.PostId)
// SELECT pe.PostId, pe.Title, pe.OwnerDisplayName, pe.PostType, pe.CommentCount, pe.UpVotes, pe.DownVotes, tt.Tag AS TopTag
// FROM PostEngagement pe JOIN TopTags tt ON pe.PostId IN (SELECT PostId FROM ProcessedTags WHERE Tag = tt.Tag) ORDER BY pe.UpVotes DESC, pe.CommentCount DESC;
//
// StringBenchmark has one row per (post, editor name, edit date) of the post's 4/5 history (one row when it has none), so each post's tags are counted once per
// such row. The IN subquery is a semi-join: each StringBenchmark row meets each top tag of its post once.
fn q26135(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, .. } = &db.post;
    let PostHistory { post_history_type_id, user_display_name, creation_date: hd, .. } = &db.post_history;
    let edits = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5]))).select(user_display_name.opt().and(hd));
    let sb: MatSet<(Id<Post>, Option<(Option<Str>, i64)>)> = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(owner_user).select(Ident::<Post>::new().and(edits.opt())).collect();
    type S = (Id<Post>, Option<(Option<Str>, i64)>);
    let sp = || Same::<S>::new().map(|(p, _): S| p);
    let usage = (&sb).select(sp().select(tags_str.flat_map(tag_list))).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let tt = top_k(drain(&usage), |&(_, n)| Reverse(n), |&(t, _)| t, 10);
    let tt: MatSet<Str> = rel(tt.into_iter().map(|x| x.0).collect()).map(|t| t).collect();
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ptags: MatSet<(Id<Post>, Str)> = (&sb).select(sp().select(Ident::<Post>::new().and(tags_str.flat_map(tag_list).select(Same::<Str>::new().with(&tt))))).collect();
    let pti: HashIdx<Id<Post>, (Id<Post>, Str)> = (&ptags).map(|(p, _)| p).inv().collect();
    let v = drain((&sb).select(Same::<S>::new().and(sp().select((&cc).and((&vc).opt()).and((&pti).map(|(_, t)| t))))));
    rows(v.into_iter().map(|(_, ((p, _), ((c, a), t)))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "owner", "type"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(t)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        DENSE_RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation >= 1000 GROUP BY U.Id, U.Reputation),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, COALESCE(UPV.UpVoteCount, 0) AS UpVoteCount, COALESCE(DNV.DownVoteCount, 0) AS DownVoteCount, ROW_NUMBER() OVER (ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Posts P LEFT JOIN (SELECT PostId, COUNT(*) AS UpVoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) UPV ON P.Id = UPV.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS DownVoteCount FROM Votes WHERE VoteTypeId = 3 GROUP BY PostId) DNV ON P.Id = DNV.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// ClosedPosts AS (SELECT P.Id AS ClosedPostId, P.Title, PH.Comment AS CloseReason, PH.CreationDate AS ClosedOn, RANK() OVER (PARTITION BY P.Id ORDER BY PH.CreationDate DESC) AS CloseRank
//     FROM Posts P INNER JOIN PostHistory PH ON P.Id = PH.PostId WHERE PH.PostHistoryTypeId IN (10, 11))
// SELECT U.UserId, U.Reputation, U.PostCount, (U.UpVotes - U.DownVotes) AS VoteBalance, PS.PostId, PS.Title AS PostTitle, PS.ViewCount, PS.RecentPostRank, CP.Title AS ClosedPostTitle, CP.CloseReason AS CloseReasonDetails,
//        CP.ClosedOn AS ClosedDate
// FROM UserActivity U LEFT JOIN PostStats PS ON U.UserId = PS.PostId LEFT JOIN ClosedPosts CP ON PS.PostId = CP.ClosedPostId WHERE U.ReputationRank <= 10 AND (U.UpVotes > 5 OR U.DownVotes < 3)
// ORDER BY U.Reputation DESC, PS.RecentPostRank LIMIT 100;
//
// ReputationRank reads only Reputation, so the users at DENSE_RANK <= 10 are picked first and the posts x votes product is driven for those alone.
// `U.UserId = PS.PostId` joins a user id to a post id, so it goes through the raw ids. A RecentPostRank tie goes to the smaller post id.
fn q24864(db: &'static So) -> String {
    let User { reputation, origid, .. } = &db.user;
    let dr = ranked(drain(db.user.with(reputation.ge(1000)).select(reputation)), |&(_, r)| Reverse(r), true);
    let tu: MatSet<Id<User>> = rel(dr.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ua = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let Post { creation_date, .. } = &db.post;
    let rp = ranked(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(creation_date)), |&(p, d)| (Reverse(d), p), false);
    let rp = rel(rp.into_iter().map(|((p, _), r)| (p, r)).collect());
    let by_raw: HashIdx<i64, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).select(&db.post.origid).inv().select(&rp).collect();
    let cp = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    type P = (Id<Post>, i64);
    let ps = origid.select(&by_raw).select(Same::<P>::new().and(Same::<P>::new().map(|(p, _): P| p).select(cp.opt()))).opt();
    let v = drain((&ua).filt(|a: [i64; 2]| a[0] > 5 || a[1] < 3).and(&pc).and(ps));
    let v = top_k(v, |&(u, (_, s))| (Reverse(reputation.get(u).unwrap()), s.map_or(true, |_| false), s.map(|((_, r), _)| r)), |&(u, (_, s))| (u, s.and_then(|(_, h)| h)), 100);
    rows(v.into_iter().map(|(u, ((a, n), s))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::I(a[0] - a[1])]);
        match s {
            Some(((p, r), h)) => {
                f.extend(post_fields(db, p, &["id", "title", "views"]));
                f.push(V::I(r));
                match h {
                    Some(h) => f.extend([post_fields(db, p, &["title"]).remove(0), ostr(db.post_history.comment.get(h)), V::T(db.post_history.creation_date.get(h).unwrap())]),
                    None => f.extend([V::Null, V::Null, V::Null]),
                }
            }
            None => f.extend((0..7).map(|_| V::Null)),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, U.DisplayName AS Author, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 YEAR' AND (P.ViewCount IS NOT NULL AND P.ViewCount > 0)),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, Author FROM RankedPosts WHERE PostRank <= 10),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId),
// PostVotes AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V GROUP BY V.PostId),
// PostHistoryCounts AS (SELECT PH.PostId, COUNT(PH.Id) AS EditCount, MAX(PH.CreationDate) AS LastEditDate FROM PostHistory PH GROUP BY PH.PostId),
// FinalResults AS (SELECT TP.PostId, TP.Title, TP.CreationDate, TP.Score, TP.Author, COALESCE(PC.CommentCount, 0) AS TotalComments, COALESCE(PV.UpVotes, 0) AS UpVoteCount, COALESCE(PV.DownVotes, 0) AS DownVoteCount,
//        COALESCE(PH.EditCount, 0) AS EditCount, PH.LastEditDate FROM TopPosts TP LEFT JOIN PostComments PC ON TP.PostId = PC.PostId LEFT JOIN PostVotes PV ON TP.PostId = PV.PostId LEFT JOIN PostHistoryCounts PH ON TP.PostId = PH.PostId)
// SELECT *, CASE WHEN Score > 100 THEN 'High Scoring' WHEN Score BETWEEN 50 AND 100 THEN 'Medium Scoring' ELSE 'Low Scoring' END AS ScoreCategory,
//        CASE WHEN LastEditDate >= cast('2024-10-01' as date) - INTERVAL '1 MONTH' THEN 'Recently Edited' ELSE 'Stale' END AS EditStatus
// FROM FinalResults ORDER BY Score DESC, TotalComments DESC;
//
// A PostRank tie goes to the smaller post id.
fn q32949(db: &'static So) -> String {
    let t0 = date(2024, 10, 1);
    let Post { creation_date, view_count, owner_user, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1)).and(view_count.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ph = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&tp).select((&cc).opt().and((&pv).opt()).and((&ph).opt())));
    rows(v.into_iter().map(|(p, ((c, a), h))| {
        let s = score.get(p).unwrap();
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(h.map_or(0, |h| h.0)), ots(h.map(|h| h.1))]);
        f.push(V::S(if s > 100 { "High Scoring" } else if s >= 50 { "Medium Scoring" } else { "Low Scoring" }));
        f.push(V::S(if h.map_or(false, |h| h.1 >= add_months(t0, -1)) { "Recently Edited" } else { "Stale" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, CreationDate, PostCount, QuestionCount, AnswerCount, PositiveScoreCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation),
// MostActiveUsers AS (SELECT U.UserId, U.DisplayName, U.Reputation, U.PostCount, U.QuestionCount, U.AnswerCount, U.PositiveScoreCount FROM TopUsers U WHERE U.Rank <= 10),
// PostActivity AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, C.Text AS CommentText, CASE WHEN PH.UserId IS NOT NULL THEN PH.UserId ELSE -1 END AS LastModifiedByUserId,
//        COALESCE(PH.CreationDate, P.CreationDate) AS LastActivityDate FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId)
// SELECT AU.DisplayName AS ActiveUser, AU.Reputation AS UserReputation, COUNT(DISTINCT PA.PostId) AS ActivePostCount, SUM(CASE WHEN PA.Score > 0 THEN 1 ELSE 0 END) AS PositivePostsCount,
//        COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT PH.Id) AS HistoryChangeCount, MAX(PA.LastActivityDate) AS LastActivePostDate
// FROM MostActiveUsers AU JOIN PostActivity PA ON PA.LastModifiedByUserId = AU.UserId LEFT JOIN Comments C ON PA.PostId = C.PostId LEFT JOIN PostHistory PH ON PA.PostId = PH.PostId
// GROUP BY AU.DisplayName, AU.Reputation ORDER BY AU.Reputation DESC, ActivePostCount DESC;
//
// A Rank tie goes to the smaller user id. PostActivity's rows are joined by the raw LastModifiedByUserId: a history row by a top user, or (for the user with
// Id -1) a post with no history or a history row with no user. Each matching row is then crossed with the post's comments and history again, as the query says.
fn q9883(db: &'static So) -> String {
    let User { reputation, origid, display_name, .. } = &db.user;
    let top = top_k(drain(reputation), |&(_, r)| Reverse(r), |&(u, _)| u, 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let by_raw: HashIdx<i64, Id<User>> = (&tu).select(origid).inv().collect();
    let PostHistory { post: hp, user_id, .. } = &db.post_history;
    let Post { score, .. } = &db.post;
    type R = (Id<User>, Id<Post>, Option<Id<Comment>>, Option<Id<PostHistory>>);
    let a: MatSet<R> = db
        .post_history
        .with(user_id.select(&by_raw))
        .select(user_id.select(&by_raw).and(hp).and(hp.select(comments_of(db)).opt()).and(Ident::<PostHistory>::new()))
        .map(|(((u, p), c), h)| (u, p, c, Some(h)))
        .collect();
    let lone = || Ident::<Post>::new().map(|_: Id<Post>| -1i64).select(&by_raw);
    let b1: MatSet<R> = db
        .post
        .minus(history_of(db))
        .select(lone().and(Ident::<Post>::new()).and(comments_of(db).opt()))
        .map(|((u, p), c)| (u, p, c, None))
        .collect();
    let b2: MatSet<R> = db
        .post_history
        .minus(user_id)
        .select(hp.select(lone()).and(hp).and(hp.select(comments_of(db)).opt()).and(Ident::<PostHistory>::new()))
        .map(|(((u, p), c), h)| (u, p, c, Some(h)))
        .collect();
    let none: MatSet<R> = (&b1).union(&b2).map(|x: R| x).collect();
    let pa = (&a).union(&none);
    type K = (Str, i64);
    let key = Same::<R>::new().map(|(u, _, _, _): R| u).select(display_name.and(reputation));
    let post_of = Same::<R>::new().map(|(_, p, _, _): R| p);
    let g = pa
        .group_by(key)
        .select(Same::<R>::new().and(post_of.select(comments_of(db).opt().and(history_of(db).opt()))))
        .fold((0i64, i64::MIN), |(n, m), ((_, p, _, h), _)| {
            let d = h.map_or(db.post.creation_date.get(p).unwrap(), |h| db.post_history.creation_date.get(h).unwrap());
            (n + (score.get(p).unwrap() > 0) as i64, m.max(d))
        });
    let dp = (&a).union(&none).group_by(Same::<R>::new().map(|(u, _, _, _): R| u).select(display_name.and(reputation))).select(Same::<R>::new().map(|(_, p, _, _): R| p)).count_distinct();
    let dc = (&a).union(&none).group_by(Same::<R>::new().map(|(u, _, _, _): R| u).select(display_name.and(reputation))).select(Same::<R>::new().map(|(_, p, _, _): R| p).select(comments_of(db))).count_distinct();
    let dh = (&a).union(&none).group_by(Same::<R>::new().map(|(u, _, _, _): R| u).select(display_name.and(reputation))).select(Same::<R>::new().map(|(_, p, _, _): R| p).select(history_of(db))).count_distinct();
    let v = drain((&g).and(&dp).and((&dc).opt()).and((&dh).opt()));
    rows(v.into_iter().map(|((n, r), ((((s, m), p), c), h)): (K, ((((i64, i64), i64), Option<i64>), Option<i64>))| {
        row(vec![V::S(n), V::I(r), V::I(p), V::I(s), V::I(c.unwrap_or(0)), V::I(h.unwrap_or(0)), V::T(m)])
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(SUM(CASE WHEN vote.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN vote.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotes, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY p.OwnerUserId), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.OwnerUserId), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.OwnerUserId), 0) AS BronzeBadges
//     FROM Posts p LEFT JOIN Votes vote ON p.Id = vote.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// PostSummary AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.UpVotes - rp.DownVotes AS NetVotes,
//        CASE WHEN rp.UpVotes - rp.DownVotes > 0 THEN 'Positive' WHEN rp.UpVotes - rp.DownVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteTrend, LEAD(rp.CreationDate) OVER (ORDER BY rp.CreationDate) AS NextPostDate
//     FROM RecentPosts rp),
// MergedPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.NetVotes, ps.VoteTrend, ps.NextPostDate,
//        CASE WHEN ps.NextPostDate IS NOT NULL AND ps.NextPostDate - ps.CreationDate < INTERVAL '1 day' THEN 'Merged' ELSE 'Standalone' END AS PostType FROM PostSummary ps)
// SELECT mp.PostId, mp.Title, mp.CreationDate, mp.ViewCount, mp.NetVotes, mp.VoteTrend, mp.PostType,
//        CASE WHEN mp.NetVotes > 10 THEN 'Highly Engaged' WHEN mp.NetVotes BETWEEN 5 AND 10 THEN 'Moderately Engaged' ELSE 'Less Engaged' END AS EngagementLevel
// FROM MergedPosts mp WHERE mp.ViewCount > 100 ORDER BY mp.NetVotes DESC, mp.CreationDate DESC LIMIT 50;
//
// RecentPosts has no GROUP BY, so every later step runs over the post x votes x owner-badges rows. The rows of one post share its CreationDate, so the LEAD
// order among them is open; the port orders by (date, post, vote, badge), and only the last row of a run of equal dates can be 'Standalone'. The badge sums
// are never read.
fn q1717(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    type R = (Id<Post>, Option<Id<Vote>>, Option<Id<Badge>>);
    let rp: MatSet<R> = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .select(Ident::<Post>::new().and(votes_of(db).opt()).and(owner_user.select(badges_of(db)).opt()))
        .map(|((p, v), b)| (p, v, b))
        .collect();
    let post_of = || Same::<R>::new().map(|(p, _, _): R| p);
    let nv = (&rp).group_by(post_of()).select(Same::<R>::new().map(|(_, v, _): R| v)).fold(0i64, |n, v| {
        let t = v.map(|v| db.vote.vote_type_id.get(v).unwrap());
        n + (t == Some(2)) as i64 - (t == Some(3)) as i64
    });
    let lead_w = (&rp)
        .group_by(Same::<R>::new().map(|_: R| 0u8))
        .select(Same::<R>::new())
        .window(lead, |(p, v, b): R| (creation_date.get(p).unwrap(), p, v, b), asc);
    type L = (R, Option<(i64, Id<Post>, Option<Id<Vote>>, Option<Id<Badge>>)>);
    let lr = rel(drain(&lead_w).into_iter().map(|x| x.1).collect::<Vec<L>>());
    let v = drain((&lr).filt(|((p, _, _), _): L| view_count.get(p).map_or(false, |w| w > 100)).select(Same::<L>::new().and(Same::<L>::new().map(|((p, _, _), _): L| p).select(&nv))));
    let v = top_k(v, |&(_, (((p, _, _), _), n))| (Reverse(n), Reverse(creation_date.get(p).unwrap())), |&(_, ((r, _), _))| r, 50);
    rows(v.into_iter().map(|(_, (((p, _, _), next), n))| {
        let d = creation_date.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.push(V::I(n));
        f.push(V::S(if n > 0 { "Positive" } else if n < 0 { "Negative" } else { "Neutral" }));
        f.push(V::S(if next.map_or(false, |x| x.0 - d < DAY_US) { "Merged" } else { "Standalone" }));
        f.push(V::S(if n > 10 { "Highly Engaged" } else if n >= 5 { "Moderately Engaged" } else { "Less Engaged" }));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 10 THEN 1 ELSE 0 END), 0) AS DeletedVotes,
//        RANK() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) DESC) AS Rank
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT pl.RelatedPostId) AS RelatedCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN PostLinks pl ON p.Id = pl.PostId WHERE p.Score > 10 AND p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 month') GROUP BY p.Id, p.Title, p.Score, p.ViewCount),
// PostHistoryRecent AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.UserDisplayName, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS rn FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12))
// SELECT uvs.UserId, uvs.DisplayName, p.Title AS PopularPostTitle, p.Score AS PopularPostScore, p.ViewCount AS PopularPostViewCount, ph.UserDisplayName AS LastActionUser, ph.CreationDate AS LastActionDate,
//        ph.PostHistoryTypeId AS LastActionType, CASE WHEN ph.PostHistoryTypeId = 10 THEN 'Closed' WHEN ph.PostHistoryTypeId = 11 THEN 'Reopened' WHEN ph.PostHistoryTypeId = 12 THEN 'Deleted' ELSE 'Other' END AS ActionDescription
// FROM UserVoteSummary uvs JOIN PopularPosts p ON uvs.Upvotes > 5 LEFT JOIN PostHistoryRecent ph ON p.PostId = ph.PostId AND ph.rn = 1 WHERE uvs.Rank <= 10 ORDER BY uvs.Upvotes DESC, p.Score DESC;
//
// The ON condition names only uvs, so the ranked users are crossed with the popular posts. The PopularPosts counts are never read. A tie in rn goes to the
// larger history id.
fn q4465(db: &'static So) -> String {
    let uv = db.user.with((&db.user.reputation).gt(0)).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let vr = ranked(drain(&uv), |&(_, a)| Reverse(a[0] - a[1]), false);
    let tu = rel(drain(rel(vr).filt(|((_, a), r): ((Id<User>, [i64; 2]), i64)| r <= 10 && a[0] > 5)).into_iter().map(|x| x.1 .0).collect::<Vec<(Id<User>, [i64; 2])>>());
    let Post { score, creation_date, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = top_per(drain(db.post_history.with(post_history_type_id.is_in([10, 11, 12])).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let ph = rel(ph.into_iter().map(|(h, p)| (p, h)).collect());
    let phi: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&ph).map(|(p, _)| p).inv().select(&ph).collect();
    let pp = rel(drain(db.post.with(score.gt(10).and(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)))).select((&phi).map(|(_, h)| h).opt())));
    let mut v = Vec::new();
    (&tu).cross(&pp).drive(|_, ((u, a), (p, h))| v.push((u, a, p, h)));
    rows(v.into_iter().map(|(u, _, p, h)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        match h {
            Some(h) => {
                let t = post_history_type_id.get(h).unwrap();
                f.extend([ostr(db.post_history.user_display_name.get(h)), V::T(hd.get(h).unwrap()), V::I(t)]);
                f.push(V::S(match t {
                    10 => "Closed",
                    11 => "Reopened",
                    _ => "Deleted",
                }));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::S("Other")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBountyGained
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostHistoryAggregates AS (SELECT ph.PostId, MIN(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS FirstClosedDate, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount
//     FROM PostHistory ph GROUP BY ph.PostId),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT ur.DisplayName, ur.Reputation, ur.TotalPosts, ur.TotalBountyGained, COALESCE(ph.CloseReopenCount, 0) AS HalfClosedCount, COALESCE(b.GoldBadges, 0) AS GoldBadges, COALESCE(b.SilverBadges, 0) AS SilverBadges,
//        COALESCE(b.BronzeBadges, 0) AS BronzeBadges, COUNT(DISTINCT rp.PostId) AS TotalRecentPosts, SUM(CASE WHEN ph.FirstClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalClosedPosts
// FROM UserReputation ur LEFT JOIN PostHistoryAggregates ph ON ur.UserId = ph.PostId LEFT JOIN RankedPosts rp ON ur.UserId = rp.OwnerUserId AND rp.PostRank <= 5 LEFT JOIN UserBadges b ON ur.UserId = b.UserId
// WHERE ur.Reputation > 1000 GROUP BY ur.DisplayName, ur.Reputation, ur.TotalPosts, ur.TotalBountyGained, b.GoldBadges, b.SilverBadges, b.BronzeBadges, ph.CloseReopenCount ORDER BY ur.Reputation DESC, ur.TotalPosts DESC;
//
// `ur.UserId = ph.PostId` joins a user id to a post id, so it goes through the raw ids. The outer GROUP BY is keyed by the tuple of the columns it names, which a
// user determines; the SUM runs over each user's rows x their five newest questions. A PostRank tie goes to the smaller post id.
fn q34541(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new());
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ur = users().select(posts_of(db).select(bounty.opt()).opt()).fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let tp = users().select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold((i64::MAX, 0i64), |(m, n), (t, d)| (if t == 10 { m.min(d) } else { m }, n + matches!(t, 10 | 11) as i64));
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let rp = top_per(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let rpi: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ph = (&db.user.origid).select(&pidx).select(&pha).opt();
    let key = || (&db.user.display_name).and(&db.user.reputation).and(&tp).and(&ur).and((&ub).opt()).and((&db.user.origid).select(&pidx).select(&pha).map(|(_, n): (i64, i64)| n).opt());
    let s = db.user.with(&ur).group_by(key()).select(ph.and((&rpi).map(|(_, p)| p).opt())).fold(0i64, |n, (h, _)| n + h.map_or(false, |(m, _)| m != i64::MAX) as i64);
    let d = db.user.with(&ur).group_by(key()).select((&rpi).map(|(_, p)| p)).count_distinct();
    let v = drain((&s).and((&d).opt()));
    rows(v.into_iter().map(|((((((n, r), t), b), g), c), (s, d))| {
        let g = g.unwrap_or([0; 3]);
        row(vec![V::S(n), V::I(r), V::I(t), V::I(b), V::I(c.unwrap_or(0)), V::I(g[0]), V::I(g[1]), V::I(g[2]), V::I(d.unwrap_or(0)), V::I(s)])
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u),
// ActivePosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.PostTypeId, p.Title, p.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, COUNT(c.Id) AS CommentCount, COALESCE(SUM(p.ViewCount), 0) AS TotalViews
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.OwnerUserId, p.PostTypeId, p.Title, p.CreationDate),
// UserPostStats AS (SELECT ur.UserId, ur.DisplayName, SUM(ap.UpVoteCount) AS TotalUpVotes, SUM(ap.DownVoteCount) AS TotalDownVotes, SUM(ap.CommentCount) AS TotalComments,
//        SUM(CASE WHEN ap.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount FROM UserReputation ur LEFT JOIN ActivePosts ap ON ur.UserId = ap.OwnerUserId GROUP BY ur.UserId, ur.DisplayName),
// CombinedStats AS (SELECT ups.UserId, ups.DisplayName, ups.TotalUpVotes, ups.TotalDownVotes, ups.TotalComments, ups.QuestionCount, ur.Reputation,
//        CASE WHEN ups.TotalUpVotes > ups.TotalDownVotes THEN 'More UpVotes' ELSE 'More DownVotes' END AS VoteTrend FROM UserPostStats ups JOIN UserReputation ur ON ups.UserId = ur.UserId)
// SELECT cs.DisplayName, cs.Reputation, cs.TotalUpVotes, cs.TotalDownVotes, cs.TotalComments, cs.QuestionCount, cs.VoteTrend,
//        CASE WHEN cs.Reputation IS NULL THEN 'No Reputation Found' ELSE 'Reputation Exists' END AS ReputationStatus
// FROM CombinedStats cs WHERE cs.QuestionCount > 0 AND cs.Reputation IS NOT NULL ORDER BY cs.Reputation DESC, cs.DisplayName ASC;
//
// Rank and TotalViews are never read.
fn q23468(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let ap = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let ups = db.post.with(&ap).with(owner_user).group_by(owner_user).select((&ap).and(post_type_id)).fold([0i64; 4], |s, (a, t)| [s[0] + a[0], s[1] + a[1], s[2] + a[2], s[3] + (t == 1) as i64]);
    let v = drain((&ups).filt(|a: [i64; 4]| a[3] > 0));
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[0] > a[1] { "More UpVotes" } else { "More DownVotes" }));
        f.push(V::S("Reputation Exists"));
        row(f)
    }))
}

// WITH PostScores AS (SELECT p.Id AS PostId, p.Title, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        (COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0)) AS NetScore, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount,
//        p.CreationDate, ROW_NUMBER() OVER (ORDER BY (COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0)) DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate),
// BadgeTypes AS (SELECT UserId, COUNT(*) FILTER (WHERE Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE Class = 2) AS SilverBadges, COUNT(*) FILTER (WHERE Class = 3) AS BronzeBadges FROM Badges GROUP BY UserId),
// RecentPosts AS (SELECT *, ROW_NUMBER() OVER (PARTITION BY PostId ORDER BY CreationDate DESC) AS rn FROM PostHistory WHERE PostHistoryTypeId IN (10, 11, 12))
// SELECT ps.PostId, ps.Title, ps.NetScore, ps.CommentCount, ps.BadgeCount, bt.GoldBadges, bt.SilverBadges, bt.BronzeBadges, rp.CreationDate AS LastActionDate,
//        CASE WHEN rp.PostHistoryTypeId = 10 THEN 'Closed' WHEN rp.PostHistoryTypeId = 11 THEN 'Reopened' WHEN rp.PostHistoryTypeId = 12 THEN 'Deleted' ELSE 'No Action' END AS LastAction,
//        CASE WHEN ps.NetScore > 0 THEN '> 0' WHEN ps.NetScore < 0 THEN '< 0' ELSE '= 0' END AS ScoreStatus
// FROM PostScores ps LEFT JOIN BadgeTypes bt ON ps.PostId = bt.UserId LEFT JOIN RecentPosts rp ON ps.PostId = rp.PostId AND rp.rn = 1 WHERE ps.Rank <= 10 ORDER BY ps.NetScore DESC, ps.CommentCount DESC;
//
// NetScore and CommentCount are counted over the votes x comments x owner-badges product, which is driven for every recent post because Rank reads it.
// `ps.PostId = bt.UserId` joins a post id to a user id, so it goes through the raw ids. Rank and rn ties go to the smaller id.
fn q22616(db: &'static So) -> String {
    let Post { creation_date, owner_user, origid, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(Ident::<Post>::new());
    let ps = recent()
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 2], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64 - (t == Some(3)) as i64, a[1] + c.is_some() as i64]);
    let bc = recent().select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let top = top_k(drain((&ps).and(&bc)), |&(_, (a, _))| Reverse(a[0]), |&(p, _)| p, 10);
    let tp = rel(top);
    let bt = db.badge.group_by(&db.badge.user_id).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let lh = top_per(drain(db.post_history.with(post_history_type_id.is_in([10, 11, 12])).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let lh = rel(lh.into_iter().map(|(h, p)| (p, h)).collect());
    let lhi: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&lh).map(|(p, _)| p).inv().select(&lh).collect();
    type T = (Id<Post>, ([i64; 2], i64));
    let v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select(origid.select(&bt).opt().and((&lhi).map(|(_, h)| h).opt())))));
    rows(v.into_iter().map(|(_, ((p, (a, b)), (g, h)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b)]);
        match g {
            Some(g) => f.extend(g.map(V::I)),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        match h {
            Some(h) => {
                f.push(V::T(hd.get(h).unwrap()));
                f.push(V::S(match post_history_type_id.get(h).unwrap() {
                    10 => "Closed",
                    11 => "Reopened",
                    _ => "Deleted",
                }));
            }
            None => f.extend([V::Null, V::S("No Action")]),
        }
        f.push(V::S(if a[0] > 0 { "> 0" } else if a[0] < 0 { "< 0" } else { "= 0" }));
        row(f)
    }))
}

// Rewritten (rewrites/1957.sql): the rn window is refined with `, PH.Id`.
// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN V.VoteTypeId = 6 THEN 1 ELSE 0 END) AS CloseVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COALESCE(UPV.UpVoteCount, 0) AS UpVoteCount, COALESCE(DOV.DownVoteCount, 0) AS DownVoteCount, P.OwnerUserId,
//        U.DisplayName AS OwnerDisplayName, (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount
//     FROM Posts P LEFT JOIN (SELECT PostId, COUNT(*) AS UpVoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) UPV ON P.Id = UPV.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS DownVoteCount FROM Votes WHERE VoteTypeId = 3 GROUP BY PostId) DOV ON P.Id = DOV.PostId JOIN Users U ON P.OwnerUserId = U.Id),
// RecentPostHistory AS (SELECT PH.PostId, PHT.Name AS ChangeType, PH.CreationDate AS ChangeDate, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC, PH.Id) AS rn
//     FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.ViewCount, PS.UpVoteCount, PS.DownVoteCount, PS.CommentCount, UVS.TotalVotes AS UserTotalVotes, UVS.UpVotes AS UserUpVotes, UVS.DownVotes AS UserDownVotes,
//        RP.ChangeType, RP.ChangeDate
// FROM PostStats PS LEFT JOIN UserVoteStats UVS ON PS.OwnerUserId = UVS.UserId LEFT JOIN RecentPostHistory RP ON PS.PostId = RP.PostId AND RP.rn = 1
// WHERE PS.Score > 0 AND PS.ViewCount > 100 AND (PS.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' OR PS.OwnerUserId IS NULL) ORDER BY PS.Score DESC, PS.ViewCount DESC;
//
// The Users join drops ownerless posts, so `OwnerUserId IS NULL` never holds.
fn q1957(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { score, view_count, creation_date, owner_user, .. } = &db.post;
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let lh = top_per(drain(db.post_history.with(hd.ge(add_days(t0, -30))).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let lh = rel(lh.into_iter().map(|(h, p)| (p, h)).collect());
    let lhi: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&lh).map(|(p, _)| p).inv().select(&lh).collect();
    let v = drain(
        db.post
            .with(score.gt(0).and(view_count.gt(100)).and(creation_date.gt(add_months(t0, -1))))
            .select(owner_user.select(&uvs).and((&pv).opt()).and((&cc).opt()).and((&lhi).map(|(_, h)| h).opt())),
    );
    rows(v.into_iter().map(|(p, (((u, a), c), h))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0)), V::I(u[0]), V::I(u[1]), V::I(u[2])]);
        match h {
            Some(h) => f.extend([V::S(htype_name(db).get(h).unwrap()), V::T(hd.get(h).unwrap())]),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS RankScore,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) OVER (PARTITION BY p.Id) AS UpVotesCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) OVER (PARTITION BY p.Id) AS DownVotesCount,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentsCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.RankScore, rp.UpVotesCount, rp.DownVotesCount, rp.CommentsCount,
//        CASE WHEN rp.Score > 10 THEN 'High Score' WHEN rp.Score BETWEEN 1 AND 10 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory FROM RankedPosts rp WHERE rp.RankScore <= 10),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, p.Title AS PostTitle, ph.CreationDate AS HistoryDate, pht.Name AS HistoryTypeName FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id
//     JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month')
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.UpVotesCount, fp.DownVotesCount, fp.CommentsCount, fp.ScoreCategory, COALESCE(phd.HistoryTypeName, 'No Recent Changes') AS RecentHistory,
//        COUNT(DISTINCT b.Id) AS BadgeCount
// FROM FilteredPosts fp LEFT JOIN PostHistoryDetails phd ON fp.PostId = phd.PostId LEFT JOIN Badges b ON b.UserId = fp.PostId AND b.Class = 1
// GROUP BY fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.UpVotesCount, fp.DownVotesCount, fp.CommentsCount, fp.ScoreCategory, phd.HistoryTypeName ORDER BY fp.Score DESC, fp.CommentsCount DESC, fp.CreationDate DESC;
//
// RankedPosts has no GROUP BY, so RankScore numbers the post x vote rows, and one post's rows can fill all ten slots of its type; the port numbers those rows
// (a tie goes to the smaller (post, vote) id) and keeps the posts they belong to. The outer GROUP BY merges a post's duplicate rows per history type.
// `b.UserId = fp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q23839(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let rows_ = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id.and(votes_of(db).opt())));
    let top = top_per(rows_, |&(_, (t, _))| t, |&(p, (_, v))| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p, v), 10, false);
    let fp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uv = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let recent = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)))).select(htype_name(db));
    let gold: HashIdx<i64, Id<Badge>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user_id).inv().collect();
    let g = (&fp).group_by(Ident::<Post>::new().and(recent.opt())).select(origid.select(&gold).opt()).buf_fold(|b| distinct_some(b.iter().copied()));
    let v = drain((&g).and(Same::<(Id<Post>, Option<Str>)>::new().map(|(p, _): (Id<Post>, Option<Str>)| p).select((&uv).and(&cc))));
    rows(v.into_iter().map(|((p, h), (b, (a, c)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if s > 10 { "High Score" } else if s >= 1 { "Medium Score" } else { "Low Score" }), V::S(h.unwrap_or("No Recent Changes")), V::I(b)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)) / 60) AS AvgActiveDuration FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopContributors AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalWikis, AcceptedAnswers, AvgActiveDuration, RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts FROM UserPostStats),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// FinalStats AS (SELECT t.UserId, t.DisplayName, t.TotalPosts, t.TotalQuestions, t.TotalAnswers, t.TotalWikis, t.AcceptedAnswers, t.AvgActiveDuration, COALESCE(b.BadgeCount, 0) AS TotalBadges,
//        COALESCE(b.GoldBadges, 0) AS TotalGoldBadges, COALESCE(b.SilverBadges, 0) AS TotalSilverBadges, COALESCE(b.BronzeBadges, 0) AS TotalBronzeBadges
//     FROM TopContributors t LEFT JOIN UserBadges b ON t.UserId = b.UserId WHERE t.RankByPosts <= 10)
// SELECT f.DisplayName, f.TotalPosts, f.TotalQuestions, f.TotalAnswers, f.TotalWikis, f.AcceptedAnswers, f.AvgActiveDuration, f.TotalBadges, f.TotalGoldBadges, f.TotalSilverBadges, f.TotalBronzeBadges
// FROM FinalStats f ORDER BY f.TotalPosts DESC;
//
// AvgActiveDuration averages a float per post; a user's posts are few, so it is the plain left-to-right sum (translation-failures.md 18).
fn q29927(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, last_activity_date, creation_date, .. } = &db.post;
    let ups = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(last_activity_date).and(creation_date)).opt())
        .fold(([0i64; 5], 0.0f64), |(a, s), p| match p {
            Some((((t, x), l), c)) => ([a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + (t == 1 && x.is_some()) as i64], s + (l - c) as f64 / 1e6 / 60.0),
            None => (a, s),
        });
    let rk = ranked(drain(&ups), |&(_, (a, _))| Reverse(a[0]), false);
    let tu: MatSet<Id<User>> = rel(rk.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain((&tu).select((&ups).and((&ub).opt())));
    rows(v.into_iter().map(|(u, ((a, s), b))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.push(if a[0] == 0 { V::Null } else { V::F(s / a[0] as f64) });
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadgeCount, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadgeCount,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        AVG(COALESCE(p.Score, 0)) AS AvgScore, AVG(p.ViewCount) AS AvgViewCount FROM Posts p GROUP BY p.OwnerUserId),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank, DENSE_RANK() OVER (ORDER BY u.LastAccessDate DESC) AS ActivityRank
//     FROM Users u WHERE u.LastAccessDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'))
// SELECT au.UserId, au.DisplayName, COALESCE(ubc.GoldBadgeCount, 0) AS GoldBadgeCount, COALESCE(ubc.SilverBadgeCount, 0) AS SilverBadgeCount, COALESCE(ubc.BronzeBadgeCount, 0) AS BronzeBadgeCount,
//        COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.TotalQuestions, 0) AS TotalQuestions, COALESCE(ps.TotalAnswers, 0) AS TotalAnswers, COALESCE(ps.AvgScore, 0) AS AvgScore, COALESCE(ps.AvgViewCount, 0) AS AvgViewCount,
//        CASE WHEN COALESCE(ubc.GoldBadgeCount, 0) > 0 THEN 'Gold Achiever' WHEN COALESCE(ubc.SilverBadgeCount, 0) > 1 THEN 'Silver Contributor' ELSE 'Newcomer' END AS UserType
// FROM ActiveUsers au LEFT JOIN UserBadgeCounts ubc ON au.UserId = ubc.UserId LEFT JOIN PostStats ps ON au.UserId = ps.OwnerUserId
// WHERE (COALESCE(ubc.GoldBadgeCount, 0) > 0 OR COALESCE(ubc.SilverBadgeCount, 0) > 3 OR COALESCE(ubc.BronzeBadgeCount, 0) > 5) AND COALESCE(ps.TotalPosts, 0) IS NOT NULL
// ORDER BY au.Reputation DESC, COALESCE(ps.TotalPosts, 0) DESC;
//
// The two DENSE_RANKs are never read, and `COALESCE(..) IS NOT NULL` always holds.
fn q1368(db: &'static So) -> String {
    let ubc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let v = drain(
        db.user
            .with((&db.user.last_access_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
            .select((&ubc).filt(|b: [i64; 3]| b[0] > 0 || b[1] > 3 || b[2] > 5).and((&ps).opt())),
    );
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        match p {
            Some(a) => f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), if a[4] == 0 { V::F(0.0) } else { avg(a[5], a[4]) }]),
            None => f.extend([V::I(0), V::I(0), V::I(0), V::F(0.0), V::F(0.0)]),
        }
        f.push(V::S(if b[0] > 0 { "Gold Achiever" } else if b[1] > 1 { "Silver Contributor" } else { "Newcomer" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, (SELECT COUNT(b.Id) FROM Badges b WHERE b.UserId = u.Id AND b.Class = 1) AS GoldBadges,
//        (SELECT COUNT(b.Id) FROM Badges b WHERE b.UserId = u.Id AND b.Class = 2) AS SilverBadges, (SELECT COUNT(b.Id) FROM Badges b WHERE b.UserId = u.Id AND b.Class = 3) AS BronzeBadges
//     FROM Users u WHERE u.Reputation IS NOT NULL),
// PostActivity AS (SELECT PostId, COUNT(*) AS ActivityCount, MAX(CreationDate) AS LastActivity FROM Votes GROUP BY PostId),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, COALESCE(ua.Reputation, 0) AS UserReputation, ua.GoldBadges, ua.SilverBadges, ua.BronzeBadges, COALESCE(pa.ActivityCount, 0) AS ActivityCount, rp.ViewCount
//     FROM RecentPosts rp LEFT JOIN UserReputation ua ON rp.OwnerUserId = ua.UserId LEFT JOIN PostActivity pa ON rp.PostId = pa.PostId)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.UserReputation, pd.GoldBadges, pd.SilverBadges, pd.BronzeBadges, pd.ActivityCount, pd.ViewCount,
//        CASE WHEN pd.ViewCount > 100 THEN 'Popular' ELSE 'Less Popular' END AS Popularity, CASE WHEN pd.UserReputation > 1000 THEN 'Respected User' ELSE 'New User' END AS UserStatus
// FROM PostDetails pd WHERE (pd.UserReputation >= 500 OR pd.ActivityCount > 5) AND pd.CreationDate <= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '7 days' ORDER BY pd.ViewCount DESC, pd.ActivityCount DESC;
//
// CommentCount and rn are never read.
fn q23762(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let pa = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(
        db.post
            .with(creation_date.gt(add_days(t0, -30)).and(creation_date.le(add_days(t0, -7))))
            .select(owner_user.select(Ident::<User>::new().and(&ur)).opt().and((&pa).opt()))
            .filt(|(u, a): (Option<(Id<User>, [i64; 3])>, Option<i64>)| u.map_or(0, |(u, _)| db.user.reputation.get(u).unwrap()) >= 500 || a.unwrap_or(0) > 5),
    );
    rows(v.into_iter().map(|(p, (u, a))| {
        let r = u.map_or(0, |(u, _)| db.user.reputation.get(u).unwrap());
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::I(r));
        match u {
            Some((_, b)) => f.extend(b.map(V::I)),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.extend([V::I(a.unwrap_or(0))]);
        f.extend(post_fields(db, p, &["views"]));
        f.push(V::S(if view_count.get(p).map_or(false, |w| w > 100) { "Popular" } else { "Less Popular" }));
        f.push(V::S(if r > 1000 { "Respected User" } else { "New User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.PostTypeId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS PostRank
//     FROM Posts p WHERE p.ViewCount > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MIN(c.CreationDate) AS FirstCommentDate, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// PostActivity AS (SELECT p.Id AS PostId, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(pc.FirstCommentDate, DATE '1900-01-01') AS FirstCommentDate,
//        COALESCE(pc.LastCommentDate, DATE '1900-01-01') AS LastCommentDate, u.Reputation AS UserReputation, NULLIF(u.Reputation, 0) AS NonZeroReputation
//     FROM Posts p LEFT JOIN PostComments pc ON p.Id = pc.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id)
// SELECT rp.Title, rp.Score, rp.PostRank, pa.CommentCount, pa.FirstCommentDate, pa.LastCommentDate, COALESCE(ur.Reputation, 0) AS OwnerReputation,
//        CASE WHEN pa.NonZeroReputation IS NULL THEN 'No reputation' WHEN pa.UserReputation > 1000 THEN 'Experienced' ELSE 'New User' END AS UserStatus,
//        CASE WHEN pa.CommentCount > 0 THEN 'Active Discussion' ELSE 'No Comments Yet' END AS CommentStatus
// FROM RankedPosts rp LEFT JOIN PostActivity pa ON rp.PostId = pa.PostId LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.PostRank <= 5 ORDER BY rp.Score DESC, pa.FirstCommentDate ASC LIMIT 10;
//
// The badge counts are never read. A PostRank tie goes to the smaller post id.
fn q22454(db: &'static So) -> String {
    let Post { view_count, post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(view_count.gt(0)).select(post_type_id));
    let rk = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), false), |&(_, t)| t);
    let rk = rel(rk.into_iter().filter(|x| x.1 <= 5).map(|((p, _), r)| (p, r)).collect());
    let pc = db.comment.group_by(&db.comment.post).select(&db.comment.creation_date).fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), d| (n + 1, lo.min(d), hi.max(d)));
    let d0 = date(1900, 1, 1);
    type R = (Id<Post>, i64);
    let v = drain((&rk).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&pc).opt().and(owner_user.select(&db.user.reputation).opt())))));
    let v = top_k(v, |&(_, ((p, _), (c, _)))| (Reverse(score.get(p).unwrap()), c.map_or(d0, |c| c.1)), |&(_, ((p, _), _))| p, 10);
    rows(v.into_iter().map(|(_, ((p, r), (c, u)))| {
        let (n, lo, hi) = c.unwrap_or((0, d0, d0));
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(r), V::I(n), V::T(lo), V::T(hi), V::I(u.unwrap_or(0))]);
        f.push(V::S(match u {
            None | Some(0) => "No reputation",
            Some(r) if r > 1000 => "Experienced",
            _ => "New User",
        }));
        f.push(V::S(if n > 0 { "Active Discussion" } else { "No Comments Yet" }));
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT p.Id AS PostId, ph.CreationDate, ph.UserId, ph.PostHistoryTypeId, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS RecentEditRank
//     FROM PostHistory ph INNER JOIN Posts p ON ph.PostId = p.Id WHERE ph.CreationDate > '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        (SELECT COUNT(*) FROM PostHistory WHERE PostId = p.Id AND PostHistoryTypeId IN (10, 11)) AS CloseReopenCount, (SELECT COUNT(*) FROM PostLinks pl WHERE pl.PostId = p.Id) AS RelatedPostsCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > '2024-10-01 12:34:56'::timestamp - INTERVAL '2 years' GROUP BY p.Id, p.Title),
// FinalReport AS (SELECT ps.PostId, ps.Title, ps.CommentCount, ps.UpVotes, ps.DownVotes, ps.CloseReopenCount, ps.RelatedPostsCount, COALESCE(rph.UserId, -1) AS LastEditorId,
//        COALESCE(rph.CreationDate, '1900-01-01 00:00:00') AS LastEditDate, rph.Comment AS LastEditComment FROM PostStatistics ps LEFT JOIN RecursivePostHistory rph ON ps.PostId = rph.PostId AND rph.RecentEditRank = 1)
// SELECT PostId, Title, CommentCount, UpVotes, DownVotes, CloseReopenCount, RelatedPostsCount,
//        CASE WHEN LastEditorId IS NULL THEN 'No edits yet' WHEN LastEditComment IS NULL THEN 'Edited with no comments' ELSE LastEditComment END AS LastEditCommentDescription,
//        CASE WHEN LastEditDate < '2024-10-01 12:34:56'::timestamp - INTERVAL '6 months' THEN 'Stale post' ELSE 'Recently active' END AS PostActivityStatus
// FROM FinalReport WHERE (UpVotes - DownVotes) > 0 ORDER BY UpVotes DESC, CommentCount DESC LIMIT 10;
//
// Not recursive despite the name. LastEditorId is never NULL. A RecentEditRank tie goes to the smaller history id.
fn q22160(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.gt(add_years(t0, -2)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_k(drain((&ps).filt(|a: [i64; 3]| a[1] - a[2] > 0)), |&(_, a)| (Reverse(a[1]), Reverse(a[0])), |&(p, _)| p, 10);
    let tp = rel(v);
    let cr = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rl = db.post_link.group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let lh = top_per(drain(db.post_history.with(hd.gt(add_years(t0, -1))).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let lh = rel(lh.into_iter().map(|(h, p)| (p, h)).collect());
    let lhi: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&lh).map(|(p, _)| p).inv().select(&lh).collect();
    type T = (Id<Post>, [i64; 3]);
    let v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select((&cr).opt().and((&rl).opt()).and((&lhi).map(|(_, h)| h).opt())))));
    rows(v.into_iter().map(|(_, ((p, a), ((c, l), h)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c.unwrap_or(0)), V::I(l.unwrap_or(0))]);
        let cm = h.and_then(|h| db.post_history.comment.get(h));
        f.push(V::S(cm.unwrap_or("Edited with no comments")));
        let d = h.map_or(date(1900, 1, 1), |h| hd.get(h).unwrap());
        f.push(V::S(if d < add_months(t0, -6) { "Stale post" } else { "Recently active" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) DESC) AS OwnerRank FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Badges b WHERE b.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY b.UserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ub.BadgeCount, ub.HighestBadgeClass, COALESCE(rp.UpVotes, 0) AS TotalUpVotes, COALESCE(rp.DownVotes, 0) AS TotalDownVotes, COALESCE(rp.OwnerRank, 0) AS PostRank
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN (SELECT p.OwnerUserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS OwnerRank FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.OwnerUserId) rp ON u.Id = rp.OwnerUserId)
// SELECT u.UserId, u.DisplayName, u.Reputation, COALESCE(u.BadgeCount, 0) AS BadgeCount,
//        CASE WHEN u.HighestBadgeClass IS NULL THEN 'No Badge' WHEN u.HighestBadgeClass = 1 THEN 'Gold' WHEN u.HighestBadgeClass = 2 THEN 'Silver' WHEN u.HighestBadgeClass = 3 THEN 'Bronze' ELSE 'Unknown' END AS HighestBadge,
//        u.TotalUpVotes, u.TotalDownVotes, u.PostRank FROM UserStats u WHERE u.Reputation > 1000 ORDER BY u.Reputation DESC, u.TotalUpVotes DESC LIMIT 10;
//
// RankedPosts is never read. The inner subquery has one row per owner, so its OwnerRank is 1 whenever the user owns a post.
fn q1401(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let ub = db.badge.with((&db.badge.date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(&db.badge.user).select(&db.badge.class).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let rp = db.post.with(&db.post.owner_user).group_by(&db.post.owner_user).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain(db.user.with(reputation.gt(1000)).select((&ub).opt().and((&rp).opt())));
    let v = top_k(v, |&(u, (_, r))| (Reverse(reputation.get(u).unwrap()), Reverse(r.map_or(0, |a| a[0]))), |&(u, _)| u, 10);
    rows(v.into_iter().map(|(u, (b, r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(b.map_or(0, |b| b.0)));
        f.push(V::S(match b.map(|b| b.1) {
            None => "No Badge",
            Some(1) => "Gold",
            Some(2) => "Silver",
            Some(3) => "Bronze",
            _ => "Unknown",
        }));
        let a = r.unwrap_or([0; 2]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(r.is_some() as i64)]);
        row(f)
    }))
}

// WITH PostScoreSummary AS (SELECT P.Id AS PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 WHEN V.VoteTypeId = 3 THEN -1 ELSE 0 END) AS NetScore, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY P.Id),
// ClosedPostAnalysis AS (SELECT PH.PostId, COUNT(*) AS CloseCount, STRING_AGG(CAST(PH.CreationDate AS VARCHAR), ', ' ORDER BY PH.CreationDate) AS CloseDates, MAX(PH.CreationDate) AS LastCloseDate
//     FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId),
// UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(COALESCE(PS.NetScore, 0)) AS TotalScore, SUM(CASE WHEN PS.NetScore > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN PS.NetScore < 0 THEN 1 ELSE 0 END) AS NegativePosts FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostScoreSummary PS ON P.Id = PS.PostId GROUP BY U.Id, U.DisplayName),
// FinalResults AS (SELECT U.UserId, U.DisplayName, U.TotalPosts, U.TotalScore, U.PositivePosts, U.NegativePosts, CPA.CloseCount, CPA.CloseDates, CPA.LastCloseDate
//     FROM UserPostStats U LEFT JOIN ClosedPostAnalysis CPA ON U.UserId = CPA.PostId ORDER BY U.TotalScore DESC, U.TotalPosts DESC)
// SELECT FR.*, CASE WHEN FR.CloseCount IS NOT NULL THEN 'Closed Post Activity Exists' ELSE 'No Closed Posts' END AS PostActivityStatus, COALESCE(CAST(FR.LastCloseDate AS VARCHAR), 'N/A') AS LastCloseDateFormatted
// FROM FinalResults FR WHERE (FR.TotalScore > 10 OR FR.PositivePosts > 5) AND FR.UserId IS NOT NULL AND (FR.CloseCount IS NULL OR FR.TotalPosts > 1) ORDER BY FR.UserId;
//
// `U.UserId = CPA.PostId` joins a user id to a post id, so it goes through the raw ids. The STRING_AGG is built with its own ORDER BY.
fn q23763(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold(0i64, |s, t| s + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&ps).opt()).opt()).fold([0i64; 4], |a, p| match p {
        Some(s) => [a[0] + 1, a[1] + s.unwrap_or(0), a[2] + (s.unwrap_or(0) > 0) as i64, a[3] + (s.unwrap_or(0) < 0) as i64],
        None => a,
    });
    let PostHistory { post_id, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cpa = db.post_history.with(post_history_type_id.eq(10)).group_by(post_id).select(hd).buf_fold(|mut d| {
        d.sort_unstable();
        let s: Vec<String> = d.iter().map(|&x| ts_text(x)).collect();
        (d.len() as i64, &*Box::leak(s.join(", ").into_boxed_str()), *d.last().unwrap())
    });
    let v = drain(
        db.user
            .select((&ups).and((&db.user.origid).select(&cpa).opt()))
            .filt(|(a, c): ([i64; 4], Option<(i64, &'static str, i64)>)| (a[1] > 10 || a[2] > 5) && (c.is_none() || a[0] > 1)),
    );
    rows(v.into_iter().map(|(u, (a, c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        match c {
            Some((n, s, m)) => f.extend([V::I(n), V::S(s), V::T(m), V::S("Closed Post Activity Exists"), V::Owned(ts_text(m))]),
            None => f.extend([V::Null, V::Null, V::Null, V::S("No Closed Posts"), V::S("N/A")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, pt.Name AS PostType, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT p.Id) AS PostedQuestions
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Comments c ON u.Id = c.UserId GROUP BY u.Id, u.DisplayName),
// ClosureDetails AS (SELECT ph.PostId, ph.Comment, ph.CreationDate, ph.UserDisplayName, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId = 10) AS CloseCount, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId = 11) AS ReopenCount
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.Comment, ph.CreationDate, ph.UserDisplayName),
// FinalReport AS (SELECT rp.PostId, rp.Title, us.DisplayName AS Owner, us.TotalUpVotes, us.CommentCount, rp.Score, rp.ViewCount, COALESCE(cd.CloseCount, 0) AS CloseCount, COALESCE(cd.ReopenCount, 0) AS ReopenCount,
//        CASE WHEN us.PostedQuestions > 0 THEN 'Active Contributor' ELSE 'Lurker' END AS UserStatus FROM RankedPosts rp JOIN UserStatistics us ON rp.PostId = us.UserId LEFT JOIN ClosureDetails cd ON rp.PostId = cd.PostId
//     WHERE rp.Rank <= 5)
// SELECT *, CASE WHEN ViewCount > 1000 AND CloseCount > 0 THEN 'Potentially Controversial Post' WHEN CloseCount = 0 AND ReopenCount > 0 THEN 'Recently Reopened' ELSE 'Normal Post' END AS PostStatus
// FROM FinalReport ORDER BY Score DESC, ViewCount DESC;
//
// `rp.PostId = us.UserId` joins a post id to a user id, so it goes through the raw ids, and the votes x questions x comments product is driven only for the
// users it reaches. A Rank tie goes to the smaller post id.
fn q23846(db: &'static So) -> String {
    let Post { creation_date, post_type, score, origid, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let hit: MatSet<Id<User>> = (&tp).select(origid).select(&uidx).collect();
    let asked = Ident::<Post>::new().with(post_type_id.eq(1));
    let us = (&hit)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(asked).opt()).and(comments_by(db).opt()))
        .fold([0i64; 2], |a, ((t, _), c)| [a[0] + (t == Some(2)) as i64, a[1] + c.is_some() as i64]);
    let pq = (&hit).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, user_display_name, .. } = &db.post_history;
    let cd = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post.and(comment.opt()).and(hd).and(user_display_name.opt())).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let cv = rel(drain(&cd).into_iter().map(|((((p, _), _), _), a)| (p, a)).collect());
    let ci: HashIdx<Id<Post>, (Id<Post>, [i64; 2])> = (&cv).map(|(p, _)| p).inv().select(&cv).collect();
    let v = drain((&tp).select(origid.select(&uidx).select(Ident::<User>::new().and(&us).and(&pq)).and((&ci).map(|(_, a)| a).opt())));
    rows(v.into_iter().map(|(p, (((u, a), q), c))| {
        let c = c.unwrap_or([0; 2]);
        let w = db.post.view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["score", "views"]));
        f.extend([V::I(c[0]), V::I(c[1]), V::S(if q > 0 { "Active Contributor" } else { "Lurker" })]);
        f.push(V::S(if w.map_or(false, |w| w > 1000) && c[0] > 0 { "Potentially Controversial Post" } else if c[0] == 0 && c[1] > 0 { "Recently Reopened" } else { "Normal Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 3 WHEN b.Class = 2 THEN 2 WHEN b.Class = 3 THEN 1 ELSE 0 END) AS TotalBadgePoints, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// CommentCount AS (SELECT p.Id AS PostId, COUNT(c.Id) AS TotalComments FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.LastActivityDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY p.Id),
// CriticalPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerUserId, COALESCE(cc.TotalComments, 0) AS CommentCount, CASE WHEN cc.TotalComments > 5 THEN 'Active' ELSE 'Inactive' END AS EngagementStatus, ur.TotalBadgePoints
//     FROM RankedPosts rp INNER JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN CommentCount cc ON rp.PostId = cc.PostId
//     WHERE rp.PostRank = 1 AND rp.Title NOT LIKE '%[closed]%' AND ur.TotalBadgePoints > 5)
// SELECT cp.Title AS InterestingQuestion, u.DisplayName, u.Reputation, cp.CommentCount, cp.TotalBadgePoints, CASE WHEN cp.CommentCount IS NULL THEN 'Comments not available' ELSE CAST(cp.CommentCount AS VARCHAR) END AS CommentCountString,
//        CASE WHEN cp.EngagementStatus = 'Active' THEN 'This question is actively engaged!' ELSE 'This question could use some interaction.' END AS EngagementMessage
// FROM CriticalPosts cp JOIN Users u ON cp.OwnerUserId = u.Id ORDER BY cp.TotalBadgePoints DESC, cp.CommentCount DESC LIMIT 10;
//
// In DuckDB's LIKE, `[` is an ordinary character, so the test is a plain substring test; a NULL Title fails it. A PostRank tie goes to the smaller post id.
fn q21011(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, creation_date, owner_user, title, last_activity_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).with(owner_user).select(owner_user));
    let lp = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let lp: MatSet<Id<Post>> = rel(lp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + match c {
        Some(1) => 3,
        Some(2) => 2,
        Some(3) => 1,
        _ => 0,
    });
    let cc = db.post.with(last_activity_date.ge(add_months(t0, -6))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&lp).with(title.filt(|t: Str| !t.contains("[closed]"))).select(owner_user.select(Ident::<User>::new().and((&ur).filt(|s| s > 5))).and((&cc).opt())));
    let v = top_k(v, |&(_, ((_, s), c))| (Reverse(s), Reverse(c.unwrap_or(0))), |&(p, _)| p, 10);
    rows(v.into_iter().map(|(p, ((u, s), c))| {
        let n = c.unwrap_or(0);
        let mut f = post_fields(db, p, &["title"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(n), V::I(s), V::Owned(n.to_string())]);
        f.push(V::S(if c.map_or(false, |c| c > 5) { "This question is actively engaged!" } else { "This question could use some interaction." }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalQuestions, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PostHistoryInfo AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS HistoryCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6, 10) GROUP BY ph.PostId, ph.PostHistoryTypeId),
// ActiveUserPostHistory AS (SELECT u.DisplayName, COUNT(DISTINCT ph.PostId) AS EditedPosts, AVG(CASE WHEN ph.PostHistoryTypeId = 4 THEN ph.HistoryCount ELSE NULL END) AS AvgTitleEdits,
//        AVG(CASE WHEN ph.PostHistoryTypeId = 5 THEN ph.HistoryCount ELSE NULL END) AS AvgBodyEdits, AVG(CASE WHEN ph.PostHistoryTypeId = 6 THEN ph.HistoryCount ELSE NULL END) AS AvgTagEdits
//     FROM UserActivity u LEFT JOIN PostHistoryInfo ph ON u.UserId = ph.PostId GROUP BY u.DisplayName)
// SELECT ra.PostId, ra.Title, ua.DisplayName AS OwnerDisplayName, ua.TotalQuestions, ua.TotalBounty, ActiveUser.EditedPosts, ActiveUser.AvgTitleEdits, ActiveUser.AvgBodyEdits, ActiveUser.AvgTagEdits
// FROM RankedPosts ra JOIN UserActivity ua ON ra.OwnerUserId = ua.UserId LEFT JOIN ActiveUserPostHistory ActiveUser ON ua.DisplayName = ActiveUser.DisplayName WHERE ra.PostRank = 1 ORDER BY ra.CreationDate DESC;
//
// UserActivity is driven only for the owners of the rank-1 questions; ActiveUserPostHistory needs only each user's id and name, joined to PostHistoryInfo by
// `u.UserId = ph.PostId`, a user id against a post id, through the raw ids. A PostRank tie goes to the smaller post id.
fn q31917(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let lp = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let lp: MatSet<Id<Post>> = rel(lp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&lp).select(owner_user).collect();
    let ua = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).opt()).fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let tq = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { post, post_history_type_id: ht, .. } = &db.post_history;
    let phi = db.post_history.with(ht.is_in([4, 5, 6, 10])).group_by(post.and(ht)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pv = rel(drain(&phi).into_iter().map(|((p, t), n)| (p, t, n)).collect());
    let by_raw: HashIdx<i64, (Id<Post>, i64, i64)> = (&pv).map(|(p, _, _)| p).select(origid).inv().select(&pv).collect();
    let ah = db.user.group_by(&db.user.display_name).select((&db.user.origid).select(&by_raw).opt()).fold([0i64; 6], |a, x| match x {
        Some((_, t, n)) => [a[0] + (t == 4) as i64, a[1] + if t == 4 { n } else { 0 }, a[2] + (t == 5) as i64, a[3] + if t == 5 { n } else { 0 }, a[4] + (t == 6) as i64, a[5] + if t == 6 { n } else { 0 }],
        None => a,
    });
    let ep = db.user.group_by(&db.user.display_name).select((&db.user.origid).select(&by_raw).map(|(p, _, _)| p)).count_distinct();
    let v = drain((&lp).select(owner_user.select(Ident::<User>::new().and(&ua).and(&tq).and((&db.user.display_name).select((&ah).and((&ep).opt())).opt()))));
    rows(v.into_iter().map(|(p, (((u, b), q), h))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "name"), V::I(q), V::I(b)]);
        match h {
            Some((a, e)) => f.extend([V::I(e.unwrap_or(0)), avg(a[1], a[0]), avg(a[3], a[2]), avg(a[5], a[4])]),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COUNT(*) OVER (PARTITION BY p.PostTypeId) AS TotalPosts FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '5 years'),
// UserActivity AS (SELECT u.Id AS UserId, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, COUNT(c.Id) AS CommentCount, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.Reputation > 1000 AND u.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id HAVING AVG(COALESCE(p.ViewCount, 0)) > 50),
// RightJoinPosts AS (SELECT p.Id, pt.Name AS PostTypeName, COALESCE(ph.Comment, 'No comments') AS HistoryComment FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10
//     RIGHT JOIN PostTypes pt ON p.PostTypeId = pt.Id),
// FinalOutput AS (SELECT up.UserId, RANK() OVER (ORDER BY up.TotalBounty DESC) AS UserRank, up.TotalBounty, up.CommentCount, up.AvgViewCount, rp.Title, rp.Score, rp.ViewCount, rj.PostTypeName, rj.HistoryComment
//     FROM UserActivity up JOIN RankedPosts rp ON up.CommentCount > 5 AND rp.Rank <= 3 LEFT JOIN RightJoinPosts rj ON rp.PostId = rj.Id)
// SELECT *, CASE WHEN TotalBounty > 1000 THEN 'High Contributor' WHEN TotalBounty BETWEEN 500 AND 1000 THEN 'Moderate Contributor' ELSE 'Low Contributor' END AS ContributorLevel,
//        CASE WHEN ViewCount IS NULL THEN 'Unviewed' WHEN ViewCount > 1000 THEN 'Popular' ELSE 'Normal' END AS ViewStatus
// FROM FinalOutput ORDER BY UserRank, Score DESC LIMIT 50;
//
// UserActivity is folded over the votes x comments x posts product of each user. The ON condition names up and rp separately, so the users are crossed with the
// top posts. UserRank is taken over the joined rows, so it counts rows, not users. A Rank tie goes to the smaller post id, and a tie at the LIMIT to the
// smaller (user, post, history) id.
fn q22541(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let User { reputation, creation_date: uc, .. } = &db.user;
    let ua = db
        .user
        .with(reputation.gt(1000).and(uc.lt(add_years(t0, -1))))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(comments_by(db).opt()).and(posts_of(db).select((&db.post.view_count).opt()).opt()))
        .fold([0i64; 4], |a, ((b, c), w)| [a[0] + b.flatten().unwrap_or(0), a[1] + c.is_some() as i64, a[2] + 1, a[3] + w.flatten().unwrap_or(0)]);
    let up = rel(drain((&ua).filt(|a: [i64; 4]| a[3] > 50 * a[2] && a[1] > 5)));
    let Post { creation_date, post_type_id, score, post_type, .. } = &db.post;
    let rp = top_per(drain(db.post.with(creation_date.gt(add_years(t0, -5))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let rj = rel(drain(rel(rp.into_iter().map(|x| x.0).collect::<Vec<Id<Post>>>()).select(Same::<Id<Post>>::new().and(Same::<Id<Post>>::new().select(closes.opt())))).into_iter().map(|x| x.1).collect::<Vec<(Id<Post>, Option<Id<PostHistory>>)>>());
    let mut v = Vec::new();
    (&up).cross(&rj).drive(|_, ((u, a), (p, h))| v.push((u, a, p, h)));
    let v = ranked(v, |&(_, a, _, _)| Reverse(a[0]), false);
    let v = top_k(v, |&((_, _, p, _), r)| (r, Reverse(score.get(p).unwrap())), |&((u, _, p, h), _)| (u, p, h), 50);
    rows(v.into_iter().map(|((u, a, p, h), r)| {
        let w = db.post.view_count.get(p);
        let mut f = vec![user_col(db, u, "uid"), V::I(r), V::I(a[0]), V::I(a[1]), avg(a[3], a[2])];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(V::S(db.post_type.name.get(post_type.get(p).unwrap()).unwrap()));
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No comments")));
        f.push(V::S(if a[0] > 1000 { "High Contributor" } else if a[0] >= 500 { "Moderate Contributor" } else { "Low Contributor" }));
        f.push(V::S(match w {
            None => "Unviewed",
            Some(w) if w > 1000 => "Popular",
            _ => "Normal",
        }));
        row(f)
    }))
}

// WITH Post_Scores AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, p.PostTypeId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId, p.PostTypeId),
// Ranked_Posts AS (SELECT ps.PostId, ps.Title, ps.Score, ps.UpVotes, ps.DownVotes, ps.CommentCount, RANK() OVER (ORDER BY ps.Score + ps.UpVotes - ps.DownVotes DESC) AS PostRank FROM Post_Scores ps),
// Top_Posts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.UpVotes, rp.DownVotes, rp.CommentCount FROM Ranked_Posts rp WHERE rp.PostRank <= 10),
// User_Scores AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(DISTINCT p.Id) AS PostCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// Top_Users AS (SELECT us.UserId, us.DisplayName, us.TotalScore, us.PostCount, ROW_NUMBER() OVER (ORDER BY us.TotalScore DESC) AS UserRank FROM User_Scores us WHERE us.PostCount > 0)
// SELECT tp.Title AS TopPostTitle, tp.Score AS TopPostScore, tu.DisplayName AS TopUserName, tu.TotalScore AS UserTotalScore, tp.UpVotes, tp.DownVotes, tp.CommentCount, tp.PostId,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = tp.PostId) AS TotalComments, CASE WHEN tp.CommentCount > 0 THEN 'Comments Available' ELSE 'No Comments' END AS CommentsStatus
// FROM Top_Posts tp JOIN Top_Users tu ON tp.UpVotes > 5 ORDER BY tp.Score DESC, tu.TotalScore DESC;
//
// The ON condition names tp and tu separately, so the top posts with more than five upvotes are crossed with every user who owns a post. UserRank is never read.
fn q33533(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let rk = ranked(drain(&ps), |&(p, a)| Reverse(score.get(p).unwrap() + a[0] - a[1]), false);
    let tp = rel(drain(rel(rk).filt(|((_, a), r): ((Id<Post>, [i64; 3]), i64)| r <= 10 && a[0] > 5)).into_iter().map(|x| x.1 .0).collect::<Vec<(Id<Post>, [i64; 3])>>());
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let us = rel(drain(db.post.with(owner_user).group_by(owner_user).select(score).fold(0i64, |s, x| s + x)));
    type T = (Id<Post>, [i64; 3]);
    let tpc = rel(drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select((&cc).opt())))).into_iter().map(|x| x.1).collect::<Vec<(T, Option<i64>)>>());
    let mut v = Vec::new();
    (&tpc).cross(&us).drive(|_, (((p, a), c), (u, s))| v.push((p, a, c, u, s)));
    rows(v.into_iter().map(|(p, a, c, u, s)| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([user_col(db, u, "name"), V::I(s), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(c.unwrap_or(0)), V::S(if a[2] > 0 { "Comments Available" } else { "No Comments" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserDetails AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, (SELECT COUNT(*) FROM Badges b WHERE b.UserId = u.Id) AS BadgeCount, (SELECT SUM(p.ViewCount) FROM Posts p WHERE p.OwnerUserId = u.Id) AS TotalViewCount FROM Users u),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, ud.DisplayName, ud.Reputation, ud.BadgeCount, ud.TotalViewCount, rp.UpVotes - rp.DownVotes AS NetVotes, rp.Rank
//     FROM RankedPosts rp JOIN UserDetails ud ON rp.OwnerUserId = ud.UserId WHERE rp.Rank = 1),
// FilteredPosts AS (SELECT *, CASE WHEN NetVotes >= 10 THEN 'High Engagement' WHEN NetVotes BETWEEN 0 AND 9 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel FROM PostStatistics)
// SELECT fp.Title, fp.CreationDate, fp.ViewCount, fp.Score, fp.AnswerCount, fp.DisplayName, fp.Reputation, fp.BadgeCount, fp.TotalViewCount, fp.EngagementLevel,
//        CASE WHEN fp.Reputation IS NULL THEN 'No Reputation' ELSE CAST(fp.Reputation AS VARCHAR) END AS ReputationStatus
// FROM FilteredPosts fp LEFT JOIN Comments c ON fp.PostId = c.PostId WHERE EXISTS (SELECT 1 FROM PostHistory ph WHERE ph.PostId = fp.PostId AND ph.PostHistoryTypeId IN (10, 11))
// ORDER BY fp.CreationDate DESC FETCH FIRST 10 ROWS ONLY;
//
// RankedPosts has no GROUP BY, so Rank numbers the post x vote rows; Rank = 1 is one row of each owner's newest post (a date tie goes to the smaller post id).
fn q23754(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user.and(votes_of(db).opt())));
    let first = top_per(v, |&(_, (u, _))| u, |&(p, (_, x))| (Reverse(creation_date.get(p).unwrap()), p, x), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let nv = (&first).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let tv = db.post.with(view_count).group_by(owner_user).select(view_count).fold(0i64, |s, w| s + w);
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).select(&db.post_history.post).collect();
    let v = drain((&closed).select((&nv).and(owner_user.select(Ident::<User>::new().and((&bc).opt()).and((&tv).opt()))).and(comments_of(db).opt())));
    let v = top_k(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), |&(p, (_, c))| (p, c), 10);
    rows(v.into_iter().map(|(p, ((n, ((u, b), t)), _))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score", "answers"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b.unwrap_or(0)), oint(t)]);
        f.push(V::S(if n >= 10 { "High Engagement" } else if n >= 0 { "Moderate Engagement" } else { "Low Engagement" }));
        f.push(V::Owned(db.user.reputation.get(u).unwrap().to_string()));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS PostsVotedOn FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id LEFT JOIN Posts p ON p.Id = v.PostId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalVotes, UpVotes, DownVotes, PostsVotedOn, RANK() OVER (ORDER BY TotalVotes DESC) AS VoteRank FROM UserVoteStats),
// PostActiveStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount, SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenCount,
//        MAX(p.LastActivityDate) AS LastActivity, p.AcceptedAnswerId IS NULL AS IsUnanswered
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.AcceptedAnswerId),
// UserPostInteraction AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(CASE WHEN up.PostId IS NOT NULL THEN 1 ELSE 0 END) AS PostsInteracted
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostActiveStats up ON up.PostId = p.Id GROUP BY u.Id HAVING COUNT(DISTINCT p.Id) > 0)
// SELECT tu.DisplayName, tu.TotalVotes, tu.UpVotes, tu.DownVotes, tu.PostsVotedOn, pas.CommentCount, pas.CloseCount, pas.ReopenCount, upi.PostsCreated, upi.PostsInteracted, pas.LastActivity, pas.IsUnanswered
// FROM TopUsers tu JOIN PostActiveStats pas ON pas.CommentCount > 5 JOIN UserPostInteraction upi ON upi.UserId = tu.UserId WHERE tu.VoteRank <= 10 ORDER BY tu.TotalVotes DESC, pas.LastActivity DESC;
//
// The ON condition names only pas, so the top voters are crossed with the recent posts that have more than five comment rows.
fn q20489(db: &'static So) -> String {
    let Vote { post, .. } = &db.vote;
    let uv = db.user.with((&db.user.reputation).gt(100)).group_by(Ident::<User>::new()).select(votes_by(db).select(vtype_name(db)).opt()).fold([0i64; 3], |a, n| match n {
        Some(n) => [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64],
        None => a,
    });
    let pv = db.user.with((&db.user.reputation).gt(100)).group_by(Ident::<User>::new()).select(votes_by(db).select(post)).count_distinct();
    let vr = ranked(drain(&uv), |&(_, a)| Reverse(a[0]), false);
    let tu: MatSet<Id<User>> = rel(vr.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { creation_date, accepted_answer_id, last_activity_date, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pas = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(10)) as i64, a[2] + (t == Some(11)) as i64]);
    let upi = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().and((&pas).opt()))).fold([0i64; 2], |a, (_, s)| [a[0] + 1, a[1] + s.is_some() as i64]);
    let users = rel(drain((&tu).select((&uv).and((&pv).opt()).and(&upi))));
    let posts = rel(drain((&pas).filt(|a: [i64; 3]| a[0] > 5)));
    let mut v = Vec::new();
    (&users).cross(&posts).drive(|_, ((u, ((a, d), i)), (p, s))| v.push((u, a, d, i, p, s)));
    rows(v.into_iter().map(|(u, a, d, i, p, s)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(d.unwrap_or(0))];
        f.extend([V::I(s[0]), V::I(s[1]), V::I(s[2]), V::I(i[0]), V::I(i[1]), V::T(last_activity_date.get(p).unwrap()), V::B(accepted_answer_id.get(p).is_none())]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COALESCE(vs.VoteScore, 0) AS VoteScore, COALESCE(c.CommentCount, 0) AS CommentCount, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 WHEN VoteTypeId = 3 THEN -1 ELSE 0 END) AS VoteScore FROM Votes GROUP BY PostId) vs ON p.Id = vs.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopCommenters AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(C.Id) AS TotalComments FROM Users U JOIN Comments C ON U.Id = C.UserId GROUP BY U.Id, U.DisplayName HAVING COUNT(C.Id) > 10),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.VoteScore, rp.CommentCount, rp.ViewCount, rp.OwnerUserId, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        CASE WHEN rp.VoteScore > 0 THEN 'Positive' WHEN rp.VoteScore < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteType, RANK() OVER (ORDER BY rp.VoteScore DESC, rp.CommentCount DESC) AS PostRank
//     FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.VoteScore IS NOT NULL)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.VoteScore, pd.CommentCount, pd.ViewCount, pd.OwnerDisplayName, pd.OwnerReputation, pd.VoteType, COALESCE(tc.TotalComments, 0) AS TotalCommentsByTopCommenters
// FROM PostDetails pd LEFT JOIN TopCommenters tc ON pd.OwnerUserId = tc.UserId WHERE pd.PostRank <= 10 ORDER BY pd.VoteScore DESC, TotalCommentsByTopCommenters DESC;
//
// OwnerPostRank is never read.
fn q22412(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |s, t| s + (t == 2) as i64 - (t == 3) as i64);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let tc = db.comment.with(&db.comment.user).group_by(&db.comment.user).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.post.with(creation_date.gt(add_days(date(2024, 10, 1), -30))).with(owner_user).select((&vs).opt().and((&cc).opt()).and(owner_user.select((&tc).filt(|n| n > 10)).opt())));
    let v = ranked(v, |&(_, ((s, c), _))| (Reverse(s.unwrap_or(0)), Reverse(c.unwrap_or(0))), false);
    let v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect();
    rows(v.into_iter().map(|(p, ((s, c), t))| {
        let s = s.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s), V::I(c.unwrap_or(0))]);
        f.extend(post_fields(db, p, &["views", "owner", "rep"]));
        f.extend([V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }), V::I(t.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RECURSIVE UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount, SUM(V.BountyAmount) AS TotalBounty, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY COUNT(P.Id) DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalBounty, UpVotes, DownVotes, UserRank FROM UserActivity WHERE UserRank <= 10),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, COALESCE(NULLIF(P.AcceptedAnswerId, -1), 0) AS AcceptedAnswerId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COALESCE(MAX(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(MAX(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
//        ROW_NUMBER() OVER (ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.AcceptedAnswerId),
// AggregatedData AS (SELECT U.DisplayName, U.Reputation, SUM(P.ViewCount) AS TotalViews, COUNT(DISTINCT P.PostId) AS TotalPosts, AVG(P.CommentCount) AS AvgCommentsPerPost, SUM(P.TotalUpVotes) AS TotalUpVotes,
//        SUM(P.TotalDownVotes) AS TotalDownVotes FROM TopUsers U JOIN PostStats P ON U.UserId = P.AcceptedAnswerId GROUP BY U.DisplayName, U.Reputation)
// SELECT A.DisplayName, A.Reputation, A.TotalViews, A.TotalPosts, A.AvgCommentsPerPost, (A.TotalUpVotes - A.TotalDownVotes) AS NetVotes FROM AggregatedData A ORDER BY A.TotalPosts DESC, A.TotalViews DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. UserRank partitions by the user's own id, so it is always 1 and TopUsers is every user; none of UserActivity's
// aggregates is read. `U.UserId = P.AcceptedAnswerId` joins a user id to a post id (0 for no accepted answer), so it goes through the raw ids, and PostStats'
// comments x votes product is driven only for the posts it reaches.
fn q34283(db: &'static So) -> String {
    let Post { accepted_answer_id, view_count, .. } = &db.post;
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let key = accepted_answer_id.opt().map(|a: Option<i64>| match a {
        Some(-1) | None => 0,
        Some(a) => a,
    });
    let hit: MatSet<Id<Post>> = db.post.with(key.select(&uidx)).collect();
    let ps = (&hit)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1].max((t == Some(2)) as i64), a[2].max((t == Some(3)) as i64)]);
    let key2 = accepted_answer_id.opt().map(|a: Option<i64>| match a {
        Some(-1) | None => 0,
        Some(a) => a,
    });
    let g = (&hit)
        .group_by(key2.select(&uidx).select((&db.user.display_name).and(&db.user.reputation)))
        .select((&ps).and(view_count.opt()))
        .fold([0i64; 6], |a, (s, w)| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + 1, a[3] + s[0], a[4] + s[1], a[5] + s[2]]);
    let v = drain(&g);
    rows(v.into_iter().map(|((n, r), a)| row(vec![V::S(n), V::I(r), nullable(a[1], a[0]), V::I(a[2]), avg(a[3], a[2]), V::I(a[4] - a[5])])))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
//        COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, DENSE_RANK() OVER(PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC) AS ActivityRank,
//        COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount FROM Posts p),
// RecentPosts AS (SELECT pa.PostId, pa.Title, pa.CreationDate, pa.LastActivityDate, pa.CommentCount, us.DisplayName FROM PostActivity pa JOIN UserStatistics us ON pa.PostId IN (SELECT Id FROM Posts WHERE OwnerUserId = us.UserId)
//     WHERE pa.LastActivityDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'),
// AggregatedData AS (SELECT us.DisplayName, us.TotalPosts, us.TotalQuestions, us.TotalAnswers, COUNT(rp.PostId) AS RecentPostCount, AVG(COALESCE(rp.CommentCount, 0)) AS AvgCommentsPerPost
//     FROM UserStatistics us LEFT JOIN RecentPosts rp ON us.DisplayName = rp.DisplayName GROUP BY us.DisplayName, us.TotalPosts, us.TotalQuestions, us.TotalAnswers)
// SELECT ad.DisplayName, ad.TotalPosts, ad.TotalQuestions, ad.TotalAnswers, ad.RecentPostCount, ad.AvgCommentsPerPost,
//        CASE WHEN ad.TotalPosts > 50 THEN 'Highly Active' WHEN ad.TotalPosts BETWEEN 21 AND 50 THEN 'Moderately Active' ELSE 'Less Active' END AS ActivityLevel
// FROM AggregatedData ad ORDER BY ad.TotalPosts DESC, ad.TotalQuestions DESC, ad.TotalAnswers DESC;
//
// The vote sums and ActivityRank are never read, and the distinct counts do not need the votes, so the vote join is not driven. The IN subquery is the owner
// join. RecentPosts is joined back by DisplayName, and the outer GROUP BY is keyed by the four columns it names. LastActivityDate is compared with
// CURRENT_TIMESTAMP as a UTC instant.
fn q248(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, owner_user, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let now = now_utc();
    let rp: HashIdx<Str, Id<Post>> = db.post.with(last_activity_date.filt(move |d: i64| ny_to_utc(d) >= now - 30 * DAY_US)).with(owner_user).select(owner_user.select(&db.user.display_name)).inv().collect();
    let g = db
        .user
        .group_by((&db.user.display_name).and(&us))
        .select((&db.user.display_name).select((&rp).select((&cc).opt()).opt()))
        .fold([0i64; 3], |a, r| match r {
            Some(c) => [a[0] + 1, a[1] + c.unwrap_or(0), a[2] + 1],
            None => [a[0], a[1], a[2] + 1],
        });
    let v = drain(&g);
    rows(v.into_iter().map(|((n, t), a)| {
        row(vec![V::S(n), V::I(t[0]), V::I(t[1]), V::I(t[2]), V::I(a[0]), avg(a[1], a[2]), V::S(if t[0] > 50 { "Highly Active" } else if t[0] >= 21 { "Moderately Active" } else { "Less Active" })])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// VotesSummary AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(v.Id) AS TotalVotes FROM Votes v GROUP BY v.PostId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.UserId, ph.CreationDate, MAX(ph.CreationDate) OVER (PARTITION BY ph.PostId) AS LatestEditDate FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (4, 5, 6) AND ph.CreationDate IS NOT NULL),
// RecentClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 AND ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes, COALESCE(vs.TotalVotes, 0) AS TotalVotes, u.UserId, u.Reputation, u.BadgeCount,
//        COALESCE(rc.CloseCount, 0) AS RecentCloseCount, CASE WHEN pd.PostId IS NOT NULL AND pd.LatestEditDate IS NULL THEN 'No Edits' ELSE 'Edited' END AS EditStatus
// FROM RankedPosts rp LEFT JOIN VotesSummary vs ON rp.PostId = vs.PostId LEFT JOIN UserReputation u ON rp.OwnerUserId = u.UserId LEFT JOIN RecentClosedPosts rc ON rp.PostId = rc.PostId
// LEFT JOIN PostHistoryDetails pd ON rp.PostId = pd.PostId WHERE rp.PostRank = 1 ORDER BY rp.CreationDate DESC LIMIT 100;
//
// A PostHistoryDetails row always has a LatestEditDate, so EditStatus is 'Edited'; each such row repeats its post.
fn q21542(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(t0, -1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let rc = db.post_history.with(post_history_type_id.eq(10).and(hd.ge(add_days(t0, -30)))).group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pd = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6])));
    let v = drain((&tp).select((&vs).opt().and(owner_user.select(Ident::<User>::new().and(&ur)).opt()).and((&rc).opt()).and(pd.opt())));
    let v = top_k(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), |&(p, (_, h))| (p, h), 100);
    rows(v.into_iter().map(|(p, (((a, u), c), _))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend(a.map(V::I));
        match u {
            Some((u, b)) => f.extend([user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(b)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.extend([V::I(c.unwrap_or(0)), V::S("Edited")]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, P.LastActivityDate, P.Score, P.ViewCount, P.AnswerCount, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS RankByScore
//     FROM Posts P WHERE P.PostTypeId = 1 AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(P.ViewCount) AS TotalViews FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UR.UserId, UR.Reputation, UR.QuestionCount, UR.TotalViews, RANK() OVER (ORDER BY UR.Reputation DESC) AS ReputationRank, RANK() OVER (ORDER BY UR.TotalViews DESC) AS ViewRank
//     FROM UserReputation UR WHERE UR.QuestionCount > 5),
// PostHistoryAnalysis AS (SELECT PH.PostId, COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount, MAX(PH.CreationDate) AS LastEditDate
//     FROM PostHistory PH GROUP BY PH.PostId)
// SELECT P.Title, P.CreationDate AS QuestionDate, P.Score AS QuestionScore, PH.CloseCount, PH.ReopenCount, PH.LastEditDate, U.DisplayName, U.Reputation, T.ViewRank, T.ReputationRank,
//        COALESCE(UP.AnswerCount, 0) AS AnswerCount, COALESCE(COALESCE(UP.ViewCount, 0) * 1.0 / NULLIF(PH.CloseCount, 0), 0) AS ViewsPerClose
// FROM RankedPosts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN PostHistoryAnalysis PH ON P.PostId = PH.PostId LEFT JOIN TopUsers T ON U.Id = T.UserId
// LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount, SUM(ViewCount) AS ViewCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) UP ON P.PostId = UP.ParentId WHERE P.RankByScore = 1 ORDER BY P.Score DESC, P.ViewCount DESC;
fn q30518(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, parent, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ur = db.post.with(post_type_id.eq(1)).with(owner_user).group_by(owner_user).select(view_count.opt()).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let tu = drain((&ur).filt(|a: [i64; 3]| a[0] > 5));
    let tu = ranked(tu, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let tu = ranked(tu, |&((_, a), _)| (a[1] == 0, Reverse(a[2])), false);
    let tr = rel(tu.into_iter().map(|(((u, _), r), w)| (u, (w, r))).collect());
    let ti: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold([0i64, 0, i64::MIN], |a, (t, d)| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2].max(d)]);
    let up = db.post.with(post_type_id.eq(2)).with(parent).group_by(parent).select(view_count.opt()).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let v = drain((&tp).select((&pha).opt().and(owner_user.select((&ti).map(|(_, x)| x).opt())).and((&up).opt())));
    rows(v.into_iter().map(|(p, ((h, t), a))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        match h {
            Some(h) => f.extend([V::I(h[0]), V::I(h[1]), V::T(h[2])]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.extend(post_fields(db, p, &["owner", "rep"]));
        match t {
            Some((w, r)) => f.extend([V::I(w), V::I(r)]),
            None => f.extend([V::Null, V::Null]),
        }
        let a = a.unwrap_or([0; 3]);
        f.push(V::I(a[0]));
        let views = if a[1] > 0 { a[2] } else { 0 };
        f.push(V::F(match h {
            Some(h) if h[0] != 0 => views as f64 * 1.0 / h[0] as f64,
            _ => 0.0,
        }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostScores AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetScore, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT l.RelatedPostId) AS LinkedPosts FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostLinks l ON p.Id = l.PostId GROUP BY p.Id),
// ImportantPosts AS (SELECT ps.PostId, ps.NetScore, ps.CommentCount, ROW_NUMBER() OVER (ORDER BY ps.NetScore DESC, ps.CommentCount DESC) AS Rank FROM PostScores ps WHERE ps.NetScore > 0),
// ActiveUsers AS (SELECT ur.UserId, ur.Reputation, ur.BadgeCount, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges, COUNT(DISTINCT p.Id) AS ActivePostCount FROM UserReputation ur JOIN Posts p ON ur.UserId = p.OwnerUserId
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '30 days' GROUP BY ur.UserId, ur.Reputation, ur.BadgeCount, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges)
// SELECT au.UserId, au.Reputation, au.BadgeCount, au.GoldBadges, au.SilverBadges, au.BronzeBadges, COUNT(ip.PostId) AS ImportantPostsCount, AVG(ip.NetScore) AS AverageNetScore,
//        SUM(CASE WHEN ip.Rank <= 10 THEN 1 ELSE 0 END) AS TopTenPostsCount
// FROM ActiveUsers au LEFT JOIN ImportantPosts ip ON au.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = ip.PostId)
// GROUP BY au.UserId, au.Reputation, au.BadgeCount, au.GoldBadges, au.SilverBadges, au.BronzeBadges HAVING SUM(CASE WHEN ip.Rank <= 10 THEN 1 ELSE 0 END) > 0 ORDER BY au.Reputation DESC, ImportantPostsCount DESC;
//
// NetScore and CommentCount run over the votes x comments x links product of every post, since the global Rank reads them; a Rank tie goes to the smaller post id.
// The scalar subquery is the post's owner. LinkedPosts is never read.
fn q22822(db: &'static So) -> String {
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(links_of(db).opt()))
        .fold([0i64; 2], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64 - (t == Some(3)) as i64, a[1] + c.is_some() as i64]);
    let ip = ranked(drain((&ps).filt(|a: [i64; 2]| a[0] > 0)), |&(p, a)| (Reverse(a[0]), Reverse(a[1]), p), false);
    let ip = rel(ip.into_iter().map(|((p, a), r)| (p, (a[0], r))).collect());
    let Post { owner_user, creation_date, .. } = &db.post;
    let by_owner: HashIdx<Id<User>, (Id<Post>, (i64, i64))> = (&ip).select(Same::<(Id<Post>, (i64, i64))>::new().map(|(p, _): (Id<Post>, (i64, i64))| p).select(owner_user)).inv().select(&ip).collect();
    let active: MatSet<Id<User>> = db.post.with(creation_date.ge(add_days(current_date(), -30))).select(owner_user).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let g = (&active).group_by(Ident::<User>::new()).select((&by_owner).map(|(_, x)| x).opt()).fold([0i64; 3], |a, x| match x {
        Some((s, r)) => [a[0] + 1, a[1] + s, a[2] + (r <= 10) as i64],
        None => a,
    });
    let v = drain((&g).filt(|a: [i64; 3]| a[2] > 0).and(&ub));
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), avg(a[1], a[0]), V::I(a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// PostWithBadges AS (SELECT rp.PostId, rp.Title, rp.CreationDate, u.DisplayName AS OwnerDisplayName, COALESCE(b.BadgeCount, 0) AS BadgeCount FROM RankedPosts rp
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON rp.OwnerUserId = b.UserId JOIN Users u ON u.Id = rp.OwnerUserId),
// CommentStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN c.Score > 0 THEN 1 ELSE 0 END) AS PositiveCommentCount FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '60 days' GROUP BY p.Id),
// Final AS (SELECT pwb.PostId, pwb.Title, pwb.CreationDate, pwb.OwnerDisplayName, pwb.BadgeCount, cs.CommentCount, cs.PositiveCommentCount,
//        CASE WHEN cs.CommentCount IS NULL THEN 'No Comments' WHEN cs.PositiveCommentCount * 1.0 / cs.CommentCount < 0.5 THEN 'Mostly Negative' ELSE 'Mixed or Positive' END AS CommentSentiment,
//        COALESCE(SUBSTRING(pw.Body FROM 1 FOR 200), 'No body content available.') AS ShortBody
//     FROM PostWithBadges pwb LEFT JOIN CommentStats cs ON pwb.PostId = cs.PostId LEFT JOIN Posts pw ON pw.Id = pwb.PostId
//     WHERE pwb.BadgeCount > 0 AND pw.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' ORDER BY pwb.BadgeCount DESC, pwb.CreationDate ASC)
// SELECT *, CASE WHEN OwnerDisplayName IS NULL THEN 'Anonymous' ELSE OwnerDisplayName END AS DisplayName FROM Final WHERE CommentSentiment NOT IN ('No Comments') ORDER BY CommentCount DESC, CreationDate ASC;
//
// Rank is never read. The WHERE asks for posts created both on or after and before the same instant, so the answer is empty whatever the data; the port still
// computes it as written.
fn q23860(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, owner_user, body, .. } = &db.post;
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let cs = db.post.with(creation_date.ge(add_days(t0, -60))).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + (s > 0) as i64],
        None => a,
    });
    let v = drain(
        db.post
            .with(creation_date.ge(add_days(t0, -30)))
            .with(creation_date.lt(add_days(t0, -30)))
            .select(owner_user.select(Ident::<User>::new().and((&bc).filt(|n| n > 0))).and((&cs).opt())),
    );
    rows(v.into_iter().map(|(p, ((u, b), c))| {
        let a = c.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(b), oint(c.map(|a| a[0])), oint(c.map(|a| a[1]))]);
        f.push(V::S(if c.is_none() { "No Comments" } else if (a[1] as f64 * 1.0 / a[0] as f64) < 0.5 { "Mostly Negative" } else { "Mixed or Positive" }));
        f.push(V::Owned(body.get(p).unwrap().chars().take(200).collect()));
        f.push(user_col(db, u, "name"));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore,
//        COUNT(c.Id) AS CommentCount, COALESCE(AVG(v.BountyAmount), 0) AS AvgBounty, T.TagName
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Tags T ON T.WikiPostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id AND v.VoteTypeId = 8
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.PostTypeId, T.TagName),
// RecentPostHistory AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate AS HistoryDate, ph.Comment, DENSE_RANK() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12, 13, 14) AND ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months'),
// CombinedPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.RankScore, rp.CommentCount, rp.AvgBounty, COALESCE(rph.PostHistoryTypeId, 0) AS LastAction,
//        COALESCE(rph.HistoryDate, DATE '9999-12-31') AS LastActionDate FROM RankedPosts rp LEFT JOIN RecentPostHistory rph ON rp.PostId = rph.PostId AND rph.HistoryRank = 1)
// SELECT cp.PostId, cp.Title, cp.Score, cp.ViewCount, cp.RankScore, cp.CommentCount, cp.AvgBounty, cp.LastAction, cp.LastActionDate,
//        CASE WHEN cp.LastAction IS NULL THEN 'No history' WHEN cp.LastAction IN (10, 12) THEN 'Closed/Deleted' WHEN cp.LastAction = 11 THEN 'Reopened' ELSE 'Locked' END AS PostStatus,
//        CASE WHEN cp.LastActionDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' THEN 'Recently Active' ELSE 'Inactive' END AS ActivityStatus
// FROM CombinedPosts cp WHERE cp.RankScore <= 5 ORDER BY cp.RankScore;
//
// The GROUP BY includes the Tags join's TagName, so a post is one group per tag whose wiki it is (one NULL group when it is none). RankScore reads only base
// columns, so the top five per type are picked first (a tie goes to the smaller (post, tag) id). HistoryRank = 1 is every history row at the latest instant.
fn q23928(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let wiki: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let g = drain(db.post.with(creation_date.ge(add_years(t0, -1)).and(post_type_id.is_in([1, 2]))).select(post_type_id.and((&wiki).opt())));
    let top = top_per(g, |&(_, (t, _))| t, |&(p, (_, w))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, w.map(|t| db.tag.tag_name.get(t).unwrap())), 5, false);
    let ranked_ = per_group(ranked(top, |&(p, (t, w))| (t, Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, w.map(|t| db.tag.tag_name.get(t).unwrap())), false), |&(_, (t, _))| t);
    let rp = rel(ranked_.into_iter().map(|((p, _), r)| (p, r)).collect());
    let sel: MatSet<Id<Post>> = (&rp).map(|(p, _)| p).collect();
    let bounty2 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let st = (&sel).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty2.opt())).fold([0i64; 3], |a, (c, b)| {
        let b = b.flatten();
        [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let recent = || db.post_history.with(post_history_type_id.is_in([10, 11, 12, 13, 14]).and(hd.ge(add_months(t0, -6))));
    let md = recent().group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = recent().select(post.and(hd)).inv().collect();
    type R = (Id<Post>, i64);
    let v = drain((&rp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&st).and(Ident::<Post>::new().and(&md).select(&at).opt())))));
    rows(v.into_iter().map(|(_, ((p, r), (a, h)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(r), V::I(a[0]), if a[1] == 0 { V::F(0.0) } else { avg(a[2], a[1]) }]);
        let (t, d) = h.map_or((0, date(9999, 12, 31)), |h| (post_history_type_id.get(h).unwrap(), hd.get(h).unwrap()));
        f.extend([V::I(t), V::T(d)]);
        f.push(V::S(match t {
            10 | 12 => "Closed/Deleted",
            11 => "Reopened",
            _ => "Locked",
        }));
        f.push(V::S(if d >= add_months(t0, -1) { "Recently Active" } else { "Inactive" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rnk, p.AcceptedAnswerId,
//        COALESCE((SELECT MAX(v.BountyAmount) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 8), 0) AS MaxBounty FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '365 DAYS'),
// UserWithBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistoryStats AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastClosedDate, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS LastReopenedDate
//     FROM PostHistory ph GROUP BY ph.PostId),
// FinalStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.AnswerCount, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ph.LastClosedDate, ph.LastReopenedDate, rp.MaxBounty,
//        CASE WHEN ph.LastClosedDate IS NOT NULL AND (ph.LastReopenedDate IS NULL OR ph.LastClosedDate > ph.LastReopenedDate) THEN 'Closed' ELSE 'Active' END AS PostStatus
//     FROM RankedPosts rp LEFT JOIN UserWithBadges ub ON rp.AcceptedAnswerId = ub.UserId LEFT JOIN PostHistoryStats ph ON rp.PostId = ph.PostId WHERE ub.BadgeCount > 0 OR rp.ViewCount > 1000)
// SELECT Title, PostStatus, ViewCount, AnswerCount, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, MaxBounty FROM FinalStats
// ORDER BY CASE WHEN PostStatus = 'Closed' THEN 1 ELSE 0 END, AnswerCount DESC, ViewCount DESC LIMIT 100;
//
// Rnk is never read. `rp.AcceptedAnswerId = ub.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q24253(db: &'static So) -> String {
    let Post { creation_date, accepted_answer_id, view_count, answer_count, .. } = &db.post;
    let mb = db.vote.with((&db.vote.vote_type_id).eq(8)).with(&db.vote.bounty_amount).group_by(&db.vote.post).select(&db.vote.bounty_amount).fold(i64::MIN, |m, b| m.max(b));
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold((i64::MIN, i64::MIN), |(c, r), (t, d)| (if t == 10 { c.max(d) } else { c }, if t == 11 { r.max(d) } else { r }));
    let closed = |h: Option<(i64, i64)>| h.map_or(false, |(c, r)| c != i64::MIN && (r == i64::MIN || c > r));
    type X = (Option<i64>, ((Option<[i64; 4]>, Option<(i64, i64)>), Option<i64>));
    let v = drain(
        db.post
            .with(creation_date.ge(add_days(date(2024, 10, 1), -365)))
            .select(view_count.opt().and(accepted_answer_id.select(&uidx).select(&ub).opt().and((&phs).opt()).and((&mb).opt())))
            .filt(|(w, ((b, _), _)): X| b.map_or(false, |b| b[0] > 0) || w.map_or(false, |w| w > 1000)),
    );
    let v: Vec<(Id<Post>, ((Option<[i64; 4]>, Option<(i64, i64)>), Option<i64>))> = v.into_iter().map(|(p, (_, x))| (p, x)).collect();
    let v = top_k(v, |&(p, ((_, h), _))| {
        let a = answer_count.get(p);
        let w = view_count.get(p);
        (closed(h) as i64, a.is_none(), Reverse(a), w.is_none(), Reverse(w))
    }, |&(p, _)| p, 100);
    rows(v.into_iter().map(|(p, ((b, h), m))| {
        let mut f = post_fields(db, p, &["title"]);
        f.push(V::S(if closed(h) { "Closed" } else { "Active" }));
        f.extend(post_fields(db, p, &["views", "answers"]));
        match b {
            Some(b) => f.extend(b.map(V::I)),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        f.push(V::I(m.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 3 WHEN b.Class = 2 THEN 2 WHEN b.Class = 3 THEN 1 ELSE 0 END) AS TotalBadgeScore
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostVoteCounts AS (SELECT p.Id AS PostId, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// CloseReasonSummary AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN cr.Name END) AS CloseReason FROM PostHistory ph LEFT JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(uv.Reputation, 0) AS UserReputation, COALESCE(uv.BadgeCount, 0) AS UserBadgeCount, COALESCE(uv.TotalBadgeScore, 0) AS TotalUserBadgeScore,
//        pc.UpVotes, pc.DownVotes, crs.CloseReason, CASE WHEN rp.Rank <= 3 THEN 'Top Performer' WHEN rp.Rank IS NULL THEN 'No Ranking' ELSE 'Regular Performer' END AS PerformanceCategory
// FROM RankedPosts rp LEFT JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) LEFT JOIN UserReputation uv ON u.Id = uv.UserId LEFT JOIN PostVoteCounts pc ON rp.PostId = pc.PostId
// LEFT JOIN CloseReasonSummary crs ON rp.PostId = crs.PostId WHERE (rp.Score > 10 OR rp.ViewCount > 1000) AND (crs.CloseReason IS NULL OR crs.CloseReason <> 'Exact Duplicate') ORDER BY rp.CreationDate DESC, rp.Score DESC;
//
// The scalar subquery is the post's owner. A Rank tie goes to the smaller post id.
fn q20298(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let rk = per_group(ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), false), |&(_, t)| t);
    let rk = rel(rk.into_iter().map(|((p, _), r)| (p, r)).collect());
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 2], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + match c { 1 => 3, 2 => 2, 3 => 1, _ => 0 }],
        None => a,
    });
    let pc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let crs = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(post_history_type_id.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason).opt()))
        .fold(None::<Str>, |m, (t, r)| if t == 10 { match (m, r) { (Some(a), Some(b)) => Some(a.max(b)), (a, b) => a.or(b) } } else { m });
    type R = (Id<Post>, i64);
    let v = drain(
        (&rk)
            .filt(|(p, _): R| score.get(p).unwrap() > 10 || view_count.get(p).map_or(false, |w| w > 1000))
            .select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(owner_user.select(Ident::<User>::new().and(&ur)).opt().and(&pc).and((&crs).opt()))))
            .filt(|(_, (_, c)): (R, ((Option<(Id<User>, [i64; 2])>, [i64; 2]), Option<Option<Str>>))| c.flatten().map_or(true, |c| c != "Exact Duplicate")),
    );
    rows(v.into_iter().map(|(_, ((p, r), ((u, a), c)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        match u {
            Some((u, b)) => f.extend([user_col(db, u, "rep"), V::I(b[0]), V::I(b[1])]),
            None => f.extend([V::I(0), V::I(0), V::I(0)]),
        }
        f.extend([V::I(a[0]), V::I(a[1]), ostr(c.flatten()), V::S(if r <= 3 { "Top Performer" } else { "Regular Performer" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, (SELECT AVG(v.BountyAmount) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId IN (8, 9)) AS AverageBounty, p.OwnerUserId
//     FROM Posts p WHERE p.Score > 0 AND (p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' OR p.AcceptedAnswerId IS NOT NULL)),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.AverageBounty, COALESCE(bg_class.BadgeClass, 0) AS BadgeClass, rp.OwnerUserId FROM RankedPosts rp
//     LEFT JOIN (SELECT b.UserId, MAX(b.Class) AS BadgeClass FROM Badges b GROUP BY b.UserId) bg_class ON bg_class.UserId = rp.OwnerUserId),
// CombinedData AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.CommentCount, pd.AverageBounty, pd.BadgeClass, COUNT(pl.RelatedPostId) AS RelatedPostsCount FROM PostDetails pd LEFT JOIN PostLinks pl ON pl.PostId = pd.PostId
//     GROUP BY pd.PostId, pd.Title, pd.CreationDate, pd.CommentCount, pd.AverageBounty, pd.BadgeClass)
// SELECT cd.PostId, cd.Title, cd.CreationDate, cd.CommentCount, cd.AverageBounty, cd.BadgeClass, cd.RelatedPostsCount,
//        CASE WHEN cd.CommentCount = 0 THEN 'No Comments' WHEN cd.CommentCount BETWEEN 1 AND 5 THEN 'Few Comments' WHEN cd.CommentCount BETWEEN 6 AND 20 THEN 'Moderate Comments' ELSE 'Many Comments' END AS CommentCategory,
//        CASE WHEN cd.AverageBounty IS NULL THEN 'No Bounty' WHEN cd.AverageBounty < 50 THEN 'Low Bounty' WHEN cd.AverageBounty BETWEEN 51 AND 150 THEN 'Medium Bounty' ELSE 'High Bounty' END AS BountyCategory
// FROM CombinedData cd WHERE cd.BadgeClass IN (1, 2) ORDER BY cd.CommentCount DESC, cd.Title;
//
// PostRank is never read. BadgeClass is the owner's highest class number, which is 3 for anyone with a bronze badge.
fn q22334(db: &'static So) -> String {
    let Post { score, creation_date, accepted_answer_id, owner_user, .. } = &db.post;
    let bc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold(0i64, |m, c| m.max(c));
    let recent = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let acc = Ident::<Post>::new().with(accepted_answer_id);
    let cls = owner_user.select(&bc).opt().map(|c: Option<i64>| c.unwrap_or(0));
    let posts: MatSet<Id<Post>> = db.post.with(score.gt(0)).with(recent.or(acc)).with(cls.is_in([1, 2])).collect();
    let cc = (&posts).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ab = (&posts).group_by(Ident::<Post>::new()).select(bounty.opt()).fold([0i64; 2], |a, b| match b.flatten() {
        Some(b) => [a[0] + 1, a[1] + b],
        None => a,
    });
    let rl = (&posts).group_by(Ident::<Post>::new()).select(links_of(db).opt()).fold(0i64, |n, l| n + l.is_some() as i64);
    let cls2 = owner_user.select(&bc).opt().map(|c: Option<i64>| c.unwrap_or(0));
    let v = drain((&cc).and(&ab).and(&rl).and(cls2));
    rows(v.into_iter().map(|(p, (((c, b), l), k))| {
        let avgb = if b[0] == 0 { None } else { Some(b[1] as f64 / b[0] as f64) };
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), ofloat(avgb), V::I(k), V::I(l)]);
        f.push(V::S(if c == 0 { "No Comments" } else if c <= 5 { "Few Comments" } else if c <= 20 { "Moderate Comments" } else { "Many Comments" }));
        f.push(V::S(match avgb {
            None => "No Bounty",
            Some(x) if x < 50.0 => "Low Bounty",
            Some(x) if (51.0..=150.0).contains(&x) => "Medium Bounty",
            _ => "High Bounty",
        }));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("20120", q20120),
    ("30915", q30915),
    ("3839", q3839),
    ("30911", q30911),
    ("1986", q1986),
    ("21467", q21467),
    ("34118", q34118),
    ("4593", q4593),
    ("317", q317),
    ("22840", q22840),
    ("30061", q30061),
    ("522", q522),
    ("31680", q31680),
    ("20493", q20493),
    ("3121", q3121),
    ("33306", q33306),
    ("32655", q32655),
    ("34736", q34736),
    ("20026", q20026),
    ("22496", q22496),
    ("3462", q3462),
    ("23062", q23062),
    ("33707", q33707),
    ("22774", q22774),
    ("31772", q31772),
    ("23639", q23639),
    ("21827", q21827),
    ("30509", q30509),
    ("302", q302),
    ("23456", q23456),
    ("20757", q20757),
    ("20653", q20653),
    ("23451", q23451),
    ("28524", q28524),
    ("30261", q30261),
    ("20911", q20911),
    ("31432", q31432),
    ("24897", q24897),
    ("2575", q2575),
    ("3436", q3436),
    ("20889", q20889),
    ("20680", q20680),
    ("30454", q30454),
    ("31374", q31374),
    ("22467", q22467),
    ("3730", q3730),
    ("621", q621),
    ("23216", q23216),
    ("33983", q33983),
    ("33605", q33605),
    ("22735", q22735),
    ("23317", q23317),
    ("20959", q20959),
    ("22494", q22494),
    ("21267", q21267),
    ("20152", q20152),
    ("21870", q21870),
    ("1309", q1309),
    ("24866", q24866),
    ("26135", q26135),
    ("24864", q24864),
    ("32949", q32949),
    ("9883", q9883),
    ("1717", q1717),
    ("4465", q4465),
    ("34541", q34541),
    ("23468", q23468),
    ("22616", q22616),
    ("1957", q1957),
    ("23839", q23839),
    ("29927", q29927),
    ("1368", q1368),
    ("23762", q23762),
    ("22454", q22454),
    ("22160", q22160),
    ("1401", q1401),
    ("23763", q23763),
    ("23846", q23846),
    ("21011", q21011),
    ("31917", q31917),
    ("22541", q22541),
    ("33533", q33533),
    ("23754", q23754),
    ("20489", q20489),
    ("22412", q22412),
    ("34283", q34283),
    ("248", q248),
    ("21542", q21542),
    ("30518", q30518),
    ("22822", q22822),
    ("23860", q23860),
    ("23928", q23928),
    ("24253", q24253),
    ("20298", q20298),
    ("22334", q22334),
];
