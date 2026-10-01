use harness::prelude::*;
use std::cmp::Reverse;

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


// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN B.UserId IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT T.UserId, T.DisplayName, T.Reputation, T.PostCount, T.QuestionCount, T.AnswerCount, T.UpVotes, T.DownVotes, T.BadgeCount FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.Rank;
//
// Rank reads only Reputation, so the top users are picked first and the posts x votes x badges product is driven for those alone.
fn q9018(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let v = top_n(drain((&s).and(user_distinct_posts(db))), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), 0);
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[1]), V::I(a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentRank
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, us.TotalBounty, us.Upvotes, us.Downvotes, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate,
//        COALESCE(rp.RecentRank, 0) AS RecentPostRank
// FROM UserStats us LEFT JOIN RecentPosts rp ON us.UserId = rp.OwnerUserId AND rp.RecentRank = 1 WHERE us.Reputation > 100 ORDER BY us.Rank, us.Reputation DESC;
fn q1543(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(100));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).opt())
        .fold([0i64; 3], |a, v| match v.flatten() {
            Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
            None => a,
        });
    let recent = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let recent = top_per(recent, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let rp = rel(recent.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let v = drain(users().select(Ident::<User>::new().and(&s).and(user_distinct_posts(db)).and((&by_user).map(|(_, p)| p).opt())));
    rows(v.into_iter().map(|(_, (((u, a), n), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match p {
            Some(p) => [harness::fmt::ostr(db.post.title.get(p)), V::T(creation_date.get(p).unwrap()), V::I(1)],
            None => [V::Null, V::Null, V::I(0)],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, u.Reputation,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// PopularTags AS (SELECT unnest(string_to_array(Tags, '>')) AS Tag FROM RankedPosts WHERE TagRank <= 10),
// TagPopularity AS (SELECT Tag, COUNT(*) AS PopularityCount FROM PopularTags GROUP BY Tag),
// TopTags AS (SELECT Tag, PopularityCount, ROW_NUMBER() OVER (ORDER BY PopularityCount DESC) AS PopularityRank FROM TagPopularity WHERE PopularityCount > 5)
// SELECT t.Tag, t.PopularityCount, r.PostId, r.Title, r.CreationDate, r.OwnerDisplayName, r.Reputation
// FROM TopTags t JOIN RankedPosts r ON t.Tag = ANY(string_to_array(r.Tags, '>')) WHERE t.PopularityRank <= 10 ORDER BY t.PopularityCount DESC, r.Score DESC;
//
// Every post in a TagRank partition has the same Tags, so which ten a tie lets through does not change the counts.
fn q29701(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, tags_str, score, .. } = &db.post;
    let split = |t: Str| t.split('>');
    let rp = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(date(2024, 10, 1), -1)))).with(owner_user);
    let top = top_per(drain(rp().select(tags_str.opt())), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pop = (&tp).select(tags_str.flat_map(split)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let tt = top_n(drain((&pop).filt(|n| n > 5)), |&(t, n)| (Reverse(n), t), 10);
    let tv = rel(tt);
    let idx: HashIdx<Str, (Str, i64)> = (&tv).map(|(t, _)| t).inv().select(&tv).collect();
    let mut v = drain(rp().select(tags_str.flat_map(split).select(&idx)));
    v.sort_by_key(|&(p, (_, n))| (Reverse(n), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (t, n))| {
        let mut f = vec![V::S(t), V::I(n)];
        f.extend(post_fields(db, p, &["id", "title", "created", "owner", "rep"]));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(v.BountyAmount) AS TotalBountyEarned, RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS ActivityRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON v.UserId = u.Id
//     WHERE u.Reputation > 1000 AND u.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, TotalBountyEarned FROM UserActivity WHERE ActivityRank <= 10)
// SELECT tu.DisplayName, tu.PostCount, tu.AnswerCount, tu.QuestionCount, tu.TotalBountyEarned, AVG(COALESCE(ph.UserId, 0)) AS AveragePostHistoryChanges
// FROM TopUsers tu LEFT JOIN PostHistory ph ON ph.UserId = tu.UserId
// GROUP BY tu.UserId, tu.DisplayName, tu.PostCount, tu.AnswerCount, tu.QuestionCount, tu.TotalBountyEarned ORDER BY tu.TotalBountyEarned DESC;
//
// ActivityRank reads only the distinct post count, so the top users are picked first and the posts x votes product is driven for those alone.
fn q8003(db: &'static So) -> String {
    let User { reputation, creation_date, .. } = &db.user;
    let users = db.user.with(reputation.gt(1000).and(creation_date.lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let v = ranked(drain(users.select(user_distinct_posts(db))), |&(_, n)| Reverse(n), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 4], |a, (t, b)| {
            let b = b.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(1)) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
        });
    let by_user: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let ph = (&tu).group_by(Ident::<User>::new()).select((&by_user).opt()).fold([0i64; 2], |a, h| [a[0] + 1, a[1] + h.map_or(0, |h| db.post_history.user_id.get(h).unwrap())]);
    let mut v = drain((&s).and(user_distinct_posts(db)).and(&ph));
    v.sort_by_key(|&(_, ((a, _), _))| (a[2] == 0, Reverse(a[3])));
    rows(v.into_iter().map(|(u, ((a, n), h))| row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(h[1], h[0])])))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, COUNT(C.CommentId) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY P.CreationDate DESC) AS RN
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN (SELECT C.PostId, C.Id AS CommentId FROM Comments C) C ON P.Id = C.PostId
//     WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.Body, P.CreationDate, P.Score, P.ViewCount, U.DisplayName),
// TopQuestions AS (SELECT RP.PostId, RP.Title, RP.Body, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, RP.CommentCount,
//        (SELECT STRING_AGG(T.TagName, ', ') FROM Tags T WHERE T.ExcerptPostId = RP.PostId) AS Tags FROM RankedPosts RP WHERE RP.RN = 1 ORDER BY RP.Score DESC, RP.ViewCount DESC LIMIT 10)
// SELECT TQ.PostId, TQ.Title, TQ.Body, TQ.CreationDate, TQ.Score, TQ.ViewCount, TQ.OwnerDisplayName, TQ.CommentCount, TQ.Tags FROM TopQuestions TQ;
//
// RN partitions by the post itself, so it is always 1. The order reads only base columns, so the ten questions are picked first.
fn q27726(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tags = (&tp).group_by(Ident::<Post>::new()).select((&excerpt).select(&db.tag.tag_name)).buf_fold(|v| -> Str { Box::leak(v.iter().copied().collect::<Vec<_>>().join(", ").into_boxed_str()) });
    rows(drain((&cc).and((&tags).opt())).into_iter().map(|(p, (c, t))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "score", "views", "owner"]);
        f.extend([V::I(c), harness::fmt::ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 0),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName)
// SELECT ua.UserId, ua.DisplayName, ua.Upvotes, ua.Downvotes, ua.PostCount, ua.CommentCount, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount
// FROM UserActivity ua LEFT JOIN RankedPosts rp ON ua.UserId = rp.OwnerUserId AND rp.PostRank = 1 WHERE ua.PostCount > 0 ORDER BY ua.Upvotes DESC, ua.Downvotes ASC LIMIT 10;
fn q2058(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt())
        .fold([0i64; 2], |a, x| match x {
            Some((t, _)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
            None => a,
        });
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rp = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).select(owner_user));
    let rp = top_per(rp, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let v = drain((&ua).and(user_distinct_posts(db).filt(|n| n > 0)).and(&cc).and((&by_user).map(|(_, p)| p).opt()));
    let v = top_n(v, |&(_, (((a, _), _), _))| (Reverse(a[0]), a[1]), 10);
    rows(v.into_iter().map(|(u, (((a, n), c), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::I(c)]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created", "score", "views"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(COUNT(DISTINCT c.Id), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COALESCE(b.Name, 'None') AS BadgeName, u.Reputation, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.Reputation, b.Name),
// PostRanking AS (SELECT PostId, Title, Score + UpVotes - DownVotes AS NetScore, CreationDate, CommentCount, Reputation,
//        ROW_NUMBER() OVER (ORDER BY Score + UpVotes - DownVotes DESC) AS Rank FROM PostStats)
// SELECT PostId, Title, CreationDate, NetScore, CommentCount, Reputation, Rank FROM PostRanking WHERE Rank <= 10;
//
// The groups are (post, badge name), so the question x owner-badge rows are materialised and grouped; COUNT(DISTINCT c.Id) in a group is its post's comment count.
fn q6332(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let j: MatSet<(Id<Post>, Option<Id<Badge>>)> = qs().select(Ident::<Post>::new().and(owner_user.select(badges_of(db)).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let name_of = (&j).flat_map(|(_, b)| b).select(&db.badge.name);
    let g = (&j)
        .group_by((&post_of).and(name_of.opt()))
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain(rel(drain(&g)).select(Same::<((Id<Post>, Option<Str>), [i64; 2])>::new().and(Same::<((Id<Post>, Option<Str>), [i64; 2])>::new().map(|((p, _), _)| p).select(&cc))));
    let net = |p: Id<Post>, a: [i64; 2]| score.get(p).unwrap() + a[0] - a[1];
    let v = top_n(v, |&(_, (((p, _), a), _))| Reverse(net(p, a)), 10);
    rows(v.into_iter().enumerate().map(|(i, (_, (((p, _), a), c)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(net(p, a)), V::I(c)]);
        f.extend(post_fields(db, p, &["rep"]));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH TopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U WHERE U.Reputation > 0),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, MIN(P.CreationDate) AS FirstPostDate FROM Posts P
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// TopPosts AS (SELECT P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate, RANK() OVER (ORDER BY P.Score DESC) AS PostRank, P.OwnerUserId FROM Posts P WHERE P.Score > 10)
// SELECT U.DisplayName, U.Reputation, COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(PS.TotalViews, 0) AS TotalViews, PP.Title, PP.ViewCount, PP.Score, PP.CreationDate,
//        CASE WHEN PP.Id IS NOT NULL THEN 'Top Post' ELSE 'No Top Post' END AS PostStatus
// FROM TopUsers U LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId LEFT JOIN TopPosts PP ON U.Id = PP.OwnerUserId AND PP.PostRank <= 5
// WHERE U.Rank <= 10 ORDER BY U.Reputation DESC, PS.TotalViews DESC;
fn q2112(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let reputation = &db.user.reputation;
    let tu = top_n(drain(db.user.with(reputation.gt(0)).select(reputation)), |&(_, r)| Reverse(r), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ps = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(view_count.opt()).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let tp = ranked(drain(db.post.with(score.gt(10)).select(score)), |&(_, s)| Reverse(s), false);
    let tp: MatSet<Id<Post>> = rel(tp.into_iter().take_while(|x| x.1 <= 5).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let by_owner: HashIdx<Id<User>, Id<Post>> = (&tp).select(owner_user).inv().collect();
    rows(drain((&tu).select(Ident::<User>::new().and((&ps).opt()).and((&by_owner).opt()))).into_iter().map(|(_, ((u, s), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match s {
            Some(a) => [V::I(a[0]), V::I(a[2])],
            None => [V::I(0), V::I(0)],
        });
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "views", "score", "created"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::S(if p.is_some() { "Top Post" } else { "No Top Post" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(P.ViewCount), 0) AS TotalViews, COALESCE(SUM(P.Score), 0) AS TotalScore,
//        COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, TotalComments, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserActivity)
// SELECT U.DisplayName, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.TotalViews, U.TotalScore, U.TotalComments,
//        ': this user has made more contributions with explanations! More positive engagements!' AS ContributionMessage
// FROM TopUsers U WHERE U.Rank <= 10 OR U.TotalPosts >= 50 ORDER BY U.TotalScore DESC;
fn q25587(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((((t, s), w), c)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + c.is_some() as i64],
            None => a,
        });
    let v = top_n(drain((&s).and(user_distinct_posts(db))), |&(_, (a, _))| Reverse(a[3]), 0);
    let v: Vec<(i64, (Id<User>, ([i64; 5], i64)))> = v.into_iter().enumerate().map(|(i, x)| (i as i64 + 1, x)).collect();
    rows(drain(rel(v).filt(|(r, (_, (_, n)))| r <= 10 || n >= 50)).into_iter().map(|(_, (_, (u, (a, n))))| {
        row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::S(": this user has made more contributions with explanations! More positive engagements!")])
    }))
}

// WITH RankedPosts AS (SELECT p.Id as PostID, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName as OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) as Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.PostID, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.OwnerDisplayName, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = tp.PostID) as CommentCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = tp.PostID AND v.VoteTypeId = 2) as UpVoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = tp.PostID AND v.VoteTypeId = 3) as DownVoteCount,
//        (SELECT STRING_AGG(DISTINCT pt.Name, ', ') FROM PostTypes pt JOIN Posts p ON p.PostTypeId = pt.Id WHERE p.Id = tp.PostID) as PostType
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q5648(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc).and(ptype_name(db))).into_iter().map(|(p, ((c, a), t))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(t)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// TopUsers AS (SELECT UserId, PostCount, Questions, Answers, AcceptedAnswers, UpVotes, DownVotes, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserActivity),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT tu.UserId, u.DisplayName, u.Reputation, tu.PostCount, tu.Questions, tu.Answers, tu.AcceptedAnswers, tu.UpVotes, tu.DownVotes, COALESCE(ub.BadgeCount, 0) AS BadgeCount
// FROM TopUsers tu JOIN Users u ON tu.UserId = u.Id LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// Rank reads only the distinct post count, so the top users are picked first and the posts x votes product is driven for those alone.
fn q7362(db: &'static So) -> String {
    let v = ranked(drain(user_distinct_posts(db)), |&(_, n)| Reverse(n), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, accepted_answer, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((t, acc), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 1 && acc.is_some()) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&s).and(user_distinct_posts(db)).and(&bc));
    v.sort_by_key(|&(_, ((_, n), _))| Reverse(n));
    rows(v.into_iter().map(|(u, ((a, n), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, U.DisplayName AS Author, P.CreationDate, P.Score, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(DISTINCT PH.Id) AS EditCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS RankByScore,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate ASC) AS RankByDate
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId LEFT JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY P.Id, P.Title, U.DisplayName, P.CreationDate, P.Score, P.OwnerUserId),
// TopPosts AS (SELECT PostId, Title, Author, CreationDate, Score, CommentCount, EditCount, RankByScore, RankByDate FROM RankedPosts WHERE RankByScore <= 10 OR RankByDate <= 5)
// SELECT TP.Title, TP.Author, TP.CreationDate, TP.Score, TP.CommentCount, TP.EditCount, PT.Name AS PostType
// FROM TopPosts TP JOIN PostTypes PT ON TP.PostId = PT.Id ORDER BY TP.Score DESC, TP.CreationDate DESC;
//
// `TP.PostId = PT.Id` joins a post id to a post type id, so it goes through the raw ids. The ranks read only base columns, so the posts are ranked and joined
// first and the comment x history product is driven for those alone.
fn q8156(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(owner_user.opt()));
    let mut top = top_per(v.clone(), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    top.extend(top_per(v, |&(_, u)| u, |&(p, _)| (creation_date.get(p).unwrap(), p), 5, false));
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pt: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let hit: MatSet<Id<Post>> = (&tp).with(origid.select(&pt)).collect();
    let s = (&hit).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let ec = (&hit).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    rows(drain((&s).and(&ec).and(origid.select(&pt).select(&db.post_type.name))).into_iter().map(|(p, ((c, e), n))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score"]);
        f.extend([V::I(c), V::I(e), V::S(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount, COALESCE(SUM(c.Score), 0) AS TotalCommentScore, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyEarned
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 GROUP BY u.Id, u.DisplayName)
// SELECT ups.DisplayName, ups.QuestionCount, ups.AnswerCount, ups.TotalCommentScore, ups.TotalBountyEarned, rp.Title, rp.ViewCount, rp.Score
// FROM UserPostStats ups LEFT JOIN RankedPosts rp ON ups.UserId = rp.PostId WHERE rp.Rank <= 3 OR rp.Rank IS NULL ORDER BY ups.TotalBountyEarned DESC, ups.TotalCommentScore DESC;
//
// `ups.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q2977(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(comments_of(db).select(&db.comment.score).opt()).and(bounty.opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some(((t, c), b)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + c.unwrap_or(0), a[3] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id)), |&(p, t)| {
        (t, Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p)
    }, false);
    let rk = rel(per_group(v, |&(_, t)| t).into_iter().map(|((p, _), r)| (origid.get(p).unwrap(), (p, r))).collect());
    let by_id: HashIdx<i64, (i64, (Id<Post>, i64))> = (&rk).map(|(i, _)| i).inv().select(&rk).collect();
    let j = (&s).and((&db.user.origid).select((&by_id).map(|(_, x)| x)).opt()).filt(|(_, r): ([i64; 4], Option<(Id<Post>, i64)>)| r.map_or(true, |(_, r)| r <= 3));
    rows(drain(j).into_iter().map(|(u, (a, r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[0]), V::I(a[2]), V::I(a[3])];
        f.extend(match r {
            Some((p, _)) => post_fields(db, p, &["title", "views", "score"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
//        COUNT(*) OVER (PARTITION BY p.PostTypeId) AS total_count FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT pp.Title AS PopularPostTitle, pp.ViewCount, up.DisplayName AS OwnerDisplayName, bt.Name AS BadgeName, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = pp.PostId) AS CommentCount,
//        CASE WHEN pp.PostTypeId = 1 THEN 'Question' WHEN pp.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END AS PostType,
//        CASE WHEN pp.ViewCount IS NULL THEN 'No views recorded' ELSE CAST(NULLIF(pp.ViewCount, 0) AS VARCHAR) END AS ViewCountDescription,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = pp.PostId AND v.VoteTypeId = 2), 0) AS Upvotes
// FROM RankedPosts pp LEFT JOIN Users up ON pp.PostId = up.Id LEFT JOIN Badges bt ON up.Id = bt.UserId
// WHERE pp.rn <= 10 AND pp.PostTypeId IN (1, 2) AND (bt.Class IS NULL OR bt.Class < 3) ORDER BY pp.Score DESC, pp.ViewCount DESC;
//
// `pp.PostId = up.Id` joins a post id to a user id, so it goes through the raw ids.
fn q21065(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let up = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let ub = origid.select(&uidx).select(Ident::<User>::new().and(badges_of(db).opt())).opt();
    let j = (&tp).with(post_type_id.is_in([1, 2])).select(ub.and(&cc).and(&up)).filt(|((x, _), _): ((Option<(Id<User>, Option<Id<Badge>>)>, i64), i64)| {
        x.and_then(|(_, b)| b).map_or(true, |b| db.badge.class.get(b).unwrap() < 3)
    });
    rows(drain(j).into_iter().map(|(p, ((x, c), u))| {
        let w = view_count.get(p);
        let t = post_type_id.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "views"]);
        f.push(x.map_or(V::Null, |(u, _)| user_col(db, u, "name")));
        f.push(x.and_then(|(_, b)| b).map_or(V::Null, |b| V::S(db.badge.name.get(b).unwrap())));
        f.extend([V::I(c), V::S(if t == 1 { "Question" } else if t == 2 { "Answer" } else { "Other" })]);
        f.push(match w {
            None => V::S("No views recorded"),
            Some(0) => V::Null,
            Some(w) => V::Owned(w.to_string()),
        });
        f.push(V::I(u));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount,
//        COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id
//     LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, BadgeCount, PostCount, CommentCount, LastPostDate, ROW_NUMBER() OVER (ORDER BY Reputation DESC, UpVotes DESC) AS Rank FROM UserStats)
// SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.UpVotes, TU.DownVotes, TU.BadgeCount, TU.PostCount, TU.CommentCount, TU.LastPostDate FROM TopUsers TU WHERE TU.Rank <= 10 ORDER BY TU.Rank;
//
// Rank leads with Reputation, so only users at or above the tenth-highest reputation can rank in the top ten; the product is driven for those alone.
// `V.UserId = U.Id` with `P.OwnerUserId = U.Id` keeps the owner's own votes on the post (own_votes).
fn q8817(db: &'static So) -> String {
    let reputation = &db.user.reputation;
    let tenth = top_n(drain(reputation), |&(_, r)| Reverse(r), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with(reputation.ge(tenth)).collect();
    let own = own_votes(db);
    let s = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.creation_date).and(comments_of(db).opt()).and((&own).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0, 0, 0, i64::MIN], |a, (p, b)| {
            let (d, t) = p.map_or((i64::MIN, None), |((d, _), t)| (d, t));
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64, a[3].max(d)]
        });
    let cc = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&s).and(user_distinct_posts(db)).and(&cc)), |&(u, ((a, _), _))| (Reverse(reputation.get(u).unwrap()), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(u, ((a, n), c))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n), V::I(c), tmax(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopPosts AS (SELECT PostId, Title, ViewCount, OwnerName FROM RankedPosts WHERE Rank <= 3),
// PostComments AS (SELECT pc.Id AS CommentId, pc.PostId, pc.Text AS CommentText, pc.CreationDate AS CommentDate, COUNT(v.Id) AS VoteCount
//     FROM Comments pc LEFT JOIN Votes v ON pc.PostId = v.PostId AND v.VoteTypeId = 2 GROUP BY pc.Id, pc.PostId, pc.Text, pc.CreationDate)
// SELECT tp.Title, tp.ViewCount, tp.OwnerName, COUNT(pc.CommentId) AS TotalComments, COALESCE(SUM(pc.VoteCount), 0) AS TotalUpvotes,
//        CASE WHEN SUM(pc.VoteCount) IS NULL THEN 'No Votes' ELSE 'Has Votes' END AS VoteStatus, (SELECT COUNT(*) FROM Posts p WHERE p.AcceptedAnswerId = tp.PostId) AS AnswersCount
// FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId GROUP BY tp.PostId, tp.Title, tp.ViewCount, tp.OwnerName ORDER BY tp.ViewCount DESC LIMIT 5;
fn q4713(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, accepted_answer, .. } = &db.post;
    let vkey = |p: Id<Post>| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    };
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(date(2024, 10, 1), -30)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (vkey(p), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cs: MatSet<Id<Comment>> = (&tp).select(comments_of(db)).collect();
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let pc = (&cs).group_by(Ident::<Comment>::new()).select((&db.comment.post).select(up.opt())).fold(0i64, |n, v| n + v.is_some() as i64);
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&pc).opt()).fold([0i64; 2], |a, c| match c {
        Some(n) => [a[0] + 1, a[1] + n],
        None => a,
    });
    let acc: HashIdx<Id<Post>, Id<Post>> = accepted_answer.inv().collect();
    let ac = (&tp).group_by(Ident::<Post>::new()).select((&acc).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let v = top_n(drain((&s).and(&ac)), |&(p, _)| vkey(p), 5);
    rows(v.into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["title", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if a[0] == 0 { "No Votes" } else { "Has Votes" }), V::I(n)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(P.Id) AS PostsCount, COUNT(DISTINCT C.Id) AS CommentsCount, COUNT(DISTINCT B.Id) AS BadgesCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id
//     LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// ActiveUsers AS (SELECT U.Id, U.DisplayName, US.Reputation, US.UpVotes - US.DownVotes AS NetVotes, US.PostsCount, US.CommentsCount, US.BadgesCount FROM Users U JOIN UserStats US ON U.Id = US.UserId
//     WHERE U.LastAccessDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND US.PostsCount > 5),
// TopUsers AS (SELECT A.DisplayName, A.Reputation, A.NetVotes, A.PostsCount, A.CommentsCount, A.BadgesCount, RANK() OVER (ORDER BY A.NetVotes DESC, A.Reputation DESC) AS Rank FROM ActiveUsers A)
// SELECT * FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// `V.UserId = U.Id` with `P.OwnerUserId = U.Id` keeps the owner's own votes on the post (own_votes).
fn q9495(db: &'static So) -> String {
    let User { last_access_date, reputation, .. } = &db.user;
    let users = || db.user.with(last_access_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let own = own_votes(db);
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and((&own).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, _)| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64 - (t == Some(3)) as i64, a[1] + p.is_some() as i64, 0]
        });
    let cc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = users().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain((&s).filt(|a| a[1] > 5).and(&cc).and(&bc)), |&(u, ((a, _), _))| (Reverse(a[0]), Reverse(reputation.get(u).unwrap())), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, c), b)), r)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS rn
//     FROM Posts P WHERE P.CreationDate > CURRENT_DATE - INTERVAL '30 days')
// SELECT U.DisplayName, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, R.Title, R.CreationDate,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = R.PostId AND C.UserId IS NOT NULL) AS CommentCount,
//        COALESCE((SELECT AVG(V.BountyAmount) FROM Votes V WHERE V.PostId = R.PostId AND V.VoteTypeId IN (8, 9)), 0) AS AverageBounty
// FROM UserBadges UB JOIN RecentPosts R ON UB.UserId = R.OwnerUserId JOIN Users U ON U.Id = R.OwnerUserId WHERE R.rn = 1
// ORDER BY UB.GoldBadges DESC, UB.SilverBadges DESC, UB.BronzeBadges DESC LIMIT 10;
fn q292(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(current_date(), -30))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().with(&db.comment.user)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ab = (&tp).group_by(Ident::<Post>::new()).select(bv.opt()).fold([0i64; 2], |a, b| match b.flatten() {
        Some(b) => [a[0] + 1, a[1] + b],
        None => a,
    });
    let v = drain((&cc).and(&ab).and(owner_user.select(Ident::<User>::new().and(&ub))));
    let v = top_n(v, |&(_, (_, (_, g)))| (Reverse(g[0]), Reverse(g[1]), Reverse(g[2])), 10);
    rows(v.into_iter().map(|(p, ((c, b), (u, g)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(g[0]), V::I(g[1]), V::I(g[2])];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(c), if b[0] == 0 { V::F(0.0) } else { avg(b[1], b[0]) }]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS ViewRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COALESCE(ub.BadgeCount, 0) AS TotalBadges, COALESCE(ub.GoldBadges, 0) AS GoldBadges, rp.Title AS TopPostTitle, rp.ViewCount AS TopPostViewCount
// FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN RankedPosts rp ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) AND rp.ViewRank = 1
// WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, rp.Title, rp.ViewCount ORDER BY TotalPosts DESC, u.DisplayName;
fn q4404(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_owner: HashIdx<Id<User>, Id<Post>> = (&tp).select(owner_user).inv().collect();
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let qc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 2], |a, c| [a[0] + 1, a[1] + (c == 1) as i64]);
    type K = (Id<User>, Option<Str>, Option<i64>);
    let keys = rel(drain(users().select(Ident::<User>::new().and((&by_owner).select((&db.post.title).opt().and(view_count.opt())).opt()))).into_iter().map(|(_, (u, x))| -> K {
        let (t, w) = x.unwrap_or((None, None));
        (u, t, w)
    }).collect());
    let g: MatSet<K> = (&keys).collect();
    let mut v = drain((&g).select(Same::<K>::new().map(|(u, _, _): K| u).select((&qc).and((&ub).opt()))));
    v.sort_by_key(|&((u, _, _), (n, _))| (Reverse(n), db.user.display_name.get(u).unwrap()));
    rows(v.into_iter().map(|((u, t, w), (n, b))| {
        let b = b.unwrap_or([0, 0]);
        row(vec![user_col(db, u, "name"), V::I(n), V::I(b[0]), V::I(b[1]), harness::fmt::ostr(t), harness::fmt::oint(w)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerName,
fn q6007(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let pt = (&tp).group_by(Ident::<Post>::new()).select((&excerpt).select(&db.tag.tag_name)).buf_fold(|v| -> Str { Box::leak(v.iter().copied().collect::<Vec<_>>().join(", ").into_boxed_str()) });
    rows(drain((&tp).select((&pc).opt().and((&pt).opt()))).into_iter().map(|(p, (c, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([harness::fmt::oint(c), harness::fmt::ostr(t)]);
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.ViewCount) AS TotalViews, SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes,
//        SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount, MAX(u.CreationDate) AS LastActiveDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViews, TotalVotes, BadgeCount, LastActiveDate,
//        RANK() OVER (ORDER BY TotalVotes DESC, TotalViews DESC, PostCount DESC) AS UserRank FROM UserStatistics)
// SELECT tu.UserRank, tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.TotalViews, tu.TotalVotes, tu.BadgeCount, tu.LastActiveDate FROM TopUsers tu WHERE tu.UserRank <= 10 ORDER BY tu.UserRank;
fn q9674(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let Vote { post, .. } = &db.vote;
    let vc = db.vote.group_by(post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and((&vc).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let (t, w, n) = p.map_or((0, None, None), |((t, w), n)| (t, w, n));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + n.unwrap_or(0), a[5] + b.is_some() as i64]
        });
    let v = ranked(drain((&s).and(user_distinct_posts(db))), |&(_, (a, n))| (Reverse(a[4]), a[2] == 0, Reverse(a[3]), Reverse(n)), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), r)| {
        row(vec![V::I(r), user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[4]), V::I(a[5]), user_col(db, u, "ucreated")])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Answered' ELSE 'Unanswered' END AS AnswerStatus
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount, SUM(v.BountyAmount) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.PostsCount, us.TotalBounty, RANK() OVER (ORDER BY us.PostsCount DESC, us.TotalBounty DESC) AS UserRank FROM UserStats us)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerStatus, tu.DisplayName, tu.UserRank
// FROM RankedPosts rp LEFT JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId WHERE rp.Rank <= 5 OR tu.UserRank IS NOT NULL ORDER BY rp.Score DESC, tu.TotalBounty DESC;
//
// `v.UserId = u.Id AND v.PostId = p.Id` keeps the owner's own votes on the post (own_votes).
fn q1798(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, accepted_answer, .. } = &db.post;
    let own = own_votes(db);
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&own).select((&db.vote.bounty_amount).opt()).opt()).opt()).fold([0i64; 2], |a, b| match b.flatten().flatten() {
        Some(b) => [a[0] + 1, a[1] + b],
        None => a,
    });
    let tu = ranked(drain((&us).and(user_distinct_posts(db))), |&(_, (a, n))| (Reverse(n), a[0] == 0, Reverse(a[1])), false);
    let tu = rel(tu.into_iter().map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let v = ranked(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt())), |&(p, u)| (u, Reverse(score.get(p).unwrap()), p), false);
    let rp = rel(per_group(v, |&(_, u)| u).into_iter().map(|((p, _), r)| (p, r)).collect());
    let j = (&rp).select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(p, _)| p).select(owner_user.select(&by_user).opt()))).filt(|((_, r), u): ((Id<Post>, i64), Option<(Id<User>, i64)>)| r <= 5 || u.is_some());
    rows(drain(j).into_iter().map(|(_, ((p, _), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::S(if accepted_answer.get(p).is_some() { "Answered" } else { "Unanswered" }));
        f.extend(match u {
            Some((u, r)) => [user_col(db, u, "name"), V::I(r)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN b.Id IS NOT NULL THEN 1 END), 0) AS BadgeCount, COALESCE(SUM(CASE WHEN v.UserId IS NOT NULL THEN 1 END), 0) AS VoteCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.BadgeCount, us.VoteCount, RANK() OVER (ORDER BY us.VoteCount DESC, us.BadgeCount DESC) AS UserRank FROM UserStats us WHERE us.BadgeCount >= 5)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.AnswerCount, tu.DisplayName, tu.BadgeCount, tu.VoteCount
// FROM RankedPosts rp JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId WHERE rp.rn = 1 ORDER BY rp.ViewCount DESC, rp.Score DESC;
fn q9044(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt().and(votes_by(db).opt())).fold([0i64; 2], |a, (b, v)| [a[0] + b.is_some() as i64, a[1] + v.is_some() as i64]);
    rows(drain((&tp).select(owner_user.select(Ident::<User>::new().and((&us).filt(|a| a[0] >= 5))))).into_iter().map(|(p, (u, a))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "answers"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(rp.UpVoteCount - rp.DownVoteCount), 0) AS VoteBalance, COUNT(rp.PostId) AS PostCount
//     FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId GROUP BY u.Id, u.Reputation)
// SELECT us.UserId, us.Reputation, us.VoteBalance, us.PostCount,
//        CASE WHEN us.PostCount > 10 THEN 'Active Contributor' WHEN us.Reputation > 1000 THEN 'Veteran User' ELSE 'New User' END AS UserCategory
// FROM UserStats us WHERE us.Reputation IS NOT NULL ORDER BY us.Reputation DESC LIMIT 100;
//
// The order reads only Reputation, so the hundred users are picked first and the comment x vote product is driven for their posts alone.
fn q2515(db: &'static So) -> String {
    let reputation = &db.user.reputation;
    let tu = top_n(drain(reputation), |&(_, r)| Reverse(r), 100);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let recent = Ident::<Post>::new().with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ps: MatSet<Id<Post>> = (&tu).select(posts_of(db).select(recent)).collect();
    let rp = (&ps).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold(0i64, |n, (_, t)| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let us = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(&rp).opt()).fold([0i64; 2], |a, b| match b {
        Some(b) => [a[0] + b, a[1] + 1],
        None => a,
    });
    rows(drain(&us).into_iter().map(|(u, a)| {
        let r = reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if a[1] > 10 { "Active Contributor" } else if r > 1000 { "Veteran User" } else { "New User" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(C.CommentsCount, 0)) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentsCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// ActivityRanked AS (SELECT UA.*, ROW_NUMBER() OVER (ORDER BY UA.PostCount DESC, UA.TotalComments DESC) AS ActivityRank FROM UserActivity UA),
// TopUsers AS (SELECT U.*, CASE WHEN U.Reputation < 100 THEN 'Newbie' WHEN U.Reputation < 1000 THEN 'Intermediate' ELSE 'Expert' END AS ReputationLevel FROM Users U
//     WHERE U.Id IN (SELECT UserId FROM ActivityRanked WHERE ActivityRank <= 10))
// SELECT TU.DisplayName, TU.Reputation, TU.ReputationLevel, AR.QuestionCount, AR.AnswerCount, AR.TotalComments
// FROM TopUsers TU JOIN ActivityRanked AR ON TU.Id = AR.UserId LEFT JOIN Badges B ON TU.Id = B.UserId AND B.Class = 1 WHERE B.Id IS NULL ORDER BY TU.Reputation DESC, AR.QuestionCount DESC;
fn q3865(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and((&cc).opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, c)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.unwrap_or(0)],
        None => a,
    });
    let v = top_n(drain((&ua).and(user_distinct_posts(db))), |&(_, (a, n))| (Reverse(n), Reverse(a[2])), 10);
    let tu = rel(v.into_iter().map(|(u, (a, _))| (u, a)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, [i64; 3])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let gold: MatSet<Id<User>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).collect();
    rows(drain(db.user.minus(&gold).select((&by_user).map(|(_, a)| a))).into_iter().map(|(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::S(if r < 100 { "Newbie" } else if r < 1000 { "Intermediate" } else { "Expert" }), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions, MAX(v.CreationDate) AS LastVoteDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// RankedUserActivity AS (SELECT ua.UserId, ua.DisplayName, ua.TotalQuestions, ua.TotalAnswers, ROW_NUMBER() OVER (ORDER BY ua.TotalQuestions DESC, ua.TotalAnswers DESC) AS Rank
//     FROM UserActivity ua WHERE ua.TotalQuestions > 0),
// RecentVotes AS (SELECT v.UserId, COUNT(*) AS RecentVoteCount FROM Votes v WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.UserId)
// SELECT rua.DisplayName, rua.TotalQuestions, rua.TotalAnswers, COALESCE(rv.RecentVoteCount, 0) AS RecentVoteCount,
//        CASE WHEN rv.RecentVoteCount IS NULL THEN 'No recent votes' WHEN rv.RecentVoteCount > 10 THEN 'Active voter' ELSE 'Occasional voter' END AS VotingStatus
// FROM RankedUserActivity rua LEFT JOIN RecentVotes rv ON rua.UserId = rv.UserId WHERE rua.Rank <= 100 ORDER BY rua.Rank;
fn q890(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((t, _)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64],
            None => a,
        });
    let Vote { user, creation_date, .. } = &db.vote;
    let rv = db.vote.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&ua).filt(|a| a[1] > 0).and((&rv).opt())), |&(_, (a, _))| (Reverse(a[1]), Reverse(a[0])), 100);
    rows(v.into_iter().map(|(u, (a, r))| {
        row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[0]), V::I(r.unwrap_or(0)), V::S(match r {
            None => "No recent votes",
            Some(r) if r > 10 => "Active voter",
            _ => "Occasional voter",
        })])
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COALESCE(COUNT(DISTINCT c.Id), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.ViewCount, pd.OwnerDisplayName, pd.CommentCount, pd.UpVotes, pd.DownVotes, (pd.UpVotes - pd.DownVotes) AS Score,
//        RANK() OVER (ORDER BY (pd.UpVotes - pd.DownVotes) DESC) AS Rank FROM PostDetails pd WHERE pd.rn = 1)
// SELECT tp.Title, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes, tp.Score FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Score DESC;
//
// rn partitions by the post itself, so it is always 1.
fn q30071(db: &'static So) -> String {
    let recent = || db.post.with((&db.post.creation_date).ge(add_years(current_date(), -1)));
    let s = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&s).and(&cc)), |&(_, (a, _))| Reverse(a[0] - a[1]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, (a, c)), _)| {
        let mut f = post_fields(db, p, &["title", "views", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount, SUM(COALESCE(p.Score, 0)) AS TotalScore FROM Posts p GROUP BY p.OwnerUserId),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, pu.PostCount, pu.QuestionCount, pu.AnswerCount, pu.TotalScore,
//        CASE WHEN pu.PostCount = 0 THEN 0 ELSE pu.TotalScore / NULLIF(pu.PostCount, 0) END AS AverageScore FROM RankedUsers u LEFT JOIN PostStats pu ON u.Id = pu.OwnerUserId)
// SELECT ups.UserId, ups.DisplayName, ups.PostCount, ups.QuestionCount, ups.AnswerCount, ups.TotalScore, ups.AverageScore, COALESCE(b.Class, 0) AS BadgeClass
// FROM UserPostStats ups LEFT JOIN (SELECT UserId, MAX(Class) AS Class FROM Badges GROUP BY UserId) b ON ups.UserId = b.UserId
// WHERE ups.AverageScore IS NOT NULL AND ups.AverageScore > 1.5 ORDER BY ups.AverageScore DESC LIMIT 10;
fn q3888(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let mc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold(0i64, |m, c| m.max(c));
    let avgs = |a: [i64; 4]| a[3] as f64 / a[0] as f64;
    let v = drain((&ps).filt(|a| avgs(a) > 1.5).and((&mc).opt()));
    let v = top_n(v, |&(_, (a, _))| Reverse(fkey(avgs(a))), 10);
    rows(v.into_iter().map(|(u, (a, c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::F(avgs(a)), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COALESCE(AVG(CASE WHEN p.Score IS NOT NULL THEN p.Score END), 0) AS AvgPostScore
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT u.DisplayName, u.Reputation, u.CreationDate, uv.TotalVotes, uv.UpVotes, uv.DownVotes, uv.AvgPostScore, rp.PostId, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostCreationDate
// FROM Users u LEFT JOIN UserVoteStats uv ON u.Id = uv.UserId LEFT JOIN RecentPosts rp ON u.Id = rp.OwnerUserId AND rp.PostRank = 1
// WHERE u.Reputation > 1000 AND (uv.TotalVotes IS NULL OR uv.TotalVotes > 10) ORDER BY uv.TotalVotes DESC NULLS LAST, u.DisplayName;
fn q4178(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let uv = users()
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.post).select(&db.post.score).opt())).opt())
        .fold([0i64; 5], |a, v| match v {
            Some((t, s)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + s.is_some() as i64, a[4] + s.unwrap_or(0)],
            None => a,
        });
    let recent = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(owner_user));
    let recent = top_per(recent, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(recent.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    rows(drain((&uv).filt(|a| a[0] > 10).and((&by_user).map(|(_, p)| p).opt())).into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["name", "rep", "ucreated"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), if a[3] == 0 { V::F(0.0) } else { avg(a[4], a[3]) }]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "created"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT ru.UserId, ru.DisplayName, ru.Reputation, ru.TotalPosts, ru.QuestionCount, ru.AnswerCount FROM RankedUsers ru WHERE ru.ReputationRank <= 10)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.QuestionCount, tu.AnswerCount, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId GROUP BY tu.UserId, tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.QuestionCount, tu.AnswerCount ORDER BY tu.Reputation DESC;
fn q9283(db: &'static So) -> String {
    let User { reputation, creation_date, .. } = &db.user;
    let v = ranked(drain(db.user.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ps = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    rows(drain((&ps).and(&bc)).into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserVoteCounts AS (SELECT v.UserId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpvoteCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownvoteCount,
//        SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVoteCount FROM Votes v GROUP BY v.UserId)
// SELECT u.DisplayName, p.PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(uv.UpvoteCount, 0) AS Upvotes, COALESCE(uv.DownvoteCount, 0) AS Downvotes,
//        CASE WHEN uf.UserPostRank = 1 THEN 'Most Recent Post' WHEN uf.UserPostRank < 5 THEN 'Top 5 Posts' ELSE 'Other Post' END AS PostCategory
// FROM Users u LEFT JOIN RankedPosts p ON u.Id = p.OwnerUserId LEFT JOIN UserVoteCounts uv ON uv.UserId = u.Id
// LEFT JOIN (SELECT DISTINCT ownerUserId, UserPostRank FROM RankedPosts) uf ON uf.OwnerUserId = p.OwnerUserId
// WHERE u.Reputation > 1000 ORDER BY u.DisplayName, p.CreationDate DESC;
//
// uf joins on the owner only, so each of a user's posts meets every rank that user has.
fn q1732(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user)), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false);
    let rp = rel(per_group(v, |&(_, u)| u).into_iter().map(|((p, u), r)| (u, (p, r))).collect());
    let posts: HashIdx<Id<User>, Id<Post>> = (&rp).map(|(u, _)| u).inv().select((&rp).map(|(_, (p, _))| p)).collect();
    let uf: MatSet<(Id<User>, i64)> = (&rp).map(|(u, (_, r))| (u, r)).collect();
    let ufi: HashIdx<Id<User>, (Id<User>, i64)> = (&uf).map(|(u, _)| u).inv().collect();
    let Vote { user, vote_type_id, .. } = &db.vote;
    let uv = db.vote.group_by(user).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let j = db.user.with((&db.user.reputation).gt(1000)).select((&uv).opt().and((&posts).select(Ident::<Post>::new().and(owner_user.select(&ufi).map(|(_, r)| r))).opt()));
    rows(drain(j).into_iter().map(|(u, (a, p))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(match p {
            Some((p, _)) => post_fields(db, p, &["id", "title", "created", "views"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        f.extend([V::I(a[0]), V::I(a[1]), V::S(match p.map(|(_, r)| r) {
            Some(1) => "Most Recent Post",
            Some(r) if r < 5 => "Top 5 Posts",
            _ => "Other Post",
        })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AcceptedAnswers, TotalUpvotes, TotalDownvotes, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserActivity)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.AcceptedAnswers, tu.TotalUpvotes, tu.TotalDownvotes,
//        (tu.TotalUpvotes * 1.0 / NULLIF(tu.TotalPosts, 0)) AS UpvoteRatio, (tu.TotalDownvotes * 1.0 / NULLIF(tu.TotalPosts, 0)) AS DownvoteRatio
// FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// Rank reads only the distinct post count, so the top users are picked first and the posts x votes product is driven for those alone.
fn q7029(db: &'static So) -> String {
    let v = top_n(drain(user_distinct_posts(db)), |&(_, n)| Reverse(n), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { post_type_id, accepted_answer, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((t, acc), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 1 && acc.is_some()) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    rows(drain((&s).and(user_distinct_posts(db))).into_iter().map(|(u, (a, n))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        f.extend([V::F(a[3] as f64 / n as f64), V::F(a[4] as f64 / n as f64)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.LastAccessDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY u.Id, u.DisplayName)
// SELECT au.DisplayName, au.PostCount, au.TotalScore, rp.Title, rp.Score, rp.UpVotes, rp.DownVotes, CASE WHEN rp.PostRank = 1 THEN 'Top Post' ELSE 'Other Post' END AS PostStatus
// FROM ActiveUsers au LEFT JOIN RankedPosts rp ON au.UserId = rp.OwnerUserId WHERE (rp.Score > 10 OR rp.UpVotes > 5) ORDER BY au.TotalScore DESC, rp.Score DESC LIMIT 50;
fn q2444(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)));
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = ranked(drain((&vc).and(owner_user.opt())), |&(p, (_, u))| (u, Reverse(score.get(p).unwrap()), p), false);
    let rk = rel(per_group(v, |&(_, (_, u))| u).into_iter().map(|((p, (a, _)), r)| (p, (a, r))).collect());
    let by_post: HashIdx<Id<Post>, (Id<Post>, ([i64; 2], i64))> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let au = db.user.with((&db.user.last_access_date).ge(add_days(date(2024, 10, 1), -30))).group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let hit = posts_of(db).select(&by_post).filt(|(p, (a, _)): (Id<Post>, ([i64; 2], i64))| score.get(p).unwrap() > 10 || a[0] > 5);
    let v = top_n(drain((&au).and(hit)), |&(_, (a, (p, _)))| (Reverse(a[1]), Reverse(score.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(u, (a, (p, (b, r))))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1])];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(b[0]), V::I(b[1]), V::S(if r == 1 { "Top Post" } else { "Other Post" })]);
        row(f)
    }))
}

// WITH Rankings AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Creator, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COALESCE(p.ViewCount, 0) AS ViewCount,
//        ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT v.Id) DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Posts a ON p.Id = a.ParentId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate, p.ViewCount),
// RankedPosts AS (SELECT PostId, Title, Creator, CommentCount, AnswerCount, UpVotes, DownVotes, ViewCount, Rank FROM Rankings WHERE Rank <= 10)
// SELECT rp.*, (UpVotes - DownVotes) AS NetVotes, CASE WHEN AnswerCount >= 5 THEN 'Popular' ELSE 'Less Popular' END AS PopularityStatus FROM RankedPosts rp ORDER BY Rank;
fn q9599(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let s = qs()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(children_of(db).opt()))
        .fold([0i64; 2], |a, ((_, t), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cnt = |r: &'static HashIdx<Id<Post>, _>| qs().group_by(Ident::<Post>::new()).select(r.opt()).fold(0i64, |n, x: Option<_>| n + x.is_some() as i64);
    let (cc, vc) = (cnt(comments_of(db)), qs().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, x| n + x.is_some() as i64));
    let ac = qs().group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let v = top_n(drain((&s).and(&cc).and(&vc).and(&ac)), |&(p, ((_, n), _))| (Reverse(n), Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().enumerate().map(|(i, (p, (((a, c), _), n)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1]), V::I(view_count.get(p).unwrap_or(0)), V::I(i as i64 + 1), V::I(a[0] - a[1]), V::S(if n >= 5 { "Popular" } else { "Less Popular" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, p.AcceptedAnswerId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// RecentVotes AS (SELECT PostId, COUNT(*) AS TotalVotes, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes WHERE CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY PostId)
// SELECT up.UserId, up.Reputation, up.PostCount, rp.Title, rp.CreationDate, rp.Score, rv.TotalVotes, rv.UpVotes, rv.DownVotes
// FROM UserReputation up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId LEFT JOIN RecentVotes rv ON rp.Id = rv.PostId
// WHERE (rp.rn = 1 OR rv.TotalVotes IS NOT NULL) ORDER BY up.Reputation DESC, rp.Score DESC NULLS LAST LIMIT 100;
fn q3082(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(owner_user)), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false);
    let rk = rel(per_group(v, |&(_, u)| u).into_iter().map(|((p, _), r)| (p, r)).collect());
    let Vote { post, vote_type_id, creation_date: vd, .. } = &db.vote;
    let rv = db.vote.with(vd.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(post).select(vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    type R = (Id<Post>, i64);
    let pid = || Same::<R>::new().map(|(p, _): R| p);
    let j = (&rk).select(Same::<R>::new().and(pid().select((&rv).opt())).and(pid().select(owner_user.select(Ident::<User>::new().and(user_distinct_posts(db))))))
        .filt(|(((_, r), v), _): ((R, Option<[i64; 3]>), (Id<User>, i64))| r == 1 || v.is_some());
    let v = top_n(drain(j), |&(_, (((p, _), _), (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())), 100);
    rows(v.into_iter().map(|(_, (((p, _), rv), (u, n)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend(match rv {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH Tag_Posts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, P.Tags, COUNT(DISTINCT C.Id) AS CommentCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, (SELECT COUNT(*) FROM Votes V2 WHERE V2.PostId = P.Id AND V2.VoteTypeId = 4) AS BountyCount,
//        U.DisplayName AS OwnerDisplayName, P.OwnerUserId
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1
//     GROUP BY P.Id, P.Title, P.Body, P.CreationDate, P.Tags, U.DisplayName, P.OwnerUserId),
// Tag_Split AS (SELECT PostId, TRIM(UNNEST(string_to_array(Tags, '>'))) AS TagName FROM Tag_Posts)
// SELECT T.TagName, COUNT(DISTINCT TP.PostId) AS PostCount, AVG(TP.CommentCount) AS AvgCommentCount, SUM(TP.UpVoteCount) AS TotalUpVotes, SUM(TP.DownVoteCount) AS TotalDownVotes,
//        SUM(TP.BountyCount) AS TotalBounties
// FROM Tag_Split T JOIN Tag_Posts TP ON T.PostId = TP.PostId WHERE T.TagName IS NOT NULL AND T.TagName <> '' GROUP BY T.TagName ORDER BY PostCount DESC, TotalUpVotes DESC;
fn q27946(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let vt = || votes_of(db).select(&db.vote.vote_type_id);
    let pv = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(vt().opt())).fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = qs().group_by(Ident::<Post>::new()).select(vt().opt()).fold(0i64, |n, t| n + (t == Some(4)) as i64);
    let pairs: MatSet<(Id<Post>, Str)> = qs().select(Ident::<Post>::new().and(tags_str.flat_map(|t: Str| t.split('>').map(str::trim)).filt(|t: Str| !t.is_empty()))).collect();
    type R = (Id<Post>, Str);
    let g = (&pairs)
        .group_by(Same::<R>::new().map(|(_, t): R| t))
        .select(Same::<R>::new().map(|(p, _): R| p).select((&pv).and(&cc).and(&bc)))
        .fold([0i64; 5], |a, ((v, c), b)| [a[0] + 1, a[1] + c, a[2] + v[0], a[3] + v[1], a[4] + b]);
    let mut v = drain(&g);
    v.sort_by_key(|&(_, a)| (Reverse(a[0]), Reverse(a[2])));
    rows(v.into_iter().map(|(t, a)| row(vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])])))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// ClosedPosts AS (SELECT p.Id AS PostId, ph.Comment AS CloseReason, p.Title FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10)
// SELECT r.PostId, r.Title, r.CreationDate, r.ViewCount, r.CommentCount, COALESCE(v.UpVotes, 0) AS TotalUpVotes, COALESCE(v.DownVotes, 0) AS TotalDownVotes,
//        CASE WHEN c.CloseReason IS NOT NULL THEN 'Closed: ' || c.CloseReason ELSE 'Open' END AS Status
// FROM RecentPosts r LEFT JOIN PostVotes v ON r.PostId = v.PostId LEFT JOIN ClosedPosts c ON r.PostId = c.PostId WHERE r.rn <= 5 ORDER BY r.CreationDate DESC;
fn q3297(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let Vote { post, vote_type_id, .. } = &db.vote;
    let pv = db.vote.group_by(post).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    rows(drain((&cc).and((&pv).opt()).and(closes.opt())).into_iter().map(|(p, ((c, v), h))| {
        let v = v.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(c), V::I(v[0]), V::I(v[1])]);
        f.push(match h.and_then(|h| db.post_history.comment.get(h)) {
            Some(r) => V::Owned(format!("Closed: {r}")),
            None => V::S("Open"),
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, COALESCE(p.ViewCount, 0) AS ViewCount, COALESCE(p.Score, 0) AS Score, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        RANK() OVER (ORDER BY COALESCE(p.Score, 0) DESC, COALESCE(p.ViewCount, 0) DESC) AS RankScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.ViewCount, p.Score),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// UserTopPost AS (SELECT p.OwnerUserId, MAX(p.ViewCount) AS MaxViewCount FROM Posts p WHERE p.OwnerUserId IS NOT NULL GROUP BY p.OwnerUserId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.CommentCount, rp.VoteCount, ub.BadgeCount, ut.MaxViewCount
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.PostId = ub.UserId LEFT JOIN UserTopPost ut ON rp.PostId = ut.OwnerUserId WHERE rp.RankScore <= 10 ORDER BY rp.RankScore;
//
// RankScore reads only base columns, so the top questions are picked first. `rp.PostId = ub.UserId` and `rp.PostId = ut.OwnerUserId` join a post id to a user id,
// so they go through the raw ids.
fn q6467(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, owner_user, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.eq(1))).select(score)), |&(p, s)| (Reverse(s), Reverse(view_count.get(p).unwrap_or(0))), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ut = db.post.with(owner_user).group_by(owner_user).select(view_count.opt()).fold(None, |m: Option<i64>, w| match (m, w) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    });
    let mut v = drain((&s).and(&vc).and(origid.select(&uidx).select(&ub).opt()).and(origid.select(&uidx).select(&ut).opt()));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p).unwrap_or(0))));
    rows(v.into_iter().map(|(p, (((c, n), b), m))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(view_count.get(p).unwrap_or(0)), V::I(score.get(p).unwrap()), V::I(c), V::I(n), harness::fmt::oint(b), harness::fmt::oint(m.flatten())]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments,
//        SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes, SUM(COALESCE(P.Score, 0)) AS TotalScore, RANK() OVER (ORDER BY COUNT(DISTINCT P.Id) DESC) AS ActivityRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id
//     WHERE U.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalVotes, TotalScore, ActivityRank FROM UserActivity WHERE ActivityRank <= 10)
// SELECT TU.DisplayName, TU.TotalPosts, TU.TotalComments, TU.TotalVotes, TU.TotalScore, COALESCE(BA.BadgeCount, 0) AS TotalBadges
// FROM TopUsers TU LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges WHERE Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY UserId) BA ON TU.UserId = BA.UserId
// ORDER BY TU.TotalScore DESC, TU.TotalPosts DESC;
//
// ActivityRank reads only the distinct post count, so the top users are picked first and the posts x comments x votes product is driven for those alone.
// `V.UserId = U.Id` with `P.OwnerUserId = U.Id` keeps the owner's own votes on the post (own_votes).
fn q8510(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = ranked(drain(db.user.with((&db.user.creation_date).ge(add_years(t0, -1))).select(user_distinct_posts(db))), |&(_, n)| Reverse(n), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let own = own_votes(db);
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and((&own).select(&db.vote.vote_type_id).opt())).opt().and(comments_by(db).opt()))
        .fold([0i64; 2], |a, (p, _)| match p {
            Some((s, t)) => [a[0] + matches!(t, Some(2 | 3)) as i64, a[1] + s],
            None => a,
        });
    let cc = (&tu).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ba = db.badge.with((&db.badge.date).ge(add_years(t0, -1))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&s).and(user_distinct_posts(db)).and(&cc).and((&ba).opt()));
    v.sort_by_key(|&(_, (((a, n), _), _))| (Reverse(a[1]), Reverse(n)));
    rows(v.into_iter().map(|(u, (((a, n), c), b))| row(vec![user_col(db, u, "name"), V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), V::I(b.unwrap_or(0))])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= (cast('2024-10-01' as date) - INTERVAL '1 YEAR')
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, COALESCE(b.Name, 'No Badge') AS UserBadge
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId AND b.Date >= (cast('2024-10-01' as date) - INTERVAL '1 YEAR') ORDER BY tp.Score DESC;
//
// Rank reads only base columns, so the top posts are picked first. `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q5042(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let since = add_years(date(2024, 10, 1), -1);
    let v = drain(db.post.with(creation_date.ge(since)).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let hit: MatSet<Id<Post>> = (&tp).with(origid.select(&uidx)).collect();
    let s = (&hit).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&hit).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let recent = Ident::<Badge>::new().with((&db.badge.date).ge(since));
    rows(drain((&s).and(&cc).and(origid.select(&uidx).select(Ident::<User>::new().and(badges_of(db).select(recent).opt())))).into_iter().map(|(p, ((a, c), (u, b)))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvotesReceived,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvotesReceived, SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     WHERE u.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, UpvotesReceived, DownvotesReceived, TotalComments,
//        RANK() OVER (ORDER BY TotalPosts DESC, UpvotesReceived DESC) AS UserRank FROM UserActivity WHERE TotalPosts > 0)
// SELECT u.* FROM TopUsers u WHERE UserRank <= 10 ORDER BY UserRank;
fn q6443(db: &'static So) -> String {
    let users = || db.user.with((&db.user.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&cc).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((t, v), c)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + c.unwrap_or(0)],
            None => a,
        });
    let v = ranked(drain((&s).and(user_distinct_posts(db).filt(|n| n > 0))), |&(_, (a, n))| (Reverse(n), Reverse(a[2])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount, SUM(u.Reputation) AS TotalReputation FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '2 years' GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, COALESCE(uv.UpVotes, 0) AS UpVotes, COALESCE(uv.DownVotes, 0) AS DownVotes, au.DisplayName, au.PostsCount, au.TotalReputation
// FROM RankedPosts rp LEFT JOIN UserVotes uv ON rp.PostId = uv.PostId JOIN ActiveUsers au ON rp.AnswerCount > 0 WHERE rp.Rank <= 10 ORDER BY rp.ViewCount DESC, rp.PostId;
//
// The ON clause names only rp, so the posts with answers are crossed with ActiveUsers.
fn q6656(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, answer_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let Vote { post, vote_type_id, .. } = &db.vote;
    let uv = db.vote.group_by(post).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let au = db.user.with((&db.user.creation_date).ge(add_years(t0, -2))).group_by(Ident::<User>::new()).select((&db.user.reputation).and(posts_of(db))).fold([0i64; 2], |a, (r, _)| [a[0] + 1, a[1] + r]);
    let left = rel(drain((&tp).with(answer_count.gt(0)).select((&uv).opt())));
    let mut v = Vec::new();
    (&left).cross(&au).drive(|(_, u), ((p, w), a)| v.push((p, w, u, a)));
    rows(v.into_iter().map(|(p, w, u, a)| {
        let w = w.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(w[0]), V::I(w[1]), user_col(db, u, "name"), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount,
//        SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CreationDate, CommentCount, UpVotes, DownVotes, CloseCount, ReopenCount, RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS RankScore,
//        RANK() OVER (ORDER BY CreationDate DESC) AS RankRecent FROM PostMetrics)
// SELECT *, (RankScore + RankRecent) AS TotalRank FROM TopPosts WHERE (RankScore + RankRecent) <= 10 ORDER BY TotalRank;
//
// Both ranks read only base columns, so the posts are ranked and cut first and the comment x vote x history product is driven for the survivors alone.
fn q7280(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, false);
    let v = ranked(v, |&((p, _), _)| Reverse(creation_date.get(p).unwrap()), false);
    let rk = rel(v.into_iter().map(|(((p, _), a), b)| (p, a, b)).collect());
    let keep = rel(drain((&rk).filt(|(_, a, b)| a + b <= 10)).into_iter().map(|x| x.1).collect());
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64, i64)> = (&keep).map(|(p, _, _)| p).inv().select(&keep).collect();
    let kp: MatSet<Id<Post>> = (&keep).map(|(p, _, _)| p).collect();
    let s = (&kp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 4], |a, ((_, t), h)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (h == Some(10)) as i64, a[3] + (h == Some(11)) as i64]);
    let cc = (&kp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&s).and(&cc).and(&by_post)).into_iter().map(|(p, ((a, c), (_, r1, r2)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.push(V::I(c));
        f.extend(a.map(V::I));
        f.extend([V::I(r1), V::I(r2), V::I(r1 + r2)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.AnswerCount, p.CommentCount, p.FavoriteCount, CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Not Accepted' END AS AnswerStatus,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId IN (2, 3)) AS VoteCount
//     FROM Posts p)
// SELECT ua.DisplayName, ua.PostCount, ua.TotalScore, ua.AvgViewCount, ps.PostId, ps.Title, ps.AnswerCount, ps.CommentCount, ps.FavoriteCount, ps.AnswerStatus, ps.RecentPostRank, ps.VoteCount
// FROM UserActivity ua JOIN PostStatistics ps ON ua.UserId = ps.PostId WHERE ua.PostCount > 5 AND ps.AnswerStatus = 'Accepted' ORDER BY ua.TotalScore DESC, ps.RecentPostRank LIMIT 100 OFFSET 0;
//
// `ua.UserId = ps.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q1141(db: &'static So) -> String {
    let Post { owner_user, creation_date, origid, accepted_answer, score, view_count, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)],
        None => [a[0] + 1, a[1], a[2]],
    });
    let v = ranked(drain(db.post.select(owner_user.opt())), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false);
    let rk = rel(per_group(v, |&(_, u)| u).into_iter().map(|((p, _), r)| (origid.get(p).unwrap(), (p, r))).collect());
    let by_id: HashIdx<i64, (i64, (Id<Post>, i64))> = (&rk).map(|(i, _)| i).inv().select((&rk).filt(|(_, (p, _))| accepted_answer.get(p).is_some())).collect();
    let v = drain((&ua).and(user_distinct_posts(db).filt(|n| n > 5)).and((&db.user.origid).select((&by_id).map(|(_, x)| x))));
    let v = top_n(v, |&(_, ((a, _), (_, r)))| (Reverse(a[1]), r), 100);
    let hit: MatSet<Id<Post>> = rel(v.iter().map(|x| x.1 .1 .0).collect()).map(|p| p).collect();
    let vc = (&hit).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    rows(v.into_iter().map(|(u, ((a, n), (p, r)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[1]), avg(a[2], a[0])];
        f.extend(post_fields(db, p, &["id", "title", "answers", "comments", "favorites"]));
        f.extend([V::S("Accepted"), V::I(r), V::I(vc.get(p).unwrap())]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(V.BountyAmount) AS TotalBounties, AVG(U.Reputation) AS AverageReputation
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY U.Id, U.DisplayName),
// PostScore AS (SELECT P.Id AS PostId, P.Title, (P.Score + COALESCE(P.FavoriteCount, 0) * 2) AS AdjustedScore,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY (P.Score + COALESCE(P.FavoriteCount, 0) * 2) DESC) AS Rank
//     FROM Posts P WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months')
// SELECT UA.DisplayName, UA.PostCount, UA.QuestionCount, UA.AnswerCount, UA.TotalBounties, UA.AverageReputation, PS.Title, PS.AdjustedScore, PS.Rank
// FROM UserActivity UA LEFT JOIN PostScore PS ON UA.UserId = PS.PostId WHERE UA.PostCount > 5 AND PS.Rank <= 10 ORDER BY UA.AverageReputation DESC, PS.AdjustedScore DESC;
//
// `UA.UserId = PS.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q2883(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, favorite_count, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let adj = |p: Id<Post>| score.get(p).unwrap() + favorite_count.get(p).unwrap_or(0) * 2;
    let v = ranked(drain(db.post.with(creation_date.gt(add_months(t0, -6))).select(post_type_id)), |&(p, t)| (t, Reverse(adj(p)), p), false);
    let rk = rel(per_group(v, |&(_, t)| t).into_iter().map(|((p, _), r)| (origid.get(p).unwrap(), (p, r))).collect());
    let by_id: HashIdx<i64, (i64, (Id<Post>, i64))> = (&rk).map(|(i, _)| i).inv().select((&rk).filt(|(_, (_, r))| r <= 10)).collect();
    let users = || db.user.with((&db.user.creation_date).gt(add_years(t0, -1)));
    let ua = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
            }
            None => a,
        });
    let v = drain((&ua).and(user_distinct_posts(db).filt(|n| n > 5)).and((&db.user.origid).select((&by_id).map(|(_, x)| x))));
    let mut v = v;
    v.sort_by_key(|&(u, (_, (p, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(adj(p))));
    rows(v.into_iter().map(|(u, ((a, n), (p, r)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::F(db.user.reputation.get(u).unwrap() as f64)];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(adj(p)), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// PostVoteStats AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(*) AS TotalVotes FROM Votes v GROUP BY v.PostId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation > 1000)
// SELECT pu.DisplayName AS UserDisplayName, COUNT(DISTINCT rp.Id) AS PostsCount, SUM(COALESCE(pvs.UpVotes, 0)) AS TotalUpVotes, SUM(COALESCE(pvs.DownVotes, 0)) AS TotalDownVotes,
//        AVG(COALESCE(rp.Score, 0)) AS AveragePostScore, MAX(rp.CreationDate) AS LastPostDate
// FROM TopUsers pu LEFT JOIN RankedPosts rp ON pu.UserId = rp.OwnerUserId LEFT JOIN PostVoteStats pvs ON rp.Id = pvs.PostId WHERE pu.UserRank <= 10
// GROUP BY pu.DisplayName ORDER BY TotalUpVotes DESC, AveragePostScore DESC;
fn q2943(db: &'static So) -> String {
    let reputation = &db.user.reputation;
    let v = ranked(drain(db.user.with(reputation.gt(1000)).select(reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Vote { post, vote_type_id, .. } = &db.vote;
    let pvs = db.vote.group_by(post).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let recent = || Ident::<Post>::new().with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let g = (&tu)
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(recent()).select(Ident::<Post>::new().and((&pvs).opt())).opt())
        .fold((0i64, [0i64; 5]), |(n, a), x| match x {
            Some((p, v)) => {
                let v = v.unwrap_or([0, 0]);
                (n + 1, [a[0] + 1, a[1] + v[0], a[2] + v[1], a[3] + db.post.score.get(p).unwrap(), a[4].max(db.post.creation_date.get(p).unwrap())])
            }
            None => (n + 1, a),
        });
    let pc = (&tu).group_by(&db.user.display_name).select(posts_of(db).select(recent()).opt()).buf_fold(distinct_some);
    let mut v = drain((&g).and(&pc));
    let avgs = |n: i64, a: [i64; 5]| a[3] as f64 / n as f64;
    v.sort_by_key(|&(_, ((n, a), _))| (Reverse(a[1]), Reverse(fkey(avgs(n, a)))));
    rows(v.into_iter().map(|(name, ((n, a), c))| row(vec![V::S(name), V::I(c), V::I(a[1]), V::I(a[2]), V::F(avgs(n, a)), if a[0] == 0 { V::Null } else { V::T(a[4]) }])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, COUNT(DISTINCT ph.UserId) AS CloseVotesCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT us.DisplayName, us.TotalPosts, us.PositiveScorePosts, us.TotalViews, rp.Title, rp.CreationDate, rp.Score, COALESCE(cp.CloseVotesCount, 0) AS CloseVotesCount
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId AND rp.UserPostRank <= 3 LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId
// WHERE us.TotalPosts > 5 ORDER BY us.TotalViews DESC, rp.Score DESC LIMIT 50;
fn q263(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, post_history_type_id, user, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(user.opt()).buf_fold(distinct_some);
    let ups = user_posts(db);
    let v = drain((&tp).select(Ident::<Post>::new().and((&cp).opt()).and(owner_user.select(Ident::<User>::new().and((&ups).filt(|a| a[1] > 5))))));
    let v = top_n(v, |&(p, (_, (_, a)))| (a[5] == 0, Reverse(a[6]), Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((_, c), (u, a)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[8]), nullable(a[6], a[5])];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.push(V::I(c.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostCommentStats AS (SELECT PostId, COUNT(*) AS CommentCount, MAX(CreationDate) AS LastCommentDate FROM Comments GROUP BY PostId)
// SELECT up.DisplayName, rp.PostId, rp.Title, rp.Score, rp.ViewCount, COALESCE(pcs.CommentCount, 0) AS CommentCount, COALESCE(ub.BadgeCount, 0) AS TotalBadges, ub.GoldBadges, ub.SilverBadges,
//        ub.BronzeBadges, rp.RankScore
// FROM Users up JOIN RankedPosts rp ON up.Id = rp.PostId LEFT JOIN UserBadges ub ON up.Id = ub.UserId LEFT JOIN PostCommentStats pcs ON rp.PostId = pcs.PostId
// WHERE rp.RankScore <= 5 ORDER BY rp.Score DESC, up.Reputation DESC;
//
// `up.Id = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q1300(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), false);
    let rk = rel(per_group(v, |&(_, t)| t).into_iter().map(|((p, _), r)| (origid.get(p).unwrap(), (p, r))).collect());
    let by_id: HashIdx<i64, (i64, (Id<Post>, i64))> = (&rk).map(|(i, _)| i).inv().select((&rk).filt(|(_, (_, r))| r <= 5)).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let pcs = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.select((&db.user.origid).select((&by_id).map(|(_, x)| x)).and((&ub).opt())));
    let mut v: Vec<_> = v.into_iter().map(|(u, ((p, r), b))| (u, p, r, b, pcs.get(p))).collect();
    v.sort_by_key(|&(u, p, ..)| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap())));
    rows(v.into_iter().map(|(u, p, r, b, c)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.push(V::I(c.unwrap_or(0)));
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::I(0), V::Null, V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// Rewritten (rewrites/1615.sql): the ROW_NUMBER order tie-broken on p.Id and the final order on tu.UserId.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(DISTINCT p.Id) > 10),
// PostVoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(*) AS TotalVotes
//     FROM Votes v GROUP BY v.PostId)
// SELECT tu.DisplayName, tu.Reputation, rp.Title, rp.CreationDate, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes, rp.Score, rp.AnswerCount
// FROM TopUsers tu JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId LEFT JOIN PostVoteSummary pvs ON rp.PostId = pvs.PostId WHERE rp.rn = 1
// ORDER BY tu.Reputation DESC, rp.Score DESC, tu.UserId LIMIT 50;
fn q1615(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let Vote { post, vote_type_id, .. } = &db.vote;
    let pvs = db.vote.group_by(post).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&tp).select((&pvs).opt().and(owner_user.select(Ident::<User>::new().with(user_distinct_posts(db).filt(|n| n > 10))))));
    let v = top_n(v, |&(p, (_, u))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), db.user.origid.get(u).unwrap()), 50);
    rows(v.into_iter().map(|(p, (w, u))| {
        let w = w.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(w[0]), V::I(w[1])]);
        f.extend(post_fields(db, p, &["score", "answers"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')),
// CommentStats AS (SELECT PostId, COUNT(*) AS TotalComments, AVG(Score) AS AverageCommentScore FROM Comments GROUP BY PostId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, COALESCE(cs.TotalComments, 0) AS TotalComments,
//        COALESCE(cs.AverageCommentScore, 0) AS AverageCommentScore FROM RankedPosts rp LEFT JOIN CommentStats cs ON rp.PostId = cs.PostId WHERE rp.RankScore = 1)
// SELECT tp.*, CASE WHEN tp.Score > 100 THEN 'Hot' WHEN tp.Score > 50 THEN 'Trending' ELSE 'New' END AS PostStatus, CASE WHEN tp.Score IS NULL THEN 'No Score' ELSE 'Scored' END AS ScoreStatus
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.TotalComments DESC LIMIT 10;
fn q460(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let v = top_n(drain(&cs), |&(p, a)| (Reverse(score.get(p).unwrap()), Reverse(a[0]), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) }, V::S(if s > 100 { "Hot" } else if s > 50 { "Trending" } else { "New" }), V::S("Scored")]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount,
//        SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikiCount, SUM(CASE WHEN V.VoteTypeId IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes, AVG(COALESCE(P.Score, 0)) AS AveragePostScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.Reputation, U.DisplayName),
// TopUsers AS (SELECT UserId, Reputation, DisplayName, PostCount, QuestionCount, AnswerCount, WikiCount, TagWikiCount, TotalVotes, AveragePostScore,
//        RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.WikiCount, TU.TagWikiCount, TU.TotalVotes, TU.AveragePostScore,
//        RANK() OVER (ORDER BY TU.Reputation DESC) AS GlobalRank
// FROM TopUsers TU WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the posts x votes product is driven for those alone.
fn q8820(db: &'static So) -> String {
    let reputation = &db.user.reputation;
    let v = ranked(drain(db.user.with(reputation.gt(1000)).select(reputation)), |&(_, r)| Reverse(r), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let s = (&by_user)
        .map(|(u, _)| u)
        .inv()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score).and(votes_of(db).opt())).opt())
        .fold([0i64; 7], |a, x| match x {
            Some(((t, s), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + matches!(t, 4 | 5) as i64, a[5] + v.is_some() as i64, a[6] + s],
            None => [a[0] + 1, a[1], a[2], a[3], a[4], a[5], a[6]],
        });
    let mut v = drain((&s).and(user_distinct_posts(db)).and(&by_user));
    v.sort_by_key(|&(_, (_, (_, r)))| r);
    rows(v.into_iter().map(|(u, ((a, n), (_, r)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[5]), avg(a[6], a[0]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedQuestionCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT *, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats WHERE QuestionCount > 0),
// RecentVotes AS (SELECT v.UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v WHERE v.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY v.UserId)
// SELECT tu.DisplayName, tu.Reputation, tu.QuestionCount, tu.AnswerCount, tu.ClosedQuestionCount, rv.TotalVotes AS RecentTotalVotes, rv.UpVotes AS RecentUpVotes, rv.DownVotes AS RecentDownVotes
// FROM TopUsers tu LEFT JOIN RecentVotes rv ON tu.UserId = rv.UserId WHERE tu.Rank <= 10 ORDER BY tu.Reputation DESC;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so the vote's local timestamp is read in the session zone (New York) before the comparison.
fn q161(db: &'static So) -> String {
    let Post { post_type_id, closed_date, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(closed_date.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, c)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64],
        None => a,
    });
    let reputation = &db.user.reputation;
    let v = top_n(drain((&us).filt(|a| a[0] > 0)), |&(u, _)| Reverse(reputation.get(u).unwrap()), 10);
    let tu: MatSet<(Id<User>, [i64; 3])> = rel(v).map(|x| x).collect();
    let cut = now_utc() - 30 * DAY_US;
    let Vote { user, vote_type_id, creation_date, .. } = &db.vote;
    let rv = db.vote.with(creation_date.filt(move |d| ny_to_utc(d) >= cut)).group_by(user).select(vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let idx: HashIdx<Id<User>, (Id<User>, [i64; 3])> = (&tu).map(|(u, _)| u).inv().collect();
    rows(drain((&idx).map(|(u, _)| u).inv().select((&idx).map(|(_, a)| a).and((&rv).opt()))).into_iter().map(|(u, (a, r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match r {
            Some(r) => r.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, U.Reputation,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, u.DisplayName AS UserName, COALESCE(ub.GoldBadges, 0) AS GoldBadges,
//        COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
// FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// WHERE rp.rn = 1 AND rp.Score > (SELECT AVG(Score) FROM Posts WHERE PostTypeId = 1)
//   AND (rp.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' OR (rp.ViewCount IS NULL AND rp.AnswerCount = 0))
// ORDER BY rp.Score DESC, rp.ViewCount DESC;
fn q160(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, answer_count, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let (n, s) = qs().select(score).fold_flat((0i64, 0i64), |(n, t), s| (n + 1, t + s));
    let mean = s as f64 / n as f64;
    let v = drain(qs().with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let keep = Ident::<Post>::new()
        .with(score.filt(move |s| s as f64 > mean))
        .with(creation_date.ge(since).or(Ident::<Post>::new().minus(view_count).with(answer_count.eq(0))));
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let mut v = drain((&tp).select(keep).select(owner_user.select(Ident::<User>::new().and((&ub).opt()))));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (u, b))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers", "comments"]);
        f.push(user_col(db, u, "name"));
        f.extend(b.unwrap_or([0, 0, 0]).map(V::I));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, AVG(U.Reputation) AS AvgReputation FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(P.ViewCount) AS TotalViews, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY SUM(P.ViewCount) DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId)
// SELECT U.DisplayName, U.Reputation, U.LastAccessDate, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.Questions, 0) AS TotalQuestions,
//        COALESCE(PS.Answers, 0) AS TotalAnswers, COALESCE(PS.TotalViews, 0) AS TotalViews, CASE WHEN U.Location IS NULL THEN 'Location Not Specified' ELSE U.Location END AS UserLocation
// FROM Users U LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// WHERE U.Reputation > 100 AND (U.Location IS NOT NULL OR U.AboutMe IS NOT NULL) ORDER BY U.Reputation DESC, PS.TotalViews DESC LIMIT 10;
fn q4788(db: &'static So) -> String {
    let User { reputation, location, about_me, .. } = &db.user;
    let Post { owner_user, creation_date, post_type_id, view_count, .. } = &db.post;
    let ps = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 5], |a, (t, w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
    });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let users = db.user.with(reputation.gt(100)).with(Ident::<User>::new().with(location).or(Ident::<User>::new().with(about_me)));
    let v = drain(users.select((&bc).and((&ps).opt())));
    let v = top_n(v, |&(u, (_, p))| {
        let w = p.filter(|a| a[3] > 0).map(|a| a[4]);
        (Reverse(reputation.get(u).unwrap()), w.is_none(), Reverse(w), u)
    }, 10);
    rows(v.into_iter().map(|(u, (b, p))| {
        let a = p.unwrap_or([0; 5]);
        let mut f = ucols(db, u, &["name", "rep", "last_access"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[4]), V::S(location.get(u).unwrap_or("Location Not Specified"))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT t.DisplayName, t.Reputation, t.PostCount, t.BadgeCount, COALESCE(SUM(CASE WHEN ph.Comment IS NOT NULL AND ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS CloseCount,
//        COALESCE(SUM(CASE WHEN ph.Comment IS NOT NULL AND ph.PostHistoryTypeId IN (11, 12) THEN 1 ELSE 0 END), 0) AS ReopenOrDeleteCount,
//        CASE WHEN t.PostCount > 0 THEN ROUND(CAST(t.Reputation AS FLOAT) / t.PostCount, 2) ELSE 0 END AS ReputationPerPost
// FROM TopUsers t LEFT JOIN PostHistory ph ON t.UserId = ph.UserId WHERE t.ReputationRank <= 10 GROUP BY t.DisplayName, t.Reputation, t.PostCount, t.BadgeCount ORDER BY t.Reputation DESC;
//
// Only the distinct counts of UserStats are read, so the vote join (which multiplies rows but not distinct ids) is not driven. FLOAT is f32.
fn q285(db: &'static So) -> String {
    let reputation = &db.user.reputation;
    let v = ranked(drain(reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let by_user: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let PostHistory { comment, post_history_type_id, .. } = &db.post_history;
    let ph = (&tu).group_by(Ident::<User>::new()).select((&by_user).select(Ident::<PostHistory>::new().with(comment)).select(post_history_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(10)) as i64, a[1] + matches!(t, Some(11 | 12)) as i64]
    });
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&ph).and(user_distinct_posts(db)).and(&bc));
    v.sort_by_key(|&(u, _)| Reverse(reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(u, ((a, n), b))| {
        let rpp = if n > 0 {
            let x = reputation.get(u).unwrap() as f32 / n as f32;
            ((x * 100.0).round() / 100.0) as f64
        } else {
            0.0
        };
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(b), V::I(a[0]), V::I(a[1]), V::F(rpp)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// ClosureData AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenedDate
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.Rank, rp.CommentCount, rp.UpVotes, rp.DownVotes, cd.ClosedDate, cd.ReopenedDate,
//        COALESCE(cd.ClosedDate IS NOT NULL, FALSE) AS IsClosed
// FROM RankedPosts rp LEFT JOIN ClosureData cd ON rp.PostId = cd.PostId WHERE rp.Rank = 1 ORDER BY rp.Score DESC LIMIT 10;
//
// Rank reads only base columns, so each owner's best post is picked first and the comment x vote product is driven for the ten survivors alone.
fn q1781(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let top = top_n(top, |&(p, _)| Reverse(score.get(p).unwrap()), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cd = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold([i64::MIN; 2], |a, (t, d)| [if t == 10 { a[0].max(d) } else { a[0] }, if t == 11 { a[1].max(d) } else { a[1] }]);
    rows(drain((&s).and((&cd).opt())).into_iter().map(|(p, (a, c))| {
        let c = c.unwrap_or([i64::MIN; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(V::I(1));
        f.extend(a.map(V::I));
        f.extend([tmax(c[0]), tmax(c[1]), V::B(c[0] != i64::MIN)]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN V.VoteTypeId = 6 THEN 1 ELSE 0 END) AS CloseVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostViewStats AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, COALESCE(UP.TotalVotes, 0) AS UserTotalVotes, ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY P.LastActivityDate DESC) AS Rnk
//     FROM Posts P LEFT JOIN UserVoteSummary UP ON P.OwnerUserId = UP.UserId WHERE P.ViewCount > 100),
// ClosedPostDetails AS (SELECT PH.PostId, PH.CreationDate, P.Title, P.Body FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id
//     WHERE PH.PostHistoryTypeId = 10 AND PH.CreationDate >= cast('2024-10-01' as date) - INTERVAL '6 months')
// SELECT PVS.PostId, PVS.Title, PVS.ViewCount, PVS.UserTotalVotes, CPD.CreationDate AS ClosureDate, CPD.Body AS ClosedPostBody
// FROM PostViewStats PVS LEFT JOIN ClosedPostDetails CPD ON PVS.PostId = CPD.PostId WHERE PVS.Rnk = 1 AND (PVS.UserTotalVotes > 10 OR CPD.PostId IS NOT NULL)
// ORDER BY PVS.ViewCount DESC, PVS.UserTotalVotes DESC;
//
// Rnk partitions by the post itself, so it is always 1.
fn q1168(db: &'static So) -> String {
    let Post { view_count, owner_user, .. } = &db.post;
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10).and(hd.ge(add_months(date(2024, 10, 1), -6)))));
    let j = db.post.with(view_count.gt(100)).select(owner_user.select(&uv).opt().and(closes.opt())).filt(|(n, h): (Option<i64>, Option<Id<PostHistory>>)| n.unwrap_or(0) > 10 || h.is_some());
    let mut v = drain(j);
    v.sort_by_key(|&(p, (n, _))| (Reverse(view_count.get(p)), Reverse(n.unwrap_or(0))));
    rows(v.into_iter().map(|(p, (n, h))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.push(V::I(n.unwrap_or(0)));
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), V::S(db.post.body.get(p).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, TotalViews, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewsRank FROM UserReputation),
// CombinedRanks AS (SELECT UserId, Reputation, TotalViews, BadgeCount, COALESCE(ReputationRank, 999) AS ReputationRank, COALESCE(ViewsRank, 999) AS ViewsRank FROM TopUsers)
// SELECT U.DisplayName, R.Reputation, R.TotalViews, R.BadgeCount, (CASE WHEN R.ReputationRank = 1 THEN 'Top Reputation' WHEN R.ViewsRank = 1 THEN 'Top Views' ELSE 'Regular User' END) AS UserCategory,
//        COALESCE((SELECT COUNT(*) FROM Posts PO WHERE PO.OwnerUserId = U.Id AND PO.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'), 0) AS RecentPostsCount
// FROM Users U JOIN CombinedRanks R ON U.Id = R.UserId WHERE U.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' ORDER BY R.Reputation DESC, R.TotalViews DESC;
fn q3328(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { view_count, creation_date, .. } = &db.post;
    let ur = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt()).opt().and(badges_of(db).opt())).fold(0i64, |n, (w, _)| n + w.flatten().unwrap_or(0));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let v = ranked(v.into_iter().map(|((u, _), r)| (u, r, ur.get(u).unwrap())).collect(), |&(_, _, w)| Reverse(w), false);
    let rk = rel(v.into_iter().map(|((u, r, w), vr)| (u, (r, vr, w))).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, (i64, i64, i64))> = (&rk).map(|(u, _)| u).inv().select(&rk).collect();
    let recent = Ident::<Post>::new().with(creation_date.gt(add_years(t0, -1)));
    let rc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(recent).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain(db.user.with((&db.user.last_access_date).gt(add_days(t0, -30))).select((&by_user).map(|(_, x)| x).and(&bc).and(&rc))).into_iter().map(|(u, (((r, vr, w), b), n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(w), V::I(b), V::S(if r == 1 { "Top Reputation" } else if vr == 1 { "Top Views" } else { "Regular User" }), V::I(n)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN HM.PostId IS NOT NULL THEN 1 ELSE 0 END), 0) AS FavoriteCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN (SELECT PostId FROM Votes WHERE VoteTypeId = 5 GROUP BY PostId) HM ON P.Id = HM.PostId
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// HighPerformingUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, FavoriteCount,
//        RANK() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserStatistics)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.PostCount, U.QuestionCount, U.AnswerCount, U.UpVotes, U.DownVotes, U.FavoriteCount, HP.Rank
// FROM UserStatistics U JOIN HighPerformingUsers HP ON U.UserId = HP.UserId WHERE HP.Rank <= 10 ORDER BY HP.Rank;
//
// Rank leads with Reputation, so only users at or above the tenth-highest reputation can rank in the top ten; the posts x votes product is driven for those alone.
fn q5097(db: &'static So) -> String {
    let reputation = &db.user.reputation;
    let tenth = top_n(drain(reputation), |&(_, r)| Reverse(r), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with(reputation.ge(tenth)).collect();
    let fav: MatSet<Id<Post>> = db.vote.with((&db.vote.vote_type_id).eq(5)).select(&db.vote.post).collect();
    let s = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(Ident::<Post>::new().with(&fav).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((t, v), h)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + h.is_some() as i64],
            None => a,
        });
    let v = ranked(drain((&s).and(user_distinct_posts(db))), |&(u, (_, n))| (Reverse(reputation.get(u).unwrap()), Reverse(n)), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), r)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RowNum FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// VoteCounts AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Score, COALESCE(vc.UpVotes, 0) AS UpVotes, COALESCE(vc.DownVotes, 0) AS DownVotes, rp.ViewCount, rp.AnswerCount, rp.RankScore
//     FROM RankedPosts rp LEFT JOIN VoteCounts vc ON rp.PostId = vc.PostId WHERE rp.RankScore <= 5)
// SELECT fp.Title, fp.Score, fp.UpVotes, fp.DownVotes, CASE WHEN fp.Score IS NOT NULL THEN 'Scored' ELSE 'Unscored' END AS ScoringStatus,
//        CONCAT('https://stackoverflow.com/questions/', fp.PostId) AS PostLink
// FROM FilteredPosts fp WHERE fp.ViewCount > 1000 ORDER BY fp.Score DESC, fp.ViewCount DESC LIMIT 10;
fn q1371(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let Vote { post, vote_type_id, .. } = &db.vote;
    let vc = db.vote.group_by(post).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = top_n(drain((&tp).with(view_count.gt(1000)).select((&vc).opt())), |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S("Scored"), V::Owned(format!("https://stackoverflow.com/questions/{}", origid.get(p).unwrap()))]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// UserRanked AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, CommentCount, UpVotes, DownVotes, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserActivity)
// SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, CommentCount, UpVotes, DownVotes, BadgeCount, ReputationRank FROM UserRanked WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the posts x comments x votes x badges product is driven for those alone.
fn q7021(db: &'static So) -> String {
    let reputation = &db.user.reputation;
    let v = ranked(drain(reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let s = (&by_user)
        .map(|(u, _)| u)
        .inv()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, _)| match p {
            Some(((t, c), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let bc = (&by_user).map(|(u, _)| u).inv().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&s).and(&bc).and(&by_user));
    v.sort_by_key(|&(u, _)| Reverse(reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(u, ((a, b), (_, r)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON U.Id = B.UserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Upvotes, Downvotes, PostCount, CommentCount, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT U.UserId, U.DisplayName, U.Reputation, COALESCE(U.Upvotes - U.Downvotes, 0) AS NetVotes, U.PostCount, U.CommentCount, U.BadgeCount,
//        CASE WHEN U.Rank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorStatus
// FROM TopUsers U WHERE U.Reputation > (SELECT AVG(Reputation) FROM Users) OR EXISTS (SELECT 1 FROM Posts P WHERE P.OwnerUserId = U.UserId AND P.AcceptedAnswerId IS NOT NULL)
// ORDER BY U.Reputation DESC LIMIT 100 OFFSET 0;
//
// The filter and the order read only base columns, so the hundred users are picked first and the posts x votes x comments x badges product is driven for those alone.
fn q2602(db: &'static So) -> String {
    let reputation = &db.user.reputation;
    let (n, s) = db.user.select(reputation).fold_flat((0i64, 0i64), |(n, t), r| (n + 1, t + r));
    let mean = s as f64 / n as f64;
    let accepted: MatSet<Id<User>> = db.post.with(&db.post.accepted_answer).select(&db.post.owner_user).collect();
    let rk = ranked(drain(reputation), |&(_, r)| Reverse(r), false);
    let rk = rel(rk.into_iter().map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&rk).map(|(u, _)| u).inv().select(&rk).collect();
    let keep = db.user.with(reputation.filt(move |r| r as f64 > mean).or(Ident::<User>::new().with(&accepted)));
    let v = top_n(drain(keep.select(&by_user)), |&(u, _)| (Reverse(reputation.get(u).unwrap()), u), 100);
    let tu: MatSet<(Id<User>, (Id<User>, i64))> = rel(v).map(|x| x).collect();
    let idx: HashIdx<Id<User>, (Id<User>, (Id<User>, i64))> = (&tu).map(|(u, _)| u).inv().collect();
    let users = || (&idx).map(|(u, _)| u).inv();
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold(0i64, |n, (p, _)| {
            let t = p.and_then(|(t, _)| t);
            n + (t == Some(2)) as i64 - (t == Some(3)) as i64
        });
    let cc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = users().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&s).and(user_distinct_posts(db)).and(&cc).and(&bc).and(&by_user));
    v.sort_by_key(|&(u, _)| (Reverse(reputation.get(u).unwrap()), u));
    rows(v.into_iter().map(|(u, ((((nv, n), c), b), (_, r)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(nv), V::I(n), V::I(c), V::I(b), V::S(if r <= 10 { "Top Contributor" } else { "Regular Contributor" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT a.Id) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.LastAccessDate >= CURRENT_TIMESTAMP - INTERVAL '30 days')
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.AnswerCount, rp.UpVotes, rp.DownVotes, au.DisplayName AS ActiveUserDisplayName, au.Reputation, au.ReputationRank
// FROM RankedPosts rp LEFT JOIN ActiveUsers au ON rp.OwnerUserId = au.UserId WHERE rp.PostRank <= 100 ORDER BY rp.CreationDate DESC, rp.UpVotes DESC;
//
// PostRank reads only CreationDate, so the hundred newest questions are picked first and the answers x votes product is driven for those alone.
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so LastAccessDate is read in the session zone (New York).
fn q7553(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(p, d)| (Reverse(d), p), 100);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let cut = now_utc() - 30 * DAY_US;
    let User { last_access_date, reputation, .. } = &db.user;
    let au = ranked(drain(db.user.with(last_access_date.filt(move |d| ny_to_utc(d) >= cut)).select(reputation)), |&(_, r)| Reverse(r), false);
    let au = rel(au.into_iter().map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&au).map(|(u, _)| u).inv().select(&au).collect();
    let mut v = drain((&s).and(&ac).and(owner_user.select(&by_user).opt()));
    v.sort_by_key(|&(p, ((a, _), _))| (Reverse(creation_date.get(p).unwrap()), Reverse(a[0])));
    rows(v.into_iter().map(|(p, ((a, n), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(match u {
            Some((u, r)) => [user_col(db, u, "name"), user_col(db, u, "rep"), V::I(r)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH FilteredPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName AS Author, pt.Name AS PostType, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(b.Id) AS BadgeCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Title IS NOT NULL GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName, pt.Name
//     HAVING COUNT(c.Id) > 5 AND COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) > 10),
// RankedPosts AS (SELECT *, RANK() OVER (ORDER BY UpVotes DESC, CommentCount DESC) AS Rank FROM FilteredPosts)
// SELECT PostId, Title, Body, Tags, CreationDate, Author, PostType, CommentCount, UpVotes, DownVotes, BadgeCount, Rank FROM RankedPosts WHERE Rank <= 10 ORDER BY Rank;
fn q26324(db: &'static So) -> String {
    let Post { creation_date, title, owner_user, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(title)
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 4], |a, ((c, t), b)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + b.is_some() as i64]);
    let v = ranked(drain((&s).filt(|a| a[0] > 5 && a[1] > 10)), |&(_, a)| (Reverse(a[1]), Reverse(a[0])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, a), r)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "owner", "type"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        AVG(EXTRACT(EPOCH FROM (COALESCE(NULLIF(p.LastActivityDate, '1970-01-01'), '2024-10-01 12:34:56') - p.CreationDate))) AS AverageActiveDuration
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.CreationDate > (CURRENT_TIMESTAMP - INTERVAL '1 year') AND p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.PostTypeId),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.PostTypeId, ps.CommentCount, ps.VoteCount, ps.UpVotes, ps.DownVotes, ps.AverageActiveDuration,
//        RANK() OVER (PARTITION BY ps.PostTypeId ORDER BY ps.VoteCount DESC, ps.CommentCount DESC) AS Rank FROM PostStats ps)
// SELECT t.Title, CASE WHEN t.PostTypeId = 1 THEN 'Question' WHEN t.PostTypeId = 2 THEN 'Answer' END AS PostType, t.CommentCount, t.VoteCount, t.UpVotes, t.DownVotes, t.AverageActiveDuration
// FROM TopPosts t WHERE t.Rank <= 10 ORDER BY t.PostTypeId, t.Rank;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so CreationDate is read in the session zone (New York). LastActivityDate is never the epoch here, so the COALESCE is the column.
fn q9842(db: &'static So) -> String {
    let Post { post_type_id, creation_date, last_activity_date, .. } = &db.post;
    let cut = add_years(now_utc(), -1);
    let ps = || db.post.with(post_type_id.is_in([1, 2])).with(creation_date.filt(move |d| ny_to_utc(d) > cut));
    let s = ps().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let vu = ps().group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.user).opt()).opt()).buf_fold(|v| distinct_some(v.iter().map(|x| x.flatten())));
    let dur = |p: Id<Post>| secs(last_activity_date.get(p).unwrap() - creation_date.get(p).unwrap());
    let v = ranked(drain((&s).and(&vu).and(post_type_id)), |&(_, ((a, n), t))| (t, Reverse(n), Reverse(a[0])), false);
    let v = per_group(v, |&(_, (_, t))| t);
    rows(v.into_iter().filter(|x| x.1 <= 10).map(|((p, ((a, n), t)), _)| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::S(if t == 1 { "Question" } else { "Answer" }), V::I(a[0]), V::I(n), V::I(a[1]), V::I(a[2]), V::F(dur(p))]);
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COALESCE(COUNT(DISTINCT P.Id), 0) AS PostCount,
//        COALESCE(SUM(CASE WHEN BH.UserId IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges BH ON U.Id = BH.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// UserPerformance AS (SELECT UserId, DisplayName, Reputation, Upvotes - Downvotes AS VoteBalance, PostCount, BadgeCount,
//        RANK() OVER (ORDER BY (Upvotes - Downvotes) DESC, PostCount DESC, Reputation DESC) AS UserRank FROM UserEngagement)
// SELECT U.UserRank, U.DisplayName, U.Reputation, U.VoteBalance, U.PostCount, U.BadgeCount,
//        CASE WHEN U.VoteBalance > 100 THEN 'Outstanding' WHEN U.VoteBalance BETWEEN 50 AND 100 THEN 'Good' WHEN U.VoteBalance BETWEEN 0 AND 49 THEN 'Average' ELSE 'Needs Improvement' END AS PerformanceCategory
// FROM UserPerformance U WHERE U.PostCount > 0 ORDER BY U.UserRank FETCH FIRST 10 ROWS ONLY;
fn q6727(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, b)| {
            let t = p.flatten();
            [a[0] + (t == Some(2)) as i64 - (t == Some(3)) as i64, a[1] + b.is_some() as i64]
        });
    let reputation = &db.user.reputation;
    let v = ranked(drain((&s).and(user_distinct_posts(db))), |&(u, (a, n))| (Reverse(a[0]), Reverse(n), Reverse(reputation.get(u).unwrap())), false);
    let v = top_n(drain(rel(v).filt(|((_, (_, n)), _)| n > 0)).into_iter().map(|x| x.1).collect(), |x| x.1, 10);
    rows(v.into_iter().map(|((u, (a, n)), r)| {
        let b = a[0];
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(n), V::I(a[1]), V::S(if b > 100 { "Outstanding" } else if b >= 50 { "Good" } else if b >= 0 { "Average" } else { "Needs Improvement" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty,
//        SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS UpVotes, SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS DownVotes,
//        ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT P.Id) DESC) AS ActivityRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE U.Reputation > 1000 AND U.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id, P.Title, P.CreationDate, P.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT UA.UserId, UA.DisplayName, UA.PostCount, UA.TotalBounty, UA.UpVotes, UA.DownVotes, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate, UA.ActivityRank
// FROM UserActivity UA LEFT JOIN RecentPosts RP ON UA.UserId = RP.OwnerUserId AND RP.PostRank = 1 WHERE UA.ActivityRank <= 10 ORDER BY UA.ActivityRank;
//
// ActivityRank reads only the distinct post count, so the top users are picked first and the posts x votes product is driven for those alone.
fn q3148(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let User { reputation, creation_date, .. } = &db.user;
    let v = top_n(drain(db.user.with(reputation.gt(1000).and(creation_date.lt(add_years(t0, -1)))).select(user_distinct_posts(db))), |&(u, n)| (Reverse(n), u), 10);
    let tu = rel(v.into_iter().enumerate().map(|(i, (u, n))| (u, (n, i as i64 + 1))).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let users = || (&by_user).map(|(u, _)| u).inv();
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).opt())
        .fold([0i64; 3], |a, v| match v.flatten() {
            Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
            None => a,
        });
    let Post { owner_user, creation_date: pd, .. } = &db.post;
    let rp = drain(db.post.with(pd.ge(add_days(t0, -30))).with(owner_user).select(owner_user));
    let rp = top_per(rp, |&(_, u)| u, |&(p, _)| (Reverse(pd.get(p).unwrap()), p), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let recent: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let mut v = drain((&s).and(&by_user).and((&recent).map(|(_, p)| p).opt()));
    v.sort_by_key(|&(_, ((_, (_, (_, r))), _))| r);
    rows(v.into_iter().map(|(u, ((a, (_, (n, r))), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserScore AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 WHEN V.VoteTypeId = 3 THEN -1 ELSE 0 END), 0) AS VoteScore,
//        COUNT(DISTINCT B.Id) AS BadgeCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year' AND P.Score > 0),
// PostDetails AS (SELECT U.UserId, T.PostId, U.VoteScore, U.BadgeCount, P.Title, P.CreationDate, P.Score, P.ViewCount FROM TopPosts T JOIN UserScore U ON T.OwnerUserId = U.UserId
//     JOIN Posts P ON T.PostId = P.Id WHERE T.PostRank <= 10)
// SELECT U.DisplayName, U.Reputation, P.Title AS PostTitle, P.CreationDate, P.Score, P.ViewCount, P.VoteScore, P.BadgeCount, (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.PostId) AS CommentCount
// FROM PostDetails P JOIN Users U ON P.UserId = U.Id ORDER BY P.Score DESC, P.CreationDate DESC LIMIT 50;
//
// PostRank reads only base columns, so the top posts are picked first and the votes x badges product is driven for their owners alone.
fn q5726(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt())).fold(0i64, |n, (t, _)| {
        n + (t == Some(2)) as i64 - (t == Some(3)) as i64
    });
    let bc = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&us).and(&bc))));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (c, ((u, s), b)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend([V::I(s), V::I(b), V::I(c)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(COALESCE(p.Score, 0)) AS AverageScore, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AverageScore, TotalComments, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank,
//        RANK() OVER (ORDER BY AverageScore DESC) AS ScoreRank FROM UserPostStats),
// FilteredTopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AverageScore, TotalComments, PostRank, ScoreRank FROM TopUsers WHERE PostRank <= 10 OR ScoreRank <= 10)
// SELECT f.DisplayName, COALESCE(b.Class, 0) AS BadgeClass, b.Name AS BadgeName, f.TotalPosts, f.TotalQuestions, f.TotalAnswers, f.AverageScore, f.TotalComments
// FROM FilteredTopUsers f LEFT JOIN Badges b ON f.UserId = b.UserId WHERE b.Class IS NOT NULL ORDER BY f.TotalPosts DESC, f.AverageScore DESC;
fn q2282(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score).and(comments_of(db).opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, s), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + c.is_some() as i64],
        None => [a[0], a[1], a[2], a[3], a[4]],
    });
    let rows_of = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db).opt()).opt()).fold(0i64, |n, _| n + 1);
    let avgs = |a: [i64; 5], k: i64| a[3] as f64 / k as f64;
    let v = ranked(drain((&s).and(&rows_of)), |&(_, (a, _))| Reverse(a[0]), false);
    let v = ranked(v, |&((_, (a, k)), _)| Reverse(fkey(avgs(a, k))), false);
    let keep = rel(drain(rel(v).filt(|(((_, _), p), q)| p <= 10 || q <= 10)).into_iter().map(|x| ((x.1).0).0).collect());
    let idx: HashIdx<Id<User>, (Id<User>, ([i64; 5], i64))> = (&keep).map(|(u, _)| u).inv().select(&keep).collect();
    let mut v = drain((&idx).map(|(u, _)| u).inv().select((&idx).map(|(_, x)| x).and(badges_of(db))));
    v.sort_by_key(|&(_, ((a, k), _))| (Reverse(a[0]), Reverse(fkey(avgs(a, k)))));
    rows(v.into_iter().map(|(u, ((a, k), b))| {
        row(vec![user_col(db, u, "name"), V::I(db.badge.class.get(b).unwrap()), V::S(db.badge.name.get(b).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(avgs(a, k)), V::I(a[4])])
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        SUM(COALESCE(c.Score, 0)) AS TotalCommentScore, AVG(LENGTH(SUBSTRING(p.Body FROM 1 FOR 300))) AS AverageTitleLength
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE u.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, Upvotes, Downvotes, TotalCommentScore, RANK() OVER (ORDER BY TotalPosts DESC) AS UserRank
//     FROM UserActivity WHERE TotalPosts > 0)
// SELECT tu.UserId, tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.Upvotes, tu.Downvotes, tu.TotalCommentScore, tu.UserRank FROM TopUsers tu WHERE tu.UserRank <= 10 ORDER BY tu.UserRank;
//
// UserRank reads only the distinct post count, so the top users are picked first and the posts x votes x comments product is driven for those alone.
fn q7235(db: &'static So) -> String {
    let users = db.user.with((&db.user.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let v = ranked(drain(users.select(user_distinct_posts(db).filt(|n| n > 0))), |&(_, n)| Reverse(n), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, n), r)| (u, (n, r))).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let s = (&by_user)
        .map(|(u, _)| u)
        .inv()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).select(&db.comment.score).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((t, v), c)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + c.unwrap_or(0)],
            None => a,
        });
    let mut v = drain((&s).and(&by_user));
    v.sort_by_key(|&(_, (_, (_, (_, r))))| r);
    rows(v.into_iter().map(|(u, (a, (_, (n, r))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserScoreStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(DISTINCT P.Id) AS PostCount, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// ClosedPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, PH.CreationDate AS CloseDate, PH.Comment AS CloseReason FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId
//     WHERE PH.PostHistoryTypeId = 10 AND P.PostTypeId = 1),
// RankedClosedPosts AS (SELECT CP.*, COUNT(*) OVER (PARTITION BY CP.OwnerUserId) AS ClosedPostCount FROM ClosedPosts CP)
// SELECT U.DisplayName, U.Reputation, U.PostCount, U.Upvotes, U.Downvotes, RC.CloseDate, RC.CloseReason, RC.ClosedPostCount
// FROM UserScoreStats U LEFT JOIN RankedClosedPosts RC ON U.UserId = RC.OwnerUserId
// WHERE (U.Reputation > 100 OR U.Upvotes > 10) AND (RC.ClosedPostCount IS NULL OR RC.ClosedPostCount < 3) ORDER BY U.Reputation DESC, U.Upvotes DESC;
fn q39(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let PostHistory { post, post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let Post { post_type_id, owner_user, .. } = &db.post;
    let cp = || db.post_history.with(post_history_type_id.eq(10)).with(post.select(Ident::<Post>::new().with(post_type_id.eq(1))));
    let by_owner: HashIdx<Id<User>, Id<PostHistory>> = cp().select(post.select(owner_user)).inv().collect();
    let cnt = cp().group_by(post.select(owner_user)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let reputation = &db.user.reputation;
    let keep = Ident::<User>::new().with(reputation.gt(100)).or(Ident::<User>::new().with((&s).filt(|a| a[0] > 10)));
    let j = db.user.with(keep).select((&s).and(user_distinct_posts(db)).and((&by_owner).and(&cnt).opt()).filt(|(_, h): (([i64; 2], i64), Option<(Id<PostHistory>, i64)>)| h.map_or(true, |(_, n)| n < 3)));
    let mut v = drain(j);
    v.sort_by_key(|&(u, ((a, _), _))| (Reverse(reputation.get(u).unwrap()), Reverse(a[0])));
    rows(v.into_iter().map(|(u, ((a, n), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some((h, c)) => [V::T(hd.get(h).unwrap()), harness::fmt::ostr(comment.get(h)), V::I(c)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// Rewritten (rewrites/34294.sql): the ROW_NUMBER order tie-broken on p.Id and the final order on rp.PostId.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS Rank,
//        COALESCE(SUM(v.BountyAmount) FILTER (WHERE v.VoteTypeId = 8) OVER (PARTITION BY p.Id), 0) AS TotalBounty, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostHistoryWithComments AS (SELECT ph.PostId, COUNT(DISTINCT ph.UserId) AS EditorsCount, COUNT(DISTINCT c.Id) AS CommentsCount, MAX(ph.CreationDate) AS LastEditDate
//     FROM PostHistory ph LEFT JOIN Comments c ON ph.PostId = c.PostId GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.Score, rp.TotalBounty, ph.CommentsCount, ph.EditorsCount, ph.LastEditDate,
//        CASE WHEN rp.CommentCount > 5 THEN 'High Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM RankedPosts rp JOIN PostHistoryWithComments ph ON rp.PostId = ph.PostId WHERE rp.Rank = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC, rp.PostId LIMIT 100;
//
// Rank numbers the post x vote x comment rows, but all rows of one post tie on the order, so the first row of each owner belongs to its newest post (smallest id on a tie),
// and the windowed sums are the same on every row of that post. The product is driven for those posts alone.
fn q34294(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt()));
    let s = (&tp).with(history_of(db)).group_by(Ident::<Post>::new()).select(bounty.opt().and(comments_of(db).opt())).fold([0i64; 2], |a, (b, c)| {
        let b = b.and_then(|(t, b)| if t == 8 { b } else { None });
        [a[0] + b.unwrap_or(0), a[1] + c.is_some() as i64]
    });
    let PostHistory { user, creation_date: hd, .. } = &db.post_history;
    let ed = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(user.opt())).buf_fold(|v| distinct_some(v.iter().copied()));
    let lh = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(hd)).fold(i64::MIN, |m, d| m.max(d));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&s).and(&ed).and(&lh).and(&cc)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 100);
    rows(v.into_iter().map(|(p, (((a, e), d), c))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "score"]);
        f.extend([V::I(a[0]), V::I(c), V::I(e), V::T(d), V::S(if a[1] > 5 { "High Engagement" } else { "Low Engagement" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, DENSE_RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// UserVotes AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 WHEN vt.Name = 'DownMod' THEN -1 ELSE 0 END) AS VoteScore FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11))
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, COALESCE(uv.VoteScore, 0) AS TotalVotes, CASE WHEN cp.PostId IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM RankedPosts rp LEFT JOIN UserVotes uv ON rp.PostId = uv.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.ScoreRank <= 10 ORDER BY rp.Score DESC, rp.CreationDate DESC;
fn q1905(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(score)), |&(p, s)| {
        (Reverse(s), Reverse(creation_date.get(p).unwrap()))
    }, true);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uv = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold(0i64, |n, t| n + (t == "UpMod") as i64 - (t == "DownMod") as i64);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let mut v = drain((&cc).and((&uv).opt()).and(closes.opt()));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((c, u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(u.unwrap_or(0)), V::S(if h.is_some() { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPosts AS (SELECT p.OwnerUserId, p.Title, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName ORDER BY PostCount DESC LIMIT 10)
// SELECT ur.DisplayName, ur.Reputation, ur.Upvotes, ur.Downvotes, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate, tt.TagName AS PopularTag
// FROM UserReputation ur LEFT JOIN RecentPosts rp ON ur.UserId = rp.OwnerUserId AND rp.rn = 1 LEFT JOIN TopTags tt ON tt.PostCount > 0
// WHERE ur.Reputation > 1000 AND ur.Reputation < 10000 OR (ur.Upvotes IS NOT NULL AND ur.Upvotes > 50) ORDER BY ur.Reputation DESC LIMIT 50;
//
// The TopTags ON clause names only tt, so the users are crossed with the ten tags.
fn q3741(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Vote { user, vote_type_id, .. } = &db.vote;
    let ur = db.vote.group_by(user).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let reputation = &db.user.reputation;
    let keep = Ident::<User>::new().with(reputation.gt(1000).and(reputation.lt(10000))).or(Ident::<User>::new().with((&ur).filt(|a| a[0] > 50)));
    let Post { owner_user, creation_date, .. } = &db.post;
    let rp = drain(db.post.with(creation_date.ge(add_days(t0, -30))).with(owner_user).select(owner_user));
    let rp = top_per(rp, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let recent: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let users = drain(db.user.with(keep).select((&ur).opt().and((&recent).map(|(_, p)| p).opt())));
    let tt = top_n(drain((&tag_stats(db)).filt(|a| a[0] > 0)), |&(t, a)| (Reverse(a[0]), t), 10);
    let v = cross_top(users, |&(u, _)| (Reverse(reputation.get(u).unwrap()), u), tt, |&(t, _)| t, 50);
    rows(v.into_iter().map(|((u, (a, p)), (t, _))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        f.push(V::S(db.tag.tag_name.get(t).unwrap()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// HighestScoredPost AS (SELECT rp.OwnerDisplayName, MAX(rp.Score) AS MaxScore FROM RankedPosts rp WHERE rp.PostRank = 1 GROUP BY rp.OwnerDisplayName),
// CommentStats AS (SELECT p.OwnerUserId, COUNT(c.Id) AS TotalComments, SUM(CASE WHEN c.Score IS NULL THEN 0 ELSE c.Score END) AS TotalCommentScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 2 GROUP BY p.OwnerUserId)
// SELECT u.DisplayName, COALESCE(cs.TotalComments, 0) AS TotalComments, COALESCE(cs.TotalCommentScore, 0) AS TotalCommentScore, hs.MaxScore
// FROM Users u LEFT JOIN CommentStats cs ON u.Id = cs.OwnerUserId LEFT JOIN HighestScoredPost hs ON u.DisplayName = hs.OwnerDisplayName
// WHERE u.Reputation > 1000 AND (hs.MaxScore IS NOT NULL OR cs.TotalComments > 0) ORDER BY u.Reputation DESC;
fn q752(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let hs = (&tp).group_by(owner_user.select(&db.user.display_name)).select(score).fold(i64::MIN, |m, s| m.max(s));
    let cs = db.post.with(post_type_id.eq(2)).group_by(owner_user).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, c| match c {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let j = db.user.with((&db.user.reputation).gt(1000)).select((&cs).opt().and((&db.user.display_name).select(&hs).opt())).filt(|(c, h): (Option<[i64; 2]>, Option<i64>)| {
        h.is_some() || c.map_or(false, |c| c[0] > 0)
    });
    let mut v = drain(j);
    v.sort_by_key(|&(u, _)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(u, (c, h))| {
        let c = c.unwrap_or([0, 0]);
        row(vec![user_col(db, u, "name"), V::I(c[0]), V::I(c[1]), harness::fmt::oint(h)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.PostTypeId = 1),
// PostVoteCounts AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostHistoryDetails AS (SELECT ph.PostId, MIN(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenedDate
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT p.Id, p.Title, COALESCE(pc.UpVotes, 0) AS UpVoteCount, COALESCE(pc.DownVotes, 0) AS DownVoteCount,
//        CASE WHEN pd.ClosedDate IS NOT NULL AND (pd.ReopenedDate IS NULL OR pd.ClosedDate > pd.ReopenedDate) THEN 'Closed' ELSE 'Open' END AS Status, p.Score, p.CreationDate, p.Tags
// FROM RankedPosts p LEFT JOIN PostVoteCounts pc ON p.Id = pc.PostId LEFT JOIN PostHistoryDetails pd ON p.Id = pd.PostId
// WHERE p.rn = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' ORDER BY p.Score DESC, p.CreationDate DESC;
fn q804(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let Vote { post, vote_type_id, .. } = &db.vote;
    let pc = db.vote.group_by(post).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post: hp, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pd = db.post_history.group_by(hp).select(post_history_type_id.and(hd)).fold([i64::MAX, i64::MIN], |a, (t, d)| [if t == 10 { a[0].min(d) } else { a[0] }, if t == 11 { a[1].max(d) } else { a[1] }]);
    let mut v = drain((&tp).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select((&pc).opt().and((&pd).opt())));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (c, d))| {
        let c = c.unwrap_or([0, 0]);
        let d = d.unwrap_or([i64::MAX, i64::MIN]);
        let closed = d[0] != i64::MAX && (d[1] == i64::MIN || d[0] > d[1]);
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(c[0]), V::I(c[1]), V::S(if closed { "Closed" } else { "Open" })]);
        f.extend(post_fields(db, p, &["score", "created", "tags"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS PostRank
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score IS NOT NULL),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.PostRank <= 5),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName)
// SELECT ta.PostId, ta.Title, ta.OwnerDisplayName, ta.CreationDate, ta.Score, ta.ViewCount, ua.DisplayName AS UserName, ua.PostsCreated, ua.TotalBountyAmount
// FROM TopPosts ta JOIN UserActivity ua ON ta.OwnerDisplayName = ua.DisplayName ORDER BY ta.Score DESC, ta.CreationDate ASC;
fn q6942(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold(0i64, |n, b| n + b.flatten().flatten().unwrap_or(0));
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let mut v = drain((&tp).select(owner_user.select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and(&ua).and(user_distinct_posts(db)))));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, ((u, b), n))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([user_col(db, u, "name"), V::I(n), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, DENSE_RANK() OVER (ORDER BY p.Score DESC) AS RankScore,
//        DENSE_RANK() OVER (ORDER BY p.ViewCount DESC) AS RankView
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName),
// TopScoringPosts AS (SELECT PostId, Title, Score, ViewCount, OwnerDisplayName, CommentCount FROM RankedPosts WHERE RankScore <= 10),
// TopViewedPosts AS (SELECT PostId, Title, Score, ViewCount, OwnerDisplayName, CommentCount FROM RankedPosts WHERE RankView <= 10)
// SELECT ts.PostId, ts.Title, ts.Score, ts.ViewCount, ts.OwnerDisplayName, ts.CommentCount, 'Top Scoring' AS PostCategory FROM TopScoringPosts ts
// UNION ALL SELECT tv.PostId, tv.Title, tv.Score, tv.ViewCount, tv.OwnerDisplayName, tv.CommentCount, 'Top Viewed' AS PostCategory FROM TopViewedPosts tv ORDER BY PostCategory, Score DESC;
fn q6419(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let base = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(score));
    let rs = ranked(base.clone(), |&(_, s)| Reverse(s), true);
    let rv = ranked(base, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, true);
    let ts: MatSet<Id<Post>> = rel(rs.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let tv: MatSet<Id<Post>> = rel(rv.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = |s: &MatSet<Id<Post>>| drain(s.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64));
    let mut v: Vec<(&'static str, Id<Post>, i64)> = cc(&ts).into_iter().map(|(p, c)| ("Top Scoring", p, c)).collect();
    v.extend(cc(&tv).into_iter().map(|(p, c)| ("Top Viewed", p, c)));
    v.sort_by_key(|&(k, p, _)| (k, Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(k, p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::I(c), V::S(k)]);
        row(f)
    }))
}

// WITH RecursiveTagSplits AS (SELECT Id AS PostId, UNNEST(string_to_array(SUBSTRING(Tags FROM 2 FOR LENGTH(Tags) - 2), '><')) AS Tag FROM Posts WHERE Tags IS NOT NULL),
// TagCounts AS (SELECT Tag, COUNT(*) AS PostCount FROM RecursiveTagSplits GROUP BY Tag),
// UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// BadgeDetails AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId)
// SELECT U.DisplayName, U.Reputation, U.PostCount, U.QuestionCount, U.AnswerCount, COALESCE(B.GoldBadges, 0) AS GoldBadges, COALESCE(B.SilverBadges, 0) AS SilverBadges,
//        COALESCE(B.BronzeBadges, 0) AS BronzeBadges, T.Tag, T.PostCount AS TagPostCount
// FROM UserReputation U LEFT JOIN BadgeDetails B ON U.UserId = B.UserId JOIN TagCounts T ON U.PostCount > 0 ORDER BY U.Reputation DESC, TagPostCount DESC LIMIT 10;
//
// The TagCounts ON clause names only U, so the users with posts are crossed with every tag.
fn q29984(db: &'static So) -> String {
    let tc = db.post.select((&db.post.tags_str).flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let ups = user_posts(db);
    let bd = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let users = drain((&ups).filt(|a| a[1] > 0).and((&bd).opt()));
    let reputation = &db.user.reputation;
    let v = cross_top(users, |&(u, _)| (Reverse(reputation.get(u).unwrap()), u), drain(&tc), |&(t, n)| (Reverse(n), t), 10);
    rows(v.into_iter().map(|((u, (a, b)), (t, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(b.unwrap_or([0, 0, 0]).map(V::I));
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'),
// UserActivity AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS TotalPosts, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id),
// PostHistorySummary AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS HistoryCount FROM PostHistory ph GROUP BY ph.PostId, ph.PostHistoryTypeId)
// SELECT u.DisplayName, ua.TotalPosts, ua.TotalBounties, rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, phs.HistoryCount,
//        CASE WHEN ua.TotalPosts > 5 THEN 'Active User' ELSE 'New User' END AS UserStatus
// FROM Users u JOIN UserActivity ua ON u.Id = ua.UserId LEFT JOIN RankedPosts rp ON u.Id = rp.PostRank LEFT JOIN PostHistorySummary phs ON rp.PostId = phs.PostId
// WHERE ua.TotalPosts > 0 ORDER BY ua.TotalBounties DESC, rp.ViewCount DESC LIMIT 10;
//
// `u.Id = rp.PostRank` joins a user id to a rank, so it goes through the raw id.
fn q382(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt())), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap())), false);
    let rk = rel(per_group(v, |&(_, u)| u).into_iter().map(|((p, _), r)| (r, p)).collect());
    let by_rank: HashIdx<i64, Id<Post>> = (&rk).map(|(r, _)| r).inv().select((&rk).map(|(_, p)| p)).collect();
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let phs = db.post_history.group_by(post.and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let phv = rel(drain(&phs));
    let by_post: HashIdx<Id<Post>, i64> = (&phv).map(|((p, _), _)| p).inv().select((&phv).map(|(_, n)| n)).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold(0i64, |n, b| n + b.flatten().flatten().unwrap_or(0));
    let j = (&ua).and(user_distinct_posts(db).filt(|n| n > 0)).and((&db.user.origid).select((&by_rank).select(Ident::<Post>::new().and((&by_post).opt()))).opt());
    let v = top_n(drain(j), |&(u, ((b, _), rp))| {
        let w = rp.and_then(|(p, _)| view_count.get(p));
        (Reverse(b), w.is_none(), Reverse(w), u, rp)
    }, 10);
    rows(v.into_iter().map(|(u, ((b, n), rp))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(b)];
        f.extend(match rp {
            Some((p, h)) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "views", "score"]);
                g.push(harness::fmt::oint(h));
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::S(if n > 5 { "Active User" } else { "New User" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 1000
//     GROUP BY U.Id, U.Reputation, U.DisplayName),
// TopUsers AS (SELECT UserId, Reputation, DisplayName, PostCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, U.DisplayName AS OwnerName, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT V.UserId) AS VoteCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.Score, P.ViewCount, U.DisplayName HAVING COUNT(DISTINCT V.UserId) > 10),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, OwnerName, CommentCount, VoteCount, ROW_NUMBER() OVER (ORDER BY VoteCount DESC) AS Rank FROM PopularPosts)
// SELECT T.UserId, T.Reputation, T.DisplayName, T.PostCount, TP.PostId, TP.Title, TP.Score, TP.ViewCount, TP.OwnerName, TP.CommentCount, TP.VoteCount
// FROM TopUsers T JOIN TopPosts TP ON T.PostCount > 5 WHERE T.Rank <= 10 AND TP.Rank <= 5 ORDER BY T.Reputation DESC, TP.VoteCount DESC;
//
// The ON clause names only T, so the top users with more than five posts are crossed with the top posts.
fn q9650(db: &'static So) -> String {
    let reputation = &db.user.reputation;
    let tu = top_n(drain(db.user.with(reputation.gt(1000)).select(user_distinct_posts(db).filt(|n| n > 0))), |&(u, _)| (Reverse(reputation.get(u).unwrap()), u), 10);
    let tu = rel(drain(rel(tu).filt(|(_, n)| n > 5)).into_iter().map(|x| x.1).collect());
    let recent = || db.post.with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let vu = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.user).opt()).opt()).buf_fold(|v| distinct_some(v.iter().map(|x| x.flatten())));
    let s = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let tp = top_n(drain((&vu).filt(|n| n > 10).and(&s)), |&(p, (n, _))| (Reverse(n), p), 5);
    let tp = rel(tp);
    let mut v = Vec::new();
    (&tu).cross(&tp).drive(|_, ((u, n), (p, (k, c)))| v.push((u, n, p, k, c)));
    v.sort_by_key(|&(u, _, _, k, _)| (Reverse(reputation.get(u).unwrap()), Reverse(k)));
    rows(v.into_iter().map(|(u, n, p, k, c)| {
        let mut f = ucols(db, u, &["uid", "rep", "name"]);
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "owner"]));
        f.extend([V::I(c), V::I(k)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY COALESCE(SUM(v.VoteTypeId), 0) DESC) AS ActivityRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(ph.Comment, 'No Comment') AS LastEditComment, ROW_NUMBER() OVER (ORDER BY p.LastActivityDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (4, 5))
// SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalComments, ua.TotalUpvotes - ua.TotalDownvotes AS NetVotes, ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.LastEditComment
// FROM UserActivity ua FULL OUTER JOIN PostStatistics ps ON ua.UserId = ps.PostId
// WHERE (ua.TotalPosts > 0 OR ps.PostId IS NOT NULL) AND (ua.TotalUpvotes - ua.TotalDownvotes) > 10 ORDER BY NetVotes DESC, ua.DisplayName, ps.PostRank LIMIT 50;
//
// The WHERE needs a UserActivity row, so the FULL OUTER JOIN keeps only its left side. `ua.UserId = ps.PostId` joins a user id to a post id, so it goes through the raw ids.
// PostRank orders by LastActivityDate; the history rows of one post tie on it, and the port orders them by history id.
fn q2507(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt())
        .fold(0i64, |n, x| {
            let t = x.and_then(|(t, _)| t);
            n + (t == Some(2)) as i64 - (t == Some(3)) as i64
        });
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let Post { origid, last_activity_date, .. } = &db.post;
    let pidx: HashIdx<i64, Id<Post>> = origid.inv().collect();
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5])));
    let j = (&ua).filt(|n| n > 10).and(user_distinct_posts(db)).and(&cc).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(edits.opt())).opt());
    let name = &db.user.display_name;
    let v = top_n(drain(j), |&(u, (_, ps))| (Reverse(ua.get(u).unwrap()), name.get(u).unwrap(), ps.map(|(p, _)| Reverse(last_activity_date.get(p).unwrap())), ps.map(|(_, h)| h), u), 50);
    rows(v.into_iter().map(|(u, (((nv, n), c), ps))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c), V::I(nv)]);
        f.extend(match ps {
            Some((p, h)) => {
                let mut g = post_fields(db, p, &["id", "title", "score", "views"]);
                g.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No Comment")));
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS EditCount, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.ViewCount),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.UpVotes, ps.DownVotes, ps.CommentCount, ps.EditCount, ps.BadgeCount,
//        RANK() OVER (ORDER BY ps.Score DESC, ps.ViewCount DESC, ps.UpVotes DESC) AS PostRank FROM PostStats ps)
// SELECT rp.Title, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes, rp.CommentCount, rp.EditCount, rp.BadgeCount, u.DisplayName AS OwnerDisplayName
// FROM RankedPosts rp JOIN Users u ON rp.PostId = u.Id WHERE rp.PostRank <= 100 ORDER BY rp.PostRank;
//
// PostRank leads with (Score, ViewCount), so only posts at or above the hundredth such key can rank in the top hundred; the vote x comment x history x badge
// product is driven for those alone. `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q9321(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, origid, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let recent = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score));
    let hundredth = key(top_n(recent.clone(), |&(p, _)| key(p), 100).last().unwrap().0);
    let cand: MatSet<Id<Post>> = rel(recent.into_iter().map(|x| x.0).collect()).filt(move |p| key(p) <= hundredth).collect();
    let s = (&cand)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 2], |a, (((t, _), _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cnt = |r: &'static HashIdx<Id<Post>, _>| (&cand).group_by(Ident::<Post>::new()).select(r.opt()).fold(0i64, |n, x: Option<_>| n + x.is_some() as i64);
    let (cc, hc) = (cnt(comments_of(db)), (&cand).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, x| n + x.is_some() as i64));
    let bc = (&cand).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let v = ranked(drain((&s).and(&cc).and(&hc).and(&bc)), |&(p, (((a, _), _), _))| (key(p), Reverse(a[0])), false);
    let rk = rel(v.into_iter().take_while(|x| x.1 <= 100).map(|((p, x), r)| (p, (x, r))).collect());
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let by_post: HashIdx<Id<Post>, (Id<Post>, (((([i64; 2], i64), i64), i64), i64))> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let mut v = drain((&by_post).map(|(_, x)| x).and(origid.select(&uidx)));
    v.sort_by_key(|&(_, ((_, r), _))| r);
    rows(v.into_iter().map(|(p, (((((a, c), h), b), _), u))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(h), V::I(b), user_col(db, u, "name")]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPostCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, ClosedPostCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserActivity),
// UserBadges AS (SELECT UB.UserId, COUNT(UB.Id) AS BadgeCount FROM Badges UB GROUP BY UB.UserId),
// UserMetrics AS (SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.PostCount, TU.AnswerCount, TU.ClosedPostCount, UB.BadgeCount, (TU.Reputation + COALESCE(UB.BadgeCount, 0) * 10) AS MetricScore
//     FROM TopUsers TU LEFT JOIN UserBadges UB ON TU.UserId = UB.UserId)
// SELECT UM.UserId, UM.DisplayName, UM.Reputation, UM.PostCount, UM.AnswerCount, UM.ClosedPostCount, UM.BadgeCount, UM.MetricScore FROM UserMetrics UM WHERE UM.MetricScore > 100
// ORDER BY UM.MetricScore DESC LIMIT 10;
fn q5715(db: &'static So) -> String {
    let Post { post_type_id, closed_date, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(closed_date.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, c)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64],
        None => a,
    });
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let reputation = &db.user.reputation;
    let ms = |u: Id<User>, b: Option<i64>| reputation.get(u).unwrap() + b.unwrap_or(0) * 10;
    let j = db.user.select(Ident::<User>::new().and(&ua).and((&ub).opt())).filt(move |((u, _), b): ((Id<User>, [i64; 3]), Option<i64>)| ms(u, b) > 100);
    let v = top_n(drain(j), |&(u, (_, b))| (Reverse(ms(u, b)), u), 10);
    rows(v.into_iter().map(|(u, ((_, a), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([harness::fmt::oint(b), V::I(ms(u, b))]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, LastAccessDate, CASE WHEN Reputation < 1000 THEN 'Newbie' WHEN Reputation BETWEEN 1000 AND 10000 THEN 'Experienced' ELSE 'Veteran' END AS ReputationTier
//     FROM Users),
// PostStats AS (SELECT p.Id AS PostId, p.OwnerUserId, p.CreationDate, COUNT(CASE WHEN c.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        MAX(COALESCE(p.ClosedDate, p.LastActivityDate)) AS LastActivity
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > '2023-01-01' GROUP BY p.Id, p.OwnerUserId, p.CreationDate),
// UserPostStats AS (SELECT u.DisplayName, u.Reputation, ur.ReputationTier, ps.PostId, ps.CommentCount, ps.VoteCount, ps.LastActivity
//     FROM UserReputation ur JOIN PostStats ps ON ur.Id = ps.OwnerUserId JOIN Users u ON ur.Id = u.Id),
// TopPosts AS (SELECT *, RANK() OVER (PARTITION BY ReputationTier ORDER BY VoteCount DESC) AS VoteRank, RANK() OVER (PARTITION BY ReputationTier ORDER BY CommentCount DESC) AS CommentRank
//     FROM UserPostStats)
// SELECT DisplayName, Reputation, ReputationTier, PostId, CommentCount, VoteCount, LastActivity FROM TopPosts WHERE VoteRank <= 5 OR CommentRank <= 5
// ORDER BY ReputationTier, GREATEST(VoteCount, CommentCount) DESC;
fn q23249(db: &'static So) -> String {
    let Post { creation_date, owner_user, closed_date, last_activity_date, .. } = &db.post;
    let ps = || db.post.with(creation_date.gt(ts(2023, 1, 1, 0, 0, 0))).with(owner_user);
    let cc = ps().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = ps().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let tier = |r: i64| if r < 1000 { "Newbie" } else if r <= 10000 { "Experienced" } else { "Veteran" };
    let v = drain((&cc).and(&vc).and(owner_user.select(&db.user.reputation).map(tier)));
    let v = ranked(v, |&(_, ((_, n), t))| (t, Reverse(n)), false);
    let v = per_group(v, |&(_, (_, t))| t);
    let v = ranked(v, |&((_, ((c, _), t)), _)| (t, Reverse(c)), false);
    let v = per_group(v, |&((_, (_, t)), _)| t);
    let mut v: Vec<_> = drain(rel(v).filt(|((_, a), b)| a <= 5 || b <= 5)).into_iter().map(|x| x.1 .0 .0).collect();
    v.sort_by_key(|&(_, ((c, n), t))| (t, Reverse(c.max(n))));
    rows(v.into_iter().map(|(p, ((c, n), t))| {
        let u = owner_user.get(p).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::S(t), V::I(db.post.origid.get(p).unwrap()), V::I(c), V::I(n), V::T(closed_date.get(p).unwrap_or(last_activity_date.get(p).unwrap()))]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// RecentPosts AS (SELECT p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT u.DisplayName, u.Reputation, COALESCE(ub.BadgeCount, 0) AS TotalBadges, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate, rp.Score AS RecentPostScore,
//        CASE WHEN rp.Score >= 10 THEN 'Highly Rated' WHEN rp.Score < 0 THEN 'Not Well Received' ELSE 'Average' END AS PostRating
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN RecentPosts rp ON u.Id = rp.OwnerUserId AND rp.rn = 1
// WHERE u.Reputation > 100 AND (u.Location IS NOT NULL OR u.WebsiteUrl IS NOT NULL) ORDER BY u.Reputation DESC FETCH FIRST 10 ROWS ONLY;
fn q2850(db: &'static So) -> String {
    let User { reputation, location, website_url, .. } = &db.user;
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let rp = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user));
    let rp = top_per(rp, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let recent: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let users = db.user.with(reputation.gt(100)).with(Ident::<User>::new().with(location).or(Ident::<User>::new().with(website_url)));
    let v = top_n(drain(users.select((&ub).opt().and((&recent).map(|(_, p)| p).opt()))), |&(u, _)| (Reverse(reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.extend(match p {
            Some(p) => {
                let s = score.get(p).unwrap();
                let mut g = post_fields(db, p, &["title", "created", "score"]);
                g.push(V::S(if s >= 10 { "Highly Rated" } else if s < 0 { "Not Well Received" } else { "Average" }));
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::S("Average")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE v.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '30 days' GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, rp.CommentCount, rp.AnswerCount, COALESCE(rv.TotalVotes, 0) AS TotalVotes, COALESCE(rv.UpVotes, 0) AS UpVotes,
//        COALESCE(rv.DownVotes, 0) AS DownVotes
// FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId WHERE rp.Rank <= 10 ORDER BY rp.Rank;
fn q5506(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap())), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let Vote { post, creation_date: vd, .. } = &db.vote;
    let rv = db.vote.with(vd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(post).select(vtype_name(db)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == "UpMod") as i64, a[2] + (t == "DownMod") as i64]);
    let mut v = drain((&cc).and(&ac).and((&rv).opt()));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((c, a), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c), V::I(a)]);
        f.extend(r.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(P.Score) AS TotalScore, AVG(P.Score) AS AvgScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.CreationDate <= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalAnswers, TotalQuestions, TotalScore, AvgScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats WHERE TotalPosts > 0)
// SELECT TU.DisplayName, TU.TotalPosts, TU.TotalAnswers, TU.TotalQuestions, TU.TotalScore, TU.AvgScore, COALESCE(B.BadgeCount, 0) AS TotalBadges, COALESCE(C.CommentCount, 0) AS TotalComments
// FROM TopUsers TU LEFT JOIN (SELECT UserId, COUNT(Id) AS BadgeCount FROM Badges GROUP BY UserId) B ON TU.UserId = B.UserId
// LEFT JOIN (SELECT C.UserId, COUNT(C.Id) AS CommentCount FROM Comments C JOIN Posts P ON C.PostId = P.Id WHERE P.OwnerUserId IS NOT NULL GROUP BY C.UserId) C ON TU.UserId = C.UserId
// WHERE TU.ScoreRank <= 10 ORDER BY TU.TotalScore DESC;
fn q5714(db: &'static So) -> String {
    let ups = user_posts(db);
    let users = db.user.with((&db.user.creation_date).le(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let v = ranked(drain(users.select((&ups).filt(|a| a[1] > 0))), |&(_, a)| Reverse(a[4]), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let b = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let Comment { user, post, .. } = &db.comment;
    let c = db.comment.with(post.select(Ident::<Post>::new().with(&db.post.owner_user))).group_by(user).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    type R = (Id<User>, [i64; 10]);
    let uid = || Same::<R>::new().map(|(u, _): R| u);
    let mut v = drain((&tu).select(Same::<R>::new().and(uid().select((&b).opt())).and(uid().select((&c).opt()))));
    v.sort_by_key(|&(_, (((_, a), _), _))| Reverse(a[4]));
    rows(v.into_iter().map(|(_, (((u, a), b), c))| {
        row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[3]), V::I(a[2]), V::I(a[4]), avg(a[4], a[1]), V::I(b.unwrap_or(0)), V::I(c.unwrap_or(0))])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS TotalComments,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
//        ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT c.Id) DESC, p.LastActivityDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.LastActivityDate, u.DisplayName),
// TopRankedPosts AS (SELECT Rank, PostId, Title, CreationDate, LastActivityDate, OwnerDisplayName, TotalComments, TotalUpVotes, TotalDownVotes FROM RankedPosts WHERE Rank <= 10)
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.LastActivityDate, trp.OwnerDisplayName, trp.TotalComments, trp.TotalUpVotes, trp.TotalDownVotes, (trp.TotalUpVotes - trp.TotalDownVotes) AS NetVotes,
//        EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - trp.LastActivityDate)) AS SecondsSinceLastActivity
// FROM TopRankedPosts trp ORDER BY NetVotes DESC, SecondsSinceLastActivity ASC;
//
// Rank reads only the per-question comment count and LastActivityDate, so the ten questions are picked first and the comment x vote product is driven for those alone.
fn q7497(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain(&cc), |&(p, n)| (Reverse(n), Reverse(last_activity_date.get(p).unwrap())), 10);
    let tp = rel(v);
    let idx: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tp).map(|(p, _)| p).inv().select(&tp).collect();
    let s = (&idx)
        .map(|(p, _)| p)
        .inv()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let mut v = drain((&s).and(&idx));
    v.sort_by_key(|&(p, (a, _))| (Reverse(a[0] - a[1]), t0 - last_activity_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (a, (_, n)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity", "owner"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::F(secs(t0 - last_activity_date.get(p).unwrap()))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1 AND p.Score > 10),
// UserAggregates AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT ua.DisplayName, ua.PostCount, ua.TotalBounty, ua.GoldBadges, ua.SilverBadges, ua.BronzeBadges, rp.Title AS LatestPostTitle, rp.CreationDate AS LatestPostDate
// FROM UserAggregates ua LEFT JOIN RankedPosts rp ON ua.UserId = rp.OwnerUserId AND rp.rn = 1 WHERE ua.PostCount > 0 ORDER BY ua.TotalBounty DESC, ua.PostCount DESC LIMIT 10;
fn q1955(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(bounty.opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (p, c)| [a[0] + p.flatten().flatten().unwrap_or(0), a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]);
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let rp = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)).and(score.gt(10))).with(owner_user).select(owner_user));
    let rp = top_per(rp, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let recent: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let v = top_n(drain((&ua).and(user_distinct_posts(db).filt(|n| n > 0)).and((&recent).map(|(_, p)| p).opt())), |&(u, ((a, n), _))| (Reverse(a[0]), Reverse(n), u), 10);
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserVoteStats AS (SELECT v.UserId, COUNT(DISTINCT v.PostId) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v GROUP BY v.UserId),
// RecentClosePosts AS (SELECT ph.PostId, MIN(ph.CreationDate) AS FirstCloseDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT p.Title, u.DisplayName AS UserName, r.PostRank, COALESCE(uvs.TotalVotes, 0) AS UserTotalVotes, COALESCE(uvs.UpVotes, 0) AS UserUpVotes, COALESCE(uvs.DownVotes, 0) AS UserDownVotes, c.FirstCloseDate
// FROM RankedPosts r JOIN Posts p ON r.PostId = p.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserVoteStats uvs ON u.Id = uvs.UserId LEFT JOIN RecentClosePosts c ON p.Id = c.PostId
// WHERE r.PostRank <= 10 AND (p.ViewCount > 100 OR p.Score > 5) ORDER BY p.CreationDate DESC OFFSET 5 ROWS FETCH NEXT 5 ROWS ONLY;
fn q2381(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap())), false);
    let rk = rel(per_group(v, |&(_, t)| t).into_iter().map(|((p, _), r)| (p, r)).collect());
    let keep = Ident::<Post>::new().with(view_count.gt(100).or(score.gt(5)));
    let by_post: HashIdx<Id<Post>, i64> = (&rk).map(|(p, _)| p).inv().select((&rk).filt(|(_, r)| r <= 10).map(|(_, r)| r)).collect();
    let Vote { user, vote_type_id, post_id, .. } = &db.vote;
    let uvs = db.vote.group_by(user).select(post_id.map(Some).and(vote_type_id)).buf_fold(|v| {
        let up = v.iter().filter(|x| x.1 == 2).count() as i64;
        let dn = v.iter().filter(|x| x.1 == 3).count() as i64;
        [distinct_some(v.iter().map(|x| x.0)), up, dn]
    });
    let PostHistory { post: hp, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let fc = db.post_history.with(post_history_type_id.eq(10)).group_by(hp).select(hd).fold(i64::MAX, |m, d| m.min(d));
    let v = drain(db.post.with(keep).select((&by_post).and(owner_user.select(Ident::<User>::new().and((&uvs).opt())).opt()).and((&fc).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().skip(5).map(|(p, ((r, u), c))| {
        let mut f = post_fields(db, p, &["title"]);
        f.push(u.map_or(V::Null, |(u, _)| user_col(db, u, "name")));
        f.push(V::I(r));
        f.extend(u.and_then(|(_, a)| a).unwrap_or([0; 3]).map(V::I));
        f.push(harness::fmt::ots(c));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(V.BountyAmount) AS TotalBounty, COALESCE(MAX(P.LastActivityDate), '1970-01-01') AS LastActive
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalBounty, LastActive, ROW_NUMBER() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserStats),
// ActiveUsers AS (SELECT * FROM TopUsers WHERE LastActive >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT AU.DisplayName, AU.Reputation, AU.PostCount, AU.QuestionCount, AU.AnswerCount, AU.TotalBounty, AU.Rank, p.Title AS RecentPostTitle, p.CreationDate AS RecentPostDate,
//        p.ViewCount AS RecentPostViews, p.Score AS RecentPostScore
// FROM ActiveUsers AU LEFT JOIN Posts p ON AU.UserId = p.OwnerUserId WHERE p.CreationDate = (SELECT MAX(CreationDate) FROM Posts WHERE OwnerUserId = AU.UserId) ORDER BY AU.Rank LIMIT 10;
//
// Rank, LastActive and the question/answer counts are per-post values the vote join only repeats, so they come from one row per post; the rows are cut first
// and the posts x votes product is driven for the ten survivors' TotalBounty alone.
fn q9823(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, creation_date, .. } = &db.post;
    let reputation = &db.user.reputation;
    let v = ranked(drain(user_distinct_posts(db)), |&(u, n)| (Reverse(reputation.get(u).unwrap()), Reverse(n), u), false);
    let rk = rel(v.into_iter().map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, i64> = (&rk).map(|(u, _)| u).inv().select((&rk).map(|(_, r)| r)).collect();
    let la = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(last_activity_date).opt()).fold(i64::MIN, |m, d| m.max(d.unwrap_or(0)));
    let md = db.post.group_by(&db.post.owner_user).select(creation_date).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<User>, i64), Id<Post>> = db.post.with(&db.post.owner_user).select((&db.post.owner_user).and(creation_date)).inv().collect();
    let active = Ident::<User>::new().with((&la).filt(|d| d >= add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let v = drain(db.user.with(active).select((&by_user).and(Ident::<User>::new().and(&md).select(&at))));
    let v = top_n(v, |&(u, (r, p))| (r, u, p), 10);
    let tu: MatSet<Id<User>> = rel(v.iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
            }
            None => a,
        });
    rows(v.into_iter().map(|(u, (r, p))| {
        let a = s.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(user_distinct_posts(db).get(u).unwrap()), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(r)]);
        f.extend(post_fields(db, p, &["title", "created", "views", "score"]));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COALESCE(SUM(P.ViewCount), 0) AS TotalViews, AVG(P.Score) AS AvgScore, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionCount, AnswerCount, CommentCount, TotalViews, AvgScore, BadgeCount, DENSE_RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank,
//        DENSE_RANK() OVER (ORDER BY QuestionCount DESC) AS QuestionRank, DENSE_RANK() OVER (ORDER BY AnswerCount DESC) AS AnswerRank FROM UserActivity)
// SELECT UserId, DisplayName, QuestionCount, AnswerCount, CommentCount, TotalViews, AvgScore, BadgeCount, ViewRank, QuestionRank, AnswerRank
// FROM TopUsers WHERE ViewRank <= 10 OR QuestionRank <= 10 OR AnswerRank <= 10 ORDER BY ViewRank, QuestionRank, AnswerRank;
fn q26699(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score).and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 6], |a, (p, _)| match p {
            Some((((t, w), s), c)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + 1, a[5] + s],
            None => a,
        });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain((&s).and(&bc)), |&(_, (a, _))| Reverse(a[3]), true);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[0]), true);
    let v = ranked(v, |&(((_, (a, _)), _), _)| Reverse(a[1]), true);
    let mut v: Vec<_> = drain(rel(v).filt(|(((_, x), y), z)| x <= 10 || y <= 10 || z <= 10)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(((_, x), y), z)| (x, y, z));
    rows(v.into_iter().map(|((((u, (a, b)), x), y), z)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4]), V::I(b), V::I(x), V::I(y), V::I(z)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS ScoreRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND P.ViewCount IS NOT NULL),
// TopPosts AS (SELECT PostId, Title, ViewCount, OwnerDisplayName FROM RankedPosts WHERE ScoreRank <= 10),
// PostLinksCTE AS (SELECT PL.PostId, COUNT(*) AS RelatedPostCount FROM PostLinks PL WHERE PL.LinkTypeId = 3 GROUP BY PL.PostId),
// FinalResults AS (SELECT TP.PostId, TP.Title, TP.ViewCount, TP.OwnerDisplayName, COALESCE(PLC.RelatedPostCount, 0) AS RelatedPosts FROM TopPosts TP LEFT JOIN PostLinksCTE PLC ON TP.PostId = PLC.PostId)
// SELECT FR.PostId, FR.Title, FR.ViewCount, FR.OwnerDisplayName, CASE WHEN FR.RelatedPosts = 0 THEN 'No related posts' ELSE CONCAT(FR.RelatedPosts, ' related posts') END AS RelatedPostInfo
// FROM FinalResults FR WHERE FR.ViewCount > (SELECT AVG(ViewCount) FROM FinalResults) ORDER BY FR.ViewCount DESC LIMIT 20;
fn q4470(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(view_count).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let (n, s) = (&tp).select(view_count).fold_flat((0i64, 0i64), |(n, t), w| (n + 1, t + w));
    let mean = s as f64 / n as f64;
    let plc = db.post_link.with((&db.post_link.link_type_id).eq(3)).group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&tp).with(view_count.filt(move |w| w as f64 > mean)).select((&plc).opt())), |&(p, _)| (Reverse(view_count.get(p)), p), 20);
    rows(v.into_iter().map(|(p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "owner"]);
        f.push(match c {
            Some(c) if c > 0 => V::Owned(format!("{c} related posts")),
            _ => V::S("No related posts"),
        });
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("9018", q9018),
    ("1543", q1543),
    ("29701", q29701),
    ("8003", q8003),
    ("27726", q27726),
    ("2058", q2058),
    ("6332", q6332),
    ("2112", q2112),
    ("25587", q25587),
    ("5648", q5648),
    ("7362", q7362),
    ("8156", q8156),
    ("2977", q2977),
    ("21065", q21065),
    ("8817", q8817),
    ("4713", q4713),
    ("9495", q9495),
    ("292", q292),
    ("4404", q4404),
    ("6007", q6007),
    ("9674", q9674),
    ("1798", q1798),
    ("9044", q9044),
    ("2515", q2515),
    ("3865", q3865),
    ("890", q890),
    ("30071", q30071),
    ("3888", q3888),
    ("4178", q4178),
    ("9283", q9283),
    ("1732", q1732),
    ("7029", q7029),
    ("2444", q2444),
    ("9599", q9599),
    ("3082", q3082),
    ("27946", q27946),
    ("3297", q3297),
    ("6467", q6467),
    ("8510", q8510),
    ("5042", q5042),
    ("6443", q6443),
    ("6656", q6656),
    ("7280", q7280),
    ("1141", q1141),
    ("2883", q2883),
    ("2943", q2943),
    ("263", q263),
    ("1300", q1300),
    ("1615", q1615),
    ("460", q460),
    ("8820", q8820),
    ("161", q161),
    ("160", q160),
    ("4788", q4788),
    ("285", q285),
    ("1781", q1781),
    ("1168", q1168),
    ("3328", q3328),
    ("5097", q5097),
    ("1371", q1371),
    ("7021", q7021),
    ("2602", q2602),
    ("7553", q7553),
    ("26324", q26324),
    ("9842", q9842),
    ("6727", q6727),
    ("3148", q3148),
    ("5726", q5726),
    ("2282", q2282),
    ("7235", q7235),
    ("39", q39),
    ("34294", q34294),
    ("1905", q1905),
    ("3741", q3741),
    ("752", q752),
    ("804", q804),
    ("6942", q6942),
    ("6419", q6419),
    ("29984", q29984),
    ("382", q382),
    ("9650", q9650),
    ("2507", q2507),
    ("9321", q9321),
    ("5715", q5715),
    ("23249", q23249),
    ("2850", q2850),
    ("5506", q5506),
    ("5714", q5714),
    ("7497", q7497),
    ("1955", q1955),
    ("2381", q2381),
    ("9823", q9823),
    ("26699", q26699),
    ("4470", q4470),
];
