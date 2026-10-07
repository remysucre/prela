use harness::prelude::*;
use std::cmp::Reverse;

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

fn epoch(us: i64) -> f64 {
    (us / DAY_US) as f64 * 86400.0 + (us % DAY_US) as f64 / 1e6
}

// WITH RECURSIVE UserReputation AS (SELECT U.Id AS UserId, U.Reputation, 0 AS Level FROM Users U WHERE U.Reputation IS NOT NULL
//     UNION ALL SELECT U.Id AS UserId, (U.Reputation + 100) AS Reputation, Level + 1 FROM Users U INNER JOIN UserReputation UR ON U.Id = UR.UserId WHERE Level < 5),
// PostStats AS (SELECT P.Id AS PostId, P.OwnerUserId, COUNT(C.Id) AS CommentCount, MAX(V.CreationDate) AS LastVoteDate
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.OwnerUserId),
// UserPostStats AS (SELECT U.Id AS UserId, COALESCE(SUM(CASE WHEN P.OwnerUserId = U.Id THEN 1 ELSE 0 END), 0) AS PostCount,
//        COALESCE(SUM(CASE WHEN PS.CommentCount > 0 THEN 1 ELSE 0 END), 0) AS PostsWithComments, AVG(PS.CommentCount) AS AvgCommentCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostStats PS ON P.Id = PS.PostId GROUP BY U.Id)
// SELECT U.DisplayName, UR.Reputation AS Reputation, UPS.PostCount, UPS.PostsWithComments, UPS.AvgCommentCount, P.Title, P.CreationDate, P.LastActivityDate
// FROM Users U JOIN UserReputation UR ON U.Id = UR.UserId JOIN UserPostStats UPS ON U.Id = UPS.UserId
// LEFT OUTER JOIN Posts P ON P.OwnerUserId = U.Id AND P.Id IN (SELECT Id FROM Posts ORDER BY Score DESC LIMIT 5)
// WHERE UR.Level >= 1 ORDER BY UR.Reputation DESC, UPS.PostCount DESC;
fn q30183(db: &'static So) -> String {
    let Post { score, .. } = &db.post;
    let ur = db.user.reach(Same::<Id<User>>::new(), 5);
    let eng = engagement(db, &db.post.id);
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&eng).opt()).fold([0i64; 3], |a, e| match e {
        Some((c, _, _, _)) => [a[0] + 1, a[1] + (c > 0) as i64, a[2] + c],
        None => a,
    });
    let top: MatSet<Id<Post>> = rel(top_n(drain(db.post.select(score)), |&(_, s)| Reverse(s), 5).into_iter().map(|x| x.0).collect()).collect();
    let v = drain((&ur).filt(|l: usize| l >= 1).inv().select(Ident::<User>::new().and(&ups).and(posts_of(db).select(Ident::<Post>::new().with(&top)).opt())));
    rows(v.into_iter().map(|(_, ((u, a), p))| {
        let mut f = ucols(db, u, &["name"]);
        f.push(V::I(db.user.reputation.get(u).unwrap() + 100));
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[0])]);
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "created", "activity"])),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RECURSIVE UserVotes AS (SELECT u.Id AS UserId, u.DisplayName, v.VoteTypeId, 1 AS VoteCount FROM Users u JOIN Votes v ON u.Id = v.UserId WHERE v.VoteTypeId IN (2, 3)
//     UNION ALL SELECT u.Id, u.DisplayName, v.VoteTypeId, uv.VoteCount + 1 FROM Users u JOIN Votes v ON u.Id = v.UserId JOIN UserVotes uv ON uv.UserId = u.Id WHERE uv.VoteCount < 10),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN bh.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseReopenedCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory bh ON p.Id = bh.PostId
//     WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, MAX(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, MAX(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        MAX(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT ps.PostId, ps.Title, ps.CommentCount, ps.UpVotes, ps.DownVotes, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, uv.DisplayName AS TopVoter, uv.VoteCount
// FROM PostStatistics ps LEFT JOIN UserBadges ub ON ub.UserId = ps.PostId
// LEFT JOIN (SELECT UserId, DisplayName, SUM(VoteCount) AS VoteCount FROM UserVotes GROUP BY UserId, DisplayName) uv ON 1=1
// WHERE ps.CloseReopenedCount > 0 ORDER BY ps.UpVotes DESC, ps.CommentCount DESC;
fn q32626(db: &'static So) -> String {
    let Vote { vote_type_id, user, .. } = &db.vote;
    let Post { creation_date, origid, .. } = &db.post;
    type X = (Id<User>, i64);
    let base = db.vote.with(vote_type_id.is_in([2, 3])).select(user.and(vote_type_id));
    let step = Same::<X>::new().map(|x: X| x.0).select(votes_by(db).select(user.and(vote_type_id)));
    let uv = rel(drain(base.reach(step, 9)));
    let uv = drain((&uv).group_by(Same::<(X, usize)>::new().map(|x: (X, usize)| x.0 .0)).fold(0i64, |a, (_, l)| a + l as i64 + 1));
    let uv = left_all(uv);
    let ub = db.badge.group_by(&db.badge.user_id).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1].max((c == 1) as i64), a[2].max((c == 2) as i64), a[3].max((c == 3) as i64)]);
    let ps = db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0))).group_by(Ident::<Post>::new())
        .select(votes_of(db).select(vote_type_id).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()).and(comments_of(db).opt()))
        .fold([0i64; 3], |a, ((v, h), _)| [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + matches!(h, Some(10 | 11)) as i64]);
    let cpp = comments_per_post(db);
    let v = drain(db.post.with((&ps).filt(|a: [i64; 3]| a[2] > 0)).select(Ident::<Post>::new().and(&ps).and(&cpp).and(origid.select((&ub).opt()))).cross(&uv));
    rows(v.into_iter().map(|(_, ((((p, a), c), b), u))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        match b {
            Some(b) => f.extend(b.iter().map(|&x| V::I(x))),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        match u {
            Some((u, n)) => f.extend([ucols(db, u, &["name"]).remove(0), V::I(n)]),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.AcceptedAnswerId, p.ParentId, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.Title, p.PostTypeId, p.AcceptedAnswerId, p.ParentId, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId WHERE ph.Level < 5),
// AnswerStats AS (SELECT p.Id AS PostId, COUNT(a.Id) AS TotalAnswers, AVG(a.Score) AS AvgAnswerScore FROM Posts p LEFT JOIN Posts a ON a.ParentId = p.Id AND a.PostTypeId = 2 GROUP BY p.Id),
// VoteStatistics AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON v.PostId = p.Id GROUP BY p.Id)
// SELECT ph.PostId, ph.Title, ph.Level, ah.TotalAnswers, ah.AvgAnswerScore, vs.UpVotes, vs.DownVotes,
//        CASE WHEN ah.TotalAnswers > 0 THEN ROUND((CAST(vs.UpVotes AS DECIMAL) / NULLIF((vs.UpVotes + vs.DownVotes), 0)) * 100, 2) ELSE NULL END AS UpvotePercentage,
//        COUNT(DISTINCT c.Id) AS CommentCount, MAX(c.CreationDate) AS LatestComment
// FROM PostHierarchy ph LEFT JOIN AnswerStats ah ON ph.PostId = ah.PostId LEFT JOIN VoteStatistics vs ON ph.PostId = vs.PostId LEFT JOIN Comments c ON c.PostId = ph.PostId
// GROUP BY ph.PostId, ph.Title, ph.Level, ah.TotalAnswers, ah.AvgAnswerScore, vs.UpVotes, vs.DownVotes ORDER BY ph.Level, ph.PostId;
fn q33009(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    type X = (Id<Post>, usize);
    let ph: MatSet<X> = rel(drain(db.post.with(post_type_id.eq(1)).reach(children_of(db), 4))).collect();
    let ans = db.post.group_by(Ident::<Post>::new()).select(answers_of(db).select(score).opt()).fold((0i64, 0i64), |(n, s), x| match x {
        Some(x) => (n + 1, s + x),
        None => (n, s),
    });
    let vs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let cs = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    let v = drain((&ph).select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.0).select((&ans).and(&vs).and(&cs)))));
    rows(v.into_iter().map(|(_, ((p, l), (((n, s), (u, d)), (c, m))))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        let pct = if n > 0 && u + d != 0 { V::F(((u as f64 / (u + d) as f64) * 100.0 * 100.0).round() / 100.0) } else { V::Null };
        f.extend([V::I(l as i64 + 1), V::I(n), avg(s, n), V::I(u), V::I(d), pct, V::I(c), if c > 0 { V::T(m) } else { V::Null }]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.AcceptedAnswerId, 0 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.Title, p.PostTypeId, p.AcceptedAnswerId, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId),
// VoteCounts AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v INNER JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// ClosedPostCount AS (SELECT p.OwnerUserId AS UserId, COUNT(*) AS ClosedPosts FROM Posts p WHERE p.Id IN (SELECT PostId FROM PostHistory WHERE PostHistoryTypeId IN (10, 12)) GROUP BY p.OwnerUserId)
// SELECT ph.PostId, ph.Title, ph.Level, COALESCE(vc.UpVotes, 0) AS UpVotes, COALESCE(vc.DownVotes, 0) AS DownVotes, ur.Reputation, COALESCE(cpc.ClosedPosts, 0) AS ClosedPosts,
//        CASE WHEN ur.Reputation > 1000 THEN 'Top Contributor' WHEN ur.Reputation > 500 THEN 'Active Contributor' ELSE 'New Contributor' END AS ContributorLevel
// FROM PostHierarchy ph LEFT JOIN VoteCounts vc ON ph.PostId = vc.PostId LEFT JOIN Users u ON ph.PostId = u.Id LEFT JOIN UserReputation ur ON u.Id = ur.UserId
// LEFT JOIN ClosedPostCount cpc ON u.Id = cpc.UserId
// WHERE ph.Level = (SELECT MAX(Level) FROM PostHierarchy) AND ur.Reputation IS NOT NULL
// ORDER BY COALESCE(cpc.ClosedPosts, 0) DESC, COALESCE(vc.UpVotes, 0) DESC, ur.Reputation DESC;
//
// `ph.PostId = u.Id` compares a post id with a user id.
fn q33934(db: &'static So) -> String {
    let Post { post_type_id, origid, owner_user, .. } = &db.post;
    type X = (Id<Post>, usize);
    let phv = rel(drain(db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX)));
    let mx = (&phv).fold_flat(0usize, |m, x: X| m.max(x.1));
    let vc = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold((0i64, 0i64), |(u, d), n| (u + (n == "UpMod") as i64, d + (n == "DownMod") as i64));
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let closed = db.post.with(history_of(db).select(&db.post_history.post_history_type_id).filt(|t: i64| t == 10 || t == 12)).group_by(owner_user).fold(0i64, |n, _| n + 1);
    let v = drain((&phv).filt(move |x: X| x.1 == mx).select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.0).select((&vc).opt().and(origid.select(&uid).select(Ident::<User>::new().and((&closed).opt())))))));
    rows(v.into_iter().map(|(_, ((p, l), (c, (u, k))))| {
        let (up, dn) = c.unwrap_or((0, 0));
        let r = db.user.reputation.get(u).unwrap();
        let lv = if r > 1000 { "Top Contributor" } else if r > 500 { "Active Contributor" } else { "New Contributor" };
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(l as i64), V::I(up), V::I(dn), V::I(r), V::I(k.unwrap_or(0)), V::S(lv)]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputationCTE AS (SELECT u.Id AS UserId, u.Reputation, u.CreationDate, 0 AS Depth FROM Users u WHERE u.Reputation IS NOT NULL
//     UNION ALL SELECT u.Id, u.Reputation, u.CreationDate, ur.Depth + 1 FROM Users u INNER JOIN UserReputationCTE ur ON u.Id = ur.UserId WHERE ur.Depth < 5),
// PostStats AS (SELECT p.Id AS PostId, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, MAX(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS HasAcceptedAnswer,
//        SUM(COALESCE(p.Score, 0)) AS TotalScore, p.OwnerUserId FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.OwnerUserId),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, ur.Reputation, ps.PostId, ps.CommentCount, ps.VoteCount, ps.HasAcceptedAnswer, ps.TotalScore
//     FROM Users u JOIN UserReputationCTE ur ON u.Id = ur.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT ups.UserId, ups.DisplayName, ups.Reputation, COUNT(ups.PostId) AS NumberOfPosts, SUM(COALESCE(ups.CommentCount, 0)) AS TotalComments, SUM(COALESCE(ups.VoteCount, 0)) AS TotalVotes,
//        SUM(COALESCE(ups.TotalScore, 0)) AS TotalScore, AVG(CASE WHEN ups.HasAcceptedAnswer = 1 THEN 1 ELSE NULL END) AS AcceptedAnswerRate
// FROM UserPostStats ups GROUP BY ups.UserId, ups.DisplayName, ups.Reputation HAVING SUM(ups.TotalScore) > 0 ORDER BY TotalScore DESC LIMIT 10;
fn q31185(db: &'static So) -> String {
    let Post { score, accepted_answer_id, .. } = &db.post;
    let ps = db.post.group_by(Ident::<Post>::new()).select(score.and(comments_of(db).opt()).and(votes_of(db).opt())).fold(0i64, |s, ((x, _), _)| s + x);
    let pc = (&db.post.id).select(Ident::<Post>::new().and(&ps).and(comments_per_post(db)).and(votes_per_post(db)));
    type X = (Id<User>, usize);
    let ur = rel(drain(db.user.reach(Same::<Id<User>>::new(), 5)));
    let g = (&ur).group_by(Same::<X>::new().map(|x: X| x.0)).select(Same::<X>::new().map(|x: X| x.0).select(posts_of(db).select(&pc).opt())).fold([0i64; 5], |a, p| match p {
        Some((((p, s), c), v)) => [a[0] + 1, a[1] + c, a[2] + v, a[3] + s, a[4] + accepted_answer_id.get(p).is_some() as i64],
        None => a,
    });
    let v = drain((&g).filt(|a: [i64; 5]| a[0] > 0 && a[3] > 0));
    let v = top_n(v, |&(_, a)| Reverse(a[3]), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), if a[4] > 0 { V::F(1.0) } else { V::Null }]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.ParentId, 0 AS Level, p.Title, p.CreationDate FROM Posts p WHERE p.ParentId IS NULL
//     UNION ALL SELECT p.Id AS PostId, p.ParentId, ph.Level + 1, p.Title, p.CreationDate FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId),
// PostStats AS (SELECT p.Id, p.Title, p.ViewCount, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(d.DownVotes, 0) AS DownVotes, COALESCE(c.CommentCount, 0) AS CommentCount,
//        COALESCE(b.BadgeCount, 0) AS BadgeCount, p.CreationDate FROM Posts p
//     LEFT JOIN (SELECT PostId, COUNT(*) AS UpVotes FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS DownVotes FROM Votes WHERE VoteTypeId = 3 GROUP BY PostId) d ON p.Id = d.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON p.OwnerUserId = b.UserId),
// ClosedPosts AS (SELECT ph.PostId, ph.Level, ph.Title, ph.CreationDate FROM PostHierarchy ph INNER JOIN PostHistory phist ON ph.PostId = phist.PostId WHERE phist.PostHistoryTypeId = 10),
// RecentPosts AS (SELECT ps.* FROM PostStats ps WHERE ps.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days')
// SELECT rp.Title, rp.UpVotes, rp.DownVotes, rp.CommentCount, COALESCE(cp.Level, -1) AS ClosedLevel, CASE WHEN COALESCE(cp.Level, -1) > -1 THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM RecentPosts rp LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId ORDER BY rp.UpVotes DESC, rp.CreationDate DESC;
fn q31229(db: &'static So) -> String {
    let Post { parent, creation_date, .. } = &db.post;
    type X = (Id<Post>, usize);
    let ph = rel(drain(db.post.minus(parent).reach(children_of(db), usize::MAX)));
    let h10 = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let cpr = rel(drain((&ph).select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.0).select(&h10))).map(|(x, _): (X, Id<PostHistory>)| x)).into_iter().map(|x| x.1).collect());
    let cp: HashIdx<Id<Post>, usize> = (&cpr).map(|x: X| x.0).inv().select((&cpr).map(|x: X| x.1)).collect();
    let cut = add_days(utc_to_ny(now_utc()), -30);
    let v = drain(db.post.with(creation_date.ge(cut)).select(Ident::<Post>::new().and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).and(comments_per_post(db)).and((&cp).opt())));
    rows(v.into_iter().map(|(_, ((((p, u), d), c), l))| {
        let l = l.map(|l| l as i64).unwrap_or(-1);
        row(vec![title(db, p), V::I(u), V::I(d), V::I(c), V::I(l), V::S(if l > -1 { "Closed" } else { "Open" })])
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id, p.Title, p.OwnerUserId, p.AcceptedAnswerId, p.CreationDate, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT a.Id, a.Title, a.OwnerUserId, a.AcceptedAnswerId, a.CreationDate, ph.Level + 1 FROM Posts a INNER JOIN PostHierarchy ph ON a.ParentId = ph.Id WHERE a.PostTypeId = 2)
// SELECT u.DisplayName AS UserName, COUNT(DISTINCT ph.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN ph.AcceptedAnswerId IS NOT NULL THEN ph.AcceptedAnswerId END) AS AcceptedAnswers,
//        AVG(EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - ph.CreationDate)) / 3600) AS AvgAgeInHours, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM PostHierarchy ph JOIN Users u ON ph.OwnerUserId = u.Id LEFT JOIN Posts ans ON ph.Id = ans.ParentId LEFT JOIN Tags t ON t.ExcerptPostId = ph.Id LEFT JOIN Votes v ON v.PostId = ph.Id
// GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT ph.Id) > 1 AND AVG(EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - ph.CreationDate)) / 3600) < 24
// ORDER BY TotalPosts DESC LIMIT 10;
fn q32329(db: &'static So) -> String {
    let Post { post_type_id, owner_user, accepted_answer_id, creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    type X = (Id<Post>, usize);
    let step = children_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2)));
    let ph = rel(drain(db.post.with(post_type_id.eq(1)).reach(step, usize::MAX)));
    let tags: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let prod = Ident::<Post>::new().and(children_of(db).opt()).and((&tags).select(&db.tag.tag_name).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt());
    type R = (((Id<Post>, Option<Id<Post>>), Option<Str>), Option<i64>);
    let g = (&ph).group_by(Same::<X>::new().map(|x: X| x.0).select(owner_user)).select(Same::<X>::new().map(|x: X| x.0).select(prod)).buf_fold(|v| {
        let mut ps: Vec<Id<Post>> = v.iter().map(|r: &R| r.0 .0 .0).collect();
        ps.sort();
        ps.dedup();
        let mut acc: Vec<i64> = v.iter().filter_map(|r: &R| accepted_answer_id.get(r.0 .0 .0)).collect();
        acc.sort();
        acc.dedup();
        let age: f64 = v.iter().map(|r: &R| epoch(t0 - creation_date.get(r.0 .0 .0).unwrap()) / 3600.0).sum::<f64>() / v.len() as f64;
        let mut t: Vec<Str> = v.iter().filter_map(|r: &R| r.0 .1).collect();
        t.sort();
        t.dedup();
        let tg = if t.is_empty() { None } else { Some(leak(t.join(", "))) };
        let up = v.iter().filter(|r: &&R| r.1 == Some(2)).count() as i64;
        let dn = v.iter().filter(|r: &&R| r.1 == Some(3)).count() as i64;
        (ps.len() as i64, acc.len() as i64, age, tg, up, dn)
    });
    let v = drain((&g).filt(|(n, _, a, _, _, _): (i64, i64, f64, Option<Str>, i64, i64)| n > 1 && a < 24.0));
    let v = top_n(v, |&(_, (n, ..))| Reverse(n), 10);
    rows(v.into_iter().map(|(u, (n, a, age, t, up, dn))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(n), V::I(a), V::F(age), ostr(t), V::I(up), V::I(dn)]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.ParentId, 0 AS Level FROM Posts p WHERE p.ParentId IS NULL
//     UNION ALL SELECT p.Id AS PostId, p.Title, p.ParentId, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId),
// UserVoteCounts AS (SELECT v.UserId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.UserId),
// RecentBadges AS (SELECT b.UserId, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b WHERE b.Date > cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY b.UserId)
// SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.UpVotes, 0) AS UpVotes, COALESCE(ub.DownVotes, 0) AS DownVotes, rb.BadgeNames, ph.PostId, ph.Title, ph.Level, p.CreationDate, p.ViewCount, p.Score
// FROM Users u LEFT JOIN UserVoteCounts ub ON u.Id = ub.UserId LEFT JOIN RecentBadges rb ON u.Id = rb.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN PostHierarchy ph ON p.Id = ph.PostId
// WHERE (ub.UpVotes - ub.DownVotes) > 10 AND (p.Score > 5 OR p.ViewCount > 100) ORDER BY p.CreationDate DESC, ph.Level ASC;
//
// The STRING_AGG order is left open; the port joins in badge id order.
fn q33993(db: &'static So) -> String {
    let Post { parent, score, view_count, .. } = &db.post;
    type X = (Id<Post>, usize);
    let ph = rel(drain(db.post.minus(parent).reach(children_of(db), usize::MAX)));
    let phi: HashIdx<Id<Post>, usize> = (&ph).map(|x: X| x.0).inv().select((&ph).map(|x: X| x.1)).collect();
    let ub = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold((0i64, 0i64), |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64));
    let rb = db.badge.with((&db.badge.date).gt(add_years(date(2024, 10, 1), -1))).group_by(&db.badge.user).select(&db.badge.name).buf_fold(|v| leak(v.join(", ")));
    let posts = posts_of(db).select(Ident::<Post>::new().with(score.and(view_count.opt()).filt(|(s, w): (i64, Option<i64>)| s > 5 || w.map_or(false, |w| w > 100))));
    let v = drain(db.user.with((&ub).filt(|(u, d): (i64, i64)| u - d > 10)).select(Ident::<User>::new().and(&ub).and((&rb).opt()).and(posts.select(Ident::<Post>::new().and((&phi).opt())))));
    rows(v.into_iter().map(|(_, (((u, (up, dn)), b), (p, l)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(up), V::I(dn), ostr(b)]);
        match l {
            Some(l) => f.extend([post_fields(db, p, &["id"]).remove(0), title(db, p), V::I(l as i64)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.extend(post_fields(db, p, &["created", "views", "score"]));
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.ParentId, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id AS PostId, p.Title, p.ParentId, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId WHERE p.PostTypeId = 2),
// PostDetails AS (SELECT ph.PostId, ph.Title, ph.Level, p.CreationDate, p.Score, COALESCE(COUNT(DISTINCT c.Id), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY p.CreationDate DESC) AS rn
//     FROM PostHierarchy ph INNER JOIN Posts p ON ph.PostId = p.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY ph.PostId, ph.Title, ph.Level, p.CreationDate, p.Score),
// Ranking AS (SELECT PD.PostId, PD.Title, PD.Level, PD.CreationDate, PD.Score, PD.CommentCount, PD.UpVotes, PD.DownVotes, RANK() OVER (ORDER BY PD.Score DESC) AS ScoreRank FROM PostDetails PD)
// SELECT R.PostId, R.Title, R.Level, R.CreationDate, R.Score, R.CommentCount, R.UpVotes, R.DownVotes, R.ScoreRank FROM Ranking R
// WHERE R.Level = 2 AND R.UpVotes > R.DownVotes ORDER BY R.ScoreRank;
//
// ScoreRank ranks every level, so it is taken before the Level = 2 filter.
fn q32732(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    type X = (Id<Post>, usize);
    let step = children_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2)));
    let ph = rel(drain(db.post.with(post_type_id.eq(1)).reach(step, usize::MAX)));
    let vs = (&ph).group_by(Same::<X>::new()).select(Same::<X>::new().map(|x: X| x.0).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))).fold((0i64, 0i64), |(u, d), (t, _)| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let pd = drain(whole(&vs).select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.0).select(score).and(&vs).and(Same::<X>::new().map(|x: X| x.0).select(comments_per_post(db))))));
    let rk = ranked(pd.into_iter().map(|x| x.1).collect(), |&(_, ((s, _), _))| Reverse(s), false);
    type Y = ((X, ((i64, (i64, i64)), i64)), i64);
    let v = drain(rel(rk).filt(|((x, ((_, (u, d)), _)), _): Y| x.1 == 1 && u > d));
    rows(v.into_iter().map(|(_, (((p, l), ((_, (u, d)), c)), r))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::I(l as i64 + 1));
        f.extend(post_fields(db, p, &["created", "score"]));
        f.extend([V::I(c), V::I(u), V::I(d), V::I(r)]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT Id, DisplayName, Reputation, CreationDate, LastAccessDate, 0 AS Depth FROM Users WHERE Reputation >= 1000
//     UNION ALL SELECT u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate, ur.Depth + 1 FROM Users u INNER JOIN UserReputation ur ON u.Id = ur.Id WHERE ur.Depth < 5),
// PostStats AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, COUNT(DISTINCT ph.Id) AS EditCount, MAX(p.CreationDate) AS LastActivity
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.OwnerUserId),
// UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ps.CommentCount, 0) AS TotalComments, COALESCE(ps.VoteCount, 0) AS TotalVotes, COALESCE(ps.UpvoteCount, 0) AS TotalUpvotes,
//        COALESCE(ps.DownvoteCount, 0) AS TotalDownvotes, COALESCE(ps.EditCount, 0) AS TotalEdits FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId),
// TopUsers AS (SELECT um.UserId, um.DisplayName, um.TotalComments, um.TotalVotes, um.TotalUpvotes, um.TotalDownvotes, um.TotalEdits, ROW_NUMBER() OVER (ORDER BY um.TotalComments DESC) AS Rank FROM UserMetrics um)
// SELECT tu.Rank, tu.DisplayName, tu.TotalComments, tu.TotalVotes, tu.TotalUpvotes, tu.TotalDownvotes, tu.TotalEdits, ur.Reputation
// FROM TopUsers tu JOIN UserReputation ur ON tu.UserId = ur.Id WHERE tu.Rank <= 10 ORDER BY ur.Reputation DESC;
fn q34678(db: &'static So) -> String {
    let ps = db.post.group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).opt()))
        .fold([0i64; 4], |a, ((c, v), _)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]);
    let um = drain(db.user.select(Ident::<User>::new().and(posts_of(db).select((&ps).and(history_per_post(db))).opt())));
    let top = top_n(um, |&(_, (_, m))| Reverse(m.map_or(0, |(a, _)| a[0])), 10);
    type X = (Id<User>, usize);
    let ur = rel(drain(db.user.with((&db.user.reputation).ge(1000)).reach(Same::<Id<User>>::new(), 5)));
    let uri: HashIdx<Id<User>, usize> = (&ur).map(|x: X| x.0).inv().select((&ur).map(|x: X| x.1)).collect();
    type T = (Id<User>, Option<([i64; 4], i64)>);
    let tr = rel(top.into_iter().enumerate().map(|(i, (_, t))| (i as i64 + 1, t)).collect());
    let v = drain((&tr).select(Same::<(i64, T)>::new().and(Same::<(i64, T)>::new().map(|(_, (u, _)): (i64, T)| u).select(&uri))));
    rows(v.into_iter().map(|(_, ((k, (u, m)), _))| {
        let (a, e) = m.unwrap_or(([0; 4], 0));
        let mut f = vec![V::I(k)];
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(e)]);
        f.extend(ucols(db, u, &["rep"]));
        row(f)
    }))
}

// WITH RECURSIVE TagHierarchy AS (SELECT Id, TagName, Count, ExcerptPostId, WikiPostId, 1 AS Level FROM Tags WHERE WikiPostId IS NOT NULL
//     UNION ALL SELECT t.Id, t.TagName, t.Count, t.ExcerptPostId, t.WikiPostId, th.Level + 1 FROM Tags t INNER JOIN TagHierarchy th ON t.ExcerptPostId = th.Id),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COALESCE(SUM(P.AnswerCount), 0) AS TotalAnswers, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 2 LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentEdits AS (SELECT PH.PostId, PH.UserId, PH.CreationDate, PH.Comment, PH.Text FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (4, 5) AND PH.CreationDate >= (CURRENT_TIMESTAMP - INTERVAL '30 days')),
// TopUsers AS (SELECT UserId, COUNT(*) AS EditCount, RANK() OVER (ORDER BY COUNT(*) DESC) AS Rank FROM RecentEdits GROUP BY UserId HAVING COUNT(*) > 1)
// SELECT U.DisplayName AS User, U.Reputation, U.Upvotes, U.Downvotes, U.TotalAnswers, TH.TagName AS TopTag, TH.Count AS TagCount, RU.EditCount, RU.Rank
// FROM UserStats U LEFT JOIN Tags T ON U.UserId = T.ExcerptPostId LEFT JOIN TagHierarchy TH ON T.Id = TH.Id JOIN TopUsers RU ON RU.UserId = U.UserId
// WHERE U.TotalAnswers > 5 AND U.Reputation > 100 AND (TH.Count IS NULL OR TH.Count > 10) ORDER BY U.Reputation DESC, RU.EditCount DESC LIMIT 50;
//
// `t.ExcerptPostId = th.Id` and `U.UserId = T.ExcerptPostId` compare a post id with a tag id and a user id. TopUsers' NULL UserId group joins nothing, so it is not built.
fn q34443(db: &'static So) -> String {
    let Tag { excerpt_post_id, origid, count, .. } = &db.tag;
    let exi: HashIdx<i64, Id<Tag>> = excerpt_post_id.inv().collect();
    type X = (Id<Tag>, usize);
    let th = rel(drain(db.tag.with(&db.tag.wiki_post_id).reach(origid.select(&exi), usize::MAX)));
    let thi: HashIdx<Id<Tag>, usize> = (&th).map(|x: X| x.0).inv().select((&th).map(|x: X| x.1)).collect();
    let us = db.user.group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(2))).select((&db.post.answer_count).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, x| match x {
            Some((n, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + n.unwrap_or(0)],
            None => a,
        });
    let cut = add_days(utc_to_ny(now_utc()), -30);
    let PostHistory { post_history_type_id, creation_date, user, .. } = &db.post_history;
    let re = db.post_history.with(post_history_type_id.is_in([4, 5])).with(creation_date.ge(cut)).group_by(user).fold(0i64, |n, _| n + 1);
    let ru = ranked(drain((&re).filt(|n: i64| n > 1)), |&(_, n)| Reverse(n), false);
    let ru = rel(ru);
    let rui: HashIdx<Id<User>, (i64, i64)> = (&ru).map(|((u, _), _)| u).inv().select((&ru).map(|((_, n), k)| (n, k))).collect();
    let ti = (&db.user.origid).select(&exi).select(Ident::<Tag>::new().and((&thi).opt())).opt();
    let v = drain(
        db.user
            .with((&db.user.reputation).gt(100))
            .with((&us).filt(|a: [i64; 3]| a[2] > 5))
            .select(Ident::<User>::new().and(&us).and(&rui).and(ti.filt(move |t: Option<(Id<Tag>, Option<usize>)>| match t {
                Some((t, Some(_))) => count.get(t).unwrap() > 10,
                _ => true,
            }))),
    );
    let v = top_n(v, |&(_, (((u, _), (n, _)), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n)), 50);
    rows(v.into_iter().map(|(_, (((u, a), (n, k)), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        match t {
            Some((t, Some(_))) => f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(count.get(t).unwrap())]),
            _ => f.extend([V::Null, V::Null]),
        }
        f.extend([V::I(n), V::I(k)]);
        row(f)
    }))
}

// WITH RECURSIVE RecursivePostTree AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.ParentId, 0 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.Title, p.OwnerUserId, p.ParentId, pt.Level + 1 FROM Posts p INNER JOIN RecursivePostTree pt ON p.ParentId = pt.PostId)
// SELECT u.DisplayName AS UserName, u.Reputation, rpt.Level, COUNT(DISTINCT p.Id) AS AnswerCount, COALESCE(SUM(CASE WHEN v.VoteTypeId IN (2, 6) THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags, COUNT(DISTINCT c.Id) AS CommentCount,
//        AVG(p.Score) AS AvgScore, MAX(p.CreationDate) AS LastActivityDate
// FROM RecursivePostTree rpt JOIN Posts p ON rpt.PostId = p.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN LATERAL (SELECT t.TagName FROM Tags t WHERE t.TagName = ANY(string_to_array(p.Tags, ','))) AS t ON TRUE
// WHERE (u.Reputation > 1000 OR u.DisplayName IS NOT NULL) GROUP BY u.Id, u.DisplayName, u.Reputation, rpt.Level ORDER BY u.Reputation DESC, rpt.Level LIMIT 10;
fn q30881(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, score, creation_date, .. } = &db.post;
    type X = (Id<Post>, usize);
    let ph = rel(drain(db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX)));
    let tn: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let tags = tags_str.flat_map(|s: Str| s.split(',')).select(&tn).select(&db.tag.tag_name);
    let prod = Ident::<Post>::new().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt()).and(tags.opt());
    let key = Same::<X>::new().map(|x: X| x.0).select(owner_user).and(Same::<X>::new().map(|x: X| x.1));
    type R = (((Id<Post>, Option<i64>), Option<Id<Comment>>), Option<Str>);
    let g = (&ph).group_by(key).select(Same::<X>::new().map(|x: X| x.0).select(prod)).buf_fold(|v| {
        let mut ps: Vec<Id<Post>> = v.iter().map(|r: &R| r.0 .0 .0).collect();
        ps.sort();
        ps.dedup();
        let mut cs: Vec<Id<Comment>> = v.iter().filter_map(|r: &R| r.0 .1).collect();
        cs.sort();
        cs.dedup();
        let mut t: Vec<Str> = v.iter().filter_map(|r: &R| r.1).collect();
        t.sort();
        t.dedup();
        let tg = if t.is_empty() { None } else { Some(leak(t.join(", "))) };
        let up = v.iter().filter(|r: &&R| matches!(r.0 .0 .1, Some(2 | 6))).count() as i64;
        let dn = v.iter().filter(|r: &&R| r.0 .0 .1 == Some(3)).count() as i64;
        let s: i64 = v.iter().map(|r: &R| score.get(r.0 .0 .0).unwrap()).sum();
        let m = v.iter().map(|r: &R| creation_date.get(r.0 .0 .0).unwrap()).max().unwrap();
        (ps.len() as i64, up, dn, tg, cs.len() as i64, s, v.len() as i64, m)
    });
    let v = top_n(drain(&g), |&((u, l), _)| (Reverse(db.user.reputation.get(u).unwrap()), l), 10);
    rows(v.into_iter().map(|((u, l), (n, up, dn, t, c, s, k, m))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(l as i64), V::I(n), V::I(up), V::I(dn), ostr(t), V::I(c), avg(s, k), V::T(m)]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT P.Id AS PostId, P.Title, P.ParentId, 0 AS Level FROM Posts P WHERE P.ParentId IS NULL
//     UNION ALL SELECT P.Id AS PostId, P.Title, P.ParentId, PH.Level + 1 FROM Posts P JOIN PostHierarchy PH ON P.ParentId = PH.PostId),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, STRING_AGG(B.Name, ', ') AS Badges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PopularPosts AS (SELECT P.Id, P.Title, P.ViewCount, ROW_NUMBER() OVER (ORDER BY P.ViewCount DESC) AS RN FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month')
// SELECT PH.PostId, PH.Title AS QuestionTitle, U.DisplayName AS OwnerDisplayName, UB.BadgeCount, UB.Badges, PP.ViewCount AS Popularity, COALESCE(COUNT(C.Id), 0) AS CommentCount,
//        SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty
// FROM PostHierarchy PH JOIN Users U ON PH.PostId = U.Id LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN Posts P ON P.Id = PH.PostId LEFT JOIN Comments C ON C.PostId = PH.PostId
// LEFT JOIN Votes V ON V.PostId = PH.PostId AND V.VoteTypeId = 8 JOIN PopularPosts PP ON P.Id = PP.Id
// WHERE P.PostTypeId = 1 GROUP BY PH.PostId, PH.Title, U.DisplayName, UB.BadgeCount, UB.Badges, PP.ViewCount ORDER BY Popularity DESC, CommentCount DESC;
//
// `PH.PostId = U.Id` compares a post id with a user id. The STRING_AGG order is left open; the port joins in badge id order. RN is never read.
fn q32893(db: &'static So) -> String {
    let Post { parent, post_type_id, creation_date, origid, .. } = &db.post;
    type X = (Id<Post>, usize);
    let ph = rel(drain(db.post.minus(parent).reach(children_of(db), usize::MAX)));
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name).opt()).buf_fold(|v| {
        let n: Vec<Str> = v.iter().filter_map(|x| *x).collect();
        (n.len() as i64, if n.is_empty() { None } else { Some(leak(n.join(", "))) })
    });
    let b8 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let pp = Ident::<Post>::new().with(post_type_id.eq(1)).with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)));
    let g = (&ph).with(Same::<X>::new().map(|x: X| x.0).select(pp).select(origid).select(&uid)).group_by(Same::<X>::new().map(|x: X| x.0)).select(Same::<X>::new().map(|x: X| x.0).select(comments_of(db).opt().and(b8.opt()))).fold((0i64, 0i64), |(c, s), (ci, b)| (c + ci.is_some() as i64, s + b.flatten().unwrap_or(0)));
    let v = drain(db.post.with(&g).select(Ident::<Post>::new().and(&g).and(origid.select(&uid).select(Ident::<User>::new().and(&ub)))));
    rows(v.into_iter().map(|(_, ((p, (c, s)), (u, (n, b))))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(n), ostr(b)]);
        f.extend(post_fields(db, p, &["views"]));
        f.extend([V::I(c), V::I(s)]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.ParentId, 1 AS Level FROM Posts p WHERE p.ParentId IS NULL
//     UNION ALL SELECT p.Id, p.ParentId, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId),
// UserStats AS (SELECT u.Id AS UserId, CASE WHEN COUNT(DISTINCT b.Id) > 0 THEN 'Has Badges' ELSE 'No Badges' END AS BadgeStatus, SUM(v.BountyAmount) AS TotalBounty
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostDetails AS (SELECT p.Id, p.Title, p.CreationDate, p.SCORE, p.ViewCount, ph.Level AS HierarchyLevel, us.BadgeStatus, us.TotalBounty,
//        ROW_NUMBER() OVER (PARTITION BY us.BadgeStatus ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN PostHierarchy ph ON p.Id = ph.PostId LEFT JOIN UserStats us ON p.OwnerUserId = us.UserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.HierarchyLevel, pd.BadgeStatus, pd.TotalBounty, STRING_AGG(t.TagName, ', ') AS Tags
// FROM PostDetails pd LEFT JOIN LATERAL (SELECT unnest(string_to_array(p.Tags, '>')) AS TagName FROM Posts p WHERE p.Id = pd.Id) t ON TRUE
// GROUP BY pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.HierarchyLevel, pd.BadgeStatus, pd.TotalBounty
// HAVING pd.Score > 5 AND pd.ViewCount > 100 ORDER BY pd.Score DESC, pd.ViewCount DESC;
//
// The HAVING reads only group keys, so it filters the posts first. STRING_AGG has no ORDER BY; DuckDB emits the pieces last to first.
fn q32009(db: &'static So) -> String {
    let Post { creation_date, score, view_count, title, tags_str, owner_user, parent_id, .. } = &db.post;
    let ph: HashIdx<Id<Post>, usize> = db.post.minus(parent_id).reach(children_of(db), usize::MAX).collect();
    let us = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold(
        (false, 0i64, 0i64),
        |(b, s, k), (bd, v)| {
            let x = v.flatten();
            (b || bd.is_some(), s + x.unwrap_or(0), k + x.is_some() as i64)
        },
    );
    type K = (Option<Str>, i64, i64, Option<i64>, Option<usize>, Option<(bool, i64, i64)>);
    type T = (K, Option<Str>);
    let pd = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(score.gt(5))
        .with(view_count.gt(100))
        .select(
            title.opt().and(creation_date).and(score).and(view_count.opt()).and((&ph).opt()).and(owner_user.select(&us).opt()).and(tags_str.flat_map(|t: Str| t.split('>')).opt()),
        )
        .map(|((((((t, c), s), w), l), u), g)| ((t, c, s, w, l, u), g));
    let r = rel(drain(&pd).into_iter().map(|x| x.1).collect::<Vec<T>>());
    let g = (&r).group_by(Same::<T>::new().map(|x: T| x.0)).select(Same::<T>::new().map(|x: T| x.1)).buf_fold(|v| {
        let p: Vec<Str> = v.iter().rev().filter_map(|x| *x).collect();
        if p.is_empty() { None } else { Some(leak(p.join(", "))) }
    });
    let v = top_n(drain(&g), |&((_, _, s, w, _, _), _)| (Reverse(s), Reverse(w)), usize::MAX);
    rows(v.into_iter().map(|((t, c, s, w, l, u), g)| {
        let (st, b) = match u {
            Some((b, s, k)) => (V::S(if b { "Has Badges" } else { "No Badges" }), nullable(s, k)),
            None => (V::Null, V::Null),
        };
        row(vec![ostr(t), V::T(c), V::I(s), oint(w), oint(l.map(|l| l as i64 + 1)), st, b, ostr(g)])
    }))
}

// WITH RECURSIVE PopularPosts AS (SELECT p.Id, p.Title, p.Score, p.OwnerUserId, 1 AS Level FROM Posts p WHERE p.Score > 10
//     UNION ALL SELECT p.Id, p.Title, p.Score, p.OwnerUserId, pp.Level + 1 FROM Posts p JOIN PopularPosts pp ON p.ParentId = pp.Id WHERE p.Score > 10),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// PostActivity AS (SELECT p.Id AS PostId, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(v.VoteCount, 0) AS VoteCount FROM Posts p
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId)
// SELECT pp.Title AS PopularPostTitle, pp.Score AS PopularPostScore, ur.UserId, ur.Reputation AS UserReputation, ur.PostCount AS UserPostCount, pa.CommentCount, pa.VoteCount
// FROM PopularPosts pp JOIN UserReputation ur ON pp.OwnerUserId = ur.UserId JOIN PostActivity pa ON pp.Id = pa.PostId
// WHERE ur.Reputation > 5000 ORDER BY pp.Score DESC, ur.Reputation DESC LIMIT 10;
fn q32489(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let hot = || Ident::<Post>::new().with(score.gt(10));
    let pp = db.post.with(score.gt(10)).reach(children_of(db).select(hot()), usize::MAX);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let users = Ident::<User>::new().with((&db.user.reputation).gt(5000)).and(&db.user.reputation).and(&pc);
    let v = drain(pp.inv().select(Ident::<Post>::new().and(score).and(owner_user.select(users)).and(comments_per_post(db)).and(votes_per_post(db))));
    let v = top_n(v, |&(_, ((((_, s), ((_, r), _)), _), _))| (Reverse(s), Reverse(r)), 10);
    rows(v.into_iter().map(|(_, ((((p, s), ((u, r), n)), c), w))| {
        let mut f = vec![title(db, p), V::I(s)];
        f.extend(ucols(db, u, &["uid"]));
        f.extend([V::I(r), V::I(n), V::I(c), V::I(w)]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id, p.Title, p.ParentId, p.CreationDate, 0 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.Title, p.ParentId, p.CreationDate, ph.Level + 1 FROM Posts p JOIN PostHierarchy ph ON p.ParentId = ph.Id),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END) AS VoteScore FROM Votes v GROUP BY v.PostId),
// PostEditHistory AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(*) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeBadges, SUM(b.Class) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT p.Id AS PostId, p.Title, p.CreationDate AS PostCreationDate, ph.Level, COALESCE(v.VoteScore, 0) AS TotalVotes, COALESCE(e.EditCount, 0) AS TotalEdits,
//        e.LastEditDate, COALESCE(ub.UserId, -1) AS OwnerId, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ub.TotalBadges
// FROM Posts p LEFT JOIN PostHierarchy ph ON p.Id = ph.Id LEFT JOIN PostVotes v ON p.Id = v.PostId LEFT JOIN PostEditHistory e ON p.Id = e.PostId
// LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// WHERE p.CreationDate >= '2023-01-01' AND (p.Score >= 10 OR p.ViewCount > 100) ORDER BY p.CreationDate DESC;
fn q34520(db: &'static So) -> String {
    let Post { creation_date, score, view_count, post_type_id, owner_user, .. } = &db.post;
    let ph: HashIdx<Id<Post>, usize> = db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX).collect();
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |s, t| s + (t == 2) as i64 - (t == 3) as i64);
    let ph_t = &db.post_history.post_history_type_id;
    let eh = db.post_history.with(ph_t.filt(|t| (4..=6).contains(&t))).group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 5], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64, a[3] + c, a[4] + 1],
        None => a,
    });
    let v = drain(
        db.post
            .with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0)))
            .with(score.ge(10).or(view_count.gt(100)))
            .select(Ident::<Post>::new().and((&ph).opt()).and((&pv).opt()).and((&eh).opt()).and(owner_user.select(Ident::<User>::new().and(&ub)).opt())),
    );
    rows(v.into_iter().map(|(_, ((((p, l), w), e), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([oint(l.map(|l| l as i64)), V::I(w.unwrap_or(0)), V::I(e.map_or(0, |e| e.0)), ots(e.map(|e| e.1))]);
        match u {
            Some((u, a)) => {
                f.extend(ucols(db, u, &["uid"]));
                f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[4])]);
            }
            None => f.extend([V::I(-1), V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id, p.Title, p.ParentId, 1 AS Depth FROM Posts p WHERE p.ParentId IS NULL
//     UNION ALL SELECT p.Id, p.Title, p.ParentId, ph.Depth + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.Id),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, AVG(v.BountyAmount) AS AverageBounty, MAX(v.CreationDate) AS LastVoteDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// VoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v GROUP BY v.PostId)
// SELECT ph.Id AS PostId, ph.Title AS PostTitle, COALESCE(us.GoldBadges, 0) AS GoldBadges, COALESCE(us.SilverBadges, 0) AS SilverBadges,
//        COALESCE(us.BronzeBadges, 0) AS BronzeBadges, ps.CommentCount, ps.AverageBounty, vs.UpVotes, vs.DownVotes, ph.Depth
// FROM PostHierarchy ph LEFT JOIN UserBadges us ON us.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = ph.Id) LEFT JOIN PostStats ps ON ps.PostId = ph.Id
// LEFT JOIN VoteSummary vs ON vs.PostId = ph.Id WHERE ph.Depth <= 5 ORDER BY ph.Depth, ps.CommentCount DESC, vs.UpVotes DESC;
//
// Depth <= 5 caps the recursion at level 4 (Depth = level + 1).
fn q32106(db: &'static So) -> String {
    let Post { parent_id, owner_user, .. } = &db.post;
    let ph = db.post.minus(parent_id).reach(children_of(db), 4);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let ps = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).fold((0i64, 0i64, 0i64), |(c, s, n), (cm, b)| {
        let b = b.flatten();
        (c + cm.is_some() as i64, s + b.unwrap_or(0), n + b.is_some() as i64)
    });
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold((0i64, 0i64), |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64));
    let v = drain(ph.inv().select(Ident::<Post>::new().and(owner_user.select(&ub).opt()).and(&ps).and((&vs).opt())));
    rows(v.into_iter().map(|(l, (((p, b), (c, s, n)), w))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(c), avg(s, n), oint(w.map(|w| w.0)), oint(w.map(|w| w.1)), V::I(l as i64 + 1)]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.ParentId, p.Title, p.CreationDate, 0 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id AS PostId, p.ParentId, p.Title, p.CreationDate, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId WHERE p.PostTypeId = 2),
// PostVoteSummary AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(*) AS TotalVotes
//     FROM Votes GROUP BY PostId),
// PostHistorySummary AS (SELECT PostId, COUNT(*) AS EditCount, MAX(CASE WHEN PostHistoryTypeId = 10 THEN CreationDate END) AS LastClosedDate FROM PostHistory GROUP BY PostId),
// UserBadgeSummary AS (SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT ph.PostId, ph.Title, ph.CreationDate, ph.Level, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes,
//        COALESCE(pvs.TotalVotes, 0) AS TotalVotes, COALESCE(phs.EditCount, 0) AS EditCount, phs.LastClosedDate, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM PostHierarchy ph LEFT JOIN PostVoteSummary pvs ON ph.PostId = pvs.PostId LEFT JOIN PostHistorySummary phs ON ph.PostId = phs.PostId
// LEFT JOIN Posts post ON ph.PostId = post.Id LEFT JOIN Users u ON post.OwnerUserId = u.Id LEFT JOIN UserBadgeSummary ub ON u.Id = ub.UserId
// WHERE (ph.Level = 0 AND (pvs.TotalVotes IS NULL OR pvs.TotalVotes > 5)) OR (ph.Level > 0 AND (pvs.UpVotes > pvs.DownVotes OR (phs.LastClosedDate IS NULL)));
fn q32433(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let ph = db.post.with(post_type_id.eq(1)).reach(answers_of(db), usize::MAX);
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let PostHistory { post_history_type_id, creation_date, post, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id.and(creation_date)).fold((0i64, None::<i64>), |(n, m), (t, d)| (n + 1, if t == 10 { m.max(Some(d)) } else { m }));
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let v = drain(
        (&ph)
            .and((&pvs).opt().and((&phs).opt()).and(owner_user.select(&ub).opt()))
            .filt(|(l, ((w, h), _)): (usize, ((Option<[i64; 3]>, Option<(i64, Option<i64>)>), Option<[i64; 3]>))| {
                if l == 0 { w.is_none_or(|w| w[2] > 5) } else { w.is_some_and(|w| w[0] > w[1]) || h.is_none_or(|h| h.1.is_none()) }
            }),
    );
    rows(v.into_iter().map(|(p, (l, ((w, h), b)))| {
        let w = w.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(l as i64), V::I(w[0]), V::I(w[1]), V::I(w[2]), V::I(h.map_or(0, |h| h.0)), ots(h.and_then(|h| h.1))]);
        match b {
            Some(b) => f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2])]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT Id, ParentId, Title, CreationDate, Score, 0 AS Level FROM Posts WHERE ParentId IS NULL
//     UNION ALL SELECT p.Id, p.ParentId, p.Title, p.CreationDate, p.Score, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.Id),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN ph.Level = 0 THEN 2 * p.Score WHEN ph.Level = 1 THEN p.Score ELSE 0 END) AS ReputationScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostHierarchy ph ON p.Id = ph.Id GROUP BY u.Id, u.DisplayName),
// ActiveBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldCount, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverCount,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeCount FROM Badges b WHERE b.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY b.UserId),
// FinalUserStats AS (SELECT u.DisplayName, COALESCE(ur.ReputationScore, 0) AS Reputation, COALESCE(ab.GoldCount, 0) AS GoldBadges, COALESCE(ab.SilverCount, 0) AS SilverBadges,
//        COALESCE(ab.BronzeCount, 0) AS BronzeBadges FROM Users u LEFT JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN ActiveBadges ab ON u.Id = ab.UserId)
// SELECT f.DisplayName, f.Reputation, f.GoldBadges, f.SilverBadges, f.BronzeBadges,
//        CASE WHEN f.Reputation >= 1000 THEN 'Expert' WHEN f.Reputation >= 100 THEN 'Intermediate' ELSE 'Novice' END AS UserLevel
// FROM FinalUserStats f ORDER BY f.Reputation DESC LIMIT 10 OFFSET 5;
fn q33289(db: &'static So) -> String {
    let Post { parent_id, score, .. } = &db.post;
    let ph: HashIdx<Id<Post>, usize> = db.post.minus(parent_id).reach(children_of(db), usize::MAX).collect();
    let ur = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and((&ph).opt())).opt()).fold(0i64, |r, x| match x {
        Some((s, Some(0))) => r + 2 * s,
        Some((s, Some(1))) => r + s,
        _ => r,
    });
    let Badge { date, class, user, .. } = &db.badge;
    let ab = db.badge.with(date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(user).select(class).fold([0i64; 3], |a, c| {
        [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]
    });
    let v = drain(db.user.select(Ident::<User>::new().and(&ur).and((&ab).opt())));
    let v = top_n(v, |&(_, ((_, r), _))| Reverse(r), 15);
    rows(v.into_iter().skip(5).map(|(_, ((u, r), b))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(r), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::S(if r >= 1000 { "Expert" } else if r >= 100 { "Intermediate" } else { "Novice" })]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT Id, ParentId, Title, OwnerUserId, CreationDate, 0 AS Level FROM Posts WHERE ParentId IS NULL
//     UNION ALL SELECT p.Id, p.ParentId, p.Title, p.OwnerUserId, p.CreationDate, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.Id),
// UserScore AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Location, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.Location),
// PostActivity AS (SELECT P.Id AS PostId, P.Title, PH.Level, U.DisplayName AS OwnerDisplayName, COUNT(CM.Id) AS CommentCount, COUNT(V.Id) AS VoteCount,
//        COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
//     FROM Posts P LEFT JOIN Comments CM ON P.Id = CM.PostId LEFT JOIN Votes V ON P.Id = V.PostId JOIN Users U ON P.OwnerUserId = U.Id
//     LEFT JOIN PostHierarchy PH ON PH.Id = P.Id GROUP BY P.Id, P.Title, PH.Level, U.DisplayName),
// ActiveUserPosts AS (SELECT PH.OwnerUserId, COUNT(DISTINCT PA.PostId) AS ActivePosts, SUM(PA.CommentCount) AS TotalComments, SUM(PA.VoteCount) AS TotalVotes,
//        SUM(PA.TotalBounty) AS TotalBountyValue FROM PostActivity PA JOIN PostHierarchy PH ON PA.PostId = PH.Id GROUP BY PH.OwnerUserId),
// FinalResult AS (SELECT U.UserId, U.DisplayName, U.Reputation, U.BadgeCount, COALESCE(AP.ActivePosts, 0) AS ActivePostCount, COALESCE(AP.TotalComments, 0) AS TotalComments,
//        COALESCE(AP.TotalVotes, 0) AS TotalVotes, COALESCE(AP.TotalBountyValue, 0) AS TotalBountyValue FROM UserScore U LEFT JOIN ActiveUserPosts AP ON U.UserId = AP.OwnerUserId)
// SELECT FR.UserId, FR.DisplayName, FR.Reputation, FR.BadgeCount, FR.ActivePostCount, FR.TotalComments, FR.TotalVotes, FR.TotalBountyValue,
//        (FR.Reputation + FR.BadgeCount * 10 + FR.ActivePostCount * 5 + FR.TotalBountyValue) AS PerformanceScore
// FROM FinalResult FR ORDER BY PerformanceScore DESC LIMIT 10;
//
// BadgeCount is a COUNT(DISTINCT B.Id), which the votes product does not change, so it is each user's badge count.
fn q33673(db: &'static So) -> String {
    let Post { parent_id, owner_user, .. } = &db.post;
    let ph: HashIdx<Id<Post>, usize> = db.post.minus(parent_id).reach(children_of(db), usize::MAX).collect();
    type K = (Id<Post>, Option<usize>);
    let pa = db
        .post
        .with(owner_user)
        .select(Ident::<Post>::new().and((&ph).opt()))
        .group_by(Same::<K>::new())
        .select(Same::<K>::new().map(|k: K| k.0).select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())))
        .fold((0i64, 0i64, 0i64), |(c, n, b), (cm, v)| (c + cm.is_some() as i64, n + v.is_some() as i64, b + v.flatten().unwrap_or(0)));
    type P = (K, (i64, i64, i64));
    let par = rel(drain(&pa));
    let aup = (&par)
        .select(Same::<P>::new().and(Same::<P>::new().map(|x: P| x.0 .0).select(&ph)))
        .group_by(Same::<(P, usize)>::new().map(|x: (P, usize)| x.0 .0 .0).select(owner_user))
        .select(Same::<(P, usize)>::new().map(|x: (P, usize)| x.0))
        .buf_fold(|v| {
            let mut ps: Vec<Id<Post>> = v.iter().map(|x| x.0 .0).collect();
            ps.sort_unstable();
            ps.dedup();
            v.iter().fold((ps.len() as i64, 0i64, 0i64, 0i64), |(n, c, w, b), x| (n, c + x.1 .0, w + x.1 .1, b + x.1 .2))
        });
    let v = drain(db.user.select(Ident::<User>::new().and(&db.user.reputation).and(badges_per_user(db)).and((&aup).opt())));
    let score = |r: i64, b: i64, a: Option<(i64, i64, i64, i64)>| {
        let a = a.unwrap_or((0, 0, 0, 0));
        r + b * 10 + a.0 * 5 + a.3
    };
    let v = top_n(v, |&(_, (((_, r), b), a))| Reverse(score(r, b, a)), 10);
    rows(v.into_iter().map(|(_, (((u, r), b), a))| {
        let s = score(r, b, a);
        let a = a.unwrap_or((0, 0, 0, 0));
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(r), V::I(b), V::I(a.0), V::I(a.1), V::I(a.2), V::I(a.3), V::I(s)]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title AS PostTitle, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId, p.AcceptedAnswerId, p.OwnerUserId, 1 AS Depth
//     FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT a.Id AS PostId, a.Title AS PostTitle, a.CreationDate, a.ViewCount, a.Score, a.PostTypeId, a.AcceptedAnswerId, a.OwnerUserId, ph.Depth + 1
//     FROM Posts a INNER JOIN PostHierarchy ph ON a.ParentId = ph.PostId WHERE a.PostTypeId = 2),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven,
//        COUNT(v.Id) AS TotalVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT ph.PostId, ph.PostTitle, ph.Depth, ph.Score, u.DisplayName AS OwnerName, ua.QuestionsAsked, ua.AnswersGiven, ua.TotalVotes
//     FROM PostHierarchy ph LEFT JOIN Users u ON ph.OwnerUserId = u.Id LEFT JOIN UserActivity ua ON u.Id = ua.UserId)
// SELECT ps.PostId, ps.PostTitle, ps.Depth, ps.Score, ps.OwnerName, ps.QuestionsAsked, ps.AnswersGiven, ps.TotalVotes,
//        RANK() OVER (PARTITION BY ps.Depth ORDER BY ps.Score DESC) AS RankByScoreDepth,
//        CASE WHEN ps.Depth = 1 AND ps.Score >= 10 THEN 'Hot Question' WHEN ps.Depth = 2 AND ps.Score < 5 THEN 'Low Engagement' ELSE 'Moderate' END AS EngagementLevel
// FROM PostStatistics ps ORDER BY ps.Depth, ps.Score DESC;
fn q31832(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let ph = rel(drain(db.post.with(post_type_id.eq(1)).reach(answers_of(db), usize::MAX)));
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(votes_of(db).opt())).opt()).fold([0i64; 3], |a, x| match x {
        Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + v.is_some() as i64],
        None => a,
    });
    type X = (Id<Post>, usize);
    let w = (&ph)
        .group_by(Same::<X>::new().map(|x: X| x.1))
        .select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.0).select(score)))
        .window(rank, |(_, s)| Reverse(s), asc);
    let v = drain((&w).select(Same::<((X, i64), i64)>::new().and(Same::<((X, i64), i64)>::new().map(|x: ((X, i64), i64)| x.0 .0 .0).select(owner_user.select(Ident::<User>::new().and(&ua)).opt()))));
    rows(v.into_iter().map(|(_, ((((p, l), s), r), u))| {
        let d = l as i64 + 1;
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(d), V::I(s)]);
        match u {
            Some((u, a)) => {
                f.extend(ucols(db, u, &["name"]));
                f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        f.push(V::I(r));
        f.push(V::S(if d == 1 && s >= 10 { "Hot Question" } else if d == 2 && s < 5 { "Low Engagement" } else { "Moderate" }));
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT Id AS PostId, ParentId, Title, CreationDate, 0 AS Level FROM Posts WHERE ParentId IS NULL
//     UNION ALL SELECT p.Id, p.ParentId, p.Title, p.CreationDate, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId),
// UserStickyPosts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS StickyPostCount FROM Posts p WHERE p.ViewCount > 1000 GROUP BY p.OwnerUserId),
// RecentPosts AS (SELECT Id, Title, OwnerUserId, CreationDate, ROW_NUMBER() OVER (PARTITION BY OwnerUserId ORDER BY CreationDate DESC) AS rn FROM Posts
//     WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'),
// PostViewCount AS (SELECT p.Id, p.Title, COUNT(v.Id) AS VoteCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title)
// SELECT ph.PostId, ph.Title AS PostTitle, ph.CreationDate AS PostDate, u.DisplayName AS OwnerName, COALESCE(sp.StickyPostCount, 0) AS OwnerStickyPosts,
//        COALESCE(rv.VoteCount, 0) AS RecentPostVotes, ph.Level AS HierarchyLevel
// FROM PostHierarchy ph LEFT JOIN Users u ON ph.PostId = u.Id LEFT JOIN UserStickyPosts sp ON u.Id = sp.OwnerUserId LEFT JOIN PostViewCount rv ON ph.PostId = rv.Id
// WHERE (u.Reputation > 1000 OR sp.StickyPostCount > 0) AND ph.Level <= 2 ORDER BY ph.Level, ph.CreationDate DESC;
//
// The user is joined on the post's own Id, as written. RecentPosts is never read. Level <= 2 caps the recursion.
fn q30603(db: &'static So) -> String {
    let Post { parent_id, view_count, owner_user, origid, .. } = &db.post;
    let ph = db.post.minus(parent_id).reach(children_of(db), 2);
    let uby: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let sp = db.post.with(view_count.gt(1000)).group_by(owner_user).fold(0i64, |n, _| n + 1);
    let u = Ident::<User>::new().and(&db.user.reputation).and((&sp).opt()).filt(|((_, r), s): ((Id<User>, i64), Option<i64>)| r > 1000 || s.is_some_and(|s| s > 0));
    let v = drain((&ph).and(origid.select(&uby).select(u)).and(votes_per_post(db)));
    rows(v.into_iter().map(|(p, ((l, ((u, _), s)), w))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(s.unwrap_or(0)), V::I(w), V::I(l as i64)]);
        row(f)
    }))
}

// WITH RECURSIVE UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, 1 AS ActivityLevel FROM Users U WHERE U.Reputation > 1000
//     UNION ALL SELECT U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, UA.ActivityLevel + 1 FROM Users U INNER JOIN UserActivity UA ON U.Id = UA.UserId
//     WHERE UA.ActivityLevel < 3),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(P.Score) AS AverageScore, SUM(P.ViewCount) AS TotalViews, MAX(P.CreationDate) AS LastPostDate FROM Posts P GROUP BY P.OwnerUserId),
// PopularUsers AS (SELECT UA.UserId, UA.DisplayName, PSA.TotalPosts, PSA.Questions, PSA.Answers, PSA.AverageScore, PSA.TotalViews,
//        ROW_NUMBER() OVER (ORDER BY PSA.TotalPosts DESC, PSA.AverageScore DESC) AS Ranking
//     FROM UserActivity UA JOIN PostStatistics PSA ON UA.UserId = PSA.OwnerUserId WHERE UA.ActivityLevel >= 1)
// SELECT PU.DisplayName, PU.TotalPosts, PU.Questions, PU.Answers, PU.AverageScore, PU.TotalViews
// FROM PopularUsers PU LEFT JOIN Badges B ON PU.UserId = B.UserId LEFT JOIN Votes V ON PU.UserId = V.UserId
// WHERE (B.Class = 1 OR B.Class = 2) AND (V.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' OR V.Id IS NULL)
// ORDER BY PU.Ranking LIMIT 10;
//
// The recursion repeats each user at levels 1-3 (max 2 steps). Ranking ties are between copies of one user, whose rows are identical.
fn q34888(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ua = db.user.with((&db.user.reputation).gt(1000)).reach(Same::<Id<User>>::new(), 2);
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0), a[5] + w.is_some() as i64]
    });
    let pu = drain((&ua).and(&ps));
    let pu = top_n(pu, |&(u, (_, a))| (Reverse(a[0]), Reverse(fkey(a[3] as f64 / a[0] as f64)), u), usize::MAX);
    type P = (usize, (Id<User>, [i64; 6]));
    let pu = rel(pu.into_iter().enumerate().map(|(i, (u, (_, a)))| (i, (u, a))).collect::<Vec<P>>());
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let x = badges_of(db)
        .select((&db.badge.class).filt(|c| c == 1 || c == 2))
        .and(votes_by(db).select(&db.vote.creation_date).opt().filt(move |d: Option<i64>| d.is_none_or(|d| d > cut)));
    let v = drain((&pu).select(Same::<P>::new().and(Same::<P>::new().map(|p: P| p.1 .0).select(x))));
    let v = top_n(v, |&(_, ((i, _), _))| i, 10);
    rows(v.into_iter().map(|(_, ((_, (u, a)), _))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), nullable(a[4], a[5])]);
        row(f)
    }))
}

// WITH RECURSIVE PopularPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 10
//     UNION ALL SELECT pp.Id, pp.Title, pp.CreationDate, pp.Score + p.Score AS AccumulatedScore, pp.ViewCount + p.ViewCount AS AccumulatedViews, pp.OwnerUserId, Level + 1
//     FROM Posts pp INNER JOIN PopularPosts p ON pp.ParentId = p.Id WHERE pp.PostTypeId = 2),
// PostStats AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id LEFT JOIN Badges b ON b.UserId = p.OwnerUserId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY p.Id, p.Title),
// TopPostStats AS (SELECT ps.PostId, ps.Title, ps.CommentCount, ps.UpVotes - ps.DownVotes AS NetVotes, ROW_NUMBER() OVER (ORDER BY ps.CommentCount DESC, ps.UpVotes DESC) AS Rank
//     FROM PostStats ps)
// SELECT pp.Id, pp.Title, pp.CreationDate, pp.Score, pp.ViewCount, tps.CommentCount, tps.NetVotes, CASE WHEN tps.Rank <= 10 THEN 'Top Post' ELSE 'Regular Post' END AS PostClassification,
//        u.DisplayName AS OwnerName, (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = pp.OwnerUserId) AS TotalPostsByOwner
// FROM PopularPosts pp JOIN TopPostStats tps ON pp.Id = tps.PostId JOIN Users u ON pp.OwnerUserId = u.Id ORDER BY pp.ViewCount DESC, pp.Score DESC LIMIT 100;
//
// Each node carries its accumulated Score and ViewCount.
fn q31948(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, owner_user, .. } = &db.post;
    type N = (Id<Post>, i64, Option<i64>);
    let step = Same::<N>::new()
        .and(Same::<N>::new().map(|n: N| n.0).select(answers_of(db)).select(Ident::<Post>::new().and(score).and(view_count.opt())))
        .map(|((_, s, w), ((c, cs), cw)): (N, ((Id<Post>, i64), Option<i64>))| (c, cs + s, cw.zip(w).map(|(a, b)| a + b)));
    let base = db.post.with(post_type_id.eq(1)).with(score.gt(10)).select(Ident::<Post>::new().and(score).and(view_count.opt())).map(|((p, s), w)| (p, s, w));
    let pp = base.reach(step, usize::MAX);
    let ps = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 3], |a, ((c, v), _)| [a[0] + c.is_some() as i64, a[1] + (v == Some(2)) as i64, a[2] + (v == Some(3)) as i64]);
    let tps = top_n(drain(&ps), |&(p, a)| (Reverse(a[0]), Reverse(a[1]), p), usize::MAX);
    type T = (Id<Post>, ([i64; 3], usize));
    let tps = rel(tps.into_iter().enumerate().map(|(i, (p, a))| (p, (a, i + 1))).collect::<Vec<T>>());
    let tpi: HashIdx<Id<Post>, ([i64; 3], usize)> = (&tps).map(|t: T| t.0).inv().select(&tps).map(|t: T| t.1).collect();
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&pp).and(Same::<N>::new().map(|n: N| n.0).select((&tpi).and(owner_user.select(Ident::<User>::new().and(&pc))))));
    let v = top_n(v, |&((_, s, w), _)| (w.is_none(), Reverse(w), Reverse(s)), 100);
    rows(v.into_iter().map(|((p, s, w), (_, ((a, r), (u, n))))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s), oint(w), V::I(a[0]), V::I(a[1] - a[2]), V::S(if r <= 10 { "Top Post" } else { "Regular Post" })]);
        f.extend(ucols(db, u, &["name"]));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id, p.Title, p.CreationDate, p.ParentId FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.Title, p.CreationDate, p.ParentId FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.Id),
// UserContribution AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName),
// RecentActivity AS (SELECT u.Id AS UserId, u.DisplayName, MAX(COALESCE(p.LastActivityDate, p.CreationDate)) AS LastActivity FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT uc.UserId, uc.DisplayName, uc.PostCount, uc.AnswerCount, uc.QuestionCount, ua.LastActivity, ROW_NUMBER() OVER (ORDER BY uc.PostCount DESC, uc.Upvotes DESC) AS Rank
//     FROM UserContribution uc JOIN RecentActivity ua ON uc.UserId = ua.UserId WHERE uc.PostCount > 0)
// SELECT tu.DisplayName, tu.PostCount, tu.AnswerCount, tu.QuestionCount, tu.LastActivity, (TIMESTAMP '2024-10-01 12:34:56' - tu.LastActivity) AS DaysSinceLastActivity,
//        COALESCE(ph.Id, 0) AS HasChildPost, CASE WHEN tu.LastActivity < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' THEN 'Inactive' ELSE 'Active' END AS ActivityStatus
// FROM TopUsers tu LEFT JOIN PostHierarchy ph ON tu.UserId = ph.Id WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// The hierarchy is joined on the user's Id against the post's Id, as written. COUNT(DISTINCT p.Id) is the number of posts, which the votes product does not change.
fn q32928(db: &'static So) -> String {
    let Post { post_type_id, origid, last_activity_date, .. } = &db.post;
    let phi: HashIdx<i64, usize> = db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX).inv().select(origid).inv().collect();
    let uc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 3], |a, x| match x {
        Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64],
        None => a,
    });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let la = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(last_activity_date)).fold(i64::MIN, |m, d| m.max(d));
    let tu = drain(db.user.select(Ident::<User>::new().and((&pc).filt(|n| n > 0)).and(&uc).and(&la)));
    let tu = top_n(tu, |&(u, (((_, n), a), _))| (Reverse(n), Reverse(a[2]), u), 10);
    type T = (Id<User>, ((i64, [i64; 3]), i64));
    let tu = rel(tu.into_iter().map(|(_, (((u, n), a), d))| (u, ((n, a), d))).collect::<Vec<T>>());
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|t: T| t.0).select((&db.user.origid).select(&phi).opt()))));
    let now = ts(2024, 10, 1, 12, 34, 56);
    rows(v.into_iter().map(|(_, ((u, ((n, a), d)), h))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::T(d), V::Iv(now - d)]);
        f.push(if h.is_some() { ucols(db, u, &["uid"]).remove(0) } else { V::I(0) });
        f.push(V::S(if d < add_days(now, -30) { "Inactive" } else { "Active" }));
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT U.Id AS UserId, U.Reputation, U.CreationDate, 0 AS Level FROM Users U WHERE U.Reputation > 1000
//     UNION ALL SELECT U.Id, U.Reputation, U.CreationDate, UR.Level + 1 FROM Users U INNER JOIN UserReputation UR ON U.Id = UR.UserId WHERE U.Reputation > 1000 + UR.Level * 500),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, AVG(V.BountyAmount) AS AvgBounty FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.OwnerUserId),
// RecentActivity AS (SELECT C.UserId, COUNT(C.Id) AS TotalComments, MAX(C.CreationDate) AS LastCommentDate FROM Comments C GROUP BY C.UserId),
// CombinedStatistics AS (SELECT U.DisplayName AS UserName, COALESCE(UR.Reputation, 0) AS UserReputation, COALESCE(PS.TotalPosts, 0) AS TotalPosts,
//        COALESCE(PS.TotalAnswers, 0) AS TotalAnswers, COALESCE(PS.TotalQuestions, 0) AS TotalQuestions, COALESCE(RA.TotalComments, 0) AS TotalComments,
//        COALESCE(RA.LastCommentDate, '1900-01-01') AS LastCommentDate,
//        CASE WHEN RA.LastCommentDate IS NULL THEN 'No Comments Yet' WHEN RA.LastCommentDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN 'Inactive' ELSE 'Active' END AS ActivityStatus
//     FROM Users U LEFT JOIN UserReputation UR ON U.Id = UR.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId LEFT JOIN RecentActivity RA ON U.Id = RA.UserId),
// RankedUsers AS (SELECT UserName, UserReputation, TotalPosts, TotalAnswers, TotalQuestions, TotalComments, LastCommentDate, ActivityStatus,
//        RANK() OVER (ORDER BY UserReputation DESC) AS ReputationRank FROM CombinedStatistics)
// SELECT UserName, UserReputation, TotalPosts, TotalAnswers, TotalQuestions, TotalComments, LastCommentDate, ActivityStatus, ReputationRank
// FROM RankedUsers WHERE UserReputation > 1500 AND ActivityStatus = 'Active' ORDER BY ReputationRank;
//
// The step's condition reads the level, so each node carries it. The rank is taken over every combined row, before the WHERE.
fn q30608(db: &'static So) -> String {
    let reputation = &db.user.reputation;
    type N = (Id<User>, i64);
    let step = Same::<N>::new().and(Same::<N>::new().map(|n: N| n.0).select(reputation)).filt(|((_, l), r): (N, i64)| r > 1000 + l * 500).map(|((u, l), _): (N, i64)| (u, l + 1));
    let ur: HashIdx<Id<User>, usize> = db.user.with(reputation.gt(1000)).map(|u| (u, 0i64)).reach(step, usize::MAX).inv().map(|n: N| n.0).inv().collect();
    let ps = db.post.group_by(&db.post.owner_user).select((&db.post.post_type_id).and(votes_of(db).opt())).fold([0i64; 3], |a, (t, _)| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64]);
    let ra = db.comment.group_by(&db.comment.user).select(&db.comment.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    type R0 = (Id<User>, Option<usize>);
    let cs = rel(drain(db.user.select(Ident::<User>::new().and((&ur).opt()))).into_iter().map(|x| x.1).collect::<Vec<R0>>());
    let w = whole(&cs)
        .select(&cs)
        .select(Same::<R0>::new().and(Same::<R0>::new().map(|x: R0| x.0).select(reputation)).map(|(x, r): (R0, i64)| (x.0, if x.1.is_some() { r } else { 0 })))
        .window(rank, |(_, r)| Reverse(r), asc);
    type W = ((Id<User>, i64), i64);
    let cut = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let v = drain(
        (&w).select(
            Same::<W>::new()
                .filt(|((_, r), _): W| r > 1500)
                .and(Same::<W>::new().map(|x: W| x.0 .0).select((&ps).opt().and((&ra).filt(move |(_, d): (i64, i64)| d >= cut)))),
        ),
    );
    rows(v.into_iter().map(|(_, (((u, r), k), (p, (n, d))))| {
        let p = p.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(r), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(n), V::T(d), V::S("Active"), V::I(k)]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS ( SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.ParentId, p.CreationDate, 0 AS Level FROM Posts p WHERE p.PostTypeId = 1
//        UNION ALL SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.ParentId, p.CreationDate, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId )
//        SELECT ph.PostId, ph.Title, ph.Level, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, MAX(b.Date) AS LatestBadgeDate FROM PostHierarchy ph LEFT JOIN Comments c ON ph.PostId = c.PostId
//        LEFT JOIN Votes v ON ph.PostId = v.PostId LEFT JOIN Badges b ON b.UserId = ph.PostId WHERE ph.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
//        GROUP BY ph.PostId, ph.Title, ph.PostTypeId, ph.ParentId, ph.CreationDate, ph.Level HAVING COUNT(DISTINCT c.Id) > 0 OR COUNT(DISTINCT v.Id) > 0
//        ORDER BY ph.Level, CommentCount DESC, ph.Title OFFSET 10 ROWS FETCH NEXT 20 ROWS ONLY;
fn q33648(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, .. } = &db.post;
    type X = (Id<Post>, usize);
    let ph = db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX);
    let phv = rel(drain(&ph));
    let cut = ts(2024, 10, 1, 12, 34, 56) - 30 * DAY_US;
    let bu: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let at = || Same::<X>::new().map(|x: X| x.0);
    let g = (&phv)
        .with(at().select(creation_date.ge(cut)))
        .group_by(Same::<X>::new())
        .select(at().select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(origid.select(&bu).select(&db.badge.date).opt())))
        .fold((0i64, 0i64, None::<i64>), |(u, d, m), ((_, t), b)| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64, m.max(b)));
    let h = (&g).and(at().select(comments_per_post(db).and(votes_per_post(db)))).filt(|(_, (c, v)): ((i64, i64, Option<i64>), (i64, i64))| c > 0 || v > 0);
    let v = top_n(drain(&h), |&((p, l), (_, (c, _)))| (l, Reverse(c), db.post.title.get(p).is_none(), db.post.title.get(p)), 0);
    rows(v.into_iter().skip(10).take(20).map(|((p, l), ((u, d, m), (c, n)))| {
        row(vec![post_fields(db, p, &["id"]).remove(0), title(db, p), V::I(l as i64), V::I(c), V::I(n), V::I(u), V::I(d), ots(m)])
    }))
}

// WITH RECURSIVE UserReputation AS ( SELECT u.Id, u.Reputation, u.DisplayName, 1 AS Level FROM Users u WHERE u.Reputation > 500 UNION ALL SELECT u.Id, u.Reputation,
//        u.DisplayName, ur.Level + 1 FROM Users u INNER JOIN UserReputation ur ON ur.Id = u.Id WHERE u.Reputation > 500 * ur.Level ), PostStats AS ( SELECT p.Id AS PostId,
//        MAX(p.Score) AS MaxScore, AVG(p.Score) AS AvgScore, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UniqueVoteCount FROM Posts p LEFT JOIN Comments c ON
//        p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id ), PostHistoryData AS ( SELECT ph.PostId,
//        COUNT(ph.Id) AS EditCount, MIN(ph.CreationDate) AS FirstEditDate, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph GROUP BY ph.PostId ) SELECT ur.DisplayName,
//        p.Title, ps.MaxScore, ps.AvgScore, phd.EditCount, phd.FirstEditDate, phd.LastEditDate FROM UserReputation ur JOIN Posts p ON ur.Id = p.OwnerUserId JOIN PostStats ps ON
//        p.Id = ps.PostId LEFT JOIN PostHistoryData phd ON p.Id = phd.PostId WHERE ps.AvgScore IS NOT NULL AND phd.FirstEditDate IS NOT NULL ORDER BY ps.AvgScore DESC,
//        phd.EditCount DESC LIMIT 10;
//
// The node carries the level, since the step's condition reads it. MaxScore and AvgScore are a post's own score: PostStats groups by p.Id.
fn q30244(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    type X = (Id<User>, i64);
    let step = Same::<X>::new()
        .and(Same::<X>::new().map(|x: X| x.0).select(&db.user.reputation))
        .filt(|((_, l), r): (X, i64)| r > 500 * l)
        .map(|((u, l), _): (X, i64)| (u, l + 1));
    let ur = db.user.with((&db.user.reputation).gt(500)).map(|u: Id<User>| (u, 1i64)).reach(step, usize::MAX);
    let urv = rel(drain(&ur));
    let ph = &db.post_history;
    let phd = ph.group_by(&ph.post).select(&ph.creation_date).fold((0i64, i64::MAX, i64::MIN), |(n, a, b), d| (n + 1, a.min(d), b.max(d)));
    let v = drain((&urv).map(|((u, _), _): (X, usize)| u).select(Ident::<User>::new().and(posts_of(db).with(creation_date.ge(date(2023, 1, 1))).select(Ident::<Post>::new().and(&phd)))));
    let v = top_n(v, |&(_, (_, (p, (n, _, _))))| (Reverse(score.get(p).unwrap()), Reverse(n)), 10);
    rows(v.into_iter().map(|(_, (u, (p, (n, a, b))))| {
        let s = score.get(p).unwrap();
        row(vec![V::S(db.user.display_name.get(u).unwrap()), title(db, p), V::I(s), V::F(s as f64), V::I(n), V::T(a), V::T(b)])
    }))
}

// WITH RECURSIVE PostHierarchy AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AcceptedAnswerId, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//        UNION ALL SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AcceptedAnswerId, ph.Level + 1 FROM Posts p JOIN PostHierarchy ph ON p.ParentId = ph.PostId )
//        SELECT ph.PostId, ph.Title, ph.CreationDate, ph.ViewCount, ph.Score, ph.Level, COALESCE(AnswerCounts.AnswerCount, 0) AS TotalAnswers, COALESCE(TopVoter.DisplayName,
//        'No Votes') AS TopVoter, COALESCE(VotingStats.VoteCount, 0) AS TotalVotes, CASE WHEN ph.Score > 0 THEN 'Popular' WHEN ph.Score < 0 THEN 'Unpopular' ELSE 'Neutral' END
//        AS PopularityLabel FROM PostHierarchy ph LEFT JOIN (SELECT p.Id, COUNT(c.Id) AS AnswerCount FROM Posts p LEFT JOIN Posts c ON c.ParentId = p.Id WHERE p.PostTypeId = 1
//        GROUP BY p.Id ) AS AnswerCounts ON AnswerCounts.Id = ph.PostId LEFT JOIN (SELECT v.PostId, COUNT(v.Id) AS VoteCount, u.DisplayName FROM Votes v LEFT JOIN Users u ON
//        v.UserId = u.Id GROUP BY v.PostId, u.DisplayName ) AS VotingStats ON VotingStats.PostId = ph.PostId LEFT JOIN (SELECT v.PostId, u.DisplayName FROM Votes v JOIN Users u
//        ON v.UserId = u.Id GROUP BY v.PostId, u.DisplayName HAVING COUNT(v.Id) = ( SELECT MAX(VoteCount) FROM (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) t
//        ) ) AS TopVoter ON TopVoter.PostId = ph.PostId WHERE ph.Level <= 2 ORDER BY ph.CreationDate DESC LIMIT 100;
fn q31240(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, score, .. } = &db.post;
    let Vote { post, post_id, user, .. } = &db.vote;
    type X = (Id<Post>, usize);
    let ph = db.post.with(post_type_id.eq(1)).reach(children_of(db), 1);
    let phv = rel(drain(&ph));
    let ac = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vs = drain(&db.vote.group_by(post.and(user.select(&db.user.display_name).opt())).fold(0i64, |n, _| n + 1));
    let vs = rel(vs);
    let vsi: HashIdx<Id<Post>, ((Id<Post>, Option<Str>), i64)> = (&vs).map(|((p, _), _)| p).inv().select(&vs).collect();
    let mx = (&db.vote.group_by(post_id).fold(0i64, |n, _| n + 1)).fold_flat(0i64, |m, n| m.max(n));
    let tv = rel(drain((&db.vote.group_by(post_id.and(user.select(&db.user.display_name))).fold(0i64, |n, _| n + 1)).filt(|n: i64| n == mx)));
    let tvi: HashIdx<i64, ((i64, Str), i64)> = (&tv).map(|((p, _), _)| p).inv().select(&tv).collect();
    let at = Same::<X>::new().map(|x: X| x.0);
    let v = drain((&phv).select(Same::<X>::new().and(at.select((&ac).opt().and((&vsi).opt()).and(origid.select(&tvi).opt())))));
    let v = top_n(v, |&(_, ((p, _), _))| Reverse(creation_date.get(p).unwrap()), 100);
    rows(v.into_iter().map(|(_, ((p, l), ((a, s), t)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        let sc = score.get(p).unwrap();
        f.extend([
            V::I(l as i64 + 1),
            V::I(a.unwrap_or(0)),
            match t {
                Some(((_, n), _)) => V::S(n),
                None => V::S("No Votes"),
            },
            V::I(s.map_or(0, |x| x.1)),
            V::S(if sc > 0 { "Popular" } else if sc < 0 { "Unpopular" } else { "Neutral" }),
        ]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputationCTE AS ( SELECT U.Id AS UserId, U.Reputation, 1 AS Level FROM Users U WHERE U.Reputation IS NOT NULL UNION ALL SELECT U.Id,
//        U.Reputation + 50, UR.Level + 1 FROM Users U JOIN UserReputationCTE UR ON U.Id = UR.UserId WHERE UR.Level < 5 ), PostStatistics AS ( SELECT P.Id AS PostId,
//        P.OwnerUserId, P.PostTypeId, COUNT(C.Id) AS CommentCount, COALESCE(SUM(CASE WHEN V.Id = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN V.Id = 3 THEN 1
//        ELSE 0 END), 0) AS Downvotes, P.CreationDate, P.Title FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN
//        (2, 3) GROUP BY P.Id, P.OwnerUserId, P.PostTypeId, P.CreationDate, P.Title ), BadgeSummary AS ( SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, MAX(B.Class) AS
//        HighestClass FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id ) SELECT U.DisplayName, UR.Reputation, PS.PostId, PS.Title, PS.CommentCount, PS.Upvotes,
//        PS.Downvotes, BS.BadgeCount, CASE WHEN PS.CommentCount > 10 THEN 'High Engagement' ELSE 'Low Engagement' END AS EngagementLevel, CASE WHEN PS.Upvotes > PS.Downvotes
//        THEN 'Positive' ELSE CASE WHEN PS.Upvotes < PS.Downvotes THEN 'Negative' ELSE 'Neutral' END END AS VoteSentiment, COALESCE(pg.Name, 'N/A') AS PostType, (SELECT
//        AVG(Score) FROM Posts WHERE PostTypeId = PS.PostTypeId) AS AvgPostScore FROM Users U JOIN UserReputationCTE UR ON U.Id = UR.UserId JOIN PostStatistics PS ON U.Id =
//        PS.OwnerUserId LEFT JOIN PostTypes pg ON PS.PostTypeId = pg.Id LEFT JOIN BadgeSummary BS ON U.Id = BS.UserId WHERE U.Reputation > 1000 ORDER BY UR.Reputation DESC,
//        PS.PostId DESC LIMIT 50;
//
// `V.Id = 2` is the vote's own id, as written.
fn q31932(db: &'static So) -> String {
    let Post { post_type_id, origid, score, .. } = &db.post;
    let User { reputation, .. } = &db.user;
    type X = (Id<User>, i64);
    let at = || Same::<X>::new().map(|x: X| x.0);
    let urep = Ident::<User>::new().and(reputation);
    let ur = db.user.select(&urep).reach(at().select(&urep).map(|(u, r): X| (u, r + 50)), 4);
    let urv = rel(drain(&ur));
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).with((&db.vote.vote_type_id).filt(|t| t == 2 || t == 3)).select(&db.vote.origid).opt()))
        .fold((0i64, 0i64, 0i64), |(c, u, d), (k, v)| (c + k.is_some() as i64, u + (v == Some(2)) as i64, d + (v == Some(3)) as i64));
    let ta = db.post.group_by(post_type_id).select(score).fold((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let v = drain(
        (&urv)
            .map(|(x, _): (X, usize)| x)
            .with(at().select(reputation.gt(1000)))
            .select(Same::<X>::new().and(at().select(posts_of(db)).select(Ident::<Post>::new().and(&ps)))),
    );
    let v = top_n(v, |&(_, ((_, r), (p, _)))| (Reverse(r), Reverse(origid.get(p).unwrap())), 50);
    let bc = badges_per_user(db);
    rows(v.into_iter().map(|(_, ((u, r), (p, (c, up, dn))))| {
        let mut f = vec![V::S(db.user.display_name.get(u).unwrap()), V::I(r)];
        f.extend(post_fields(db, p, &["id", "title"]));
        let (s, n) = (&ta).get(post_type_id.get(p).unwrap()).unwrap();
        f.extend([
            V::I(c),
            V::I(up),
            V::I(dn),
            V::I((&bc).get(u).unwrap()),
            V::S(if c > 10 { "High Engagement" } else { "Low Engagement" }),
            V::S(if up > dn { "Positive" } else if up < dn { "Negative" } else { "Neutral" }),
            V::S(ptype_name(db).get(p).unwrap_or("N/A")),
            avg(s, n),
        ]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS ( SELECT p.Id AS PostId, p.ParentId, p.Title, 1 AS Level FROM Posts p WHERE p.ParentId IS NULL UNION ALL SELECT p.Id AS PostId,
//        p.ParentId, p.Title, ph.Level + 1 AS Level FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId ) , UserReputation AS ( SELECT u.Id AS UserId,
//        u.DisplayName, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, SUM(COALESCE(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS UpVotes, SUM(COALESCE(CASE WHEN
//        v.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName ) SELECT ph.PostId, ph.Title,
//        ph.Level, u.DisplayName AS Owner, ur.TotalBounty, ur.UpVotes, ur.DownVotes, CASE WHEN ur.UpVotes IS NOT NULL AND ur.DownVotes IS NOT NULL THEN (ur.UpVotes -
//        ur.DownVotes) ELSE NULL END AS NetVotes, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT p2.Id) AS AnswerCount, MAX(uh.CreationDate) AS LastActivityDate FROM
//        PostHierarchy ph LEFT JOIN Posts p2 ON ph.PostId = p2.ParentId LEFT JOIN Users u ON ph.PostId = u.Id LEFT JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN
//        Comments c ON c.PostId = ph.PostId LEFT JOIN (SELECT PostId, CreationDate FROM PostHistory WHERE PostHistoryTypeId IN (10, 11)) uh ON uh.PostId = ph.PostId GROUP BY
//        ph.PostId, ph.Title, ph.Level, u.DisplayName, ur.TotalBounty, ur.UpVotes, ur.DownVotes HAVING COUNT(DISTINCT c.Id) > 0 OR COUNT(DISTINCT p2.Id) > 0 ORDER BY ph.Level,
//        NetVotes DESC;
//
// Every aggregate is DISTINCT or MAX, so each (post, level) the recursion reaches is one group however often it is reached. `u.Id = ph.PostId` is a raw-id join.
fn q34204(db: &'static So) -> String {
    let Post { parent_id, origid, .. } = &db.post;
    type X = (Id<Post>, usize);
    let ph = db.post.minus(parent_id).reach(children_of(db), usize::MAX);
    let phv = rel(drain(&ph));
    let g: MatSet<X> = (&phv).collect();
    let ui: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt().and(&db.vote.vote_type_id)).opt())
        .fold((0i64, 0i64, 0i64), |(b, u, d), v| match v {
            Some((x, t)) => (b + x.unwrap_or(0), u + (t == 2) as i64, d + (t == 3) as i64),
            None => (b, u, d),
        });
    let hs = &db.post_history;
    let uh = hs.with((&hs.post_history_type_id).filt(|t| t == 10 || t == 11)).group_by(&hs.post).select(&hs.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let h = (&g)
        .select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.0).select(
            comments_per_post(db).and(answers_per_post(db)).and(origid.select(&ui).select(Ident::<User>::new().and(&ur)).opt()).and((&uh).opt()),
        )))
        .filt(|(_, (((c, a), _), _)): (X, (((i64, i64), Option<(Id<User>, (i64, i64, i64))>), Option<i64>))| c > 0 || a > 0);
    rows(drain(&h).into_iter().map(|(_, ((p, l), (((c, a), u), m)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::I(l as i64 + 1));
        match u {
            Some((u, (b, up, dn))) => f.extend([V::S(db.user.display_name.get(u).unwrap()), V::I(b), V::I(up), V::I(dn), V::I(up - dn)]),
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        f.extend([V::I(c), V::I(a), ots(m)]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS ( SELECT U.Id, U.DisplayName, U.Reputation, U.CreationDate, 1 AS Depth FROM Users U WHERE U.Reputation IS NOT NULL UNION ALL SELECT
//        U.Id, U.DisplayName, U.Reputation + (COALESCE(SUM(V.BountyAmount), 0) / 10) AS Reputation, U.CreationDate, UR.Depth + 1 FROM Users U LEFT JOIN Votes V ON U.Id =
//        V.UserId JOIN UserReputation UR ON U.Id = UR.Id WHERE UR.Depth < 3 GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, UR.Depth ), PostStatistics AS ( SELECT
//        P.Id AS PostId, P.Title, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT A.Id) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, AVG(EXTRACT(EPOCH FROM (P.LastActivityDate - P.CreationDate))) AS AvgOpenDuration FROM Posts P
//        LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Posts A ON P.Id = A.ParentId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title ),
//        FinalStatistics AS ( SELECT PS.PostId, PS.Title, PS.CommentCount, PS.AnswerCount, PS.UpVoteCount - PS.DownVoteCount AS NetVoteCount, U.DisplayName, UR.Reputation AS
//        UserReputation FROM PostStatistics PS JOIN Users U ON PS.PostId = U.Id LEFT JOIN UserReputation UR ON U.Id = UR.Id ) SELECT F.Title, F.CommentCount, F.AnswerCount,
//        F.NetVoteCount, F.UserReputation FROM FinalStatistics F WHERE F.UserReputation IS NOT NULL ORDER BY F.NetVoteCount DESC, F.AnswerCount DESC FETCH FIRST 10 ROWS ONLY;
//
// The CTE's Reputation column is the base case's INTEGER, so DuckDB casts the recursive member's DOUBLE back, rounding half to even. `PS.PostId = U.Id` is a raw-id join.
fn q33357(db: &'static So) -> String {
    let Post { post_type_id, origid, .. } = &db.post;
    let User { reputation, .. } = &db.user;
    type X = (Id<User>, i64);
    let bs = (&db.vote.user).inv().select(&db.vote.bounty_amount).dense_fold_outer(db.user.id.n, 0i64, |a, b| a + b);
    let step = Same::<X>::new()
        .map(|x: X| x.0)
        .select(Ident::<User>::new().and(reputation).and(&bs))
        .map(|((u, r), b): ((Id<User>, i64), i64)| (u, (r as f64 + b as f64 / 10.0).round_ties_even() as i64));
    let ur = db.user.select(Ident::<User>::new().and(reputation)).reach(step, 2);
    let urv = rel(drain(&ur));
    let uri: HashIdx<Id<User>, (X, usize)> = (&urv).map(|((u, _), _): (X, usize)| u).inv().select(&urv).collect();
    let ui: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ps = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold(0i64, |n, (_, t)| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let v = drain((&ps).and(origid.select(&ui).select(&uri)));
    let ac = answers_per_post(db);
    let cc = comments_per_post(db);
    let v = top_n(v, |&(p, (n, _))| (Reverse(n), Reverse((&ac).get(p).unwrap())), 10);
    rows(v.into_iter().map(|(p, (n, ((_, r), _)))| row(vec![title(db, p), V::I((&cc).get(p).unwrap()), V::I((&ac).get(p).unwrap()), V::I(n), V::I(r)])))
}

// WITH RECURSIVE PostHierarchy AS ( SELECT p.Id, p.ParentId, p.Title, p.CreationDate, 1 AS Level FROM Posts p WHERE p.ParentId IS NULL UNION ALL SELECT p.Id, p.ParentId,
//        p.Title, p.CreationDate, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.Id ), PostVoteInfo AS ( SELECT p.Id AS PostId, COUNT(v.Id) FILTER
//        (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id ),
//        RecentClosedPosts AS ( SELECT p.Id, p.Title, ph.Level, ph.CreationDate, ph.ParentId, COALESCE(pvi.UpVotes, 0) AS UpVotes, COALESCE(pvi.DownVotes, 0) AS DownVotes FROM
//        Posts p INNER JOIN PostHierarchy ph ON p.Id = ph.Id LEFT JOIN PostVoteInfo pvi ON p.Id = pvi.PostId WHERE p.ClosedDate IS NOT NULL AND p.CreationDate >=
//        cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' ), BadgesPerUser AS ( SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS
//        BadgeNames FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id ) SELECT rcp.Title AS ClosedPostTitle, rcp.CreationDate AS ClosedDate, rcp.UpVotes,
//        rcp.DownVotes, bp.BadgeCount, bp.BadgeNames, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rcp.Id) AS CommentCount FROM RecentClosedPosts rcp LEFT JOIN
//        BadgesPerUser bp ON rcp.ParentId = bp.UserId ORDER BY rcp.CreationDate DESC LIMIT 10;
//
// `rcp.ParentId = bp.UserId` is a raw-id join.
fn q34077(db: &'static So) -> String {
    let Post { parent_id, closed_date, creation_date, .. } = &db.post;
    type X = (Id<Post>, usize);
    let ph = db.post.minus(parent_id).reach(children_of(db), usize::MAX);
    let phv = rel(drain(&ph));
    let pvi = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let ui: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let bp = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name).opt()).buf_fold(|v| {
        let n: Vec<&str> = v.iter().filter_map(|x| *x).collect();
        (n.len() as i64, if n.is_empty() { None } else { Some(leak(n.join(", "))) })
    });
    let rcp = (&phv)
        .map(|x: X| x.0)
        .with(closed_date)
        .with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)))
        .select(Ident::<Post>::new().and(&pvi).and(parent_id.select(&ui).select(&bp).opt()));
    let v = top_n(drain(rcp), |&(_, ((p, _), _))| Reverse(creation_date.get(p).unwrap()), 10);
    let cc = comments_per_post(db);
    rows(v.into_iter().map(|(_, ((p, (u, d)), b))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::I(u), V::I(d)]);
        match b {
            Some((n, s)) => f.extend([V::I(n), ostr(s)]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(V::I((&cc).get(p).unwrap()));
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS ( SELECT p.Id AS PostId, p.Title, p.ParentId, 1 AS Level FROM Posts p WHERE p.ParentId IS NULL UNION ALL SELECT p.Id AS PostId, p.Title,
//        p.ParentId, ph.Level + 1 AS Level FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId ), UserVoteCounts AS ( SELECT v.UserId, COUNT(CASE WHEN vt.Name =
//        'UpMod' THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVoteCount FROM Votes v INNER JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP
//        BY v.UserId ), PostStatistics AS ( SELECT p.Id AS PostId, p.Title, COALESCE(ph.Level, 0) AS PostLevel, p.Score, p.ViewCount, COALESCE(uv.UpVoteCount, 0) AS
//        UpVoteCount, COALESCE(uv.DownVoteCount, 0) AS DownVoteCount, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank FROM Posts p LEFT JOIN PostHierarchy
//        ph ON p.Id = ph.PostId LEFT JOIN UserVoteCounts uv ON p.OwnerUserId = uv.UserId ) SELECT ps.PostId, ps.Title, ps.PostLevel, ps.Score, ps.ViewCount, ps.UpVoteCount,
//        ps.DownVoteCount, CASE WHEN ps.UpVoteCount - ps.DownVoteCount > 0 THEN 'Positive' WHEN ps.UpVoteCount - ps.DownVoteCount < 0 THEN 'Negative' ELSE 'Neutral' END AS
//        VoteSentiment FROM PostStatistics ps WHERE ps.PostLevel > 0 AND ps.Score > 10 ORDER BY ps.Rank FETCH FIRST 10 ROWS ONLY;
//
// `PostLevel > 0` keeps only the rows the recursion reached, and the Rank order is Score, ViewCount (NULLs last), both descending. `p.OwnerUserId = uv.UserId` is raw.
fn q34515(db: &'static So) -> String {
    let Post { parent_id, score, view_count, owner_user_id, .. } = &db.post;
    type X = (Id<Post>, usize);
    let ph = db.post.minus(parent_id).reach(children_of(db), usize::MAX);
    let phv = rel(drain(&ph));
    let uv = db
        .vote
        .group_by(&db.vote.user_id)
        .select(vtype_name(db))
        .fold((0i64, 0i64), |(u, d), n| (u + (n == "UpMod") as i64, d + (n == "DownMod") as i64));
    let v = drain(
        (&phv)
            .with(Same::<X>::new().map(|x: X| x.0).select(score.gt(10)))
            .select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.0).select(owner_user_id.select(&uv).opt()))),
    );
    let v = top_n(v, |&(_, ((p, _), _))| (Reverse(score.get(p).unwrap()), view_count.get(p).is_none(), Reverse(view_count.get(p))), 10);
    rows(v.into_iter().map(|(_, ((p, l), u))| {
        let (up, dn) = u.unwrap_or((0, 0));
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::I(l as i64 + 1));
        f.extend(post_fields(db, p, &["score", "views"]));
        f.extend([V::I(up), V::I(dn), V::S(if up - dn > 0 { "Positive" } else if up - dn < 0 { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputationHistory AS ( SELECT U.Id AS UserId, U.Reputation, U.CreationDate, 1 AS Level FROM Users U WHERE U.Reputation > 0 UNION ALL SELECT U.Id,
//        U.Reputation + (SELECT COUNT(*) FROM Votes V WHERE V.UserId = U.Id) AS Reputation, U.CreationDate, UH.Level + 1 FROM Users U JOIN UserReputationHistory UH ON U.Id =
//        UH.UserId WHERE UH.Level < 5 ), PostVoteCounts AS ( SELECT P.Id AS PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3
//        THEN 1 END) AS DownVotes, COUNT(V.Id) AS TotalVotes FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id ), PopularPosts AS ( SELECT P.Id AS PostId,
//        P.Title, PVC.UpVotes, PVC.DownVotes, PVC.TotalVotes, ROW_NUMBER() OVER (ORDER BY PVC.UpVotes DESC, PVC.TotalVotes DESC) AS PopularityRank FROM Posts P JOIN
//        PostVoteCounts PVC ON P.Id = PVC.PostId WHERE P.CreationDate > CURRENT_TIMESTAMP - INTERVAL '30 days' ) SELECT U.DisplayName, URH.Reputation, PP.Title, PP.UpVotes,
//        PP.DownVotes, PP.TotalVotes FROM Users U JOIN UserReputationHistory URH ON U.Id = URH.UserId JOIN PopularPosts PP ON PP.UpVotes > 5 WHERE U.Id IN ( SELECT DISTINCT
//        C.UserId FROM Comments C WHERE C.CreationDate > CURRENT_TIMESTAMP - INTERVAL '1 year' GROUP BY C.UserId HAVING COUNT(*) > 10 ) ORDER BY URH.Reputation DESC,
//        PP.TotalVotes DESC;
//
// Empty while CURRENT_TIMESTAMP is more than a year past the data.
fn q31107(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    type X = (Id<User>, i64);
    let now = utc_to_ny(now_utc());
    let (c30, c1y) = (ny_to_utc(add_days(now, -30)), ny_to_utc(add_years(now, -1)));
    let vc = votes_per_user(db);
    let step = Same::<X>::new().map(|x: X| x.0).select(Ident::<User>::new().and(reputation).and(&vc)).map(|((u, r), n): ((Id<User>, i64), i64)| (u, r + n));
    let urh = db.user.with(reputation.gt(0)).select(Ident::<User>::new().and(reputation)).reach(step, 4);
    let urv = rel(drain(&urh));
    let cm = &db.comment;
    let active: MatSet<Id<User>> = (&cm.with((&cm.creation_date).filt(move |d| ny_to_utc(d) > c1y)).group_by(&cm.user).fold(0i64, |n, _| n + 1))
        .filt(|n: i64| n > 10)
        .inv()
        .collect();
    let pvc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold((0i64, 0i64, 0i64), |(u, d, n), t| {
        (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64, n + t.is_some() as i64)
    });
    let pp = rel(drain(db.post.with((&db.post.creation_date).filt(move |d| ny_to_utc(d) > c30)).select(Ident::<Post>::new().and(&pvc)).filt(|(_, (u, _, _)): (Id<Post>, (i64, i64, i64))| u > 5)));
    let v = drain((&urv).map(|(x, _): (X, usize)| x).with(Same::<X>::new().map(|x: X| x.0).select(&active)).cross(&pp));
    let v = top_n(v, |&(_, ((_, r), (_, (_, (_, _, n)))))| (Reverse(r), Reverse(n)), 0);
    rows(v.into_iter().map(|(_, ((u, r), (_, (p, (up, dn, n)))))| row(vec![V::S(db.user.display_name.get(u).unwrap()), V::I(r), title(db, p), V::I(up), V::I(dn), V::I(n)])))
}

// WITH RECURSIVE RecursivePostHierarchy AS ( SELECT p.Id, p.Title, p.CreationDate, p.ParentId, 1 AS Level FROM Posts p WHERE p.ParentId IS NULL UNION ALL SELECT p.Id,
//        p.Title, p.CreationDate, p.ParentId, rph.Level + 1 FROM Posts p INNER JOIN RecursivePostHierarchy rph ON p.ParentId = rph.Id ), PostVoteStats AS ( SELECT p.Id AS
//        PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(CASE WHEN v.VoteTypeId IN (2, 3) THEN
//        1 END) AS TotalVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id ), PostScoreRanked AS ( SELECT p.Id, p.Title, p.Score, COUNT(c.Id) AS
//        CommentCount, ps.UpVotes, ps.DownVotes, RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId JOIN PostVoteStats ps ON
//        p.Id = ps.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, ps.UpVotes, ps.DownVotes ), ClosedPosts AS ( SELECT p.Id, p.Title, ph.UserDisplayName AS
//        LastEditor, ph.CreationDate AS CloseDate FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10 ) SELECT p.Title AS QuestionTitle,
//        ph.Title AS ParentTitle, p.Score AS QuestionScore, COALESCE(ups.UpVotes, 0) AS UpVotes, COALESCE(downs.DownVotes, 0) AS DownVotes, COALESCE(c.CommentCount, 0) AS
//        CommentCount, cp.LastEditor AS ClosedBy, cp.CloseDate, ph.Level AS HierarchyLevel FROM PostScoreRanked p LEFT JOIN RecursivePostHierarchy ph ON p.Id = ph.Id LEFT JOIN
//        ClosedPosts cp ON p.Id = cp.Id LEFT JOIN PostVoteStats ups ON p.Id = ups.PostId LEFT JOIN PostVoteStats downs ON p.Id = downs.PostId LEFT JOIN (SELECT PostId, COUNT(*)
//        AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId WHERE p.Score > 0 ORDER BY p.Score DESC, p.ScoreRank;
fn q30455(db: &'static So) -> String {
    let Post { parent_id, post_type_id, score, .. } = &db.post;
    let ph = db.post.minus(parent_id).reach(children_of(db), usize::MAX);
    let phi: HashIdx<Id<Post>, usize> = (&ph).collect();
    let pvs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let hs = &db.post_history;
    let cp = history_of(db).with((&hs.post_history_type_id).eq(10)).select((&hs.user_display_name).opt().and(&hs.creation_date));
    let v = drain(db.post.with(post_type_id.eq(1)).with(score.gt(0)).select(Ident::<Post>::new().and((&phi).opt()).and(cp.opt()).and(&pvs)));
    let cc = comments_per_post(db);
    rows(v.into_iter().map(|(_, (((p, l), c), (u, d)))| {
        let pt = if l.is_some() { title(db, p) } else { V::Null };
        let mut f = vec![title(db, p), pt, V::I(score.get(p).unwrap()), V::I(u), V::I(d), V::I((&cc).get(p).unwrap())];
        match c {
            Some((n, t)) => f.extend([ostr(n), V::T(t)]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(match l {
            Some(l) => V::I(l as i64 + 1),
            None => V::Null,
        });
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS ( SELECT p.Id, p.Title, p.ParentId, p.CreationDate, 0 AS Level FROM Posts p WHERE p.ParentId IS NULL UNION ALL SELECT p.Id, p.Title,
//        p.ParentId, p.CreationDate, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.Id ), UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName,
//        COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        AVG(COALESCE(v.VoteCount, 0)) AS AvgScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN ( SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY
//        PostId ) v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName ), PostUpdates AS ( SELECT p.Id, p.Title, MAX(ph.Level) AS MaxLevel, COUNT(DISTINCT ph.Title) AS
//        UniqueHierarchyTitles FROM Posts p LEFT JOIN PostHierarchy ph ON p.Id = ph.Id GROUP BY p.Id, p.Title ), FeaturedUsers AS ( SELECT u.Id AS UserId, u.DisplayName,
//        u.Reputation, u.CreationDate, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation > 1000 ) SELECT u.DisplayName AS UserName,
//        u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.AvgScore, p.Title AS PostTitle, pu.MaxLevel, pu.UniqueHierarchyTitles, fu.UserRank FROM UserPostStats u JOIN Posts p
//        ON u.UserId = p.OwnerUserId JOIN PostUpdates pu ON p.Id = pu.Id LEFT JOIN FeaturedUsers fu ON u.UserId = fu.UserId WHERE pu.MaxLevel > 0 AND (SELECT COUNT(*) FROM
//        Comments c WHERE c.PostId = p.Id) > 0 ORDER BY u.AvgScore DESC, u.TotalPosts DESC LIMIT 10;
fn q31020(db: &'static So) -> String {
    let Post { parent_id, post_type_id, owner_user, title: pt, .. } = &db.post;
    type X = (Id<Post>, usize);
    let ph = db.post.minus(parent_id).reach(children_of(db), usize::MAX);
    let phv = rel(drain(&ph));
    let pu = (&phv).group_by(Same::<X>::new().map(|x: X| x.0)).select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.0).select(pt.opt()))).fold((0usize, 0i64), |(m, t), ((_, l), s)| (m.max(l), t.max(s.is_some() as i64)));
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_per_post(db))))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, q, a, s), (t, v)| (n + 1, q + (t == 1) as i64, a + (t == 2) as i64, s + v));
    let fu = rel(ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), true));
    let fui: HashIdx<Id<User>, ((Id<User>, i64), i64)> = (&fu).map(|((u, _), _)| u).inv().select(&fu).collect();
    let v = drain(
        db.post
            .with((&pu).filt(|(m, _): (usize, i64)| m > 0))
            .with(comments_of(db))
            .select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ups).and((&fui).opt()))).and(&pu)),
    );
    let v = top_n(v, |&(_, ((_, ((_, (n, _, _, s)), _)), _))| (Reverse(fkey(s as f64 / n as f64)), Reverse(n)), 10);
    rows(v.into_iter().map(|(_, ((p, ((u, (n, q, a, s)), r)), (m, t)))| {
        row(vec![
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(n),
            V::I(q),
            V::I(a),
            V::F(s as f64 / n as f64),
            title(db, p),
            V::I(m as i64),
            V::I(t),
            r.map_or(V::Null, |x| V::I(x.1)),
        ])
    }))
}

// WITH RECURSIVE PostHierarchy AS ( SELECT p.Id AS PostId, p.Title, p.ParentId, 0 AS Level FROM Posts p WHERE p.ParentId IS NULL UNION ALL SELECT p.Id, p.Title,
//        p.ParentId, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId ), UserActivity AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT
//        p.Id) AS PostCount, SUM(b.Class) AS BadgeCount, MAX(p.CreationDate) AS LastActive, DENSE_RANK() OVER (PARTITION BY u.Id ORDER BY MAX(p.CreationDate) DESC) AS UserRank
//        FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName ), MostPopularPosts AS ( SELECT p.Id,
//        p.Title, p.ViewCount, ROW_NUMBER() OVER (ORDER BY p.ViewCount DESC) AS ViewRank FROM Posts p WHERE p.ViewCount IS NOT NULL ), ActiveUsers AS ( SELECT ua.UserId,
//        ua.DisplayName, ua.PostCount, ua.BadgeCount FROM UserActivity ua WHERE ua.LastActive >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND ua.PostCount
//        > 0 ) SELECT ph.Title AS PostTitle, ph.Level AS PostLevel, up.UserId, up.DisplayName AS UserName, COALESCE(mp.ViewCount, 0) AS Popularity, CASE WHEN up.BadgeCount > 5
//        THEN 'Active Contributor' ELSE 'Regular Member' END AS UserStatus FROM PostHierarchy ph LEFT JOIN ActiveUsers up ON ph.PostId = up.UserId LEFT JOIN MostPopularPosts mp
//        ON ph.PostId = mp.Id WHERE (up.UserId IS NULL OR up.BadgeCount IS NOT NULL) AND EXISTS ( SELECT 1 FROM Comments c WHERE c.PostId = ph.PostId AND c.CreationDate >=
//        cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '7 days' ) ORDER BY ph.Level, UserStatus DESC, Popularity DESC LIMIT 50;
//
// `ph.PostId = up.UserId` is a raw-id join. The EXISTS reads only ph, so it is applied first, and UserActivity is folded only for the users that join can reach.
fn q30831(db: &'static So) -> String {
    let Post { parent_id, origid, creation_date, view_count, .. } = &db.post;
    type X = (Id<Post>, usize);
    let at = || Same::<X>::new().map(|x: X| x.0);
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ph = db.post.minus(parent_id).reach(children_of(db), usize::MAX);
    let phv = rel(drain(&ph));
    let recent = comments_of(db).with((&db.comment.creation_date).ge(t0 - 7 * DAY_US));
    let ph1 = rel(drain((&phv).with(at().select(recent))).into_iter().map(|x| x.1).collect());
    let ui: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let cand: MatSet<Id<User>> = (&ph1).select(at().select(origid).select(&ui)).collect();
    let ua = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(creation_date).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold((None::<i64>, None::<i64>), |(m, s), (d, c)| (m.max(d), match c {
            Some(c) => Some(s.unwrap_or(0) + c),
            None => s,
        }));
    let up = (&ua).filt(move |(m, _): (Option<i64>, Option<i64>)| m.is_some_and(|m| m >= t0 - 30 * DAY_US));
    let v = drain(
        (&ph1)
            .select(Same::<X>::new().and(at().select(origid.select(&ui).select(Ident::<User>::new().and(up)).opt())))
            .filt(|(_, u): (X, Option<(Id<User>, (Option<i64>, Option<i64>))>)| u.is_none_or(|(_, (_, b))| b.is_some())),
    );
    let st = |u: Option<(Id<User>, (Option<i64>, Option<i64>))>| u.is_some_and(|(_, (_, b))| b.unwrap_or(0) > 5);
    let v = top_n(v, |&(_, ((p, l), u))| (l, st(u), Reverse(view_count.get(p).unwrap_or(0))), 50);
    rows(v.into_iter().map(|(_, ((p, l), u))| {
        let mut f = vec![title(db, p), V::I(l as i64)];
        match u {
            Some((u, _)) => f.extend([V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap())]),
            None => f.extend([V::Null, V::Null]),
        }
        f.extend([V::I(view_count.get(p).unwrap_or(0)), V::S(if st(u) { "Active Contributor" } else { "Regular Member" })]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputationCTE AS ( SELECT U.Id, U.DisplayName, U.Reputation, U.LastAccessDate, 1 AS Level FROM Users U WHERE U.Reputation > 0 UNION ALL SELECT
//        U.Id, U.DisplayName, U.Reputation, U.LastAccessDate, UR.Level + 1 FROM Users U INNER JOIN Votes V ON U.Id = V.UserId INNER JOIN UserReputationCTE UR ON V.PostId IN (
//        SELECT P.Id FROM Posts P WHERE P.OwnerUserId = UR.Id ) WHERE UR.Level < 3 ) , PostWithComments AS ( SELECT P.Id AS PostId, P.Title, P.Body, COALESCE(COUNT(C.Id), 0)
//        AS CommentCount, P.CreationDate, P.LastActivityDate FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS
//        TIMESTAMP) - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.Body, P.CreationDate, P.LastActivityDate ) , RankedPosts AS ( SELECT P.Title, P.CommentCount, P.CreationDate,
//        RANK() OVER (ORDER BY P.CommentCount DESC) AS RankByComments FROM PostWithComments P ) SELECT UR.DisplayName, UR.Reputation, P.Title, P.CommentCount,
//        P.RankByComments, P.CreationDate FROM UserReputationCTE UR JOIN RankedPosts P ON P.RankByComments <= 10 WHERE UR.LastAccessDate >= CAST('2024-10-01 12:34:56' AS
//        TIMESTAMP) - INTERVAL '30 days' AND UR.Reputation > 100 ORDER BY UR.Reputation DESC, P.RankByComments;
//
// Postgres oracle. The step is one row per vote cast on a post the previous level's user owns.
fn q30985(db: &'static So) -> String {
    let User { reputation, last_access_date, .. } = &db.user;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let step = posts_of(db).select(votes_of(db)).select(&db.vote.user);
    let ur = db.user.with(reputation.gt(0)).reach(step, 2);
    let urv = rel(drain(&ur).into_iter().map(|x| x.0).collect());
    let pwc = drain(db.post.with((&db.post.creation_date).ge(add_years(t0, -1))).select(comments_per_post(db)));
    let rp = rel(ranked(pwc, |&(_, n)| Reverse(n), false));
    let v = drain((&urv).with(last_access_date.ge(t0 - 30 * DAY_US)).with(reputation.gt(100)).cross((&rp).filt(|(_, r): ((Id<Post>, i64), i64)| r <= 10)));
    rows(v.into_iter().map(|(_, (u, ((p, n), r)))| {
        let mut f = vec![V::S(db.user.display_name.get(u).unwrap()), V::I(reputation.get(u).unwrap()), title(db, p), V::I(n), V::I(r)];
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.ParentId, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT a.Id AS PostId, a.Title, a.ParentId, ph.Level + 1 FROM Posts a INNER JOIN PostHierarchy ph ON a.ParentId = ph.PostId WHERE a.PostTypeId = 2),
// GroupedVotes AS (SELECT v.PostId, vt.Name AS VoteType, COUNT(*) AS VoteCount FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId, vt.Name),
// BadgeStats AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Date) AS LastBadgeDate FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(gv.VoteCount, 0) AS TotalVotes, COALESCE(b.BadgeCount, 0) AS UserBadgeCount,
//        COALESCE(b.LastBadgeDate, '1900-01-01') AS LastBadgeDate, ph.Level AS HierarchyLevel
//     FROM Posts p LEFT JOIN GroupedVotes gv ON p.Id = gv.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN BadgeStats b ON u.Id = b.UserId
//     LEFT JOIN PostHierarchy ph ON ph.PostId = p.Id)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.TotalVotes, ps.UserBadgeCount, ps.LastBadgeDate, ps.HierarchyLevel FROM PostStats ps
// WHERE ps.TotalVotes > 0 AND ps.UserBadgeCount > 1 ORDER BY ps.TotalVotes DESC, ps.HierarchyLevel ASC LIMIT 10;
fn q31915(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let answers: HashIdx<Id<Post>, Id<Post>> = db.post.with(post_type_id.eq(2)).select(&db.post.parent).inv().collect();
    let ph: HashIdx<Id<Post>, usize> = (&db.post.with(post_type_id.eq(1)).reach(&answers, usize::MAX)).collect();
    let gv = rel(drain(db.vote.group_by((&db.vote.post).and(vtype_name(db))).fold(0i64, |a, _| a + 1)));
    type G = ((Id<Post>, Str), i64);
    let gvi: HashIdx<Id<Post>, G> = (&gv).map(|g: G| g.0.0).inv().select(&gv).collect();
    let bs = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain(db.post.select(Ident::<Post>::new().and(owner_user.select((&bs).filt(|(n, _): (i64, i64)| n > 1))).and(&gvi).and((&ph).opt())));
    let v = top_n(v, |&(_, (((_, _), (_, c)), l))| (Reverse(c), l.is_none(), l), 10);
    rows(v.into_iter().map(|(_, (((p, (n, d)), (_, c)), l))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(n), V::T(d), oint(l.map(|l| l as i64 + 1))]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, 0 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT a.Id AS PostId, a.Title, a.OwnerUserId, ph.Level + 1 AS Level FROM Posts a INNER JOIN PostHierarchy ph ON a.ParentId = ph.PostId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(DISTINCT p.Id) AS PostsCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, ph.Title, COUNT(DISTINCT pht.Id) AS CloseCount FROM PostHierarchy ph
//     LEFT JOIN PostHistory pht ON ph.PostId = pht.PostId AND pht.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.Title)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.Upvotes, us.Downvotes, us.PostsCount, cp.Title AS ClosedPostTitle, cp.CloseCount
// FROM UserStats us LEFT JOIN ClosedPosts cp ON us.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = cp.PostId)
// WHERE us.Reputation > 1000 ORDER BY us.Reputation DESC, cp.CloseCount DESC, us.UserId, cp.PostId OFFSET 0 ROWS FETCH NEXT 50 ROWS ONLY;
//
// Ported from rewrites/33470.sql (`, us.UserId, cp.PostId` added: each user's closed-post rows tie on CloseCount and differ in Title).
// ClosedPosts groups by post, so a post the recursion reaches more than once is one row.
fn q33470(db: &'static So) -> String {
    let Post { post_type_id, origid, .. } = &db.post;
    let ph = db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX);
    let phs: MatSet<Id<Post>> = (&ph).inv().collect();
    let close = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let uv = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id))).fold((0i64, 0i64), |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64));
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |a, _| a + 1);
    let cp = posts_of(db).select(Ident::<Post>::new().with(&phs).and(&close));
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select(Ident::<User>::new().and(&db.user.reputation).and((&uv).opt()).and((&pc).opt()).and(cp.opt())));
    let v = top_n(v, |&(_, ((((u, r), _), _), c))| (Reverse(r), c.is_none(), Reverse(c.map(|c| c.1)), db.user.origid.get(u).unwrap(), c.is_none(), c.map(|c| origid.get(c.0).unwrap())), 50);
    rows(v.into_iter().map(|(_, ((((u, _), uv), pc), c))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        let (up, dn) = uv.unwrap_or((0, 0));
        f.extend([V::I(up), V::I(dn), V::I(pc.unwrap_or(0))]);
        match c {
            Some((p, n)) => f.extend([title(db, p), V::I(n)]),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.ParentId, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.Title, p.ParentId, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Score, p.CreationDate, COALESCE(ph.Level, 0) AS HierarchyLevel, COALESCE(ue.VoteCount, 0) AS UserEngagementCount,
//        COALESCE(ue.UpVotes, 0) AS UpVoteCount, COALESCE(ue.DownVotes, 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN PostHierarchy ph ON p.Id = ph.PostId LEFT JOIN UserEngagement ue ON p.OwnerUserId = ue.UserId),
// UserPostEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(ps.PostId) AS TotalPosts, SUM(CASE WHEN ps.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN ps.Score <= 0 THEN 1 ELSE 0 END) AS NonPositivePosts, AVG(ps.Score) AS AverageScore
//     FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT u.DisplayName, up.TotalPosts, up.PositivePosts, up.NonPositivePosts, up.AverageScore, SUM(ps.UserEngagementCount) AS TotalEngagement,
//        SUM(ps.UpVoteCount) AS TotalUpVotes, SUM(ps.DownVoteCount) AS TotalDownVotes, MAX(ps.CreationDate) AS LatestPostDate
// FROM UserPostEngagement up JOIN PostStats ps ON up.UserId = ps.OwnerUserId JOIN Users u ON u.Id = up.UserId
// GROUP BY u.DisplayName, up.TotalPosts, up.PositivePosts, up.NonPositivePosts, up.AverageScore HAVING SUM(ps.Score) > 0 ORDER BY TotalEngagement DESC LIMIT 10;
//
// A post is a PostStats row once per time the recursion reaches it (once if never). UserPostEngagement and the outer join read the same
// rows of the same owner, so both are one fold per user, and the users are then grouped by the outer key.
fn q31162(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let ph = db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX);
    let phi: HashIdx<Id<Post>, usize> = (&ph).collect();
    let ue = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold((0i64, 0i64, 0i64), |(n, u, d), t| {
        (n + t.is_some() as i64, u + (t == Some(2)) as i64, d + (t == Some(3)) as i64)
    });
    let ps = posts_of(db).select(score.and(creation_date).and((&phi).opt())).and(&ue);
    type F = [i64; 8];
    let per = db.user.group_by(Ident::<User>::new()).select(ps).fold([0i64, 0, 0, 0, 0, 0, 0, i64::MIN], |a: F, (((s, c), _), (n, u, d))| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s <= 0) as i64, a[3] + s, a[4] + n, a[5] + u, a[6] + d, a[7].max(c)]
    });
    type K = (Str, i64, i64, i64, u64);
    let r = rel(drain(db.user.select(Ident::<User>::new().and(&db.user.display_name).and(&per))));
    type R = (Id<User>, ((Id<User>, Str), F));
    let g = (&r)
        .group_by(Same::<R>::new().map(|(_, ((_, nm), a)): R| (nm, a[0], a[1], a[2], (a[3] as f64 / a[0] as f64).to_bits())))
        .fold([0i64, 0, 0, 0, i64::MIN], |b, (_, (_, a)): R| [b[0] + a[3], b[1] + a[4], b[2] + a[5], b[3] + a[6], b[4].max(a[7])]);
    let v = drain((&g).filt(|b: [i64; 5]| b[0] > 0));
    let v = top_n(v, |&(_, b)| Reverse(b[1]), 10);
    rows(v.into_iter().map(|((nm, tp, pp, np, avg), b): (K, [i64; 5])| {
        row(vec![V::S(nm), V::I(tp), V::I(pp), V::I(np), V::F(f64::from_bits(avg)), V::I(b[1]), V::I(b[2]), V::I(b[3]), V::T(b[4])])
    }))
}

// WITH RECURSIVE PostTree AS (SELECT Id, Title, ParentId, CreationDate, Score, 1 AS Level FROM Posts WHERE ParentId IS NULL
//     UNION ALL SELECT p.Id, p.Title, p.ParentId, p.CreationDate, p.Score, pt.Level + 1 FROM Posts p INNER JOIN PostTree pt ON p.ParentId = pt.Id),
// UserVotes AS (SELECT v.PostId, v.UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v GROUP BY v.PostId, v.UserId),
// PostStats AS (SELECT p.Id, p.Title, p.OwnerUserId, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName, COALESCE(tree.Level, 0) AS PostLevel,
//        COALESCE(v.UpVotes, 0) AS TotalUpVotes, COALESCE(v.DownVotes, 0) AS TotalDownVotes,
//        CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted Answer' ELSE 'No Accepted Answer' END AS AnswerStatus
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserVotes v ON p.Id = v.PostId LEFT JOIN PostTree tree ON p.Id = tree.Id)
// SELECT ps.Id, ps.Title, ps.OwnerDisplayName, ps.PostLevel, ps.TotalUpVotes, ps.TotalDownVotes, ps.AnswerStatus,
//        CASE WHEN ps.TotalUpVotes - ps.TotalDownVotes > 0 THEN 'Positive' WHEN ps.TotalUpVotes - ps.TotalDownVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = ps.Id) AS CommentCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = ps.Id AND v.VoteTypeId = 2) AS UpVoteCount
// FROM PostStats ps WHERE (ps.TotalUpVotes > 5 OR ps.TotalDownVotes > 3) AND ps.AnswerStatus = 'Accepted Answer'
// ORDER BY ps.PostLevel DESC, ps.TotalUpVotes DESC LIMIT 100;
fn q32809(db: &'static So) -> String {
    let Post { parent, accepted_answer_id, owner_user, .. } = &db.post;
    let tree = db.post.minus(parent).reach(children_of(db), usize::MAX);
    let ti: HashIdx<Id<Post>, usize> = (&tree).collect();
    let uv = rel(drain(db.vote.group_by((&db.vote.post).and((&db.vote.user_id).opt())).select(&db.vote.vote_type_id).fold((0i64, 0i64), |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64))));
    type G = ((Id<Post>, Option<i64>), (i64, i64));
    let uvi: HashIdx<Id<Post>, (i64, i64)> = (&uv).map(|g: G| g.0.0).inv().select(&uv).map(|g: G| g.1).collect();
    let v = drain(
        db.post
            .with(accepted_answer_id)
            .select(Ident::<Post>::new().and((&uvi).opt()).and((&ti).opt()))
            .filt(|((_, c), _): ((Id<Post>, Option<(i64, i64)>), Option<usize>)| {
                let (u, d) = c.unwrap_or((0, 0));
                u > 5 || d > 3
            }),
    );
    let v = top_n(v, |&(_, ((_, c), l))| (Reverse(l.map_or(0, |l| l + 1)), Reverse(c.unwrap_or((0, 0)).0)), 100);
    let cpp = comments_per_post(db);
    let up = votes_of_type(db, 2);
    rows(v.into_iter().map(|(_, ((p, c), l))| {
        let (u, d) = c.unwrap_or((0, 0));
        let name = owner_user.get(p).map_or("Anonymous", |u| db.user.display_name.get(u).unwrap());
        let s = if u - d > 0 { "Positive" } else if u - d < 0 { "Negative" } else { "Neutral" };
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::S(name), V::I(l.map_or(0, |l| l as i64 + 1)), V::I(u), V::I(d), V::S("Accepted Answer"), V::S(s), V::I(cpp.get(p).unwrap()), V::I(up.get(p).unwrap())]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT Id, ParentId, Title, Score, CreationDate, 0 AS Level FROM Posts WHERE ParentId IS NULL
//     UNION ALL SELECT p.Id, p.ParentId, p.Title, p.Score, p.CreationDate, ph.Level + 1 AS Level FROM Posts p JOIN PostHierarchy ph ON p.ParentId = ph.Id),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(v.UpVoteCount, 0) AS UpVoteCount, COALESCE(v.DownVoteCount, 0) AS DownVoteCount, ph.Level,
//        COALESCE(c.CommentCount, 0) AS CommentCount
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//         FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId LEFT JOIN PostHierarchy ph ON p.Id = ph.Id),
// RecentPostHistory AS (SELECT ph.PostId, ph.UserDisplayName, ph.CreationDate, MAX(CASE WHEN pht.Name = 'Post Closed' THEN ph.CreationDate END) AS LastClosedDate,
//        COUNT(CASE WHEN pht.Name = 'Post Closed' THEN 1 END) AS CloseCount
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId, ph.UserDisplayName, ph.CreationDate)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Score AS PostScore, pd.UpVoteCount, pd.DownVoteCount, pd.CommentCount, rph.UserDisplayName AS LastCloser,
//        rph.LastClosedDate, rph.CloseCount, pd.Level
// FROM PostDetails pd LEFT JOIN RecentPostHistory rph ON pd.PostId = rph.PostId WHERE pd.Score > 0
// ORDER BY pd.Score DESC, pd.CommentCount DESC, pd.CreationDate DESC LIMIT 50;
fn q34789(db: &'static So) -> String {
    let Post { parent, score, creation_date, .. } = &db.post;
    let ph = db.post.minus(parent).reach(children_of(db), usize::MAX);
    let phi: HashIdx<Id<Post>, usize> = (&ph).collect();
    let PostHistory { post, user_display_name, creation_date: hdate, .. } = &db.post_history;
    let rph = rel(drain(
        db.post_history
            .group_by(post.and(user_display_name.opt()).and(hdate))
            .select(htype_name(db).and(hdate))
            .fold((i64::MIN, 0i64), |(m, n), (t, d)| if t == "Post Closed" { (m.max(d), n + 1) } else { (m, n) }),
    ));
    type G = (((Id<Post>, Option<Str>), i64), (i64, i64));
    let rphi: HashIdx<Id<Post>, G> = (&rph).map(|g: G| g.0.0.0).inv().select(&rph).collect();
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold((0i64, 0i64), |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64));
    let cpp = comments_per_post(db);
    let v = drain(db.post.with(score.gt(0)).select(Ident::<Post>::new().and(score).and(&cpp).and((&phi).opt()).and((&rphi).opt())));
    let v = top_n(v, |&(_, ((((p, s), c), _), _))| (Reverse(s), Reverse(c), Reverse(creation_date.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(_, ((((p, _), c), l), r))| {
        let (u, d) = vc.get(p).unwrap_or((0, 0));
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(u), V::I(d), V::I(c)]);
        match r {
            Some((((_, n), _), (m, k))) => f.extend([ostr(n), if k == 0 { V::Null } else { V::T(m) }, V::I(k)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.push(oint(l.map(|l| l as i64)));
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.ParentId, p.Title, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.ParentId, p.Title, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId),
// PostViews AS (SELECT p.Id, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COALESCE(COUNT(c.Id), 0) AS TotalComments, COUNT(DISTINCT v.UserId) AS UniqueVoters,
//        AVG(v.BountyAmount) AS AvgBounty
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 10) LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// ClosedPosts AS (SELECT ph.PostId, ph.Title, ph.Level, p.CreationDate, p.LastActivityDate FROM PostHierarchy ph INNER JOIN Posts p ON ph.PostId = p.Id
//     WHERE p.Id IN (SELECT PostId FROM PostHistory WHERE PostHistoryTypeId = 10)),
// PostMetrics AS (SELECT p.Id AS PostId, p.Title, COALESCE(cv.TotalBounty, 0) AS TotalBounty, cv.TotalComments, cv.UniqueVoters FROM Posts p LEFT JOIN PostViews cv ON p.Id = cv.Id)
// SELECT pm.PostId, pm.Title, pm.TotalBounty, pm.TotalComments, pm.UniqueVoters, ch.Level AS HierarchyLevel,
//        CASE WHEN pm.TotalComments = 0 THEN 'No Comments' WHEN pm.TotalComments > 10 THEN 'Highly Discussed' ELSE 'Moderately Discussed' END AS DiscussionLevel,
//        CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Not Accepted' END AS AcceptanceStatus,
//        CASE WHEN p.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM PostMetrics pm LEFT JOIN ClosedPosts ch ON pm.PostId = ch.PostId LEFT JOIN Posts p ON pm.PostId = p.Id ORDER BY pm.TotalBounty DESC, pm.UniqueVoters DESC;
fn q30788(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, closed_date, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, user_id, .. } = &db.vote;
    let ph = db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX);
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).collect();
    let ch: HashIdx<Id<Post>, usize> = (&ph).inv().with(&closed).inv().collect();
    let bv = || votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.filt(|t| t == 2 || t == 10)));
    let pv = db
        .post
        .group_by(Ident::<Post>::new())
        .select(bv().select(bounty_amount.opt()).opt().and(comments_of(db).opt()))
        .fold((0i64, 0i64), |(b, n), (v, c)| (b + v.flatten().unwrap_or(0), n + c.is_some() as i64));
    let uv = per_post_distinct(db, bv().select(user_id));
    let v = drain(db.post.select(Ident::<Post>::new().and(&pv).and((&ch).opt())));
    rows(v.into_iter().map(|(_, ((p, (b, n)), l))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        let d = if n == 0 { "No Comments" } else if n > 10 { "Highly Discussed" } else { "Moderately Discussed" };
        f.extend([V::I(b), V::I(n), V::I(uv.get(p).unwrap_or(0)), oint(l.map(|l| l as i64 + 1)), V::S(d)]);
        f.push(V::S(if accepted_answer_id.get(p).is_some() { "Accepted" } else { "Not Accepted" }));
        f.push(V::S(if closed_date.get(p).is_some() { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.ParentId, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.ParentId, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId),
// PostStats AS (SELECT p.Id AS PostId, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, AVG(u.Reputation) AS AverageReputation
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id GROUP BY p.Id),
// RecentActivity AS (SELECT p.Id AS PostId, p.LastActivityDate, EXTRACT(EPOCH FROM (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - p.LastActivityDate)) / 86400 AS DaysSinceLastActivity
//     FROM Posts p WHERE p.LastActivityDate IS NOT NULL),
// FinalStats AS (SELECT ph.PostId, ph.Title, ps.CommentCount, ps.VoteCount, ra.DaysSinceLastActivity,
//        CASE WHEN ra.DaysSinceLastActivity < 30 THEN 'Active' WHEN ra.DaysSinceLastActivity BETWEEN 30 AND 90 THEN 'Moderately Active' ELSE 'Inactive' END AS ActivityStatus,
//        COALESCE(ps.VoteCount, 0) AS SafeVoteCount, COALESCE(ps.CommentCount, 0) AS SafeCommentCount
//     FROM PostHierarchy ph LEFT JOIN PostStats ps ON ph.PostId = ps.PostId LEFT JOIN RecentActivity ra ON ph.PostId = ra.PostId)
// SELECT f.Title, f.CommentCount, f.VoteCount, f.DaysSinceLastActivity, f.ActivityStatus,
//        CONCAT('Post ID: ', f.PostId, ', has ', f.SafeVoteCount, ' votes and ', f.SafeCommentCount, ' comments.') AS Summary
// FROM FinalStats f WHERE f.ActivityStatus = 'Inactive' ORDER BY f.DaysSinceLastActivity DESC LIMIT 50;
fn q30583(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, origid, .. } = &db.post;
    let ph = db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX);
    let days = |p: Id<Post>| {
        let us = ts(2024, 10, 1, 12, 34, 56) - last_activity_date.get(p).unwrap();
        ((us / DAY_US) as f64 * 86400.0 + (us % DAY_US) as f64 / 1e6) / 86400.0
    };
    let v = drain((&ph).inv().map(|p: Id<Post>| p).filt(move |p: Id<Post>| {
        let d = days(p);
        !(d < 30.0 || (30.0..=90.0).contains(&d))
    }));
    let v = top_n(v, |&(_, p)| Reverse(fkey(days(p))), 50);
    let cpp = comments_per_post(db);
    let vpp = votes_per_post(db);
    rows(v.into_iter().map(|(_, p)| {
        let (c, n) = (cpp.get(p).unwrap(), vpp.get(p).unwrap());
        row(vec![title(db, p), V::I(c), V::I(n), V::F(days(p)), V::S("Inactive"), V::Owned(format!("Post ID: {}, has {} votes and {} comments.", origid.get(p).unwrap(), n, c))])
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId, p.CreationDate, p.ViewCount, p.Score, 1 AS Level
//     FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p2.Id AS PostId, p2.Title, p2.PostTypeId, COALESCE(p2.AcceptedAnswerId, 0) AS AcceptedAnswerId, p2.CreationDate, p2.ViewCount, p2.Score, ph.Level + 1
//     FROM Posts p2 INNER JOIN PostHierarchy ph ON ph.PostId = p2.ParentId),
// PostScores AS (SELECT ph.PostId, ph.Title, ph.CreationDate, ph.ViewCount, ph.Score, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS rn FROM PostHierarchy ph),
// UserBadges AS (SELECT u.DisplayName, b.Name AS BadgeName, b.Class, b.Date FROM Badges b JOIN Users u ON u.Id = b.UserId
//     WHERE b.Date > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// ClosePosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT p.PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(c.CloseCount, 0) AS CloseCount, ub.BadgeName, ub.Class AS BadgeClass
// FROM PostScores p LEFT JOIN ClosePosts c ON p.PostId = c.PostId
// LEFT JOIN (SELECT ub.DisplayName, MAX(ub.BadgeName) AS BadgeName, MAX(ub.Class) AS Class FROM UserBadges ub GROUP BY ub.DisplayName) ub
//     ON ub.DisplayName = (SELECT DisplayName FROM Users WHERE Id = p.PostId)
// WHERE p.rn = 1 ORDER BY p.Score DESC, p.ViewCount DESC LIMIT 10;
//
// Every row of a post in PostHierarchy carries the same columns, so `rn = 1` is one row per post the recursion reaches. The badge join
// compares the post's Id with a user's Id, as written.
fn q34568(db: &'static So) -> String {
    let Post { post_type_id, origid, score, view_count, .. } = &db.post;
    let ph = db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX);
    let phs: MatSet<Id<Post>> = (&ph).inv().collect();
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db
        .badge
        .with((&db.badge.date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by((&db.badge.user).select(&db.user.display_name))
        .select((&db.badge.name).and(&db.badge.class))
        .fold(None, |a: Option<(Str, i64)>, (n, c)| Some(a.map_or((n, c), |(m, k)| (m.max(n), k.max(c)))));
    let cc = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let v = drain((&phs).select(Ident::<Post>::new().and(score).and(view_count.opt()).and(&cc).and(origid.select(&uid).select(&db.user.display_name).select(&ub).opt())));
    let v = top_n(v, |&(_, ((((_, s), w), _), _))| (Reverse(s), w.is_none(), Reverse(w)), 10);
    rows(v.into_iter().map(|(_, ((((p, _), _), c), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(V::I(c));
        match b.flatten() {
            Some((n, k)) => f.extend([V::S(n), V::I(k)]),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, 1 AS Level, p.CreationDate FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id AS PostId, p.Title, p.OwnerUserId, ph.Level + 1, p.CreationDate FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId WHERE p.PostTypeId = 2),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPosts AS (SELECT ph.PostId, ph.Title, ph.OwnerUserId, ph.Level, ph.CreationDate, ROW_NUMBER() OVER (PARTITION BY ph.OwnerUserId ORDER BY ph.CreationDate DESC) AS RecentRank
//     FROM PostHierarchy ph WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 DAYS')
// SELECT u.DisplayName, u.Reputation, ur.BadgeCount, ur.TotalBounty, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostCreationDate
// FROM UserReputation ur LEFT JOIN RecentPosts rp ON ur.UserId = rp.OwnerUserId LEFT JOIN Users u ON ur.UserId = u.Id
// WHERE rp.RecentRank = 1 ORDER BY ur.Reputation DESC, rp.CreationDate DESC LIMIT 50;
fn q33742(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let ph = db.post.with(post_type_id.eq(1)).reach(answers_of(db), usize::MAX);
    let w = (&ph)
        .inv()
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, c): (Id<Post>, i64)| Reverse(c), asc);
    let rp: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, n): ((Id<Post>, i64), i64)| n == 1).map(|((p, _), _)| p).collect();
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64), |(n, s), (_, b)| (n, s + b.flatten().unwrap_or(0)));
    let bc = badges_per_user(db);
    let v = drain(db.user.select(Ident::<User>::new().and(&db.user.reputation).and(&rp).and(&ur)));
    let v = top_n(v, |&(_, (((_, r), p), _))| (Reverse(r), Reverse(creation_date.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(_, (((u, _), p), (_, s)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(bc.get(u).unwrap()), V::I(s)]);
        f.extend(post_fields(db, p, &["title", "created"]));
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.ParentId, 1 AS Depth FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.Title, p.ParentId, ph.Depth + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT b.Id) AS BadgeCount, COALESCE(SUM(ph.Depth), 0) AS PostDepthSum
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     LEFT JOIN PostHierarchy ph ON p.Id = ph.PostId GROUP BY u.Id, u.DisplayName),
// RecentQuestions AS (SELECT Id AS QuestionId, Title, CreationDate, ViewCount, Score, COALESCE(ROUND(Score * 1.0 / NULLIF(ViewCount, 0), 2), 0) AS ScorePerView,
//        ROW_NUMBER() OVER (ORDER BY CreationDate DESC) AS RecentRank FROM Posts WHERE PostTypeId = 1)
// SELECT ua.UserId, ua.DisplayName, ua.QuestionCount, ua.UpVotes, ua.DownVotes, ua.BadgeCount, ua.PostDepthSum, rq.QuestionId, rq.Title, rq.CreationDate, rq.ViewCount,
//        rq.Score, rq.ScorePerView
// FROM UserActivity ua LEFT JOIN RecentQuestions rq ON ua.QuestionCount > 0 AND rq.RecentRank <= 5 ORDER BY ua.QuestionCount DESC, ua.UpVotes DESC, rq.ScorePerView DESC;
fn q31584(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let ph = db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX);
    let phi: HashIdx<Id<Post>, usize> = (&ph).collect();
    let q = || Ident::<Post>::new().with(post_type_id.eq(1));
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(q().select(votes_of(db).select(&db.vote.vote_type_id).opt().and((&phi).opt()))).opt().and(badges_of(db).opt()))
        .fold((0i64, 0i64, 0i64), |(u, d, s), (r, _)| match r {
            Some((t, l)) => (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64, s + l.map_or(0, |l| l as i64 + 1)),
            None => (u, d, s),
        });
    let qc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(q())).fold(0i64, |a, _| a + 1);
    let rq = top_n(drain(db.post.with(post_type_id.eq(1))), |&(p, _)| Reverse(creation_date.get(p).unwrap()), 5);
    let rq = rel(rq.into_iter().map(|x| x.1).collect());
    let bc = badges_per_user(db);
    let with_q = drain(db.user.select(Ident::<User>::new().and(&qc)).cross(&rq));
    let without = drain(db.user.minus(&qc));
    let ufields = |u: Id<User>, n: i64| {
        let (up, dn, s) = ua.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(up), V::I(dn), V::I(bc.get(u).unwrap()), V::I(s)]);
        f
    };
    let a = with_q.into_iter().map(|(_, ((u, n), p))| {
        let mut f = ufields(u, n);
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        let spv = match view_count.get(p) {
            Some(w) if w != 0 => ((score.get(p).unwrap() as f64 / w as f64) * 100.0).round() / 100.0,
            _ => 0.0,
        };
        f.push(V::F(spv));
        row(f)
    });
    let b = without.into_iter().map(|(u, _)| {
        let mut f = ufields(u, 0);
        f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null]);
        row(f)
    });
    rows(a.chain(b))
}

// WITH RECURSIVE UserHierarchy AS (SELECT Id, DisplayName, Reputation, CreationDate, CAST(DisplayName AS VARCHAR(255)) AS Path FROM Users WHERE Id = 1
//     UNION ALL SELECT u.Id, u.DisplayName, u.Reputation, u.CreationDate, CONCAT(uh.Path, ' -> ', u.DisplayName) FROM Users u JOIN UserHierarchy uh ON u.Id = uh.Id + 1),
// PostData AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT b.Id) AS BadgeCount, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.AcceptedAnswerId, p.OwnerUserId),
// RankedPosts AS (SELECT pd.*, RANK() OVER (ORDER BY pd.UpVotes DESC, pd.CommentCount DESC) AS VoteRank FROM PostData pd)
// SELECT up.DisplayName, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.BadgeCount, CASE WHEN rp.AcceptedAnswerId > 0 THEN 'Yes' ELSE 'No' END AS HasAcceptedAnswer, uh.Path
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id JOIN UserHierarchy uh ON up.Id = uh.Id WHERE rp.VoteRank <= 10 ORDER BY rp.VoteRank;
//
// The recursion's node carries the Path built so far.
fn q30795(db: &'static So) -> String {
    let Post { creation_date, owner_user, accepted_answer_id, .. } = &db.post;
    let User { origid, display_name, .. } = &db.user;
    type N = (Id<User>, Str);
    let uid: HashIdx<i64, Id<User>> = origid.inv().collect();
    let named = || Ident::<User>::new().and(display_name);
    let step = Same::<N>::new()
        .map(|n: N| n.0)
        .select(origid)
        .map(|i: i64| i + 1)
        .select(&uid)
        .select(named())
        .and(Same::<N>::new())
        .map(|((v, nm), (_, path)): (N, N)| (v, &*Box::leak(format!("{path} -> {nm}").into_boxed_str())));
    let uh = rel(drain(db.user.with(origid.eq(1)).select(named()).reach(step, usize::MAX)));
    type L = (N, usize);
    let uhi: HashIdx<Id<User>, Str> = (&uh).map(|x: L| x.0.0).inv().select(&uh).map(|x: L| x.0.1).collect();
    let pd = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold((0i64, 0i64, 0i64), |(c, u, d), ((ci, t), _)| (c + ci.is_some() as i64, u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let r = ranked(drain(&pd), |&(_, (c, u, _))| (Reverse(u), Reverse(c)), false);
    let top = rel(r);
    type T = ((Id<Post>, (i64, i64, i64)), i64);
    let v = drain((&top).filt(|(_, k): T| k <= 10).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0.0).select(owner_user).select(Ident::<User>::new().and(&uhi)))));
    let bc = badges_per_user(db);
    rows(v.into_iter().map(|(_, (((p, (c, up, dn)), _), (u, path)))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(c), V::I(up), V::I(dn), V::I(bc.get(u).unwrap()), V::S(if accepted_answer_id.get(p).unwrap_or(0) > 0 { "Yes" } else { "No" }), V::S(path)]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputationCTE AS (SELECT U.Id, U.Reputation, U.DisplayName, 0 AS Level, CAST(U.DisplayName AS VARCHAR(255)) AS Path FROM Users U WHERE U.Reputation > 1000
//     UNION ALL SELECT U.Id, U.Reputation, U.DisplayName, C.Level + 1, CAST(C.Path || ' -> ' || U.DisplayName AS VARCHAR(255)) AS Path
//     FROM Users U INNER JOIN Votes V ON U.Id = V.UserId INNER JOIN UserReputationCTE C ON V.PostId IN (SELECT P.Id FROM Posts P WHERE P.OwnerUserId = C.Id)
//     WHERE U.Reputation > C.Reputation),
// RecentPosts AS (SELECT P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, P.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) as RecentPostRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostDetails AS (SELECT RP.Title, RP.CreationDate, U.DisplayName AS OwnerDisplayName,
//        CASE WHEN V.VoteTypeId = 2 THEN 'Upvote' WHEN V.VoteTypeId = 3 THEN 'Downvote' ELSE 'Other' END AS VoteType, PH.Comment AS CloseReason,
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM RecentPosts RP JOIN Users U ON RP.OwnerUserId = U.Id LEFT JOIN Votes V ON RP.Id = V.PostId LEFT JOIN PostHistory PH ON RP.Id = PH.PostId AND PH.PostHistoryTypeId = 10
//     LEFT JOIN Comments C ON RP.Id = C.PostId WHERE RP.RecentPostRank = 1 GROUP BY RP.Title, RP.CreationDate, U.DisplayName, V.VoteTypeId, PH.Comment)
// SELECT PD.Title, PD.CreationDate, PD.OwnerDisplayName, COALESCE(PD.CloseReason, 'Not Closed') AS CloseReason, PD.CommentCount, PD.UpvoteCount, PD.DownvoteCount,
//        (SELECT AVG(Reputation) FROM UserReputationCTE) AS AvgReputationOfActiveUsers
// FROM PostDetails PD ORDER BY PD.CreationDate DESC LIMIT 100;
//
// Only AVG(Reputation) is read from the recursion, so its node is the user alone; Level and Path are never projected.
fn q34809(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let reputation = &db.user.reputation;
    let step = posts_of(db)
        .select(votes_of(db))
        .select(&db.vote.user)
        .select(Ident::<User>::new().and(reputation))
        .and(reputation)
        .filt(|((_, rv), rc): ((Id<User>, i64), i64)| rv > rc)
        .map(|((v, _), _): ((Id<User>, i64), i64)| v);
    let cte = db.user.with(reputation.gt(1000)).reach(step, usize::MAX);
    let (s, n) = (&cte).inv().select(reputation).fold_flat((0i128, 0i64), |(s, n), r| (s + r as i128, n + 1));
    let avg = (n > 0).then(|| s as f64 / n as f64);
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, c): (Id<Post>, i64)| Reverse(c), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, n): ((Id<Post>, i64), i64)| n == 1).map(|((p, _), _)| p).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let prod = rel(drain(
        (&rp).select(
            (&db.post.title)
                .opt()
                .and(creation_date)
                .and(owner_user.select(&db.user.display_name))
                .and(votes_of(db).select(&db.vote.vote_type_id).opt())
                .and(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(comment.opt()).opt())
                .and(comments_of(db).opt()),
        ),
    ));
    type K = (Option<Str>, i64, Str, Option<i64>, Option<Option<Str>>);
    type R = (Id<Post>, (((((Option<Str>, i64), Str), Option<i64>), Option<Option<Str>>), Option<Id<Comment>>));
    let g = (&prod)
        .group_by(Same::<R>::new().map(|(_, (((((t, c), u), vt), h), _)): R| (t, c, u, vt, h)))
        .fold((0i64, 0i64, 0i64), |(c, u, d), (_, (((((_, _), _), vt), _), ci)): R| (c + ci.is_some() as i64, u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64));
    let v = top_n(drain(&g), |&((_, c, _, _, _), _): &(K, (i64, i64, i64))| Reverse(c), 100);
    rows(v.into_iter().map(|((t, c, u, _, h), (n, up, dn))| {
        row(vec![ostr(t), V::T(c), V::S(u), V::S(h.flatten().unwrap_or("Not Closed")), V::I(n), V::I(up), V::I(dn), ofloat(avg)])
    }))
}

// WITH RECURSIVE HighRankingUsers AS (SELECT Id, DisplayName, Reputation FROM Users WHERE Reputation > 5000
//     UNION ALL SELECT u.Id, u.DisplayName, u.Reputation FROM Users u INNER JOIN HighRankingUsers hru ON u.Id = hru.Id + 1 WHERE u.Reputation > 5000),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews, MAX(p.CreationDate) AS LastPostDate FROM Posts p GROUP BY p.OwnerUserId),
// UserPostInfo AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ub.BadgeNames, 'None') AS BadgeNames,
//        COALESCE(ps.TotalQuestions, 0) AS TotalQuestions, COALESCE(ps.TotalAnswers, 0) AS TotalAnswers, COALESCE(ps.TotalScore, 0) AS TotalScore,
//        COALESCE(ps.TotalViews, 0) AS TotalViews, ps.LastPostDate
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId),
// HighlyActiveUsers AS (SELECT u.DisplayName, u.BadgeCount, u.TotalQuestions + u.TotalAnswers AS TotalPosts, RANK() OVER (ORDER BY u.TotalViews DESC) AS ViewRank
//     FROM UserPostInfo u WHERE u.TotalQuestions + u.TotalAnswers > 10)
// SELECT ha.DisplayName, ha.BadgeCount, ha.TotalPosts, ha.ViewRank, hru.Reputation AS HighRankReputation
// FROM HighlyActiveUsers ha LEFT JOIN HighRankingUsers hru ON ha.DisplayName = hru.DisplayName ORDER BY ha.ViewRank, ha.TotalPosts DESC;
//
// BadgeNames (an unordered STRING_AGG) is never projected.
fn q34195(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let User { origid, reputation, display_name, .. } = &db.user;
    let uid: HashIdx<i64, Id<User>> = origid.inv().collect();
    let step = origid.map(|i: i64| i + 1).select(&uid).select(Ident::<User>::new().with(reputation.gt(5000)));
    let hru = rel(drain(db.user.with(reputation.gt(5000)).reach(step, usize::MAX)));
    type L = (Id<User>, usize);
    let hidx: HashIdx<Str, i64> = (&hru).map(|x: L| x.0).select(display_name).inv().select(&hru).map(|x: L| x.0).select(reputation).collect();
    let ps = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt())))
        .fold((0i64, 0i64), |(n, w), (t, v)| (n + (t == 1 || t == 2) as i64, w + v.unwrap_or(0)));
    let ha = ranked(drain((&ps).filt(|(n, _): (i64, i64)| n > 10)), |&(_, (_, w))| Reverse(w), false);
    let ha = rel(ha);
    type H = ((Id<User>, (i64, i64)), i64);
    let v = drain((&ha).select(Same::<H>::new().and(Same::<H>::new().map(|x: H| x.0.0).select(display_name).select((&hidx).opt()))));
    let bc = badges_per_user(db);
    rows(v.into_iter().map(|(_, (((u, (n, _)), k), r))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(bc.get(u).unwrap()), V::I(n), V::I(k), oint(r)]);
        row(f)
    }))
}

// WITH RECURSIVE RecursivePosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.LastActivityDate, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p2.Id AS PostId, p2.Title, p2.OwnerUserId, p2.LastActivityDate, rp.Level + 1 FROM Posts p2 INNER JOIN RecursivePosts rp ON p2.ParentId = rp.PostId WHERE p2.PostTypeId = 2),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        AVG(CASE WHEN v.VoteTypeId = 2 THEN 1.0 ELSE 0 END) AS UpvoteRatio
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName)
// SELECT u.DisplayName, ua.PostsCount, ua.TotalBounty, ua.UpvoteRatio, COUNT(DISTINCT rp.PostId) AS RelatedPostCount, MAX(rp.LastActivityDate) AS LastActivityDate
// FROM UserActivity ua JOIN Users u ON u.Id = ua.UserId LEFT JOIN RecursivePosts rp ON rp.OwnerUserId = u.Id
// WHERE ua.PostsCount > 0 GROUP BY u.DisplayName, ua.PostsCount, ua.TotalBounty, ua.UpvoteRatio HAVING COUNT(DISTINCT rp.PostId) > 0
// ORDER BY ua.UpvoteRatio DESC, LastActivityDate DESC;
//
// The groups are keyed by DisplayName, not user, so same-named users with equal stats merge; the ratio is keyed by its bits.
fn q33932(db: &'static So) -> String {
    let Post { post_type_id, owner_user, last_activity_date, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let rp = db.post.with(post_type_id.eq(1)).reach(answers_of(db), usize::MAX);
    type X = (Id<Post>, usize);
    let rpp = rel(drain(&rp)).map(|x: X| x.0);
    let byu: HashIdx<Id<User>, Id<Post>> = (&rpp).select(owner_user).inv().select(&rpp).collect();
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let ua = db.user.with(&pc)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(vote_type_id.and(bounty_amount.opt())).opt()))
        .fold((0i64, 0i64, 0i64), |(r, u, b), v| (r + 1, u + (v.map(|v| v.0) == Some(2)) as i64, b + v.and_then(|v| v.1).unwrap_or(0)));
    type K = (Str, i64, i64, u64);
    let key = Ident::<User>::new().and(&db.user.display_name).and(&pc).and(&ua).map(|(((_, n), c), (r, u, b)): (((Id<User>, Str), i64), (i64, i64, i64))| -> K {
        (n, c, b, (u as f64 / r as f64).to_bits())
    });
    let g = db.user.with(&pc).group_by(key).select((&byu).select(Ident::<Post>::new().and(last_activity_date))).buf_fold(|v| {
        let mut ps: Vec<Id<Post>> = v.iter().map(|x| x.0).collect();
        ps.sort_unstable();
        ps.dedup();
        (ps.len() as i64, v.iter().map(|x| x.1).max().unwrap())
    });
    let v = drain(&g);
    rows(v.into_iter().map(|((n, c, b, r), (k, la))| row(vec![V::S(n), V::I(c), V::I(b), V::F(f64::from_bits(r)), V::I(k), V::T(la)])))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.OwnerDisplayName, p.CreationDate, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT a.Id AS PostId, a.Title, a.PostTypeId, a.OwnerDisplayName, a.CreationDate, ph.Level + 1 FROM Posts a INNER JOIN Posts q ON a.ParentId = q.Id
//     INNER JOIN PostHierarchy ph ON q.Id = ph.PostId)
// SELECT p.Id AS QuestionId, p.Title AS QuestionTitle, p.OwnerDisplayName AS QuestionOwner, ph.PostId AS AnswerId, ph.Title AS AnswerTitle, ph.OwnerDisplayName AS AnswerOwner,
//        ph.Level AS AnswerLevel, pv.TotalVotes, uv.Reputation AS UserReputation, ba.BadgeCount,
//        CASE WHEN ph.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN 'Old Answer' ELSE 'New Answer' END AS AnswerAgeCategory
// FROM PostHierarchy ph JOIN Posts p ON ph.PostId = p.AcceptedAnswerId
// LEFT JOIN (SELECT PostId, COUNT(*) AS TotalVotes FROM Votes WHERE VoteTypeId IN (2, 3) GROUP BY PostId) pv ON pv.PostId = ph.PostId
// JOIN Users uv ON ph.OwnerDisplayName = uv.DisplayName
// LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) ba ON ba.UserId = uv.Id
// WHERE ph.PostTypeId = 2 ORDER BY p.CreationDate DESC, ph.Level DESC;
fn q32939(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, owner_display_name, creation_date, .. } = &db.post;
    let ph = db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX);
    type X = (Id<Post>, usize);
    let phv = rel(drain(&ph));
    let acc: HashIdx<Id<Post>, Id<Post>> = accepted_answer.inv().collect();
    let byname: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let pv = db.vote.with((&db.vote.vote_type_id).is_in([2, 3])).group_by(&db.vote.post).fold(0i64, |n, _| n + 1);
    let ba = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let a = || Same::<X>::new().map(|x: X| x.0);
    let q = (&phv)
        .with(a().select(post_type_id.eq(2)))
        .select(Same::<X>::new().and(a().select(&acc)).and(a().select((&pv).opt())).and(a().select(owner_display_name).select(&byname).select(Ident::<User>::new().and((&ba).opt()))));
    let old = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    rows(drain(q).into_iter().map(|(_, ((((a, l), p), n), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner_name"]);
        f.extend(post_fields(db, a, &["id", "title", "owner_name"]));
        f.extend([V::I(l as i64 + 1), oint(n), V::I(db.user.reputation.get(u).unwrap()), oint(b)]);
        f.push(V::S(if creation_date.get(a).unwrap() < old { "Old Answer" } else { "New Answer" }));
        row(f)
    }))
}

// WITH RECURSIVE PopularPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, p.ViewCount, 1 AS Level, p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 10
//     UNION ALL SELECT p.Id, p.Title, p.Score, p.CreationDate, p.ViewCount, pp.Level + 1, p.OwnerUserId FROM Posts p INNER JOIN PopularPosts pp ON p.AcceptedAnswerId = pp.Id
//     WHERE p.PostTypeId = 2),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName)
// SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalBounties, ua.TotalUpVotes, ua.TotalDownVotes, pp.Title AS PopularPostTitle, pp.ViewCount AS PopularPostViews,
//        pp.Score AS PopularPostScore, pp.CreationDate AS PopularPostDate,
//        CASE WHEN ua.TotalPosts > 0 THEN ROUND((ua.TotalUpVotes * 1.0 / NULLIF(ua.TotalPosts, 0)), 2) ELSE 0 END AS UpvoteRatio,
//        CASE WHEN ua.TotalPosts > 0 THEN ROUND((ua.TotalDownVotes * 1.0 / NULLIF(ua.TotalPosts, 0)), 2) ELSE 0 END AS DownvoteRatio
// FROM UserActivity ua LEFT JOIN PopularPosts pp ON ua.UserId = pp.OwnerUserId ORDER BY ua.TotalPosts DESC, pp.Score DESC LIMIT 10;
fn q32343(db: &'static So) -> String {
    let Post { post_type_id, score, accepted_answer, owner_user, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let st: HashIdx<Id<Post>, Id<Post>> = db.post.with(post_type_id.eq(2)).select(accepted_answer).inv().collect();
    let pp = db.post.with(post_type_id.eq(1)).with(score.gt(10)).reach(&st, usize::MAX);
    type X = (Id<Post>, usize);
    let ppv = rel(drain(&pp)).map(|x: X| x.0);
    let byu: HashIdx<Id<User>, Id<Post>> = (&ppv).select(owner_user).inv().select(&ppv).collect();
    let pc = user_distinct_posts(db);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(vote_type_id.and(bounty_amount.opt())).opt()).opt())
        .fold([0i64; 3], |a, v| match v.flatten() {
            Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
            None => a,
        });
    let v = drain(db.user.select(Ident::<User>::new().and(&pc).and(&ua).and((&byu).select(Ident::<Post>::new().and(score)).opt())));
    let v = top_n(v, |&(_, (((u, n), _), p))| (Reverse(n), p.is_none(), Reverse(p.map(|p| p.1)), u, p.map(|p| p.0)), 10);
    let r = |x: i64, n: i64| if n > 0 { V::F(((x as f64 / n as f64) * 100.0).round() / 100.0) } else { V::F(0.0) };
    rows(v.into_iter().map(|(_, (((u, n), a), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        match p {
            Some((p, _)) => f.extend(post_fields(db, p, &["title", "views", "score", "created"])),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        f.extend([r(a[1], n), r(a[2], n)]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.ParentId, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.Title, p.ParentId, h.Level + 1 FROM Posts p INNER JOIN PostHierarchy h ON p.ParentId = h.PostId),
// UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id, p.Title, p.ViewCount, COALESCE(p.AnswerCount, 0) AS AnswerCount, COALESCE(p.CommentCount, 0) AS CommentCount, p.CreationDate,
//        COALESCE(ph.Level, 0) AS HierarchyLevel FROM Posts p LEFT JOIN PostHierarchy ph ON p.Id = ph.PostId WHERE p.LastActivityDate >= CURRENT_DATE - INTERVAL '3 months'),
// VoteSummary AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId)
// SELECT u.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ps.Title, ps.ViewCount, ps.AnswerCount, ps.CommentCount, ps.CreationDate,
//        COALESCE(vs.UpVotes, 0) AS TotalUpVotes, COALESCE(vs.DownVotes, 0) AS TotalDownVotes, ps.HierarchyLevel
// FROM Users u JOIN UserBadges ub ON u.Id = ub.UserId JOIN Posts p ON u.Id = p.OwnerUserId JOIN PostStats ps ON p.Id = ps.Id LEFT JOIN VoteSummary vs ON p.Id = vs.PostId
// WHERE ub.BadgeCount > 0 ORDER BY TotalUpVotes DESC, ps.ViewCount DESC;
fn q31874(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, answer_count, .. } = &db.post;
    let phi: HashIdx<Id<Post>, usize> = db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| {
        [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]
    });
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cut = add_months(current_date(), -3);
    let v = drain(db.user.select(Ident::<User>::new().and(&ub).and(posts_of(db).with(last_activity_date.ge(cut)).select(Ident::<Post>::new().and((&phi).opt()).and((&vs).opt())))));
    rows(v.into_iter().map(|(_, ((u, b), ((p, l), s)))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend(b.iter().map(|&x| V::I(x)));
        f.extend(post_fields(db, p, &["title", "views"]));
        f.push(V::I(answer_count.get(p).unwrap_or(0)));
        f.extend(post_fields(db, p, &["comments", "created"]));
        let s = s.unwrap_or([0; 2]);
        f.extend([V::I(s[0]), V::I(s[1]), V::I(l.map_or(0, |l| l as i64 + 1))]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title AS PostTitle, p.ParentId, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id AS PostId, p.Title AS PostTitle, p.ParentId, ph.Level + 1 AS Level FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId)
// SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, MAX(ph.Level) AS MaxAnswerDepth,
//        STRING_AGG(DISTINCT t.TagName, ', ') AS TagsUsed, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COUNT(DISTINCT v.Id) AS TotalVotes
// FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostHierarchy ph ON p.Id = ph.PostId LEFT JOIN PostLinks pl ON p.Id = pl.PostId
//      LEFT JOIN Tags t ON t.Id = pl.RelatedPostId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 0 AND COUNT(DISTINCT v.Id) > 0 ORDER BY TotalPosts DESC, TotalAnswers DESC LIMIT 10;
//
// The two SUMs are over the whole product; the distinct counts and the tag list are folded separately. Tags are joined in name order.
fn q34876(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let phi: HashIdx<Id<Post>, usize> = db.post.with(post_type_id.eq(1)).reach(children_of(db), usize::MAX).collect();
    let tags: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let pc = user_distinct_posts(db);
    let vc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db))).fold(0i64, |n, _| n + 1);
    let tg = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(links_of(db)).select(&db.post_link.related_post_id).select(&tags).select(&db.tag.tag_name)).buf_fold(|v| {
        let mut v: Vec<Str> = v.to_vec();
        v.sort();
        v.dedup();
        leak(v.join(", "))
    });
    let agg = db
        .user
        .with((&pc).gt(0))
        .with(&vc)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and((&phi).opt()).and(links_of(db).select((&db.post_link.related_post_id).select(&tags).opt()).opt()).and(votes_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold((0i64, 0i64, None::<usize>), |(a, g, m), (p, b)| {
            let g = g + (b == Some(1)) as i64;
            match p {
                Some((((t, l), _), _)) => (a + (t == 2) as i64, g, m.max(l)),
                None => (a, g, m),
            }
        });
    let v = drain(db.user.select(Ident::<User>::new().and(&pc).and(&agg).and(&vc).and((&tg).opt())));
    let v = top_n(v, |&(_, ((((u, n), (a, _, _)), _), _))| (Reverse(n), Reverse(a), u), 10);
    rows(v.into_iter().map(|(_, ((((u, n), (a, g, m)), c), t))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a), oint(m.map(|m| m as i64 + 1)), ostr(t), V::I(g), V::I(c)]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.ParentId, p.Title, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.ParentId, ph.Title, ph.Level + 1 FROM Posts p JOIN PostHierarchy ph ON p.ParentId = ph.PostId WHERE p.PostTypeId = 2),
// UserScore AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COALESCE(SUM(CASE WHEN v.CreationDate IS NOT NULL THEN 1 ELSE 0 END), 0) AS VoteCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// MostActiveUsers AS (SELECT OwnerUserId AS UserId, COUNT(*) AS PostCount FROM Posts GROUP BY OwnerUserId HAVING COUNT(*) > 10),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ph.Level, u.DisplayName AS OwnerDisplayName, UserScore.TotalBounties, UserScore.VoteCount, p.OwnerUserId -- Added to GROUP BY
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN PostHierarchy ph ON p.Id = ph.PostId LEFT JOIN UserScore ON u.Id = UserScore.UserId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, ph.Level, u.DisplayName, UserScore.TotalBounties, UserScore.VoteCount, p.OwnerUserId)
// SELECT pd.PostId, pd.Title, pd.UpVotes, pd.DownVotes, pd.Level, pd.OwnerDisplayName, pd.TotalBounties, pd.VoteCount,
//        CASE WHEN pd.UpVotes - pd.DownVotes > 0 THEN 'Positive' WHEN pd.UpVotes - pd.DownVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS Sentiment
// FROM PostDetails pd JOIN MostActiveUsers mau ON pd.OwnerUserId = mau.UserId ORDER BY pd.UpVotes - pd.DownVotes DESC, pd.PostId;
fn q31150(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let phi: HashIdx<Id<Post>, usize> = db.post.with(post_type_id.eq(1)).reach(answers_of(db), usize::MAX).collect();
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold((0i64, 0i64), |(s, n), b| match b {
        Some(b) => (s + b.unwrap_or(0), n + 1),
        None => (s, n),
    });
    let pc = user_distinct_posts(db);
    let pd = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user.select((&pc).gt(10)))
        .group_by(Ident::<Post>::new().and((&phi).opt()))
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    type K = (Id<Post>, Option<usize>);
    let v = drain((&pd).and(Same::<K>::new().map(|k: K| k.0).select(owner_user).select(Ident::<User>::new().and(&us))));
    rows(v.into_iter().map(|((p, l), (a, (u, (s, n))))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), oint(l.map(|l| l as i64 + 1))]);
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(s), V::I(n)]);
        let d = a[0] - a[1];
        f.push(V::S(if d > 0 { "Positive" } else if d < 0 { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT Id, Title, ParentId, CreationDate, Score, 0 AS Level FROM Posts WHERE ParentId IS NULL
//     UNION ALL SELECT p.Id, p.Title, p.ParentId, p.CreationDate, p.Score, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.Id),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostsCount, SUM(CASE WHEN p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days') THEN 1 ELSE 0 END) AS RecentPostsCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id, p.Title, COALESCE(com.CommentCount, 0) AS CommentCount, COALESCE(vote.UpVotesCount, 0) AS UpVotesCount, COALESCE(vote.DownVotesCount, 0) AS DownVotesCount,
//        ph.Level, p.OwnerUserId FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) com ON p.Id = com.PostId
//     LEFT JOIN (SELECT p.Id, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//         FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id) vote ON p.Id = vote.Id
//     LEFT JOIN PostHierarchy ph ON p.Id = ph.Id)
// SELECT ua.DisplayName, SUM(ps.CommentCount) AS TotalComments, SUM(ps.UpVotesCount) AS TotalUpVotes, SUM(ps.DownVotesCount) AS TotalDownVotes,
//        COUNT(DISTINCT ps.Id) AS PostsParticipated, COUNT(DISTINCT ph.ParentId) FILTER (WHERE ph.Level = 1) AS DirectAnswers
// FROM UserActivity ua LEFT JOIN PostStats ps ON ua.UserId = ps.OwnerUserId LEFT JOIN PostHierarchy ph ON ps.Id = ph.Id
// GROUP BY ua.UserId, ua.DisplayName HAVING COUNT(DISTINCT ps.Id) > 0 ORDER BY TotalUpVotes DESC;
//
// UserActivity is one row per user and only its name is read. The sums run over the PostStats x PostHierarchy rows; the distinct counts are folded separately.
fn q33230(db: &'static So) -> String {
    let Post { parent_id, .. } = &db.post;
    let phi: HashIdx<Id<Post>, usize> = db.post.minus(parent_id).reach(children_of(db), usize::MAX).collect();
    let cc = comments_per_post(db);
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pc = user_distinct_posts(db);
    let sums = db
        .user
        .with((&pc).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&cc).and((&pv).opt()).and((&phi).opt()).and((&phi).opt())))
        .fold([0i64; 3], |a, (((c, v), _), _)| {
            let v = v.unwrap_or([0; 2]);
            [a[0] + c, a[1] + v[0], a[2] + v[1]]
        });
    let da = db.user.group_by(Ident::<User>::new()).select(posts_of(db).with((&phi).eq(1)).select(parent_id)).count_distinct();
    let v = drain(db.user.select(Ident::<User>::new().and(&sums).and(&pc).and((&da).opt())));
    rows(v.into_iter().map(|(_, (((u, s), n), d))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(s[0]), V::I(s[1]), V::I(s[2]), V::I(n), V::I(d.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.ParentId, p.Title, p.CreationDate, 1 AS Level FROM Posts p WHERE p.ParentId IS NULL
//     UNION ALL SELECT p.Id AS PostId, p.ParentId, p.Title, p.CreationDate, ph.Level + 1 AS Level FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 GROUP BY u.Id, u.Reputation, u.DisplayName),
// TopTags AS (SELECT LOWER(TRIM(t.TagName)) AS TagName, COUNT(DISTINCT p.Id) AS TagCount FROM Tags t INNER JOIN Posts p ON t.Id = p.Id GROUP BY LOWER(TRIM(t.TagName)) HAVING COUNT(DISTINCT p.Id) > 5),
// AggregatedHistory AS (SELECT ph.PostId, COUNT(DISTINCT ph.PostId) AS EditCount, MAX(ph.CreationDate) AS LastEditDate, STRING_AGG(DISTINCT pht.Name, ', ') AS PostHistoryTypes
//     FROM PostHistory h JOIN PostHistoryTypes pht ON h.PostHistoryTypeId = pht.Id JOIN Posts p ON p.Id = h.PostId JOIN PostHierarchy ph ON ph.PostId = p.Id GROUP BY ph.PostId),
// CombinedData AS (SELECT u.Id AS UserId, u.DisplayName, ur.Reputation, ur.PostCount, ur.TotalBounty, oh.EditCount, oh.LastEditDate, oh.PostHistoryTypes, tt.TagCount
//     FROM UserReputation ur JOIN Users u ON u.Id = ur.UserId LEFT JOIN AggregatedHistory oh ON ur.PostCount > 0 LEFT JOIN TopTags tt ON tt.TagName IS NOT NULL)
// SELECT c.UserId, c.DisplayName, c.Reputation, COALESCE(c.TotalBounty, 0) AS TotalBounty, COALESCE(c.EditCount, 0) AS EditCount, COALESCE(c.LastEditDate, '1970-01-01') AS LastEditDate,
//        COALESCE(c.PostHistoryTypes, 'None') AS PostHistoryTypes, COALESCE(c.TagCount, 0) AS TagCount
// FROM CombinedData c WHERE c.Reputation > 100 AND COALESCE(c.EditCount, 0) >= 5 ORDER BY c.Reputation DESC, c.PostCount DESC LIMIT 100;
//
// `COALESCE(EditCount, 0) >= 5` rejects the unmatched LEFT JOIN rows, so the AggregatedHistory join is an inner join on its filtered rows.
fn q24907(db: &'static So) -> String {
    let Post { parent_id, creation_date, .. } = &db.post;
    let phi: HashIdx<Id<Post>, usize> = db.post.minus(parent_id).reach(children_of(db), usize::MAX).collect();
    let pc = user_distinct_posts(db);
    let tb = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).with((&db.vote.vote_type_id).eq(9)).select((&db.vote.bounty_amount).opt()).opt()).opt())
        .fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let byid: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let tt = db
        .tag
        .group_by((&db.tag.tag_name).map(|n: Str| leak(n.trim().to_lowercase())))
        .select((&db.tag.origid).select(&byid))
        .count_distinct();
    let tt = left_all(drain((&tt).filt(|n: i64| n > 5)));
    let oh = db.post.with(&phi).group_by(Ident::<Post>::new()).select(Ident::<Post>::new().and(&phi).and(creation_date).and(history_of(db).select(htype_name(db)))).buf_fold(|v| {
        let mut ps: Vec<Id<Post>> = v.iter().map(|x| ((x.0).0).0).collect();
        ps.sort_unstable();
        ps.dedup();
        let mut ns: Vec<Str> = v.iter().map(|x| x.1).collect();
        ns.sort();
        ns.dedup();
        (ps.len() as i64, v.iter().map(|x| (x.0).1).max().unwrap(), leak(ns.join(", ")))
    });
    let oh5 = rel(drain((&oh).filt(|(e, _, _): (i64, i64, Str)| e >= 5)));
    let v = drain(db.user.with((&db.user.reputation).gt(100)).with((&pc).gt(0)).select(Ident::<User>::new().and(&pc).and(&tb)).cross(&oh5).cross(&tt));
    let v = top_n(v, |&(_, ((((u, n), _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u), 100);
    rows(v.into_iter().map(|(_, ((((u, _), b), (_, (e, d, h))), t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(e), V::T(d), V::S(h), V::I(t.map_or(0, |t| t.1))]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT Id, ParentId, Title, CreationDate, 0 AS Level FROM Posts WHERE ParentId IS NULL
//     UNION ALL SELECT p.Id, p.ParentId, p.Title, p.CreationDate, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.Id),
// RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        (SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END)) AS Score
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS TotalBadges, pp.Title, pp.CreationDate, pp.ViewCount, pp.CommentCount, pp.UpVotes AS TotalUpVotes,
//        pp.DownVotes AS TotalDownVotes, pp.Score, ph.Level
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId JOIN RankedPosts pp ON u.Id = pp.OwnerUserId LEFT JOIN PostHierarchy ph ON pp.Id = ph.Id
// WHERE pp.PostRank = 1 AND pp.ViewCount > 100 ORDER BY pp.Score DESC, pp.ViewCount DESC LIMIT 50;
//
// The newest post of each owner is picked first, and the comment x vote product is driven only for those.
fn q30100(db: &'static So) -> String {
    let Post { parent_id, owner_user, creation_date, view_count, .. } = &db.post;
    let phi: HashIdx<Id<Post>, usize> = db.post.minus(parent_id).reach(children_of(db), usize::MAX).collect();
    let w = db.post.group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).with(view_count.gt(100)).collect();
    let pp = (&top).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&pp).and(owner_user.select(Ident::<User>::new().and(&ub))).and((&phi).opt()));
    let v = top_n(v, |&(p, ((a, _), _))| (Reverse(a[1] - a[2]), Reverse(view_count.get(p)), p), 50);
    rows(v.into_iter().map(|(p, ((a, (u, b)), l))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2]), oint(l.map(|l| l as i64))]);
        row(f)
    }))
}

// WITH RECURSIVE TagHierarchy AS (SELECT Id, TagName, Count, ExcerptPostId, WikiPostId, 1 AS Level FROM Tags WHERE Count > 0
//     UNION ALL SELECT t.Id, t.TagName, t.Count, t.ExcerptPostId, t.WikiPostId, th.Level + 1 FROM Tags t JOIN TagHierarchy th ON t.Id = th.Id + 1),
// UserVotes AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.VoteTypeId) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, pt.Name AS PostType,
//        COALESCE(ph.Comment, 'No comments') AS LastChangeComment, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS ChangeRank
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= (CAST('2024-10-01' AS DATE) - INTERVAL '1 year')),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.OwnerUserId, pd.ViewCount, pd.Score, pd.PostType, COUNT(c.Id) AS CommentCount FROM PostDetails pd LEFT JOIN Comments c ON pd.PostId = c.PostId
//     GROUP BY pd.PostId, pd.Title, pd.OwnerUserId, pd.ViewCount, pd.Score, pd.PostType HAVING COUNT(c.Id) > 5)
// SELECT tp.Title, tp.ViewCount, tp.Score, u.DisplayName AS Owner, uv.VoteCount, uv.Upvotes, uv.Downvotes, th.TagName AS TopTag
// FROM TopPosts tp INNER JOIN Users u ON tp.OwnerUserId = u.Id LEFT JOIN UserVotes uv ON u.Id = uv.UserId LEFT JOIN TagHierarchy th ON tp.PostId = th.Id
// ORDER BY tp.Score DESC, tp.ViewCount DESC LIMIT 10;
//
// The recursive step is the tag whose Id is one more; TagHierarchy is joined on the raw ids, so a post meets the tag that happens to share its Id.
fn q30466(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, origid, .. } = &db.post;
    let Tag { origid: tid, count, .. } = &db.tag;
    let tags: HashIdx<i64, Id<Tag>> = tid.inv().collect();
    let th = db.tag.with(count.gt(0)).reach(tid.map(|i: i64| i + 1).select(&tags), usize::MAX);
    type X = (Id<Tag>, usize);
    let thv = rel(drain(&th)).map(|x: X| x.0);
    let byid: HashIdx<i64, Id<Tag>> = (&thv).select(tid).inv().select(&thv).collect();
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let tp = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(Ident::<Post>::new())
        .select(history_of(db).opt().and(comments_of(db).opt()))
        .fold(0i64, |n, (_, c)| n + c.is_some() as i64);
    let v = drain((&tp).filt(|n: i64| n > 5).map(|_| ()).and(owner_user.select(Ident::<User>::new().and(&uv))).and(origid.select(&byid).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p)), Reverse(view_count.get(p)), p), 10);
    rows(v.into_iter().map(|(p, ((_, (u, a)), t))| {
        let mut f = post_fields(db, p, &["title", "views", "score"]);
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), ostr(t.map(|t| db.tag.tag_name.get(t).unwrap()))]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT Id, Reputation, CreationDate, LastAccessDate, DisplayName, 0 AS Level FROM Users WHERE Reputation IS NOT NULL
//     UNION ALL SELECT U.Id, U.Reputation, U.CreationDate, U.LastAccessDate, U.DisplayName, UR.Level + 1 FROM Users U JOIN UserReputation UR ON U.Id = UR.Id WHERE UR.Level < 5),
// PostActivity AS (SELECT P.Id AS PostId, P.OwnerUserId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(V.Id) AS VoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, MAX(P.CreationDate) AS LastActivityDate
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.OwnerUserId),
// TopPosters AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.AnswerCount, 0)) AS TotalAnswers,
//        SUM(COALESCE(P.CommentCount, 0)) AS TotalComments, ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT P.Id) DESC) AS RN
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 100 GROUP BY U.Id, U.DisplayName)
// SELECT UR.DisplayName AS UserDisplayName, UR.Reputation, PP.TotalPosts, PP.TotalViews, PP.TotalAnswers, PP.TotalComments, PP.RN
// FROM TopPosters PP JOIN UserReputation UR ON PP.UserId = UR.Id WHERE PP.RN <= 10 ORDER BY UR.Reputation DESC, PP.TotalPosts DESC;
//
// PostActivity is never read. The recursion repeats each user at levels 0-5, so each top poster appears six times.
fn q31296(db: &'static So) -> String {
    let Post { view_count, answer_count, comment_count, .. } = &db.post;
    let ur: HashIdx<Id<User>, usize> = db.user.reach(Same::<Id<User>>::new(), 5).collect();
    let tp = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(answer_count.opt()).and(comment_count)).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((w, n), c)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + n.unwrap_or(0), a[3] + c],
            None => a,
        });
    let top = top_n(drain(&tp), |&(u, a)| (Reverse(a[0]), u), 10);
    let top = rel(top.into_iter().enumerate().map(|(i, (u, a))| (u, a, i as i64 + 1)).collect());
    type T = (Id<User>, [i64; 4], i64);
    let v = drain((&top).select(Same::<T>::new().and(Same::<T>::new().map(|t: T| t.0).select(&ur))));
    rows(v.into_iter().map(|(_, ((u, a, k), _))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(k)]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT P.Id AS PostId, P.ParentId, P.Title, P.OwnerUserId, 0 AS Level FROM Posts P WHERE P.ParentId IS NULL
//     UNION ALL SELECT P.Id, P.ParentId, P.Title, P.OwnerUserId, PH.Level + 1 FROM Posts P JOIN PostHierarchy PH ON P.ParentId = PH.PostId),
// UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostStats AS (SELECT PH.PostId, PH.Title, U.DisplayName AS Owner, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY PH.OwnerUserId ORDER BY COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) DESC) AS Rank
//     FROM PostHierarchy PH LEFT JOIN Comments C ON PH.PostId = C.PostId LEFT JOIN Votes V ON PH.PostId = V.PostId LEFT JOIN Users U ON PH.OwnerUserId = U.Id
//     GROUP BY PH.PostId, PH.Title, PH.OwnerUserId, U.DisplayName),
// CombinedStats AS (SELECT PS.PostId, PS.Title, PS.Owner, PS.CommentCount, PS.UpVotes, PS.DownVotes, UR.Reputation AS OwnerReputation, UR.PostCount AS OwnerPostCount,
//        UR.TotalBounty AS OwnerTotalBounty, CASE WHEN PS.UpVotes > PS.DownVotes THEN 'Positive' WHEN PS.UpVotes < PS.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
//     FROM PostStats PS JOIN UserReputation UR ON PS.Owner = UR.DisplayName)
// SELECT CS.PostId, CS.Title, CS.Owner, CS.CommentCount, CS.UpVotes, CS.DownVotes, CS.OwnerReputation, CS.OwnerPostCount, CS.OwnerTotalBounty, CS.VoteSentiment
// FROM CombinedStats CS WHERE CS.OwnerReputation > 1000 ORDER BY CS.OwnerReputation DESC, CS.CommentCount DESC;
//
// Rank is never read. PostStats groups every PostHierarchy row of a post together, and its owner is matched to UserReputation by name.
fn q30668(db: &'static So) -> String {
    let Post { parent_id, owner_user, .. } = &db.post;
    let ph = db.post.minus(parent_id).reach(children_of(db), usize::MAX);
    type X = (Id<Post>, usize);
    let phv = rel(drain(&ph)).map(|x: X| x.0);
    let ps = (&phv).group_by(Same::<Id<Post>>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let pc = user_distinct_posts(db);
    let tb = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).opt())
        .fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let byname: HashIdx<Str, Id<User>> = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.display_name).inv().collect();
    let v = drain((&ps).and(owner_user.select(&db.user.display_name).select(&byname).select(Ident::<User>::new().and(&pc).and(&tb))));
    rows(v.into_iter().map(|(p, (a, ((u, n), b)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(ucols(db, u, &["rep"]));
        f.extend([V::I(n), V::I(b)]);
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RECURSIVE UserReputationCTE AS (SELECT U.Id AS UserId, U.Reputation, 1 AS Level FROM Users U WHERE U.Reputation > 1000
//     UNION ALL SELECT UR.UserId, UR.Reputation + U.Reputation, Level + 1 FROM UserReputationCTE UR JOIN Users U ON UR.UserId = U.Id WHERE UR.Reputation + U.Reputation < 5000),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.Body, U.DisplayName AS OwnerDisplayName, P.CreationDate, COALESCE(P.AcceptedAnswerId, 0) AS AcceptedAnswerId, P.Tags, P.Score,
//        COALESCE(COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END), 0) AS UpVotes, COALESCE(COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END), 0) AS DownVotes,
//        COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount, RANK() OVER (ORDER BY P.CreationDate DESC) AS Rank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.CreationDate > (CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 YEAR') GROUP BY P.Id, U.DisplayName, P.Title, P.Body, P.CreationDate, P.AcceptedAnswerId, P.Tags, P.Score),
// RecentPostHistory AS (SELECT PH.PostId, PH.PostHistoryTypeId, PH.CreationDate, PH.UserDisplayName, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS rn
//     FROM PostHistory PH WHERE PH.CreationDate > (CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '30 DAY'))
// SELECT U.Id AS UserId, U.DisplayName, UR.Reputation, PD.PostId, PD.Title, PD.Score, PD.UpVotes, PD.DownVotes, PD.CommentCount, PD.Rank, K.CreationDate AS LastPostDate, PH.UserDisplayName AS LastEditor
// FROM Users U JOIN UserReputationCTE UR ON U.Id = UR.UserId JOIN PostDetails PD ON U.DisplayName = PD.OwnerDisplayName
// LEFT JOIN (SELECT DISTINCT PH.PostId, PH.CreationDate FROM PostHistory PH ORDER BY PH.PostId, PH.CreationDate DESC) K ON PD.PostId = K.PostId
// LEFT JOIN RecentPostHistory PH ON PD.PostId = PH.PostId AND PH.rn = 1
// WHERE UR.Reputation BETWEEN 1000 AND 5000 ORDER BY UR.Reputation DESC, PD.Score DESC;
//
// The recursion carries the accumulated reputation in the node. A tie for the latest history row goes to the smaller history id (the SQL leaves it open).
fn q31396(db: &'static So) -> String {
    let User { reputation, display_name, .. } = &db.user;
    let Post { creation_date, owner_user, .. } = &db.post;
    type N = (Id<User>, i64);
    let step = Same::<N>::new().and(Same::<N>::new().map(|n: N| n.0).select(reputation)).map(|((u, r), x): (N, i64)| (u, r + x)).filt(|(_, r): N| r < 5000);
    let ur = rel(drain(db.user.with(reputation.gt(1000)).select(Ident::<User>::new().and(reputation)).reach(step, usize::MAX))).map(|x: (N, usize)| x.0);
    let rp = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rk = ranked(drain(&rp).into_iter().map(|x| x.0).collect(), |&p| Reverse(creation_date.get(p).unwrap()), false);
    let rk = rel(rk);
    let rkx: HashIdx<Id<Post>, i64> = (&rk).map(|x: (Id<Post>, i64)| x.0).inv().select((&rk).map(|x: (Id<Post>, i64)| x.1)).collect();
    let pd = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 3], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]
    });
    let byname: HashIdx<Str, Id<Post>> = (&rp).select(owner_user).select(display_name).inv().collect();
    let PostHistory { post, creation_date: hd, user_display_name, .. } = &db.post_history;
    let k: MatSet<(Id<Post>, i64)> = db.post_history.select(post.and(hd)).collect();
    let k: HashIdx<Id<Post>, i64> = (&k).map(|x: (Id<Post>, i64)| x.0).inv().select((&k).map(|x: (Id<Post>, i64)| x.1)).collect();
    let w = db.post_history.with(hd.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d)| (Reverse(d), h), asc);
    let last: HashIdx<Id<Post>, Id<PostHistory>> = (&w).filt(|(_, n): ((Id<PostHistory>, i64), i64)| n == 1).map(|((h, _), _)| h).collect();
    let v = drain(
        (&ur)
            .with(Same::<N>::new().map(|n: N| n.1).filt(|r: i64| (1000..=5000).contains(&r)))
            .select(Same::<N>::new().and(Same::<N>::new().map(|n: N| n.0).select(display_name).select(&byname).select(Ident::<Post>::new().and(&pd).and(&rkx).and((&k).opt()).and((&last).opt())))),
    );
    let v = top_n(v, |&(_, ((u, r), ((((p, _), _), _), _)))| (Reverse(r), Reverse(db.post.score.get(p).unwrap()), u, p), 0);
    rows(v.into_iter().map(|(_, ((u, r), ((((p, a), n), d), h)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(r));
        f.extend(post_fields(db, p, &["id", "title", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n), ots(d), ostr(h.and_then(|h| user_display_name.get(h)))]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("30183", q30183),
    ("32626", q32626),
    ("33009", q33009),
    ("33934", q33934),
    ("31185", q31185),
    ("31229", q31229),
    ("32329", q32329),
    ("33993", q33993),
    ("32732", q32732),
    ("34678", q34678),
    ("34443", q34443),
    ("30881", q30881),
    ("32893", q32893),
    ("32009", q32009),
    ("32489", q32489),
    ("34520", q34520),
    ("32106", q32106),
    ("32433", q32433),
    ("33289", q33289),
    ("33673", q33673),
    ("31832", q31832),
    ("30603", q30603),
    ("34888", q34888),
    ("31948", q31948),
    ("32928", q32928),
    ("30608", q30608),
    ("33648", q33648),
    ("30244", q30244),
    ("31240", q31240),
    ("31932", q31932),
    ("34204", q34204),
    ("33357", q33357),
    ("34077", q34077),
    ("34515", q34515),
    ("31107", q31107),
    ("30455", q30455),
    ("31020", q31020),
    ("30831", q30831),
    ("30985", q30985),
    ("31915", q31915),
    ("33470", q33470),
    ("31162", q31162),
    ("32809", q32809),
    ("34789", q34789),
    ("30788", q30788),
    ("30583", q30583),
    ("34568", q34568),
    ("33742", q33742),
    ("31584", q31584),
    ("30795", q30795),
    ("34809", q34809),
    ("34195", q34195),
    ("33932", q33932),
    ("32939", q32939),
    ("32343", q32343),
    ("31874", q31874),
    ("34876", q34876),
    ("31150", q31150),
    ("33230", q33230),
    ("24907", q24907),
    ("30100", q30100),
    ("30466", q30466),
    ("31296", q31296),
    ("30668", q30668),
    ("31396", q31396),
];
