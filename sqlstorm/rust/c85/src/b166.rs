use harness::prelude::*;
use std::cmp::Reverse;

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, P.OwnerUserId,
//        RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS RankByScore
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND P.PostTypeId = 1),
// TotalVotes AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId),
// AcceptedAnswers AS (SELECT A.AcceptedAnswerId, COUNT(A.Id) AS AcceptedCount FROM Posts A WHERE A.PostTypeId = 2 GROUP BY A.AcceptedAnswerId)
// SELECT RP.Title, RP.Score, RP.ViewCount, COALESCE(TV.UpVotes, 0) AS UpVotes, COALESCE(TV.DownVotes, 0) AS DownVotes, COALESCE(PC.CommentCount, 0) AS CommentCount,
//        COALESCE(AA.AcceptedCount, 0) AS AcceptedCount, RP.CreationDate, U.DisplayName AS OwnerDisplayName
// FROM RankedPosts RP LEFT JOIN TotalVotes TV ON RP.PostId = TV.PostId LEFT JOIN PostComments PC ON RP.PostId = PC.PostId
// LEFT JOIN AcceptedAnswers AA ON RP.PostId = AA.AcceptedAnswerId JOIN Users U ON RP.OwnerUserId = U.Id
// WHERE RP.RankByScore <= 5 ORDER BY RP.Score DESC, RP.ViewCount DESC;
fn q749(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, accepted_answer, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let acc: HashIdx<Id<Post>, Id<Post>> = db.post.with(post_type_id.eq(2)).select(accepted_answer).inv().collect();
    let ac = (&tp).group_by(Ident::<Post>::new()).select((&acc).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    rows(drain((&tv).and(&pc).and(&ac)).into_iter().map(|(p, ((t, c), a))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(t[0]), V::I(t[1]), V::I(c), V::I(a)]);
        f.extend(post_fields(db, p, &["created", "owner"]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, AVG(p.Score) AS AverageScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// BadgeCounts AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostHistoryRank AS (SELECT ph.UserId, COUNT(*) AS EditCount, RANK() OVER (PARTITION BY ph.UserId ORDER BY COUNT(*) DESC) AS UserEditRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.UserId)
// SELECT us.DisplayName, us.PostCount, us.PositivePosts, us.NegativePosts, COALESCE(bc.GoldBadges, 0) AS GoldBadges, COALESCE(bc.SilverBadges, 0) AS SilverBadges,
//        COALESCE(bc.BronzeBadges, 0) AS BronzeBadges, us.AverageScore, COALESCE(phr.EditCount, 0) AS EditCount, phr.UserEditRank
// FROM UserStats us LEFT JOIN BadgeCounts bc ON us.UserId = bc.UserId LEFT JOIN PostHistoryRank phr ON us.UserId = phr.UserId
// WHERE us.PostCount > 0 ORDER BY us.AverageScore DESC, us.PostCount DESC FETCH FIRST 10 ROWS ONLY;
//
// PostHistoryRank has one row per UserId partition, so UserEditRank is 1 wherever it joins.
fn q2350(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.score)).fold([0i64; 4], |a, s| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + s]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let ed = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&us).filt(|a| a[0] > 0).and((&bc).opt()).and((&ed).opt()));
    let v = top_n(v, |&(_, ((a, _), _))| (Reverse(fkey(a[3] as f64 / a[0] as f64)), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(u, ((a, b), e))| {
        let b = b.unwrap_or([0; 3]);
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b[0]), V::I(b[1]), V::I(b[2]), avg(a[3], a[0]), V::I(e.unwrap_or(0)), if e.is_some() { V::I(1) } else { V::Null }])
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId = 4 THEN 1 ELSE 0 END) AS TagWikiExcerpts,
//        SUM(CASE WHEN P.PostTypeId = 5 THEN 1 ELSE 0 END) AS TagWikis, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgesEarned
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId
//     WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// UserRanking AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TagWikiExcerpts, TagWikis, UpVotes, DownVotes, BadgesEarned,
//        RANK() OVER (ORDER BY TotalPosts DESC, UpVotes - DownVotes DESC, BadgesEarned DESC) AS Rank FROM UserActivity)
// SELECT Rank, DisplayName, TotalPosts, Questions, Answers, TagWikiExcerpts, TagWikis, UpVotes, DownVotes, BadgesEarned
// FROM UserRanking WHERE Rank <= 10 ORDER BY Rank;
fn q8558(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let ua = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let t = p.map(|x| x.0);
            let v = p.and_then(|x| x.1);
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(4)) as i64, a[3] + (t == Some(5)) as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64, a[6] + b.is_some() as i64]
        });
    let np = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&ua).and(&np)), |&(_, (a, n))| (Reverse(n), Reverse(a[4] - a[5]), Reverse(a[6])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), r)| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END, 0)) AS QuestionCount,
//        SUM(COALESCE(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END, 0)) AS AnswerCount, SUM(COALESCE(CASE WHEN P.PostTypeId IN (1, 2) THEN P.Score ELSE 0 END, 0)) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopQuestions AS (SELECT P.Id AS QuestionId, P.Title, P.CreationDate, P.ViewCount, RANK() OVER (ORDER BY P.Score DESC) AS ScoreRank FROM Posts P WHERE P.PostTypeId = 1)
// SELECT US.UserId, US.DisplayName, US.Reputation, US.PostCount, US.QuestionCount, US.AnswerCount, US.TotalScore, TQ.QuestionId, TQ.Title, TQ.CreationDate, TQ.ViewCount,
//        COALESCE(B.BadgeCount, 0) AS BulkBadgeCount
// FROM UserStats US LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON US.UserId = B.UserId
// LEFT JOIN TopQuestions TQ ON US.QuestionCount > 0 AND TQ.ScoreRank <= 10
// WHERE US.Reputation > 1000 AND US.PostCount > 5 AND NOT EXISTS (SELECT 1 FROM Votes V WHERE V.UserId = US.UserId AND V.VoteTypeId IN (3, 12))
// ORDER BY US.TotalScore DESC, US.DisplayName ASC;
//
// The TQ join condition names only US, so users with questions are crossed with the top questions and the others keep one NULL row.
fn q20794(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score))).fold([0i64; 4], |a, (t, s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 || t == 2 { s } else { 0 }]
    });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let down = votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([3, 12])));
    let base: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(1000)).minus(down).collect();
    let x = (&base).select((&us).filt(|a| a[0] > 5).and((&bc).opt()));
    let tq = ranked(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(_, s)| Reverse(s), false);
    let tq = rel(tq.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect());
    let mut out: Vec<(Id<User>, [i64; 4], i64, Option<Id<Post>>)> = Vec::new();
    (&x).filt(|(a, _)| a[1] > 0).cross(&tq).drive(|(u, _), ((a, b), p)| out.push((u, a, b.unwrap_or(0), Some(p))));
    (&x).filt(|(a, _)| a[1] == 0).drive(|u, (a, b)| out.push((u, a, b.unwrap_or(0), None)));
    rows(out.into_iter().map(|(u, a, b, p)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "created", "views"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::I(b));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS RankWithinType
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY p.Id, p.Title, p.Tags, p.Score, p.ViewCount, pt.Name),
// BestPosts AS (SELECT rp.PostId, rp.Title, rp.Tags, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes, pt.Name AS PostType
//     FROM RankedPosts rp JOIN PostTypes pt ON rp.RankWithinType = 1)
// SELECT bp.PostId, bp.Title, bp.PostType, bp.Tags, bp.Score, bp.ViewCount, bp.CommentCount, bp.UpVotes, bp.DownVotes,
//        CASE WHEN bp.UpVotes > bp.DownVotes THEN 'Positive Feedback' WHEN bp.UpVotes < bp.DownVotes THEN 'Negative Feedback' ELSE 'Neutral Feedback' END AS Feedback
// FROM BestPosts bp ORDER BY bp.Score DESC, bp.ViewCount DESC;
//
// RankWithinType reads only base columns, so the best post of each type is picked first. The BestPosts join names only rp, so it crosses with every post type.
// The ROW_NUMBER tie is broken by post id (the answer is empty on this data, so the tie is not tested).
fn q27976(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(current_date(), -6))).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&s).cross(&db.post_type.name)).into_iter().map(|((p, _), (a, n))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(n));
        f.extend(post_fields(db, p, &["tags", "score", "views"]));
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] > a[2] { "Positive Feedback" } else if a[1] < a[2] { "Negative Feedback" } else { "Neutral Feedback" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.UpVotes, u.DownVotes, (u.UpVotes - u.DownVotes) AS NetVotes, COUNT(DISTINCT p.Id) AS QuestionsAsked
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName, u.UpVotes, u.DownVotes),
// BadgeStats AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Badges b WHERE b.Date >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY b.UserId)
// SELECT ua.DisplayName, ua.NetVotes, ua.QuestionsAsked, COALESCE(bs.BadgeCount, 0) AS BadgeCount, COALESCE(bs.HighestBadgeClass, 0) AS HighestBadgeClass, rp.Title, rp.Score, rp.CreationDate
// FROM UserActivity ua LEFT JOIN BadgeStats bs ON ua.UserId = bs.UserId LEFT JOIN RankedPosts rp ON ua.UserId = rp.PostId
// WHERE ua.NetVotes >= 0 ORDER BY ua.NetVotes DESC, ua.QuestionsAsked DESC, rp.Score DESC;
//
// `ua.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q2537(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let User { up_votes, down_votes, origid, .. } = &db.user;
    let since = add_years(ts(2024, 10, 1, 0, 0, 0), -1);
    let qa = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bs = db.badge.with((&db.badge.date).ge(since)).group_by(&db.badge.user).select(&db.badge.class).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let rp: HashIdx<i64, Id<Post>> = db.post.with(post_type_id.eq(1).and(creation_date.ge(since))).select(&db.post.origid).inv().collect();
    let net = up_votes.and(down_votes).map(|(a, b)| a - b);
    let v = drain(db.user.with((&net).ge(0)).select((&net).and(&qa).and((&bs).opt()).and(origid.select(&rp).opt())));
    rows(v.into_iter().map(|(u, (((n, q), b), p))| {
        let b = b.unwrap_or((0, 0));
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(q), V::I(b.0), V::I(b.1)];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score", "created"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(b.Class) AS TotalBadges,
//        MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// ActiveUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, CommentCount, UpVotes, DownVotes, TotalBadges, LastPostDate,
//        DENSE_RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts, DENSE_RANK() OVER (ORDER BY UpVotes DESC) AS RankByUpVotes
//     FROM UserActivity WHERE LastPostDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR')
// SELECT a.UserId, a.DisplayName, a.Reputation, a.PostCount, a.CommentCount, a.UpVotes, a.DownVotes, a.TotalBadges, a.RankByPosts, a.RankByUpVotes
// FROM ActiveUsers a WHERE a.RankByPosts <= 10 OR a.RankByUpVotes <= 10 ORDER BY a.RankByPosts, a.RankByUpVotes;
//
// LastPostDate reads only the posts, so the active users are picked first and the product is driven for those alone.
fn q9139(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let last = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(creation_date)).fold(i64::MIN, |m, d| m.max(d));
    let active: MatSet<Id<User>> = db.user.with((&last).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).collect();
    let ua = (&active)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let v = p.and_then(|x| x.1);
            [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
        });
    let np = (&active).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = (&active).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let v = ranked(drain((&ua).and(&np).and((&nc).opt())), |&(_, ((_, n), _))| Reverse(n), true);
    let v = ranked(v, |&((_, ((a, _), _)), _)| Reverse(a[0]), true);
    let mut v: Vec<_> = drain(rel(v).filt(|((_, r), s): ((_, i64), i64)| r <= 10 || s <= 10)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&((_, r), s)| (r, s));
    rows(v.into_iter().map(|(((u, ((a, n), c)), r), s)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(r), V::I(s)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount, u.Reputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, BadgeCount, RANK() OVER (ORDER BY PostCount DESC, Reputation DESC) AS Rank FROM UserActivity)
// SELECT tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.UpVotes, tu.DownVotes, tu.BadgeCount, COUNT(c.Id) AS CommentCount, COUNT(ph.Id) AS HistoryCount
// FROM TopUsers tu LEFT JOIN Comments c ON c.UserId = tu.UserId LEFT JOIN PostHistory ph ON ph.UserId = tu.UserId
// WHERE tu.Rank <= 10 GROUP BY tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.UpVotes, tu.DownVotes, tu.BadgeCount, tu.Rank ORDER BY tu.Rank;
//
// Rank reads only the post count and reputation, so the top ten users are picked first and both products are driven for those alone.
fn q9723(db: &'static So) -> String {
    let base = || db.user.with((&db.user.reputation).gt(0));
    let np = base().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&np).and(&db.user.reputation)), |&(_, (n, r))| (Reverse(n), Reverse(r)), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let t = p.map(|x| x.0);
            let w = p.and_then(|x| x.1);
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (w == Some(2)) as i64, a[3] + (w == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let hist_by: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let ch = (&tu).group_by(Ident::<User>::new()).select(comments_by(db).opt().and((&hist_by).opt())).fold([0i64; 2], |a, (c, h)| [a[0] + c.is_some() as i64, a[1] + h.is_some() as i64]);
    let mut v = drain((&ua).and(&ch).and(&np).and(&db.user.reputation));
    v.sort_by_key(|&(_, (((_, _), n), r))| (Reverse(n), Reverse(r)));
    rows(v.into_iter().map(|(u, (((a, c), n), _))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        f.extend(c.map(V::I));
        row(f)
    }))
}

// WITH RecursiveCTE AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// LatestPosts AS (SELECT OwnerUserId, MAX(CreationDate) AS LastActiveDate FROM RecursiveCTE GROUP BY OwnerUserId),
// ActiveUserSummary AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(r.CommentCount, 0) AS TotalComments, COALESCE(r.Upvotes - r.Downvotes, 0) AS NetVotes, lp.LastActiveDate
//     FROM Users u LEFT JOIN RecursiveCTE r ON u.Id = r.OwnerUserId JOIN LatestPosts lp ON u.Id = lp.OwnerUserId)
// SELECT a.DisplayName, a.TotalComments, a.NetVotes, CASE WHEN a.LastActiveDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') THEN 'Active' ELSE 'Inactive' END AS UserStatus
// FROM ActiveUserSummary a WHERE a.NetVotes >= 0 ORDER BY a.TotalComments DESC FETCH FIRST 10 ROWS ONLY;
//
// Not recursive despite the name. The join to LatestPosts keeps only users who own a post, so every user row has a RecursiveCTE match.
fn q34959(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let r = db
        .post
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64 - (t == Some(3)) as i64]);
    let lp = db.post.group_by(owner_user).select(creation_date).fold(i64::MIN, |m, d| m.max(d));
    let v = drain((&r).filt(|a| a[1] >= 0).and(owner_user.select(Ident::<User>::new().and(&lp))));
    let v = top_n(v, |&(_, (a, _))| Reverse(a[0]), 10);
    let t = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    rows(v.into_iter().map(|(_, (a, (u, d)))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::S(if d > t { "Active" } else { "Inactive" })])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.ViewCount, rp.Score, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank <= 10),
// UsersWithBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.Id)
// SELECT u.DisplayName, tp.Title, tp.ViewCount, tp.Score, tp.CommentCount, COALESCE(ub.BadgeCount, 0) AS BadgeCount
// FROM Users u JOIN UsersWithBadges ub ON u.Id = ub.UserId JOIN Comments cm ON cm.UserId = u.Id JOIN TopPosts tp ON tp.Id = cm.PostId
// LEFT JOIN Posts p ON p.Id = tp.Id LEFT JOIN PostHistory ph ON ph.PostId = p.Id AND ph.PostHistoryTypeId = 10
// WHERE p.Score > 0 AND (ph.CreationDate IS NULL OR ph.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
// ORDER BY tp.Score DESC, BadgeCount DESC;
//
// The ROW_NUMBER numbers the post x comment rows, so a post takes one rank per comment; the rows of one post tie and are identical in what TopPosts keeps.
fn q491(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let j = drain(recent().select(ptype_name(db).and(comments_of(db).opt())));
    let top = top_per(j, |&(_, (t, _))| t, |&(p, (_, c))| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p, c), 10, false);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tp = rel(top.into_iter().map(|(p, _)| p).collect());
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let t = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let closed = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(hd).opt().filt(move |d: Option<i64>| d.map_or(true, |d| d > t));
    let chain = Ident::<Post>::new()
        .with(score.gt(0))
        .select(Ident::<Post>::new().and(&cc).and(comments_of(db).select(&db.comment.user).select(Ident::<User>::new().and(&bc))).and(closed));
    rows(drain((&tp).select(chain)).into_iter().map(|(_, (((p, c), (u, b)), _))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend([V::I(c), V::I(b)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostsWithBadges AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COUNT(B.Id) AS BadgeCount, ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY P.CreationDate DESC) AS BadgeRank
//     FROM Posts P LEFT JOIN Badges B ON P.OwnerUserId = B.UserId GROUP BY P.Id, P.Title, P.CreationDate),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, C.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes C ON CAST(PH.Comment AS INTEGER) = C.Id WHERE PH.PostHistoryTypeId = 10)
// SELECT U.DisplayName, U.TotalVotes, U.UpVotes, U.DownVotes, P.Title AS PostTitle, P.BadgeCount, C.CloseReason, COALESCE(C.CreationDate, P.CreationDate) AS CloseOrCreationDate
// FROM UserVoteStats U LEFT JOIN PostsWithBadges P ON U.UserId = P.PostId LEFT JOIN ClosedPosts C ON P.PostId = C.PostId
// WHERE (U.UpVotes > U.DownVotes OR U.DownVotes IS NULL) OR (P.BadgeCount > 0 AND P.BadgeRank = 1) ORDER BY COALESCE(C.CreationDate, P.CreationDate) DESC;
//
// `U.UserId = P.PostId` joins a user id to a post id, so it goes through the raw ids. BadgeRank is 1 for every post (one row per partition), and DownVotes is never NULL.
fn q2644(db: &'static So) -> String {
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let pwb = db.post.group_by(Ident::<Post>::new()).select((&db.post.owner_user).select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let closed = history_of(db)
        .select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)))
        .select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)));
    let q = (&uvs)
        .and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&pwb).and(closed.opt())).opt())
        .filt(|(a, p): ([i64; 3], Option<((Id<Post>, i64), Option<(i64, Str)>)>)| a[1] > a[2] || p.map_or(false, |((_, b), _)| b > 0));
    rows(drain(q).into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(match p {
            Some(((p, b), c)) => vec![
                harness::fmt::ostr(db.post.title.get(p)),
                V::I(b),
                c.map_or(V::Null, |(_, r)| V::S(r)),
                V::T(c.map_or(db.post.creation_date.get(p).unwrap(), |(d, _)| d)),
            ],
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerName,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVotes, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE v.CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY v.PostId),
// Combined AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerName, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes, rp.PostRank
//     FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId)
// SELECT c.*, (CASE WHEN c.UpVotes > c.DownVotes THEN 'Positive' WHEN c.UpVotes < c.DownVotes THEN 'Negative' ELSE 'Neutral' END) AS Sentiment,
//        CONCAT('Post Title: ', c.Title, ' | Owner: ', c.OwnerName) AS Summary
// FROM Combined c WHERE c.PostRank = 1 ORDER BY c.Score DESC, c.CreationDate DESC LIMIT 100 OFFSET 0;
fn q72(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let today = current_date();
    let v = drain(db.post.with(creation_date.ge(add_years(today, -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_months(today, -6)))).select(vtype_name(db));
    let rv = (&tp).group_by(Ident::<Post>::new()).select(recent.opt()).fold([0i64; 2], |a, n| [a[0] + (n == Some("UpMod")) as i64, a[1] + (n == Some("DownMod")) as i64]);
    let v = top_n(drain(&rv), |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(1)]);
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        let name = db.user.display_name.get(owner_user.get(p).unwrap()).unwrap();
        f.push(V::Owned(format!("Post Title: {} | Owner: {}", db.post.title.get(p).unwrap_or(""), name)));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserProfile AS (SELECT u.Id, u.DisplayName, u.Reputation, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(ps.TotalScore, 0) AS TotalScore,
//        COALESCE(ps.AvgViewCount, 0) AS AvgViewCount FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT up.DisplayName, up.Reputation, up.BadgeCount, up.PostCount, up.TotalScore, up.AvgViewCount, RANK() OVER (ORDER BY up.TotalScore DESC) AS ScoreRank
// FROM UserProfile up WHERE up.Reputation > 1000 ORDER BY up.TotalScore DESC, up.DisplayName ASC LIMIT 10 OFFSET 0;
fn q737(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&bc).and((&ps).opt())));
    let v = ranked(v, |&(_, (_, p))| Reverse(p.map_or(0, |a| a[1])), false);
    let v = top_n(v, |&((u, (_, p)), _)| (Reverse(p.map_or(0, |a| a[1])), db.user.display_name.get(u).unwrap()), 10);
    rows(v.into_iter().map(|((u, (b, p)), r)| {
        let a = p.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), if a[2] == 0 { V::F(0.0) } else { avg(a[3], a[2]) }, V::I(r)]);
        row(f)
    }))
}

// WITH PostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= DATE '2022-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.LastActivityDate),
// TopPosts AS (SELECT pa.PostId, pa.Title, pa.CreationDate, pa.LastActivityDate, pa.CommentCount, pa.VoteCount, pa.UpVoteCount, pa.DownVoteCount,
//        RANK() OVER (ORDER BY pa.VoteCount DESC, pa.CommentCount DESC) AS Rank FROM PostActivity pa)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.LastActivityDate, tp.CommentCount, tp.VoteCount, tp.UpVoteCount, tp.DownVoteCount, pt.Name AS PostTypeName, COUNT(b.Id) AS BadgeCount
// FROM TopPosts tp JOIN PostTypes pt ON tp.PostId = pt.Id LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId)
// WHERE tp.Rank <= 10 GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.LastActivityDate, tp.CommentCount, tp.VoteCount, tp.UpVoteCount, tp.DownVoteCount, pt.Name
// ORDER BY tp.VoteCount DESC, tp.CommentCount DESC;
//
// `tp.PostId = pt.Id` joins a post id to a post type id, so it goes through the raw ids.
fn q9404(db: &'static So) -> String {
    let recent = || db.post.with((&db.post.creation_date).ge(ts(2022, 1, 1, 0, 0, 0)));
    let pa = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let nv = recent().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = ranked(drain((&pa).and(&nv)), |&(_, (a, n))| (Reverse(n), Reverse(a[0])), false);
    let tp = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let ptype: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let Post { origid, owner_user, .. } = &db.post;
    let bc = db.post.group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type X = (Id<Post>, ([i64; 3], i64));
    let q = (&tp).select(Same::<X>::new().and(Same::<X>::new().map(|(p, _): X| p).select(origid.select(&ptype).select(&db.post_type.name).and(&bc))));
    rows(drain(q).into_iter().map(|(_, ((p, (a, n)), (t, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity"]);
        f.extend([V::I(a[0]), V::I(n), V::I(a[1]), V::I(a[2]), V::S(t), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.UserDisplayName, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS comment_rn
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12))
// SELECT rp.PostId, rp.Title, rp.CreationDate, COALESCE(pvd.UpVotes, 0) AS UpVotes, COALESCE(pvd.DownVotes, 0) AS DownVotes,
//        CASE WHEN hp.UserDisplayName IS NOT NULL THEN CONCAT('Last changed by ', hp.UserDisplayName, ' on ', hp.CreationDate) ELSE 'No recent history on this post' END AS RecentHistory
// FROM RankedPosts rp LEFT JOIN RecentVotes pvd ON rp.PostId = pvd.PostId LEFT JOIN PostHistoryDetails hp ON rp.PostId = hp.PostId AND hp.comment_rn = 1
// WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC FETCH FIRST 10 ROWS ONLY;
//
// rn reads only base columns, so the ten newest first questions are picked before the joins. A tie on the latest history date goes to the larger history id.
fn q2189(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first = top_n(first, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, user_display_name, .. } = &db.post_history;
    let hs = drain((&tp).select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11, 12])))));
    let hs = top_per(hs, |&(p, _)| p, |&(_, h)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let hs = rel(hs);
    let last: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&hs).map(|(p, _)| p).inv().select(&hs).collect();
    rows(drain((&rv).and((&last).map(|(_, h)| h).opt())).into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(match h.and_then(|h| user_display_name.get(h).map(|n| (n, hd.get(h).unwrap()))) {
            Some((n, d)) => V::Owned(format!("Last changed by {} on {}", n, ts_text(d))),
            None => V::S("No recent history on this post"),
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(MAX(P.CreationDate), '1970-01-01'::date) AS LastPostDate, COUNT(DISTINCT P.Id) AS TotalPosts,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentActivity AS (SELECT UA.UserId, UA.DisplayName, UA.Reputation, RANK() OVER (ORDER BY UA.Reputation DESC) AS ReputationRank, RANK() OVER (ORDER BY UA.LastPostDate DESC) AS ActivityRank
//     FROM UserActivity UA WHERE UA.TotalPosts > 0),
// TopUsers AS (SELECT * FROM RecentActivity WHERE ReputationRank <= 10 OR ActivityRank <= 10),
// BadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT TU.DisplayName, TU.Reputation, TU.ReputationRank, TU.ActivityRank, COALESCE(BC.BadgeCount, 0) AS BadgeCount,
//        CASE WHEN TU.ActivityRank <= 10 THEN 'Highly Active' WHEN TU.ReputationRank <= 10 THEN 'Highly Reputable' ELSE 'Regular User' END AS UserCategory
// FROM TopUsers TU LEFT JOIN BadgeCounts BC ON TU.UserId = BC.UserId ORDER BY TU.Reputation DESC, TU.ActivityRank ASC LIMIT 20;
fn q4487(db: &'static So) -> String {
    let last = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.creation_date)).fold(i64::MIN, |m, d| m.max(d));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain((&last).and(&db.user.reputation).and(&bc)), |&(_, ((_, r), _))| Reverse(r), false);
    let v = ranked(v, |&((_, ((d, _), _)), _)| Reverse(d), false);
    let kept = rel(v.into_iter().map(|(((u, ((_, r), b)), rr), ar)| (u, r, b, rr, ar)).collect());
    let v = drain((&kept).filt(|(_, _, _, rr, ar)| rr <= 10 || ar <= 10));
    let v = top_n(v, |&(_, (_, r, _, _, ar))| (Reverse(r), ar), 20);
    rows(v.into_iter().map(|(_, (u, r, b, rr, ar))| {
        row(vec![user_col(db, u, "name"), V::I(r), V::I(rr), V::I(ar), V::I(b), V::S(if ar <= 10 { "Highly Active" } else if rr <= 10 { "Highly Reputable" } else { "Regular User" })])
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(P.ViewCount) AS TotalViews,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName),
// BadgeSummary AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges FROM Badges GROUP BY UserId),
// TopUsers AS (SELECT UA.UserId, UA.DisplayName, UA.QuestionCount, UA.TotalViews, UA.Upvotes, UA.Downvotes, BS.GoldBadges, BS.SilverBadges, BS.BronzeBadges,
//        RANK() OVER (ORDER BY UA.QuestionCount DESC, UA.TotalViews DESC) AS Rank FROM UserActivity UA LEFT JOIN BadgeSummary BS ON UA.UserId = BS.UserId)
// SELECT UserId, DisplayName, QuestionCount, TotalViews, Upvotes, Downvotes, GoldBadges, SilverBadges, BronzeBadges, Rank FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q4683(db: &'static So) -> String {
    let asked = || posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(1)));
    let base = || db.user.with((&db.user.reputation).gt(0));
    let ua = base()
        .group_by(Ident::<User>::new())
        .select(asked().select((&db.post.view_count).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 4], |a, (w, t)| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let qc = base().group_by(Ident::<User>::new()).select(asked().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bs = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = ranked(drain((&qc).and((&ua).opt()).and((&bs).opt())), |&(_, ((n, a), _))| {
        let w = a.filter(|a| a[0] > 0).map(|a| a[1]);
        (Reverse(n), w.is_none(), Reverse(w))
    }, false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((n, a), b)), r)| {
        let a = a.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3])]);
        f.extend(match b {
            Some(b) => Vec::from(b.map(V::I)),
            None => vec![V::Null, V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// Rewritten (rewrites/8074.sql): the RankedPosts ROW_NUMBER is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC, p.Id) AS Rank
//     FROM Posts p WHERE p.PostTypeId IN (1, 2) AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.PostCount, us.QuestionCount, us.AnswerCount, us.TotalScore, ROW_NUMBER() OVER (ORDER BY us.TotalScore DESC) AS UserRank FROM UserStats us)
// SELECT tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.TotalScore, rp.Title, rp.Score, rp.CreationDate
// FROM TopUsers tu JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId WHERE tu.UserRank <= 10 AND rp.Rank = 1 ORDER BY tu.TotalScore DESC, rp.Score DESC;
fn q8074(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let us = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score))).fold([0i64; 4], |a, (t, s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]
    });
    let tu = top_n(drain(&us), |&(_, a)| Reverse(a[3]), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.select(&tu)));
    let rp = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(rp);
    let q = (&rp).select(Same::<(Id<Post>, Id<User>)>::new().and(Same::<(Id<Post>, Id<User>)>::new().map(|(_, u)| u).select(&us)));
    rows(drain(q).into_iter().map(|(_, ((p, u), a))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "score", "created"]));
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// UserBadges AS (SELECT UserId, COUNT(*) AS TotalBadges FROM Badges GROUP BY UserId),
// UserActivity AS (SELECT ue.UserId, ue.DisplayName, ue.TotalPosts, ue.Questions, ue.Answers, ue.TotalComments, ue.Upvotes, ue.Downvotes, COALESCE(ub.TotalBadges, 0) AS TotalBadges
//     FROM UserEngagement ue LEFT JOIN UserBadges ub ON ue.UserId = ub.UserId)
// SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.Questions, ua.Answers, ua.TotalComments, ua.Upvotes, ua.Downvotes, ua.TotalBadges,
//        RANK() OVER (ORDER BY ua.TotalPosts DESC, ua.Upvotes DESC) AS EngagementRank
// FROM UserActivity ua WHERE ua.TotalPosts > 0 ORDER BY EngagementRank FETCH FIRST 100 ROWS ONLY;
fn q9220(db: &'static So) -> String {
    let ue = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 6], |a, ((t, c), v)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let v = ranked(drain((&ue).and((&bc).opt())), |&(_, (a, _))| (Reverse(a[0]), Reverse(a[4])), false);
    let v = top_n(v, |x| x.1, 100);
    rows(v.into_iter().map(|((u, (a, b)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(b.unwrap_or(0)), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// ClosedPosts AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.Title, rp.Score, rp.ViewCount, ur.Reputation, ur.BadgeCount, pc.CommentCount, cp.LastClosedDate, CASE WHEN cp.LastClosedDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostComments pc ON rp.Id = pc.PostId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId
// WHERE rp.RankScore <= 5 ORDER BY rp.Score DESC, rp.Title;
fn q1870(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(hd)).fold(i64::MIN, |m, d| m.max(d));
    rows(drain((&tp).select(owner_user.select(Ident::<User>::new().and(&bc)).and((&pc).opt()).and((&cp).opt()))).into_iter().map(|(p, (((u, b), c), d))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([user_col(db, u, "rep"), V::I(b), harness::fmt::oint(c), harness::fmt::ots(d), V::S(if d.is_some() { "Closed" } else { "Active" })]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank, COUNT(DISTINCT p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, ReputationRank, TotalPosts, QuestionCount, AnswerCount FROM RankedUsers WHERE ReputationRank <= 10),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT u.DisplayName, u.Reputation, u.TotalPosts, u.QuestionCount, u.AnswerCount, COALESCE(b.BadgeCount, 0) AS TotalBadges, COALESCE(b.GoldBadges, 0) AS GoldBadges,
//        COALESCE(b.SilverBadges, 0) AS SilverBadges, COALESCE(b.BronzeBadges, 0) AS BronzeBadges
// FROM TopUsers u LEFT JOIN UserBadges b ON u.UserId = b.UserId ORDER BY u.Reputation DESC;
fn q8519(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let ub = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    rows(drain((&pc).and(&ub)).into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, DENSE_RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, P.Score, P.ViewCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostStatistics AS (SELECT U.UserId, U.DisplayName, COUNT(DISTINCT RP.PostId) AS TotalPosts, COALESCE(SUM(RP.Score), 0) AS TotalScore, COALESCE(SUM(RP.ViewCount), 0) AS TotalViews
//     FROM UserReputation U LEFT JOIN RecentPosts RP ON U.UserId = RP.OwnerUserId GROUP BY U.UserId, U.DisplayName),
// VoteCounts AS (SELECT P.OwnerUserId, COUNT(V.Id) AS TotalVotes FROM Posts P JOIN Votes V ON P.Id = V.PostId GROUP BY P.OwnerUserId)
// SELECT PS.UserId, PS.DisplayName, PS.TotalPosts, PS.TotalScore, PS.TotalViews, COALESCE(VC.TotalVotes, 0) AS TotalVotes, R.ReputationRank
// FROM PostStatistics PS LEFT JOIN VoteCounts VC ON PS.UserId = VC.OwnerUserId JOIN UserReputation R ON PS.UserId = R.UserId
// WHERE PS.TotalPosts > 10 AND PS.TotalScore > 100 ORDER BY PS.TotalScore DESC, R.ReputationRank ASC LIMIT 5;
fn q2638(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let ps = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)]);
    let vc = db.post.group_by(owner_user).select(votes_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&ps).filt(|a| a[0] > 10 && a[1] > 100).and((&vc).opt()).and((&rank).map(|(_, r)| r)));
    let v = top_n(v, |&(_, ((a, _), r))| (Reverse(a[1]), r), 5);
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n.unwrap_or(0)), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopContributors AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, us.AnswerCount, us.QuestionCount, us.TotalViews, RANK() OVER (ORDER BY us.Reputation DESC) AS ReputationRank
//     FROM UserStats us WHERE us.PostCount > 0),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS ScoreRank FROM Posts p WHERE p.PostTypeId = 1)
// SELECT tc.UserId, tc.DisplayName, tc.Reputation, tc.PostCount, tc.AnswerCount, tc.QuestionCount, tc.TotalViews, tp.PostId, tp.Title, tp.Score, tp.ViewCount AS PostViewCount,
//        tp.CreationDate AS PostCreationDate
// FROM TopContributors tc JOIN TopPosts tp ON tp.OwnerUserId = tc.UserId WHERE tc.ReputationRank <= 10 AND tp.ScoreRank <= 5 ORDER BY tc.Reputation DESC, tp.Score DESC;
fn q7727(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, view_count, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt()))).fold([0i64; 4], |a, (t, w)| {
        [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + w.unwrap_or(0)]
    });
    let v = ranked(drain((&us).and(&db.user.reputation)), |&(_, (_, r))| Reverse(r), false);
    let tc: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let tp = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), p), 5);
    let tp: MatSet<Id<Post>> = rel(tp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    rows(drain((&tp).select(owner_user.select(Ident::<User>::new().with(&tc).and(&us)))).into_iter().map(|(p, (u, a))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "created"]));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        COALESCE(AVG(LENGTH(CASE WHEN P.Body IS NOT NULL THEN P.Body END)), 0) AS AvgPostLength
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUserStats AS (SELECT UserId, DisplayName, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes, AvgPostLength, RANK() OVER (ORDER BY TotalAnswers DESC) AS AnswerRank,
//        RANK() OVER (ORDER BY TotalUpVotes DESC) AS UpVoteRank FROM UserStatistics)
// SELECT T.DisplayName, T.TotalQuestions, T.TotalAnswers, T.TotalUpVotes, T.TotalDownVotes, T.AvgPostLength, T.AnswerRank, T.UpVoteRank,
//        CASE WHEN T.AnswerRank <= 10 THEN 'Top Contributor' WHEN T.UpVoteRank <= 10 THEN 'Popular User' ELSE 'Regular User' END AS UserCategory
// FROM TopUserStats T WHERE T.TotalAnswers > 0 OR T.TotalUpVotes > 0 ORDER BY T.UpVoteRank, T.AnswerRank;
fn q29614(db: &'static So) -> String {
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&db.post.body).map(|b: Str| b.chars().count() as i64)).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, l), v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + 1, a[5] + l],
            None => a,
        });
    let v = ranked(drain(&us), |&(_, a)| Reverse(a[0]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[2]), false);
    let kept = rel(v.into_iter().map(|(((u, a), ar), vr)| (u, a, ar, vr)).collect());
    rows(drain((&kept).filt(|(_, a, _, _)| a[0] > 0 || a[2] > 0)).into_iter().map(|(_, (u, a, ar, vr))| {
        row(vec![
            user_col(db, u, "name"),
            V::I(a[1]),
            V::I(a[0]),
            V::I(a[2]),
            V::I(a[3]),
            if a[4] == 0 { V::F(0.0) } else { avg(a[5], a[4]) },
            V::I(ar),
            V::I(vr),
            V::S(if ar <= 10 { "Top Contributor" } else if vr <= 10 { "Popular User" } else { "Regular User" }),
        ])
    }))
}

// WITH RankedUsers AS (SELECT Id, DisplayName, Reputation, CreationDate, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM Users WHERE Reputation > 100),
// PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(AVG(v.BountyAmount), 0) AS AverageBounty, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
//        SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= '2022-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount),
// TopPosts AS (SELECT pm.*, RANK() OVER (ORDER BY pm.ViewCount DESC) AS PostRank FROM PostMetrics pm WHERE pm.CommentCount > 5)
// SELECT ru.DisplayName AS TopUser, tp.Title AS TopPost, tp.ViewCount, tp.AverageBounty, tp.CommentCount, tp.UpvoteCount, tp.DownvoteCount, tp.CloseCount
// FROM RankedUsers ru JOIN TopPosts tp ON ru.Id = tp.PostId WHERE tp.PostRank <= 10 ORDER BY ru.Reputation DESC, tp.ViewCount DESC;
//
// CommentCount is a distinct count and PostRank reads only ViewCount, so the top posts are picked first and the vote x comment x history product is driven for those alone.
// `ru.Id = tp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q32153(db: &'static So) -> String {
    let Post { creation_date, view_count, origid, .. } = &db.post;
    let recent = db.post.with(creation_date.ge(ts(2022, 1, 1, 0, 0, 0)));
    let cc = recent.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = ranked(drain((&cc).filt(|n| n > 5)), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let pm = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt().and(comments_of(db).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 5], |a, ((v, _), h)| {
            let b = v.and_then(|x| x.1);
            let t = v.map(|x| x.0);
            [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64, a[4] + (h == Some(10)) as i64]
        });
    let uidx: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(100)).select(&db.user.origid).inv().collect();
    let v = drain((&pm).and(&cc).and(origid.select(&uidx)));
    rows(v.into_iter().map(|(p, ((a, c), u))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) }, V::I(c), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= (CAST('2024-10-01' AS DATE) - INTERVAL '1 year') AND p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.OwnerUserId),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CommentCount FROM RankedPosts WHERE Rank <= 5),
// UserRankings AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(DISTINCT p.Id) AS TotalPosts
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT ur.UserId, ur.DisplayName, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges, ur.TotalPosts, tp.Title, tp.Score, tp.ViewCount, tp.CommentCount
// FROM UserRankings ur JOIN TopPosts tp ON ur.TotalPosts > 0 ORDER BY ur.TotalPosts DESC, tp.Score DESC LIMIT 10;
//
// The ON clause names only ur, so users with posts are crossed with TopPosts. The order leads with TotalPosts, so the first ten rows can only come
// from users ranked at most 10 by it; only those are crossed. Ties within an owner's ROW_NUMBER go to the lower post id.
fn q4415(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let v = ranked(drain(&np), |&(_, n)| Reverse(n), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ur = (&tu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt()))
        .fold([0i64; 3], |a, (c, _)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = drain((&ur).and(&np).cross(&cc));
    let v = top_n(v, |&((u, p), ((_, n), _))| (Reverse(n), Reverse(score.get(p).unwrap()), u, p), 10);
    rows(v.into_iter().map(|((u, p), ((a, n), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopEngagedUsers AS (SELECT ue.UserId, ue.DisplayName, ue.TotalPosts, ue.UpVotes, ue.DownVotes, RANK() OVER (ORDER BY ue.UpVotes - ue.DownVotes DESC) AS EngagementRank FROM UserEngagement ue)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, teu.DisplayName AS TopUser, teu.UpVotes AS UserUpVotes, teu.DownVotes AS UserDownVotes, teu.TotalPosts AS UserTotalPosts
// FROM RankedPosts rp JOIN TopEngagedUsers teu ON rp.Score > 0 WHERE rp.Rank <= 5 ORDER BY rp.CreationDate DESC, rp.Score DESC;
//
// The ON clause names only rp, so the top positive posts are crossed with every engaged user.
fn q5109(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1)))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ue = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let np = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&tp).with(score.gt(0)).cross((&ue).and(&np))).into_iter().map(|((p, u), (_, (a, n)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n)]);
        row(f)
    }))
}

// Rewritten (rewrites/5468.sql): the RankedPosts ROW_NUMBER is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS AuthorName, p.CreationDate, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) DESC, p.Id) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, AuthorName, CreationDate, CommentCount, UpVoteCount, DownVoteCount, Rank FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.AuthorName, tp.CreationDate, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount
// FROM TopPosts tp JOIN PostTypes pt ON pt.Id = (SELECT PostTypeId FROM Posts p WHERE p.Id = tp.PostId) WHERE pt.Name IN ('Question', 'Answer')
// ORDER BY tp.UpVoteCount DESC, tp.CommentCount DESC;
fn q5468(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&s).and(post_type_id));
    let top = top_per(v, |&(_, (_, t))| t, |&(p, (a, _))| (Reverse(a[1] - a[2]), p), 5, false);
    let tp = rel(top);
    type R = (Id<Post>, ([i64; 3], i64));
    let q = (&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(ptype_name(db).filt(|n: Str| n == "Question" || n == "Answer"))));
    rows(drain(q).into_iter().map(|(_, ((p, (a, _)), _))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount END), 0) AS AnsweredQuestions,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 5 THEN 1 END), 0) AS TagWikiCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId IN (3, 4) THEN 1 END), 0) AS WikiCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// BadgeCounts AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount FROM Badges B GROUP BY B.UserId),
// UserActivity AS (SELECT U.UserId, U.DisplayName, U.Reputation, U.PostCount, U.AnsweredQuestions, U.AnswerCount, U.TagWikiCount, U.WikiCount, COALESCE(BC.BadgeCount, 0) AS BadgeCount
//     FROM UserStats U LEFT JOIN BadgeCounts BC ON U.UserId = BC.UserId),
// RankedUsers AS (SELECT UA.*, RANK() OVER (ORDER BY UA.Reputation DESC, UA.PostCount DESC) AS UserRank FROM UserActivity UA)
// SELECT RU.UserId, RU.DisplayName, RU.Reputation, RU.PostCount, RU.AnsweredQuestions, RU.AnswerCount, RU.TagWikiCount, RU.WikiCount, RU.BadgeCount, RU.UserRank
// FROM RankedUsers RU WHERE RU.UserRank <= 10 ORDER BY RU.Reputation DESC, RU.PostCount DESC;
fn q6342(db: &'static So) -> String {
    let Post { post_type_id, answer_count, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(answer_count.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, n)) => [a[0] + 1, a[1] + if t == 1 { n.unwrap_or(0) } else { 0 }, a[2] + (t == 2) as i64, a[3] + (t == 5) as i64, a[4] + (t == 3 || t == 4) as i64],
        None => a,
    });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain((&us).and(&bc).and(&db.user.reputation)), |&(_, ((a, _), r))| (Reverse(r), Reverse(a[0])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, b), _)), k)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(b), V::I(k)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPosts, SUM(CASE WHEN p.FavoriteCount > 0 THEN 1 ELSE 0 END) AS FavoritedPosts FROM Posts p GROUP BY p.OwnerUserId),
// ActiveUsers AS (SELECT u.Id, u.DisplayName, ub.BadgeCount, ps.TotalPosts, ps.Questions, ps.Answers, ps.ClosedPosts, ps.FavoritedPosts
//     FROM Users u JOIN UserBadges ub ON u.Id = ub.UserId JOIN PostStatistics ps ON u.Id = ps.OwnerUserId WHERE u.Reputation > 1000),
// TopContributors AS (SELECT DisplayName, BadgeCount, TotalPosts, Questions, Answers, ClosedPosts, FavoritedPosts, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM ActiveUsers)
// SELECT DisplayName, BadgeCount, TotalPosts, Questions, Answers, ClosedPosts, FavoritedPosts FROM TopContributors WHERE PostRank <= 10 ORDER BY PostRank;
fn q6749(db: &'static So) -> String {
    let Post { post_type_id, closed_date, favorite_count, owner_user, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.group_by(owner_user).select(post_type_id.and(closed_date.opt()).and(favorite_count.opt())).fold([0i64; 5], |a, ((t, c), f)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + f.map_or(false, |f| f > 0) as i64]
    });
    let v = ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select((&bc).and(&ps))), |&(_, (_, a))| Reverse(a[0]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (b, a)), _)| {
        let mut f = vec![user_col(db, u, "name"), V::I(b)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserBadgeCount AS (SELECT UserId, COUNT(*) AS BadgeCount, MAX(Date) AS LastBadgeDate FROM Badges GROUP BY UserId),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(a.AnswerCount, 0) AS AnswerCount,
//        RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(SUM(p.Score), 0) AS TotalScore, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u LEFT JOIN UserBadgeCount ub ON u.Id = ub.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, ub.BadgeCount)
// SELECT tu.UserId, tu.DisplayName, tu.BadgeCount, tu.TotalScore, tu.PostCount, ps.PostId, ps.Title, ps.CreationDate, ps.RankScore
// FROM TopUsers tu JOIN PostStatistics ps ON tu.PostCount > 0 ORDER BY tu.TotalScore DESC, ps.RankScore ASC LIMIT 10;
//
// The ON clause names only tu, so users with posts are crossed with PostStatistics. The order leads with TotalScore, so the first ten rows can only
// come from users ranked at most 10 by it; only those are crossed. CommentCount and AnswerCount are never read, so they are not computed.
fn q3841(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let ps = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score.and(creation_date))), |&(_, (s, d))| (Reverse(s), Reverse(d)), false);
    let ps = rel(ps.into_iter().map(|((p, _), r)| (p, r)).collect());
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold([0i64; 2], |a, s| [a[0] + s, a[1] + 1]);
    let v = ranked(drain(&tu), |&(_, a)| Reverse(a[0]), false);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let v = drain((&top).select((&tu).and((&bc).opt())).cross(&ps));
    let v = top_n(v, |&(_, ((a, _), (_, r)))| (Reverse(a[0]), r), 10);
    rows(v.into_iter().map(|((u, _), ((a, b), (p, r)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount, p.OwnerUserId FROM Posts p WHERE p.CreationDate >= '2023-10-01 12:34:56'::timestamp - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostScores AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, ur.Reputation, ur.BadgeCount,
//        CASE WHEN rp.Score >= 100 THEN 'High' WHEN rp.Score BETWEEN 50 AND 99 THEN 'Medium' ELSE 'Low' END AS ScoreCategory
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.Rank = 1)
// SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.CommentCount, ps.Reputation AS UserReputation, ps.BadgeCount, ps.ScoreCategory
// FROM PostScores ps WHERE ps.Reputation > 1000 ORDER BY ps.Score DESC, ps.ViewCount ASC OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY;
fn q4141(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2023, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)).and(&bc))));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), w)
    }, 15);
    rows(v.into_iter().skip(5).map(|(p, (c, (u, b)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), user_col(db, u, "rep"), V::I(b), V::S(if s >= 100 { "High" } else if s >= 50 && s <= 99 { "Medium" } else { "Low" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, U.DisplayName AS OwnerDisplayName, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS RankByScore FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, Score, ViewCount, AnswerCount, CommentCount FROM RankedPosts WHERE RankByScore <= 10),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT PH.PostId) AS PostsEdited FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN PostHistory PH ON U.Id = PH.UserId GROUP BY U.Id, U.DisplayName)
// SELECT TP.PostId, TP.Title, TP.OwnerDisplayName, TP.CreationDate, TP.Score, TP.ViewCount, TP.AnswerCount, TP.CommentCount, US.DisplayName AS EditorDisplayName, US.UpVotes, US.DownVotes, US.PostsEdited
// FROM TopPosts TP LEFT JOIN UserStats US ON TP.OwnerDisplayName = US.DisplayName ORDER BY TP.Score DESC, TP.ViewCount DESC;
//
// UserStats is only read through the name join, so it is computed for the users whose name matches a top post's owner.
fn q9524(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let names: MatSet<Str> = (&tp).select(owner_user.select(&db.user.display_name)).collect();
    let cand: MatSet<Id<User>> = (&names).select(&by_name).collect();
    let hist_by: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and((&hist_by).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pe = (&cand).group_by(Ident::<User>::new()).select((&hist_by).select(&db.post_history.post)).count_distinct();
    let usv = rel(drain((&us).and((&pe).opt())));
    let us_name: HashIdx<Str, (Id<User>, ([i64; 2], Option<i64>))> = (&usv).map(|(u, _)| u).select(&db.user.display_name).inv().select(&usv).collect();
    rows(drain((&tp).select(owner_user.select(&db.user.display_name).select(&us_name).opt())).into_iter().map(|(p, u)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views", "answers", "comments"]);
        f.extend(match u {
            Some((u, (a, e))) => vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(e.unwrap_or(0))],
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 10),
// PostComments AS (SELECT c.PostId, COUNT(*) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// PostHistoryAggregates AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT rp.Id, rp.Title, rp.OwnerDisplayName, rp.Score, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(ph.EditCount, 0) AS EditCount, ph.LastEditDate, rp.CreationDate,
//        CASE WHEN rp.UserPostRank = 1 THEN 'Most Recent Post' ELSE 'Older Post' END AS PostStatus
// FROM RankedPosts rp LEFT JOIN PostComments pc ON rp.Id = pc.PostId LEFT JOIN PostHistoryAggregates ph ON rp.Id = ph.PostId
// WHERE rp.UserPostRank <= 5 OR (COALESCE(pc.CommentCount, 0) > 5 AND rp.Score > 20) ORDER BY rp.Score DESC, rp.CreationDate DESC;
fn q2815(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(10))).with(owner_user);
    let v = ranked(drain(base().select(owner_user)), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false);
    let v = per_group(v, |&(_, u)| u);
    let rk = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let pc = base().group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = base().group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6]))).select(hd)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    type R = (Id<Post>, i64);
    let q = (&rk)
        .select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&pc).opt().and((&ph).opt()))))
        .filt(|((p, r), (c, _)): (R, (Option<i64>, Option<(i64, i64)>))| r <= 5 || (c.unwrap_or(0) > 5 && score.get(p).unwrap() > 20));
    rows(drain(q).into_iter().map(|(_, ((p, r), (c, h)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(h.map_or(0, |x| x.0)), harness::fmt::ots(h.map(|x| x.1)), V::T(creation_date.get(p).unwrap())]);
        f.push(V::S(if r == 1 { "Most Recent Post" } else { "Older Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, p.OwnerUserId
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, u.DisplayName, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN r.Rank = 1 THEN 1 ELSE 0 END) AS TopPostsCount, SUM(r.ViewCount) AS TotalViews, SUM(r.Score) AS TotalScore,
//        SUM(r.CommentCount) AS TotalComments, SUM(r.VoteCount) AS TotalVotes FROM Users u LEFT JOIN RankedPosts r ON u.Id = r.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT us.UserId, us.DisplayName, us.TopPostsCount, us.TotalViews, us.TotalScore, us.TotalComments, us.TotalVotes, ROW_NUMBER() OVER (ORDER BY us.TotalViews DESC) AS UserRank
// FROM UserStats us WHERE us.TotalViews > 0 ORDER BY us.TotalViews DESC LIMIT 10;
//
// The two counts are COUNT(DISTINCT ..), so each is the post's own number of comments or votes.
fn q5725(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let first = top_per(drain(recent().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let nc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let nv = recent().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let us = recent()
        .group_by(owner_user)
        .select(Ident::<Post>::new().with(&first).opt().and(view_count.opt()).and(score).and(&nc).and(&nv))
        .fold([0i64; 6], |a, ((((f, w), s), c), v)| [a[0] + f.is_some() as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + c, a[5] + v]);
    let v = top_n(drain((&us).filt(|a| a[1] > 0 && a[2] > 0)), |&(_, a)| Reverse(a[2]), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[5]), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END) AS QuestionAnswers, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(P.ViewCount) AS AvgViewsPerPost
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Views, UpVotes, DownVotes, PostCount, CommentCount, QuestionAnswers, AnswerCount, AvgViewsPerPost,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS RN FROM UserStats)
// SELECT T.UserId, T.DisplayName, T.Reputation, T.Views, T.UpVotes, T.DownVotes, T.PostCount, T.CommentCount, T.QuestionAnswers, T.AnswerCount, T.AvgViewsPerPost, COALESCE(B.BadgeCount, 0) AS BadgeCount
// FROM TopUsers T LEFT JOIN (SELECT UserId, COUNT(Id) AS BadgeCount FROM Badges GROUP BY UserId) B ON T.UserId = B.UserId WHERE T.RN <= 10 ORDER BY T.Reputation DESC;
//
// RN reads only Reputation, so the top ten users are picked first and the posts x comments product is driven for those alone.
fn q7034(db: &'static So) -> String {
    let Post { post_type_id, answer_count, view_count, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(answer_count.opt()).and(view_count.opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((((t, n), w), _)) => [a[0] + if t == 1 { n.unwrap_or(0) } else { 0 }, a[1] + (t == 2) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)],
            None => a,
        });
    let np = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&us).and(&np).and(&nc).and(&bc)).into_iter().map(|(u, (((a, n), c), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews", "uup", "udown"]);
        f.extend([V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(b)]);
        row(f)
    }))
}

// WITH UserVotes AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, COALESCE(SUM(CASE WHEN C.UserId IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN PH.PostId IS NOT NULL THEN 1 ELSE 0 END), 0) AS EditCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score),
// RankedPosts AS (SELECT PS.*, RANK() OVER (ORDER BY PS.Score DESC, PS.ViewCount DESC) AS PostRank FROM PostStatistics PS)
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.ViewCount, RP.Score, RP.CommentCount, RP.EditCount, UV.DisplayName, UV.TotalVotes, UV.UpVotes, UV.DownVotes
// FROM RankedPosts RP JOIN UserVotes UV ON UV.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = RP.PostId) WHERE RP.PostRank <= 10 ORDER BY RP.PostRank;
//
// PostRank reads only base columns, so the top posts are picked first and the comments x history product is driven for those alone.
fn q1044(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let v = ranked(drain(score), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select((&db.comment.user).opt()).opt().and(history_of(db).opt()))
        .fold([0i64; 2], |a, (c, h)| [a[0] + c.flatten().is_some() as i64, a[1] + h.is_some() as i64]);
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    rows(drain((&ps).and(owner_user.select(Ident::<User>::new().and(&uv)))).into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), user_col(db, u, "name")]);
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(v.BountyAmount) AS TotalBounties, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation),
// ClosedPosts AS (SELECT p.Id AS PostId, COUNT(DISTINCT ph.Id) AS CloseEvents, MAX(ph.CreationDate) AS LastClosed FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (10, 11) GROUP BY p.Id)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.QuestionCount, us.AnswerCount, us.TotalBounties, cp.CloseEvents, cp.LastClosed,
//        CASE WHEN cp.CloseEvents IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus, CASE WHEN us.Rank <= 10 THEN 'Top Contributor' ELSE 'Contributor' END AS UserLevel
// FROM UserStats us LEFT JOIN ClosedPosts cp ON us.UserId = cp.PostId WHERE us.TotalPosts > 5 ORDER BY us.Rank, us.Reputation DESC LIMIT 100;
//
// Rank reads only Reputation and TotalPosts is a distinct count, so the hundred users are picked first and the posts x votes product is driven for those alone.
// A tie on Reputation inside the ROW_NUMBER goes to the lower user id. `us.UserId = cp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q164(db: &'static So) -> String {
    let base = || db.user.with((&db.user.reputation).gt(0));
    let np = base().group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let v = ranked(drain(base().select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), false);
    let rk = rel(v.into_iter().map(|((u, _), r)| (u, r)).collect());
    let kept = drain((&rk).select(Same::<(Id<User>, i64)>::new().and(Same::<(Id<User>, i64)>::new().map(|(u, _)| u).select((&np).filt(|n| n > 5)))));
    let kept = top_n(kept, |&(_, ((_, r), _))| r, 100);
    let tu = rel(kept.into_iter().map(|(_, ((u, r), n))| (u, (r, n))).collect());
    let top: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let us = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt()))).fold([0i64; 4], |a, (t, b)| {
        let b = b.flatten();
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
    });
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(&db.post_history.post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    type T = (Id<User>, (i64, i64));
    let q = (&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select((&us).and((&db.user.origid).select(&pidx).select(&cp).opt()))));
    rows(drain(q).into_iter().map(|(_, ((u, (r, n)), (a, c)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2])]);
        f.extend(match c {
            Some((k, d)) => [V::I(k), V::T(d), V::S("Closed")],
            None => [V::Null, V::Null, V::S("Active")],
        });
        f.push(V::S(if r <= 10 { "Top Contributor" } else { "Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, P.LastActivityDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS PostRank FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '90 days' AND P.ViewCount IS NOT NULL),
// CumulativeScores AS (SELECT PostId, Title, Score, OwnerDisplayName, SUM(Score) OVER (PARTITION BY OwnerDisplayName ORDER BY LastActivityDate) AS CumulativeScore,
//        ROW_NUMBER() OVER (PARTITION BY OwnerDisplayName ORDER BY LastActivityDate DESC) AS Ranking FROM RankedPosts WHERE PostRank <= 5)
// SELECT CS.OwnerDisplayName, CS.Title, CS.Score, CS.CumulativeScore, COALESCE(PHT.Name, 'No History') AS PostHistoryType,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = CS.PostId AND V.VoteTypeId = 2) AS Upvotes, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = CS.PostId AND V.VoteTypeId = 3) AS Downvotes
// FROM CumulativeScores CS LEFT JOIN PostHistory PH ON CS.PostId = PH.PostId LEFT JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// WHERE CS.Ranking <= 3 ORDER BY CS.OwnerDisplayName, CS.Score DESC;
//
// The running SUM is a prela window over each name's posts ordered by LastActivityDate, with equal dates summed together (RANGE peers).
// A tie on LastActivityDate inside the Ranking ROW_NUMBER goes to the higher post id.
fn q23233(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, last_activity_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -90))).with(view_count).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let running = |g: &[(i64, (Id<Post>, i64))], out: &mut Vec<i64>| {
        let mut i = 0;
        let mut acc = 0;
        while i < g.len() {
            let mut j = i;
            while j < g.len() && g[j].0 == g[i].0 {
                acc += g[j].1 .1;
                j += 1;
            }
            out.extend(std::iter::repeat(acc).take(j - i));
            i = j;
        }
    };
    let cum = (&tp)
        .group_by(owner_user.select(&db.user.display_name))
        .select(Ident::<Post>::new().and(last_activity_date).map(|(p, d)| (d, (p, score.get(p).unwrap()))))
        .window(|g: &[(i64, (i64, (Id<Post>, i64)))], out: &mut Vec<i64>| {
            let h: Vec<(i64, (Id<Post>, i64))> = g.iter().map(|&(k, (_, r))| (k, r)).collect();
            running(&h, out)
        }, |(d, _)| d, asc);
    let v = drain(&cum);
    let kept = top_per(v, |&(n, _)| n, |&(_, ((d, (p, _)), _))| (Reverse(d), Reverse(p)), 3, false);
    let kr = rel(kept.into_iter().map(|(n, ((_, (p, _)), c))| (p, (n, c))).collect());
    type K = (Id<Post>, (Str, i64));
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let q = (&kr).select(Same::<K>::new().and(Same::<K>::new().map(|(p, _): K| p).select(history_of(db).select(htype_name(db)).opt().and(&vc))));
    rows(drain(q).into_iter().map(|(_, ((p, (n, c)), (h, a)))| {
        let mut f = vec![V::S(n)];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(c), V::S(h.unwrap_or("No History")), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COALESCE(AVG(CASE WHEN c.Score IS NOT NULL THEN c.Score ELSE 0 END), 0) AS AverageCommentScore,
//        ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.Upvotes, rp.Downvotes, rp.AverageCommentScore FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.Upvotes, tp.Downvotes, tp.AverageCommentScore, u.DisplayName AS CreatorDisplayName, u.Reputation AS CreatorReputation, b.Count AS BadgeCount
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id LEFT JOIN (SELECT UserId, COUNT(*) AS Count FROM Badges GROUP BY UserId) b ON u.Id = b.UserId ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the top questions are picked first and the votes x comments product is driven for those alone.
// `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q7119(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, origid, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).select(&db.comment.score).opt()))
        .fold([0i64; 4], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + 1, a[3] + c.unwrap_or(0)]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    rows(drain((&rp).and(origid.select(&uidx).select(Ident::<User>::new().and((&bc).opt())))).into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[3], a[2])]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(harness::fmt::oint(b));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AvgUpVotes, AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AvgDownVotes
//     FROM RankedPosts rp LEFT JOIN Comments c ON c.PostId = rp.PostId LEFT JOIN Votes v ON v.PostId = rp.PostId GROUP BY rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.OwnerDisplayName)
// SELECT ps.PostId, ps.Title, ps.ViewCount, ps.Score, ps.OwnerDisplayName, ps.CommentCount, ps.VoteCount, ps.AvgUpVotes, ps.AvgDownVotes,
//        CASE WHEN ps.Score > 100 THEN 'High Score' WHEN ps.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM PostStatistics ps WHERE ps.CommentCount > 0 ORDER BY ps.ViewCount DESC, ps.Score DESC LIMIT 100;
fn q7350(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let ps = db
        .post
        .with(post_type_id.eq(1).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 5], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + t.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64, a[4] + 1]);
    let v = top_n(drain((&ps).filt(|a| a[0] > 0)), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(score.get(p).unwrap()))
    }, 100);
    rows(v.into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[4]), avg(a[3], a[4])]);
        f.push(V::S(if s > 100 { "High Score" } else if s >= 50 { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT t.TagName, COUNT(p.Id) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
//        SUM(COALESCE(vs.UpVotes, 0)) AS TotalUpVotes, SUM(COALESCE(vs.DownVotes, 0)) AS TotalDownVotes, SUM(p.ViewCount) AS TotalViews
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Comments c ON c.PostId = p.Id
//     LEFT JOIN (SELECT p.Id, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Votes v ON v.PostId = p.Id GROUP BY p.Id) vs ON vs.Id = p.Id
//     WHERE p.PostTypeId = 1 GROUP BY t.TagName),
// RankedTags AS (SELECT TagName, QuestionCount, AnswerCount, CommentCount, TotalUpVotes, TotalDownVotes, TotalViews, RANK() OVER (ORDER BY QuestionCount DESC, TotalViews DESC) AS TagRank
//     FROM TagStatistics WHERE QuestionCount > 0)
// SELECT rt.TagName, rt.QuestionCount, rt.AnswerCount, rt.CommentCount, rt.TotalUpVotes, rt.TotalDownVotes, rt.TotalViews FROM RankedTags rt WHERE rt.TagRank <= 10 ORDER BY rt.TagRank;
fn q26841(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|(_, t)| t).inv().map(|(p, _)| p).collect();
    let vs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ts = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(post_type_id.and(view_count.opt()).and(&vs).and(comments_of(db).opt())))
        .fold([0i64; 7], |a, (((t, w), v), c)| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + v[0], a[4] + v[1], a[5] + w.is_some() as i64, a[6] + w.unwrap_or(0)]);
    let v = ranked(drain(&ts), |&(_, a)| {
        let w = if a[5] == 0 { None } else { Some(a[6]) };
        (Reverse(a[0]), w.is_none(), Reverse(w))
    }, false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((t, a), _)| {
        row(vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), nullable(a[6], a[5])])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE u.Reputation > 1000 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostVoteSummary AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// PostCommenSummary AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT tp.PostId, tp.Title, tp.Score, tp.CreationDate, tp.ViewCount, COALESCE(pvs.VoteCount, 0) AS VoteCount, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes,
//        COALESCE(cs.CommentCount, 0) AS CommentCount
// FROM TopPosts tp LEFT JOIN PostVoteSummary pvs ON tp.PostId = pvs.PostId LEFT JOIN PostCommenSummary cs ON tp.PostId = cs.PostId ORDER BY tp.ViewCount DESC, tp.Score DESC;
fn q5227(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.select(rich)));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&pv).and(&cc)).into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "views"]);
        f.extend(a.map(V::I));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, RANK() OVER (ORDER BY COUNT(a.Id) DESC) AS AnswerRank
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON v.UserId = u.Id WHERE u.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.AnswerCount, rp.UpVoteCount, rp.DownVoteCount, ua.UserId, ua.DisplayName, ua.PostsCreated, ua.UpVotesReceived
// FROM RankedPosts rp JOIN UserActivity ua ON ua.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) WHERE rp.AnswerRank <= 10 ORDER BY rp.AnswerCount DESC;
//
// `p.PostTypeId = 1` in the answers' ON clause is already implied by the WHERE, so it is the plain children join.
fn q5549(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rp = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1))))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = ranked(drain(&rp), |&(_, a)| Reverse(a[0]), false);
    let tp = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let recent = || db.user.with((&db.user.creation_date).ge(add_years(t0, -1)));
    let ua = recent().group_by(Ident::<User>::new()).select(posts_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt())).fold(0i64, |n, (_, t)| n + (t == Some(2)) as i64);
    let np = recent().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    type R = (Id<Post>, [i64; 3]);
    let q = (&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(owner_user.select(Ident::<User>::new().and(&np).and(&ua)))));
    rows(drain(q).into_iter().map(|(_, ((p, a), ((u, n), up)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(n), V::I(up)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT B.Id) AS BadgeCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(P.ViewCount) AS TotalViews
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, UpVotes, DownVotes, QuestionCount, AnswerCount, TotalViews, RANK() OVER (ORDER BY Reputation DESC, TotalViews DESC, BadgeCount DESC) AS UserRank
//     FROM UserStats),
// TopUsers AS (SELECT * FROM RankedUsers WHERE UserRank <= 10)
// SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.BadgeCount, TU.UpVotes, TU.DownVotes, TU.QuestionCount, TU.AnswerCount, TU.TotalViews, PT.Name AS PostType
// FROM TopUsers TU JOIN Posts P ON TU.UserId = P.OwnerUserId JOIN PostTypes PT ON P.PostTypeId = PT.Id
// WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' ORDER BY TU.Reputation DESC, TU.TotalViews DESC;
//
// UserRank leads with Reputation, so every user it keeps has RANK() <= 10 by Reputation alone; those are picked first and the badges x posts x votes product is driven for them.
fn q7879(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let cand: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select((&db.post.post_type_id).and((&db.post.view_count).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 6], |a, (_, p)| match p {
            Some(((t, w), v)) => [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
            None => a,
        });
    let bc = (&cand).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let key = |u: Id<User>, a: [i64; 6], b: i64| {
        let w = if a[4] == 0 { None } else { Some(a[5]) };
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w), Reverse(b))
    };
    let v = ranked(drain((&us).and(&bc)), |&(u, (a, b))| key(u, a, b), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let Post { creation_date, .. } = &db.post;
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(ptype_name(db));
    type R = (Id<User>, ([i64; 6], i64));
    let q = (&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(recent)));
    rows(drain(q).into_iter().map(|(_, ((u, (a, b)), t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4]), V::S(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, U.DisplayName AS OwnerDisplayName, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// RecentlyEditedPosts AS (SELECT p.Id, p.Title, ph.CreationDate AS EditDate, ph.UserDisplayName AS EditorDisplayName, ph.Comment FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id
//     WHERE ph.PostHistoryTypeId IN (4, 5, 6) AND ph.CreationDate >= cast('2024-10-01' as date) - INTERVAL '6 months'),
// PostVoteCounts AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId)
// SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes, rec.EditDate, rec.EditorDisplayName, rec.Comment
// FROM RankedPosts rp LEFT JOIN PostVoteCounts pvc ON rp.Id = pvc.PostId LEFT JOIN RecentlyEditedPosts rec ON rp.Id = rec.Id WHERE rp.PostRank = 1 ORDER BY rp.CreationDate DESC LIMIT 100;
fn q30581(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let d0 = ts(2024, 10, 1, 0, 0, 0);
    let v = drain(db.post.with(creation_date.ge(add_years(d0, -1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let rec = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6]).and(hd.ge(add_months(d0, -6)))));
    let v = drain((&pv).and(rec.opt()));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 100);
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), harness::fmt::ostr(db.post_history.user_display_name.get(h)), harness::fmt::ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldCount, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeCount FROM Badges GROUP BY UserId),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RN
//     FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserScores AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, COALESCE(SUM(P.Score), 0) AS TotalScore
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 100 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT U.UserId, U.DisplayName, B.BadgeCount, U.TotalScore, U.TotalBounty, ROW_NUMBER() OVER (ORDER BY U.TotalScore DESC, U.TotalBounty DESC) AS Rank
//     FROM UserScores U JOIN UserBadges B ON U.UserId = B.UserId)
// SELECT T.UserId, T.DisplayName, T.BadgeCount, T.TotalScore, T.TotalBounty, (SELECT COUNT(*) FROM RecentPosts RP WHERE RP.OwnerUserId = T.UserId) AS RecentPostCount,
//        CASE WHEN T.BadgeCount > 5 THEN 'High Achiever' ELSE 'Needs Improvement' END AS PerformanceLabel
// FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.TotalScore DESC, T.TotalBounty DESC;
fn q669(db: &'static So) -> String {
    let us = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(posts_of(db).select(&db.post.score).opt()))
        .fold([0i64; 2], |a, (b, s)| [a[0] + b.flatten().unwrap_or(0), a[1] + s.unwrap_or(0)]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&us).and(&bc)), |&(u, (a, _))| (Reverse(a[1]), Reverse(a[0]), u), 10);
    let tu: MatSet<Id<User>> = rel(v.iter().map(|x| x.0).collect()).map(|u| u).collect();
    let rpc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&us).and(&bc).and(&rpc)).into_iter().map(|(u, ((a, b), n))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[0]), V::I(n), V::S(if b > 5 { "High Achiever" } else { "Needs Improvement" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.ViewCount, p.OwnerUserId),
// UserAggregates AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(vBounty.BountyAmount) AS TotalBountyWon, COUNT(DISTINCT p.Id) AS TotalQuestions,
//        COUNT(DISTINCT p.Id) FILTER (WHERE p.AcceptedAnswerId IS NOT NULL) AS TotalAcceptedAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes vBounty ON p.Id = vBounty.PostId AND vBounty.VoteTypeId IN (8, 9) WHERE u.Reputation > 5000 GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT ru.DisplayName AS UserDisplayName, ru.Reputation, COUNT(DISTINCT rp.PostId) AS TotalPosts, SUM(rp.ViewCount) AS TotalViews, SUM(ua.TotalBountyWon) AS TotalBounty,
//        MAX(ua.TotalQuestions) AS TotalQuestions, MAX(ua.TotalAcceptedAnswers) AS TotalAcceptedQuestions
// FROM RankedPosts rp JOIN UserAggregates ua ON rp.PostId = ua.UserId JOIN Users ru ON ua.UserId = ru.Id GROUP BY ru.Id, ru.DisplayName, ru.Reputation ORDER BY TotalViews DESC LIMIT 10;
//
// `rp.PostId = ua.UserId` joins a post id to a user id, so it goes through the raw ids. Nothing of RankedPosts but the post and its ViewCount is read.
fn q9722(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, accepted_answer, origid, .. } = &db.post;
    let rich = || db.user.with((&db.user.reputation).gt(5000));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ua = rich().group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt())).fold([0i64; 2], |a, b| {
        let b = b.flatten();
        [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0)]
    });
    let tq = rich().group_by(Ident::<User>::new()).select(posts_of(db).select(accepted_answer.opt()).opt()).fold([0i64; 2], |a, x| match x {
        Some(x) => [a[0] + 1, a[1] + x.is_some() as i64],
        None => a,
    });
    let uidx: HashIdx<i64, Id<User>> = rich().select(&db.user.origid).inv().collect();
    let g = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)))
        .group_by(origid.select(&uidx))
        .select(Ident::<Post>::new().and(view_count.opt()).and(origid.select(&uidx).select((&ua).opt())))
        .fold([0i64; 5], |a, ((_, w), b)| {
            [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + b.map_or(0, |b| (b[0] > 0) as i64), a[4] + b.map_or(0, |b| b[1])]
        });
    let v = top_n(drain((&g).and(&tq)), |&(_, (a, _))| {
        let w = if a[1] == 0 { None } else { Some(a[2]) };
        (w.is_none(), Reverse(w))
    }, 10);
    rows(v.into_iter().map(|(u, (a, t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1]), nullable(a[4], a[3]), V::I(t[0]), V::I(t[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges, ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CreationDate, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0)
// SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalComments, ua.TotalUpVotes, ua.TotalDownVotes, ua.TotalBadges, ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.AnswerCount, ps.CreationDate
// FROM UserActivity ua JOIN PostStatistics ps ON ua.TotalPosts > 5 WHERE ua.UserRank <= 10 AND ps.PostRank <= 20 ORDER BY ua.UserRank, ps.Score DESC;
//
// UserRank reads only the distinct post count, so the top ten users are picked first and the product is driven for those alone.
// `v.UserId = u.Id` keeps the votes each user cast on their own posts. The ON clause names only ua, so the users are crossed with the top questions.
fn q7787(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let tu = top_n(drain(&np), |&(u, n)| (Reverse(n), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ov = own_votes(db);
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and((&ov).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, b)| {
            let t = p.and_then(|x| x.1);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]
        });
    let nc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ps = top_n(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(score.and(creation_date))), |&(p, (s, d))| (Reverse(s), Reverse(d), p), 20);
    let ps = rel(ps.into_iter().map(|x| x.0).collect());
    let v = drain((&ua).and(&nc).and((&np).filt(|n| n > 5)).cross(&ps));
    rows(v.into_iter().map(|((u, _), (((a, c), n), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "answers", "created"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.Reputation AS OwnerReputation, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' AND p.PostTypeId = 1),
// TopRankedPosts AS (SELECT rp.Id, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.OwnerReputation FROM RankedPosts rp WHERE rp.PostRank <= 3),
// PostAnalytics AS (SELECT trp.Title, trp.Score, trp.ViewCount, trp.AnswerCount, COALESCE(badge_count.BadgeCount, 0) AS UserBadgeCount FROM TopRankedPosts trp
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) badge_count ON trp.OwnerReputation = badge_count.UserId)
// SELECT pa.Title, pa.Score, pa.ViewCount, pa.AnswerCount, pa.UserBadgeCount,
//        CASE WHEN pa.Score >= 10 THEN 'High Engagement' WHEN pa.Score BETWEEN 5 AND 9 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM PostAnalytics pa ORDER BY pa.Score DESC, pa.ViewCount DESC;
//
// `trp.OwnerReputation = badge_count.UserId` compares a reputation with a user id, so it goes through the raw ids. Ties inside an owner's ROW_NUMBER go to the lower post id.
fn q6088(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let tp = rel(top);
    let bc = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let q = (&tp).select(Same::<(Id<Post>, Id<User>)>::new().and(Same::<(Id<Post>, Id<User>)>::new().map(|(_, u)| u).select((&db.user.reputation).select(&bc).opt())));
    rows(drain(q).into_iter().map(|(_, ((p, _), b))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "score", "views", "answers"]);
        f.push(V::I(b.unwrap_or(0)));
        f.push(V::S(if s >= 10 { "High Engagement" } else if s >= 5 { "Moderate Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// Rewritten (rewrites/32784.sql): both ROW_NUMBERs are tie-broken on the id (`, p.Id` and `, ph.Id`).
// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.Id) AS OwnerPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// ErroneousPosts AS (SELECT ph.PostId, p.Title, ph.CreationDate, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC, ph.Id) AS HistoryRank
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId IN (10, 11, 12))
// SELECT rp.Id, rp.Title, rp.CreationDate, rp.OwnerDisplayName, COALESCE(pv.UpVotes, 0) AS UpVotes, COALESCE(pv.DownVotes, 0) AS DownVotes, rp.OwnerPostRank, ep.Comment AS LastChangeComment
// FROM RankedPosts rp LEFT JOIN PostVotes pv ON rp.Id = pv.PostId LEFT JOIN ErroneousPosts ep ON rp.Id = ep.PostId AND ep.HistoryRank = 1 JOIN Users u ON rp.OwnerDisplayName = u.DisplayName
// WHERE rp.OwnerPostRank = 1 ORDER BY rp.CreationDate DESC LIMIT 100;
fn q32784(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let hs = drain((&tp).select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11, 12])))));
    let hs = rel(top_per(hs, |&(p, _)| p, |&(_, h)| (Reverse(hd.get(h).unwrap()), h), 1, false));
    let last: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&hs).map(|(p, _)| p).inv().select(&hs).collect();
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let v = drain((&pv).and((&last).map(|(_, h)| h).opt()).and(owner_user.select(&db.user.display_name).select(&by_name)));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 100);
    rows(v.into_iter().map(|(p, ((a, h), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(1), h.map_or(V::Null, |h| harness::fmt::ostr(db.post_history.comment.get(h)))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.CreationDate, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS RankScore,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount FROM Posts P WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, PH.Comment, CRT.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes CRT ON PH.Comment::int = CRT.Id WHERE PH.PostHistoryTypeId IN (10, 11)),
// UserStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, AVG(P.ViewCount) AS AvgViews FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.Reputation)
// SELECT RP.PostId, RP.Title, RP.ViewCount, RP.RankScore, RP.CommentCount, COALESCE(CP.CloseReason, 'Not Closed') AS CloseReason, US.UserId, US.Reputation, US.PostCount, US.TotalScore, US.AvgViews
// FROM RankedPosts RP LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId JOIN UserStats US ON RP.PostId IN (SELECT P.Id FROM Posts P WHERE P.OwnerUserId = US.UserId)
// WHERE RP.RankScore <= 5 ORDER BY RP.RankScore, US.Reputation DESC LIMIT 100;
fn q4863(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1))).select(post_type_id));
    let v = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |&(_, t)| t);
    let rp = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rp = drain((&rp).filt(|(_, r)| r <= 5));
    let rp = rel(rp.into_iter().map(|x| x.1).collect());
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]))).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason));
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt()))).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    type R = (Id<Post>, i64);
    let q = (&rp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&cc).and(cp.opt()).and(owner_user.select(Ident::<User>::new().and(&us))))));
    let v = top_n(drain(q), |&(_, ((_, r), (_, (u, _))))| (r, Reverse(db.user.reputation.get(u).unwrap())), 100);
    rows(v.into_iter().map(|(_, ((p, r), ((c, cr), (u, a))))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(r), V::I(c), V::S(cr.unwrap_or("Not Closed"))]);
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[3], a[2])]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, CreationDate, LastAccessDate, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS DenseRank
//     FROM Users WHERE Reputation > 1000),
// ActivePosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.ViewCount, P.CreationDate, P.LastActivityDate, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     GROUP BY P.Id, P.OwnerUserId, P.ViewCount, P.CreationDate, P.LastActivityDate),
// TopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, UR.Rank FROM Users U JOIN UserReputation UR ON U.Id = UR.Id WHERE UR.Rank <= 10),
// PostStatistics AS (SELECT AP.OwnerUserId, COUNT(AP.PostId) AS TotalPosts, SUM(AP.ViewCount) AS TotalViews, SUM(AP.UpVotes) AS TotalUpVotes, SUM(AP.DownVotes) AS TotalDownVotes FROM ActivePosts AP GROUP BY AP.OwnerUserId)
// SELECT TU.DisplayName, TU.Reputation, PS.TotalPosts, PS.TotalViews, PS.TotalUpVotes, PS.TotalDownVotes FROM TopUsers TU LEFT JOIN PostStatistics PS ON TU.Id = PS.OwnerUserId ORDER BY TU.Reputation DESC, PS.TotalViews DESC;
fn q6271(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, .. } = &db.post;
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ap = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ps = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(Ident::<Post>::new().and(view_count.opt()).and(&ap)).fold([0i64; 5], |a, ((_, w), v)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + v[0], a[4] + v[1]]);
    rows(drain((&tu).select((&ps).opt())).into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match a {
            Some(a) => vec![V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::I(a[4])],
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties, SUM(COALESCE(b.Class, 0)) AS TotalBadgeClass
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostHistoryCounts AS (SELECT PostId, COUNT(*) AS HistoryCount FROM PostHistory WHERE CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY PostId)
// SELECT us.UserId, us.DisplayName, us.PostCount, us.TotalBounties, us.TotalBadgeClass, rp.Title, rp.Score, rp.RecentPostRank, COALESCE(phc.HistoryCount, 0) AS RecentHistoryCount
// FROM UserStatistics us LEFT JOIN RankedPosts rp ON us.UserId = rp.PostId LEFT JOIN PostHistoryCounts phc ON rp.PostId = phc.PostId WHERE us.PostCount > 5 ORDER BY us.TotalBounties DESC, us.PostCount DESC;
//
// PostCount is a distinct count, so the users with more than five posts are picked first and the posts x votes x badges product is driven for those alone.
// `us.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q772(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let many: MatSet<Id<User>> = db.user.with((&np).filt(|n| n > 5)).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let us = (&many)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(bounty.opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 2], |a, (b, c)| [a[0] + b.flatten().flatten().unwrap_or(0), a[1] + c.unwrap_or(0)]);
    let v = drain(db.post.with(creation_date.gt(add_years(t0, -1))).select(owner_user.opt()));
    let v = per_group(ranked(v, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let rk = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let hc = db.post_history.with((&db.post_history.creation_date).gt(add_days(t0, -30))).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let rp = (&db.user.origid).select(&pidx).select((&by_post).and((&hc).opt()));
    rows(drain((&us).and(&np).and(rp.opt())).into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(match r {
            Some(((p, k), h)) => {
                let mut g = post_fields(db, p, &["title", "score"]);
                g.extend([V::I(k), V::I(h.unwrap_or(0))]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::I(0)],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RankRecent
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PopularTags AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName HAVING COUNT(DISTINCT p.Id) >= 5),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(*) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, rp.RankScore, pt.TagName, phc.EditCount
// FROM RankedPosts rp JOIN PostHistoryCounts phc ON rp.PostId = phc.PostId JOIN PopularTags pt ON rp.RankScore <= 10 WHERE rp.RankRecent <= 10 ORDER BY rp.RankScore, rp.ViewCount DESC;
//
// The PopularTags ON clause names only rp, so the ranked posts are crossed with every popular tag. A tie inside the RankRecent ROW_NUMBER goes to the lower post id.
fn q7101(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let sk = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let v = per_group(ranked(v, |&(p, t)| (t, sk(p)), false), |&(_, t)| t);
    let v = per_group(ranked(v, |&((p, t), _)| (t, Reverse(creation_date.get(p).unwrap()), p), false), |&((_, t), _)| t);
    let rk = rel(v.into_iter().map(|(((p, _), s), r)| (p, s, r)).collect());
    let rk = rel(drain((&rk).filt(|(_, s, r)| s <= 10 && r <= 10)).into_iter().map(|x| x.1).collect());
    let ec = db.post_history.with((&db.post_history.post_history_type_id).is_in([4, 5, 6])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let lt = tag_mentions(db);
    let pc = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t)).select(Same::<(Id<Post>, Id<Tag>)>::new()).fold(0i64, |n, _| n + 1);
    let pt: MatSet<Id<Tag>> = db.tag.with((&pc).filt(|n| n >= 5)).collect();
    type R = (Id<Post>, i64, i64);
    let q = (&rk).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _, _): R| p).select(&ec))).cross(&pt);
    rows(drain(q).into_iter().map(|((_, t), (((p, s, _), e), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.push(V::S(db.post.owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(s), V::S(db.tag.tag_name.get(t).unwrap()), V::I(e)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON c.UserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id LEFT JOIN Badges b ON b.UserId = u.Id
//     WHERE u.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// UserRanked AS (SELECT ua.UserId, ua.DisplayName, ua.QuestionCount, ua.CommentCount, ua.UpVotes, ua.DownVotes, ua.GoldBadges, ua.SilverBadges, ua.BronzeBadges,
//        ROW_NUMBER() OVER (ORDER BY ua.UpVotes - ua.DownVotes DESC, ua.QuestionCount DESC, ua.CommentCount DESC) AS Rank FROM UserActivity ua)
// SELECT ur.Rank, ur.DisplayName, ur.QuestionCount, ur.CommentCount, ur.UpVotes, ur.DownVotes, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges FROM UserRanked ur WHERE ur.Rank <= 10 ORDER BY ur.Rank;
fn q25221(db: &'static So) -> String {
    let ua = db
        .user
        .with((&db.user.creation_date).ge(add_years(current_date(), -1)))
        .group_by(Ident::<User>::new())
        .select(
            posts_of(db)
                .select(&db.post.post_type_id)
                .opt()
                .and(comments_by(db).opt())
                .and(votes_by(db).select(&db.vote.vote_type_id).opt())
                .and(badges_of(db).select(&db.badge.class).opt()),
        )
        .fold([0i64; 7], |a, (((t, c), v), b)| {
            [a[0] + (t == Some(1)) as i64, a[1] + c.is_some() as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + (b == Some(1)) as i64, a[5] + (b == Some(2)) as i64, a[6] + (b == Some(3)) as i64]
        });
    let v = top_n(drain(&ua), |&(u, a)| (Reverse(a[2] - a[3]), Reverse(a[0]), Reverse(a[1]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, a))| {
        let mut f = vec![V::I(i as i64 + 1), user_col(db, u, "name")];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, TRIM(LEADING '<' FROM TRIM(TRAILING '>' FROM p.Tags)) AS CleanedTags, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') AND p.ViewCount > 100 GROUP BY p.Id, p.Title, p.Tags, p.CreationDate, p.Score, p.PostTypeId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CleanedTags, rp.CommentCount, rp.VoteCount, rp.UpVotes, rp.DownVotes, rp.CreationDate FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT fp.PostId, fp.Title, fp.CleanedTags, fp.CommentCount, fp.VoteCount, fp.UpVotes, fp.DownVotes,
//        CASE WHEN fp.UpVotes > fp.DownVotes THEN 'Positive' WHEN fp.UpVotes < fp.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM FilteredPosts fp ORDER BY fp.UpVotes DESC, fp.CommentCount DESC;
//
// Rank reads only base columns, so the top posts are picked first and the comments x votes product is driven for those alone.
fn q28299(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, tags_str, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(view_count.gt(100))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let nv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&s).and(&nv)).into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(harness::fmt::ostr(tags_str.get(p).map(|t: Str| t.trim_end_matches('>').trim_start_matches('<'))));
        f.extend([V::I(a[0]), V::I(n), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS ClosedPosts
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.BadgeCount, us.ClosedPosts, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.Rank WHERE us.Reputation > 1000 AND rp.Rank <= 5 ORDER BY us.Reputation DESC, rp.Score DESC LIMIT 10;
//
// `us.UserId = rp.Rank` joins a user id to a row number, so it goes through the raw ids. Rank reads only base columns, so the ranked posts that meet a
// user are picked first and both products are driven for those alone. A tie inside an owner's ROW_NUMBER goes to the lower post id.
fn q2605(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let v = drain(db.post.select(owner_user.opt()));
    let v = per_group(ranked(v, |&(p, u)| (u, Reverse(score.get(p).unwrap()), p), false), |&(_, u)| u);
    let rk = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let uidx: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.origid).inv().collect();
    let hits = rel(drain((&rk).filt(|(_, r)| r <= 5).select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(_, r)| r).select(&uidx)))).into_iter().map(|x| x.1).collect());
    let hp: MatSet<Id<Post>> = (&hits).map(|((p, _), _)| p).collect();
    let hu: MatSet<Id<User>> = (&hits).map(|(_, u)| u).collect();
    let rp = (&hp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let us = (&hu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).opt()))
        .fold(0i64, |n, (_, h)| n + (h.flatten() == Some(10)) as i64);
    let bc = (&hu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type H = ((Id<Post>, i64), Id<User>);
    let q = (&hits).select(Same::<H>::new().and(Same::<H>::new().map(|((p, _), _): H| p).select(&rp)).and(Same::<H>::new().map(|(_, u): H| u).select((&us).and(&bc))));
    let v = top_n(drain(q), |&(_, ((((p, _), u), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(_, ((((p, _), u), a), (c, b)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(c)]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p),
// FilteredUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(b.Name, 'No Badge') AS BadgeName, COUNT(DISTINCT ph.PostId) AS ClosedPostCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId AND b.Class = 1 LEFT JOIN PostHistory ph ON u.Id = ph.UserId AND ph.PostHistoryTypeId = 10 WHERE u.Reputation > 1000
//     GROUP BY u.Id, u.DisplayName, u.Reputation, b.Name),
// PostActivity AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, MAX(v.CreationDate) AS LastVoteDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT fu.UserId, fu.DisplayName, fu.Reputation, fu.BadgeName, p.PostId, p.Title, p.CreationDate, p.Score, pa.CommentCount, pa.LastVoteDate
// FROM FilteredUsers fu JOIN RankedPosts p ON fu.UserId = p.PostRank JOIN PostActivity pa ON p.PostId = pa.PostId
// WHERE p.PostRank <= 3 AND (pa.CommentCount > 0 OR pa.LastVoteDate IS NOT NULL) ORDER BY fu.Reputation DESC, p.CreationDate DESC;
//
// `fu.UserId = p.PostRank` joins a user id to a row number, so it goes through the raw ids. ClosedPostCount is never read, so it is not computed;
// FilteredUsers is one row per (user, gold badge name). A tie inside an owner's ROW_NUMBER goes to the lower post id.
fn q2569(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.select(owner_user.opt()));
    let v = per_group(ranked(v, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let rk = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))).select(&db.badge.name);
    let fu = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new().and(gold.opt())).select(Ident::<User>::new()).fold(0i64, |n, _| n + 1);
    let fv = rel(drain(&fu).into_iter().map(|((u, n), _)| (db.user.origid.get(u).unwrap(), (u, n))).collect());
    let fidx: HashIdx<i64, (i64, (Id<User>, Option<Str>))> = (&fv).map(|(o, _)| o).inv().select(&fv).collect();
    let hits = rel(drain((&rk).filt(|(_, r)| r <= 3).select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(_, r)| r).select(&fidx)))).into_iter().map(|x| x.1).collect());
    let hp: MatSet<Id<Post>> = (&hits).map(|((p, _), _)| p).collect();
    let pa = (&hp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.creation_date).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, d.map_or(m, |d| m.max(d))));
    type H = ((Id<Post>, i64), (i64, (Id<User>, Option<Str>)));
    let q = (&hits).select(Same::<H>::new().and(Same::<H>::new().map(|((p, _), _): H| p).select((&pa).filt(|(n, m)| n > 0 || m != i64::MIN))));
    rows(drain(q).into_iter().map(|(_, (((p, _), (_, (u, b))), (n, m)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::S(b.unwrap_or("No Badge")));
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend([V::I(n), tmax(m)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
//        SUM(CASE WHEN COALESCE(V.VoteTypeId, 0) = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN COALESCE(V.VoteTypeId, 0) = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        DENSE_RANK() OVER (ORDER BY SUM(COALESCE(P.ViewCount, 0)) DESC) AS ViewRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostWithMaxVotes AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, COUNT(V.Id) AS VoteCount, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY COUNT(V.Id) DESC) AS PostRank
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.PostCount, U.TotalViews, U.UpVotes, U.DownVotes, P.Title AS TopPostTitle, P.CreationDate AS TopPostDate, P.VoteCount AS TopPostVoteCount, U.ViewRank
// FROM UserStatistics U LEFT JOIN PostWithMaxVotes P ON U.UserId = P.OwnerUserId AND P.PostRank = 1 WHERE U.Reputation > 1000 ORDER BY U.Reputation DESC, U.TotalViews DESC LIMIT 10 OFFSET 0;
//
// The ownerless questions form their own partition, which the join on OwnerUserId never reaches, so they are left out before ranking.
fn q3124(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((w, t)) => [a[0] + w.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
            None => a,
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&us).and(&np)), |&(_, (a, _))| Reverse(a[0]), true);
    let vr = rel(v.into_iter().map(|((u, (a, n)), r)| (u, (a, n, r))).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, ([i64; 3], i64, i64))> = (&vr).map(|(u, _)| u).inv().select(&vr).collect();
    let qs = || db.post.with(post_type_id.eq(1));
    let vc = qs().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let pm = top_per(drain((&vc).and(owner_user)), |&(_, (_, u))| u, |&(_, (n, _))| Reverse(n), 1, true);
    let pm = rel(pm.into_iter().map(|(p, (n, u))| (u, (p, n))).collect());
    let top: HashIdx<Id<User>, (Id<User>, (Id<Post>, i64))> = (&pm).map(|(u, _)| u).inv().select(&pm).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&by_user).map(|(_, x)| x).and((&top).map(|(_, x)| x).opt())));
    let v = top_n(v, |&(u, ((a, _, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(u, ((a, n, r), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match p {
            Some((p, k)) => {
                let mut g = post_fields(db, p, &["title", "created"]);
                g.push(V::I(k));
                g
            }
            None => vec![V::Null, V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(p.ViewCount) AS TotalViews FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate),
// RankedUsers AS (SELECT us.*, RANK() OVER (ORDER BY us.Reputation DESC, us.PostCount DESC) AS ReputationRank FROM UserStats us),
// TopPostCounts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts FROM Posts p GROUP BY p.OwnerUserId HAVING COUNT(p.Id) > 5),
// ClosedPosts AS (SELECT ph.UserId, COUNT(ph.Id) AS ClosedPostCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId)
// SELECT ru.UserId, ru.DisplayName, ru.PostCount, ru.AnswerCount, ru.TotalViews, ru.ReputationRank, COALESCE(top.TotalPosts, 0) AS TotalPostsByUser, COALESCE(cp.ClosedPostCount, 0) AS ClosedPostsByUser
// FROM RankedUsers ru LEFT JOIN TopPostCounts top ON ru.UserId = top.OwnerUserId LEFT JOIN ClosedPosts cp ON ru.UserId = cp.UserId WHERE ru.ReputationRank <= 10 ORDER BY ru.ReputationRank;
fn q4311(db: &'static So) -> String {
    let Post { post_type_id, view_count, owner_user, .. } = &db.post;
    let us = db.user.with((&db.user.reputation).gt(0)).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, w)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)],
        None => a,
    });
    let v = ranked(drain((&us).and(&db.user.reputation)), |&(_, (a, r))| (Reverse(r), Reverse(a[0])), false);
    let top = db.post.group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rk = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, _)), r)| (u, (a, r))).collect());
    type R = (Id<User>, ([i64; 4], i64));
    let q = (&rk).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&top).filt(|n| n > 5).opt().and((&cp).opt()))));
    rows(drain(q).into_iter().map(|(_, ((u, (a, r)), (t, c)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(r), V::I(t.unwrap_or(0)), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, Score, CreationDate, OwnerName FROM RankedPosts WHERE rn <= 3),
// PostVoteCounts AS (SELECT PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS Downvotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY PostId),
// PostWithVoteCounts AS (SELECT tp.*, COALESCE(pvc.Upvotes, 0) AS Upvotes, COALESCE(pvc.Downvotes, 0) AS Downvotes FROM TopPosts tp LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId)
// SELECT pwc.PostId, pwc.Title, pwc.Score, pwc.Upvotes, pwc.Downvotes, pwc.CreationDate, pwc.OwnerName, ROUND((pwc.Upvotes::decimal / NULLIF((pwc.Upvotes + pwc.Downvotes), 0)) * 100, 2) AS UpvotePercentage
// FROM PostWithVoteCounts pwc WHERE pwc.Upvotes + pwc.Downvotes > 0 ORDER BY pwc.Score DESC, UpvotePercentage DESC LIMIT 10;
//
// A tie inside an owner's ROW_NUMBER goes to the lower post id.
fn q3439(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db)).opt()).fold([0i64; 2], |a, n| [a[0] + (n == Some("UpMod")) as i64, a[1] + (n == Some("DownMod")) as i64]);
    let pct = |a: [i64; 2]| (a[0] as f64 / (a[0] + a[1]) as f64 * 100.0 * 100.0).round() / 100.0;
    let v = top_n(drain((&pv).filt(|a| a[0] + a[1] > 0)), |&(p, a)| (Reverse(score.get(p).unwrap()), Reverse(fkey(pct(a)))), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["created", "owner"]));
        f.push(V::F(pct(a)));
        row(f)
    }))
}

// WITH TagSplit AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagCount AS (SELECT Tag, COUNT(DISTINCT PostId) AS PostCount FROM TagSplit GROUP BY Tag),
// UserEngagement AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvotesReceived,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvotesReceived, AVG(EXTRACT(EPOCH FROM (v.CreationDate - u.CreationDate))/86400) AS AvgPostAge
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 50 GROUP BY u.Id),
// ActiveTags AS (SELECT ts.Tag, SUM(ue.QuestionCount) AS TotalPosts, AVG(ue.UpvotesReceived) AS AvgUpvotes, AVG(ue.DownvotesReceived) AS AvgDownvotes
//     FROM TagCount tc JOIN TagSplit ts ON tc.Tag = ts.Tag JOIN UserEngagement ue ON ts.PostId = ue.UserId GROUP BY ts.Tag ORDER BY TotalPosts DESC LIMIT 10)
// SELECT tc.Tag, tc.PostCount, ae.TotalPosts, ae.AvgUpvotes, ae.AvgDownvotes FROM TagCount tc JOIN ActiveTags ae ON tc.Tag = ae.Tag ORDER BY tc.PostCount DESC;
//
// `ts.PostId = ue.UserId` joins a post id to a user id, so it goes through the raw ids. AvgPostAge is never read, so it is not computed.
fn q25798(db: &'static So) -> String {
    let Post { post_type_id, tags_str, origid, .. } = &db.post;
    let ts = rel(drain(db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list))));
    type T = (Id<Post>, Str);
    let tc = (&ts).group_by(Same::<T>::new().map(|(_, t): T| t)).select(Same::<T>::new().map(|(p, _): T| p)).count_distinct();
    let rich = || db.user.with((&db.user.reputation).gt(50));
    let asked = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let ud = rich().group_by(Ident::<User>::new()).select(asked().select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let qc = rich().group_by(Ident::<User>::new()).select(asked().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = rich().select(&db.user.origid).inv().collect();
    let at = (&ts)
        .group_by(Same::<T>::new().map(|(_, t): T| t))
        .select(Same::<T>::new().map(|(p, _): T| p).select(origid.select(&uidx).select((&qc).and(&ud))))
        .fold([0i64; 4], |a, (q, d)| [a[0] + 1, a[1] + q, a[2] + d[0], a[3] + d[1]]);
    let top = top_n(drain(&at), |&(_, a)| Reverse(a[1]), 10);
    let top = rel(top);
    let q = (&top).select(Same::<(Str, [i64; 4])>::new().and(Same::<(Str, [i64; 4])>::new().map(|(t, _)| t).select(&tc)));
    rows(drain(q).into_iter().map(|(_, ((t, a), n))| row(vec![V::S(t), V::I(n), V::I(a[1]), avg(a[2], a[0]), avg(a[3], a[0])])))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalBounties, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats WHERE TotalPosts > 0)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalBounties,
//        CASE WHEN tu.TotalQuestions > tu.TotalAnswers THEN 'More Questions than Answers' WHEN tu.TotalQuestions < tu.TotalAnswers THEN 'More Answers than Questions' ELSE 'Equal Questions and Answers' END AS PostBalance,
//        ph.CreationDate AS LastActivity
// FROM TopUsers tu LEFT JOIN Posts ph ON tu.UserId = ph.OwnerUserId LEFT JOIN PostHistory phs ON ph.Id = phs.PostId
// WHERE tu.Rank <= 10 AND tu.Reputation > (SELECT AVG(Reputation) FROM UserStats) ORDER BY tu.Reputation DESC;
//
// Rank reads only Reputation among users with a post (a distinct count), so the ten users are picked first and the posts x votes product is driven for those alone.
// The average reputation is an uncorrelated scalar over one row per user, computed once.
fn q2635(db: &'static So) -> String {
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let tu = top_n(drain((&np).and(&db.user.reputation)), |&(u, (_, r))| (Reverse(r), u), 10);
    let (s, n) = (&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let mean = s as f64 / n as f64;
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let tu: MatSet<Id<User>> = (&tu).with((&db.user.reputation).filt(move |r| r as f64 > mean)).collect();
    let us = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 4], |a, (t, b)| {
        let b = b.flatten();
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
    });
    let q = (&us).and(&np).and(posts_of(db).select(Ident::<Post>::new().and(history_of(db).opt())).opt());
    rows(drain(q).into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[3])]);
        f.push(V::S(if a[0] > a[1] { "More Questions than Answers" } else if a[0] < a[1] { "More Answers than Questions" } else { "Equal Questions and Answers" }));
        f.push(p.map_or(V::Null, |(p, _)| V::T(db.post.creation_date.get(p).unwrap())));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(p.Score) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(v.VoteCount, 0) AS VoteCount, p.OwnerUserId
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId),
// TopUsers AS (SELECT UserId, TotalPosts, TotalScore, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserStats)
// SELECT tu.UserId, tu.TotalPosts, tu.TotalScore, ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.CommentCount, ps.VoteCount FROM TopUsers tu JOIN PostStats ps ON tu.UserId = ps.OwnerUserId WHERE tu.Rank <= 10;
fn q14727(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let tu = top_n(drain(&us), |&(u, a)| (Reverse(a[1]), u), 10);
    let tu = rel(tu);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    type R = (Id<User>, [i64; 2]);
    let q = (&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(posts_of(db).select(Ident::<Post>::new().and(&cc).and(&vc)))));
    rows(drain(q).into_iter().map(|(_, ((u, a), ((p, c), v)))| {
        let mut f = vec![user_col(db, u, "uid"), V::I(a[0]), V::I(a[1])];
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend([V::I(c), V::I(v)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.Views, U.UpVotes, U.DownVotes, COUNT(DISTINCT P.Id) AS PostCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE U.Reputation > 1000 AND U.CreationDate < (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR') GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.Views, U.UpVotes, U.DownVotes),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotesCount, DownVotesCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation)
// SELECT T.DisplayName, T.Reputation, T.PostCount, T.QuestionCount, T.AnswerCount, T.UpVotesCount, T.DownVotesCount, CASE WHEN T.Rank <= 10 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributorStatus
// FROM TopUsers T WHERE T.Rank <= 50 ORDER BY T.Rank;
//
// Rank reads only Reputation, so the fifty users are picked first and the posts x votes product is driven for those alone.
fn q5483(db: &'static So) -> String {
    let User { reputation, creation_date, .. } = &db.user;
    let v = top_n(drain(db.user.with(reputation.gt(1000).and(creation_date.lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(reputation)), |&(u, r)| (Reverse(r), u), 50);
    let tu = rel(v.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let top: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let ur = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    type R = (Id<User>, i64);
    let q = (&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&np).and(&ur))));
    rows(drain(q).into_iter().map(|(_, ((u, r), (n, a)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top Contributor" } else { "Contributor" }));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END), 0) AS AcceptedAnswers,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, RANK() OVER (ORDER BY COUNT(P.Id) DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, AcceptedAnswers, UpVotes, DownVotes, UserRank FROM UserPostStats WHERE UserRank <= 10)
// SELECT TU.DisplayName, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.AcceptedAnswers, TU.UpVotes, TU.DownVotes, U.Reputation,
//        (SELECT COUNT(*) FROM Badges B WHERE B.UserId = TU.UserId) AS BadgeCount, (SELECT COUNT(*) FROM Comments C WHERE C.UserId = TU.UserId) AS CommentCount
// FROM TopUsers TU JOIN Users U ON TU.UserId = U.Id ORDER BY TU.PostCount DESC, TU.UserId;
fn q6068(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, x), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && x.is_some()) as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64],
            None => a,
        });
    let v = ranked(drain(&ups), |&(_, a)| Reverse(a[0]), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cc = (&tu).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&ups).and(&bc).and(&cc)).into_iter().map(|(u, ((a, b), c))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([user_col(db, u, "rep"), V::I(b), V::I(c)]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(v.Id) AS TotalVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostSummary AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViews, SUM(p.AnswerCount) AS TotalAnswers FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserActivityRanked AS (SELECT u.Id, u.DisplayName, us.Upvotes, us.Downvotes, us.TotalVotes, ps.TotalPosts, ps.TotalScore, ps.AvgViews, ps.TotalAnswers,
//        RANK() OVER (ORDER BY COALESCE(ps.TotalPosts, 0) DESC, COALESCE(us.Upvotes, 0) DESC) AS ActivityRank FROM Users u LEFT JOIN UserVoteSummary us ON u.Id = us.UserId LEFT JOIN PostSummary ps ON u.Id = ps.OwnerUserId)
// SELECT uar.DisplayName, uar.Upvotes, uar.Downvotes, uar.TotalVotes, uar.TotalPosts, uar.TotalScore, uar.AvgViews, uar.TotalAnswers, uar.ActivityRank FROM UserActivityRanked uar WHERE uar.ActivityRank <= 10 ORDER BY uar.ActivityRank;
fn q7013(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, answer_count, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1],
        None => a,
    });
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()).and(answer_count.opt()))
        .fold([0i64; 6], |a, ((s, w), n)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + n.is_some() as i64, a[5] + n.unwrap_or(0)]);
    let v = ranked(drain((&us).and((&ps).opt())), |&(_, (u, p))| (Reverse(p.map_or(0, |a| a[0])), Reverse(u[0])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, p)), r)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(match p {
            Some(b) => vec![V::I(b[0]), V::I(b[1]), avg(b[3], b[2]), nullable(b[5], b[4])],
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.Body, COUNT(c.Id) AS CommentCount, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpvoteCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownvoteCount,
//        COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Body, p.OwnerUserId, p.CreationDate),
// FilteredPosts AS (SELECT ps.*, CASE WHEN UpvoteCount > DownvoteCount THEN 'Popular' ELSE 'Less Popular' END AS Popularity FROM PostStatistics ps WHERE CommentCount > 5 AND PostRank <= 10)
// SELECT ps.PostId, ps.Title, ps.Body, ps.CommentCount, ps.UpvoteCount, ps.DownvoteCount, ps.GoldBadges, ps.SilverBadges, ps.BronzeBadges, ps.Popularity FROM FilteredPosts ps ORDER BY ps.UpvoteCount DESC, ps.CommentCount DESC;
//
// PostRank reads only base columns, so each owner's ten newest posts are picked first and the comments x votes x badges product is driven for those alone.
fn q29112(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db)).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, ((c, t), b)| {
            [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (b == Some(1)) as i64, a[4] + (b == Some(2)) as i64, a[5] + (b == Some(3)) as i64]
        });
    rows(drain((&s).filt(|a| a[0] > 5)).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "body"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] > a[2] { "Popular" } else { "Less Popular" }));
        row(f)
    }))
}

// Rewritten (rewrites/2811.sql): the rn ROW_NUMBER is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// AggregatedVotes AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, COALESCE(v.Upvotes, 0) AS Upvotes, COALESCE(v.Downvotes, 0) AS Downvotes,
//        (COALESCE(v.Upvotes, 0) - COALESCE(v.Downvotes, 0)) AS VoteBalance FROM RankedPosts rp LEFT JOIN AggregatedVotes v ON rp.PostId = v.PostId WHERE rp.rn = 1)
// SELECT pd.Title, pd.CreationDate, pd.OwnerDisplayName, pd.Score, pd.Upvotes, pd.Downvotes, pd.VoteBalance,
//        CASE WHEN pd.VoteBalance > 0 THEN 'Positive' WHEN pd.VoteBalance < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteStatus
// FROM PostDetails pd ORDER BY pd.VoteBalance DESC, pd.CreationDate DESC LIMIT 10;
fn q2811(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let av = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain(&av), |&(p, a)| (Reverse(a[0] - a[1]), Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(p, a)| {
        let b = a[0] - a[1];
        let mut f = post_fields(db, p, &["title", "created", "owner", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b), V::S(if b > 0 { "Positive" } else if b < 0 { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.OwnerUserId, RANK() OVER (ORDER BY P.ViewCount DESC) AS PopularityRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND P.Score > 0),
// PostVoteCounts AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount FROM Votes V GROUP BY V.PostId),
// CommentsSummary AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId)
// SELECT U.DisplayName, U.Reputation, U.ReputationRank, P.Title, P.PopularityRank, COALESCE(PV.UpvoteCount, 0) AS TotalUpvotes, COALESCE(PV.DownvoteCount, 0) AS TotalDownvotes,
//        COALESCE(CS.CommentCount, 0) AS TotalComments, P.CreationDate, P.Score
// FROM UserReputation U JOIN PopularPosts P ON U.UserId = P.OwnerUserId LEFT JOIN PostVoteCounts PV ON P.PostId = PV.PostId LEFT JOIN CommentsSummary CS ON P.PostId = CS.PostId
// WHERE U.Reputation > 1000 AND (P.AnswerCount > 5 OR P.Score > 10) ORDER BY U.Reputation DESC, P.PopularityRank;
fn q635(db: &'static So) -> String {
    let Post { creation_date, score, view_count, answer_count, owner_user, .. } = &db.post;
    let ur = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let urk: HashIdx<Id<User>, (Id<User>, i64)> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    let pp = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1)).and(score.gt(0))).select(view_count.opt())), |&(_, w)| (w.is_none(), Reverse(w)), false);
    let pp = rel(pp.into_iter().map(|((p, _), r)| (p, r)).collect());
    let posts: MatSet<Id<Post>> = (&pp).map(|(p, _)| p).collect();
    let pv = (&posts).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&posts).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let keep = Ident::<Post>::new().with(answer_count.gt(5).or(score.gt(10)));
    let owner = owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))).select(&urk);
    type R = (Id<Post>, i64);
    let q = (&pp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(keep).select(owner.and(&pv).and(&cc))));
    rows(drain(q).into_iter().map(|(_, ((p, k), (((u, r), a), c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(r));
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(k), V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.extend(post_fields(db, p, &["created", "score"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS TagRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.ViewCount > 50
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Tags, p.Score),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRanking FROM Users u WHERE u.Reputation IS NOT NULL),
// TopPosts AS (SELECT rp.*, ur.Reputation FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.TagRank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.CommentCount, tp.UpVoteCount, tp.Reputation,
//        CASE WHEN tp.UpVoteCount IS NULL THEN 'No Votes' WHEN tp.Reputation > 1000 THEN 'High Reputation User' ELSE 'Normal User' END AS UserType
// FROM TopPosts tp WHERE (tp.Reputation > 500 OR tp.CommentCount > 10) ORDER BY tp.UpVoteCount DESC, tp.CommentCount DESC FETCH FIRST 10 ROWS ONLY;
//
// TagRank reads only base columns, so the top posts per tag list are picked first and the comments x votes product is driven for those alone.
// A tie inside a tag list's ROW_NUMBER goes to the lower post id.
fn q2584(db: &'static So) -> String {
    let Post { creation_date, view_count, tags_str, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(view_count.gt(50))).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let q = (&s).and(owner_user.select(&db.user.reputation)).filt(|(a, r): ([i64; 2], i64)| r > 500 || a[0] > 10);
    let v = top_n(drain(q), |&(_, (a, _))| (Reverse(a[1]), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(p, (a, r))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(r), V::S(if r > 1000 { "High Reputation User" } else { "Normal User" })]);
        row(f)
    }))
}

// WITH RankedQuestions AS (SELECT p.Id AS QuestionId, p.Title, p.CreationDate, u.DisplayName AS OwnerName, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopQuestions AS (SELECT q.QuestionId, q.Title, q.OwnerName, q.ViewCount, q.AnswerCount FROM RankedQuestions q WHERE q.Rank <= 10),
// QuestionStatistics AS (SELECT tq.QuestionId, tq.Title, tq.OwnerName, tq.ViewCount, tq.AnswerCount, COALESCE(SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY tq.QuestionId), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY tq.QuestionId), 0) AS DownVotes, COALESCE(COUNT(c.Id) OVER (PARTITION BY tq.QuestionId), 0) AS CommentCount
//     FROM TopQuestions tq LEFT JOIN Votes vt ON tq.QuestionId = vt.PostId LEFT JOIN Comments c ON tq.QuestionId = c.PostId)
// SELECT qs.QuestionId, qs.Title, qs.OwnerName, qs.ViewCount, qs.AnswerCount, qs.UpVotes, qs.DownVotes, qs.CommentCount, (qs.UpVotes - qs.DownVotes) AS Score FROM QuestionStatistics qs ORDER BY Score DESC;
//
// The windows have no ORDER BY, so each joined row carries its question's totals over the votes x comments product; one output row per joined row.
fn q6712(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, owner_user, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(view_count.opt())), |&(p, w)| (w.is_none(), Reverse(w), p), 10);
    let tq: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let prod = || votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt());
    let qs = (&tq).group_by(Ident::<Post>::new()).select(prod()).fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    rows(drain((&tq).select(prod()).and(&qs)).into_iter().map(|(p, (_, a))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "views", "answers"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// Rewritten (rewrites/30218.sql): the ActivityRank ROW_NUMBER is tie-broken on p.Id.
// WITH RECURSIVE UserPosts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(p.ViewCount) AS TotalViews FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// RecentActivity AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS ActivityRank
//     FROM Posts p WHERE p.CreationDate > (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')),
// TopTags AS (SELECT t.TagName, COUNT(p.Id) AS TagCount FROM Tags t JOIN Posts p ON p.Tags ILIKE '%' || t.TagName || '%' GROUP BY t.TagName ORDER BY TagCount DESC LIMIT 10)
// SELECT u.DisplayName, u.Reputation, COALESCE(up.TotalPosts, 0) AS TotalPosts, COALESCE(up.PositivePosts, 0) AS PositivePosts, COALESCE(up.NegativePosts, 0) AS NegativePosts,
//        COALESCE(up.TotalViews, 0) AS TotalViews, ra.PostId, ra.Title, ra.CreationDate AS RecentPostDate, tt.TagName, tt.TagCount
// FROM Users u LEFT JOIN UserPosts up ON u.Id = up.UserId LEFT JOIN RecentActivity ra ON u.Id = ra.OwnerUserId AND ra.ActivityRank = 1 JOIN TopTags tt ON tt.TagCount > 0
// WHERE u.Reputation > 100 ORDER BY u.Reputation DESC, TotalPosts DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. The TopTags ON clause names only tt, so the users are crossed with the top tags.
// ILIKE is a substring test on the lowercased tag list. A tag name holds no '<' or '>', so a match lies inside one tag of the list, and the test runs
// on the distinct tags as in `tag_mentions`.
fn q30218(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, tags_str, .. } = &db.post;
    let up = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + w.unwrap_or(0)],
        None => a,
    });
    let recent = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let ra = rel(top_per(recent, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false).into_iter().map(|(p, u)| (u, p)).collect());
    let last: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&ra).map(|(u, _)| u).inv().select(&ra).collect();
    let lower = |s: Str| -> Str { Box::leak(s.to_lowercase().into_boxed_str()) };
    let elems: MatSet<Str> = tags_str.flat_map(tag_list).collect();
    let low: HashIdx<Str, Str> = (&elems).map(lower).collect();
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).map(lower).inv().collect();
    let hit: HashIdx<Str, Id<Tag>> = (&low).select_where(&names, |l: Str, n: Str| l.contains(n)).collect();
    let pairs: MatSet<(Id<Post>, Id<Tag>)> = db.post.select(Ident::<Post>::new().and(tags_str.flat_map(tag_list).select(&hit))).collect();
    type PT = (Id<Post>, Id<Tag>);
    let tc = (&pairs).group_by(Same::<PT>::new().map(|(_, t): PT| t)).select(Same::<PT>::new()).fold(0i64, |n, _| n + 1);
    let tt = rel(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10));
    let users = db.user.with((&db.user.reputation).gt(100)).select((&up).and((&last).map(|(_, p)| p).opt()));
    let mut out = Vec::new();
    users.cross((&tt).filt(|(_, n)| n > 0)).drive(|(u, _), ((a, p), (t, n))| out.push((u, a, p, t, n)));
    rows(out.into_iter().map(|(u, a, p, t, n)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "created"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS RankByViews
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CommentCount, RankByScore, RankByViews FROM RankedPosts WHERE RankByScore <= 10 OR RankByViews <= 10),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, us.DisplayName AS Author, us.PostsCount, us.Upvotes, us.Downvotes FROM TopPosts tp JOIN UserStats us ON tp.PostId = us.UserId ORDER BY tp.RankByScore, tp.RankByViews;
//
// `tp.PostId = us.UserId` joins a post id to a user id, so it goes through the raw ids; UserStats is computed for the users it reaches.
fn q9521(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let v = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |&(_, t)| t);
    let v = per_group(ranked(v, |&((p, t), _)| {
        let w = view_count.get(p);
        (t, w.is_none(), Reverse(w))
    }, false), |&((_, t), _)| t);
    let rk = rel(v.into_iter().map(|(((p, _), s), w)| (p, s, w)).collect());
    let tp = rel(drain((&rk).filt(|(_, s, w)| s <= 10 || w <= 10)).into_iter().map(|x| x.1).collect());
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let hit: MatSet<Id<User>> = (&tp).map(|(p, _, _)| p).select(origid).select(&uidx).collect();
    let us = (&hit).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let np = (&hit).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    type R = (Id<Post>, i64, i64);
    let q = (&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _, _): R| p).select((&cc).and(origid.select(&uidx).select(Ident::<User>::new().and(&np).and(&us))))));
    rows(drain(q).into_iter().map(|(_, ((p, _, _), (c, ((u, n), a))))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(c), user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(a.Body, 'No accepted answer') AS AcceptedAnswerBody, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, a.Body, p.OwnerUserId),
// UserDetails AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, CASE WHEN u.LastAccessDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' THEN 'Inactive' ELSE 'Active' END AS UserStatus FROM Users u)
// SELECT pd.PostId, pd.Title, pd.AcceptedAnswerBody, pd.CommentCount, pd.UpVotes, pd.DownVotes, ud.DisplayName AS UserDisplayName, ud.Reputation, ud.UserStatus, pd.UserPostRank
// FROM PostDetails pd JOIN Users u ON pd.UserPostRank = 1 AND u.Id = pd.UserPostRank JOIN UserDetails ud ON u.Id = ud.UserId
// WHERE pd.CommentCount > 5 ORDER BY pd.UpVotes DESC, pd.CreationDate DESC LIMIT 10 OFFSET 5;
//
// The Users join reads `u.Id = pd.UserPostRank = 1`: every first question meets the user whose id is 1, a cross join. UserPostRank reads only base columns,
// so the first questions are picked before the comments x votes product. A tie inside an owner's ROW_NUMBER goes to the lower post id.
fn q196(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, accepted_answer, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let fp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pd = (&fp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let one: MatSet<Id<User>> = db.user.with((&db.user.origid).eq(1)).collect();
    let v = drain((&pd).filt(|a| a[0] > 5).cross(&one));
    let v = top_n(v, |&((p, _), (a, _))| (Reverse(a[1]), Reverse(creation_date.get(p).unwrap())), 15);
    rows(v.into_iter().skip(5).map(|((p, u), (a, _))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(accepted_answer.get(p).map_or("No accepted answer", |x| db.post.body.get(x).unwrap())));
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::S(if db.user.last_access_date.get(u).unwrap() < add_years(ts(2024, 10, 1, 12, 34, 56), -1) { "Inactive" } else { "Active" }));
        f.push(V::I(1));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 2) AS UpvoteCount,
//        COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 3) AS DownvoteCount, RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank, p.OwnerUserId  -- Added OwnerUserId to the GROUP BY clause
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.OwnerUserId  -- Added necessary columns to GROUP BY),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CommentCount, UpvoteCount, DownvoteCount, OwnerUserId  -- Added OwnerUserId to TopPosts to use in the final SELECT
//     FROM RankedPosts WHERE ScoreRank <= 10)
// SELECT tp.*, u.DisplayName AS OwnerDisplayName, b.Name AS BadgeName, CASE WHEN tp.UpvoteCount > tp.DownvoteCount THEN 'Positive' ELSE 'Negative or Neutral' END AS Sentiment
// FROM TopPosts tp JOIN Users u ON tp.OwnerUserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId AND b.Date >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// ScoreRank reads only Score, so the top posts are picked first and the comments x votes product is driven for those alone.
fn q8648(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(t0, -1))).select(score)), |&(_, s)| Reverse(s), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let Vote { vote_type_id, user, .. } = &db.vote;
    let up = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2))).select(user)).count_distinct();
    let down = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(3))).select(user)).count_distinct();
    let recent = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.date).ge(add_years(t0, -1)))).select(&db.badge.name);
    let q = (&cc).and((&up).opt()).and((&down).opt()).and(owner_user.select(Ident::<User>::new().and(recent.opt())));
    rows(drain(q).into_iter().map(|(p, (((c, u), d), (o, b)))| {
        let (u, d) = (u.unwrap_or(0), d.unwrap_or(0));
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::I(u), V::I(d), user_col(db, o, "uid"), user_col(db, o, "name"), harness::fmt::ostr(b)]);
        f.push(V::S(if u > d { "Positive" } else { "Negative or Neutral" }));
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT T.TagName, COUNT(P.Id) AS PostCount, COUNT(DISTINCT U.Id) AS UserCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AvgScore
//     FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE CONCAT('%<', T.TagName, '>%') LEFT JOIN Users U ON U.Id = P.OwnerUserId WHERE P.PostTypeId = 1 GROUP BY T.TagName),
// TopTags AS (SELECT TagName, PostCount, UserCount, TotalViews, AvgScore, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM TagStatistics),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, T.TagName, P.Score AS PostScore, COALESCE(CNT.CommentCount, 0) AS CommentCount
//     FROM Posts P JOIN Tags T ON P.Tags LIKE CONCAT('%<', T.TagName, '>%') LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) CNT ON CNT.PostId = P.Id
//     WHERE P.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days' ORDER BY P.CreationDate DESC)
// SELECT T.TagName, T.PostCount, T.UserCount, T.TotalViews, T.AvgScore, R.PostId, R.Title, R.CreationDate, R.ViewCount, R.CommentCount
// FROM TopTags T JOIN RecentPosts R ON T.TagName = R.TagName WHERE T.Rank <= 10 ORDER BY T.Rank, R.CreationDate DESC;
//
// `LIKE '%<' || TagName || '>%'` matches a whole tag of the list, which is the exploded Post.tags edge.
fn q27712(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, score, tags, creation_date, .. } = &db.post;
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = tags.inv().collect();
    let tst = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(view_count.opt().and(score)))
        .fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let uc = db.tag.group_by(Ident::<Tag>::new()).select((&by_tag).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(owner_user)).count_distinct();
    let tt = rel(top_n(drain((&tst).and((&uc).opt())), |&(t, (a, _))| (Reverse(a[0]), t), 10));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let recent = (&by_tag).select(Ident::<Post>::new().with(creation_date.gt(add_days(ts(2024, 10, 1, 0, 0, 0), -30)))).select(Ident::<Post>::new().and((&cc).opt()));
    type R = (Id<Tag>, ([i64; 4], Option<i64>));
    let q = (&tt).select(Same::<R>::new().and(Same::<R>::new().map(|(t, _): R| t).select(recent)));
    rows(drain(q).into_iter().map(|(_, ((t, (a, u)), (p, c)))| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(u.unwrap_or(0)), nullable(a[2], a[1]), avg(a[3], a[0])];
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.push(V::I(c.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' AND p.Score > 0),
// RecentUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, (SELECT COUNT(*) FROM Posts pp WHERE pp.OwnerUserId = u.Id AND pp.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months') AS RecentPostsCount
//     FROM Users u WHERE u.Reputation > 100 ORDER BY u.CreationDate DESC LIMIT 10),
// PostVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ru.DisplayName AS TopUser, ru.Reputation, COALESCE(pv.Upvotes, 0) AS TotalUpvotes, COALESCE(pv.Downvotes, 0) AS TotalDownvotes
// FROM RankedPosts rp LEFT JOIN RecentUsers ru ON rp.OwnerUserId = ru.UserId LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId WHERE rp.PostRank = 1 AND ru.RecentPostsCount > 0 ORDER BY rp.Score DESC, rp.CreationDate DESC;
fn q1223(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let since = add_months(ts(2024, 10, 1, 12, 34, 56), -6);
    let v = drain(db.post.with(creation_date.ge(since).and(score.gt(0))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 1, true);
    let tp = rel(top);
    let ru = top_n(drain(db.user.with((&db.user.reputation).gt(100)).select(&db.user.creation_date)), |&(u, d)| (Reverse(d), u), 10);
    let ru: MatSet<Id<User>> = rel(ru.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let rc = (&ru).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(since))).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    type R = (Id<Post>, Id<User>);
    let q = (&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(_, u): R| u).select((&rc).filt(|n| n > 0))).and(Same::<R>::new().map(|(p, _): R| p).select(&pv)));
    rows(drain(q).into_iter().map(|(_, (((p, u), _), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(b.Class), 0) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PostAggregates AS (SELECT rp.Id, rp.Title, rp.ViewCount, rp.CreationDate, ur.Reputation, ur.TotalBadges, COALESCE(cp.CloseCount, 0) AS CloseCount
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId)
// SELECT pa.Title, pa.ViewCount, pa.Reputation, pa.TotalBadges, pa.CloseCount, CASE WHEN pa.CloseCount > 0 THEN 'Closed' ELSE 'Active' END AS PostStatus,
//        DENSE_RANK() OVER (ORDER BY pa.Reputation DESC) AS ReputationRank
// FROM PostAggregates pa WHERE pa.Reputation > 500 ORDER BY pa.ViewCount DESC, pa.Reputation DESC FETCH FIRST 10 ROWS ONLY;
//
// UserRank is never read, so it is not computed.
fn q4090(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let tb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold(0i64, |n, c| n + c);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pa = drain(db.post.with(post_type_id.eq(1)).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(500)).and((&tb).opt())).and((&cp).opt())));
    let v = ranked(pa, |&(_, ((u, _), _))| Reverse(db.user.reputation.get(u).unwrap()), true);
    let v = top_n(v, |&((p, ((u, _), _)), _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(db.user.reputation.get(u).unwrap()))
    }, 10);
    rows(v.into_iter().map(|((p, ((u, b), c)), r)| {
        let c = c.unwrap_or(0);
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([user_col(db, u, "rep"), V::I(b.unwrap_or(0)), V::I(c), V::S(if c > 0 { "Closed" } else { "Active" }), V::I(r)]);
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikiCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TagWikiCount, UpvoteCount, DownvoteCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserMetrics)
// SELECT T.DisplayName, T.Reputation, T.PostCount, T.QuestionCount, T.AnswerCount, T.TagWikiCount, T.UpvoteCount, T.DownvoteCount, T.Rank, (SELECT AVG(Reputation) FROM Users) AS AvgReputation,
//        (SELECT COUNT(*) FROM Posts WHERE CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days') AS RecentPostCount
// FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.Rank;
//
// Rank reads only Reputation, so the top ten users are picked first and the posts x votes product is driven for those alone. The two scalars are computed once.
fn q5073(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(tu.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let top: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let um = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 4 || t == 5) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let np = (&top).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let (s, n) = (&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let recent = count(db.post.with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    type R = (Id<User>, i64);
    let q = (&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&np).and(&um))));
    rows(drain(q).into_iter().map(|(_, ((u, r), (c, a)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(c));
        f.extend(a.map(V::I));
        f.extend([V::I(r), avg(s, n), V::I(recent)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        AVG(COALESCE(P.ViewCount, 0)) AS AvgViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id GROUP BY U.Id, U.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalUpvotes, TotalDownvotes, TotalQuestions, TotalAnswers, AvgViews, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank,
//        RANK() OVER (ORDER BY TotalUpvotes DESC) AS UpvoteRank FROM UserActivity)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.TotalComments, U.TotalUpvotes, U.TotalDownvotes, U.TotalQuestions, U.TotalAnswers, U.AvgViews, UR.PostRank, UR.UpvoteRank
// FROM UserActivity U JOIN RankedUsers UR ON U.UserId = UR.UserId WHERE U.TotalPosts > 0 ORDER BY U.TotalPosts DESC, U.TotalUpvotes DESC LIMIT 10;
//
// `V.UserId = U.Id` keeps the votes each user cast on their own posts.
fn q6051(db: &'static So) -> String {
    let ov = own_votes(db);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&db.post.view_count).opt()).and(comments_of(db).opt()).and((&ov).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some((((t, w), _), v)) => [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + 1, a[5] + w.unwrap_or(0)],
            None => [a[0], a[1], a[2], a[3], a[4] + 1, a[5]],
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&ua).and(&np).and(&nc)), |&(_, ((_, n), _))| Reverse(n), false);
    let v = ranked(v, |&((_, ((a, _), _)), _)| Reverse(a[0]), false);
    let kept = rel(v.into_iter().map(|(((u, ((a, n), c)), pr), ur)| (u, a, n, c, pr, ur)).collect());
    let v = top_n(drain((&kept).filt(|(_, _, n, _, _, _)| n > 0)), |&(_, (_, a, n, _, _, _))| (Reverse(n), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(_, (u, a, n, c, pr, ur))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4]), V::I(pr), V::I(ur)]);
        row(f)
    }))
}

// WITH RecursiveTagCounts AS (SELECT TagName, COUNT(*) AS PostCount FROM Tags GROUP BY TagName),
// UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.CreationDate, P.Title AS PostTitle, PP.LastActivityDate, PT.Name AS PostTypeName,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id LEFT JOIN Posts PP ON P.ParentId = PP.Id
//     WHERE P.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' AND P.OwnerUserId IS NOT NULL)
// SELECT U.DisplayName AS AuthorDisplayName, U.Reputation AS AuthorReputation, U.ReputationRank, RT.TagName, TC.PostCount, RP.PostTitle, RP.CreationDate AS RecentPostDate, RP.PostTypeName, RP.RecentPostRank,
//        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = RP.PostId AND V.VoteTypeId = 2), 0) AS UpVotes
// FROM UserReputation U JOIN RecentPosts RP ON U.UserId = RP.OwnerUserId JOIN PostLinks PL ON PL.PostId = RP.PostId JOIN Tags RT ON RT.Id = PL.RelatedPostId JOIN RecursiveTagCounts TC ON RT.TagName = TC.TagName
// WHERE RP.RecentPostRank = 1 AND RP.LastActivityDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 week' AND TC.PostCount > 5 ORDER BY U.Reputation DESC, TC.PostCount DESC;
//
// Not recursive despite the name. `RT.Id = PL.RelatedPostId` joins a tag id to a post id, so it goes through the raw ids. A tie inside an owner's ROW_NUMBER goes to the lower post id.
fn q30899(db: &'static So) -> String {
    let Post { owner_user, creation_date, parent, last_activity_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.gt(add_months(t0, -1))).select(owner_user));
    let rp = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ur = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let urk: HashIdx<Id<User>, (Id<User>, i64)> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    let tc = db.tag.group_by(&db.tag.tag_name).select(Ident::<Tag>::new()).fold(0i64, |n, _| n + 1);
    let tidx: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let links: HashIdx<Id<Post>, Id<PostLink>> = (&db.post_link.post).inv().collect();
    let tags = (&links).select(&db.post_link.related_post_id).select(&tidx).select(Ident::<Tag>::new().and((&db.tag.tag_name).select((&tc).filt(|n| n > 5))));
    let up = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let q = (&rp).with(parent.select(last_activity_date.gt(add_days(t0, -7)))).select(owner_user.select(&urk).and(tags).and(&up));
    let mut v = drain(q);
    v.sort_by_key(|&(_, (((u, _), (_, n)), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n)));
    rows(v.into_iter().map(|(p, (((u, r), (t, n)), k))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(r), V::S(db.tag.tag_name.get(t).unwrap()), V::I(n)]);
        f.extend(post_fields(db, p, &["title", "created", "type"]));
        f.extend([V::I(1), V::I(k)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN h.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount,
//        SUM(CASE WHEN h.PostHistoryTypeId = 12 THEN 1 ELSE 0 END) AS DeleteCount, SUM(CASE WHEN h.PostHistoryTypeId = 1 OR h.PostHistoryTypeId = 4 THEN 1 ELSE 0 END) AS TitleEditCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory h ON p.Id = h.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, AcceptedAnswers, UpVotes, DownVotes, CloseCount, DeleteCount, TitleEditCount, RANK() OVER (ORDER BY PostCount DESC) AS UserRank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, AnswerCount, AcceptedAnswers, UpVotes, DownVotes, CloseCount, DeleteCount, TitleEditCount, UserRank FROM TopUsers WHERE UserRank <= 10 ORDER BY UserRank;
//
// UserRank reads only the distinct post count, so the top users are picked first and the posts x votes x history product is driven for those alone.
fn q7132(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, .. } = &db.post;
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain(&np), |&(_, n)| Reverse(n), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, n), r)| (u, (n, r))).collect());
    let top: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let ua = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt())))
        .fold([0i64; 7], |a, (((t, x), v), h)| {
            [a[0] + (t == 2) as i64, a[1] + (t == 1 && x.is_some()) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + (h == Some(10)) as i64, a[5] + (h == Some(12)) as i64, a[6] + (h == Some(1) || h == Some(4)) as i64]
        });
    type R = (Id<User>, (i64, i64));
    let q = (&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&ua).opt())));
    rows(drain(q).into_iter().map(|(_, ((u, (n, r)), a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.unwrap_or([0; 7]).map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY COUNT(c.Id) DESC) AS RankByComments
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount > 100
//     GROUP BY p.Id, p.Title, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.PostTypeId, rp.CommentCount, rp.UpVotes, rp.DownVotes, CASE WHEN rp.CommentCount = 0 THEN 0 ELSE (CAST(rp.UpVotes AS FLOAT) / NULLIF(rp.CommentCount, 0)) END AS UpvoteToCommentRatio
//     FROM RankedPosts rp WHERE rp.RankByComments <= 5)
// SELECT tp.PostId, tp.Title, pt.Name AS PostType, tp.CommentCount, tp.UpVotes, tp.DownVotes, tp.UpvoteToCommentRatio, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
// FROM TopPosts tp JOIN PostTypes pt ON tp.PostTypeId = pt.Id JOIN Users u ON u.Id = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = tp.PostId) ORDER BY tp.UpvoteToCommentRatio DESC, tp.CommentCount DESC;
//
// FLOAT is single precision: the ratio is computed as f32.
fn q8799(db: &'static So) -> String {
    let Post { creation_date, view_count, post_type_id, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(view_count.gt(100)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let top = top_per(drain((&rp).and(post_type_id)), |&(_, (_, t))| t, |&(p, (a, _))| (Reverse(a[0]), p), 5, false);
    let tp = rel(top);
    type R = (Id<Post>, ([i64; 3], i64));
    let q = (&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(owner_user)));
    rows(drain(q).into_iter().map(|(_, ((p, (a, _)), u))| {
        let r = if a[0] == 0 { 0.0 } else { (a[1] as f32 / a[0] as f32) as f64 };
        let mut f = post_fields(db, p, &["id", "title", "type"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(r)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RN
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1),
// RecentBadges AS (SELECT b.UserId, b.Name AS BadgeName, COUNT(b.Id) AS BadgeCount, RANK() OVER (ORDER BY COUNT(b.Id) DESC) AS BadgeRank FROM Badges b GROUP BY b.UserId, b.Name),
// UserMetrics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(RB.BadgeCount, 0) AS BadgeCount, COALESCE(RankedPosts.Score, 0) AS MaxPostScore
//     FROM Users U LEFT JOIN (SELECT DISTINCT UserId, BadgeCount FROM RecentBadges) RB ON U.Id = RB.UserId
//     LEFT JOIN (SELECT OwnerUserId, MAX(Score) AS Score FROM Posts GROUP BY OwnerUserId) RankedPosts ON U.Id = RankedPosts.OwnerUserId)
// SELECT UM.DisplayName, UM.Reputation, UM.BadgeCount, RP.Title AS TopPostTitle, RP.Score AS TopPostScore, RP.ViewCount AS TopPostViews
// FROM UserMetrics UM LEFT JOIN RankedPosts RP ON UM.UserId = RP.OwnerUserId AND RP.RN = 1 WHERE (UM.Reputation > 1000 OR UM.BadgeCount > 0) AND RP.Score IS NOT NULL
// ORDER BY UM.Reputation DESC, UM.BadgeCount DESC LIMIT 50;
//
// MaxPostScore is never read, so it is not computed. A tie inside an owner's ROW_NUMBER goes to the lower post id.
fn q31797(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let rp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let best: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let Badge { user, name, .. } = &db.badge;
    let rb = db.badge.group_by(user.and(name)).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let pairs: MatSet<(Id<User>, i64)> = rel(drain(&rb)).map(|((u, _), n)| (u, n)).collect();
    let bc: HashIdx<Id<User>, i64> = (&pairs).map(|(u, _)| u).inv().map(|(_, n)| n).collect();
    let q = db
        .user
        .select((&db.user.reputation).and((&bc).opt()).and((&best).map(|(_, p)| p)))
        .filt(|((r, b), _): ((i64, Option<i64>), Id<Post>)| r > 1000 || b.unwrap_or(0) > 0)
        .map(|((_, b), p)| (b.unwrap_or(0), p));
    let v = drain(q);
    let v = top_n(v, |&(u, (b, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(b)), 50);
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// ClosedPosts AS (SELECT PH.PostId, PH.UserId, PH.CreationDate, C.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes C ON CAST(PH.Comment AS INTEGER) = C.Id WHERE PH.PostHistoryTypeId = 10),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats),
// TopUsers AS (SELECT * FROM RankedUsers WHERE ReputationRank <= 10)
// SELECT T.DisplayName, T.TotalQuestions, T.TotalAnswers, T.TotalViews, COALESCE(CP.CloseReason, 'No Closure') AS RecentCloseReason, (SELECT COUNT(*) FROM Comments C WHERE C.UserId = T.UserId) AS TotalComments
// FROM TopUsers T LEFT JOIN ClosedPosts CP ON T.UserId = CP.UserId WHERE T.TotalPosts > 5 ORDER BY T.Reputation DESC, T.DisplayName;
fn q2230(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt()))).fold([0i64; 4], |a, (t, w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0)]
    });
    let cc = (&tu).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, user, .. } = &db.post_history;
    let closer: HashIdx<Id<User>, (Id<PostHistory>, Str)> = db.post_history.with(post_history_type_id.eq(10)).select(user).inv().select(Ident::<PostHistory>::new().and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))).collect();
    rows(drain((&us).filt(|a| a[0] > 5).and(&cc).and((&closer).opt())).into_iter().map(|(u, ((a, c), r))| {
        row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::S(r.map_or("No Closure", |x| x.1)), V::I(c)])
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY COUNT(rp.PostId) DESC) AS UserRank FROM Users u JOIN RecentPosts rp ON u.Id = rp.OwnerUserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(rp.PostId) > 5)
// SELECT u.UserId, u.DisplayName, u.Reputation, COALESCE(SUM(rp.UpVotes) - SUM(rp.DownVotes), 0) AS NetVotes, COALESCE(SUM(rp.CommentCount), 0) AS TotalComments,
//        CASE WHEN u.Reputation > 1000 THEN 'Veteran' ELSE 'Novice' END AS UserCategory
// FROM TopUsers u LEFT JOIN RecentPosts rp ON u.UserId = rp.OwnerUserId GROUP BY u.UserId, u.DisplayName, u.Reputation HAVING COUNT(rp.PostId) > 0 ORDER BY NetVotes DESC, TotalComments DESC;
//
// PostRank and UserRank are never read, so they are not computed.
fn q954(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rp = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let per = recent().group_by(owner_user).select(&rp).fold([0i64; 4], |a, x| [a[0] + 1, a[1] + x[1], a[2] + x[2], a[3] + x[0]]);
    rows(drain((&per).filt(|a| a[0] > 5)).into_iter().map(|(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1] - a[2]), V::I(a[3]), V::S(if r > 1000 { "Veteran" } else { "Novice" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.OwnerDisplayName, pd.CommentCount, pd.UpVoteCount, pd.DownVoteCount FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC;
fn q7881(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = top_n(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).with(owner_user).select(score)), |&(_, s)| Reverse(s), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
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

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER(PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// PostVoteDetails AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v INNER JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// PostHistoryFilter AS (SELECT ph.PostId, COUNT(CASE WHEN pht.Name IN ('Post Locked', 'Post Closed') THEN 1 END) AS LockCloseCount FROM PostHistory ph INNER JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, COALESCE(pvd.UpVotes, 0) AS UpVotes, COALESCE(pvd.DownVotes, 0) AS DownVotes, rp.ViewCount, rp.CreationDate, COALESCE(phf.LockCloseCount, 0) AS LockCloseCount,
//        CASE WHEN rp.Rank <= 3 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory
// FROM RankedPosts rp LEFT JOIN PostVoteDetails pvd ON rp.PostId = pvd.PostId LEFT JOIN PostHistoryFilter phf ON rp.PostId = phf.PostId
// WHERE COALESCE(phf.LockCloseCount, 0) > 0 AND rp.Rank <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// A tie inside a post type's ROW_NUMBER goes to the lower post id (the answer is empty on this data, so the tie is not tested).
fn q33524(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(post_type_id));
    let v = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), false), |&(_, t)| t);
    let rk = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let lc = history_of(db).select(htype_name(db)).opt().map(|n: Option<Str>| matches!(n, Some("Post Locked" | "Post Closed")) as i64);
    let lcc = db.post.group_by(Ident::<Post>::new()).select(lc).fold(0i64, |n, x| n + x);
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db)).opt()).fold([0i64; 2], |a, n| [a[0] + (n == Some("UpMod")) as i64, a[1] + (n == Some("DownMod")) as i64]);
    type R = (Id<Post>, i64);
    let q = (&rk).filt(|(_, r): R| r <= 10).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&lcc).filt(|n| n > 0).and(&pv))));
    rows(drain(q).into_iter().map(|(_, ((p, r), (n, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["views", "created"]));
        f.extend([V::I(n), V::S(if r <= 3 { "Top Post" } else { "Regular Post" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopUsers AS (SELECT OwnerDisplayName, COUNT(*) AS PostCount, SUM(Score) AS TotalScore FROM RankedPosts WHERE Rank <= 3 GROUP BY OwnerDisplayName HAVING COUNT(*) > 0),
// UserBadges AS (SELECT u.Id, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id),
// PostsWithBadges AS (SELECT pu.OwnerDisplayName, pu.PostCount, pu.TotalScore, ub.BadgeCount FROM TopUsers pu LEFT JOIN UserBadges ub ON pu.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = ub.Id))
// SELECT p.OwnerDisplayName, p.PostCount, p.TotalScore, COALESCE(p.BadgeCount, 0) AS BadgeCount,
//        CASE WHEN p.BadgeCount >= 5 THEN 'Top Contributor' WHEN p.BadgeCount >= 3 THEN 'Active Contributor' ELSE 'New Contributor' END AS ContributorLevel
// FROM PostsWithBadges p ORDER BY p.TotalScore DESC LIMIT 10;
//
// Which of an owner's tied posts the ROW_NUMBER keeps cannot change the count or the score sum.
fn q3018(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tu = (&tp).group_by(owner_user.select(&db.user.display_name)).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let ub = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ubv = rel(drain(&ub));
    let by_name: HashIdx<Str, (Id<User>, i64)> = (&ubv).map(|(u, _)| u).select(&db.user.display_name).inv().select(&ubv).collect();
    let v = drain((&tu).and((&by_name).map(|(_, n)| n).opt()));
    let v = top_n(v, |&(_, (a, _))| Reverse(a[1]), 10);
    rows(v.into_iter().map(|(n, (a, b))| {
        let lvl = match b {
            Some(b) if b >= 5 => "Top Contributor",
            Some(b) if b >= 3 => "Active Contributor",
            _ => "New Contributor",
        };
        row(vec![V::S(n), V::I(a[0]), V::I(a[1]), V::I(b.unwrap_or(0)), V::S(lvl)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(NULLIF(u.DisplayName, ''), 'Anonymous') AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.UserId END) AS UpVotes, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 3 THEN v.UserId END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId),
// PostHistorySummary AS (SELECT p.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditedDate FROM PostHistory ph JOIN RankedPosts p ON ph.PostId = p.PostId
//     WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY p.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes, phs.EditCount, phs.LastEditedDate
// FROM RankedPosts rp LEFT JOIN PostHistorySummary phs ON rp.PostId = phs.PostId WHERE rp.PostRank <= 5 ORDER BY rp.PostId, rp.CreationDate DESC;
//
// PostRank reads only base columns, so the five newest posts of each type are picked first and the comments x votes product is driven for those alone.
fn q5877(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.is_in([1, 2])).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let Vote { vote_type_id, user, .. } = &db.vote;
    let up = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2))).select(user)).count_distinct();
    let down = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(3))).select(user)).count_distinct();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6]))).select(hd)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    rows(drain((&cc).and((&up).opt()).and((&down).opt()).and((&ph).opt())).into_iter().map(|(p, (((c, u), d), h))| {
        let name = owner_user.get(p).map(|u| db.user.display_name.get(u).unwrap()).filter(|n| !n.is_empty()).unwrap_or("Anonymous");
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::S(name), V::I(c), V::I(u.unwrap_or(0)), V::I(d.unwrap_or(0))]);
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id LEFT JOIN Badges B ON U.Id = B.UserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// ActiveUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, CommentCount, UpvoteCount, DownvoteCount, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY PostCount DESC) AS PostCountRank, RANK() OVER (ORDER BY CommentCount DESC) AS CommentCountRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, CommentCount, UpvoteCount, DownvoteCount, BadgeCount, ReputationRank, PostCountRank, CommentCountRank
// FROM ActiveUsers WHERE Reputation > 1000 ORDER BY ReputationRank, PostCountRank, CommentCountRank OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
//
// The three ranks read only Reputation and two distinct counts, so the ten rows are picked first and the product is driven for those alone.
// `V.UserId = U.Id` keeps the votes each user cast on their own posts.
fn q8833(db: &'static So) -> String {
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&db.user.reputation).and(&np).and(&nc)), |&(_, ((r, _), _))| Reverse(r), false);
    let v = ranked(v, |&((_, ((_, n), _)), _)| Reverse(n), false);
    let v = ranked(v, |&(((_, (_, c)), _), _)| Reverse(c), false);
    let all = rel(v.into_iter().map(|((((u, ((r, n), c)), a), b), k)| (u, r, n, c, a, b, k)).collect());
    let v = top_n(drain((&all).filt(|(_, r, _, _, _, _, _)| r > 1000)), |&(_, (_, _, _, _, a, b, k))| (a, b, k), 20);
    let sel = rel(v.into_iter().skip(10).map(|x| x.1).collect());
    let top: MatSet<Id<User>> = (&sel).map(|(u, _, _, _, _, _, _)| u).collect();
    let ov = own_votes(db);
    let us = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and((&ov).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, b)| {
            let t = p.and_then(|x| x.1);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]
        });
    type R = (Id<User>, i64, i64, i64, i64, i64, i64);
    let q = (&sel).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _, _, _, _, _, _): R| u).select(&us)));
    rows(drain(q).into_iter().map(|(_, ((u, _, n, c, a, b, k), s))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(c), V::I(s[0]), V::I(s[1]), V::I(s[2]), V::I(a), V::I(b), V::I(k)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("749", q749),
    ("2350", q2350),
    ("8558", q8558),
    ("20794", q20794),
    ("27976", q27976),
    ("2537", q2537),
    ("9139", q9139),
    ("9723", q9723),
    ("34959", q34959),
    ("491", q491),
    ("2644", q2644),
    ("72", q72),
    ("737", q737),
    ("9404", q9404),
    ("2189", q2189),
    ("4487", q4487),
    ("4683", q4683),
    ("8074", q8074),
    ("9220", q9220),
    ("1870", q1870),
    ("8519", q8519),
    ("2638", q2638),
    ("7727", q7727),
    ("29614", q29614),
    ("32153", q32153),
    ("4415", q4415),
    ("5109", q5109),
    ("5468", q5468),
    ("6342", q6342),
    ("6749", q6749),
    ("3841", q3841),
    ("4141", q4141),
    ("9524", q9524),
    ("2815", q2815),
    ("5725", q5725),
    ("7034", q7034),
    ("1044", q1044),
    ("164", q164),
    ("23233", q23233),
    ("7119", q7119),
    ("7350", q7350),
    ("26841", q26841),
    ("5227", q5227),
    ("5549", q5549),
    ("7879", q7879),
    ("30581", q30581),
    ("669", q669),
    ("9722", q9722),
    ("7787", q7787),
    ("6088", q6088),
    ("32784", q32784),
    ("4863", q4863),
    ("6271", q6271),
    ("772", q772),
    ("7101", q7101),
    ("25221", q25221),
    ("28299", q28299),
    ("2605", q2605),
    ("2569", q2569),
    ("3124", q3124),
    ("4311", q4311),
    ("3439", q3439),
    ("25798", q25798),
    ("2635", q2635),
    ("14727", q14727),
    ("5483", q5483),
    ("6068", q6068),
    ("7013", q7013),
    ("29112", q29112),
    ("2811", q2811),
    ("635", q635),
    ("2584", q2584),
    ("6712", q6712),
    ("30218", q30218),
    ("9521", q9521),
    ("196", q196),
    ("8648", q8648),
    ("27712", q27712),
    ("1223", q1223),
    ("4090", q4090),
    ("5073", q5073),
    ("6051", q6051),
    ("30899", q30899),
    ("7132", q7132),
    ("8799", q8799),
    ("31797", q31797),
    ("2230", q2230),
    ("954", q954),
    ("7881", q7881),
    ("33524", q33524),
    ("3018", q3018),
    ("5877", q5877),
    ("8833", q8833),
];
