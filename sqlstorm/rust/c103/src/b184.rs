use harness::prelude::*;
use std::cmp::Reverse;

fn top_k<X, K: Ord, T: Ord>(mut v: Vec<X>, k: impl Fn(&X) -> K, t: impl Fn(&X) -> T, n: usize) -> Vec<X> {
    v.sort_by(|a, b| k(a).cmp(&k(b)).then_with(|| t(a).cmp(&t(b))));
    if n > 0 && n < v.len() && k(&v[n - 1]) == k(&v[n]) {
        eprintln!("tie at the LIMIT cut");
    }
    if n > 0 {
        v.truncate(n);
    }
    v
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS RankByViews
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostCloseReasons AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount, STRING_AGG(cr.Name, ', ') AS CloseReasons
//     FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rb.BadgeCount, pc.CloseCount, pc.CloseReasons
// FROM RankedPosts rp LEFT JOIN UserBadges rb ON rp.PostId = rb.UserId LEFT JOIN PostCloseReasons pc ON rp.PostId = pc.PostId
// WHERE rp.RankByScore <= 10 OR rp.RankByViews <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// `rp.PostId = rb.UserId` joins a post id to a user id, so it goes through the raw ids. A Score tie in RankByScore goes to the smaller post id.
// STRING_AGG has no ORDER BY; every selected post has at most one close reason name, so its order cannot be observed.
fn q2461(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let mut top = top_per(v.clone(), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    top.extend(top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, 10, true));
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let pc = db
        .post_history
        .with(post_history_type_id.in_v(vec![10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|it| {
            let n: Vec<Str> = it.into_iter().collect();
            (n.len() as i64, &*Box::leak(n.join(", ").into_boxed_str()))
        });
    let v = drain((&rp).select(origid.select(&uidx).select(&bc).opt().and((&pc).opt())));
    rows(v.into_iter().map(|(p, (b, c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.push(oint(b));
        f.extend(match c {
            Some((n, s)) => [V::I(n), V::S(s)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.Tags, P.CreationDate, U.DisplayName AS Author,
//        RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank, LENGTH(P.Body) AS BodyLength
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1),
// ProcessedTags AS (SELECT PostId, UNNEST(string_to_array(SUBSTRING(Tags FROM 2 FOR LENGTH(Tags) - 2), '><')) AS Tag FROM RankedPosts WHERE Tags IS NOT NULL),
// FilteredTags AS (SELECT Tag, COUNT(*) AS TagUsage FROM ProcessedTags GROUP BY Tag HAVING COUNT(*) > 5),
// FinalBenchmark AS (SELECT RP.PostId, RP.Title, RP.Author, RP.CreationDate, RP.BodyLength, FT.Tag, FT.TagUsage
//     FROM RankedPosts RP JOIN FilteredTags FT ON RP.PostId IN (SELECT PostId FROM ProcessedTags WHERE Tag = FT.Tag) WHERE RP.PostRank = 1)
// SELECT *, EXTRACT(EPOCH FROM (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - CreationDate)) AS AgeInSeconds
// FROM FinalBenchmark ORDER BY AgeInSeconds DESC, TagUsage DESC LIMIT 100;
//
// The IN subquery is a semi-join: one row per (post, tag) pair, however often the tag repeats in the post's list.
fn q26654(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, tags_str, body, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let usage = qs().select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let first = top_per(drain(qs().select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let hits: MatSet<(Id<Post>, (Str, i64))> =
        (&rp).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list).select(Same::<Str>::new().and((&usage).filt(|n| n > 5))))).collect();
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = top_k(drain(&hits).into_iter().map(|x| x.1).collect(), |&(p, (_, u))| (creation_date.get(p).unwrap(), Reverse(u)), |&(p, (t, _))| (p, t), 100);
    rows(v.into_iter().map(|(p, (t, u))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([V::I(body.get(p).unwrap().chars().count() as i64), V::S(t), V::I(u), V::F(secs(t0 - creation_date.get(p).unwrap()))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// FilteredTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.Tags IS NOT NULL),
// TagStats AS (SELECT Tag, COUNT(*) AS TagCount, SUM(CASE WHEN rp.ScoreRank = 1 THEN 1 ELSE 0 END) AS TopScoreQuestions
//     FROM FilteredTags ft JOIN RankedPosts rp ON ft.PostId = rp.PostId GROUP BY Tag),
// TopTags AS (SELECT Tag, TagCount, TopScoreQuestions, ROW_NUMBER() OVER (ORDER BY TopScoreQuestions DESC, TagCount DESC) AS TagRank FROM TagStats)
// SELECT tt.Tag, tt.TagCount, tt.TopScoreQuestions,
//        CONCAT('Tag ', tt.Tag, ' was used in ', tt.TagCount, ' questions and had ', tt.TopScoreQuestions, ' top scoring questions.') AS BenchmarkSummary
// FROM TopTags tt WHERE tt.TagRank <= 10 ORDER BY tt.TagCount DESC;
fn q27930(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, tags_str, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let top = top_per(drain(qs().select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let r1: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ts = qs().group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new().with(&r1).opt()).fold([0i64; 2], |a, r| [a[0] + 1, a[1] + r.is_some() as i64]);
    let v = top_k(drain(&ts), |&(_, a)| (Reverse(a[1]), Reverse(a[0])), |&(t, _)| t, 10);
    rows(v.into_iter().map(|(t, a)| {
        row(vec![V::S(t), V::I(a[0]), V::I(a[1]), V::Owned(format!("Tag {t} was used in {} questions and had {} top scoring questions.", a[0], a[1]))])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, p.OwnerUserId,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        MAX(b.Date) AS LastBadgeDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON v.UserId = u.Id LEFT JOIN Badges b ON b.UserId = u.Id
//     GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.DisplayName, us.Reputation, rp.Title, rp.ViewCount, rp.Score, us.QuestionCount, us.TotalBounty,
//        CASE WHEN us.LastBadgeDate IS NOT NULL THEN 'Has Badge' ELSE 'No Badge' END AS BadgeStatus,
//        COALESCE(rp.ViewCount / NULLIF(us.QuestionCount, 0), 0) AS AvgViewsPerQuestion
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId WHERE rp.PostRank = 1 ORDER BY us.Reputation DESC, rp.ViewCount DESC;
//
// PostRank reads only base columns, so the rank-1 questions are picked first and the questions x votes x badges product is driven for their owners alone.
fn q487(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, view_count, .. } = &db.post;
    let recent = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let top = top_per(recent, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 1, true);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&rp).select(owner_user).collect();
    let asked = || Ident::<Post>::new().with(post_type_id.eq(1));
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(asked()).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).and(badges_of(db).opt()))
        .fold([0i64; 3], |a, ((_, v), b)| {
            let v = v.flatten();
            [a[0] + v.is_some() as i64, a[1] + v.unwrap_or(0), a[2] + b.is_some() as i64]
        });
    let qc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(asked()).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&rp).select(owner_user.select(Ident::<User>::new().and(&us).and(&qc))));
    rows(v.into_iter().map(|(p, ((u, a), q))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend([V::I(q), V::I(a[1]), V::S(if a[2] > 0 { "Has Badge" } else { "No Badge" })]);
        f.push(V::F(match view_count.get(p) {
            Some(w) if q != 0 => w as f64 / q as f64,
            _ => 0.0,
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.AnswerCount, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostCloseReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON cr.Id = CAST(ph.Comment AS SMALLINT)
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT up.Id AS UserId, up.DisplayName, up.Reputation, rb.PostId, rb.Title, rb.Score, rb.CreationDate, rb.AnswerCount, cb.BadgeCount, cb.HighestBadgeClass, pcr.CloseReasons
// FROM Users up LEFT JOIN RankedPosts rb ON up.Id = rb.PostId LEFT JOIN UserBadges cb ON up.Id = cb.UserId LEFT JOIN PostCloseReasons pcr ON rb.PostId = pcr.PostId
// WHERE up.Reputation > 1000 AND (pcr.CloseReasons IS NOT NULL OR cb.BadgeCount > 0) ORDER BY up.Reputation DESC, rb.Score DESC;
//
// `up.Id = rb.PostId` joins a user id to a post id, so it goes through the raw ids; PostRank is never read. STRING_AGG has no ORDER BY; no post
// reached here has more than one close reason, so its order cannot be observed.
fn q2774(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let rb = Ident::<Post>::new().with(post_type_id.eq(1).and(score.gt(0)));
    let cb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let pcr = db
        .post_history
        .with(post_history_type_id.in_v(vec![10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|it| &*Box::leak(it.into_iter().collect::<Vec<Str>>().join(", ").into_boxed_str()));
    let joined = db
        .user
        .with((&db.user.reputation).gt(1000))
        .select((&db.user.origid).select(&pidx).select(rb.and((&pcr).opt())).opt().and(&cb))
        .filt(|(r, (n, _)): (Option<(Id<Post>, Option<Str>)>, (i64, i64))| r.map_or(false, |(_, c)| c.is_some()) || n > 0);
    let v = drain(joined);
    rows(v.into_iter().map(|(u, (r, (n, m)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(match r {
            Some((p, _)) => post_fields(db, p, &["id", "title", "score", "created", "answers"]),
            None => (0..5).map(|_| V::Null).collect(),
        });
        f.extend([V::I(n), if n == 0 { V::Null } else { V::I(m) }, r.and_then(|(_, c)| c).map_or(V::Null, V::S)]);
        row(f)
    }))
}

// WITH UserScore AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, (UpVotes - DownVotes) AS NetScore FROM UserScore WHERE PostCount > 0 ORDER BY NetScore DESC LIMIT 10),
// PostActivity AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.LastActivityDate, ROW_NUMBER() OVER (ORDER BY p.LastActivityDate DESC) AS ActivityRank
//     FROM Posts p WHERE p.ViewCount IS NOT NULL),
// RecentPostActivity AS (SELECT PostId, Title, ViewCount, LastActivityDate FROM PostActivity WHERE ActivityRank <= 5)
// SELECT tu.DisplayName AS TopUser, rpa.Title, rpa.ViewCount, rpa.LastActivityDate
// FROM TopUsers tu JOIN RecentPostActivity rpa ON tu.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rpa.PostId)
// LEFT JOIN Badges b ON tu.UserId = b.UserId AND b.Class = 1
// WHERE b.Id IS NULL OR b.Date < (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') ORDER BY rpa.ViewCount DESC;
fn q1066(db: &'static So) -> String {
    let Post { view_count, last_activity_date, owner_user, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = top_k(drain((&us).and((&pc).filt(|n| n > 0))), |&(_, (a, _))| Reverse(a[0] - a[1]), |&(u, _)| u, 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let rpa = top_k(drain(db.post.with(view_count).select(last_activity_date)), |&(_, d)| Reverse(d), |&(p, _)| p, 5);
    let rpa: MatSet<Id<Post>> = rel(rpa.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))).select(&db.badge.date);
    let cut = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let v = drain((&rpa).select(owner_user.select(Ident::<User>::new().with(&tu).and(gold.opt()))).filt(move |(_, d): (Id<User>, Option<i64>)| d.map_or(true, |d| d < cut)));
    rows(v.into_iter().map(|(p, (u, _))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "views", "activity"]));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, COALESCE(SUM(P.Score), 0) AS TotalScore, COALESCE(SUM(P.ViewCount), 0) AS TotalViews,
//        COUNT(DISTINCT CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN P.Id END) AS ClosedPosts
//     FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// UserPostStats AS (SELECT UR.UserId, UR.DisplayName, UR.Reputation, PS.PostCount, PS.TotalScore, PS.TotalViews, PS.ClosedPosts
//     FROM UserReputation UR LEFT JOIN PostStatistics PS ON UR.UserId = PS.OwnerUserId)
// SELECT U.UserId, U.DisplayName, COALESCE(UP.TotalScore, 0) AS TotalScore, COALESCE(UP.ClosedPosts, 0) AS ClosedPosts, U.Rank,
//        CASE WHEN UP.PostCount IS NULL THEN 'No Posts' WHEN UP.TotalScore = 0 THEN 'Low Engagement' ELSE 'Active Contributor' END AS ContributionStatus,
//        (SELECT COUNT(DISTINCT C.Id) FROM Comments C WHERE C.UserId = U.UserId) AS CommentCount
// FROM UserReputation U LEFT JOIN UserPostStats UP ON U.UserId = UP.UserId WHERE U.Rank <= 10 ORDER BY U.Rank;
//
// Rank reads only Reputation, so the ten users are picked first and the posts x history product is driven for them alone.
fn q3940(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let recent = || posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let ps = db.user.group_by(Ident::<User>::new()).select(recent().select(score.and(history_of(db).opt()))).fold([0i64; 2], |a, (s, _)| [a[0] + 1, a[1] + s]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).in_v(vec![10, 11])));
    let cl = db.user.group_by(Ident::<User>::new()).select(recent().with(closes)).fold(0i64, |n, _| n + 1);
    let cc = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&by_user).map(|(_, r)| r).and((&ps).opt()).and((&cl).opt()).and(&cc));
    rows(v.into_iter().map(|(u, (((r, a), c), n))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a.map_or(0, |a| a[1])), V::I(c.unwrap_or(0)), V::I(r)]);
        f.push(V::S(match a {
            None => "No Posts",
            Some(a) if a[1] == 0 => "Low Engagement",
            _ => "Active Contributor",
        }));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '5 years' AND p.ViewCount IS NOT NULL),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(b.Class), 0) AS TotalBadges, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) >= 3),
// MostActiveUsers AS (SELECT UserId, TotalBadges, PostCount, RANK() OVER (ORDER BY TotalBadges DESC) AS UserRank FROM TopUsers)
// SELECT pu.DisplayName, rp.Title, rp.ViewCount, rp.CreationDate, mu.UserRank, CASE WHEN (mu.TotalBadges > 0) THEN 'Has Badges' ELSE 'No Badges' END AS BadgeStatus,
//        CASE WHEN rp.ViewCount IS NULL THEN 'No Views' ELSE 'Has Views' END AS ViewStatus
// FROM MostActiveUsers mu JOIN RankedPosts rp ON mu.UserId = rp.OwnerUserId JOIN Users pu ON mu.UserId = pu.Id
// WHERE mu.UserRank <= 5 AND rp.Rank = 1 ORDER BY mu.UserRank, rp.ViewCount DESC;
//
// A ViewCount tie for a user's first row goes to the smaller post id.
fn q23779(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, .. } = &db.post;
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt()))
        .fold(0i64, |s, (c, _)| s + c.unwrap_or(0));
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let mu = ranked(drain((&tu).and((&pc).filt(|n| n >= 3))), |&(_, (s, _))| Reverse(s), false);
    let mu = rel(mu.into_iter().take_while(|x| x.1 <= 5).map(|((u, (s, _)), r)| (u, (s, r))).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&mu).map(|(u, _)| u).inv().select(&mu).collect();
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -5))).with(view_count).with(owner_user).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(view_count.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&rp).select(owner_user.select(&by_user)));
    rows(v.into_iter().map(|(p, (u, (s, r)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "views", "created"]));
        f.extend([V::I(r), V::S(if s > 0 { "Has Badges" } else { "No Badges" }), V::S("Has Views")]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CreationDate, u.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - interval '1 year' AND p.Score IS NOT NULL),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CreationDate, OwnerReputation FROM RankedPosts WHERE RankScore <= 10)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.OwnerReputation,
//        (SELECT AVG(ViewCount) FROM Posts WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - interval '1 year') AS AvgViews,
//        COALESCE((SELECT COUNT(DISTINCT c.Id) FROM Comments c WHERE c.PostId IN (SELECT PostId FROM TopPosts)), 0) AS TotalComments,
//        CASE WHEN tp.Score > (SELECT AVG(Score) FROM TopPosts) THEN 'Above Average' WHEN tp.Score = (SELECT AVG(Score) FROM TopPosts) THEN 'Average' ELSE 'Below Average' END AS ScoreComparison
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId WHERE ph.CreationDate > tp.CreationDate AND ph.PostHistoryTypeId IN (10, 11, 12)
// ORDER BY tp.Score DESC, tp.ViewCount DESC LIMIT 20;
//
// A Score tie at the tenth place of a type goes to the smaller post id. The WHERE on ph makes the LEFT JOIN an inner one.
fn q574(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let v = drain(db.post.with(creation_date.ge(since)).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let av = db.post.with(creation_date.ge(since)).select(view_count).fold_flat([0i64; 2], |a, w| [a[0] + 1, a[1] + w]);
    let tc = (&tp).select(comments_of(db)).fold_flat(0i64, |n, _| n + 1);
    let ts_ = (&tp).select(score).fold_flat([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let later = (&tp).select(creation_date.and(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.in_v(vec![10, 11, 12]))).select(hd)))
        .filt(|(c, d): (i64, i64)| d > c);
    let v = drain(later);
    let v = top_k(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, |&(p, (_, d))| (p, d), 20);
    rows(v.into_iter().map(|(p, _)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "score", "views", "rep"]);
        f.extend([avg(av[1], av[0]), V::I(tc)]);
        f.push(V::S(if s * ts_[0] > ts_[1] { "Above Average" } else if s * ts_[0] == ts_[1] { "Average" } else { "Below Average" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank, p.PostTypeId, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.*, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id
//     WHERE rp.PostRank <= 5 AND rp.Score > 0)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.CommentCount, fp.VoteCount, fp.OwnerDisplayName, fp.OwnerReputation, pt.Name AS PostTypeName, b.Name AS BadgeName,
//        STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
// FROM FilteredPosts fp JOIN PostTypes pt ON fp.PostTypeId = pt.Id LEFT JOIN Badges b ON b.UserId = fp.OwnerUserId AND b.Class = 1 LEFT JOIN Tags t ON t.ExcerptPostId = fp.PostId
// GROUP BY fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.CommentCount, fp.VoteCount, fp.OwnerDisplayName, fp.OwnerReputation, pt.Name, b.Name
// ORDER BY fp.Score DESC, fp.CreationDate DESC;
//
// PostRank reads only base columns, so the posts are picked first and the comment x vote product is driven for those alone. A post is the excerpt
// of at most one tag, so the STRING_AGG has at most one value and its order cannot be observed.
fn q7412(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = ranked(drain(db.post.select(post_type_id)), |&(p, t)| (t, Reverse(creation_date.get(p).unwrap())), true);
    let v = per_group(v, |&(_, t)| t);
    let top: MatSet<Id<Post>> = rel(v.into_iter().filter(|x| x.1 <= 5).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let fp: MatSet<Id<Post>> = (&top).with(score.gt(0)).with(owner_user).collect();
    let cc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))).select(&db.badge.name);
    let groups: MatSet<(Id<Post>, Option<Str>)> = (&fp).select(Ident::<Post>::new().and(owner_user.select(gold).opt())).collect();
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tags = (&fp).group_by(Ident::<Post>::new()).select((&excerpt).select(&db.tag.tag_name)).buf_fold(|it| {
        let mut n: Vec<Str> = it.into_iter().collect();
        n.sort_unstable();
        n.dedup();
        &*Box::leak(n.join(", ").into_boxed_str())
    });
    type G = (Id<Post>, Option<Str>);
    let g = rel(drain(&groups).into_iter().map(|x| x.1).collect());
    let v = drain((&g).select(Same::<G>::new().and(Same::<G>::new().map(|(p, _): G| p).select((&cc).and(&vc).and((&tags).opt())))));
    rows(v.into_iter().map(|(_, ((p, b), ((c, n), t)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(post_fields(db, p, &["owner", "rep", "type"]));
        f.extend([ostr(b), ostr(t)]);
        row(f)
    }))
}

// WITH TaggedPosts AS (SELECT p.Id AS PostId, p.Title AS PostTitle, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// RankedPosts AS (SELECT tp.PostId, tp.PostTitle, tp.Tag, ROW_NUMBER() OVER (PARTITION BY tp.Tag ORDER BY p.CreationDate DESC) AS TagRank
//     FROM TaggedPosts tp JOIN Posts p ON p.Id = tp.PostId WHERE p.ViewCount > 1000),
// RecentEdits AS (SELECT ph.PostId AS EditedPostId, MAX(ph.CreationDate) AS LastEditDate, COUNT(*) AS EditCount FROM PostHistory ph GROUP BY ph.PostId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT rp.PostId, rp.Tag, rp.PostTitle, re.LastEditDate, re.EditCount, ur.DisplayName AS UserName, ur.Reputation AS UserReputation, ur.BadgeCount AS UserBadgeCount
// FROM RankedPosts rp JOIN RecentEdits re ON rp.PostId = re.EditedPostId JOIN Posts p ON p.Id = rp.PostId JOIN Users u ON u.Id = p.OwnerUserId
// JOIN UserReputation ur ON ur.UserId = u.Id WHERE rp.TagRank <= 5 ORDER BY rp.Tag, re.LastEditDate DESC;
//
// A CreationDate tie at a tag's fifth place goes to the smaller post id.
fn q29222(db: &'static So) -> String {
    let Post { post_type_id, view_count, tags_str, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(view_count.gt(1000)).select(tags_str.flat_map(tag_list)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let re = db.post_history.group_by(post).select(hd).fold((i64::MIN, 0i64), |(m, n), d| (m.max(d), n + 1));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type R = (Id<Post>, Str);
    let r = rel(top);
    let v = drain((&r).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&re).and(owner_user.select(Ident::<User>::new().and(&bc)))))));
    rows(v.into_iter().map(|(_, ((p, t), ((d, n), (u, b))))| {
        let mut f = post_fields(db, p, &["id"]);
        f.push(V::S(t));
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::T(d), V::I(n)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') AND p.PostTypeId = 1),
// FrequentUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 10),
// QualityPosts AS (SELECT pp.PostId, pp.Title, pp.ViewCount, pp.Score, ph.Comment FROM RankedPosts pp LEFT JOIN PostHistory ph ON pp.PostId = ph.PostId AND ph.PostHistoryTypeId IN (10, 11)
//     WHERE pp.Rank = 1 AND pp.Score > 5)
// SELECT fu.DisplayName, COALESCE(qp.Title, 'No Quality Posts') AS Title, COALESCE(qp.ViewCount, 0) AS ViewCount, COALESCE(qp.Score, 0) AS Score,
//        CASE WHEN qp.Comment IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM FrequentUsers fu LEFT JOIN QualityPosts qp ON fu.UserId = qp.PostId ORDER BY fu.PostCount DESC, qp.Score DESC LIMIT 20;
//
// `fu.UserId = qp.PostId` joins a user id to a post id, so it goes through the raw ids. The ownerless questions are their own Rank partition.
fn q3152(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let v = drain(db.post.with(creation_date.ge(since).and(post_type_id.eq(1))).select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let qp: MatSet<Id<Post>> = (&first).with(score.gt(5)).collect();
    let fu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(since)))).fold(0i64, |n, _| n + 1);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).in_v(vec![10, 11])));
    let q = Ident::<Post>::new().with(&qp).and(closes.select((&db.post_history.comment).opt()).opt());
    let v = drain((&fu).filt(|n| n > 10).and((&db.user.origid).select(&pidx).select(q).opt()));
    let v = top_k(v, |&(_, (n, q))| (Reverse(n), q.is_none(), Reverse(q.map(|(p, _)| score.get(p).unwrap()))), |&(u, (_, q))| (u, q.map(|(p, _)| p)), 20);
    rows(v.into_iter().map(|(u, (_, q))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(match q {
            Some((p, c)) => [
                db.post.title.get(p).map_or(V::S("No Quality Posts"), V::S),
                V::I(db.post.view_count.get(p).unwrap_or(0)),
                V::I(score.get(p).unwrap()),
                V::S(if c.flatten().is_some() { "Closed" } else { "Open" }),
            ],
            None => [V::S("No Quality Posts"), V::I(0), V::I(0), V::S("Open")],
        });
        row(f)
    }))
}

// WITH UserRank AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U),
// PostSummary AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, P.ViewCount, P.AnswerCount, COALESCE(P.AcceptedAnswerId IS NOT NULL, FALSE) AS IsAccepted,
//        T.TagName, U.DisplayName AS OwnerDisplayName
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id JOIN UNNEST(string_to_array(P.Tags, '>')) AS T(TagName) ON TRUE WHERE P.PostTypeId = 1),
// MostActiveUsers AS (SELECT PostSummary.OwnerDisplayName, COUNT(PostSummary.PostId) AS QuestionCount, SUM(PostSummary.ViewCount) AS TotalViews,
//        SUM(PostSummary.AnswerCount) AS TotalAnswers, R.UserRank
//     FROM PostSummary JOIN UserRank R ON PostSummary.OwnerDisplayName = R.DisplayName GROUP BY PostSummary.OwnerDisplayName, R.UserRank),
// TopActiveUsers AS (SELECT OwnerDisplayName, QuestionCount, TotalViews, TotalAnswers, RANK() OVER (ORDER BY QuestionCount DESC, TotalViews DESC) AS ActivityRank FROM MostActiveUsers)
// SELECT OwnerDisplayName, QuestionCount, TotalViews, TotalAnswers, ActivityRank FROM TopActiveUsers WHERE ActivityRank <= 10 ORDER BY ActivityRank;
//
// Splitting '<a><b>' on '>' gives '<a', '<b' and a trailing '', so each question has one row per tag plus one. The name join pairs each question
// with every user of its owner's name, and the group is (name, that user's rank).
fn q27292(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, view_count, answer_count, .. } = &db.post;
    let urank = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let urank: HashIdx<Id<User>, i64> = (&urank).map(|(u, _)| u).inv().select((&urank).map(|(_, r)| r)).collect();
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let j: MatSet<(Id<Post>, Id<User>)> =
        db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name).select(&by_name))).collect();
    let post_of = (&j).map(|(p, _)| p);
    let user_of = (&j).map(|(_, u)| u);
    let g = (&j)
        .group_by((&user_of).select((&db.user.display_name).and(&urank)))
        .select((&post_of).select(tags_str.flat_map(|t: Str| t.split('>')).and(view_count.opt()).and(answer_count.opt())))
        .fold([0i64; 5], |a, ((_, w), n)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + n.is_some() as i64, a[4] + n.unwrap_or(0)]);
    let v = ranked(drain(&g), |&(_, a)| (Reverse(a[0]), a[1] == 0, Reverse(a[2])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|(((name, _), a), r)| row(vec![V::S(name), V::I(a[0]), nullable(a[2], a[1]), nullable(a[4], a[3]), V::I(r)])))
}

// WITH UserVoteStats AS (SELECT UserId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN VoteTypeId = 1 THEN 1 END) AS AcceptedVotes, COUNT(DISTINCT PostId) AS TotalVotes FROM Votes GROUP BY UserId),
// RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostsWithVotes AS (SELECT rp.Title, rp.CreationDate, u.DisplayName, COALESCE(uvs.UpVotes, 0) AS UpVotes, COALESCE(uvs.DownVotes, 0) AS DownVotes,
//        COALESCE(uvs.AcceptedVotes, 0) AS AcceptedVotes
//     FROM RecentPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserVoteStats uvs ON u.Id = uvs.UserId WHERE rp.PostRank = 1)
// SELECT pwv.Title, pwv.CreationDate, pwv.DisplayName, pwv.UpVotes, pwv.DownVotes, pwv.AcceptedVotes,
//        CASE WHEN pwv.UpVotes - pwv.DownVotes > 10 THEN 'High Engagement' WHEN pwv.UpVotes - pwv.DownVotes BETWEEN 1 AND 10 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM PostsWithVotes pwv WHERE pwv.UpVotes IS NOT NULL OR pwv.DownVotes IS NOT NULL ORDER BY pwv.CreationDate DESC LIMIT 50;
//
// The ownerless posts are their own PostRank partition. After the COALESCEs the WHERE holds for every row.
fn q4492(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uvs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 1) as i64]);
    let v = drain((&rp).select(owner_user.select(Ident::<User>::new().and((&uvs).opt())).opt()));
    let v = top_k(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), |&(p, _)| p, 50);
    rows(v.into_iter().map(|(p, u)| {
        let a = u.and_then(|(_, a)| a).unwrap_or([0; 3]);
        let net = a[0] - a[1];
        let mut f = post_fields(db, p, &["title", "created"]);
        f.push(u.map_or(V::Null, |(u, _)| user_col(db, u, "name")));
        f.extend(a.map(V::I));
        f.push(V::S(if net > 10 { "High Engagement" } else if (1..=10).contains(&net) { "Moderate Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.AnswerCount, p.ViewCount, u.DisplayName AS Author,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2)),
// AggregatedTags AS (SELECT t.TagName, COUNT(pt.Id) AS PostCount, SUM(pt.ViewCount) AS TotalViews FROM Tags t JOIN Posts pt ON pt.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// RecentEdits AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.UserDisplayName, ph.CreationDate, STRING_AGG(DISTINCT p.Title, ', ') AS RelatedPosts
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId IN (4, 5, 24) GROUP BY ph.PostId, ph.PostHistoryTypeId, ph.UserDisplayName, ph.CreationDate)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Author, rp.AnswerCount, rp.ViewCount, ag.TagName, ag.PostCount, ag.TotalViews, re.UserDisplayName AS Editor,
//        re.CreationDate AS EditDate, re.RelatedPosts
// FROM RankedPosts rp LEFT JOIN AggregatedTags ag ON ag.PostCount > 0 LEFT JOIN RecentEdits re ON re.PostId = rp.PostId WHERE rp.PostRank = 1 ORDER BY rp.CreationDate DESC;
//
// The first ON names only ag, so the newest question and answer are crossed with every tag that matches a post. A CreationDate tie for a type's
// newest post goes to the smaller id. Every row of a RecentEdits group is the same post, so RelatedPosts is that post's title.
fn q26537(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, title, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2])).with(owner_user).select(post_type_id));
    let first = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tg = rel(drain((&tag_stats(db)).filt(|a| a[0] > 0)));
    let PostHistory { post, post_history_type_id, user_display_name, creation_date: hd, .. } = &db.post_history;
    let re = db
        .post_history
        .with(post_history_type_id.in_v(vec![4, 5, 24]))
        .group_by(post.and(post_history_type_id).and(user_display_name.opt()).and(hd))
        .select(post.select(title.opt()))
        .buf_fold(|it| {
            let mut n: Vec<Str> = it.into_iter().flatten().collect();
            n.sort_unstable();
            n.dedup();
            if n.is_empty() { None } else { Some(&*Box::leak(n.join(", ").into_boxed_str())) }
        });
    let rv = rel(drain(&re));
    type K = (((Id<Post>, i64), Option<Str>), i64);
    let by_post: HashIdx<Id<Post>, (K, Option<Str>)> = (&rv).map(|((((p, _), _), _), _): (K, Option<Str>)| p).inv().select(&rv).collect();
    let left = rel(drain((&rp).select((&by_post).opt())));
    let mut v = Vec::new();
    (&left).cross(&tg).drive(|_, ((p, e), (t, a))| v.push((p, e, t, a)));
    rows(v.into_iter().map(|(p, e, t, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "answers", "views"]);
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), nullable(a[2], a[1])]);
        f.extend(match e {
            Some(((((_, _), u), d), r)) => [ostr(u), V::T(d), ostr(r)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, U.DisplayName AS OwnerDisplayName, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.Tags,
//        ROW_NUMBER() OVER (PARTITION BY U.Location ORDER BY P.Score DESC, P.ViewCount DESC) AS RankByPerformance
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.Score > 10 AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AggregatedData AS (SELECT U.Location, COUNT(RP.PostId) AS NumberOfQuestions, AVG(RP.Score) AS AverageScore, SUM(RP.AnswerCount) AS TotalAnswers, SUM(RP.ViewCount) AS TotalViews
//     FROM RankedPosts RP JOIN Users U ON RP.OwnerDisplayName = U.DisplayName GROUP BY U.Location),
// PopularTags AS (SELECT DISTINCT UNNEST(string_to_array(RP.Tags, '><')) AS Tag FROM RankedPosts RP WHERE RankByPerformance <= 5),
// TagPopularity AS (SELECT Tag, COUNT(*) AS TagCount FROM PopularTags GROUP BY Tag)
// SELECT AD.Location, AD.NumberOfQuestions, AD.AverageScore, AD.TotalAnswers, AD.TotalViews, TP.Tag, TP.TagCount
// FROM AggregatedData AD JOIN TagPopularity TP ON AD.Location = TP.Tag ORDER BY AD.NumberOfQuestions DESC, AD.AverageScore DESC LIMIT 10;
//
// The owner-name join pairs each question with every user of that name, and AggregatedData groups by that user's Location. A tie at a
// location's fifth place goes to the smaller post id.
fn q28296(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, view_count, answer_count, tags_str, .. } = &db.post;
    let rp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1).and(score.gt(10)).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .with(owner_user)
        .collect();
    let v = drain((&rp).select(owner_user.select((&db.user.location).opt())));
    let top = top_per(v, |&(_, l)| l, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let r5: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tags: MatSet<Str> = (&r5).select(tags_str.flat_map(|t: Str| t.split("><"))).collect();
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let j: MatSet<(Id<Post>, Id<User>)> = (&rp).select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name).select(&by_name))).collect();
    let post_of = (&j).map(|(p, _)| p);
    let user_of = (&j).map(|(_, u)| u);
    let ad = (&j)
        .group_by((&user_of).select((&db.user.location).opt()))
        .select((&post_of).select(score.and(answer_count.opt()).and(view_count.opt())))
        .fold([0i64; 6], |a, ((s, n), w)| [a[0] + 1, a[1] + s, a[2] + n.is_some() as i64, a[3] + n.unwrap_or(0), a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]);
    let v = drain((&tags).map(|t| Some(t)).select(&ad));
    let v = top_k(v, |&(_, a)| (Reverse(a[0]), Reverse(fkey(a[1] as f64 / a[0] as f64))), |&(t, _)| t, 10);
    rows(v.into_iter().map(|(t, a)| row(vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), nullable(a[5], a[4]), V::S(t), V::I(1)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.ViewCount IS NOT NULL),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges,
//        COUNT(DISTINCT v.PostId) AS VoteCount, AVG(v.BountyAmount) AS AvgBounty
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '2 years'
//     GROUP BY u.Id, u.DisplayName)
// SELECT us.DisplayName, up.PostId, up.Title, up.ViewCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges, us.VoteCount, us.AvgBounty,
//        CASE WHEN up.rn = 1 THEN 'Top Post' ELSE 'Other Post' END AS PostRank
// FROM UserStats us JOIN RankedPosts up ON us.UserId = up.PostId LEFT JOIN Comments c ON up.PostId = c.PostId
// WHERE us.VoteCount > 5 AND c.CreationDate IS NULL ORDER BY us.GoldBadges DESC, up.ViewCount DESC;
//
// `us.UserId = up.PostId` joins a user id to a post id, so it goes through the raw ids. `c.CreationDate IS NULL` after the LEFT JOIN keeps the
// posts with no comment. Only the users whose id is such a post's id can reach the output, so the badges x votes product is driven for them alone.
// A Score tie for an owner's first post goes to the smaller id.
fn q3837(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(view_count);
    let first = top_per(drain(recent().select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let up: MatSet<Id<Post>> = recent().minus(comments_of(db)).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let old = Ident::<User>::new().with((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -2)));
    let cand: MatSet<Id<User>> = (&up).select((&db.post.origid).select(&uidx).select(old)).collect();
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 5], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
        });
    let vc = (&cand).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.post_id)).buf_fold(|it| {
        let mut x: Vec<i64> = it.into_iter().collect();
        x.sort_unstable();
        x.dedup();
        x.len() as i64
    });
    let v = drain((&up).select((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&us).and((&vc).filt(|n| n > 5)))).and(Ident::<Post>::new().with(&first).opt()));
    let mut v = v;
    v.sort_by_key(|&(p, (((_, a), _), _))| (Reverse(a[0]), Reverse(view_count.get(p))));
    rows(v.into_iter().map(|(p, (((u, a), n), t))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n), if a[3] == 0 { V::Null } else { V::F(a[4] as f64 / a[3] as f64) }]);
        f.push(V::S(if t.is_some() { "Top Post" } else { "Other Post" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounties
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON U.Id = V.UserId AND V.VoteTypeId = 9 WHERE U.Reputation > 1000 GROUP BY U.Id, U.Reputation),
// RecentPostActivity AS (SELECT P.Id AS PostId, P.OwnerUserId, COUNT(C) AS CommentCount, MAX(P.LastActivityDate) AS LastActivity,
//        SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, P.OwnerUserId),
// RankedPosts AS (SELECT R.OwnerUserId, R.PostId, R.CommentCount, R.LastActivity, R.CloseCount,
//        ROW_NUMBER() OVER (PARTITION BY R.OwnerUserId ORDER BY R.CommentCount DESC, R.LastActivity DESC) AS Rank FROM RecentPostActivity R)
// SELECT U.Id AS UserId, U.DisplayName, UR.Reputation, UR.PostCount, UR.TotalBounties, RP.PostId, RP.CommentCount, RP.LastActivity, RP.CloseCount
// FROM Users U JOIN UserReputation UR ON U.Id = UR.UserId LEFT JOIN RankedPosts RP ON U.Id = RP.OwnerUserId WHERE RP.Rank <= 5 OR RP.Rank IS NULL
// ORDER BY UR.Reputation DESC, RP.CommentCount DESC NULLS LAST;
//
// `COUNT(C)` counts the whole row of C, which a LEFT JOIN makes non-NULL even when unmatched, so it is the comment x history row count.
// A tie for an owner's fifth place goes to the smaller post id.
fn q193(db: &'static So) -> String {
    let Post { creation_date, owner_user, last_activity_date, .. } = &db.post;
    let users: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(1000)).collect();
    let bounty = votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let ur = (&users).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(bounty.opt())).fold(0i64, |s, (_, b)| s + b.flatten().unwrap_or(0));
    let pc = (&users).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let recent = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user.select(Ident::<User>::new().with(&users)));
    let rpa = recent
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + 1, a[1] + (t == Some(10)) as i64]);
    let v = drain((&rpa).and(owner_user));
    let top = top_per(v, |&(_, (_, u))| u, |&(p, (a, _))| (Reverse(a[0]), Reverse(last_activity_date.get(p).unwrap()), p), 5, false);
    let tr = rel(top.into_iter().map(|(p, (a, u))| (u, (p, a))).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, (Id<Post>, [i64; 2]))> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let v = drain((&ur).and(&pc).and((&by_user).map(|(_, x)| x).opt()));
    rows(v.into_iter().map(|(u, ((b, n), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(b)]);
        f.extend(match r {
            Some((p, a)) => [V::I(db.post.origid.get(p).unwrap()), V::I(a[0]), V::T(last_activity_date.get(p).unwrap()), V::I(a[1])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT ur.UserId, ur.Reputation, ur.BadgeCount FROM UserReputation ur WHERE ur.Reputation > 1000),
// PostDetails AS (SELECT rp.Id AS PostId, rp.Title, rp.ViewCount, rp.CreationDate, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName, u.Reputation AS OwnerReputation
//     FROM RankedPosts rp LEFT JOIN Users u ON rp.Id = u.Id WHERE rp.rn = 1)
// SELECT pd.PostId, pd.Title, pd.ViewCount, pd.OwnerDisplayName, pd.OwnerReputation, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = pd.PostId) AS CommentCount,
//        (SELECT STRING_AGG(t.TagName, ', ') FROM Tags t WHERE t.WikiPostId = pd.PostId) AS Tags
// FROM PostDetails pd JOIN TopUsers tu ON pd.OwnerReputation = tu.Reputation WHERE pd.ViewCount > (SELECT AVG(ViewCount) FROM Posts)
// ORDER BY pd.ViewCount DESC LIMIT 10 OFFSET 5;
//
// `rp.Id = u.Id` joins a post id to a user id, so it goes through the raw ids, and `pd.OwnerReputation = tu.Reputation` pairs each post with every
// user of that reputation. A post is the wiki of at most one tag, so the STRING_AGG order cannot be observed. The ownerless questions are their own rn partition.
fn q2733(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let pd: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let av = db.post.select(view_count).fold_flat([0i64; 2], |a, w| [a[0] + 1, a[1] + w]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let by_rep: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation).inv().collect();
    let cc = (&pd).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let wiki: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let tags = (&pd).group_by(Ident::<Post>::new()).select((&wiki).select(&db.tag.tag_name)).buf_fold(|it| &*Box::leak(it.into_iter().collect::<Vec<Str>>().join(", ").into_boxed_str()));
    let owner = (&db.post.origid).select(&uidx);
    let v = drain(
        (&pd)
            .with(view_count.filt(move |w| w * av[0] > av[1]))
            .select(owner.select(Ident::<User>::new().and((&db.user.reputation).select(&by_rep))))
            .and(&cc)
            .and((&tags).opt()),
    );
    let v = top_k(v, |&(p, _)| Reverse(view_count.get(p)), |&(p, ((_, t), _))| (p, t), 15);
    rows(v.into_iter().skip(5).map(|(p, (((u, _), c), t))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(c), ostr(t)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT B.Id) AS TotalBadges, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views),
// PostActivity AS (SELECT P.OwnerUserId, COUNT(*) AS PostCount, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers, AVG(P.Score) AS AvgScore
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId)
// SELECT U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes, U.TotalBadges, U.ReputationRank, COALESCE(PA.PostCount, 0) AS TotalPosts,
//        COALESCE(PA.AcceptedAnswers, 0) AS TotalAcceptedAnswers, COALESCE(PA.AvgScore, 0) AS AverageScore
// FROM UserStatistics U LEFT JOIN PostActivity PA ON U.UserId = PA.OwnerUserId
// WHERE U.Reputation > (SELECT AVG(Reputation) FROM UserStatistics) AND (U.Views IS NULL OR U.Views > 1000) ORDER BY U.Reputation DESC, U.Views DESC LIMIT 100;
//
// ReputationRank reads only Reputation, and the WHERE only base columns, so the users are picked first and the votes x badges product is
// driven for them alone. A Reputation tie in ReputationRank goes to the smaller user id.
fn q2188(db: &'static So) -> String {
    let User { reputation, views, .. } = &db.user;
    let rr = ranked(drain(reputation), |&(u, r)| (Reverse(r), u), false);
    let rr = rel(rr.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, i64> = (&rr).map(|(u, _)| u).inv().select((&rr).map(|(_, r)| r)).collect();
    let ar = db.user.select(reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let cand: MatSet<Id<User>> = db.user.with(reputation.filt(move |r| r * ar[0] > ar[1])).with(views.gt(1000)).collect();
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let tb = (&cand).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { creation_date, accepted_answer, score, owner_user, .. } = &db.post;
    let pa = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(accepted_answer.opt().and(score))
        .fold([0i64; 3], |a, (x, s)| [a[0] + 1, a[1] + x.is_some() as i64, a[2] + s]);
    let v = drain((&us).and(&tb).and(&rank).and((&pa).opt()));
    let v = top_k(v, |&(u, _)| (Reverse(reputation.get(u).unwrap()), Reverse(views.get(u).unwrap())), |&(u, _)| u, 100);
    rows(v.into_iter().map(|(u, (((a, b), r), p))| {
        let mut f = ucols(db, u, &["name", "rep", "uviews"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b), V::I(r)]);
        f.extend(match p {
            Some(p) => [V::I(p[0]), V::I(p[1]), avg(p[2], p[0])],
            None => [V::I(0), V::I(0), V::F(0.0)],
        });
        row(f)
    }))
}

// WITH RecentPostData AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(COUNT(DISTINCT c.Id), 0) AS CommentCount, COALESCE(pt.Name, 'Unknown') AS PostType,
//        p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY p.Id, p.OwnerUserId, pt.Name, p.Title, p.CreationDate),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation IS NOT NULL)
// SELECT up.UserId, up.DisplayName, up.Reputation, COUNT(rpd.PostId) AS RecentPostCount, SUM(rpd.UpVotes) AS TotalUpVotes, SUM(rpd.DownVotes) AS TotalDownVotes,
//        SUM(rpd.CommentCount) AS TotalComments
// FROM UserReputation up LEFT JOIN RecentPostData rpd ON up.UserId = rpd.OwnerUserId WHERE up.UserRank <= 10 GROUP BY up.UserId, up.DisplayName, up.Reputation ORDER BY up.Reputation DESC;
//
// UserRank reads only Reputation, so the ten users are picked first and each recent post's votes x comments product is driven for them alone.
fn q602(db: &'static So) -> String {
    let tu = top_k(drain(&db.user.reputation), |&(_, r)| Reverse(r), |&(u, _)| u, 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let recent = posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let rpd = (&tu).select(recent).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 2], |a, (t, _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let cc = (&tu).select(posts_of(db)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let recent = posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let g = (&tu).group_by(Ident::<User>::new()).select(recent.select((&rpd).and(&cc)).opt()).fold([0i64; 4], |a, x| match x {
        Some((r, c)) => [a[0] + 1, a[1] + r[0], a[2] + r[1], a[3] + c],
        None => a,
    });
    let v = drain(&g);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), nullable(a[1], a[0]), nullable(a[2], a[0]), nullable(a[3], a[0])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounties, SUM(CASE WHEN V.UserId IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount,
//        RANK() OVER (ORDER BY COUNT(DISTINCT P.Id) DESC) AS ActivityRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalBounties, VoteCount FROM UserActivity WHERE ActivityRank <= 10)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.TotalBounties, TU.VoteCount, P.Title AS LastPostTitle, P.CreationDate AS LastPostDate,
//        P.ViewCount AS LastPostViewCount
// FROM TopUsers TU LEFT JOIN Posts P ON TU.UserId = P.OwnerUserId WHERE P.LastActivityDate = (SELECT MAX(P2.LastActivityDate) FROM Posts P2 WHERE P2.OwnerUserId = TU.UserId)
// ORDER BY TU.Reputation DESC;
//
// ActivityRank reads only the distinct post count, so the top users are picked first and the posts x votes product is driven for them alone.
fn q5059(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, .. } = &db.post;
    let newer = db.user.with((&db.user.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pc = newer.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Vote { bounty_amount, user, .. } = &db.vote;
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(bounty_amount.opt().and(user.opt())).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + v.and_then(|v| v.0).unwrap_or(0), a[3] + v.map_or(false, |v| v.1.is_some()) as i64],
            None => a,
        });
    let ml = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(last_activity_date)).fold(i64::MIN, |m, d| m.max(d));
    let last = (&tu).select(Ident::<User>::new().and(&ml).and(posts_of(db).select(Ident::<Post>::new().and(last_activity_date)))).filt(|((_, m), (_, d)): ((Id<User>, i64), (Id<Post>, i64))| d == m);
    let v = drain(last.select(Same::<((Id<User>, i64), (Id<Post>, i64))>::new().and(Same::<((Id<User>, i64), (Id<Post>, i64))>::new().map(|((u, _), _)| u).select((&pc).and(&ua)))));
    let mut v = v;
    v.sort_by_key(|&(_, (((u, _), _), _))| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(_, (((u, _), (p, _)), (n, a)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostWithFlags AS (SELECT p.Id AS PostId, p.Title, p.Score, CASE WHEN postFlags.AggrFlag > 0 THEN 'Flagged' ELSE 'Not Flagged' END AS FlagStatus,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS LatestPostRank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS AggrFlag FROM Votes WHERE VoteTypeId IN (10, 12) GROUP BY PostId) AS postFlags ON p.Id = postFlags.PostId
//     WHERE p.CreationDate > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'))
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalUpvotes, us.TotalDownvotes, us.PostCount, pwf.PostId, pwf.Title, pwf.Score, pwf.FlagStatus
// FROM UserStats us FULL OUTER JOIN PostWithFlags pwf ON us.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pwf.PostId LIMIT 1)
// WHERE us.Reputation IS NOT NULL OR pwf.PostId IS NOT NULL ORDER BY us.Reputation DESC, pwf.Score DESC FETCH FIRST 50 ROWS ONLY;
//
// The LIMIT 1 subquery looks a post up by its id, so it is the post's owner. The FULL OUTER JOIN is the users' LEFT JOIN to their recent posts
// in union with the recent posts whose owner is not in UserStats. The WHERE holds for every row of it.
fn q21405(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let pos = || Ident::<User>::new().with((&db.user.reputation).gt(0));
    let us = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let flagged: MatSet<Id<Post>> = db.vote.with((&db.vote.vote_type_id).is_in([10, 12])).select(&db.vote.post).collect();
    let since = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let pwf = posts_of(db).select(Ident::<Post>::new().with(creation_date.gt(since)));
    let flag = || Ident::<Post>::new().and(Ident::<Post>::new().with(&flagged).opt());
    type P = Option<(Id<Post>, Option<Id<Post>>)>;
    type U = Option<(Id<User>, ([i64; 2], i64))>;
    let left: Vec<(U, P)> = drain((&us).and(&pc).and(pwf.select(flag()).opt())).into_iter().map(|(u, (x, p))| (Some((u, x)), p)).collect();
    let right: Vec<(U, P)> = drain(db.post.with(creation_date.gt(since)).minus(owner_user.select(pos())).select(flag())).into_iter().map(|(_, p)| (None, Some(p))).collect();
    let (left, right) = (rel(left), rel(right));
    let v: Vec<(U, P)> = drain((&left).union(&right)).into_iter().map(|x| x.1).collect();
    let v = top_k(
        v,
        |&(u, p)| (u.is_none(), Reverse(u.map(|(u, _)| db.user.reputation.get(u).unwrap())), p.is_none(), Reverse(p.map(|(p, _)| score.get(p).unwrap()))),
        |&(u, p)| (u.map(|x| x.0), p.map(|x| x.0)),
        50,
    );
    rows(v.into_iter().map(|(u, p)| {
        let mut f = match u {
            Some((u, (a, n))) => {
                let mut f = ucols(db, u, &["uid", "name", "rep"]);
                f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
                f
            }
            None => (0..6).map(|_| V::Null).collect(),
        };
        match p {
            Some((p, fl)) => {
                f.extend(post_fields(db, p, &["id", "title", "score"]));
                f.push(V::S(if fl.is_some() { "Flagged" } else { "Not Flagged" }));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotesCount,
//        COUNT(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 END) AS TotalVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON P.OwnerUserId = B.UserId
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.TotalUpVotes - PS.TotalDownVotes AS NetScore, RANK() OVER (ORDER BY PS.TotalUpVotes DESC, PS.CommentCount DESC) AS Rank FROM PostStatistics PS)
// SELECT UP.UserId, UP.DisplayName, TP.PostId, TP.Title, TP.NetScore, UP.TotalVotes AS UserTotalVotes, UP.UpVotesCount, UP.DownVotesCount
// FROM UserVoteCounts UP JOIN TopPosts TP ON UP.TotalVotes > 0 WHERE TP.Rank <= 10 ORDER BY TP.NetScore DESC, UP.TotalVotes DESC;
//
// The ON names only UP, so the voters are crossed with the top posts.
fn q6645(db: &'static So) -> String {
    let uvc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + matches!(t, Some(2 | 3)) as i64]
    });
    let ps = db
        .post
        .with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and((&db.post.owner_user).select(badges_of(db)).opt()))
        .fold([0i64; 3], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let v = ranked(drain(&ps), |&(_, a)| (Reverse(a[0]), Reverse(a[2])), false);
    let tp = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let voters = rel(drain((&uvc).filt(|a| a[2] > 0)));
    let mut v = Vec::new();
    (&voters).cross(&tp).drive(|_, ((u, a), (p, s))| v.push((u, a, p, s)));
    v.sort_by_key(|&(_, a, _, s)| (Reverse(s[0] - s[1]), Reverse(a[2])));
    rows(v.into_iter().map(|(u, a, p, s)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(s[0] - s[1]), V::I(a[2]), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// Rewritten (rewrites/28041.sql): the PostRank window is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Tags, u.DisplayName AS OwnerDisplayName, p.ViewCount, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, Tags, OwnerDisplayName, ViewCount, Score FROM RankedPosts WHERE PostRank = 1),
// TagUsage AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount FROM Posts p JOIN LATERAL unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS t(TagName) ON TRUE
//     WHERE p.PostTypeId = 1 GROUP BY t.TagName),
// MostUsedTags AS (SELECT TagName, PostCount, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagUsage WHERE PostCount > 100)
// SELECT trp.OwnerDisplayName, trp.Title, trp.CreationDate, trp.ViewCount, trp.Score, mut.TagName AS MostUsedTag, mut.PostCount AS MostUsedTagPostCount
// FROM TopRankedPosts trp JOIN MostUsedTags mut ON trp.Tags LIKE '%' || mut.TagName || '%' ORDER BY trp.Score DESC, trp.ViewCount DESC;
//
// The LIKE is a raw substring test on the Tags text, run over the
// distinct Tags strings with select_where.
fn q28041(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, tags_str, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 1, false);
    let trp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pairs: MatSet<(Id<Post>, Str)> = db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list))).collect();
    let tu = (&pairs).group_by(Same::<(Id<Post>, Str)>::new().map(|(_, t): (Id<Post>, Str)| t)).select(Same::<(Id<Post>, Str)>::new()).fold(0i64, |n, _| n + 1);
    let mu = rel(drain((&tu).filt(|n| n > 100)));
    let mut_: HashIdx<Str, (Str, i64)> = (&mu).map(|(t, _)| t).inv().select(&mu).collect();
    let full: MatSet<Str> = (&trp).select(tags_str).collect();
    let like: HashIdx<Str, (Str, i64)> = (&full).select_where(&mut_, |f: Str, t: Str| f.contains(t)).collect();
    let v = drain((&trp).select(tags_str.select(&like)));
    rows(v.into_iter().map(|(p, (t, n))| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "views", "score"]);
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostEngagement AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, COUNT(C.ID) AS CommentCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR')
//     GROUP BY P.Id, P.Title, P.ViewCount, P.Score),
// TopUsers AS (SELECT UM.UserId, UM.DisplayName, UM.Reputation, ROW_NUMBER() OVER (ORDER BY UM.Reputation DESC, UM.TotalPosts DESC) AS Rank FROM UserMetrics UM WHERE UM.TotalPosts > 10)
// SELECT TU.Rank, TU.DisplayName, TU.Reputation, P.Title AS PostTitle, P.Score AS PostScore, P.ViewCount AS PostViews, P.CommentCount, P.UpVotes, P.DownVotes
// FROM TopUsers TU JOIN PostEngagement P ON TU.UserId = P.PostId WHERE P.Score > 5 ORDER BY TU.Rank, P.ViewCount DESC FETCH FIRST 10 ROWS ONLY;
//
// `TU.UserId = P.PostId` joins a user id to a post id, so it goes through the raw ids, and only the posts it reaches get the comment x vote product.
// A tie in Rank goes to the smaller user id.
fn q2924(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&pc).filt(|n| n > 10)), |&(u, n)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u), false);
    let tu = rel(v.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let pe = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(score.gt(5));
    let hit: MatSet<Id<Post>> = (&rank).map(|(u, _)| u).select((&db.user.origid).select(&pidx).select(pe)).collect();
    let eng = (&hit).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain((&rank).map(|(_, r)| r).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&eng))));
    let v = top_k(v, |&(_, (r, (p, _)))| (r, Reverse(view_count.get(p))), |_| 0, 10);
    rows(v.into_iter().map(|(u, (r, (p, a)))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId = p.Id WHERE u.CreationDate >= '2023-01-01' GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalViews, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews FROM UserActivity)
// SELECT tu.DisplayName, tu.PostCount, tu.TotalViews, tu.UpVotes, tu.DownVotes,
//        CASE WHEN tu.DownVotes > 0 THEN 'Needs Improvement' WHEN tu.UpVotes > tu.DownVotes THEN 'Positive Impact' ELSE 'Neutral Contribution' END AS ContributionStatus
// FROM TopUsers tu WHERE tu.RankByViews <= 10
// UNION ALL
// SELECT 'Average' AS DisplayName, AVG(PostCount) AS PostCount, AVG(TotalViews) AS TotalViews, AVG(UpVotes) AS UpVotes, AVG(DownVotes) AS DownVotes,
//        CASE WHEN AVG(DownVotes) > 0 THEN 'Needs Improvement' WHEN AVG(UpVotes) > AVG(DownVotes) THEN 'Positive Impact' ELSE 'Neutral Contribution' END AS ContributionStatus
// FROM TopUsers HAVING COUNT(UserId) > 0;
//
// The vote join names both the user and the post, so it is the user's own votes on their own post (`own_votes`). The UNION with AVG makes every
// numeric column a DOUBLE.
fn q2222(db: &'static So) -> String {
    let own = own_votes(db);
    let ua = db
        .user
        .with((&db.user.creation_date).ge(ts(2023, 1, 1, 0, 0, 0)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(own.select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((w, t)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    let status = |up: f64, down: f64| if down > 0.0 { "Needs Improvement" } else if up > down { "Positive Impact" } else { "Neutral Contribution" };
    let v = ranked(drain(&ua), |&(_, a)| Reverse(a[1]), false);
    let mut out: Vec<String> = v
        .iter()
        .take_while(|x| x.1 <= 10)
        .map(|&((u, a), _)| row(vec![user_col(db, u, "name"), V::F(a[0] as f64), V::F(a[1] as f64), V::F(a[2] as f64), V::F(a[3] as f64), V::S(status(a[2] as f64, a[3] as f64))]))
        .collect();
    let s = (&ua).fold_flat([0i64; 5], |s, a| [s[0] + 1, s[1] + a[0], s[2] + a[1], s[3] + a[2], s[4] + a[3]]);
    if s[0] > 0 {
        let m = |x: i64| x as f64 / s[0] as f64;
        out.push(row(vec![V::S("Average"), V::F(m(s[1])), V::F(m(s[2])), V::F(m(s[3])), V::F(m(s[4])), V::S(status(m(s[3]), m(s[4])))]));
    }
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// PopularPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount FROM RankedPosts rp WHERE rp.Score > 10 ORDER BY rp.ViewCount DESC LIMIT 10)
// SELECT pp.Title, pp.CreationDate, pp.ViewCount, pp.OwnerDisplayName, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = pp.Id AND v.VoteTypeId = 2) AS UpVotes,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = pp.Id AND v.VoteTypeId = 3) AS DownVotes,
//        (SELECT STRING_AGG(t.TagName, ', ') FROM Tags t WHERE pp.Id IN (SELECT p.Id FROM Posts p WHERE p.Tags LIKE CONCAT('%<', t.TagName, '>%'))) AS RelatedTags
// FROM PopularPosts pp JOIN PostHistory ph ON pp.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10
// GROUP BY pp.Id, pp.Title, pp.CreationDate, pp.ViewCount, pp.OwnerDisplayName ORDER BY pp.ViewCount DESC;
//
// `LIKE '%<name>%'` matches exactly the post's own tags. STRING_AGG has no ORDER BY, so the port joins the names in Tags id order.
// Forty posts with no ViewCount tie at the LIMIT 10 cut; none has a close record, so which ones are kept cannot change the (empty) result.
fn q9522(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).with(score.gt(10)).select(view_count.opt()));
    let pp = top_k(v, |&(_, w)| (w.is_none(), Reverse(w)), |&(p, _)| p, 10);
    let pp: MatSet<Id<Post>> = rel(pp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let closed: MatSet<Id<Post>> = (&pp).with(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)))).collect();
    let votes = (&closed).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let tags = (&closed).group_by(Ident::<Post>::new()).select((&db.post.tags).opt()).buf_fold(|it| {
        let mut t: Vec<Id<Tag>> = it.into_iter().flatten().collect();
        t.sort_unstable();
        let n: Vec<Str> = t.into_iter().map(|t| db.tag.tag_name.get(t).unwrap()).collect();
        if n.is_empty() { None } else { Some(&*Box::leak(n.join(", ").into_boxed_str())) }
    });
    let v = drain((&votes).and(&tags));
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), ostr(t)]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U WHERE U.Reputation IS NOT NULL),
// ActivePostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments, SUM(V.BountyAmount) AS TotalBounty
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
//     WHERE P.CreationDate > (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') GROUP BY P.OwnerUserId),
// UserBadges AS (SELECT B.UserId, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId)
// SELECT R.DisplayName, R.Reputation, A.TotalPosts, A.TotalComments, COALESCE(A.TotalBounty, 0) AS TotalBounty, COALESCE(UB.GoldBadges, 0) AS GoldBadges,
//        COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges, R.UserRank
// FROM RankedUsers R LEFT JOIN ActivePostStats A ON R.Id = A.OwnerUserId LEFT JOIN UserBadges UB ON R.Id = UB.UserId WHERE R.UserRank <= 100
// ORDER BY R.Reputation DESC, A.TotalPosts DESC NULLS LAST, R.DisplayName LIMIT 50;
//
// UserRank reads only Reputation, so the hundred users are picked first and the comment x vote product is driven for them alone. A Reputation
// tie in UserRank goes to the smaller user id.
fn q23099(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 100).map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, i64> = (&tr).map(|(u, _)| u).inv().select((&tr).map(|(_, r)| r)).collect();
    let tu: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let recent = posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let aps = (&tu).group_by(Ident::<User>::new()).select(recent.select(comments_of(db).opt().and(bounty.opt()))).fold([0i64; 3], |a, (_, b)| {
        let b = b.flatten();
        [a[0] + 1, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
    });
    let recent = posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let tc = (&tu).group_by(Ident::<User>::new()).select(recent.select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&rank).and((&aps).opt()).and((&tc).opt()).and((&ub).opt()));
    let v = top_k(v, |&(u, (((_, a), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), a.is_none(), Reverse(a.map(|a| a[0])), db.user.display_name.get(u).unwrap()), |&(u, _)| u, 50);
    rows(v.into_iter().map(|(u, (((r, a), c), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(c.unwrap_or(0)), V::I(a[2])],
            None => [V::Null, V::Null, V::I(0)],
        });
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVotes,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVotes, RANK() OVER (ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// PostWithBadge AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.UpVotes, rp.DownVotes, b.Name AS BadgeName
//     FROM RankedPosts rp LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) AND b.Class = 1 WHERE rp.PostRank <= 10),
// PostHistoryInfo AS (SELECT p.Id AS PostId, STRING_AGG(ph.Comment, '; ') AS EditComments, MAX(ph.CreationDate) AS LastEditDate
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId IN (4, 5) GROUP BY p.Id)
// SELECT pw.PostId, pw.Title, pw.CreationDate, pw.ViewCount, pw.UpVotes, pw.DownVotes, pw.BadgeName, COALESCE(phe.EditComments, 'No edits made') AS EditComments, phe.LastEditDate
// FROM PostWithBadge pw LEFT JOIN PostHistoryInfo phe ON pw.PostId = phe.PostId WHERE pw.UpVotes - pw.DownVotes > 5 ORDER BY pw.CreationDate DESC NULLS LAST FETCH FIRST 20 ROWS ONLY;
//
// The LIMIT 1 subquery looks a post up by its id, so it is the post's owner. STRING_AGG has no ORDER BY, so the port joins the comments in history id order.
fn q21168(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(_, d)| Reverse(d), false);
    let rp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let ud = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))).select(&db.badge.name);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let edits = db.post_history.with(post_history_type_id.in_v(vec![4, 5]));
    let phi = edits.group_by(post).select(Ident::<PostHistory>::new().and(comment.opt()).and(hd)).buf_fold(|it| {
        let mut x: Vec<((Id<PostHistory>, Option<Str>), i64)> = it.into_iter().collect();
        x.sort_unstable_by_key(|((h, _), _)| *h);
        let c: Vec<Str> = x.iter().filter_map(|((_, c), _)| *c).collect();
        let m = x.iter().map(|(_, d)| *d).max().unwrap();
        (if c.is_empty() { None } else { Some(&*Box::leak(c.join("; ").into_boxed_str())) }, m)
    });
    let v = drain((&ud).filt(|a| a[0] - a[1] > 5).and(owner_user.select(gold).opt()).and((&phi).opt()));
    let v = top_k(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), |&(p, ((_, b), _))| (p, b), 20);
    rows(v.into_iter().map(|(p, ((a, b), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), ostr(b)]);
        f.extend(match h {
            Some((c, d)) => [V::S(c.unwrap_or("No edits made")), V::T(d)],
            None => [V::S("No edits made"), V::Null],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostAnalytics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.ClosedDate,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RowNum, P.OwnerUserId
//     FROM Posts P WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '6 months')
// SELECT U.DisplayName, U.Reputation, U.TotalUpVotes, U.TotalDownVotes, U.TotalPosts, U.TotalComments, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount,
//        CASE WHEN P.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM UserStatistics U JOIN PostAnalytics P ON U.UserId = P.OwnerUserId WHERE U.Reputation > 1000 AND P.RowNum <= 5 ORDER BY U.Reputation DESC, P.Score DESC;
//
// The WHERE on Reputation reads a base column, so the posts x comments x votes product is driven for those users alone. A CreationDate tie at
// an owner's fifth place goes to the smaller post id.
fn q1140(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let hi: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(1000)).collect();
    let st = (&hi)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, x| {
            let t = x.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let pc = (&hi).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = (&hi).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).with(owner_user.select(Ident::<User>::new().with(&hi))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&st).and(&pc).and(&cc))));
    rows(v.into_iter().map(|(p, (((u, a), n), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::I(c)]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views", "answers", "comments"]));
        f.push(V::S(if db.post.closed_date.get(p).is_some() { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        COUNT(DISTINCT V.PostId) AS TotalVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostSummary AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(PC.AcceptedAnswerId, 0) AS AcceptedAnswerId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(DISTINCT V.Id) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS UserPostRank, P.OwnerUserId
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Posts PC ON P.Id = PC.ParentId
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND P.Score > 0 GROUP BY P.Id, P.Title, P.CreationDate, P.Score, PC.AcceptedAnswerId, P.OwnerUserId)
// SELECT U.DisplayName, PS.Title, PS.CreationDate, PS.Score, PS.AcceptedAnswerId, PS.CommentCount, PS.VoteCount, U.TotalUpvotes, U.TotalDownvotes, PS.UserPostRank
// FROM UserVoteSummary U INNER JOIN PostSummary PS ON U.UserId = PS.OwnerUserId WHERE (U.TotalUpvotes - U.TotalDownvotes) > 10 AND PS.CommentCount > 5
// ORDER BY U.TotalUpvotes DESC, PS.Score DESC LIMIT 50;
//
// PostSummary groups the post x comment x vote x child rows by (post, child AcceptedAnswerId), so those rows are materialised and grouped. A
// CreationDate tie in UserPostRank goes to the smaller (post, AcceptedAnswerId).
fn q2373(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, accepted_answer_id, .. } = &db.post;
    let uvs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    type J = (((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>), Option<Id<Post>>);
    let j: MatSet<J> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(score.gt(0))
        .select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt()).and(children_of(db).opt()))
        .collect();
    let post_of = (&j).map(|(((p, _), _), _): J| p);
    let comment_of = (&j).map(|(((_, c), _), _): J| c);
    let vote_of = (&j).map(|((_, v), _): J| v);
    let child_of = (&j).flat_map(|(_, ch): J| ch);
    let ps = (&j)
        .group_by((&post_of).and((&child_of).select(accepted_answer_id).opt()))
        .select((&comment_of).and(&vote_of))
        .buf_fold(|it| {
            let x: Vec<(Option<Id<Comment>>, Option<Id<Vote>>)> = it.into_iter().collect();
            let mut vs: Vec<Id<Vote>> = x.iter().filter_map(|x| x.1).collect();
            vs.sort_unstable();
            vs.dedup();
            (x.iter().filter(|x| x.0.is_some()).count() as i64, vs.len() as i64)
        });
    let g = rel(drain(&ps));
    let g = ranked(drain(&g), |&(_, ((p, a), _))| (owner_user.get(p), Reverse(creation_date.get(p).unwrap()), p, a), false);
    let g = per_group(g, |&(_, ((p, _), _))| owner_user.get(p));
    type G = (usize, (((Id<Post>, Option<i64>), (i64, i64)), i64));
    let g = rel(g.into_iter().map(|((i, x), r)| (i, (x, r))).collect::<Vec<G>>());
    let v = drain((&g).filt(|(_, ((_, (c, _)), _)): G| c > 5).select(Same::<G>::new().and(Same::<G>::new().map(|(_, (((p, _), _), _)): G| p).select(owner_user.select(Ident::<User>::new().and((&uvs).filt(|a| a[0] - a[1] > 10)))))));
    let v = top_k(v, |&(_, ((_, (((p, _), _), _)), (_, a)))| (Reverse(a[0]), Reverse(score.get(p).unwrap())), |&(_, ((_, (((p, x), _), _)), _))| (p, x), 50);
    rows(v.into_iter().map(|(_, ((_, (((p, x), (c, n)), r)), (u, a)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(x.unwrap_or(0)), V::I(c), V::I(n), V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS rn
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, COALESCE(SUM(P.ViewCount), 0) AS TotalPostViews, COUNT(DISTINCT B.Id) AS BadgeCount,
//        COUNT(DISTINCT C.Id) AS CommentCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.Views, U.TotalPostViews, U.BadgeCount, U.CommentCount, RP.Title, RP.Score, RP.ViewCount, RP.UpVotes, RP.DownVotes
// FROM UserStats U LEFT JOIN RankedPosts RP ON U.UserId = RP.PostId WHERE U.Reputation > 1000 AND (U.Views IS NULL OR U.Views > 500) AND (RP.rn IS NULL OR RP.rn <= 5)
// ORDER BY U.TotalPostViews DESC, U.Reputation DESC LIMIT 10 OFFSET 0;
//
// `U.UserId = RP.PostId` joins a user id to a post id, so it goes through the raw ids. The WHERE on Reputation and Views reads base columns, so
// the posts x badges x comments product is driven for those users alone. The ownerless posts are their own rn partition; a CreationDate tie
// goes to the smaller post id.
fn q4187(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let us: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(1000)).with((&db.user.views).gt(500)).collect();
    let st = (&us)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold(0i64, |s, (p, _)| s + p.and_then(|(w, _)| w).unwrap_or(0));
    let bc = (&us).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cc = (&us).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let recent = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rn = ranked(drain(recent.select(owner_user.opt())), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false);
    let rn = per_group(rn, |&(_, u)| u);
    let rn = rel(rn.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rn: HashIdx<Id<Post>, i64> = (&rn).map(|(p, _)| p).inv().select((&rn).map(|(_, r)| r)).collect();
    let recent = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ud = recent.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let rp = (&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&rn).and(&ud));
    let v = drain((&st).and(&bc).and(&cc).and(rp.opt()).filt(|(_, r): (((i64, i64), i64), Option<((Id<Post>, i64), [i64; 2])>)| r.map_or(true, |((_, n), _)| n <= 5)));
    let v = top_k(v, |&(u, (((s, _), _), _))| (Reverse(s), Reverse(db.user.reputation.get(u).unwrap())), |&(u, _)| u, 10);
    rows(v.into_iter().map(|(u, (((s, b), c), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
        f.extend([V::I(s), V::I(b), V::I(c)]);
        f.extend(match r {
            Some(((p, _), a)) => {
                let mut g = post_fields(db, p, &["title", "score", "views"]);
                g.extend([V::I(a[0]), V::I(a[1])]);
                g
            }
            None => (0..5).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, U.DisplayName AS OwnerName, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS RankByScore
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND P.Score IS NOT NULL),
// CloseReasons AS (SELECT PH.PostId, C.Name AS CloseReason FROM PostHistory PH LEFT JOIN CloseReasonTypes C ON PH.Comment = C.Id::text WHERE PH.PostHistoryTypeId IN (10, 11)),
// UserActivity AS (SELECT U.Id AS UserId, COUNT(DISTINCT V.PostId) AS UpVotesGiven, COUNT(DISTINCT C.Id) AS CommentsMade, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId AND V.VoteTypeId = 2 LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, RP.CreationDate, RP.OwnerName, COALESCE(CR.CloseReason, 'Not Closed') AS CloseStatus, UA.UpVotesGiven, UA.CommentsMade, UA.GoldBadges
// FROM RankedPosts RP LEFT JOIN CloseReasons CR ON RP.PostId = CR.PostId LEFT JOIN UserActivity UA ON RP.OwnerName = UA.UserId::text WHERE RP.RankByScore <= 5 ORDER BY RP.Score DESC, RP.PostId ASC;
//
// `RP.OwnerName = UA.UserId::text` compares a display name with a user id's text, so the name joins the user whose id prints as that name; the
// UserActivity product is driven for those users alone. `PH.Comment = C.Id::text` is an exact text match.
fn q823(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let exact = |s: Str| s.parse::<i64>().ok().filter(|i| i.to_string() == s);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let named = owner_user.select(&db.user.display_name).flat_map(exact).select(&uidx);
    let cand: MatSet<Id<User>> = (&rp).select(named).collect();
    let up = votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).select(&db.vote.post_id);
    let ua = (&cand).group_by(Ident::<User>::new()).select(up.opt().and(comments_by(db).opt()).and(badges_of(db).select(&db.badge.class).opt())).buf_fold(|it| {
        let x: Vec<((Option<i64>, Option<Id<Comment>>), Option<i64>)> = it.into_iter().collect();
        let mut vp: Vec<i64> = x.iter().filter_map(|x| x.0 .0).collect();
        vp.sort_unstable();
        vp.dedup();
        let mut cs: Vec<Id<Comment>> = x.iter().filter_map(|x| x.0 .1).collect();
        cs.sort_unstable();
        cs.dedup();
        [vp.len() as i64, cs.len() as i64, x.iter().filter(|x| x.1 == Some(1)).count() as i64]
    });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).in_v(vec![10, 11])));
    let cr = closes.select((&db.post_history.comment).flat_map(exact).select(&reason).opt());
    let named = owner_user.select(&db.user.display_name).flat_map(exact).select(&uidx);
    let v = drain((&rp).select(cr.opt().and(named.select(&ua).opt())));
    let mut v = v;
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), db.post.origid.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner"]);
        f.push(V::S(c.flatten().unwrap_or("Not Closed")));
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.Reputation >= 1000 GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, COUNT(C.Id) AS CommentCount, COALESCE(SUM(CASE WHEN PH.RevisionGUID IS NOT NULL THEN 1 ELSE 0 END), 0) AS EditCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY P.Id, P.Title),
// RankedUsers AS (SELECT UA.UserId, UA.DisplayName, UA.PostCount, UA.UpVotes, UA.DownVotes, UA.BadgeCount, RANK() OVER (ORDER BY UA.UpVotes DESC) AS VoteRank FROM UserActivity UA)
// SELECT RU.DisplayName, RU.PostCount, RU.UpVotes, RU.DownVotes, RU.BadgeCount, PS.Title, PS.CommentCount, PS.EditCount
// FROM RankedUsers RU JOIN PostStatistics PS ON RU.UserId = PS.PostId WHERE RU.VoteRank <= 10 ORDER BY RU.UpVotes DESC, RU.BadgeCount DESC LIMIT 5 OFFSET 0;
//
// `RU.UserId = PS.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q3438(db: &'static So) -> String {
    let hi = || db.user.with((&db.user.reputation).ge(1000));
    let ua = hi()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, _)| {
            let t = p.flatten();
            [a[0] + p.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
        });
    let bc = hi().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain((&ua).and(&bc)), |&(_, (a, _))| Reverse(a[1]), false);
    let ru: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let recent = Ident::<Post>::new().with((&db.post.creation_date).ge(add_years(current_date(), -1)));
    let hit: MatSet<Id<Post>> = (&ru).select((&db.user.origid).select(&pidx).select(recent)).collect();
    let ps = (&hit).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).opt())).fold([0i64; 2], |a, (c, h)| [a[0] + c.is_some() as i64, a[1] + h.is_some() as i64]);
    let v = drain((&ru).select(Ident::<User>::new().and(&ua).and(&bc).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&ps)))));
    let v = top_k(v, |&(_, (((_, a), b), _))| (Reverse(a[1]), Reverse(b)), |&(u, _)| u, 5);
    rows(v.into_iter().map(|(_, (((u, a), b), (p, s)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b)]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(s[0]), V::I(s[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.UpVotes, 0)) AS TotalUpVotes, SUM(COALESCE(v.DownVotes, 0)) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY b.UserId)
// SELECT ua.DisplayName, ua.PostCount, ua.TotalUpVotes, ua.TotalDownVotes, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate, tb.BadgeCount,
//        CASE WHEN tb.BadgeCount IS NOT NULL THEN 'Has Badges' ELSE 'No Badges' END AS BadgeStatus
// FROM UserActivity ua LEFT JOIN RecentPosts rp ON ua.UserId = rp.PostId LEFT JOIN TopBadges tb ON ua.UserId = tb.UserId WHERE ua.PostCount > 5 ORDER BY ua.TotalUpVotes DESC, ua.PostCount DESC;
//
// `ua.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids; rn is never read. The vote subquery has one row per
// post, so the user's sums are over their posts.
fn q2245(db: &'static So) -> String {
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ua = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&pv).opt()).opt())
        .fold([0i64; 3], |a, x| match x {
            Some(v) => {
                let v = v.unwrap_or([0; 2]);
                [a[0] + 1, a[1] + v[0], a[2] + v[1]]
            }
            None => a,
        });
    let tb = db.badge.with((&db.badge.date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let rp = Ident::<Post>::new().with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let v = drain((&ua).filt(|a| a[0] > 5).and((&db.user.origid).select(&pidx).select(rp).opt()).and((&tb).opt()));
    rows(v.into_iter().map(|(u, ((a, p), b))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        f.extend([oint(b), V::S(if b.is_some() { "Has Badges" } else { "No Badges" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStats AS (SELECT P.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT UR.DisplayName, UR.Reputation, PS.TotalPosts, PS.Questions, PS.Answers, PS.AverageScore FROM UserReputation UR JOIN PostStats PS ON UR.UserId = PS.OwnerUserId WHERE UR.Reputation > 1000)
// SELECT U.DisplayName, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.Questions, 0) AS Questions, COALESCE(PS.Answers, 0) AS Answers,
//        COALESCE(AVG(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotesGiven, COALESCE(AVG(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotesGiven,
//        CASE WHEN PS.AverageScore IS NULL THEN 'No Posts' WHEN PS.AverageScore > 5 THEN 'High Score' ELSE 'Low Score' END AS ScoreCategory
// FROM Users U LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId LEFT JOIN Votes V ON U.Id = V.UserId
// WHERE U.Reputation > 500 AND U.LastAccessDate > (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// GROUP BY U.DisplayName, PS.TotalPosts, PS.Questions, PS.Answers, PS.AverageScore HAVING COUNT(DISTINCT U.Id) >= 1 ORDER BY U.DisplayName;
//
// The groups are (name, PostStats row): users sharing both merge. AverageScore is a group key; two users with the same TotalPosts have the same
// average exactly when their score sums agree, so the sum stands in for it in the key.
fn q555(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(&db.post.owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let g = db
        .user
        .with((&db.user.reputation).gt(500))
        .with((&db.user.last_access_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by((&db.user.display_name).and((&ps).opt()))
        .select(votes_by(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain(&g);
    rows(v.into_iter().map(|((name, p), a)| {
        let p4 = p.unwrap_or([0; 4]);
        let mut f = vec![V::S(name), V::I(p4[0]), V::I(p4[1]), V::I(p4[2])];
        f.extend([V::F(a[1] as f64 / a[0] as f64), V::F(a[2] as f64 / a[0] as f64)]);
        f.push(V::S(match p {
            None => "No Posts",
            Some(p) if p[3] > 5 * p[0] => "High Score",
            _ => "Low Score",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(CASE WHEN c.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(COALESCE(rp.Score, 0)) AS TotalScore, SUM(COALESCE(rp.ViewCount, 0)) AS TotalViews,
//        SUM(COALESCE(rp.CommentCount, 0)) AS TotalComments, SUM(COALESCE(rp.VoteCount, 0)) AS TotalVotes, COUNT(rp.PostId) AS PostCount
//     FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalScore, us.TotalViews, us.TotalComments, us.TotalVotes, us.PostCount,
//        CASE WHEN us.Reputation >= 1000 THEN 'Gold User' WHEN us.Reputation >= 500 THEN 'Silver User' ELSE 'New User' END AS UserRank
// FROM UserStats us WHERE us.PostCount > 5 ORDER BY us.TotalScore DESC, us.TotalViews DESC LIMIT 10;
//
// UserPostRank is never read.
fn q7546(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let rp = recent().select(score.and(view_count.opt()).and(&cc).and(&vc));
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(rp).opt()).fold([0i64; 5], |a, x| match x {
        Some((((s, w), c), n)) => [a[0] + s, a[1] + w.unwrap_or(0), a[2] + c, a[3] + n, a[4] + 1],
        None => a,
    });
    let v = top_k(drain((&us).filt(|a| a[4] > 5)), |&(_, a)| (Reverse(a[0]), Reverse(a[1])), |&(u, _)| u, 10);
    rows(v.into_iter().map(|(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::S(if r >= 1000 { "Gold User" } else if r >= 500 { "Silver User" } else { "New User" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(COUNT(c.Id), 0) AS CommentCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// ClosedPosts AS (SELECT p.Id AS ClosedPostId, ph.UserId, ph.CreationDate AS CloseDate FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10),
// TopUsers AS (SELECT ur.Id, ur.Reputation, ur.ReputationRank FROM UserReputation ur WHERE ur.ReputationRank <= 10)
// SELECT u.DisplayName, COUNT(DISTINCT ps.PostId) AS TotalPosts, SUM(COALESCE(ps.Score, 0)) AS TotalScore, COUNT(DISTINCT cp.ClosedPostId) AS TotalClosedPosts
// FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostStats ps ON p.Id = ps.PostId LEFT JOIN ClosedPosts cp ON p.Id = cp.ClosedPostId
// WHERE u.Reputation > 1000 AND (u.Location IS NOT NULL OR u.AboutMe IS NOT NULL) GROUP BY u.DisplayName HAVING SUM(COALESCE(ps.ViewCount, 0)) > 50
// ORDER BY TotalPosts DESC LIMIT 5 OFFSET 0;
//
// PostStats has one row per recent post and only its Score and ViewCount are read, so `ps` is the post itself when it is recent. The groups are
// display names, so users sharing a name merge. UserReputation and TopUsers are never referenced.
fn q981(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let users = || {
        db.user
            .with((&db.user.reputation).gt(1000))
            .with((&db.user.location).or(&db.user.about_me))
            .group_by(&db.user.display_name)
    };
    let recent = || Ident::<Post>::new().with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let closes = || history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let s = users().select(posts_of(db).select(recent().select(score.and(view_count.opt())).opt().and(closes().opt()))).fold([0i64; 2], |a, (p, _)| match p {
        Some((s, w)) => [a[0] + s, a[1] + w.unwrap_or(0)],
        None => a,
    });
    let tp = users().select(posts_of(db).select(recent())).fold(0i64, |n, _| n + 1);
    let tc = users().select(posts_of(db).with(closes())).fold(0i64, |n, _| n + 1);
    let v = drain((&s).filt(|a| a[1] > 50).and((&tp).opt()).and((&tc).opt()));
    let v = top_k(v, |&(_, ((_, n), _))| Reverse(n.unwrap_or(0)), |&(name, _)| name, 5);
    rows(v.into_iter().map(|(name, ((a, n), c))| row(vec![V::S(name), V::I(n.unwrap_or(0)), V::I(a[0]), V::I(c.unwrap_or(0))])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END), 0) AS VoteBalance,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.TotalComments, us.TotalBadges, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.VoteBalance
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId WHERE rp.RecentPostRank <= 3 ORDER BY us.Reputation DESC, rp.Score DESC, rp.VoteBalance DESC LIMIT 100;
//
// RecentPostRank reads only base columns, so each owner's three newest questions are picked first, and the posts x comments x badges product
// is driven only for their owners. A CreationDate tie at an owner's third place goes to the smaller id.
fn q6058(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&rp).select(owner_user).collect();
    let tb = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db).opt()).opt().and(badges_of(db).opt())).fold(0i64, |n, (_, b)| n + b.is_some() as i64);
    let tp = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ps = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + if t == Some(2) { 1 } else if t == Some(3) { -1 } else { 0 }]
    });
    let v = drain((&ps).and(owner_user.select(Ident::<User>::new().and(&tp).and(&tc).and(&tb))));
    let v = top_k(v, |&(p, (a, (((u, _), _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), Reverse(a[1])), |&(p, _)| p, 100);
    rows(v.into_iter().map(|(p, (a, (((u, n), c), b)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(c), V::I(b)]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, STRING_AGG(t.TagName, ', ') AS Tags
//     FROM Posts p JOIN Tags t ON t.WikiPostId = p.Id OR t.ExcerptPostId = p.Id WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.Tags FROM RankedPosts rp WHERE rp.PostRank = 1),
// UserScores AS (SELECT u.Id AS UserId, u.DisplayName, (SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END)) AS ReputationScore
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT up.UserId, up.DisplayName, tp.Title, tp.Body, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, up.ReputationScore, tp.Tags AS TagList
// FROM UserScores up JOIN TopPosts tp ON up.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) ORDER BY up.ReputationScore DESC, tp.CreationDate DESC LIMIT 10;
//
// The OR join is the union of the wiki and excerpt joins, each (post, tag) pair once. The LIMIT 1 subquery looks a post up by its id, so it is
// the post's owner. STRING_AGG has no ORDER BY, so the port joins the names in Tags id order. The ownerless questions are their own PostRank
// partition; a CreationDate tie goes to the smaller id.
fn q26450(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let wiki: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let qs = || db.post.with(post_type_id.eq(1));
    let pairs: MatSet<(Id<Post>, Id<Tag>)> = qs().select(Ident::<Post>::new().and(&wiki)).union(qs().select(Ident::<Post>::new().and(&excerpt))).collect();
    let tags = (&pairs).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _): (Id<Post>, Id<Tag>)| p)).select(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t): (Id<Post>, Id<Tag>)| t)).buf_fold(|it| {
        let mut t: Vec<Id<Tag>> = it.into_iter().collect();
        t.sort_unstable();
        let n: Vec<Str> = t.into_iter().map(|t| db.tag.tag_name.get(t).unwrap()).collect();
        &*Box::leak(n.join(", ").into_boxed_str())
    });
    let ps: MatSet<Id<Post>> = (&pairs).map(|(p, _)| p).collect();
    let v = drain((&ps).select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&us))).and(&tags));
    let v = top_k(v, |&(p, ((_, s), _))| (Reverse(s), Reverse(creation_date.get(p).unwrap())), |&(p, _)| p, 10);
    rows(v.into_iter().map(|(p, ((u, s), t))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "body", "created", "score", "views", "answers", "comments"]));
        f.extend([V::I(s), V::S(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.ViewCount, p.Score, RANK() OVER (PARTITION BY STRING_AGG(t.TagName, ',') ORDER BY p.Score DESC) AS TagRank
//     FROM Posts p JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%' WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.ViewCount, p.Score),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate AS CloseDate, ph.UserDisplayName AS ClosedBy, ph.Comment AS CloseReason FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10),
// UserVotes AS (SELECT p.OwnerUserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId)
// SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, u.DisplayName AS Owner, rp.ViewCount, rp.Score, cb.CloseDate, cb.ClosedBy, cb.CloseReason, uv.VoteCount, uv.UpVotes, uv.DownVotes, rp.TagRank
// FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN ClosedPosts cb ON rp.PostId = cb.PostId LEFT JOIN UserVotes uv ON rp.OwnerUserId = uv.OwnerUserId
// WHERE rp.TagRank = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 100;
//
// The partition key is the post's matching tag names joined by ','; STRING_AGG has no ORDER BY, so the port joins them in Tags id order.
// The LIKE is `tag_mentions`. `uv` joins on the owner, so an ownerless question gets no UserVotes row.
fn q25444(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let lt = tag_mentions(db);
    let key = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _): (Id<Post>, Id<Tag>)| p)).select(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t): (Id<Post>, Id<Tag>)| t)).buf_fold(|it| {
        let mut t: Vec<Id<Tag>> = it.into_iter().collect();
        t.sort_unstable();
        let n: Vec<Str> = t.into_iter().map(|t| db.tag.tag_name.get(t).unwrap()).collect();
        &*Box::leak(n.join(",").into_boxed_str())
    });
    let v = drain(db.post.with(post_type_id.eq(1)).select(&key));
    let top = top_per(v, |&(_, k)| k, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uv = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&rp).select(closes.opt().and(owner_user.select(&uv).opt())));
    let v = top_k(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, |&(p, (h, _))| (p, h), 100);
    let PostHistory { creation_date: hd, user_display_name, comment, .. } = &db.post_history;
    rows(v.into_iter().map(|(p, (h, u))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner", "views", "score"]);
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), ostr(user_display_name.get(h)), ostr(comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match u {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::I(1));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserStatistics AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT b.Id) AS TotalBadges, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation),
// PostVotes AS (SELECT p.Id AS PostId, COUNT(DISTINCT v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT us.UserId, us.Reputation, us.TotalPosts, us.TotalBadges, us.TotalBountyAmount, rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, pv.TotalVotes, pv.UpVotes, pv.DownVotes
// FROM UserStatistics us JOIN RankedPosts rp ON us.UserId = rp.PostId LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId
// WHERE us.TotalPosts > 5 AND (pv.UpVotes IS NULL OR pv.UpVotes > 10) ORDER BY us.Reputation DESC, rp.Score DESC LIMIT 100;
//
// `us.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids; ScoreRank is never read. Every post has a PostVotes row,
// so the WHERE is `UpVotes > 10`. Only the users whose id is such a post's id can reach the output, so the posts x badges x votes product is
// driven for them alone.
fn q2722(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let rp: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with((&pv).filt(|a| a[1] > 10)).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let cand: MatSet<Id<User>> = (&rp).select((&db.post.origid).select(&uidx)).collect();
    let bounty = || votes_by(db).select((&db.vote.bounty_amount).opt());
    let us = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).opt()).and(bounty().opt())).fold(0i64, |s, (_, b)| s + b.flatten().unwrap_or(0));
    let tp = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tb = (&cand).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&rp).select((&db.post.origid).select(&uidx).select(Ident::<User>::new().and((&tp).filt(|n| n > 5)).and(&tb).and(&us))).and(&pv));
    let v = top_k(v, |&(p, ((((u, _), _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())), |&(p, _)| p, 100);
    rows(v.into_iter().map(|(p, ((((u, n), b), s), a))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::I(b), V::I(s)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS ViewRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetVoteScore,
//        COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// PostVoteDetails AS (SELECT p.Id AS PostId, COUNT(v.Id) AS TotalVotes, AVG(v.BountyAmount) AS AverageBounty FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT up.DisplayName, rp.Title, rp.ViewCount, rp.CreationDate, us.NetVoteScore, us.BadgeCount, pvd.TotalVotes, pvd.AverageBounty,
//        CASE WHEN pvd.TotalVotes IS NULL THEN 'No Votes' WHEN pvd.TotalVotes > 50 THEN 'Highly Voted' ELSE 'Moderate Votes' END AS VoteCategory
// FROM RankedPosts rp JOIN Users up ON rp.PostId = up.Id JOIN UserStats us ON up.Id = us.UserId LEFT JOIN PostVoteDetails pvd ON rp.PostId = pvd.PostId
// WHERE rp.ViewRank = 1 ORDER BY us.NetVoteScore DESC, rp.ViewCount DESC;
//
// `rp.PostId = up.Id` joins a post id to a user id, so it goes through the raw ids, and the votes x badges product is driven for the users it
// reaches. The ownerless questions are their own ViewRank partition; a ViewCount tie goes to the smaller id.
fn q849(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let cand: MatSet<Id<User>> = (&rp).select((&db.post.origid).select(&uidx).select(Ident::<User>::new().with((&db.user.reputation).gt(100)))).collect();
    let us = (&cand).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt())).fold(0i64, |n, (t, _)| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let bc = (&cand).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pvd = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).fold([0i64; 3], |a, v| match v {
        Some(b) => [a[0] + 1, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)],
        None => a,
    });
    let v = drain((&rp).select((&db.post.origid).select(&uidx).select(Ident::<User>::new().with(&cand).and(&us).and(&bc))).and(&pvd));
    let mut v = v;
    v.sort_by_key(|&(p, (((_, s), _), _))| (Reverse(s), Reverse(view_count.get(p))));
    rows(v.into_iter().map(|(p, (((u, s), b), a))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "views", "created"]));
        f.extend([V::I(s), V::I(b), V::I(a[0]), avg(a[2], a[1]), V::S(if a[0] > 50 { "Highly Voted" } else { "Moderate Votes" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score >= 0),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostCommentStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// PostClosureReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph LEFT JOIN CloseReasonTypes cr ON ph.Comment = cr.Id::text
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ub.BadgeCount, ub.HighestBadgeClass, pcs.CommentCount, pcs.LastCommentDate,
//        COALESCE(pcr.CloseReasons, 'Not Closed') AS ClosureInfo
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostCommentStats pcs ON rp.PostId = pcs.PostId LEFT JOIN PostClosureReasons pcr ON rp.PostId = pcr.PostId
// WHERE rp.Rank = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 100;
//
// The ownerless questions are their own Rank partition; a CreationDate tie goes to the smaller id. STRING_AGG has no ORDER BY; no post reached
// here has more than one close reason name, so its order cannot be observed.
fn q22513(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.ge(0))).select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let pcs = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    let exact = |s: Str| s.parse::<i64>().ok().filter(|i| i.to_string() == s);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let pcr = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(comment.flat_map(exact).select(&reason).opt()).buf_fold(|it| {
        let n: Vec<Str> = it.into_iter().flatten().collect();
        if n.is_empty() { None } else { Some(&*Box::leak(n.join(", ").into_boxed_str())) }
    });
    let v = drain((&pcs).and(owner_user.select(&ub).opt()).and((&pcr).opt()));
    let v = top_k(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, |&(p, _)| p, 100);
    rows(v.into_iter().map(|(p, (((n, m), b), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(match b {
            Some((k, x)) => [V::I(k), if k == 0 { V::Null } else { V::I(x) }],
            None => [V::Null, V::Null],
        });
        f.extend([V::I(n), tmax(m), V::S(c.flatten().unwrap_or("Not Closed"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(v.BountyAmount) AS TotalBounty, AVG(p.Score) AS AvgScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// CommentSummary AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, u.UserId, u.TotalPosts, u.TotalBounty, u.AvgScore, COALESCE(cs.CommentCount, 0) AS CommentCount,
//        CASE WHEN COALESCE(cs.CommentCount, 0) > 10 THEN 'Popular' WHEN COALESCE(cs.CommentCount, 0) BETWEEN 5 AND 10 THEN 'Moderately Discussed' ELSE 'Less Discussed' END AS DiscussionLevel,
//        CASE WHEN u.TotalPosts IS NULL THEN 'No Posts' ELSE 'Has Posts' END AS UserPostStatus
// FROM RankedPosts r INNER JOIN UserStats u ON r.OwnerUserId = u.UserId LEFT JOIN CommentSummary cs ON r.PostId = cs.PostId
// WHERE r.RecentPostRank = 1 AND r.Score > (SELECT AVG(Score) FROM Posts WHERE PostTypeId = 1) AND r.ViewCount IS NOT NULL ORDER BY r.Score DESC, r.ViewCount DESC LIMIT 50 OFFSET 0;
//
// RecentPostRank reads only base columns, so each owner's newest question is picked first and the posts x votes product is driven for those
// owners alone. AvgScore averages over the joined rows, so a post counts once per vote.
fn q21473(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let aq = db.post.with(post_type_id.eq(1)).select(score).fold_flat([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let r: MatSet<Id<Post>> = (&first).with(score.filt(move |s| s * aq[0] > aq[1])).with(view_count).collect();
    let owners: MatSet<Id<User>> = (&r).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((s, b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + s, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
            }
            None => a,
        });
    let tp = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cs = (&r).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cs).and(owner_user.select(Ident::<User>::new().and(&tp).and(&us))));
    let v = top_k(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p).unwrap())), |&(p, _)| p, 50);
    rows(v.into_iter().map(|(p, (c, ((u, n), a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([user_col(db, u, "uid"), V::I(n), nullable(a[3], a[2]), avg(a[1], a[0]), V::I(c)]);
        f.push(V::S(if c > 10 { "Popular" } else if (5..=10).contains(&c) { "Moderately Discussed" } else { "Less Discussed" }));
        f.push(V::S("Has Posts"));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COALESCE(SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, EXTRACT(YEAR FROM p.CreationDate) AS PostYear, COUNT(c.Id) AS CommentCount, COALESCE(MAX(h.CreationDate), '1970-01-01') AS LastEditDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory h ON p.Id = h.PostId GROUP BY p.Id, p.Title, p.CreationDate),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, RANK() OVER (ORDER BY us.Reputation DESC) AS UserRank FROM UserStats us WHERE us.PostCount > 0)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.UserRank, pa.Title, pa.CommentCount, pa.LastEditDate, pa.PostYear, us.TotalBounty, us.UpVotes, us.DownVotes, us.BadgeCount
// FROM TopUsers tu JOIN UserStats us ON tu.UserId = us.UserId JOIN PostActivity pa ON us.UserId = pa.PostId WHERE pa.PostYear = 2023 ORDER BY tu.UserRank, pa.CommentCount DESC LIMIT 50;
//
// `us.UserId = pa.PostId` joins a user id to a post id, so it goes through the raw ids. UserRank reads Reputation over the users with a post,
// and only users whose id is a 2023 post's id reach the output, so the posts x votes x badges product is driven for them alone.
fn q6156(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let rr = ranked(drain(db.user.with(&pc).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let rr = rel(rr.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, i64> = (&rr).map(|(u, _)| u).inv().select((&rr).map(|(_, r)| r)).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let y2023 = || Ident::<Post>::new().with(creation_date.map(year).eq(2023));
    let cand: MatSet<Id<User>> = db.user.with(&rank).with((&db.user.origid).select(&pidx).select(y2023())).collect();
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let v = p.flatten();
            [a[0] + v.and_then(|v| v.1).unwrap_or(0), a[1] + (v.map(|v| v.0) == Some(2)) as i64, a[2] + (v.map(|v| v.0) == Some(3)) as i64, a[3] + b.is_some() as i64]
        });
    let hit: MatSet<Id<Post>> = (&cand).select((&db.user.origid).select(&pidx)).collect();
    let pa = (&hit).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt())).fold((0i64, i64::MIN), |(n, m), (c, d)| {
        (n + c.is_some() as i64, m.max(d.unwrap_or(i64::MIN)))
    });
    let v = drain((&cand).select(Ident::<User>::new().and(&rank).and(&us).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&pa)))));
    let v = top_k(v, |&(_, (((_, r), _), (_, (n, _))))| (r, Reverse(n)), |&(u, _)| u, 50);
    rows(v.into_iter().map(|(_, (((u, r), a), (p, (n, m))))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(r));
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(n), V::T(if m == i64::MIN { 0 } else { m }), V::I(2023)]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, DisplayName, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostMetrics AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPosts
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostLinks pl ON p.Id = pl.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate),
// TopPosts AS (SELECT pm.PostId, pm.Title, pm.OwnerUserId, pm.CreationDate, pm.UpVotes, pm.DownVotes, pm.CommentCount, ur.DisplayName AS OwnerName, ur.ReputationRank
//     FROM PostMetrics pm JOIN UserReputation ur ON pm.OwnerUserId = ur.UserId WHERE pm.UpVotes - pm.DownVotes > 0 ORDER BY pm.UpVotes - pm.DownVotes DESC LIMIT 10)
// SELECT tp.Title, tp.OwnerName, tp.UpVotes, tp.DownVotes, tp.CommentCount, CASE WHEN tp.ReputationRank <= 10 THEN 'Top Influencer' ELSE 'Community Member' END AS UserType
// FROM TopPosts tp LEFT JOIN Posts p ON tp.PostId = p.Id WHERE p.ClosedDate IS NULL AND p.AnswerCount > 0 ORDER BY tp.UpVotes DESC, tp.CommentCount DESC;
//
// RelatedPosts is never read, but the PostLinks join multiplies the rows the sums run over, so it is driven.
fn q1776(db: &'static So) -> String {
    let Post { creation_date, owner_user, closed_date, answer_count, .. } = &db.post;
    let rank = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, i64> = (&rank).map(|(u, _)| u).inv().select((&rank).map(|(_, r)| r)).collect();
    let pm = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(links_of(db).opt()))
        .fold([0i64; 3], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let v = top_k(drain((&pm).filt(|a| a[0] - a[1] > 0)), |&(_, a)| Reverse(a[0] - a[1]), |&(p, _)| p, 10);
    let tp = rel(v);
    type T = (Id<Post>, [i64; 3]);
    let v = drain((&tp).select(Same::<T>::new().with(Same::<T>::new().map(|(p, _): T| p).select(Ident::<Post>::new().minus(closed_date).with(answer_count.gt(0))))
        .and(Same::<T>::new().map(|(p, _): T| p).select(owner_user.select(&rank)))));
    let mut v = v;
    v.sort_by_key(|&(_, ((_, a), _))| (Reverse(a[0]), Reverse(a[2])));
    rows(v.into_iter().map(|(_, ((p, a), r))| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(if r <= 10 { "Top Influencer" } else { "Community Member" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN p.Score ELSE 0 END) AS PositiveScore,
//        SUM(CASE WHEN p.Score < 0 THEN p.Score ELSE 0 END) AS NegativeScore, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, PositiveScore, NegativeScore, DENSE_RANK() OVER (ORDER BY TotalPosts DESC, PositiveScore DESC) AS Rank FROM UserActivity),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END AS IsAcceptedAnswer,
//        p.OwnerUserId FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId)
// SELECT tu.DisplayName, tu.TotalPosts, tu.PositiveScore, tu.NegativeScore, pd.Title, pd.CreationDate, pd.ViewCount, pd.CommentCount, pd.IsAcceptedAnswer
// FROM TopUsers tu JOIN PostDetails pd ON tu.UserId = pd.OwnerUserId WHERE tu.Rank <= 10 AND pd.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// ORDER BY tu.TotalPosts DESC, tu.PositiveScore DESC;
fn q2975(db: &'static So) -> String {
    let Post { score, creation_date, accepted_answer, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (s, _)| {
        let s = s.unwrap_or(0);
        [a[0] + s.max(0), a[1] + s.min(0)]
    });
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&tp).and(&ua)), |&(_, (n, a))| (Reverse(n), Reverse(a[0])), true);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let pd: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with((&db.post.owner_user).select(Ident::<User>::new().with(&tu)))
        .collect();
    let cc = (&pd).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain((&cc).and((&db.post.owner_user).select(Ident::<User>::new().and(&tp).and(&ua))));
    v.sort_by_key(|&(_, (_, ((_, n), a)))| (Reverse(n), Reverse(a[0])));
    rows(v.into_iter().map(|(p, (c, ((u, n), a)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1])];
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.extend([V::I(c), V::I(accepted_answer.get(p).is_some() as i64)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.Reputation),
// PostVotingStats AS (SELECT p.Id AS PostId, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT up.UserId, up.Reputation, up.PostCount, rp.PostId, rp.Title, rp.CreationDate, COALESCE(pvs.UpVotes, 0) AS TotalUpVotes, COALESCE(pvs.DownVotes, 0) AS TotalDownVotes,
//        (rp.Score * 1.0 / NULLIF(rp.ViewCount, 0)) AS ScorePerView, CASE WHEN rp.Score > 0 THEN 'Positive' WHEN rp.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreSentiment,
//        CASE WHEN rp.Score IS NULL THEN 'No Score' ELSE 'Score Available' END AS ScoreAvailability
// FROM UserReputation up JOIN RankedPosts rp ON up.UserId = rp.PostId LEFT JOIN PostVotingStats pvs ON rp.PostId = pvs.PostId WHERE rp.rn = 1 ORDER BY up.Reputation DESC, rp.CreationDate DESC;
//
// `up.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids. The ownerless posts are their own rn partition; a
// CreationDate tie goes to the smaller id.
fn q3721(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pvs = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let mut v = drain((&pvs).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&pc))));
    v.sort_by_key(|&(p, (_, (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (a, (u, n)))| {
        let s = score.get(p).unwrap();
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(match view_count.get(p) {
            Some(w) if w != 0 => V::F(s as f64 / w as f64),
            _ => V::Null,
        });
        f.extend([V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }), V::S("Score Available")]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS rn,
//        COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// UserReputation AS (SELECT u.Id AS UserId, COALESCE(SUM(b.Class), 0) AS TotalBadges, MAX(u.Reputation) AS Reputation FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostEngagement AS (SELECT p.Id AS PostId, COUNT(DISTINCT v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ue.Reputation, ue.TotalBadges, pe.VoteCount, pe.UpVotes, pe.DownVotes
// FROM RankedPosts rp JOIN UserReputation ue ON ue.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) LEFT JOIN PostEngagement pe ON pe.PostId = rp.PostId
// WHERE rp.rn <= 5 AND (rp.ViewCount IS NULL OR rp.ViewCount > 100) AND (ue.Reputation > 50 OR ue.TotalBadges > 2) ORDER BY rp.Score DESC, rp.CreationDate ASC;
//
// The LIMIT 1 subquery looks a post up by its id, so it is the post's owner. A tie in rn goes to the smaller id.
fn q21902(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + c.unwrap_or(0));
    let ue = Ident::<User>::new().and(&tb).filt(|(u, b): (Id<User>, i64)| db.user.reputation.get(u).unwrap() > 50 || b > 2);
    let pe = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let keep = Ident::<Post>::new().minus(view_count).or(Ident::<Post>::new().with(view_count.gt(100)));
    let mut v = drain((&rp).with(keep).select((&pe).and(owner_user.select(ue))));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([user_col(db, u, "rep"), V::I(b)]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostStats AS (SELECT p.OwnerUserId, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore FROM Posts p GROUP BY p.OwnerUserId),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(*) AS TotalVotes
//     FROM Votes v WHERE v.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 24 THEN 1 END) AS EditsCount
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT u.DisplayName, u.Reputation, ps.TotalPosts, ps.TotalViews, ps.TotalScore, COALESCE(rv.UpVotes, 0) AS RecentUpVotes, COALESCE(rv.DownVotes, 0) AS RecentDownVotes,
//        COALESCE(phe.CloseReopenCount, 0) AS CloseReopenCount, COALESCE(phe.EditsCount, 0) AS EditsCount, ur.ReputationRank
// FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN RecentVotes rv ON ps.OwnerUserId = rv.PostId LEFT JOIN PostHistorySummary phe ON ps.OwnerUserId = phe.PostId
// JOIN UserReputation ur ON u.Id = ur.UserId WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users WHERE Reputation IS NOT NULL) ORDER BY ur.ReputationRank;
//
// `ps.OwnerUserId = rv.PostId` and `= phe.PostId` join a user id to a post id, so they go through the raw ids (and only when the user has a
// PostStats row). RecentVotes groups on the raw Votes.PostId.
fn q3568(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let rank = rel(ranked(drain(reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, i64> = (&rank).map(|(u, _)| u).inv().select((&rank).map(|(_, r)| r)).collect();
    let ar = db.user.select(reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let Post { owner_user, view_count, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(view_count.opt().and(score)).fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let Vote { post_id, vote_type_id, creation_date: vd, .. } = &db.vote;
    let rv = db.vote.with(vd.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(post_id).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post_id: hp, post_history_type_id, .. } = &db.post_history;
    let phe = db.post_history.group_by(hp).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + matches!(t, 10 | 11) as i64, a[1] + (t == 24) as i64]);
    let side = (&db.user.origid).select((&rv).opt().and((&phe).opt()));
    let v = drain(db.user.with(reputation.filt(move |r| r * ar[0] > ar[1])).select((&ps).and(side).opt().and(&rank)));
    rows(v.into_iter().map(|(u, (p, r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        match p {
            Some((a, (v, h))) => {
                f.extend([V::I(a[0]), nullable(a[2], a[1]), V::I(a[3])]);
                f.extend(v.unwrap_or([0; 2]).map(V::I));
                f.extend(h.unwrap_or([0; 2]).map(V::I));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::I(0), V::I(0), V::I(0), V::I(0)]),
        }
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS total_posts FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostVoteStats AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 WHEN vt.Name = 'DownMod' THEN -1 ELSE 0 END) AS VoteScore FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId)
// SELECT up.DisplayName, COUNT(DISTINCT rp.Id) AS TotalQuestions, SUM(COALESCE(pvs.VoteScore, 0)) AS TotalVotes, ub.BadgeCount,
//        CASE WHEN ub.HighestBadgeClass IS NULL THEN 'No Badge' ELSE CASE WHEN ub.HighestBadgeClass = 1 THEN 'Gold' WHEN ub.HighestBadgeClass = 2 THEN 'Silver' ELSE 'Bronze' END END AS HighestBadge
// FROM Users up LEFT JOIN RankedPosts rp ON up.Id = rp.OwnerUserId LEFT JOIN UserBadges ub ON up.Id = ub.UserId LEFT JOIN PostVoteStats pvs ON rp.Id = pvs.PostId
// GROUP BY up.Id, up.DisplayName, ub.BadgeCount, ub.HighestBadgeClass HAVING COUNT(DISTINCT rp.Id) > 5 ORDER BY TotalVotes DESC, TotalQuestions DESC;
//
// rn and total_posts are never read. Each joined row is one question (UserBadges and PostVoteStats have one row per key).
fn q330(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let pvs = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold(0i64, |s, n| s + if n == "UpMod" { 1 } else if n == "DownMod" { -1 } else { 0 });
    let q = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))));
    let g = db.user.group_by(Ident::<User>::new()).select(q.select((&pvs).opt())).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s.unwrap_or(0)]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let mut v = drain((&g).filt(|a| a[0] > 5).and(&ub));
    v.sort_by_key(|&(_, (a, _))| (Reverse(a[1]), Reverse(a[0])));
    rows(v.into_iter().map(|(u, (a, (n, m)))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n), V::S(if n == 0 { "No Badge" } else if m == 1 { "Gold" } else if m == 2 { "Silver" } else { "Bronze" })])
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldCount, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeCount FROM Badges GROUP BY UserId),
// PostStats AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
//     GROUP BY p.Id, p.OwnerUserId),
// RankedPosts AS (SELECT ps.PostId, ps.OwnerUserId, ps.CommentCount, ps.UpVotes, ps.DownVotes, RANK() OVER (PARTITION BY ps.OwnerUserId ORDER BY ps.UpVotes - ps.DownVotes DESC) AS Rank FROM PostStats ps)
// SELECT u.DisplayName, u.Reputation, u.CreationDate, COALESCE(ub.BadgeCount, 0) AS TotalBadges, COALESCE(ub.GoldCount, 0) AS GoldBadges, COALESCE(ub.SilverCount, 0) AS SilverBadges,
//        COALESCE(ub.BronzeCount, 0) AS BronzeBadges, rp.PostId, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.Rank
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId
// WHERE u.Reputation > 1000 AND (ub.BadgeCount IS NOT NULL OR rp.PostId IS NOT NULL) AND (rp.Rank <= 5 OR rp.PostId IS NULL) ORDER BY u.Reputation DESC, rp.UpVotes DESC NULLS LAST;
//
// The WHERE on Reputation reads a base column, so the comment x vote product is driven only for those users' recent posts.
fn q297(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let hi: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(1000)).collect();
    let recent = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user.select(Ident::<User>::new().with(&hi)));
    let ps = recent.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let recent = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user.select(Ident::<User>::new().with(&hi)));
    let cc = recent.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&ps).and(&cc).and(owner_user)), |&(_, ((a, _), u))| (u, Reverse(a[0] - a[1])), false);
    let v = per_group(v, |&(_, (_, u))| u);
    let rp = rel(v.into_iter().map(|((p, ((a, c), u)), r)| (u, (p, a, c, r))).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, (Id<Post>, [i64; 2], i64, i64))> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    type X = (Option<[i64; 4]>, Option<(Id<User>, (Id<Post>, [i64; 2], i64, i64))>);
    let v = drain((&hi).select((&ub).opt().and((&by_user).opt())).filt(|(b, r): X| (b.is_some() || r.is_some()) && r.map_or(true, |(_, (_, _, _, k))| k <= 5)));
    let mut v = v;
    v.sort_by_key(|&(u, (_, r))| (Reverse(db.user.reputation.get(u).unwrap()), r.is_none(), Reverse(r.map(|(_, (_, a, _, _))| a[0]))));
    rows(v.into_iter().map(|(u, (b, r))| {
        let mut f = ucols(db, u, &["name", "rep", "ucreated"]);
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.extend(match r {
            Some((_, (p, a, c, k))) => [V::I(db.post.origid.get(p).unwrap()), V::I(c), V::I(a[0]), V::I(a[1]), V::I(k)],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// Rewritten (rewrites/29540.sql): the ORDER BY moved out of the FinalResult CTE into the outer query and refined with `, CreationDate, Title`.
// WITH PostTagCounts AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagCounts AS (SELECT Tag, COUNT(*) AS Count FROM PostTagCounts GROUP BY Tag), TopTags AS (SELECT Tag FROM TagCounts ORDER BY Count DESC LIMIT 10),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ARRAY_AGG(DISTINCT tt.Tag) AS Tags
//     FROM Posts p JOIN PostTagCounts ptc ON p.Id = ptc.PostId JOIN TopTags tt ON ptc.Tag = tt.Tag GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// AuthorInfo AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.Tags, u.DisplayName AS AuthorDisplayName, u.Reputation AS AuthorReputation, COUNT(c.Id) AS CommentCount
//     FROM PostDetails pd LEFT JOIN Users u ON pd.PostId = u.Id LEFT JOIN Comments c ON pd.PostId = c.PostId
//     GROUP BY pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.Tags, u.DisplayName, u.Reputation),
// FinalResult AS (SELECT AuthorDisplayName, AuthorReputation, Title, CreationDate, Score, ViewCount, Tags, CommentCount FROM AuthorInfo)
// SELECT * FROM FinalResult ORDER BY ViewCount DESC, Score DESC, CreationDate, Title LIMIT 20;
//
// `pd.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. ARRAY_AGG(DISTINCT) is sorted. The ten top tags have
// distinct counts at the cut.
fn q29540(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, score, creation_date, title, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let tc = qs().select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let top = top_k(drain(&tc), |&(_, n)| Reverse(n), |&(t, _)| t, 10);
    let tt: MatSet<Str> = rel(top.into_iter().map(|x| x.0).collect()).map(|t| t).collect();
    let pd = qs().group_by(Ident::<Post>::new()).select(tags_str.flat_map(tag_list).select(Same::<Str>::new().with(&tt))).buf_fold(|it| {
        let mut t: Vec<Str> = it.into_iter().collect();
        t.sort_unstable();
        t.dedup();
        &*Box::leak(t.into_boxed_slice())
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pk: MatSet<Id<Post>> = db.post.with(&pd).collect();
    let cc = (&pk).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&pd).and(&cc).and((&db.post.origid).select(&uidx).opt()));
    let v = top_k(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), title.get(p).is_none(), title.get(p))
    }, |&(p, _)| p, 20);
    rows(v.into_iter().map(|(p, ((t, c), u))| {
        let mut f = match u {
            Some(u) => ucols(db, u, &["name", "rep"]),
            None => vec![V::Null, V::Null],
        };
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend([V::L(t.iter().map(|s| V::S(s)).collect()), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.Body, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0 AND p.ViewCount > 100),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.Body, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.rn = 1),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments, STRING_AGG(c.Text, ' || ') AS CommentTexts FROM Comments c GROUP BY c.PostId),
// FinalResults AS (SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.AnswerCount, fp.CommentCount, pc.TotalComments, pc.CommentTexts, fp.OwnerDisplayName
//     FROM FilteredPosts fp LEFT JOIN PostComments pc ON fp.PostId = pc.PostId)
// SELECT fr.PostId, fr.Title, fr.CreationDate, fr.Score, fr.ViewCount, fr.AnswerCount, fr.CommentCount, COALESCE(fr.TotalComments, 0) AS TotalComments,
//        COALESCE(fr.CommentTexts, 'No comments') AS CommentTexts, fr.OwnerDisplayName
// FROM FinalResults fr ORDER BY fr.Score DESC, fr.ViewCount DESC LIMIT 20;
//
// STRING_AGG has no ORDER BY; the port joins the texts in comment id order, which is the order DuckDB produced here. A CreationDate tie for an
// owner's first question goes to the smaller id.
fn q27809(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0)).and(view_count.gt(100))).with(owner_user).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let fp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().and(&db.comment.text)).opt()).buf_fold(|it| {
        let mut x: Vec<(Id<Comment>, Str)> = it.into_iter().flatten().collect();
        x.sort_unstable_by_key(|x| x.0);
        let t: Vec<Str> = x.iter().map(|x| x.1).collect();
        (x.len() as i64, if t.is_empty() { None } else { Some(&*Box::leak(t.join(" || ").into_boxed_str())) })
    });
    let v = top_k(drain(&pc), |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p))), |&(p, _)| p, 20);
    rows(v.into_iter().map(|(p, (n, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([V::I(n), V::S(t.unwrap_or("No comments"))]);
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

// WITH RecentQuestions AS (SELECT p.Id AS QuestionId, p.Title, p.CreationDate, p.OwnerUserId, p.Body, STRING_AGG(t.TagName, ', ') AS Tags
//     FROM Posts p JOIN Tags t ON ',' || p.Tags || ',' LIKE '%,' || t.TagName || ',%' WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Body),
// UserEngagement AS (SELECT q.QuestionId, u.Id AS UserId, u.DisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpvoteCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownvoteCount
//     FROM RecentQuestions q JOIN Comments c ON q.QuestionId = c.PostId JOIN Users u ON u.Id = q.OwnerUserId LEFT JOIN Votes v ON v.PostId = q.QuestionId GROUP BY q.QuestionId, u.Id, u.DisplayName),
// RankedEngagement AS (SELECT ue.QuestionId, ue.DisplayName, ue.CommentCount, ue.UpvoteCount, ue.DownvoteCount,
//        ROW_NUMBER() OVER (ORDER BY (ue.CommentCount + ue.UpvoteCount - ue.DownvoteCount) DESC) AS EngagementRank FROM UserEngagement ue)
// SELECT rq.QuestionId, rq.DisplayName AS QuestionOwner, rq.CommentCount, rq.UpvoteCount, rq.DownvoteCount, rq.EngagementRank, r.Title, r.CreationDate, r.Body, r.Tags
// FROM RankedEngagement rq JOIN RecentQuestions r ON rq.QuestionId = r.QuestionId WHERE rq.EngagementRank <= 10 ORDER BY rq.EngagementRank;
//
// `',' || p.Tags || ','` is a '<a><b>' string wrapped in commas, which contains `,name,` only when the whole Tags string is exactly `name`
// (never, since it starts with '<'); the substring test is done over the distinct Tags strings with select_where, as the query says.
fn q28965(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, .. } = &db.post;
    let rq: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).collect();
    let strs: MatSet<Str> = (&rq).select(tags_str).collect();
    let hit: HashIdx<Str, Id<Tag>> = (&strs).select_where((&db.tag.tag_name).inv(), |s: Str, n: Str| format!(",{s},").contains(&format!(",{n},"))).collect();
    let tags = (&rq).group_by(Ident::<Post>::new()).select(tags_str.select(&hit)).buf_fold(|it| {
        let mut t: Vec<Id<Tag>> = it.into_iter().collect();
        t.sort_unstable();
        let n: Vec<Str> = t.into_iter().map(|t| db.tag.tag_name.get(t).unwrap()).collect();
        &*Box::leak(n.join(", ").into_boxed_str())
    });
    let tq: MatSet<Id<Post>> = db.post.with(&tags).collect();
    let ue = (&tq)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (_, t)| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&ue).and(&tags).and((&db.post.owner_user).select(Ident::<User>::new())));
    let v = top_k(v, |&(_, ((a, _), _))| Reverse(a[0] + a[1] - a[2]), |&(p, _)| p, 10);
    rows(v.into_iter().enumerate().map(|(i, (p, ((a, t), u)))| {
        let mut f = post_fields(db, p, &["id"]);
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        f.push(V::I(i as i64 + 1));
        f.extend(post_fields(db, p, &["title", "created", "body"]));
        f.push(V::S(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank, p.OwnerUserId FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.ViewCount > 1000),
// PostComments AS (SELECT pc.PostId, COUNT(pc.Id) AS CommentCount, STRING_AGG(pc.Text, '; ') AS Comments FROM Comments pc GROUP BY pc.PostId),
// PostsWithBadges AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(b.BadgeCount, 0) AS BadgeCount, rp.OwnerUserId
//     FROM RankedPosts rp LEFT JOIN (SELECT UserId, COUNT(Id) AS BadgeCount FROM Badges WHERE Class = 1 /* Gold badges */ GROUP BY UserId) b ON rp.OwnerUserId = b.UserId),
// FinalResults AS (SELECT pwb.Title, pwb.ViewCount, pwb.Score, pwb.BadgeCount, pc.CommentCount, pc.Comments, pwb.CreationDate FROM PostsWithBadges pwb LEFT JOIN PostComments pc ON pwb.PostId = pc.PostId)
// SELECT f.Title, f.ViewCount, f.Score, f.BadgeCount, f.CommentCount, f.Comments, DENSE_RANK() OVER (ORDER BY f.Score DESC, f.ViewCount DESC) AS RankScore
// FROM FinalResults f WHERE f.BadgeCount > 0 OR f.CommentCount > 5 ORDER BY RankScore, f.CreationDate DESC LIMIT 100;
//
// STRING_AGG has no ORDER BY; the port joins the texts in comment id order, which is the order DuckDB produced here. Rank is never read.
fn q31269(db: &'static So) -> String {
    let Post { view_count, owner_user, score, creation_date, .. } = &db.post;
    let gold = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let rp: MatSet<Id<Post>> = db.post.with(view_count.gt(1000)).with(owner_user).collect();
    let pc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().and(&db.comment.text))).buf_fold(|it| {
        let mut x: Vec<(Id<Comment>, Str)> = it.into_iter().collect();
        x.sort_unstable_by_key(|x| x.0);
        (x.len() as i64, &*Box::leak(x.iter().map(|x| x.1).collect::<Vec<Str>>().join("; ").into_boxed_str()))
    });
    type X = (Option<i64>, Option<(i64, Str)>);
    let v = drain((&rp).select(owner_user.select(&gold).opt().and((&pc).opt())).filt(|(b, c): X| b.unwrap_or(0) > 0 || c.map_or(false, |c| c.0 > 5)));
    let key = |p: Id<Post>| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p).unwrap()));
    let v = ranked(v, |&(p, _)| key(p), true);
    let v = top_k(v, |&((p, _), r)| (r, Reverse(creation_date.get(p).unwrap())), |&((p, _), _)| p, 100);
    rows(v.into_iter().map(|((p, (b, c)), r)| {
        let mut f = post_fields(db, p, &["title", "views", "score"]);
        f.push(V::I(b.unwrap_or(0)));
        f.extend(match c {
            Some((n, t)) => [V::I(n), V::S(t)],
            None => [V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(uc.UserCount, 0) AS UserCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(DISTINCT UserId) AS UserCount FROM Votes GROUP BY PostId) uc ON p.Id = uc.PostId
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostComments AS (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId),
// ClosedPosts AS (SELECT ph.PostId FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 AND ph.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months'),
// OverallStats AS (SELECT p.Id AS PostId, p.Title, p.Score, COALESCE(pc.CommentCount, 0) AS CommentCount, GREATEST(COALESCE(tp.UserCount, 0), 1) AS ActiveUsers
//     FROM Posts p LEFT JOIN PostComments pc ON p.Id = pc.PostId LEFT JOIN TopPosts tp ON p.Id = tp.PostId)
// SELECT ost.PostId, ost.Title, ost.Score, ost.CommentCount, CASE WHEN cp.PostId IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus, ur.Reputation, ur.ReputationRank
// FROM OverallStats ost LEFT JOIN ClosedPosts cp ON ost.PostId = cp.PostId JOIN Users u ON ost.PostId = u.Id JOIN UserReputation ur ON u.Id = ur.UserId
// WHERE ost.Score > 0 AND ur.ReputationRank <= 100 ORDER BY ost.Score DESC, ur.Reputation DESC;
//
// `ost.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. TopPosts has at most one row per post and only ActiveUsers
// (never projected) reads it, so it is not computed. ClosedPosts can repeat a post, and each repeat is a row.
fn q3778(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let ur = rel(v.into_iter().take_while(|x| x.1 <= 100).map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, i64> = (&ur).map(|(u, _)| u).inv().select((&ur).map(|(_, r)| r)).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10).and(hd.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ost = Ident::<Post>::new().with((&db.post.score).gt(0)).and(&cc).and(closes.opt());
    let mut v = drain((&rank).and((&db.user.origid).select(&pidx).select(ost)));
    v.sort_by_key(|&(u, (_, ((p, _), _)))| (Reverse(db.post.score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap())));
    rows(v.into_iter().map(|(u, (r, ((p, c), h)))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(c), V::S(if h.is_some() { "Closed" } else { "Open" }), user_col(db, u, "rep"), V::I(r)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS EditCount,
//        FIRST_VALUE(u.DisplayName) OVER (PARTITION BY p.Id ORDER BY p.CreationDate) AS FirstEditor
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN Users u ON p.LastEditorUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.Score, ps.TotalBounty, ps.CommentCount, ps.EditCount, RANK() OVER (ORDER BY ps.Score DESC, ps.TotalBounty DESC) AS PostRank,
//        CASE WHEN ps.CommentCount = 0 THEN 'No Comments' WHEN ps.CommentCount BETWEEN 1 AND 5 THEN 'Few Comments' ELSE 'Many Comments' END AS CommentCategory FROM PostStats ps)
// SELECT rp.PostId, rp.Title, rp.Score, rp.TotalBounty, rp.CommentCount, rp.EditCount, rp.PostRank, rp.CommentCategory, CASE WHEN rp.EditCount > 0 THEN 'Edited' ELSE 'Not Edited' END AS EditStatus,
//        CASE WHEN rp.TotalBounty IS NULL THEN 'No Bounty' ELSE 'Has Bounty' END AS BountyStatus
// FROM RankedPosts rp WHERE rp.PostRank <= 10 ORDER BY rp.PostRank;
//
// The last editor is at most one user, so each group is one post; FirstEditor is never read. TotalBounty is a COALESCE, never NULL, so
// BountyStatus is always 'Has Bounty'.
fn q21032(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)))
        .group_by(Ident::<Post>::new())
        .select(bounty.opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold(0i64, |s, ((b, _), _)| s + b.flatten().unwrap_or(0));
    let cc = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ec = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = ranked(drain((&ps).and(&cc).and(&ec)), |&(p, ((b, _), _))| (Reverse(score.get(p).unwrap()), Reverse(b)), false);
    let mut v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).collect();
    v.sort_by_key(|&((p, _), r)| (r, p));
    rows(v.into_iter().map(|((p, ((b, c), e)), r)| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(b), V::I(c), V::I(e), V::I(r)]);
        f.push(V::S(if c == 0 { "No Comments" } else if c <= 5 { "Few Comments" } else { "Many Comments" }));
        f.extend([V::S(if e > 0 { "Edited" } else { "Not Edited" }), V::S("Has Bounty")]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, ROW_NUMBER() OVER (PARTITION BY CASE WHEN Reputation < 1000 THEN 'Low' WHEN Reputation BETWEEN 1000 AND 5000 THEN 'Medium' ELSE 'High' END
//        ORDER BY Reputation DESC) AS Rank FROM Users),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.PostTypeId, P.CreationDate, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     GROUP BY P.Id, P.OwnerUserId, P.PostTypeId, P.CreationDate),
// PostStats AS (SELECT R.OwnerUserId, R.PostId, R.Upvotes, R.Downvotes, R.CommentCount, U.Rank AS ReputationRank FROM RecentPosts R JOIN UserReputation U ON R.OwnerUserId = U.Id)
// SELECT U.DisplayName, COUNT(DISTINCT P.PostId) AS TotalPosts, SUM(P.Upvotes) AS TotalUpvotes, SUM(P.Downvotes) AS TotalDownvotes, AVG(P.CommentCount) AS AvgComments,
//        CASE WHEN AVG(P.CommentCount) IS NULL THEN 'No Comments' ELSE CASE WHEN AVG(P.CommentCount) = 0 THEN 'No Interaction' ELSE 'Engaged' END END AS EngagementStatus
// FROM PostStats P JOIN Users U ON P.OwnerUserId = U.Id GROUP BY U.DisplayName, P.ReputationRank HAVING COUNT(DISTINCT P.PostId) > 5 ORDER BY TotalUpvotes DESC NULLS LAST;
//
// The group is (owner name, owner's rank in its tier); a Reputation tie in that rank goes to the smaller user id. Each PostStats row is one post.
fn q137(db: &'static So) -> String {
    let tier = |r: i64| if r < 1000 { 0 } else if r <= 5000 { 1 } else { 2 };
    let v = ranked(drain(&db.user.reputation), |&(u, r)| (tier(r), Reverse(r), u), false);
    let v = per_group(v, |&(_, r)| tier(r));
    let ur = rel(v.into_iter().map(|((u, _), k)| (u, k)).collect());
    let rank: HashIdx<Id<User>, i64> = (&ur).map(|(u, _)| u).inv().select((&ur).map(|(_, k)| k)).collect();
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user);
    let rp = recent.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 3], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]
    });
    let rv = db.post.with(&rp).collect::<MatSet<Id<Post>>>();
    let g = (&rv).group_by(owner_user.select((&db.user.display_name).and(&rank))).select(&rp).fold([0i64; 4], |a, x| [a[0] + 1, a[1] + x[0], a[2] + x[1], a[3] + x[2]]);
    let mut v = drain((&g).filt(|a| a[0] > 5));
    v.sort_by_key(|&(_, a)| Reverse(a[1]));
    rows(v.into_iter().map(|((name, _), a)| {
        row(vec![V::S(name), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::S(if a[3] == 0 { "No Interaction" } else { "Engaged" })])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// HighScorePosts AS (SELECT r.PostId, r.Title, r.OwnerUserId, r.Rank, u.Reputation FROM RankedPosts r JOIN UserReputation u ON r.OwnerUserId = u.UserId
//     WHERE r.Score > (SELECT AVG(Score) FROM Posts) AND r.Rank <= 2)
// SELECT p.Title, u.DisplayName, COALESCE(b.Class, 0) AS BadgeClass, COALESCE(b.Name, 'No Badge') AS BadgeName, SUM(v.BountyAmount) AS TotalBounty, COUNT(DISTINCT c.Id) AS CommentCount,
//        CASE WHEN COUNT(DISTINCT v.UserId) > 0 THEN true ELSE false END AS HasVotes
// FROM HighScorePosts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId AND b.Class = 1 LEFT JOIN Comments c ON p.PostId = c.PostId
// LEFT JOIN Votes v ON p.PostId = v.PostId AND v.VoteTypeId IN (2, 3)
// GROUP BY p.PostId, p.Title, u.DisplayName, b.Class, b.Name HAVING COUNT(DISTINCT c.Id) > 0 OR SUM(v.BountyAmount) > 0 ORDER BY SUM(v.BountyAmount) DESC, p.Title;
//
// The group is (post, owner's gold badge name), so the joined post x badge x comment x vote rows are materialised and grouped. A CreationDate
// tie at an owner's second place goes to the smaller id.
fn q20897(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, title, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 2, false);
    let top: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let av = db.post.select(score).fold_flat([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let hsp: MatSet<Id<Post>> = (&top).with(score.filt(move |s| s * av[0] > av[1])).collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    type J = (((Id<Post>, Option<Id<Badge>>), Option<Id<Comment>>), Option<Id<Vote>>);
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let j: MatSet<J> = (&hsp).select(Ident::<Post>::new().and(owner_user.select(gold).opt()).and(comments_of(db).opt()).and(up.opt())).collect();
    let post_of = (&j).map(|(((p, _), _), _): J| p);
    let badge_of = (&j).flat_map(|(((_, b), _), _): J| b);
    let g = (&j)
        .group_by((&post_of).and((&badge_of).select(&db.badge.name).opt()))
        .select(Same::<J>::new().map(|((_, c), v): J| (c, v)))
        .buf_fold(|it| {
            let x: Vec<(Option<Id<Comment>>, Option<Id<Vote>>)> = it.into_iter().collect();
            let bs: Vec<i64> = x.iter().filter_map(|x| x.1).filter_map(|v| db.vote.bounty_amount.get(v)).collect();
            let mut cs: Vec<Id<Comment>> = x.iter().filter_map(|x| x.0).collect();
            cs.sort_unstable();
            cs.dedup();
            let voters = distinct_some(x.iter().map(|x| x.1.and_then(|v| db.vote.user_id.get(v))));
            (if bs.is_empty() { None } else { Some(bs.iter().sum::<i64>()) }, cs.len() as i64, voters)
        });
    let mut v = drain((&g).filt(|(b, c, _)| c > 0 || b.map_or(false, |b| b > 0)));
    v.sort_by_key(|&((p, _), (b, _, _))| (b.is_none(), Reverse(b), title.get(p).is_none(), title.get(p)));
    rows(v.into_iter().map(|((p, n), (b, c, k))| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend([V::I(if n.is_some() { 1 } else { 0 }), V::S(n.unwrap_or("No Badge")), oint(b), V::I(c), V::B(k > 0)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COUNT(CASE WHEN b.Name IS NOT NULL THEN 1 END) AS BadgeCount,
//        COUNT(DISTINCT p.Id) AS TotalPosts
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.TotalBounties, ua.BadgeCount, ua.TotalPosts, RANK() OVER (ORDER BY ua.Reputation DESC) AS UserRank FROM UserActivity ua)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalBounties, tu.BadgeCount, tu.TotalPosts,
//        CASE WHEN tu.TotalPosts > 10 THEN 'High Contributor' WHEN tu.TotalPosts BETWEEN 5 AND 10 THEN 'Moderate Contributor' ELSE 'Low Contributor' END AS ContributorLevel,
//        rp.Title, rp.Score, rp.ViewCount
// FROM TopUsers tu LEFT JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId AND rp.rn = 1 WHERE tu.UserRank <= 10 ORDER BY tu.UserRank;
//
// UserRank reads only Reputation, so the top users are picked first and the votes x badges x posts product is driven for them alone. A tie for
// an owner's first post goes to the smaller id.
fn q1405(db: &'static So) -> String {
    let newer = db.user.with((&db.user.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let v = ranked(drain(newer.select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, i64> = (&tr).map(|(u, _)| u).inv().select((&tr).map(|(_, r)| r)).collect();
    let tu: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()).and(posts_of(db).opt()))
        .fold([0i64; 2], |a, ((b, g), _)| [a[0] + b.flatten().unwrap_or(0), a[1] + g.is_some() as i64]);
    let tp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user.select(Ident::<User>::new().with(&tu))).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(first.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, Id<Post>> = (&rp).map(|(u, _)| u).inv().select((&rp).map(|(_, p)| p)).collect();
    let mut v = drain((&rank).and(&ua).and(&tp).and((&by_user).opt()));
    v.sort_by_key(|&(_, (((r, _), _), _))| r);
    rows(v.into_iter().map(|(u, (((_, a), n), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        f.push(V::S(if n > 10 { "High Contributor" } else if n >= 5 { "Moderate Contributor" } else { "Low Contributor" }));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score", "views"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, t.TagName,
//        RANK() OVER (PARTITION BY t.TagName ORDER BY p.ViewCount DESC) AS TagRank, COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpvoteCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id CROSS JOIN LATERAL (SELECT unnest(string_to_array(p.Tags, '><')) AS TagName) AS t
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopRankedPosts AS (SELECT PostId, Title, Body, CreationDate, ViewCount, Score, OwnerDisplayName, TagName, TagRank, UpvoteCount FROM RankedPosts WHERE TagRank = 1),
// PostStatistics AS (SELECT TagName, COUNT(*) AS PostCount, AVG(Score) AS AvgScore, AVG(ViewCount) AS AvgViewCount, SUM(UpvoteCount) AS TotalUpvotes FROM TopRankedPosts GROUP BY TagName)
// SELECT ps.TagName, ps.PostCount, ps.AvgScore, ps.AvgViewCount, ps.TotalUpvotes, th.CreatedBy AS TopContributor
// FROM PostStatistics ps JOIN (SELECT p.Tags, u.DisplayName AS CreatedBy FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1
//     GROUP BY p.Tags, u.DisplayName ORDER BY COUNT(*) DESC LIMIT 1) th ON th.Tags = ps.TagName ORDER BY ps.PostCount DESC;
//
// `string_to_array(p.Tags, '><')` splits on '><' only, so the pieces keep their outer '<' and '>'. The th group with the most questions is
// unique on this data (41 against 32).
fn q25903(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, view_count, score, .. } = &db.post;
    let recent = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user);
    let v = drain(recent.select(tags_str.flat_map(|t: Str| t.split("><"))));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, 1, true);
    type R = (Id<Post>, Str);
    let trp = rel(top);
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let upc = (&trp).map(|(p, _): R| p).collect::<MatSet<Id<Post>>>();
    let upc = (&upc).group_by(Ident::<Post>::new()).select(up.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let ps = (&trp)
        .group_by(Same::<R>::new().map(|(_, t): R| t))
        .select(Same::<R>::new().map(|(p, _): R| p).select(score.and(view_count.opt()).and(&upc)))
        .fold([0i64; 5], |a, ((s, w), u)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + u]);
    let th = db.post.with(post_type_id.eq(1)).with(owner_user).group_by(tags_str.opt().and(owner_user.select(&db.user.display_name))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let th = top_k(drain(&th), |&(_, n)| Reverse(n), |&(k, _)| k, 1);
    let th = rel(th.into_iter().map(|x| x.0).collect());
    let th: HashIdx<Option<Str>, Str> = (&th).map(|(t, _)| t).inv().select((&th).map(|(_, n)| n)).collect();
    let mut v = drain((&ps).and(Same::<Str>::new().map(|t: Str| Some(t)).select(&th)));
    v.sort_by_key(|&(_, (a, _))| Reverse(a[0]));
    rows(v.into_iter().map(|(t, (a, n))| row(vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4]), V::S(n)])))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(VB.BountyAmount) FILTER (WHERE VB.VoteTypeId = 9), 0) AS TotalBounty,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes,
//        COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN P.Id END) AS TotalAcceptedAnswers, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
//     LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Votes VB ON U.Id = VB.UserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, TotalBounty, TotalUpvotes, TotalDownvotes, TotalPosts, TotalAcceptedAnswers, TotalComments,
//        RANK() OVER (ORDER BY TotalBounty DESC, TotalUpvotes DESC) AS UserRank FROM UserStats)
// SELECT RU.DisplayName, RU.TotalBounty, RU.TotalUpvotes, RU.TotalDownvotes, RU.TotalPosts, RU.TotalAcceptedAnswers, RU.TotalComments,
//        CASE WHEN RU.TotalPosts > 0 THEN ROUND((CAST(RU.TotalUpvotes AS DECIMAL) / NULLIF(RU.TotalPosts, 0)) * 100, 2) ELSE 0 END AS UpvotePercentage,
//        CASE WHEN RU.TotalPosts > 0 THEN ROUND((CAST(RU.TotalAcceptedAnswers AS DECIMAL) / NULLIF(RU.TotalPosts, 0)) * 100, 2) ELSE 0 END AS AcceptanceRate
// FROM RankedUsers RU WHERE RU.UserRank <= 10 ORDER BY RU.TotalBounty DESC, RU.TotalUpvotes DESC;
//
// The FROM is the per-user product of recent posts x their votes x the user's own votes x the posts' comments, and it is driven.
fn q1843(db: &'static So) -> String {
    let Post { creation_date, accepted_answer, .. } = &db.post;
    let recent = || posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(recent().select(votes_of(db).select(vote_type_id).opt().and(comments_of(db).opt())).opt().and(votes_by(db).select(vote_type_id.and(bounty_amount.opt())).opt()))
        .fold([0i64; 3], |a, (p, vb)| {
            let t = p.and_then(|(t, _)| t);
            let b = vb.and_then(|(t, b)| if t == 9 { b } else { None });
            [a[0] + b.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
        });
    let tp = db.user.group_by(Ident::<User>::new()).select(recent().select(accepted_answer.opt()).opt()).fold([0i64; 2], |a, p| match p {
        Some(x) => [a[0] + 1, a[1] + x.is_some() as i64],
        None => a,
    });
    let tc = db.user.group_by(Ident::<User>::new()).select(recent().select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&us).and(&tp).and(&tc)), |&(_, ((a, _), _))| (Reverse(a[0]), Reverse(a[1])), false);
    let pct = |x: i64, n: i64| if n > 0 { V::F((x as f64 / n as f64 * 100.0 * 100.0).round() / 100.0) } else { V::F(0.0) };
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, p), c)), _)| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(p[0]), V::I(p[1]), V::I(c), pct(a[1], p[0]), pct(p[1], p[0])])
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("2461", q2461),
    ("26654", q26654),
    ("27930", q27930),
    ("487", q487),
    ("2774", q2774),
    ("1066", q1066),
    ("3940", q3940),
    ("23779", q23779),
    ("574", q574),
    ("7412", q7412),
    ("29222", q29222),
    ("3152", q3152),
    ("27292", q27292),
    ("4492", q4492),
    ("26537", q26537),
    ("28296", q28296),
    ("3837", q3837),
    ("193", q193),
    ("2733", q2733),
    ("2188", q2188),
    ("602", q602),
    ("5059", q5059),
    ("21405", q21405),
    ("6645", q6645),
    ("28041", q28041),
    ("2924", q2924),
    ("2222", q2222),
    ("9522", q9522),
    ("23099", q23099),
    ("21168", q21168),
    ("1140", q1140),
    ("2373", q2373),
    ("4187", q4187),
    ("823", q823),
    ("3438", q3438),
    ("2245", q2245),
    ("555", q555),
    ("7546", q7546),
    ("981", q981),
    ("6058", q6058),
    ("26450", q26450),
    ("25444", q25444),
    ("2722", q2722),
    ("849", q849),
    ("22513", q22513),
    ("21473", q21473),
    ("6156", q6156),
    ("1776", q1776),
    ("2975", q2975),
    ("3721", q3721),
    ("21902", q21902),
    ("3568", q3568),
    ("330", q330),
    ("297", q297),
    ("29540", q29540),
    ("27809", q27809),
    ("28965", q28965),
    ("31269", q31269),
    ("3778", q3778),
    ("21032", q21032),
    ("137", q137),
    ("20897", q20897),
    ("1405", q1405),
    ("25903", q25903),
    ("1843", q1843),
];
