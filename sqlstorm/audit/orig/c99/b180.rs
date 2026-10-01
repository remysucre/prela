use harness::prelude::*;
use std::cmp::Reverse;

// WITH RecursivePostCounts AS (SELECT P.Id AS PostId, COUNT(A.Id) AS AnswerCount, COUNT(DISTINCT C.Id) AS CommentCount FROM Posts P
//     LEFT JOIN Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2 LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.PostTypeId = 1 GROUP BY P.Id),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// FilteredPosts AS (SELECT P.Id, P.Title, P.CreationDate, P.LastActivityDate, R.AnswerCount, R.CommentCount, U.Reputation AS OwnerReputation, U.DisplayName AS OwnerDisplayName, P.OwnerUserId
//     FROM Posts P INNER JOIN RecursivePostCounts R ON P.Id = R.PostId INNER JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND (R.AnswerCount > 5 OR R.CommentCount > 10)),
// RankedPosts AS (SELECT FP.*, ROW_NUMBER() OVER (PARTITION BY FP.OwnerUserId ORDER BY FP.LastActivityDate DESC) AS UserRank FROM FilteredPosts FP)
// SELECT F.Title, F.CreationDate, F.LastActivityDate, F.OwnerDisplayName, F.OwnerReputation, COALESCE(B.GoldBadges, 0) AS GoldBadges, COALESCE(B.SilverBadges, 0) AS SilverBadges,
//        COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM RankedPosts F LEFT JOIN UserReputation B ON F.OwnerUserId = B.UserId WHERE F.UserRank <= 3 ORDER BY F.LastActivityDate DESC;
//
// The CreationDate filter on P is applied before the counts, which only changes which questions the counts are computed for. A LastActivityDate tie within an owner goes to the smaller post id.
fn q30664(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, last_activity_date, .. } = &db.post;
    let qs: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).collect();
    let ac = (&qs).group_by(Ident::<Post>::new()).select(answers_of(db).opt().and(comments_of(db).opt())).fold(0i64, |n, (a, _)| n + a.is_some() as i64);
    let cc = (&qs).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&ac).and(&cc).filt(|(a, c)| a > 5 || c > 10).and(owner_user));
    let top = top_per(v, |&(_, (_, u))| u, |&(p, _)| (Reverse(last_activity_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and((&ub).opt()))));
    let v = top_n(v, |&(p, _)| Reverse(last_activity_date.get(p).unwrap()), 0);
    rows(v.into_iter().map(|(p, (u, b))| {
        let mut f = post_fields(db, p, &["title", "created", "activity"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u),
// TopPosts AS (SELECT p.OwnerUserId, COUNT(*) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
//        SUM(p.ViewCount) AS TotalViews FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// PostVoteStatistics AS (SELECT p.OwnerUserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(v.Id) AS TotalVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.OwnerUserId)
// SELECT ur.DisplayName, ur.Reputation, COALESCE(tp.PostCount, 0) AS PostCount, COALESCE(tp.PositivePosts, 0) AS PositivePosts, COALESCE(tp.NegativePosts, 0) AS NegativePosts,
//        COALESCE(tp.TotalViews, 0) AS TotalViews, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes, COALESCE(pvs.TotalVotes, 0) AS TotalVotes,
//        CASE WHEN ur.Reputation > 1000 THEN 'High Reputation' WHEN ur.Reputation BETWEEN 500 AND 1000 THEN 'Moderate Reputation' ELSE 'Low Reputation' END AS ReputationCategory
// FROM UserReputation ur LEFT JOIN TopPosts tp ON ur.UserId = tp.OwnerUserId LEFT JOIN PostVoteStatistics pvs ON ur.UserId = pvs.OwnerUserId
// WHERE ur.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '3 months' OR ur.Reputation IS NULL ORDER BY ur.Reputation DESC, ur.DisplayName LIMIT 20;
fn q187(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let tp = db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + w.unwrap_or(0)]
    });
    let pvs = db.post.with(owner_user).group_by(owner_user).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]
    });
    let v = drain(db.user.with((&db.user.creation_date).lt(add_months(t0, -3))).select((&tp).opt().and((&pvs).opt())));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), db.user.display_name.get(u).unwrap()), 20);
    rows(v.into_iter().map(|(u, (t, p))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(t.unwrap_or([0; 4]).map(V::I));
        f.extend(p.unwrap_or([0; 3]).map(V::I));
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Moderate Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT Users.Id AS UserId, COUNT(CASE WHEN Badges.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Badges.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN Badges.Class = 3 THEN 1 END) AS BronzeBadges FROM Users LEFT JOIN Badges ON Users.Id = Badges.UserId GROUP BY Users.Id),
// RecentPosts AS (SELECT Posts.Id AS PostId, Posts.OwnerUserId, Posts.Title, Posts.CreationDate, Posts.Score,
//        ROW_NUMBER() OVER (PARTITION BY Posts.OwnerUserId ORDER BY Posts.CreationDate DESC) AS PostRank,
//        COUNT(CASE WHEN Votes.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY Posts.Id) AS UpVotesCount,
//        COUNT(CASE WHEN Votes.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY Posts.Id) AS DownVotesCount
//     FROM Posts LEFT JOIN Votes ON Posts.Id = Votes.PostId WHERE Posts.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT U.DisplayName, U.Reputation, COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
//        RP.Title, RP.CreationDate, RP.Score, RP.UpVotesCount, RP.DownVotesCount,
//        CASE WHEN RP.Score IS NULL THEN 'Unknown' WHEN RP.Score > 10 THEN 'Highly Scored' WHEN RP.Score BETWEEN 1 AND 10 THEN 'Moderately Scored' ELSE 'Low Scored' END AS ScoreCategory
// FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN RecentPosts RP ON U.Id = RP.OwnerUserId
// WHERE RP.PostRank = 1 AND (U.Reputation > 100 OR (RP.Score IS NOT NULL AND RP.Score > 0)) ORDER BY U.Reputation DESC, RP.CreationDate DESC LIMIT 50;
//
// PostRank numbers post x vote rows, and every row of one post projects the same columns, so the owner's newest post is picked first (a CreationDate tie goes to the
// smaller post id) and its votes counted afterwards.
fn q21057(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let rp = (&vc).and(owner_user.select(Ident::<User>::new().and(&db.user.reputation))).and(score).filt(|((_, (_, r)), s)| r > 100 || s > 0);
    let v = drain(rp.map(|((a, (u, _)), _)| (a, u)).and(owner_user.select(&ub)));
    let v = top_n(v, |&(p, ((_, u), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(p, ((a, u), b))| {
        let s = score.get(p).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend(a.map(V::I));
        f.push(V::S(if s > 10 { "Highly Scored" } else if s >= 1 { "Moderately Scored" } else { "Low Scored" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS Rank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, pt.Name),
// PostHistoryDetails AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, PHT.Name AS HistoryType, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph JOIN PostHistoryTypes PHT ON ph.PostHistoryTypeId = PHT.Id
//     WHERE (PHT.Id IN (10, 11) OR ph.Comment IS NOT NULL) AND ph.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// SELECT rp.Id AS PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes, COALESCE(phd.UserId, -1) AS LastEditorId,
//        COALESCE(phd.HistoryType, 'No History') AS LastEditType, COALESCE(phd.Comment, 'No Comments') AS LastEditComment, phd.CreationDate AS LastEditDate
// FROM RankedPosts rp LEFT JOIN PostHistoryDetails phd ON rp.Id = phd.PostId AND phd.HistoryRank = 1 WHERE rp.Rank = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 100;
//
// Rank reads only base columns, so each type's newest post is picked first (a tie goes to the smaller post id) and the comment x vote product is driven for those alone.
// A tie on the newest history row of a post goes to the larger history id.
fn q4572(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_days(t0, -30))).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let keep = Ident::<PostHistory>::new().with(post_history_type_id.and(comment.opt()).filt(|(t, c): (i64, Option<Str>)| t == 10 || t == 11 || c.is_some())).with(hd.ge(add_years(t0, -1)));
    let hs = drain((&tp).select(history_of(db).select(keep)));
    let last = top_per(hs, |&(p, _)| p, |&(p, h)| (Reverse(hd.get(h).unwrap()), Reverse(h), p), 1, false);
    let lv = rel(last);
    let lidx: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&lv).map(|(p, _)| p).inv().select(&lv).collect();
    let v = drain((&s).and((&lidx).map(|(_, h)| h).opt()));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 100);
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.extend(a.map(V::I));
        f.extend(match h {
            Some(h) => [
                V::I(db.post_history.user_id.get(h).unwrap_or(-1)),
                V::S(htype_name(db).get(h).unwrap()),
                V::S(comment.get(h).unwrap_or("No Comments")),
                V::T(hd.get(h).unwrap()),
            ],
            None => [V::I(-1), V::S("No History"), V::S("No Comments"), V::Null],
        });
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName FROM RecentPosts rp WHERE rp.PostRank = 1),
// PostStats AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(c.Id) AS CommentCount, p.ViewCount, p.AcceptedAnswerId FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.ViewCount, p.AcceptedAnswerId),
// FinalResults AS (SELECT tp.Title, ps.UpVotes, ps.DownVotes, ps.CommentCount, CASE WHEN ps.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Not Accepted' END AS AnswerStatus
//     FROM TopPosts tp JOIN PostStats ps ON tp.PostId = ps.PostId)
// SELECT fr.Title, fr.UpVotes, fr.DownVotes, fr.CommentCount, fr.AnswerStatus,
//        CASE WHEN fr.UpVotes > fr.DownVotes THEN 'Positive' WHEN fr.UpVotes < fr.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM FinalResults fr ORDER BY fr.UpVotes DESC, fr.CommentCount DESC LIMIT 10;
//
// PostStats is only read for the TopPosts rows, so the vote x comment product is driven for those alone. A CreationDate tie within an owner goes to the smaller post id.
fn q4775(db: &'static So) -> String {
    let Post { owner_user, creation_date, accepted_answer_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let v = top_n(drain(&s), |&(p, a)| (Reverse(a[0]), Reverse(a[2]), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend(a.map(V::I));
        f.push(V::S(if accepted_answer_id.get(p).is_some() { "Accepted" } else { "Not Accepted" }));
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// PostHistoryWithComments AS (SELECT ph.PostId, ph.UserId, ph.CreationDate AS HistoryDate, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation IS NOT NULL)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpVoteCount - rp.DownVoteCount AS NetVotes, COALESCE(ph.Comment, 'No recent action') AS RecentAction,
//        ur.Reputation, ur.ReputationRank
// FROM RankedPosts rp LEFT JOIN PostHistoryWithComments ph ON rp.PostId = ph.PostId AND ph.HistoryRank = 1 LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId
// WHERE rp.Score > 0 AND (DATE '2024-10-01' - rp.CreationDate) <= INTERVAL '30 days' AND (rp.CommentCount > 5 OR ur.Reputation > 1000)
// ORDER BY rp.Score DESC, rp.CreationDate DESC FETCH FIRST 50 ROWS ONLY;
//
// PostRank is never read. The WHERE on Score and CreationDate reads base columns, so it is applied before the comment x vote product; `DATE '2024-10-01' - CreationDate <= 30 days`
// is `CreationDate >= 2024-09-01`. A tie on the newest history row goes to the larger history id.
fn q24368(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let base: MatSet<Id<Post>> = db.post.with(score.gt(0).and(creation_date.ge(date(2024, 9, 1)))).collect();
    let s = (&base)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, r), k)| (u, (r, k))).collect());
    let ur: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let hs = drain((&base).select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11, 12])))));
    let last = rel(top_per(hs, |&(p, _)| p, |&(_, h)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false));
    let lidx: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&last).map(|(p, _)| p).inv().select(&last).collect();
    let rows_ = (&s).and(owner_user.select(&ur).map(|(_, x)| x).opt()).filt(|(a, u): ([i64; 3], Option<(i64, i64)>)| a[0] > 5 || u.map_or(false, |(r, _)| r > 1000));
    let v = drain(rows_.and((&lidx).map(|(_, h)| h).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((a, u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1] - a[2]), V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No recent action"))]);
        f.extend(match u {
            Some((r, k)) => [V::I(r), V::I(k)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS PostRank FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.Score > 0),
// RecentActivity AS (SELECT P.Id AS PostId, COUNT(CASE WHEN C.UserId IS NOT NULL THEN 1 END) AS CommentCount, COUNT(V.Id) AS VoteCount FROM Posts P
//     LEFT JOIN Comments C ON P.Id = C.PostId AND C.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     LEFT JOIN Votes V ON P.Id = V.PostId AND V.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id),
// PostHistoryData AS (SELECT PH.PostId, COUNT(PH.Id) AS EditCount, MAX(PH.CreationDate) AS LastEditDate FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id
//     WHERE PH.PostHistoryTypeId IN (4, 5, 6) GROUP BY PH.PostId)
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.AnswerCount, RA.CommentCount AS RecentCommentCount, RA.VoteCount AS RecentVoteCount,
//        COALESCE(PHD.EditCount, 0) AS EditCount, PHD.LastEditDate, RP.OwnerDisplayName
// FROM RankedPosts RP LEFT JOIN RecentActivity RA ON RP.PostId = RA.PostId LEFT JOIN PostHistoryData PHD ON RP.PostId = PHD.PostId
// WHERE RP.PostRank <= 5 ORDER BY RP.Score DESC, RP.ViewCount DESC;
//
// RecentActivity and PostHistoryData are only read for the ranked rows, so they are computed for those alone.
fn q34068(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rc = comments_of(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).gt(add_days(t0, -30)))).select((&db.comment.user_id).opt());
    let rv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).gt(add_days(t0, -30))));
    let ra = (&tp).with(creation_date.gt(add_years(t0, -1))).group_by(Ident::<Post>::new()).select(rc.opt().and(rv.opt())).fold([0i64; 2], |a, (c, v)| {
        [a[0] + c.flatten().is_some() as i64, a[1] + v.is_some() as i64]
    });
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phd = (&tp)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6]))).select(hd))
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = drain((&tp).select(Ident::<Post>::new().and((&ra).opt()).and((&phd).opt())));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(_, ((p, a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::I(0), V::Null],
        });
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

// WITH PostSummary AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpvoteCount,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownvoteCount, COALESCE(SUM(b.Class), 0) AS TotalBadgeClass,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS RecentActivityRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount),
// ClosedPostLinks AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS RelatedPostsCount FROM PostLinks pl JOIN Posts p ON p.Id = pl.PostId WHERE p.ClosedDate IS NOT NULL GROUP BY pl.PostId),
// FinalPostSummary AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.CommentCount, ps.ViewCount, ps.UpvoteCount, ps.DownvoteCount, ps.TotalBadgeClass,
//        COALESCE(cpl.RelatedPostsCount, 0) AS ClosedRelatedPostCount, CASE WHEN ps.RecentActivityRank = 1 THEN 'Recent Activity' ELSE 'No Recent Activity' END AS ActivityStatus
//     FROM PostSummary ps LEFT JOIN ClosedPostLinks cpl ON ps.PostId = cpl.PostId)
// SELECT *, CASE WHEN ClosedRelatedPostCount > 0 THEN 'Has Closed Links' ELSE 'No Closed Links' END AS LinkStatus
// FROM FinalPostSummary WHERE TotalBadgeClass > 5 ORDER BY ViewCount DESC, CreationDate ASC LIMIT 10;
//
// RecentActivityRank partitions the grouped rows by post, one row each, so it is always 1.
fn q2668(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, closed_date, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let tb = qs()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(owner_user.select(badges_of(db).select(&db.badge.class)).opt()))
        .fold(0i64, |s, (_, b)| s + b.unwrap_or(0));
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = qs().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cl = db.post_link.with((&db.post_link.post).select(Ident::<Post>::new().with(closed_date))).group_by(&db.post_link.post).select(&db.post_link.related_post_id).fold(0i64, |n, _| n + 1);
    let v = drain((&tb).filt(|s| s > 5).and(&cc).and(&vc).and((&cl).opt()));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), creation_date.get(p).unwrap(), p)
    }, 10);
    rows(v.into_iter().map(|(p, (((s, c), u), l))| {
        let l = l.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["views"]));
        f.extend([V::I(u[0]), V::I(u[1]), V::I(s), V::I(l), V::S("Recent Activity"), V::S(if l > 0 { "Has Closed Links" } else { "No Closed Links" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, SUM(b.Class) AS BadgeCount
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS TotalComments FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// CloseReasonCounts AS (SELECT ph.PostId, ph.Comment as CloseReason, COUNT(ph.Id) AS ReasonCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.Comment)
// SELECT up.DisplayName, up.CommentCount, up.UpVoteCount, up.DownVoteCount, COALESCE(pc.TotalComments, 0) AS TotalComments, rp.Title, rp.CreationDate, rp.Score AS PostScore,
//        rp.ViewCount, cr.CloseReason, COALESCE(cr.ReasonCount, 0) AS CloseReasonCount
// FROM UserActivity up JOIN RankedPosts rp ON up.UserId = rp.PostId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId LEFT JOIN CloseReasonCounts cr ON rp.PostId = cr.PostId
// WHERE up.CommentCount > 10 AND rp.PostRank <= 3 AND (up.DownVoteCount IS NULL OR up.DownVoteCount < 5) ORDER BY rp.ViewCount DESC, up.UpVoteCount DESC LIMIT 100;
//
// `up.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids; PostRank reads only base columns, so the ranked posts are picked first (a Score tie
// goes to the smaller post id) and the comment x vote x badge product is driven only for the users they join.
fn q21300(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let us: MatSet<Id<User>> = (&tp).select((&db.post.origid).select(&uidx)).collect();
    let ua = (&us)
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, ((c, t), b)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + b.unwrap_or(0), a[4] + b.is_some() as i64]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db.post_history.with(post_history_type_id.eq(10)).group_by(post.and(comment.opt())).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let crv = rel(drain(&cr));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, Option<Str>), i64)> = (&crv).map(|((p, _), _)| p).inv().select(&crv).collect();
    let v = drain((&tp).select(Ident::<Post>::new().and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and((&ua).filt(|a| a[0] > 10 && a[2] < 5)))).and(&pc).and((&by_post).opt())));
    let v = top_n(v, |&(_, (((p, (_, a)), _), _))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(a[1]), p)
    }, 100);
    rows(v.into_iter().map(|(_, (((p, (u, a)), c), r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c)];
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend(match r {
            Some(((_, s), n)) => [ostr(s), V::I(n)],
            None => [V::Null, V::I(0)],
        });
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, p.Score),
// PostScoreRanked AS (SELECT rp.*, RANK() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS ScoreRank FROM RecentPosts rp),
// TopPosts AS (SELECT ps.* FROM PostScoreRanked ps WHERE ScoreRank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, u.DisplayName AS OwnerName, tp.ViewCount, tp.Score, COALESCE(tp.CommentCount, 0) AS CommentCount, tp.UpVotes, tp.DownVotes,
//        COALESCE(MAX(CASE WHEN bt.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadge, COALESCE(MAX(CASE WHEN bt.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadge,
//        COALESCE(MAX(CASE WHEN bt.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadge
// FROM TopPosts tp JOIN Users u ON tp.OwnerUserId = u.Id LEFT JOIN Badges bt ON u.Id = bt.UserId
// GROUP BY tp.PostId, tp.Title, tp.CreationDate, u.DisplayName, tp.ViewCount, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes HAVING SUM(bt.Class) IS NOT NULL
// ORDER BY tp.Score DESC, tp.ViewCount DESC LIMIT 5;
//
// ScoreRank reads only base columns, so the top posts are picked first and the comment x vote x badge product is driven for those alone.
fn q22474(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let v = ranked(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))), |&(p, _)| key(p), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let bt = (&tp)
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(owner_user.select(badges_of(db).select(&db.badge.class)).opt())
        .fold([0i64; 4], |a, c| [a[0] + c.is_some() as i64, a[1].max((c == Some(1)) as i64), a[2].max((c == Some(2)) as i64), a[3].max((c == Some(3)) as i64)]);
    let v = drain((&s).and((&bt).filt(|b| b[0] > 0)));
    let v = top_n(v, |&(p, _)| (key(p), p), 5);
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "views", "score"]);
        f.extend(a.map(V::I));
        f.extend([V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId = 1),
// PostVoteStats AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// ClosedPostDetails AS (SELECT ph.PostId, ph.CreationDate AS CloseDate, crt.Name AS CloseReason FROM PostHistory ph JOIN CloseReasonTypes crt ON ph.Comment::int = crt.Id WHERE ph.PostHistoryTypeId = 10),
// FinalPostStats AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerDisplayName, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes,
//        cpd.CloseDate, cpd.CloseReason, CASE WHEN cpd.CloseDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
//     FROM RankedPosts rp LEFT JOIN PostVoteStats pvs ON rp.Id = pvs.PostId LEFT JOIN ClosedPostDetails cpd ON rp.Id = cpd.PostId WHERE rp.PostRank <= 5)
// SELECT Title, CreationDate, ViewCount, Score, OwnerDisplayName, UpVotes, DownVotes, CloseDate, CloseReason, PostStatus FROM FinalPostStats ORDER BY Score DESC, CreationDate ASC LIMIT 10 OFFSET 0;
//
// A Score tie within an owner's questions goes to the smaller post id.
fn q1372(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)).and(post_type_id.eq(1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let closed = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(Ident::<PostHistory>::new().and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)));
    let v = drain((&pvs).and(closed.opt()));
    let v = top_n(v, |&(p, (_, c))| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p, c.map(|c| c.0)), 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score", "owner"]);
        f.extend(a.map(V::I));
        f.extend(match c {
            Some((h, r)) => [V::T(hd.get(h).unwrap()), V::S(r), V::S("Closed")],
            None => [V::Null, V::Null, V::S("Open")],
        });
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount, COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// ActivitySummary AS (SELECT p.OwnerUserId AS UserId, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore FROM Posts p
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT um.UserId, um.DisplayName, um.BadgeCount, um.GoldBadges, um.SilverBadges, um.BronzeBadges, COALESCE(asum.TotalPosts, 0) AS TotalPosts,
//        COALESCE(asum.TotalViews, 0) AS TotalViews, COALESCE(asum.TotalScore, 0) AS TotalScore FROM UserMetrics um LEFT JOIN ActivitySummary asum ON um.UserId = asum.UserId)
// SELECT up.DisplayName, up.BadgeCount, up.GoldBadges, up.SilverBadges, up.BronzeBadges, up.TotalPosts, up.TotalViews, up.TotalScore,
//        RANK() OVER (ORDER BY up.TotalScore DESC, up.TotalPosts DESC) AS UserRank
// FROM UserPerformance up WHERE (up.TotalPosts > 5 OR up.BadgeCount > 0) ORDER BY UserRank FETCH FIRST 10 ROWS ONLY;
fn q471(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let um = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt()))
        .fold([0i64; 4], |a, (c, _)| [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]);
    let asum = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 3], |a, (s, w)| {
        [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]
    });
    let v = drain((&um).and((&asum).opt()).filt(|(b, s): ([i64; 4], Option<[i64; 3]>)| s.map_or(0, |s| s[0]) > 5 || b[0] > 0));
    let v = ranked(v, |&(_, (_, s))| {
        let s = s.unwrap_or([0; 3]);
        (Reverse(s[2]), Reverse(s[0]))
    }, false);
    let v = top_n(v, |x| x.1, 10);
    rows(v.into_iter().map(|((u, (b, s)), r)| {
        let s = s.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend([V::I(s[0]), V::I(s[1]), V::I(s[2]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.CreationDate, P.Score, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS rn,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY P.Id) AS UpvoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY P.Id) AS DownvoteCount,
//        COALESCE((P.Score * 1.0 / NULLIF(P.ViewCount, 0)) * 100, 0) AS ScorePerView
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate BETWEEN TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND TIMESTAMP '2024-10-01 12:34:56'),
// PostSummary AS (SELECT RP.PostId, RP.Title, RP.ViewCount, RP.CreationDate, RP.Score, RP.UpvoteCount, RP.DownvoteCount, RP.ScorePerView,
//        CASE WHEN RP.UpvoteCount IS NULL OR RP.UpvoteCount = 0 THEN 'No Votes' WHEN RP.DownvoteCount > RP.UpvoteCount THEN 'More Downvotes' ELSE 'Popular' END AS Popularity
//     FROM RankedPosts RP WHERE RP.rn = 1)
// SELECT PS.PostId, PS.Title, PS.ViewCount, PS.CreationDate, PS.Score, PS.UpvoteCount, PS.DownvoteCount, PS.ScorePerView, PS.Popularity, U.DisplayName AS Owner,
//        COALESCE(B.Name, 'No Badge') AS UserBadge, PH.Comment AS PostHistoryComment
// FROM PostSummary PS LEFT JOIN Users U ON U.Id = (SELECT OwnerUserId FROM Posts P WHERE P.Id = PS.PostId LIMIT 1) LEFT JOIN Badges B ON U.Id = B.UserId AND B.Class = 1
// LEFT JOIN PostHistory PH ON PH.PostId = PS.PostId AND PH.PostHistoryTypeId = 24 WHERE PS.ScorePerView > 0.5 ORDER BY PS.Score DESC, PS.Popularity DESC;
//
// rn numbers post x vote rows and every row of one post projects the same columns, so each type's newest post is picked first (a tie goes to the smaller post id).
// The correlated `LIMIT 1` looks a post up by its Id, which is unique, so it is the post's owner.
fn q22767(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.between(add_years(t0, -1), t0)).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let spv = |p: Id<Post>| match view_count.get(p) {
        Some(w) if w != 0 => score.get(p).unwrap() as f64 * 1.0 / w as f64 * 100.0,
        _ => 0.0,
    };
    let vc = (&tp)
        .with(Ident::<Post>::new().map(spv).filt(|x: f64| x > 0.5))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let ph = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(24)));
    let v = drain((&vc).and(owner_user.select(Ident::<User>::new().and(gold.opt())).opt()).and(ph.opt()));
    let pop = |a: [i64; 2]| if a[0] == 0 { "No Votes" } else if a[1] > a[0] { "More Downvotes" } else { "Popular" };
    let v = top_n(v, |&(p, ((a, _), _))| (Reverse(score.get(p).unwrap()), Reverse(pop(a))), 0);
    rows(v.into_iter().map(|(p, ((a, u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::F(spv(p)), V::S(pop(a))]);
        f.extend(match u {
            Some((u, b)) => [user_col(db, u, "name"), V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap()))],
            None => [V::Null, V::S("No Badge")],
        });
        f.push(h.map_or(V::Null, |h| ostr(db.post_history.comment.get(h))));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, COALESCE(b.Name, 'No Badge') AS UserBadge FROM RankedPosts rp
//     LEFT JOIN Badges b ON rp.PostId = b.UserId AND b.Class = 1 WHERE rp.PostRank > 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN up.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN up.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        COUNT(DISTINCT p.Id) AS TotalPosts, AVG(p.Score) AS AvgScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes up ON p.Id = up.PostId GROUP BY u.Id, u.DisplayName)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.CommentCount, us.DisplayName AS PostOwner, us.TotalUpvotes, us.TotalDownvotes, us.TotalPosts, us.AvgScore,
//        CASE WHEN fp.UserBadge = 'No Badge' AND us.TotalPosts = 0 THEN 'New User' ELSE 'Active User' END AS UserCategory
// FROM FilteredPosts fp JOIN UserStats us ON us.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = fp.PostId)
// WHERE fp.CommentCount >= 2 AND (us.TotalUpvotes - us.TotalDownvotes) > 5 ORDER BY fp.CreationDate DESC LIMIT 100;
//
// PostRank numbers the post x comment rows, so the joined rows are materialised and numbered per owner (ties go to the smaller post, then comment id), and only the
// first row of each owner is dropped. `rp.PostId = b.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q20416(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let recent: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).collect();
    let jr = drain((&recent).select(comments_of(db).opt()));
    let r = ranked(jr, |&(p, c)| (owner_user.get(p), Reverse(creation_date.get(p).unwrap()), p, c), false);
    let r = per_group(r, |&(p, _)| owner_user.get(p));
    let kept = rel(drain(rel(r).filt(|x: ((Id<Post>, Option<Id<Comment>>), i64)| x.1 > 1)).into_iter().map(|x| x.1 .0 .0).collect());
    let cc = (&recent).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(votes_of(db).select(&db.vote.vote_type_id).opt()))).fold([0i64; 4], |a, (s, t)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + 1, a[3] + s]
    });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let gold = (&db.post.origid).select(&uidx).select(badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))));
    let v = drain((&kept).select(Ident::<Post>::new().and((&cc).filt(|n| n >= 2)).and(owner_user.select(Ident::<User>::new().and((&us).filt(|a| a[0] - a[1] > 5)).and(&np))).and(gold.opt())));
    let v = top_n(v, |&(i, (((p, _), _), _))| (Reverse(creation_date.get(p).unwrap()), i), 100);
    rows(v.into_iter().map(|(_, (((p, c), ((u, a), n)), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n), avg(a[3], a[2])]);
        f.push(V::S(if b.is_none() && n == 0 { "New User" } else { "Active User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        COALESCE(cc.CommentCount, 0) AS CommentCount FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) cc ON p.Id = cc.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(p.Score) AS TotalScore, SUM(u.UpVotes) AS UpVoteCount, SUM(u.DownVotes) AS DownVoteCount,
//        AVG(COALESCE(rp.CommentCount, 0)) AS AvgCommentsPerPost FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT us.UserId, us.DisplayName, us.TotalPosts, us.TotalScore, us.UpVoteCount, us.DownVoteCount, us.AvgCommentsPerPost,
//        (CASE WHEN us.TotalScore > 100 THEN 'High Scorer' WHEN us.TotalScore BETWEEN 50 AND 100 THEN 'Medium Scorer' ELSE 'Low Scorer' END) AS ScoreCategory,
//        (SELECT COUNT(*) FROM Badges b WHERE b.UserId = us.UserId AND b.Class = 1) AS GoldBadges, (SELECT COUNT(*) FROM Badges b WHERE b.UserId = us.UserId AND b.Class = 2) AS SilverBadges,
//        (SELECT COUNT(*) FROM Badges b WHERE b.UserId = us.UserId AND b.Class = 3) AS BronzeBadges
// FROM UserStats us WHERE us.TotalPosts > 0 ORDER BY us.TotalScore DESC, us.DisplayName ASC LIMIT 100 OFFSET 0;
fn q758(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let recent: HashIdx<Id<User>, Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user).inv().collect();
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let us = db
        .user
        .with(posts_of(db))
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and((&recent).select(Ident::<Post>::new().and((&cc).opt())).opt().and(posts_of(db).select(score).opt())))
        .fold([0i64; 5], |a, ((u, d), (r, s))| [a[0] + 1, a[1] + s.unwrap_or(0), a[2] + u, a[3] + d, a[4] + r.and_then(|r| r.1).unwrap_or(0)]);
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&us).and(&np).and((&bc).opt()));
    let v = top_n(v, |&(u, ((a, _), _))| (Reverse(a[1]), db.user.display_name.get(u).unwrap(), u), 100);
    rows(v.into_iter().map(|(u, ((a, n), b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0])]);
        f.push(V::S(if a[1] > 100 { "High Scorer" } else if a[1] >= 50 { "Medium Scorer" } else { "Low Scorer" }));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(v.BountyAmount) AS TotalBounties, COALESCE(NULLIF(SUM(v.BountyAmount), 0), 0) AS AdjustedBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// FilteredUsers AS (SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.PostCount, ua.QuestionCount, ua.AnswerCount, ua.TotalBounties, RANK() OVER (ORDER BY ua.Reputation DESC) AS ReputationRank
//     FROM UserActivity ua WHERE ua.Reputation > 1000 AND ua.PostCount > 10),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentRank FROM Posts p
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month')
// SELECT fu.UserId, fu.DisplayName, fu.Reputation, fu.PostCount, fu.QuestionCount, fu.AnswerCount, fu.TotalBounties, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate,
//        COALESCE(rp.RecentRank, 0) AS IsRecentPost
// FROM FilteredUsers fu LEFT JOIN RecentPosts rp ON fu.UserId = rp.OwnerUserId WHERE (fu.QuestionCount > 5 OR fu.AnswerCount > 10) ORDER BY fu.Reputation DESC, fu.UserId LIMIT 50;
//
// A CreationDate tie within an owner's recent posts goes to the smaller post id.
fn q3998(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8))).select(bounty_amount.opt());
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(bounty.opt())))
        .fold([0i64; 4], |a, (t, b)| {
            let b = b.flatten();
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let rp = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user));
    let rp = per_group(ranked(rp, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let rv = rel(rp.into_iter().map(|((p, u), r)| (u, (p, r))).collect());
    let ridx: HashIdx<Id<User>, (Id<User>, (Id<Post>, i64))> = (&rv).map(|(u, _)| u).inv().select(&rv).collect();
    let v = drain((&ua).filt(|a| a[0] > 5 || a[1] > 10).and((&np).filt(|n| n > 10)).and((&ridx).map(|(_, x)| x).opt()));
    let v = top_n(v, |&(u, (_, r))| (Reverse(db.user.reputation.get(u).unwrap()), db.user.origid.get(u).unwrap(), r.map(|r| r.1)), 50);
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2])]);
        f.extend(match r {
            Some((p, k)) => {
                let mut g = post_fields(db, p, &["title", "created"]);
                g.push(V::I(k));
                g
            }
            None => vec![V::Null, V::Null, V::I(0)],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT P.Id) AS PostCount,
//        ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY COALESCE(SUM(V.BountyAmount), 0) DESC) AS RN
//     FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON V.PostId = P.Id WHERE U.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT * FROM UserActivity WHERE RN = 1),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS Author, P.Score, P.ViewCount, COALESCE(PC.CommentCount, 0) AS CommentCount, COALESCE(PH.Revisions, 0) AS RevisionCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) PC ON PC.PostId = P.Id
//     LEFT JOIN (SELECT PostId, COUNT(*) AS Revisions FROM PostHistory GROUP BY PostId) PH ON PH.PostId = P.Id
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' AND P.Score > 0),
// RankedPosts AS (SELECT PD.*, ROW_NUMBER() OVER (ORDER BY PD.ViewCount DESC, PD.Score DESC) AS PostRank FROM PostDetails PD)
// SELECT TU.DisplayName, TU.Reputation, RP.Title, RP.CreationDate, RP.ViewCount, RP.CommentCount, RP.RevisionCount
// FROM TopUsers TU JOIN RankedPosts RP ON RP.Author = TU.DisplayName WHERE TU.Reputation > 1000 ORDER BY TU.Reputation DESC, RP.ViewCount DESC LIMIT 10;
//
// RN partitions the grouped rows by user, one row each, so it is always 1, and none of UserActivity's aggregates is projected: TopUsers is the users created in the last year.
// PostRank is never read.
fn q1127(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let tu: MatSet<Id<User>> = db.user.with((&db.user.creation_date).ge(add_years(t0, -1)).and((&db.user.reputation).gt(1000))).collect();
    let by_name: HashIdx<Str, Id<User>> = (&tu).select(&db.user.display_name).inv().collect();
    let pd: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_months(t0, -1)).and(score.gt(0))).collect();
    let cc = (&pd).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rc = (&pd).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&cc).and(&rc).and(owner_user.select(&db.user.display_name).select(&by_name)));
    let v = top_n(v, |&(p, (_, u))| {
        let w = view_count.get(p);
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w), p, u)
    }, 10);
    rows(v.into_iter().map(|(p, ((c, r), u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.extend([V::I(c), V::I(r)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostSummary AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(P.ViewCount) AS TotalViews, MAX(P.CreationDate) AS LastPostDate FROM Posts P GROUP BY P.OwnerUserId),
// ActiveUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.LastAccessDate, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.TotalPosts, 0) AS TotalPosts,
//        COALESCE(PS.TotalViews, 0) AS TotalViews, PS.LastPostDate FROM Users U LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId LEFT JOIN PostSummary PS ON U.Id = PS.OwnerUserId
//     WHERE U.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RankedUsers AS (SELECT A.*, RANK() OVER (ORDER BY A.Reputation DESC, A.BadgeCount DESC) AS UserRank FROM ActiveUsers A)
// SELECT RU.UserId, RU.DisplayName, RU.Reputation, RU.BadgeCount, RU.TotalPosts, RU.TotalViews, RU.LastPostDate,
//        CASE WHEN RU.UserRank <= 10 THEN 'Top User' WHEN RU.Reputation > 1000 THEN 'Active Contributor' ELSE 'Regular User' END AS UserCategory,
//        CASE WHEN RU.LastPostDate IS NULL THEN 'No Posts' ELSE 'Has Posts' END AS PostStatus, (SELECT COUNT(*) FROM Comments C WHERE C.UserId = RU.UserId) AS CommentCount
// FROM RankedUsers RU WHERE RU.TotalPosts > 0 ORDER BY RU.UserRank LIMIT 20;
fn q4756(db: &'static So) -> String {
    let Post { owner_user, view_count, creation_date, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.with(owner_user).group_by(owner_user).select(view_count.opt().and(creation_date)).fold([0i64, 0, i64::MIN], |a, (w, d)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2].max(d)]);
    let cc = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain(db.user.with((&db.user.last_access_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select((&bc).and((&ps).opt()).and(&cc)));
    let v = ranked(v, |&(u, ((b, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(b)), false);
    let v = drain(rel(v).filt(|((_, ((_, p), _)), _): ((Id<User>, ((i64, Option<[i64; 3]>), i64)), i64)| p.is_some()));
    let v = top_n(v.into_iter().map(|x| x.1).collect(), |x| x.1, 20);
    rows(v.into_iter().map(|((u, ((b, p), c)), r)| {
        let p = p.unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(p[0]), V::I(p[1]), V::T(p[2])]);
        f.push(V::S(if r <= 10 { "Top User" } else if db.user.reputation.get(u).unwrap() > 1000 { "Active Contributor" } else { "Regular User" }));
        f.extend([V::S("Has Posts"), V::I(c)]);
        row(f)
    }))
}

// WITH RankedBadges AS (SELECT B.UserId, B.Name, B.Class, RANK() OVER (PARTITION BY B.UserId ORDER BY B.Date DESC) AS BadgeRank FROM Badges B WHERE B.Class = 1),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, CASE WHEN U.Reputation IS NULL THEN 0 ELSE U.Reputation END AS AdjustedReputation FROM Users U),
// QuestionStats AS (SELECT P.OwnerUserId, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AvgScore FROM Posts P WHERE P.PostTypeId = 1 GROUP BY P.OwnerUserId),
// ClosedQuestions AS (SELECT PH.UserId, COUNT(DISTINCT PH.PostId) AS ClosedCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId)
// SELECT U.DisplayName, U.Location, U.CreationDate, COALESCE(RB.Name, 'No Gold Badge') AS GoldBadge, COALESCE(QS.QuestionCount, 0) AS QuestionsAsked, COALESCE(QS.TotalViews, 0) AS TotalViews,
//        COALESCE(QS.AvgScore, 0) AS AvgScore, COALESCE(CQ.ClosedCount, 0) AS ClosedQuestions, UR.AdjustedReputation,
//        CASE WHEN UR.AdjustedReputation > 1000 THEN 'Elite' WHEN UR.AdjustedReputation BETWEEN 500 AND 1000 THEN 'Intermediate' ELSE 'Novice' END AS ReputationCategory
// FROM Users U LEFT JOIN RankedBadges RB ON U.Id = RB.UserId AND RB.BadgeRank = 1 LEFT JOIN QuestionStats QS ON U.Id = QS.OwnerUserId LEFT JOIN ClosedQuestions CQ ON U.Id = CQ.UserId
// JOIN UserReputation UR ON U.Id = UR.UserId WHERE UR.AdjustedReputation IS NOT NULL ORDER BY UR.AdjustedReputation DESC, U.DisplayName LIMIT 50;
fn q23840(db: &'static So) -> String {
    let Badge { user, class, date: bd, .. } = &db.badge;
    let gb = drain(db.badge.with(class.eq(1)).select(user));
    let gb = rel(top_per(gb, |&(_, u)| u, |&(b, _)| Reverse(bd.get(b).unwrap()), 1, true).into_iter().map(|(b, u)| (u, b)).collect());
    let latest: HashIdx<Id<User>, (Id<User>, Id<Badge>)> = (&gb).map(|(u, _)| u).inv().select(&gb).collect();
    let Post { post_type_id, owner_user, view_count, score, .. } = &db.post;
    let qs = db.post.with(post_type_id.eq(1)).with(owner_user).group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let PostHistory { user: hu, post, post_history_type_id, .. } = &db.post_history;
    let cq = db.post_history.with(post_history_type_id.eq(10)).with(hu).group_by(hu).select(post).count_distinct();
    let v = drain(db.user.select((&latest).map(|(_, b)| b).opt().and((&qs).opt()).and((&cq).opt())));
    let v = top_n(v, |&(u, ((b, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), db.user.display_name.get(u).unwrap(), u, b), 50);
    rows(v.into_iter().map(|(u, ((b, q), c))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "name"), ostr(db.user.location.get(u)), user_col(db, u, "ucreated")];
        f.push(V::S(b.map_or("No Gold Badge", |b| db.badge.name.get(b).unwrap())));
        f.extend(match q {
            Some(a) => [V::I(a[0]), V::I(a[1]), avg(a[2], a[0])],
            None => [V::I(0), V::I(0), V::F(0.0)],
        });
        f.extend([V::I(c.unwrap_or(0)), V::I(r), V::S(if r > 1000 { "Elite" } else if r >= 500 { "Intermediate" } else { "Novice" })]);
        row(f)
    }))
}

// WITH RecursivePostStats AS (SELECT p.Id, p.Title, p.CreationDate, p.PostTypeId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.CreationDate, p.PostTypeId),
// PostHistorySummary AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS ChangeCount FROM PostHistory ph WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 MONTH'
//     GROUP BY ph.PostId, ph.PostHistoryTypeId),
// RankedPosts AS (SELECT r.Id, r.Title, r.CommentCount, r.UpVotes - r.DownVotes AS NetVotes, ROW_NUMBER() OVER (PARTITION BY r.PostTypeId ORDER BY r.CommentCount DESC, r.CreationDate ASC) AS Rank
//     FROM RecursivePostStats r WHERE r.CommentCount > 0)
// SELECT p.Id, p.Title, ps.CommentCount, ps.UpVotes, ps.DownVotes, phs.ChangeCount AS HistoryChangeCount, CASE WHEN p.PostTypeId = 1 THEN 'Question' ELSE 'Answer' END AS PostType,
//        COALESCE(rp.Rank, 0) AS PostRank
// FROM Posts p JOIN RecursivePostStats ps ON p.Id = ps.Id LEFT JOIN PostHistorySummary phs ON p.Id = phs.PostId LEFT JOIN RankedPosts rp ON p.Id = rp.Id
// WHERE ps.CommentCount > 5 AND ((ps.UpVotes - ps.DownVotes) > 0 OR (phs.ChangeCount IS NOT NULL AND phs.ChangeCount > 2)) ORDER BY ps.UpVotes DESC, ps.CommentCount DESC, phs.ChangeCount DESC;
//
// A (CommentCount, CreationDate) tie in Rank goes to the smaller post id.
fn q23792(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ps = db
        .post
        .with(creation_date.ge(add_years(t0, -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let rk = per_group(ranked(drain((&ps).filt(|a| a[0] > 0)), |&(p, a)| (post_type_id.get(p).unwrap(), Reverse(a[0]), creation_date.get(p).unwrap(), p), false), |&(p, _)| post_type_id.get(p).unwrap());
    let rk = rel(rk.into_iter().map(|((p, _), r)| (p, r)).collect());
    let ridx: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(hd.ge(add_months(t0, -6))).group_by(post.and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pv = rel(drain(&phs));
    let pidx: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&pv).map(|((p, _), _)| p).inv().select(&pv).collect();
    let v = drain(
        (&ps)
            .filt(|a| a[0] > 5)
            .and((&pidx).map(|(_, n)| n).opt())
            .filt(|(a, n): ([i64; 3], Option<i64>)| a[1] - a[2] > 0 || n.map_or(false, |n| n > 2))
            .and((&ridx).map(|(_, r)| r).opt()),
    );
    rows(v.into_iter().map(|(p, ((a, n), r))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), oint(n), V::S(if post_type_id.get(p).unwrap() == 1 { "Question" } else { "Answer" }), V::I(r.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RecursivePostStats AS (SELECT Posts.Id AS PostId, Posts.Title, Posts.CreationDate, Posts.Score, Posts.AnswerCount, Posts.ViewCount, Users.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY Posts.Id ORDER BY Posts.CreationDate DESC) AS RowNum, Posts.OwnerUserId FROM Posts JOIN Users ON Posts.OwnerUserId = Users.Id WHERE Posts.PostTypeId = 1),
// PostVoteSummary AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN ph.Comment IS NOT NULL THEN 1 ELSE 0 END) AS Edits,
//        DENSE_RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS Rank FROM Users AS u JOIN Posts AS p ON u.Id = p.OwnerUserId LEFT JOIN PostHistory AS ph ON p.Id = ph.PostId
//     GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 5)
// SELECT rps.PostId, rps.Title, rps.CreationDate, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes, rps.Score, rps.ViewCount, rps.OwnerReputation, mau.UserId,
//        mau.DisplayName AS MostActiveUser, mau.PostCount AS ActiveUserPostCount, mau.Edits AS UserEdits, mau.Rank AS UserRank
// FROM RecursivePostStats AS rps LEFT JOIN PostVoteSummary AS pvs ON rps.PostId = pvs.PostId LEFT JOIN MostActiveUsers AS mau ON rps.OwnerUserId = mau.UserId
// WHERE rps.RowNum = 1 ORDER BY rps.CreationDate DESC, rps.Score DESC;
//
// RowNum partitions by the post id, one row each, so it is always 1.
fn q34047(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let pvs = db.post.with(post_type_id.eq(1)).with(owner_user).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let ed = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(history_of(db).select((&db.post_history.comment).opt()).opt())).fold(0i64, |n, c| n + c.flatten().is_some() as i64);
    let mv = ranked(drain((&pc).filt(|n| n > 5).and(&ed)), |&(_, (n, _))| Reverse(n), true);
    let mv = rel(mv.into_iter().map(|((u, a), r)| (u, (a, r))).collect());
    let mau: HashIdx<Id<User>, (Id<User>, ((i64, i64), i64))> = (&mv).map(|(u, _)| u).inv().select(&mv).collect();
    let v = drain((&pvs).and(owner_user.select((&mau).map(|(_, x)| x)).opt()));
    rows(v.into_iter().map(|(p, (a, m))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["score", "views", "rep"]));
        f.extend(match m {
            Some(((n, e), r)) => {
                let u = owner_user.get(p).unwrap();
                vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n), V::I(e), V::I(r)]
            }
            None => (0..5).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COALESCE(SUM(case when v.VoteTypeId = 2 then 1 else 0 end), 0) AS UpVotesCount,
//        COALESCE(SUM(case when v.VoteTypeId = 3 then 1 else 0 end), 0) AS DownVotesCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.ViewCount, p.AcceptedAnswerId, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostsCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostLinks pl ON p.Id = pl.PostId GROUP BY p.Id, p.Title, p.OwnerUserId, p.ViewCount, p.AcceptedAnswerId),
// ReputationRanked AS (SELECT UserId, DisplayName, Reputation, TotalBounties, UpVotesCount, DownVotesCount, RANK() OVER (ORDER BY Reputation DESC) as ReputationRank FROM UserReputation)
// SELECT ps.PostId, ps.Title, ps.ViewCount, ps.CommentCount, u.DisplayName AS OwnerDisplayName, rr.Reputation, rr.ReputationRank,
//        (CASE WHEN ps.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Not Accepted' END) AS AcceptanceStatus, (CASE WHEN rr.TotalBounties > 0 THEN 'Has Bounties' ELSE 'No Bounties' END) AS BountyStatus
// FROM PostStats ps JOIN Users u ON ps.OwnerUserId = u.Id JOIN ReputationRanked rr ON u.Id = rr.UserId WHERE ps.ViewCount > 100
// ORDER BY rr.Reputation DESC, ps.ViewCount DESC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
//
// The WHERE on ViewCount reads a base column, so it is applied before the comment x link product.
fn q623(db: &'static So) -> String {
    let Post { owner_user, view_count, accepted_answer_id, .. } = &db.post;
    let tb = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), k)| (u, k)).collect());
    let ridx: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let ps = db.post.with(view_count.gt(100)).with(owner_user).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(links_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let v = drain((&ps).and(owner_user.select(Ident::<User>::new().and(&tb).and((&ridx).map(|(_, k)| k)))));
    let v = top_n(v, |&(p, (_, ((u, _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(view_count.get(p)), p), 20);
    rows(v.into_iter().skip(10).map(|(p, (c, ((u, b), k)))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(c), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(k)]);
        f.push(V::S(if accepted_answer_id.get(p).is_some() { "Accepted" } else { "Not Accepted" }));
        f.push(V::S(if b > 0 { "Has Bounties" } else { "No Bounties" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(v.BountyAmount) OVER (PARTITION BY p.Id) AS TotalBounty, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
//     WHERE p.PostTypeId IN (1, 2) AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// FilteredPosts AS (SELECT rp.PostId, rp.CommentCount, rp.TotalBounty, rp.CreationDate, p.OwnerUserId, p.Title, DENSE_RANK() OVER (ORDER BY rp.CommentCount DESC, rp.TotalBounty DESC) AS Rank
//     FROM RankedPosts rp JOIN Posts p ON rp.PostId = p.Id WHERE rp.UserPostRank = 1 AND rp.CommentCount > 5),
// PostHistoryAggregated AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount,
//        COUNT(DISTINCT ph.UserId) AS UniqueEditors FROM PostHistory ph GROUP BY ph.PostId)
// SELECT fp.Title, u.DisplayName AS Owner, fp.CommentCount, fp.TotalBounty, ph.CloseCount, ph.ReopenCount, ph.UniqueEditors, fp.Rank
// FROM FilteredPosts fp JOIN Users u ON fp.OwnerUserId = u.Id LEFT JOIN PostHistoryAggregated ph ON fp.PostId = ph.PostId
// WHERE (ph.CloseCount > 1 OR ph.ReopenCount > 1) AND (fp.TotalBounty IS NOT NULL AND fp.TotalBounty > 0) AND u.Reputation >= 1000 ORDER BY fp.Rank, fp.CommentCount DESC;
//
// UserPostRank numbers post x comment x vote rows and every row of one post carries the same windowed values, so each owner's newest post is picked first (a tie goes to
// the smaller post id) and the product is driven for those alone.
fn q24216(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let rp = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 3], |a, (c, b)| {
        let b = b.flatten();
        [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
    });
    let fp = ranked(drain((&rp).filt(|a| a[0] > 5)), |&(_, a)| (Reverse(a[0]), a[1] == 0, Reverse(a[2])), true);
    let fp = rel(fp.into_iter().map(|((p, a), r)| (p, (a, r))).collect());
    let fidx: HashIdx<Id<Post>, (Id<Post>, ([i64; 3], i64))> = (&fp).map(|(p, _)| p).inv().select(&fp).collect();
    let PostHistory { post_history_type_id, user, .. } = &db.post_history;
    let pha = (&fidx).map(|(p, _)| p).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let ue = (&fidx).map(|(p, _)| p).group_by(Ident::<Post>::new()).select(history_of(db).select(user)).count_distinct();
    let v = drain(
        (&fidx)
            .map(|(_, x)| x)
            .filt(|(a, _): ([i64; 3], i64)| a[1] > 0 && a[2] > 0)
            .and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).ge(1000))))
            .and((&pha).filt(|a| a[0] > 1 || a[1] > 1))
            .and((&ue).opt()),
    );
    let v = top_n(v, |&(_, ((((a, r), _), _), _))| (r, Reverse(a[0])), 0);
    rows(v.into_iter().map(|(p, ((((a, r), u), h), e))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[2]), V::I(h[0]), V::I(h[1]), V::I(e.unwrap_or(0)), V::I(r)]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount, P.OwnerUserId
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON P.OwnerUserId = B.UserId GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId),
// ReputationCounts AS (SELECT U.Id AS UserId, U.Reputation, AVG(P.Score) AS AvgPostScore, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId
//     GROUP BY U.Id, U.Reputation)
// SELECT P.Title, U.DisplayName AS OwnerName, U.Reputation, UVS.UpVotes, UVS.DownVotes, P.ViewCount, P.CommentCount, P.BadgeCount, RC.ReputationRank, P.CreationDate,
//        CASE WHEN P.Score > 100 THEN 'High Score' WHEN P.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM PostDetails P JOIN UserVoteSummary UVS ON P.OwnerUserId = UVS.UserId JOIN Users U ON P.OwnerUserId = U.Id JOIN ReputationCounts RC ON U.Id = RC.UserId
// WHERE U.Reputation > 100 AND P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '7 days') ORDER BY RC.ReputationRank, P.CreationDate DESC LIMIT 50;
//
// The WHERE on P.CreationDate reads a base column, so it is applied before the comment x badge product.
fn q3924(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let rc = rel(ranked(drain(db.user.with(posts_of(db)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), k)| (u, k)).collect());
    let ridx: HashIdx<Id<User>, (Id<User>, i64)> = (&rc).map(|(u, _)| u).inv().select(&rc).collect();
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pd = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -7)))
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100))))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64]);
    let v = drain((&pd).and(owner_user.select(Ident::<User>::new().and(&uvs).and((&ridx).map(|(_, k)| k)))));
    let v = top_n(v, |&(p, (_, ((_, _), k)))| (k, Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (a, ((u, x), k)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title"]);
        f.extend([user_col(db, u, "name"), user_col(db, u, "rep"), V::I(x[0]), V::I(x[1])]);
        f.extend(post_fields(db, p, &["views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(k), V::T(creation_date.get(p).unwrap())]);
        f.push(V::S(if s > 100 { "High Score" } else if s >= 50 { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM Users),
// TopUsers AS (SELECT UserId FROM UserReputation WHERE Rank <= 10),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(c.Count, 0) AS CommentCount, COALESCE(v.UpVoteCount, 0) AS UpVotes, COALESCE(v.DownVoteCount, 0) AS DownVotes,
//        CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Yes' ELSE 'No' END AS IsAccepted
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS Count FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.CommentCount, pd.UpVotes, pd.DownVotes, pd.IsAccepted, RANK() OVER (ORDER BY (pd.UpVotes - pd.DownVotes) DESC) AS PostRank
//     FROM PostDetails pd WHERE pd.CommentCount > 5)
// SELECT u.DisplayName, tp.Title, tp.CreationDate, tp.CommentCount, tp.UpVotes, tp.DownVotes, tp.IsAccepted
// FROM TopUsers tu JOIN Users u ON tu.UserId = u.Id JOIN Posts p ON u.Id = p.OwnerUserId JOIN TopPosts tp ON p.Id = tp.PostId WHERE tp.PostRank <= 5
// ORDER BY u.Reputation DESC, tp.UpVotes - tp.DownVotes DESC;
//
// A Reputation tie at the tenth user goes to the smaller user id.
fn q4003(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, accepted_answer_id, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pd = db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1)).and(post_type_id.eq(1)));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let tp = ranked(drain(pd.select((&cc).filt(|n| n > 5).and((&vc).opt()))), |&(_, (_, v))| Reverse(v.map_or(0, |v| v[0] - v[1])), false);
    let tp = rel(tp.into_iter().filter(|x| x.1 <= 5).map(|((p, (c, v)), _)| (p, (c, v.unwrap_or([0; 2])))).collect());
    let tidx: HashIdx<Id<Post>, (Id<Post>, (i64, [i64; 2]))> = (&tp).map(|(p, _)| p).inv().select(&tp).collect();
    let v = drain((&tidx).map(|(_, x)| x).and(owner_user.select(Ident::<User>::new().with(&tu))));
    let v = top_n(v, |&(p, ((_, a), u))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0] - a[1]), p), 0);
    rows(v.into_iter().map(|(p, ((c, a), u))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(if accepted_answer_id.get(p).is_some() { "Yes" } else { "No" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) as RankPerUser
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        COALESCE(SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostsWithBadges AS (SELECT up.UserId, up.Upvotes, up.Downvotes, up.BadgeCount, rp.PostId, rp.Title, rp.Score, rp.RankPerUser FROM UserStats up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId)
// SELECT pb.UserId, u.DisplayName, pb.Title, pb.Score, pb.Upvotes, pb.Downvotes, pb.BadgeCount,
//        CASE WHEN pb.RankPerUser = 1 THEN 'Top Post' WHEN pb.RankPerUser <= 3 THEN 'High Ranking Post' ELSE 'Regular Post' END AS PostCategory
// FROM PostsWithBadges pb JOIN Users u ON pb.UserId = u.Id
// WHERE pb.Upvotes - pb.Downvotes > 5 AND EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = pb.UserId AND p.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '3 months')
// ORDER BY pb.Score DESC, pb.BadgeCount DESC;
//
// UserStats is only read through the join with RankedPosts, so the post x vote x badge product is driven only for the users who own a recent post.
fn q1018(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rp = drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).select(owner_user.and(score)));
    let rp = per_group(ranked(rp, |&(_, (u, s))| (u, Reverse(s)), true), |&(_, (u, _))| u);
    let owners: MatSet<Id<User>> = rel(rp.iter().map(|x| x.0 .1 .0).collect()).map(|u| u).collect();
    let old = posts_of(db).select(Ident::<Post>::new().with(creation_date.lt(add_months(t0, -3))));
    let us = (&owners)
        .with(old)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (t, b)| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]
        });
    let v = drain(rel(rp.into_iter().map(|((p, (u, _)), r)| (u, (p, r))).collect()).select(Same::<(Id<User>, (Id<Post>, i64))>::new().and(Same::<(Id<User>, (Id<Post>, i64))>::new().map(|(u, _)| u).select((&us).filt(|a| a[0] - a[1] > 5)))));
    rows(v.into_iter().map(|(_, ((u, (p, r)), a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend(a.map(V::I));
        f.push(V::S(if r == 1 { "Top Post" } else if r <= 3 { "High Ranking Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH UserScore AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, COALESCE(COUNT(DISTINCT P.Id), 0) AS TotalPosts,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers,
//        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON U.Id = V.UserId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalBounty, TotalPosts, TotalQuestions, TotalAnswers, UserRank FROM UserScore WHERE UserRank <= 10),
// PostMetrics AS (SELECT P.Id AS PostId, P.Title, COALESCE(COUNT(C.Id), 0) AS CommentCount, COALESCE(MAX(PH.CreationDate), '2000-01-01') AS LastEditDate, P.ViewCount, P.Score,
//        CASE WHEN P.ClosedDate IS NOT NULL THEN 'Closed' WHEN P.AnswerCount = 0 THEN 'Unanswered' ELSE 'Answered' END AS Status
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.Id, P.Title, P.ViewCount, P.Score, P.ClosedDate, P.AnswerCount)
// SELECT TU.DisplayName, TM.Title, TM.CommentCount, TM.LastEditDate, TM.ViewCount, TM.Score, TM.Status, TU.TotalBounty, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers
// FROM TopUsers TU JOIN PostMetrics TM ON TU.UserId = TM.PostId WHERE (TM.Status = 'Answered' OR TM.Status = 'Unanswered') ORDER BY TU.TotalBounty DESC, TM.Score DESC LIMIT 5;
//
// UserRank reads only Reputation, so the ten users are picked first (a tie goes to the smaller user id) and the post x vote product is driven for those alone.
// `TU.UserId = TM.PostId` joins a user id to a post id, so it goes through the raw ids, and PostMetrics is computed only for the posts it reaches.
fn q4861(db: &'static So) -> String {
    let Post { post_type_id, closed_date, answer_count, score, .. } = &db.post;
    let tu = top_n(drain((&db.user.reputation).gt(0)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + b.flatten().unwrap_or(0), a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64]);
    let np = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let tm: MatSet<Id<Post>> = (&tu).select((&db.user.origid).select(&pidx)).collect();
    let status = |p: Id<Post>| if closed_date.get(p).is_some() { "Closed" } else if answer_count.get(p) == Some(0) { "Unanswered" } else { "Answered" };
    let pm = (&tm)
        .with(Ident::<Post>::new().map(status).filt(|s: &'static str| s != "Closed"))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, d.map_or(m, |d| m.max(d))));
    let v = drain((&us).and(&np).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&pm))));
    let v = top_n(v, |&(u, ((a, _), (p, _)))| (Reverse(a[0]), Reverse(score.get(p).unwrap()), u, p), 5);
    rows(v.into_iter().map(|(u, ((a, n), (p, (c, m))))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(c), V::T(if m == i64::MIN { date(2000, 1, 1) } else { m })]);
        f.extend(post_fields(db, p, &["views", "score"]));
        f.extend([V::S(status(p)), V::I(a[0]), V::I(n), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// UserPostStats AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionsCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswersCount,
//        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore, MAX(p.CreationDate) AS LastPostDate FROM Posts p GROUP BY p.OwnerUserId),
// TopUsers AS (SELECT u.Id, u.DisplayName, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(ps.QuestionsCount, 0) AS QuestionsCount, COALESCE(ps.AnswersCount, 0) AS AnswersCount, COALESCE(ps.TotalViews, 0) AS TotalViews, COALESCE(ps.TotalScore, 0) AS TotalScore, ps.LastPostDate
//     FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN UserPostStats ps ON u.Id = ps.OwnerUserId),
// RankedUsers AS (SELECT *, RANK() OVER (ORDER BY TotalScore DESC, TotalViews DESC) AS Rank FROM TopUsers)
// SELECT u.Rank, u.DisplayName, u.GoldBadges, u.SilverBadges, u.BronzeBadges, u.QuestionsCount, u.AnswersCount, u.TotalViews, u.TotalScore, u.LastPostDate
// FROM RankedUsers u WHERE u.Rank <= 10 AND u.TotalViews IS NOT NULL AND u.LastPostDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' ORDER BY u.Rank;
fn q4761(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, score, creation_date, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let ps = db.post.with(owner_user).group_by(owner_user).select(post_type_id.and(view_count.opt()).and(score).and(creation_date)).fold([0, 0, 0, 0, i64::MIN], |a, (((t, w), s), d)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4].max(d)]
    });
    let v = ranked(drain((&ub).and((&ps).opt())), |&(_, (_, p))| {
        let p = p.unwrap_or([0; 5]);
        (Reverse(p[3]), Reverse(p[2]))
    }, false);
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let v = drain(rel(v.into_iter().take_while(|x| x.1 <= 10).collect()).filt(|((_, (_, p)), _): ((Id<User>, ([i64; 3], Option<[i64; 5]>)), i64)| p.map_or(false, |p| p[4] >= since)));
    rows(v.into_iter().map(|(_, ((u, (b, p)), r))| {
        let p = p.unwrap();
        let mut f = vec![V::I(r), user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[3]), V::T(p[4])]);
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(c.Score) AS TotalCommentScore, SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE u.CreationDate < '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// EngagementSummary AS (SELECT ue.UserId, ue.DisplayName, ue.TotalPosts, ue.TotalQuestions, ue.TotalAnswers, ue.TotalCommentScore, ue.VoteCount, COALESCE(ub.GoldBadges, 0) AS GoldBadges,
//        COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges FROM UserEngagement ue LEFT JOIN UserBadges ub ON ue.UserId = ub.UserId)
// SELECT DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalCommentScore, VoteCount, GoldBadges, SilverBadges, BronzeBadges,
//        RANK() OVER (ORDER BY TotalPosts DESC, TotalQuestions DESC, TotalAnswers DESC) AS EngagementRank
// FROM EngagementSummary WHERE TotalPosts > 0 ORDER BY EngagementRank FETCH FIRST 100 ROWS ONLY;
fn q5360(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let users = || db.user.with((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(posts_of(db));
    let ue = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(comments_of(db).select(&db.comment.score).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 5], |a, ((t, c), v)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + c.unwrap_or(0), a[4] + matches!(v, Some(2 | 3)) as i64]);
    let np = users().group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = ranked(drain((&ue).and(&np).and((&ub).opt())), |&(_, ((a, n), _))| (Reverse(n), Reverse(a[0]), Reverse(a[1])), false);
    let v = top_n(v, |x| x.1, 100);
    rows(v.into_iter().map(|((u, ((a, n), b)), r)| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[4])];
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId IN (10, 11) THEN p.Id END) AS PostVoteCount, COUNT(DISTINCT v.Id) AS TotalVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RowNum,
//        p.OwnerUserId FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')),
// PostSummary AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, COALESCE(uv.UpVotesCount, 0) AS UserUpVotes, COALESCE(uv.DownVotesCount, 0) AS UserDownVotes,
//        (rp.Score - COALESCE(uv.DownVotesCount, 0) + COALESCE(uv.UpVotesCount, 0)) AS AdjustedScore FROM RecentPosts rp LEFT JOIN UserVoteSummary uv ON rp.OwnerUserId = uv.UserId WHERE rp.RowNum = 1)
// SELECT ps.Title, ps.OwnerDisplayName, ps.CreationDate, ps.Score, ps.UserUpVotes, ps.UserDownVotes, ps.AdjustedScore, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = ps.PostId) AS CommentCount,
//        (SELECT COUNT(*) FROM Badges b WHERE b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = ps.PostId)) AS BadgeCount
// FROM PostSummary ps WHERE ps.AdjustedScore > (SELECT AVG(AdjustedScore) FROM PostSummary) ORDER BY ps.AdjustedScore DESC;
//
// A CreationDate tie within an owner goes to the smaller post id. PostVoteCount and TotalVotes are never read. The AVG is a scalar subquery over PostSummary, computed once.
fn q23175(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ps: Fold<Id<Post>, [i64; 3]> = (&tp).group_by(Ident::<Post>::new()).select(score.and(owner_user.select(&uv).opt())).fold([0i64; 3], |_, (s, u)| {
        let u = u.unwrap_or([0; 2]);
        [u[0], u[1], s - u[1] + u[0]]
    });
    let (n, sum) = (&ps).fold_flat((0i64, 0i64), |(n, s), a| (n + 1, s + a[2]));
    let mean = sum as f64 / n as f64;
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&ps).filt(|a| a[2] as f64 > mean).and(&cc).and(owner_user.select(&bc)));
    rows(v.into_iter().map(|(p, ((a, c), b))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c), V::I(b)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        ROW_NUMBER() OVER (ORDER BY SUM(COALESCE(P.Score, 0)) DESC) AS Rank FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalViews, TotalScore FROM UserStats WHERE Rank <= 10),
// RecentVotes AS (SELECT V.UserId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes V WHERE V.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY V.UserId),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges B GROUP BY B.UserId)
// SELECT T.DisplayName, T.Reputation, T.PostCount, T.TotalViews, T.TotalScore, COALESCE(R.VoteCount, 0) AS RecentVoteCount, COALESCE(R.UpVotes, 0) AS RecentUpVotes,
//        COALESCE(R.DownVotes, 0) AS RecentDownVotes, COALESCE(B.GoldBadges, 0) AS GoldBadges, COALESCE(B.SilverBadges, 0) AS SilverBadges, COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM TopUsers T LEFT JOIN RecentVotes R ON T.UserId = R.UserId LEFT JOIN UserBadges B ON T.UserId = B.UserId ORDER BY T.TotalScore DESC, T.Reputation DESC;
//
// A TotalScore tie at the tenth user goes to the smaller user id.
fn q3627(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt().and(score)).opt()).fold([0i64; 3], |a, p| match p {
        Some((w, s)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s],
        None => a,
    });
    let tu = top_n(drain(&us), |&(u, a)| (Reverse(a[2]), u), 10);
    let tu = rel(tu);
    let tidx: HashIdx<Id<User>, (Id<User>, [i64; 3])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let Vote { user, creation_date, vote_type_id, .. } = &db.vote;
    let rv = db.vote.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(user).group_by(user).select(vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let mut v = drain((&tidx).map(|(_, a)| a).and((&rv).opt()).and((&ub).opt()));
    v.sort_by_key(|&(u, ((a, _), _))| (Reverse(a[2]), Reverse(db.user.reputation.get(u).unwrap())));
    rows(v.into_iter().map(|(u, ((a, r), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(r.unwrap_or([0; 3]).map(V::I));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// PostAnalytics AS (SELECT rp.PostId, rp.Title, rp.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM RankedPosts rp LEFT JOIN Votes v ON rp.PostId = v.PostId LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId)
//     GROUP BY rp.PostId, rp.Title, rp.Score),
// FilteredPosts AS (SELECT PostId, Title, Score, UpVotes, DownVotes, CommentCount, RANK() OVER (ORDER BY Score DESC, UpVotes DESC) AS PostRank FROM PostAnalytics WHERE Score > 0 OR CommentCount > 10)
// SELECT fp.PostId, fp.Title, fp.Score, fp.UpVotes, fp.DownVotes, fp.CommentCount, CASE WHEN fp.PostRank <= 10 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory,
//        COALESCE(ph.UserDisplayName, 'Anonymous') AS LastEditor
// FROM FilteredPosts fp LEFT JOIN Posts p ON fp.PostId = p.Id LEFT JOIN (SELECT PostId, UserDisplayName FROM PostHistory WHERE PostHistoryTypeId = 24 ORDER BY CreationDate DESC) ph ON p.Id = ph.PostId
// WHERE p.PostTypeId IN (1, 2) ORDER BY fp.PostRank;
//
// rn is never read. The correlated subquery looks a post up by its Id, which is unique, so it is the post's owner.
fn q4584(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, post_type_id, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let up = rp()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let fp = drain((&up).and(&cc).and(score).filt(|((_, c), s): (([i64; 2], i64), i64)| s > 0 || c > 10));
    let fp = ranked(fp, |&(p, ((a, _), _))| (Reverse(score.get(p).unwrap()), Reverse(a[0])), false);
    let fp = rel(fp.into_iter().map(|((p, ((a, c), _)), r)| (p, (a, c, r))).collect());
    let fidx: HashIdx<Id<Post>, (Id<Post>, ([i64; 2], i64, i64))> = (&fp).map(|(p, _)| p).inv().select(&fp).collect();
    let ph = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(24))).select((&db.post_history.user_display_name).opt());
    let v = drain((&fidx).map(|(_, x)| x).and(Ident::<Post>::new().with(post_type_id.is_in([1, 2]))).and(ph.opt()));
    rows(v.into_iter().map(|(p, ((a, _), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(a.0[0]), V::I(a.0[1]), V::I(a.1), V::S(if a.2 <= 10 { "Top Post" } else { "Regular Post" }), V::S(h.flatten().unwrap_or("Anonymous"))]);
        row(f)
    }))
}

// WITH UserBadgeSummary AS (SELECT U.Id AS UserId, U.Reputation, COUNT(B.Id) AS BadgeCount, MAX(B.Class) AS HighestBadgeClass,
//        CASE WHEN COUNT(B.Id) = 0 THEN 'No Badges' WHEN MAX(B.Class) = 1 THEN 'Gold' WHEN MAX(B.Class) = 2 THEN 'Silver' ELSE 'Bronze' END AS BadgeLevel
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.OwnerUserId, COALESCE(P.AcceptedAnswerId, -1) AS AnswerStatus,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS UserPostRank FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND P.PostTypeId = 1),
// AnswerStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS AnswerCount, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedCount FROM Posts P WHERE P.PostTypeId = 2 GROUP BY P.OwnerUserId)
// SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(UB.BadgeCount, 0) AS TotalBadges, R.PostId, R.Title, R.CreationDate, R.ViewCount, R.AnswerStatus, A.AnswerCount, A.AcceptedCount,
//        CASE WHEN R.UserPostRank = 1 THEN 'Newest post' WHEN R.UserPostRank <= 5 THEN 'Recent posts' ELSE 'Other posts' END AS PostCategory
// FROM Users U LEFT JOIN UserBadgeSummary UB ON U.Id = UB.UserId LEFT JOIN RecentPosts R ON U.Id = R.OwnerUserId LEFT JOIN AnswerStats A ON U.Id = A.OwnerUserId
// WHERE (UB.BadgeLevel = 'Gold' OR R.ViewCount > 100) AND (A.AcceptedCount > 0 OR R.AnswerStatus = -1) ORDER BY U.Reputation DESC, R.CreationDate DESC;
//
// A CreationDate tie within an owner's recent questions goes to the smaller post id.
fn q20310(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, accepted_answer_id, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |(n, m), c| (n + c.is_some() as i64, m.max(c.unwrap_or(0))));
    let rp = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.eq(1))).with(owner_user).select(owner_user));
    let rp = per_group(ranked(rp, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let rv = rel(rp.into_iter().map(|((p, u), r)| (u, (p, r))).collect());
    let ridx: HashIdx<Id<User>, (Id<User>, (Id<Post>, i64))> = (&rv).map(|(u, _)| u).inv().select(&rv).collect();
    let ast = db.post.with(post_type_id.eq(2)).with(owner_user).group_by(owner_user).select(accepted_answer_id.opt()).fold([0i64; 2], |a, x| [a[0] + 1, a[1] + x.is_some() as i64]);
    type R = ((i64, i64), Option<(Id<User>, (Id<Post>, i64))>);
    let v = drain(
        db.user
            .select((&ub).and((&ridx).opt()))
            .filt(|((n, m), r): R| (n > 0 && m == 1) || r.map_or(false, |(_, (p, _))| view_count.get(p).map_or(false, |w| w > 100)))
            .and((&ast).opt())
            .filt(|((_, r), a): (R, Option<[i64; 2]>)| a.map_or(false, |a| a[1] > 0) || r.map_or(false, |(_, (p, _))| accepted_answer_id.get(p).is_none())),
    );
    let v = top_n(v, |&(u, ((_, r), _))| (Reverse(db.user.reputation.get(u).unwrap()), r.map(|(_, (p, _))| Reverse(creation_date.get(p).unwrap()))), 0);
    rows(v.into_iter().map(|(u, (((n, _), r), a))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(match r {
            Some((_, (p, _))) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "views"]);
                g.push(V::I(accepted_answer_id.get(p).unwrap_or(-1)));
                g
            }
            None => (0..5).map(|_| V::Null).collect(),
        });
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        f.push(V::S(match r {
            Some((_, (_, 1))) => "Newest post",
            Some((_, (_, k))) if k <= 5 => "Recent posts",
            _ => "Other posts",
        }));
        row(f)
    }))
}

// WITH RECURSIVE UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, b.Name AS BadgeName, b.Class, b.Date, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY b.Date DESC) AS BadgeRank
//     FROM Users u JOIN Badges b ON u.Id = b.UserId WHERE b.Class = 1),
// PostViews AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.OwnerUserId),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, p.Title, RANK() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS ClosureRank FROM PostHistory ph JOIN Posts p ON p.Id = ph.PostId
//     WHERE ph.PostHistoryTypeId = 10),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, MAX(CASE WHEN p.OwnerUserId IS NOT NULL THEN 'Active' ELSE 'Inactive' END) AS UserActivity FROM Users u
//     LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY u.Id, u.DisplayName)
// SELECT u.DisplayName, ub.BadgeName AS GoldBadge, COALESCE(pv.VoteCount, 0) AS TotalVotes, COALESCE(pv.UpVotes, 0) AS UpVotes, COALESCE(pv.DownVotes, 0) AS DownVotes, cp.Title AS ClosedPostTitle,
//        cp.CreationDate AS ClosedOn, au.UserActivity
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId AND ub.BadgeRank = 1 LEFT JOIN PostViews pv ON u.Id = pv.OwnerUserId LEFT JOIN ClosedPosts cp ON u.Id = cp.PostId AND cp.ClosureRank = 1
// LEFT JOIN ActiveUsers au ON u.Id = au.UserId WHERE (ub.BadgeName IS NOT NULL OR pv.VoteCount > 0 OR cp.Title IS NOT NULL OR au.UserActivity = 'Active') ORDER BY u.Reputation DESC, u.DisplayName;
//
// WITH RECURSIVE, but no CTE refers to itself. A Date tie between a user's newest gold badges goes to the larger badge id. `u.Id = cp.PostId` joins a user id to a post id,
// so it goes through the raw ids.
fn q34459(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let Badge { user, class, date: bd, .. } = &db.badge;
    let gb = top_per(drain(db.badge.with(class.eq(1)).select(user)), |&(_, u)| u, |&(b, _)| (Reverse(bd.get(b).unwrap()), Reverse(b)), 1, false);
    let gb = rel(gb.into_iter().map(|(b, u)| (u, b)).collect());
    let gidx: HashIdx<Id<User>, (Id<User>, Id<Badge>)> = (&gb).map(|(u, _)| u).inv().select(&gb).collect();
    let pv = db.post.with(owner_user).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = drain(db.post_history.with(post_history_type_id.eq(10)).select(post));
    let cp = rel(top_per(cp, |&(_, p)| p, |&(h, _)| Reverse(hd.get(h).unwrap()), 1, true).into_iter().map(|(h, p)| (db.post.origid.get(p).unwrap(), (p, h))).collect());
    let cidx: HashIdx<i64, (i64, (Id<Post>, Id<PostHistory>))> = (&cp).map(|(o, _)| o).inv().select(&cp).collect();
    let active: MatSet<Id<User>> = db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user).collect();
    type R = (((Option<Id<Badge>>, Option<[i64; 3]>), Option<(Id<Post>, Id<PostHistory>)>), Option<Id<User>>);
    let v = drain(
        db.user
            .select(
                (&gidx)
                    .map(|(_, b)| b)
                    .opt()
                    .and(posts_of(db).select(&pv).opt())
                    .and((&db.user.origid).select((&cidx).map(|(_, x)| x)).opt())
                    .and(Ident::<User>::new().with(&active).opt()),
            )
            .filt(|(((b, p), c), a): R| b.is_some() || p.map_or(false, |p| p[0] > 0) || c.map_or(false, |(p, _)| db.post.title.get(p).is_some()) || a.is_some()),
    );
    rows(v.into_iter().map(|(u, (((b, p), c), a))| {
        let p = p.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "name"), b.map_or(V::Null, |b| V::S(db.badge.name.get(b).unwrap())), V::I(p[0]), V::I(p[1]), V::I(p[2])];
        f.extend(match c {
            Some((p, h)) => [ostr(db.post.title.get(p)), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if a.is_some() { "Active" } else { "Inactive" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.PostTypeId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v
//     WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY v.PostId),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, ur.Reputation, ur.BadgeCount, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId WHERE rp.Rank = 1)
// SELECT pd.PostId, pd.Title, pd.Score, pd.Reputation, pd.BadgeCount, pd.UpVotes, pd.DownVotes,
//        CASE WHEN pd.UpVotes > pd.DownVotes THEN 'More Upvotes' WHEN pd.UpVotes < pd.DownVotes THEN 'More Downvotes' ELSE 'Equal Votes' END AS VoteSummary
// FROM PostDetails pd WHERE pd.Score > 10 ORDER BY pd.Score DESC, pd.Reputation DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// A CreationDate tie within an owner goes to the smaller post id.
fn q20117(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let rv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_months(t0, -1)))).select(&db.vote.vote_type_id);
    let pd = (&tp).with(score.gt(10)).group_by(Ident::<Post>::new()).select(rv.opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&pd).and(owner_user.select(Ident::<User>::new().and(&bc))));
    let v = top_n(v, |&(p, (_, (u, _)))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([user_col(db, u, "rep"), V::I(b), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if a[0] > a[1] { "More Upvotes" } else if a[0] < a[1] { "More Downvotes" } else { "Equal Votes" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, MAX(ph.CreationDate) AS LastCloseDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rb.UserId, rb.DisplayName, COUNT(DISTINCT rp.Id) AS TotalPosts, COALESCE(SUM(cp.CloseCount), 0) AS TotalClosedPosts, COALESCE(SUM(rp.CommentCount), 0) AS TotalComments,
//        COALESCE(SUM(rb.GoldBadges), 0) AS TotalGoldBadges, COALESCE(SUM(rb.SilverBadges), 0) AS TotalSilverBadges, COALESCE(SUM(rb.BronzeBadges), 0) AS TotalBronzeBadges
// FROM UserStats rb JOIN RankedPosts rp ON rb.UserId = rp.OwnerUserId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId WHERE rb.Reputation > 100 GROUP BY rb.UserId, rb.DisplayName
// ORDER BY TotalPosts DESC, TotalComments DESC LIMIT 10;
//
// Rank is never read.
fn q1696(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select((&ub).and(recent.select((&cc).opt().and((&cp).opt()))))
        .fold([0i64; 6], |a, (b, (c, k))| [a[0] + 1, a[1] + k.unwrap_or(0), a[2] + c.unwrap_or(0), a[3] + b[0], a[4] + b[1], a[5] + b[2]]);
    let v = top_n(drain(&s), |&(u, a)| (Reverse(a[0]), Reverse(a[2]), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostHistorySummary AS (SELECT PH.PostId, COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE NULL END) AS CloseCount, COUNT(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 ELSE NULL END) AS ReopenCount,
//        COUNT(CASE WHEN PH.PostHistoryTypeId IN (12, 13) THEN 1 ELSE NULL END) AS DeleteCount FROM PostHistory PH GROUP BY PH.PostId),
// TopUsers AS (SELECT US.Id, US.DisplayName, US.Reputation, RANK() OVER (ORDER BY US.Reputation DESC) AS ReputationRank FROM UserStats US WHERE US.Reputation > 1000)
// SELECT TU.DisplayName, TU.Reputation, P.Title, P.CreationDate, COALESCE(PHS.CloseCount, 0) AS CloseCount, COALESCE(PHS.ReopenCount, 0) AS ReopenCount, COALESCE(PHS.DeleteCount, 0) AS DeleteCount,
//        US.TotalBounty, ROW_NUMBER() OVER (PARTITION BY TU.ReputationRank ORDER BY P.CreationDate DESC) AS PostRank
// FROM TopUsers TU JOIN Posts P ON TU.Id = P.OwnerUserId LEFT JOIN PostHistorySummary PHS ON P.Id = PHS.PostId JOIN UserStats US ON TU.Id = US.Id
// WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND (P.Title LIKE '%SQL%' OR P.Tags LIKE '%SQL%') ORDER BY TU.Reputation DESC, P.CreationDate DESC;
//
// A CreationDate tie inside PostRank goes to the smaller post id.
fn q4109(db: &'static So) -> String {
    let Post { creation_date, title, tags_str, .. } = &db.post;
    let rr = rel(ranked(drain((&db.user.reputation).gt(1000)), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), k)| (u, k)).collect());
    let ridx: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let tb = (&ridx).map(|(u, _)| u).group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let sql = title.opt().and(tags_str.opt()).filt(|(t, g): (Option<Str>, Option<Str>)| t.map_or(false, |t| t.contains("SQL")) || g.map_or(false, |g| g.contains("SQL")));
    let ps = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(current_date(), -1))).with(sql));
    let phs = db.post.group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id)).fold([0i64; 3], |a, t| {
        [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2] + matches!(t, 12 | 13) as i64]
    });
    let v = drain((&ridx).map(|(_, k)| k).and(&tb).and(ps.select(Ident::<Post>::new().and((&phs).opt()))));
    let v = per_group(ranked(v, |&(_, ((k, _), (p, _)))| (k, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, ((k, _), _))| k);
    let v = top_n(v, |&((u, (_, (p, _))), _)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())), 0);
    rows(v.into_iter().map(|((u, ((_, b), (p, h))), r)| {
        let h = h.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(h[0]), V::I(h[1]), V::I(h[2]), V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        AVG(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS AveragePostScore FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(DISTINCT A.Id) AS AnswerCount, MAX(P.Score) AS MaxScore
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Posts A ON P.Id = A.ParentId WHERE P.PostTypeId = 1
//     GROUP BY P.Id, P.Title, P.CreationDate),
// CombinedStats AS (SELECT U.DisplayName, U.TotalVotes, U.UpVotes, U.DownVotes, P.PostId, P.Title, P.CreationDate, P.TotalBounty, P.CommentCount, P.AnswerCount, P.MaxScore,
//        ROW_NUMBER() OVER (PARTITION BY U.UserId ORDER BY P.MaxScore DESC) AS Rank FROM UserVoteStats U JOIN PostDetails P ON U.UserId = P.PostId)
// SELECT CS.DisplayName, CS.TotalVotes, CS.UpVotes, CS.DownVotes, CS.Title, CS.CreationDate, CS.TotalBounty, CS.CommentCount, CS.AnswerCount, CS.MaxScore,
//        CASE WHEN CS.Rank = 1 THEN 'Top Post' ELSE 'Other Post' END AS PostRank
// FROM CombinedStats CS WHERE CS.AnswerCount > 0 ORDER BY CS.MaxScore DESC, CS.DisplayName ASC;
//
// `U.UserId = P.PostId` joins a user id to a post id, so it goes through the raw ids; each user then has at most one post, so Rank is always 1.
// AveragePostScore is never read.
fn q2617(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let qs = || db.post.with(post_type_id.eq(1)).with((&db.post.origid).select(&uidx));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let pd = qs()
        .group_by(Ident::<Post>::new())
        .select(bounty.opt().and(comments_of(db).opt()).and(children_of(db).opt()))
        .fold([0i64; 2], |a, ((b, c), _)| [a[0] + b.flatten().unwrap_or(0), a[1] + c.is_some() as i64]);
    let ac = qs().group_by(Ident::<Post>::new()).select(children_of(db)).fold(0i64, |n, _| n + 1);
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain((&pd).and(&ac).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&uv))));
    rows(v.into_iter().map(|(p, ((a, n), (u, x)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(x[0]), V::I(x[1]), V::I(x[2])];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        f.extend(post_fields(db, p, &["score"]));
        f.push(V::S("Top Post"));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank, COALESCE(P.AcceptedAnswerId, -1) AS AcceptedAnswerId
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '2 months' AND P.Score IS NOT NULL),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate AS ClosedDate, CR.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes CR ON PH.Comment::int = CR.Id WHERE PH.PostHistoryTypeId IN (10, 11)),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, CP.ClosedDate, CP.CloseReason,
//        (SELECT AVG(V.BountyAmount) FROM Votes V WHERE V.PostId = RP.PostId AND V.VoteTypeId IN (8, 9)) AS AverageBounty
//     FROM RankedPosts RP LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId WHERE RP.Rank <= 5)
// SELECT T.Title, T.CreationDate, T.Score, T.ViewCount, T.OwnerDisplayName, CASE WHEN T.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus,
//        COALESCE(T.CloseReason, 'N/A') AS CloseReason, CASE WHEN T.AverageBounty IS NULL THEN 'No Bounty' ELSE CONCAT('Average Bounty: ', T.AverageBounty) END AS BountyInfo,
//        CONCAT('<a href="https://example.com/posts/', T.PostId, '">View Post</a>') AS PostLink
// FROM TopPosts T ORDER BY T.Score DESC, T.CreationDate DESC;
//
// A (Score, CreationDate) tie in Rank goes to the smaller post id. AVG(BountyAmount) is a DOUBLE, printed the way DuckDB casts one to text; no answer row has a bounty, so that
// branch is not checked by the oracle.
fn q23898(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(date(2024, 10, 1), -2))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let closed = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]))).select(Ident::<PostHistory>::new().and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select(&db.vote.bounty_amount);
    let ab = (&tp).group_by(Ident::<Post>::new()).select(bounty).fold((0i64, 0i64), |(n, s), b| (n + 1, s + b));
    let v = drain((&tp).select(Ident::<Post>::new().and(closed.opt()).and((&ab).opt())));
    let v = top_n(v, |&(_, ((p, c), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, c.map(|c| c.0)), 0);
    rows(v.into_iter().map(|(_, ((p, c), b))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend(match c {
            Some((_, r)) => [V::S("Closed"), V::S(r)],
            None => [V::S("Active"), V::S("N/A")],
        });
        f.push(match b {
            Some((n, s)) => V::Owned(format!("Average Bounty: {:?}", s as f64 / n as f64)),
            None => V::S("No Bounty"),
        });
        f.push(V::Owned(format!("<a href=\"https://example.com/posts/{}\">View Post</a>", db.post.origid.get(p).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AcceptedAnswerId, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PopularityRank FROM Posts p WHERE p.PostTypeId = 1),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.DisplayName),
// PostHistoryInfo AS (SELECT ph.PostId, ph.CreationDate AS HistoryCreationDate, ph.Comment, COUNT(CASE WHEN pht.Name = 'Post Closed' THEN 1 END) AS CloseCount,
//        COUNT(CASE WHEN pht.Name = 'Post Reopened' THEN 1 END) AS ReopenCount FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId, ph.CreationDate, ph.Comment)
// SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalAnswers, ua.TotalQuestions, rp.Title, rp.CreationDate AS PostCreationDate, rp.Score, rp.ViewCount, COALESCE(pHi.CloseCount, 0) AS CloseCount,
//        COALESCE(pHi.ReopenCount, 0) AS ReopenCount, CASE WHEN rp.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Not Accepted' END AS AcceptanceStatus
// FROM UserActivity ua JOIN RankedPosts rp ON ua.UserId = rp.AcceptedAnswerId LEFT JOIN PostHistoryInfo pHi ON rp.PostId = pHi.PostId
// WHERE ua.TotalPosts > 5 AND rp.PopularityRank <= 5 ORDER BY ua.TotalPosts DESC, rp.Score DESC;
//
// `ua.UserId = rp.AcceptedAnswerId` joins a user id to a post id, so it goes through the raw ids. A CreationDate tie within an owner (NULL owners form one partition)
// goes to the smaller post id.
fn q30075(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, accepted_answer_id, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostHistory { post, creation_date: hd, comment, .. } = &db.post_history;
    let phi = db.post_history.group_by(post.and(hd).and(comment.opt())).select(htype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "Post Closed") as i64, a[1] + (n == "Post Reopened") as i64]);
    let pv = rel(drain(&phi));
    let pidx: HashIdx<Id<Post>, (((Id<Post>, i64), Option<Str>), [i64; 2])> = (&pv).map(|(((p, _), _), _)| p).inv().select(&pv).collect();
    let v = drain((&tp).select(Ident::<Post>::new().and(accepted_answer_id.select(&uidx).select(Ident::<User>::new().and((&ua).filt(|a| a[0] > 5)))).and((&pidx).map(|(_, a)| a).opt())));
    let v = top_n(v, |&(_, ((p, (_, a)), _))| (Reverse(a[0]), Reverse(score.get(p).unwrap())), 0);
    rows(v.into_iter().map(|(_, ((p, (u, a)), h))| {
        let h = h.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend([V::I(h[0]), V::I(h[1]), V::S("Accepted")]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalBounty, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserReputation),
// RecentChanges AS (SELECT PH.UserId, PH.PostId, PH.CreationDate, P.Title, MAX(PH.CreationDate) OVER (PARTITION BY PH.PostId) AS LastEditDate, PH.Comment, PH.UserDisplayName, PH.PostHistoryTypeId,
//        RANK() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS EditRank FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.PostHistoryTypeId IN (4, 5, 6))
// SELECT TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.TotalBounty, RC.PostId, RC.Title, RC.LastEditDate, RC.Comment, RC.UserDisplayName AS Editor,
//        RC.PostHistoryTypeId, RC.EditRank
// FROM TopUsers TU LEFT JOIN RecentChanges RC ON TU.UserId = RC.UserId WHERE TU.UserRank <= 10 AND (RC.LastEditDate IS NULL OR RC.EditRank = 1) ORDER BY TU.Reputation DESC, RC.LastEditDate DESC;
//
// UserRank reads only Reputation, so the ten users are picked first (a tie goes to the smaller user id) and the post x vote product is driven for those alone.
fn q1409(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let s = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(bounty.opt())).opt()).fold([0i64; 3], |a, x| match x {
        Some((t, b)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let np = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { post, user, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let edits = || db.post_history.with(post_history_type_id.is_in([4, 5, 6]));
    let last = edits().group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let by_user: HashIdx<Id<User>, Id<PostHistory>> = edits().with(user).select(user).inv().collect();
    let rc = (&by_user).select(Ident::<PostHistory>::new().and(hd.and(post.select(&last)).map(|(d, m)| d == m)));
    let v = drain((&s).and(&np).and(rc.opt()).filt(|(_, x): (([i64; 3], i64), Option<(Id<PostHistory>, bool)>)| x.map_or(true, |(_, r)| r)));
    let v: Vec<_> = v.into_iter().map(|(u, (a, x))| (u, (a, x.map(|(h, _)| h)))).collect();
    let v = top_n(v, |&(u, (_, h))| (Reverse(db.user.reputation.get(u).unwrap()), h.map(|h| Reverse(hd.get(h).unwrap()))), 0);
    rows(v.into_iter().map(|(u, ((a, n), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match h {
            Some(h) => {
                let p = post.get(h).unwrap();
                vec![
                    V::I(db.post.origid.get(p).unwrap()),
                    ostr(db.post.title.get(p)),
                    V::T(hd.get(h).unwrap()),
                    ostr(db.post_history.comment.get(h)),
                    ostr(db.post_history.user_display_name.get(h)),
                    V::I(post_history_type_id.get(h).unwrap()),
                    V::I(1),
                ]
            }
            None => (0..7).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RecentPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Reputation FROM RankedPosts rp WHERE rp.PostRank = 1),
// PostSummary AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Reputation, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(v.TotalVotes, 0) AS TotalVotes
//     FROM RecentPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) pc ON rp.PostId = pc.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId) v ON rp.PostId = v.PostId)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.Reputation, ps.CommentCount, ps.TotalVotes,
//        CASE WHEN ps.Reputation > 1000 THEN 'High Reputation User' ELSE 'User with Low Reputation' END AS UserReputationCategory,
//        CASE WHEN ps.Score > 10 THEN 'Popular Post' WHEN ps.Score BETWEEN 1 AND 10 THEN 'Moderate Post' ELSE 'Less Popular Post' END AS PostPopularityCategory
// FROM PostSummary ps WHERE ps.Reputation IS NOT NULL ORDER BY ps.Score DESC, ps.ViewCount DESC LIMIT 50;
//
// A (Score, CreationDate) tie within an owner goes to the smaller post id.
fn q1366(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&cc).and(&vc)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, (c, n))| {
        let s = score.get(p).unwrap();
        let r = db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep"]);
        f.extend([V::I(c), V::I(n), V::S(if r > 1000 { "High Reputation User" } else { "User with Low Reputation" })]);
        f.push(V::S(if s > 10 { "Popular Post" } else if s >= 1 { "Moderate Post" } else { "Less Popular Post" }));
        row(f)
    }))
}

// WITH RECURSIVE UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, (SELECT COUNT(*) FROM Badges B WHERE B.UserId = U.Id) AS TotalBadges, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U),
// TopQuestions AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, P.ViewCount, COALESCE(P.AcceptedAnswerId, -1) AS AcceptedAnswerId,
//        ROW_NUMBER() OVER (ORDER BY P.Score DESC, P.ViewCount DESC) AS QuestionRank FROM Posts P WHERE P.PostTypeId = 1 AND P.Score > 0),
// ActiveUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, U.LastAccessDate, CASE WHEN U.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 'Active' ELSE 'Inactive' END AS UserStatus FROM Users U),
// VoteCounts AS (SELECT V.PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes V GROUP BY V.PostId)
// SELECT Q.Title, Q.CreationDate, U.DisplayName AS OwnerDisplayName, U.Reputation, B.TotalBadges, QC.UpVotes, QC.DownVotes, CASE WHEN QC.UpVotes IS NOT NULL THEN QC.UpVotes ELSE 0 END AS UpVoteCount,
//        CASE WHEN QC.DownVotes IS NOT NULL THEN QC.DownVotes ELSE 0 END AS DownVoteCount, UA.UserStatus
// FROM TopQuestions Q JOIN Users U ON Q.OwnerUserId = U.Id LEFT JOIN UserBadges B ON U.Id = B.UserId LEFT JOIN VoteCounts QC ON Q.PostId = QC.PostId JOIN ActiveUsers UA ON U.Id = UA.Id
// WHERE Q.QuestionRank <= 10 ORDER BY Q.Score DESC, Q.ViewCount DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. A (Score, ViewCount) tie at the tenth question goes to the smaller post id.
fn q31949(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let tq = top_n(drain(db.post.with(post_type_id.eq(1).and(score.gt(0)))), |&(p, _)| (key(p), p), 10);
    let tq: MatSet<Id<Post>> = rel(tq.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let qc = (&tq).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let mut v = drain((&tq).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&bc))).and((&qc).opt())));
    v.sort_by_key(|&(_, ((p, _), _))| key(p));
    let since = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    rows(v.into_iter().map(|(_, ((p, (u, b)), q))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(b));
        f.extend(match q {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        let a = q.unwrap_or([0; 2]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if db.user.last_access_date.get(u).unwrap() >= since { "Active" } else { "Inactive" })]);
        row(f)
    }))
}

// Rewritten (rewrites/2096.sql): TopPosts' ORDER BY Score DESC LIMIT 10 ties at the cut; refined with `, P.Id`.
// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(COALESCE(P.Score, 0)) AS AvgScore, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentPostVotes AS (SELECT P.Id AS PostId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY P.Id),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, COALESCE(RPV.VoteCount, 0) AS TotalVotes, COALESCE(RPV.UpVotes, 0) - COALESCE(RPV.DownVotes, 0) AS Score FROM Posts P
//     LEFT JOIN RecentPostVotes RPV ON P.Id = RPV.PostId WHERE P.PostTypeId = 1 ORDER BY Score DESC, P.Id LIMIT 10)
// SELECT US.DisplayName, US.Reputation, US.PostCount, US.QuestionCount, US.AvgScore, TP.Title AS TopPostTitle, TP.TotalVotes, TP.Score, TP.PostId AS TopPostId, RPT.*
// FROM UserStats US INNER JOIN TopPosts TP ON US.UserId = TP.PostId LEFT JOIN PostHistory RPT ON TP.PostId = RPT.PostId
// WHERE RPT.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = TP.PostId) ORDER BY US.Reputation DESC, TP.Score DESC;
//
// `US.UserId = TP.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q2096(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score: pscore, .. } = &db.post;
    let rpv = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tp = drain(db.post.with(post_type_id.eq(1)).select((&rpv).opt()));
    let tp = top_n(tp, |&(p, r)| (Reverse(r.map_or(0, |a| a[1] - a[2])), p), 10);
    let tp = rel(tp.into_iter().map(|(p, r)| (p, r.unwrap_or([0; 3]))).collect());
    let tidx: HashIdx<Id<Post>, (Id<Post>, [i64; 3])> = (&tp).map(|(p, _)| p).inv().select(&tp).collect();
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(pscore)).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + s, a[3] + 1],
        None => [a[0], a[1], a[2], a[3] + 1],
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostHistory { creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(&db.post_history.post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let latest = history_of(db).select(Ident::<PostHistory>::new().and(hd.and((&db.post_history.post).select(&md)).map(|(d, m)| d == m)));
    let v = drain((&tidx).map(|(_, a)| a).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&us))).and(latest.filt(|(_, t): (Id<PostHistory>, bool)| t).map(|(h, _)| h)));
    let v = top_n(v, |&(_, ((a, (u, _)), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1] - a[2])), 0);
    rows(v.into_iter().map(|(p, ((a, (u, s)), h))| {
        let PostHistory { origid, post_history_type_id, post_id, revision_guid, user_id, user_display_name, comment, text, content_license, .. } = &db.post_history;
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(s[0]), V::I(s[1]), avg(s[2], s[3])]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(a[1] - a[2]), V::I(db.post.origid.get(p).unwrap())]);
        f.extend([
            V::I(origid.get(h).unwrap()),
            V::I(post_history_type_id.get(h).unwrap()),
            V::I(post_id.get(h).unwrap()),
            V::S(revision_guid.get(h).unwrap()),
            V::T(hd.get(h).unwrap()),
            oint(user_id.get(h)),
            ostr(user_display_name.get(h)),
            ostr(comment.get(h)),
            ostr(text.get(h)),
            ostr(content_license.get(h)),
        ]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// RecentVotes AS (SELECT v.PostId, COUNT(*) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId),
// PostHistoryAggregates AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId = 19 THEN 1 END) AS ProtectedCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, COALESCE(rv.VoteCount, 0) AS VoteCount, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes,
//        COALESCE(ph.CloseCount, 0) AS CloseCount, COALESCE(ph.ReopenCount, 0) AS ReopenCount, COALESCE(ph.ProtectedCount, 0) AS ProtectedCount,
//        CASE WHEN rp.Rank = 1 THEN 'Most Recent' ELSE 'Earlier' END AS PostRanking
// FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN PostHistoryAggregates ph ON rp.PostId = ph.PostId
// WHERE (rp.Score > 0 OR rv.VoteCount IS NOT NULL) AND rp.Rank <= 5 ORDER BY rp.CreationDate DESC;
fn q33881(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(owner_user));
    let r = per_group(ranked(v, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap())), true), |&(_, u)| u);
    let rp = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect());
    let ridx: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let rv = db.vote.with((&db.vote.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(&db.vote.post).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| {
        [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]
    });
    let pha = db.post.group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id)).fold([0i64; 3], |a, t| {
        [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2] + (t == 19) as i64]
    });
    let v = drain(
        (&ridx)
            .map(|(_, k)| k)
            .filt(|k| k <= 5)
            .and((&rv).opt())
            .and(score)
            .filt(|((_, r), s): ((i64, Option<[i64; 3]>), i64)| s > 0 || r.is_some())
            .and((&pha).opt()),
    );
    rows(v.into_iter().map(|(p, (((k, r), _), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(r.unwrap_or([0; 3]).map(V::I));
        f.extend(h.unwrap_or([0; 3]).map(V::I));
        f.push(V::S(if k == 1 { "Most Recent" } else { "Earlier" }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT b.UserId, COUNT(*) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 END) AS AcceptedAnswerCount,
//        SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// TopPostOwners AS (SELECT ps.OwnerUserId, ps.QuestionCount, ps.AcceptedAnswerCount, ps.TotalViews, COALESCE(ubc.GoldBadges, 0) AS GoldBadges, COALESCE(ubc.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ubc.BronzeBadges, 0) AS BronzeBadges, RANK() OVER (ORDER BY ps.TotalViews DESC) AS RankByViews FROM PostStatistics ps LEFT JOIN UserBadgeCounts ubc ON ps.OwnerUserId = ubc.UserId
//     WHERE ps.TotalViews > 1000),
// RecentPosts AS (SELECT p.Id, p.Title, p.LastActivityDate, p.OwnerUserId, CASE WHEN p.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS Status FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT u.DisplayName, tp.QuestionCount, tp.AcceptedAnswerCount, tp.TotalViews, tp.GoldBadges, tp.SilverBadges, tp.BronzeBadges, rp.Title, rp.Status
// FROM TopPostOwners tp JOIN Users u ON tp.OwnerUserId = u.Id LEFT JOIN RecentPosts rp ON rp.OwnerUserId = u.Id WHERE tp.RankByViews <= 10 ORDER BY tp.RankByViews;
//
// The NULL-owner group of PostStatistics has no user, so the join to Users drops it before it could rank; it is kept in the ranking, as in SQL.
fn q2785(db: &'static So) -> String {
    let Post { owner_user, post_type_id, accepted_answer_id, view_count, creation_date, closed_date, .. } = &db.post;
    let ps = db.post.group_by(owner_user.opt()).select(post_type_id.and(accepted_answer_id.opt()).and(view_count.opt())).fold([0i64; 4], |a, ((t, x), w)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2 && x.is_some()) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = ranked(drain((&ps).filt(|a| a[2] > 0 && a[3] > 1000)), |&(_, a)| Reverse(a[3]), false);
    let tp = rel(v.into_iter().take_while(|x| x.1 <= 10).collect());
    let tidx: HashIdx<Id<User>, ((Option<Id<User>>, [i64; 4]), i64)> = (&tp).flat_map(|((u, _), _): ((Option<Id<User>>, [i64; 4]), i64)| u).inv().select(&tp).collect();
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let mut v = drain((&tidx).map(|((_, a), r)| (a, r)).and((&ub).opt()).and(recent.opt()));
    v.sort_by_key(|&(_, (((_, r), _), _))| r);
    rows(v.into_iter().map(|(u, (((a, _), b), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[3])];
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend(match p {
            Some(p) => [ostr(db.post.title.get(p)), V::S(if closed_date.get(p).is_some() { "Closed" } else { "Open" })],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.PostTypeId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation > 1000 THEN 'High' WHEN u.Reputation BETWEEN 100 AND 1000 THEN 'Medium' ELSE 'Low' END AS ReputationLevel FROM Users u),
// PostHistoryEntry AS (SELECT ph.PostId, MAX(ph.CreationDate) FILTER (WHERE ph.PostHistoryTypeId = 12) AS LastDeleted, MAX(ph.CreationDate) FILTER (WHERE ph.PostHistoryTypeId = 10) AS LastClosed
//     FROM PostHistory ph GROUP BY ph.PostId),
// FinalOutput AS (SELECT rp.PostId, rp.Title, ur.ReputationLevel, COALESCE(ph.LastDeleted, ph.LastClosed) AS LastActionDate, rp.CommentCount FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId
//     LEFT JOIN PostHistoryEntry ph ON rp.PostId = ph.PostId WHERE rp.rn = 1 AND (COALESCE(ph.LastDeleted, ph.LastClosed) IS NOT NULL OR ur.ReputationLevel = 'High'))
// SELECT fo.Title, fo.ReputationLevel, fo.LastActionDate, CASE WHEN fo.LastActionDate IS NOT NULL THEN 'Active' ELSE 'Inactive' END AS PostStatus
// FROM FinalOutput fo WHERE fo.CommentCount > 10 ORDER BY fo.ReputationLevel DESC, fo.LastActionDate DESC LIMIT 100;
//
// rn reads only base columns, so each type's newest post is picked first (a tie goes to the smaller post id) and its comments counted afterwards.
fn q21161(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phe = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd))).fold((i64::MIN, i64::MIN), |(dl, cl), (t, d)| {
        (if t == 12 { dl.max(d) } else { dl }, if t == 10 { cl.max(d) } else { cl })
    });
    let level = |r: i64| if r > 1000 { "High" } else if r >= 100 { "Medium" } else { "Low" };
    let last = |h: Option<(i64, i64)>| h.and_then(|(d, c)| if d != i64::MIN { Some(d) } else if c != i64::MIN { Some(c) } else { None });
    let v = drain(
        (&cc)
            .filt(|n| n > 10)
            .and(owner_user.select(&db.user.reputation))
            .and((&phe).opt())
            .filt(|((_, r), h): ((i64, i64), Option<(i64, i64)>)| last(h).is_some() || level(r) == "High"),
    );
    let v = top_n(v, |&(p, ((_, r), h))| (Reverse(level(r)), Reverse(last(h)), p), 100);
    rows(v.into_iter().map(|(p, ((_, r), h))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::S(level(r)), ots(last(h)), V::S(if last(h).is_some() { "Active" } else { "Inactive" })]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputationCTE AS (SELECT Id, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM Users),
// PostVoteSummary AS (SELECT P.Id AS PostId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        MAX(P.Score) AS MaxScore, MIN(P.Score) AS MinScore FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id),
// PostHistoryGrouped AS (SELECT PH.PostId, PH.PostHistoryTypeId, COUNT(*) AS ChangeCount, MAX(PH.CreationDate) AS LastChangeDate FROM PostHistory PH GROUP BY PH.PostId, PH.PostHistoryTypeId),
// RecentPosts AS (SELECT P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, PT.Name AS PostType, PHG.ChangeCount AS EditCount FROM Posts P JOIN PostHistoryGrouped PHG ON P.Id = PHG.PostId
//     JOIN PostTypes PT ON P.PostTypeId = PT.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT U.DisplayName, U.Reputation, COALESCE(UI.Rank, 0) AS UserRank, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.PostType, COALESCE(VS.VoteCount, 0) AS TotalVotes,
//        COALESCE(VS.UpVotes, 0) AS TotalUpVotes, COALESCE(VS.DownVotes, 0) AS TotalDownVotes, CASE WHEN PHG.ChangeCount IS NULL THEN 'No Changes' ELSE CONCAT(PHG.ChangeCount, ' edits') END AS EditStatus
// FROM Users U LEFT JOIN UserReputationCTE UI ON U.Id = UI.Id LEFT JOIN RecentPosts RP ON U.Id = RP.Id LEFT JOIN PostVoteSummary VS ON RP.Id = VS.PostId LEFT JOIN PostHistoryGrouped PHG ON RP.Id = PHG.PostId
// WHERE U.Reputation >= 100 AND RP.Score > 0 ORDER BY U.Reputation DESC, RP.ViewCount DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. `U.Id = RP.Id` joins a user id to a post id, so it goes through the raw ids; RP.Score > 0 makes that join inner.
// A Reputation tie in UserRank goes to the smaller user id.
fn q34613(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let ur = rel(ranked(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), false).into_iter().map(|((u, _), k)| (u, k)).collect());
    let uidx: HashIdx<Id<User>, (Id<User>, i64)> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let phg = db.post_history.group_by(post.and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let gv = rel(drain(&phg));
    let gidx: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&gv).map(|((p, _), _)| p).inv().select(&gv).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let rp = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)));
    let vs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain(db.user.with((&db.user.reputation).ge(100)).select((&uidx).map(|(_, k)| k).and((&db.user.origid).select(&pidx).select(rp.and(&vs).and(&gidx).and(&gidx)))));
    let v = top_n(v, |&(u, (_, (((p, _), _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(db.post.view_count.get(p))), 0);
    rows(v.into_iter().map(|(u, (k, (((p, a), _), (_, n))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(k));
        f.extend(post_fields(db, p, &["title", "created", "score", "views", "type"]));
        f.extend(a.map(V::I));
        f.push(V::Owned(format!("{n} edits")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        COALESCE(c.CommentCount, 0) AS CommentCount FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopUsers AS (SELECT u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(DISTINCT p.Id) AS PostCount, u.Id AS OwnerUserId -- Added to allow join with RankedPosts
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId = 1 GROUP BY u.DisplayName, u.Id HAVING COUNT(DISTINCT p.Id) > 10),
// VotesSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, tu.DisplayName AS TopUser, tu.TotalScore, tu.PostCount, COALESCE(vs.UpVotes, 0) AS TotalUpVotes,
//        COALESCE(vs.DownVotes, 0) AS TotalDownVotes, CASE WHEN rp.Rank <= 5 THEN 'Top 5 Posts' ELSE 'Other Posts' END AS PostRankCategory
// FROM RankedPosts rp LEFT JOIN TopUsers tu ON rp.OwnerUserId = tu.OwnerUserId LEFT JOIN VotesSummary vs ON rp.PostId = vs.PostId WHERE rp.Rank <= 10 ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// A (Score, CreationDate) tie within an owner (the ownerless questions form one partition) goes to the smaller post id.
fn q224(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let r = per_group(ranked(v, |&(p, u)| (u, Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let rp = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect());
    let ridx: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let tu = db.post.with(post_type_id.eq(1)).with(owner_user).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + s, a[1] + 1]);
    let vs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&ridx).map(|(_, k)| k).filt(|k| k <= 10).and((&cc).opt()).and(owner_user.select(Ident::<User>::new().and((&tu).filt(|a| a[1] > 10))).opt()).and((&vs).opt()));
    rows(v.into_iter().map(|(p, (((k, c), t), s))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.push(V::I(c.unwrap_or(0)));
        f.extend(match t {
            Some((u, a)) => [user_col(db, u, "name"), V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(s.unwrap_or([0; 2]).map(V::I));
        f.push(V::S(if k <= 5 { "Top 5 Posts" } else { "Other Posts" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN 1 ELSE 0 END) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount, COUNT(c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT pv.Id) AS VoteCount FROM Posts p
//     LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes pv ON p.Id = pv.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.PostCount, ua.UpVotes, ua.DownVotes, ua.BadgeCount, ua.CommentCount, RANK() OVER (ORDER BY ua.UpVotes DESC, ua.PostCount DESC) AS Rank
//     FROM UserActivity ua WHERE ua.PostCount > 0)
// SELECT u.DisplayName AS UserName, u.PostCount, u.UpVotes, u.DownVotes, u.BadgeCount, ps.PostId, ps.Title AS PostTitle, ps.CreationDate AS PostCreatedDate, ps.ViewCount AS PostViewCount,
//        ps.Score AS PostScore, ps.CommentCount AS PostCommentCount, ps.VoteCount AS PostVoteCount
// FROM TopUsers u JOIN PostStatistics ps ON u.PostCount > 3 WHERE u.Rank <= 10 ORDER BY u.UpVotes DESC, u.PostCount DESC, ps.ViewCount DESC;
//
// The ON condition names only u, so TopUsers and PostStatistics are crossed. The user side is the full users x posts x votes x badges x comments product (3.1e8 rows).
fn q5304(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v, c) = p.map_or((0, None, None), |((t, v), c)| (t, v, c));
            [a[0] + matches!(t, 1 | 2) as i64, a[1] + (v == Some(2)) as i64, a[2] + (v == Some(3)) as i64, a[3] + b.is_some() as i64, a[4] + c.is_some() as i64]
        });
    let tu = ranked(drain((&ua).filt(|a| a[0] > 0)), |&(_, a)| (Reverse(a[1]), Reverse(a[0])), false);
    let tu = drain(rel(tu.into_iter().take_while(|x| x.1 <= 10).collect()).filt(|((_, a), _): ((Id<User>, [i64; 5]), i64)| a[0] > 3));
    let tu = rel(tu.into_iter().map(|x| x.1 .0).collect());
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ps = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let pv = recent().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let psr = rel(drain((&ps).and(&pv)));
    let mut v = Vec::new();
    (&tu).cross(&psr).drive(|_, ((u, a), (p, (c, n)))| v.push((u, a, p, c, n)));
    rows(v.into_iter().map(|(u, a, p, c, n)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, ARRAY_LENGTH(string_to_array(p.Tags, '>'), 1) AS TagCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate AS CloseDate, COUNT(*) FILTER (WHERE h.Id IS NOT NULL) AS CloseReasonCount FROM PostHistory ph LEFT JOIN PostHistoryTypes h ON ph.PostHistoryTypeId = h.Id
//     WHERE h.Name = 'Post Closed' GROUP BY ph.PostId, ph.CreationDate),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes, AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.TagCount, u.DisplayName AS OwnerDisplayName, us.BadgeCount, us.TotalUpVotes, us.TotalDownVotes, us.AvgReputation,
//        COALESCE(c.CloseDate, NULL) AS ClosedDate, CASE WHEN c.CloseReasonCount IS NULL THEN 'Not Closed' ELSE 'Closed' END AS ClosureStatus,
//        CASE WHEN rp.Rank <= 3 THEN 'Top Post' ELSE 'Regular Post' END AS PostTier
// FROM RankedPosts rp JOIN Users u ON rp.PostId = u.Id JOIN UserStatistics us ON u.Id = us.UserId LEFT JOIN ClosedPosts c ON rp.PostId = c.PostId
// WHERE us.AvgReputation > 1000 ORDER BY rp.Score DESC, rp.ViewCount DESC, rp.PostId DESC LIMIT 50;
//
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. A Score tie within an owner (the ownerless posts form one partition) goes to the smaller post id.
fn q21497(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, tags_str, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let r = per_group(ranked(v, |&(p, u)| (u, Reverse(score.get(p).unwrap()), p), false), |&(_, u)| u);
    let rp = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect());
    let ridx: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let us = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select((&db.user.up_votes).and(&db.user.down_votes).and(badges_of(db).opt())).fold([0i64; 4], |a, ((u, d), b)| {
        [a[0] + b.is_some() as i64, a[1] + u, a[2] + d, a[3] + 1]
    });
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(htype_name(db).filt(|n: Str| n == "Post Closed")).group_by(post.and(hd)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cv = rel(drain(&cp));
    let cidx: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&cv).map(|((p, _), _)| p).inv().select(&cv).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&ridx).map(|(_, k)| k).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&us))).and((&cidx).opt()));
    let v = top_n(v, |&(p, (_, c))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), Reverse(db.post.origid.get(p).unwrap()), c.map(|((_, d), _)| d))
    }, 50);
    rows(v.into_iter().map(|(p, ((k, (u, a)), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.push(oint(tags_str.get(p).map(|t| t.split('>').count() as i64)));
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(db.user.reputation.get(u).unwrap() as f64)]);
        f.extend(match c {
            Some(((_, d), _)) => [V::T(d), V::S("Closed")],
            None => [V::Null, V::S("Not Closed")],
        });
        f.push(V::S(if k <= 3 { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(COALESCE(p.Score, 0)) AS TotalScore, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount,
//        RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS PostRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, AvgViewCount, PostRank FROM UserStatistics WHERE TotalPosts > 0 ORDER BY PostRank LIMIT 10),
// PostComments AS (SELECT p.OwnerUserId AS UserId, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.OwnerUserId)
// SELECT u.UserId, u.DisplayName, u.Reputation, COALESCE(p.TopicCount, 0) AS TopicCount, COALESCE(c.CommentCount, 0) AS CommentCount,
//        CASE WHEN u.Reputation > 1000 THEN 'High Reputation' WHEN u.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
// FROM TopUsers u LEFT JOIN (SELECT OwnerUserId AS UserId, COUNT(DISTINCT Tags) AS TopicCount FROM Posts WHERE Tags IS NOT NULL AND Tags != '' GROUP BY OwnerUserId) p ON u.UserId = p.UserId
// LEFT JOIN PostComments c ON u.UserId = c.UserId ORDER BY u.Reputation DESC FETCH FIRST 10 ROWS ONLY;
fn q3535(db: &'static So) -> String {
    let Post { owner_user, tags_str, .. } = &db.post;
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let tu = top_n(drain(&np), |&(_, n)| Reverse(n), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let tc = db.post.with(owner_user).with(tags_str.filt(|t: Str| !t.is_empty())).group_by(owner_user).select(tags_str).count_distinct();
    let pc = db.post.with(owner_user).group_by(owner_user).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&tu).select(Ident::<User>::new().and((&tc).opt()).and((&pc).opt())));
    let v = top_n(v, |&(_, ((u, _), _))| Reverse(db.user.reputation.get(u).unwrap()), 10);
    rows(v.into_iter().map(|(_, ((u, t), c))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(t.unwrap_or(0)), V::I(c.unwrap_or(0)), V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" })]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        COUNT(DISTINCT V.PostId) AS TotalPostsVoted FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.OwnerUserId, COUNT(DISTINCT C.Id) AS CommentCount, ROW_NUMBER() OVER (ORDER BY P.CreationDate DESC) AS RowNum
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.OwnerUserId),
// PostHistorySummary AS (SELECT PH.PostId, MIN(PHT.Name) AS FirstAction, COUNT(PH.Id) AS TotalActions, MIN(PH.CreationDate) AS FirstActionDate, MAX(PH.CreationDate) AS LastActionDate
//     FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id GROUP BY PH.PostId)
// SELECT U.DisplayName, U.TotalUpvotes, U.TotalDownvotes, U.TotalPostsVoted, R.PostId, R.Title, R.CreationDate, R.Score, R.CommentCount, PHS.FirstAction, PHS.TotalActions, PHS.FirstActionDate,
//        PHS.LastActionDate, COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = R.PostId AND V.VoteTypeId = 2), 0) AS UpvoteCount,
//        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = R.PostId AND V.VoteTypeId = 3), 0) AS DownvoteCount
// FROM UserVoteStats U JOIN RecentPosts R ON R.OwnerUserId = U.UserId LEFT JOIN PostHistorySummary PHS ON R.PostId = PHS.PostId
// WHERE U.TotalPostsVoted > 10 AND R.RowNum <= 10 ORDER BY U.TotalUpvotes DESC, R.CreationDate DESC;
//
// RowNum reads only CreationDate, so the ten newest posts are picked first (a tie goes to the smaller post id).
fn q33265(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = top_n(drain(db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30)))), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    let rp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let Vote { user, post_id, vote_type_id, .. } = &db.vote;
    let uv = db.vote.with(user).group_by(user).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ud = db.vote.with(user).group_by(user).select(post_id).count_distinct();
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { creation_date: hd, .. } = &db.post_history;
    let phs = (&rp).group_by(Ident::<Post>::new()).select(history_of(db).select(htype_name(db).and(hd))).fold((None::<Str>, 0i64, i64::MAX, i64::MIN), |(m, n, lo, hi), (t, d)| {
        (Some(m.map_or(t, |m: Str| m.min(t))), n + 1, lo.min(d), hi.max(d))
    });
    let vc = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and((&uv).opt()).and((&ud).filt(|n| n > 10)))).and((&phs).opt()).and((&vc).opt()));
    let v = top_n(v, |&(p, (((_, ((_, u), _)), _), _))| (Reverse(u.map_or(0, |a| a[0])), Reverse(creation_date.get(p).unwrap())), 0);
    rows(v.into_iter().map(|(p, (((c, ((u, a), n)), h), x))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n)];
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.push(V::I(c));
        f.extend(match h {
            Some((m, n, lo, hi)) => [ostr(m), V::I(n), V::T(lo), V::T(hi)],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(x.unwrap_or([0; 2]).map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostEngagement AS (SELECT P.OwnerUserId, COUNT(*) AS TotalPosts, SUM(P.ViewCount) AS TotalViews, SUM(P.AnswerCount) AS TotalAnswers, SUM(P.CommentCount) AS TotalComments, SUM(P.FavoriteCount) AS TotalFavorites
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(B.BadgeCount, 0) AS BadgeCount, COALESCE(B.GoldBadges, 0) AS GoldBadges, COALESCE(B.SilverBadges, 0) AS SilverBadges,
//        COALESCE(B.BronzeBadges, 0) AS BronzeBadges, COALESCE(E.TotalPosts, 0) AS TotalPosts, COALESCE(E.TotalViews, 0) AS TotalViews, COALESCE(E.TotalAnswers, 0) AS TotalAnswers,
//        COALESCE(E.TotalComments, 0) AS TotalComments, COALESCE(E.TotalFavorites, 0) AS TotalFavorites FROM Users U LEFT JOIN UserBadges B ON U.Id = B.UserId LEFT JOIN PostEngagement E ON U.Id = E.OwnerUserId)
// SELECT U.UserId, U.DisplayName, U.BadgeCount, U.GoldBadges, U.SilverBadges, U.BronzeBadges, U.TotalPosts, U.TotalViews, U.TotalAnswers, U.TotalComments, U.TotalFavorites,
//        RANK() OVER (ORDER BY U.TotalViews DESC) AS ViewRank, RANK() OVER (ORDER BY U.TotalPosts DESC) AS PostRank
// FROM UserStats U ORDER BY U.TotalViews DESC, U.TotalPosts DESC LIMIT 10;
fn q26318(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, answer_count, comment_count, favorite_count, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| {
        [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]
    });
    let pe = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(owner_user)
        .select(view_count.opt().and(answer_count.opt()).and(comment_count).and(favorite_count.opt()))
        .fold([0i64; 5], |a, (((w, n), c), f)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + n.unwrap_or(0), a[3] + c, a[4] + f.unwrap_or(0)]);
    let v = drain((&ub).and((&pe).opt()));
    let v: Vec<_> = v.into_iter().map(|(u, (b, e))| (u, (b, e.unwrap_or([0; 5])))).collect();
    let v = ranked(v, |&(_, (_, e))| Reverse(e[1]), false);
    let v = ranked(v, |&((_, (_, e)), _)| Reverse(e[0]), false);
    let v = top_n(v, |&(((_, (_, e)), _), _)| (Reverse(e[1]), Reverse(e[0])), 10);
    rows(v.into_iter().map(|(((u, (b, e)), vr), pr)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(e.map(V::I));
        f.extend([V::I(vr), V::I(pr)]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u),
// RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.CreationDate, p.Title, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId, p.CreationDate, p.Title),
// TopPostDetails AS (SELECT rp.PostId, rp.Title, r.DisplayName AS OwnerDisplayName, rp.CommentCount, CASE WHEN rp.CommentCount > 10 THEN 'Hot' WHEN rp.CommentCount > 5 THEN 'Trending' ELSE 'Normal' END AS PostStatus
//     FROM RecentPosts rp JOIN RankedUsers r ON rp.OwnerUserId = r.UserId WHERE r.Rank <= 100),
// PostVoteCounts AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// FinalBenchmark AS (SELECT p.PostId, p.Title, p.OwnerDisplayName, p.CommentCount, pv.UpVotes, pv.DownVotes, p.PostStatus, COALESCE(pv.UpVotes - pv.DownVotes, 0) AS NetVotes,
//        CASE WHEN p.CommentCount IS NULL THEN 'No Comments' ELSE CONCAT(p.CommentCount, ' Comments') END AS CommentSummary FROM TopPostDetails p LEFT JOIN PostVoteCounts pv ON p.PostId = pv.PostId)
// SELECT f.PostId, f.Title, f.OwnerDisplayName, f.CommentSummary, f.UpVotes, f.DownVotes, f.NetVotes, f.PostStatus FROM FinalBenchmark f ORDER BY f.NetVotes DESC, f.CommentCount DESC LIMIT 50;
//
// Rank reads only Reputation, so the hundred users are picked first (a tie goes to the smaller user id).
fn q1633(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 100);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let rp = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user.select(Ident::<User>::new().with(&tu)));
    let cc = rp.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&cc).and((&pv).opt()));
    let v = top_n(v, |&(p, (c, x))| (Reverse(x.map_or(0, |a| a[0] - a[1])), Reverse(c), p), 50);
    rows(v.into_iter().map(|(p, (c, x))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.push(V::Owned(format!("{c} Comments")));
        f.extend(match x {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])],
            None => [V::Null, V::Null, V::I(0)],
        });
        f.push(V::S(if c > 10 { "Hot" } else if c > 5 { "Trending" } else { "Normal" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.ParentId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostWithComments AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount FROM RecentPosts rp
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON rp.PostId = c.PostId WHERE rp.RowNum = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate AS UserCreationDate, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.Score) AS TotalScore FROM Users u
//     JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStats)
// SELECT u.UserId, u.DisplayName, u.Reputation, u.PostCount, u.TotalScore, p.PostId, p.Title, p.CreationDate, p.Score, p.ViewCount,
//        CASE WHEN p.CommentCount > 10 THEN 'Hot' WHEN p.CommentCount > 0 THEN 'Active' ELSE 'Quiet' END AS ActivityStatus
// FROM TopUsers u JOIN PostWithComments p ON u.UserId = p.PostId WHERE u.Reputation > 1000 ORDER BY u.TotalScore DESC, p.ViewCount DESC LIMIT 50;
//
// `u.UserId = p.PostId` joins a user id to a post id, so it goes through the raw ids. A CreationDate tie within an owner (the ownerless posts form one partition)
// goes to the smaller post id. ScoreRank is never read.
fn q30115(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let us = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&cc).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&us))));
    let v = top_n(v, |&(p, (_, (_, a)))| (Reverse(a[1]), Reverse(view_count.get(p)), p), 50);
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.push(V::S(if c > 10 { "Hot" } else if c > 0 { "Active" } else { "Quiet" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(b.Class), 0) AS TotalBadges, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AverageScore, MAX(p.ViewCount) AS MaxViews, MIN(p.ViewCount) AS MinViews,
//        SUM(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPosts FROM Posts p GROUP BY p.OwnerUserId),
// CombinedStatistics AS (SELECT uu.DisplayName, uu.TotalBadges, ps.PostCount, ps.AverageScore, ps.MaxViews, ps.MinViews, ps.ClosedPosts, RANK() OVER (ORDER BY uu.TotalBadges DESC) AS BadgeRank,
//        RANK() OVER (ORDER BY ps.AverageScore DESC) AS ScoreRank FROM UserReputation uu LEFT JOIN PostStatistics ps ON uu.UserId = ps.OwnerUserId)
// SELECT cs.DisplayName, cs.TotalBadges, cs.PostCount, cs.AverageScore, cs.MaxViews, cs.MinViews, cs.ClosedPosts, CASE WHEN cs.BadgeRank IS NULL THEN 'Unranked' ELSE CAST(cs.BadgeRank AS VARCHAR) END AS BadgeRanking,
//        CASE WHEN cs.ScoreRank IS NULL THEN 'Unranked' ELSE CAST(cs.ScoreRank AS VARCHAR) END AS ScoreRanking
// FROM CombinedStatistics cs WHERE cs.PostCount > 5 AND cs.TotalBadges > 0 AND COALESCE(cs.ClosedPosts, 0) < (SELECT AVG(ClosedPosts) FROM PostStatistics) ORDER BY cs.MaxViews DESC, cs.AverageScore DESC;
//
// TotalBadges sums Class over the badges x posts product of each user. The ranks are taken over every user, before the WHERE; the AVG over PostStatistics includes its NULL-owner group.
fn q20692(db: &'static So) -> String {
    let Post { owner_user, score, view_count, closed_date, .. } = &db.post;
    let tb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt())).fold(0i64, |s, (c, _)| s + c.unwrap_or(0));
    let ps = db.post.group_by(owner_user.opt()).select(score.and(view_count.opt()).and(closed_date.opt())).fold([0, 0, i64::MIN, i64::MAX, 0, 0], |a, ((s, w), c)| {
        [a[0] + 1, a[1] + s, w.map_or(a[2], |w| a[2].max(w)), w.map_or(a[3], |w| a[3].min(w)), a[4] + c.is_some() as i64, a[5] + w.is_some() as i64]
    });
    let (gn, gs) = (&ps).fold_flat((0i64, 0i64), |(n, s), a| (n + 1, s + a[4]));
    let mean = gs as f64 / gn as f64;
    let mut v = drain(db.user.select((&tb).and(Ident::<User>::new().map(Some).select(&ps).opt())));
    let avg_key = |a: Option<[i64; 6]>| a.map(|a| fkey(a[1] as f64 / a[0] as f64));
    let v = ranked(std::mem::take(&mut v), |&(_, (b, _))| Reverse(b), false);
    let v = ranked(v, |&((_, (_, a)), _)| (avg_key(a).is_none(), Reverse(avg_key(a))), false);
    type R = (((Id<User>, (i64, Option<[i64; 6]>)), i64), i64);
    let v = drain(rel(v).filt(|(((_, (b, a)), _), _): R| a.map_or(false, |a| a[0] > 5 && (a[4] as f64) < mean) && b > 0));
    let v = top_n(v.into_iter().map(|x| x.1).collect(), |&(((_, (_, a)), _), _)| {
        let a = a.unwrap();
        (a[5] == 0, Reverse(a[2]), Reverse(avg_key(Some(a))))
    }, 0);
    rows(v.into_iter().map(|(((u, (b, a)), br), sr)| {
        let a = a.unwrap();
        row(vec![user_col(db, u, "name"), V::I(b), V::I(a[0]), avg(a[1], a[0]), omax(a[2], a[5]), omax(a[3], a[5]), V::I(a[4]), V::Owned(br.to_string()), V::Owned(sr.to_string())])
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, CreationDate, COALESCE(AboutMe, 'No description provided') AS UserAbout, Views, UpVotes, DownVotes FROM Users WHERE Reputation > 1000),
// PostSummary AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        MIN(P.CreationDate) AS FirstPostDate, MAX(P.LastActivityDate) AS LastPostDate FROM Posts P GROUP BY P.OwnerUserId),
// UserPostAnalytics AS (SELECT U.UserId, U.Reputation, P.TotalPosts, P.TotalQuestions, P.TotalAnswers, P.FirstPostDate, P.LastPostDate, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM UserReputation U LEFT JOIN PostSummary P ON U.UserId = P.OwnerUserId),
// ClosedPostAnalytics AS (SELECT PH.UserId, COUNT(*) AS TotalCloseVotes, COUNT(DISTINCT PH.PostId) AS ClosedPosts FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId)
// SELECT UPA.UserId, UPA.Reputation, UPA.TotalPosts, UPA.TotalQuestions, UPA.TotalAnswers, UPA.FirstPostDate, UPA.LastPostDate, COALESCE(CPA.TotalCloseVotes, 0) AS TotalCloseVotes,
//        COALESCE(CPA.ClosedPosts, 0) AS ClosedPosts, CASE WHEN UPA.ReputationRank <= 10 THEN 'Top User' WHEN UPA.Reputation >= 5000 THEN 'Expert' ELSE 'Novice' END AS UserCategory
// FROM UserPostAnalytics UPA LEFT JOIN ClosedPostAnalytics CPA ON UPA.UserId = CPA.UserId WHERE UPA.TotalPosts IS NOT NULL ORDER BY UPA.Reputation DESC;
fn q2642(db: &'static So) -> String {
    let Post { owner_user, post_type_id, creation_date, last_activity_date, .. } = &db.post;
    let ps = db.post.with(owner_user).group_by(owner_user).select(post_type_id.and(creation_date).and(last_activity_date)).fold([0, 0, 0, i64::MAX, i64::MIN], |a, ((t, c), l)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].min(c), a[4].max(l)]
    });
    let v = ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select((&ps).opt())), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = drain(rel(v).filt(|((_, p), _): ((Id<User>, Option<[i64; 5]>), i64)| p.is_some()));
    let PostHistory { user, post, post_history_type_id, .. } = &db.post_history;
    let cv = db.post_history.with(post_history_type_id.eq(10)).with(user).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cd = db.post_history.with(post_history_type_id.eq(10)).with(user).group_by(user).select(post).count_distinct();
    let v = drain(rel(v.into_iter().map(|x| x.1).collect()).select(Same::<((Id<User>, Option<[i64; 5]>), i64)>::new().and(Same::<((Id<User>, Option<[i64; 5]>), i64)>::new().map(|((u, _), _)| u).select((&cv).opt().and((&cd).opt())))));
    rows(v.into_iter().map(|(_, (((u, p), r), (n, d)))| {
        let p = p.unwrap();
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), V::T(p[3]), V::T(p[4]), V::I(n.unwrap_or(0)), V::I(d.unwrap_or(0))]);
        f.push(V::S(if r <= 10 { "Top User" } else if rep >= 5000 { "Expert" } else { "Novice" }));
        row(f)
    }))
}

// Rewritten (rewrites/24303.sql): RankByScore's ORDER BY refined with `, p.Id` (tag-wiki posts all score 0 and tie at the fifth place).
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id) AS RankByScore
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserScores AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.Reputation),
// PostsWithBadges AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, b.Name AS BadgeName, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY b.Date DESC) AS BadgeRank
//     FROM Posts p LEFT JOIN Badges b ON p.OwnerUserId = b.UserId WHERE b.Class = 1)
// SELECT u.DisplayName, us.Reputation, COALESCE(ps.BadgeName, 'No Badge') AS LatestBadge, SUM(ps.Score) AS TotalScore, SUM(COALESCE(rp.Score, 0)) AS YearlyTopScores,
//        COUNT(DISTINCT p.Id) FILTER (WHERE rp.RankByScore <= 5) AS Top5Posts, COUNT(DISTINCT c.Id) AS CommentCount
// FROM Users u JOIN UserScores us ON u.Id = us.UserId LEFT JOIN PostsWithBadges ps ON u.Id = ps.PostId LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN RankedPosts rp ON rp.PostId = p.Id
// LEFT JOIN Comments c ON c.PostId = p.Id WHERE us.PostCount > 0 GROUP BY u.Id, u.DisplayName, us.Reputation, ps.BadgeName HAVING SUM(ps.Score) > 0 OR COUNT(DISTINCT p.Id) > 1
// ORDER BY us.Reputation DESC, TotalScore DESC LIMIT 10;
//
// `u.Id = ps.PostId` joins a user id to a post id, so it goes through the raw ids; BadgeRank is never read, so every gold badge of that post's owner is a row. The joined
// (user, badge) rows are materialised and grouped by (user, badge name); every group of one user sees all of that user's posts, so its distinct post and comment counts are
// the user's own.
fn q24303(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let recent = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let rk = per_group(ranked(recent, |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), false), |&(_, t)| t);
    let rk = rel(rk.into_iter().map(|((p, _), r)| (p, r)).collect());
    let ridx: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let users = || db.user.with(posts_of(db));
    let jr: MatSet<(Id<User>, Option<(Id<Post>, Id<Badge>)>)> = users().select(Ident::<User>::new().and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(owner_user.select(gold))).opt())).collect();
    type J = (Id<User>, Option<(Id<Post>, Id<Badge>)>);
    let g = (&jr)
        .group_by(Same::<J>::new().map(|(u, _): J| u).and(Same::<J>::new().map(|(_, b): J| b.map(|(_, b)| db.badge.name.get(b).unwrap()))))
        .select(Same::<J>::new().map(|(_, b): J| b.map(|(p, _)| score.get(p).unwrap())).and(Same::<J>::new().map(|(u, _): J| u).select(posts_of(db).select((&ridx).map(|(_, r)| r).opt().and(score).and(comments_of(db).opt())).opt())))
        .fold((0i64, 0i64, 0i64), |(n, s, y), (ps, p)| {
            let ry = p.map_or(0, |((r, sc), _)| if r.is_some() { sc } else { 0 });
            (n + ps.is_some() as i64, s + ps.unwrap_or(0), y + ry)
        });
    let np = users().group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let t5 = users().group_by(Ident::<User>::new()).select(posts_of(db).select((&ridx).map(|(_, r)| r).filt(|r| r <= 5)).opt()).fold(0i64, |n, r| n + r.is_some() as i64);
    let cc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let gv = rel(drain(&g));
    type G = ((Id<User>, Option<Str>), (i64, i64, i64));
    let v = drain(
        (&gv).select(Same::<G>::new().and(Same::<G>::new().map(|((u, _), _): G| u).select((&np).and(&t5).and(&cc)))).filt(|((_, (n, s, _)), ((p, _), _)): (G, ((i64, i64), i64))| (n > 0 && s > 0) || p > 1),
    );
    let v = top_n(v, |&(_, (((u, b), (n, s, _)), _))| (Reverse(db.user.reputation.get(u).unwrap()), n == 0, Reverse(s), b), 10);
    rows(v.into_iter().map(|(_, (((u, b), (n, s, y)), ((_, t), c)))| {
        row(vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::S(b.unwrap_or("No Badge")), nullable(s, n), V::I(y), V::I(t), V::I(c)])
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotesCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotesCount, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate),
// RecentPosts AS (SELECT P.Id, P.Title, P.CreationDate, P.Score, P.OwnerUserId, DENSE_RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank FROM Posts P
//     WHERE P.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')),
// UserRecentPosts AS (SELECT U.DisplayName, COUNT(RP.Id) AS RecentPostsCount, AVG(RP.Score) AS AvgPostScore FROM UserStats U LEFT JOIN RecentPosts RP ON U.UserId = RP.OwnerUserId GROUP BY U.DisplayName)
// SELECT U.DisplayName, U.Reputation, U.TotalPosts, U.TotalComments, COALESCE(URP.RecentPostsCount, 0) AS RecentPostsCount, COALESCE(URP.AvgPostScore, 0) AS AvgPostScore,
//        CASE WHEN U.Reputation > 1000 THEN 'High Reputation' WHEN U.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
// FROM UserStats U LEFT JOIN UserRecentPosts URP ON U.DisplayName = URP.DisplayName WHERE U.TotalPosts > 0 ORDER BY U.Reputation DESC, URP.RecentPostsCount DESC LIMIT 10;
//
// UserRecentPosts groups by DisplayName, so users who share a name share its counts. UpVotesCount and DownVotesCount are never read.
fn q361(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(score);
    let urp = db.user.group_by(&db.user.display_name).select(recent.opt()).fold((0i64, 0i64), |(n, s), x| (n + x.is_some() as i64, s + x.unwrap_or(0)));
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&np).and(&cc).and((&db.user.display_name).select(&urp)));
    let v = top_n(v, |&(u, (_, (n, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n)), 10);
    rows(v.into_iter().map(|(u, ((p, c), (n, s)))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p), V::I(c), V::I(n), if n == 0 { V::F(0.0) } else { avg(s, n) }]);
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
//        COALESCE(SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalBadges FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT us.DisplayName, us.Reputation, us.TotalUpVotes, us.TotalDownVotes, COALESCE(phs.CloseCount, 0) AS CloseCount, COALESCE(phs.ReopenCount, 0) AS ReopenCount, COALESCE(phs.DeleteCount, 0) AS DeleteCount,
//        rp.Title, rp.CreationDate, rp.ViewCount, rp.Score
// FROM UserStats us LEFT JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId AND rp.Rank = 1 LEFT JOIN PostHistoryStats phs ON rp.PostId = phs.PostId
// WHERE us.Reputation > 100 AND us.TotalBadges > 2 ORDER BY us.Reputation DESC, rp.ViewCount DESC FETCH FIRST 100 ROWS ONLY;
//
// A CreationDate tie within an owner goes to the smaller post id.
fn q20462(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]);
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let top = rel(top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false).into_iter().map(|(p, u)| (u, p)).collect());
    let tidx: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&top).map(|(u, _)| u).inv().select(&top).collect();
    let phs = db.post.group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id)).fold([0i64; 3], |a, t| {
        [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2] + (t == 12) as i64]
    });
    let v = drain((&us).filt(|a| a[2] > 2).and((&tidx).map(|(_, p)| p).select(Ident::<Post>::new().and((&phs).opt())).opt()));
    let v = top_n(v, |&(u, (_, r))| (Reverse(db.user.reputation.get(u).unwrap()), r.map(|(p, _)| view_count.get(p)).map(|w| (w.is_none(), Reverse(w)))), 100);
    rows(v.into_iter().map(|(u, (a, r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(r.and_then(|(_, h)| h).unwrap_or([0; 3]).map(V::I));
        f.extend(match r {
            Some((p, _)) => post_fields(db, p, &["title", "created", "views", "score"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.Score, P.CreationDate, COALESCE(UPV.TotalVotes, 0) AS OwnerTotalVotes, DENSE_RANK() OVER (PARTITION BY P.Id ORDER BY P.Score DESC) AS ScoreRank
//     FROM Posts P LEFT JOIN UserVoteStats UPV ON P.OwnerUserId = UPV.UserId),
// CombinedPosts AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.OwnerTotalVotes, PH.Comment, PH.CreationDate AS HistoryDate FROM PostStats PS LEFT JOIN PostHistory PH ON PS.PostId = PH.PostId
//     WHERE PH.PostHistoryTypeId IN (10, 11) AND PH.CreationDate BETWEEN PS.CreationDate AND cast('2024-10-01 12:34:56' as timestamp)),
// FinalResults AS (SELECT CP.PostId, CP.Title, CP.CreationDate, CP.Score, CP.OwnerTotalVotes, COUNT(DISTINCT CP.Comment) AS CommentCount, MAX(CP.HistoryDate) AS LastHistoryDate FROM CombinedPosts CP
//     GROUP BY CP.PostId, CP.Title, CP.CreationDate, CP.Score, CP.OwnerTotalVotes)
// SELECT FR.PostId, FR.Title, FR.CreationDate, FR.Score, FR.OwnerTotalVotes, FR.CommentCount, CASE WHEN FR.LastHistoryDate IS NOT NULL THEN 'Edited' ELSE 'Not Edited' END AS EditStatus
// FROM FinalResults FR WHERE FR.Score > 0 ORDER BY FR.Score DESC, FR.CreationDate DESC LIMIT 10;
//
// The WHERE on PH makes the LEFT JOIN inner, so a post needs a close/reopen row between its creation and the reference time. ScoreRank is never read.
fn q2687(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ch = db.post_history.with(post_history_type_id.is_in([10, 11])).with(hd.and(post.select(creation_date)).filt(move |(d, c): (i64, i64)| d >= c && d <= t0));
    let cd = ch.group_by(post).select(comment).count_distinct();
    let any = db.post_history.with(post_history_type_id.is_in([10, 11])).with(hd.and(post.select(creation_date)).filt(move |(d, c): (i64, i64)| d >= c && d <= t0)).group_by(post).select(hd).fold(0i64, |n, _| n + 1);
    let tv = db.user.group_by(Ident::<User>::new()).select(votes_by(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&any).and(score.gt(0)).and((&cd).opt()).and(owner_user.select(&tv).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(p, (((_, _), c), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(t.unwrap_or(0)), V::I(c.unwrap_or(0)), V::S("Edited")]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        COALESCE(SUM(V.BountyAmount), 0) AS TotalBountyAmount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalBountyAmount, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank, RANK() OVER (ORDER BY TotalBountyAmount DESC) AS BountyRank
//     FROM UserActivity),
// PostStatistics AS (SELECT P.Title, P.CreationDate, P.ViewCount, COALESCE(NULLIF(P.AcceptedAnswerId, -1), 0) AS AcceptedAnswerCount, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.AcceptedAnswerId)
// SELECT TU.DisplayName, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.TotalBountyAmount, PS.Title, PS.CreationDate, PS.ViewCount, PS.AcceptedAnswerCount, PS.CommentCount,
//        COALESCE(PH.Comment, 'No comments available') AS LastEditComment
// FROM TopUsers TU JOIN PostStatistics PS ON TU.UserId = (SELECT OwnerUserId FROM Posts ORDER BY CreationDate DESC LIMIT 1 OFFSET 0) LEFT JOIN PostHistory PH ON PS.Title = PH.Comment
// WHERE TU.PostRank <= 10 OR TU.BountyRank <= 10 ORDER BY TU.TotalPosts DESC, TU.TotalBountyAmount DESC;
//
// The ON condition names only TU (the owner of the newest post; no CreationDate tie at the top), so the users and PostStatistics are crossed. CURRENT_TIMESTAMP is a
// TIMESTAMPTZ, so CreationDate is read as New York local time.
fn q3510(db: &'static So) -> String {
    let Post { owner_user, post_type_id, creation_date, accepted_answer_id, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 4], |a, (t, b)| [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + b.flatten().unwrap_or(0)]);
    let newest = top_n(drain(db.post.select(owner_user.opt())), |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1);
    let owner: MatSet<Id<User>> = rel(newest.into_iter().map(|x| x.1).collect()).flat_map(|u: Option<Id<User>>| u).collect();
    let v = ranked(drain(&ua), |&(_, a)| Reverse(a[0]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[3]), false);
    type T = (((Id<User>, [i64; 4]), i64), i64);
    let tu = rel(drain(rel(v).filt(|((_, p), b): T| p <= 10 || b <= 10)).into_iter().map(|x| x.1).collect());
    let tu = rel(drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(((u, _), _), _): T| u).select(Ident::<User>::new().with(&owner))))).into_iter().map(|x| x.1 .0).collect());
    let since = add_years(now_utc(), -1);
    let recent = || db.post.with(creation_date.map(ny_to_utc).ge(since));
    let ps = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let by_comment: HashIdx<Str, Id<PostHistory>> = db.post_history.select(&db.post_history.comment).inv().collect();
    let psr = rel(drain((&ps).and((&db.post.title).select(&by_comment).opt())));
    let mut v = Vec::new();
    (&tu).cross(&psr).drive(|_, ((((u, a), _), _), (p, (c, h)))| v.push((u, a, p, c, h)));
    let v = top_n(v, |&(_, a, _, _, _)| (Reverse(a[0]), Reverse(a[3])), 0);
    rows(v.into_iter().map(|(u, a, p, c, h)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])];
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.extend([V::I(accepted_answer_id.get(p).filter(|&x| x != -1).unwrap_or(0)), V::I(c), V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No comments available"))]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, UpVoteCount, DownVoteCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStatistics),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, CASE WHEN PH.PostId IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(DISTINCT PL.RelatedPostId) AS RelatedPostCount FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId AND PH.PostHistoryTypeId IN (10, 11) LEFT JOIN Comments C ON P.Id = C.PostId
//     LEFT JOIN PostLinks PL ON P.Id = PL.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId, PH.PostId)
// SELECT TU.DisplayName, TU.Reputation, PD.PostId, PD.Title, PD.CreationDate, PD.PostStatus, PD.CommentCount, PD.RelatedPostCount
// FROM TopUsers TU JOIN PostDetails PD ON TU.UserId = PD.OwnerUserId WHERE TU.Rank <= 10 ORDER BY TU.Reputation DESC, PD.PostStatus;
//
// Rank reads only Reputation, so the ten users are picked first (a tie goes to the smaller user id); PostDetails is only read for their posts. Grouping by PH.PostId puts a
// post with close/reopen rows in one group whose comment count runs over the history x comment x link product.
fn q2017(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let tp: MatSet<Id<Post>> = (&tu).select(posts_of(db)).map(|p| p).collect();
    let tp: MatSet<Id<Post>> = rel(drain(&tp).into_iter().map(|x| x.1).collect()).map(|p| p).collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let pd = (&tp)
        .group_by(Ident::<Post>::new())
        .select(closes.opt().and(comments_of(db).opt()).and(links_of(db).opt()))
        .fold((false, 0i64), |(h, n), ((x, c), _)| (h || x.is_some(), n + c.is_some() as i64));
    let rc = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let v = drain((&pd).and((&rc).opt()).and(&db.post.owner_user));
    rows(v.into_iter().map(|(p, (((h, c), r), u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::S(if h { "Closed" } else { "Open" }), V::I(c), V::I(r.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserVoteStats AS (SELECT u.Id AS UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId IN (2, 4) THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(ups.UpVotes, 0) AS UpVotes, COALESCE(ups.DownVotes, 0) AS DownVotes, COALESCE(pc.CommentCount, 0) AS CommentCount,
//        COALESCE(phs.EditCount, 0) AS EditCount, phs.LastEditDate, u.Reputation, u.DisplayName, CASE WHEN rp.RowNum = 1 THEN 'Latest Post' ELSE 'Earlier Post' END AS PostStatus
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserVoteStats ups ON rp.OwnerUserId = ups.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId
// LEFT JOIN PostHistoryStats phs ON rp.PostId = phs.PostId WHERE rp.PostId IN (SELECT DISTINCT ph.PostId FROM PostHistory ph WHERE ph.Comment IS NOT NULL) ORDER BY rp.ViewCount DESC, rp.CreationDate DESC;
//
// A CreationDate tie within an owner goes to the smaller post id.
fn q33920(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(owner_user));
    let r = per_group(ranked(v, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let rp = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect());
    let ridx: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let commented: MatSet<Id<Post>> = db.post_history.with(&db.post_history.comment).select(&db.post_history.post).collect();
    let ups = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + matches!(t, 2 | 4) as i64, a[1] + (t == 3) as i64]);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post.group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5]))).select(hd)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&ridx).map(|(_, k)| k).and(Ident::<Post>::new().with(&commented)).map(|(k, _)| k).and(owner_user.select(&ups).opt()).and((&cc).opt()).and((&phs).opt()));
    rows(v.into_iter().map(|(p, (((k, u), c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(u.unwrap_or([0; 2]).map(V::I));
        f.push(V::I(c.unwrap_or(0)));
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::I(0), V::Null],
        });
        f.extend(post_fields(db, p, &["rep", "owner"]));
        f.push(V::S(if k == 1 { "Latest Post" } else { "Earlier Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.Reputation, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostWithBadge AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.Reputation, b.Name AS BadgeName FROM RankedPosts rp LEFT JOIN Badges b ON rp.PostId = b.UserId
//     AND b.Date >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' WHERE rp.Rank = 1),
// PostScoreRanked AS (SELECT p.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS NetVotes,
//        ROW_NUMBER() OVER (ORDER BY COUNT(c.Id) DESC, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) DESC) AS CommentRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Title)
// SELECT pwb.PostId, pwb.Title, pwb.Score, pwb.Reputation, pwb.BadgeName, ps.CommentCount, ps.NetVotes, CASE WHEN pwb.Reputation IS NULL THEN 'No Badge' ELSE pwb.BadgeName END AS BadgeDescription
// FROM PostWithBadge pwb FULL OUTER JOIN PostScoreRanked ps ON pwb.Title = ps.Title WHERE (pwb.Reputation > 100 OR ps.CommentCount > 5) AND (pwb.BadgeName IS NOT NULL OR ps.CommentCount IS NOT NULL)
// ORDER BY COALESCE(pwb.Score, 0) DESC, COALESCE(ps.NetVotes, 0) DESC;
//
// The FULL OUTER JOIN is the LEFT JOIN of PostWithBadge onto PostScoreRanked plus the PostScoreRanked rows no PostWithBadge title matches (the NULL-title group never matches).
// `rp.PostId = b.UserId` joins a post id to a user id, so it goes through the raw ids. A CreationDate tie within an owner goes to the smaller post id. CommentRank is never read.
fn q4668(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, title, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let recent_badges = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.date).ge(add_years(t0, -1))));
    let pwb: MatSet<(Id<Post>, Option<Id<Badge>>)> = (&tp).select(Ident::<Post>::new().and((&db.post.origid).select(&uidx).select(recent_badges).opt())).collect();
    let ps = db
        .post
        .with(post_type_id.eq(1))
        .group_by(title.opt())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64 - (t == Some(3)) as i64]);
    type W = (Id<Post>, Option<Id<Badge>>);
    let rep = |p: Id<Post>| db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
    let left = drain(
        (&pwb)
            .select(Same::<W>::new().and(Same::<W>::new().map(|(p, _): W| p).select(title).map(Some).select(&ps).opt()))
            .filt(move |((p, b), s): (W, Option<[i64; 2]>)| (rep(p) > 100 || s.map_or(false, |s| s[0] > 5)) && (b.is_some() || s.is_some())),
    );
    let titles: MatSet<Option<Str>> = (&pwb).select(Same::<W>::new().map(|(p, _): W| p).select(title).map(Some)).map(|t| t).collect();
    let titles: MatSet<Option<Str>> = rel(drain(&titles).into_iter().map(|x| x.1).collect()).map(|t| t).collect();
    let right = drain((&ps).minus(&titles).filt(|s: [i64; 2]| s[0] > 5));
    let l: VecRel<usize, (Option<W>, Option<[i64; 2]>)> = rel(left.into_iter().map(|(_, (w, s))| (Some(w), s)).collect());
    let r: VecRel<usize, (Option<W>, Option<[i64; 2]>)> = rel(right.into_iter().map(|(_, s)| (None, Some(s))).collect());
    rows(drain((&l).union(&r)).into_iter().map(|(_, (w, s))| {
        let mut f = match w {
            Some((p, b)) => {
                let bn = b.map(|b| db.badge.name.get(b).unwrap());
                let mut g = post_fields(db, p, &["id", "title", "score"]);
                g.extend([V::I(rep(p)), ostr(bn)]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        };
        f.extend(match s {
            Some(s) => [V::I(s[0]), V::I(s[1])],
            None => [V::Null, V::Null],
        });
        f.push(match w {
            Some((_, b)) => ostr(b.map(|b| db.badge.name.get(b).unwrap())),
            None => V::S("No Badge"),
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, SUM(u.Reputation) AS TotalReputation, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// AggregatedVotes AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Posts p
//     LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// PostHistoryAnalysis AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT p.Title, p.CreationDate, CASE WHEN ur.TotalReputation > 1000 THEN 'High Reputation' WHEN ur.TotalReputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory,
//        av.Upvotes, av.Downvotes, ph.CloseCount, ph.DeleteCount
// FROM RankedPosts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN AggregatedVotes av ON p.PostId = av.PostId LEFT JOIN PostHistoryAnalysis ph ON p.PostId = ph.PostId
// WHERE p.PostRank = 1 AND ur.BadgeCount IS NOT NULL AND (av.Upvotes IS NULL OR av.Upvotes > av.Downvotes) ORDER BY p.CreationDate DESC LIMIT 100;
//
// TotalReputation sums Reputation over each user's badge rows. Every owner has a UserReputation row and every post an AggregatedVotes row, so neither IS NULL test can hold
// for a NULL. A CreationDate tie within an owner goes to the smaller post id.
fn q23230(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ur = db.user.group_by(Ident::<User>::new()).select((&db.user.reputation).and(badges_of(db).opt())).fold(0i64, |s, (r, _)| s + r);
    let av = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pha = db.post.group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 12) as i64]);
    let v = drain((&av).filt(|a| a[0] > a[1]).and(owner_user.select(&ur)).and((&pha).opt()));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 100);
    rows(v.into_iter().map(|(p, ((a, r), h))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" }));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [V::I(h[0]), V::I(h[1])],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank, COALESCE(v.VoteCount, 0) AS TotalVotes
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(VoteTypeId) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.PostTypeId, rp.CreationDate, rp.PostRank, rp.TotalVotes, CASE WHEN rp.PostTypeId = 1 AND rp.TotalVotes > 10 THEN 'Popular Question'
//        WHEN rp.PostTypeId = 2 AND rp.TotalVotes > 5 THEN 'Trending Answer' ELSE 'Regular Post' END AS PostCategory FROM RankedPosts rp WHERE rp.PostRank <= 10),
// UserDetails AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(b.BadgeCount, 0) AS BadgeCount FROM Users u LEFT JOIN (SELECT UserId, COUNT(Id) AS BadgeCount FROM Badges GROUP BY UserId) b ON u.Id = b.UserId)
// SELECT fp.Title, fp.TotalVotes, ud.DisplayName, ud.Reputation, ud.BadgeCount, fp.PostCategory FROM FilteredPosts fp LEFT JOIN Users u ON fp.PostTypeId = u.Id LEFT JOIN UserDetails ud ON u.Id = ud.UserId
// WHERE u.Reputation IS NOT NULL AND (ud.BadgeCount IS NULL OR ud.BadgeCount > 0) AND (fp.TotalVotes > 2 OR fp.PostCategory = 'Popular Question')
// ORDER BY fp.TotalVotes DESC, fp.CreationDate DESC OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY;
//
// `fp.PostTypeId = u.Id` joins a post type id to a user id, so it goes through the raw ids. A CreationDate tie within a type goes to the smaller post id.
fn q22899(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let cat = |p: Id<Post>, n: i64| match post_type_id.get(p).unwrap() {
        1 if n > 10 => "Popular Question",
        2 if n > 5 => "Trending Answer",
        _ => "Regular Post",
    };
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain(
        (&tv)
            .and(Ident::<Post>::new())
            .filt(move |(n, p): (i64, Id<Post>)| n > 2 || cat(p, n) == "Popular Question")
            .map(|(n, _)| n)
            .and(post_type_id.select(&uidx).select(Ident::<User>::new().and((&bc).filt(|b| b > 0)))),
    );
    let v = top_n(v, |&(p, (n, _))| (Reverse(n), Reverse(creation_date.get(p).unwrap()), p), 15);
    rows(v.into_iter().skip(5).map(|(p, (n, (u, b)))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(n), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::S(cat(p, n))]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT B.Id) AS BadgeCount, COUNT(DISTINCT P.Id) AS QuestionCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 GROUP BY U.Id, U.Reputation),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserActivity AS (SELECT UR.UserId, UR.Reputation, UR.TotalBounty, UR.BadgeCount, UR.QuestionCount, COALESCE(RP.PostCount, 0) AS RecentPostCount FROM UserReputation UR
//     LEFT JOIN (SELECT OwnerUserId, COUNT(PostId) AS PostCount FROM RecentPosts WHERE PostRank <= 10 GROUP BY OwnerUserId) RP ON UR.UserId = RP.OwnerUserId)
// SELECT UA.UserId, UA.Reputation, UA.TotalBounty, UA.BadgeCount, UA.QuestionCount, UA.RecentPostCount,
//        CASE WHEN UA.Reputation >= 1000 THEN 'High Reputation' WHEN UA.Reputation >= 100 THEN 'Moderate Reputation' ELSE 'Low Reputation' END AS ReputationCategory,
//        CASE WHEN UA.RecentPostCount = 0 THEN 'No Recent Posts' ELSE 'Active User' END AS UserActivityStatus
// FROM UserActivity UA LEFT JOIN (SELECT UserId, COUNT(DISTINCT PostId) AS ClosedPosts FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY UserId) ClosedP ON UA.UserId = ClosedP.UserId
// ORDER BY UA.Reputation DESC, UA.BadgeCount DESC LIMIT 100;
//
// The ORDER BY leads with Reputation, so only users at or above the hundredth reputation can be returned; the votes x badges x questions product is driven for those alone.
// ClosedP is never read and is one row per user, so it neither filters nor multiplies. A tie within an owner's recent posts does not change how many are ranked <= 10.
fn q22617(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, .. } = &db.post;
    let cut = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 100).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&db.user.reputation).ge(cut)).collect();
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let tb = (&cand)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()).and(qs().opt()))
        .fold(0i64, |s, ((b, _), _)| s + b.flatten().unwrap_or(0));
    let bc = (&cand).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let qc = (&cand).group_by(Ident::<User>::new()).select(qs().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let rp = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user));
    let rp = rel(top_per(rp, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false));
    let rc = (&rp).group_by(Same::<(Id<Post>, Id<User>)>::new().map(|(_, u): (Id<Post>, Id<User>)| u)).select(Same::<(Id<Post>, Id<User>)>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tb).and(&bc).and(&qc).and((&rc).opt()));
    let v = top_n(v, |&(u, ((_, b), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(b)), 100);
    rows(v.into_iter().map(|(u, (((t, b), q), r))| {
        let rep = db.user.reputation.get(u).unwrap();
        let r = r.unwrap_or(0);
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(t), V::I(b), V::I(q), V::I(r)]);
        f.push(V::S(if rep >= 1000 { "High Reputation" } else if rep >= 100 { "Moderate Reputation" } else { "Low Reputation" }));
        f.push(V::S(if r == 0 { "No Recent Posts" } else { "Active User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank,
//        COUNT(DISTINCT c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// ActiveUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, u.Views, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// PostMetrics AS (SELECT rp.Id AS PostId, rp.Title, CASE WHEN rp.Score >= 10 THEN 'Highly Scored' WHEN rp.Score BETWEEN 5 AND 9 THEN 'Moderately Scored' ELSE 'Low Scored' END AS ScoreCategory,
//        au.DisplayName AS UserName, au.Reputation, rp.CommentCount, rp.ViewCount FROM RankedPosts rp JOIN ActiveUsers au ON rp.OwnerUserId = au.Id)
// SELECT pm.PostId, pm.Title, pm.ScoreCategory, pm.UserName, pm.Reputation, pm.CommentCount, pm.ViewCount, COALESCE(h.UserId, 0) AS LastUserId,
//        COALESCE(h.CreationDate, cast('2024-10-01 12:34:56' as timestamp)) AS LastActivityDate, h.Comment AS LastAction
// FROM PostMetrics pm LEFT JOIN PostHistory h ON pm.PostId = h.PostId AND h.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = pm.PostId) ORDER BY pm.Reputation DESC, pm.ViewCount DESC;
//
// Rank is never read. Every history row at a post's latest instant joins.
fn q2398(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let pm = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))));
    let cc = pm.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let latest = history_of(db).select(Ident::<PostHistory>::new().and(hd.and(post.select(&md)).map(|(d, m)| d == m)).filt(|(_, t): (Id<PostHistory>, bool)| t).map(|(h, _)| h));
    let v = drain((&cc).and(latest.opt()));
    rows(v.into_iter().map(|(p, (c, h))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(if s >= 10 { "Highly Scored" } else if s >= 5 { "Moderately Scored" } else { "Low Scored" }));
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["views"]));
        f.extend(match h {
            Some(h) => [V::I(db.post_history.user_id.get(h).unwrap_or(0)), V::T(hd.get(h).unwrap()), ostr(db.post_history.comment.get(h))],
            None => [V::I(0), V::T(ts(2024, 10, 1, 12, 34, 56)), V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.Score IS NOT NULL AND p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount FROM RankedPosts rp WHERE rp.PostRank <= 5),
// CommentsWithScores AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments, SUM(CASE WHEN c.Score > 0 THEN 1 ELSE 0 END) AS PositiveComments FROM Comments c GROUP BY c.PostId),
// CombinedStats AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, COALESCE(cs.TotalComments, 0) AS TotalComments, COALESCE(cs.PositiveComments, 0) AS PositiveComments,
//        CASE WHEN tp.AnswerCount > 0 THEN (CAST(tp.Score AS FLOAT) / tp.AnswerCount) ELSE NULL END AS ScorePerAnswer FROM TopPosts tp LEFT JOIN CommentsWithScores cs ON tp.PostId = cs.PostId)
// SELECT cs.PostId, cs.Title, cs.CreationDate, cs.Score, cs.ViewCount, cs.AnswerCount, cs.TotalComments, cs.PositiveComments, cs.ScorePerAnswer,
//        CASE WHEN cs.Score IS NULL THEN 'No Score' ELSE CASE WHEN cs.Score >= 100 THEN 'High Score' WHEN cs.Score >= 50 THEN 'Medium Score' ELSE 'Low Score' END END AS ScoreCategory
// FROM CombinedStats cs ORDER BY cs.Score DESC, cs.CreationDate ASC;
//
// CAST AS FLOAT is a 4-byte REAL, so the division is done in f32. A (Score, CreationDate) tie within a type goes to the smaller post id.
fn q2430(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, answer_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| [a[0] + s.is_some() as i64, a[1] + (s.unwrap_or(0) > 0) as i64]);
    let v = drain(&cs);
    rows(v.into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(match answer_count.get(p) {
            Some(n) if n > 0 => V::F((s as f32 / n as f32) as f64),
            _ => V::Null,
        });
        f.push(V::S(if s >= 100 { "High Score" } else if s >= 50 { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= DATE_TRUNC('month', CAST('2024-10-01' AS DATE))),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostActivity AS (SELECT ph.PostId, ph.CreationDate, ph.PostHistoryTypeId, CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 'Closure Activity' WHEN ph.PostHistoryTypeId IN (6, 4) THEN 'Tag or Title Activity'
//        ELSE 'Other Activity' END AS ActivityType, COUNT(DISTINCT ph.UserId) AS UniqueUserCount FROM PostHistory ph GROUP BY ph.PostId, ph.CreationDate, ph.PostHistoryTypeId)
// SELECT rp.PostId, rp.Title, rp.Tags, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, pa.ActivityType, pa.UniqueUserCount,
//        CASE WHEN pa.UniqueUserCount > 5 THEN 'Highly Active' WHEN pa.UniqueUserCount BETWEEN 3 AND 5 THEN 'Moderately Active' ELSE 'Low Activity' END AS ActivityLevel
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostActivity pa ON rp.PostId = pa.PostId
// WHERE rp.UserPostRank <= 3 AND (pa.ActivityType IS NOT NULL OR ub.BadgeCount > 0) ORDER BY rp.Score DESC, rp.CreationDate ASC;
//
// A CreationDate tie within an owner (the ownerless posts form one partition) goes to the smaller post id.
fn q22619(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(trunc_month(date(2024, 10, 1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| {
        [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]
    });
    let PostHistory { post, creation_date: hd, post_history_type_id, user, .. } = &db.post_history;
    let pa = db.post_history.group_by(post.and(hd).and(post_history_type_id)).select(user).count_distinct();
    let pa0 = db.post_history.group_by(post.and(hd).and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pv = rel(drain((&pa0).and((&pa).opt())));
    let pidx: HashIdx<Id<Post>, (((Id<Post>, i64), i64), (i64, Option<i64>))> = (&pv).map(|(((p, _), _), _)| p).inv().select(&pv).collect();
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&ub).opt()).and((&pidx).opt())).filt(|((_, b), a): ((Id<Post>, Option<[i64; 4]>), Option<(((Id<Post>, i64), i64), (i64, Option<i64>))>)| a.is_some() || b.map_or(false, |b| b[0] > 0)));
    rows(v.into_iter().map(|(_, ((p, b), a))| {
        let mut f = post_fields(db, p, &["id", "title", "tags"]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match a {
            Some((((_, _), t), (_, n))) => {
                let n = n.unwrap_or(0);
                [
                    V::S(if matches!(t, 10 | 11) { "Closure Activity" } else if matches!(t, 6 | 4) { "Tag or Title Activity" } else { "Other Activity" }),
                    V::I(n),
                    V::S(if n > 5 { "Highly Active" } else if n >= 3 { "Moderately Active" } else { "Low Activity" }),
                ]
            }
            None => [V::Null, V::Null, V::S("Low Activity")],
        });
        row(f)
    }))
}

// WITH RECURSIVE TopUsers AS (SELECT Id, DisplayName, Reputation, CreationDate, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM Users WHERE Reputation > 1000),
// PostsWithBadges AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(b.Id) AS BadgeCount, AVG(u.Reputation) AS AvgReputation FROM Posts p LEFT JOIN Badges b ON b.UserId = p.OwnerUserId
//     JOIN Users u ON u.Id = p.OwnerUserId GROUP BY p.Id, p.Title, p.CreationDate),
// RecentActivePosts AS (SELECT p.PostTypeId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.LastActivityDate DESC) AS ActivityRank FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// ClosedPosts AS (SELECT ph.PostId, pt.Name AS PostHistoryTypeName, COUNT(*) AS CloseCount FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id WHERE pt.Name = 'Post Closed'
//     GROUP BY ph.PostId, pt.Name),
// FinalReport AS (SELECT pu.UserRank, pu.DisplayName, wp.Title, wp.BadgeCount, rp.Score AS RecentScore, rp.ViewCount AS RecentViewCount, cp.CloseCount FROM TopUsers pu
//     JOIN PostsWithBadges wp ON wp.PostId IN (SELECT Posts.Id FROM Posts WHERE Posts.OwnerUserId = pu.Id) LEFT JOIN RecentActivePosts rp ON rp.Title = wp.Title LEFT JOIN ClosedPosts cp ON cp.PostId = wp.PostId
//     WHERE pu.UserRank <= 10),
// FinalOutput AS (SELECT *, COALESCE(CASE WHEN CloseCount IS NOT NULL THEN 'Closed Post' ELSE 'Open Post' END, 'Open Post') AS PostStatus,
//        CONCAT('User: ', DisplayName, ' | Title: ', Title, ' | Score: ', RecentScore) AS ReportDetails FROM FinalReport)
// SELECT * FROM FinalOutput ORDER BY UserRank, BadgeCount DESC, RecentScore DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. ActivityRank and AvgReputation are never read. CONCAT treats a NULL as the empty string.
fn q33109(db: &'static So) -> String {
    let Post { creation_date, title, score, .. } = &db.post;
    let tu = ranked(drain((&db.user.reputation).gt(1000)), |&(_, r)| Reverse(r), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ra: HashIdx<Str, Id<Post>> = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(title).inv().collect();
    let cp = db.post_history.with(htype_name(db).filt(|n: Str| n == "Post Closed")).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tu).select(Same::<(Id<User>, i64)>::new().and(Same::<(Id<User>, i64)>::new().map(|(u, _)| u).select(Ident::<User>::new().and(&bc)).map(|(u, b)| (u, b)).and(Same::<(Id<User>, i64)>::new().map(|(u, _)| u).select(posts_of(db).select(Ident::<Post>::new().and(title.select(&ra).opt()).and((&cp).opt())))))));
    rows(v.into_iter().map(|(_, ((u, r), ((_, b), ((p, x), c))))| {
        let name = db.user.display_name.get(u).unwrap();
        let t = title.get(p);
        let s = x.map(|x| score.get(x).unwrap());
        let mut f = vec![V::I(r), V::S(name), ostr(t), V::I(b), oint(s), x.map_or(V::Null, |x| oint(db.post.view_count.get(x))), oint(c)];
        f.push(V::S(if c.is_some() { "Closed Post" } else { "Open Post" }));
        f.push(V::Owned(format!("User: {name} | Title: {} | Score: {}", t.unwrap_or(""), s.map_or(String::new(), |s| s.to_string()))));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, COUNT(*) OVER () AS TotalPosts
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount FROM Votes v WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId),
// ClosedPostCounts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.TotalPosts, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(rv.VoteCount, 0) AS RecentVoteCount, COALESCE(cpc.CloseCount, 0) AS ClosedCount,
//        CASE WHEN rp.Score IS NOT NULL AND rp.Score > 5 THEN 'High Score' WHEN rp.Score IS NOT NULL AND rp.Score BETWEEN 1 AND 5 THEN 'Medium Score' ELSE 'No Score' END AS ScoreCategory
// FROM RankedPosts rp LEFT JOIN Users u ON rp.PostId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN ClosedPostCounts cpc ON rp.PostId = cpc.PostId
// WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC;
//
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. A CreationDate tie within an owner (the ownerless questions form one partition) goes to the
// smaller post id. TotalPosts is a window over every question, computed once.
fn q21503(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let total = count(qs());
    let top = top_per(drain(qs().select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let rv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))))).fold(0i64, |n, _| n + 1);
    let cpc = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(Ident::<Post>::new().and((&db.post.origid).select(&uidx).select(&ub).opt()).and((&rv).opt()).and((&cpc).opt())));
    rows(v.into_iter().map(|(_, (((p, b), r), c))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(total));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend([V::I(r.unwrap_or(0)), V::I(c.unwrap_or(0)), V::S(if s > 5 { "High Score" } else if s >= 1 { "Medium Score" } else { "No Score" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.PostTypeId, p.Title, p.Score, p.ViewCount, p.LastActivityDate, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND p.Score IS NOT NULL),
// UserVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(*) AS EditCount, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseOpenCount FROM PostHistory ph GROUP BY ph.PostId),
// CombinedData AS (SELECT rp.*, uv.UpVotes, uv.DownVotes, phc.EditCount, phc.CloseOpenCount FROM RankedPosts rp LEFT JOIN UserVotes uv ON rp.PostId = uv.PostId LEFT JOIN PostHistoryCounts phc ON rp.PostId = phc.PostId)
// SELECT cd.PostId, cd.OwnerUserId, cd.Title, cd.Score, cd.ViewCount, cd.LastActivityDate, COALESCE(cd.UpVotes, 0) AS TotalUpVotes, COALESCE(cd.DownVotes, 0) AS TotalDownVotes, cd.EditCount, cd.CloseOpenCount,
//        CASE WHEN cd.PostRank <= 3 THEN 'Top Performer' WHEN cd.EditCount > 5 THEN 'Heavily Edited' ELSE 'Standard' END AS PostClassification
// FROM CombinedData cd WHERE cd.PostRank <= 10 GROUP BY cd.PostId, cd.OwnerUserId, cd.Title, cd.Score, cd.ViewCount, cd.LastActivityDate, cd.UpVotes, cd.DownVotes, cd.EditCount, cd.CloseOpenCount, cd.PostRank
// ORDER BY cd.Score DESC, cd.ViewCount DESC
//
// The outer GROUP BY is over one row per post, so it changes nothing.
fn q21056(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let r = per_group(ranked(top, |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |&(_, t)| t);
    let rp = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect());
    let ridx: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let uv = db.vote.with(&db.vote.post).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let phc = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + matches!(t, 10 | 11) as i64]);
    let v = drain((&ridx).map(|(_, k)| k).and((&uv).opt()).and((&phc).opt()));
    rows(v.into_iter().map(|(p, ((k, u), h))| {
        let mut f = post_fields(db, p, &["id", "owner_id", "title", "score", "views", "activity"]);
        f.extend(u.unwrap_or([0; 2]).map(V::I));
        f.extend(match h {
            Some(h) => [V::I(h[0]), V::I(h[1])],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if k <= 3 { "Top Performer" } else if h.map_or(false, |h| h[0] > 5) { "Heavily Edited" } else { "Standard" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// ActiveUsers AS (SELECT ur.UserId, ur.DisplayName, ur.Reputation, ps.TotalPosts, ps.QuestionCount, ps.AnswerCount, ps.TotalViews FROM UserReputation ur JOIN PostStats ps ON ur.UserId = ps.OwnerUserId WHERE ur.Reputation > 1000),
// RecentPostActivity AS (SELECT p.OwnerUserId, MAX(p.LastActivityDate) AS LastActivity FROM Posts p GROUP BY p.OwnerUserId),
// FinalMetrics AS (SELECT au.UserId, au.DisplayName, au.Reputation, au.TotalPosts, au.QuestionCount, au.AnswerCount, au.TotalViews, rpa.LastActivity,
//        CASE WHEN rpa.LastActivity IS NULL THEN 'Inactive' WHEN rpa.LastActivity < (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') THEN 'Dormant' ELSE 'Active' END AS ActivityStatus
//     FROM ActiveUsers au LEFT OUTER JOIN RecentPostActivity rpa ON au.UserId = rpa.OwnerUserId)
// SELECT f.UserId, f.DisplayName, f.Reputation, f.TotalPosts, f.QuestionCount, f.AnswerCount, f.TotalViews, f.ActivityStatus, COALESCE(NULLIF(f.QuestionCount, 0), 1) AS SafeQuestionCount,
//        (CAST(f.TotalViews AS FLOAT) / NULLIF(f.QuestionCount, 0)) AS AverageViewsPerQuestion
// FROM FinalMetrics f ORDER BY f.Reputation DESC, f.TotalPosts DESC LIMIT 100;
//
// CAST AS FLOAT is a 4-byte REAL, so the division is done in f32. ReputationRank is never read.
fn q168(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, last_activity_date, .. } = &db.post;
    let ps = db.post.with(owner_user).group_by(owner_user).select(post_type_id.and(view_count.opt()).and(last_activity_date)).fold([0, 0, 0, 0, 0, i64::MIN], |a, ((t, w), l)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5].max(l)]
    });
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select(&ps));
    let v = top_n(v, |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0])), 100);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3])]);
        f.push(V::S(if a[5] < add_years(ts(2024, 10, 1, 12, 34, 56), -1) { "Dormant" } else { "Active" }));
        f.push(V::I(if a[1] == 0 { 1 } else { a[1] }));
        f.push(if a[3] == 0 || a[1] == 0 { V::Null } else { V::F((a[4] as f32 / a[1] as f32) as f64) });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM Users),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// AcceptedAnswers AS (SELECT p.Id AS AnswerId, p.AcceptedAnswerId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 2 GROUP BY p.Id, p.AcceptedAnswerId),
// PostLinksSummary AS (SELECT pl.PostId, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostCount, SUM(CASE WHEN lt.Name = 'Duplicate' THEN 1 ELSE 0 END) AS DuplicateCount FROM PostLinks pl
//     JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id GROUP BY pl.PostId)
// SELECT u.DisplayName, u.Reputation, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount, aa.TotalUpvotes AS AcceptedAnswerUpvotes, pls.RelatedPostCount, pls.DuplicateCount
// FROM UserReputation u JOIN RecentPosts rp ON u.Id = rp.OwnerUserId LEFT JOIN AcceptedAnswers aa ON aa.AcceptedAnswerId = rp.PostId LEFT JOIN PostLinksSummary pls ON pls.PostId = rp.PostId
// WHERE u.Reputation > 1000 AND (rp.UpvoteCount - rp.DownvoteCount) > 10 ORDER BY u.Reputation DESC, rp.CreationDate DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// `aa.AcceptedAnswerId = rp.PostId` joins through the raw ids: the answers whose own AcceptedAnswerId names the post.
fn q4587(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, accepted_answer_id, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let aa: HashIdx<i64, Id<Post>> = db.post.with(post_type_id.eq(2)).select(accepted_answer_id).inv().collect();
    let up = db.post.with(post_type_id.eq(2)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64);
    let pl = db.post_link.group_by(&db.post_link.post).select((&db.post_link.link_type).select(&db.link_type.name)).fold([0i64; 1], |a, n| [a[0] + (n == "Duplicate") as i64]);
    let pr = db.post_link.group_by(&db.post_link.post).select(&db.post_link.related_post_id).count_distinct();
    let v = drain((&rp).filt(|a| a[1] - a[2] > 10).and((&db.post.origid).select(&aa).select(&up).opt()).and((&pl).and(&pr).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(db.user.reputation.get(owner_user.get(p).unwrap()).unwrap()), Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(p, ((a, x), l))| {
        let mut f = post_fields(db, p, &["owner", "rep", "title", "created"]);
        f.extend(a.map(V::I));
        f.push(oint(x));
        f.extend(match l {
            Some((d, r)) => [V::I(r), V::I(d[0])],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPostComments AS (SELECT c.PostId, c.UserId, c.CreationDate, ROW_NUMBER() OVER (PARTITION BY c.PostId ORDER BY c.CreationDate DESC) AS CommentRank,
//        RANK() OVER (PARTITION BY c.PostId ORDER BY c.Score DESC) AS CommentScoreRank FROM Comments c),
// PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT pc.UserId) AS UniqueCommenters, AVG(u.Reputation) AS AvgReputation
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) LEFT JOIN Comments pc ON p.Id = pc.PostId LEFT JOIN Users u ON pc.UserId = u.Id
//     WHERE p.CreationDate > '2020-01-01' AND p.Score IS NOT NULL GROUP BY p.Id, p.Title, p.ViewCount),
// BizarreJoin AS (SELECT pm.PostId, pm.Title, pm.ViewCount, pm.TotalBounty, pm.UniqueCommenters, pm.AvgReputation, CASE WHEN pm.UniqueCommenters = 0 THEN 'No comments yet!'
//        WHEN pm.UniqueCommenters = 1 THEN 'Just one bold commenter.' ELSE CONCAT(pm.UniqueCommenters, ' unique commenters.') END AS CommentStatus
//     FROM PostMetrics pm LEFT JOIN RankedPostComments rpc ON pm.PostId = rpc.PostId AND rpc.CommentRank = 1)
// SELECT b.PostId, b.Title, b.ViewCount, b.TotalBounty, b.UniqueCommenters, b.CommentStatus, COALESCE(b.AvgReputation, 0) AS AvgUserReputation,
//        CASE WHEN b.TotalBounty <= 0 THEN 'No bounties awarded.' WHEN b.TotalBounty < 50 THEN 'A small bounty.' ELSE 'A generous bounty!' END AS BountyStatus
// FROM BizarreJoin b WHERE b.CommentStatus IS NOT NULL ORDER BY b.ViewCount DESC, b.TotalBounty DESC;
//
// The CommentRank = 1 join matches at most one comment per post and nothing from it is read, so it neither filters nor multiplies.
fn q23713(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let base = || db.post.with(creation_date.gt(ts(2020, 1, 1, 0, 0, 0)));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let pm = base()
        .group_by(Ident::<Post>::new())
        .select(bounty.opt().and(comments_of(db).select((&db.comment.user).select(&db.user.reputation).opt()).opt()))
        .fold([0i64; 3], |a, (b, r)| {
            let r = r.flatten();
            [a[0] + b.flatten().unwrap_or(0), a[1] + r.is_some() as i64, a[2] + r.unwrap_or(0)]
        });
    let uc = base().group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.user_id)).count_distinct();
    let v = drain((&pm).and((&uc).opt()));
    rows(v.into_iter().map(|(p, (a, n))| {
        let n = n.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(a[0]), V::I(n)]);
        f.push(match n {
            0 => V::S("No comments yet!"),
            1 => V::S("Just one bold commenter."),
            _ => V::Owned(format!("{n} unique commenters.")),
        });
        f.push(if a[1] == 0 { V::F(0.0) } else { avg(a[2], a[1]) });
        f.push(V::S(if a[0] <= 0 { "No bounties awarded." } else if a[0] < 50 { "A small bounty." } else { "A generous bounty!" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.Reputation AS OwnerReputation, ROW_NUMBER() OVER(PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id),
// PostStatistics AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerReputation, COALESCE(AVG(V.BountyAmount) FILTER (WHERE V.VoteTypeId = 8), 0) AS AverageBounty,
//        COALESCE(SUM(CASE WHEN C.UserId IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM RankedPosts RP LEFT JOIN Votes V ON RP.PostId = V.PostId LEFT JOIN Comments C ON RP.PostId = C.PostId LEFT JOIN Badges B ON RP.OwnerReputation >= B.Class * 100
//     WHERE RP.Rank <= 10 GROUP BY RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerReputation),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, T.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes T ON CAST(PH.Comment AS INT) = T.Id WHERE PH.PostHistoryTypeId = 10),
// FinalStats AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.ViewCount, PS.OwnerReputation, PS.AverageBounty, PS.CommentCount, COALESCE(CP.CloseReason, 'Not Closed') AS CloseReason
//     FROM PostStatistics PS LEFT JOIN ClosedPosts CP ON PS.PostId = CP.PostId)
// SELECT F.PostId, F.Title, F.CreationDate, F.Score, F.ViewCount, F.OwnerReputation, F.AverageBounty, F.CommentCount, F.CloseReason FROM FinalStats F
// WHERE (F.Score > 100 OR F.ViewCount > 1000) AND F.OwnerReputation BETWEEN 100 AND 1000 ORDER BY F.Score DESC, F.ViewCount ASC;
//
// The final WHERE reads only PostStatistics' grouping columns, so it is applied to the ranked posts before the votes x comments x badges product; the badge join
// is a theta join on the owner's reputation, driven with select_where. A Score tie within a type
// goes to the smaller post id.
fn q20116(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rep = owner_user.select(&db.user.reputation);
    let keep = (&tp).with(score.and(view_count.opt()).filt(|(s, w): (i64, Option<i64>)| s > 100 || w.map_or(false, |w| w > 1000))).with(rep.between(100, 1000));
    let badges = owner_user.select(&db.user.reputation).select_where((&db.badge.class).inv(), |r: i64, c: i64| r >= c * 100);
    let ps = keep
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt().and(comments_of(db).select((&db.comment.user).opt()).opt()).and(badges))
        .fold([0i64; 3], |a, ((v, c), _)| {
            let b = v.and_then(|(t, b)| if t == 8 { b } else { None });
            [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + c.flatten().is_some() as i64]
        });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason));
    let mut v = drain((&ps).and(cp.opt()));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), view_count.get(p)));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep"]);
        f.extend([if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) }, V::I(a[2]), V::S(c.unwrap_or("Not Closed"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldCount, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 100 GROUP BY u.Id),
// HighScoredPosts AS (SELECT rp.Id AS PostId, rp.Title, rb.UserId, rb.BadgeCount, rb.GoldCount, rb.SilverCount, rb.BronzeCount, rp.Score FROM RankedPosts rp JOIN UserBadges rb ON rp.OwnerUserId = rb.UserId
//     WHERE rp.Score > 100 AND rb.BadgeCount IS NOT NULL),
// CommentedPosts AS (SELECT PostId, AVG(Score) AS AverageScore FROM Comments GROUP BY PostId)
// SELECT hsp.PostId, hsp.Title, hsp.BadgeCount, COALESCE(cp.AverageScore, 0) AS AverageCommentScore,
//        (CASE WHEN hsp.GoldCount > 0 THEN 'Gold' WHEN hsp.SilverCount > 0 THEN 'Silver' WHEN hsp.BronzeCount > 0 THEN 'Bronze' ELSE 'No Badge' END) AS HighestBadge
// FROM HighScoredPosts hsp LEFT JOIN CommentedPosts cp ON hsp.PostId = cp.PostId ORDER BY hsp.Score DESC, hsp.BadgeCount DESC LIMIT 10;
//
// PostRank and CommentCount are never read.
fn q21769(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let ub = db.user.with((&db.user.reputation).gt(100)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| {
        [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]
    });
    let cp = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(100))).select(owner_user.select(&ub).and((&cp).opt())));
    let v = top_n(v, |&(p, (b, _))| (Reverse(score.get(p).unwrap()), Reverse(b[0])), 10);
    rows(v.into_iter().map(|(p, (b, c))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::I(b[0]));
        f.push(match c {
            Some((n, s)) => avg(s, n),
            None => V::F(0.0),
        });
        f.push(V::S(if b[1] > 0 { "Gold" } else if b[2] > 0 { "Silver" } else if b[3] > 0 { "Bronze" } else { "No Badge" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerName,
//        CASE WHEN p.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostStatistics AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, rp.OwnerName, rp.PostStatus,
//        ROW_NUMBER() OVER (PARTITION BY rp.PostStatus ORDER BY rp.Score DESC) AS Rank FROM RecentPosts rp),
// TopPosts AS (SELECT * FROM PostStatistics WHERE Rank <= 5),
// PostVoteCounts AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN v.VoteTypeId = 5 THEN 1 ELSE 0 END) AS Favorites FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.AnswerCount, tp.CommentCount, tp.OwnerName, tp.PostStatus, COALESCE(v.UpVotes, 0) AS TotalUpVotes, COALESCE(v.DownVotes, 0) AS TotalDownVotes,
//        COALESCE(v.Favorites, 0) AS TotalFavorites, CASE WHEN v.UpVotes IS NOT NULL AND v.DownVotes IS NOT NULL THEN (CAST(v.UpVotes AS decimal) / NULLIF((v.UpVotes + v.DownVotes), 0)) * 100 ELSE 0 END AS UpVotePercentage
// FROM TopPosts tp LEFT JOIN PostVoteCounts v ON tp.Id = v.PostId ORDER BY tp.PostStatus, tp.Score DESC;
//
// A Score tie within a status goes to the smaller post id. Every post has a PostVoteCounts row whose sums are never NULL, so the percentage is NULL only when it has no up
// or down vote.
fn q2867(db: &'static So) -> String {
    let Post { creation_date, owner_user, closed_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(closed_date.opt().map(|c: Option<i64>| c.is_some())));
    let top = top_per(v, |&(_, c)| c, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(5)) as i64]
    });
    let v = drain(&pv);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score", "answers", "comments", "owner"]);
        f.push(V::S(if closed_date.get(p).is_some() { "Closed" } else { "Open" }));
        f.extend(a.map(V::I));
        f.push(if a[0] + a[1] == 0 { V::Null } else { V::F(a[0] as f64 / (a[0] + a[1]) as f64 * 100.0) });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp
//     WHERE rp.Rank <= 5 AND NOT EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 3) AND (rp.Score > 100 OR rp.ViewCount > 1000)),
// CommentsInfo AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, AVG(c.Score) AS AvgCommentScore FROM Comments c GROUP BY c.PostId)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.OwnerDisplayName, ci.CommentCount, COALESCE(ci.AvgCommentScore, 0) AS AvgCommentScore, pt.Name AS PostType,
//        COALESCE(bt.Name, 'No Badge') AS Badge, CASE WHEN fp.ViewCount > 5000 THEN 'Highly Viewed' WHEN fp.ViewCount BETWEEN 1000 AND 5000 THEN 'Moderately Viewed' ELSE 'Less Viewed' END AS ViewStatus
// FROM FilteredPosts fp LEFT JOIN Posts p ON fp.PostId = p.Id LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Badges bt ON bt.UserId = p.OwnerUserId AND bt.Class = 1
// LEFT JOIN CommentsInfo ci ON ci.PostId = fp.PostId WHERE fp.Score >= 50 OR (fp.Score < 50 AND fp.ViewCount > 2000) ORDER BY fp.CreationDate DESC LIMIT 100;
//
// A (Score, CreationDate) tie within a type goes to the smaller post id.
fn q22894(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let down = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(3)));
    let sw = score.and(view_count.opt());
    let fp = (&tp)
        .minus(down)
        .with(sw.filt(|(s, w): (i64, Option<i64>)| (s > 100 || w.map_or(false, |w| w > 1000)) && (s >= 50 || w.map_or(false, |w| w > 2000))));
    let ci = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let gold = owner_user.select(badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))));
    let v = drain(fp.select(Ident::<Post>::new().and((&ci).opt()).and(gold.opt())));
    let v = top_n(v, |&(_, ((p, _), b))| (Reverse(creation_date.get(p).unwrap()), p, b), 100);
    rows(v.into_iter().map(|(_, ((p, c), b))| {
        let w = view_count.get(p).unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(match c {
            Some((n, s)) => [V::I(n), avg(s, n)],
            None => [V::Null, V::F(0.0)],
        });
        f.extend(post_fields(db, p, &["type"]));
        f.push(V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        f.push(V::S(if w > 5000 { "Highly Viewed" } else if w >= 1000 { "Moderately Viewed" } else { "Less Viewed" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COALESCE(vs.UpVoteCount, 0) AS UpVoteCount, COALESCE(vs.DownVoteCount, 0) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank FROM Posts p
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN vt.Id = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN vt.Id = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
//     GROUP BY PostId) vs ON p.Id = vs.PostId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, (rp.UpVoteCount - rp.DownVoteCount) AS NetVotes, rp.UserPostRank FROM RankedPosts rp
//     WHERE rp.UserPostRank = 1 AND rp.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' AND rp.UpVoteCount > 0),
// PostDetails AS (SELECT fp.PostId, fp.Title, fp.NetVotes, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = fp.PostId) AS CommentCount,
//        (SELECT AVG(CASE WHEN bp.UserId IS NOT NULL THEN 1 ELSE 0 END) FROM Badges bp WHERE bp.UserId = fp.PostId) AS AvgBadgesPerUser FROM FilteredPosts fp)
// SELECT pd.PostId, pd.Title, pd.NetVotes, pd.CommentCount, pd.AvgBadgesPerUser, u.DisplayName AS OwnerDisplayName,
//        CASE WHEN pd.NetVotes IS NULL THEN 'No Votes' WHEN pd.NetVotes > 0 THEN 'Positive' WHEN pd.NetVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteStatus
// FROM PostDetails pd LEFT JOIN Users u ON pd.PostId = u.Id WHERE pd.AvgBadgesPerUser IS NOT NULL ORDER BY pd.NetVotes DESC, pd.CommentCount DESC LIMIT 10;
//
// `bp.UserId = fp.PostId` and `pd.PostId = u.Id` join a post id to a user id, so they go through the raw ids; the AVG of ones is 1.0 when that user has a badge and NULL
// otherwise. A CreationDate tie within an owner (the ownerless posts form one partition) goes to the smaller post id.
fn q24204(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&tp).with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let with_badges = (&db.post.origid).select(&uidx).select(Ident::<User>::new().with(badges_of(db)));
    let v = drain((&vs).filt(|a| a[0] > 0).and(with_badges).and((&cc).opt()));
    let v = top_n(v, |&(p, ((a, _), c))| (Reverse(a[0] - a[1]), Reverse(c.unwrap_or(0)), p), 10);
    rows(v.into_iter().map(|(p, ((a, u), c))| {
        let n = a[0] - a[1];
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(n), V::I(c.unwrap_or(0)), V::F(1.0), user_col(db, u, "name")]);
        f.push(V::S(if n > 0 { "Positive" } else if n < 0 { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, COUNT(DISTINCT B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation, U.DisplayName),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, COUNT(C.Id) AS CommentCount, ROW_NUMBER() OVER (ORDER BY P.Score DESC, P.ViewCount DESC) AS PopularityRank FROM Posts P
//     LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.Score, P.ViewCount),
// PostVoteDetails AS (SELECT P.Id AS PostId, COUNT(V.Id) FILTER(WHERE V.VoteTypeId = 2) AS Upvotes, COUNT(V.Id) FILTER(WHERE V.VoteTypeId = 3) AS Downvotes FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id),
// UserPostStats AS (SELECT U.UserId, COALESCE(SUM(PV.Upvotes), 0) AS TotalUpvotes, COALESCE(SUM(PV.Downvotes), 0) AS TotalDownvotes FROM UserReputation U LEFT JOIN PostVoteDetails PV ON U.UserId = PV.PostId
//     GROUP BY U.UserId)
// SELECT U.DisplayName, U.Reputation, U.BadgeCount, PP.PostId, PP.Title, PP.Score, PP.ViewCount, PP.CommentCount, COALESCE(V.Upvotes, 0) AS PostUpvotes, COALESCE(V.Downvotes, 0) AS PostDownvotes,
//        CASE WHEN UR.TotalUpvotes > UR.TotalDownvotes THEN 'Positive Contributor' WHEN UR.TotalDownvotes > UR.TotalUpvotes THEN 'Negative Contributor' ELSE 'Neutral Contributor' END AS UserContributionType
// FROM UserReputation U JOIN PopularPosts PP ON PP.PopularityRank <= 10 LEFT JOIN PostVoteDetails V ON PP.PostId = V.PostId LEFT JOIN UserPostStats UR ON U.UserId = UR.UserId
// WHERE U.Reputation > 1000 ORDER BY PP.Score DESC, PP.ViewCount DESC;
//
// The ON condition names only PP, so the users and the ten popular posts are crossed. `U.UserId = PV.PostId` joins a user id to a post id, so it goes through the raw ids.
// A (Score, ViewCount) tie at the tenth post goes to the smaller post id.
fn q23272(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let pp = top_n(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))), |&(p, _)| (key(p), p), 10);
    let pp: MatSet<Id<Post>> = rel(pp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&pp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ppr = rel(drain((&cc).and(&pv)));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let ur = rel(drain(db.user.with((&db.user.reputation).gt(1000)).select((&bc).and((&db.user.origid).select(&pidx).select(&pv).opt()))));
    let mut v = Vec::new();
    (&ur).cross(&ppr).drive(|_, ((u, (b, x)), (p, (c, a)))| v.push((u, b, x, p, c, a)));
    rows(v.into_iter().map(|(u, b, x, p, c, a)| {
        let x = x.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if x[0] > x[1] { "Positive Contributor" } else if x[1] > x[0] { "Negative Contributor" } else { "Neutral Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS Author, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND p.ViewCount > 0),
// TopPosts AS (SELECT rp.PostID, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Author FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostVoteCounts AS (SELECT p.Id AS PostID, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// DetailedPostInfo AS (SELECT tp.PostID, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.Author, p.UpVotes, p.DownVotes, p.TotalVotes FROM TopPosts tp JOIN PostVoteCounts p ON tp.PostID = p.PostID)
// SELECT dpi.PostID, dpi.Title, dpi.CreationDate, dpi.Score, COALESCE(dpi.ViewCount, 0) AS ViewCount, COALESCE(dpi.Author, 'Unknown Author') AS Author, COALESCE(dpi.UpVotes, 0) AS UpVotes,
//        COALESCE(dpi.DownVotes, 0) AS DownVotes, CASE WHEN dpi.Score IS NULL THEN 'No Score Available' WHEN dpi.Score > 0 THEN 'Positive Feedback' ELSE 'Negative Feedback' END AS Feedback
// FROM DetailedPostInfo dpi LEFT JOIN PostHistory ph ON dpi.PostID = ph.PostId AND ph.PostHistoryTypeId IN (10, 11) ORDER BY dpi.Score DESC, dpi.CreationDate ASC LIMIT 100;
//
// A Score tie within a type goes to the smaller post id.
fn q21729(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(view_count.gt(0)).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ph = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let v = drain((&pv).and(ph.opt()));
    let v = top_n(v, |&(p, (_, h))| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p, h), 100);
    rows(v.into_iter().map(|(p, (a, _))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if s > 0 { "Positive Feedback" } else { "Negative Feedback" })]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT ub.UserId, COUNT(CASE WHEN ub.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN ub.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN ub.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges ub GROUP BY ub.UserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerUserId, rp.Score, rp.CommentCount, u.DisplayName AS OwnerName, ROW_NUMBER() OVER (ORDER BY rp.Score DESC) AS PostRank
//     FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, tp.PostId, tp.Title AS PostTitle, tp.CreationDate AS PostCreationDate, tp.Score AS PostScore,
//        tp.CommentCount AS PostCommentCount
// FROM TopUsers tu LEFT JOIN TopPosts tp ON tu.UserId = tp.OwnerUserId WHERE tu.Rank <= 10 AND (tp.PostRank IS NULL OR tp.PostRank <= 5) ORDER BY tu.Reputation DESC, tp.Score DESC;
//
// A Reputation tie at the tenth user and a Score tie in PostRank go to the smaller id. UserPostRank is never read.
fn q1213(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let rp = ranked(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user)), |&(p, _)| (Reverse(score.get(p).unwrap()), p), false);
    let rk = rel(rp.into_iter().map(|((p, _), r)| (p, r)).collect());
    let ridx: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let tpu = posts_of(db).select(Ident::<Post>::new().and((&ridx).map(|(_, r)| r)).and((&cc).opt()));
    type T = ((Id<User>, Option<[i64; 3]>), Option<((Id<Post>, i64), Option<i64>)>);
    let v = drain((&tu).select(Ident::<User>::new().and((&ub).opt()).and(tpu.opt())).filt(|(_, t): T| t.map_or(true, |((_, r), _)| r <= 5)));
    rows(v.into_iter().map(|(_, ((u, b), t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend(match t {
            Some(((p, _), c)) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "score"]);
                g.push(V::I(c.unwrap_or(0)));
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserInteractions AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, MAX(p.CreationDate) AS LastPostDate, u.Reputation FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId, pt.Name AS PostType, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.LastActivityDate, p.AcceptedAnswerId, pt.Name),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.LastActivityDate, pd.AcceptedAnswerId, pd.PostType, pd.CommentCount, pd.Upvotes, pd.Downvotes,
//        ROW_NUMBER() OVER (PARTITION BY pd.PostType ORDER BY pd.Upvotes DESC) AS Ranking FROM PostDetails pd)
// SELECT ui.DisplayName, tp.Title, tp.PostType, tp.CommentCount, tp.Upvotes, tp.Downvotes, tp.LastActivityDate, (SELECT COUNT(*) FROM Users u WHERE u.Reputation > ui.Reputation) AS HigherReputationCount
// FROM UserInteractions ui JOIN TopPosts tp ON ui.PostCount > 5 AND tp.Ranking <= 10 WHERE tp.LastActivityDate > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days') ORDER BY ui.Reputation DESC, tp.Upvotes DESC;
//
// The ON condition names each side only, so the users with more than five posts are crossed with the top posts. An Upvotes tie in Ranking goes to the smaller post id.
// HigherReputationCount is the count of users with a strictly higher reputation, which is RANK() - 1 over all users.
fn q8009(db: &'static So) -> String {
    let Post { last_activity_date, post_type, .. } = &db.post;
    let pd = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let top = top_per(drain(&pd), |&(p, _)| post_type.get(p), |&(p, a)| (Reverse(a[1]), p), 10, false);
    let since = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let tp = rel(drain(rel(top).filt(move |(p, _): (Id<Post>, [i64; 3])| last_activity_date.get(p).unwrap() > since)).into_iter().map(|x| x.1).collect());
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let hr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let hr = rel(hr.into_iter().map(|((u, _), k)| (u, k - 1)).collect());
    let hidx: HashIdx<Id<User>, (Id<User>, i64)> = (&hr).map(|(u, _)| u).inv().select(&hr).collect();
    let ui = rel(drain((&pc).filt(|n| n > 5).and((&hidx).map(|(_, k)| k))));
    let mut v = Vec::new();
    (&ui).cross(&tp).drive(|_, ((u, (_, h)), (p, a))| v.push((u, h, p, a)));
    rows(v.into_iter().map(|(u, h, p, a)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "type"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::T(last_activity_date.get(p).unwrap()), V::I(h)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.CreationDate, P.ViewCount, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// PostVotes AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V GROUP BY V.PostId),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId),
// PostHistories AS (SELECT PH.PostId, COUNT(PH.Id) AS EditCount, MAX(PH.CreationDate) AS LastEditDate FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (4, 5, 6) GROUP BY PH.PostId)
// SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, RP.OwnerDisplayName, COALESCE(PV.UpVotes, 0) AS TotalUpVotes, COALESCE(PV.DownVotes, 0) AS TotalDownVotes, COALESCE(PC.CommentCount, 0) AS TotalComments,
//        COALESCE(PH.EditCount, 0) AS TotalEdits, CASE WHEN RP.Score IS NULL OR RP.Score < 0 THEN 'No Score' WHEN RP.Score >= 0 AND RP.Score < 10 THEN 'Low Score' WHEN RP.Score >= 10 AND RP.Score <= 50 THEN 'Moderate Score'
//        ELSE 'High Score' END AS ScoreCategory, RANK() OVER (ORDER BY RP.Score DESC, RP.ViewCount DESC) AS GlobalRank
// FROM RankedPosts RP LEFT JOIN PostVotes PV ON RP.PostId = PV.PostId LEFT JOIN PostComments PC ON RP.PostId = PC.PostId LEFT JOIN PostHistories PH ON RP.PostId = PH.PostId
// WHERE RP.PostRank = 1 ORDER BY RP.CreationDate DESC;
//
// A CreationDate tie within a type goes to the smaller post id.
fn q32966(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ec = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6]))).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = ranked(drain((&pv).and(&cc).and(&ec)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, false);
    rows(v.into_iter().map(|((p, ((a, c), e)), r)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(e)]);
        f.push(V::S(if s < 0 { "No Score" } else if s < 10 { "Low Score" } else if s <= 50 { "Moderate Score" } else { "High Score" }));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS TotalComments,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS PostRank FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') AND P.Score IS NOT NULL AND (P.ViewCount > 0 OR P.AcceptedAnswerId IS NOT NULL)
//     GROUP BY P.Id, P.Title, P.Score, P.ViewCount, U.DisplayName, P.PostTypeId, P.CreationDate),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, RP.OwnerDisplayName, RP.TotalComments FROM RankedPosts RP WHERE RP.PostRank <= 5),
// PostVoteCounts AS (SELECT V.PostId, SUM(CASE WHEN VT.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN VT.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVoteCount FROM Votes V
//     INNER JOIN VoteTypes VT ON V.VoteTypeId = VT.Id GROUP BY V.PostId)
// SELECT TP.Title, TP.Score, TP.ViewCount, TP.OwnerDisplayName, TP.TotalComments, COALESCE(PVC.UpVoteCount, 0) AS UpVotes, COALESCE(PVC.DownVoteCount, 0) AS DownVotes,
//        CASE WHEN TP.Score > 0 THEN 'Positive' WHEN TP.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreCategory, CASE WHEN TP.TotalComments > 10 THEN 'Highly Engaged' ELSE 'Low Engagement' END AS EngagementLevel
// FROM TopPosts TP LEFT JOIN PostVoteCounts PVC ON TP.PostId = PVC.PostId ORDER BY TP.Score DESC, TP.TotalComments DESC FETCH FIRST 10 ROWS ONLY;
//
// PostRank reads only base columns, so the newest posts of each type are picked first (a tie goes to the smaller post id) and their comments counted afterwards.
fn q21054(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, accepted_answer_id, score, .. } = &db.post;
    let base = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(view_count.opt().and(accepted_answer_id.opt()).filt(|(w, a): (Option<i64>, Option<i64>)| w.map_or(false, |w| w > 0) || a.is_some()));
    let top = top_per(drain(base.select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pvc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db)).opt()).fold([0i64; 2], |a, n| [a[0] + (n == Some("UpMod")) as i64, a[1] + (n == Some("DownMod")) as i64]);
    let v = top_n(drain((&cc).and(&pvc)), |&(p, (c, _))| (Reverse(score.get(p).unwrap()), Reverse(c)), 10);
    rows(v.into_iter().map(|(p, (c, a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }));
        f.push(V::S(if c > 10 { "Highly Engaged" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount, SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS UpVotes,
//        SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS DownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, UpVotes, DownVotes, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS Author, COALESCE(COUNT(C.Id), 0) AS CommentCount FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id
//     LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate, U.DisplayName),
// RankedPosts AS (SELECT PostId, Title, CreationDate, Author, CommentCount, RANK() OVER (ORDER BY CreationDate DESC) AS PostRank FROM RecentPosts)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.AnswerCount, TU.QuestionCount, TU.UpVotes, TU.DownVotes, RP.Title, RP.CreationDate, RP.CommentCount, RP.PostRank
// FROM TopUsers TU LEFT JOIN RankedPosts RP ON TU.DisplayName = RP.Author WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC, RP.CreationDate DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the posts x votes product is driven for those alone. CURRENT_TIMESTAMP is a TIMESTAMPTZ, so
// CreationDate is read as New York local time.
fn q911(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 4], |a, x| match x {
        Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
        None => a,
    });
    let np = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let recent = drain(db.post.with(creation_date.map(ny_to_utc).ge(add_days(now_utc(), -30))).select(comments_of(db).opt()));
    let rc = ranked(recent, |&(p, _)| Reverse(creation_date.get(p).unwrap()), false);
    let rc = rel(rc);
    type RC = ((Id<Post>, Option<Id<Comment>>), i64);
    let rpg = (&rc).group_by(Same::<RC>::new().map(|((p, _), r): RC| (p, r))).select(Same::<RC>::new()).fold(0i64, |n, ((_, c), _): RC| n + c.is_some() as i64);
    let rpv = rel(drain(&rpg));
    let by_author: HashIdx<Str, ((Id<Post>, i64), i64)> = (&rpv).flat_map(|((p, _), _): ((Id<Post>, i64), i64)| owner_user.get(p).map(|u| db.user.display_name.get(u).unwrap())).inv().select(&rpv).collect();
    let v = drain((&us).and(&np).and((&db.user.display_name).select(&by_author).opt()));
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(match r {
            Some(((p, k), c)) => {
                let mut g = post_fields(db, p, &["title", "created"]);
                g.extend([V::I(c), V::I(k)]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(*) FILTER (WHERE Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE Class = 2) AS SilverBadges, COUNT(*) FILTER (WHERE Class = 3) AS BronzeBadges FROM Badges GROUP BY UserId),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(p.Score) AS AverageScore, MAX(p.CreationDate) AS LastPostDate FROM Posts p GROUP BY p.OwnerUserId),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        ps.TotalPosts, ps.Questions, ps.Answers, ps.AverageScore, ps.LastPostDate, DENSE_RANK() OVER (PARTITION BY CASE WHEN COALESCE(ub.GoldBadges, 0) > 0 THEN 1 WHEN COALESCE(ub.SilverBadges, 0) > 0 THEN 2 ELSE 3 END
//        ORDER BY ps.TotalPosts DESC) AS RankWithinBadgeClass FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId)
// SELECT ue.*, CASE WHEN ue.Answers > 0 THEN (ue.Questions::decimal / ue.Answers) ELSE NULL END AS QuestionToAnswerRatio,
//        CASE WHEN ue.LastPostDate IS NOT NULL THEN EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - ue.LastPostDate)) / 86400 ELSE NULL END AS DaysSinceLastPost
// FROM UserEngagement ue WHERE (ue.TotalPosts > 10 OR ue.GoldBadges > 0) AND (ue.AverageScore IS NOT NULL AND ue.AverageScore > 5) ORDER BY ue.RankWithinBadgeClass, ue.TotalPosts DESC;
//
// The rank is taken over every user, before the WHERE; users with no posts rank last (DESC puts NULLs last).
fn q544(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, creation_date, .. } = &db.post;
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ps = db.post.with(owner_user).group_by(owner_user).select(post_type_id.and(score).and(creation_date)).fold([0, 0, 0, 0, i64::MIN], |a, ((t, s), d)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4].max(d)]
    });
    let v = drain(db.user.select((&ub).opt().and((&ps).opt())));
    let class = |b: Option<[i64; 3]>| {
        let b = b.unwrap_or([0; 3]);
        if b[0] > 0 { 1 } else if b[1] > 0 { 2 } else { 3 }
    };
    let v = per_group(ranked(v, |&(_, (b, p))| (class(b), p.is_none(), Reverse(p.map(|p| p[0]))), true), |&(_, (b, _))| class(b));
    type R = ((Id<User>, (Option<[i64; 3]>, Option<[i64; 5]>)), i64);
    let v = drain(rel(v).filt(|((_, (b, p)), _): R| p.map_or(false, |p| (p[0] > 10 || b.map_or(false, |b| b[0] > 0)) && p[3] * 1 > 5 * p[0])));
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    rows(v.into_iter().map(|(_, ((u, (b, p)), r))| {
        let p = p.unwrap();
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), avg(p[3], p[0]), V::T(p[4]), V::I(r)]);
        f.push(if p[2] > 0 { V::F(p[1] as f64 / p[2] as f64) } else { V::Null });
        f.push(V::F(secs(t0 - p[4]) / 86400.0));
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCount AS (SELECT UserId, COUNT(*) AS BadgeCount, MIN(Date) AS FirstBadgeDate FROM Badges GROUP BY UserId),
// PostAnalytics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(CASE WHEN v.CreationDate IS NOT NULL THEN 1 ELSE 0 END), 0) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// TopPosts AS (SELECT pa.PostId, pa.Title, pa.CreationDate, pa.Score, ub.BadgeCount, pa.CommentCount, pa.VoteCount FROM PostAnalytics pa JOIN UserBadgeCount ub ON pa.OwnerUserId = ub.UserId
//     WHERE ub.BadgeCount > 0 ORDER BY pa.Score DESC LIMIT 10),
// RecentPostHistory AS (SELECT ph.PostId, ph.CreationDate, p.Title, p.OwnerUserId, p.Score, PHT.Name AS HistoryType FROM PostHistory ph JOIN PostHistoryTypes PHT ON ph.PostHistoryTypeId = PHT.Id
//     JOIN Posts p ON ph.PostId = p.Id WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
// SELECT tp.Title AS TopPostTitle, tp.CreationDate AS TopPostDate, tp.Score AS TopPostScore, tp.CommentCount AS TopPostCommentCount, tp.VoteCount AS TopPostVoteCount, rph.Title AS RecentPostTitle,
//        rph.CreationDate AS RecentEditDate, rph.HistoryType AS RecentEditType
// FROM TopPosts tp LEFT JOIN RecentPostHistory rph ON tp.PostId = rph.PostId ORDER BY tp.Score DESC, rph.CreationDate DESC NULLS LAST;
//
// WITH RECURSIVE, but no CTE refers to itself. TopPosts' LIMIT reads only Score (top_n warns on a tie at the cut), so the ten posts are picked before the comment x vote product.
fn q31175(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let base = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user.select(Ident::<User>::new().with(badges_of(db)))));
    let tp = top_n(base, |&(p, _)| Reverse(score.get(p).unwrap()), 10);
    let tp: MatSet<Id<Post>> = rel(tp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pa = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let PostHistory { creation_date: hd, .. } = &db.post_history;
    let rph = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let v = drain((&pa).and(rph.opt()));
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [post_fields(db, p, &["title"]).remove(0), V::T(hd.get(h).unwrap()), V::S(htype_name(db).get(h).unwrap())],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount,
//        SUM(CASE WHEN P.LastActivityDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' THEN 1 ELSE 0 END) AS RecentPostCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes),
// BadgeStats AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges B GROUP BY B.UserId),
// CombinedStats AS (SELECT US.UserId, US.DisplayName, US.Reputation, US.Views, US.UpVotes, US.DownVotes, US.PostCount, US.QuestionCount, US.AnswerCount, US.WikiCount, US.RecentPostCount,
//        COALESCE(BS.GoldBadges, 0) AS GoldBadges, COALESCE(BS.SilverBadges, 0) AS SilverBadges, COALESCE(BS.BronzeBadges, 0) AS BronzeBadges FROM UserStats US LEFT JOIN BadgeStats BS ON US.UserId = BS.UserId),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Views, UpVotes, DownVotes, PostCount, QuestionCount, AnswerCount, WikiCount, RecentPostCount, GoldBadges, SilverBadges, BronzeBadges,
//        RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM CombinedStats)
// SELECT * FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q9502(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, .. } = &db.post;
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let tidx: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let since = add_months(ts(2024, 10, 1, 12, 34, 56), -1);
    let us = (&tidx).map(|(u, _)| u).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(last_activity_date)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, l)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + (l >= since) as i64],
        None => a,
    });
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&tidx).map(|(_, r)| r).and(&us).and((&bs).opt()));
    rows(v.into_iter().map(|(u, ((r, a), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews", "uup", "udown"]);
        f.extend(a.map(V::I));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("30664", q30664),
    ("187", q187),
    ("21057", q21057),
    ("4572", q4572),
    ("4775", q4775),
    ("24368", q24368),
    ("34068", q34068),
    ("2668", q2668),
    ("21300", q21300),
    ("22474", q22474),
    ("1372", q1372),
    ("471", q471),
    ("22767", q22767),
    ("20416", q20416),
    ("758", q758),
    ("3998", q3998),
    ("1127", q1127),
    ("4756", q4756),
    ("23840", q23840),
    ("23792", q23792),
    ("34047", q34047),
    ("623", q623),
    ("24216", q24216),
    ("3924", q3924),
    ("4003", q4003),
    ("1018", q1018),
    ("4861", q4861),
    ("4761", q4761),
    ("5360", q5360),
    ("23175", q23175),
    ("3627", q3627),
    ("4584", q4584),
    ("20310", q20310),
    ("34459", q34459),
    ("20117", q20117),
    ("1696", q1696),
    ("4109", q4109),
    ("2617", q2617),
    ("23898", q23898),
    ("30075", q30075),
    ("1409", q1409),
    ("1366", q1366),
    ("31949", q31949),
    ("2096", q2096),
    ("33881", q33881),
    ("2785", q2785),
    ("21161", q21161),
    ("34613", q34613),
    ("224", q224),
    ("5304", q5304),
    ("21497", q21497),
    ("3535", q3535),
    ("33265", q33265),
    ("26318", q26318),
    ("1633", q1633),
    ("30115", q30115),
    ("20692", q20692),
    ("2642", q2642),
    ("24303", q24303),
    ("361", q361),
    ("20462", q20462),
    ("2687", q2687),
    ("3510", q3510),
    ("2017", q2017),
    ("33920", q33920),
    ("4668", q4668),
    ("23230", q23230),
    ("22899", q22899),
    ("22617", q22617),
    ("2398", q2398),
    ("2430", q2430),
    ("22619", q22619),
    ("33109", q33109),
    ("21503", q21503),
    ("21056", q21056),
    ("168", q168),
    ("4587", q4587),
    ("23713", q23713),
    ("20116", q20116),
    ("21769", q21769),
    ("2867", q2867),
    ("22894", q22894),
    ("24204", q24204),
    ("23272", q23272),
    ("21729", q21729),
    ("1213", q1213),
    ("8009", q8009),
    ("32966", q32966),
    ("21054", q21054),
    ("911", q911),
    ("544", q544),
    ("31175", q31175),
    ("9502", q9502),
];
