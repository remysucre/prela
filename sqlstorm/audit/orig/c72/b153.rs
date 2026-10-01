use harness::prelude::*;
use std::cmp::Reverse;

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC, COUNT(c.Id) DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT fp.PostId, fp.Title, fp.OwnerDisplayName, fp.CommentCount, fp.UpVotes, fp.DownVotes, (fp.UpVotes - fp.DownVotes) AS NetVotes
// FROM FilteredPosts fp ORDER BY NetVotes DESC, fp.CommentCount DESC;
fn q8256(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_per(drain(&s), |&(p, _)| post_type_id.get(p).unwrap(), |&(p, a)| (Reverse(a[1]), Reverse(a[0]), p), 10, false);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT c.Id) AS CommentCount, ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT a.Id) DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, u.DisplayName),
// ActiveBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId)
// SELECT rp.PostId, rp.Title, rp.Body, rp.OwnerDisplayName, rp.AnswerCount, rp.UpVotes, rp.DownVotes, rb.BadgeCount AS GoldBadgeCount, rp.CommentCount
// FROM RankedPosts rp LEFT JOIN ActiveBadges rb ON rp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = rb.UserId)
// WHERE rp.Rank <= 10 ORDER BY rp.Rank;
//
// Rank reads only COUNT(DISTINCT a.Id), the number of children, so the top ten questions are picked first and the answer x vote x comment product is driven for those alone.
fn q27973(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let ac = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let top = top_n(drain(&ac), |&(p, n)| (Reverse(n), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt()))
        .fold([0i64; 2], |a, ((_, t), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ab = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = db.user.with(&ab).select(&db.user.display_name).inv().collect();
    let v = drain((&ac).and(&s).and(&cc).and(owner_user.select(&db.user.display_name).select((&by_name).select(&ab)).opt()));
    rows(v.into_iter().map(|(p, (((n, a), c), b))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "owner"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), oint(b), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT a.Id) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
//     GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.ViewCount),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u)
// SELECT up.DisplayName, up.Reputation, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.AnswerCount, ur.ReputationRank
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId JOIN Users up ON rp.OwnerUserId = up.Id
// WHERE rp.PostRank <= 5 ORDER BY ur.ReputationRank, rp.CreationDate DESC;
fn q9561(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let an = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let v = drain((&cc).and(&an).and(owner_user.select(&by_user)));
    rows(v.into_iter().map(|(p, ((c, a), (u, r)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(a), V::I(r)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostsCreated, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersProvided FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS TotalBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// FinalComparison AS (SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.PostsCreated, TU.QuestionsAsked, TU.AnswersProvided, UB.TotalBadges, TU.ReputationRank
//     FROM TopUsers TU LEFT JOIN UserBadges UB ON TU.UserId = UB.UserId)
// SELECT DisplayName, Reputation, PostsCreated, QuestionsAsked, AnswersProvided, TotalBadges, ReputationRank FROM FinalComparison WHERE TotalBadges > 5 ORDER BY ReputationRank;
fn q5728(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let v = drain((&ups).and((&bc).filt(|b| b > 5)).and((&by_user).map(|(_, r)| r)));
    rows(v.into_iter().map(|(u, ((a, b), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.CreationDate, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS Upvotes,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS Downvotes, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, ROW_NUMBER() OVER (ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.PostTypeId = 1
//     GROUP BY P.Id, P.Title, P.ViewCount, P.CreationDate, U.DisplayName),
// TopPosts AS (SELECT RP.*, (Upvotes - Downvotes) AS Score, RANK() OVER (ORDER BY (Upvotes - Downvotes) DESC, ViewCount DESC) AS ScoreRank FROM RankedPosts RP)
// SELECT TP.PostId, TP.Title, TP.OwnerDisplayName, TP.ViewCount, TP.Upvotes, TP.Downvotes, TP.CommentCount, TP.Score, TP.ScoreRank,
//        CASE WHEN TP.ScoreRank <= 10 THEN 'Top 10 Post' ELSE 'Not Ranked Top 10' END AS PostCategory
// FROM TopPosts TP WHERE TP.PostRank <= 50 ORDER BY TP.Score DESC, TP.ViewCount DESC;
fn q8161(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let r = ranked(drain(&s), |&(p, a)| {
        let w = view_count.get(p);
        (Reverse(a[0] - a[1]), w.is_none(), Reverse(w))
    }, false);
    let rr = rel(r.into_iter().map(|((p, a), k)| (p, (a, k))).collect());
    let by_post: HashIdx<Id<Post>, ([i64; 3], i64)> = (&rr).map(|(p, _)| p).inv().select((&rr).map(|(_, x)| x)).collect();
    let newest = top_n(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(creation_date)), |&(p, d)| (Reverse(d), p), 50);
    let np: MatSet<Id<Post>> = rel(newest.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&np).select(&by_post));
    rows(v.into_iter().map(|(p, (a, k))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[0] - a[1]), V::I(k), V::S(if k <= 10 { "Top 10 Post" } else { "Not Ranked Top 10" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) AS VoteCount, DENSE_RANK() OVER (ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3)
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, CommentCount, VoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.VoteCount, COALESCE(pht.Name, 'No History') AS PostHistoryType,
//        ph.CreationDate AS HistoryDate
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only CreationDate, so the ten newest dates are picked first and the distinct counts are taken for those posts alone.
fn q9152(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let r = ranked(drain(db.post.with(owner_user).select(creation_date)), |&(_, d)| Reverse(d), true);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ud = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let vc = (&tp).group_by(Ident::<Post>::new()).select(ud.opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(&vc).and(history_of(db).opt()));
    rows(v.into_iter().map(|(p, ((c, n), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(match h {
            Some(h) => [V::S(htype_name(db).get(h).unwrap()), V::T(db.post_history.creation_date.get(h).unwrap())],
            None => [V::S("No History"), V::Null],
        });
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// TopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COALESCE(UBC.BadgeCount, 0) AS BadgeCount
//     FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId WHERE U.Reputation > 1000 ORDER BY U.Reputation DESC LIMIT 10),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AvgScore FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// CombinedStats AS (SELECT TU.Id AS UserId, TU.DisplayName, TU.Reputation, TU.CreationDate, TU.LastAccessDate, TU.BadgeCount, COALESCE(PS.PostCount, 0) AS PostCount,
//        COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(PS.AvgScore, 0) AS AvgScore FROM TopUsers TU LEFT JOIN PostStats PS ON TU.Id = PS.OwnerUserId)
// SELECT CS.UserId, CS.DisplayName, CS.Reputation, CS.CreationDate, CS.LastAccessDate, CS.BadgeCount, CS.PostCount, CS.TotalViews, CS.AvgScore,
//        RANK() OVER (ORDER BY CS.TotalViews DESC) AS ViewsRank, RANK() OVER (ORDER BY CS.AvgScore DESC) AS ScoreRank
// FROM CombinedStats CS ORDER BY CS.Reputation DESC, CS.BadgeCount DESC;
fn q6573(db: &'static So) -> String {
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { owner_user, creation_date, view_count, score, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(view_count.opt().and(score))
        .fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + w.is_some() as i64, a[3] + s]);
    let v = drain((&bc).and((&ps).opt()));
    let v: Vec<_> = v.into_iter().map(|(u, (b, a))| {
        let a = a.unwrap_or([0; 4]);
        let s = if a[0] == 0 { 0.0 } else { a[3] as f64 / a[0] as f64 };
        (u, b, a, s)
    }).collect();
    let v = ranked(v, |&(_, _, a, _)| Reverse(a[1]), false);
    let v = ranked(v, |&((_, _, _, s), _)| Reverse(fkey(s)), false);
    rows(v.into_iter().map(|(((u, b, a, s), vr), sr)| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated", "last_access"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::F(s), V::I(vr), V::I(sr)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, u.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate ASC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVoteSummary AS (SELECT PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVotes, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN vt.Name = 'Close' THEN 1 END) AS CloseVotes, COUNT(CASE WHEN vt.Name = 'Reopen' THEN 1 END) AS ReopenVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerUserId, rp.OwnerReputation, pvs.UpVotes, pvs.DownVotes, pvs.CloseVotes, pvs.ReopenVotes
// FROM RankedPosts rp LEFT JOIN PostVoteSummary pvs ON rp.PostId = pvs.PostId WHERE rp.PostRank <= 5 ORDER BY rp.OwnerUserId, rp.Score DESC;
fn q8160(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvs = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold([0i64; 4], |a, n| {
        [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64, a[2] + (n == "Close") as i64, a[3] + (n == "Reopen") as i64]
    });
    let v = drain((&tp).select(Ident::<Post>::new().and((&pvs).opt())));
    rows(v.into_iter().map(|(_, (p, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner_id", "rep"]);
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate AS CloseDate, ph.UserId AS CloserId, crt.Name AS CloseReason FROM PostHistory ph
//     JOIN CloseReasonTypes crt ON CAST(ph.Comment AS INTEGER) = crt.Id WHERE ph.PostHistoryTypeId IN (10, 11))
// SELECT up.UserId, up.Reputation, up.TotalBounty, rp.Title, rp.CreationDate, rp.Score, cp.CloseDate, cp.CloseReason
// FROM UserReputation up JOIN RecentPosts rp ON up.UserId = rp.OwnerUserId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId
// WHERE up.Reputation >= 100 AND rp.rn = 1 ORDER BY up.Reputation DESC, rp.Score DESC LIMIT 50;
fn q136(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let fp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tb = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.in_v(vec![10, 11]))).select(Ident::<PostHistory>::new().and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)));
    let up = Ident::<User>::new().with((&db.user.reputation).ge(100)).and(&tb);
    let v = drain((&fp).select(owner_user.select(up).and(closes.opt())));
    let v = top_n(v, |&(p, ((u, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(p, ((u, b), c))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend(match c {
            Some((h, r)) => [V::T(db.post_history.creation_date.get(h).unwrap()), V::S(r)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, AVG(LENGTH(p.Body)) AS AvgBodyLength, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes vt ON p.Id = vt.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostRanked AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, AvgBodyLength, LastPostDate,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, AvgBodyLength, LastPostDate, ReputationRank
// FROM PostRanked WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
//
// ReputationRank reads only Reputation, so the ten users are picked first and the post x vote product is driven for them alone.
fn q7639(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tv = rel(tu.iter().enumerate().map(|(i, &(u, _))| (u, i as i64 + 1)).collect());
    let rank: HashIdx<Id<User>, i64> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, r)| r)).collect();
    let Post { post_type_id, body, creation_date, .. } = &db.post;
    let s = db
        .user
        .with(&rank)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(body).and(creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0, 0, 0, 0, 0, 0, i64::MIN], |a, x| match x {
            Some((((t, b), d), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5] + b.chars().count() as i64, a[6].max(d)],
            None => a,
        });
    let pc = db.user.with(&rank).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&s).and(&pc).and(&rank));
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[5], a[0]), tmax(a[6]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS QuestionCount, SUM(COALESCE(b.Class, 0)) AS TotalBadges, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName
//     ORDER BY QuestionCount DESC, TotalScore DESC LIMIT 10)
// SELECT tu.DisplayName, tu.QuestionCount, tu.TotalBadges, MAX(rp.ViewCount) AS MaxViewCount, MAX(rp.Score) AS MaxScore, COUNT(DISTINCT rp.PostId) AS TotalPosts,
//        (SELECT COUNT(*) FROM Posts WHERE OwnerUserId = tu.UserId AND PostTypeId = 2) AS AnswerCount
// FROM TopUsers tu JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId GROUP BY tu.UserId, tu.DisplayName, tu.QuestionCount, tu.TotalBadges
// HAVING COUNT(DISTINCT rp.PostId) > 5 ORDER BY tu.QuestionCount DESC, MaxScore DESC;
fn q7824(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let q = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(q().select(score).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (s, c)| [a[0] + s.is_some() as i64, a[1] + c.unwrap_or(0), a[2] + s.unwrap_or(0)]);
    let top = top_n(drain(&tu), |&(u, a)| (Reverse(a[0]), a[0] == 0, Reverse(a[2]), u), 10);
    let tv = rel(top);
    let tops: HashIdx<Id<User>, [i64; 3]> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, a)| a)).collect();
    let rp = db.user.with(&tops).group_by(Ident::<User>::new()).select(q().select(view_count.opt().and(score))).fold((0i64, None, i64::MIN), |(n, w, s), (v, x)| {
        (n + 1, match (w, v) { (Some(a), Some(b)) => Some(std::cmp::max(a, b)), (a, b) => a.or(b) }, s.max(x))
    });
    let ans = db.user.with(&tops).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2))).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&tops).and((&rp).filt(|x: (i64, Option<i64>, i64)| x.0 > 5)).and(&ans));
    rows(v.into_iter().map(|(u, ((a, (n, w, s)), an))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), oint(w), V::I(s), V::I(n), V::I(an)])))
}

// WITH PostTags AS (SELECT p.Id AS PostId, UNNEST(string_to_array(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS QuestionsAsked, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// TagPopularity AS (SELECT pt.Tag, COUNT(pt.PostId) AS NumberOfPosts FROM PostTags pt GROUP BY pt.Tag ORDER BY NumberOfPosts DESC LIMIT 10)
// SELECT u.DisplayName, u.Reputation, us.QuestionsAsked, us.TotalUpVotes, us.TotalDownVotes, tp.Tag, tp.NumberOfPosts
// FROM Users u JOIN UserStats us ON u.Id = us.UserId
// JOIN TagPopularity tp ON tp.Tag IN (SELECT UNNEST(string_to_array(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><')) FROM Posts p WHERE p.OwnerUserId = u.Id AND p.PostTypeId = 1)
// ORDER BY us.TotalUpVotes DESC, us.QuestionsAsked DESC;
fn q29991(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user, .. } = &db.post;
    let freq = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let top = rel(top_n(drain(&freq), |&(t, n)| (Reverse(n), t), 10));
    let tp: HashIdx<Str, i64> = (&top).map(|(t, _)| t).inv().select((&top).map(|(_, n)| n)).collect();
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, v| [a[0] + (v == Some(Some(2))) as i64, a[1] + (v == Some(Some(3))) as i64]);
    let qa = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    type UT = (Id<User>, Str);
    let ut: MatSet<UT> = db.post.with(post_type_id.eq(1)).select(owner_user.and(tags_str.flat_map(tag_list))).collect();
    let v = drain((&ut).select(Same::<UT>::new().map(|(_, t): UT| t).select(&tp).and(Same::<UT>::new().map(|(u, _): UT| u).select((&us).and(&qa)))));
    rows(v.into_iter().map(|((u, t), (n, (a, q)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(q), V::I(a[0]), V::I(a[1]), V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT a.Id) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.PostTypeId = 1 AND p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR')
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.OwnerUserId),
// TopRankedPosts AS (SELECT rp.* FROM RankedPosts rp WHERE PostRank = 1)
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, trp.OwnerDisplayName, trp.CommentCount, trp.AnswerCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = trp.PostId AND v.VoteTypeId IN (2, 3)) AS VoteCount
// FROM TopRankedPosts trp ORDER BY trp.Score DESC, trp.ViewCount DESC;
fn q8824(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let an = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ud = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let vc = (&tp).group_by(Ident::<Post>::new()).select(ud.opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(&an).and(&vc));
    rows(v.into_iter().map(|(p, ((c, a), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(a), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS rn
//     FROM Posts P WHERE P.PostTypeId = 1 AND P.Score > 0),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostHistoryStats AS (SELECT PH.PostId, COUNT(*) AS EditCount, MAX(PH.CreationDate) AS LastEditDate FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (4, 5, 24) GROUP BY PH.PostId)
// SELECT UP.DisplayName, UP.TotalBounty, UP.BadgeCount, RP.Title, RP.CreationDate, RP.Score, PH.EditCount, PH.LastEditDate
// FROM UserStats UP JOIN RankedPosts RP ON RP.rn = 1 AND RP.PostId = UP.UserId LEFT JOIN PostHistoryStats PH ON RP.PostId = PH.PostId
// WHERE UP.TotalBounty > 0 ORDER BY UP.TotalBounty DESC, UP.BadgeCount DESC, RP.Score DESC LIMIT 50;
//
// RP.PostId = UP.UserId compares a post id with a user id, so it goes through origid.
fn q1041(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()))
        .fold(0i64, |n, (b, _)| n + b.flatten().unwrap_or(0));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(post_history_type_id.is_in([4, 5, 24])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&tp).select(origid.select(&uidx).select(Ident::<User>::new().and((&us).filt(|b| b > 0)).and(&bc)).and((&phs).opt())));
    let v = top_n(v, |&(p, (((_, b), c), _))| (Reverse(b), Reverse(c), Reverse(score.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(p, (((u, b), c), h))| {
        let mut f = vec![user_col(db, u, "name"), V::I(b), V::I(c)];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(c.Id) AS CommentCount, SUM(b.Class) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, CommentCount, BadgeCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserActivity)
// SELECT t.DisplayName, t.PostCount, t.QuestionCount, t.AnswerCount, t.UpVotes, t.DownVotes, t.CommentCount, t.BadgeCount FROM TopUsers t WHERE t.Rank <= 10 ORDER BY t.Rank;
//
// Rank reads only COUNT(DISTINCT p.Id), so the ten users with most posts are picked first and the post x vote x comment x badge product is driven for them alone.
fn q7206(db: &'static So) -> String {
    let pc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let top = top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let per_post = (&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt());
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(per_post).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let (t, v, c) = match p {
                Some(((t, v), c)) => (Some(t), v, c.is_some()),
                None => (None, None, false),
            };
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + c as i64, a[5] + b.unwrap_or(0), a[6] + b.is_some() as i64]
        });
    let v = drain((&pc).and(&s));
    rows(v.into_iter().map(|(u, (n, a))| row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), nullable(a[5], a[6])])))
}

// WITH FilteredPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName AS OwnerDisplayName, ph.UserDisplayName AS LastEditor,
//        MAX(ph.CreationDate) AS LastEditDate, COUNT(c.Id) AS CommentCount, COUNT(a.Id) AS AnswerCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '7 days' AND p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName, ph.UserDisplayName),
// TagCounts AS (SELECT PostId, COUNT(DISTINCT t.TagName) AS UniqueTagCount FROM FilteredPosts fp CROSS JOIN UNNEST(string_to_array(fp.Tags, '>')) AS t(TagName) GROUP BY PostId)
// SELECT fp.PostId, fp.Title, fp.OwnerDisplayName, fp.LastEditor, fp.LastEditDate, fp.CommentCount, fc.UniqueTagCount, fp.AnswerCount
// FROM FilteredPosts fp LEFT JOIN TagCounts fc ON fp.PostId = fc.PostId ORDER BY fp.CreationDate DESC;
fn q27120(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, .. } = &db.post;
    let fp = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -7)).and(post_type_id.eq(1)));
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = fp().select(Ident::<Post>::new().and(history_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let PostHistory { user_display_name, creation_date: hd, .. } = &db.post_history;
    let g = (&j)
        .group_by((&post_of).and((&hist_of).select(user_display_name).opt()))
        .select((&hist_of).select(hd).opt().and((&post_of).select(children_of(db).opt().and(comments_of(db).opt()))))
        .fold((i64::MIN, 0i64, 0i64), |(m, c, a), (d, (x, y))| (m.max(d.unwrap_or(i64::MIN)), c + y.is_some() as i64, a + x.is_some() as i64));
    let tc = fp().group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split('>'))).count_distinct();
    type K = (Id<Post>, Option<Str>);
    let v = drain((&g).and(Same::<K>::new().map(|(p, _): K| p).select((&tc).opt())));
    rows(v.into_iter().map(|((p, e), ((m, c, a), t))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([ostr(e), tmax(m), V::I(c), oint(t), V::I(a)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.Score, rp.ViewCount, us.DisplayName AS OwnerDisplayName, us.GoldBadges, us.SilverBadges, us.BronzeBadges
//     FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId WHERE rp.PostRank <= 5)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.GoldBadges, tp.SilverBadges, tp.BronzeBadges FROM TopPosts tp ORDER BY tp.Score DESC LIMIT 10 OFFSET 0;
fn q1379(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(current_date(), -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let us = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = drain((&tp).select(owner_user.select(&us)));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(V.BountyAmount) AS TotalBountyAmount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// PopularUsers AS (SELECT US.UserId, US.DisplayName, US.Reputation, US.TotalPosts, US.TotalComments, US.TotalQuestions, US.TotalAnswers, US.TotalBountyAmount,
//        DENSE_RANK() OVER (ORDER BY US.Reputation DESC) AS ReputationRank FROM UserStatistics US WHERE US.TotalPosts > 0)
// SELECT PU.UserId, PU.DisplayName, PU.Reputation, PU.TotalPosts, PU.TotalComments, PU.TotalQuestions, PU.TotalAnswers, PU.TotalBountyAmount, PU.ReputationRank
// FROM PopularUsers PU WHERE PU.ReputationRank <= 10 ORDER BY PU.Reputation DESC;
//
// ReputationRank reads only Reputation over the users with a post, so those users are picked first and the post x comment x vote product is driven for them alone.
fn q6818(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let r = ranked(drain((&pc).and(&db.user.reputation)), |&(_, (_, r))| Reverse(r), true);
    let tv = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, (n, _)), k)| (u, (n, k))).collect());
    let tops: HashIdx<Id<User>, (i64, i64)> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, x)| x)).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let s = db
        .user
        .with(&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(bounty.opt())))
        .fold([0i64; 4], |a, ((t, _), b)| {
            let b = b.flatten();
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
        });
    let cc = db.user.with(&tops).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let v = drain((&tops).and(&s).and((&cc).opt()));
    rows(v.into_iter().map(|(u, (((n, k), a), c))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(k)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= '2022-01-01' AND p.CreationDate < '2023-01-01' GROUP BY p.Id, p.Title),
// RankedPosts AS (SELECT PostId, Title, CommentCount, Upvotes, Downvotes, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY Upvotes - Downvotes DESC) AS PostRank FROM PostStats)
// SELECT rp.PostId, rp.Title, rp.CommentCount, rp.Upvotes, rp.Downvotes, rp.GoldBadges, rp.SilverBadges, rp.BronzeBadges, rp.PostRank FROM RankedPosts rp WHERE rp.PostRank <= 10 ORDER BY rp.PostRank;
fn q8176(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(ts(2022, 1, 1, 0, 0, 0)).and(creation_date.lt(ts(2023, 1, 1, 0, 0, 0))))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db).select(&db.badge.class)).opt()))
        .fold([0i64; 6], |a, ((c, t), b)| {
            [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (b == Some(1)) as i64, a[4] + (b == Some(2)) as i64, a[5] + (b == Some(3)) as i64]
        });
    let r = ranked(drain(&s), |&(_, a)| Reverse(a[1] - a[2]), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((p, a), k)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(a.map(V::I));
        f.push(V::I(k));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS Author, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT V.Id) AS VoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS rn
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY P.Id, U.DisplayName, P.Title, P.CreationDate, P.PostTypeId),
// FilteredPosts AS (SELECT PostId, Title, CreationDate, Author, CommentCount, VoteCount, UpVotes, DownVotes FROM RankedPosts WHERE rn <= 5)
// SELECT FP.PostId, FP.Title, FP.CreationDate, FP.Author, FP.CommentCount, FP.VoteCount, FP.UpVotes, FP.DownVotes,
//        CASE WHEN FP.UpVotes > FP.DownVotes THEN 'Positive' WHEN FP.UpVotes < FP.DownVotes THEN 'Negative' ELSE 'Neutral' END AS Sentiment
// FROM FilteredPosts FP ORDER BY FP.CreationDate DESC;
//
// rn reads only base columns, so the five newest posts of each type are picked first and the comment x vote product is driven for those alone.
fn q6912(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and(&cc).and(&vc));
    rows(v.into_iter().map(|(p, ((a, c), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1]), V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, u.DisplayName AS OwnerDisplayName, ub.Reputation AS OwnerReputation
// FROM TopPosts tp JOIN Posts p ON tp.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id JOIN Users ub ON p.OwnerUserId = ub.Id
// WHERE tp.UpVoteCount > tp.DownVoteCount ORDER BY tp.Score DESC, tp.CommentCount DESC;
//
// Rank reads only base columns, so the ten questions are picked first and the comment x vote product is driven for those alone.
fn q7766(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(score));
    let top = top_n(v, |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&s).filt(|a| a[1] > a[2]).and(owner_user));
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// UserRanked AS (SELECT UserId, DisplayName, QuestionCount, AnswerCount, CommentCount, UpVotes, DownVotes, BadgeCount,
//        RANK() OVER (ORDER BY UpVotes DESC, QuestionCount DESC, AnswerCount DESC) AS ActivityRank FROM UserActivity)
// SELECT UserId, DisplayName, QuestionCount, AnswerCount, CommentCount, UpVotes, DownVotes, BadgeCount, ActivityRank FROM UserRanked WHERE ActivityRank <= 10 ORDER BY ActivityRank;
fn q5498(db: &'static So) -> String {
    let per_post = (&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt());
    let s = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(per_post).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, _)| match p {
            Some(((t, c), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let bc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let r = ranked(drain((&s).and(&bc)), |&(_, (a, _))| (Reverse(a[3]), Reverse(a[0]), Reverse(a[1])), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, b)), k)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(b), V::I(k)]);
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT U.Id AS UserId, COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpVotes, COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 3) AS DownVotes, COUNT(V.Id) AS TotalVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id),
// PostAnswerCounts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS AnswerCount FROM Posts P WHERE P.PostTypeId = 2 GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT U.Id, U.DisplayName, COALESCE(UAC.UpVotes, 0) AS UpVotes, COALESCE(UAC.DownVotes, 0) AS DownVotes, COALESCE(PAC.AnswerCount, 0) AS Answers, U.Reputation,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(UAC.UpVotes, 0) DESC, U.Reputation DESC) AS Rank
//     FROM Users U LEFT JOIN UserVoteCounts UAC ON U.Id = UAC.UserId LEFT JOIN PostAnswerCounts PAC ON U.Id = PAC.OwnerUserId)
// SELECT TU.DisplayName, TU.UpVotes, TU.DownVotes, TU.Answers, TU.Reputation, CASE WHEN TU.Rank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorType
// FROM TopUsers TU WHERE TU.Reputation > 100 ORDER BY TU.UpVotes DESC, TU.Answers DESC;
fn q4080(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pac = db.post.with((&db.post.post_type_id).eq(2)).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&uv).and(&db.user.reputation));
    let top = top_n(v, |&(u, (a, r))| (Reverse(a[0]), Reverse(r), u), 10);
    let tops: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(100)).select((&uv).and((&pac).opt()).and(Ident::<User>::new().with(&tops).opt())));
    rows(v.into_iter().map(|(u, ((a, n), t))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n.unwrap_or(0)), user_col(db, u, "rep"), V::S(if t.is_some() { "Top Contributor" } else { "Regular Contributor" })])
    }))
}

// WITH PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount, COUNT(CASE WHEN PH.Id IS NOT NULL THEN 1 END) AS EditCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, VoteCount, EditCount, RANK() OVER (ORDER BY Score DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY ViewCount DESC) AS ViewRank FROM PostStats)
// SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, VoteCount, EditCount, ScoreRank, ViewRank
// FROM TopPosts WHERE ScoreRank <= 10 OR ViewRank <= 10 ORDER BY ScoreRank, ViewRank;
//
// Both ranks read only base columns, so they are taken over every post first and the comment x vote x history product is driven for the posts they keep.
fn q14722(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let r = ranked(drain(score), |&(_, s)| Reverse(s), false);
    let r = ranked(r, |&((p, _), _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, false);
    let tv = rel(r.into_iter().filter(|&((_, s), w)| s <= 10 || w <= 10).map(|(((p, _), s), w)| (p, (s, w))).collect());
    let tops: HashIdx<Id<Post>, (i64, i64)> = (&tv).map(|(p, _)| p).inv().select((&tv).map(|(_, x)| x)).collect();
    let s = db
        .post
        .with(&tops)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 3], |a, ((c, v), h)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64, a[2] + h.is_some() as i64]);
    let v = drain((&s).and(&tops));
    rows(v.into_iter().map(|(p, (a, (sr, wr)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend(a.map(V::I));
        f.extend([V::I(sr), V::I(wr)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(pv.VoteCount, 0) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY COALESCE(pv.VoteCount, 0) DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) pv ON p.Id = pv.PostId
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// FilteredUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(b.Class) AS TotalBadgeClass, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName HAVING SUM(b.Class) > 3)
// SELECT rp.Title, rp.PostId, rp.CreationDate, rp.Score, rp.VoteCount, fu.DisplayName, fu.TotalBadgeClass, fu.BadgeCount
// FROM RankedPosts rp JOIN FilteredUsers fu ON rp.PostId IN (SELECT c.PostId FROM Comments c WHERE c.UserId = fu.UserId)
// WHERE rp.PostRank <= 5 ORDER BY rp.CreationDate DESC LIMIT 100;
fn q3606(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let vc = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).opt())
        .fold(0i64, |n, v| n + v.is_some() as i64);
    let top = top_per(drain(&vc), |&(p, _)| post_type_id.get(p).unwrap(), |&(p, n)| (Reverse(n), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tv = rel(top);
    let tp: HashIdx<Id<Post>, i64> = (&tv).map(|(p, _)| p).inv().select((&tv).map(|(_, n)| n)).collect();
    let fu = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 2], |a, c| [a[0] + c, a[1] + 1]);
    type PU = (Id<Post>, Id<User>);
    let pairs: MatSet<PU> = db.post.with(&tp).select(Ident::<Post>::new().and(comments_of(db).select(&db.comment.user))).collect();
    let v = drain((&pairs).select(Same::<PU>::new().map(|(p, _): PU| p).select(&tp).and(Same::<PU>::new().map(|(_, u): PU| u).select((&fu).filt(|a| a[0] > 3)))));
    let v = top_n(v, |&((p, u), _)| (Reverse(creation_date.get(p).unwrap()), p, u), 100);
    rows(v.into_iter().map(|((p, u), (n, a))| {
        let mut f = post_fields(db, p, &["title", "id", "created", "score"]);
        f.extend([V::I(n), user_col(db, u, "name"), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.ViewCount, p.OwnerUserId),
// TopPosts AS (SELECT pm.PostId, pm.Title, pm.Score, pm.ViewCount, pm.CommentCount, pm.UpVotes, pm.DownVotes,
//        CASE WHEN pm.UserPostRank < 5 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributorLevel FROM PostMetrics pm WHERE pm.ViewCount > 100)
// SELECT pm.Title, pm.Score, pm.ViewCount, pm.CommentCount, pm.UpVotes, pm.DownVotes, COALESCE(pm.ContributorLevel, 'New Contributor') AS ContributorLevel
// FROM TopPosts pm WHERE pm.CommentCount > 0 AND pm.UpVotes - pm.DownVotes >= 5 ORDER BY pm.Score DESC LIMIT 10;
fn q4650(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, score, .. } = &db.post;
    let top = top_per(drain(db.post.select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 4, false);
    let tops: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = db
        .post
        .with(view_count.gt(100))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&s).filt(|a| a[0] > 0 && a[1] - a[2] >= 5).and(Ident::<Post>::new().with(&tops).opt()));
    let v = top_n(v, |&(p, _)| Reverse(score.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S(if t.is_some() { "Top Contributor" } else { "Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId, p.Score),
// TopPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE Rank <= 5)
// SELECT t.Title, t.OwnerDisplayName, t.CommentCount, t.UpVotes, t.DownVotes,
//        CASE WHEN t.UpVotes + t.DownVotes > 0 THEN ROUND(CAST(t.UpVotes AS FLOAT) / (t.UpVotes + t.DownVotes) * 100, 2) ELSE 0 END AS UpvotePercentage
// FROM TopPosts t ORDER BY t.UpVotes DESC, t.CommentCount DESC;
//
// Rank reads only Score, so the posts are ranked first and the comment x vote product is driven for those kept.
fn q7863(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id.and(score)));
    let r = per_group(ranked(v, |&(_, (t, s))| (t, Reverse(s)), true), |&(_, (t, _))| t);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().filter(|x| x.1 <= 5).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::F(if a[1] + a[2] > 0 { ((a[1] as f32 / (a[1] + a[2]) as f32 * 100.0 * 100.0).round() / 100.0) as f64 } else { 0.0 }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankByScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE RankByScore <= 5),
// PostVoteCounts AS (SELECT PostId, COUNT(*) FILTER (WHERE VoteTypeId = 2) AS UpVotes, COUNT(*) FILTER (WHERE VoteTypeId = 3) AS DownVotes FROM Votes GROUP BY PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes,
//        (COALESCE(pvc.UpVotes, 0) - COALESCE(pvc.DownVotes, 0)) AS NetVotes
// FROM TopPosts tp LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q9035(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvc = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&tp).select(Ident::<Post>::new().and((&pvc).opt())));
    rows(v.into_iter().map(|(_, (p, a))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.Views, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(DISTINCT P.Id) AS TotalPosts,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions, COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.Views),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, CreationDate, Views, GoldBadges, SilverBadges, BronzeBadges, TotalPosts, TotalQuestions, TotalAnswers,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT *, (GoldBadges * 3 + SilverBadges * 2 + BronzeBadges * 1) AS BadgeScore FROM TopUsers WHERE Rank <= 10 ORDER BY BadgeScore DESC, Reputation DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the badge x post product is driven for them alone.
fn q9322(db: &'static So) -> String {
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tv = rel(tu.iter().enumerate().map(|(i, &(u, _))| (u, i as i64 + 1)).collect());
    let rank: HashIdx<Id<User>, i64> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, r)| r)).collect();
    let b = db
        .user
        .with(&rank)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt()))
        .fold([0i64; 3], |a, (c, _)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let p = db.user.with(&rank).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64]
    });
    let v = drain((&b).and(&p).and(&rank));
    rows(v.into_iter().map(|(u, ((b, p), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated", "uviews"]);
        f.extend(b.map(V::I));
        f.extend(p.map(V::I));
        f.extend([V::I(r), V::I(b[0] * 3 + b[1] * 2 + b[2])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(U.Reputation) AS AvgReputation, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// PostRanking AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, ROW_NUMBER() OVER (ORDER BY P.Score DESC) AS Rank, P.OwnerUserId FROM Posts P WHERE P.PostTypeId = 1)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.AvgReputation, U.TotalUpvotes, U.TotalDownvotes, P.PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.Rank
// FROM UserStats U JOIN PostRanking P ON U.UserId = P.OwnerUserId WHERE P.Rank <= 10 ORDER BY U.TotalPosts DESC, U.AvgReputation DESC;
//
// UserStats is only read through the join, so it is taken for the owners of the ten top questions alone.
fn q12990(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tv = rel(top.iter().enumerate().map(|(i, &(p, _))| (p, i as i64 + 1)).collect());
    let rank: HashIdx<Id<Post>, i64> = (&tv).map(|(p, _)| p).inv().select((&tv).map(|(_, r)| r)).collect();
    let owners: MatSet<Id<User>> = db.post.with(&rank).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 6], |a, (r, p)| {
            let (t, v) = p.map_or((None, None), |(t, v)| (Some(t), v));
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + r, a[3] + 1, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64]
        });
    let tp = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain(db.post.with(&rank).select(Ident::<Post>::new().and(&rank).and(owner_user.select(Ident::<User>::new().and(&us).and(&tp)))));
    rows(v.into_iter().map(|(_, ((p, r), ((u, a), n)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), avg(a[2], a[3]), V::I(a[4]), V::I(a[5])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, CASE WHEN Reputation < 100 THEN 'Newbie' WHEN Reputation BETWEEN 100 AND 1000 THEN 'Intermediate' ELSE 'Expert' END AS ReputationLevel FROM Users),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.CreationDate),
// TopPosts AS (SELECT *, RANK() OVER (PARTITION BY Score ORDER BY CommentCount DESC) AS Rank FROM PostStatistics WHERE Score > 10)
// SELECT ur.DisplayName, ur.ReputationLevel, tp.Title, tp.Score, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount
// FROM UserReputation ur JOIN Posts p ON ur.Id = p.OwnerUserId JOIN TopPosts tp ON p.Id = tp.PostId WHERE ur.ReputationLevel = 'Expert' AND tp.Rank <= 5
// ORDER BY tp.CommentCount DESC, tp.Score DESC;
fn q42(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(10)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let r = per_group(ranked(drain(&s), |&(p, a)| (score.get(p).unwrap(), Reverse(a[0])), false), |&(p, _)| score.get(p).unwrap());
    let tv = rel(r.into_iter().filter(|x| x.1 <= 5).map(|x| x.0).collect());
    let tp: HashIdx<Id<Post>, [i64; 3]> = (&tv).map(|(p, _)| p).inv().select((&tv).map(|(_, a)| a)).collect();
    let v = drain(db.post.with(&tp).select((&tp).and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))));
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = vec![user_col(db, u, "name"), V::S("Expert")];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        ROW_NUMBER() OVER (ORDER BY COUNT(p.Id) DESC) AS UserRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, TotalViews FROM UserPostStats WHERE UserRank <= 10),
// PostComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COALESCE(MAX(c.Score), 0) AS MaxCommentScore FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT tu.DisplayName, tu.PostCount, tu.TotalScore, tu.TotalViews, pc.CommentCount, pc.MaxCommentScore,
//        CASE WHEN pc.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus
// FROM TopUsers tu LEFT JOIN PostComments pc ON pc.PostId = (SELECT Id FROM Posts WHERE OwnerUserId = tu.UserId ORDER BY Id DESC LIMIT 1) ORDER BY tu.TotalScore DESC;
//
// The correlated ORDER BY Id DESC LIMIT 1 is a per-user arg-max fold on the post id.
fn q2741(db: &'static So) -> String {
    let Post { score, view_count, origid, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)],
        None => a,
    });
    let top = top_n(drain(&ups), |&(u, a)| (Reverse(a[0]), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let last = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().and(origid))).fold(None, |m: Option<(Id<Post>, i64)>, (p, o)| match m {
        Some((_, mo)) if mo >= o => m,
        _ => Some((p, o)),
    });
    let pc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold((0i64, i64::MIN), |(n, m), s| match s {
        Some(s) => (n + 1, m.max(s)),
        None => (n, m),
    });
    let v = drain((&tu).select((&ups).and((&last).flat_map(|m: Option<(Id<Post>, i64)>| m.map(|x| x.0)).select(&pc).opt())));
    rows(v.into_iter().map(|(u, (a, c))| {
        let (n, m) = c.unwrap_or((0, i64::MIN));
        row(vec![
            user_col(db, u, "name"),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            if c.is_some() { V::I(n) } else { V::Null },
            if c.is_some() { V::I(if n == 0 { 0 } else { m }) } else { V::Null },
            V::S(if n > 0 { "Has Comments" } else { "No Comments" }),
        ])
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate, u.UpVotes, u.DownVotes, COUNT(DISTINCT p.Id) AS PostCount,
//        COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate, u.UpVotes, u.DownVotes),
// TopPosts AS (SELECT p.OwnerUserId, p.Id AS PostId, p.Title, p.Score, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank FROM Posts p)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.CreationDate, us.LastAccessDate, us.UpVotes, us.DownVotes, us.PostCount, us.CommentCount, tp.PostId, tp.Title, tp.Score
// FROM UserStatistics us LEFT JOIN TopPosts tp ON us.UserId = tp.OwnerUserId AND tp.PostRank = 1 ORDER BY us.Reputation DESC, us.PostCount DESC;
//
// TotalUpVotes and TotalDownVotes are never projected, so the vote product is not driven; the two distinct counts need one row per post.
fn q14054(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let top = top_per(drain(db.post.with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tv = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, Id<Post>> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, p)| p)).collect();
    let v = drain((&pc).and(&cc).and((&by_user).opt()));
    rows(v.into_iter().map(|(u, ((n, c), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated", "last_access", "uup", "udown"]);
        f.extend([V::I(n), V::I(c)]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "score"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// PostRanking AS (SELECT u.UserId, u.DisplayName, u.Reputation, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalUpvotes, u.TotalDownvotes, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM UserStatistics u)
// SELECT pr.DisplayName, pr.Reputation, pr.TotalPosts, pr.TotalQuestions, pr.TotalAnswers, pr.TotalUpvotes, pr.TotalDownvotes, pr.ReputationRank
// FROM PostRanking pr WHERE pr.TotalPosts > 5 AND pr.TotalAnswers > 10 ORDER BY pr.Reputation DESC, pr.TotalUpvotes DESC LIMIT 50;
fn q8749(db: &'static So) -> String {
    let t = &db.post.post_type_id;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(t).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64]);
    let vs = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, v| {
        [a[0] + (v == Some(Some(2))) as i64, a[1] + (v == Some(Some(3))) as i64]
    });
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, i64> = (&rr).map(|(u, _)| u).inv().select((&rr).map(|(_, r)| r)).collect();
    let v = drain((&pc).filt(|a| a[0] > 5 && a[2] > 10).and(&vs).and(&rank));
    let v = top_n(v, |&(u, ((_, b), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(b[0])), 50);
    rows(v.into_iter().map(|(u, ((a, b), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b[0]), V::I(b[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 END), 0) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 END), 0) AS QuestionCount, COALESCE(SUM(CASE WHEN P.PostTypeId IN (2, 3) THEN 1 END), 0) AS TotalPosts,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Views, AnswerCount, QuestionCount, TotalPosts, TotalUpVotes, TotalDownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT T.UserId, T.DisplayName, T.Reputation, T.Views, T.AnswerCount, T.QuestionCount, T.TotalPosts, T.TotalUpVotes, T.TotalDownVotes FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.Rank;
//
// Rank reads only Reputation, so the ten users are picked first and the post x vote product is driven for them alone.
fn q8830(db: &'static So) -> String {
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (t == 2 || t == 3) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    rows(drain(&s).into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// Rewritten (rewrites/28777.sql): the float AVG of post ages became the exact-integer mean.
// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(p.Score) AS TotalScore,
//        SUM(epoch_us(cast('2024-10-01 12:34:56' as timestamp)) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.CreationDate) / 1e6 / 3600 AS AvgPostAgeHours
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// PopularTags AS (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS TagName FROM Posts p WHERE p.PostTypeId = 1),
// TagUsage AS (SELECT t.TagName, COUNT(*) AS UsageCount FROM PopularTags pt JOIN Tags t ON t.TagName = pt.TagName GROUP BY t.TagName)
// SELECT ups.DisplayName, ups.TotalPosts, ups.Questions, ups.Answers, ups.TotalScore, ups.AvgPostAgeHours, tu.TagName, tu.UsageCount
// FROM UserPostStats ups JOIN TagUsage tu ON tu.UsageCount = (SELECT MAX(UsageCount) FROM TagUsage) ORDER BY ups.TotalPosts DESC, ups.TotalScore DESC;
//
// The ON clause names only tu (against an uncorrelated scalar), so the users are crossed with the most used tags.
fn q28777(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, tags_str, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ups = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(creation_date)).opt())
        .fold(([0i64; 4], 0i128), |(a, s), p| match p {
            Some(((t, sc), d)) => ([a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + sc], s + (t0 - d) as i128),
            None => (a, s),
        });
    let tidx: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let usage = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list).select(&tidx)).group_by(Same::<Id<Tag>>::new()).select(Same::<Id<Tag>>::new()).fold(0i64, |n, _| n + 1);
    let mx = (&usage).fold_flat(0i64, |m, n| m.max(n));
    let most = rel(drain((&usage).filt(|n| n == mx)));
    let mut v = Vec::new();
    (&ups).cross(&most).drive(|(u, _), (a, (t, n))| v.push((u, a, t, n)));
    rows(v.into_iter().map(|(u, (a, s), t, n)| {
        row(vec![
            user_col(db, u, "name"),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            nullable(a[3], a[0]),
            if a[0] == 0 { V::Null } else { V::F(s as f64 / a[0] as f64 / 1e6 / 3600.0) },
            V::S(db.tag.tag_name.get(t).unwrap()),
            V::I(n),
        ])
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(CASE WHEN p.AnswerCount > 0 THEN 1 ELSE 0 END) AS QuestionsWithAnswers,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, PositivePosts, NegativePosts, QuestionsWithAnswers, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY PostCount DESC) AS RankScore
//     FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, PositivePosts, NegativePosts, QuestionsWithAnswers, GoldBadges, SilverBadges, BronzeBadges FROM TopUsers WHERE RankScore <= 10 ORDER BY PostCount DESC;
//
// RankScore reads only COUNT(DISTINCT p.Id), so the users are ranked on their post counts first and the post x badge product is driven for those kept.
fn q7977(db: &'static So) -> String {
    let pc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let r = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let tv = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let tops: HashIdx<Id<User>, i64> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, n)| n)).collect();
    let Post { score, answer_count, .. } = &db.post;
    let s = db
        .user
        .with(&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(answer_count.opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, c)| {
            let (pos, neg, qa) = p.map_or((false, false, false), |(s, n)| (s > 0, s < 0, n.map_or(false, |n| n > 0)));
            [a[0] + pos as i64, a[1] + neg as i64, a[2] + qa as i64, a[3] + (c == Some(1)) as i64, a[4] + (c == Some(2)) as i64, a[5] + (c == Some(3)) as i64]
        });
    rows(drain((&tops).and(&s)).into_iter().map(|(u, (n, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.ViewCount DESC) AS Rank
//     FROM Posts AS P JOIN Users AS U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND P.PostTypeId IN (1, 2)),
// TopRankedPosts AS (SELECT PostId, Title, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// PostDetails AS (SELECT TRP.*, C.CommentCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty FROM TopRankedPosts AS TRP
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) AS C ON TRP.PostId = C.PostId
//     LEFT JOIN Votes AS V ON TRP.PostId = V.PostId AND V.VoteTypeId = 8 GROUP BY TRP.PostId, TRP.Title, TRP.Score, TRP.ViewCount, TRP.OwnerDisplayName, C.CommentCount)
// SELECT PD.Title, PD.Score, PD.ViewCount, PD.OwnerDisplayName, PD.CommentCount, PD.TotalBounty FROM PostDetails AS PD ORDER BY PD.Score DESC, PD.ViewCount DESC;
fn q9461(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.is_in([1, 2]))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let b = (&tp).group_by(Ident::<Post>::new()).select(bounty.opt()).fold(0i64, |n, x| n + x.flatten().unwrap_or(0));
    let v = drain((&b).and(Ident::<Post>::new().select(&cc).opt()));
    rows(v.into_iter().map(|(p, (b, c))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "owner"]);
        f.extend([oint(c), V::I(b)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// ActivityRanks AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes, TotalBadges, LastPostDate,
//        RANK() OVER (ORDER BY TotalPosts DESC, TotalUpVotes DESC, TotalBadges DESC) AS UserRank FROM UserActivity)
// SELECT ar.UserId, ar.DisplayName, ar.TotalPosts, ar.TotalQuestions, ar.TotalAnswers, ar.TotalUpVotes, ar.TotalDownVotes, ar.TotalBadges, ar.LastPostDate, ar.UserRank
// FROM ActivityRanks ar WHERE ar.UserRank <= 10 ORDER BY ar.UserRank;
fn q9736(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0, 0, 0, 0, 0, 0, i64::MIN], |a, (p, b)| {
            let (n, t, d, v) = match p {
                Some(((t, d), v)) => (1, t, d, v),
                None => (0, 0, i64::MIN, None),
            };
            [a[0] + n, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5] + b.is_some() as i64, a[6].max(d)]
        });
    let r = ranked(drain(&s), |&(_, a)| (Reverse(a[0]), Reverse(a[3]), Reverse(a[5])), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), k)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a[..6].iter().map(|&x| V::I(x)));
        f.extend([tmax(a[6]), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS RankByViews
//     FROM Posts p WHERE p.CreationDate > DATE '2024-10-01' - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(b.Class), 0) AS TotalBadges, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT p.Id AS PostId, COUNT(ph.Id) AS CloseCount FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10 GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, ur.DisplayName, ur.TotalBadges, ur.TotalBounties, COALESCE(cp.CloseCount, 0) AS CloseCount
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.RankByViews <= 5 ORDER BY rp.RankByViews, ur.TotalBadges DESC;
fn q932(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.unwrap_or(0), a[1] + b.flatten().unwrap_or(0)]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&ur)).and((&cp).opt())));
    rows(v.into_iter().map(|(p, ((u, a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVoteCount, DownVoteCount, LastPostDate, RANK() OVER (ORDER BY PostCount DESC) AS UserRank FROM UserActivity)
// SELECT u.UserId, u.DisplayName, u.PostCount, u.QuestionCount, u.AnswerCount, u.UpVoteCount, u.DownVoteCount, u.LastPostDate,
//        CASE WHEN u.UserRank <= 10 THEN 'Top Contributor' WHEN u.UserRank BETWEEN 11 AND 50 THEN 'Active Contributor' ELSE 'Regular User' END AS UserCategory
// FROM TopUsers u WHERE u.UserRank <= 50 ORDER BY u.PostCount DESC, u.LastPostDate DESC;
//
// UserRank reads only COUNT(DISTINCT p.Id), so the users are ranked on post counts first and the post x vote product is driven for those kept.
fn q9581(db: &'static So) -> String {
    let pc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let r = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let tv = rel(r.into_iter().take_while(|x| x.1 <= 50).map(|((u, n), k)| (u, (n, k))).collect());
    let tops: HashIdx<Id<User>, (i64, i64)> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, x)| x)).collect();
    let Post { post_type_id, creation_date, .. } = &db.post;
    let s = db
        .user
        .with(&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some(((t, d), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4].max(d)],
            None => a,
        });
    rows(drain((&tops).and(&s)).into_iter().map(|(u, ((n, k), a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), tmax(a[4])]);
        f.push(V::S(if k <= 10 { "Top Contributor" } else if k <= 50 { "Active Contributor" } else { "Regular User" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT b.Id) AS BadgeCount, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// PopularUsers AS (SELECT UserId, DisplayName, QuestionCount, AnswerCount, UpVotes, DownVotes, BadgeCount, CommentCount, RANK() OVER (ORDER BY UpVotes DESC) AS VoteRank,
//        RANK() OVER (ORDER BY QuestionCount DESC) AS QuestionRank FROM UserActivity)
// SELECT pu.DisplayName, pu.QuestionCount, pu.AnswerCount, pu.UpVotes, pu.DownVotes, pu.BadgeCount, pu.CommentCount, LEAST(pu.VoteRank, pu.QuestionRank) AS OverallRank
// FROM PopularUsers pu WHERE pu.QuestionCount > 0 OR pu.AnswerCount > 0 ORDER BY OverallRank, pu.UpVotes DESC LIMIT 10;
fn q6266(db: &'static So) -> String {
    let per_post = (&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt());
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(per_post).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, _)| match p {
            Some(((t, v), _)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let r = ranked(drain((&s).and(&bc).and(&cc)), |&(_, ((a, _), _))| Reverse(a[2]), false);
    let r = ranked(r, |&((_, ((a, _), _)), _)| Reverse(a[0]), false);
    let tv = rel(r.into_iter().map(|(((u, x), vr), qr)| (u, (x, vr.min(qr)))).collect());
    let ranks: HashIdx<Id<User>, ((([i64; 4], i64), i64), i64)> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, x)| x)).collect();
    let v = drain(db.user.select((&ranks).filt(|x: ((([i64; 4], i64), i64), i64)| x.0 .0 .0[0] > 0 || x.0 .0 .0[1] > 0)));
    let v = top_n(v, |&(_, (((a, _), _), o))| (o, Reverse(a[2])), 10);
    rows(v.into_iter().map(|(u, (((a, b), c), o))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([V::I(b), V::I(c), V::I(o)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes, COALESCE(c.CommentCount, 0) AS CommentCount,
//        COALESCE(a.AnswerCount, 0) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(Id) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId)
// SELECT r.PostId, r.Title, r.CreationDate, r.UpVotes, r.DownVotes, r.CommentCount, r.AnswerCount FROM RankedPosts r WHERE r.Rank <= 5 ORDER BY r.CreationDate DESC LIMIT 20;
fn q7499(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let top = top_n(top, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 20);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let c = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let a = db.post.with(post_type_id.eq(2)).group_by(&db.post.parent).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(Ident::<Post>::new().and((&v).opt()).and((&c).opt()).and((&a).opt())));
    rows(v.into_iter().map(|(_, (((p, v), c), a))| {
        let v = v.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(v[0]), V::I(v[1]), V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0))]);
        row(f)
    }))
}

// Rewritten (rewrites/8409.sql): the OwnerPostRank ROW_NUMBER is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS OwnerPostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2022-01-01' GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate, p.OwnerUserId),
// FilteredPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, CommentCount, VoteCount FROM RankedPosts WHERE OwnerPostRank <= 5)
// SELECT fp.PostId, fp.Title, fp.OwnerDisplayName, fp.CreationDate, fp.CommentCount, fp.VoteCount, t.TagName, pt.Name AS PostTypeName, COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM FilteredPosts fp LEFT JOIN Posts p ON fp.PostId = p.Id LEFT JOIN Tags t ON t.ExcerptPostId = p.Id LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON p.OwnerUserId = b.UserId ORDER BY fp.CreationDate DESC, fp.VoteCount DESC;
//
// OwnerPostRank reads only base columns, so each owner's five newest posts are picked first and the comment x vote product is driven for those alone.
fn q8409(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(ts(2022, 1, 1, 0, 0, 0))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&cc).and(&vc).and((&excerpt).opt()).and(owner_user.select(&bc).opt()));
    rows(v.into_iter().map(|(p, (((c, n), t), b))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([V::I(c), V::I(n), t.map_or(V::Null, |t| V::S(db.tag.tag_name.get(t).unwrap())), post_fields(db, p, &["type"]).remove(0), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT bh.Id) AS BadgeCount,
//        RANK() OVER (ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) DESC) AS ScoreRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges bh ON p.OwnerUserId = bh.UserId GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.UpVotes, rp.DownVotes, rp.CommentCount, rp.BadgeCount, rp.OwnerUserId FROM RankedPosts rp WHERE rp.ScoreRank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.UpVotes, tp.DownVotes, tp.CommentCount, tp.BadgeCount, u.DisplayName AS OwnerDisplayName
// FROM TopPosts tp JOIN Users u ON tp.OwnerUserId = u.Id ORDER BY tp.UpVotes DESC, tp.CreationDate DESC;
fn q9691(db: &'static So) -> String {
    let owner_user = &db.post.owner_user;
    let s = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let r = ranked(drain(&s), |&(_, a)| Reverse(a[0] - a[1]), false);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&tp).select((&s).and(&cc).and(&bc).and(owner_user)));
    rows(v.into_iter().map(|(p, (((a, c), b), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(b), user_col(db, u, "name")]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(vs.UpVoteScore, 0) AS UpVoteCount, COALESCE(vs.DownVoteScore, 0) AS DownVoteCount,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteScore, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteScore
//     FROM Votes GROUP BY PostId) vs ON p.Id = vs.PostId WHERE p.PostTypeId = 1),
// LastActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, MAX(p.LastActivityDate) AS LastActiveDate FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.UpVoteCount, rp.DownVoteCount, rp.CommentCount, lau.DisplayName AS OwnerDisplayName, lau.LastActiveDate
// FROM RankedPosts rp JOIN LastActiveUsers lau ON rp.PostId = lau.UserId WHERE rp.RowNum <= 10 ORDER BY rp.CreationDate DESC;
//
// rp.PostId = lau.UserId compares a post id with a user id, so it goes through origid.
fn q7322(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(p, d)| (Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let lau = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.last_activity_date)).fold(i64::MIN, |m, d| m.max(d));
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&cc).and((&vs).opt()).and(origid.select(&uidx).select(Ident::<User>::new().and(&lau))));
    rows(v.into_iter().map(|(p, ((c, a), (u, d)))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), user_col(db, u, "name"), V::T(d)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS rn FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerDisplayName FROM RankedPosts WHERE rn <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
// GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q7884(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStats AS (SELECT o.Id AS OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        COUNT(DISTINCT p.Tags) AS UniqueTags FROM Posts p JOIN Users o ON p.OwnerUserId = o.Id GROUP BY o.Id),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.BadgeCount, ps.TotalPosts, ps.Questions, ps.Answers, ps.UniqueTags,
//        ROW_NUMBER() OVER (ORDER BY us.Reputation DESC, us.BadgeCount DESC) AS Rank FROM UserStats us JOIN PostStats ps ON us.UserId = ps.OwnerUserId)
// SELECT Rank, DisplayName, Reputation, BadgeCount, TotalPosts, Questions, Answers, UniqueTags FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q25667(db: &'static So) -> String {
    let Post { owner_user, post_type_id, tags_str, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.group_by(owner_user).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let ut = db.post.group_by(owner_user).select(tags_str).count_distinct();
    let v = drain((&bc).and(&ps).and((&ut).opt()));
    let v = top_n(v, |&(u, ((b, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(b), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, ((b, a), t)))| {
        let mut f = vec![V::I(i as i64 + 1)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(t.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE u.Reputation > 1000 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Votes v GROUP BY v.PostId),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Badges b GROUP BY b.UserId)
// SELECT rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, pvs.UpVotes, pvs.DownVotes, pvs.TotalVotes, ub.BadgeCount, ub.HighestBadgeClass
// FROM RankedPosts rp LEFT JOIN PostVoteSummary pvs ON rp.Id = pvs.PostId LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId WHERE rp.PostRank <= 3 ORDER BY rp.OwnerUserId, rp.CreationDate DESC;
fn q5502(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.select(rich)));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, i64::MIN), |(n, m), c| (n + 1, m.max(c)));
    let v = drain((&tp).select(Ident::<Post>::new().and((&pvs).opt()).and(owner_user.select(&ub).opt())));
    rows(v.into_iter().map(|(_, ((p, a), b))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match b {
            Some((n, m)) => [V::I(n), V::I(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Author, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank,
//        COUNT(DISTINCT v.Id) AS VoteCount, COUNT(c.Id) AS CommentCount, COUNT(b.Id) AS BadgeCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE p.CreationDate > CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Author, rp.CreationDate, rp.ViewCount, rp.Score, rp.Rank, rp.VoteCount, rp.CommentCount, rp.BadgeCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT fp.PostId, fp.Title, fp.Author, fp.CreationDate, fp.ViewCount, fp.Score, fp.VoteCount, fp.CommentCount, fp.BadgeCount FROM FilteredPosts fp ORDER BY fp.Score DESC, fp.ViewCount DESC;
//
// Rank reads only base columns, so the posts are picked first and the vote x comment x badge product is driven for those alone.
fn q7693(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(current_date(), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).opt().and(comments_of(db).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 2], |a, ((_, c), b)| [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64]);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&s).and(&vc)).into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "score"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5),
// VoteStatistics AS (SELECT p.Id AS PostId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY p.Id)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, vs.TotalVotes, vs.UpVotes, vs.DownVotes, COALESCE(ph.Comment, 'No closure comment') AS ClosureComment,
//        COALESCE(PR.Name, 'No close reason') AS CloseReason
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.Id = ph.PostId AND ph.PostHistoryTypeId = 10 LEFT JOIN CloseReasonTypes PR ON CAST(ph.Comment AS INT) = PR.Id
// LEFT JOIN VoteStatistics vs ON tp.Id = vs.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q9708(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db)).opt()).fold([0i64; 3], |a, n| {
        [a[0] + n.is_some() as i64, a[1] + (n == Some("UpMod")) as i64, a[2] + (n == Some("DownMod")) as i64]
    });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(comment.opt().and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason).opt()));
    let v = drain((&vs).and(closes.opt()));
    rows(v.into_iter().map(|(p, (a, h))| {
        let (c, r) = h.unwrap_or((None, None));
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        f.extend([V::S(c.unwrap_or("No closure comment")), V::S(r.unwrap_or("No close reason"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(v.Id) AS UpvoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.RankScore, rp.CommentCount, rp.UpvoteCount FROM RankedPosts rp WHERE rp.RankScore <= 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CreationDate, tp.CommentCount, tp.UpvoteCount, u.DisplayName AS OwnerDisplayName, u.Reputation,
//        (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = tp.PostId AND ph.PostHistoryTypeId IN (10, 11, 12)) AS HistoryCount
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// tp.PostId = u.Id compares a post id with a user id, so it goes through origid.
fn q9058(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let uc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up.opt())).fold(0i64, |n, (_, v)| n + v.is_some() as i64);
    let hist = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11, 12])));
    let hc = (&tp).group_by(Ident::<Post>::new()).select(hist.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&cc).and(&uc).and(&hc).and(origid.select(&uidx)));
    rows(v.into_iter().map(|(p, (((c, n), h), u))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(h));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS TotalComments, AVG(C.Score) AS AverageCommentScore FROM Comments C GROUP BY C.PostId),
// FinalResults AS (SELECT TP.PostId, TP.Title, TP.CreationDate, TP.Score, TP.ViewCount, TP.OwnerDisplayName, PC.TotalComments, PC.AverageCommentScore
//     FROM TopPosts TP LEFT JOIN PostComments PC ON TP.PostId = PC.PostId)
// SELECT *, (CASE WHEN Score >= 1000 THEN 'High Score' WHEN Score >= 500 THEN 'Medium Score' ELSE 'Low Score' END) AS ScoreCategory FROM FinalResults ORDER BY Score DESC, ViewCount DESC;
fn q8076(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let v = drain((&tp).select(Ident::<Post>::new().and((&pc).opt())));
    rows(v.into_iter().map(|(_, (p, c))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(match c {
            Some(a) => [V::I(a[0]), avg(a[1], a[0])],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if s >= 1000 { "High Score" } else if s >= 500 { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS Author, t.TagName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(COUNT(ph.Id), 0) AS EditHistoryCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Tags t ON t.Id = (SELECT MIN(Id) FROM Tags WHERE Tags.ExcerptPostId = p.Id)
//     LEFT JOIN Votes v ON v.PostId = p.Id LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN PostHistory ph ON ph.PostId = p.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, u.DisplayName, t.TagName),
// RankedPosts AS (SELECT pd.*, ROW_NUMBER() OVER (ORDER BY UpVotes DESC, CommentCount DESC, CreationDate ASC) AS Rank FROM PostDetails pd)
// SELECT rp.Rank, rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.Author, rp.TagName, rp.UpVotes, rp.DownVotes, rp.CommentCount, rp.EditHistoryCount
// FROM RankedPosts rp WHERE rp.Rank <= 10 ORDER BY rp.Rank;
fn q28254(db: &'static So) -> String {
    let s = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 4], |a, ((t, c), h)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64, a[3] + h.is_some() as i64]);
    let top = top_n(drain(&s), |&(p, a)| (Reverse(a[0]), Reverse(a[2]), db.post.creation_date.get(p).unwrap(), p), 10);
    let min_tag = db.tag.group_by(&db.tag.excerpt_post).select(&db.tag.origid).fold(i64::MAX, |m, o| m.min(o));
    let tidx: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let tv = rel(top.into_iter().enumerate().map(|(i, (p, a))| (p, (i as i64 + 1, a))).collect());
    let v = drain((&tv).select(Same::<(Id<Post>, (i64, [i64; 4]))>::new().and(Same::<(Id<Post>, (i64, [i64; 4]))>::new().map(|(p, _)| p).select((&min_tag).select(&tidx)).opt())));
    rows(v.into_iter().map(|(_, ((p, (r, a)), t))| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["id", "title", "body", "created", "owner"]));
        f.push(t.map_or(V::Null, |t| V::S(db.tag.tag_name.get(t).unwrap())));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId
//     WHERE rp.Rank <= 5 GROUP BY rp.PostId, rp.Title, rp.OwnerDisplayName, rp.Score)
// SELECT p.Title, p.OwnerDisplayName, p.Score, p.CommentCount, p.UpVotes, p.DownVotes, (p.UpVotes - p.DownVotes) AS NetVotes,
//        CASE WHEN p.Score > 10 THEN 'High' WHEN p.Score BETWEEN 5 AND 10 THEN 'Medium' ELSE 'Low' END AS ScoreCategory FROM TopPosts p ORDER BY p.Score DESC, p.CommentCount DESC;
fn q5009(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "owner", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2]), V::S(if sc > 10 { "High" } else if sc >= 5 { "Medium" } else { "Low" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS Rank,
//        u.DisplayName AS OwnerDisplayName FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, COUNT(c.Id) AS NumberOfComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
// GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q5894(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankByType
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2) AND p.Score > 0),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.RankByType <= 5),
// PostWithVotes AS (SELECT tp.*, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM TopPosts tp LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName)
// SELECT pwv.PostId, pwv.Title, pwv.CreationDate, pwv.Score, pwv.ViewCount, pwv.OwnerDisplayName, pwv.UpVotes, pwv.DownVotes, (pwv.UpVotes - pwv.DownVotes) AS NetVotes
// FROM PostWithVotes pwv ORDER BY pwv.Score DESC, pwv.ViewCount DESC;
fn q7404(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// Rewritten (rewrites/27505.sql): the ORDER BY moved out of the FilteredPosts CTE and tie-broken on PostId.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS OwnerName, COUNT(a.Id) AS AnswerCount, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 /* Considering only questions */ GROUP BY p.Id, p.Title, p.Body, p.CreationDate, u.DisplayName),
// FilteredPosts AS (SELECT rp.*, (UpVoteCount - DownVoteCount) AS NetVoteCount FROM RankedPosts rp WHERE AnswerCount > 0)
// SELECT fp.PostId, fp.Title, fp.OwnerName, fp.CreationDate, fp.AnswerCount, fp.CommentCount, fp.NetVoteCount FROM FilteredPosts fp
// WHERE fp.rn = 1 AND fp.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' ORDER BY fp.NetVoteCount DESC, fp.PostId LIMIT 10;
//
// rn partitions by p.Id, so it is 1 on every row.
fn q27505(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, ((x, c), t)| [a[0] + x.is_some() as i64, a[1] + c.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let v = top_n(drain((&s).filt(|a| a[0] > 0)), |&(p, a)| (Reverse(a[2] - a[3]), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2] - a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName AS OwnerName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, AnswerCount, CommentCount, FavoriteCount, OwnerName FROM RankedPosts WHERE Rank <= 5)
// SELECT t.PostId, t.Title, t.CreationDate, t.ViewCount, t.Score, t.AnswerCount, t.CommentCount, t.FavoriteCount, t.OwnerName, COUNT(c.Id) AS CommentCountTotal,
//        COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
// FROM TopPosts t LEFT JOIN Comments c ON t.PostId = c.PostId LEFT JOIN Votes v ON t.PostId = v.PostId AND v.VoteTypeId = 8
// GROUP BY t.PostId, t.Title, t.CreationDate, t.ViewCount, t.Score, t.AnswerCount, t.CommentCount, t.FavoriteCount, t.OwnerName ORDER BY t.Score DESC, t.CreationDate DESC;
fn q5290(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "favorites", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Rank, u.DisplayName AS OwnerDisplayName, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(b.BadgeCount, 0) AS BadgeCount
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON rp.Id = c.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON u.Id = b.UserId)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.BadgeCount FROM TopPosts tp WHERE tp.Rank <= 5 ORDER BY tp.Score DESC;
fn q8961(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and((&bc).opt()))).and((&cc).opt())));
    rows(v.into_iter().map(|(_, ((p, (u, b)), c))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([user_col(db, u, "name"), V::I(c.unwrap_or(0)), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT V.Id) AS VoteCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.Reputation),
// PostStatistics AS (SELECT P.Id AS PostId, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.FavoriteCount, P.CreationDate, U.Reputation AS OwnerReputation,
//        COALESCE((SELECT COUNT(*) FROM Votes WHERE PostId = P.Id), 0) AS VoteCount FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id),
// TopPosts AS (SELECT PS.PostId, PS.Score, PS.ViewCount, PS.AnswerCount, PS.CommentCount, PS.FavoriteCount, PS.CreationDate, PS.OwnerReputation, PS.VoteCount,
//        ROW_NUMBER() OVER (ORDER BY PS.Score DESC, PS.ViewCount DESC) AS Rank FROM PostStatistics PS)
// SELECT U.UserId, U.Reputation AS UserReputation, COUNT(TP.PostId) AS TopPostCount, AVG(TP.Score) AS AvgPostScore, AVG(TP.ViewCount) AS AvgPostViewCount
// FROM UserReputation U JOIN TopPosts TP ON U.UserId = TP.OwnerReputation WHERE TP.Rank <= 100 GROUP BY U.UserId, U.Reputation ORDER BY U.Reputation DESC;
//
// U.UserId = TP.OwnerReputation compares a user id with a reputation, so it goes through origid. PostCount and VoteCount are never read.
fn q14241(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let top = top_n(drain(score), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 100);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let g = (&tp).group_by(owner_user.select(&db.user.reputation).select(&uidx)).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    rows(drain(&g).into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, COALESCE(B.BadgeCount, 0) AS BadgeCount, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank
//     FROM Users U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON U.Id = B.UserId),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount FROM Posts P GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT US.DisplayName, US.Reputation, PS.PostCount, PS.TotalScore, PS.AvgViewCount, RANK() OVER (ORDER BY PS.TotalScore DESC) AS ScoreRank
//     FROM UserStats US JOIN PostStats PS ON US.UserId = PS.OwnerUserId)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.TotalScore, TU.AvgViewCount,
//        CASE WHEN TU.ScoreRank <= 10 THEN 'Top Contributor' WHEN TU.ScoreRank <= 50 THEN 'Moderate Contributor' ELSE 'New Contributor' END AS ContributionLevel
// FROM TopUsers TU WHERE TU.Reputation > (SELECT AVG(Reputation) FROM Users) OR (TU.PostCount > 5 AND TU.TotalScore > 20) ORDER BY TU.TotalScore DESC LIMIT 20;
fn q883(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let (rs, rn) = db.user.select(&db.user.reputation).fold_flat((0i128, 0i128), |(s, n), r| (s + r as i128, n + 1));
    let r = ranked(drain(&ps), |&(_, a)| Reverse(a[1]), false);
    let tv = rel(r.into_iter().map(|((u, a), k)| (u, (a, k))).collect());
    let ranks: HashIdx<Id<User>, ([i64; 4], i64)> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, x)| x)).collect();
    let keep = (&db.user.reputation).and(&ranks).filt(move |(r, (a, _)): (i64, ([i64; 4], i64))| r as i128 * rn > rs || (a[0] > 5 && a[1] > 20));
    let v = top_n(drain(db.user.select(keep)), |&(_, (_, (a, _)))| Reverse(a[1]), 20);
    rows(v.into_iter().map(|(u, (_, (a, k)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::S(if k <= 10 { "Top Contributor" } else if k <= 50 { "Moderate Contributor" } else { "New Contributor" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, RANK() OVER (ORDER BY PostCount DESC) AS PostRank,
//        RANK() OVER (ORDER BY UpVotes DESC) AS UpVoteRank FROM UserActivity)
// SELECT t.UserId, t.DisplayName, t.PostCount, t.QuestionCount, t.AnswerCount, t.UpVotes, t.DownVotes, (SELECT COUNT(*) FROM TopUsers WHERE PostRank <= t.PostRank) AS TotalTopPosters,
//        (SELECT COUNT(*) FROM TopUsers WHERE UpVoteRank <= t.UpVoteRank) AS TotalTopUpvoted
// FROM TopUsers t WHERE t.PostCount > 10 ORDER BY t.PostCount DESC, t.UpVotes DESC FETCH FIRST 10 ROWS ONLY;
//
// The two correlated counts are taken for the ten output rows only, as a select_where over the distinct ranks.
fn q5533(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let r = ranked(drain((&pc).and(&s)), |&(_, (n, _))| Reverse(n), false);
    let r = ranked(r, |&((_, (_, a)), _)| Reverse(a[2]), false);
    let tv = rel(r.into_iter().map(|(((u, _), pr), ur)| (u, pr, ur)).collect());
    let prank: HashIdx<Id<User>, i64> = (&tv).map(|(u, _, _)| u).inv().select((&tv).map(|(_, r, _)| r)).collect();
    let urank: HashIdx<Id<User>, i64> = (&tv).map(|(u, _, _)| u).inv().select((&tv).map(|(_, _, r)| r)).collect();
    let by_pr: HashIdx<i64, Id<User>> = (&tv).map(|(_, r, _)| r).inv().select((&tv).map(|(u, _, _)| u)).collect();
    let by_ur: HashIdx<i64, Id<User>> = (&tv).map(|(_, _, r)| r).inv().select((&tv).map(|(u, _, _)| u)).collect();
    let top = top_n(drain((&pc).filt(|n| n > 10).and(&s)), |&(u, (n, a))| (Reverse(n), Reverse(a[2]), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let tp = (&tu).group_by(Ident::<User>::new()).select((&prank).select_where(&by_pr, |r: i64, d: i64| d <= r)).fold(0i64, |n, _| n + 1);
    let tq = (&tu).group_by(Ident::<User>::new()).select((&urank).select_where(&by_ur, |r: i64, d: i64| d <= r)).fold(0i64, |n, _| n + 1);
    rows(drain((&pc).and(&s).and(&tp).and(&tq)).into_iter().map(|(u, (((n, a), x), y))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(x), V::I(y)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerName, COUNT(a.Id) AS AnswerCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.LastActivityDate DESC) AS ActivityRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.LastActivityDate),
// TopPosts AS (SELECT PostId, Title, CreationDate, OwnerName, AnswerCount, UpVotes, DownVotes FROM RankedPosts WHERE ActivityRank = 1 ORDER BY UpVotes - DownVotes DESC, AnswerCount DESC LIMIT 10)
// SELECT tp.Title, tp.OwnerName, tp.CreationDate, tp.AnswerCount, tp.UpVotes, tp.DownVotes, COALESCE(b.Name, 'No Badge') AS UserBadge
// FROM TopPosts tp LEFT JOIN Badges b ON tp.OwnerName = (SELECT DisplayName FROM Users WHERE Id = b.UserId) ORDER BY tp.CreationDate DESC;
//
// ActivityRank partitions by p.Id, so it is 1 on every row.
fn q9248(db: &'static So) -> String {
    let owner_user = &db.post.owner_user;
    let s = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (x, t)| [a[0] + x.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let top = top_n(drain(&s), |&(p, a)| (Reverse(a[1] - a[2]), Reverse(a[0]), p), 10);
    let tv = rel(top);
    let tp: HashIdx<Id<Post>, [i64; 3]> = (&tv).map(|(p, _)| p).inv().select((&tv).map(|(_, a)| a)).collect();
    let by_name: HashIdx<Str, Id<Badge>> = db.badge.select((&db.badge.user).select(&db.user.display_name)).inv().collect();
    let v = drain(db.post.with(&tp).select((&tp).and(owner_user.select(&db.user.display_name).select(&by_name).opt())));
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        f.extend(a.map(V::I));
        f.push(V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY p.Id, u.DisplayName, p.Title, p.CreationDate, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerName, rp.CommentCount, rp.Upvotes, rp.Downvotes, rp.Rank FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.Title, tp.OwnerName, tp.CommentCount, tp.Upvotes, tp.Downvotes,
//        CASE WHEN tp.Upvotes - tp.Downvotes > 0 THEN 'Positive' WHEN tp.Upvotes - tp.Downvotes < 0 THEN 'Negative' ELSE 'Neutral' END AS Sentiment
// FROM TopPosts tp ORDER BY tp.Upvotes - tp.Downvotes DESC, tp.CommentCount DESC;
//
// Rank reads only base columns, so the posts are picked first and the comment x vote product is driven for those alone.
fn q9779(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH PostDetails AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName, P.CreationDate, P.LastActivityDate,
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Posts A ON P.Id = A.ParentId
//     WHERE P.PostTypeId IN (1, 2) GROUP BY P.Id, P.Title, P.ViewCount, P.Score, U.DisplayName, P.CreationDate, P.LastActivityDate),
// TopPosts AS (SELECT PostId, Title, ViewCount, Score, OwnerDisplayName, CreationDate, LastActivityDate, CommentCount, AnswerCount, RANK() OVER (ORDER BY Score DESC) AS RankByScore,
//        RANK() OVER (ORDER BY ViewCount DESC) AS RankByViews FROM PostDetails)
// SELECT PostId, Title, ViewCount, Score, OwnerDisplayName, CreationDate, LastActivityDate, CommentCount, AnswerCount, RankByScore, RankByViews
// FROM TopPosts WHERE RankByScore <= 10 OR RankByViews <= 10 ORDER BY RankByScore, RankByViews;
//
// Both ranks read only base columns, so they are taken first and the comment x answer product is driven for the posts they keep.
fn q14546(db: &'static So) -> String {
    let Post { score, view_count, post_type_id, .. } = &db.post;
    let r = ranked(drain(db.post.with(post_type_id.is_in([1, 2])).select(score)), |&(_, s)| Reverse(s), false);
    let r = ranked(r, |&((p, _), _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, false);
    let tv = rel(r.into_iter().filter(|&((_, s), w)| s <= 10 || w <= 10).map(|(((p, _), s), w)| (p, (s, w))).collect());
    let tops: HashIdx<Id<Post>, (i64, i64)> = (&tv).map(|(p, _)| p).inv().select((&tv).map(|(_, x)| x)).collect();
    let s = db
        .post
        .with(&tops)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).opt()))
        .fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    rows(drain((&s).and(&tops)).into_iter().map(|(p, (a, (sr, wr)))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "owner", "created", "activity"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(sr), V::I(wr)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(a.AnswerCount, 0) AS AnswerCount, COALESCE(c.CommentCount, 0) AS CommentCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC, p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.AnswerCount, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.Title, tp.Score, tp.AnswerCount, tp.CommentCount, u.DisplayName AS OwnerName, u.Reputation AS OwnerReputation
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id ORDER BY tp.Score DESC, tp.AnswerCount DESC;
//
// tp.PostId = u.Id compares a post id with a user id, so it goes through origid.
fn q9826(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(score.get(p).unwrap())), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let a = db.post.with(post_type_id.eq(2)).group_by(&db.post.parent).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let c = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&tp).select(Ident::<Post>::new().and((&a).opt()).and((&c).opt()).and(origid.select(&uidx))));
    rows(v.into_iter().map(|(_, (((p, a), c), u))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(a.unwrap_or(0)), V::I(c.unwrap_or(0))]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(P.Score) AS TotalScore,
//        SUM(P.ViewCount) AS TotalViews FROM Posts P GROUP BY P.OwnerUserId),
// UserPerformance AS (SELECT UR.DisplayName, UR.Reputation, PS.QuestionCount, PS.AnswerCount, PS.TotalScore, PS.TotalViews,
//        COALESCE(PS.QuestionCount * 3 + PS.AnswerCount * 2 + PS.TotalScore, 0) AS PerformanceScore FROM UserReputation UR LEFT JOIN PostStats PS ON UR.UserId = PS.OwnerUserId),
// TopUsers AS (SELECT *, RANK() OVER (ORDER BY PerformanceScore DESC) AS ScoreRank FROM UserPerformance)
// SELECT U.DisplayName, U.Reputation, U.QuestionCount, U.AnswerCount, U.TotalScore, U.TotalViews, U.PerformanceScore,
//        CASE WHEN U.ScoreRank <= 10 THEN 'Top Performer' ELSE 'Regular User' END AS UserCategory
// FROM TopUsers U WHERE U.Reputation > 5000 AND U.QuestionCount > 5 ORDER BY U.PerformanceScore DESC;
fn q3666(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 5], |a, ((t, s), w)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
    });
    let up = drain(db.user.select((&ps).opt()));
    let perf = |a: &Option<[i64; 5]>| a.map_or(0, |a| a[0] * 3 + a[1] * 2 + a[2]);
    let r = ranked(up, |(_, a)| Reverse(perf(a)), false);
    let tv = rel(r.into_iter().map(|((u, a), k)| (u, (a, k))).collect());
    let ranks: HashIdx<Id<User>, (Option<[i64; 5]>, i64)> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, x)| x)).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(5000)).select((&ranks).filt(|(a, _): (Option<[i64; 5]>, i64)| a.map_or(false, |a| a[0] > 5))));
    rows(v.into_iter().map(|(u, (a, k))| {
        let a = a.unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), V::I(a[0] * 3 + a[1] * 2 + a[2]), V::S(if k <= 10 { "Top Performer" } else { "Regular User" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AggregateData AS (SELECT rp.OwnerDisplayName, COUNT(rp.PostId) AS TotalPosts, SUM(rp.Score) AS TotalScore, SUM(rp.ViewCount) AS TotalViews FROM RankedPosts rp WHERE rp.PostRank <= 5
//     GROUP BY rp.OwnerDisplayName),
// ClosedPosts AS (SELECT p.OwnerUserId, COUNT(ph.Id) AS TotalClosed FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10 GROUP BY p.OwnerUserId)
// SELECT ad.OwnerDisplayName, ad.TotalPosts, ad.TotalScore, ad.TotalViews, COALESCE(cp.TotalClosed, 0) AS TotalClosedPosts
// FROM AggregateData ad LEFT JOIN ClosedPosts cp ON ad.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = cp.OwnerUserId) ORDER BY ad.TotalScore DESC, ad.TotalPosts DESC;
fn q6066(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ad = (&tp).group_by(owner_user.select(&db.user.display_name)).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by((&db.post_history.post).select(owner_user)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = db.user.with(&cp).select(&db.user.display_name).inv().collect();
    let v = drain((&ad).and(Same::<Str>::new().select((&by_name).select(&cp)).opt()));
    rows(v.into_iter().map(|(n, (a, c))| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(c.unwrap_or(0))])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL),
// ClosedPostCounts AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseOpenCount FROM PostHistory ph GROUP BY ph.PostId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT rp.Title, rp.Score, rp.CreationDate, u.DisplayName, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, cpc.CloseOpenCount
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN ClosedPostCounts cpc ON rp.Id = cpc.PostId LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// WHERE rp.Rank = 1 AND (ub.GoldBadges > 0 OR ub.SilverBadges > 1 OR ub.BronzeBadges > 2) ORDER BY rp.Score DESC NULLS LAST FETCH FIRST 10 ROWS ONLY;
fn q191(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cpc = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold(0i64, |n, t| n + (t == 10 || t == 11) as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and((&ub).filt(|a| a[0] > 0 || a[1] > 1 || a[2] > 2))).and((&cpc).opt())));
    let v = top_n(v, |&(p, _)| Reverse(score.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, ((u, a), c))| {
        let mut f = post_fields(db, p, &["title", "score", "created"]);
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        f.push(oint(c));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.LastActivityDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= DATE('2024-10-01') - INTERVAL '30 days'),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CreationDate, LastActivityDate, OwnerUserId, OwnerDisplayName FROM RankedPosts WHERE RankScore <= 10),
// PostVoteSummary AS (SELECT p.Id AS PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CreationDate, tp.LastActivityDate, tp.OwnerUserId, tp.OwnerDisplayName, pvs.UpVotes, pvs.DownVotes
// FROM TopPosts tp JOIN PostVoteSummary pvs ON tp.PostId = pvs.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q7401(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain(&pvs).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "activity", "owner_id", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.Score > 5),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount, AVG(v.BountyAmount) AS AvgBounty FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation),
// PostHistories AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT u.DisplayName, u.Reputation, up.BadgeCount, up.AvgBounty, rp.Title, rp.CreationDate, rp.Score, ph.CloseCount, ph.ReopenCount
// FROM Users u JOIN UserReputation up ON u.Id = up.UserId JOIN RankedPosts rp ON u.Id = rp.PostId LEFT JOIN PostHistories ph ON rp.PostId = ph.PostId
// WHERE up.Reputation > 1000 AND ph.CloseCount > 0 ORDER BY up.Reputation DESC, rp.CreationDate DESC LIMIT 50;
//
// u.Id = rp.PostId compares a user id with a post id, so it goes through origid. PostRank is never read.
fn q21(db: &'static So) -> String {
    let Post { score, origid, creation_date, .. } = &db.post;
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let ur = rich()
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 2], |a, (_, b)| {
            let b = b.flatten();
            [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0)]
        });
    let bc = rich().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ph = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain(db.post.with(score.gt(5)).select(origid.select(&uidx).select(Ident::<User>::new().and(&ur).and(&bc)).and((&ph).filt(|a| a[0] > 0))));
    let v = top_n(v, |&(p, (((u, _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(p, (((u, a), b), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), avg(a[1], a[0])]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(h[0]), V::I(h[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, p.OwnerUserId
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Date >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY b.UserId),
// PostVoteStats AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.Score, rp.ViewCount, up.BadgeCount, pvs.UpVotes, pvs.DownVotes
// FROM RankedPosts rp LEFT JOIN UserBadges up ON rp.OwnerUserId = up.UserId LEFT JOIN PostVoteStats pvs ON rp.PostId = pvs.PostId WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.CreationDate DESC;
fn q6054(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.badge.with((&db.badge.date).ge(add_years(t0, -1))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&ub).opt()).and((&pvs).opt())));
    rows(v.into_iter().map(|(_, ((p, b), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        f.push(oint(b));
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(COUNT(a.Id), 0) AS AnswerCount, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, ua.DisplayName AS TopUser, ua.UpVotes, ua.DownVotes, ua.BadgeCount
// FROM RankedPosts rp JOIN UserActivity ua ON ua.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId LIMIT 1) WHERE rp.Rank <= 10 ORDER BY rp.Rank;
//
// Rank reads only base columns, so the ten questions are picked first; UserActivity is taken for their owners alone.
fn q6390(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let r = ranked(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap())), false);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt().and(comments_of(db).opt())).fold([0i64; 2], |a, (x, c)| [a[0] + x.is_some() as i64, a[1] + c.is_some() as i64]);
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]);
    rows(drain((&s).and(owner_user.select(Ident::<User>::new().and(&ua)))).into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), user_col(db, u, "name"), V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId IN (1, 2) THEN P.Score ELSE 0 END) AS TotalScore,
//        SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalScore, GoldBadges, SilverBadges, BronzeBadges, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank
//     FROM UserStats)
// SELECT Rank, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalScore, GoldBadges, SilverBadges, BronzeBadges FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q9565(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(0));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score)).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, c)| {
            let (t, s) = p.unwrap_or((0, 0));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + if t == 1 || t == 2 { s } else { 0 }, a[3] + (c == Some(1)) as i64, a[4] + (c == Some(2)) as i64, a[5] + (c == Some(3)) as i64]
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let top = top_n(drain((&s).and(&pc)), |&(u, (a, _))| (Reverse(a[2]), u), 10);
    rows(top.into_iter().enumerate().map(|(i, (u, (a, n)))| {
        let mut f = vec![V::I(i as i64 + 1)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedQuestions,
//        AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AverageUpvotes, AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AverageDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AcceptedQuestions, AverageUpvotes, AverageDownvotes, RANK() OVER (ORDER BY TotalPosts DESC) AS Rank
//     FROM UserPostStats WHERE TotalPosts > 0)
// SELECT u.Id, u.DisplayName, u.Reputation, u.CreationDate, ts.TotalPosts, ts.TotalQuestions, ts.TotalAnswers, ts.AcceptedQuestions, ts.AverageUpvotes, ts.AverageDownvotes
// FROM TopUsers ts JOIN Users u ON ts.UserId = u.Id WHERE ts.Rank <= 10 ORDER BY ts.Rank;
fn q26905(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((t, x), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && x.is_some()) as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64, a[6] + 1],
            None => {
                let mut a = a;
                a[6] += 1;
                a
            }
        });
    let r = ranked(drain((&s).filt(|a| a[0] > 0)), |&(_, a)| Reverse(a[0]), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[6]), avg(a[5], a[6])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.CreationDate >= '2020-01-01' GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalAnswers, TotalQuestions, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalAnswers, TU.TotalQuestions, TU.TotalUpVotes, TU.TotalDownVotes, PCT.Name AS PostCategory
// FROM TopUsers TU JOIN Posts P ON TU.UserId = P.OwnerUserId JOIN PostTypes PCT ON P.PostTypeId = PCT.Id WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC, TU.TotalPosts DESC;
//
// ReputationRank reads only Reputation, so the users are ranked first and the post x vote product is driven for those kept.
fn q5630(db: &'static So) -> String {
    let r = ranked(drain(db.user.with((&db.user.creation_date).ge(ts(2020, 1, 1, 0, 0, 0))).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&s).and(&pc).and(posts_of(db).select(ptype_name(db)))).into_iter().map(|(u, ((a, n), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::S(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT rp.*, (UpVoteCount - DownVoteCount) AS NetVoteScore, CASE WHEN p.PostTypeId = 1 THEN 'Question' WHEN p.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END AS PostType
//     FROM RankedPosts rp JOIN Posts p ON rp.PostId = p.Id WHERE Rank <= 10)
// SELECT PostId, Title, CreationDate, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount, NetVoteScore, PostType FROM TopPosts ORDER BY NetVoteScore DESC, CreationDate DESC;
//
// Rank reads only base columns, so the posts are picked first and the comment x vote product is driven for those alone.
fn q6294(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&s).and(post_type_id)).into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2]), V::S(if t == 1 { "Question" } else if t == 2 { "Answer" } else { "Other" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, pt.Name, p.Title, p.CreationDate, p.ViewCount),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.CommentCount, tp.UpVotes, tp.DownVotes,
//        CASE WHEN tp.UpVotes > tp.DownVotes THEN 'Positive' WHEN tp.UpVotes < tp.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM TopPosts tp ORDER BY tp.ViewCount DESC;
fn q6111(db: &'static So) -> String {
    let Post { creation_date, view_count, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&s).and(ptype_name(db)));
    let top = top_per(v, |&(_, (_, t))| t, |&(p, (a, _))| {
        let w = view_count.get(p);
        (Reverse(a[1]), w.is_none(), Reverse(w), p)
    }, 10, false);
    rows(top.into_iter().map(|(p, (a, _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopBadges AS (SELECT b.UserId, b.Name AS BadgeName, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId, b.Name),
// RankedBadges AS (SELECT UserId, BadgeName, BadgeCount, RANK() OVER (PARTITION BY UserId ORDER BY BadgeCount DESC) AS BadgeRank FROM TopBadges)
// SELECT us.DisplayName, us.Reputation, us.PostCount, us.QuestionCount, us.AnswerCount, us.UpVotes, us.DownVotes, rb.BadgeName, rb.BadgeCount
// FROM UserStats us LEFT JOIN RankedBadges rb ON us.UserId = rb.UserId AND rb.BadgeRank = 1 WHERE us.Reputation > 1000 ORDER BY us.Reputation DESC, us.PostCount DESC;
fn q6289(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let s = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tb = db.badge.group_by((&db.badge.user).and(&db.badge.name)).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let top = top_per(drain(&tb), |&((u, _), _)| u, |&(_, n)| Reverse(n), 1, true);
    let tv = rel(top.into_iter().map(|((u, name), n)| (u, (name, n))).collect());
    let rb: HashIdx<Id<User>, (Str, i64)> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, x)| x)).collect();
    rows(drain((&pc).and(&s).and((&rb).opt())).into_iter().map(|(u, ((n, a), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend(match b {
            Some((name, c)) => [V::S(name), V::I(c)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, VoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT t.*, pt.Name AS PostTypeName, bh.Name AS BadgeName, (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = t.PostId) AS EditHistoryCount
// FROM TopPosts t LEFT JOIN PostTypes pt ON t.PostId = pt.Id LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = t.PostId LIMIT 1)
// LEFT JOIN PostHistoryTypes bh ON b.Class = bh.Id ORDER BY t.Score DESC, t.ViewCount DESC;
//
// t.PostId = pt.Id and b.Class = bh.Id join across tables' ids, so both go through origid.
fn q7302(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ptidx: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let htidx: HashIdx<i64, Id<PostHistoryType>> = (&db.post_history_type.origid).inv().collect();
    let badge = owner_user.select(badges_of(db).select((&db.badge.class).select(&htidx).opt())).opt();
    let v = drain((&cc).and(&vc).and(&hc).and(origid.select(&ptidx).opt()).and(badge));
    rows(v.into_iter().map(|(p, ((((c, n), h), t), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(c), V::I(n)]);
        f.push(t.map_or(V::Null, |t| V::S(db.post_type.name.get(t).unwrap())));
        f.push(b.flatten().map_or(V::Null, |t| V::S(db.post_history_type.name.get(t).unwrap())));
        f.push(V::I(h));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(c.Score) AS TotalCommentScore,
//        SUM(p.Score) AS TotalPostScore, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUserActivity AS (SELECT ua.*, RANK() OVER (ORDER BY ua.Reputation DESC, ua.TotalPosts DESC, ua.TotalComments DESC) AS ActivityRank FROM UserActivity ua),
// TopActiveUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalComments, TotalCommentScore, TotalPostScore, GoldBadges, SilverBadges, BronzeBadges FROM RankedUserActivity WHERE ActivityRank <= 10)
// SELECT tua.DisplayName, tua.Reputation, tua.TotalPosts, tua.TotalComments, tua.TotalCommentScore, tua.TotalPostScore,
//        CONCAT('Gold: ', tua.GoldBadges, ', Silver: ', tua.SilverBadges, ', Bronze: ', tua.BronzeBadges) AS BadgeSummary
// FROM TopActiveUsers tua ORDER BY tua.Reputation DESC;
//
// ActivityRank reads only Reputation and the two distinct counts, so the users are ranked on those first and the post x comment x badge product is driven for those kept.
fn q25164(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let r = ranked(drain((&pc).and(&cc)), |&(u, (p, c))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(p), Reverse(c)), false);
    let tv = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let tops: HashIdx<Id<User>, (i64, i64)> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, x)| x)).collect();
    let s = db
        .user
        .with(&tops)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(comments_of(db).select(&db.comment.score).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let (ps, cs) = match p {
                Some((s, c)) => (Some(s), c),
                None => (None, None),
            };
            [a[0] + cs.is_some() as i64, a[1] + cs.unwrap_or(0), a[2] + ps.is_some() as i64, a[3] + ps.unwrap_or(0), a[4] + (b == Some(1)) as i64, a[5] + (b == Some(2)) as i64, a[6] + (b == Some(3)) as i64]
        });
    rows(drain((&tops).and(&s)).into_iter().map(|(u, ((p, c), a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p), V::I(c), nullable(a[1], a[0]), nullable(a[3], a[2]), V::Owned(format!("Gold: {}, Silver: {}, Bronze: {}", a[4], a[5], a[6]))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, COALESCE(SUM(b.Class), 0) AS BadgePoints, MAX(u.Reputation) AS UserReputation FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.PostHistoryTypeId) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.Id AS PostId, rp.Title, u.DisplayName AS Owner, ur.UserReputation, ur.BadgePoints, COALESCE(cp.CloseCount, 0) AS CloseCount,
//        CASE WHEN ur.UserReputation >= 5000 THEN 'Active Contributor' ELSE 'New Contributor' END AS ContributorLevel
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId
// WHERE rp.rn = 1 AND ur.BadgePoints > 0 ORDER BY ur.UserReputation DESC, rp.ViewCount DESC;
fn q3720(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ur = db.user.group_by(Ident::<User>::new()).select((&db.user.reputation).and(badges_of(db).select(&db.badge.class).opt())).fold((0i64, i64::MIN), |(s, m), (r, c)| (s + c.unwrap_or(0), m.max(r)));
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and((&ur).filt(|(s, _): (i64, i64)| s > 0))).and((&cp).opt())));
    rows(v.into_iter().map(|(p, ((u, (s, r)), c))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "name"), V::I(r), V::I(s), V::I(c.unwrap_or(0)), V::S(if r >= 5000 { "Active Contributor" } else { "New Contributor" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVoteCounts AS (SELECT PostId, COUNT(*) FILTER (WHERE VoteTypeId = 2) AS UpVoteCount, COUNT(*) FILTER (WHERE VoteTypeId = 3) AS DownVoteCount FROM Votes GROUP BY PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseReasonCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) AND ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months'
//     GROUP BY ph.PostId)
// SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, RP.OwnerDisplayName, PVC.UpVoteCount, PVC.DownVoteCount, COALESCE(CP.CloseReasonCount, 0) AS CloseReasonCount
// FROM RankedPosts RP LEFT JOIN PostVoteCounts PVC ON RP.PostId = PVC.PostId LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId WHERE RP.ScoreRank <= 5 ORDER BY RP.Score DESC, RP.ViewCount DESC LIMIT 10;
fn q4503(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvc = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11]).and(hd.ge(add_months(t0, -6)))).group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(Ident::<Post>::new().and((&pvc).opt()).and((&cp).opt())));
    let v = top_n(v, |&(_, ((p, _), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10);
    rows(v.into_iter().map(|(_, ((p, a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        f.push(V::I(c.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserVotes AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
//        COUNT(DISTINCT p.Id) AS PostsVotedOn FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotesCount, DownVotesCount, PostsVotedOn, ROW_NUMBER() OVER (ORDER BY UpVotesCount DESC) AS Rnk FROM UserVotes WHERE PostsVotedOn > 10)
// SELECT t.UserId, t.DisplayName, t.UpVotesCount, t.DownVotesCount, (t.UpVotesCount - t.DownVotesCount) AS NetVotes, ph.TotalPostHistoryCount, MAX(ph.MostRecentEditDate) AS MostRecentEdit
// FROM TopUsers t JOIN (SELECT ph.UserId, COUNT(*) AS TotalPostHistoryCount, MAX(ph.CreationDate) AS MostRecentEditDate FROM PostHistory ph GROUP BY ph.UserId) ph ON t.UserId = ph.UserId
// WHERE t.Rnk <= 10 GROUP BY t.UserId, t.DisplayName, t.UpVotesCount, t.DownVotesCount, ph.TotalPostHistoryCount ORDER BY NetVotes DESC;
fn q8962(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let uv = rich().group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.post).opt())).opt()).fold([0i64; 2], |a, v| match v {
        Some((t, _)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64],
        None => a,
    });
    let pv = rich().group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.post).opt()).opt()).buf_fold(|v| distinct_some(v.into_iter().map(|x| x.flatten())));
    let top = top_n(drain((&uv).and((&pv).filt(|n| n > 10))), |&(u, (a, _))| (Reverse(a[0]), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    rows(drain((&tu).select((&uv).and(&ph))).into_iter().map(|(u, (a, (n, d)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::I(n), V::T(d)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT UB.UserId, UB.DisplayName, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(UB.TotalBadges, 0) AS TotalBadges, COALESCE(PS.AverageScore, 0) AS AverageScore
//     FROM UserBadges UB LEFT JOIN PostStats PS ON UB.UserId = PS.OwnerUserId)
// SELECT TU.DisplayName, TU.TotalPosts, TU.TotalBadges, TU.AverageScore, RANK() OVER (ORDER BY TU.TotalPosts DESC, TU.TotalBadges DESC, TU.AverageScore DESC) AS Rank
// FROM TopUsers TU WHERE TU.TotalPosts > 0 ORDER BY Rank LIMIT 10;
fn q102(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.group_by(&db.post.owner_user).select(&db.post.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let r = ranked(drain((&bc).and(&ps)), |&(_, (b, a))| (Reverse(a[0]), Reverse(b), Reverse(fkey(a[1] as f64 / a[0] as f64))), false);
    let r = top_n(r, |x| x.1, 10);
    rows(r.into_iter().map(|((u, (b, a)), k)| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(b), avg(a[1], a[0]), V::I(k)])))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges, COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Score, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentRank FROM Posts P
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// UserScores AS (SELECT U.Id AS UserId, COALESCE(SUM(COALESCE(P.Score, 0)), 0) AS TotalScore, MAX(COALESCE(B.GoldBadges, 0)) AS GoldBadges, MAX(COALESCE(B.SilverBadges, 0)) AS SilverBadges,
//        MAX(COALESCE(B.BronzeBadges, 0)) AS BronzeBadges FROM Users U LEFT JOIN RecentPosts P ON U.Id = P.OwnerUserId LEFT JOIN UserBadges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT UDetails.DisplayName, UDetails.Reputation, U.TotalScore, U.GoldBadges, U.SilverBadges, U.BronzeBadges FROM UserScores U JOIN Users UDetails ON U.UserId = UDetails.Id
// WHERE UDetails.Reputation > 1000 AND (U.GoldBadges > 0 OR U.SilverBadges > 0) ORDER BY U.TotalScore DESC, UDetails.Reputation DESC LIMIT 10;
//
// RecentRank is never read. UserBadges is one row per user, so its MAX is the value itself.
fn q1665(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let recent = posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(&db.post.score);
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(recent.opt().and(&ub))
        .fold([0i64, i64::MIN, i64::MIN, i64::MIN], |a, (s, b)| [a[0] + s.unwrap_or(0), a[1].max(b[0]), a[2].max(b[1]), a[3].max(b[2])]);
    let v = top_n(drain((&us).filt(|a| a[1] > 0 || a[2] > 0)), |&(u, a)| (Reverse(a[0]), Reverse(db.user.reputation.get(u).unwrap())), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS Author, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Author FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostVoteSummary AS (SELECT p.Id AS PostId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// FinalReport AS (SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.Author, vs.TotalVotes, vs.UpVotes, vs.DownVotes FROM TopPosts tp JOIN PostVoteSummary vs ON tp.PostId = vs.PostId)
// SELECT * FROM FinalReport ORDER BY Score DESC, ViewCount DESC;
fn q7335(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&vs).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10),
// PostVotes AS (SELECT p.Id AS PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName, pv.UpVotes, pv.DownVotes
// FROM TopPosts tp JOIN PostVotes pv ON tp.PostId = pv.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q9809(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain(&pv).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Views, UpVotes, DownVotes, PostCount, QuestionCount, AnswerCount, AcceptedAnswerCount,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserStats)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes, U.PostCount, U.QuestionCount, U.AnswerCount, U.AcceptedAnswerCount,
//        RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank, RANK() OVER (ORDER BY U.PostCount DESC) AS PostCountRank
// FROM TopUsers U WHERE U.Rank <= 10 ORDER BY U.Reputation DESC, U.PostCount DESC;
fn q6084(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let s = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && x.is_some()) as i64],
        None => a,
    });
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let top = top_n(drain(&s), |&(u, a)| (Reverse(rep(u)), Reverse(a[0]), u), 10);
    let r = ranked(top, |&(u, _)| Reverse(rep(u)), false);
    let r = ranked(r, |&((_, a), _)| Reverse(a[0]), false);
    rows(r.into_iter().map(|(((u, a), rr), pr)| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews", "uup", "udown"]);
        f.extend(a.map(V::I));
        f.extend([V::I(rr), V::I(pr)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10),
// PostStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, trp.OwnerDisplayName, ps.CommentCount, ps.UpVotes, ps.DownVotes
// FROM TopRankedPosts trp JOIN PostStats ps ON trp.PostId = ps.PostId ORDER BY trp.Score DESC, trp.ViewCount DESC;
//
// PostStats is only read through the join, so the comment x vote product is driven for the top posts alone.
fn q8123(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10),
// PostVoteStats AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName, pvs.UpVotes, pvs.DownVotes, pvs.TotalVotes
// FROM TopPosts tp JOIN PostVoteStats pvs ON tp.PostId = pvs.PostId ORDER BY tp.Score DESC
fn q8475(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]);
    rows(drain(&pvs).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers", "comments", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RecursivePostCounts AS (SELECT p.OwnerUserId, COUNT(*) AS AnswerCount FROM Posts p WHERE p.PostTypeId = 2 GROUP BY p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(rc.AnswerCount, 0) AS AnswerCount FROM Users u LEFT JOIN RecursivePostCounts rc ON u.Id = rc.OwnerUserId),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Badges b GROUP BY b.UserId),
// TopUsers AS (SELECT ur.UserId, ur.Reputation, ur.AnswerCount, ub.BadgeCount, ub.HighestBadgeClass, RANK() OVER (ORDER BY ur.Reputation DESC, ur.AnswerCount DESC) AS Rank
//     FROM UserReputation ur LEFT JOIN UserBadges ub ON ur.UserId = ub.UserId WHERE ur.Reputation > 1000)
// SELECT tu.UserId, tu.Reputation, tu.AnswerCount, COALESCE(tu.BadgeCount, 0) AS BadgeCount,
//        CASE WHEN tu.HighestBadgeClass = 1 THEN 'Gold' WHEN tu.HighestBadgeClass = 2 THEN 'Silver' WHEN tu.HighestBadgeClass = 3 THEN 'Bronze' ELSE 'None' END AS HighestBadge, tu.Rank
// FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Rank;
fn q33056(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let ac = rich().group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(2))).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, i64::MIN), |(n, m), c| (n + 1, m.max(c)));
    let r = ranked(drain((&ac).and((&ub).opt())), |&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a)), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, b)), k)| {
        let (n, m) = b.unwrap_or((0, 0));
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a), V::I(n), V::S(match m {
            1 => "Gold",
            2 => "Silver",
            3 => "Bronze",
            _ => "None",
        }), V::I(k)]);
        row(f)
    }))
}

// WITH RecentQuestions AS (SELECT p.Id AS QuestionId, p.Title, p.CreationDate, p.OwnerUserId, p.AnswerCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserQuestionRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_DATE - INTERVAL '1 MONTH'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation, u.DisplayName),
// TopContributors AS (SELECT UserId, Reputation, DisplayName, TotalBounty, RANK() OVER (ORDER BY Reputation + TotalBounty DESC) AS ContributorRank FROM UserReputation)
// SELECT q.QuestionId, q.Title, u.DisplayName AS OwnerDisplayName, q.CreationDate, q.Score, RANK() OVER (ORDER BY q.Score DESC) AS QuestionRank, COALESCE(ut.ContributorRank, 0) AS ContributorRank
// FROM RecentQuestions q JOIN Users u ON q.OwnerUserId = u.Id LEFT JOIN TopContributors ut ON q.OwnerUserId = ut.UserId WHERE q.UserQuestionRank = 1
// ORDER BY QuestionRank, ContributorRank LIMIT 10 OFFSET 0;
fn q29(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_months(current_date(), -1)))).with(owner_user).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let fq: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tb = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let cr = ranked(drain(&tb), |&(u, b)| Reverse(db.user.reputation.get(u).unwrap() + b), false);
    let cv = rel(cr.into_iter().map(|((u, _), k)| (u, k)).collect());
    let crank: HashIdx<Id<User>, i64> = (&cv).map(|(u, _)| u).inv().select((&cv).map(|(_, k)| k)).collect();
    let v = drain((&fq).select(owner_user.select(Ident::<User>::new().and((&crank).opt()))));
    let v = ranked(v, |&(p, _)| Reverse(score.get(p).unwrap()), false);
    let v = top_n(v, |&((_, (_, c)), q)| (q, c.unwrap_or(0)), 10);
    rows(v.into_iter().map(|((p, (u, c)), q)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(user_col(db, u, "name"));
        f.extend(post_fields(db, p, &["created", "score"]));
        f.extend([V::I(q), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("8256", q8256),
    ("27973", q27973),
    ("9561", q9561),
    ("5728", q5728),
    ("8161", q8161),
    ("9152", q9152),
    ("6573", q6573),
    ("8160", q8160),
    ("136", q136),
    ("7639", q7639),
    ("7824", q7824),
    ("29991", q29991),
    ("8824", q8824),
    ("1041", q1041),
    ("7206", q7206),
    ("27120", q27120),
    ("1379", q1379),
    ("6818", q6818),
    ("8176", q8176),
    ("6912", q6912),
    ("7766", q7766),
    ("5498", q5498),
    ("4080", q4080),
    ("14722", q14722),
    ("3606", q3606),
    ("4650", q4650),
    ("7863", q7863),
    ("9035", q9035),
    ("9322", q9322),
    ("12990", q12990),
    ("42", q42),
    ("2741", q2741),
    ("14054", q14054),
    ("8749", q8749),
    ("8830", q8830),
    ("28777", q28777),
    ("7977", q7977),
    ("9461", q9461),
    ("9736", q9736),
    ("932", q932),
    ("9581", q9581),
    ("6266", q6266),
    ("7499", q7499),
    ("8409", q8409),
    ("9691", q9691),
    ("7322", q7322),
    ("7884", q7884),
    ("25667", q25667),
    ("5502", q5502),
    ("7693", q7693),
    ("9708", q9708),
    ("9058", q9058),
    ("8076", q8076),
    ("28254", q28254),
    ("5009", q5009),
    ("5894", q5894),
    ("7404", q7404),
    ("27505", q27505),
    ("5290", q5290),
    ("8961", q8961),
    ("14241", q14241),
    ("883", q883),
    ("5533", q5533),
    ("9248", q9248),
    ("9779", q9779),
    ("14546", q14546),
    ("9826", q9826),
    ("3666", q3666),
    ("6066", q6066),
    ("191", q191),
    ("7401", q7401),
    ("21", q21),
    ("6054", q6054),
    ("6390", q6390),
    ("9565", q9565),
    ("26905", q26905),
    ("5630", q5630),
    ("6294", q6294),
    ("6111", q6111),
    ("6289", q6289),
    ("7302", q7302),
    ("25164", q25164),
    ("3720", q3720),
    ("4503", q4503),
    ("8962", q8962),
    ("102", q102),
    ("1665", q1665),
    ("7335", q7335),
    ("9809", q9809),
    ("6084", q6084),
    ("8123", q8123),
    ("8475", q8475),
    ("33056", q33056),
    ("29", q29),
];
