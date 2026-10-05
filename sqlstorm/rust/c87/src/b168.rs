use harness::prelude::*;
use std::cmp::Reverse;

fn segments(s: Str) -> Vec<Str> {
    s.split('<').skip(1).filter_map(|x| x.find('>').map(|j| &x[..j])).collect()
}

/// The (post, tag) pairs whose Tags contains '<' || TagName || '>': POSITION(..) > 0, or `LIKE '%<' || TagName || '>%'` when
/// `pat`. A name free of '<' and '>' (and, under LIKE, of '%' and '_') can only occur as a whole bracketed segment; any other
/// name is matched against every distinct Tags string.
fn bracketed(db: &'static So, pat: bool) -> MatSet<(Id<Post>, Id<Tag>)> {
    let name = &db.tag.tag_name;
    let plain = move |n: Str| !n.contains(['<', '>']) && !(pat && n.contains(['%', '_']));
    let seg: HashIdx<Str, Id<Tag>> = db.tag.with(name.filt(plain)).select(name).inv().collect();
    let odd: HashIdx<Str, Id<Tag>> = db.tag.with(name.filt(move |n| !plain(n))).select(name).inv().collect();
    let strs: MatSet<Str> = (&db.post.tags_str).collect();
    let hit: HashIdx<Str, Id<Tag>> = (&strs)
        .select_where(&odd, move |s: Str, n: Str| if pat { like(s, &format!("%<{n}>%")) } else { s.contains(&format!("<{n}>")) })
        .collect();
    let tags = &db.post.tags_str;
    db.post.select(Ident::<Post>::new().and(tags.flat_map(segments).select(&seg))).union(db.post.select(Ident::<Post>::new().and(tags.select(&hit)))).collect()
}

fn by_first<A: Copy + Eq + std::hash::Hash, B: Copy + Eq + std::hash::Hash>(m: &MatSet<(A, B)>) -> HashIdx<A, B> {
    m.map(|(a, _)| a).inv().map(|(_, b): (A, B)| b).collect()
}

// WITH PostTagCounts AS (SELECT P.Id AS PostId, COUNT(DISTINCT T.TagName) AS UniqueTagCount FROM Posts P LEFT JOIN Tags T ON POSITION(CONCAT('<', T.TagName, '>') IN P.Tags) > 0
//     WHERE P.PostTypeId = 1 GROUP BY P.Id),
// UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(COALESCE(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END, 0)) AS GoldBadgeCount,
//        SUM(COALESCE(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END, 0)) AS SilverBadgeCount, SUM(COALESCE(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END, 0)) AS BronzeBadgeCount,
//        COALESCE(SUM(P.Score), 0) AS TotalScore, AVG(TC.UniqueTagCount) AS AvgTagsPerQuestion
//     FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id LEFT JOIN Badges B ON B.UserId = U.Id LEFT JOIN PostTagCounts TC ON TC.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// MostActiveUsers AS (SELECT UserId, DisplayName, QuestionCount, GoldBadgeCount, SilverBadgeCount, BronzeBadgeCount, TotalScore, AvgTagsPerQuestion,
//        ROW_NUMBER() OVER (ORDER BY QuestionCount DESC) AS Rank FROM UserPostStats WHERE QuestionCount > 0)
// SELECT Rank, DisplayName, QuestionCount, GoldBadgeCount, SilverBadgeCount, BronzeBadgeCount, TotalScore, AvgTagsPerQuestion FROM MostActiveUsers WHERE Rank <= 10 ORDER BY TotalScore DESC;
//
// The tag match `POSITION('<' || TagName || '>' IN Tags) > 0` is the local bracketed() match.
// Rank reads only the distinct post count, so the ten users are picked first and the posts x badges product is driven for them alone.
fn q25105(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    type L = (Id<Post>, Id<Tag>);
    let bt = bracketed(db, false);
    let tags: HashIdx<Id<Post>, Id<Tag>> = (&bt).map(|(p, _): L| p).inv().map(|(_, t): L| t).collect();
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let w = whole(&pc).select(Ident::<User>::new().and(&pc)).window(row_number, |(u, n)| (Reverse(n), u), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let rank = by_first(&rk);
    let tu: MatSet<Id<User>> = (&rk).map(|(u, _)| u).collect();
    let tcd = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select((&tags).select(&db.tag.tag_name)).count_distinct();
    let tc = db.post.with(post_type_id.eq(1)).select((&tcd).opt().map(|n: Option<i64>| n.unwrap_or(0)));
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and((&tc).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, c)| {
            let (s, t) = p.map_or((0, None), |(s, t)| (s, t));
            [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + s, a[4] + t.is_some() as i64, a[5] + t.unwrap_or(0)]
        });
    let v = drain((&s).and(&pc).and(&rank));
    rows(v.into_iter().map(|(u, ((a, n), r))| row(vec![V::I(r), user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4])])))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty,
//        DENSE_RANK() OVER (PARTITION BY U.Id ORDER BY COALESCE(SUM(V.BountyAmount), 0) DESC) AS BountyRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopActiveUsers AS (SELECT UserId, DisplayName, AnswerCount, QuestionCount, TotalBounty, BountyRank FROM UserActivity WHERE BountyRank <= 10),
// PostSummary AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN P.Score > 0 THEN 1 END) AS PositiveQuestions,
//        COUNT(CASE WHEN P.ClosedDate IS NOT NULL THEN 1 END) AS ClosedQuestions FROM Posts P GROUP BY P.OwnerUserId)
// SELECT U.DisplayName, UA.AnswerCount, UA.QuestionCount, UA.TotalBounty, PS.TotalQuestions, PS.PositiveQuestions, PS.ClosedQuestions
// FROM TopActiveUsers UA JOIN Users U ON UA.UserId = U.Id LEFT JOIN PostSummary PS ON U.Id = PS.OwnerUserId WHERE UA.TotalBounty > 0 ORDER BY UA.TotalBounty DESC, UA.AnswerCount DESC;
//
// BountyRank is partitioned by the user it ranks, so it is 1 for every row.
fn q4852(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, closed_date, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([8, 9]))).select(bounty_amount.opt());
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(bounty.opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(closed_date.opt())).fold([0i64; 3], |a, ((t, s), c)| {
        [a[0] + (t == 1) as i64, a[1] + (s > 0) as i64, a[2] + c.is_some() as i64]
    });
    let v = drain((&ua).filt(|a| a[2] > 0).and((&ps).opt()));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(match p {
            Some(p) => p.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS Questions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS Answers, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScorePosts, SUM(b.Class) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, PositiveScorePosts, NegativeScorePosts, TotalBadges,
//        RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT u.UserId, u.DisplayName, u.Reputation, u.TotalPosts, u.Questions, u.Answers, u.PositiveScorePosts, u.NegativeScorePosts, u.TotalBadges,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 END), 0) AS TotalDownVotes
// FROM TopUsers u LEFT JOIN Votes v ON u.UserId = v.UserId
// GROUP BY u.UserId, u.DisplayName, u.Reputation, u.TotalPosts, u.Questions, u.Answers, u.PositiveScorePosts, u.NegativeScorePosts, u.TotalBadges, u.ReputationRank
// ORDER BY u.ReputationRank LIMIT 10;
//
// ReputationRank reads only Reputation, so the ten users are picked first and the posts x badges product is driven for them alone.
fn q5747(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (s, c)| [a[0] + (s.map_or(false, |s| s > 0)) as i64, a[1] + (s.map_or(false, |s| s < 0)) as i64, a[2] + c.is_some() as i64, a[3] + c.unwrap_or(0)]);
    let dist = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let vs = (&tu).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&s).and(&dist).and(&vs));
    rows(v.into_iter().map(|(u, ((a, d), w))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(d[0]), V::I(d[1]), V::I(d[2]), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(w[0]), V::I(w[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId IN (10, 11) THEN 1 ELSE 0 END) AS ClosedPosts,
//        AVG(EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - u.CreationDate))/86400) AS AccountAgeInDays
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, ClosedPosts, AccountAgeInDays, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.ClosedPosts, tu.AccountAgeInDays, pt.Name AS PostType, SUM(v.BountyAmount) AS TotalBounties
// FROM TopUsers tu JOIN Posts p ON tu.UserId = p.OwnerUserId JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
// WHERE tu.ReputationRank <= 10
// GROUP BY tu.UserId, tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.ClosedPosts, tu.AccountAgeInDays, pt.Name ORDER BY tu.Reputation DESC, TotalBounties DESC;
fn q8712(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select((&db.user.creation_date).and(posts_of(db).select(post_type_id).opt()))
        .fold((0i64, [0i64; 3], 0.0f64), |(n, a, s), (c, t)| {
            let t = t.unwrap_or(0);
            (n + 1, [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 10 | 11) as i64], s + secs(t0 - c) / 86400.0)
        });
    let pc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&pc).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let g = db
        .post
        .with(owner_user.select(&tu))
        .group_by(owner_user.and(ptype_name(db)))
        .select(bounty.opt())
        .fold([0i64; 2], |a, b| match b.flatten() {
            Some(b) => [a[0] + 1, a[1] + b],
            None => a,
        });
    let gv: MatSet<((Id<User>, Str), [i64; 2])> = whole(&g).select(Same::<(Id<User>, Str)>::new().and(&g)).collect();
    let v = drain((&gv).select(Same::<((Id<User>, Str), [i64; 2])>::new().and(Same::<((Id<User>, Str), [i64; 2])>::new().map(|((u, _), _)| u).select((&us).and(&pc)))));
    rows(v.into_iter().map(|(_, (((u, t), b), ((n, a, s), c)))| {
        row(vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(s / n as f64), V::S(t), nullable(b[1], b[0])])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE rn <= 5),
// PostMetrics AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COALESCE(SUM(CASE WHEN b.Class IS NOT NULL THEN b.Class ELSE 0 END), 0) AS TotalBadges,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM TopPosts tp LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) LEFT JOIN Votes v ON v.PostId = tp.PostId
//     GROUP BY tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName)
// SELECT pm.PostId, pm.Title, pm.Score, pm.ViewCount, pm.OwnerDisplayName, pm.TotalBadges, pm.UpVoteCount, pm.DownVoteCount FROM PostMetrics pm ORDER BY pm.Score DESC, pm.ViewCount DESC;
fn q6433(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(owner_user.select(badges_of(db).select(&db.badge.class)).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, u.DisplayName, u.Reputation),
// FilteredPosts AS (SELECT PostId, Title, Body, CreationDate, ViewCount, OwnerDisplayName, OwnerReputation, CommentCount, AnswerCount,
//        RANK() OVER (ORDER BY ViewCount DESC) AS ViewRank, RANK() OVER (ORDER BY AnswerCount DESC) AS AnswerRank FROM RankedPosts)
// SELECT fp.PostId, fp.Title, fp.Body, fp.ViewCount, fp.CommentCount, fp.AnswerCount, fp.OwnerDisplayName, fp.OwnerReputation,
//        CASE WHEN fp.ViewRank <= 10 THEN 'Top Viewed' WHEN fp.AnswerRank <= 10 THEN 'Top Answered' ELSE 'Other' END AS PostCategory
// FROM FilteredPosts fp WHERE fp.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//   AND (SELECT COUNT(*) FROM Votes v WHERE v.PostId = fp.PostId AND v.VoteTypeId = 2) >= 5
// ORDER BY fp.ViewCount DESC, fp.AnswerCount DESC;
fn q26238(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let ac = qs().group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let w = whole(qs()).select(Ident::<Post>::new().and(&ac).and(view_count.opt())).window(rank, |(_, w)| (w.is_none(), Reverse(w)), asc);
    let w = (&w).window(rank, |(((_, n), _), _): (((Id<Post>, i64), Option<i64>), i64)| Reverse(n), asc);
    let rk: MatSet<(Id<Post>, (i64, i64, i64))> = (&w).map(|((((p, n), _), vr), ar)| (p, (n, vr, ar))).collect();
    let by_post = by_first(&rk);
    let up = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)))).fold(0i64, |n, _| n + 1);
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain(qs().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with((&up).filt(|n| n >= 5)).select((&by_post).and(&cc)));
    rows(v.into_iter().map(|(p, ((n, vr, ar), c))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "views"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.push(V::S(if vr <= 10 { "Top Viewed" } else if ar <= 10 { "Top Answered" } else { "Other" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CreationDate, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostWithVotes AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CreationDate, tp.OwnerDisplayName,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM TopPosts tp LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CreationDate, tp.OwnerDisplayName)
// SELECT pwv.PostId, pwv.Title, pwv.Score, pwv.ViewCount, pwv.AnswerCount, pwv.CreationDate, pwv.OwnerDisplayName, pwv.UpVotes, pwv.DownVotes, (pwv.UpVotes - pwv.DownVotes) AS NetVotes
// FROM PostWithVotes pwv ORDER BY NetVotes DESC, pwv.Score DESC FETCH FIRST 10 ROWS ONLY;
fn q8987(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, a)| (Reverse(a[0] - a[1]), Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 /* Questions */),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation < 100 THEN 'Low Reputation' WHEN u.Reputation BETWEEN 100 AND 1000 THEN 'Medium Reputation'
//        ELSE 'High Reputation' END AS ReputationLevel FROM Users u),
// RecentVotes AS (SELECT v.PostId, COUNT(*) AS VoteCount FROM Votes v WHERE v.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY v.PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) /* Closed / Reopened */ GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, u.ReputationLevel, COALESCE(rv.VoteCount, 0) AS RecentVoteCount, COALESCE(cp.CloseCount, 0) AS CloseCount
// FROM RankedPosts rp LEFT JOIN UserReputation u ON rp.OwnerUserId = u.UserId LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE rp.Rank = 1 /* Get the most recent question for each user */ ORDER BY rp.ViewCount DESC FETCH FIRST 50 ROWS ONLY;
//
// The ownerless questions are one partition of their own (NULL OwnerUserId), whose newest question is kept too.
fn q3454(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, creation_date, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).in_v(vec![10, 11])));
    let rv = (&tp).group_by(Ident::<Post>::new()).select(recent.opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let cp = (&tp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let v = top_n(drain((&rv).and(&cp)), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, (r, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.push(match owner_user.get(p) {
            Some(u) => {
                let rep = db.user.reputation.get(u).unwrap();
                V::S(if rep < 100 { "Low Reputation" } else if rep <= 1000 { "Medium Reputation" } else { "High Reputation" })
            }
            None => V::Null,
        });
        f.extend([V::I(r), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.LastActivityDate, p.Score, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.LastActivityDate, rp.Score, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.PostRank = 1),
// TagCount AS (SELECT p.Id AS PostId, ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><'), 1) AS TagCount FROM Posts p WHERE p.Tags IS NOT NULL)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.LastActivityDate, fp.Score, fp.OwnerDisplayName, COALESCE(tc.TagCount, 0) AS TagCount, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
// FROM FilteredPosts fp LEFT JOIN Comments c ON fp.PostId = c.PostId LEFT JOIN Votes v ON fp.PostId = v.PostId LEFT JOIN TagCount tc ON fp.PostId = tc.PostId
// GROUP BY fp.PostId, fp.Title, fp.CreationDate, fp.LastActivityDate, fp.Score, fp.OwnerDisplayName, tc.TagCount ORDER BY UpvoteCount DESC, CommentCount DESC, fp.CreationDate DESC LIMIT 50;
fn q25018(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, tags_str, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, a)| (Reverse(a[1]), Reverse(a[0]), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity", "score", "owner"]);
        f.push(V::I(tags_str.get(p).map_or(0, |t| tag_list(t).count() as i64)));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount, COUNT(DISTINCT C.Id) AS CommentCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.OwnerUserId),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UBC.BadgeCount, 0) AS BadgeCount, COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(PS.TotalScore, 0) AS TotalScore,
//        COALESCE(PS.AvgViewCount, 0) AS AvgViewCount, COALESCE(PS.CommentCount, 0) AS CommentCount
//     FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId)
// SELECT U.UserId, U.DisplayName, U.BadgeCount, U.PostCount, U.TotalScore, U.AvgViewCount, U.CommentCount, RANK() OVER (ORDER BY U.TotalScore DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY U.BadgeCount DESC) AS BadgeRank
// FROM UserStats U WHERE U.PostCount > 0 ORDER BY U.TotalScore DESC, U.BadgeCount DESC LIMIT 10;
fn q25806(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db
        .post
        .group_by(owner_user)
        .select(score.and(view_count.opt()).and(comments_of(db).opt()))
        .fold([0i64; 5], |a, ((s, w), c)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c.is_some() as i64]);
    type X = ((Id<User>, i64), [i64; 5]);
    let w = whole((&ps).filt(|a| a[0] > 0)).select(Ident::<User>::new().and(&bc).and(&ps)).window(rank, |(_, a)| Reverse(a[1]), asc);
    let w = (&w).window(rank, |(((_, b), _), _): (X, i64)| Reverse(b), asc);
    let v = top_n(drain(&w), |&(_, ((((u, b), a), _), _))| (Reverse(a[1]), Reverse(b), u), 10);
    rows(v.into_iter().map(|(_, ((((u, b), a), sr), br))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), if a[2] == 0 { V::F(0.0) } else { avg(a[3], a[2]) }, V::I(a[4]), V::I(sr), V::I(br)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, ARRAY_LENGTH(string_to_array(p.Tags, '><'), 1) AS TagCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND (p.PostTypeId = 1 OR p.PostTypeId = 2)),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.Score, rp.TagCount, rp.RankByScore FROM RankedPosts rp WHERE rp.RankByScore <= 10),
// PostCommentCounts AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostVoteCounts AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Votes v GROUP BY v.PostId)
// SELECT tp.Title, tp.OwnerDisplayName, tp.Score, tp.TagCount, COALESCE(pcc.CommentCount, 0) AS TotalComments, COALESCE(pvc.Upvotes, 0) AS TotalUpvotes, COALESCE(pvc.Downvotes, 0) AS TotalDownvotes
// FROM TopPosts tp LEFT JOIN PostCommentCounts pcc ON tp.PostId = pcc.PostId LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId ORDER BY tp.Score DESC, tp.Title;
//
// Splitting the raw `<a><b>` on '><' gives one element per tag.
fn q28890(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, tags_str, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["title", "owner", "score"]);
        f.extend([harness::fmt::oint(tags_str.get(p).map(|t| t.split("><").count() as i64)), V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT b.Id) AS BadgeCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// RecentComments AS (SELECT c.Id AS CommentId, c.PostId, c.Text, c.CreationDate, c.UserDisplayName, RANK() OVER (PARTITION BY c.PostId ORDER BY c.CreationDate DESC) AS CommentRank
//     FROM Comments c WHERE c.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '7 days')
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ua.DisplayName AS TopUser, ua.BadgeCount, ua.TotalBounties, rc.CommentId, rc.Text AS RecentCommentText,
//        rc.CreationDate AS RecentCommentDate
// FROM RankedPosts rp LEFT JOIN UserActivity ua ON ua.BadgeCount = (SELECT MAX(BadgeCount) FROM UserActivity)
// LEFT JOIN RecentComments rc ON rc.PostId = rp.PostId AND rc.CommentRank = 1 WHERE rp.PostRank <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// The first ON names only ua and a scalar, so the ranked posts are crossed with the users holding the most badges; the badges x votes product is driven for those users alone.
fn q3435(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_days(t0, -30))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let most = (&bc).fold_flat(0i64, |m, n| m.max(n));
    let tops: MatSet<Id<User>> = db.user.with((&bc).filt(|n| n == most)).collect();
    let ua = (&tops)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold(0i64, |s, (_, b)| s + b.flatten().unwrap_or(0));
    let ua: HashIdx<(), (Id<User>, i64)> = whole(&ua).select(Ident::<User>::new().and(&ua)).collect();
    let Comment { post, creation_date: cd, .. } = &db.comment;
    let w = db.comment.with(cd.ge(add_days(t0, -7))).group_by(post).select(Ident::<Comment>::new().and(cd)).window(rank, |(_, d)| Reverse(d), asc);
    let last: HashIdx<Id<Post>, Id<Comment>> = (&w).filt(|(_, r)| r == 1).map(|((c, _), _)| c).collect();
    let v = drain((&tp).select((&last).opt().and(Ident::<Post>::new().map(|_| ()).select((&ua).opt()))));
    rows(v.into_iter().map(|(p, (c, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(match u {
            Some((u, b)) => [user_col(db, u, "name"), V::I(most), V::I(b)],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match c {
            Some(c) => [V::I(db.comment.origid.get(c).unwrap()), V::S(db.comment.text.get(c).unwrap()), V::T(cd.get(c).unwrap())],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') AND p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS TotalVotes FROM Votes v WHERE v.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months') GROUP BY v.PostId),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT rp.Title, rp.CreationDate, u.DisplayName AS OwnerDisplayName, COALESCE(rv.TotalVotes, 0) AS RecentVoteCount, ub.BadgeCount AS TotalBadges, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN RecentVotes rv ON rp.Id = rv.PostId LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// WHERE rp.UserPostRank <= 5 ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 100;
fn q3046(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1)).and(post_type_id.eq(1))).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_months(t0, -6))));
    let rv = (&tp).group_by(Ident::<Post>::new()).select(recent.opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain((&rv).and(owner_user.select((&ub).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (r, b))| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.push(V::I(r));
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
//        COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AvgScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 100 GROUP BY U.Id, U.DisplayName),
// PostEngagement AS (SELECT P.Id AS PostId, P.Title, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(C.Comments) AS CommentCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN (SELECT PostId, COUNT(*) AS Comments FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, P.Title),
// RankedPosts AS (SELECT PE.PostId, PE.Title, PE.UpVotes, PE.DownVotes, PE.CommentCount, RANK() OVER (ORDER BY PE.UpVotes DESC) AS VoteRank FROM PostEngagement PE)
// SELECT UPS.DisplayName, UPS.PostCount, UPS.QuestionCount, UPS.AnswerCount, UPS.TotalViews, UPS.AvgScore, RP.Title, RP.UpVotes, RP.DownVotes, RP.CommentCount, RP.VoteRank
// FROM UserPostStats UPS LEFT JOIN RankedPosts RP ON UPS.UserId = RP.PostId WHERE UPS.PostCount > 10 ORDER BY UPS.AvgScore DESC, RP.VoteRank ASC LIMIT 10;
//
// `UPS.UserId = RP.PostId` joins a user id to a post id, so it goes through the raw ids. AvgScore is ordered as an exact fraction.
#[derive(PartialEq, Eq, Clone, Copy)]
struct Frac(i64, i64);
impl PartialOrd for Frac {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Frac {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        (self.0 as i128 * o.1 as i128).cmp(&(o.0 as i128 * self.1 as i128))
    }
}

fn q1061(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let ups = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s],
            None => a,
        });
    let pe = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(Ident::<Post>::new().with(comments_of(db)).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let w = whole(&pe).select(Ident::<Post>::new().and(&pe)).window(rank, |(_, a)| Reverse(a[0]), asc);
    type E = (Id<Post>, ([i64; 3], i64));
    let rk: MatSet<E> = (&w).map(|((p, a), r)| (p, (a, r))).collect();
    let by_id: HashIdx<i64, E> = (&rk).map(|(p, _): E| p).select(&db.post.origid).inv().collect();
    let v = drain((&ups).filt(|a| a[0] > 10).and((&db.user.origid).select(&by_id).opt()));
    let v = top_n(v, |&(u, (a, r))| (Reverse(Frac(a[5], a[0])), r.is_none(), r.map(|(_, (_, k))| k), u), 10);
    rows(v.into_iter().map(|(u, (a, r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), avg(a[5], a[0])];
        f.extend(match r {
            Some((p, (e, k))) => vec![title(db, p), V::I(e[0]), V::I(e[1]), V::I(e[2]), V::I(k)],
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY EXTRACT(YEAR FROM p.CreationDate) ORDER BY p.ViewCount DESC) AS YearlyRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '5 years'),
// TopPosts AS (SELECT * FROM RankedPosts WHERE YearlyRank <= 5),
// CommentStats AS (SELECT PostId, COUNT(*) AS TotalComments, AVG(Score) AS AvgCommentScore FROM Comments GROUP BY PostId),
// PostVotes AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId)
// SELECT tp.Id, tp.Title, tp.CreationDate, COALESCE(cs.TotalComments, 0) AS TotalComments, COALESCE(cs.AvgCommentScore, 0) AS AvgCommentScore, COALESCE(v.Upvotes, 0) AS Upvotes,
//        COALESCE(v.Downvotes, 0) AS Downvotes,
//        CASE WHEN COALESCE(v.Upvotes, 0) + COALESCE(v.Downvotes, 0) > 0 THEN (COALESCE(v.Upvotes, 0) * 1.0 / (COALESCE(v.Upvotes, 0) + COALESCE(v.Downvotes, 0)) * 100) ELSE NULL END AS ApprovalRating
// FROM TopPosts tp LEFT JOIN CommentStats cs ON tp.Id = cs.PostId LEFT JOIN PostVotes v ON tp.Id = v.PostId ORDER BY tp.CreationDate DESC;
fn q3847(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(date(2024, 10, 1), -5))))
        .with(owner_user)
        .group_by(creation_date.map(year))
        .select(Ident::<Post>::new().and(view_count.opt()))
        .window(row_number, |(p, w)| (w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cs).and(&pv)).into_iter().map(|(p, (c, v))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c[0]), if c[0] == 0 { V::F(0.0) } else { avg(c[1], c[0]) }, V::I(v[0]), V::I(v[1])]);
        f.push(if v[0] + v[1] > 0 { V::F(v[0] as f64 * 1.0 / (v[0] + v[1]) as f64 * 100.0) } else { V::Null });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.ScoreRank <= 5),
// PostVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerDisplayName, COALESCE(pv.UpVotes, 0) AS UpVotes, COALESCE(pv.DownVotes, 0) AS DownVotes,
//        CASE WHEN tp.Score >= 0 THEN 'Positive' WHEN tp.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreCategory FROM TopPosts tp LEFT JOIN PostVotes pv ON tp.PostId = pv.PostId)
// SELECT pd.Title, pd.CreationDate, pd.OwnerDisplayName, pd.UpVotes, pd.DownVotes, pd.ScoreCategory,
//        'This post has ' || COALESCE(pd.UpVotes, 0) || ' upvotes and ' || COALESCE(pd.DownVotes, 0) || ' downvotes' AS VoteDescription
// FROM PostDetails pd WHERE pd.UpVotes > pd.DownVotes ORDER BY pd.CreationDate DESC LIMIT 10;
fn q3902(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain((&pv).filt(|a| a[0] > a[1])), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if score.get(p).unwrap() >= 0 { "Positive" } else { "Negative" })]);
        f.push(V::Owned(format!("This post has {} upvotes and {} downvotes", a[0], a[1])));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, LastPostDate, RANK() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserActivity),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days')
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, COALESCE(rp.Title, 'No Recent Posts') AS RecentPostTitle, COALESCE(rp.ViewCount, 0) AS RecentPostViews,
//        COALESCE(rp.Score, 0) AS RecentPostScore, tu.LastPostDate,
//        CASE WHEN tu.LastPostDate < cast('2024-10-01' as date) - INTERVAL '1 year' THEN 'Inactive' ELSE 'Active' END AS ActivityStatus
// FROM TopUsers tu LEFT JOIN RecentPosts rp ON tu.UserId = rp.OwnerUserId AND rp.rn = 1 WHERE tu.Rank <= 10 ORDER BY tu.TotalPosts DESC;
fn q1095(db: &'static So) -> String {
    let Post { owner_user, creation_date, title, view_count, score, .. } = &db.post;
    let ups = user_posts(db);
    let w = whole(&ups).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| Reverse(a[1]), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let w = db.post.with(creation_date.gt(add_days(date(2024, 10, 1), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let last: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let v = drain((&tu).select((&ups).and((&last).opt())));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3])];
        f.push(V::S(p.and_then(|p| title.get(p)).unwrap_or("No Recent Posts")));
        f.extend([V::I(p.and_then(|p| view_count.get(p)).unwrap_or(0)), V::I(p.map_or(0, |p| score.get(p).unwrap())), tmax(a[7])]);
        f.push(V::S(if a[7] != i64::MIN && a[7] < add_years(date(2024, 10, 1), -1) { "Inactive" } else { "Active" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT ub.UserId, COUNT(CASE WHEN ub.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN ub.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN ub.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges ub GROUP BY ub.UserId),
// RecentPosts AS (SELECT p.OwnerUserId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostMetrics AS (SELECT rp.OwnerUserId, COUNT(*) AS RecentPostCount, SUM(rp.ViewCount) AS TotalViews FROM RecentPosts rp WHERE rp.rn <= 5 GROUP BY rp.OwnerUserId),
// UserScores AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(pm.RecentPostCount, 0) AS RecentPostCount, COALESCE(pm.TotalViews, 0) AS TotalViews
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostMetrics pm ON u.Id = pm.OwnerUserId)
// SELECT us.UserId, us.Reputation, us.GoldBadges, us.SilverBadges, us.BronzeBadges, us.RecentPostCount, us.TotalViews, RANK() OVER (ORDER BY us.Reputation DESC) AS ReputationRank
// FROM UserScores us WHERE us.Reputation > 1000 AND us.RecentPostCount > 0 ORDER BY us.TotalViews DESC FETCH FIRST 10 ROWS ONLY;
fn q2817(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let pm = (&rp).group_by(owner_user).select(view_count.opt()).fold([0i64; 2], |a, w| [a[0] + 1, a[1] + w.unwrap_or(0)]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let w = whole(db.user.with((&db.user.reputation).gt(1000)).with(&pm))
        .select(Ident::<User>::new().and(&db.user.reputation).and(&ub).and(&pm))
        .window(rank, |(((_, r), _), _)| Reverse(r), asc);
    let v = top_n(drain(&w), |&(_, ((((u, _), _), m), _))| (Reverse(m[1]), u), 10);
    rows(v.into_iter().map(|(_, ((((u, _), b), m), r))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(m[0]), V::I(m[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, TotalViews, QuestionCount, AnswerCount, DENSE_RANK() OVER (ORDER BY TotalScore DESC) AS RankByScore FROM UserPostStats),
// TopQuestions AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerName, COUNT(v.Id) AS VoteCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName ORDER BY VoteCount DESC LIMIT 10)
// SELECT tu.DisplayName AS TopUser, tu.TotalScore, tu.TotalViews, tq.Title AS TopQuestionTitle, tq.ViewCount AS QuestionViews, tq.VoteCount AS QuestionVoteCount, tq.CreationDate
// FROM TopUsers tu JOIN TopQuestions tq ON tq.OwnerName = tu.DisplayName WHERE tu.RankByScore <= 5 ORDER BY tu.TotalScore DESC, tq.VoteCount DESC;
fn q9791(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let ups = user_posts(db);
    let w = whole(&ups).select(Ident::<User>::new().and(&ups)).window(dense_rank, |(_, a)| Reverse(a[4]), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 5).map(|((u, _), _)| u).collect();
    let tq = db.post.with(post_type_id.eq(1)).with(owner_user).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let tq = top_n(drain(&tq), |&(p, n)| (Reverse(n), p), 10);
    let tq = rel(tq);
    let by_name: HashIdx<Str, (Id<Post>, i64)> = (&tq).select(Same::<(Id<Post>, i64)>::new().map(|(p, _): (Id<Post>, i64)| p).select(owner_user.select(&db.user.display_name))).map(|n| n).inv().select(&tq).collect();
    let v = drain((&tu).select((&ups).and((&db.user.display_name).select(&by_name))));
    rows(v.into_iter().map(|(u, (a, (p, n)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[4]), V::I(a[6])];
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(n)]);
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
}

// WITH RecursivePostCTE AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COALESCE(pv.VoteCount, 0) AS VoteCount, COALESCE(c.CommentCount, 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) pv ON p.Id = pv.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerUserId, rp.CreationDate, rp.VoteCount, rp.CommentCount, u.DisplayName AS OwnerDisplayName, u.Reputation
//     FROM RecursivePostCTE rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE u.Reputation >= 1000 AND rp.CommentCount > 5 AND rp.rn <= 3),
// RankedPosts AS (SELECT fp.*, RANK() OVER (ORDER BY fp.VoteCount DESC, fp.CreationDate DESC) AS PostRank FROM FilteredPosts fp)
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.VoteCount, rp.CommentCount, rp.Reputation, CASE WHEN rp.VoteCount IS NULL THEN 'No Votes' ELSE 'Has Votes' END AS VoteStatus
// FROM RankedPosts rp WHERE rp.PostRank <= 10 ORDER BY rp.VoteCount DESC;
//
// Not recursive: the CTE is only named that way.
fn q32583(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let w = db.post.group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let rich = owner_user.select(Ident::<User>::new().with((&db.user.reputation).ge(1000)));
    let w = whole((&tp).with(rich).with((&cc).filt(|n| n > 5)))
        .select(Ident::<Post>::new().and(&cc).and(&vc).and(creation_date))
        .window(rank, |(((_, _), n), d)| (Reverse(n), Reverse(d)), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((((p, c), n), _), _))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(n), V::I(c)]);
        f.extend(post_fields(db, p, &["rep"]));
        f.push(V::S("Has Votes"));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName AS OwnerName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.Score, rp.ViewCount, rp.OwnerName FROM RankedPosts rp WHERE rp.PostRank <= 5),
// PostVoteSummary AS (SELECT p.Id, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// CommentStats AS (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId)
// SELECT tp.Id, tp.Title, tp.Score, tp.ViewCount, tp.OwnerName, pvs.UpVotes, pvs.DownVotes, pvs.TotalBounty, COALESCE(cs.CommentCount, 0) AS CommentCount,
//        CASE WHEN pvs.UpVotes > pvs.DownVotes THEN 'Positive' WHEN pvs.UpVotes < pvs.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM TopPosts tp LEFT JOIN PostVoteSummary pvs ON tp.Id = pvs.Id LEFT JOIN CommentStats cs ON tp.Id = cs.PostId ORDER BY tp.Score DESC;
fn q1007(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).fold([0i64; 3], |a, x| match x {
        Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + b.unwrap_or(0)],
        None => a,
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&pv).and(&cc)).into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c)]);
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, MAX(B.Class) AS HighestBadgeClass FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostMetrics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT U.Id, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PM.PostCount, 0) AS PostCount, COALESCE(PM.TotalScore, 0) AS TotalScore,
//        COALESCE(PM.AvgViewCount, 0) AS AvgViewCount, RANK() OVER (ORDER BY COALESCE(PM.TotalScore, 0) DESC, COALESCE(UB.BadgeCount, 0) DESC) AS UserRank
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostMetrics PM ON U.Id = PM.OwnerUserId)
// SELECT U.DisplayName, U.BadgeCount, U.PostCount, U.TotalScore, U.AvgViewCount, CASE WHEN U.BadgeCount > 10 THEN 'Expert' WHEN U.BadgeCount > 5 THEN 'Intermediate' ELSE 'Beginner' END AS UserLevel,
//        CASE WHEN U.TotalScore > 1000 THEN 'High Engagement' WHEN U.TotalScore BETWEEN 500 AND 1000 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM TopUsers U WHERE U.UserRank <= 50 ORDER BY U.UserRank ASC;
fn q1753(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pm = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&bc).and((&pm).opt())).window(rank, |((_, b), m)| (Reverse(m.map_or(0, |m| m[1])), Reverse(b)), asc);
    rows(drain((&w).filt(|(_, r)| r <= 50)).into_iter().map(|(_, (((u, b), m), _))| {
        let m = m.unwrap_or([0; 4]);
        row(vec![
            user_col(db, u, "name"),
            V::I(b),
            V::I(m[0]),
            V::I(m[1]),
            if m[2] == 0 { V::F(0.0) } else { avg(m[3], m[2]) },
            V::S(if b > 10 { "Expert" } else if b > 5 { "Intermediate" } else { "Beginner" }),
            V::S(if m[1] > 1000 { "High Engagement" } else if m[1] >= 500 { "Moderate Engagement" } else { "Low Engagement" }),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostDetails AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, us.DisplayName, COALESCE(pc.CommentCount, 0) AS TotalComments
//     FROM RankedPosts rp JOIN Users u ON u.Id = rp.OwnerUserId JOIN UserStats us ON us.UserId = u.Id LEFT JOIN PostComments pc ON pc.PostId = rp.Id WHERE rp.PostRank = 1)
// SELECT pd.Title, pd.CreationDate, pd.Score, pd.DisplayName, pd.TotalComments, CASE WHEN pd.Score > 100 THEN 'High Score' ELSE 'Standard Score' END AS ScoreCategory
// FROM PostDetails pd ORDER BY pd.Score DESC LIMIT 10 OFFSET 0;
fn q151(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain(&cc), |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, c)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "owner"]);
        f.extend([V::I(c), V::S(if score.get(p).unwrap() > 100 { "High Score" } else { "Standard Score" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(c.Id) DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > cast('2024-10-01' as date) - interval '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.*, CASE WHEN rp.UpVoteCount + rp.DownVoteCount = 0 THEN NULL ELSE ROUND((CAST(rp.UpVoteCount AS DECIMAL) / (rp.UpVoteCount + rp.DownVoteCount)) * 100, 2) END AS UpvotePercentage
//     FROM RankedPosts rp WHERE rp.rn = 1)
// SELECT fp.OwnerUserId, u.DisplayName, fp.Title, fp.CreationDate, fp.CommentCount, fp.UpVoteCount, fp.DownVoteCount, fp.UpvotePercentage,
//        CASE WHEN fp.UpvotePercentage IS NULL THEN 'No Votes' WHEN fp.UpvotePercentage > 75 THEN 'High Engagement' WHEN fp.UpvotePercentage BETWEEN 50 AND 75 THEN 'Moderate Engagement'
//        ELSE 'Low Engagement' END AS EngagementLevel
// FROM FilteredPosts fp JOIN Users u ON fp.OwnerUserId = u.Id WHERE u.Reputation > 1000 ORDER BY fp.CommentCount DESC, fp.UpVoteCount DESC FETCH FIRST 10 ROWS ONLY;
fn q1173(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.gt(add_years(date(2024, 10, 1), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = db
        .post
        .with(creation_date.gt(add_years(date(2024, 10, 1), -1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(&rp))
        .window(row_number, |(p, a)| (Reverse(a[0]), p), asc);
    let first: HashIdx<Id<User>, (Id<Post>, [i64; 3])> = (&w).filt(|(_, r)| r == 1).map(|(x, _)| x).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select(&first));
    let v = top_n(v, |&(_, (p, a))| (Reverse(a[0]), Reverse(a[1]), p), 10);
    rows(v.into_iter().map(|(u, (p, a))| {
        let pct = if a[1] + a[2] == 0 { None } else { Some((a[1] as f64 / (a[1] + a[2]) as f64 * 100.0 * 100.0).round() / 100.0) };
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), harness::fmt::ofloat(pct)]);
        f.push(V::S(match pct {
            None => "No Votes",
            Some(x) if x > 75.0 => "High Engagement",
            Some(x) if x >= 50.0 => "Moderate Engagement",
            _ => "Low Engagement",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, OwnerDisplayName, CommentCount FROM RankedPosts WHERE rn = 1),
// PostVoteCounts AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.OwnerDisplayName, tp.CommentCount, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes,
//        CASE WHEN tp.Score > 10 THEN 'Highly Active' WHEN tp.Score BETWEEN 5 AND 10 THEN 'Moderately Active' ELSE 'Less Active' END AS ActivityLevel,
//        CASE WHEN tp.CommentCount = 0 THEN 'No Comments' ELSE 'Comments Available' END AS CommentStatus
// FROM TopPosts tp LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId ORDER BY tp.CreationDate DESC LIMIT 100;
fn q806(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(current_date(), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain((&cc).and(&pv)), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (c, a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if s > 10 { "Highly Active" } else if s >= 5 { "Moderately Active" } else { "Less Active" }));
        f.push(V::S(if c == 0 { "No Comments" } else { "Comments Available" }));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        COUNT(DISTINCT p.Id) AS PostsCount, COUNT(DISTINCT b.Id) AS BadgesCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(a.OwnerDisplayName, 'Community User') AS OwnerDisplayName,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.OwnerDisplayName, pd.CommentCount FROM PostDetails pd WHERE pd.RN <= 5)
// SELECT us.DisplayName AS UserDisplayName, us.Upvotes, us.Downvotes, COUNT(DISTINCT fp.PostId) AS TotalPosts, SUM(fp.ViewCount) AS TotalViews, MAX(fp.CreationDate) AS LastActivePostDate
// FROM UserStatistics us LEFT JOIN FilteredPosts fp ON us.DisplayName = fp.OwnerDisplayName GROUP BY us.UserId, us.DisplayName, us.Upvotes, us.Downvotes
// ORDER BY TotalPosts DESC, UserDisplayName;
//
// `a.OwnerDisplayName` is the accepted answer's Posts.OwnerDisplayName column, not its owner's Users.DisplayName.
fn q2194(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, accepted_answer, owner_display_name, view_count, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt()).and(badges_of(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let shown = |n: Option<Option<Str>>| -> Str { n.flatten().unwrap_or("Community User") };
    let by_name: HashIdx<Str, Id<Post>> = (&fp).select(accepted_answer.select(owner_display_name.opt()).opt().map(shown)).inv().collect();
    let g = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.display_name).select((&by_name).select(view_count.opt().and(creation_date))).opt())
        .fold([0i64, 0, 0, i64::MIN], |a, x| match x {
            Some((w, d)) => [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3].max(d)],
            None => a,
        });
    rows(drain((&us).and(&g)).into_iter().map(|(u, (a, g))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(g[0]), nullable(g[2], g[1]), tmax(g[3])])))
}

// Rewritten (rewrites/2980.sql): the final ORDER BY is tie-broken on FPS.PostId.
// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerName,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerName FROM RankedPosts WHERE PostRank = 1),
// PostInteractionStats AS (SELECT P.Id AS PostId, COALESCE(COUNT(V.Id), 0) AS VoteCount, COALESCE(COUNT(C.Id), 0) AS CommentCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id),
// FinalPostStats AS (SELECT TRP.PostId, TRP.Title, TRP.CreationDate, TRP.Score + PIS.VoteCount AS TotalScore, PIS.CommentCount
//     FROM TopRankedPosts TRP JOIN PostInteractionStats PIS ON TRP.PostId = PIS.PostId)
// SELECT FPS.PostId, FPS.Title, FPS.CreationDate, FPS.TotalScore, FPS.CommentCount,
//        CASE WHEN FPS.TotalScore > 100 THEN 'Highly Engaging' WHEN FPS.TotalScore BETWEEN 50 AND 100 THEN 'Moderately Engaging' ELSE 'Less Engaging' END AS EngagementLevel
// FROM FinalPostStats FPS ORDER BY FPS.TotalScore DESC, FPS.PostId LIMIT 10;
fn q2980(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let pis = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt().and(comments_of(db).opt())).fold([0i64; 2], |a, (v, c)| [a[0] + v.is_some() as i64, a[1] + c.is_some() as i64]);
    let v = top_n(drain(&pis), |&(p, a)| (Reverse(score.get(p).unwrap() + a[0]), origid.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, a)| {
        let t = score.get(p).unwrap() + a[0];
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(t), V::I(a[1]), V::S(if t > 100 { "Highly Engaging" } else if t >= 50 { "Moderately Engaging" } else { "Less Engaging" })]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COUNT(DISTINCT C.Id) AS TotalComments, SUM(V.BountyAmount) AS TotalBounties
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalComments, TotalBounties, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserStatistics),
// ActiveUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalComments, TotalBounties FROM TopUsers WHERE UserRank <= 10)
// SELECT AU.DisplayName, AU.Reputation, AU.TotalPosts, AU.TotalQuestions, AU.TotalAnswers, AU.TotalComments, AU.TotalBounties, COALESCE(B.BadgeCount, 0) AS BadgeCount
// FROM ActiveUsers AU LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON AU.UserId = B.UserId ORDER BY AU.Reputation DESC;
//
// UserRank reads only Reputation, so the ten users are picked first and the posts x comments x own-votes product is driven for them alone.
fn q7164(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let ov = own_votes(db);
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(comments_of(db).opt()).and((&ov).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some(((t, _), b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
            }
            None => a,
        });
    let dp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let dc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&s).and(&dp).and(&dc).and(&bc)).into_iter().map(|(u, (((a, p), c), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1]), V::I(c), nullable(a[3], a[2]), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, Score, ViewCount FROM RankedPosts WHERE Rank <= 10),
// PostStats AS (SELECT tp.Title, tp.OwnerDisplayName, tp.Score, tp.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(COUNT(c.Id), 0) AS CommentCount
//     FROM TopPosts tp LEFT JOIN Votes v ON tp.PostId = v.PostId LEFT JOIN Comments c ON tp.PostId = c.PostId GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.Score, tp.ViewCount),
// FinalOutput AS (SELECT fs.*, (UpVotes - DownVotes) AS NetVotes,
//        CASE WHEN ViewCount > 1000 THEN 'Popular' WHEN ViewCount BETWEEN 501 AND 1000 THEN 'Moderately Popular' ELSE 'Less Popular' END AS PopularityStatus FROM PostStats fs)
// SELECT * FROM FinalOutput ORDER BY Score DESC, ViewCount DESC;
fn q8484(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[0] - a[1])]);
        f.push(V::S(match w {
            Some(w) if w > 1000 => "Popular",
            Some(w) if w >= 501 => "Moderately Popular",
            _ => "Less Popular",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, MAX(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS HasDownVote
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// RecentBadges AS (SELECT b.UserId, b.Name AS BadgeName, ROW_NUMBER() OVER (PARTITION BY b.UserId ORDER BY b.Date DESC) AS BadgeRank FROM Badges b
//     WHERE b.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT up.DisplayName, up.Reputation, rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVoteCount, CASE WHEN rp.HasDownVote = 1 THEN 'Yes' ELSE 'No' END AS HasDownVote, rb.BadgeName
// FROM Users up LEFT JOIN RankedPosts rp ON up.Id = rp.OwnerUserId AND rp.OwnerPostRank = 1 LEFT JOIN RecentBadges rb ON up.Id = rb.UserId AND rb.BadgeRank = 1
// WHERE up.Reputation > (SELECT AVG(Reputation) FROM Users) AND (up.Location IS NOT NULL AND up.Location LIKE '%USA%') ORDER BY up.Reputation DESC, rp.UpVoteCount DESC LIMIT 10;
//
// OwnerPostRank reads only CreationDate, so each owner's newest post is picked first and the comment x vote product is driven for those alone.
fn q23633(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let newest: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2].max((t == Some(3)) as i64)]);
    let Badge { user, date: bd, .. } = &db.badge;
    let w = db.badge.with(bd.ge(add_years(t0, -1))).group_by(user).select(Ident::<Badge>::new().and(bd)).window(row_number, |(b, d)| (Reverse(d), b), asc);
    let last: HashIdx<Id<User>, Id<Badge>> = (&w).filt(|(_, r)| r == 1).map(|((b, _), _)| b).collect();
    let (n, sum) = db.user.select(&db.user.reputation).fold_flat((0i128, 0i128), |(n, s), r| (n + 1, s + r as i128));
    let users = db
        .user
        .with((&db.user.reputation).filt(|r| r as i128 * n > sum))
        .with((&db.user.location).filt(|l: Str| l.contains("USA")));
    let v = drain(users.select((&newest).select(Ident::<Post>::new().and(&rp)).opt().and((&last).opt())));
    let v = top_n(v, |&(u, (p, _))| (Reverse(db.user.reputation.get(u).unwrap()), p.is_none(), Reverse(p.map(|x| x.1[1])), u), 10);
    rows(v.into_iter().map(|(u, (p, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match p {
            Some((p, a)) => {
                let mut g = post_fields(db, p, &["id", "title", "created"]);
                g.extend([V::I(a[0]), V::I(a[1]), V::S(if a[2] == 1 { "Yes" } else { "No" })]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null, V::S("No")],
        });
        f.push(b.map_or(V::Null, |b| V::S(db.badge.name.get(b).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= '2023-01-01' AND p.CreationDate < '2024-01-01' AND p.Score > 10),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, (SELECT COUNT(*) FROM Posts pp WHERE pp.OwnerUserId = u.Id) AS PostCount,
//        (SELECT COUNT(*) FROM Badges b WHERE b.UserId = u.Id AND b.Class = 1) AS GoldBadges FROM Users u WHERE u.Reputation > 1000),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT pp.PostId, pp.Title, pp.CreationDate, pp.Score, u.Reputation AS UserReputation, COALESCE(rc.CommentCount, 0) AS TotalComments, rc.LastCommentDate,
//        CASE WHEN pp.PostRank = 1 THEN 'Latest' ELSE 'Earlier' END AS PostStatus, CASE WHEN u.GoldBadges > 0 THEN 'Gold' ELSE 'Regular' END AS UserType
// FROM RankedPosts pp JOIN UserReputation u ON pp.OwnerUserId = u.UserId LEFT JOIN RecentComments rc ON pp.PostId = rc.PostId
// WHERE pp.Score > (SELECT AVG(Score) FROM Posts WHERE Score > 0) AND pp.PostId IN (SELECT DISTINCT PostId FROM Votes v WHERE v.VoteTypeId IN (2, 3))
// ORDER BY pp.CreationDate DESC LIMIT 100;
fn q22495(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(date(2023, 1, 1)).and(creation_date.lt(date(2024, 1, 1))).and(score.gt(10)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let rank = by_first(&rk);
    let (n, sum) = db.post.with(score.gt(0)).select(score).fold_flat((0i128, 0i128), |(n, s), x| (n + 1, s + x as i128));
    let voted: MatSet<Id<Post>> = db.vote.with((&db.vote.vote_type_id).is_in([2, 3])).select(&db.vote.post).collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let ur = Ident::<User>::new().with((&db.user.reputation).gt(1000)).and(Ident::<User>::new().with(gold).opt());
    let rc = comments_of(db).select(&db.comment.creation_date).opt();
    let base = db.post.with(score.filt(|s| s as i128 * n > sum)).with(&voted);
    let g = base.group_by(Ident::<Post>::new()).select(rc).fold((0i64, i64::MIN), |(k, m), d| match d {
        Some(d) => (k + 1, m.max(d)),
        None => (k, m),
    });
    let v = drain((&g).and(&rank).and(owner_user.select(ur)));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((c, r), (u, gd)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([user_col(db, u, "rep"), V::I(c.0), tmax(c.1), V::S(if r == 1 { "Latest" } else { "Earlier" }), V::S(if gd.is_some() { "Gold" } else { "Regular" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, CASE WHEN Reputation > 1000 THEN 'High' WHEN Reputation > 100 THEN 'Medium' ELSE 'Low' END AS ReputationLevel FROM Users),
// RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// PostWithComments AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, rp.OwnerUserId
//     FROM RecentPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON rp.PostId = c.PostId),
// PostSummary AS (SELECT wp.PostId, wp.Title, wp.CreationDate, wp.Score, wp.ViewCount, wp.CommentCount, ur.ReputationLevel, wp.OwnerUserId
//     FROM PostWithComments wp JOIN UserReputation ur ON wp.OwnerUserId = ur.Id WHERE wp.Score >= 5)
// SELECT ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.CommentCount, CONCAT(ur.DisplayName, ' - ', ps.ReputationLevel) AS UserInfo
// FROM PostSummary ps JOIN UserReputation ur ON ps.OwnerUserId = ur.Id WHERE ps.CommentCount > 5 ORDER BY ps.Score DESC, ps.ViewCount ASC LIMIT 10;
fn q690(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let cc = db
        .post
        .with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(score.ge(5)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db))
        .fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&cc).filt(|n| n > 5)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), w, p)
    }, 10);
    rows(v.into_iter().map(|(p, c)| {
        let u = owner_user.get(p).unwrap();
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(c), V::Owned(format!("{} - {}", db.user.display_name.get(u).unwrap(), if rep > 1000 { "High" } else if rep > 100 { "Medium" } else { "Low" }))]);
        row(f)
    }))
}

// Rewritten (rewrites/3332.sql): the ReputationRank window is tie-broken on u.Id and the final ORDER BY on pr.PostId, bg.Id.
// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC, u.Id) AS ReputationRank FROM Users u),
// PostRanked AS (SELECT rp.*, ur.DisplayName AS OwnerDisplayName, ur.Reputation AS OwnerReputation, ur.ReputationRank FROM RecentPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId)
// SELECT pr.PostId, pr.Title, pr.CreationDate, pr.ViewCount, pr.CommentCount, pr.UpVotes, pr.DownVotes, pr.OwnerDisplayName, pr.OwnerReputation, pr.ReputationRank,
//        COALESCE(bg.Id, 0) AS BadgeId, COALESCE(bg.Name, 'No Badge') AS BadgeName
// FROM PostRanked pr LEFT JOIN Badges bg ON pr.OwnerUserId = bg.UserId AND bg.Class = 1 WHERE pr.ViewCount > (SELECT AVG(ViewCount) FROM RecentPosts)
// ORDER BY pr.ReputationRank, pr.ViewCount DESC, pr.PostId, bg.Id LIMIT 10;
fn q3332(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, origid, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let (n, sum) = recent().select(view_count).fold_flat((0i128, 0i128), |(n, s), w| (n + 1, s + w as i128));
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation).and(&db.user.origid)).window(row_number, |((_, r), o)| (Reverse(r), o), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).map(|(((u, _), _), r)| (u, r)).collect();
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rk).map(|(u, _)| u).inv().collect();
    let rp = recent()
        .with(view_count.filt(|w| w as i128 * n > sum))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let v = drain((&rp).and(owner_user.select((&rank).and(gold.opt()))));
    let v = top_n(v, |&(p, (_, ((_, r), b)))| (r, Reverse(view_count.get(p).unwrap()), origid.get(p).unwrap(), b.is_none(), b.map(|b| db.badge.origid.get(b).unwrap())), 10);
    rows(v.into_iter().map(|(p, (a, ((u, r), b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(r));
        f.extend(match b {
            Some(b) => [V::I(db.badge.origid.get(b).unwrap()), V::S(db.badge.name.get(b).unwrap())],
            None => [V::I(0), V::S("No Badge")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score)
// SELECT pd.Title, pd.OwnerDisplayName, pd.CreationDate, pd.Score, pd.CommentCount, pd.UpVotes, pd.DownVotes,
//        CASE WHEN pd.UpVotes - pd.DownVotes > 10 THEN 'Popular' WHEN pd.Score > 10 THEN 'High-Score' ELSE 'Regular' END AS Classification
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.CommentCount DESC LIMIT 100;
fn q6664(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, a)| (Reverse(score.get(p).unwrap()), Reverse(a[0]), p), 100);
    rows(v.into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "owner", "created", "score"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] - a[2] > 10 { "Popular" } else if s > 10 { "High-Score" } else { "Regular" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.PostTypeId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CreationDate, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CreationDate, tp.OwnerDisplayName)
// SELECT pd.PostId, pd.Title, pd.Score, pd.ViewCount, pd.CreationDate, pd.OwnerDisplayName, pd.CommentCount, pd.UpVotes, pd.DownVotes,
//        CASE WHEN pd.Score > 100 THEN 'High Performer' WHEN pd.Score BETWEEN 50 AND 100 THEN 'Medium Performer' ELSE 'Low Performer' END AS PerformanceCategory
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.CreationDate DESC;
fn q9038(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.gt(add_days(date(2024, 10, 1), -30)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(if s > 100 { "High Performer" } else if s >= 50 { "Medium Performer" } else { "Low Performer" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, UpVotes, DownVotes, CommentCount, RANK() OVER (ORDER BY PostCount DESC) AS PostRank,
//        RANK() OVER (ORDER BY UpVotes DESC) AS UpVotesRank FROM UserActivity),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT tu.DisplayName, tu.PostCount, tu.AnswerCount, tu.QuestionCount, tu.UpVotes, tu.DownVotes, tu.CommentCount, ub.BadgeCount, tu.PostRank, tu.UpVotesRank
// FROM TopUsers tu JOIN UserBadges ub ON tu.UserId = ub.UserId WHERE ub.BadgeCount > 5 ORDER BY tu.PostCount DESC, tu.UpVotes DESC LIMIT 10;
fn q5860(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((t, v), c)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + c.is_some() as i64],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type X = ((Id<User>, [i64; 5]), i64);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ua).and(&pc)).window(rank, |(_, n)| Reverse(n), asc);
    let w = (&w).window(rank, |(((_, a), _), _): (X, i64)| Reverse(a[2]), asc);
    let rk: MatSet<(Id<User>, ([i64; 5], i64, i64, i64))> = (&w).map(|((((u, a), n), pr), vr)| (u, (a, n, pr, vr))).collect();
    let by_user = by_first(&rk);
    let v = drain(db.user.select((&by_user).and((&bc).filt(|b| b > 5))));
    let v = top_n(v, |&(u, ((a, n, _, _), _))| (Reverse(n), Reverse(a[2]), u), 10);
    rows(v.into_iter().map(|(u, ((a, n, pr, vr), b))| row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(b), V::I(pr), V::I(vr)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.TotalAnswers, us.GoldBadges, us.SilverBadges, us.BronzeBadges, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount,
//        rp.CommentCount
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId WHERE rp.Rank <= 5 ORDER BY us.Reputation DESC, rp.Score DESC;
//
// The posts x badges product is driven only for the owners of ranked questions, the only users the inner join keeps.
fn q6682(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]);
    let dp = (&owners).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&us).and(&dp))));
    rows(v.into_iter().map(|(p, (c, ((u, a), n)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.Reputation),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, pt.Name AS PostType, p.OwnerUserId
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// TopUsers AS (SELECT us.UserId, us.Reputation, us.PostCount, us.BadgeCount, us.UpVotes, us.DownVotes, ROW_NUMBER() OVER (ORDER BY us.Reputation DESC) AS Rank FROM UserStats us)
// SELECT tu.Rank, tu.UserId, tu.Reputation, tu.PostCount, tu.BadgeCount, tu.UpVotes, tu.DownVotes, ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.AnswerCount,
//        ps.CommentCount, ps.FavoriteCount, ps.PostType
// FROM TopUsers tu JOIN PostStats ps ON tu.UserId = ps.OwnerUserId WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// Rank reads only Reputation, so the ten users are picked first and the posts x badges x votes product is driven for them alone.
fn q11146(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let rank = by_first(&rk);
    let tu: MatSet<Id<User>> = (&rk).map(|(u, _)| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (v, _)| {
            let v = v.flatten();
            [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64]
        });
    let dp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let db_ = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let v = drain((&tu).select(&rank).and(&us).and(&dp).and(&db_).and(recent));
    rows(v.into_iter().map(|(u, ((((r, a), n), b), p))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend([V::I(n), V::I(b), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "type"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT p.Title, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName, ub.BadgeCount, phc.EditCount, phc.LastEditDate,
//        CASE WHEN phc.EditCount > 0 THEN 'Edited' ELSE 'Not Edited' END AS EditStatus
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostHistoryCounts phc ON p.Id = phc.PostId
// WHERE p.Score > 0 AND p.ViewCount IS NOT NULL AND (ub.BadgeCount IS NULL OR ub.BadgeCount > 0) ORDER BY p.ViewCount DESC, p.CreationDate DESC FETCH FIRST 100 ROWS ONLY;
//
// RankedPosts is never read.
fn q31166(db: &'static So) -> String {
    let Post { owner_user, score, view_count, creation_date, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phc = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain(db.post.with(score.gt(0)).with(view_count).select(owner_user.select((&bc).filt(|b| b > 0)).and((&phc).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(view_count.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (b, h))| {
        let mut f = post_fields(db, p, &["title", "views", "created", "owner"]);
        f.push(V::I(b));
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d), V::S("Edited")],
            None => [V::Null, V::Null, V::S("Not Edited")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.ViewCount, p.AnswerCount, p.CommentCount, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// UserActivity AS (SELECT u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.CommentCount, 0)) AS TotalComments,
//        SUM(COALESCE(p.Score, 0)) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 GROUP BY u.DisplayName, u.Reputation),
// TopActiveUsers AS (SELECT ua.DisplayName, ua.Reputation, ua.QuestionCount, ua.TotalViews, ua.TotalComments, ua.TotalScore, DENSE_RANK() OVER (ORDER BY ua.TotalViews DESC) AS RankByViews
//     FROM UserActivity ua WHERE ua.QuestionCount > 0)
// SELECT t.DisplayName, t.Reputation, t.QuestionCount, t.TotalViews, t.TotalComments, t.TotalScore, rp.Title AS LatestPostTitle, rp.CreationDate AS LatestPostDate
// FROM TopActiveUsers t LEFT JOIN RankedPosts rp ON t.DisplayName = (SELECT DisplayName FROM Users WHERE Id = rp.OwnerUserId)
// WHERE t.RankByViews <= 10 ORDER BY t.TotalViews DESC, t.TotalScore DESC;
//
// rn is never read, so every RankedPosts row whose owner has the name joins.
fn q26414(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, comment_count, score, .. } = &db.post;
    let asked = Ident::<Post>::new().with(post_type_id.eq(1));
    let ua = db
        .user
        .group_by((&db.user.display_name).and(&db.user.reputation))
        .select(posts_of(db).select(asked).select(view_count.opt().and(comment_count).and(score)).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((w, c), s)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + c, a[3] + s],
            None => a,
        });
    type K = (Str, i64);
    let w = whole((&ua).filt(|a| a[0] > 0)).select(Same::<K>::new().and(&ua)).window(dense_rank, |(_, a)| Reverse(a[1]), asc);
    let v: MatSet<(K, [i64; 4])> = (&w).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let rp: HashIdx<Str, Id<Post>> = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(current_date(), -1))))
        .select(owner_user.select(&db.user.display_name))
        .inv()
        .collect();
    type R = ((Str, i64), [i64; 4]);
    let v = drain((&v).select(Same::<R>::new().and(Same::<R>::new().map(|((n, _), _): R| n).select(&rp).opt())));
    rows(v.into_iter().map(|(_, (((n, r), a), p))| {
        let mut f = vec![V::S(n), V::I(r), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.AnswerCount, p.ViewCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.LastActivityDate >= CURRENT_DATE - INTERVAL '6 MONTH'),
// PostVoteStats AS (SELECT PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Votes v GROUP BY PostId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.AnswerCount, rp.ViewCount, pvs.UpVotes, pvs.DownVotes, pvs.TotalVotes
//     FROM RankedPosts rp LEFT JOIN PostVoteStats pvs ON rp.PostId = pvs.PostId WHERE rp.Rank <= 5)
// SELECT tp.*, COALESCE(tp.UpVotes, 0) - COALESCE(tp.DownVotes, 0) AS NetVotes,
//        CASE WHEN tp.ViewCount > 1000 THEN 'High Traffic' WHEN tp.ViewCount BETWEEN 500 AND 1000 THEN 'Medium Traffic' ELSE 'Low Traffic' END AS TrafficCategory
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId
// WHERE ph.CreationDate BETWEEN CURRENT_DATE - INTERVAL '1 YEAR' AND CURRENT_DATE AND ph.PostHistoryTypeId NOT IN (12, 10) ORDER BY NetVotes DESC, tp.CreationDate DESC;
//
// PostVoteStats groups Votes by PostId, the raw value, so a vote whose post is outside the dump still has a group; none of those can match a ranked post.
fn q4191(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, last_activity_date, view_count, .. } = &db.post;
    let today = current_date();
    let w = db
        .post
        .with(last_activity_date.ge(add_months(today, -6)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (Reverse(s), d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let pvs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = history_of(db).select(Ident::<PostHistory>::new().with(hd.between(add_years(today, -1), today)).with(post_history_type_id.filt(|t| t != 12 && t != 10)));
    let v = drain((&tp).select((&pvs).opt().and(ph)));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a.map_or(0, |a| a[0] - a[1])), Reverse(creation_date.get(p).unwrap())), 0);
    rows(v.into_iter().map(|(p, (a, _))| {
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "answers", "views"]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2])],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::I(a.map_or(0, |a| a[0] - a[1])));
        f.push(V::S(match w {
            Some(w) if w > 1000 => "High Traffic",
            Some(w) if w >= 500 => "Medium Traffic",
            _ => "Low Traffic",
        }));
        row(f)
    }))
}

// WITH UserRankings AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetVotes, p.OwnerUserId
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId),
// UserPostInfo AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT ur.DisplayName AS UserDisplayName, ur.Reputation, ur.ReputationRank, upi.PostCount, upi.TotalScore, pp.Title AS PopularPostTitle, pp.NetVotes
// FROM UserRankings ur JOIN UserPostInfo upi ON ur.UserId = upi.UserId LEFT JOIN PopularPosts pp ON upi.UserId = pp.OwnerUserId
// WHERE (upi.PostCount > 5 OR ur.Reputation > 100) AND pp.NetVotes > 10 ORDER BY ur.Reputation DESC, pp.NetVotes DESC LIMIT 10;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so CreationDate is compared as a New York instant.
fn q2347(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let since = ny_to_utc(add_years(utc_to_ny(now_utc()), -1));
    let pp = db
        .post
        .with(creation_date.filt(move |d| ny_to_utc(d) >= since))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), r)| (u, r)).collect();
    let rank = by_first(&rk);
    let users = db.user.with((&ups).filt(|a| a[1] > 5).or((&db.user.reputation).gt(100)));
    let v = drain(users.select((&rank).and(&ups).and(posts_of(db).select(Ident::<Post>::new().and((&pp).filt(|n| n > 10))))));
    let v = top_n(v, |&(u, (_, (p, n)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u, p), 10);
    rows(v.into_iter().map(|(u, ((r, a), (p, n)))| row(vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(r), V::I(a[1]), nullable(a[4], a[1]), title(db, p), V::I(n)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS Author, DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.Author FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.Author, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.Author)
// SELECT pd.PostId, pd.Title, pd.Score, pd.ViewCount, pd.AnswerCount, pd.Author, pd.CommentCount, pd.UpVotes, pd.DownVotes,
//        (CASE WHEN pd.UpVotes + pd.DownVotes > 0 THEN CAST(pd.UpVotes AS FLOAT) / (pd.UpVotes + pd.DownVotes) * 100 ELSE 0 END) AS UpvotePercentage
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC;
//
// CAST(.. AS FLOAT) is a 4-byte REAL, so the percentage is computed in f32.
fn q5071(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, post_type_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(dense_rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::F(if a[1] + a[2] > 0 { (a[1] as f32 / (a[1] + a[2]) as f32 * 100f32) as f64 } else { 0.0 }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(b.UserId, 0) AS HasBadge, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation, u.DisplayName, b.UserId),
// PostHistory AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS HistoryCount, MIN(ph.CreationDate) AS FirstChange, MAX(ph.CreationDate) AS LastChange
//     FROM PostHistory ph GROUP BY ph.PostId, ph.PostHistoryTypeId)
// SELECT up.UserId, up.DisplayName, up.Reputation, rp.Title AS TopPostTitle, rp.CreationDate AS PostCreationDate, rp.Score AS PostScore, ph.HistoryCount AS ChangeFrequency,
//        ph.FirstChange AS FirstEditDate, ph.LastChange AS LastEditDate, CASE WHEN up.Reputation > 1000 THEN 'Active' ELSE 'New User' END AS UserStatus
// FROM UserReputation up LEFT JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId AND rp.Rank = 1 LEFT JOIN PostHistory ph ON rp.Id = ph.PostId
// WHERE up.HasBadge IS NULL ORDER BY up.Reputation DESC, rp.Score DESC LIMIT 10;
//
// HasBadge is a COALESCE with a non-NULL default, so `HasBadge IS NULL` holds for no group and the result is empty whatever the data. The filter is applied to the
// per-(user, b.UserId) groups before anything else is computed; Upvotes is never projected.
fn q380(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let has: MatSet<Id<User>> = db.badge.select(&db.badge.user).collect();
    let hb = |b: Option<Id<User>>| -> Option<i64> { Some(b.map_or(0, |u| db.user.origid.get(u).unwrap())) };
    let up: MatSet<Id<User>> = db.user.with(Ident::<User>::new().with(&has).opt().map(hb).filt(|h: Option<i64>| h.is_none())).collect();
    let w = db.post.with(post_type_id.eq(1).and(score.gt(0))).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let best: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.group_by(post.and(post_history_type_id)).select(hd).fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), d| (n + 1, lo.min(d), hi.max(d)));
    type G = ((Id<Post>, i64), (i64, i64, i64));
    let phv: MatSet<G> = whole(&ph).select(Same::<(Id<Post>, i64)>::new().and(&ph)).collect();
    let by_post: HashIdx<Id<Post>, (i64, i64, i64)> = (&phv).map(|((p, _), _): G| p).inv().map(|(_, a): G| a).collect();
    let v = drain((&up).select((&best).select(Ident::<Post>::new().and((&by_post).opt())).opt()));
    let v = top_n(v, |&(u, p)| (Reverse(db.user.reputation.get(u).unwrap()), p.is_none(), Reverse(p.map(|(p, _)| score.get(p).unwrap())), u), 10);
    rows(v.into_iter().map(|(u, p)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(match p {
            Some((p, h)) => {
                let mut g = post_fields(db, p, &["title", "created", "score"]);
                g.extend(match h {
                    Some((n, lo, hi)) => [V::I(n), V::T(lo), V::T(hi)],
                    None => [V::Null, V::Null, V::Null],
                });
                g
            }
            None => (0..6).map(|_| V::Null).collect(),
        });
        f.push(V::S(if db.user.reputation.get(u).unwrap() > 1000 { "Active" } else { "New User" }));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, AVG(COALESCE(P.ViewCount, 0)) AS AvgViewCount,
//        COUNT(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 END) AS AcceptedAnswers, DENSE_RANK() OVER (ORDER BY SUM(COALESCE(P.Score, 0)) DESC) AS ScoreRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, AvgViewCount, AcceptedAnswers FROM UserPostStats WHERE ScoreRank <= 10),
// CommentStats AS (SELECT C.UserId, COUNT(C.Id) AS CommentCount, AVG(C.Score) AS AvgCommentScore FROM Comments C GROUP BY C.UserId),
// UserPerformance AS (SELECT U.UserId, U.DisplayName, U.PostCount, U.TotalScore, U.AvgViewCount, U.AcceptedAnswers, COALESCE(C.CommentCount, 0) AS TotalComments,
//        COALESCE(C.AvgCommentScore, 0) AS AvgCommentScore FROM TopUsers U LEFT JOIN CommentStats C ON U.UserId = C.UserId)
// SELECT UP.DisplayName, UP.PostCount, UP.TotalScore, UP.AvgViewCount, UP.AcceptedAnswers, UP.TotalComments, UP.AvgCommentScore
// FROM UserPerformance UP WHERE UP.TotalScore > (SELECT AVG(TotalScore) FROM UserPostStats) ORDER BY UP.TotalScore DESC OFFSET 5 ROWS FETCH NEXT 5 ROWS ONLY;
fn q3719(db: &'static So) -> String {
    let Post { score, view_count, accepted_answer_id, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(accepted_answer_id.opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((s, w), x)) => [a[0] + 1, a[1] + 1, a[2] + s, a[3] + w.unwrap_or(0), a[4] + x.is_some() as i64],
            None => [a[0] + 1, a[1], a[2], a[3], a[4]],
        });
    let (n, sum) = (&ups).fold_flat((0i128, 0i128), |(n, s), a| (n + 1, s + a[2] as i128));
    let w = whole(&ups).select(Ident::<User>::new().and(&ups)).window(dense_rank, |(_, a)| Reverse(a[2]), asc);
    let tu: MatSet<(Id<User>, [i64; 5])> = (&w).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let cs = db.comment.group_by(&db.comment.user).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let v = drain((&tu).filt(|(_, a): (Id<User>, [i64; 5])| a[2] as i128 * n > sum).select(Same::<(Id<User>, [i64; 5])>::new().and(Same::<(Id<User>, [i64; 5])>::new().map(|(u, _)| u).select((&cs).opt()))));
    let v = top_n(v, |&(_, ((u, a), _))| (Reverse(a[2]), u), 10);
    rows(v.into_iter().skip(5).map(|(_, ((u, a), c))| {
        let c = c.unwrap_or([0, 0]);
        row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::I(a[4]), V::I(c[0]), if c[0] == 0 { V::F(0.0) } else { avg(c[1], c[0]) }])
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// PostStatistics AS (SELECT r.PostId, r.Title, r.CreationDate, r.OwnerName, r.Upvotes, r.Downvotes, r.CommentCount, ROW_NUMBER() OVER (ORDER BY r.Upvotes - r.Downvotes DESC) AS Rank,
//        CASE WHEN r.Upvotes > r.Downvotes THEN 'Positive' WHEN r.Upvotes < r.Downvotes THEN 'Negative' ELSE 'Neutral' END AS Sentiment FROM RecentPosts r)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.OwnerName, ps.Upvotes, ps.Downvotes, ps.CommentCount, ps.Rank, ps.Sentiment, NULLIF(ps.Upvotes - ps.Downvotes, 0) AS VoteDifference,
//        CASE WHEN ps.CommentCount > 10 THEN 'High Engagement' WHEN ps.CommentCount BETWEEN 5 AND 10 THEN 'Medium Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM PostStatistics ps WHERE ps.Rank <= 10 ORDER BY ps.Rank;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so CreationDate is compared as a New York instant.
fn q4412(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let since = ny_to_utc(add_days(utc_to_ny(now_utc()), -30));
    let rp = db
        .post
        .with(creation_date.filt(move |d| ny_to_utc(d) >= since))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let w = whole(&rp).select(Ident::<Post>::new().and(&rp)).window(row_number, |(p, a)| (Reverse(a[0] - a[1]), p), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((p, a), i))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(i)]);
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        f.push(if a[0] == a[1] { V::Null } else { V::I(a[0] - a[1]) });
        f.push(V::S(if a[2] > 10 { "High Engagement" } else if a[2] >= 5 { "Medium Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.OwnerDisplayName, rp.CreationDate, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId GROUP BY rp.PostId, rp.Title, rp.Body, rp.Tags, rp.OwnerDisplayName, rp.CreationDate),
// FilteredPosts AS (SELECT ps.*, ROW_NUMBER() OVER (ORDER BY ps.UpVotes - ps.DownVotes DESC, ps.CommentCount DESC, ps.TotalBounties DESC) AS ScoreRank FROM PostStats ps WHERE ps.CommentCount > 0)
// SELECT fp.PostId, fp.Title, fp.Body, fp.Tags, fp.OwnerDisplayName, fp.CreationDate, fp.UpVotes, fp.DownVotes, fp.CommentCount, fp.TotalBounties, fp.ScoreRank
// FROM FilteredPosts fp WHERE fp.ScoreRank <= 10 ORDER BY fp.ScoreRank;
fn q29508(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()))
        .fold([0i64; 4], |a, (c, v)| {
            let (t, b) = v.map_or((0, None), |(t, b)| (t, b));
            [a[0] + c.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + (t == 2) as i64, a[3] + (t == 3) as i64]
        });
    let w = whole((&ps).filt(|a| a[0] > 0)).select(Ident::<Post>::new().and(&ps)).window(row_number, |(p, a)| (Reverse(a[2] - a[3]), Reverse(a[0]), Reverse(a[1]), p), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((p, a), i))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "owner", "created"]);
        f.extend([V::I(a[2]), V::I(a[3]), V::I(a[0]), V::I(a[1]), V::I(i)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, LastAccessDate, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionsCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswersCount,
//        SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore, MAX(P.CreationDate) AS LastPostDate FROM Posts P GROUP BY P.OwnerUserId),
// UserActivity AS (SELECT UR.Id AS UserId, UR.DisplayName, COALESCE(PS.QuestionsCount, 0) AS TotalQuestions, COALESCE(PS.AnswersCount, 0) AS TotalAnswers, COALESCE(PS.TotalViews, 0) AS TotalViews,
//        COALESCE(PS.AverageScore, 0) AS AverageScore, UR.LastAccessDate, UR.ReputationRank FROM UserReputation UR LEFT JOIN PostStats PS ON UR.Id = PS.OwnerUserId),
// RecentActivities AS (SELECT U.Id AS UserId, U.DisplayName, RANK() OVER (PARTITION BY U.Id ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Users U JOIN Posts P ON P.OwnerUserId = U.Id WHERE P.CreationDate > CURRENT_TIMESTAMP - INTERVAL '30 days')
// SELECT UA.DisplayName, UA.TotalQuestions, UA.TotalAnswers, UA.TotalViews, UA.AverageScore, UA.LastAccessDate, UA.ReputationRank, RA.RecentPostRank
// FROM UserActivity UA LEFT JOIN RecentActivities RA ON UA.UserId = RA.UserId WHERE UA.TotalQuestions > 0 ORDER BY UA.ReputationRank DESC, UA.TotalViews DESC;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so CreationDate is compared as a New York instant.
fn q3742(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, score, creation_date, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), r)| (u, r)).collect();
    let urank = by_first(&rk);
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s, a[5] + 1]
    });
    let since = ny_to_utc(add_days(utc_to_ny(now_utc()), -30));
    let w = db.post.with(creation_date.filt(move |d| ny_to_utc(d) > since)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let ra: HashIdx<Id<User>, i64> = (&w).map(|(_, r)| r).collect();
    let v = drain(db.user.with((&ps).filt(|a| a[0] > 0)).select((&ps).and(&urank).and((&ra).opt())));
    rows(v.into_iter().map(|(u, ((a, r), x))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[3]), avg(a[4], a[5]), user_col(db, u, "last_access"), V::I(r), harness::fmt::oint(x)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVoteStats AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Votes v GROUP BY v.PostId),
// RecentEdits AS (SELECT pe.PostId, COUNT(*) AS EditCount FROM PostHistory pe WHERE pe.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     AND pe.PostHistoryTypeId IN (4, 5, 6) GROUP BY pe.PostId)
// SELECT rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes,
//        COALESCE(pvs.TotalVotes, 0) AS TotalVotes, COALESCE(re.EditCount, 0) AS RecentEditsCount
// FROM RankedPosts rp LEFT JOIN PostVoteStats pvs ON rp.Id = pvs.PostId LEFT JOIN RecentEdits re ON rp.Id = re.PostId
// WHERE rp.OwnerPostRank = 1 AND (COALESCE(pvs.TotalVotes, 0) > 10 OR re.EditCount > 2) ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 10;
//
// The ownerless posts are one partition of their own (NULL OwnerUserId), whose newest post is kept too.
fn q4142(db: &'static So) -> String {
    let Post { owner_user, owner_user_id, creation_date, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let pvs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1],
        None => a,
    });
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let edits = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_days(t0, -30))).with(post_history_type_id.is_in([4, 5, 6])));
    let re = (&tp).group_by(Ident::<Post>::new()).select(edits.opt()).fold(0i64, |n, e| n + e.is_some() as i64);
    let v = drain((&pvs).and(&re).filt(|(a, e): ([i64; 3], i64)| a[2] > 10 || e > 2));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (a, e))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.push(owner_user.get(p).map_or(V::S("Community User"), |u| user_col(db, u, "name")));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(e)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(v.BountyAmount) AS TotalBounty, RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS PostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT p.Id AS PostId, p.Title, PH.CreationDate AS ClosedDate, EXTRACT(EPOCH FROM (PH.CreationDate - p.CreationDate)) / 60 AS DurationUntilClosed
//     FROM Posts p JOIN PostHistory PH ON p.Id = PH.PostId WHERE PH.PostHistoryTypeId = 10),
// TopTags AS (SELECT t.Id, t.TagName, SUM(p.ViewCount) AS TotalViews, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.Id, t.TagName
//     HAVING SUM(p.ViewCount) > 1000),
// RankedUserStats AS (SELECT us.*, COALESCE(cp.Count, 0) AS ClosedPostCount FROM UserStats us LEFT JOIN (SELECT UserId, COUNT(*) AS Count FROM ClosedPosts GROUP BY UserId) AS cp ON us.UserId = cp.UserId)
// SELECT ru.DisplayName, ru.PostCount, ru.ClosedPostCount, tt.TagName, tt.TotalViews FROM RankedUserStats ru CROSS JOIN TopTags tt
// WHERE ru.PostCount > 5 AND tt.PostCount > 2 AND ru.TotalBounty > 50 ORDER BY ru.PostRank ASC, tt.TotalViews DESC;
//
// ClosedPosts has no UserId, so DuckDB binds it to us.UserId and runs the subquery as a LATERAL: every user's ClosedPostCount is the number of ClosedPosts rows.
fn q4371(db: &'static So) -> String {
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 2], |a, (_, b)| match b.flatten() {
            Some(b) => [a[0] + 1, a[1] + b],
            None => a,
        });
    let pc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let closed = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).fold_flat(0i64, |n, _| n + 1);
    let ru: MatSet<Id<User>> = db.user.with((&pc).filt(|n| n > 5)).with((&us).filt(|a| a[0] > 0 && a[1] > 50)).collect();
    let tags = tag_stats(db);
    let tt: MatSet<(Id<Tag>, [i64; 6])> = db.tag.select(Ident::<Tag>::new().and((&tags).filt(|a| a[0] > 2 && a[1] > 0 && a[2] > 1000))).collect();
    let v = drain((&ru).select(&pc).cross(&tt));
    rows(v.into_iter().map(|((u, _), (n, (t, a)))| row(vec![user_col(db, u, "name"), V::I(n), V::I(closed), V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[2])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC, p.CreationDate DESC) AS TagRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, u.DisplayName AS OwnerDisplayName FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.TagRank = 1),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v GROUP BY v.PostId)
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.ViewCount, trp.Score, trp.OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(pv.VoteCount, 0) AS TotalVotes,
//        COALESCE(pv.UpVotes, 0) AS UpVotes, COALESCE(pv.DownVotes, 0) AS DownVotes
// FROM TopRankedPosts trp LEFT JOIN PostComments pc ON trp.PostId = pc.PostId LEFT JOIN PostVotes pv ON trp.PostId = pv.PostId ORDER BY trp.Score DESC, trp.CreationDate DESC LIMIT 10;
fn q25510(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, tags_str, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(date(2024, 10, 1), -1))))
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|(((p, _), _), _)| p).collect();
    let owned = (&tp).with(owner_user);
    let cc = owned.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let v = top_n(drain((&cc).and(&pv)), |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS RN,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVoteCount
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR')
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpVoteCount, rp.DownVoteCount, COALESCE(pht.Name, 'Unknown') AS HistoryType, COALESCE(lt.Name, 'No Links') AS LinkType,
//        SUM(CASE WHEN a.Answered = 1 THEN 1 ELSE 0 END) AS TotalAnswers, NULLIF(MAX(v.BountyAmount), 0) AS MaxBounty
// FROM RankedPosts rp LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id LEFT JOIN PostLinks pl ON rp.PostId = pl.PostId
// LEFT JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id LEFT JOIN (SELECT ParentId, COUNT(*) AS Answered FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON rp.PostId = a.ParentId
// LEFT JOIN Votes v ON rp.PostId = v.PostId AND v.VoteTypeId = 8
// WHERE rp.RN <= 5 GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpVoteCount, rp.DownVoteCount, pht.Name, lt.Name ORDER BY TotalAnswers DESC, rp.ViewCount DESC;
//
// The GROUP BY names the history and link type, so the joined rows are materialised and grouped by (post, type name, link name).
fn q21422(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), d, p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db)).fold(0i64, |n, _| n + 1);
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8)));
    type J = (Id<Post>, Option<Id<PostHistory>>, Option<Id<PostLink>>, Option<Id<Vote>>);
    let j: MatSet<J> = (&tp).select(Ident::<Post>::new().and(history_of(db).opt()).and(links_of(db).opt()).and(bv.opt())).map(|(((p, h), l), v)| (p, h, l, v)).collect();
    let post_of = (&j).map(|(p, _, _, _): J| p);
    let hist_of = (&j).flat_map(|(_, h, _, _): J| h);
    let link_of = (&j).flat_map(|(_, _, l, _): J| l);
    let vote_of = (&j).flat_map(|(_, _, _, v): J| v);
    let lname = (&db.post_link.link_type).select(&db.link_type.name);
    let g = (&j)
        .group_by((&post_of).and((&hist_of).select(htype_name(db)).opt()).and((&link_of).select(lname).opt()))
        .select((&post_of).select((&ac).opt()).and((&vote_of).select(&db.vote.bounty_amount).opt()))
        .fold((0i64, None), |(n, m): (i64, Option<i64>), (a, b)| (n + (a == Some(1)) as i64, match (m, b) {
            (Some(x), Some(y)) => Some(x.max(y)),
            (x, None) => x,
            (None, y) => y,
        }));
    type K = ((Id<Post>, Option<Str>), Option<Str>);
    type G = (K, (i64, Option<i64>));
    let gv: MatSet<G> = whole(&g).select(Same::<K>::new().and(&g)).collect();
    let v = drain((&gv).select(Same::<G>::new().and(Same::<G>::new().map(|(((p, _), _), _): G| p).select(&uv))));
    rows(v.into_iter().map(|(_, ((((p, h), l), (n, m)), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(u[0]), V::I(u[1]), V::S(h.unwrap_or("Unknown")), V::S(l.unwrap_or("No Links")), V::I(n), m.filter(|&x| x != 0).map_or(V::Null, V::I)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT b.UserId, COUNT(*) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 WHEN vt.Name = 'DownMod' THEN -1 ELSE 0 END) AS VoteScore FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, COALESCE(pv.VoteScore, 0) AS PostVoteScore, rp.RankScore
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = ub.UserId) LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId
// WHERE rp.RankScore <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 10;
//
// UserBadges joins by display name, so a post matches the badge counts of every user with its owner's name.
fn q755(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), r)| (p, r)).collect();
    let rank = by_first(&rk);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ubv: MatSet<(Id<User>, [i64; 3])> = whole(&ub).select(Ident::<User>::new().and(&ub)).collect();
    let by_name: HashIdx<Str, (Id<User>, [i64; 3])> = (&ubv).map(|(u, _)| u).select(&db.user.display_name).inv().collect();
    let pv = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold(0i64, |s, n| s + if n == "UpMod" { 1 } else if n == "DownMod" { -1 } else { 0 });
    let v = drain((&rank).and(owner_user.select(&db.user.display_name).select(&by_name).map(|(_, a)| a).opt()).and((&pv).opt()));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10);
    rows(v.into_iter().map(|(p, ((r, b), s))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(s.unwrap_or(0)), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.ViewCount DESC) AS rn,
//        COUNT(*) OVER (PARTITION BY pt.Name) AS TotalPostsByType, DENSE_RANK() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS rank_creation
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.ViewCount IS NOT NULL),
// LatestVotes AS (SELECT PostId, COUNT(*) AS VoteCount, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes GROUP BY PostId),
// PostHistoryCounts AS (SELECT PostId, COUNT(*) FILTER (WHERE PostHistoryTypeId IN (10, 11, 12, 13)) AS HistoryCount FROM PostHistory GROUP BY PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, COALESCE(rv.VoteCount, 0) AS TotalVotes, COALESCE(rv.UpVotes, 0) AS UpVoteCount, COALESCE(rv.DownVotes, 0) AS DownVoteCount, rp.Score, rp.CreationDate,
//        phc.HistoryCount, CASE WHEN rp.rank_creation = 1 THEN 'Most Recent' WHEN rp.rn <= 3 THEN 'Top 3' ELSE 'Others' END AS PostRank
// FROM RankedPosts rp LEFT JOIN LatestVotes rv ON rp.PostId = rv.PostId LEFT JOIN PostHistoryCounts phc ON rp.PostId = phc.PostId
// WHERE rp.TotalPostsByType > 1 ORDER BY rp.ViewCount DESC, rp.Score DESC, rp.CreationDate DESC LIMIT 100;
fn q20388(db: &'static So) -> String {
    let Post { view_count, score, creation_date, .. } = &db.post;
    let w = db
        .post
        .with(view_count)
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and(view_count).and(creation_date))
        .window(row_number, |((p, w), _)| (Reverse(w), p), asc);
    let w = (&w).window(dense_rank, |(((_, _), d), _): (((Id<Post>, i64), i64), i64)| Reverse(d), asc);
    let size = db.post.with(view_count).group_by(ptype_name(db)).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let rk: MatSet<(Id<Post>, (i64, i64))> = (&w).and((&size).filt(|n| n > 1)).map(|(((((p, _), _), r), d), _)| (p, (r, d))).collect();
    let kept = by_first(&rk);
    let lv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let phc = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold(0i64, |n, t| n + matches!(t, 10 | 11 | 12 | 13) as i64);
    let v = drain((&kept).and((&lv).opt()).and((&phc).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(view_count.get(p).unwrap()), Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (((r, d), a), h))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["score", "created"]));
        f.push(harness::fmt::oint(h));
        f.push(V::S(if d == 1 { "Most Recent" } else if r <= 3 { "Top 3" } else { "Others" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserScores AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) * 10 AS ReputationScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// MartialMasters AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// FinalRanking AS (SELECT us.UserId, us.DisplayName, us.UpVotes, us.DownVotes, us.ReputationScore + COALESCE(mm.BadgeCount, 0) AS TotalScore
//     FROM UserScores us LEFT JOIN MartialMasters mm ON us.UserId = mm.UserId)
// SELECT fr.UserId, fr.DisplayName, fr.TotalScore, rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.Score
// FROM FinalRanking fr JOIN RankedPosts rp ON fr.UserId = rp.OwnerUserId WHERE fr.TotalScore > 50 ORDER BY fr.TotalScore DESC, rp.CreationDate DESC;
//
// UpVotes and DownVotes are never projected or filtered on, so the questions x votes product behind them is not driven; PostRank is never read either.
fn q27814(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let asked = || Ident::<Post>::new().with(post_type_id.eq(1));
    let qc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(asked()).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let fr = (&qc).and(&bc).map(|(q, b)| q * 10 + b);
    let v = drain(db.user.select((&fr).filt(|t| t > 50).and(posts_of(db).select(asked()))));
    rows(v.into_iter().map(|(u, (t, p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(t));
        f.extend(post_fields(db, p, &["id", "title", "body", "created", "score"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5) GROUP BY ph.PostId)
// SELECT us.UserId, us.DisplayName, rp.Title, rp.CreationDate, rp.Score, us.Upvotes, us.Downvotes, COALESCE(phe.EditCount, 0) AS EditCount, phe.LastEditDate, us.BadgeCount,
//        CASE WHEN us.UserId IS NULL THEN 'Anonymous' ELSE 'User' END AS UserType
// FROM UserStats us FULL OUTER JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId LEFT JOIN PostHistoryStats phe ON rp.Id = phe.PostId
// WHERE (rp.rn <= 5 OR rp.rn IS NULL) AND (us.Upvotes - us.Downvotes) > 3 ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// `(us.Upvotes - us.Downvotes) > 3` rejects the rows only RankedPosts has, so the FULL OUTER JOIN keeps just the UserStats side: every such user with its ranked
// questions (rn <= 5), or one NULL row when it has none (then rn IS NULL).
fn q2603(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]);
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let rp: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phe = db.post_history.with(post_history_type_id.is_in([4, 5])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&us).filt(|a| a[0] - a[1] > 3).and((&rp).select(Ident::<Post>::new().and((&phe).opt())).opt()));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        let (pf, e) = match p {
            Some((p, e)) => (post_fields(db, p, &["title", "created", "score"]), e),
            None => (vec![V::Null, V::Null, V::Null], None),
        };
        f.extend(pf);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(e.map_or(0, |e| e.0)), e.map_or(V::Null, |e| V::T(e.1)), V::I(a[2]), V::S("User")]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId),
// RecentPosts AS (SELECT P.OwnerUserId, COUNT(*) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// ClosedPosts AS (SELECT PH.UserId, COUNT(*) AS ClosedCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId)
// SELECT U.DisplayName, U.Reputation, COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(RP.PostCount, 0) AS TotalPosts, COALESCE(RP.QuestionsCount, 0) AS TotalQuestions, COALESCE(RP.AnswersCount, 0) AS TotalAnswers, COALESCE(CP.ClosedCount, 0) AS TotalClosedPosts
// FROM RankedUsers U LEFT JOIN UserBadges UB ON U.UserId = UB.UserId LEFT JOIN RecentPosts RP ON U.UserId = RP.OwnerUserId LEFT JOIN ClosedPosts CP ON U.UserId = CP.UserId
// WHERE U.ReputationRank <= 10 ORDER BY U.Reputation DESC;
fn q3968(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, .. } = &db.post;
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let rp = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tu).select((&ub).opt().and((&rp).opt()).and((&cp).opt())));
    rows(v.into_iter().map(|(u, ((b, p), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend(p.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(c.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostScoreStats AS (SELECT p.Id AS PostId, p.Title, p.Score + COALESCE(SUM(v.BountyAmount), 0) AS TotalScore, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.Score),
// ClosedPostStats AS (SELECT ph.PostId, COUNT(*) AS CloseCount, MIN(ph.CreationDate) AS FirstCloseDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// TopUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation > 0)
// SELECT u.UserId, u.DisplayName, u.UpVotes, u.DownVotes, p.PostId, p.Title, p.TotalScore, COALESCE(c.CloseCount, 0) AS CloseCount, c.FirstCloseDate, tu.UserRank
// FROM UserVoteStats u JOIN PostScoreStats p ON u.UserId = p.PostId LEFT JOIN ClosedPostStats c ON p.PostId = c.PostId JOIN TopUsers tu ON u.UserId = tu.Id
// WHERE p.TotalScore > 0 ORDER BY p.TotalScore DESC, u.UpVotes - u.DownVotes DESC LIMIT 100;
//
// `u.UserId = p.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q24610(db: &'static So) -> String {
    let Post { creation_date, score, origid, .. } = &db.post;
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(score.and(comments_of(db).opt()).and(bounty.opt()))
        .fold((0i64, 0i64, 0i64), |(_, b, c), ((x, cm), bo)| (x, b + bo.flatten().unwrap_or(0), c + cm.is_some() as i64));
    let w = whole((&db.user.reputation).gt(0)).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), r)| (u, r)).collect();
    let rank = by_first(&rk);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cps = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold((0i64, i64::MAX), |(n, m), d| (n + 1, m.min(d)));
    type P = (Id<Post>, ((i64, i64, i64), Option<(i64, i64)>));
    let psv: MatSet<P> = db.post.select(Ident::<Post>::new().and((&ps).filt(|(s, b, _)| s + b > 0).and((&cps).opt()))).collect();
    let by_id: HashIdx<i64, P> = (&psv).map(|(p, _): P| p).select(origid).inv().collect();
    let v = drain((&uv).and((&db.user.origid).select(&by_id)).and(&rank));
    let v = top_n(v, |&(u, ((a, (p, ((s, b, _), _))), _))| (Reverse(s + b), Reverse(a[0] - a[1]), u, p), 100);
    rows(v.into_iter().map(|(u, ((a, (p, ((s, b, _), c))), r))| {
        let t = s + b;
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(t), V::I(c.map_or(0, |c| c.0)), c.map_or(V::Null, |c| V::T(c.1)), V::I(r)]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, u.DisplayName),
// RankedPosts AS (SELECT Id, Title, ViewCount, CreationDate, OwnerDisplayName, CommentCount, UpvoteCount, DownvoteCount, ROW_NUMBER() OVER (ORDER BY UpvoteCount DESC, CommentCount DESC) AS Rank
//     FROM RecentPosts)
// SELECT r.Id, r.Title, r.ViewCount, r.CreationDate, r.OwnerDisplayName, r.CommentCount, r.UpvoteCount, r.DownvoteCount,
//        CASE WHEN r.UpvoteCount - r.DownvoteCount > 0 THEN 'Positive' WHEN r.UpvoteCount - r.DownvoteCount < 0 THEN 'Negative' ELSE 'Neutral' END AS Sentiment,
//        COALESCE(ph.Comment, 'No recent changes') AS RecentChange
// FROM RankedPosts r LEFT JOIN PostHistory ph ON r.Id = ph.PostId AND ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = r.Id)
// WHERE r.Rank <= 10 ORDER BY r.Rank;
fn q71(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let top = top_n(drain(&rp), |&(p, a)| (Reverse(a[1]), Reverse(a[0]), p), 10);
    let top = rel(top);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    type T = (Id<Post>, [i64; 3]);
    let last = Same::<T>::new().map(|(p, _): T| p).select(Ident::<Post>::new().and(&md).select(&at)).opt();
    let mut v = drain((&top).select(Same::<T>::new().and(last)));
    v.sort_by_key(|x| x.0);
    rows(v.into_iter().map(|(_, ((p, a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if a[1] - a[2] > 0 { "Positive" } else if a[1] - a[2] < 0 { "Negative" } else { "Neutral" }));
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No recent changes")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '90 days'),
// TopRankedPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostVoteCounts AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Votes v GROUP BY v.PostId)
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, trp.AnswerCount, trp.CommentCount AS PostCommentCount, COALESCE(pc.CommentCount, 0) AS TotalComments,
//        COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes, COALESCE(pvc.TotalVotes, 0) AS TotalVotes, trp.OwnerDisplayName
// FROM TopRankedPosts trp LEFT JOIN PostComments pc ON trp.PostId = pc.PostId LEFT JOIN PostVoteCounts pvc ON trp.PostId = pvc.PostId ORDER BY trp.Score DESC, trp.ViewCount DESC;
fn q9271(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -90)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1],
        None => a,
    });
    rows(drain((&cc).and(&pv)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

/// SQL `s LIKE pat`: `%` matches any run of characters, `_` exactly one; no escape character.
fn like(s: &str, pat: &str) -> bool {
    let (s, p): (Vec<char>, Vec<char>) = (s.chars().collect(), pat.chars().collect());
    let (mut i, mut j, mut star, mut mark) = (0, 0, usize::MAX, 0);
    while i < s.len() {
        if j < p.len() && (p[j] == '_' || (p[j] != '%' && p[j] == s[i])) {
            i += 1;
            j += 1;
        } else if j < p.len() && p[j] == '%' {
            star = j;
            mark = i;
            j += 1;
        } else if star != usize::MAX {
            j = star + 1;
            mark += 1;
            i = mark;
        } else {
            return false;
        }
    }
    while j < p.len() && p[j] == '%' {
        j += 1;
    }
    j == p.len()
}

// WITH TagStats AS (SELECT TRIM(Tags) AS Tag, COUNT(*) AS PostCount, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Posts WHERE Tags IS NOT NULL GROUP BY TRIM(Tags)),
// TopTags AS (SELECT Tag, PostCount, QuestionCount, AnswerCount, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagStats WHERE PostCount > 10),
// UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON U.Id = C.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// ActiveUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalComments, RANK() OVER (ORDER BY TotalComments DESC, TotalPosts DESC) AS UserRank FROM UserReputation WHERE Reputation > 50)
// SELECT T.Tag, T.PostCount, T.QuestionCount, T.AnswerCount, U.UserId AS TopUserId, U.DisplayName AS TopUserDisplayName, U.Reputation AS TopUserReputation,
//        ROW_NUMBER() OVER (PARTITION BY T.Tag ORDER BY U.TotalComments DESC) AS UserRankInTag
// FROM TopTags T JOIN UserReputation U ON T.Tag LIKE '%' || U.DisplayName || '%' WHERE U.TotalComments > 5 ORDER BY T.Tag, UserRankInTag LIMIT 50;
//
// The display name is a LIKE pattern, so its `%` and `_` are wildcards; the join tests each distinct name against each distinct tag string. Only
// TotalComments is read from UserReputation, a COUNT(DISTINCT), so its posts x comments product is not driven. ActiveUsers is never read.
fn q29781(db: &'static So) -> String {
    let Post { tags_str, post_type_id, .. } = &db.post;
    let ts_ = db.post.group_by(tags_str.map(|t: Str| t.trim())).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let tt: MatSet<Str> = db.post.select(tags_str.map(|t: Str| t.trim())).select(Same::<Str>::new().with((&ts_).filt(|a| a[0] > 10))).collect();
    let tc = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let names: HashIdx<Str, Id<User>> = db.user.with((&tc).filt(|n| n > 5)).select(&db.user.display_name).inv().collect();
    let hit: HashIdx<Str, Id<User>> = (&tt).select_where(&names, |t: Str, n: Str| like(t, &format!("%{n}%"))).collect();
    type H = (Str, Id<User>);
    let hm: MatSet<H> = (&tt).select(Same::<Str>::new().and(&hit)).collect();
    let w = (&hm)
        .group_by(Same::<H>::new().map(|(t, _): H| t))
        .select(Same::<H>::new().and(Same::<H>::new().map(|(_, u): H| u).select(&tc)).and(Same::<H>::new().map(|(t, _): H| t).select(&ts_)))
        .window(row_number, |(((_, u), n), _)| (Reverse(n), u), asc);
    let v = top_n(drain(&w), |&(t, ((((_, u), _), _), r))| (t, r, u), 50);
    rows(v.into_iter().map(|(t, ((((_, u), _), a), r))| {
        let mut f = vec![V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(P.Score) AS TotalScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalScore, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats),
// ClosedPosts AS (SELECT PH.PostId, P.Title, PH.CreationDate AS ClosedDate, CT.Name AS CloseReason FROM PostHistory PH INNER JOIN Posts P ON PH.PostId = P.Id
//     LEFT JOIN CloseReasonTypes CT ON CAST(PH.Comment AS integer) = CT.Id WHERE PH.PostHistoryTypeId = 10),
// UserClosedPosts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CP.PostId) AS ClosedPostCount FROM Users U LEFT JOIN ClosedPosts CP ON U.Id = (SELECT OwnerUserId FROM Posts WHERE Id = CP.PostId)
//     GROUP BY U.Id, U.DisplayName)
// SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.TotalScore, COALESCE(UCP.ClosedPostCount, 0) AS ClosedPostCount
// FROM TopUsers TU LEFT JOIN UserClosedPosts UCP ON TU.UserId = UCP.UserId WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC, TU.TotalScore DESC LIMIT 10;
//
// The CloseReasonTypes join matches at most one row per history row, so it cannot change the count, and is not computed.
fn q3867(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let ucp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(closes).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&ucp).and(&ups)), |&(u, (_, a))| (Reverse(db.user.reputation.get(u).unwrap()), a[1] == 0, Reverse(a[4]), u), 10);
    rows(v.into_iter().map(|(u, (c, a))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), V::I(c)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostsCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgesCount, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PopularTags AS (SELECT T.TagName, COUNT(P.Id) AS UsageCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName ORDER BY UsageCount DESC LIMIT 10)
// SELECT UA.UserId, UA.DisplayName, UA.PostsCount, UA.AnswersCount, UA.QuestionsCount, UA.BadgesCount, UA.LastPostDate, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate,
//        RP.Score AS RecentPostScore, PT.TagName AS PopularTag, PT.UsageCount AS TagUsageCount
// FROM UserActivity UA LEFT JOIN RecentPosts RP ON UA.UserId = RP.OwnerUserId AND RP.PostRank = 1 CROSS JOIN PopularTags PT WHERE UA.PostsCount > 0 ORDER BY UA.PostsCount DESC, UA.LastPostDate DESC;
fn q6133(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(creation_date)).opt().and(badges_of(db).opt()))
        .fold([0, 0, 0, 0, i64::MIN], |a, (p, b)| match p {
            Some((t, d)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + b.is_some() as i64, a[4].max(d)],
            None => [a[0], a[1], a[2], a[3] + b.is_some() as i64, a[4]],
        });
    let w = db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let recent: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let lt = tag_mentions(db);
    let usage = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t)).select(Same::<(Id<Post>, Id<Tag>)>::new()).fold(0i64, |n, _| n + 1);
    let byname = db.tag.group_by(&db.tag.tag_name).select((&usage).opt()).fold(0i64, |s, n| s + n.unwrap_or(0));
    let pt: MatSet<(Str, i64)> = rel(top_n(drain((&byname).filt(|n| n > 0)), |&(t, n)| (Reverse(n), t), 10)).map(|x| x).collect();
    let v = drain((&ua).filt(|a| a[0] > 0).and((&recent).opt()).cross(&pt));
    rows(v.into_iter().map(|((u, _), ((a, p), (t, n)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), tmax(a[4])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created", "score"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation > 0),
// PostSummary AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(P.ViewCount) AS AvgViews, MAX(P.CreationDate) AS LastPostDate FROM Posts P GROUP BY P.OwnerUserId),
// ClosedPosts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS ClosedCount FROM Posts P WHERE P.PostTypeId = 1 AND P.Id IN (SELECT PH.PostId FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10)
//     GROUP BY P.OwnerUserId)
// SELECT UM.DisplayName, UM.Reputation, PS.TotalPosts, PS.Questions, PS.Answers, PS.AvgViews, COALESCE(CP.ClosedCount, 0) AS ClosedPosts, UM.ReputationRank,
//        CASE WHEN UM.Reputation < 1000 THEN 'Novice' WHEN UM.Reputation BETWEEN 1000 AND 4999 THEN 'Intermediate' ELSE 'Expert' END AS UserLevel
// FROM UserMetrics UM LEFT JOIN PostSummary PS ON UM.UserId = PS.OwnerUserId LEFT JOIN ClosedPosts CP ON UM.UserId = CP.OwnerUserId
// WHERE (PS.TotalPosts IS NOT NULL AND PS.TotalPosts > 10) OR UM.ReputationRank < 50 ORDER BY UM.Reputation DESC, PS.AvgViews DESC;
fn q1887(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, .. } = &db.post;
    let w = whole((&db.user.reputation).gt(0)).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), r)| (u, r)).collect();
    let rank = by_first(&rk);
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 5], |a, (t, w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
    });
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).collect();
    let cp = db.post.with(post_type_id.eq(1)).with(&closed).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&rank).and((&ps).opt()).and((&cp).opt()).filt(|((r, p), _): ((i64, Option<[i64; 5]>), Option<i64>)| p.map_or(false, |p| p[0] > 10) || r < 50));
    rows(v.into_iter().map(|(u, ((r, p), c))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match p {
            Some(p) => [V::I(p[0]), V::I(p[1]), V::I(p[2]), avg(p[4], p[3])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend([V::I(c.unwrap_or(0)), V::I(r), V::S(if rep < 1000 { "Novice" } else if rep <= 4999 { "Intermediate" } else { "Expert" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS RankByViews,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT rp.Title, rp.CreationDate, rp.Score, rb.GoldBadges, rb.SilverBadges, rb.BronzeBadges, pc.CommentCount, pc.LastCommentDate,
//        CASE WHEN pc.LastCommentDate IS NULL THEN 'No comments yet' ELSE 'Comments available' END AS CommentStatus,
//        CASE WHEN rp.RankByViews = 1 THEN 'Most Viewed' WHEN rp.RankByScore = 1 THEN 'Top Scoring' ELSE 'Regular Post' END AS PostRankStatus
// FROM RankedPosts rp LEFT JOIN UserBadges rb ON rp.OwnerUserId = rb.UserId LEFT JOIN PostComments pc ON rp.Id = pc.PostId
// WHERE rp.RankByViews <= 5 AND rp.RankByScore <= 5 ORDER BY rp.ViewCount DESC, rp.Score DESC;
fn q3779(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, creation_date, score, view_count, .. } = &db.post;
    type X = ((Id<Post>, Option<i64>), i64);
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(view_count.opt()).and(score))
        .window(rank, |((_, w), _)| (w.is_none(), Reverse(w)), asc);
    let w = (&w).window(rank, |((_, s), _): (X, i64)| Reverse(s), asc);
    let rp: MatSet<(Id<Post>, (i64, i64))> = (&w).filt(|((_, a), b): ((X, i64), i64)| a <= 5 && b <= 5).map(|((((p, _), _), a), b)| (p, (a, b))).collect();
    let kept = by_first(&rp);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let pc = db.comment.group_by(&db.comment.post).select(&db.comment.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&kept).and(owner_user.select(&ub).opt()).and((&pc).opt()));
    rows(v.into_iter().map(|(p, (((a, b), u), c))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend(match u {
            Some(u) => u.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match c {
            Some((n, d)) => [V::I(n), V::T(d), V::S("Comments available")],
            None => [V::Null, V::Null, V::S("No comments yet")],
        });
        f.push(V::S(if a == 1 { "Most Viewed" } else if b == 1 { "Top Scoring" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 6, 10, 11) GROUP BY ph.PostId)
// SELECT rp.Title, rp.ViewCount, rp.Score, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, COALESCE(phc.EditCount, 0) AS TotalEdits, phc.LastEditDate
// FROM RankedPosts rp LEFT JOIN Users u ON rp.PostId = u.Id JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostHistoryCounts phc ON rp.PostId = phc.PostId
// WHERE rp.rn <= 3 AND (ub.BadgeCount > 0 OR rp.Score > 10) ORDER BY rp.Score DESC, rp.ViewCount DESC, ub.BadgeCount DESC LIMIT 10;
//
// rn numbers the post x comment rows, so the joined rows are materialised and ranked; rows of one post that tie on CreationDate agree in every projected column.
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q21112(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let j: MatSet<(Id<Post>, Option<Id<Comment>>)> = recent().select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    type J = (Id<Post>, Option<Id<Comment>>);
    let jpost = || Same::<J>::new().map(|(p, _): J| p);
    let w = (&j).group_by(jpost().select(post_type_id)).select(Same::<J>::new().and(jpost().select(creation_date))).window(row_number, |((p, c), d)| (Reverse(d), p, c), asc);
    let tj: MatSet<J> = (&w).filt(|(_, r)| r <= 3).map(|((x, _), _)| x).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phc = db.post_history.with(post_history_type_id.is_in([4, 6, 10, 11])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    type R = ((i64, [i64; 4]), Option<(i64, i64)>);
    let v = drain((&tj).select(jpost().select(score.and(origid.select(&uid).select(&ub)).and((&phc).opt()))).filt(|((s, b), _): R| b[0] > 0 || s > 10));
    let v = top_n(v, |&((p, c), ((_, b), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), Reverse(b[0]), p, c)
    }, 10);
    rows(v.into_iter().map(|((p, _), ((_, b), h))| {
        let mut f = post_fields(db, p, &["title", "views", "score"]);
        f.extend(b.map(V::I));
        f.extend([V::I(h.map_or(0, |h| h.0)), h.map_or(V::Null, |h| V::T(h.1))]);
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// PostVoteCounts AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes, COALESCE(bc.BadgeCount, 0) AS UserBadgeCount
//     FROM Posts p LEFT JOIN PostVoteCounts v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserBadgeCounts bc ON u.Id = bc.UserId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// CloseReasons AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastCloseDate, STRING_AGG(cr.Name, ', ') AS CloseReasonNames
//     FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.UpVotes, rp.DownVotes, rp.UserBadgeCount, cr.LastCloseDate, cr.CloseReasonNames,
//        CASE WHEN cr.LastCloseDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM RecentPosts rp LEFT JOIN CloseReasons cr ON rp.PostId = cr.PostId ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 100;
//
// WITH RECURSIVE, but no CTE refers to itself. The STRING_AGG has no ORDER BY; the names are joined in PostHistory id order.
fn q30992(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let rp = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let pv = votes_of(db).select(&db.vote.vote_type_id);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, origid, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(origid.and(hd).and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .buf_fold(|mut v| {
            v.sort();
            let m = v.iter().map(|x| x.0 .1).max().unwrap();
            let names: Vec<&str> = v.iter().map(|x| x.1).collect();
            (m, &*Box::leak(names.join(", ").into_boxed_str()))
        });
    let v = drain(rp.group_by(Ident::<Post>::new()).select(pv.opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]).and(owner_user.select(&bc).opt()).and((&cr).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((a, b), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b.unwrap_or(0))]);
        f.extend(match c {
            Some((d, n)) => [V::T(d), V::S(n), V::S("Closed")],
            None => [V::Null, V::Null, V::S("Open")],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COUNT(DISTINCT c.Id) AS CommentCount, SUM(b.Class) AS TotalBadges, MAX(p.CreationDate) AS LastPostDate,
//        CUME_DIST() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, CommentCount, TotalBadges, LastPostDate, ReputationRank FROM UserStatistics
//     WHERE PostCount > 0 ORDER BY ReputationRank LIMIT 10)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.CommentCount, tu.TotalBadges, tu.LastPostDate,
//        COUNT(DISTINCT CASE WHEN ph.PostId IS NOT NULL THEN ph.Id END) AS PostHistoryCount
// FROM TopUsers tu LEFT JOIN PostHistory ph ON tu.UserId = ph.UserId GROUP BY tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.CommentCount, tu.TotalBadges,
//        tu.LastPostDate ORDER BY tu.Reputation DESC;
//
// ReputationRank reads only Reputation (a CUME_DIST over it orders users as Reputation DESC does), so the ten users with posts are picked first and the
// posts x comments x badges product is driven for them alone.
fn q5062(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let users = db.user.with((&db.user.reputation).gt(0)).with(posts_of(db));
    let top = top_n(drain(users.select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(creation_date).and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0, 0, 0, 0, i64::MIN], |a, (p, b)| {
            let (t, d) = p.map_or((0, i64::MIN), |((t, d), _)| (t, d));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0), a[4].max(d)]
        });
    let dp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let dc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hist: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let key = (&db.user.display_name).and(&db.user.reputation).and(&dp).and(&s).and(&dc);
    let g = (&tu).group_by(key).select((&hist).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    rows(drain(&g).into_iter().map(|(((((n, r), p), a), c), h)| {
        row(vec![V::S(n), V::I(r), V::I(p), V::I(a[0]), V::I(a[1]), V::I(c), nullable(a[3], a[2]), tmax(a[4]), V::I(h)])
    }))
}

// WITH RECURSIVE UserPostCounts AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, RANK() OVER (ORDER BY COUNT(p.Id) DESC) AS Rank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// TopUsers AS (SELECT UserId FROM UserPostCounts WHERE Rank <= 10),
// PostVoteCounts AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Badges b WHERE b.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 years' GROUP BY b.UserId)
// SELECT u.DisplayName, u.Reputation, COALESCE(up.PostCount, 0) AS PostCount, COALESCE(v.PostId, 0) AS PostId, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes,
//        COALESCE(b.BadgeCount, 0) AS BadgeCount,
//        CASE WHEN b.HighestBadgeClass = 1 THEN 'Gold' WHEN b.HighestBadgeClass = 2 THEN 'Silver' WHEN b.HighestBadgeClass = 3 THEN 'Bronze' ELSE 'None' END AS HighestBadge
// FROM Users u LEFT JOIN UserPostCounts up ON u.Id = up.UserId LEFT JOIN PostVoteCounts v ON up.UserId = v.PostId LEFT JOIN UserBadges b ON u.Id = b.UserId
// WHERE u.Id IN (SELECT UserId FROM TopUsers) ORDER BY u.Reputation DESC, PostCount DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. `up.UserId = v.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q33977(db: &'static So) -> String {
    let Post { creation_date, origid, .. } = &db.post;
    let ups = user_posts(db);
    let w = whole(&ups).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| Reverse(a[1]), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let pv = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pvv: MatSet<(Id<Post>, [i64; 2])> = db.post.select(Ident::<Post>::new().and(&pv)).collect();
    let by_id: HashIdx<i64, (Id<Post>, [i64; 2])> = (&pvv).map(|(p, _)| p).select(origid).inv().collect();
    let Badge { user, date: bd, class, .. } = &db.badge;
    let ub = db.badge.with(bd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -2))).group_by(user).select(class).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let v = drain((&tu).select((&ups).and((&db.user.origid).select(&by_id).opt()).and((&ub).opt())));
    rows(v.into_iter().map(|(u, ((a, p), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(a[1]));
        f.extend(match p {
            Some((p, x)) => [V::I(origid.get(p).unwrap()), V::I(x[0]), V::I(x[1])],
            None => [V::I(0), V::I(0), V::I(0)],
        });
        f.push(V::I(b.map_or(0, |b| b.0)));
        f.push(V::S(match b.map(|b| b.1) {
            Some(1) => "Gold",
            Some(2) => "Silver",
            Some(3) => "Bronze",
            _ => "None",
        }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, DisplayName, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStats AS (SELECT Posts.Id AS PostId, Posts.Title, Posts.CreationDate, COUNT(Comments.Id) AS CommentCount, COALESCE(SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, COALESCE(SUM(CASE WHEN Votes.VoteTypeId = 1 THEN 1 ELSE 0 END), 0) AS AcceptedByOriginatorCount
//     FROM Posts LEFT JOIN Comments ON Posts.Id = Comments.PostId LEFT JOIN Votes ON Posts.Id = Votes.PostId WHERE Posts.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY Posts.Id, Posts.Title, Posts.CreationDate),
// PostMetrics AS (SELECT PostId, Title, CreationDate, CommentCount, UpVoteCount, DownVoteCount, (UpVoteCount - DownVoteCount) AS Score,
//        CASE WHEN AcceptedByOriginatorCount > 0 THEN 'Accepted' ELSE 'Not Accepted' END AS AcceptanceStatus FROM PostStats)
// SELECT UR.DisplayName, UR.Reputation, UR.ReputationRank, PM.Title, PM.CreationDate, PM.CommentCount, PM.UpVoteCount, PM.DownVoteCount, PM.Score, PM.AcceptanceStatus
// FROM UserReputation UR JOIN Posts P ON P.OwnerUserId = UR.UserId JOIN PostMetrics PM ON P.Id = PM.PostId WHERE PM.Score > 0 AND UR.ReputationRank <= 10 ORDER BY UR.Reputation DESC, PM.Score DESC;
fn q1282(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rk).map(|(u, _)| u).inv().collect();
    let posts: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with((&db.post.owner_user).select(&rank)).collect();
    let pm = (&posts)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (t == Some(1)) as i64]);
    let v = drain((&pm).filt(|a| a[1] - a[2] > 0).and((&db.post.owner_user).select(&rank)));
    rows(v.into_iter().map(|(p, (a, (u, r)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(r));
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2]), V::S(if a[3] > 0 { "Accepted" } else { "Not Accepted" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId IN (1, 2) AND p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR')),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.Tags FROM RankedPosts rp WHERE rp.Rank <= 10),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount, COALESCE(SUM(p.ViewCount), 0) AS TotalViews, COALESCE(SUM(p.Score), 0) AS TotalScore,
//        COALESCE(SUM(p.AnswerCount), 0) AS TotalAnswers, COALESCE(SUM(p.CommentCount), 0) AS TotalComments FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.Tags, ups.UserId, ups.DisplayName, ups.PostsCount, ups.TotalViews, ups.TotalScore, ups.TotalAnswers, ups.TotalComments
// FROM TopPosts tp JOIN UserPostStats ups ON tp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = ups.UserId) ORDER BY tp.Score DESC, ups.TotalScore DESC;
//
// `tp.PostId IN (posts owned by ups.UserId)` is a join of each top post to its owner.
fn q7739(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, answer_count, comment_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.is_in([1, 2]).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(score).and(answer_count.opt()).and(comment_count)).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((((w, s), n), c)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + n.unwrap_or(0), a[4] + c],
            None => a,
        });
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&ups))));
    rows(v.into_iter().map(|(p, (u, a))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers", "comments", "tags"]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, AVG(u.Reputation) AS AverageReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation > 50 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalBounties, AverageReputation, RANK() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserActivity),
// ClosedPosts AS (SELECT p.Id AS PostId, p.Title, ph.CreationDate AS CloseDate, ph.UserDisplayName AS ClosedBy, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS CloseEvent
//     FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10)
// SELECT tu.DisplayName AS User, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalBounties, cp.Title AS ClosedPostTitle, cp.CloseDate, cp.ClosedBy,
//        CASE WHEN tu.TotalPosts > 100 THEN 'Highly Active' WHEN tu.TotalPosts BETWEEN 50 AND 100 THEN 'Moderately Active' ELSE 'Less Active' END AS ActivityLevel
// FROM TopUsers tu LEFT JOIN ClosedPosts cp ON tu.DisplayName = cp.ClosedBy ORDER BY tu.Rank, tu.TotalPosts DESC;
//
// CloseEvent is never read.
fn q2861(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(50));
    let ua = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.flatten().unwrap_or(0)]);
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { post_history_type_id, user_display_name, .. } = &db.post_history;
    let cp: HashIdx<Str, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(user_display_name).inv().collect();
    let v = drain((&ua).and(&pc).and((&db.user.display_name).select(&cp).opt()));
    rows(v.into_iter().map(|(u, ((a, n), h))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(match h {
            Some(h) => [title(db, db.post_history.post.get(h).unwrap()), V::T(db.post_history.creation_date.get(h).unwrap()), V::S(user_display_name.get(h).unwrap())],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if n > 100 { "Highly Active" } else if n >= 50 { "Moderately Active" } else { "Less Active" }));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, U.CreationDate, DENSE_RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// TopPostCounts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// RecentVotes AS (SELECT V.UserId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes V WHERE V.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY V.UserId),
// UserStats AS (SELECT RU.Id AS UserId, RU.DisplayName, COALESCE(TPC.PostCount, 0) AS PostCount, COALESCE(RV.VoteCount, 0) AS VoteCount, COALESCE(RV.UpVotes, 0) AS UpVotes,
//        COALESCE(RV.DownVotes, 0) AS DownVotes, RU.ReputationRank FROM RankedUsers RU LEFT JOIN TopPostCounts TPC ON RU.Id = TPC.OwnerUserId LEFT JOIN RecentVotes RV ON RU.Id = RV.UserId)
// SELECT US.UserId, US.DisplayName, US.ReputationRank, US.PostCount, US.VoteCount, (US.UpVotes - US.DownVotes) AS NetVotes,
//        CASE WHEN US.ReputationRank <= 10 THEN 'Top Contributor' WHEN US.ReputationRank BETWEEN 11 AND 50 THEN 'Moderate Contributor' ELSE 'New Contributor' END AS ContributorType
// FROM UserStats US WHERE US.PostCount > 0 ORDER BY US.ReputationRank LIMIT 100;
fn q3894(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { owner_user, creation_date, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(dense_rank, |(_, r)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), r)| (u, r)).collect();
    let urank = by_first(&rk);
    let tpc = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let Vote { user, creation_date: vd, vote_type_id, .. } = &db.vote;
    let rv = db.vote.with(vd.ge(add_months(t0, -1))).group_by(user).select(vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let v = drain((&tpc).filt(|n| n > 0).and(&urank).and((&rv).opt()));
    let v = top_n(v, |&(u, ((_, r), _))| (r, u), 100);
    rows(v.into_iter().map(|(u, ((n, r), a))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(r), V::I(n), V::I(a[0]), V::I(a[1] - a[2])]);
        f.push(V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Moderate Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStatistics),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ub.GoldBadgeCount, 0) AS GoldBadgeCount,
//        COALESCE(ub.SilverBadgeCount, 0) AS SilverBadgeCount, COALESCE(ub.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId WHERE tu.Rank <= 10 ORDER BY tu.Reputation DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the posts x votes product is driven for them alone.
fn q2766(db: &'static So) -> String {
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((t, _)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64],
            None => a,
        });
    let dp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    rows(drain((&s).and(&dp).and((&ub).opt())).into_iter().map(|(u, ((a, n), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.OwnerUserId, p.CreationDate),
// PopularTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName ORDER BY PostCount DESC LIMIT 5),
// LatestUserActivity AS (SELECT u.Id AS UserId, u.DisplayName, MAX(p.LastActivityDate) AS LastActivityDate FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT ru.DisplayName AS UserName, ru.Reputation, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, lut.LastActivityDate, pt.TagName
// FROM RankedUsers ru JOIN RecentPosts rp ON ru.UserId = rp.OwnerUserId LEFT JOIN LatestUserActivity lut ON ru.UserId = lut.UserId JOIN PopularTags pt ON pt.PostCount >= 1
// WHERE ru.ReputationRank <= 10 AND rp.CommentCount IS NOT NULL AND lut.LastActivityDate IS NOT NULL ORDER BY ru.Reputation DESC, rp.UpVoteCount DESC;
//
// The last ON names only pt, so the rows are crossed with the popular tags.
fn q4923(db: &'static So) -> String {
    let Post { owner_user, creation_date, last_activity_date, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let rp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user.select(&tu))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let lua = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(last_activity_date)).fold(i64::MIN, |m, d| m.max(d));
    let lt = tag_mentions(db);
    let usage = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t)).select(Same::<(Id<Post>, Id<Tag>)>::new()).fold(0i64, |n, _| n + 1);
    let byname = db.tag.group_by(&db.tag.tag_name).select((&usage).opt()).fold(0i64, |s, n| s + n.unwrap_or(0));
    let pt: MatSet<(Str, i64)> = rel(top_n(drain((&byname).filt(|n| n > 0)), |&(t, n)| (Reverse(n), t), 5)).map(|x| x).collect();
    let v = drain((&rp).and(owner_user.select(&lua)).cross((&pt).filt(|(_, n): (Str, i64)| n >= 1)));
    rows(v.into_iter().map(|((p, _), ((a, d), (t, _)))| {
        let mut f = post_fields(db, p, &["owner", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::T(d), V::S(t)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT P.Id) AS QuestionCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopQuestions AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, U.DisplayName, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS ScoreRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1),
// RecentActivity AS (SELECT PH.PostId, PH.CreationDate, PH.UserDisplayName, PH.Comment, RANK() OVER (ORDER BY PH.CreationDate DESC) AS ActivityRank FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11))
// SELECT US.DisplayName AS UserName, US.Reputation, US.UpVotes AS TotalUpVotes, US.DownVotes AS TotalDownVotes, US.QuestionCount AS TotalQuestions, TQ.Title AS TopQuestionTitle,
//        TQ.Score AS TopQuestionScore, TQ.ViewCount AS TopQuestionViewCount, RA.UserDisplayName AS RecentActivityUser, RA.Comment AS RecentActivityComment, RA.CreationDate AS RecentActivityDate
// FROM UserStats US JOIN TopQuestions TQ ON US.QuestionCount > 0 JOIN RecentActivity RA ON TQ.PostId = RA.PostId WHERE TQ.ScoreRank <= 5 ORDER BY US.Reputation DESC, US.UpVotes DESC;
//
// The first ON names only US, so the users with questions are crossed with the top questions' close history. ActivityRank is never read.
fn q7654(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let asked = || Ident::<Post>::new().with(post_type_id.eq(1));
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(asked()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let qc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(asked())).fold(0i64, |n, _| n + 1);
    let w = whole(db.post.with(post_type_id.eq(1)).with(owner_user)).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tq: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let ra = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let right: MatSet<(Id<Post>, Id<PostHistory>)> = (&tq).select(Ident::<Post>::new().and(ra)).collect();
    let v = drain((&us).and(&qc).cross(&right));
    rows(v.into_iter().map(|((u, _), ((a, n), (p, h)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([
            harness::fmt::ostr(db.post_history.user_display_name.get(h)),
            harness::fmt::ostr(db.post_history.comment.get(h)),
            V::T(db.post_history.creation_date.get(h).unwrap()),
        ]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(vt.UpVotes, 0)) AS TotalUpVotes, SUM(COALESCE(vt.DownVotes, 0)) AS TotalDownVotes,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) vt ON p.Id = vt.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalUpVotes, TotalDownVotes, QuestionCount, AnswerCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY PostCount DESC) AS PostCountRank FROM UserStats)
// SELECT u.DisplayName, u.Reputation, u.PostCount, u.TotalUpVotes, u.TotalDownVotes, u.QuestionCount, u.AnswerCount,
//        CASE WHEN u.ReputationRank <= 10 THEN 'Top Reputation' ELSE 'Standard Reputation' END AS ReputationCategory,
//        CASE WHEN u.PostCountRank <= 10 THEN 'Top Contributor' ELSE 'Standard Contributor' END AS ContributionCategory
// FROM TopUsers u WHERE u.PostCount > 0 ORDER BY u.Reputation DESC, u.PostCount DESC LIMIT 50;
fn q8724(db: &'static So) -> String {
    let vt = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&vt).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, x)) => {
                let x = x.unwrap_or([0, 0]);
                [a[0] + 1, a[1] + x[0], a[2] + x[1], a[3] + (t == 1) as i64, a[4] + (t == 2) as i64]
            }
            None => a,
        });
    type X = ((Id<User>, [i64; 5]), i64);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&us).and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let w = (&w).window(rank, |(((_, a), _), _): (X, i64)| Reverse(a[0]), asc);
    let v = drain((&w).filt(|((((_, a), _), _), _)| a[0] > 0));
    let v = top_n(v, |&(_, ((((u, a), rep), _), _))| (Reverse(rep), Reverse(a[0]), u), 50);
    rows(v.into_iter().map(|(_, ((((u, a), _), r), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top Reputation" } else { "Standard Reputation" }));
        f.push(V::S(if c <= 10 { "Top Contributor" } else { "Standard Contributor" }));
        row(f)
    }))
}

// Rewritten (rewrites/1143.sql): the final ORDER BY is tie-broken on ub.UserId, rp.PostId.
// WITH UserBadgeCounts AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges FROM Badges GROUP BY UserId),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.AcceptedAnswerId),
// RankedPosts AS (SELECT pd.*, RANK() OVER (ORDER BY pd.Score DESC) AS Rank FROM PostDetails pd)
// SELECT ub.UserId, u.DisplayName, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, rp.PostId, rp.Title, rp.Score, rp.Rank, rp.CommentCount, rp.Upvotes, rp.Downvotes
// FROM UserBadgeCounts ub JOIN Users u ON ub.UserId = u.Id LEFT JOIN RankedPosts rp ON u.Id = rp.AcceptedAnswerId
// WHERE ub.GoldBadges > 0 AND NOT EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = u.Id AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month')
// ORDER BY ub.GoldBadges DESC, rp.Score DESC, ub.UserId, rp.PostId LIMIT 10;
//
// `u.Id = rp.AcceptedAnswerId` joins a user id to a post id, so it goes through the raw ids. Rank reads only Score, so it is taken over every recent post
// and the comment x vote product is driven only for the posts that join a user.
fn q1143(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, score, accepted_answer_id, origid, .. } = &db.post;
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_months(t0, -1))));
    let users: MatSet<Id<User>> = db.user.with((&ub).filt(|a| a[0] > 0)).minus(recent).collect();
    let last_year = || db.post.with(creation_date.ge(add_years(t0, -1)));
    let w = whole(last_year()).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let prank = by_first(&rk);
    let acc = accepted_answer_id.opt().map(|a: Option<i64>| a.unwrap_or(0));
    let uid: HashIdx<i64, Id<User>> = (&users).select(&db.user.origid).inv().collect();
    let joined: MatSet<Id<Post>> = last_year().with((&acc).select(&uid)).collect();
    let pd = (&joined)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let by_acc: HashIdx<i64, Id<Post>> = (&joined).select(&acc).inv().collect();
    let v = drain((&users).select((&ub).and((&db.user.origid).select((&by_acc).select(Ident::<Post>::new().and(&pd).and(&prank))).opt())));
    let v = top_n(v, |&(u, (b, p))| (Reverse(b[0]), p.is_none(), Reverse(p.map(|((p, _), _)| score.get(p).unwrap())), db.user.origid.get(u).unwrap(), p.map(|((p, _), _)| origid.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(match p {
            Some(((p, a), r)) => {
                let mut g = post_fields(db, p, &["id", "title", "score"]);
                g.extend([V::I(r), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
                g
            }
            None => (0..7).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes, p.OwnerUserId FROM RankedPosts rp JOIN Posts p ON rp.PostId = p.Id
//     WHERE rp.RN = 1 AND rp.CommentCount > 5 AND (rp.UpVotes - rp.DownVotes) > 0),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT f.PostId) AS TotalPosts FROM Users u LEFT JOIN FilteredPosts f ON u.Id = f.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, COALESCE(MAX(f.UpVotes), 0) AS BestPostUpVotes
// FROM UserStats us LEFT JOIN FilteredPosts f ON us.UserId = f.OwnerUserId GROUP BY us.UserId, us.DisplayName, us.Reputation, us.TotalPosts HAVING us.Reputation > 1000
// ORDER BY us.TotalPosts DESC, BestPostUpVotes DESC;
//
// RN reads only CreationDate, so each owner's newest question is picked first and the comment x vote product is driven for those alone.
fn q2287(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let kept: MatSet<Id<Post>> = (&tp).with((&rp).filt(|a| a[0] > 5 && a[1] - a[2] > 0)).collect();
    let fp: HashIdx<Id<User>, Id<Post>> = (&kept).select(owner_user).inv().collect();
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&fp).select(&rp).opt()));
    rows(v.into_iter().map(|(u, f)| {
        let mut r = ucols(db, u, &["uid", "name", "rep"]);
        r.extend([V::I(f.is_some() as i64), V::I(f.map_or(0, |a| a[1]))]);
        row(r)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(CASE WHEN V.Id IS NOT NULL AND V.VoteTypeId = 2 THEN 1 END) AS UpvoteCount, COUNT(CASE WHEN V.Id IS NOT NULL AND V.VoteTypeId = 3 THEN 1 END) AS DownvoteCount,
//        COUNT(CASE WHEN PH.Id IS NOT NULL THEN 1 END) AS HistoryCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId IS NOT NULL LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.CreationDate, P.Score),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.CommentCount, PS.UpvoteCount, PS.DownvoteCount, PS.HistoryCount,
//        ROW_NUMBER() OVER (ORDER BY PS.Score DESC, PS.CommentCount DESC) AS Rank FROM PostStats PS)
// SELECT U.DisplayName, UReputation.Reputation, TP.Title, TP.CreationDate, TP.Score, TP.CommentCount, TP.UpvoteCount, TP.DownvoteCount, TP.HistoryCount
// FROM TopPosts TP JOIN Users U ON U.Id = (SELECT OwnerUserId FROM Posts WHERE Id = TP.PostId) JOIN UserReputation UReputation ON U.Id = UReputation.UserId
// WHERE TP.Rank <= 10 ORDER BY UReputation.Reputation DESC, TP.Score DESC;
//
// Rank leads with Score, so only posts ranked in the top ten by Score can reach it; the comment x vote x history product is driven for those alone.
fn q2609(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let w = whole(recent()).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let cand: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let voted = votes_of(db).select(Ident::<Vote>::new().with(&db.vote.user_id));
    let ps = (&cand)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(voted.select(&db.vote.vote_type_id).opt()).and(history_of(db).opt()))
        .fold([0i64; 4], |a, ((c, t), h)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + h.is_some() as i64]);
    let top = top_n(drain(&ps), |&(p, a)| (Reverse(score.get(p).unwrap()), Reverse(a[0]), p), 10);
    let tp = rel(top);
    type T = (Id<Post>, [i64; 4]);
    let v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select(owner_user))));
    rows(v.into_iter().map(|(_, ((p, a), u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH PostActivity AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COALESCE(pc.CommentCount, 0) AS TotalComments, COALESCE(pa.AnswerCount, 0) AS TotalAnswers,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) pc ON p.Id = pc.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) pa ON p.Id = pa.ParentId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 3 WHEN b.Class = 2 THEN 2 ELSE 1 END) AS BadgePoints FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName),
// PostSummary AS (SELECT pa.PostId, pa.Title, pa.TotalComments, pa.TotalAnswers, ur.DisplayName, ur.BadgePoints,
//        DENSE_RANK() OVER (ORDER BY pa.TotalAnswers DESC, pa.TotalComments DESC) AS PopularityRank FROM PostActivity pa JOIN UserReputation ur ON pa.OwnerUserId = ur.UserId)
// SELECT ps.Title, ps.TotalComments, ps.TotalAnswers, ps.DisplayName, ps.BadgePoints, ps.PopularityRank FROM PostSummary ps
// WHERE ps.BadgePoints > 0 AND (ps.TotalAnswers > 0 OR ps.TotalComments > 0) ORDER BY ps.PopularityRank LIMIT 10;
//
// UserPostRank is never read.
fn q4088(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = recent().group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let bp = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + match c {
        Some(1) => 3,
        Some(2) => 2,
        _ => 1,
    });
    let w = whole(&cc).select(Ident::<Post>::new().and(&cc).and(&ac).and(owner_user.select(&bp))).window(dense_rank, |(((_, c), a), _)| (Reverse(a), Reverse(c)), asc);
    let v = drain((&w).filt(|((((_, c), a), b), _)| b > 0 && (a > 0 || c > 0)));
    let v = top_n(v, |&(_, ((((p, _), _), _), r))| (r, p), 10);
    rows(v.into_iter().map(|(_, ((((p, c), a), b), r))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(c), V::I(a)]);
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.ParentId, 1 AS Level, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, COALESCE(NULLIF(p.Tags, ''), 'No Tags') AS Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.ParentId, ph.Level + 1, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, COALESCE(NULLIF(p.Tags, ''), 'No Tags') AS Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 2)
// SELECT ph.PostId, ph.Title, ph.Score, ph.CreationDate, ph.OwnerDisplayName, ph.Level, ph.Tags, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = ph.PostId), 0) AS CommentCount,
//        COALESCE((SELECT SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) FROM Votes v WHERE v.PostId = ph.PostId), 0) AS UpVoteCount,
//        COALESCE((SELECT SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) FROM Votes v WHERE v.PostId = ph.PostId), 0) AS DownVoteCount
// FROM PostHierarchy ph WHERE ph.Level = 1 ORDER BY ph.Score DESC, ph.CreationDate DESC LIMIT 10;
//
// Only the base case (Level = 1) is read, so the recursion is never needed.
fn q33316(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, tags_str, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner"]);
        f.push(V::I(1));
        f.push(V::S(tags_str.get(p).filter(|t| !t.is_empty()).unwrap_or("No Tags")));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(COALESCE(b.Class, 0)) AS TotalBadgeClass, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName, u.Reputation),
// QuestionStats AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.AnswerCount = 0 THEN 1 END) AS UnansweredCount, MAX(p.CreationDate) AS LatestQuestionDate FROM Posts p WHERE p.PostTypeId = 1
//     GROUP BY p.OwnerUserId),
// PerformanceBenchmark AS (SELECT us.DisplayName, us.Reputation, us.TotalPosts, us.TotalQuestions, us.TotalAnswers, us.TotalBadgeClass, q.UnansweredCount, q.LatestQuestionDate,
//        (us.TotalPosts * 1.0 / NULLIF(us.Reputation, 0)) AS EngagementScore FROM UserStats us LEFT JOIN QuestionStats q ON us.UserId = q.OwnerUserId)
// SELECT DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalBadgeClass, UnansweredCount, LatestQuestionDate, EngagementScore
// FROM PerformanceBenchmark WHERE EngagementScore > 0.1 ORDER BY EngagementScore DESC LIMIT 50 OFFSET 0;
//
// EngagementScore reads only the distinct post count and Reputation, so the fifty users are picked first and the posts x badges product is driven for them alone.
fn q1765(db: &'static So) -> String {
    let Post { post_type_id, owner_user, answer_count, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(100));
    let tp = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let es = (&tp).and(&db.user.reputation).map(|(n, r)| n as f64 * 1.0 / r as f64);
    let v = top_n(drain((&es).filt(|e| e > 0.1)), |&(u, e)| (Reverse(fkey(e)), u), 50);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + c.unwrap_or(0)]);
    let qs = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(answer_count.opt().and(creation_date)).fold((0i64, i64::MIN), |(n, m), (a, d)| (n + (a == Some(0)) as i64, m.max(d)));
    rows(drain((&s).and(&tp).and(&es).and((&qs).opt())).into_iter().map(|(u, (((a, n), e), q))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match q {
            Some((k, d)) => [V::I(k), V::T(d)],
            None => [V::Null, V::Null],
        });
        f.push(V::F(e));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, MAX(v.CreationDate) AS LastVoteDate FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
//        DENSE_RANK() OVER (ORDER BY COUNT(c.Id) DESC) AS CommentRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) GROUP BY p.Id, p.Title),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.CreationDate)
// SELECT p.Id AS PostId, p.Title, ps.CommentCount, ps.UpvoteCount, ps.DownvoteCount, cr.CloseCount, ur.DisplayName, ur.Reputation, ur.LastVoteDate
// FROM Posts p JOIN PostStatistics ps ON p.Id = ps.PostId LEFT JOIN ClosedPosts cr ON p.Id = cr.PostId LEFT JOIN Users owner ON p.OwnerUserId = owner.Id JOIN UserReputation ur ON owner.Id = ur.UserId
// WHERE p.CreationDate > (DATE '2024-10-01' - INTERVAL '1 year') AND (cr.CloseCount IS NULL OR cr.CloseCount = 0) ORDER BY ps.CommentRank, ps.CommentCount DESC;
//
// CommentRank is never projected and there is no LIMIT, so it only orders the output, and PostStatistics is driven for the selected posts alone.
fn q1807(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let posts: MatSet<Id<Post>> = db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1))).with(owner_user).collect();
    let ud = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select(&db.vote.vote_type_id);
    let ps = (&posts)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(ud.opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cr = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post.and(hd)).select(post_history_type_id).fold(0i64, |n, t| n + (t == 10) as i64);
    type G = ((Id<Post>, i64), i64);
    let crv: MatSet<G> = whole(&cr).select(Same::<(Id<Post>, i64)>::new().and(&cr)).collect();
    let by_post: HashIdx<Id<Post>, i64> = (&crv).map(|((p, _), _): G| p).inv().map(|(_, n): G| n).collect();
    let lv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.creation_date)).fold(i64::MIN, |m, d| m.max(d));
    let v = drain((&ps).and((&by_post).opt()).filt(|(_, n): ([i64; 3], Option<i64>)| n.map_or(true, |n| n == 0)).and(owner_user.select(Ident::<User>::new().and((&lv).opt()))));
    rows(v.into_iter().map(|(p, ((a, n), (u, l)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), harness::fmt::oint(n)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(harness::fmt::ots(l));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(a.Id) AS AnswerCount, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate,
//        DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, AVG(p.Score) AS AverageScore
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopClosedPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.ClosedDate, us.DisplayName AS UserDisplayName, us.Reputation AS UserReputation
//     FROM RankedPosts rp JOIN UserStatistics us ON rp.OwnerUserId = us.UserId WHERE rp.ClosedDate IS NOT NULL)
// SELECT tcp.Title, tcp.CreationDate, tcp.Score, tcp.ViewCount, tcp.ClosedDate, tcp.UserDisplayName, tcp.UserReputation FROM TopClosedPosts tcp ORDER BY tcp.ClosedDate DESC FETCH FIRST 10 ROWS ONLY;
//
// UserStatistics keeps every user with a post, and every question's owner has one, so the join is to the owner.
fn q7925(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(history_of(db).select(post_history_type_id.and(hd)).opt()))
        .fold(i64::MIN, |m, (_, h)| match h {
            Some((10, d)) => m.max(d),
            _ => m,
        });
    let v = top_n(drain((&rp).filt(|d| d != i64::MIN)), |&(p, d)| (Reverse(d), p), 10);
    rows(v.into_iter().map(|(p, d)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.push(V::T(d));
        f.extend(post_fields(db, p, &["owner", "rep"]));
        row(f)
    }))
}

// Rewritten (rewrites/8319.sql): the RankByScore window is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS RankByScore
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId = 1),
// UserVotes AS (SELECT v.UserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId),
// UserInformation AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(b.BatchCount, 0) AS BatchCount, COALESCE(uv.VoteCount, 0) AS UserVoteCount, COALESCE(uv.UpVotes, 0) AS UserUpVotes,
//        COALESCE(uv.DownVotes, 0) AS UserDownVotes FROM Users u LEFT JOIN (SELECT UserId, COUNT(Id) AS BatchCount FROM Badges GROUP BY UserId) b ON u.Id = b.UserId LEFT JOIN UserVotes uv ON u.Id = uv.UserId)
// SELECT p.PostId, p.Title, p.CreationDate, u.DisplayName, u.UserVoteCount, u.UserUpVotes, u.UserDownVotes, RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS GlobalRank
// FROM RankedPosts p JOIN UserInformation u ON p.OwnerUserId = u.UserId WHERE p.RankByScore <= 5 ORDER BY GlobalRank, p.Score DESC;
//
// BatchCount is never projected, so the badge counts are not computed.
fn q8319(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)).and(post_type_id.eq(1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let uv = db.vote.group_by(&db.vote.user).select(vtype_name(db)).fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let w = whole(&tp).select(Ident::<Post>::new().and(score).and(view_count.opt()).and(owner_user.select((&uv).opt()))).window(rank, |(((_, s), w), _)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    rows(drain(&w).into_iter().map(|(_, ((((p, _), _), a), r))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserRankings AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.CommentCount) AS TotalComments, SUM(p.Score) AS TotalScore, AVG(p.TotalBounty) AS AvgBounty
//     FROM Users u JOIN RankedPosts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopRankedUsers AS (SELECT UserId, DisplayName, TotalComments, TotalScore, AvgBounty, RANK() OVER (ORDER BY TotalScore DESC) AS UserRank FROM UserRankings)
// SELECT u.UserId, u.DisplayName, u.TotalComments, u.TotalScore, u.AvgBounty,
//        CASE WHEN u.UserRank <= 10 THEN 'Top Contributor' WHEN u.TotalComments > 50 THEN 'Active Contributor' ELSE 'New Contributor' END AS ContributorType
// FROM TopRankedUsers u WHERE u.UserRank <= 20 OR (u.TotalComments > 20 AND u.AvgBounty > 0) ORDER BY u.TotalScore DESC;
//
// PostRank is never read.
fn q30331(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(bounty.opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let ur = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select((&rp).and(score))
        .fold([0i64; 4], |a, (x, s)| [a[0] + x[0], a[1] + s, a[2] + x[1], a[3] + 1]);
    let w = whole(&ur).select(Ident::<User>::new().and(&ur)).window(rank, |(_, a)| Reverse(a[1]), asc);
    let v = drain((&w).filt(|((_, a), r)| r <= 20 || (a[0] > 20 && a[2] > 0)));
    rows(v.into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[3])]);
        f.push(V::S(if r <= 10 { "Top Contributor" } else if a[0] > 50 { "Active Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS rn, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.rn <= 5),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, COALESCE(ub.BadgeCount, 0) AS BadgeCount, (COALESCE(tp.UpVotes, 0) - COALESCE(tp.DownVotes, 0)) AS NetVotes,
//        CASE WHEN tp.CommentCount > 10 THEN 'Highly Engaged' WHEN tp.CommentCount BETWEEN 5 AND 10 THEN 'Moderately Engaged' ELSE 'Low Engagement' END AS EngagementLevel
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId ORDER BY tp.Score DESC, tp.ViewCount DESC LIMIT 50;
//
// `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. rn reads only Score, so the top posts are picked first.
fn q2849(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&s).and(origid.select(&uid).select(&bc)));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(b), V::I(a[1] - a[2])]);
        f.push(V::S(if a[0] > 10 { "Highly Engaged" } else if a[0] >= 5 { "Moderately Engaged" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND pt.Id IN (1, 2)),
// TopPosts AS (SELECT * FROM RankedPosts WHERE Rank <= 5),
// PostStatistics AS (SELECT pp.PostId, pp.Title, pp.OwnerDisplayName, pp.Score, pp.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(cv.VoteCount, 0) AS VoteCount
//     FROM TopPosts pp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON pp.PostId = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) cv ON pp.PostId = cv.PostId)
// SELECT ps.PostId, ps.Title, ps.OwnerDisplayName, ps.Score, ps.ViewCount, ps.CommentCount, ps.VoteCount,
//        CASE WHEN ps.Score > 10 THEN 'High Score' WHEN ps.Score BETWEEN 1 AND 10 THEN 'Moderate Score' ELSE 'Low Score' END AS ScoreCategory
// FROM PostStatistics ps ORDER BY ps.Score DESC, ps.ViewCount DESC;
fn q7300(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])))
        .with(owner_user)
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, n))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views"]);
        f.extend([V::I(c), V::I(n), V::S(if s > 10 { "High Score" } else if s >= 1 { "Moderate Score" } else { "Low Score" })]);
        row(f)
    }))
}

// WITH RecursiveUserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS TotalQuestions, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties, RANK() OVER (ORDER BY COUNT(p.Id) DESC) AS PostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 500 GROUP BY u.Id, u.DisplayName),
// UserActivityTrend AS (SELECT UserId, DisplayName, TotalPosts, TotalAnswers, TotalQuestions, TotalBounties, PostRank, LAG(TotalPosts) OVER (ORDER BY PostRank) AS PreviousPostCount FROM RecursiveUserActivity),
// ActivityGrowth AS (SELECT UserId, DisplayName, TotalPosts, TotalAnswers, TotalQuestions, TotalBounties, CASE WHEN PreviousPostCount IS NULL THEN NULL ELSE TotalPosts - PreviousPostCount END AS PostGrowth
//     FROM UserActivityTrend)
// SELECT u.Id, u.DisplayName, COALESCE(ag.PostGrowth, 0) AS RecentPostChange, ag.TotalPosts, ag.TotalAnswers, ag.TotalQuestions, ag.TotalBounties
// FROM ActivityGrowth ag JOIN Users u ON ag.UserId = u.Id WHERE ag.PostGrowth IS NOT NULL AND ag.PostGrowth > 0 ORDER BY ag.PostGrowth DESC LIMIT 10;
//
// Not recursive: the CTE is only named that way. LAG over PostRank, which ranks TotalPosts descending, takes the previous row's count; users tied on the rank are
// ordered by id.
fn q34552(db: &'static So) -> String {
    let ua = db
        .user
        .with((&db.user.reputation).gt(500))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(ptype_name(db).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((n, b)) => [a[0] + 1, a[1] + (n == "Answer") as i64, a[2] + (n == "Question") as i64, a[3] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let w = whole(&ua).select(Ident::<User>::new().and(&ua)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let w = (&w).window(lag, |((u, a), r): ((Id<User>, [i64; 4]), i64)| (r, u, a[0]), asc);
    let g = drain((&w).map(|(((u, a), _), p)| (u, a, p.map(|(_, _, n)| a[0] - n))).filt(|(_, _, d)| d.map_or(false, |d| d > 0)));
    let g = top_n(g, |&(_, (u, _, d))| (Reverse(d), u), 10);
    rows(g.into_iter().map(|(_, (u, a, d))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(d.unwrap_or(0)));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// Rewritten (rewrites/1050.sql): the Rank window is tie-broken on p.Id, v.Id, c.Id and the final ORDER BY on rp.PostId, rp.Rank.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.Id, v.Id, c.Id) AS Rank,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotes,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, p.OwnerUserId
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT up.DisplayName AS UserName, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes, rp.CommentCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
//        CASE WHEN rp.Rank = 1 THEN 'Top Post' ELSE 'Regular Post' END AS PostRank
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id JOIN UserBadges ub ON up.Id = ub.UserId WHERE rp.CommentCount > 10 AND ub.GoldBadges > 0
// ORDER BY rp.Score DESC, rp.ViewCount DESC, rp.PostId, rp.Rank LIMIT 50;
//
// RankedPosts has no GROUP BY, so its rows are the post x vote x comment rows; they are materialised, numbered per owner, and the per-post windows are folds.
fn q1050(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let posts = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    type J = (Id<Post>, Option<Id<Vote>>, Option<Id<Comment>>);
    let j: MatSet<J> = posts().select(Ident::<Post>::new().and(votes_of(db).opt()).and(comments_of(db).opt())).map(|((p, v), c)| (p, v, c)).collect();
    let pa = posts()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let jp = || Same::<J>::new().map(|(p, _, _): J| p);
    let w = (&j)
        .group_by(jp().select(owner_user))
        .select(Same::<J>::new().and(jp().select(score)))
        .window(row_number, |((p, v, c), s)| (Reverse(s), p, v.is_none(), v, c.is_none(), c), asc);
    let rk: MatSet<(J, i64)> = (&w).map(|((x, _), r)| (x, r)).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    type R = (J, i64);
    let post_of = Same::<R>::new().map(|((p, _, _), _): R| p);
    let v = drain((&rk).select(Same::<R>::new().and(post_of.select((&pa).filt(|a| a[2] > 10).and(owner_user.select((&ub).filt(|b| b[0] > 0)))))));
    let v = top_n(v, |&(_, (((p, _, _), r), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, r)
    }, 50);
    rows(v.into_iter().map(|(_, (((p, _, _), r), (a, b)))| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        f.push(V::S(if r == 1 { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.ViewCount > 100),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation > 500),
// PostVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// EnhancedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, pu.UserId, pu.DisplayName AS OwnerDisplayName, pv.UpVotes, pv.DownVotes, pu.Reputation AS OwnerReputation,
//        pu.UserRank FROM RankedPosts rp LEFT JOIN Posts p ON rp.PostId = p.Id LEFT JOIN TopUsers pu ON p.OwnerUserId = pu.UserId LEFT JOIN PostVotes pv ON p.Id = pv.PostId WHERE rp.Rank <= 5)
// SELECT ep.Title, ep.CreationDate, ep.ViewCount, ep.Score, COALESCE(ep.UpVotes, 0) AS UpVotes, COALESCE(ep.DownVotes, 0) AS DownVotes, ep.OwnerDisplayName,
//        CASE WHEN ep.OwnerReputation IS NULL THEN 'Unknown' WHEN ep.OwnerReputation >= 1000 THEN 'Elite' ELSE 'Newbie' END AS ReputationCategory
// FROM EnhancedPosts ep WHERE ep.OwnerDisplayName IS NOT NULL ORDER BY ep.Score DESC, ep.CreationDate DESC;
fn q3710(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, score, .. } = &db.post;
    let w = db.post.with(view_count.gt(100)).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let tu = Ident::<User>::new().with((&db.user.reputation).gt(500));
    let v = drain((&tp).select(owner_user.select(tu).and((&pv).opt())));
    rows(v.into_iter().map(|(p, (u, a))| {
        let a = a.unwrap_or([0; 2]);
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), user_col(db, u, "name"), V::S(if rep >= 1000 { "Elite" } else { "Newbie" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, COALESCE(NULLIF(u.DisplayName, ''), 'Anonymous') AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TagStats AS (SELECT t.TagName, COUNT(pt.Id) AS PostCount, SUM(CASE WHEN pt.Score > 10 THEN 1 ELSE 0 END) AS HighScorePosts FROM Tags t JOIN Posts pt ON pt.Tags LIKE '%' || t.TagName || '%'
//     GROUP BY t.TagName),
// PostHistoryDetails AS (SELECT ph.PostId, p.Title, ph.CreationDate AS EditDate, ph.Comment, p.ViewCount FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id
//     WHERE ph.PostHistoryTypeId IN (4, 5) AND ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months')
// SELECT rp.Title, rp.OwnerDisplayName, rp.CreationDate AS PostCreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, ts.TagName, ts.PostCount, ts.HighScorePosts, ph.EditDate, ph.Comment AS EditComment
// FROM RankedPosts rp LEFT JOIN TagStats ts ON ts.PostCount > 5 LEFT JOIN PostHistoryDetails ph ON rp.PostId = ph.PostId WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// The TagStats ON names only ts, so every ranked post is crossed with the tags used more than five times.
fn q5673(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db
        .post
        .with(creation_date.ge(add_years(t0, -1)).and(score.gt(0)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let lt = tag_mentions(db);
    let per_tag = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t)).select(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p).select(score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + (s > 10) as i64]);
    let tsn = db.tag.group_by(&db.tag.tag_name).select(&per_tag).fold([0i64; 2], |a, x| [a[0] + x[0], a[1] + x[1]]);
    let tsv: HashIdx<(), (Str, [i64; 2])> = whole((&tsn).filt(|a| a[0] > 5)).select(Same::<Str>::new().and(&tsn)).collect();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5])).with(hd.ge(add_months(t0, -6))));
    let v = drain((&tp).select(ph.opt().and(Ident::<Post>::new().map(|_| ()).select((&tsv).opt()))));
    rows(v.into_iter().map(|(p, (h, t))| {
        let mut f = post_fields(db, p, &["title"]);
        f.push(V::S(owner_user.get(p).map(|u| db.user.display_name.get(u).unwrap()).filter(|n| !n.is_empty()).unwrap_or("Anonymous")));
        f.extend(post_fields(db, p, &["created", "views", "score", "answers"]));
        f.extend(match t {
            Some((n, a)) => [V::S(n), V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), harness::fmt::ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount, COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//        COALESCE(SUM(b.Class) FILTER (WHERE b.Class IS NOT NULL), 0) AS TotalBadgeClass
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, CommentCount, QuestionCount, AnswerCount, TotalBadgeClass,
//        RANK() OVER (ORDER BY UpVotes DESC, DownVotes ASC, CommentCount DESC, TotalBadgeClass DESC) AS Rank FROM UserActivity)
// SELECT tu.DisplayName, tu.UpVotes, tu.DownVotes, tu.CommentCount, tu.QuestionCount, tu.AnswerCount, tu.TotalBadgeClass, CASE WHEN tu.Rank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorType,
//        (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = tu.UserId) AS TotalPosts, (SELECT COUNT(*) FROM Comments c WHERE c.UserId = tu.UserId) AS TotalUserComments
// FROM TopUsers tu WHERE tu.Rank <= 50 ORDER BY tu.Rank;
fn q27515(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let (t, v, c) = p.map_or((0, None, false), |((t, v), c)| (t, v, c.is_some()));
            [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + c as i64, a[3] + (t == 1) as i64, a[4] + (t == 2) as i64, a[5] + b.unwrap_or(0)]
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ua)).window(rank, |(_, a)| (Reverse(a[0]), a[1], Reverse(a[2]), Reverse(a[5])), asc);
    let tu: MatSet<(Id<User>, ([i64; 6], i64))> = (&w).filt(|(_, r)| r <= 50).map(|((u, a), r)| (u, (a, r))).collect();
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    type T = (Id<User>, ([i64; 6], i64));
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select((&pc).and(&cc)))));
    rows(v.into_iter().map(|(_, ((u, (a, r)), (p, c)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top Contributor" } else { "Regular Contributor" }));
        f.extend([V::I(p), V::I(c)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, COALESCE(vote.VoteCount, 0) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId IN (2, 3) GROUP BY PostId) vote ON p.Id = vote.PostId WHERE p.PostTypeId = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation > 1000),
// UserPostStats AS (SELECT u.UserId, COUNT(ps.PostId) AS TotalPosts, SUM(ps.VoteCount) AS TotalVotes, AVG(ps.Score) AS AverageScore
//     FROM TopUsers u LEFT JOIN PostStats ps ON u.UserId = ps.OwnerUserId GROUP BY u.UserId HAVING COUNT(ps.PostId) > 0)
// SELECT tu.DisplayName, tu.Reputation, ups.TotalPosts, ups.TotalVotes, ups.AverageScore, tu.UserRank,
//        CASE WHEN ups.AverageScore IS NULL THEN 'No Score Yet' WHEN ups.AverageScore > 0 THEN 'Positive Score' ELSE 'Negative Score' END AS ScoreStatus
// FROM TopUsers tu JOIN UserPostStats ups ON tu.UserId = ups.UserId ORDER BY ups.TotalVotes DESC, ups.TotalPosts DESC;
//
// rn is never read.
fn q766(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let w = whole((&db.user.reputation).gt(1000)).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), r)| (u, r)).collect();
    let urank = by_first(&rk);
    let vc = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let pvc = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(vc.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let tu: MatSet<Id<User>> = (&rk).map(|(u, _)| u).collect();
    let ups = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(&pvc)).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((s, n)) => [a[0] + 1, a[1] + n, a[2] + s],
            None => a,
        });
    let v = drain((&ups).filt(|a| a[0] > 0).and(&urank));
    rows(v.into_iter().map(|(u, (a, r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), V::I(r), V::S(if a[2] > 0 { "Positive Score" } else { "Negative Score" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.LastActivityDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS rn,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.ViewCount > 100),
// PopularUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users WHERE LastAccessDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month') GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT p.Title, p.ViewCount, u.DisplayName AS Owner, r.CommentCount, p.LastActivityDate,
//        CASE WHEN p.ViewCount > 1000 THEN 'High' WHEN p.ViewCount BETWEEN 500 AND 1000 THEN 'Medium' ELSE 'Low' END AS PopularityLevel,
//        CASE WHEN r.rn IS NULL THEN 'No Posts' ELSE CONCAT('Has Posts: ', r.rn) END AS PostStatus
// FROM RankedPosts r JOIN Posts p ON r.Id = p.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN PopularUsers pu ON u.Id = pu.UserId
// WHERE pu.UserId IS NOT NULL ORDER BY p.LastActivityDate DESC LIMIT 50 OFFSET 0;
//
// RankedPosts has no GROUP BY, so rn numbers the post x comment rows; they are materialised and numbered per owner (ties broken by post and comment id).
// Restricting to the popular owners first keeps whole partitions, so it cannot change rn. BadgeCount is never read.
fn q4544(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, view_count, owner_user, last_activity_date, .. } = &db.post;
    let (n, sum) = db.user.with((&db.user.last_access_date).ge(add_months(t0, -1))).select(&db.user.reputation).fold_flat((0i128, 0i128), |(n, s), r| (n + 1, s + r as i128));
    let popular = move || Ident::<User>::new().with((&db.user.reputation).filt(move |r| r as i128 * n > sum));
    let posts = || db.post.with(creation_date.ge(add_years(t0, -1))).with(view_count.gt(100)).with(owner_user.select(popular()));
    type J = (Id<Post>, Option<Id<Comment>>);
    let j: MatSet<J> = posts().select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    let cc = posts().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let jp = || Same::<J>::new().map(|(p, _): J| p);
    let w = (&j).group_by(jp().select(owner_user)).select(Same::<J>::new().and(jp().select(view_count))).window(row_number, |((p, c), w)| (Reverse(w), p, c), asc);
    let rk: MatSet<(J, i64)> = (&w).map(|((x, _), r)| (x, r)).collect();
    type R = (J, i64);
    let v = drain((&rk).select(Same::<R>::new().and(Same::<R>::new().map(|((p, _), _): R| p).select(&cc))));
    let v = top_n(v, |&(_, (((p, c), r), _))| (Reverse(last_activity_date.get(p).unwrap()), p, r, c), 50);
    rows(v.into_iter().map(|(_, (((p, _), r), c))| {
        let w = view_count.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "views", "owner"]);
        f.extend([V::I(c), V::T(last_activity_date.get(p).unwrap())]);
        f.push(V::S(if w > 1000 { "High" } else if w >= 500 { "Medium" } else { "Low" }));
        f.push(V::Owned(format!("Has Posts: {r}")));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("25105", q25105),
    ("4852", q4852),
    ("5747", q5747),
    ("8712", q8712),
    ("6433", q6433),
    ("26238", q26238),
    ("8987", q8987),
    ("3454", q3454),
    ("25018", q25018),
    ("25806", q25806),
    ("28890", q28890),
    ("3435", q3435),
    ("3046", q3046),
    ("1061", q1061),
    ("3847", q3847),
    ("3902", q3902),
    ("1095", q1095),
    ("2817", q2817),
    ("9791", q9791),
    ("32583", q32583),
    ("1007", q1007),
    ("1753", q1753),
    ("151", q151),
    ("1173", q1173),
    ("806", q806),
    ("2194", q2194),
    ("2980", q2980),
    ("7164", q7164),
    ("8484", q8484),
    ("23633", q23633),
    ("22495", q22495),
    ("690", q690),
    ("3332", q3332),
    ("6664", q6664),
    ("9038", q9038),
    ("5860", q5860),
    ("6682", q6682),
    ("11146", q11146),
    ("31166", q31166),
    ("26414", q26414),
    ("4191", q4191),
    ("2347", q2347),
    ("5071", q5071),
    ("380", q380),
    ("3719", q3719),
    ("4412", q4412),
    ("29508", q29508),
    ("3742", q3742),
    ("4142", q4142),
    ("4371", q4371),
    ("25510", q25510),
    ("21422", q21422),
    ("755", q755),
    ("20388", q20388),
    ("27814", q27814),
    ("2603", q2603),
    ("3968", q3968),
    ("24610", q24610),
    ("71", q71),
    ("9271", q9271),
    ("29781", q29781),
    ("3867", q3867),
    ("6133", q6133),
    ("1887", q1887),
    ("3779", q3779),
    ("21112", q21112),
    ("30992", q30992),
    ("5062", q5062),
    ("33977", q33977),
    ("1282", q1282),
    ("7739", q7739),
    ("2861", q2861),
    ("3894", q3894),
    ("2766", q2766),
    ("4923", q4923),
    ("7654", q7654),
    ("8724", q8724),
    ("1143", q1143),
    ("2287", q2287),
    ("2609", q2609),
    ("4088", q4088),
    ("33316", q33316),
    ("1765", q1765),
    ("1807", q1807),
    ("7925", q7925),
    ("8319", q8319),
    ("30331", q30331),
    ("2849", q2849),
    ("7300", q7300),
    ("34552", q34552),
    ("1050", q1050),
    ("3710", q3710),
    ("5673", q5673),
    ("27515", q27515),
    ("766", q766),
    ("4544", q4544),
];
