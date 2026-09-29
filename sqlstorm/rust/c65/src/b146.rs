use harness::prelude::*;
use std::cmp::Reverse;

// ORDER BY (a key, b key) LIMIT n over a CROSS JOIN, the product driven by prela's `.cross`. The order leads with the a key, so an a row
// with k rows strictly ahead of it starts at position k * |b| + 1: only a rows whose key is at most that of the row at index (n - 1) / |b|
// can reach the first n. That cut-off key is picked first and the a side filtered on it in prela, as for a rank read from base columns.
fn cross_top<A: Copy, B: Copy, KA: Ord, KB: Ord>(a: Vec<A>, ka: impl Fn(&A) -> KA, b: Vec<B>, kb: impl Fn(&B) -> KB, n: usize) -> Vec<(A, B)> {
    let n = if n == usize::MAX { 0 } else { n };
    let a = top_n(a, &ka, 0);
    let ra = rel(a);
    let rb = rel(b);
    let v = if n > 0 && !rb.v.is_empty() && (n - 1) / rb.v.len() + 1 < ra.v.len() {
        let cut = ka(&ra.v[(n - 1) / rb.v.len()]);
        drain((&ra).filt(|x| ka(&x) <= cut).cross(&rb))
    } else {
        drain((&ra).cross(&rb))
    };
    top_n(v, |(_, (x, y))| (ka(x), kb(y)), n).into_iter().map(|x| x.1).collect()
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.AnswerCount, p.ViewCount,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank FROM Posts p WHERE p.PostTypeId = 1),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(p.Score) AS TotalScore, AVG(p.Score) AS AvgScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.QuestionCount, us.TotalScore, us.AvgScore, rp.PostId, rp.Title, rp.CreationDate, rp.Score AS PostScore, rp.ScoreRank
// FROM UserStatistics us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId WHERE rp.ScoreRank <= 5 ORDER BY us.Reputation DESC, rp.Score DESC;
fn q8526(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt())
        .fold((0i64, 0i64), |(n, s), x| (n + x.is_some() as i64, s + x.unwrap_or(0)));
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt().and(score)));
    let v = ranked(v, |&(_, (u, s))| (u, Reverse(s)), false);
    let mut out = Vec::new();
    let mut start = 0;
    for i in 0..v.len() {
        if i == 0 || v[i - 1].0 .1 .0 != v[i].0 .1 .0 {
            start = v[i].1 - 1;
        }
        out.push((v[i].0 .0, v[i].1 - start));
    }
    let rp = rel(out.into_iter().filter(|x| x.1 <= 5).collect());
    let mut v = drain((&rp).map(|(p, _)| p).select(owner_user.select(Ident::<User>::new().and(&us))));
    v.sort_by_key(|&(i, (u, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(rp.get(i).unwrap().0).unwrap())));
    rows(v.into_iter().map(|(i, (u, (n, s)))| {
        let (p, r) = rp.get(i).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), nullable(s, n), avg(s, n)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        DENSE_RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, VoteCount FROM RankedPosts WHERE ScoreRank <= 10)
// SELECT t.PostId, t.Title, t.CreationDate, t.Score, t.ViewCount, t.CommentCount, t.VoteCount, u.DisplayName AS OwnerDisplayName
// FROM TopPosts t JOIN Users u ON t.PostId IN (SELECT AcceptedAnswerId FROM Posts WHERE Id = t.PostId) ORDER BY t.Score DESC, t.ViewCount DESC;
//
// No post is its own accepted answer, so the join is empty.
fn q9562(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, accepted_answer_id, view_count, .. } = &db.post;
    let v = ranked(
        drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(score)),
        |&(_, s)| Reverse(s),
        true,
    );
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    let own = rel(drain((&s).and(origid.and(accepted_answer_id)).filt(|(_, (o, a))| o == a)));
    let users = rel(drain(db.user.select(&db.user.display_name)));
    let mut v = drain((&own).cross(&users));
    v.sort_by_key(|&(_, ((p, _), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(_, ((p, ((n, m), _)), (_, name)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(n), V::I(m), V::S(name)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id as PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount,
//        u.DisplayName as OwnerDisplayName, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) as Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// VoteStats AS (SELECT PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) as UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) as DownVotes,
//        COUNT(CASE WHEN v.VoteTypeId = 1 THEN 1 END) as AcceptedVotes FROM Votes v GROUP BY PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.FavoriteCount, rp.OwnerDisplayName,
//        vs.UpVotes, vs.DownVotes, vs.AcceptedVotes
// FROM RankedPosts rp LEFT JOIN VoteStats vs ON rp.PostId = vs.PostId WHERE rp.Rank <= 100 ORDER BY rp.CreationDate DESC;
fn q10144(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 1) as i64]);
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(owner_user.and(creation_date))), |&(_, (_, d))| Reverse(d), 100);
    let tp = rel(v);
    let mut v = drain((&tp).map(|(p, _)| p).select(Ident::<Post>::new().and((&vs).opt())));
    v.sort_by_key(|x| x.0);
    rows(v.into_iter().map(|(_, (p, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner"]);
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotes,
//        COALESCE(SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS TotalBadges,
//        ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= (CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '30 days'))
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.OwnerDisplayName, rp.UpVotes, rp.DownVotes, rp.TotalBadges, rp.Rank
// FROM RankedPosts rp WHERE rp.Rank <= 10 ORDER BY rp.Rank;
//
// No GROUP BY: one row per joined row, so the top post repeats once per
// vote-and-badge pair.
fn q7007(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let rows_of = || votes_of(db).select(&db.vote.vote_type_id).opt().and((&db.post.owner_user).select(badges_of(db)).opt());
    let qs = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let s = qs().group_by(Ident::<Post>::new()).select(rows_of()).fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]);
    let joined = drain(qs().select(score.and(rows_of()).and(&s)));
    let v = top_n(joined, |&(p, ((sc, _), _))| (Reverse(sc), p), 10);
    rows(v.into_iter().enumerate().map(|(i, (p, (_, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, U.DisplayName AS OwnerDisplayName, COUNT(C.Id) AS CommentCount, P.ViewCount, P.Score,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.ViewCount DESC) AS Rank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.Score > 0 GROUP BY P.Id, P.Title, U.DisplayName, P.ViewCount, P.Score, P.PostTypeId),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, CommentCount, ViewCount, Score FROM RankedPosts WHERE Rank <= 10),
// BadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId)
// SELECT TP.Title, TP.OwnerDisplayName, TP.CommentCount, TP.ViewCount, TP.Score, COALESCE(BC.BadgeCount, 0) AS UserBadgeCount
// FROM TopPosts TP LEFT JOIN BadgeCounts BC ON TP.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = BC.UserId)
// ORDER BY TP.Score DESC, TP.ViewCount DESC;
//
// The rank only reads Score and ViewCount, so the posts are picked before
// their comments are counted.
fn q6180(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(score.gt(0)).select(post_type_id.and(score).and(view_count.opt())));
    let top = top_per(v, |&(_, ((t, _), _))| t, |&(_, ((_, s), w))| (Reverse(s), w.is_none(), Reverse(w)), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let bu: MatSet<Id<User>> = db.badge.select(&db.badge.user).collect();
    let by_name: HashIdx<Str, Id<User>> = (&bu).select(&db.user.display_name).inv().collect();
    let mut v = drain((&cc).and(owner_user.select(&db.user.display_name).select(&by_name).select(&bc).opt()));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["views", "score"]));
        f.push(V::I(b.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COUNT(a.Id) FILTER (WHERE p.PostTypeId = 2) AS TotalAnswers,
//        COUNT(q.Id) FILTER (WHERE p.PostTypeId = 1) AS TotalQuestions, SUM(COALESCE(p.Score, 0)) AS TotalScore, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Posts q ON p.AcceptedAnswerId = q.Id
//     GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalAnswers, TotalQuestions, TotalScore, AvgViewCount, PostRank, ScoreRank
// FROM TopUsers WHERE PostRank <= 10 OR ScoreRank <= 10 ORDER BY PostRank, ScoreRank;
fn q12444(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, accepted_answer, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(children_of(db).opt()).and(accepted_answer.opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((((t, s), w), c), q)) => [a[0] + 1, a[1] + (t == 2 && c.is_some()) as i64, a[2] + (t == 1 && q.is_some()) as i64, a[3] + s, a[4] + w.unwrap_or(0), a[5] + 1],
            None => [a[0], a[1], a[2], a[3], a[4], a[5] + 1],
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[3]), false);
    let mut v: Vec<_> = v.into_iter().filter(|&((_, p), s)| p <= 10 || s <= 10).collect();
    v.sort_by_key(|&((_, p), s)| (p, s));
    rows(v.into_iter().map(|(((u, a), p), s)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[5]), V::I(p), V::I(s)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViewCount,
//        AVG(p.Score) AS AvgScore, AVG(CASE WHEN p.ViewCount > 0 THEN p.Score / p.ViewCount ELSE 0 END) AS ScorePerView
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopActiveUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, TotalViewCount, AvgScore, ScorePerView FROM TopActiveUsers WHERE Rank <= 10;
fn q11552(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold(([0i64; 6], 0.0f64), |(a, f), p| match p {
            Some(((t, s), w)) => (
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
                f + match w {
                    Some(w) if w > 0 => s as f64 / w as f64,
                    _ => 0.0,
                },
            ),
            None => ([a[0], a[1], a[2], a[3], a[4], a[5]], f),
        });
    let v = top_n(drain(&s), |&(_, (a, _))| Reverse(a[0]), 10);
    rows(v.into_iter().map(|(u, (a, f))| {
        let mut r = ucols(db, u, &["uid", "name"]);
        r.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[0]), nullable(a[5], a[4]), avg(a[3], a[0]), V::F(f / a[0].max(1) as f64)]);
        row(r)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// TopPosts AS (SELECT ..., RANK() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS PostRank FROM RecentPosts rp)
// SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, CommentCount, VoteCount FROM TopPosts WHERE PostRank <= 10;
fn q13348(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).select(score.and(view_count.opt()))), |&(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    rows(drain(&s).into_iter().map(|(p, (n, m))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(n), V::I(m)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(p.Score) AS AverageScore,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AverageScore, TotalUpvotes, TotalDownvotes, Rank FROM TopUsers WHERE Rank <= 10;
fn q11265(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::I(a[4]), V::I(a[5]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, LastPostDate, ScoreRank, PostRank
// FROM TopUsers WHERE ScoreRank <= 10 OR PostRank <= 10 ORDER BY ScoreRank, PostRank;
fn q12658(db: &'static So) -> String {
    rows(two_ranks(db, true, false).into_iter().map(|(((u, a), s), p)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), nullable(a[6], a[5]), tmax(a[7]), V::I(s), V::I(p)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore,
//        AVG(p.ViewCount) AS AvgViewsPerPost, AVG(p.Score) AS AvgScorePerPost
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, AvgViewsPerPost, AvgScorePerPost, ScoreRank
// FROM TopUsers WHERE ScoreRank <= 10 ORDER BY ScoreRank;
fn q13171(db: &'static So) -> String {
    rows(top_score_users(db).into_iter().map(|(u, a, r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1]), avg(a[6], a[5]), avg(a[4], a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewsRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, TotalScore, TotalViews, TotalComments, ScoreRank, ViewsRank
// FROM TopUsers WHERE ScoreRank <= 10 OR ViewsRank <= 10 ORDER BY ScoreRank, ViewsRank;
fn q11147(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let Post { score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and((&cc).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((s, w), c)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + c.unwrap_or(0)],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[1]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[2]), false);
    let mut v: Vec<_> = v.into_iter().filter(|&((_, s), w)| s <= 10 || w <= 10).collect();
    v.sort_by_key(|&((_, s), w)| (s, w));
    rows(v.into_iter().map(|(((u, a), s), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH TagCounts AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Tags AS t LEFT JOIN Posts AS p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, QuestionCount, AnswerCount, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagCounts),
// UserReputation AS (SELECT u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users AS u)
// SELECT t.TagName, t.PostCount, t.QuestionCount, t.AnswerCount, u.DisplayName AS TopUserDisplayName, u.Reputation AS TopUserReputation, u.UserRank
// FROM TopTags AS t JOIN UserReputation AS u ON u.UserRank <= 5 WHERE t.TagRank <= 10 ORDER BY t.TagRank, u.Reputation DESC;
fn q27293(db: &'static So) -> String {
    let ts = tag_stats(db);
    let tt = ranked(drain(&ts), |&(_, a)| Reverse(a[0]), false);
    let tt: Vec<_> = tt.into_iter().take_while(|x| x.1 <= 10).collect();
    let us = top_n(drain(db.user.select(&db.user.reputation)), |&(_, r)| Reverse(r), 5);
    let v = cross_top(tt, |&(_, r)| r, us.into_iter().enumerate().collect(), |&(i, _)| i, usize::MAX);
    rows(v.into_iter().map(|(((t, a), _), (i, (u, _)))| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[4]), V::I(a[5])];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// PopularPosts AS (SELECT rp.OwnerDisplayName, COUNT(*) AS PostCount, SUM(rp.Score) AS TotalScore FROM RankedPosts rp WHERE rp.RankScore <= 5 GROUP BY rp.OwnerDisplayName),
// UserBadges AS (SELECT u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName)
// SELECT pp.OwnerDisplayName, pp.PostCount, pp.TotalScore, COALESCE(ub.BadgeCount, 0) AS BadgeCount
// FROM PopularPosts pp LEFT JOIN UserBadges ub ON pp.OwnerDisplayName = ub.DisplayName
// WHERE pp.TotalScore > (SELECT AVG(TotalScore) FROM PopularPosts) ORDER BY pp.TotalScore DESC LIMIT 10;
fn q2739(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user.and(score)));
    let top = top_per(v, |&(_, (u, _))| u, |&(_, (_, s))| Reverse(s), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pp = (&tp).group_by(owner_user.select(&db.user.display_name)).select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let (n, sum) = (&pp).fold_flat((0i64, 0i64), |(n, t), (_, s)| (n + 1, t + s));
    let mean = sum as f64 / n as f64;
    let ub = db.user.group_by(&db.user.display_name).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = top_n(drain((&pp).filt(move |(_, s)| s as f64 > mean).and((&ub).opt())), |&(_, ((_, s), _))| Reverse(s), 10);
    rows(v.into_iter().map(|(name, ((n, s), b))| row(vec![V::S(name), V::I(n), V::I(s), V::I(b.unwrap_or(0))])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 3)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
// GROUP BY tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q6360(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, title, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user.and(score).and(creation_date)));
    let top = top_per(v, |&(_, ((u, _), _))| u, |&(_, ((_, s), d))| (Reverse(s), Reverse(d)), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(title.opt().and(score).and(view_count.opt()).and(owner_user.select(&db.user.display_name)))
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain(&s);
    v.sort_by_key(|&((((_, s), w), _), _)| (Reverse(s), w.is_none(), Reverse(w)));
    rows(v.into_iter().map(|((((t, s), w), n), a)| {
        let mut f = vec![t.map_or(V::Null, V::S), V::I(s), w.map_or(V::Null, V::I), V::S(n)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(P.Id) AS PostsCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation)
// SELECT TU.UserId, U.DisplayName, TU.Reputation, TU.PostsCount, TU.QuestionsCount, TU.AnswersCount, TU.GoldBadges + TU.SilverBadges + TU.BronzeBadges AS TotalBadges
// FROM TopUsers TU JOIN Users U ON TU.UserId = U.Id WHERE TU.Rank <= 10 ORDER BY TU.Reputation DESC;
//
// The rank only reads Reputation, so the ten users are picked before their
// joined rows are aggregated.
fn q9398(db: &'static So) -> String {
    let top = top_n(drain(db.user.select(&db.user.reputation)), |&(_, r)| Reverse(r), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (t, c)| [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + c.map_or(0, |c| (1..=3).contains(&c) as i64)]);
    let mut v = drain(&s);
    v.sort_by_key(|&(u, _)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, TRIM(UNNEST(STRING_TO_ARRAY(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><'))) AS TagName
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserPostReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        COUNT(DISTINCT p.Id) FILTER (WHERE p.AcceptedAnswerId IS NOT NULL) AS AcceptedAnswers
//     FROM Users u JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.Reputation),
// TopTags AS (SELECT pt.TagName, COUNT(pt.PostId) AS TagUsage FROM PostTags pt GROUP BY pt.TagName ORDER BY TagUsage DESC LIMIT 10)
// SELECT u.DisplayName, u.Reputation, upr.TotalScore, upr.PostCount, upr.AcceptedAnswers, tt.TagName, tt.TagUsage
// FROM UserPostReputation upr CROSS JOIN TopTags tt JOIN Users u ON u.Id = upr.UserId WHERE upr.PostCount > 5
// ORDER BY upr.Reputation DESC, tt.TagUsage DESC;
fn q26429(db: &'static So) -> String {
    let Post { post_type_id, score, accepted_answer_id, .. } = &db.post;
    let tu = db.post.with(post_type_id.eq(1)).select((&db.post.tags_str).flat_map(tag_list)).group_by(Same::new()).fold(0i64, |n, _| n + 1);
    let tt = top_n(drain(&tu), |&(_, n)| Reverse(n), 10);
    let upr = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(accepted_answer_id.opt())))
        .fold([0i64; 3], |a, (s, x)| [a[0] + 1, a[1] + s, a[2] + x.is_some() as i64]);
    let us = drain((&upr).filt(|a| a[0] > 5));
    let v = cross_top(us, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), tt, |&(_, n)| Reverse(n), usize::MAX);
    rows(v.into_iter().map(|((u, a), (t, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[0]), V::I(a[2]), V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews,
//        AVG(p.Score) AS AvgScore, AVG(p.ViewCount) AS AvgViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewsRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, TotalViews, AvgScore, AvgViews, ScoreRank, ViewsRank
// FROM TopUsers ORDER BY ScoreRank, ViewsRank;
fn q12787(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| (a[1] == 0, Reverse(a[4])), false);
    let mut v = ranked(v, |&((_, a), _)| (a[5] == 0, Reverse(a[6])), false);
    v.sort_by_key(|&((_, s), w)| (s, w));
    rows(v.into_iter().map(|(((u, a), s), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), nullable(a[6], a[5]), avg(a[4], a[1]), avg(a[6], a[5]), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) AS VoteCount, COUNT(DISTINCT ph.Id) AS HistoryEntryCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// RankedPosts AS (SELECT *, RANK() OVER (ORDER BY Score DESC, ViewCount DESC, CreationDate DESC) AS PostRank FROM PostMetrics)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.VoteCount, rp.HistoryEntryCount,
//        pt.Name AS PostTypeName, ut.DisplayName AS OwnerDisplayName
// FROM RankedPosts rp JOIN PostTypes pt ON rp.PostId = pt.Id JOIN Users ut ON rp.PostId = ut.Id WHERE rp.PostRank <= 10 ORDER BY rp.PostRank;
//
// The rank only reads post columns, so the posts are picked before their
// joined rows are aggregated. None of them has an Id small enough to be a
// PostTypes Id.
fn q6957(db: &'static So) -> String {
    let Post { creation_date, score, view_count, origid, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(date(2023, 1, 1))).select(score.and(view_count.opt()).and(creation_date))), |&(_, ((s, w), d))| (Reverse(s), w.is_none(), Reverse(w), Reverse(d)), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let c = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt()).and(history_of(db).opt())).fold(0i64, |n, ((c, _), _)| n + c.is_some() as i64);
    let dv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).buf_fold(distinct_some);
    let dh = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).buf_fold(distinct_some);
    let pts: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let uids: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let mut v = drain((&c).and(&dv).and(&dh).and(origid.select(&pts)).and(origid.select(&uids)));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), Reverse(creation_date.get(p).unwrap()))
    });
    rows(v.into_iter().map(|(p, ((((n, m), h), t), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(n), V::I(m), V::I(h), V::S(db.post_type.name.get(t).unwrap()), user_col(db, u, "name")]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName, COALESCE(COUNT(c.Id), 0) AS CommentCount
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId
// GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName
// ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q13503(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2])).select(post_type_id.and(score).and(owner_user)));
    let top = top_per(v, |&(_, ((t, _), _))| t, |&(p, ((_, s), _))| (Reverse(s), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let mut v = drain((&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.push(V::I(n));
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(co.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, p.Score, p.ViewCount
//     FROM Posts p LEFT JOIN Comments co ON p.Id = co.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2023-01-01' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT ..., RANK() OVER (ORDER BY Score DESC, UpVoteCount DESC, CreationDate DESC) AS Rank FROM PostStatistics)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, tp.Score, tp.ViewCount
// FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Rank;
fn q6885(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(date(2023, 1, 1)).and(post_type_id.eq(1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = ranked(drain(&s), |&(p, a)| (Reverse(score.get(p).unwrap()), Reverse(a[1]), Reverse(creation_date.get(p).unwrap())), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, a), _)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["score", "views"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS QuestionCount, SUM(p.Score) AS TotalScore
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName HAVING COUNT(p.Id) >= 5)
// SELECT ru.DisplayName, ru.QuestionCount, ru.TotalScore, rp.Title, rp.Score, rp.CreationDate, ph.Comment AS LastEditComment, ph.CreationDate AS LastEditDate
// FROM TopUsers ru JOIN RankedPosts rp ON ru.UserId = rp.PostId LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId AND ph.PostHistoryTypeId = 4
// WHERE rp.UserPostRank <= 3 ORDER BY ru.TotalScore DESC, rp.Score DESC;
//
// Recent question Ids are far above every User Id, so the join is empty.
fn q7109(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt().and(score)));
    let rp = top_per(v, |&(_, (u, _))| u, |&(p, (_, s))| (Reverse(s), p), 3, false);
    let rp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).with(post_type_id.eq(1)).select(score))
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let uids: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let edits: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with((&db.post_history.post_history_type_id).eq(4)).select(&db.post_history.post).inv().collect();
    let mut v = drain((&rp).select(Ident::<Post>::new().and(origid.select(&uids).select(Ident::<User>::new().and((&tu).filt(|(n, _)| n >= 5)))).and((&edits).opt())));
    v.sort_by_key(|&(_, ((p, (_, (_, s))), _))| (Reverse(s), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((p, (u, (n, s))), h))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(s)];
        f.extend(post_fields(db, p, &["title", "score", "created"]));
        f.extend(match h {
            Some(h) => [harness::fmt::ostr(db.post_history.comment.get(h)), V::T(db.post_history.creation_date.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserPostStatistics AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews, COUNT(C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewsRank FROM UserPostStatistics)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, TotalComments, ScoreRank, ViewsRank
// FROM TopUsers WHERE ScoreRank <= 10 OR ViewsRank <= 10 ORDER BY ScoreRank, ViewsRank;
fn q12806(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some((((t, s), w), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + c.is_some() as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| (a[0] == 0, Reverse(a[3])), false);
    let v = ranked(v, |&((_, a), _)| (a[4] == 0, Reverse(a[5])), false);
    let mut v: Vec<_> = v.into_iter().filter(|&((_, s), w)| s <= 10 || w <= 10).collect();
    v.sort_by_key(|&((_, s), w)| (s, w));
    rows(v.into_iter().map(|(((u, a), s), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[0]), nullable(a[5], a[4]), V::I(a[6]), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2020-01-01' GROUP BY p.Id, u.DisplayName, p.Title, p.Score, p.CreationDate, p.PostTypeId),
// TopPosts AS (SELECT *, RANK() OVER (ORDER BY Score DESC, CommentCount DESC) AS OverallRank FROM RankedPosts)
// SELECT tp.PostId, tp.Title, tp.Score, tp.OwnerDisplayName, tp.Rank, tp.OverallRank, tp.CommentCount, tp.UpVotes, tp.DownVotes
// FROM TopPosts tp WHERE tp.OverallRank <= 10 ORDER BY tp.OverallRank;
fn q6219(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(date(2020, 1, 1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = ranked(drain(&s), |&(p, _)| (post_type_id.get(p).unwrap(), Reverse(score.get(p).unwrap()), p), false);
    let mut first = 0;
    let mut tr = Vec::new();
    for i in 0..v.len() {
        let ((p, a), r) = v[i];
        if i == 0 || post_type_id.get(p) != post_type_id.get(v[i - 1].0 .0) {
            first = r - 1;
        }
        tr.push(((p, a), r - first));
    }
    let v = ranked(tr, |&((p, a), _)| (Reverse(score.get(p).unwrap()), Reverse(a[0])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|(((p, a), r), o)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "owner"]);
        f.extend([V::I(r), V::I(o)]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT tu.DisplayName, tu.TotalPosts, tu.Questions, tu.Answers, tu.TotalScore, tu.AvgViewCount, CAST(u.CreationDate AS VARCHAR(10)) AS AccountCreationDate,
//        CASE WHEN tu.ScoreRank <= 10 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributorStatus
// FROM TopUsers tu JOIN Users u ON tu.UserId = u.Id WHERE tu.TotalPosts > 5 ORDER BY tu.ScoreRank;
//
// DuckDB ignores the VARCHAR length, so the whole timestamp is kept.
fn q6241(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| (a[1] == 0, Reverse(a[4])), false);
    rows(drain(rel(v).filt(|x| x.0 .1[1] > 5)).into_iter().map(|x| x.1).map(|((u, a), r)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), avg(a[6], a[5])];
        f.push(V::S(Box::leak(ts_text(db.user.creation_date.get(u).unwrap()).into_boxed_str())));
        f.push(V::S(if r <= 10 { "Top Contributor" } else { "Contributor" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.Reputation AS OwnerReputation, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.Reputation),
// TopPosts AS (SELECT ..., RANK() OVER (ORDER BY rp.Score DESC) AS RankScore FROM RecentPosts rp)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.OwnerReputation, tp.CommentCount, tp.VoteCount, tp.RankScore
// FROM TopPosts tp WHERE tp.RankScore <= 10 ORDER BY tp.RankScore;
fn q13044(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).select(score)), |&(_, s)| Reverse(s), false);
    let v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect();
    let tp: MatSet<Id<Post>> = rel(v.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    rows(v.into_iter().map(|(p, r)| {
        let (n, m) = s.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "views", "score", "rep"]);
        f.extend([V::I(n), V::I(m), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, AVG(p.Score) AS AveragePostScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotes, TotalDownvotes, AveragePostScore, PostRank FROM TopUsers WHERE PostRank <= 10;
fn q11734(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5] + s],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[5], a[0]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.AnswerCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.CreationDate DESC) AS RankView
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.AnswerCount, rp.OwnerDisplayName, ub.BadgeCount, rp.RankScore, rp.RankView
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerDisplayName = (SELECT U.DisplayName FROM Users U WHERE U.Id = ub.UserId)
// WHERE rp.RankScore <= 10 AND rp.RankView <= 10 ORDER BY rp.RankScore, rp.RankView;
fn q5203(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id.and(owner_user)));
    let ty = |p: Id<Post>| post_type_id.get(p).unwrap();
    fn within<X: Copy>(v: Vec<(X, i64)>, g: impl Fn(&X) -> i64) -> Vec<(X, i64)> {
        let mut first = 0;
        (0..v.len())
            .map(|i| {
                if i == 0 || g(&v[i].0) != g(&v[i - 1].0) {
                    first = v[i].1 - 1;
                }
                (v[i].0, v[i].1 - first)
            })
            .collect()
    }
    let rs = within(ranked(v, |&(p, _)| (ty(p), Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), false), |x| ty(x.0));
    let rv = within(
        ranked(rs, |&((p, _), _)| {
            let w = view_count.get(p);
            (ty(p), w.is_none(), Reverse(w), Reverse(creation_date.get(p).unwrap()), p)
        }, false),
        |x| ty(x.0 .0),
    );
    let keep: Vec<(Id<Post>, (i64, i64))> = rv.into_iter().map(|(((p, _), s), w)| (p, (s, w))).filter(|&(_, (s, w))| s <= 10 && w <= 10).collect();
    let tp = rel(keep);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let bu: MatSet<Id<User>> = db.badge.select(&db.badge.user).collect();
    let by_name: HashIdx<Str, Id<User>> = (&bu).select(&db.user.display_name).inv().collect();
    let mut v = drain((&tp).and((&tp).map(|(p, _)| p).select(owner_user.select(&db.user.display_name).select(&by_name).select(&bc)).opt()));
    v.sort_by_key(|&(_, ((_, r), _))| r);
    rows(v.into_iter().map(|(_, ((p, (s, w)), b))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "answers", "owner"]);
        f.extend([b.map_or(V::Null, V::I), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score,
//        ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><'), 1) AS TagCount,
//        COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.TagCount, rp.OwnerDisplayName FROM RankedPosts rp
//     WHERE rp.TagCount > 5 AND rp.PostRank <= 5)
// SELECT fp.Title, fp.CreationDate, fp.ViewCount, fp.Score, fp.OwnerDisplayName,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = fp.PostId) AS CommentCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = fp.PostId AND v.VoteTypeId = 2) AS UpVoteCount
// FROM FilteredPosts fp ORDER BY fp.Score DESC, fp.ViewCount DESC;
fn q26487(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, tags_str, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt().and(score)));
    let top = top_per(v, |&(_, (u, _))| u, |&(_, (_, s))| Reverse(s), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let fp = (&tp).with(tags_str.map(|t: Str| tag_list(t).count() as i64).gt(5));
    let cc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let up = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).with((&db.vote.vote_type_id).eq(2)).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let mut v = drain((&cc).and(&up));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (c, u))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.push(V::S(owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(c), V::I(u)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN p.Score ELSE 0 END) AS TotalScore,
//        AVG(CASE WHEN p.PostTypeId IN (1, 2) THEN p.Score END) AS AvgScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS TotalScoreRank, RANK() OVER (ORDER BY PostCount DESC) AS PostCountRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, AvgScore, TotalScoreRank, PostCountRank
// FROM TopUsers WHERE TotalScoreRank <= 10 OR PostCountRank <= 10 ORDER BY TotalScore DESC, PostCount DESC;
fn q11749(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score)).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, s)) => {
                let qa = t == 1 || t == 2;
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if qa { s } else { 0 }, a[4] + qa as i64]
            }
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[3]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[0]), false);
    let mut v: Vec<_> = v.into_iter().filter(|&((_, s), p)| s <= 10 || p <= 10).collect();
    v.sort_by_key(|&(((_, a), _), _)| (Reverse(a[3]), Reverse(a[0])));
    rows(v.into_iter().map(|(((u, a), s), p)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[3], a[4]), V::I(s), V::I(p)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT T.DisplayName, T.TotalPosts, T.QuestionCount, T.AnswerCount, T.TotalViews, T.TotalScore,
//        CASE WHEN T.ScoreRank <= 10 THEN 'Top Contributor' WHEN T.ScoreRank <= 50 THEN 'Contributor' ELSE 'Regular User' END AS UserCategory
// FROM TopUsers T WHERE T.TotalPosts > 50 ORDER BY T.TotalScore DESC, T.TotalPosts DESC;
fn q9593(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| (a[0] == 0, Reverse(a[5])), false);
    let mut v: Vec<_> = drain(rel(v).filt(|x| x.0 .1[0] > 50)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&((_, a), _)| (Reverse(a[5]), Reverse(a[0])));
    rows(v.into_iter().map(|((u, a), r)| {
        row(vec![
            user_col(db, u, "name"),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            nullable(a[4], a[3]),
            V::I(a[5]),
            V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Contributor" } else { "Regular User" }),
        ])
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes,
//        CASE WHEN Rank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorType
// FROM TopUsers WHERE PostCount > 0 ORDER BY PostCount DESC, UpVotes DESC;
fn q9004(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let mut v = drain(&s);
    v.sort_by_key(|&(u, a)| (Reverse(a[0]), u));
    let v: Vec<_> = drain(rel(v.into_iter().enumerate().collect::<Vec<_>>()).filt(|x| x.1 .1[0] > 0)).into_iter().map(|x| x.1).collect();
    let mut v = v;
    v.sort_by_key(|&(_, (_, a))| (Reverse(a[0]), Reverse(a[3])));
    rows(v.into_iter().map(|(i, (u, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::S(if i < 10 { "Top Contributor" } else { "Regular Contributor" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.ViewCount) AS TotalViewCount, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.ViewCount, p.Score, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank FROM Posts p)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, us.QuestionCount, us.AnswerCount, us.TotalViewCount, us.TotalScore,
//        tp.PostId, tp.Title, tp.ViewCount AS PostViewCount, tp.Score AS PostScore
// FROM UserStats us LEFT JOIN TopPosts tp ON us.UserId = tp.OwnerUserId AND tp.Rank <= 5 ORDER BY us.Reputation DESC, us.PostCount DESC;
fn q14576(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let v = drain(db.post.select(owner_user.opt().and(score)));
    let top = top_per(v, |&(_, (u, _))| u, |&(_, (_, s))| Reverse(s), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_owner: HashIdx<Id<User>, Id<Post>> = (&tp).select(owner_user).inv().collect();
    let ups = user_posts(db);
    let mut v = drain(db.user.with((&db.user.reputation).gt(0)).select((&ups).and((&by_owner).opt())));
    v.sort_by_key(|&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1])));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1])]);
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "views", "score"])),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopUsers AS (SELECT OwnerUserId, COUNT(*) AS PostCount, SUM(Score) AS TotalScore FROM RankedPosts WHERE Rank <= 5 GROUP BY OwnerUserId)
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, tu.PostCount, tu.TotalScore, COALESCE(b.Count, 0) AS BadgeCount,
//        COALESCE((SELECT AVG(CAST(Score AS FLOAT)) FROM RankedPosts r WHERE r.OwnerUserId = u.Id), 0) AS AveragePostScore
// FROM Users u JOIN TopUsers tu ON u.Id = tu.OwnerUserId LEFT JOIN (SELECT UserId, COUNT(*) AS Count FROM Badges GROUP BY UserId) b ON b.UserId = u.Id
// WHERE u.Reputation > 1000 ORDER BY tu.TotalScore DESC, u.DisplayName ASC;
//
// Scores are small integers, exact as FLOAT, so their average is the exact one.
fn q9738(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1).and(score.gt(0)));
    let v = drain(rp().select(owner_user.and(score).and(creation_date)));
    let top = top_per(v, |&(_, ((u, _), _))| u, |&(_, ((_, s), d))| (Reverse(s), Reverse(d)), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tu = (&tp).group_by(owner_user).select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let all = rp().group_by(owner_user).select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&tu).and((&bc).opt()).and(&all)));
    v.sort_by_key(|&(u, (((_, s), _), _))| (Reverse(s), db.user.display_name.get(u).unwrap()));
    rows(v.into_iter().map(|(u, (((n, s), b), (m, t)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(s), V::I(b.unwrap_or(0)), avg(t, m)]);
        row(f)
    }))
}

// WITH PostSummary AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        COUNT(c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2021-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName),
// PostRanked AS (SELECT *, RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostSummary)
// SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerDisplayName, TotalComments, TotalUpVotes, TotalDownVotes, Rank
// FROM PostRanked WHERE Rank <= 100 ORDER BY Rank;
fn q13452(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(date(2021, 1, 1))).select(score.and(view_count.opt()))), |&(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), false);
    let v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 100).map(|((p, _), r)| (p, r)).collect();
    let tp: MatSet<Id<Post>> = rel(v.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(v.into_iter().map(|(p, r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend(s.get(p).unwrap().map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RECURSIVE UserHierarchy AS (SELECT Id, DisplayName, Reputation, CreationDate, LastAccessDate, WebsiteUrl, Location, UpVotes, DownVotes, 1 AS Level
//     FROM Users WHERE Reputation > 1000
//     UNION ALL SELECT U.Id, ..., UH.Level + 1 FROM Users U JOIN UserHierarchy UH ON U.Reputation < UH.Reputation)
// SELECT U.DisplayName, U.Reputation, U.Location, COUNT(DISTINCT PH.PostId) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, AVG(PH.CommentEvaluation) AS AvgCommentEval
// FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN (SELECT PH.UserId, PH.PostId, CASE WHEN PH.PostHistoryTypeId = 10 THEN COUNT(*) ELSE 0 END AS CommentEvaluation
//            FROM PostHistory PH GROUP BY PH.UserId, PH.PostId, PH.PostHistoryTypeId) AS PH ON P.Id = PH.PostId
// WHERE U.Reputation > 500 GROUP BY U.DisplayName, U.Reputation, U.Location HAVING COUNT(DISTINCT P.Id) > 5
// ORDER BY TotalScore DESC, U.Reputation ASC OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY;
//
// UserHierarchy is never read.
fn q30944(db: &'static So) -> String {
    let PostHistory { user_id, post, post_history_type_id, .. } = &db.post_history;
    let phg = rel(drain(db.post_history.group_by(user_id.opt().and(post).and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1)));
    let ph_of: HashIdx<Id<Post>, usize> = (&phg).map(|(((_, p), _), _)| p).inv().collect();
    let evals = (&phg).map(|(((_, _), t), n)| if t == 10 { n } else { 0 });
    let User { display_name, reputation, location, .. } = &db.user;
    let s = db
        .user
        .with(reputation.gt(500))
        .group_by(display_name.and(reputation).and(location.opt()))
        .select(posts_of(db).select(Ident::<Post>::new().and(&db.post.score).and((&ph_of).select(evals).opt())).opt())
        .buf_fold(|it| {
            let (mut ps, mut hp) = (Vec::new(), Vec::new());
            let (mut tot, mut en, mut es) = (0i64, 0i64, 0i64);
            for x in it {
                if let Some(((p, sc), e)) = x {
                    ps.push(p);
                    tot += sc;
                    if let Some(e) = e {
                        hp.push(p);
                        en += 1;
                        es += e;
                    }
                }
            }
            ps.sort_unstable();
            ps.dedup();
            hp.sort_unstable();
            hp.dedup();
            (ps.len() as i64, hp.len() as i64, tot, en, es)
        });
    let mut v = drain((&s).filt(|x| x.0 > 5));
    v.sort_by_key(|&(((_, r), _), (_, _, t, _, _))| (Reverse(t), r));
    rows(v.into_iter().skip(5).take(10).map(|(((n, r), l), (_, h, t, en, es))| row(vec![V::S(n), V::I(r), l.map_or(V::Null, V::S), V::I(h), V::I(t), avg(es, en)])))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AverageScore FROM Posts p GROUP BY p.OwnerUserId),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, ub.BadgeCount, ps.PostCount, ps.QuestionCount, ps.AnswerCount, ps.TotalViews, ps.AverageScore
//     FROM Users u JOIN UserBadges ub ON u.Id = ub.UserId JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT au.DisplayName, au.BadgeCount, au.PostCount, au.QuestionCount, au.AnswerCount, au.TotalViews, au.AverageScore,
//        RANK() OVER (ORDER BY au.PostCount DESC, au.TotalViews DESC) AS UserRank
// FROM ActiveUsers au WHERE au.BadgeCount > 0 ORDER BY UserRank LIMIT 10;
fn q5834(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let v = ranked(drain((&bc).filt(|n| n > 0).and((&ups).filt(|a| a[1] > 0))), |&(_, (_, a))| (Reverse(a[1]), a[5] == 0, Reverse(a[6])), false);
    rows(v.into_iter().take(10).map(|((u, (b, a)), r)| {
        row(vec![user_col(db, u, "name"), V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), avg(a[4], a[1]), V::I(r)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.Id) AS RN,
//        COUNT(c.Id) AS CommentCount, COALESCE(SUM(case when v.VoteTypeId = 2 then 1 else 0 end), 0) AS UpVotes,
//        COALESCE(SUM(case when v.VoteTypeId = 3 then 1 else 0 end), 0) AS DownVotes
//     FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, pt.Name)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes, (rp.UpVotes - rp.DownVotes) AS NetScore,
//        CASE WHEN rp.RN = 1 THEN 'Top Post' ELSE 'Regular Post' END AS PostRank
// FROM RankedPosts rp WHERE rp.Score > 0 OR (rp.CommentCount > 5 AND rp.Score IS NULL)
// ORDER BY NetScore DESC, rp.CreationDate DESC, rp.PostId FETCH FIRST 100 ROWS ONLY;
//
// rewrites/2254.sql: the window and the final order tie-broken on the Id.
fn q2254(db: &'static So) -> String {
    let Post { creation_date, score, origid, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain(&s);
    let tops = top_per(v, |&(p, _)| db.post.post_type.get(p).map(|t| db.post_type.name.get(t).unwrap()), |&(p, _)| (Reverse(score.get(p).unwrap()), origid.get(p).unwrap()), 1, false);
    let tops: MatSet<Id<Post>> = rel(tops.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain(db.post.with(score.gt(0)).select(&s).and(Ident::<Post>::new().with(&tops).opt()));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[1] - a[2]), Reverse(creation_date.get(p).unwrap()), origid.get(p).unwrap()), 100);
    rows(v.into_iter().map(|(p, (a, top))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(a.map(V::I));
        f.push(V::I(a[1] - a[2]));
        f.push(V::S(if top.is_some() { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT t.PostId, t.Title, t.CommentCount, t.UpVotes, t.DownVotes,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = t.PostId AND v.VoteTypeId = 10) AS DeletionVotes,
//        (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = t.PostId AND ph.PostHistoryTypeId = 10) AS CloseVotes
// FROM TopPosts t ORDER BY t.CommentCount DESC, (t.UpVotes - t.DownVotes) DESC;
fn q9904(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let v = drain(db.post.select(post_type_id.and(score).and(creation_date)));
    let top = top_per(v, |&(_, ((t, _), _))| t, |&(_, ((_, s), d))| (Reverse(s), Reverse(d)), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let g = || (&tp).group_by(Ident::<Post>::new());
    let s = g()
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let dv = g().select(votes_of(db).with((&db.vote.vote_type_id).eq(10)).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let cv = g().select(history_of(db).with((&db.post_history.post_history_type_id).eq(10)).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let mut v = drain((&s).and(&dv).and(&cv));
    v.sort_by_key(|&(_, ((a, _), _))| (Reverse(a[0]), Reverse(a[1] - a[2])));
    rows(v.into_iter().map(|(p, ((a, d), c))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(a.map(V::I));
        f.extend([V::I(d), V::I(c)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT ub.UserId, COUNT(ub.Id) AS BadgeCount FROM Badges ub GROUP BY ub.UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COALESCE(bc.BadgeCount, 0) AS BadgeCount, COALESCE(ps.PostCount, 0) AS PostCount,
//        COALESCE(ps.TotalScore, 0) AS TotalScore, COALESCE(ps.TotalViews, 0) AS TotalViews
//     FROM Users u LEFT JOIN UserBadgeCounts bc ON u.Id = bc.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.CreationDate, ua.BadgeCount, ua.PostCount, ua.TotalScore, ua.TotalViews,
//        DENSE_RANK() OVER (ORDER BY ua.Reputation DESC) AS ReputationRank, DENSE_RANK() OVER (ORDER BY ua.BadgeCount DESC) AS BadgeRank,
//        DENSE_RANK() OVER (ORDER BY ua.TotalScore DESC) AS ScoreRank
// FROM UserActivity ua WHERE ua.Reputation >= 100 ORDER BY ua.Reputation DESC, ua.TotalScore DESC LIMIT 100;
fn q9400(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let v = drain(db.user.with((&db.user.reputation).ge(100)).select((&bc).and(&ups)));
    let v = ranked(v, |&(u, _)| Reverse(rep(u)), true);
    let v = ranked(v, |&((_, (b, _)), _)| Reverse(b), true);
    let v = ranked(v, |&(((_, (_, a)), _), _)| Reverse(a[4]), true);
    let v = top_n(v, |&((((u, (_, a)), _), _), _)| (Reverse(rep(u)), Reverse(a[4])), 100);
    rows(v.into_iter().map(|((((u, (b, a)), r), bk), sk)| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[4]), V::I(a[6]), V::I(r), V::I(bk), V::I(sk)]);
        row(f)
    }))
}

// WITH UserPostStatistics AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews FROM UserPostStatistics)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, PositivePosts, TotalViews, AverageScore, RankByPosts, RankByViews
// FROM TopUsers WHERE RankByPosts <= 10 OR RankByViews <= 10 ORDER BY RankByPosts, RankByViews;
fn q13474(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| Reverse(a[1]), false);
    let v = ranked(v, |&((_, a), _)| (a[5] == 0, Reverse(a[6])), false);
    let mut v: Vec<_> = v.into_iter().filter(|&((_, p), w)| p <= 10 || w <= 10).collect();
    v.sort_by_key(|&((_, p), w)| (p, w));
    rows(v.into_iter().map(|(((u, a), p), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[8]), nullable(a[6], a[5]), avg(a[4], a[1]), V::I(p), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= '2023-01-01'),
// UserScores AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.CreationDate >= '2023-01-01' GROUP BY u.Id, u.DisplayName),
// HighScorers AS (SELECT us.UserId, us.DisplayName, us.TotalScore FROM UserScores us WHERE us.TotalScore > 1000)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, hs.DisplayName AS TopUser, hs.TotalScore
// FROM RankedPosts rp LEFT JOIN HighScorers hs ON rp.OwnerUserId = hs.UserId WHERE rp.Rank = 1 ORDER BY rp.Score DESC LIMIT 10;
//
// Rank = 1 keeps one joined row per owner: one of the owner's top-scoring posts.
fn q4372(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(date(2023, 1, 1)));
    let v = drain(recent().select(owner_user.opt().and(score)));
    let top = top_per(v, |&(_, (u, _))| u, |&(p, (_, s))| (Reverse(s), p), 1, false);
    let top = top_n(top, |&(p, (_, s))| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let us = recent().group_by(owner_user).select(score).fold(0i64, |s, x| s + x);
    let mut v = drain((&cc).and(owner_user.select(Ident::<User>::new().and((&us).filt(|s| s > 1000))).opt()));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), p));
    rows(v.into_iter().map(|(p, (c, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(c));
        f.extend(match h {
            Some((u, s)) => [user_col(db, u, "name"), V::I(s)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// CombinedData AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, COALESCE(rc.CommentCount, 0) AS CommentCount,
//        rc.LastCommentDate, rp.PostRank FROM RankedPosts rp LEFT JOIN RecentComments rc ON rp.PostId = rc.PostId)
// SELECT cd.Title, cd.CreationDate, cd.Score, cd.ViewCount, cd.OwnerDisplayName, cd.CommentCount, cd.PostRank
// FROM CombinedData cd WHERE cd.PostRank = 1 ORDER BY cd.Score DESC, cd.ViewCount DESC LIMIT 10;
fn q5345(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.and(creation_date)));
    let top = top_per(v, |&(_, (u, _))| u, |&(_, (_, d))| Reverse(d), 1, false);
    let top = top_n(top, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10);
    let tp = rel(top.into_iter().map(|x| x.0).collect());
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&tp).select(Ident::<Post>::new().and((&cc).opt())));
    v.sort_by_key(|x| x.0);
    rows(v.into_iter().map(|(_, (p, c))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(1)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate ASC) AS PostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u)
// SELECT tu.DisplayName, tu.Reputation, rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, rp.CreationDate
// FROM RankedPosts rp JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId WHERE tu.ReputationRank <= 10
// ORDER BY tu.Reputation DESC, rp.Score DESC, rp.CreationDate DESC LIMIT 50;
//
// The order reads no aggregate, so the fifty posts are picked before their
// joined rows are counted.
fn q6752(db: &'static So) -> String {
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let tu: Vec<Id<User>> = ranked(drain(db.user.select(&db.user.reputation)), |&(_, r)| Reverse(r), false).into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect();
    let tu = rel(tu);
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let v = drain((&tu).select(Ident::<User>::new().and(posts_of(db).with(post_type_id.is_in([1, 2])))));
    let v = top_n(v, |&(_, (u, p))| (Reverse(rep(u)), Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 50);
    let tp = v.into_iter().map(|x| x.1).collect::<Vec<_>>();
    let tpp: MatSet<Id<Post>> = rel(tp.clone()).map(|(_, p)| p).collect();
    let agg = (&tpp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(n, m), (c, t)| (n + c.is_some() as i64, m + (t == Some(2)) as i64));
    rows(tp.into_iter().map(|(u, p)| {
        let (n, m) = agg.get(p).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend([V::I(n), V::I(m)]);
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts, RANK() OVER (ORDER BY UpVoteCount DESC) AS RankByUpVotes FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVoteCount, DownVoteCount, RankByPosts, RankByUpVotes
// FROM TopUsers WHERE RankByPosts <= 10 OR RankByUpVotes <= 10 ORDER BY RankByPosts, RankByUpVotes;
fn q13238(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[3]), false);
    let mut v: Vec<_> = v.into_iter().filter(|&((_, p), q)| p <= 10 || q <= 10).collect();
    v.sort_by_key(|&((_, p), q)| (p, q));
    rows(v.into_iter().map(|(((u, a), p), q)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(p), V::I(q)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName,
//        ph.UserDisplayName AS LastEditor, ph.CreationDate AS LastEditDate
// FROM RankedPosts rp LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId
//     AND ph.CreationDate = (SELECT MAX(ph2.CreationDate) FROM PostHistory ph2 WHERE ph2.PostId = rp.PostId AND ph2.PostHistoryTypeId IN (4, 5))
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
fn q8541(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).select(post_type_id.and(score).and(creation_date).and(owner_user)));
    let top = top_per(v, |&(_, (((t, _), _), _))| t, |&(_, (((_, s), d), _))| (Reverse(s), Reverse(d)), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let md = db.post_history.with(post_history_type_id.is_in([4, 5])).group_by(post).select(&db.post_history.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(&db.post_history.creation_date)).inv().collect();
    let mut v = drain((&tp).select(Ident::<Post>::new().and(&md).select(&at).opt()));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend(match h {
            Some(h) => [harness::fmt::ostr(db.post_history.user_display_name.get(h)), V::T(db.post_history.creation_date.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT ur.UserId, ur.Reputation, ur.BadgeCount, rp.PostId, rp.Title, rp.CreationDate FROM UserReputation ur JOIN RecentPosts rp ON ur.UserId = rp.OwnerUserId
//     WHERE ur.Reputation > 1000 ORDER BY ur.Reputation DESC LIMIT 10)
// SELECT tu.UserId, u.DisplayName, tu.Reputation, tu.BadgeCount, tu.Title, tu.CreationDate, p.Tags, v.VoteTypeId, COUNT(v.Id) AS VoteCount
// FROM TopUsers tu JOIN Users u ON tu.UserId = u.Id LEFT JOIN Posts p ON tu.PostId = p.Id LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY tu.UserId, u.DisplayName, tu.Reputation, tu.BadgeCount, tu.Title, tu.CreationDate, p.Tags, v.VoteTypeId
// ORDER BY tu.Reputation DESC, tu.CreationDate DESC;
fn q5589(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, tags_str, .. } = &db.post;
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let v = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))));
    let v = top_n(v, |&(p, u)| (Reverse(rep(u)), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let g = (&tp)
        .select(owner_user.and(title.opt()).and(creation_date).and(tags_str.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .group_by(Same::new())
        .fold(0i64, |n, k: ((((Id<User>, Option<Str>), i64), Option<Str>), Option<i64>)| n + k.1.is_some() as i64);
    let mut v = drain(&g);
    v.sort_by_key(|&(((((u, _), d), _), _), _)| (Reverse(rep(u)), Reverse(d)));
    rows(v.into_iter().map(|(((((u, t), d), tg), vt), n)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(bc.get(u).unwrap()), t.map_or(V::Null, V::S), V::T(d), tg.map_or(V::Null, V::S), vt.map_or(V::Null, V::I), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, U.DisplayName AS OwnerDisplayName, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS TagRank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1),
// TopPostsPerTag AS (SELECT PostId, Title, CreationDate, Score, OwnerDisplayName, Tags FROM RankedPosts WHERE TagRank = 1),
// PostCommentStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY p.Id)
// SELECT t.PostId, t.Title, t.CreationDate, t.Score, t.OwnerDisplayName, t.Tags, pcs.CommentCount, pcs.TotalBountyAmount
// FROM TopPostsPerTag t JOIN PostCommentStats pcs ON t.PostId = pcs.PostId ORDER BY t.CreationDate DESC, pcs.TotalBountyAmount DESC;
fn q28896(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, tags_str, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.and(tags_str.opt()).and(creation_date)));
    let top = top_per(v, |&(_, ((_, t), _))| t, |&(p, (_, d))| (Reverse(d), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).is_in([8, 9])).select(&db.vote.post).inv().collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&bounty).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64), |(n, b), (c, v)| (n + c.is_some() as i64, b + v.flatten().unwrap_or(0)));
    let mut v = drain(&s);
    v.sort_by_key(|&(p, (_, b))| (Reverse(creation_date.get(p).unwrap()), Reverse(b)));
    rows(v.into_iter().map(|(p, (n, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner", "tags"]);
        f.extend([V::I(n), V::I(b)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AvgScore,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY PostCount DESC) AS RankByPostCount, RANK() OVER (ORDER BY TotalViews DESC) AS RankByTotalViews,
//        RANK() OVER (ORDER BY AvgScore DESC) AS RankByAvgScore FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, TotalViews, AvgScore, QuestionCount, AnswerCount, RankByPostCount, RankByTotalViews, RankByAvgScore
// FROM TopUsers WHERE RankByPostCount <= 10 OR RankByTotalViews <= 10 OR RankByAvgScore <= 10 ORDER BY RankByPostCount, RankByTotalViews, RankByAvgScore;
fn q11936(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| Reverse(a[1]), false);
    let v = ranked(v, |&((_, a), _)| (a[5] == 0, Reverse(a[6])), false);
    let v = ranked(v, |&(((_, a), _), _)| (a[1] == 0, Reverse(fkey(a[4] as f64 / a[1].max(1) as f64))), false);
    let mut v: Vec<_> = v.into_iter().filter(|&(((_, p), w), s)| p <= 10 || w <= 10 || s <= 10).collect();
    v.sort_by_key(|&(((_, p), w), s)| (p, w, s));
    rows(v.into_iter().map(|((((u, a), p), w), s)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), nullable(a[6], a[5]), avg(a[4], a[1]), V::I(a[2]), V::I(a[3]), V::I(p), V::I(w), V::I(s)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// TopPosts AS (SELECT ..., RANK() OVER (ORDER BY Score DESC) AS ScoreRank, RANK() OVER (ORDER BY ViewCount DESC) AS ViewRank FROM RankedPosts)
// SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, CommentCount, VoteCount, ScoreRank, ViewRank
// FROM TopPosts WHERE ScoreRank <= 10 OR ViewRank <= 10 ORDER BY ScoreRank, ViewRank;
fn q14139(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score.and(view_count.opt())));
    let v = ranked(v, |&(_, (s, _))| Reverse(s), false);
    let v = ranked(v, |&((_, (_, w)), _)| (w.is_none(), Reverse(w)), false);
    let mut v: Vec<_> = v.into_iter().filter(|&((_, s), w)| s <= 10 || w <= 10).map(|(((p, _), s), w)| (p, (s, w))).collect();
    v.sort_by_key(|&(_, r)| r);
    let tp: MatSet<Id<Post>> = rel(v.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let agg = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    rows(v.into_iter().map(|(p, (s, w))| {
        let (n, m) = agg.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(n), V::I(m), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// PostStatistics AS (SELECT rp.OwnerUserId, COUNT(rp.Id) AS TotalQuestions, SUM(rp.Score) AS TotalScore, AVG(rp.ViewCount) AS AverageViewCount FROM RankedPosts rp GROUP BY rp.OwnerUserId),
// TopUsers AS (SELECT ps.OwnerUserId, ps.TotalQuestions, ps.TotalScore, ps.AverageViewCount, RANK() OVER (ORDER BY ps.TotalScore DESC) AS UserRank FROM PostStatistics ps)
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, tu.TotalQuestions, tu.TotalScore, tu.AverageViewCount, tu.UserRank
// FROM Users u LEFT JOIN TopUsers tu ON u.Id = tu.OwnerUserId WHERE (u.Reputation >= 100 OR tu.TotalQuestions IS NOT NULL) ORDER BY tu.UserRank ASC NULLS LAST;
fn q33451(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let ps = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let tu = rel(ranked(drain(&ps), |&(_, a)| Reverse(a[1]), false));
    let by_user: HashIdx<Id<User>, usize> = (&tu).map(|((u, _), _)| u).inv().collect();
    let mut v = drain(db.user.select(Ident::<User>::new().and(&db.user.reputation).and((&by_user).select(&tu).opt())).filt(|((_, r), t)| r >= 100 || t.is_some()));
    v.sort_by_key(|&(_, (_, t))| t.map_or(i64::MAX, |x| x.1));
    rows(v.into_iter().map(|(u, (_, t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(match t {
            Some(((_, a), r)) => [V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(r)],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        DENSE_RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.AnswerCount,
//        ROW_NUMBER() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS RN FROM RankedPosts rp WHERE rp.ScoreRank <= 10)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        u.Reputation AS OwnerReputation, u.Location AS OwnerLocation
// FROM TopPosts tp JOIN Users u ON tp.Id = u.Id ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q6279(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, origid, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, _)| key(p), true);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let dc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(distinct_some);
    let da = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).buf_fold(distinct_some);
    let uids: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let mut v = drain((&dc).and(&da).and(origid.select(&uids)));
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, ((c, a), u))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(harness::fmt::ostr(db.user.location.get(u)));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN p.AnswerCount ELSE 0 END) AS QuestionsAsked, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, BadgeCount, QuestionsAsked, AnswersGiven, UpVotesReceived, DownVotesReceived
// FROM TopUsers WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
//
// The rank only reads Reputation, so the ten users are picked before their
// joined rows are aggregated.
fn q9512(db: &'static So) -> String {
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let top: Vec<Id<User>> = ranked(drain(db.user.select(&db.user.reputation)), |&(_, r)| Reverse(r), false).into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect();
    let tu: MatSet<Id<User>> = rel(top).map(|u| u).collect();
    let Post { post_type_id, answer_count, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(answer_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold((0i64, None, 0i64, 0i64, 0i64), |(b, q, a, u, d): (i64, Option<i64>, i64, i64, i64), (x, p)| {
            let (qa, t, v) = match p {
                Some(((t, n), v)) => (if t == 1 { n } else { Some(0) }, Some(t), v),
                None => (None, None, None),
            };
            (b + x.is_some() as i64, match qa {
                Some(n) => Some(q.unwrap_or(0) + n),
                None => q,
            }, a + (t == Some(2)) as i64, u + (v == Some(2)) as i64, d + (v == Some(3)) as i64)
        });
    let mut v = drain(&s);
    v.sort_by_key(|&(u, _)| Reverse(rep(u)));
    rows(v.into_iter().map(|(u, (b, q, a, up, dn))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), q.map_or(V::Null, V::I), V::I(a), V::I(up), V::I(dn)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PostHistoryAggregated AS (SELECT p.Id AS PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEdited
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, pha.EditCount, pha.LastEdited, rp.RankScore
// FROM RankedPosts rp LEFT JOIN PostHistoryAggregated pha ON rp.PostId = pha.PostId WHERE rp.RankScore <= 10 ORDER BY rp.RankScore, rp.Score DESC;
fn q5666(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).select(post_type_id.and(owner_user)));
    let ty = |p: Id<Post>| post_type_id.get(p).unwrap();
    let v = ranked(v, |&(p, _)| (ty(p), Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()), false);
    let mut first = 0;
    let mut tp = Vec::new();
    for i in 0..v.len() {
        let ((p, _), r) = v[i];
        if i == 0 || ty(p) != ty(v[i - 1].0 .0) {
            first = r - 1;
        }
        if r - first <= 10 {
            tp.push((p, r - first));
        }
    }
    let tp = rel(tp);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let pha = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = drain((&tp).and((&tp).map(|(p, _)| p).select(&pha).opt()));
    v.sort_by_key(|&(_, ((p, r), _))| (r, Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((p, r), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerName, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT PostId, Title, CreationDate, OwnerName, Score, ViewCount FROM RankedPosts WHERE Rank <= 5),
// PostVotes AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerName, tp.Score, tp.ViewCount, pv.VoteCount
// FROM TopPosts tp JOIN PostVotes pv ON tp.PostId = pv.PostId ORDER BY tp.Score DESC, pv.VoteCount DESC;
fn q6743(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).select(post_type_id.and(score).and(creation_date).and(owner_user)));
    let top = top_per(v, |&(_, (((t, _), _), _))| t, |&(_, (((_, s), d), _))| (Reverse(s), Reverse(d)), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let mut v = drain(&vc);
    v.sort_by_key(|&(p, n)| (Reverse(score.get(p).unwrap()), Reverse(n)));
    rows(v.into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON a.ParentId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName, p.OwnerUserId),
// FilteredPosts AS (SELECT PostId, Title, CreationDate, ViewCount, OwnerDisplayName, AnswerCount, UpVotes, DownVotes FROM RankedPosts WHERE rn <= 5)
// SELECT f.OwnerDisplayName, COUNT(f.PostId) AS TotalPosts, SUM(f.ViewCount) AS TotalViews, AVG(f.AnswerCount * (f.UpVotes - f.DownVotes)) AS EngagementScore
// FROM FilteredPosts f GROUP BY f.OwnerDisplayName ORDER BY TotalViews DESC, EngagementScore DESC;
//
// The row number only reads CreationDate, so the posts are picked before
// their joined rows are counted.
fn q5028(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.and(creation_date)));
    let top = top_per(v, |&(_, (u, _))| u, |&(_, (_, d))| Reverse(d), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let e = (&tp)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(a, n), (c, t)| (a + c.is_some() as i64, n + (t == Some(2)) as i64 - (t == Some(3)) as i64));
    let g = (&tp)
        .group_by(owner_user.select(&db.user.display_name))
        .select(view_count.opt().and(&e))
        .fold([0i64; 4], |a, (w, (x, n))| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + x * n]);
    let mut v = drain(&g);
    v.sort_by(|a, b| {
        let k = |x: &(Str, [i64; 4])| (x.1[1] == 0, Reverse(x.1[2]));
        k(a).cmp(&k(b)).then((b.1[3] as f64 / b.1[0] as f64).total_cmp(&(a.1[3] as f64 / a.1[0] as f64)))
    });
    rows(v.into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0])])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.OwnerUserId, p.CreationDate),
// TopPosts AS (SELECT Id, Title, Score, ViewCount, CommentCount, VoteCount FROM RankedPosts WHERE PostRank <= 5),
// PostUserDetails AS (SELECT u.DisplayName, u.Reputation, tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, tp.VoteCount FROM TopPosts tp JOIN Users u ON tp.Id = u.Id)
// SELECT pud.DisplayName, pud.Reputation, pud.Title, pud.Score, pud.ViewCount, pud.CommentCount, pud.VoteCount FROM PostUserDetails pud
// ORDER BY pud.Reputation DESC, pud.Score DESC;
//
// Recent post Ids are far above every User Id, so the join is empty.
fn q7372(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt().and(score).and(creation_date)));
    let top = top_per(v, |&(_, ((u, _), _))| u, |&(_, ((_, s), d))| (Reverse(s), Reverse(d)), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uids: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let up: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.post).inv().collect();
    let joined: MatSet<Id<Post>> = (&tp).with(origid.select(&uids).map(|_| true)).collect();
    let s = (&joined)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&up).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    let mut v = drain((&s).and(origid.select(&uids)));
    v.sort_by_key(|&(p, (_, u))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((n, m), u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(n), V::I(m)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Title, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// TopPosts AS (SELECT rp.Title, rp.OwnerDisplayName, rb.BadgeCount, rp.CreationDate, rp.Score, rp.ViewCount
//     FROM RankedPosts rp LEFT JOIN UserBadges rb ON rp.OwnerUserId = rb.UserId WHERE rp.Rank <= 5)
// SELECT tp.Title, tp.OwnerDisplayName, tp.BadgeCount, tp.CreationDate, tp.Score, tp.ViewCount,
//        EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - tp.CreationDate)) / 3600 AS AgeInHours
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q9753(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).select(post_type_id.and(owner_user)));
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let top = top_per(v, |&(_, (t, _))| t, |&(p, _)| (key(p), p), 5, false);
    let tp = rel(top);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&tp).and((&tp).map(|(_, (_, u))| u).select(&bc).opt()));
    v.sort_by_key(|&(_, ((p, _), _))| key(p));
    rows(v.into_iter().map(|(_, ((p, _), b))| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.push(b.map_or(V::Null, V::I));
        f.extend(post_fields(db, p, &["created", "score", "views"]));
        f.push(V::F(secs(ts(2024, 10, 1, 12, 34, 56) - creation_date.get(p).unwrap()) / 3600.0));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(p.Score, 0) AS Score, COALESCE(p.ViewCount, 0) AS ViewCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(p.Score, 0) DESC, COALESCE(p.ViewCount, 0) DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= '2023-01-01 00:00:00'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, u.DisplayName AS OwnerDisplayName, ub.BadgeCount, pc.CommentCount
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId
// WHERE rp.Rank <= 10 ORDER BY rp.Rank, rp.Score DESC;
fn q28326(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(date(2023, 1, 1)))).select(score.and(view_count.opt()))), |&(_, (s, w))| (Reverse(s), Reverse(w.unwrap_or(0))), 10);
    let tp = rel(v.into_iter().map(|x| x.0).collect());
    let ub = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and((&ub).opt()))).and((&pc).opt())));
    v.sort_by_key(|x| x.0);
    rows(v.into_iter().map(|(_, ((p, (u, b)), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(view_count.get(p).unwrap_or(0)));
        f.extend([user_col(db, u, "name"), b.map_or(V::Null, V::I), c.map_or(V::Null, V::I)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(b.Class), 0) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT up.UserId, up.Reputation, up.BadgeCount, rp.Title, rp.Score, ps.CommentCount, ps.VoteCount
// FROM UserReputation up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId LEFT JOIN PostStats ps ON rp.Id = ps.PostId
// WHERE rp.rn = 1 AND up.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY up.Reputation DESC, rp.Score DESC LIMIT 10;
//
// The order reads no aggregate, so the ten posts are picked before their
// joined rows are counted.
fn q1495(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let (n, s) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let mean = s as f64 / n as f64;
    let above = Ident::<User>::new().with((&db.user.reputation).filt(move |r| r as f64 > mean));
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.select(above).and(creation_date)));
    let top = top_per(v, |&(_, (u, _))| u, |&(p, (_, d))| (Reverse(d), p), 1, false);
    let top = top_n(top, |&(p, (u, _))| (Reverse(rep(u)), Reverse(score.get(p).unwrap())), 10);
    let tp: MatSet<Id<Post>> = rel(top.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    let bs = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + c.unwrap_or(0));
    rows(top.into_iter().map(|(p, (u, _))| {
        let (n, m) = ps.get(p).unwrap();
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(bs.get(u).unwrap()));
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(n), V::I(m)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount,
//        RANK() OVER (ORDER BY COUNT(c.Id) DESC) AS CommentRank, p.OwnerUserId
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT rp.*, pt.Name AS PostType, COALESCE(badgeCount.BadgeCount, 0) AS UserBadges
//     FROM RankedPosts rp JOIN PostTypes pt ON pt.Id = 1
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) badgeCount ON badgeCount.UserId = rp.OwnerUserId)
// SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerName, CommentCount, PostType, UserBadges FROM TopPosts
// WHERE UserBadges > 0 ORDER BY Score DESC, CommentCount DESC LIMIT 10;
fn q7259(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let cc = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let t1 = drain(db.post_type.with((&db.post_type.origid).eq(1)).select(&db.post_type.name));
    let v = drain((&cc).and(owner_user.select(&bc)));
    let v = top_n(v, |&(p, (c, _))| (Reverse(score.get(p).unwrap()), Reverse(c)), 10);
    let v = cross_top(v, |_| 0, t1, |_| 0, usize::MAX);
    rows(v.into_iter().map(|((p, (c, b)), (_, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::S(t), V::I(b)]);
        row(f)
    }))
}

// WITH UserPostStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS Wikis,
//        SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStatistics)
// SELECT tu.DisplayName, tu.TotalPosts, tu.Questions, tu.Answers, tu.Wikis, tu.PositiveScorePosts, tu.TotalViews,
//        CASE WHEN tu.Rank <= 10 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributorType
// FROM TopUsers tu WHERE tu.TotalViews > 1000 ORDER BY tu.TotalPosts DESC, tu.TotalViews DESC;
fn q6330(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (3..=5).contains(&t) as i64, a[4] + (s > 0) as i64, a[5] + w.is_some() as i64, a[6] + w.unwrap_or(0)],
            None => a,
        });
    let mut v = drain(&s);
    v.sort_by_key(|&(u, a)| (Reverse(a[0]), u));
    let mut v: Vec<_> = drain(rel(v.into_iter().enumerate().collect::<Vec<_>>()).filt(|x| x.1 .1[5] > 0 && x.1 .1[6] > 1000)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(_, (_, a))| (Reverse(a[0]), Reverse(a[6])));
    rows(v.into_iter().map(|(i, (u, a))| {
        row(vec![
            user_col(db, u, "name"),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            V::I(a[4]),
            V::I(a[6]),
            V::S(if i < 10 { "Top Contributor" } else { "Contributor" }),
        ])
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        COUNT(c.Id) AS TotalComments, COUNT(v.Id) AS TotalVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName),
// TopPosts AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostStats)
// SELECT PostId, Title, CreationDate, ViewCount, Score, AnswerCount, CommentCount, OwnerDisplayName, TotalComments, TotalVotes FROM TopPosts WHERE Rank <= 10;
fn q14213(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let v = top_n(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score.and(view_count.opt()))), |&(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    rows(drain(&s).into_iter().map(|(p, (n, m))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner"]);
        f.extend([V::I(n), V::I(m)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, COUNT(v.Id) AS VoteCount,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.PostTypeId = 1 AND (p.ClosedDate IS NULL OR p.ClosedDate > '2024-10-01 12:34:56') GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId),
// UserDetails AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, UPPER(u.Location) AS Location, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Location)
// SELECT rp.PostId, rp.Title, rp.Score, ud.DisplayName AS OwnerDisplayName, ud.Reputation AS OwnerReputation, ud.Location, rp.VoteCount, rp.CreationDate, rp.ScoreRank, ud.BadgeCount
// FROM RankedPosts rp JOIN UserDetails ud ON rp.OwnerUserId = ud.UserId WHERE rp.ScoreRank <= 5 ORDER BY rp.Score DESC, rp.CreationDate DESC;
fn q9876(db: &'static So) -> String {
    let Post { post_type_id, closed_date, owner_user, score, creation_date, .. } = &db.post;
    let cut = ts(2024, 10, 1, 12, 34, 56);
    let open = db.post.with(post_type_id.eq(1)).with(closed_date.opt().filt(move |c: Option<i64>| c.map_or(true, |c| c > cut)));
    let v = drain(open.select(owner_user.opt().and(score)));
    let v = ranked(v, |&(_, (u, s))| (u, Reverse(s)), false);
    let mut first = 0;
    let mut tp = Vec::new();
    for i in 0..v.len() {
        let ((p, (u, _)), r) = v[i];
        if i == 0 || u != v[i - 1].0 .1 .0 {
            first = r - 1;
        }
        if r - first <= 5 {
            if let Some(u) = u {
                tp.push((p, (u, r - first)));
            }
        }
    }
    let tp = rel(tp);
    let up: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.post).inv().collect();
    let vc = db.post.group_by(Ident::<Post>::new()).select((&up).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&tp).and((&tp).map(|(p, _)| p).select(&vc)).and((&tp).map(|(_, (u, _))| u).select(&bc)));
    v.sort_by_key(|&(_, (((p, _), _), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, (((p, (u, r)), n), b))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(db.user.location.get(u).map_or(V::Null, |l| V::S(Box::leak(l.to_uppercase().into_boxed_str()))));
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["created"]));
        f.extend([V::I(r), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.AnswerCount, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ub.BadgeCount FROM Users u JOIN UserBadges ub ON u.Id = ub.UserId
//     WHERE u.Reputation > 1000 ORDER BY u.Reputation DESC LIMIT 10)
// SELECT tp.UserId, tp.DisplayName, tp.Reputation, tp.BadgeCount, rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.AnswerCount, rp.Score
// FROM TopUsers tp JOIN RankedPosts rp ON tp.UserId = rp.OwnerUserId WHERE rp.rn = 1 ORDER BY tp.Reputation DESC, rp.Score DESC;
fn q7180(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.select(Ident::<User>::new().with(&tu)).and(creation_date)));
    let top = top_per(v, |&(_, (u, _))| u, |&(_, (_, d))| Reverse(d), 1, false);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut top = top;
    top.sort_by_key(|&(p, (u, _))| (Reverse(rep(u)), Reverse(db.post.score.get(p).unwrap())));
    rows(top.into_iter().map(|(p, (u, _))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(bc.get(u).unwrap()));
        f.extend(post_fields(db, p, &["id", "title", "views", "created", "answers", "score"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopRankedPosts AS (SELECT PostId, Title, Score, CreationDate, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// PostVotes AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id)
// SELECT trp.Title, trp.OwnerDisplayName, trp.Score, trp.CreationDate, pv.UpVotes, pv.DownVotes, (pv.UpVotes - pv.DownVotes) AS NetScore
// FROM TopRankedPosts trp JOIN PostVotes pv ON trp.PostId = pv.PostId ORDER BY NetScore DESC, trp.Score DESC;
fn q5808(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user.and(score).and(creation_date)));
    let top = top_per(v, |&(_, ((u, _), _))| u, |&(_, ((_, s), d))| (Reverse(s), Reverse(d)), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let mut v = drain(&pv);
    v.sort_by_key(|&(p, (u, d))| (Reverse(u - d), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (u, d))| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "created"]);
        f.extend([V::I(u), V::I(d), V::I(u - d)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CreationDate FROM RankedPosts WHERE Rank <= 10),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId HAVING COUNT(b.Id) > 3)
// SELECT up.DisplayName, tp.Title AS TopPostTitle, tp.Score, tp.ViewCount, ub.BadgeCount
// FROM Users up JOIN TopPosts tp ON tp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = up.Id) LEFT JOIN UserBadges ub ON ub.UserId = up.Id
// WHERE up.Reputation > 500 ORDER BY tp.Score DESC, up.Reputation DESC LIMIT 100;
//
// No GROUP BY: RankedPosts has a row per comment, so a post can fill more
// than one of its type's ten places.
fn q2001(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let rows_ = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).select(post_type_id.and(score).and(comments_of(db).opt())));
    let top = top_per(rows_, |&(_, ((t, _), _))| t, |&(p, ((_, s), c))| (Reverse(s), p, c), 10, false);
    let tp = rel(top.into_iter().map(|x| x.0).collect());
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(500)).and((&bc).filt(|n| n > 3).opt())))));
    let v = top_n(v, |&(i, (p, (u, _)))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), i), 100);
    rows(v.into_iter().map(|(_, (p, (u, b)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(b.map_or(V::Null, V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.ViewCount, p.Score, U.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, ViewCount, Score, CommentCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.Title, tp.ViewCount, tp.Score, tp.CommentCount, tp.OwnerDisplayName, ph.CreationDate AS LastEditDate, U2.DisplayName AS LastEditorDisplayName
// FROM TopPosts tp LEFT JOIN Posts p ON tp.PostId = p.Id LEFT JOIN Users U2 ON p.LastEditorUserId = U2.Id LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// WHERE ph.PostHistoryTypeId IN (4, 5, 6) ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q6531(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, last_editor_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id.and(owner_user)));
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let top = top_per(v, |&(_, (t, _))| t, |&(p, _)| key(p), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain((&cc).and(history_of(db).with((&db.post_history.post_history_type_id).is_in([4, 5, 6]))).and(last_editor_user.opt()));
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, ((c, h), e))| {
        let mut f = post_fields(db, p, &["title", "views", "score"]);
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["owner"]));
        f.push(V::T(db.post_history.creation_date.get(h).unwrap()));
        f.push(e.map_or(V::Null, |e| user_col(db, e, "name")));
        row(f)
    }))
}

// WITH PostSummary AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON a.ParentId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id LEFT JOIN Comments c ON c.PostId = p.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// TopPosts AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS PostRank FROM PostSummary)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.AnswerCount, tp.UpVotes, tp.DownVotes, tp.CommentCount
// FROM TopPosts tp WHERE tp.PostRank <= 10;
fn q9814(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(score.and(view_count.opt()))), |&(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt()))
        .fold([0i64; 4], |a, ((x, t), c)| [a[0] + x.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + c.is_some() as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, OwnerDisplayName FROM RankedPosts WHERE Rank = 1),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount FROM Votes v WHERE v.VoteTypeId IN (2, 3) GROUP BY v.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(pv.VoteCount, 0) AS VoteCount
// FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId LEFT JOIN PostVotes pv ON tp.PostId = pv.PostId
// ORDER BY tp.Score DESC, tp.CreationDate DESC LIMIT 10;
fn q6248(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2])).select(owner_user.and(score)));
    let top = top_per(v, |&(_, (u, _))| u, |&(p, (_, s))| (Reverse(s), p), 1, false);
    let top = top_n(top, |&(p, (_, s))| (Reverse(s), Reverse(creation_date.get(p).unwrap())), 10);
    let tp = rel(top.into_iter().map(|x| x.0).collect());
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let pv = db.vote.with((&db.vote.vote_type_id).is_in([2, 3])).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&tp).select(Ident::<Post>::new().and((&pc).opt()).and((&pv).opt())));
    v.sort_by_key(|x| x.0);
    rows(v.into_iter().map(|(_, ((p, c), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(n.unwrap_or(0))]);
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore, AVG(u.Reputation) AS AvgUserReputation
//     FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%<', t.TagName, '>%') JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY t.TagName),
// TopTags AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY PostCount DESC, TotalScore DESC) AS TagRank FROM TagStatistics)
// SELECT t.TagName, t.PostCount, t.TotalViews, t.TotalScore, t.AvgUserReputation, ph.CreationDate AS LastPostEditedDate, u.DisplayName AS LastEditorDisplayName
// FROM TopTags t LEFT JOIN Posts p ON p.Tags LIKE CONCAT('%<', t.TagName, '>%')
// LEFT JOIN PostHistory ph ON ph.PostId = p.Id AND ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = p.Id)
// LEFT JOIN Users u ON ph.UserId = u.Id WHERE t.TagRank <= 10 ORDER BY t.TagRank;
//
// Tag names hold no '<' or '>', so '%<name>%' matches exactly the posts
// listing that tag.
fn q28562(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, tags_str, .. } = &db.post;
    let tagged: HashIdx<Str, Id<Post>> = db.post.select(tags_str.flat_map(tag_list)).inv().collect();
    let posts_of_tag = (&db.tag.tag_name).select(&tagged);
    let recent = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let st = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&db.tag.tag_name).select(&tagged).select(recent).select(view_count.opt().and(score).and(owner_user.select(&db.user.reputation))))
        .fold([0i64; 5], |a, ((w, s), r)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + r]);
    let tt = rel(top_n(drain(&st), |&(_, a)| (Reverse(a[0]), Reverse(a[3])), 10));
    let PostHistory { post, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(&db.post_history.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(&db.post_history.creation_date)).inv().collect();
    let last = Ident::<Post>::new().and(&md).select(&at).select((&db.post_history.creation_date).and((&db.post_history.user).opt()));
    let mut v = drain((&tt).map(|(t, _)| t).select(posts_of_tag.select(last.opt()).opt()));
    v.sort_by_key(|x| x.0);
    rows(v.into_iter().map(|(i, h)| {
        let (t, a) = tt.get(i).unwrap();
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), avg(a[4], a[0])];
        f.extend(match h.flatten() {
            Some((d, u)) => [V::T(d), u.map_or(V::Null, |u| user_col(db, u, "name"))],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostsCount, SUM(COALESCE(IP.VoteCount, 0)) AS VotesReceived,
//        SUM(COALESCE(C.CommentCount, 0)) AS CommentsCount, AVG(U.Reputation) AS AverageReputation
//     FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) AS IP ON IP.PostId = P.Id
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) AS C ON C.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT ..., DENSE_RANK() OVER (ORDER BY VotesReceived DESC) AS Rank FROM UserEngagement)
// SELECT UserId, DisplayName, PostsCount, VotesReceived, CommentsCount, AverageReputation FROM TopUsers WHERE Rank <= 10 ORDER BY VotesReceived DESC;
fn q5738(db: &'static So) -> String {
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&vc).opt().and((&cc).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((v, c)) => [a[0] + 1, a[1] + v.unwrap_or(0), a[2] + c.unwrap_or(0)],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[1]), true);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS RankByViews,
//        u.Reputation, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') AND p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation, p.OwnerUserId),
// TopPosts AS (SELECT *, GREATEST(RankByScore, RankByViews) AS OverallRank FROM RankedPosts)
// SELECT p.Title, p.CreationDate, p.Score, p.ViewCount, p.CommentCount, p.VoteCount, u.DisplayName, u.Reputation
// FROM TopPosts p JOIN Users u ON p.OwnerUserId = u.Id WHERE OverallRank <= 10 ORDER BY OverallRank;
fn q7367(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).select(owner_user));
    let a = per_group(ranked(v, |&(p, u)| (u, Reverse(score.get(p).unwrap())), false), |x| x.1);
    let b = per_group(
        ranked(a, |&((p, u), _)| {
            let w = view_count.get(p);
            (u, w.is_none(), Reverse(w))
        }, false),
        |x| x.0 .1,
    );
    let mut keep: Vec<(Id<Post>, i64)> = b.into_iter().map(|(((p, _), x), y)| (p, x.max(y))).filter(|x| x.1 <= 10).collect();
    keep.sort_by_key(|x| x.1);
    let tp: MatSet<Id<Post>> = rel(keep.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let up: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.post).inv().collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&up).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    rows(keep.into_iter().map(|(p, _)| {
        let (n, m) = s.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(n), V::I(m)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS Owner, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT b.Id) AS BadgeCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, Owner, CommentCount, BadgeCount FROM RankedPosts WHERE PostRank <= 3)
// SELECT tp.Owner, COUNT(tp.PostId) AS TopPostCount, AVG(tp.Score) AS AvgScore, SUM(tp.ViewCount) AS TotalViews, SUM(tp.CommentCount) AS TotalComments,
//        SUM(tp.BadgeCount) AS TotalBadges
// FROM TopPosts tp GROUP BY tp.Owner HAVING AVG(tp.Score) > 10 ORDER BY TotalViews DESC;
fn q5958(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt().and(score).and(creation_date)));
    let top = top_per(v, |&(_, ((u, _), _))| u, |&(_, ((_, s), d))| (Reverse(s), Reverse(d)), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(distinct_some);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    let g = (&tp)
        .group_by(owner_user.select(&db.user.display_name).opt())
        .select(score.and(view_count.opt()).and(&cc).and(owner_user.select(&bc).opt()))
        .fold([0i64; 6], |a, (((s, w), c), b)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c, a[5] + b.unwrap_or(0)]);
    let mut v = drain((&g).filt(|a| a[1] > 10 * a[0]));
    v.sort_by_key(|&(_, a)| (a[2] == 0, Reverse(a[3])));
    rows(v.into_iter().map(|(n, a)| row(vec![n.map_or(V::Null, V::S), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4]), V::I(a[5])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate BETWEEN '2023-01-01' AND '2023-12-31' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// FinalRanking AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, rp.CommentCount, rp.VoteCount,
//        RANK() OVER (ORDER BY rp.Score DESC, rp.CommentCount DESC) AS Rank FROM RankedPosts rp WHERE rp.rn = 1)
// SELECT fr.PostId, fr.Title, fr.CreationDate, fr.Score, fr.OwnerDisplayName, fr.CommentCount, fr.VoteCount FROM FinalRanking fr WHERE fr.Rank <= 10 ORDER BY fr.Rank;
fn q7823(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let ps = || db.post.with(creation_date.ge(date(2023, 1, 1)).and(creation_date.le(date(2023, 12, 31))).and(post_type_id.is_in([1, 2]))).with(owner_user);
    let up: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.post).inv().collect();
    let cc = ps().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and((&up).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let dv = ps().group_by(Ident::<Post>::new()).select((&up).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&cc).and(&dv)), |&(p, (c, _))| (Reverse(score.get(p).unwrap()), Reverse(c)), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, (c, n)), _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId, p.LastActivityDate,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopUserPosts AS (SELECT up.OwnerUserId, COUNT(up.PostId) AS TotalPosts, SUM(up.Score) AS TotalScore, AVG(up.ViewCount) AS AvgViewCount
//     FROM RankedPosts up WHERE up.RankByScore <= 5 GROUP BY up.OwnerUserId),
// UsersWithBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT u.DisplayName, up.TotalPosts, up.TotalScore, up.AvgViewCount, ub.BadgeCount
// FROM Users u JOIN TopUserPosts up ON u.Id = up.OwnerUserId JOIN UsersWithBadges ub ON u.Id = ub.UserId
// WHERE ub.BadgeCount > 0 ORDER BY up.TotalScore DESC, up.TotalPosts DESC;
fn q7192(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt().and(score)));
    let top = top_per(v, |&(_, (u, _))| u, |&(_, (_, s))| Reverse(s), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let up = (&tp).group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&up).and((&bc).filt(|n| n > 0)));
    v.sort_by_key(|&(_, (a, _))| (Reverse(a[1]), Reverse(a[0])));
    rows(v.into_iter().map(|(u, (a, b))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(b)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerName, u.Reputation AS OwnerReputation, p.CreationDate, p.ViewCount, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 10),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT rp.PostId, rp.Title, rp.OwnerName, rp.OwnerReputation, rp.CreationDate, rp.ViewCount, rp.Score, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM RankedPosts rp JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId WHERE rp.PostRank <= 5 ORDER BY rp.OwnerReputation DESC, rp.CreationDate DESC;
fn q7258(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(10))).select(owner_user.and(creation_date)));
    let mut top = top_per(v, |&(_, (u, _))| u, |&(_, (_, d))| Reverse(d), 5, false);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    top.sort_by_key(|&(_, (u, d))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(d)));
    rows(top.into_iter().map(|(p, (u, _))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(post_fields(db, p, &["created", "views", "score"]));
        f.extend(ub.get(u).unwrap().map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, U.DisplayName AS Owner, COUNT(C.Id) AS CommentCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.Body, U.DisplayName, P.OwnerUserId, P.CreationDate),
// PopularPosts AS (SELECT PostId, Title, Owner, UpVotes, DownVotes, CommentCount, (UpVotes - DownVotes) AS NetVotes FROM RankedPosts WHERE Rank = 1)
// SELECT PP.Title, PP.Owner, PP.CommentCount, PP.NetVotes,
//        CASE WHEN PP.NetVotes > 0 THEN 'Popular' WHEN PP.NetVotes < 0 THEN 'Unpopular' ELSE 'Neutral' END AS PopularityStatus
// FROM PopularPosts PP ORDER BY PP.NetVotes DESC, PP.CommentCount DESC LIMIT 10;
fn q25766(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.and(creation_date)));
    let top = top_per(v, |&(_, (u, _))| u, |&(_, (_, d))| Reverse(d), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(n, m), (c, t)| (n + c.is_some() as i64, m + (t == Some(2)) as i64 - (t == Some(3)) as i64));
    let v = top_n(drain(&s), |&(_, (c, n))| (Reverse(n), Reverse(c)), 10);
    rows(v.into_iter().map(|(p, (c, n))| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend([V::I(c), V::I(n), V::S(if n > 0 { "Popular" } else if n < 0 { "Unpopular" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS UpvoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND p.ViewCount > 100
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, OwnerDisplayName, Rank FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName, ph.Comment, ph.CreationDate AS HistoryDate
// FROM TopPosts tp JOIN PostHistory ph ON tp.PostId = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11, 12, 14) ORDER BY tp.Score DESC, ph.CreationDate DESC LIMIT 50;
//
// None of the top recent posts was closed, reopened or locked, so the join
// is empty.
fn q8800(db: &'static So) -> String {
    let Post { creation_date, view_count, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(view_count.gt(100))).select(post_type_id.and(score)));
    let top = top_per(v, |&(_, (t, _))| t, |&(p, (_, s))| (Reverse(s), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let v = drain((&tp).select(history_of(db).with(post_history_type_id.is_in([10, 11, 12, 14]))));
    let v = top_n(v, |&(p, h)| (Reverse(score.get(p).unwrap()), Reverse(hd.get(h).unwrap())), 50);
    rows(v.into_iter().map(|(p, h)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([harness::fmt::ostr(comment.get(h)), V::T(hd.get(h).unwrap())]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName, COUNT(v.Id) AS VoteCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.OwnerDisplayName, rp.VoteCount, rp.Rank FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.OwnerDisplayName, tp.VoteCount, pt.Name AS PostTypeName
// FROM TopPosts tp JOIN PostTypes pt ON tp.PostId = pt.Id ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// No recent post has an Id small enough to be a PostTypes Id, so the join is empty.
fn q7623(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let top = top_per(v, |&(_, t)| t, |&(p, _)| key(p), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pts: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let mut v = drain((&vc).and(origid.select(&pts)));
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, (n, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner"]);
        f.extend([V::I(n), V::S(db.post_type.name.get(t).unwrap())]);
        row(f)
    }))
}

// WITH TagStats AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS ClosedCount,
//        SUM(CASE WHEN ph.PostHistoryTypeId = 52 THEN 1 ELSE 0 END) AS HotCount, AVG(u.Reputation) AS AverageReputation
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN PostHistory ph ON ph.PostId = p.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     GROUP BY t.TagName),
// TagRanked AS (SELECT ..., RANK() OVER (ORDER BY PostCount DESC) AS PostCountRank, RANK() OVER (ORDER BY ClosedCount DESC) AS ClosedCountRank,
//        RANK() OVER (ORDER BY HotCount DESC) AS HotCountRank, RANK() OVER (ORDER BY AverageReputation DESC) AS ReputationRank FROM TagStats)
// SELECT TagName, PostCount, ClosedCount, HotCount, AverageReputation, PostCountRank, ClosedCountRank, HotCountRank, ReputationRank,
//        (PostCountRank + ClosedCountRank + HotCountRank + ReputationRank) AS OverallRank
// FROM TagRanked ORDER BY OverallRank;
fn q27796(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let s = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select(history_of(db).select(&db.post_history.post_history_type_id).opt().and((&db.post.owner_user).select(&db.user.reputation).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((h, r)) => [a[0] + 1, a[1] + (h == Some(10)) as i64, a[2] + (h == Some(52)) as i64, a[3] + r.is_some() as i64, a[4] + r.unwrap_or(0)],
            None => a,
        });
    let mean = |a: [i64; 5]| a[4] as f64 / a[3] as f64;
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[1]), false);
    let v = ranked(v, |&(((_, a), _), _)| Reverse(a[2]), false);
    let mut v = ranked(v, |&((((_, a), _), _), _)| (a[3] == 0, Reverse(fkey(mean(a)))), false);
    v.sort_by_key(|&(((((_, _), p), c), h), r)| p + c + h + r);
    rows(v.into_iter().map(|(((((t, a), p), c), h), r)| {
        row(vec![
            V::S(db.tag.tag_name.get(t).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            if a[3] == 0 { V::Null } else { V::F(mean(a)) },
            V::I(p),
            V::I(c),
            V::I(h),
            V::I(r),
            V::I(p + c + h + r),
        ])
    }))
}

// WITH RankedComments AS (SELECT c.Id AS CommentId, c.PostId, c.UserId, u.DisplayName AS UserDisplayName, c.Score,
//        ROW_NUMBER() OVER (PARTITION BY c.PostId ORDER BY c.CreationDate DESC) AS CommentRank FROM Comments c JOIN Users u ON c.UserId = u.Id),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.AnswerCount, p.ViewCount, p.PostTypeId,
//        (SELECT COUNT(*) FROM RankedComments rc WHERE rc.PostId = p.Id AND rc.CommentRank <= 5) AS RecentCommentCount
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopTags AS (SELECT t.TagName, COUNT(pt.Id) AS PostCount, RANK() OVER (ORDER BY COUNT(pt.Id) DESC) AS TagRank
//     FROM Tags t JOIN Posts pt ON pt.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.AnswerCount, rp.ViewCount, rp.RecentCommentCount, tt.TagName, tt.PostCount AS TagUsageCount
// FROM RecentPosts rp LEFT JOIN TopTags tt ON tt.TagRank = 1 WHERE rp.PostTypeId = 1 ORDER BY rp.ViewCount DESC, rp.RecentCommentCount DESC LIMIT 10;
fn q29610(db: &'static So) -> String {
    let Comment { post, user, creation_date: cd, .. } = &db.comment;
    let rc = drain(db.comment.with(user).select(post.and(cd)));
    let rc = top_per(rc, |&(_, (p, _))| p, |&(c, (_, d))| (Reverse(d), c), 5, false);
    let rc: MatSet<Id<Comment>> = rel(rc.into_iter().map(|x| x.0).collect()).map(|c| c).collect();
    let rcc = (&rc).group_by(post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let Post { creation_date, post_type_id, view_count, .. } = &db.post;
    let rp = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.eq(1))).select(view_count.opt().and((&rcc).opt())));
    let rp = top_n(rp, |&(_, (w, n))| (w.is_none(), Reverse(w), Reverse(n.unwrap_or(0))), 10);
    let ts_ = tag_stats(db);
    let tt: Vec<_> = ranked(drain((&ts_).filt(|a| a[0] > 0)), |&(_, a)| Reverse(a[0]), false).into_iter().take_while(|x| x.1 <= 1).map(|x| x.0).collect();
    let v = cross_top(rp, |_| 0, tt, |_| 0, usize::MAX);
    rows(v.into_iter().map(|((p, (_, n)), (t, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "answers", "views"]);
        f.extend([V::I(n.unwrap_or(0)), V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(B.Id) AS BadgeCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END) AS QuestionsAnswered, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AvgScore
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY Reputation DESC) AS RankByReputation, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews FROM UserStats)
// SELECT T.DisplayName, T.Reputation, T.BadgeCount, T.QuestionsAnswered, T.TotalViews, T.AvgScore,
//        CASE WHEN T.RankByReputation <= 10 THEN 'Top 10 by Reputation' WHEN T.RankByViews <= 10 THEN 'Top 10 by Views' ELSE 'Below Top 10' END AS RankCategory
// FROM TopUsers T WHERE T.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY T.Reputation DESC, T.TotalViews DESC;
fn q798(db: &'static So) -> String {
    let Post { post_type_id, answer_count, view_count, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(answer_count.opt()).and(view_count.opt()).and(score)).opt()))
        .fold((0i64, None, [0i64; 4]), |(b, q, a): (i64, Option<i64>, [i64; 4]), (x, p)| {
            let qa = match p {
                Some((((t, n), _), _)) if t == 1 => n,
                _ => Some(0),
            };
            let q = match (q, qa) {
                (q, None) => q,
                (q, Some(n)) => Some(q.unwrap_or(0) + n),
            };
            let a = match p {
                Some(((_, w), s)) => [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + 1, a[3] + s],
                None => a,
            };
            (b + x.is_some() as i64, q, a)
        });
    let (n, t) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, t), r| (n + 1, t + r));
    let mean = t as f64 / n as f64;
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let v = ranked(drain(&s), |&(u, _)| Reverse(rep(u)), false);
    let v = ranked(v, |&((_, (_, _, a)), _)| (a[0] == 0, Reverse(a[1])), false);
    let mut v: Vec<_> = drain(rel(v).filt(|x| rep(x.0 .0 .0) as f64 > mean)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(((u, (_, _, a)), _), _)| (Reverse(rep(u)), a[0] == 0, Reverse(a[1])));
    rows(v.into_iter().map(|(((u, (b, q, a)), r), w)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), q.map_or(V::Null, V::I), nullable(a[1], a[0]), avg(a[3], a[2])]);
        f.push(V::S(if r <= 10 { "Top 10 by Reputation" } else if w <= 10 { "Top 10 by Views" } else { "Below Top 10" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ub.BadgeCount, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC, ub.BadgeCount DESC) AS UserRank
//     FROM Users u JOIN UserBadges ub ON u.Id = ub.UserId)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CreationDate
// FROM TopUsers tu JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId WHERE tu.UserRank <= 10 AND rp.PostRank <= 5 ORDER BY tu.Reputation DESC, rp.Score DESC;
fn q9923(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let tu = top_n(drain(&bc), |&(u, b)| (Reverse(rep(u)), Reverse(b)), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.and(score).and(creation_date)));
    let mut top: Vec<_> = top_per(v, |&(_, ((u, _), _))| u, |&(_, ((_, s), d))| (Reverse(s), Reverse(d)), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.drain(..).map(|x| x.0).collect()).map(|p| p).collect();
    let mut v = drain((&tp).select(owner_user.select(Ident::<User>::new().with(&tu))));
    v.sort_by_key(|&(p, u)| (Reverse(rep(u)), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, u)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "answers", "created"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, U.DisplayName AS OwnerDisplayName, P.CreationDate, P.Score, P.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.Score > 0),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(RP.PostId) AS PostCount, SUM(RP.Score) AS TotalScore
//     FROM RankedPosts RP JOIN Users U ON RP.OwnerUserId = U.Id GROUP BY U.Id, U.DisplayName HAVING COUNT(RP.PostId) >= 5),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT TU.DisplayName, TU.PostCount, TU.TotalScore, UB.BadgeCount,
//        (SELECT COUNT(*) FROM Posts P WHERE P.OwnerUserId = TU.UserId AND P.AcceptedAnswerId IS NOT NULL) AS AcceptedAnswersCount
// FROM TopUsers TU JOIN UserBadges UB ON TU.UserId = UB.UserId ORDER BY TU.TotalScore DESC, TU.PostCount DESC;
fn q5145(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, accepted_answer_id, .. } = &db.post;
    let tu = db.post.with(post_type_id.eq(1).and(score.gt(0))).group_by(owner_user).select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let acc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).with(accepted_answer_id).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let mut v = drain((&tu).filt(|(n, _)| n >= 5).and(&bc).and(&acc));
    v.sort_by_key(|&(_, (((n, s), _), _))| (Reverse(s), Reverse(n)));
    rows(v.into_iter().map(|(u, (((n, s), b), a))| row(vec![user_col(db, u, "name"), V::I(n), V::I(s), V::I(b), V::I(a)])))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// PostMetrics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore FROM Posts P GROUP BY P.OwnerUserId),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, UR.Reputation, UR.BadgeCount, PM.TotalPosts, PM.QuestionCount, PM.AnswerCount, PM.TotalViews, PM.TotalScore
//     FROM Users U JOIN UserReputation UR ON U.Id = UR.UserId LEFT JOIN PostMetrics PM ON U.Id = PM.OwnerUserId),
// RankedUsers AS (SELECT UA.*, RANK() OVER (ORDER BY UA.Reputation DESC) AS ReputationRank, RANK() OVER (ORDER BY UA.TotalPosts DESC) AS PostRank FROM UserActivity UA)
// SELECT UserId, DisplayName, Reputation, BadgeCount, TotalPosts, QuestionCount, AnswerCount, TotalViews, TotalScore, ReputationRank, PostRank
// FROM RankedUsers WHERE TotalPosts > 10 ORDER BY ReputationRank, PostRank;
fn q7970(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let v = ranked(drain((&bc).and(&ups)), |&(u, _)| Reverse(rep(u)), false);
    let v = ranked(v, |&((_, (_, a)), _)| (a[1] == 0, Reverse(a[1])), false);
    let mut v: Vec<_> = drain(rel(v).filt(|x| x.0 .0 .1 .1[1] > 10)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&((_, r), p)| (r, p));
    rows(v.into_iter().map(|(((u, (b, a)), r), p)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), V::I(a[4]), V::I(r), V::I(p)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount FROM RankedPosts rp WHERE rp.ScoreRank <= 5)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.CreationDate, tp.OwnerDisplayName, tp.CommentCount, COALESCE(b.BadgeCount, 0) AS UserBadgeCount
// FROM TopPosts tp LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON tp.OwnerDisplayName = (SELECT u.DisplayName FROM Users u WHERE u.Id = b.UserId)
// ORDER BY tp.Score DESC;
fn q7498(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2])).select(post_type_id.and(score).and(creation_date)));
    let top = top_per(v, |&(_, ((t, _), _))| t, |&(_, ((_, s), d))| (Reverse(s), Reverse(d)), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let bu: MatSet<Id<User>> = db.badge.select(&db.badge.user).collect();
    let by_name: HashIdx<Str, Id<User>> = (&bu).select(&db.user.display_name).inv().collect();
    let mut v = drain((&cc).and(owner_user.select(&db.user.display_name).select(&by_name).select(&bc).opt()));
    v.sort_by_key(|&(p, _)| Reverse(score.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "created", "owner"]);
        f.extend([V::I(c), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserPosts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPosts WHERE PostCount > 5),
// RecentPosts AS (SELECT P.Id, P.Title, P.CreationDate, COUNT(C.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank, P.OwnerUserId
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
//     GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId)
// SELECT TU.DisplayName, TU.PostCount, TU.TotalScore, RP.Title AS RecentPostTitle, RP.CommentCount
// FROM TopUsers TU LEFT JOIN RecentPosts RP ON TU.UserId = RP.OwnerUserId WHERE RP.RecentPostRank = 1 ORDER BY TU.ScoreRank, TU.DisplayName;
fn q4986(db: &'static So) -> String {
    let ups = user_posts(db);
    let tu = rel(ranked(drain((&ups).filt(|a| a[1] > 5)), |&(_, a)| Reverse(a[4]), false));
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user.and(creation_date)));
    let rp = top_per(v, |&(_, (u, _))| u, |&(_, (_, d))| Reverse(d), 1, false);
    let rp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let by_owner: HashIdx<Id<User>, Id<Post>> = (&rp).select(owner_user).inv().collect();
    let mut v = drain((&tu).map(|((u, _), _)| u).select(&by_owner));
    v.sort_by_key(|&(i, _)| {
        let ((u, _), r) = tu.get(i).unwrap();
        (r, db.user.display_name.get(u).unwrap())
    });
    rows(v.into_iter().map(|(i, p)| {
        let ((u, a), _) = tu.get(i).unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[4])];
        f.extend(post_fields(db, p, &["title"]));
        f.push(V::I(cc.get(p).unwrap()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, U.DisplayName AS Author,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= '2023-01-01'),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, Author FROM RankedPosts WHERE PostRank <= 5),
// PostCommentStats AS (SELECT PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY PostId),
// PostVoteStats AS (SELECT PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V GROUP BY PostId)
// SELECT TP.PostId, TP.Title, TP.Score, TP.ViewCount, TP.Author, PCS.CommentCount, PVS.UpVotes, PVS.DownVotes
// FROM TopPosts TP LEFT JOIN PostCommentStats PCS ON TP.PostId = PCS.PostId LEFT JOIN PostVoteStats PVS ON TP.PostId = PVS.PostId ORDER BY TP.Score DESC, TP.ViewCount DESC;
fn q8743(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(date(2023, 1, 1))).with(owner_user).select(post_type_id.and(score).and(creation_date)));
    let top = top_per(v, |&(_, ((t, _), _))| t, |&(_, ((_, s), d))| (Reverse(s), Reverse(d)), 5, true);
    let tp = rel(top.into_iter().map(|x| x.0).collect());
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold((0i64, 0i64), |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64));
    let mut v = drain((&tp).select(Ident::<Post>::new().and((&pc).opt()).and((&pv).opt())));
    v.sort_by_key(|&(_, ((p, _), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(_, ((p, c), x))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.push(c.map_or(V::Null, V::I));
        f.extend(match x {
            Some((u, d)) => [V::I(u), V::I(d)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        RANK() OVER (ORDER BY COUNT(c.Id) DESC) AS RankByComments
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2023-01-01' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, Author, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE RankByComments <= 10)
// SELECT tp.Title, tp.Author, tp.CommentCount, tp.UpVotes, tp.DownVotes,
//        CASE WHEN (tp.UpVotes + tp.DownVotes) > 0 THEN ROUND((tp.UpVotes * 1.0 / (tp.UpVotes + tp.DownVotes)) * 100, 2) ELSE 0 END AS UpvotePercentage
// FROM TopPosts tp ORDER BY tp.CommentCount DESC, tp.UpVotes DESC;
fn q9529(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(date(2023, 1, 1)).and(post_type_id.eq(1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v: Vec<_> = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false).into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect();
    v.sort_by_key(|&(_, a)| (Reverse(a[0]), Reverse(a[1])));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend(a.map(V::I));
        let t = a[1] + a[2];
        f.push(V::F(if t > 0 { ((a[1] as f64 / t as f64) * 100.0 * 100.0).round() / 100.0 } else { 0.0 }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT up.DisplayName, up.Reputation, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes
// FROM RankedPosts rp JOIN UserReputation up ON rp.OwnerUserId = up.UserId WHERE rp.Rank <= 5 ORDER BY up.Reputation DESC, rp.Score DESC;
//
// The row number only reads CreationDate, so the posts are picked before
// their joined rows are counted.
fn q9473(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.and(creation_date)));
    let top = top_per(v, |&(_, (u, _))| u, |&(_, (_, d))| Reverse(d), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&s).and(owner_user));
    v.sort_by_key(|&(p, (_, u))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Title, p.Body, p.CreationDate, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, u.DisplayName),
// FilteredPosts AS (SELECT r.Title, r.Body, r.CreationDate, r.Author, r.CommentCount, r.UpVotes, r.DownVotes FROM RankedPosts r WHERE r.rn = 1)
// SELECT fp.Title, fp.Body, fp.CreationDate, fp.Author, fp.CommentCount, fp.UpVotes, fp.DownVotes, (fp.UpVotes - fp.DownVotes) AS Score,
//        CASE WHEN fp.UpVotes > 100 THEN 'Hot' WHEN fp.UpVotes BETWEEN 50 AND 100 THEN 'Trending' ELSE 'Normal' END AS Popularity
// FROM FilteredPosts fp WHERE fp.UpVotes >= 10 ORDER BY fp.UpVotes DESC, fp.CommentCount DESC, fp.CreationDate LIMIT 10;
//
// rewrites/29442.sql: the CTE's ORDER BY, which SQL drops, moved to the outer
// query and tie-broken on CreationDate.
fn q29442(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain((&s).filt(|a| a[1] >= 10)), |&(p, a)| (Reverse(a[1]), Reverse(a[0]), creation_date.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "body", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(a[1] - a[2]));
        f.push(V::S(if a[1] > 100 { "Hot" } else if a[1] >= 50 { "Trending" } else { "Normal" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= DATE '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE PostRank <= 10)
// SELECT PostId, Title, CreationDate, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount, (UpVoteCount - DownVoteCount) AS NetScore
// FROM TopPosts ORDER BY Score DESC, CreationDate DESC;
//
// The rank only reads CreationDate, so the posts are picked before their
// joined rows are counted.
fn q9452(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(date(2023, 1, 1))).select(post_type_id.and(creation_date)));
    let v = per_group(ranked(v, |&(_, (t, d))| (t, Reverse(d)), true), |x| x.1 .0);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().filter(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain(&s);
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(a[1] - a[2]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT PostId, Title, Score, CreationDate, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10),
// PostComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// FinalReport AS (SELECT tp.PostId, tp.Title, tp.Score, tp.CreationDate, tp.OwnerDisplayName, pc.CommentCount FROM TopPosts tp JOIN PostComments pc ON tp.PostId = pc.PostId)
// SELECT f.PostId, f.Title, f.Score, f.CreationDate, f.OwnerDisplayName, f.CommentCount FROM FinalReport f ORDER BY f.Score DESC, f.CreationDate ASC;
fn q9845(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user).select(post_type_id.and(score).and(creation_date)));
    let top = top_per(v, |&(_, ((t, _), _))| t, |&(_, ((_, s), d))| (Reverse(s), Reverse(d)), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let mut v = drain((&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner"]);
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 GROUP BY u.Id, u.Reputation),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT up.UserId, up.Reputation, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rc.CommentCount, rc.LastCommentDate
// FROM UserReputation up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId LEFT JOIN RecentComments rc ON rp.Id = rc.PostId
// WHERE up.Reputation > 100 AND rp.PostRank = 1 ORDER BY up.Reputation DESC, rp.Score DESC LIMIT 10;
fn q3667(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100))).and(creation_date)));
    let top = top_per(v, |&(_, (u, _))| u, |&(_, (_, d))| Reverse(d), 1, false);
    let top = top_n(top, |&(p, (u, _))| (Reverse(rep(u)), Reverse(score.get(p).unwrap())), 10);
    let tp = rel(top);
    let rc = db.comment.group_by(&db.comment.post).select(&db.comment.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = drain((&tp).map(|(p, _)| p).select(Ident::<Post>::new().and((&rc).opt())));
    v.sort_by_key(|x| x.0);
    rows(v.into_iter().map(|(i, (p, c))| {
        let (_, (u, _)) = tp.get(i).unwrap();
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views", "answers"]));
        f.extend(match c {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("8526", q8526),
    ("9562", q9562),
    ("10144", q10144),
    ("7007", q7007),
    ("6180", q6180),
    ("12444", q12444),
    ("11552", q11552),
    ("13348", q13348),
    ("11265", q11265),
    ("12658", q12658),
    ("13171", q13171),
    ("11147", q11147),
    ("27293", q27293),
    ("2739", q2739),
    ("6360", q6360),
    ("9398", q9398),
    ("26429", q26429),
    ("12787", q12787),
    ("6957", q6957),
    ("13503", q13503),
    ("6885", q6885),
    ("7109", q7109),
    ("12806", q12806),
    ("6219", q6219),
    ("6241", q6241),
    ("13044", q13044),
    ("11734", q11734),
    ("5203", q5203),
    ("26487", q26487),
    ("11749", q11749),
    ("9593", q9593),
    ("9004", q9004),
    ("14576", q14576),
    ("9738", q9738),
    ("13452", q13452),
    ("30944", q30944),
    ("5834", q5834),
    ("2254", q2254),
    ("9904", q9904),
    ("9400", q9400),
    ("13474", q13474),
    ("4372", q4372),
    ("5345", q5345),
    ("6752", q6752),
    ("13238", q13238),
    ("8541", q8541),
    ("5589", q5589),
    ("28896", q28896),
    ("11936", q11936),
    ("14139", q14139),
    ("33451", q33451),
    ("6279", q6279),
    ("9512", q9512),
    ("5666", q5666),
    ("6743", q6743),
    ("5028", q5028),
    ("7372", q7372),
    ("9753", q9753),
    ("28326", q28326),
    ("1495", q1495),
    ("7259", q7259),
    ("6330", q6330),
    ("14213", q14213),
    ("9876", q9876),
    ("7180", q7180),
    ("5808", q5808),
    ("2001", q2001),
    ("6531", q6531),
    ("9814", q9814),
    ("6248", q6248),
    ("28562", q28562),
    ("5738", q5738),
    ("7367", q7367),
    ("5958", q5958),
    ("7823", q7823),
    ("7192", q7192),
    ("7258", q7258),
    ("25766", q25766),
    ("8800", q8800),
    ("7623", q7623),
    ("27796", q27796),
    ("29610", q29610),
    ("798", q798),
    ("9923", q9923),
    ("5145", q5145),
    ("7970", q7970),
    ("7498", q7498),
    ("4986", q4986),
    ("8743", q8743),
    ("9529", q9529),
    ("9473", q9473),
    ("29442", q29442),
    ("9452", q9452),
    ("9845", q9845),
    ("3667", q3667),
];
